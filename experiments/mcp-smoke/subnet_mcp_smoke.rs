//! Loaded only into a disposable checkout of the pinned DFINITY imcp2 test crate.
//! Real MCP JSON-RPC, signed ingress, metadata and the actual cloud Wasm.
//! Web discovery and delegation issuance use upstream's test-only fixtures;
//! real II issuance is checked separately by ../agent-provenance.
use crate::{
    discover::webfixture,
    identities::{Identities, IiInstance},
    tools::IcTools,
};
use candid::{decode_one, CandidType, Principal};
use ic_agent::{identity::BasicIdentity, Agent, Identity};
use rmcp::{
    model::{CallToolRequestParams, CallToolResult},
    service::{RunningService, ServiceExt},
    RoleClient,
};
use serde::Deserialize;
use std::{path::PathBuf, sync::Arc, time::SystemTime};

// Deliberate reply projections: ignore unrelated fields while checking ownership,
// content and attribution against the service's real published Candid contract.
#[derive(CandidType, Deserialize, Debug)]
struct Entry {
    object: Object,
    latest: Option<Revision>,
}
#[derive(CandidType, Deserialize, Debug)]
struct Object {
    id: u64,
    owner: Principal,
    name: String,
    version: u64,
}
#[derive(CandidType, Deserialize, Debug)]
struct Revision {
    id: u64,
    author: Principal,
    bytes: u64,
}
#[derive(CandidType, Deserialize, Debug)]
struct Grant {
    id: u64,
    version: u64,
}
#[derive(CandidType, Deserialize, Debug)]
struct Receipt {
    object: u64,
    version: u64,
}
#[derive(CandidType, Deserialize, Debug, PartialEq)]
enum Error {
    Inaccessible,
    InvalidInput(String),
    Conflict,
    NameConflict,
    Limit(String),
    Expired,
    Internal,
}
type Reply<T> = Result<T, Error>;

fn encode(did: &str, method: &str, text: &str, reply: bool) -> Vec<u8> {
    let ast: candid_parser::IDLProg = did.parse().unwrap();
    let mut env = candid::TypeEnv::new();
    let actor = candid_parser::check_prog(&mut env, &ast).unwrap().unwrap();
    let method = env.get_method(&actor, method).unwrap();
    candid_parser::parse_idl_args(text)
        .unwrap()
        .to_bytes_with_types(&env, if reply { &method.rets } else { &method.args })
        .unwrap()
}

async fn direct<T: CandidType + for<'de> Deserialize<'de>>(
    agent: &Agent,
    cloud: Principal,
    did: &str,
    method: &str,
    args: &str,
) -> T {
    let bytes = agent
        .update(&cloud, method)
        .with_arg(encode(did, method, args, false))
        .call_and_wait()
        .await
        .unwrap();
    decode_one(&bytes).unwrap()
}

fn text(result: &CallToolResult) -> String {
    result
        .content
        .iter()
        .filter_map(|c| c.as_text().map(|t| t.text.as_str()))
        .collect::<Vec<_>>()
        .join("\n")
}

async fn tool(
    client: &RunningService<RoleClient, ()>,
    name: &str,
    args: serde_json::Value,
) -> CallToolResult {
    client
        .call_tool(
            CallToolRequestParams::new(name.to_owned())
                .with_arguments(args.as_object().unwrap().clone()),
        )
        .await
        .unwrap()
}

struct Mcp<'a> {
    client: &'a RunningService<RoleClient, ()>,
    cloud: Principal,
    origin: &'a str,
    identity_origin: &'a str,
    did: &'a str,
}
impl Mcp<'_> {
    async fn call<T: CandidType + for<'de> Deserialize<'de>>(
        &self,
        method: &str,
        args: &str,
        query: bool,
    ) -> T {
        let result = tool(
            self.client,
            if query {
                "canister_query"
            } else {
                "canister_update_call"
            },
            serde_json::json!({"canister_id": self.cloud.to_text(), "app_url": self.origin,
                "derivation_origin": self.identity_origin, "method": method, "args": args}),
        )
        .await;
        assert_ne!(result.is_error, Some(true), "{method}: {}", text(&result));
        let provenance = result.structured_content.as_ref().unwrap();
        assert_eq!(provenance["is_anonymous"], false);
        // II's browser flow and imcp2 both map gateway aliases to ic0.app
        // before deriving an account. The served pointer stays on icp.net.
        assert_eq!(
            provenance["derived_for_origin"],
            self.identity_origin.replace(".icp.net", ".ic0.app")
        );
        assert_eq!(provenance["declared_by"], self.origin);
        let reply = result.content[0].as_text().unwrap().text.as_str();
        decode_one(&encode(self.did, method, reply, true)).unwrap()
    }
}

fn file(parent: u64, name: &str, request: &str) -> String {
    format!(
        r#"(record {{ parent = {parent}; name = "{name}"; request_id = "{request}";
        bytes = blob "Created through MCP";
        format = record {{ media_type = "text/plain"; craft = null }} }})"#
    )
}

