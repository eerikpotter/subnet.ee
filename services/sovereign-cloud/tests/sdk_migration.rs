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
        if method == "commit_upload" && self.lose_commit_reply.replace(false) {
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
    for app in ["photocraft", "pdfcraft", "wordcraft", "deckcraft", "soundcraft"] {
        let kind = FileKind::for_app(app).unwrap();
        let mut state = UploadState { file: None, upload: None, expected_revision: None, name: app.into(), request_id: format!("typed-upgrade-{app}") };
        let revision = block_on(client.save_typed(&mut state, app.as_bytes(), &kind, |_| {})).unwrap();
        saved.push((kind, state.file.unwrap().id, revision.id, app.as_bytes().to_vec()));
    }
    pic.upgrade_canister(canister, wasm, encode_args(()).unwrap(), None).unwrap();
    let account = block_on(client.account()).unwrap();
    let files = block_on(client.files(account.account.personal_space, None, 20)).unwrap();
    assert_eq!(files.files.len(), 5);
    assert_eq!(block_on(client.projects()).unwrap().len(), 1); // Legacy PhotoCraft picker.
    for (kind, id, revision, bytes) in saved {
        let projects = block_on(client.projects_for(&kind)).unwrap();
        assert_eq!(projects.len(), 1);
        assert_eq!(projects[0].id, id);
        assert_eq!(block_on(client.file(account.account.personal_space, id)).unwrap().kind, kind);
        assert_eq!(block_on(client.download(id, revision, |_| {})).unwrap().1, bytes);
    }
}
