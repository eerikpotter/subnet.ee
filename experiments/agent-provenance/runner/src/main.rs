//! Executes real signed HTTP ingress against local II and a probe canister.
//! PocketIC installs canisters; it never injects callers into the tested calls.
mod cloud;
mod ii;
use candid::{decode_args, decode_one, encode_args, utils::ArgumentEncoder, CandidType, Principal};
use ic_agent::{
    identity::{BasicIdentity, DelegatedIdentity, Delegation, SignedDelegation},
    Agent, Identity,
};
use serde::{de::DeserializeOwned, Deserialize};
use sha2::{Digest, Sha256};
use std::{sync::Arc, time::SystemTime};

const ORIGIN: &str = "https://cloud.example";
const MCP_ORIGIN: &str = "https://mcp.example";
const II_SHA256: &str = "a0e242ae7c802b688cfe18977878a56701eb92576aa75fef27da7a001b5a430c";

#[derive(CandidType, Deserialize, Debug, PartialEq)]
struct Observation {
    caller: Principal,
    info_signer: Option<Principal>,
    info: Vec<u8>,
}

fn client(url: &str, root: &[u8], identity: impl Identity + 'static) -> Agent {
    let agent = Agent::builder()
        .with_url(url)
        .with_identity(identity)
        .build()
        .unwrap();
    // Trusted fixture key, never fetched from a network status endpoint.
    agent.set_root_key(root.to_vec());
    agent
}

async fn update<T: CandidType + DeserializeOwned>(
    agent: &Agent,
    id: Principal,
    method: &str,
    args: impl ArgumentEncoder,
) -> T {
    let bytes = agent
        .update(&id, method)
        .with_arg(encode_args(args).unwrap())
        .call_and_wait()
        .await
        .unwrap_or_else(|e| panic!("{method}: {e}"));
    decode_one(&bytes).unwrap_or_else(|e| panic!("{method} reply: {e}"))
}
async fn query<T: CandidType + DeserializeOwned>(
    agent: &Agent,
    id: Principal,
    method: &str,
    args: impl ArgumentEncoder,
) -> T {
    let bytes = agent
        .query(&id, method)
        .with_arg(encode_args(args).unwrap())
        .call()
        .await
        .unwrap_or_else(|e| panic!("{method}: {e}"));
    decode_one(&bytes).unwrap_or_else(|e| panic!("{method} reply: {e}"))
}

fn delegated(
    root: &[u8],
    user_key: Vec<u8>,
    session: BasicIdentity,
    signed: ii::Signed,
) -> DelegatedIdentity {
    let d = signed.delegation;
    let permissions = match d.permissions.as_deref() {
        None => None,
        Some("queries") => Some(ic_agent::identity::DelegationPermissions::Queries),
        Some(p) => panic!("unexpected permissions: {p}"),
    };
    DelegatedIdentity::new_with_root_key(
        user_key,
        Box::new(session),
        vec![SignedDelegation {
            delegation: Delegation {
                pubkey: d.pubkey,
                expiration: d.expiration,
                targets: d.targets,
                permissions,
            },
            signature: signed.signature,
        }],
        root,
    )
    .expect("cryptographically verify II delegation against fixture root")
}

async fn mcp_session(
    device: &Agent,
    ii_id: Principal,
    anchor: u64,
    root: &[u8],
    url: &str,
    permission: ii::Permissions,
    seed: u8,
) -> Agent {
    let browser_registration_key = BasicIdentity::from_raw_key(&[seed; 32]);
    let registration_pubkey = browser_registration_key.public_key().unwrap();
    let prepared: Result<ii::PreparedRegistration, String> = update(
        device,
        ii_id,
        "prepare_mcp_registration_delegation",
        (
            anchor,
            &registration_pubkey,
            Some(permission),
            Some(3_600_000_000_000_u64),
        ),
    )
    .await;
    let prepared = prepared.unwrap();
    assert_eq!(prepared.trusted_url, format!("{MCP_ORIGIN}/mcp"));
    let signed: Result<ii::Signed, String> = query(
        device,
        ii_id,
        "get_mcp_registration_delegation",
        (
            anchor,
            registration_pubkey,
            &prepared.user_key,
            prepared.expiration,
        ),
    )
    .await;
    let registration = client(
        url,
        root,
        delegated(
            root,
            prepared.user_key,
            browser_registration_key,
            signed.unwrap(),
        ),
    );
    let server_key = BasicIdentity::from_raw_key(&[seed + 1; 32]);
    let registered: Result<ii::Registration, String> = update(
        &registration,
        ii_id,
        "mcp_register_v2",
        (server_key.public_key().unwrap(),),
    )
    .await;
    assert!(registered.unwrap().expiration > 0);
    client(url, root, server_key)
}