#[tokio::test]
async fn subnet_mcp_smoke() {
    let repo = PathBuf::from(std::env::var("SUBNET_REPO").expect("set SUBNET_REPO"));
    let wasm = std::fs::read(
        std::env::var("CLOUD_WASM").expect("set CLOUD_WASM to the recipe-built Wasm"),
    )
    .unwrap();
    let did = std::fs::read_to_string(repo.join("services/sovereign-cloud/cloud.did")).unwrap();
    let mut pic = pocket_ic::PocketIcBuilder::new()
        .with_nns_subnet()
        .with_application_subnet()
        .build_async()
        .await;
    pic.set_time(SystemTime::now().into()).await;
    pic.tick().await;
    let cloud = pic.create_canister().await;
    pic.add_cycles(cloud, 5_000_000_000_000).await;
    pic.install_canister(cloud, wasm, candid::encode_args(()).unwrap(), None)
        .await;
    let frontend = pic.create_canister().await;
    let gateway = pic.make_live(None).await;
    let root_key = pic.root_key().await.unwrap();
    let anonymous = Agent::builder().with_url(gateway.as_str()).build().unwrap();
    anonymous.set_root_key(root_key.clone());
    let alice_identity = BasicIdentity::from_raw_key(&[41; 32]);
    let alice_id = alice_identity.sender().unwrap();
    let alice = Agent::builder()
        .with_url(gateway.as_str())
        .with_identity(alice_identity)
        .build()
        .unwrap();
    alice.set_root_key(root_key);

    // Consume the actual deployment generator's output. Only public web retrieval
    // is substituted: upstream correctly disallows fetching loopback origins.
    let fixture = std::env::temp_dir().join(format!("subnet-mcp-discovery-{}", std::process::id()));
    std::fs::create_dir_all(&fixture).unwrap();
    let generated = std::process::Command::new("python3").arg("-c").arg(
        "import importlib.util,os,pathlib,sys; s=importlib.util.spec_from_file_location('m',sys.argv[1]); m=importlib.util.module_from_spec(s); s.loader.exec_module(m); m.generate(pathlib.Path(sys.argv[2]),os.environ)")
        .arg(repo.join("scripts/mcp-discovery.py")).arg(&fixture)
        .env("ICP_CLI_CID_CLOUD", cloud.to_text()).env("ICP_CLI_CID_FRONTEND", frontend.to_text())
        .env("ICP_CLI_NETWORK", "ic").status().unwrap();
    assert!(generated.success());
    let manifest = std::fs::read_to_string(fixture.join(".well-known/ic-architecture")).unwrap();
    let identity_origin =
        std::fs::read_to_string(fixture.join(".well-known/ii-derivation-origin")).unwrap();
    let identity_origin = identity_origin.trim();
    let origin = "https://subnet-mcp.e2e.test";
    webfixture::serve(
        origin,
        webfixture::Site {
            architecture: Some(manifest),
            architecture_content_type: "application/json".into(),
            derivation_origin: Some(identity_origin.into()),
            ..Default::default()
        },
    );
    webfixture::serve(
        identity_origin,
        webfixture::Site::default().authorizing(&[origin]),
    );
    let identities = Identities::new(
        IiInstance {
            name: "fixture",
            ii_url: "https://ii.invalid".into(),
            ii_canister: Principal::anonymous(),
        },
        "https://mcp.invalid".into(),
        anonymous.clone(),
    );
    let bob_id = identities
        .seed_app_identity("subnet-smoke", identity_origin)
        .await
        .unwrap();
    let tools = IcTools::new(
        anonymous,
        identities,
        Arc::new(|_| Some("subnet-smoke".into())),
    );
    let (server_io, client_io) = tokio::io::duplex(1 << 20);
    let (server, client) = tokio::join!(tools.serve(server_io), ().serve(client_io));
    let server = server.unwrap();
    let client = client.unwrap();
    println!(
        "Negotiated MCP: {:?}",
        client.peer_info().unwrap().protocol_version
    );
    let advertised = client.list_all_tools().await.unwrap();
    for name in [
        "get_canister_candid",
        "get_canister_api_doc",
        "canister_update_call",
        "canister_query",
    ] {
        assert!(advertised.iter().any(|t| t.name == name));
    }
    let discovered = tool(
        &client,
        "get_canister_candid",
        serde_json::json!({
        "canister_id": cloud.to_text(), "app_url": origin }),
    )
    .await;
    assert_ne!(discovered.is_error, Some(true), "{}", text(&discovered));
    assert!(text(&discovered).contains("drive_create_file"));
    assert_eq!(
        discovered.structured_content.as_ref().unwrap()["api_doc_available"],
        true
    );
    let doc = tool(
        &client,
        "get_canister_api_doc",
        serde_json::json!({
        "canister_id": cloud.to_text(), "app_url": origin }),
    )
    .await;
    assert_ne!(doc.is_error, Some(true), "{}", text(&doc));
    assert!(text(&doc).contains("identical account authority"));
    let undeclared = tool(
        &client,
        "get_canister_candid",
        serde_json::json!({
        "canister_id": frontend.to_text(), "app_url": origin }),
    )
    .await;
    assert_eq!(undeclared.is_error, Some(true));
    assert!(text(&undeclared).contains("/.well-known/ic-architecture"));
    println!("PASS MCP discovery: generated manifest, published Candid/API doc, undeclared-canister rejection");

    let mcp = Mcp {
        client: &client,
        cloud,
        origin,
        identity_origin,
        did: &did,
    };
    let bob_root: Reply<Entry> = mcp.call("drive_initialize", "()", false).await;
    assert_eq!(bob_root.unwrap().object.owner, bob_id);
    let alice_root: Reply<Entry> = direct(&alice, cloud, &did, "drive_initialize", "()").await;
    let alice_root = alice_root.unwrap().object.id;
    let folder: Reply<Entry> = direct(&alice, cloud, &did, "drive_create_folder", &format!(
        r#"(record {{ parent = {alice_root}; name = "Shared"; request_id = "mcp-alice-folder-1" }})"#)).await;
    let folder = folder.unwrap().object.id;
    let original: Reply<Entry> = direct(
        &alice,
        cloud,
        &did,
        "drive_create_file",
        &file(folder, "Original.txt", "mcp-original-file-1"),
    )
    .await;
    let original = original.unwrap().object.id;
    let denied: Reply<Entry> = mcp.call("drive_get", &format!("({original})"), false).await;
    assert!(matches!(denied, Err(Error::Inaccessible)));
    let capabilities =
        "record { read = true; create = true; edit = false; delete = false; propose = false }";
    let grant: Reply<Grant> = direct(
        &alice,
        cloud,
        &did,
        "drive_set_grant",
        &format!(
            r#"(record {{
        request_id = "mcp-alice-grant-1"; id = null; expected_version = null; root = {folder};
        subject = variant {{ User = principal "{bob_id}" }}; capabilities = {capabilities};
        expires_at = null; active = true }})"#
        ),
    )
    .await;
    let grant = grant.unwrap();
    let create = file(folder, "From-MCP.txt", "mcp-create-retry-1");
    let created: Reply<Entry> = mcp.call("drive_create_file", &create, false).await;
    let created = created.unwrap();
    assert_eq!(created.object.owner, alice_id);
    let rev = created.latest.unwrap();
    assert_eq!(rev.author, bob_id);
    assert_eq!(rev.bytes, 19);
    let retry: Reply<Entry> = mcp.call("drive_create_file", &create, false).await;
    assert_eq!(retry.unwrap().object.id, created.object.id);
    let read: Reply<Vec<u8>> = mcp
        .call(
            "drive_current_chunk",
            &format!("({}, {}, 0)", created.object.id, rev.id),
            true,
        )
        .await;
    assert_eq!(read.unwrap(), b"Created through MCP");
    let from_owner: Reply<Entry> = direct(
        &alice,
        cloud,
        &did,
        "drive_get",
        &format!("({})", created.object.id),
    )
    .await;
    assert_eq!(from_owner.unwrap().object.name, "From-MCP.txt");
    println!("PASS MCP create/read: shared folder, owner sees file, account attribution, byte equality, idempotent retry");

    for (action, request) in [
        ("Rename(\"changed\")", "mcp-denied-edit-1"),
        ("Trash", "mcp-denied-delete-1"),
    ] {
        let change = if action == "Trash" {
            "variant { Trash }"
        } else {
            "variant { Rename = \"changed\" }"
        };
        let denied: Reply<Receipt> = mcp
            .call(
                "drive_organize",
                &format!(
                    r#"(record {{ object = {};
            expected_version = {}; request_id = "{request}"; change = {change} }})"#,
                    created.object.id, created.object.version
                ),
                false,
            )
            .await;
        assert!(matches!(denied, Err(Error::Inaccessible)));
    }
    let denied: Reply<Entry> = mcp
        .call(
            "drive_create_file",
            &file(alice_root, "Private.txt", "mcp-outside-scope-1"),
            false,
        )
        .await;
    assert!(matches!(denied, Err(Error::Inaccessible)));
    let revoked: Reply<Grant> = direct(
        &alice,
        cloud,
        &did,
        "drive_set_grant",
        &format!(
            r#"(record {{
        request_id = "mcp-alice-revoke-1"; id = opt {}; expected_version = opt {}; root = {folder};
        subject = variant {{ User = principal "{bob_id}" }}; capabilities = {capabilities};
        expires_at = null; active = false }})"#,
            grant.id, grant.version
        ),
    )
    .await;
    revoked.unwrap();
    let denied: Reply<Entry> = mcp.call("drive_create_file", &create, false).await;
    assert!(matches!(denied, Err(Error::Inaccessible)));
    let denied: Reply<Vec<u8>> = mcp
        .call(
            "drive_current_chunk",
            &format!("({}, {}, 0)", created.object.id, rev.id),
            true,
        )
        .await;
    assert!(matches!(denied, Err(Error::Inaccessible)));
    println!("PASS MCP authorization: Edit/Delete/outside-share denied; revocation blocks reads and creation retries");
    client.cancel().await.unwrap();
    server.cancel().await.unwrap();
    std::fs::remove_dir_all(fixture).unwrap();
}
