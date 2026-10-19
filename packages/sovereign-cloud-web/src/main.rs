//! Small browser facade: all identity and file operations use the same Rust SDK as Craft.
#![forbid(unsafe_code)]
#![deny(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
use futures::channel::oneshot;
use sovereign_cloud::browser::{self, BrowserClient, BrowserSession};
use sovereign_cloud::email::*;
use sovereign_cloud::people::*;
use sovereign_cloud::public_link::*;
use sovereign_cloud::search::*;
use sovereign_cloud::spaces::*;
use sovereign_cloud::{ObjectId, args, drive::*};
use sovereign_cloud_web::requests;
use std::{
    cell::{Cell, RefCell},
    rc::Rc,
    sync::{Arc, Mutex},
};
use wasm_bindgen::prelude::*;

fn main() {}
fn js_error(error: impl ToString) -> JsValue {
    js_sys::Error::new(&error.to_string()).into()
}
fn json(value: serde_json::Value) -> Result<JsValue, JsValue> {
    js_sys::JSON::parse(&value.to_string())
}
fn sequence(value: &str) -> Result<u64, JsValue> {
    value
        .parse()
        .map_err(|_| js_error("Invalid revision or operation number"))
}

fn object_id(value: &str) -> Result<ObjectId, JsValue> {
    value.parse().map_err(js_error)
}

#[wasm_bindgen]
pub struct CloudPortal {
    session: Rc<RefCell<Option<BrowserSession>>>,
    signed_in: Cell<bool>,
}
#[wasm_bindgen]
impl CloudPortal {
    #[wasm_bindgen(constructor)]
    pub fn new() -> Self {
        Self {
            session: Rc::new(RefCell::new(None)),
            signed_in: Cell::new(false),
        }
    }

