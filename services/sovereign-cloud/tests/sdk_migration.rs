//! Exercise the extracted SDK against the deployed V1 contract and pre-extraction Wasm.
use candid::{Principal, encode_args};
use futures::executor::block_on;
use pocket_ic::{PocketIc, PocketIcBuilder};
use sovereign_cloud::{CloudResult, FileClient, Transport, UploadState, protocol::CHUNK_BYTES};
use std::cell::Cell;

struct PocketTransport<'a> {
    pic: &'a PocketIc,
    canister: Principal,
    caller: Principal,
    lose_commit_reply: Cell<bool>,
}
impl Transport for PocketTransport<'_> {
    async fn update(&self, method: &str, args: Vec<u8>) -> CloudResult<Vec<u8>> {
        let bytes = self.pic.update_call(self.canister, self.caller, method, args).map_err(|e| format!("{e:?}"))?;
        if matches!(method, "commit_upload" | "drive_commit_upload") && self.lose_commit_reply.replace(false) {
            return Err("Simulated dropped response after consensus".into());
        }
        Ok(bytes)
    }
    async fn query(&self, method: &str, args: Vec<u8>) -> CloudResult<Vec<u8>> {
        self.pic.query_call(self.canister, self.caller, method, args).map_err(|e| format!("{e:?}"))
    }
}

#[test]
fn legacy_save_survives_sdk_upgrade_and_lost_commit_response() {
    let old_wasm = std::fs::read(std::env::var("PHOTOCRAFT_BASELINE_WASM").expect("Set PHOTOCRAFT_BASELINE_WASM to the pre-extraction canister")).unwrap();
    let new_wasm = std::fs::read(std::env::var("PHOTOCRAFT_CLOUD_WASM").expect("Set PHOTOCRAFT_CLOUD_WASM to the updated canister")).unwrap();
    let pic = PocketIcBuilder::new().with_application_subnet().build();
    let canister = pic.create_canister();
    pic.add_cycles(canister, 20_000_000_000_000);
    pic.install_canister(canister, old_wasm, encode_args(()).unwrap(), None);
    let owner = Principal::self_authenticating([31; 32]);
    let client = FileClient::with_transport(PocketTransport { pic: &pic, canister, caller: owner, lose_commit_reply: Cell::new(true) });
    let bytes: Vec<_> = (0..CHUNK_BYTES + 17).map(|i| (i % 251) as u8).collect();
    let mut attempt = UploadState { file: None, upload: None, expected_revision: None, name: "Preserved file".into(), request_id: "sdk-migration-0001".into() };
    assert!(block_on(client.save(&mut attempt, &bytes, |_| {})).is_err());
    assert!(attempt.file.is_some());
    assert!(attempt.upload.is_some());
    pic.upgrade_canister(canister, new_wasm, encode_args(()).unwrap(), None).unwrap();
    let revision = block_on(client.save(&mut attempt, &bytes, |_| {})).unwrap();
    let file = attempt.file.as_ref().unwrap().id;
    let (_, restored) = block_on(client.download(file, revision.id, |_| {})).unwrap();
    assert_eq!(restored, bytes);
    assert_eq!(block_on(client.projects()).unwrap().len(), 1);
    let account = block_on(client.account()).unwrap();
    assert_eq!(account.account.owner, owner);
    assert_eq!(block_on(client.account()).unwrap().account, account.account);
    let files = block_on(client.files(account.account.personal_space, None, 20)).unwrap();
    assert_eq!(files.files.len(), 1);
    assert_eq!(files.files[0].id, file);
    assert_eq!(files.files[0].latest.id, revision.id);
    assert_eq!(block_on(client.file(account.account.personal_space, file)).unwrap().project.id, file);
    // An edit through the legacy client remains the same catalog file, at a new revision.
    let mut edit = attempt.clone();
    edit.upload = None;
    edit.expected_revision = Some(revision.id);
    edit.request_id = "sdk-catalog-edit-0001".into();
    let changed = b"Changed document";
    let newer = block_on(client.save(&mut edit, changed, |_| {})).unwrap();
    let files = block_on(client.files(account.account.personal_space, None, 20)).unwrap();
    assert_eq!(files.files.len(), 1);
    assert_eq!(files.files[0].version_count, 2);
    assert_eq!(files.files[0].latest.id, newer.id);
    assert_eq!(block_on(client.download(file, newer.id, |_| {})).unwrap().1, changed);
    let other = FileClient::with_transport(PocketTransport {
        pic: &pic,
        canister,
        caller: Principal::self_authenticating([32; 32]),
        lose_commit_reply: Cell::new(false),
    });
    assert!(block_on(other.download(file, revision.id, |_| {})).is_err());
    assert!(block_on(other.projects()).unwrap().is_empty());
    assert!(block_on(other.files(account.account.personal_space, None, 20)).is_err());
    assert!(block_on(other.file(account.account.personal_space, file)).is_err());
}

