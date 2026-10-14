//! Exercises the actual shared-drive canister with signed browser/MCP credentials.
//! This is a Candid boundary test, not a JSON-RPC MCP transport test.
use crate::{client, query, update};
use candid::{encode_args, Principal};
use ic_agent::{identity::BasicIdentity, Agent};
use sovereign_cloud::drive::*;

pub async fn verify(
    pic: &pocket_ic::nonblocking::PocketIc,
    url: &str,
    root: &[u8],
    bob: &Agent,
    bob_mcp: &Agent,
    bob_id: Principal,
    path: &str,
) {
    let wasm = std::fs::read(path).unwrap();
    let canister = pic.create_canister().await;
    pic.add_cycles(canister, 5_000_000_000_000).await;
    pic.install_canister(canister, wasm.clone(), encode_args(()).unwrap(), None)
        .await;
    let alice = client(url, root, BasicIdentity::from_raw_key(&[10; 32]));
    let alice_root: DriveResult<DriveEntry> =
        update(&alice, canister, "drive_initialize", ()).await;
    let alice_root = alice_root.unwrap();
    let bob_root: DriveResult<DriveEntry> = update(bob, canister, "drive_initialize", ()).await;
    bob_root.unwrap();
    let folder: DriveResult<DriveEntry> = update(
        &alice,
        canister,
        "drive_create_folder",
        (CreateFolder {
            parent: alice_root.object.id,
            name: "Shared with Bob".into(),
            request_id: "signed-folder-creation".into(),
        },),
    )
    .await;
    let folder = folder.unwrap();
    let make_file = |name: &str, parent| InlineFile {
        parent,
        name: name.into(),
        request_id: format!("signed-file-{name}"),
        bytes: format!("Contents of {name}").into_bytes(),
        format: FileFormat {
            media_type: "text/plain".into(),
            craft: None,
        },
    };
    let original: DriveResult<DriveEntry> = update(
        &alice,
        canister,
        "drive_create_file",
        (make_file("Alice.txt", folder.object.id),),
    )
    .await;
    let original = original.unwrap();
    let denied: DriveResult<DriveEntry> =
        update(bob_mcp, canister, "drive_get", (original.object.id,)).await;
    assert!(matches!(denied, Err(DriveError::Inaccessible)));
    let rights = Capabilities {
        create: true,
        ..Capabilities::READ
    };
    let grant: DriveResult<AccessGrant> = update(
        &alice,
        canister,
        "drive_set_grant",
        (GrantRequest {
            request_id: "signed-grant-request-1".into(),
            id: None,
            expected_version: None,
            root: folder.object.id,
            subject: Participant::User(bob_id),
            capabilities: rights,
            expires_at: None,
            active: true,
        },),
    )
    .await;
    let grant = grant.unwrap();
    for (agent, name) in [(bob, "Bob-browser.txt"), (bob_mcp, "Bob-MCP.txt")] {
        let created: DriveResult<DriveEntry> = update(
            agent,
            canister,
            "drive_create_file",
            (make_file(name, folder.object.id),),
        )
        .await;
        let created = created.unwrap();
        assert_eq!(created.object.owner, alice_root.object.owner);
        assert_eq!(
            created.attribution.unwrap().actor,
            Participant::User(bob_id)
        );
        assert_eq!(created.capabilities, Capabilities::READ);
        let rev = created.latest.unwrap();
        let data: DriveResult<Vec<u8>> = query(
            agent,
            canister,
            "drive_current_chunk",
            (created.object.id, rev.id, 0_u64),
        )
        .await;
        assert_eq!(data.unwrap(), make_file(name, folder.object.id).bytes);
        let denied: DriveResult<DriveEntry> = update(
            agent,
            canister,
            "drive_create_file",
            (make_file("Private.txt", alice_root.object.id),),
        )
        .await;
        assert!(matches!(denied, Err(DriveError::Inaccessible)));
        let denied: DriveResult<MutationReceipt> = update(
            agent,
            canister,
            "drive_organize",
            (OrganizeRequest {
                object: created.object.id,
                expected_version: 1,
                request_id: "no-edit-through-create".into(),
                change: OrganizationChange::Rename("Changed.txt".into()),
            },),
        )
        .await;
        assert!(matches!(denied, Err(DriveError::Inaccessible)));
        let denied: DriveResult<MutationReceipt> = update(
            agent,
            canister,
            "drive_organize",
            (OrganizeRequest {
                object: created.object.id,
                expected_version: 1,
                request_id: "no-delete-through-create".into(),
                change: OrganizationChange::Trash,
            },),
        )
        .await;
        assert!(matches!(denied, Err(DriveError::Inaccessible)));
    }
    pic.upgrade_canister(canister, wasm, encode_args(()).unwrap(), None)
        .await
        .unwrap();
    let page: DriveResult<DrivePage> = update(
        bob_mcp,
        canister,
        "drive_list",
        (Some(folder.object.id), None::<DriveCursor>, 50_u32),
    )
    .await;
    assert_eq!(page.unwrap().entries.len(), 3);
    let revoked: DriveResult<AccessGrant> = update(
        &alice,
        canister,
        "drive_set_grant",
        (GrantRequest {
            request_id: "signed-grant-request-2".into(),
            id: Some(grant.id),
            expected_version: Some(grant.version),
            root: folder.object.id,
            subject: Participant::User(bob_id),
            capabilities: rights,
            expires_at: None,
            active: false,
        },),
    )
    .await;
    revoked.unwrap();
    for agent in [bob, bob_mcp] {
        let denied: DriveResult<DriveEntry> =
            update(agent, canister, "drive_get", (original.object.id,)).await;
        assert!(matches!(denied, Err(DriveError::Inaccessible)));
        let retry: DriveResult<DriveEntry> = update(
            agent,
            canister,
            "drive_create_file",
            (make_file("Bob-MCP.txt", folder.object.id),),
        )
        .await;
        assert!(matches!(retry, Err(DriveError::Inaccessible)));
    }
    println!("PASS actual cloud canister: signed users and II-issued MCP delegation share Read+Create, create/read identical bytes, deny Edit/Delete/outside scope, preserve through upgrade, and revoke access including retries");
}
