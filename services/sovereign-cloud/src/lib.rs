//! Principal-scoped accounts and a UUID object catalog in stable storage.
#![forbid(unsafe_code)]
#![deny(clippy::unwrap_used, clippy::expect_used, clippy::panic, clippy::unimplemented, clippy::todo, clippy::unreachable)]
use sovereign_cloud::ObjectId;
mod mail;
mod permissions;
mod store;
use ic_cdk::api::{msg_caller, time};
use ic_stable_structures::DefaultMemoryImpl;
use permissions::Actor;
use sovereign_cloud::drive::*;
use sovereign_cloud::email::*;
use sovereign_cloud::people::*;
use sovereign_cloud::protocol::*;
use std::cell::RefCell;

thread_local! { static STORE: RefCell<store::Store<DefaultMemoryImpl>> = RefCell::new(store::Store::new(DefaultMemoryImpl::default())); }
#[ic_cdk::init]
fn init() {
    STORE.with(|_| {});
}
#[ic_cdk::post_upgrade]
fn post_upgrade() {
    STORE.with(|_| {});
}

#[ic_cdk::query]
fn get_api_doc() -> String {
    include_str!("../api.md").to_string()
}

// Fresh entropy after install/upgrade; allocation and authorization then execute
// together in one message. No caller authority is obtained from the seed callback.
async fn prepare_object_ids(caller: candid::Principal) -> CloudResult<()> {
    if caller == candid::Principal::anonymous() {
        return Err("Sign in first".into());
    }
    if !STORE.with(|s| s.borrow().ids_ready()) {
        let seed = ic_cdk_management_canister::raw_rand().await.map_err(|_| "Could not initialize file identities; retry")?;
        let seed: [u8; 32] = seed.try_into().map_err(|_| "Invalid identity entropy response")?;
        STORE.with(|s| s.borrow_mut().seed_ids(seed));
    }
    Ok(())
}

fn refresh_mail_config() -> CloudResult<()> {
    STORE.with(|s| s.borrow_mut().email_configure(mail::configuration()?))
}
#[ic_cdk::update]
fn email_profile() -> CloudResult<EmailProfile> {
    refresh_mail_config()?;
    STORE.with(|s| s.borrow().email_profile(msg_caller(), time()))
}
async fn deliver_email(caller: candid::Principal, id: u64) -> CloudResult<()> {
    refresh_mail_config()?;
    let claim = STORE.with(|s| s.borrow_mut().email_claim_delivery(caller, id, time()))?;
    if let Some(claim) = claim {
        let key_id = format!("subnet-{}-{}", ic_cdk::api::canister_self(), claim.idempotency);
        let status = mail::send(&claim.key, &key_id, claim.payload).await;
        STORE.with(|s| s.borrow_mut().email_finish_delivery(id, claim.attempt, status))?;
    }
    Ok(())
}
#[ic_cdk::update]
async fn email_request(request: EmailRequest) -> CloudResult<EmailProfile> {
    refresh_mail_config()?;
    let caller = msg_caller();
    let (id, prepare) = STORE.with(|s| s.borrow_mut().email_prepare(caller, request, time()))?;
    if prepare {
        let random = ic_cdk_management_canister::raw_rand().await.map_err(|_| "Could not prepare verification. Please request another code later.")?;
        STORE.with(|s| s.borrow_mut().email_issue(caller, id, &random, time()))?;
        deliver_email(caller, id).await?;
    }
    STORE.with(|s| s.borrow().email_profile(caller, time()))
}
#[ic_cdk::update]
fn email_verify(id: u64, code: String) -> CloudResult<EmailVerified> {
    STORE.with(|s| s.borrow_mut().email_verify(msg_caller(), id, code, time()))
}
#[ic_cdk::update]
fn email_confirm_transfer(id: u64) -> CloudResult<EmailVerified> {
    STORE.with(|s| s.borrow_mut().email_confirm_transfer(msg_caller(), id, time()))
}
#[ic_cdk::update]
fn email_remove(email: String) -> CloudResult<()> {
    STORE.with(|s| s.borrow_mut().email_remove(msg_caller(), email))
}
#[ic_cdk::update]
async fn email_retry(id: u64) -> CloudResult<()> {
    deliver_email(msg_caller(), id).await
}
#[ic_cdk::update]
async fn email_share(request: EmailShare) -> CloudResult<EmailInvitation> {
    refresh_mail_config()?;
    let caller = msg_caller();
    let invite = STORE.with(|s| s.borrow_mut().email_share(caller, request, time()))?;
    // A failed notification does not undo a grant or invitation. The owner sees its status.
    let _ = deliver_email(caller, invite.id).await;
    STORE.with(|s| s.borrow().email_invitation_owner(caller, invite.id, time()))
}
#[ic_cdk::update]
fn email_invitations(object: ObjectId) -> CloudResult<Vec<EmailInvitation>> {
    STORE.with(|s| s.borrow().email_invitations(msg_caller(), object, time()))
}
#[ic_cdk::update]
fn email_revoke_invitation(id: u64) -> CloudResult<()> {
    STORE.with(|s| s.borrow_mut().email_revoke_invitation(msg_caller(), id, time()))
}
#[ic_cdk::update]
fn email_accept(id: u64) -> CloudResult<ObjectId> {
    STORE.with(|s| s.borrow_mut().email_accept(msg_caller(), id, time()))
}