#[test]
fn all_app_formats_round_trip_and_survive_upgrade_without_cross_app_listing() {
    use sovereign_cloud::protocol::FileKind;
    let wasm = std::fs::read(std::env::var("PHOTOCRAFT_CLOUD_WASM").expect("Set PHOTOCRAFT_CLOUD_WASM")).unwrap();
    let pic = PocketIcBuilder::new().with_application_subnet().build();
    let canister = pic.create_canister();
    pic.add_cycles(canister, 20_000_000_000_000);
    pic.install_canister(canister, wasm.clone(), encode_args(()).unwrap(), None);
    let owner = Principal::self_authenticating([71; 32]);
    let client = FileClient::with_transport(PocketTransport { pic: &pic, canister, caller: owner, lose_commit_reply: Cell::new(false) });
    let mut saved = vec![];
    for app in ["photocraft", "pdfcraft", "wordcraft", "deckcraft", "soundcraft", "cadcraft", "gridcraft", "figcraft"] {
        let kind = FileKind::for_app(app).unwrap();
        let mut state = UploadState { file: None, upload: None, expected_revision: None, name: app.into(), request_id: format!("typed-upgrade-{app}") };
        let revision = block_on(client.save_typed(&mut state, app.as_bytes(), &kind, |_| {})).unwrap();
        saved.push((kind, state.file.unwrap().id, revision.id, app.as_bytes().to_vec()));
    }
    pic.upgrade_canister(canister, wasm, encode_args(()).unwrap(), None).unwrap();
    let account = block_on(client.account()).unwrap();
    let files = block_on(client.files(account.account.personal_space, None, 20)).unwrap();
    assert_eq!(files.files.len(), 8);
    assert_eq!(block_on(client.projects()).unwrap().len(), 1); // Legacy PhotoCraft picker.
    for (kind, id, revision, bytes) in saved {
        let projects = block_on(client.projects_for(&kind)).unwrap();
        assert_eq!(projects.len(), 1);
        assert_eq!(projects[0].id, id);
        assert_eq!(block_on(client.file(account.account.personal_space, id)).unwrap().kind, kind);
        assert_eq!(block_on(client.download(id, revision, |_| {})).unwrap().1, bytes);
    }
}

