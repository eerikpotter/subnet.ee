//! Run against a separate PocketIC instance; never mutates the interactive demo canisters.
use candid::{CandidType, Principal, decode_one, encode_args, utils::ArgumentEncoder};
use pocket_ic::{PocketIc, PocketIcBuilder};
use serde::de::DeserializeOwned;
use sha2::{Digest, Sha256};
use sovereign_cloud::protocol::*;

fn call<T: CandidType + DeserializeOwned>(pic: &PocketIc, canister: Principal, user: Principal, method: &str, args: impl ArgumentEncoder) -> CloudResult<T> {
    let bytes = pic.update_call(canister, user, method, encode_args(args).unwrap()).map_err(|e| format!("{e:?}"))?;
    decode_one(&bytes).unwrap()
}

#[test]
fn save_restore_upgrade_and_access_control() {
    let wasm = std::fs::read(std::env::var("PHOTOCRAFT_CLOUD_WASM").expect("Set PHOTOCRAFT_CLOUD_WASM to the release canister Wasm")).unwrap();
    let mut builder = PocketIcBuilder::new().with_application_subnet();
    if let Ok(url) = std::env::var("POCKET_IC_SERVER_URL") {
        builder = builder.with_server_url(url.parse().unwrap());
    }
    let pic = builder.build();
    let canister = pic.create_canister();
    pic.add_cycles(canister, 20_000_000_000_000);
    pic.install_canister(canister, wasm.clone(), encode_args(()).unwrap(), None);
    let owner = Principal::self_authenticating([1; 32]);
    let other = Principal::self_authenticating([2; 32]);
    let project: Project = call(&pic, canister, owner, "create_project", ("Upgrade test",)).unwrap();
    let bytes: Vec<u8> = (0..(CHUNK_BYTES * 2 + 71)).map(|i| (i % 251) as u8).collect();
    let req = BeginUpload {
        project_id: project.id,
        expected_revision: None,
        bytes: bytes.len() as u64,
        sha256: Sha256::digest(&bytes).to_vec(),
        request_id: "pocketic-request-0001".into(),
    };
    let upload: Upload = call(&pic, canister, owner, "begin_upload", (req.clone(),)).unwrap();
    assert!(call::<Revision>(&pic, canister, owner, "commit_upload", (upload.id,)).is_err());
    for (i, chunk) in bytes.chunks(CHUNK_BYTES).enumerate() {
        call::<()>(&pic, canister, owner, "put_chunk", (upload.id, i as u64, chunk.to_vec())).unwrap();
    }
    // Upgrade with an upload in progress, then commit. Stable state must preserve both.
    pic.upgrade_canister(canister, wasm.clone(), encode_args(()).unwrap(), None).unwrap();
    let revision: Revision = call(&pic, canister, owner, "commit_upload", (upload.id,)).unwrap();
    assert_eq!(revision.sha256, req.sha256);
    assert_eq!(revision, call::<Revision>(&pic, canister, owner, "commit_upload", (upload.id,)).unwrap());
    assert!(call::<Upload>(&pic, canister, owner, "begin_upload", (req,)).is_err());
    pic.upgrade_canister(canister, wasm, encode_args(()).unwrap(), None).unwrap();
    let restored: Vec<Project> = call(&pic, canister, owner, "list_projects", ()).unwrap();
    assert_eq!(restored[0].revisions[0], revision);
    let mut actual = Vec::new();
    for i in 0..revision.chunk_count {
        let args = encode_args((project.id, revision.id, i)).unwrap();
        let result = pic.query_call(canister, owner, "get_chunk", args.clone()).unwrap();
        actual.extend(decode_one::<CloudResult<Vec<u8>>>(&result).unwrap().unwrap());
        let denied = pic.query_call(canister, other, "get_chunk", args).unwrap();
        assert!(decode_one::<CloudResult<Vec<u8>>>(&denied).unwrap().is_err());
    }
    assert_eq!(actual, bytes);
    assert!(call::<Vec<Project>>(&pic, canister, Principal::anonymous(), "list_projects", ()).is_err());
    assert!(call::<()>(&pic, canister, other, "delete_revision_or_upload", (upload.id,)).is_err());
    call::<()>(&pic, canister, owner, "delete_revision_or_upload", (upload.id,)).unwrap();
    call::<()>(&pic, canister, owner, "delete_project", (project.id,)).unwrap();
    assert!(call::<Vec<Project>>(&pic, canister, owner, "list_projects", ()).unwrap().is_empty());
}