async fn mcp_app(
    session: &Agent,
    ii_id: Principal,
    root: &[u8],
    url: &str,
    seed: u8,
    read_only: bool,
) -> (Agent, Principal) {
    let key = BasicIdentity::from_raw_key(&[seed; 32]);
    let prepared: Result<ii::Prepared, ii::Error> = update(
        session,
        ii_id,
        "mcp_prepare_delegation",
        (ORIGIN, None::<u64>, key.public_key().unwrap(), None::<u64>),
    )
    .await;
    let prepared = prepared.unwrap();
    let principal = Principal::self_authenticating(&prepared.user_key);
    let signed: Result<ii::Signed, ii::Error> = query(
        session,
        ii_id,
        "mcp_get_delegation",
        (
            ORIGIN,
            prepared.account_number,
            key.public_key().unwrap(),
            prepared.expiration,
        ),
    )
    .await;
    let signed = signed.unwrap();
    assert_eq!(
        signed.delegation.permissions.as_deref(),
        read_only.then_some("queries")
    );
    (
        client(url, root, delegated(root, prepared.user_key, key, signed)),
        principal,
    )
}

#[tokio::main]
async fn main() {
    let ii_path =
        std::env::var("II_WASM").expect("II_WASM must point to release-2026-10-13 backend.wasm.gz");
    let probe_path =
        std::env::var("PROBE_WASM").expect("PROBE_WASM must point to built probe Wasm");
    let ii_wasm = std::fs::read(ii_path).unwrap();
    assert_eq!(
        hex::encode(Sha256::digest(&ii_wasm)),
        II_SHA256,
        "II release artifact changed"
    );
    let probe_wasm = std::fs::read(probe_path).unwrap();
    let server = std::env::var("POCKET_IC_BIN").expect("POCKET_IC_BIN must point to a PocketIC 16 server; this experiment never downloads executables");
    let mut pic = pocket_ic::PocketIcBuilder::new()
        .with_server_binary(server.into())
        .with_nns_subnet()
        .with_application_subnet()
        .build_async()
        .await;
    pic.set_time(SystemTime::now().into()).await;
    pic.tick().await;
    let ii_id = pic.create_canister().await;
    pic.add_cycles(ii_id, 2_000_000_000_000).await;
    pic.install_canister(
        ii_id,
        ii_wasm,
        encode_args((Some(ii::Init {
            captcha_config: Some(ii::CaptchaConfig {
                max_unsolved_captchas: 500,
                captcha_trigger: ii::CaptchaTrigger::Static(
                    ii::StaticCaptchaTrigger::CaptchaDisabled,
                ),
            }),
        }),))
        .unwrap(),
        None,
    )
    .await;
    let root = pic.root_key().await.unwrap();
    let url = pic.make_live(None).await.to_string();

    let device_key: Arc<dyn Identity> = Arc::new(BasicIdentity::from_raw_key(&[1; 32]));
    let device = client(&url, &root, device_key.clone());
    let challenge: ii::Challenge = update(&device, ii_id, "create_challenge", ()).await;
    let registered: ii::RegisterResponse = update(
        &device,
        ii_id,
        "register",
        (
            ii::Device {
                pubkey: device_key.public_key().unwrap(),
                alias: "disposable provenance probe".into(),
                purpose: ii::Purpose::Authentication,
                key_type: ii::KeyType::Unknown,
                protection: ii::Protection::Unprotected,
            },
            ii::ChallengeResult {
                key: challenge.challenge_key,
                chars: "a".into(),
            },
            None::<Principal>,
        ),
    )
    .await;
    let anchor = match registered {
        ii::RegisterResponse::Registered { user_number } => user_number,
        other => panic!("registration: {other:?}"),
    };
    let configured: Result<(), String> = update(
        &device,
        ii_id,
        "mcp_set_config",
        (
            anchor,
            ii::McpConfig {
                enabled: true,
                url: Some(format!("{MCP_ORIGIN}/mcp")),
            },
        ),
    )
    .await;
    configured.unwrap();

    // Normal app/browser delegation through II's authentication protocol.
    let browser_key = BasicIdentity::from_raw_key(&[2; 32]);
    let bytes = device
        .update(&ii_id, "prepare_delegation")
        .with_arg(
            encode_args((
                anchor,
                ORIGIN,
                browser_key.public_key().unwrap(),
                None::<u64>,
            ))
            .unwrap(),
        )
        .call_and_wait()
        .await
        .unwrap();
    let (user_key, expiration): (Vec<u8>, u64) = decode_args(&bytes).unwrap();
    let human = Principal::self_authenticating(&user_key);
    let signed: ii::GetDelegationResponse = query(
        &device,
        ii_id,
        "get_delegation",
        (
            anchor,
            ORIGIN,
            browser_key.public_key().unwrap(),
            expiration,
        ),
    )
    .await;
    let signed = match signed {
        ii::GetDelegationResponse::Signed(s) => s,
        _ => panic!("missing browser delegation"),
    };
    let browser = client(&url, &root, delegated(&root, user_key, browser_key, signed));
    let probe = pic.create_canister().await;
    pic.add_cycles(probe, 2_000_000_000_000).await;
    pic.install_canister(probe, probe_wasm, encode_args((human,)).unwrap(), None)
        .await;
    let expected = Observation {
        caller: human,
        info_signer: None,
        info: vec![],
    };
    let browser_observed: Observation = update(&browser, probe, "observe_update", ()).await;
    assert_eq!(browser_observed, expected);

    let session = mcp_session(&device, ii_id, anchor, &root, &url, ii::Permissions::All, 3).await;
    let (mcp, mcp_principal) = mcp_app(&session, ii_id, &root, &url, 5, false).await;
    assert_eq!(mcp_principal, human);
    let mcp_observed: Observation = update(&mcp, probe, "observe_update", ()).await;
    assert_eq!(mcp_observed, browser_observed);
    let result: Result<u64, String> = update(&mcp, probe, "edit", ()).await;
    assert_eq!(
        result,
        Ok(1),
        "ordinary human endpoint accepts the full MCP delegation"
    );
    println!("PASS browser and full MCP delegation expose identical caller and empty caller-info; MCP can use the human edit endpoint");

    if let Ok(path) = std::env::var("CLOUD_WASM") {
        cloud::verify(&pic, &url, &root, &browser, &mcp, human, &path).await;
    }

    let readonly_session = mcp_session(
        &device,
        ii_id,
        anchor,
        &root,
        &url,
        ii::Permissions::Queries,
        6,
    )
    .await;
    let (readonly, principal) = mcp_app(&readonly_session, ii_id, &root, &url, 8, true).await;
    assert_eq!(principal, human);
    let observed: Observation = query(&readonly, probe, "observe_query", ()).await;
    assert_eq!(observed, expected);
    let denied = readonly
        .update(&probe, "edit")
        .with_arg(encode_args(()).unwrap())
        .call_and_wait()
        .await;
    let error = denied
        .expect_err("query-only delegation must not submit updates")
        .to_string();
    assert!(
        error.to_lowercase().contains("permission") || error.to_lowercase().contains("queries"),
        "unexpected denial: {error}"
    );
    println!("PASS query-only MCP delegation is enforced by the replica, while query caller-info remains empty");

    // Separate key demonstrates the canister-level boundary we need. This is
    // not an imcp2 connection option or a production credential exchange.
    let scoped_key = BasicIdentity::from_raw_key(&[9; 32]);
    let scoped_principal = scoped_key.sender().unwrap();
    let scoped = client(&url, &root, scoped_key);
    let granted: Result<(), String> =
        update(&browser, probe, "grant_agent", (Some(scoped_principal),)).await;
    granted.unwrap();
    let read: Result<(u64, Principal, Option<Principal>), String> =
        query(&scoped, probe, "read", ()).await;
    assert_eq!(read, Ok((1, scoped_principal, Some(human))));
    let edit: Result<u64, String> = update(&scoped, probe, "edit", ()).await;
    assert_eq!(edit, Err("denied".into()));
    let escalate: Result<(), String> =
        update(&scoped, probe, "grant_agent", (None::<Principal>,)).await;
    assert_eq!(escalate, Err("denied".into()));
    let revoked: Result<(), String> =
        update(&browser, probe, "grant_agent", (None::<Principal>,)).await;
    revoked.unwrap();
    let read: Result<(u64, Principal, Option<Principal>), String> =
        query(&scoped, probe, "read", ()).await;
    assert_eq!(read, Err("denied".into()));
    println!("PASS distinct agent principal supports attribution, lower rights, denied human endpoint, denied escalation, immediate app revocation");

    // Revoking II blocks new issuance, but an already issued full delegation
    // remains usable until expiry. App-level revocation needs a visible binding.
    let disabled: Result<(), String> = update(
        &device,
        ii_id,
        "mcp_set_config",
        (
            anchor,
            ii::McpConfig {
                enabled: false,
                url: None,
            },
        ),
    )
    .await;
    disabled.unwrap();
    let mint: Result<ii::Prepared, ii::Error> = update(
        &readonly_session,
        ii_id,
        "mcp_prepare_delegation",
        (ORIGIN, None::<u64>, vec![1_u8], None::<u64>),
    )
    .await;
    assert!(matches!(mint, Err(ii::Error::Unauthorized(_))));
    let edit: Result<u64, String> = update(&mcp, probe, "edit", ()).await;
    assert_eq!(edit, Ok(2));
    println!("PASS II revocation blocks new issuance; already issued delegation remains valid before expiry");
    pic.stop_live().await;
}
