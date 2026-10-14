//! Small browser facade: all identity and file operations use the same Rust SDK as Craft.
#![forbid(unsafe_code)]
#![deny(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
use futures::channel::oneshot;
use sovereign_cloud::browser::{self, BrowserClient, BrowserSession};
use sovereign_cloud::{args, drive::*, protocol::Revision};
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
fn id(value: &str) -> Result<u64, JsValue> {
    value
        .parse()
        .map_err(|_| js_error("Invalid file reference"))
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
            let home: DriveHome = BrowserClient::new(&session)
                .map_err(js_error)?
                .drive_call("drive_home", args(()).map_err(js_error)?)
                .await
                .map_err(js_error)?;
            let account = home.account;
            if !session.is_authenticated() {
                return Err(js_error("Session changed during account loading"));
            }
            serde_json::json!({"authenticated":true, "principal":account.account.owner.to_text(), "accountId":account.account.id.to_string(), "spaceId":account.account.personal_space.to_string(), "usedBytes":account.used_bytes, "quotaBytes":account.quota_bytes, "home":browse_json(home.view)?})
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
        let number = |key: &str| -> Result<u64, JsValue> { id(&string(key)?) };
        let value = match operation.as_str() {
            "initialize" | "get" | "folder" => {
                let (method, input) = match operation.as_str() {
                    "initialize" => ("drive_initialize", args(())),
                    "get" => ("drive_get", args((number("id")?,))),
                    _ => (
                        "drive_create_folder",
                        args((CreateFolder {
                            parent: number("parent")?,
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
                        args((number("parent")?, cursor, 50u32)).map_err(js_error)?,
                    )
                    .await
                    .map_err(js_error)?;
                browse_json(view)?
            }
            "list" => {
                let parent = p["parent"].as_str().map(id).transpose()?;
                let cursor: Option<DriveCursor> = p["cursor"]
                    .as_str()
                    .map(serde_json::from_str)
                    .transpose()
                    .map_err(js_error)?;
                let page: DrivePage = client
                    .drive_call(
                        "drive_list",
                        args((parent, cursor, 50u32)).map_err(js_error)?,
                    )
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
            "organize" => {
                let change = match string("change")?.as_str() {
                    "rename" => OrganizationChange::Rename(string("name")?),
                    "move" => OrganizationChange::Move(number("parent")?),
                    "trash" => OrganizationChange::Trash,
                    "restore" => OrganizationChange::Restore,
                    _ => return Err(js_error("Unknown operation")),
                };
                let request = OrganizeRequest {
                    object: number("id")?,
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
            "history" => {
                let revisions: Vec<Revision> = client
                    .drive_call("drive_history", args((number("id")?,)).map_err(js_error)?)
                    .await
                    .map_err(js_error)?;
                serde_json::json!(revisions.into_iter().map(|r| serde_json::json!({"id":r.id.to_string(), "bytes":r.bytes, "updatedMs":r.created_at/1_000_000,"author":r.author.to_text()})).collect::<Vec<_>>())
            }
            "grants" => {
                let grants: Vec<AccessGrant> = client
                    .drive_call("drive_grants", args((number("id")?,)).map_err(js_error)?)
                    .await
                    .map_err(js_error)?;
                serde_json::json!(grants.into_iter().map(|g| serde_json::json!({"id":g.id.to_string(), "root":g.root.to_string(), "version":g.version.to_string(), "subject":match g.subject {Participant::User(p)=>p.to_text(), _=>String::new()}, "capabilities":g.capabilities,"active":g.active})).collect::<Vec<_>>())
            }
            "grant" => {
                let request = GrantRequest {
                    request_id: string("requestId")?,
                    root: number("root")?,
                    id: p["id"].as_str().map(id).transpose()?,
                    expected_version: p["version"].as_str().map(id).transpose()?,
                    subject: Participant::User(string("subject")?.parse().map_err(js_error)?),
                    capabilities: serde_json::from_value(p["capabilities"].clone())
                        .map_err(js_error)?,
                    active: p["active"].as_bool().unwrap_or(true),
                    expires_at: None,
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
                file: id(file)?,
                expected_revision: id(s("revision")?)?,
            }
        } else {
            UploadTarget::New {
                parent: id(s("parent")?)?,
                name: s("name")?.into(),
                format: FileFormat {
                    media_type: s("mediaType")?.into(),
                    craft: serde_json::from_value(p["craft"].clone()).map_err(js_error)?,
                },
            }
        };
        let result = BrowserClient::new(&session)
            .map_err(js_error)?
            .drive_save(target, &bytes, s("requestId")?.into(), |done, total| {
                let _ = progress.call2(
                    &JsValue::NULL,
                    &JsValue::from_f64(done as f64),
                    &JsValue::from_f64(total as f64),
                );
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
            .drive_download(id(&file)?, id(&revision)?)
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
            .files(id(&space)?, after.map(|s| id(&s)).transpose()?, 20)
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

fn entry_json(entry: DriveEntry) -> serde_json::Value {
    let object = entry.object;
    let format = match object.kind {
        ObjectKind::File(format) => Some(format),
        _ => None,
    };
    serde_json::json!({"id":object.id.to_string(),"spaceId":object.space.to_string(),"owner":object.owner.to_text(),
        "parent":object.parent.map(|id|id.to_string()),"name":object.name,"folder":format.is_none(),"format":format,
        "version":object.version.to_string(),"updatedMs":entry.latest.as_ref().map_or(object.updated_at, |r|r.created_at)/1_000_000,
        "bytes":entry.latest.as_ref().map(|r|r.bytes),"revision":entry.latest.map(|r|r.id.to_string()),
        "trashed":object.trashed_at.is_some(),"capabilities":entry.capabilities,"sharing":entry.sharing})
}

fn browse_json(view: DriveBrowse) -> Result<serde_json::Value, JsValue> {
    Ok(
        serde_json::json!({"catalog":{"entries":view.catalog.entries.into_iter().map(entry_json).collect::<Vec<_>>(),"completeFolders":view.catalog.complete_folders.into_iter().map(|id|id.to_string()).collect::<Vec<_>>()},"folder":entry_json(view.folder), "breadcrumbs":view.breadcrumbs.into_iter().map(entry_json).collect::<Vec<_>>(),
        "entries":view.page.entries.into_iter().map(entry_json).collect::<Vec<_>>(), "cursor":view.page.next.map(|c|serde_json::to_string(&c)).transpose().map_err(js_error)?}),
    )
}