    /// Rehydrate from the shared adapter, never from independently interpreted JS credentials.
    pub async fn restore(&self) -> Result<JsValue, JsValue> {
        let session = BrowserSession::load().await.map_err(js_error)?;
        let signed_in = session.is_authenticated();
        let state = if signed_in {
            let home: BrowseHome = BrowserClient::new(&session)
                .map_err(js_error)?
                .drive_call("drive_browser_home", args(()).map_err(js_error)?)
                .await
                .map_err(js_error)?;
            let account = home.account;
            let preferences = home.page.preferences.clone();
            if !session.is_authenticated() {
                return Err(js_error("Session changed during account loading"));
            }
            serde_json::json!({"authenticated":true, "principal":account.account.owner.to_text(), "accountId":account.account.id.to_string(), "spaceId":account.account.personal_space.to_string(), "usedBytes":account.used_bytes, "quotaBytes":account.quota_bytes, "home":query_json(home.page)?, "preferences":preferences_json(preferences)})
        } else {
            serde_json::json!({"authenticated":false})
        };
        self.signed_in.set(signed_in);
        *self.session.borrow_mut() = Some(session);
        json(state)
    }
    pub fn needs_refresh(&self) -> bool {
        self.session
            .borrow()
            .as_ref()
            .is_none_or(|s| !s.is_current() || s.is_authenticated() != self.signed_in.get())
    }
    /// Starts the popup synchronously inside the browser's click gesture.
    pub fn sign_in(&self) -> Result<js_sys::Promise, JsValue> {
        let session = self
            .session
            .borrow()
            .clone()
            .ok_or_else(|| js_error("Sign-in is still loading"))?;
        let (sender, receiver) = oneshot::channel::<Result<(), String>>();
        let success = Arc::new(Mutex::new(Some(sender)));
        let failure = success.clone();
        browser::login(
            &session,
            move || {
                if let Some(sender) = success
                    .lock()
                    .unwrap_or_else(std::sync::PoisonError::into_inner)
                    .take()
                {
                    let _ = sender.send(Ok(()));
                }
            },
            move |e| {
                if let Some(sender) = failure
                    .lock()
                    .unwrap_or_else(std::sync::PoisonError::into_inner)
                    .take()
                {
                    let _ = sender.send(Err(e));
                }
            },
        )
        .map_err(js_error)?;
        Ok(wasm_bindgen_futures::future_to_promise(async move {
            receiver.await.map_err(js_error)?.map_err(js_error)?;
            Ok(JsValue::UNDEFINED)
        }))
    }
    pub async fn sign_out(&self) -> Result<(), JsValue> {
        let session = self.session.borrow().clone();
        if let Some(session) = session {
            session.logout().await.map_err(js_error)?;
        }
        self.signed_in.set(false);
        Ok(())
    }
    pub async fn public_read(
        &self,
        operation: String,
        payload: String,
    ) -> Result<JsValue, JsValue> {
        let p: serde_json::Value = serde_json::from_str(&payload).map_err(js_error)?;
        let id = object_id(p["id"].as_str().ok_or_else(|| js_error("Missing item"))?)?;
        let client = BrowserClient::public_reader().map_err(js_error)?;
        let value = match operation.as_str() {
            "get" => {
                let item: PublicItem = client
                    .drive_call("public_get", args((id,)).map_err(js_error)?)
                    .await
                    .map_err(js_error)?;
                public_item_json(item)
            }
            "list" => {
                let cursor: Option<PageCursor> = p["cursor"]
                    .as_str()
                    .map(serde_json::from_str)
                    .transpose()
                    .map_err(js_error)?;
                let page: PublicPage = client
                    .drive_call("public_list", args((id, cursor, 50u32)).map_err(js_error)?)
                    .await
                    .map_err(js_error)?;
                serde_json::json!({"entries":page.entries.into_iter().map(public_item_json).collect::<Vec<_>>(),"cursor":page.next.map(|c|serde_json::to_string(&c)).transpose().map_err(js_error)?})
            }
            _ => return Err(js_error("Unknown public read")),
        };
        json(value)
    }
    pub async fn public_download(&self, file: String) -> Result<Vec<u8>, JsValue> {
        BrowserClient::public_reader()
            .map_err(js_error)?
            .public_download(object_id(&file)?)
            .await
            .map_err(js_error)
    }
    /// The facade translates UI data only. Authorization stays in the canister.
    pub async fn drive(&self, operation: String, payload: String) -> Result<JsValue, JsValue> {
        let session = self
            .session
            .borrow()
            .clone()
            .ok_or_else(|| js_error("Sign in first"))?;
        let client = BrowserClient::new(&session).map_err(js_error)?;
        let p: serde_json::Value = serde_json::from_str(&payload).map_err(js_error)?;
        let string = |key: &str| -> Result<String, JsValue> {
            p[key]
                .as_str()
                .map(str::to_owned)
                .ok_or_else(|| js_error("Missing field"))
        };
        let number = |key: &str| -> Result<u64, JsValue> { sequence(&string(key)?) };
        let object = |key: &str| -> Result<ObjectId, JsValue> { object_id(&string(key)?) };
        let value = match operation.as_str() {
            "contentQuery" => {
                let request = ContentQuery {
                    text: string("text")?,
                    folder: p["parent"].as_str().map(object_id).transpose()?,
                    cursor: p["cursor"]
                        .as_str()
                        .map(serde_json::from_str)
                        .transpose()
                        .map_err(js_error)?,
                    limit: 50,
                };
                let page: ContentPage = client
                    .drive_call("content_query", args((request,)).map_err(js_error)?)
                    .await
                    .map_err(js_error)?;
                let entries=page.hits.into_iter().map(|hit|{let mut entry=entry_json(hit.entry);entry["contentMatch"]=serde_json::json!({"revision":hit.revision.to_string(),"location":hit.location,"snippet":hit.snippet,"partial":hit.partial});entry}).collect::<Vec<_>>();
                serde_json::json!({"entries":entries,"cursor":page.next.map(|c|serde_json::to_string(&c)).transpose().map_err(js_error)?})
            }
            "contentStatus" | "contentRebuild" => {
                let status: ContentStatus = client
                    .drive_call(
                        if operation == "contentStatus" {
                            "content_status"
                        } else {
                            "content_rebuild"
                        },
                        args((object("id")?,)).map_err(js_error)?,
                    )
                    .await
                    .map_err(js_error)?;
                serde_json::json!({"state":status.state,"revision":status.revision.to_string(),"extractorVersion":status.extractor_version,"sections":status.sections})
            }
            "publicLink" | "publicLinkSet" => {
                let state: PublicLinkState = if operation == "publicLink" {
                    client
                        .drive_call(
                            "public_link_state",
                            args((object("id")?,)).map_err(js_error)?,
                        )
                        .await
                } else {
                    client
                        .drive_call(
                            "public_link_set",
                            args((SetPublicLink {
                                object: object("id")?,
                                enabled: p["enabled"].as_bool().unwrap_or(false),
                                expires_at: p["expiresAt"].as_str().map(sequence).transpose()?,
                                expected_version: number("version")?,
                                request_id: string("requestId")?,
                            },))
                            .map_err(js_error)?,
                        )
                        .await
                }
                .map_err(js_error)?;
                serde_json::json!({"enabled":state.direct.enabled,"version":state.direct.version.to_string(),"expiresAt":state.direct.expires_at.map(|v|v.to_string()),"inherited":state.inherited,"effective":state.effective,"allowed":state.allowed})
            }
            "spaces" => {
                let spaces: Vec<SpaceView> = client
                    .drive_call("space_list", args(()).map_err(js_error)?)
                    .await
                    .map_err(js_error)?;
                serde_json::Value::Array(spaces.into_iter().map(space_view_json).collect())
            }
            "spaceGet" | "spaceCreate" => {
                let space: SpaceView = if operation == "spaceGet" {
                    client
                        .drive_call("space_get", args((object("id")?,)).map_err(js_error)?)
                        .await
                } else {
                    client
                        .drive_call(
                            "space_create",
                            args((CreateSpace {
                                name: string("name")?,
                                request_id: string("requestId")?,
                            },))
                            .map_err(js_error)?,
                        )
                        .await
                }
                .map_err(js_error)?;
                space_view_json(space)
            }
            "spaceMembers" => {
                let after = p["after"]
                    .as_str()
                    .map(candid::Principal::from_text)
                    .transpose()
                    .map_err(js_error)?;
                let page: SpaceMembers = client
                    .drive_call(
                        "space_members",
                        args((object("id")?, after, 50u32)).map_err(js_error)?,
                    )
                    .await
                    .map_err(js_error)?;
                serde_json::json!({"space":team_json(page.space), "members":page.members.into_iter().map(|m|serde_json::json!({"principal":m.principal.to_text(),"role":m.role})).collect::<Vec<_>>(), "next":page.next.map(|p|p.to_text())})
            }
            "spaceMember" | "spaceSettings" => {
                let space: TeamSpace = if operation == "spaceMember" {
                    let role = p["role"]
                        .as_str()
                        .map(|r| match r {
                            "Manager" => Ok(SpaceRole::Manager),
                            "Editor" => Ok(SpaceRole::Editor),
                            "Viewer" => Ok(SpaceRole::Viewer),
                            _ => Err(js_error("Invalid team role")),
                        })
                        .transpose()?;
                    client
                        .drive_call(
                            "space_change_member",
                            args((ChangeMember {
                                space: object("id")?,
                                principal: candid::Principal::from_text(string("principal")?)
                                    .map_err(js_error)?,
                                role,
                                expected_version: number("version")?,
                                request_id: string("requestId")?,
                            },))
                            .map_err(js_error)?,
                        )
                        .await
                } else {
                    client
                        .drive_call(
                            "space_change",
                            args((ChangeSpace {
                                space: object("id")?,
                                name: string("name")?,
                                external_sharing: p["externalSharing"]
                                    .as_bool()
                                    .ok_or_else(|| js_error("Missing sharing policy"))?,
                                expected_version: number("version")?,
                                request_id: string("requestId")?,
                            },))
                            .map_err(js_error)?,
                        )
                        .await
                }
                .map_err(js_error)?;
                team_json(space)
            }
            "emailProfile" | "emailRequest" => {
                let profile: EmailProfile = if operation == "emailProfile" {
                    client
                        .update("email_profile", args(()).map_err(js_error)?)
                        .await
                } else {
                    client
                        .update(
                            "email_request",
                            args((EmailRequest {
                                email: string("email")?,
                                request_id: string("requestId")?,
                            },))
                            .map_err(js_error)?,
                        )
                        .await
                }
                .map_err(js_error)?;
                serde_json::json!({"enabled":profile.enabled,"emails":profile.emails,"pending":profile.pending.map(|v|serde_json::json!({"id":v.id.to_string(),"email":v.email,"expiresAt":v.expires_at.to_string(),"delivery":v.delivery,"transferRequired":v.transfer_required}))})
            }
            "emailVerify" | "emailConfirmTransfer" => {
                let result: EmailVerified = if operation == "emailVerify" {
                    client
                        .update(
                            "email_verify",
                            args((number("id")?, string("code")?)).map_err(js_error)?,
                        )
                        .await
                } else {
                    client
                        .update(
                            "email_confirm_transfer",
                            args((number("id")?,)).map_err(js_error)?,
                        )
                        .await
                }
                .map_err(js_error)?;
                serde_json::json!(result)
            }
            "emailRemove" | "emailRetry" | "emailRevoke" => {
                let (method, arguments) =
                    requests::email_action(&operation, &p).map_err(js_error)?;
                client
                    .update::<()>(method, arguments)
                    .await
                    .map_err(js_error)?;
                serde_json::Value::Null
            }
            "emailShare" => {
                let role = match string("role")?.as_str() {
                    "viewer" => EmailRole::Viewer,
                    "editor" => EmailRole::Editor,
                    _ => return Err(js_error("Choose Viewer or Editor for email sharing")),
                };
                let result: EmailInvitation = client
                    .update(
                        "email_share",
                        args((EmailShare {
                            grant_expires_at: p["expiresAt"].as_str().map(sequence).transpose()?,
                            object: object("id")?,
                            email: string("email")?,
                            role,
                            request_id: string("requestId")?,
                        },))
                        .map_err(js_error)?,
                    )
                    .await
                    .map_err(js_error)?;
                email_invitation_json(result)
            }
            "emailInvitations" => {
                let list: Vec<EmailInvitation> = client
                    .update(
                        "email_invitations",
                        args((object("id")?,)).map_err(js_error)?,
                    )
                    .await
                    .map_err(js_error)?;
                serde_json::json!(
                    list.into_iter()
                        .map(email_invitation_json)
                        .collect::<Vec<_>>()
                )
            }
            "emailAccept" => {
                let object: ObjectId = client
                    .update("email_accept", args((number("id")?,)).map_err(js_error)?)
                    .await
                    .map_err(js_error)?;
                serde_json::json!({"id":object.to_string()})
            }
            "suggestions" => {
                let home: SuggestedHome = client
                    .drive_call("drive_suggested_home", args(()).map_err(js_error)?)
                    .await
                    .map_err(js_error)?;
                serde_json::json!({"entries":home.files.into_iter().map(suggestion_json).collect::<Vec<_>>(), "quickFolders":home.folders.into_iter().map(suggestion_json).collect::<Vec<_>>(), "cursor":null})
            }
            "acknowledge" => {
                client
                    .drive_call::<()>(
                        "drive_acknowledge_open",
                        args((
                            object("id")?,
                            p["revision"].as_str().map(sequence).transpose()?,
                        ))
                        .map_err(js_error)?,
                    )
                    .await
                    .map_err(js_error)?;
                serde_json::Value::Null
            }
            "dismiss" => {
                client
                    .drive_call::<()>(
                        "drive_dismiss_suggestion",
                        args((object("id")?,)).map_err(js_error)?,
                    )
                    .await
                    .map_err(js_error)?;
                serde_json::Value::Null
            }
            "query" => {
                let scope = match string("scope")?.as_str() {
                    "item" => BrowseScope::Item(object("id")?),
                    "folder" => BrowseScope::Folder(object("parent")?),
                    "subtree" => BrowseScope::Subtree(object("parent")?),
                    "all" => BrowseScope::All,
                    "shared" => BrowseScope::Shared,
                    "recent" => BrowseScope::Recent,
                    "starred" => BrowseScope::Starred,
                    "trash" => BrowseScope::Trash,
                    _ => return Err(js_error("Unknown browse scope")),
                };
                let millis = |key: &str| -> Result<Option<u64>, JsValue> {
                    p[key]
                        .as_u64()
                        .map(|ms| {
                            ms.checked_mul(1_000_000)
                                .ok_or_else(|| js_error("Invalid date"))
                        })
                        .transpose()
                };
                let query = BrowseQuery {
                    scope,
                    text: p["text"].as_str().unwrap_or_default().into(),
                    file_type: p["fileType"].as_str().unwrap_or_default().into(),
                    owner: p["owner"]
                        .as_str()
                        .map(str::parse)
                        .transpose()
                        .map_err(js_error)?,
                    modified_after: millis("afterMs")?,
                    modified_before: millis("beforeMs")?,
                    sort: sort_field(p["sort"].as_str().unwrap_or("name"))?,
                    descending: p["descending"].as_bool().unwrap_or(false),
                };
                let cursor: Option<BrowseCursor> = p["cursor"]
                    .as_str()
                    .map(serde_json::from_str)
                    .transpose()
                    .map_err(js_error)?;
                let page: BrowsePage = client
                    .drive_call(
                        "drive_query",
                        args((query, cursor, 50u32)).map_err(js_error)?,
                    )
                    .await
                    .map_err(js_error)?;
                query_json(page)?
            }
            "preferences" => {
                let view = match string("view")?.as_str() {
                    "list" => ViewMode::List,
                    "grid" => ViewMode::Grid,
                    _ => return Err(js_error("Unknown view")),
                };
                client
                    .drive_call::<()>(
                        "drive_set_preferences",
                        args((BrowsePreferences {
                            view,
                            sort: sort_field(&string("sort")?)?,
                            descending: p["descending"].as_bool().unwrap_or(false),
                        },))
                        .map_err(js_error)?,
                    )
                    .await
                    .map_err(js_error)?;
                serde_json::Value::Null
            }
            "star" => {
                client
                    .drive_call::<()>(
                        "drive_star",
                        args((
                            object("id")?,
                            p["starred"]
                                .as_bool()
                                .ok_or_else(|| js_error("Missing starred state"))?,
                        ))
                        .map_err(js_error)?,
                    )
                    .await
                    .map_err(js_error)?;
                serde_json::Value::Null
            }
            "open" => {
                let entry: DriveEntry = client
                    .drive_call("drive_open", args((object("id")?,)).map_err(js_error)?)
                    .await
                    .map_err(js_error)?;
                entry_json(entry)
            }
            "account" => {
                let account = client.account().await.map_err(js_error)?;
                serde_json::json!({"principal":account.account.owner.to_text(), "usedBytes":account.used_bytes, "quotaBytes":account.quota_bytes})
            }
            "initialize" | "get" | "folder" => {
                let (method, input) = match operation.as_str() {
                    "initialize" => ("drive_initialize", args(())),
                    "get" => ("drive_get", args((object("id")?,))),
                    _ => (
                        "drive_create_folder",
                        args((CreateFolder {
                            parent: object("parent")?,
                            name: string("name")?,
                            request_id: string("requestId")?,
                        },)),
                    ),
                };
                let entry: DriveEntry = client
                    .drive_call(method, input.map_err(js_error)?)
                    .await
                    .map_err(js_error)?;
                entry_json(entry)
            }
            "browse" => {
                let cursor: Option<DriveCursor> = p["cursor"]
                    .as_str()
                    .map(serde_json::from_str)
                    .transpose()
                    .map_err(js_error)?;
                let view: DriveBrowse = client
                    .drive_call(
                        "drive_browse",
                        args((object("parent")?, cursor, 50u32)).map_err(js_error)?,
                    )
                    .await
                    .map_err(js_error)?;
                browse_json(view)?
            }
            "list" => {
                let page: DrivePage = client
                    .drive_call("drive_list", requests::list(&p).map_err(js_error)?)
                    .await
                    .map_err(js_error)?;
                serde_json::json!({"entries":page.entries.into_iter().map(entry_json).collect::<Vec<_>>(), "cursor":page.next.map(|c| serde_json::to_string(&c)).transpose().map_err(js_error)?})
            }
            "trash" => {
                let cursor: Option<DriveCursor> = p["cursor"]
                    .as_str()
                    .map(serde_json::from_str)
                    .transpose()
                    .map_err(js_error)?;
                let page: DrivePage = client
                    .drive_call("drive_trash", args((cursor, 50u32)).map_err(js_error)?)
                    .await
                    .map_err(js_error)?;
                serde_json::json!({"entries":page.entries.into_iter().map(entry_json).collect::<Vec<_>>(), "cursor":page.next.map(|c|serde_json::to_string(&c)).transpose().map_err(js_error)?})
            }
            "purge" => {
                let items = p["items"]
                    .as_array()
                    .ok_or_else(|| js_error("Missing items"))?
                    .iter()
                    .map(|item| {
                        Ok(MutationReceipt {
                            object: object_id(
                                item["id"].as_str().ok_or_else(|| js_error("Missing ID"))?,
                            )?,
                            version: sequence(
                                item["version"]
                                    .as_str()
                                    .ok_or_else(|| js_error("Missing version"))?,
                            )?,
                        })
                    })
                    .collect::<Result<Vec<_>, JsValue>>()?;
                let receipt: PurgeReceipt = client
                    .drive_call(
                        "drive_purge_trash",
                        args((PurgeRequest {
                            items,
                            request_id: string("requestId")?,
                        },))
                        .map_err(js_error)?,
                    )
                    .await
                    .map_err(js_error)?;
                serde_json::json!({"deletedObjects":receipt.deleted_objects,"freedBytes":receipt.freed_bytes})
            }
            "details" => {
                let details: DriveDetails = client
                    .drive_call("drive_details", args((object("id")?,)).map_err(js_error)?)
                    .await
                    .map_err(js_error)?;
                let location =
                    |p: ObjectLocation| serde_json::json!({"id":p.id.to_string(),"name":p.name});
                serde_json::json!({"entry":entry_json(details.entry),"locations":details.locations.into_iter().map(location).collect::<Vec<_>>(),
                    "versions":details.revision_count,"access":details.access.into_iter().map(|a| serde_json::json!({"inherited":a.inherited,"location":a.location.map(location),"capabilities":a.capabilities,"expiresMs":a.expires_at.map(|v|v/1_000_000)})).collect::<Vec<_>>()})
            }
            "movePreview" | "restorePreview" => {
                let (method, arguments) = if operation == "restorePreview" {
                    (
                        "drive_restore_preview",
                        args((object("id")?, object("parent")?, string("name")?)),
                    )
                } else {
                    (
                        "drive_move_preview",
                        args((object("id")?, object("parent")?)),
                    )
                };
                let preview: MovePreview = client
                    .drive_call(method, arguments.map_err(js_error)?)
                    .await
                    .map_err(js_error)?;
                serde_json::json!({"version":preview.object_version.to_string(),"treeVersion":preview.tree_version.to_string(),"affectedObjects":preview.affected_objects,"affectedSubjects":preview.affected_subjects,"changes":preview.changes.into_iter().map(|c| serde_json::json!({"subject":match c.subject {Participant::User(p)=>p.to_text(), Participant::Agent(id)=>format!("Agent {id}")},"gains":c.gains,"loses":c.loses,"objects":c.objects})).collect::<Vec<_>>()})
            }
            "organize" => {
                let change = match string("change")?.as_str() {
                    "rename" => OrganizationChange::Rename(string("name")?),
                    "move" => {
                        if p["treeVersion"].is_string() {
                            OrganizationChange::MoveReviewed {
                                parent: object("parent")?,
                                tree_version: number("treeVersion")?,
                            }
                        } else {
                            OrganizationChange::Move(object("parent")?)
                        }
                    }
                    "trash" => OrganizationChange::Trash,
                    "restore" => OrganizationChange::Restore,
                    "restoreTo" => OrganizationChange::RestoreReviewed {
                        parent: object("parent")?,
                        name: string("name")?,
                        tree_version: number("treeVersion")?,
                    },
                    _ => return Err(js_error("Unknown operation")),
                };
                let request = OrganizeRequest {
                    object: object("id")?,
                    expected_version: number("version")?,
                    request_id: string("requestId")?,
                    change,
                };
                let receipt: MutationReceipt = client
                    .drive_call("drive_organize", args((request,)).map_err(js_error)?)
                    .await
                    .map_err(js_error)?;
                serde_json::json!({"id":receipt.object.to_string(),"version":receipt.version.to_string()})
            }
            "shortcut" => {
                let request = CreateShortcut {
                    parent: object("parent")?,
                    target: object("target")?,
                    name: string("name")?,
                    request_id: string("requestId")?,
                };
                let entry: DriveEntry = client
                    .drive_call("drive_create_shortcut", args((request,)).map_err(js_error)?)
                    .await
                    .map_err(js_error)?;
                entry_json(entry)
            }
            "shortcutTarget" => {
                let entry: DriveEntry = client
                    .drive_call(
                        "drive_shortcut_target",
                        args((object("id")?,)).map_err(js_error)?,
                    )
                    .await
                    .map_err(js_error)?;
                entry_json(entry)
            }
            "sharingSafety" => {
                let result: SharingSafety = client
                    .drive_call("drive_sharing_safety", args(()).map_err(js_error)?)
                    .await
                    .map_err(js_error)?;
                serde_json::json!({"blocked":result.blocked.iter().map(|p|p.to_text()).collect::<Vec<_>>(),"hidden":result.hidden.into_iter().map(|h|serde_json::json!({"id":h.object.to_string(),"name":h.name})).collect::<Vec<_>>()})
            }
            "block" => {
                let other: candid::Principal = string("principal")?.parse().map_err(js_error)?;
                let _: () = client
                    .drive_call(
                        "drive_block",
                        args((other, p["blocked"].as_bool().unwrap_or(true))).map_err(js_error)?,
                    )
                    .await
                    .map_err(js_error)?;
                serde_json::Value::Null
            }
            "hideShared" => {
                let _: () = client
                    .drive_call(
                        "drive_hide_shared",
                        args((object("id")?, p["hidden"].as_bool().unwrap_or(true)))
                            .map_err(js_error)?,
                    )
                    .await
                    .map_err(js_error)?;
                serde_json::Value::Null
            }
            "report" => {
                let _: () = client
                    .drive_call(
                        "drive_report",
                        args((object("id")?, string("reason")?)).map_err(js_error)?,
                    )
                    .await
                    .map_err(js_error)?;
                serde_json::Value::Null
            }
            "requestAccess" => {
                let _: () = client
                    .drive_call(
                        "drive_request_access",
                        args((object("id")?, string("message")?)).map_err(js_error)?,
                    )
                    .await
                    .map_err(js_error)?;
                serde_json::Value::Null
            }
            "decideAccess" => {
                let request = AccessDecision {
                    object: object("id")?,
                    requester: string("requester")?.parse().map_err(js_error)?,
                    requested_at: number("requestedAt")?,
                    approve: p["approve"].as_bool().unwrap_or(false),
                    editor: p["role"].as_str() == Some("editor"),
                    expires_at: p["expiresAt"].as_str().map(sequence).transpose()?,
                    request_id: string("requestId")?,
                };
                let _: () = client
                    .drive_call("drive_decide_access", args((request,)).map_err(js_error)?)
                    .await
                    .map_err(js_error)?;
                serde_json::Value::Null
            }
            "inbox" => {
                let inbox: Inbox = client
                    .drive_call("drive_inbox", args(()).map_err(js_error)?)
                    .await
                    .map_err(js_error)?;
                serde_json::json!({"notifications":inbox.notifications,"items":inbox.items.into_iter().map(|item|serde_json::json!({"id":item.notice.id.to_string(),"object":item.notice.object.to_string(),"sender":item.notice.sender.to_text(),"kind":item.notice.kind,"at":item.notice.at.to_string(),"atMs":item.notice.at/1_000_000,"read":item.notice.read,"name":item.name,"message":item.message,"canRespond":item.can_respond})).collect::<Vec<_>>()})
            }
            "readNotice" => {
                let _: () = client
                    .drive_call(
                        "drive_read_notice",
                        args((number("id")?,)).map_err(js_error)?,
                    )
                    .await
                    .map_err(js_error)?;
                serde_json::Value::Null
            }
            "notifications" => {
                let _: () = client
                    .drive_call(
                        "drive_notifications",
                        args((p["enabled"].as_bool().unwrap_or(true),)).map_err(js_error)?,
                    )
                    .await
                    .map_err(js_error)?;
                serde_json::Value::Null
            }
            "contacts" => {
                let contacts: Vec<Contact> = client
                    .drive_call("drive_contacts", args(()).map_err(js_error)?)
                    .await
                    .map_err(js_error)?;
                serde_json::json!(contacts.into_iter().map(|c|serde_json::json!({"principal":c.principal.to_text(),"label":c.label})).collect::<Vec<_>>())
            }
            "contact" => {
                let principal: candid::Principal =
                    string("principal")?.parse().map_err(js_error)?;
                let _: () = client
                    .drive_call(
                        "drive_contact",
                        args((principal, string("label")?)).map_err(js_error)?,
                    )
                    .await
                    .map_err(js_error)?;
                serde_json::Value::Null
            }
            "notify" => {
                let recipient: candid::Principal =
                    string("recipient")?.parse().map_err(js_error)?;
                let _: () = client
                    .drive_call(
                        "drive_notify",
                        args((object("id")?, recipient)).map_err(js_error)?,
                    )
                    .await
                    .map_err(js_error)?;
                serde_json::Value::Null
            }
            "folderColor" => {
                let color = p["color"].as_str().map(str::to_owned);
                let _: () = client
                    .drive_call(
                        "drive_folder_color",
                        args((object("id")?, color)).map_err(js_error)?,
                    )
                    .await
                    .map_err(js_error)?;
                serde_json::Value::Null
            }
            "activity" => {
                let page: ActivityPage = client
                    .drive_call(
                        "drive_activity",
                        args((
                            p["id"].as_str().map(object_id).transpose()?,
                            p["before"].as_str().map(sequence).transpose()?,
                            50u32,
                        ))
                        .map_err(js_error)?,
                    )
                    .await
                    .map_err(js_error)?;
                serde_json::json!({"items": page.items.into_iter().map(|v| serde_json::json!({"id":v.event.id.to_string(),"object":v.event.object.to_string(),"name":v.name,"trashed":v.trashed,"atMs":v.event.at/1_000_000,"kind":v.event.kind,"actor":match v.event.actor {Participant::User(p)=>p.to_text(),Participant::Agent(id)=>format!("Agent {id}")}})).collect::<Vec<_>>(),"before":page.before.map(|v|v.to_string())})
            }
            "storage" => {
                let v: StorageOverview = client
                    .drive_call("drive_storage", args(()).map_err(js_error)?)
                    .await
                    .map_err(js_error)?;
                serde_json::json!({"currentBytes":v.current_bytes,"historyBytes":v.history_bytes,"trashBytes":v.trash_bytes,"pendingBytes":v.pending_bytes,"usedBytes":v.used_bytes,"quotaBytes":v.quota_bytes,
                    "files":v.files.into_iter().map(|f|serde_json::json!({"id":f.object.to_string(),"name":f.name,"bytes":f.current_bytes,"historyBytes":f.history_bytes,"versions":f.revision_count,"trashed":f.trashed})).collect::<Vec<_>>()})
            }
            "revisionLabel" => {
                let request = RevisionLabelRequest {
                    object: object("id")?,
                    revision: number("revision")?,
                    expected_version: number("version")?,
                    label: string("label")?,
                    keep: p["keep"].as_bool().unwrap_or(false),
                    request_id: string("requestId")?,
                };
                let _: RevisionProperties = client
                    .drive_call("drive_label_revision", args((request,)).map_err(js_error)?)
                    .await
                    .map_err(js_error)?;
                serde_json::Value::Null
            }
            "revisionDelete" => {
                let request = RevisionDeleteRequest {
                    object: object("id")?,
                    revision: number("revision")?,
                    expected_object_version: number("version")?,
                    expected_properties_version: number("propertiesVersion")?,
                    request_id: string("requestId")?,
                };
                let r: MutationReceipt = client
                    .drive_call("drive_delete_revision", args((request,)).map_err(js_error)?)
                    .await
                    .map_err(js_error)?;
                serde_json::json!({"id":r.object.to_string(),"version":r.version.to_string()})
            }
            "history" => {
                let revisions: Vec<RevisionInfo> = client
                    .drive_call("drive_versions", args((object("id")?,)).map_err(js_error)?)
                    .await
                    .map_err(js_error)?;
                serde_json::json!(revisions.into_iter().map(|info| {let r=info.revision;serde_json::json!({"id":r.id.to_string(), "bytes":r.bytes, "updatedMs":r.created_at/1_000_000,"author":r.author.to_text(),"label":info.properties.label,"keep":info.properties.keep,"automatic":info.properties.automatic,"propertiesVersion":info.properties.version.to_string()})}).collect::<Vec<_>>())
            }
            "grants" => {
                let grants: Vec<AccessGrant> = client
                    .drive_call("drive_grants", args((object("id")?,)).map_err(js_error)?)
                    .await
                    .map_err(js_error)?;
                serde_json::json!(grants.into_iter().filter(|g| matches!(g.subject,Participant::User(_))).map(|g| serde_json::json!({"id":g.id.to_string(), "root":g.root.to_string(), "version":g.version.to_string(), "subject":match g.subject {Participant::User(p)=>p.to_text(), _=>String::new()}, "capabilities":g.capabilities,"active":g.active,"expiresAt":g.expires_at.map(|v|v.to_string())})).collect::<Vec<_>>())
            }
            "grant" => {
                let request = GrantRequest {
                    request_id: string("requestId")?,
                    root: object("root")?,
                    id: p["id"].as_str().map(sequence).transpose()?,
                    expected_version: p["version"].as_str().map(sequence).transpose()?,
                    subject: Participant::User(string("subject")?.parse().map_err(js_error)?),
                    capabilities: serde_json::from_value(p["capabilities"].clone())
                        .map_err(js_error)?,
                    active: p["active"].as_bool().unwrap_or(true),
                    expires_at: p["expiresAt"].as_str().map(sequence).transpose()?,
                };
                let _: AccessGrant = client
                    .drive_call("drive_set_grant", args((request,)).map_err(js_error)?)
                    .await
                    .map_err(js_error)?;
                serde_json::Value::Null
            }
            "pending" => {
                let stages: Vec<DriveUpload> = client
                    .drive_call("drive_pending_uploads", args(()).map_err(js_error)?)
                    .await
                    .map_err(js_error)?;
                serde_json::json!(
                    stages
                        .into_iter()
                        .map(|s| s.id.to_string())
                        .collect::<Vec<_>>()
                )
            }
            "abort" => {
                client
                    .drive_call::<()>(
                        "drive_abort_upload",
                        args((number("id")?,)).map_err(js_error)?,
                    )
                    .await
                    .map_err(js_error)?;
                serde_json::Value::Null
            }
            _ => return Err(js_error("Unknown drive operation")),
        };
        if !session.is_authenticated() {
            return Err(js_error("Session changed during file loading"));
        }
        json(value)
    }
    pub async fn upload(
        &self,
        payload: String,
        bytes: Vec<u8>,
        progress: js_sys::Function,
    ) -> Result<JsValue, JsValue> {
        let session = self
            .session
            .borrow()
            .clone()
            .ok_or_else(|| js_error("Sign in first"))?;
        let p: serde_json::Value = serde_json::from_str(&payload).map_err(js_error)?;
        let s = |key: &str| {
            p[key]
                .as_str()
                .ok_or_else(|| js_error("Missing upload field"))
        };
        let target = if let Some(file) = p["file"].as_str() {
            UploadTarget::Replace {
                file: object_id(file)?,
                expected_revision: sequence(s("revision")?)?,
            }
        } else {
            UploadTarget::New {
                parent: object_id(s("parent")?)?,
                name: s("name")?.into(),
                format: FileFormat {
                    media_type: s("mediaType")?.into(),
                    craft: serde_json::from_value(p["craft"].clone()).map_err(js_error)?,
                },
            }
        };
        let result = BrowserClient::new(&session)
            .map_err(js_error)?
            .drive_save_controlled(target, &bytes, s("requestId")?.into(), |done, total| {
                session.is_authenticated()
                    && progress
                        .call2(
                            &JsValue::NULL,
                            &JsValue::from_f64(done as f64),
                            &JsValue::from_f64(total as f64),
                        )
                        .is_ok_and(|value| value.as_bool() != Some(false))
            })
            .await
            .map_err(js_error)?;
        if !session.is_authenticated() {
            return Err(js_error("Session changed during upload"));
        }
        browser::notify_files_changed();
        json(serde_json::json!({"id":result.file.to_string()}))
    }
    pub async fn download(&self, file: String, revision: String) -> Result<Vec<u8>, JsValue> {
        let session = self
            .session
            .borrow()
            .clone()
            .ok_or_else(|| js_error("Sign in first"))?;
        let bytes = BrowserClient::new(&session)
            .map_err(js_error)?
            .drive_download(object_id(&file)?, sequence(&revision)?)
            .await
            .map_err(js_error)?;
        if !session.is_authenticated() {
            return Err(js_error("Session changed during download"));
        }
        Ok(bytes)
    }
    pub async fn files(&self, space: String, after: Option<String>) -> Result<JsValue, JsValue> {
        let session = self
            .session
            .borrow()
            .clone()
            .ok_or_else(|| js_error("Sign in first"))?;
        let page = BrowserClient::new(&session)
            .map_err(js_error)?
            .files(
                object_id(&space)?,
                after.map(|s| object_id(&s)).transpose()?,
                20,
            )
            .await
            .map_err(js_error)?;
        if !session.is_authenticated() {
            return Err(js_error("Session changed during file loading"));
        }
        let files: Vec<_> = page
            .files
            .into_iter()
            .map(|file| {
                serde_json::json!({
                    "id":file.id.to_string(), "spaceId":file.space_id.to_string(), "name":file.name,
                    "formatId":file.kind.format_id, "formatVersion":file.kind.format_version,
                    "bytes":file.latest.bytes, "updatedMs":file.latest.created_at / 1_000_000,
                    "revision":file.latest.id.to_string(), "versions":file.version_count
                })
            })
            .collect();
        json(
            serde_json::json!({"files":files,"nextCursor":page.next_cursor.map(|id| id.to_string())}),
        )
    }
}
impl Default for CloudPortal {
    fn default() -> Self {
        Self::new()
    }
}

