//! Shared accounts and file catalog, preserving the PhotoCraft V1 storage contract.
#![forbid(unsafe_code)]
#![deny(clippy::unwrap_used, clippy::expect_used, clippy::panic, clippy::unimplemented, clippy::todo, clippy::unreachable)]
mod store;
use ic_cdk::api::{msg_caller, time};
use ic_stable_structures::DefaultMemoryImpl;
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
fn candid_interface_matches_deployed_contract() {
    assert_eq!(candid_interface().trim(), include_str!("../cloud.did").trim());
}