#[test]
fn shared_drive_sdk_journey_survives_upgrade_and_uncertain_commit() {
    use sovereign_cloud::{args, drive::*, protocol::Revision};
    let wasm = std::fs::read(std::env::var("PHOTOCRAFT_CLOUD_WASM").expect("Set PHOTOCRAFT_CLOUD_WASM")).unwrap();
    let pic = PocketIcBuilder::new().with_application_subnet().build();
    let canister = pic.create_canister();
    pic.add_cycles(canister, 20_000_000_000_000);
    let initial = std::env::var("DRIVE_PRE_INDEX_WASM").map(|path| std::fs::read(path).unwrap()).unwrap_or_else(|_| wasm.clone());
    pic.install_canister(canister, initial, args(()).unwrap(), None);
    let alice = Principal::self_authenticating([51; 32]);
    let bob = Principal::self_authenticating([52; 32]);
    let a = FileClient::with_transport(PocketTransport { pic: &pic, canister, caller: alice, lose_commit_reply: Cell::new(false) });
    let b = FileClient::with_transport(PocketTransport { pic: &pic, canister, caller: bob, lose_commit_reply: Cell::new(true) });
    let root: DriveEntry = block_on(a.drive_call("drive_initialize", args(()).unwrap())).unwrap();
    let _: DriveEntry = block_on(b.drive_call("drive_initialize", args(()).unwrap())).unwrap();
    let folder: DriveEntry = block_on(a.drive_call(
        "drive_create_folder",
        args((CreateFolder { parent: root.object.id, name: "Shared work".into(), request_id: "sdk-folder-journey".into() },)).unwrap(),
    ))
    .unwrap();
    let share = GrantRequest {
        request_id: "sdk-grant-journey-1".into(),
        root: folder.object.id,
        id: None,
        expected_version: None,
        subject: Participant::User(bob),
        capabilities: Capabilities { create: true, ..Capabilities::READ },
        active: true,
        expires_at: None,
    };
    let grant: AccessGrant = block_on(a.drive_call("drive_set_grant", args((share.clone(),)).unwrap())).unwrap();
    let target = UploadTarget::New { parent: folder.object.id, name: "report.txt".into(), format: FileFormat { media_type: "text/plain".into(), craft: None } };
    let bytes = vec![123; CHUNK_BYTES + 13];
    assert!(block_on(b.drive_save(target.clone(), &bytes, "sdk-shared-upload-1".into(), |_, _| {})).is_err());
    let pending: DriveUpload = block_on(
        b.drive_call(
            "drive_begin_upload",
            args((DriveUploadRequest {
                target: UploadTarget::New {
                    parent: folder.object.id,
                    name: "inflight.txt".into(),
                    format: FileFormat { media_type: "text/plain".into(), craft: None },
                },
                bytes: bytes.len() as u64,
                sha256: {
                    use sha2::Digest;
                    sha2::Sha256::digest(&bytes).to_vec()
                },
                request_id: "sdk-inflight-upgrade".into(),
            },))
            .unwrap(),
        ),
    )
    .unwrap();
    block_on(b.drive_call::<()>("drive_put_chunk", args((pending.id, 0u64, bytes[..CHUNK_BYTES].to_vec())).unwrap())).unwrap();
    pic.upgrade_canister(canister, wasm, args(()).unwrap(), None).unwrap();
    assert_eq!(block_on(b.drive_call::<Vec<DriveUpload>>("drive_pending_uploads", args(()).unwrap())).unwrap()[0].id, pending.id);
    assert_eq!(block_on(a.drive_call::<Vec<DriveUpload>>("drive_pending_uploads", args(()).unwrap())).unwrap()[0].id, pending.id);
    block_on(b.drive_call::<()>("drive_put_chunk", args((pending.id, 1u64, bytes[CHUNK_BYTES..].to_vec())).unwrap())).unwrap();
    let _: DriveUpload = block_on(b.drive_call("drive_commit_upload", args((pending.id,)).unwrap())).unwrap();
    assert!(block_on(b.drive_call::<Vec<DriveUpload>>("drive_pending_uploads", args(()).unwrap())).unwrap().is_empty());
    assert_eq!(block_on(b.drive_download(pending.file, pending.id)).unwrap(), bytes);
    let upload = block_on(b.drive_save(target, &bytes, "sdk-shared-upload-1".into(), |_, _| {})).unwrap();
    let home: DriveHome = block_on(a.drive_call("drive_home", args(()).unwrap())).unwrap();
    assert_eq!(home.account.account.owner, alice);
    assert_eq!(home.view.folder.object.id, root.object.id);
    assert_eq!(home.view.page.entries[0].object.id, folder.object.id);
    assert_eq!(upload.committed.as_ref().unwrap().author, bob);
    let browse: DriveBrowse = block_on(b.drive_call("drive_browse", args((folder.object.id, None::<DriveCursor>, 50u32)).unwrap())).unwrap();
    assert_eq!(browse.breadcrumbs.len(), 1);
    assert_eq!(browse.page.entries[0].object.id, upload.file);
    assert_eq!(block_on(a.drive_download(upload.file, upload.id)).unwrap(), bytes);
    assert_eq!(block_on(b.drive_download(upload.file, upload.id)).unwrap(), bytes);
    assert!(block_on(b.drive_call::<Vec<Revision>>("drive_history", args((upload.file,)).unwrap())).is_err());
    let original: DriveEntry = block_on(a.drive_call("drive_get", args((upload.file,)).unwrap())).unwrap();
    let denied = OrganizeRequest {
        object: upload.file,
        expected_version: original.object.version,
        change: OrganizationChange::Trash,
        request_id: "sdk-denied-trash-1".into(),
    };
    assert!(block_on(b.drive_call::<MutationReceipt>("drive_organize", args((denied,)).unwrap())).is_err());
    for (change, token) in [
        (OrganizationChange::Rename("renamed.txt".into()), "sdk-rename-file-1"),
        (OrganizationChange::Move(root.object.id), "sdk-move-file-001"),
        (OrganizationChange::Trash, "sdk-trash-file-01"),
    ] {
        let file: DriveEntry = block_on(a.drive_call("drive_get", args((upload.file,)).unwrap())).unwrap();
        let _: MutationReceipt = block_on(a.drive_call(
            "drive_organize",
            args((OrganizeRequest { object: upload.file, expected_version: file.object.version, change, request_id: token.into() },)).unwrap(),
        ))
        .unwrap();
    }
    assert!(block_on(a.drive_download(upload.file, upload.id)).is_err());
    let trash: DrivePage = block_on(a.drive_call("drive_trash", args((None::<DriveCursor>, 50u32)).unwrap())).unwrap();
    assert_eq!(trash.entries.len(), 1);
    let _: MutationReceipt = block_on(
        a.drive_call(
            "drive_organize",
            args((OrganizeRequest {
                object: upload.file,
                expected_version: trash.entries[0].object.version,
                change: OrganizationChange::Restore,
                request_id: "sdk-restore-file-1".into(),
            },))
            .unwrap(),
        ),
    )
    .unwrap();
    assert_eq!(block_on(a.drive_download(upload.file, upload.id)).unwrap(), bytes);
    // Moving outside the shared subtree takes effect immediately, including fixed-revision reads.
    assert!(block_on(b.drive_download(upload.file, upload.id)).is_err());
    let _: AccessGrant = block_on(
        a.drive_call(
            "drive_set_grant",
            args(
                (GrantRequest { id: Some(grant.id), expected_version: Some(grant.version), active: false, request_id: "sdk-revoke-grant-1".into(), ..share },),
            )
            .unwrap(),
        ),
    )
    .unwrap();
    assert!(block_on(b.drive_call::<DrivePage>("drive_list", args((Some(folder.object.id), None::<DriveCursor>, 50u32)).unwrap())).is_err());
}

