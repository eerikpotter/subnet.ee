//! Real canister journey: personal data, team roles, departure and populated upgrade.
use candid::{CandidType, Principal, decode_one, encode_args, utils::ArgumentEncoder};
use pocket_ic::PocketIcBuilder;
use serde::de::DeserializeOwned;
use sovereign_cloud::{drive::*, spaces::*};
fn call<T: CandidType + DeserializeOwned>(pic: &pocket_ic::PocketIc, id: Principal, who: Principal, method: &str, args: impl ArgumentEncoder) -> T {
    decode_one(&pic.update_call(id, who, method, encode_args(args).unwrap()).unwrap()).unwrap()
}
#[test]
fn team_membership_and_content_survive_upgrade_without_founder_authority() {
    let wasm = std::fs::read(std::env::var("PHOTOCRAFT_CLOUD_WASM").expect("Set current Wasm")).unwrap();
    let pic = PocketIcBuilder::new().with_application_subnet().build();
    let canister = pic.create_canister();
    pic.add_cycles(canister, 100_000_000_000_000);
    pic.install_canister(canister, wasm.clone(), encode_args(()).unwrap(), None);
    let founder = Principal::self_authenticating([231; 32]);
    let manager = Principal::self_authenticating([232; 32]);
    for who in [founder, manager] {
        call::<DriveResult<DriveEntry>>(&pic, canister, who, "drive_initialize", ()).unwrap();
    }
    let team = call::<DriveResult<SpaceView>>(
        &pic,
        canister,
        founder,
        "space_create",
        (CreateSpace { name: "Research".into(), request_id: "research-team-create".into() },),
    )
    .unwrap();
    let id = team.space.id;
    let changed = call::<DriveResult<TeamSpace>>(
        &pic,
        canister,
        founder,
        "space_change_member",
        (ChangeMember { space: id, principal: manager, role: Some(SpaceRole::Manager), expected_version: 1, request_id: "research-add-manager".into() },),
    )
    .unwrap();
    let file = call::<DriveResult<DriveEntry>>(
        &pic,
        canister,
        manager,
        "drive_create_file",
        (InlineFile {
            parent: id,
            name: "Findings.txt".into(),
            bytes: b"team findings".to_vec(),
            format: FileFormat { media_type: "text/plain".into(), craft: None },
            request_id: "research-create-file".into(),
        },),
    )
    .unwrap();
    call::<DriveResult<TeamSpace>>(
        &pic,
        canister,
        manager,
        "space_change_member",
        (ChangeMember { space: id, principal: founder, role: None, expected_version: changed.version, request_id: "research-remove-founder".into() },),
    )
    .unwrap();
    pic.upgrade_canister(canister, wasm, encode_args(()).unwrap(), None).unwrap();
    assert!(call::<DriveResult<DriveEntry>>(&pic, canister, founder, "drive_get", (file.object.id,)).is_err());
    assert!(call::<DriveResult<SpaceMembers>>(&pic, canister, founder, "space_members", (id, None::<Principal>, 50u32)).is_err());
    let current = call::<DriveResult<DriveEntry>>(&pic, canister, manager, "drive_get", (file.object.id,)).unwrap();
    assert_eq!(current.object.id, file.object.id);
    assert!(current.space_access.unwrap().manage);
    let bytes = pic.query_call(canister, manager, "drive_current_chunk", encode_args((file.object.id, file.latest.unwrap().id, 0u64)).unwrap()).unwrap();
    assert_eq!(decode_one::<DriveResult<Vec<u8>>>(&bytes).unwrap().unwrap(), b"team findings");
    let view = call::<DriveResult<SpaceView>>(&pic, canister, manager, "space_get", (id,)).unwrap();
    assert_eq!(view.used_bytes, 13);
    assert!(
        call::<DriveResult<TeamSpace>>(
            &pic,
            canister,
            manager,
            "space_change_member",
            (ChangeMember { space: id, principal: manager, role: None, expected_version: view.space.version, request_id: "research-remove-last".into() },)
        )
        .is_err()
    );
}