fn public_item_json(item: PublicItem) -> serde_json::Value {
    let format = match item.kind.clone() {
        ObjectKind::File(f) => Some(f),
        _ => None,
    };
    serde_json::json!({"id":item.id.to_string(),"name":item.name,"folder":item.kind==ObjectKind::Folder,"format":format,"updatedMs":item.updated_at/1_000_000,"bytes":item.latest.as_ref().map(|r|r.bytes),"revision":item.latest.map(|r|r.id.to_string())})
}

fn entry_json(entry: DriveEntry) -> serde_json::Value {
    let object = entry.object;
    let folder = object.kind == ObjectKind::Folder;
    let shortcut = if let ObjectKind::Shortcut(id) = object.kind {
        Some(id.to_string())
    } else {
        None
    };
    let format = match object.kind {
        ObjectKind::File(format) => Some(format),
        _ => None,
    };
    serde_json::json!({"id":object.id.to_string(),"spaceId":object.space.to_string(),"owner":object.owner.to_text(),
        "parent":object.parent.map(|id|id.to_string()),"name":object.name,"folder":folder,"shortcut":shortcut,"format":format,
        "version":object.version.to_string(),"createdMs":object.created_at/1_000_000,"updatedMs":object.updated_at/1_000_000,
        "bytes":entry.latest.as_ref().map(|r|r.bytes),"revision":entry.latest.map(|r|r.id.to_string()),
        "trashed":object.trashed_at.is_some(),"capabilities":entry.capabilities,"sharing":entry.sharing,
        "spaceAccess":entry.space_access.map(|s|serde_json::json!({"teamName":s.team_name,"manage":s.manage}))})
}

