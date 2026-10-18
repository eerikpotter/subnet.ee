//! Upgrade the current schema with real catalog, content and authority before
//! exercising the same recovery/detail calls exposed to browser and MCP clients.
use candid::{CandidType, Principal, decode_one, encode_args, utils::ArgumentEncoder};
use pocket_ic::{PocketIc, PocketIcBuilder};
use serde::de::DeserializeOwned;
use sovereign_cloud::drive::*;
fn call<T: CandidType + DeserializeOwned>(pic: &PocketIc, id: Principal, caller: Principal, method: &str, args: impl ArgumentEncoder) -> T {
    decode_one(&pic.update_call(id, caller, method, encode_args(args).unwrap()).unwrap()).unwrap()
}
#[test]
fn populated_upgrade_preserves_content_grants_and_receipts_then_restores_elsewhere() {
    let current = std::fs::read(std::env::var("PHOTOCRAFT_CLOUD_WASM").unwrap()).unwrap();
    let pic = PocketIcBuilder::new().with_application_subnet().build();
    let id = pic.create_canister();
    pic.add_cycles(id, 100_000_000_000_000);
    pic.install_canister(id, current.clone(), encode_args(()).unwrap(), None);
    let owner = Principal::self_authenticating([221; 32]);
    let reader = Principal::self_authenticating([222; 32]);
    let root: DriveResult<DriveEntry> = call(&pic, id, owner, "drive_initialize", ());
    let root = root.unwrap();
    let _: DriveResult<DriveEntry> = call(&pic, id, reader, "drive_initialize", ());
    let folder: DriveResult<DriveEntry> = call(
        &pic,
        id,
        owner,
        "drive_create_folder",
        (CreateFolder { parent: root.object.id, name: "Private".into(), request_id: "recovery-folder-001".into() },),
    );
    let folder = folder.unwrap();
    let file: DriveResult<DriveEntry> = call(
        &pic,
        id,
        owner,
        "drive_create_file",
        (InlineFile {
            parent: folder.object.id,
            name: "Preserved.txt".into(),
            format: FileFormat { media_type: "text/plain".into(), craft: None },
            bytes: b"preserve these bytes".to_vec(),
            request_id: "recovery-file-001".into(),
        },),
    );
    let file = file.unwrap();
    let grant: DriveResult<AccessGrant> = call(
        &pic,
        id,
        owner,
        "drive_set_grant",
        (GrantRequest {
            root: file.object.id,
            id: None,
            expected_version: None,
            subject: Participant::User(reader),
            capabilities: Capabilities::READ,
            expires_at: None,
            active: true,
            request_id: "recovery-grant-001".into(),
        },),
    );
    let grant = grant.unwrap();
    let star: DriveResult<()> = call(&pic, id, reader, "drive_star", (file.object.id, true));
    star.unwrap();
    let trash = OrganizeRequest {
        object: file.object.id,
        expected_version: file.object.version,
        request_id: "recovery-trash-001".into(),
        change: OrganizationChange::Trash,
    };
    let receipt: DriveResult<MutationReceipt> = call(&pic, id, owner, "drive_organize", (trash.clone(),));
    let receipt = receipt.unwrap();
    pic.upgrade_canister(id, current.clone(), encode_args(()).unwrap(), None).unwrap();
    let retry: DriveResult<MutationReceipt> = call(&pic, id, owner, "drive_organize", (trash,));
    assert_eq!(retry.unwrap(), receipt);
    let parent_trash: DriveResult<MutationReceipt> = call(
        &pic,
        id,
        owner,
        "drive_organize",
        (OrganizeRequest {
            object: folder.object.id,
            expected_version: folder.object.version,
            request_id: "recovery-parent-trash".into(),
            change: OrganizationChange::Trash,
        },),
    );
    parent_trash.unwrap();
    let preview: DriveResult<MovePreview> = call(&pic, id, owner, "drive_restore_preview", (file.object.id, root.object.id, "Recovered.txt".to_string()));
    let preview = preview.unwrap();
    let restore = OrganizeRequest {
        object: file.object.id,
        expected_version: receipt.version,
        request_id: "recovery-restore-001".into(),
        change: OrganizationChange::RestoreReviewed { parent: root.object.id, name: "Recovered.txt".into(), tree_version: preview.tree_version },
    };
    let restored: DriveResult<MutationReceipt> = call(&pic, id, owner, "drive_organize", (restore.clone(),));
    let restored = restored.unwrap();
    pic.upgrade_canister(id, current, encode_args(()).unwrap(), None).unwrap();
    let retry: DriveResult<MutationReceipt> = call(&pic, id, owner, "drive_organize", (restore,));
    assert_eq!(retry.unwrap(), restored);
    let details: DriveResult<DriveDetails> = call(&pic, id, reader, "drive_details", (file.object.id,));
    let details = details.unwrap();
    assert_eq!(details.entry.object.name, "Recovered.txt");
    assert_eq!(details.entry.object.parent, Some(root.object.id));
    assert!(details.locations.is_empty());
    assert_eq!(details.access.len(), 1);
    assert_eq!(details.revision_count, 1);
    let bytes: DriveResult<Vec<u8>> = call(&pic, id, reader, "drive_current_chunk", (file.object.id, file.latest.unwrap().id, 0u64));
    assert_eq!(bytes.unwrap(), b"preserve these bytes");
    let revoked: DriveResult<AccessGrant> = call(
        &pic,
        id,
        owner,
        "drive_set_grant",
        (GrantRequest {
            root: grant.root,
            id: Some(grant.id),
            expected_version: Some(grant.version),
            subject: grant.subject,
            capabilities: grant.capabilities,
            expires_at: None,
            active: false,
            request_id: "recovery-revoke-001".into(),
        },),
    );
    revoked.unwrap();
    let denied: DriveResult<DriveDetails> = call(&pic, id, reader, "drive_details", (file.object.id,));
    assert!(matches!(denied, Err(DriveError::Inaccessible)));
}