#[ic_cdk::update]
async fn drive_home() -> DriveResult<DriveHome> {
    let caller = msg_caller();
    prepare_object_ids(caller).await.map_err(|_| DriveError::Internal)?;
    let actor = Actor::user(caller)?;
    let now = time();
    STORE.with(|s| {
        let mut store = s.borrow_mut();
        let root = store.drive_initialize(caller, now)?;
        let account = store.account(caller, now).map_err(|_| DriveError::Internal)?;
        Ok(DriveHome { account, view: store.drive_browse(&actor, root.object.id, None, 50, now)? })
    })
}
#[ic_cdk::update]
async fn drive_initialize() -> DriveResult<DriveEntry> {
    let caller = msg_caller();
    prepare_object_ids(caller).await.map_err(|_| DriveError::Internal)?;
    STORE.with(|s| s.borrow_mut().drive_initialize(caller, time()))
}
#[ic_cdk::update]
fn drive_get(id: ObjectId) -> DriveResult<DriveEntry> {
    let actor = Actor::user(msg_caller())?;
    STORE.with(|s| s.borrow().drive_get(&actor, id, time()))
}
#[ic_cdk::update]
fn drive_list(parent: Option<ObjectId>, cursor: Option<DriveCursor>, limit: u32) -> DriveResult<DrivePage> {
    let actor = Actor::user(msg_caller())?;
    STORE.with(|s| s.borrow().drive_list(&actor, parent, cursor, limit, time()))
}
#[ic_cdk::update]
fn drive_browse(parent: ObjectId, cursor: Option<DriveCursor>, limit: u32) -> DriveResult<DriveBrowse> {
    let actor = Actor::user(msg_caller())?;
    STORE.with(|s| s.borrow().drive_browse(&actor, parent, cursor, limit, time()))
}
#[ic_cdk::update]
fn drive_query(query: BrowseQuery, cursor: Option<BrowseCursor>, limit: u32) -> DriveResult<BrowsePage> {
    let actor = Actor::user(msg_caller())?;
    STORE.with(|s| s.borrow().drive_query(&actor, query, cursor, limit, time()))
}
#[ic_cdk::update]
fn drive_suggested_home() -> DriveResult<SuggestedHome> {
    let actor = Actor::user(msg_caller())?;
    STORE.with(|s| s.borrow().drive_suggested_home(&actor, time()))
}
#[ic_cdk::update]
fn drive_acknowledge_open(id: ObjectId, revision: Option<u64>) -> DriveResult<()> {
    let actor = Actor::user(msg_caller())?;
    STORE.with(|s| s.borrow_mut().drive_acknowledge_open(&actor, id, revision, time()))
}
#[ic_cdk::update]
fn drive_dismiss_suggestion(id: ObjectId) -> DriveResult<()> {
    let actor = Actor::user(msg_caller())?;
    STORE.with(|s| s.borrow_mut().drive_dismiss_suggestion(&actor, id, time()))
}
#[ic_cdk::update]
async fn drive_browser_home() -> DriveResult<BrowseHome> {
    let caller = msg_caller();
    prepare_object_ids(caller).await.map_err(|_| DriveError::Internal)?;
    let actor = Actor::user(caller)?;
    let now = time();
    STORE.with(|s| {
        let mut store = s.borrow_mut();
        let root = store.drive_initialize(caller, now)?;
        let preferences = store.drive_preferences(&actor)?;
        let query = BrowseQuery {
            scope: BrowseScope::Folder(root.object.id),
            text: String::new(),
            file_type: String::new(),
            owner: None,
            modified_after: None,
            modified_before: None,
            sort: preferences.sort,
            descending: preferences.descending,
        };
        let page = store.drive_query(&actor, query, None, 50, now)?;
        let account = store.account(caller, now).map_err(|_| DriveError::Internal)?;
        Ok(BrowseHome { account, page })
    })
}
#[ic_cdk::update]
fn drive_preferences() -> DriveResult<BrowsePreferences> {
    let actor = Actor::user(msg_caller())?;
    STORE.with(|s| s.borrow().drive_preferences(&actor))
}
#[ic_cdk::update]
fn drive_set_preferences(preferences: BrowsePreferences) -> DriveResult<()> {
    let actor = Actor::user(msg_caller())?;
    STORE.with(|s| s.borrow_mut().drive_set_preferences(&actor, preferences))
}
#[ic_cdk::update]
fn drive_star(id: ObjectId, starred: bool) -> DriveResult<()> {
    let actor = Actor::user(msg_caller())?;
    STORE.with(|s| s.borrow_mut().drive_star(&actor, id, starred, time()))
}
#[ic_cdk::update]
fn drive_open(id: ObjectId) -> DriveResult<DriveEntry> {
    let actor = Actor::user(msg_caller())?;
    STORE.with(|s| s.borrow_mut().drive_open(&actor, id, time()))
}
#[ic_cdk::update]
async fn drive_create_folder(request: CreateFolder) -> DriveResult<DriveEntry> {
    let caller = msg_caller();
    prepare_object_ids(caller).await.map_err(|_| DriveError::Internal)?;
    let actor = Actor::user(caller)?;
    STORE.with(|s| s.borrow_mut().drive_create_folder(&actor, request, time()))
}
#[ic_cdk::update]
async fn drive_create_file(request: InlineFile) -> DriveResult<DriveEntry> {
    let caller = msg_caller();
    prepare_object_ids(caller).await.map_err(|_| DriveError::Internal)?;
    let actor = Actor::user(caller)?;
    STORE.with(|s| s.borrow_mut().drive_write_inline(&actor, request, time()))
}
#[ic_cdk::update]
fn drive_set_grant(request: GrantRequest) -> DriveResult<AccessGrant> {
    let actor = Actor::user(msg_caller())?;
    STORE.with(|s| s.borrow_mut().drive_grant(&actor, request, time()))
}
#[ic_cdk::update]
fn drive_grants(object: ObjectId) -> DriveResult<Vec<AccessGrant>> {
    let actor = Actor::user(msg_caller())?;
    STORE.with(|s| s.borrow().drive_grants(&actor, object))
}
#[ic_cdk::update]
fn drive_move_preview(object: ObjectId, parent: ObjectId) -> DriveResult<MovePreview> {
    let actor = Actor::user(msg_caller())?;
    STORE.with(|s| s.borrow().drive_move_preview(&actor, object, parent, time()))
}
#[ic_cdk::update]
fn drive_restore_preview(object: ObjectId, parent: ObjectId, name: String) -> DriveResult<MovePreview> {
    let actor = Actor::user(msg_caller())?;
    STORE.with(|s| s.borrow().drive_restore_preview(&actor, object, parent, &name, time()))
}
#[ic_cdk::update]
fn drive_details(object: ObjectId) -> DriveResult<DriveDetails> {
    let actor = Actor::user(msg_caller())?;
    STORE.with(|s| s.borrow().drive_details(&actor, object, time()))
}
#[ic_cdk::update]
fn drive_organize(request: OrganizeRequest) -> DriveResult<MutationReceipt> {
    let actor = Actor::user(msg_caller())?;
    STORE.with(|s| s.borrow_mut().drive_organize(&actor, request, time()))
}
#[ic_cdk::query]
fn drive_current_chunk(file: ObjectId, revision: u64, index: u64) -> DriveResult<Vec<u8>> {
    let actor = Actor::user(msg_caller())?;
    STORE.with(|s| s.borrow().drive_current_chunk(&actor, file, revision, index, time()))
}

