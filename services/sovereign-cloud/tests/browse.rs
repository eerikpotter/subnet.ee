//! New-schema upgrade and shared browsing journey on an isolated PocketIC canister.
use candid::{CandidType, Principal, decode_one, encode_args, utils::ArgumentEncoder};
use pocket_ic::PocketIcBuilder;
use serde::de::DeserializeOwned;
use sovereign_cloud::drive::*;
fn call<T: CandidType + DeserializeOwned>(pic: &pocket_ic::PocketIc, id: Principal, caller: Principal, method: &str, args: impl ArgumentEncoder) -> T {
    decode_one(&pic.update_call(id, caller, method, encode_args(args).unwrap()).unwrap()).unwrap()
}
fn query(scope: BrowseScope) -> BrowseQuery {
    BrowseQuery {
        scope,
        text: String::new(),
        file_type: String::new(),
        owner: None,
        modified_after: None,
        modified_before: None,
        sort: SortField::Name,
        descending: false,
    }
}
#[test]
fn browse_upgrade_preserves_files_grants_staging_and_private_preferences() {
    let wasm = std::fs::read(std::env::var("PHOTOCRAFT_CLOUD_WASM").expect("Set current Wasm")).unwrap();
    let pic = PocketIcBuilder::new().with_application_subnet().build();
    let id = pic.create_canister();
    pic.add_cycles(id, 100_000_000_000_000);
    pic.install_canister(id, wasm.clone(), encode_args(()).unwrap(), None);
    let a = Principal::self_authenticating([201; 32]);
    let b = Principal::self_authenticating([202; 32]);
    let root: DriveResult<DriveEntry> = call(&pic, id, a, "drive_initialize", ());
    let root = root.unwrap().object.id;
    let other: DriveResult<DriveEntry> = call(&pic, id, b, "drive_initialize", ());
    other.unwrap();
    let request = CreateFolder { parent: root, name: "Shared plans".into(), request_id: "browse-upgrade-folder".into() };
    let folder: DriveResult<DriveEntry> = call(&pic, id, a, "drive_create_folder", (request.clone(),));
    let folder = folder.unwrap().object.id;
    let file: DriveResult<DriveEntry> = call(
        &pic,
        id,
        a,
        "drive_create_file",
        (InlineFile {
            parent: folder,
            name: "Roadmap.txt".into(),
            format: FileFormat { media_type: "text/plain".into(), craft: None },
            bytes: b"the plan".to_vec(),
            request_id: "browse-upgrade-file".into(),
        },),
    );
    let file = file.unwrap();
    let grant = GrantRequest {
        root: folder,
        id: None,
        expected_version: None,
        subject: Participant::User(b),
        capabilities: Capabilities::READ,
        active: true,
        expires_at: None,
        request_id: "browse-upgrade-grant".into(),
    };
    let granted: DriveResult<AccessGrant> = call(&pic, id, a, "drive_set_grant", (grant.clone(),));
    let granted = granted.unwrap();
    let stage: DriveResult<DriveUpload> = call(
        &pic,
        id,
        a,
        "drive_begin_upload",
        (DriveUploadRequest {
            automatic: None,
            target: UploadTarget::New { parent: folder, name: "Pending.txt".into(), format: FileFormat { media_type: "text/plain".into(), craft: None } },
            bytes: 1,
            sha256: vec![0; 32],
            request_id: "browse-upgrade-stage".into(),
        },),
    );
    let stage = stage.unwrap();
    pic.upgrade_canister(id, wasm.clone(), encode_args(()).unwrap(), None).unwrap();
    let retry: DriveResult<DriveEntry> = call(&pic, id, a, "drive_create_folder", (request,));
    assert_eq!(retry.unwrap().object.id, folder);
    let pending: DriveResult<Vec<DriveUpload>> = call(&pic, id, a, "drive_pending_uploads", ());
    assert_eq!(pending.unwrap()[0].id, stage.id);
    let mut search = query(BrowseScope::All);
    search.text = "ROAD".into();
    let found: DriveResult<BrowsePage> = call(&pic, id, b, "drive_query", (search.clone(), None::<BrowseCursor>, 50u32));
    let found = found.unwrap();
    assert_eq!(found.items.len(), 1);
    assert_eq!(found.items[0].entry.object.id, file.object.id);
    let bytes = pic.query_call(id, b, "drive_current_chunk", encode_args((file.object.id, file.latest.unwrap().id, 0u64)).unwrap()).unwrap();
    assert_eq!(decode_one::<DriveResult<Vec<u8>>>(&bytes).unwrap().unwrap(), b"the plan");
    let star: DriveResult<()> = call(&pic, id, b, "drive_star", (file.object.id, true));
    star.unwrap();
    let opened: DriveResult<DriveEntry> = call(&pic, id, b, "drive_open", (file.object.id,));
    opened.unwrap();
    let preferences = BrowsePreferences { view: ViewMode::Grid, sort: SortField::Modified, descending: true };
    let saved: DriveResult<()> = call(&pic, id, b, "drive_set_preferences", (preferences.clone(),));
    saved.unwrap();
    pic.upgrade_canister(id, wasm, encode_args(()).unwrap(), None).unwrap();
    let restored: DriveResult<BrowseHome> = call(&pic, id, b, "drive_browser_home", ());
    assert_eq!(restored.unwrap().page.preferences, preferences);
    for scope in [BrowseScope::Starred, BrowseScope::Recent] {
        let page: DriveResult<BrowsePage> = call(&pic, id, b, "drive_query", (query(scope.clone()), None::<BrowseCursor>, 50u32));
        assert_eq!(page.unwrap().items[0].entry.object.id, file.object.id);
        let private: DriveResult<BrowsePage> = call(&pic, id, a, "drive_query", (query(scope), None::<BrowseCursor>, 50u32));
        assert!(private.unwrap().items.is_empty());
    }
    let revoked: DriveResult<AccessGrant> = call(
        &pic,
        id,
        a,
        "drive_set_grant",
        (GrantRequest { id: Some(granted.id), expected_version: Some(granted.version), active: false, request_id: "browse-upgrade-revoke".into(), ..grant },),
    );
    revoked.unwrap();
    for scope in [BrowseScope::All, BrowseScope::Starred, BrowseScope::Recent] {
        search.scope = scope;
        let page: DriveResult<BrowsePage> = call(&pic, id, b, "drive_query", (search.clone(), None::<BrowseCursor>, 50u32));
        assert!(page.unwrap().items.is_empty());
    }
    let denied: DriveResult<DriveEntry> = call(&pic, id, b, "drive_open", (file.object.id,));
    assert!(denied.is_err());
}