#[test]
fn maximum_document_fits_one_commit_message_budget() {
    let wasm = std::fs::read(std::env::var("PHOTOCRAFT_CLOUD_WASM").expect("Set PHOTOCRAFT_CLOUD_WASM")).unwrap();
    let mut builder = PocketIcBuilder::new().with_application_subnet();
    if let Ok(url) = std::env::var("POCKET_IC_SERVER_URL") {
        builder = builder.with_server_url(url.parse().unwrap());
    }
    let pic = builder.build();
    let canister = pic.create_canister();
    pic.add_cycles(canister, 20_000_000_000_000);
    pic.install_canister(canister, wasm, encode_args(()).unwrap(), None);
    let owner = Principal::self_authenticating([3; 32]);
    let project: Project = call(&pic, canister, owner, "create_project", ("64 MiB budget test",)).unwrap();
    let chunk = vec![17; CHUNK_BYTES];
    let count = MAX_DOCUMENT_BYTES / CHUNK_BYTES as u64;
    let mut hash = Sha256::new();
    for _ in 0..count {
        hash.update(&chunk);
    }
    let request = BeginUpload {
        project_id: project.id,
        expected_revision: None,
        bytes: MAX_DOCUMENT_BYTES,
        sha256: hash.finalize().to_vec(),
        request_id: "maximum-document-test".into(),
    };
    let upload: Upload = call(&pic, canister, owner, "begin_upload", (request,)).unwrap();
    for index in 0..count {
        call::<()>(&pic, canister, owner, "put_chunk", (upload.id, index, chunk.clone())).unwrap();
    }
    let revision: Revision = call(&pic, canister, owner, "commit_upload", (upload.id,)).unwrap();
    assert_eq!(revision.bytes, MAX_DOCUMENT_BYTES);
    assert_eq!(revision.chunk_count, count);
}