#[test]
fn craft_shared_folder_permissions_and_conflicts_use_the_same_drive_contract() {
    use sovereign_cloud::{args, drive::*, protocol::FileKind};
    let wasm = std::fs::read(std::env::var("PHOTOCRAFT_CLOUD_WASM").expect("Set PHOTOCRAFT_CLOUD_WASM")).unwrap();
    let pic = PocketIcBuilder::new().with_application_subnet().build();
    let canister = pic.create_canister();
    pic.add_cycles(canister, 20_000_000_000_000);
    pic.install_canister(canister, wasm, args(()).unwrap(), None);
    let alice = Principal::self_authenticating([61; 32]);
    let bob = Principal::self_authenticating([62; 32]);
    let a = FileClient::with_transport(PocketTransport { pic: &pic, canister, caller: alice, lose_commit_reply: Cell::new(false) });
    let b = FileClient::with_transport(PocketTransport { pic: &pic, canister, caller: bob, lose_commit_reply: Cell::new(false) });
    let root: DriveEntry = block_on(a.drive_call("drive_initialize", args(()).unwrap())).unwrap();
    let _: DriveEntry = block_on(b.drive_call("drive_initialize", args(()).unwrap())).unwrap();
    let folder: DriveEntry = block_on(a.drive_call(
        "drive_create_folder",
        args((CreateFolder { parent: root.object.id, name: "Shared Craft work".into(), request_id: "craft-folder-test-001".into() },)).unwrap(),
    ))
    .unwrap();
    let mut grant: AccessGrant = block_on(
        a.drive_call(
            "drive_set_grant",
            args((GrantRequest {
                root: folder.object.id,
                id: None,
                expected_version: None,
                subject: Participant::User(bob),
                capabilities: Capabilities::EDIT,
                active: true,
                expires_at: None,
                request_id: "craft-grant-test-001".into(),
            },))
            .unwrap(),
        ),
    )
    .unwrap();
    let mut files = Vec::new();
    for app in ["photocraft", "wordcraft", "pdfcraft", "deckcraft", "gridcraft", "soundcraft", "cadcraft", "figcraft"] {
        let kind = FileKind::for_app(app).unwrap();
        let bytes = format!("synthetic {app} adapter payload").into_bytes();
        let target = UploadTarget::New {
            parent: folder.object.id,
            name: format!("{app}.test"),
            format: FileFormat { media_type: "application/octet-stream".into(), craft: Some(kind.clone()) },
        };
        let upload = block_on(b.drive_save_craft(target, &bytes, format!("craft-create-{app}-001"), &kind, |_, _| {})).unwrap();
        let entry = block_on(b.drive_craft_file(folder.object.space, upload.file, &kind)).unwrap();
        assert_eq!(entry.object.owner, alice);
        assert_eq!(entry.latest.as_ref().unwrap().author, bob);
        assert!(entry.capabilities.edit);
        assert!(!entry.capabilities.delete);
        assert_eq!(block_on(a.drive_download(upload.file, upload.id)).unwrap(), bytes);
        // Owner changes the same file. Bob's old version must never overwrite it.
        let changed = block_on(a.drive_save_craft(
            UploadTarget::Replace { file: upload.file, expected_revision: upload.id },
            b"Alice's newer version",
            format!("craft-owner-{app}-001"),
            &kind,
            |_, _| {},
        ))
        .unwrap();
        let error = block_on(b.drive_save_craft(
            UploadTarget::Replace { file: upload.file, expected_revision: upload.id },
            b"Stale Bob edit",
            format!("craft-conflict-{app}-001"),
            &kind,
            |_, _| {},
        ))
        .unwrap_err();
        assert!(error.contains("changed"), "{error}");
        assert_eq!(block_on(b.drive_download(upload.file, changed.id)).unwrap(), b"Alice's newer version");
        assert!(block_on(b.drive_craft_file(folder.object.space + 999, upload.file, &kind)).is_err());
        files.push((kind, upload.file, changed.id));
    }
    grant = block_on(
        a.drive_call(
            "drive_set_grant",
            args((GrantRequest {
                root: folder.object.id,
                id: Some(grant.id),
                expected_version: Some(grant.version),
                subject: Participant::User(bob),
                capabilities: Capabilities::READ,
                active: true,
                expires_at: None,
                request_id: "craft-readonly-test-001".into(),
            },))
            .unwrap(),
        ),
    )
    .unwrap();
    for (kind, file, revision) in &files {
        assert!(!block_on(b.drive_craft_file(folder.object.space, *file, kind)).unwrap().capabilities.edit);
        assert_eq!(block_on(b.drive_download(*file, *revision)).unwrap(), b"Alice's newer version");
        let error = block_on(b.drive_save_craft(
            UploadTarget::Replace { file: *file, expected_revision: *revision },
            b"Denied",
            format!("craft-readonly-{}-001", kind.created_by_app),
            kind,
            |_, _| {},
        ))
        .unwrap_err();
        assert!(error.contains("read-only"));
    }
    let _: AccessGrant = block_on(
        a.drive_call(
            "drive_set_grant",
            args((GrantRequest {
                root: folder.object.id,
                id: Some(grant.id),
                expected_version: Some(grant.version),
                subject: Participant::User(bob),
                capabilities: Capabilities::READ,
                active: false,
                expires_at: None,
                request_id: "craft-revoke-test-001".into(),
            },))
            .unwrap(),
        ),
    )
    .unwrap();
    assert!(block_on(b.drive_craft_file(folder.object.space, files[0].1, &files[0].0)).is_err());
    assert!(block_on(b.drive_download(files[0].1, files[0].2)).is_err());
}