#[ic_cdk::update]
async fn drive_begin_upload(request: DriveUploadRequest) -> DriveResult<DriveUpload> {
    let caller = msg_caller();
    prepare_object_ids(caller).await.map_err(|_| DriveError::Internal)?;
    let actor = Actor::user(caller)?;
    STORE.with(|s| s.borrow_mut().drive_begin(&actor, request, time()))
}
#[ic_cdk::update]
fn drive_put_chunk(upload: u64, index: u64, bytes: Vec<u8>) -> DriveResult<()> {
    let actor = Actor::user(msg_caller())?;
    STORE.with(|s| s.borrow_mut().drive_put(&actor, upload, index, bytes, time()))
}
#[ic_cdk::update]
fn drive_commit_upload(upload: u64) -> DriveResult<DriveUpload> {
    let actor = Actor::user(msg_caller())?;
    STORE.with(|s| s.borrow_mut().drive_commit(&actor, upload, time()))
}
#[ic_cdk::update]
fn drive_abort_upload(upload: u64) -> DriveResult<()> {
    let actor = Actor::user(msg_caller())?;
    STORE.with(|s| s.borrow_mut().drive_abort(&actor, upload))
}
#[ic_cdk::update]
fn drive_pending_uploads() -> DriveResult<Vec<DriveUpload>> {
    let actor = Actor::user(msg_caller())?;
    STORE.with(|s| s.borrow().drive_pending(&actor))
}
#[ic_cdk::update]
async fn drive_create_shortcut(request: CreateShortcut) -> DriveResult<DriveEntry> {
    let caller = msg_caller();
    prepare_object_ids(caller).await.map_err(|_| DriveError::Internal)?;
    let actor = Actor::user(caller)?;
    STORE.with(|s| s.borrow_mut().drive_create_shortcut(&actor, request, time()))
}
#[ic_cdk::update]
fn drive_shortcut_target(object: ObjectId) -> DriveResult<DriveEntry> {
    let actor = Actor::user(msg_caller())?;
    STORE.with(|s| s.borrow().drive_shortcut_target(&actor, object, time()))
}
#[ic_cdk::update]
fn drive_folder_color(object: ObjectId, color: Option<String>) -> DriveResult<()> {
    let actor = Actor::user(msg_caller())?;
    STORE.with(|s| s.borrow_mut().drive_folder_color(&actor, object, color, time()))
}
#[ic_cdk::update]
fn drive_activity(object: Option<ObjectId>, before: Option<u64>, limit: u32) -> DriveResult<ActivityPage> {
    let actor = Actor::user(msg_caller())?;
    STORE.with(|s| s.borrow().drive_activity(&actor, object, before, limit, time()))
}
#[ic_cdk::update]
fn drive_storage() -> DriveResult<StorageOverview> {
    let actor = Actor::user(msg_caller())?;
    STORE.with(|s| s.borrow().drive_storage(&actor))
}
#[ic_cdk::update]
fn drive_versions(object: ObjectId) -> DriveResult<Vec<RevisionInfo>> {
    let actor = Actor::user(msg_caller())?;
    STORE.with(|s| s.borrow().drive_versions(&actor, object, time()))
}
#[ic_cdk::update]
fn drive_label_revision(request: RevisionLabelRequest) -> DriveResult<RevisionProperties> {
    let actor = Actor::user(msg_caller())?;
    STORE.with(|s| s.borrow_mut().drive_label_revision(&actor, request, time()))
}
#[ic_cdk::update]
fn drive_delete_revision(request: RevisionDeleteRequest) -> DriveResult<MutationReceipt> {
    let actor = Actor::user(msg_caller())?;
    STORE.with(|s| s.borrow_mut().drive_delete_revision(&actor, request, time()))
}
#[ic_cdk::update]
fn drive_history(file: ObjectId) -> DriveResult<Vec<Revision>> {
    let actor = Actor::user(msg_caller())?;
    STORE.with(|s| s.borrow().drive_history(&actor, file, time()))
}
#[ic_cdk::update]
fn drive_trash(cursor: Option<DriveCursor>, limit: u32) -> DriveResult<DrivePage> {
    let actor = Actor::user(msg_caller())?;
    STORE.with(|s| s.borrow().drive_trash(&actor, cursor, limit, time()))
}
#[ic_cdk::update]
fn drive_purge_trash(request: PurgeRequest) -> DriveResult<PurgeReceipt> {
    let actor = Actor::user(msg_caller())?;
    STORE.with(|s| s.borrow_mut().drive_purge_trash(&actor, request))
}
#[ic_cdk::query]
fn drive_revision_chunk(file: ObjectId, revision: u64, index: u64) -> DriveResult<Vec<u8>> {
    let actor = Actor::user(msg_caller())?;
    STORE.with(|s| s.borrow().drive_revision_chunk(&actor, file, revision, index, time()))
}

