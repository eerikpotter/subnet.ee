//! Shared accounts and file catalog, preserving the PhotoCraft V1 storage contract.
#![forbid(unsafe_code)]
#![deny(clippy::unwrap_used, clippy::expect_used, clippy::panic, clippy::unimplemented, clippy::todo, clippy::unreachable)]
mod permissions;
mod store;
use ic_cdk::api::{msg_caller, time};
use ic_stable_structures::DefaultMemoryImpl;
use permissions::Actor;
use sovereign_cloud::drive::*;
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

#[ic_cdk::update]
fn drive_home() -> DriveResult<DriveHome> {
    let caller = msg_caller();
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
fn drive_initialize() -> DriveResult<DriveEntry> {
    STORE.with(|s| s.borrow_mut().drive_initialize(msg_caller(), time()))
}
#[ic_cdk::update]
fn drive_get(id: u64) -> DriveResult<DriveEntry> {
    let actor = Actor::user(msg_caller())?;
    STORE.with(|s| s.borrow().drive_get(&actor, id, time()))
}
#[ic_cdk::update]
fn drive_list(parent: Option<u64>, cursor: Option<DriveCursor>, limit: u32) -> DriveResult<DrivePage> {
    let actor = Actor::user(msg_caller())?;
    STORE.with(|s| s.borrow().drive_list(&actor, parent, cursor, limit, time()))
}
#[ic_cdk::update]
fn drive_browse(parent: u64, cursor: Option<DriveCursor>, limit: u32) -> DriveResult<DriveBrowse> {
    let actor = Actor::user(msg_caller())?;
    STORE.with(|s| s.borrow().drive_browse(&actor, parent, cursor, limit, time()))
}
#[ic_cdk::update]
fn drive_create_folder(request: CreateFolder) -> DriveResult<DriveEntry> {
    let actor = Actor::user(msg_caller())?;
    STORE.with(|s| s.borrow_mut().drive_create_folder(&actor, request, time()))
}
#[ic_cdk::update]
fn drive_create_file(request: InlineFile) -> DriveResult<DriveEntry> {
    let actor = Actor::user(msg_caller())?;
    STORE.with(|s| s.borrow_mut().drive_write_inline(&actor, request, time()))
}
#[ic_cdk::update]
fn drive_set_grant(request: GrantRequest) -> DriveResult<AccessGrant> {
    let actor = Actor::user(msg_caller())?;
    STORE.with(|s| s.borrow_mut().drive_grant(&actor, request, time()))
}
#[ic_cdk::update]
fn drive_grants(object: u64) -> DriveResult<Vec<AccessGrant>> {
    let actor = Actor::user(msg_caller())?;
    STORE.with(|s| s.borrow().drive_grants(&actor, object))
}
#[ic_cdk::update]
fn drive_organize(request: OrganizeRequest) -> DriveResult<MutationReceipt> {
    let actor = Actor::user(msg_caller())?;
    STORE.with(|s| s.borrow_mut().drive_organize(&actor, request, time()))
}
#[ic_cdk::query]
fn drive_current_chunk(file: u64, revision: u64, index: u64) -> DriveResult<Vec<u8>> {
    let actor = Actor::user(msg_caller())?;
    STORE.with(|s| s.borrow().drive_current_chunk(&actor, file, revision, index, time()))
}

#[ic_cdk::update]
fn drive_begin_upload(request: DriveUploadRequest) -> DriveResult<DriveUpload> {
    let actor = Actor::user(msg_caller())?;
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
fn drive_history(file: u64) -> DriveResult<Vec<Revision>> {
    let actor = Actor::user(msg_caller())?;
    STORE.with(|s| s.borrow().drive_history(&actor, file, time()))
}
#[ic_cdk::update]
fn drive_trash(cursor: Option<DriveCursor>, limit: u32) -> DriveResult<DrivePage> {
    let actor = Actor::user(msg_caller())?;
    STORE.with(|s| s.borrow().drive_trash(&actor, cursor, limit, time()))
}
#[ic_cdk::query]
fn drive_revision_chunk(file: u64, revision: u64, index: u64) -> DriveResult<Vec<u8>> {
    let actor = Actor::user(msg_caller())?;
    STORE.with(|s| s.borrow().drive_revision_chunk(&actor, file, revision, index, time()))
}

#[ic_cdk::update]
fn get_or_create_account() -> CloudResult<AccountContext> {
    STORE.with(|s| s.borrow_mut().account(msg_caller(), time()))
}
#[ic_cdk::update]
fn list_files(space: u64, after: Option<u64>, limit: u32) -> CloudResult<FilePage> {
    STORE.with(|s| s.borrow().files(msg_caller(), space, after, limit))
}
#[ic_cdk::update]
fn get_file(space: u64, file: u64) -> CloudResult<FileDetails> {
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
fn create_file(name: String, kind: FileKind) -> CloudResult<Project> {
    STORE.with(|s| s.borrow_mut().create_typed(msg_caller(), name, kind, time()))
}
#[ic_cdk::update]
fn create_project(name: String) -> CloudResult<Project> {
    STORE.with(|s| s.borrow_mut().create(msg_caller(), name, time()))
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
fn get_revision(project: u64, revision: u64) -> CloudResult<Revision> {
    STORE.with(|s| s.borrow().revision(msg_caller(), project, revision))
}
#[ic_cdk::query]
fn get_chunk(project: u64, revision: u64, index: u64) -> CloudResult<Vec<u8>> {
    STORE.with(|s| s.borrow().chunk(msg_caller(), project, revision, index))
}
#[ic_cdk::update]
fn delete_revision_or_upload(id: u64) -> CloudResult<()> {
    STORE.with(|s| s.borrow_mut().remove_upload(msg_caller(), id))
}
#[ic_cdk::update]
fn delete_project(id: u64) -> CloudResult<()> {
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