fn team_json(space: TeamSpace) -> serde_json::Value {
    serde_json::json!({"id":space.id.to_string(),"name":space.name,"version":space.version.to_string(),"externalSharing":space.external_sharing})
}
fn space_view_json(view: SpaceView) -> serde_json::Value {
    let mut value = team_json(view.space);
    value["role"] = serde_json::json!(view.role);
    value["usedBytes"] = view.used_bytes.into();
    value["quotaBytes"] = view.quota_bytes.into();
    value
}

fn sort_field(value: &str) -> Result<SortField, JsValue> {
    match value {
        "name" => Ok(SortField::Name),
        "modified" => Ok(SortField::Modified),
        "size" => Ok(SortField::Size),
        "opened" => Ok(SortField::Opened),
        _ => Err(js_error("Unknown sort field")),
    }
}
fn preferences_json(p: BrowsePreferences) -> serde_json::Value {
    serde_json::json!({"view":match p.view {ViewMode::List=>"list",ViewMode::Grid=>"grid"},"sort":match p.sort {SortField::Name=>"name",SortField::Modified=>"modified",SortField::Size=>"size",SortField::Opened=>"opened"},"descending":p.descending})
}

fn browse_json(view: DriveBrowse) -> Result<serde_json::Value, JsValue> {
    Ok(
        serde_json::json!({"catalog":{"entries":view.catalog.entries.into_iter().map(entry_json).collect::<Vec<_>>(),"completeFolders":view.catalog.complete_folders.into_iter().map(|id|id.to_string()).collect::<Vec<_>>()},"folder":entry_json(view.folder), "breadcrumbs":view.breadcrumbs.into_iter().map(entry_json).collect::<Vec<_>>(),
        "entries":view.page.entries.into_iter().map(entry_json).collect::<Vec<_>>(), "cursor":view.page.next.map(|c|serde_json::to_string(&c)).transpose().map_err(js_error)?}),
    )
}

