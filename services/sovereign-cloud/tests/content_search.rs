//! Actual Wasm timer, canonical extraction, live grants, public reads and upgrade.
use candid::{CandidType, Principal, decode_one, encode_args, utils::ArgumentEncoder};
use serde::de::DeserializeOwned;
use sovereign_cloud::{drive::*, protocol::FileKind, public_link::*, search::*};
use std::time::Duration;
fn call<T: CandidType + DeserializeOwned>(pic: &pocket_ic::PocketIc, id: Principal, who: Principal, method: &str, args: impl ArgumentEncoder) -> T {
    decode_one(&pic.update_call(id, who, method, encode_args(args).unwrap()).unwrap()).unwrap()
}
#[test]
fn all_formats_are_indexed_by_the_timer_and_survive_populated_upgrade() {
    let wasm = std::fs::read(std::env::var("PHOTOCRAFT_CLOUD_WASM").expect("Set current Wasm")).unwrap();
    let pic = pocket_ic::PocketIcBuilder::new().with_application_subnet().build();
    let id = pic.create_canister();
    pic.add_cycles(id, 100_000_000_000_000);
    pic.install_canister(id, wasm.clone(), encode_args(()).unwrap(), None);
    let a = Principal::self_authenticating([171; 32]);
    let b = Principal::self_authenticating([172; 32]);
    let root = call::<DriveResult<DriveEntry>>(&pic, id, a, "drive_initialize", ()).unwrap().object.id;
    call::<DriveResult<DriveEntry>>(&pic, id, b, "drive_initialize", ()).unwrap();
    let mut files = Vec::new();
    for (name, bytes, media) in [
        ("notes.txt", include_bytes!("fixtures/search/notes.txt").as_slice(), "text/plain"),
        ("document.pdf", include_bytes!("fixtures/search/document.pdf").as_slice(), "application/pdf"),
        (
            "document.docx",
            include_bytes!("fixtures/search/document.docx").as_slice(),
            "application/vnd.openxmlformats-officedocument.wordprocessingml.document",
        ),
        ("workbook.xlsx", include_bytes!("fixtures/search/workbook.xlsx").as_slice(), "application/vnd.openxmlformats-officedocument.spreadsheetml.sheet"),
        (
            "presentation.pptx",
            include_bytes!("fixtures/search/presentation.pptx").as_slice(),
            "application/vnd.openxmlformats-officedocument.presentationml.presentation",
        ),
        ("word.wcraft.json", include_bytes!("fixtures/search/word.wcraft.json").as_slice(), "application/json"),
    ] {
        files.push(
            call::<DriveResult<DriveEntry>>(
                &pic,
                id,
                a,
                "drive_create_file",
                (InlineFile {
                    parent: root,
                    name: name.into(),
                    bytes: bytes.to_vec(),
                    format: FileFormat { media_type: media.into(), craft: if name.ends_with(".pdf") { FileKind::for_app("pdfcraft") } else { None } },
                    request_id: format!("indexed-{name}-request"),
                },),
            )
            .unwrap(),
        );
    }
    // Timers are rebuilt on upgrade, while durable work remains in stable memory.
    pic.upgrade_canister(id, wasm.clone(), encode_args(()).unwrap(), None).unwrap();
    for _ in 0..12 {
        pic.advance_time(Duration::from_secs(3));
        for _ in 0..5 {
            pic.tick();
        }
    }
    for file in &files {
        let state = call::<DriveResult<ContentStatus>>(&pic, id, a, "content_status", (file.object.id,)).unwrap();
        assert_eq!(state.state, IndexState::Ready, "{}: {:?}", file.object.name, state);
    }
    let query = ContentQuery { text: "neptuun kinnitatud".into(), folder: None, cursor: None, limit: 50 };
    let hits = call::<DriveResult<ContentPage>>(&pic, id, a, "content_query", (query.clone(),)).unwrap();
    assert_eq!(hits.hits.len(), 6);
    assert!(hits.hits.iter().any(|h| h.location.starts_with("page:")));
    assert!(hits.hits.iter().any(|h| h.location.starts_with("sheet:Tulud")));
    assert!(call::<DriveResult<ContentPage>>(&pic, id, b, "content_query", (query.clone(),)).unwrap().hits.is_empty());
    let file = &files[1];
    let grant = call::<DriveResult<AccessGrant>>(
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
            request_id: "indexed-share-request".into(),
        },),
    )
    .unwrap();
    assert_eq!(call::<DriveResult<ContentPage>>(&pic, id, b, "content_query", (query.clone(),)).unwrap().hits.len(), 1);
    call::<DriveResult<PublicLinkState>>(
        &pic,
        id,
        a,
        "public_link_set",
        (SetPublicLink { object: file.object.id, enabled: true, expires_at: None, expected_version: 0, request_id: "indexed-public-request".into() },),
    )
    .unwrap();
    assert!(call::<DriveResult<PublicItem>>(&pic, id, Principal::anonymous(), "public_get", (file.object.id,)).is_ok());
    let anonymous = pic.update_call(id, Principal::anonymous(), "content_query", encode_args((query.clone(),)).unwrap());
    assert!(anonymous.is_err() || decode_one::<DriveResult<ContentPage>>(&anonymous.unwrap()).unwrap().is_err());
    assert!(call::<DriveResult<()>>(&pic, id, a, "content_process_self", (file.object.id, file.latest.as_ref().unwrap().id)).is_err());
    pic.upgrade_canister(id, wasm, encode_args(()).unwrap(), None).unwrap();
    assert_eq!(call::<DriveResult<ContentPage>>(&pic, id, a, "content_query", (query.clone(),)).unwrap().hits.len(), 6);
    call::<DriveResult<AccessGrant>>(
        &pic,
        id,
        a,
        "drive_set_grant",
        (GrantRequest {
            root: file.object.id,
            id: Some(grant.id),
            expected_version: Some(grant.version),
            subject: Participant::User(b),
            capabilities: Capabilities::READ,
            active: false,
            expires_at: None,
            request_id: "indexed-revoke-request".into(),
        },),
    )
    .unwrap();
    assert!(call::<DriveResult<ContentPage>>(&pic, id, b, "content_query", (query,)).unwrap().hits.is_empty());
}
