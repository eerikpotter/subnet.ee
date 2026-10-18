//! Disposable replica tests: no interactive or production user data is touched.
use candid::{CandidType, Principal, decode_one, encode_args, utils::ArgumentEncoder};
use pocket_ic::{PocketIc, PocketIcBuilder};
use serde::de::DeserializeOwned;
use sha2::{Digest, Sha256};
use sovereign_cloud::{drive::*, protocol::*};

fn call<T: CandidType + DeserializeOwned>(pic: &PocketIc, canister: Principal, owner: Principal, method: &str, args: impl ArgumentEncoder) -> T {
    decode_one(&pic.update_call(canister, owner, method, encode_args(args).unwrap()).unwrap()).unwrap()
}

#[test]
fn full_storage_trash_purge_is_atomic_and_retryable_across_upgrade() {
    let wasm = std::fs::read(std::env::var("PHOTOCRAFT_CLOUD_WASM").expect("Set PHOTOCRAFT_CLOUD_WASM")).unwrap();
    let pic = PocketIcBuilder::new().with_application_subnet().build();
    let canister = pic.create_canister();
    pic.add_cycles(canister, 100_000_000_000_000);
    pic.install_canister(canister, wasm.clone(), encode_args(()).unwrap(), None);
    let owner = Principal::self_authenticating([101; 32]);
    let other = Principal::self_authenticating([102; 32]);
    let root: DriveResult<DriveEntry> = call(&pic, canister, owner, "drive_initialize", ());
    let root = root.unwrap().object.id;
    let chunk = vec![17; CHUNK_BYTES];
    let mut hash = Sha256::new();
    for _ in 0..MAX_DOCUMENT_BYTES / CHUNK_BYTES as u64 {
        hash.update(&chunk);
    }
    let sha256 = hash.finalize().to_vec();
    let mut ids = Vec::new();
    // Fill the entire per-owner byte budget, including real multi-chunk content.
    for index in 0..USER_QUOTA_BYTES / MAX_DOCUMENT_BYTES {
        let p: CloudResult<Project> = call(&pic, canister, owner, "create_project", (format!("Budget file {index}"),));
        let p = p.unwrap();
        let upload: CloudResult<Upload> = call(
            &pic,
            canister,
            owner,
            "begin_upload",
            (BeginUpload {
                project_id: p.id,
                expected_revision: None,
                bytes: MAX_DOCUMENT_BYTES,
                sha256: sha256.clone(),
                request_id: format!("purge-budget-{index:016}"),
            },),
        );
        let upload = upload.unwrap();
        for n in 0..MAX_DOCUMENT_BYTES / CHUNK_BYTES as u64 {
            // This is a storage/accounting test, not a write-throughput benchmark.
            // Let the replica replenish its heap-delta budget between batches.
            if n % 16 == 0 {
                pic.advance_time(std::time::Duration::from_secs(60));
                pic.tick();
            }
            let result: CloudResult<()> = call(&pic, canister, owner, "put_chunk", (upload.id, n, chunk.clone()));
            result.unwrap();
        }
        let committed: CloudResult<Revision> = call(&pic, canister, owner, "commit_upload", (upload.id,));
        committed.unwrap();
        ids.push((p.id, upload.id));
    }
    let init: DriveResult<DriveEntry> = call(&pic, canister, owner, "drive_initialize", ());
    init.unwrap();
    let mut items = Vec::new();
    for (id, _) in &ids {
        let entry: DriveResult<DriveEntry> = call(&pic, canister, owner, "drive_get", (*id,));
        let entry = entry.unwrap();
        let receipt: DriveResult<MutationReceipt> = call(
            &pic,
            canister,
            owner,
            "drive_organize",
            (OrganizeRequest {
                object: *id,
                expected_version: entry.object.version,
                change: OrganizationChange::Trash,
                request_id: format!("purge-trash-{id:016}"),
            },),
        );
        items.push(receipt.unwrap());
    }
    // UUID identities and file bytes must survive the actual Wasm upgrade.
    // Management upload_chunk rejects while the canister's per-round heap-delta
    // debit is replenishing. Advance rounds after the large write/purge batches.
    for _ in 0..256 {
        pic.tick();
    }
    pic.upgrade_canister(canister, wasm.clone(), encode_args(()).unwrap(), None).unwrap();
    let request = PurgeRequest { items, request_id: "purge-max-storage-0001".into() };
    let denied: DriveResult<PurgeReceipt> = call(&pic, canister, other, "drive_purge_trash", (request.clone(),));
    assert_eq!(denied.unwrap_err(), DriveError::Inaccessible);
    // The existing inspect-message hook rejects anonymous ingress before Candid
    // dispatch; the update handler also constructs an authenticated Actor.
    assert!(pic.update_call(canister, Principal::anonymous(), "drive_purge_trash", encode_args((request.clone(),)).unwrap()).is_err());
    let result: DriveResult<PurgeReceipt> = call(&pic, canister, owner, "drive_purge_trash", (request.clone(),));
    let receipt = result.unwrap();
    assert_eq!(receipt.freed_bytes, USER_QUOTA_BYTES);
    assert_eq!(receipt.deleted_objects, 4);
    for _ in 0..256 {
        pic.tick();
    }
    pic.upgrade_canister(canister, wasm, encode_args(()).unwrap(), None).unwrap();
    let repeated: DriveResult<PurgeReceipt> = call(&pic, canister, owner, "drive_purge_trash", (request,));
    assert_eq!(repeated.unwrap(), receipt);
    let account: CloudResult<AccountContext> = call(&pic, canister, owner, "get_or_create_account", ());
    assert_eq!(account.unwrap().used_bytes, 0);
    let trash: DriveResult<DrivePage> = call(&pic, canister, owner, "drive_trash", (None::<DriveCursor>, 50u32));
    assert!(trash.unwrap().entries.is_empty());
    for (id, revision) in ids {
        let result = pic.query_call(canister, owner, "get_chunk", encode_args((id, revision, 0u64)).unwrap()).unwrap();
        assert!(decode_one::<CloudResult<Vec<u8>>>(&result).unwrap().is_err());
    }
    let created: DriveResult<DriveEntry> = call(
        &pic,
        canister,
        owner,
        "drive_create_file",
        (InlineFile {
            parent: root,
            name: "After purge.txt".into(),
            format: FileFormat { media_type: "text/plain".into(), craft: None },
            bytes: b"new".to_vec(),
            request_id: "purge-new-file-0001".into(),
        },),
    );
    assert!(created.is_ok());
}