fn query_json(page: BrowsePage) -> Result<serde_json::Value, JsValue> {
    Ok(
        serde_json::json!({"entries":page.items.into_iter().map(|item| {
                    let mut value=entry_json(item.entry); value["starred"]=item.starred.into();value["color"]=item.color.into();
                    value["openedMs"]=item.opened_at.map(|time|time/1_000_000).into();value
                }).collect::<Vec<_>>(), "cursor":page.next.map(|c|serde_json::to_string(&c)).transpose().map_err(js_error)?,
                "folder":page.folder.map(entry_json),"breadcrumbs":page.breadcrumbs.into_iter().map(entry_json).collect::<Vec<_>>(),
                "preferences":preferences_json(page.preferences)}),
    )
}

fn suggestion_json(s: SuggestedItem) -> serde_json::Value {
    let mut value = entry_json(s.item.entry);
    value["starred"] = s.item.starred.into();
    value["color"] = s.item.color.into();
    value["openedMs"] = s.item.opened_at.map(|v| v / 1_000_000).into();
    value["reason"] = match s.reason {
        SuggestionReason::UpdatedSinceOpened => "updated",
        SuggestionReason::RecentlyOpened => "opened",
        SuggestionReason::Starred => "starred",
        SuggestionReason::RecentlyCreated => "created",
    }
    .into();
    value["reasonMs"] = (s.reason_at / 1_000_000).into();
    value
}

fn email_invitation_json(v: EmailInvitation) -> serde_json::Value {
    serde_json::json!({"id":v.id.to_string(),"object":v.object.to_string(),"email":v.email,"role":v.role,"expiresAt":v.expires_at.to_string(),"active":v.active,"accepted":v.accepted,"delivery":v.delivery})
}