#[test]
fn every_file_endpoint_denies_other_principals_before_and_after_upgrade() {
    let wasm = std::fs::read(std::env::var("PHOTOCRAFT_CLOUD_WASM").expect("Set PHOTOCRAFT_CLOUD_WASM")).unwrap();
    let mut builder = PocketIcBuilder::new().with_application_subnet();
    if let Ok(url) = std::env::var("POCKET_IC_SERVER_URL") {
        builder = builder.with_server_url(url.parse().unwrap());
    }
    let pic = builder.build();
    let canister = pic.create_canister();
    pic.add_cycles(canister, 20_000_000_000_000);
    pic.install_canister(canister, wasm.clone(), encode_args(()).unwrap(), None);
    let alice = Principal::self_authenticating([11; 32]);
    let bob = Principal::self_authenticating([12; 32]);
    let anonymous = Principal::anonymous();
    let alice_file: Project = call(&pic, canister, alice, "create_project", ("Alice private file",)).unwrap();
    let bob_file: Project = call(&pic, canister, bob, "create_project", ("Bob private file",)).unwrap();
    let bytes = b"private document bytes".to_vec();
    let request = BeginUpload {
        project_id: alice_file.id,
        expected_revision: None,
        bytes: bytes.len() as u64,
        sha256: Sha256::digest(&bytes).to_vec(),
        request_id: "authorization-matrix-01".into(),
    };
    let upload: Upload = call(&pic, canister, alice, "begin_upload", (request.clone(),)).unwrap();
    // Pending uploads are protected, including discard and direct chunk writes.
    for caller in [bob, anonymous] {
        assert!(call::<Upload>(&pic, canister, caller, "begin_upload", (request.clone(),)).is_err());
        assert!(call::<()>(&pic, canister, caller, "put_chunk", (upload.id, 0u64, bytes.clone())).is_err());
        assert!(call::<Revision>(&pic, canister, caller, "commit_upload", (upload.id,)).is_err());
        assert!(call::<()>(&pic, canister, caller, "delete_revision_or_upload", (upload.id,)).is_err());
        assert!(call::<()>(&pic, canister, caller, "delete_project", (alice_file.id,)).is_err());
    }
    call::<()>(&pic, canister, alice, "put_chunk", (upload.id, 0u64, bytes.clone())).unwrap();
    let revision: Revision = call(&pic, canister, alice, "commit_upload", (upload.id,)).unwrap();
    for upgraded in [false, true] {
        if upgraded {
            pic.upgrade_canister(canister, wasm.clone(), encode_args(()).unwrap(), None).unwrap();
        }
        let alice_files: Vec<Project> = call(&pic, canister, alice, "list_projects", ()).unwrap();
        let bob_files: Vec<Project> = call(&pic, canister, bob, "list_projects", ()).unwrap();
        assert_eq!(alice_files.iter().map(|p| p.id).collect::<Vec<_>>(), [alice_file.id]);
        assert_eq!(bob_files.iter().map(|p| p.id).collect::<Vec<_>>(), [bob_file.id]);
        assert!(call::<Vec<Project>>(&pic, canister, anonymous, "list_projects", ()).is_err());
        assert!(call::<Project>(&pic, canister, anonymous, "create_project", ("Anonymous",)).is_err());
        for caller in [bob, anonymous] {
            assert!(call::<Revision>(&pic, canister, caller, "get_revision", (alice_file.id, revision.id)).is_err());
            let denied = pic.query_call(canister, caller, "get_chunk", encode_args((alice_file.id, revision.id, 0u64)).unwrap()).unwrap();
            assert!(decode_one::<CloudResult<Vec<u8>>>(&denied).unwrap().is_err());
            assert!(call::<Upload>(&pic, canister, caller, "begin_upload", (request.clone(),)).is_err());
            assert!(call::<()>(&pic, canister, caller, "put_chunk", (upload.id, 0u64, bytes.clone())).is_err());
            // Even the idempotent commit shortcut must check ownership first.
            assert!(call::<Revision>(&pic, canister, caller, "commit_upload", (upload.id,)).is_err());
            assert!(call::<()>(&pic, canister, caller, "delete_revision_or_upload", (revision.id,)).is_err());
            assert!(call::<()>(&pic, canister, caller, "delete_project", (alice_file.id,)).is_err());
        }
        // A valid owned file ID cannot be paired with someone else's revision ID.
        assert!(call::<Revision>(&pic, canister, bob, "get_revision", (bob_file.id, revision.id)).is_err());
        let denied = pic.query_call(canister, bob, "get_chunk", encode_args((bob_file.id, revision.id, 0u64)).unwrap()).unwrap();
        assert!(decode_one::<CloudResult<Vec<u8>>>(&denied).unwrap().is_err());
        let actual = pic.query_call(canister, alice, "get_chunk", encode_args((alice_file.id, revision.id, 0u64)).unwrap()).unwrap();
        assert_eq!(decode_one::<CloudResult<Vec<u8>>>(&actual).unwrap().unwrap(), bytes);
        assert_eq!(call::<Revision>(&pic, canister, alice, "get_revision", (alice_file.id, revision.id)).unwrap(), revision);
    }
    call::<()>(&pic, canister, alice, "delete_revision_or_upload", (revision.id,)).unwrap();
    call::<()>(&pic, canister, alice, "delete_project", (alice_file.id,)).unwrap();
    assert!(call::<Vec<Project>>(&pic, canister, alice, "list_projects", ()).unwrap().is_empty());
    assert_eq!(call::<Vec<Project>>(&pic, canister, bob, "list_projects", ()).unwrap()[0].id, bob_file.id);
}