#[test]
fn suggested_home_upgrade_reads_existing_viewer_records_and_preserves_new_summaries() {
    let wasm = std::fs::read(std::env::var("PHOTOCRAFT_CLOUD_WASM").expect("Set current Wasm")).unwrap();
    let pic = PocketIcBuilder::new().with_application_subnet().build();
    let id = pic.create_canister();
    pic.add_cycles(id, 100_000_000_000_000);
    pic.install_canister(id, wasm.clone(), encode_args(()).unwrap(), None);
    let a = Principal::self_authenticating([211; 32]);
    let b = Principal::self_authenticating([212; 32]);
    let root: DriveResult<DriveEntry> = call(&pic, id, a, "drive_initialize", ());
    let root = root.unwrap();
    let other: DriveResult<DriveEntry> = call(&pic, id, b, "drive_initialize", ());
    other.unwrap();
    let request = InlineFile {
        parent: root.object.id,
        name: "Upgrade plan.txt".into(),
        format: FileFormat { media_type: "text/plain".into(), craft: None },
        bytes: b"preserved content".to_vec(),
        request_id: "suggestion-upgrade-file".into(),
    };
    let file: DriveResult<DriveEntry> = call(&pic, id, a, "drive_create_file", (request.clone(),));
    let file = file.unwrap();
    let grant: DriveResult<AccessGrant> = call(
        &pic,
        id,
        a,
        "drive_set_grant",
        (GrantRequest {
            root: file.object.id,
            id: None,
            expected_version: None,
            subject: Participant::User(b),
            capabilities: Capabilities::READ,
            active: true,
            expires_at: None,
            request_id: "suggestion-upgrade-grant".into(),
        },),
    );
    grant.unwrap();
    let star: DriveResult<()> = call(&pic, id, b, "drive_star", (file.object.id, true));
    star.unwrap();
    let opened: DriveResult<DriveEntry> = call(&pic, id, b, "drive_open", (file.object.id,));
    opened.unwrap();
    let pending: DriveResult<DriveUpload> = call(
        &pic,
        id,
        a,
        "drive_begin_upload",
        (DriveUploadRequest {
            automatic: None,
            target: UploadTarget::New {
                parent: root.object.id,
                name: "Pending.txt".into(),
                format: FileFormat { media_type: "text/plain".into(), craft: None },
            },
            bytes: 1,
            sha256: vec![0; 32],
            request_id: "suggestion-upgrade-pending".into(),
        },),
    );
    let pending = pending.unwrap();
    pic.upgrade_canister(id, wasm.clone(), encode_args(()).unwrap(), None).unwrap();
    let home: DriveResult<SuggestedHome> = call(&pic, id, b, "drive_suggested_home", ());
    let home = home.unwrap();
    assert_eq!(home.files[0].item.entry.object.id, file.object.id);
    assert!(home.files[0].item.starred);
    assert_eq!(home.files[0].reason, SuggestionReason::RecentlyOpened);
    let retry: DriveResult<DriveEntry> = call(&pic, id, a, "drive_create_file", (request,));
    assert_eq!(retry.unwrap().object.id, file.object.id);
    let stages: DriveResult<Vec<DriveUpload>> = call(&pic, id, a, "drive_pending_uploads", ());
    assert_eq!(stages.unwrap()[0].id, pending.id);
    let revision = file.latest.unwrap().id;
    let ack: DriveResult<()> = call(&pic, id, b, "drive_acknowledge_open", (file.object.id, Some(revision)));
    ack.unwrap();
    let dismiss: DriveResult<()> = call(&pic, id, b, "drive_dismiss_suggestion", (file.object.id,));
    dismiss.unwrap();
    pic.upgrade_canister(id, wasm, encode_args(()).unwrap(), None).unwrap();
    let home: DriveResult<SuggestedHome> = call(&pic, id, b, "drive_suggested_home", ());
    assert!(home.unwrap().files.is_empty());
    for scope in [BrowseScope::Recent, BrowseScope::Starred] {
        let page: DriveResult<BrowsePage> = call(&pic, id, b, "drive_query", (query(scope), None::<BrowseCursor>, 50u32));
        assert_eq!(page.unwrap().items[0].entry.object.id, file.object.id);
    }
    let owner: DriveResult<SuggestedHome> = call(&pic, id, a, "drive_suggested_home", ());
    assert_eq!(owner.unwrap().files[0].reason, SuggestionReason::RecentlyCreated);
    let bytes = pic.query_call(id, b, "drive_current_chunk", encode_args((file.object.id, revision, 0u64)).unwrap()).unwrap();
    assert_eq!(decode_one::<DriveResult<Vec<u8>>>(&bytes).unwrap().unwrap(), b"preserved content");
    pic.advance_time(std::time::Duration::from_secs(8 * 24 * 60 * 60));
    let home: DriveResult<SuggestedHome> = call(&pic, id, b, "drive_suggested_home", ());
    assert_eq!(home.unwrap().files.len(), 1);
}