#[ic_cdk::update]
async fn get_or_create_account() -> CloudResult<AccountContext> {
    let caller = msg_caller();
    prepare_object_ids(caller).await?;
    STORE.with(|s| s.borrow_mut().account(caller, time()))
}
#[ic_cdk::update]
fn list_files(space: ObjectId, after: Option<ObjectId>, limit: u32) -> CloudResult<FilePage> {
    STORE.with(|s| s.borrow().files(msg_caller(), space, after, limit))
}
#[ic_cdk::update]
fn get_file(space: ObjectId, file: ObjectId) -> CloudResult<FileDetails> {
    STORE.with(|s| s.borrow().file(msg_caller(), space, file))
}

// Metadata is read using updates: manifests and ownership come from consensus.
// Chunk queries are checked against the manifest SHA-256 by the browser.
#[ic_cdk::update]
fn list_projects() -> CloudResult<Vec<Project>> {
    STORE.with(|s| s.borrow().list_kind(msg_caller(), &FileKind::photocraft()))
}
#[ic_cdk::update]
fn list_app_files(kind: FileKind) -> CloudResult<Vec<Project>> {
    STORE.with(|s| s.borrow().list_kind(msg_caller(), &kind))
}
#[ic_cdk::update]
async fn create_file(name: String, kind: FileKind) -> CloudResult<Project> {
    let caller = msg_caller();
    prepare_object_ids(caller).await?;
    STORE.with(|s| s.borrow_mut().create_typed(caller, name, kind, time()))
}
#[ic_cdk::update]
async fn create_project(name: String) -> CloudResult<Project> {
    let caller = msg_caller();
    prepare_object_ids(caller).await?;
    STORE.with(|s| s.borrow_mut().create(caller, name, time()))
}
#[ic_cdk::update]
fn begin_upload(request: BeginUpload) -> CloudResult<Upload> {
    STORE.with(|s| s.borrow_mut().begin(msg_caller(), request, time()))
}
#[ic_cdk::update]
fn put_chunk(upload: u64, index: u64, bytes: Vec<u8>) -> CloudResult<()> {
    STORE.with(|s| s.borrow_mut().put(msg_caller(), upload, index, bytes))
}
#[ic_cdk::update]
fn commit_upload(upload: u64) -> CloudResult<Revision> {
    STORE.with(|s| s.borrow_mut().commit(msg_caller(), upload, time()))
}
#[ic_cdk::update]
fn get_revision(project: ObjectId, revision: u64) -> CloudResult<Revision> {
    STORE.with(|s| s.borrow().revision(msg_caller(), project, revision))
}
#[ic_cdk::query]
fn get_chunk(project: ObjectId, revision: u64, index: u64) -> CloudResult<Vec<u8>> {
    STORE.with(|s| s.borrow().chunk(msg_caller(), project, revision, index))
}
#[ic_cdk::update]
fn delete_revision_or_upload(id: u64) -> CloudResult<()> {
    STORE.with(|s| s.borrow_mut().remove_upload(msg_caller(), id, time()))
}
#[ic_cdk::update]
fn delete_project(id: ObjectId) -> CloudResult<()> {
    STORE.with(|s| s.borrow_mut().delete_project(msg_caller(), id))
}

#[ic_cdk::inspect_message]
fn inspect_message() {
    // An optimization only: every Store method independently enforces authorization.
    if msg_caller() != candid::Principal::anonymous() {
        ic_cdk::api::accept_message();
    }
}
ic_cdk::export_candid!();

pub fn candid_interface() -> String {
    __export_service()
}

#[test]
fn candid_interface_matches_canonical_contract() {
    assert_eq!(candid_interface().trim(), include_str!("../cloud.did").trim());
}

#[ic_cdk::update]
fn drive_request_access(object: ObjectId, message: String) -> DriveResult<()> {
    let actor = Actor::user(msg_caller())?;
    STORE.with(|s| s.borrow_mut().drive_request_access(&actor, object, message, time()))
}
#[ic_cdk::update]
fn drive_decide_access(decision: AccessDecision) -> DriveResult<()> {
    let actor = Actor::user(msg_caller())?;
    STORE.with(|s| s.borrow_mut().drive_decide_access(&actor, decision, time()))
}
#[ic_cdk::update]
fn drive_inbox() -> DriveResult<Inbox> {
    let actor = Actor::user(msg_caller())?;
    STORE.with(|s| s.borrow().drive_inbox(&actor, time()))
}
#[ic_cdk::update]
fn drive_read_notice(id: u64) -> DriveResult<()> {
    let actor = Actor::user(msg_caller())?;
    STORE.with(|s| s.borrow_mut().drive_read_notice(&actor, id))
}
#[ic_cdk::update]
fn drive_notifications(enabled: bool) -> DriveResult<()> {
    let actor = Actor::user(msg_caller())?;
    STORE.with(|s| s.borrow_mut().drive_notifications(&actor, enabled))
}
#[ic_cdk::update]
fn drive_contacts() -> DriveResult<Vec<Contact>> {
    let actor = Actor::user(msg_caller())?;
    STORE.with(|s| s.borrow().drive_contacts(&actor))
}
#[ic_cdk::update]
fn drive_contact(principal: candid::Principal, label: String) -> DriveResult<()> {
    let actor = Actor::user(msg_caller())?;
    STORE.with(|s| s.borrow_mut().drive_contact(&actor, principal, label))
}
#[ic_cdk::update]
fn drive_notify(object: ObjectId, recipient: candid::Principal) -> DriveResult<()> {
    let actor = Actor::user(msg_caller())?;
    STORE.with(|s| s.borrow_mut().drive_notify(&actor, object, recipient, time()))
}
#[ic_cdk::update]
fn drive_sharing_safety() -> DriveResult<SharingSafety> {
    let actor = Actor::user(msg_caller())?;
    STORE.with(|s| s.borrow().drive_sharing_safety(&actor, time()))
}
#[ic_cdk::update]
fn drive_block(other: candid::Principal, blocked: bool) -> DriveResult<()> {
    let actor = Actor::user(msg_caller())?;
    STORE.with(|s| s.borrow_mut().drive_block(&actor, other, blocked))
}
#[ic_cdk::update]
fn drive_hide_shared(object: ObjectId, hidden: bool) -> DriveResult<()> {
    let actor = Actor::user(msg_caller())?;
    STORE.with(|s| s.borrow_mut().drive_hide_shared(&actor, object, hidden, time()))
}
#[ic_cdk::update]
fn drive_report(object: ObjectId, reason: String) -> DriveResult<()> {
    let actor = Actor::user(msg_caller())?;
    STORE.with(|s| s.borrow_mut().drive_report(&actor, object, reason, time()))
}
#[ic_cdk::update]
fn admin_sharing_reports(after: Option<(candid::Principal, ObjectId)>) -> DriveResult<Vec<SharingReport>> {
    if !ic_cdk::api::is_controller(&msg_caller()) {
        return Err(DriveError::Inaccessible);
    }
    STORE.with(|s| s.borrow().sharing_reports(after))
}

#[cfg(test)]
pub(crate) fn test_id(n: u64) -> ObjectId {
    let mut bytes = [0; 16];
    bytes[8..].copy_from_slice(&n.to_be_bytes());
    ObjectId::from_random_bytes(bytes)
}
