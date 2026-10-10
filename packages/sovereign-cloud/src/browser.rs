//! Browser identity adapter. Never learns a trust key from an unauthenticated replica request.
use crate::{CloudResult, FileClient, Transport, config::ServiceEndpoint};
use ic_agent::Agent;
pub use ic_auth_client::AuthClient;
use ic_auth_client::AuthClientLoginOptions;
use sha2::{Digest, Sha256};
use wasm_bindgen::JsCast;

// Keep the established Rust AuthClient keys so existing PhotoCraft sessions survive.
// Both the portal's Wasm facade and Craft hosts use this exact adapter.
pub fn session_stamp() -> CloudResult<String> {
    let store = storage()?;
    let mut hash = Sha256::new();
    for name in ["ic-identity", "ic-delegation"] {
        let value = store.get_item(name).map_err(|_| "Cannot read identity storage")?.unwrap_or_default();
        hash.update(value.len().to_le_bytes());
        hash.update(value.as_bytes());
    }
    Ok(hex::encode(hash.finalize()))
}

fn storage() -> CloudResult<web_sys::Storage> {
    web_sys::window().ok_or("Browser unavailable")?.local_storage().map_err(|_| "Identity storage unavailable")?.ok_or("Identity storage unavailable".into())
}

pub fn forget_session() -> CloudResult<()> {
    let store = storage()?;
    for name in ["ic-delegation", "ic-identity", "ic-iv"] {
        store.remove_item(name).map_err(|_| "Could not clear identity storage")?;
    }
    Ok(())
}

pub fn notify_files_changed() {
    if let Ok(store) = storage() {
        let _ = store.set_item("subnet.files.changed", &js_sys::Date::now().to_string());
    }
}

/// Snapshot of an origin-wide session. Every transport call rejects a stale snapshot.
/// Clearing storage does not revoke an already issued delegation at the replica.
#[derive(Clone)]
pub struct BrowserSession {
    auth: AuthClient,
    stamp: String,
}
impl BrowserSession {
    pub async fn load() -> CloudResult<Self> {
        let auth = create_auth_client().await?;
        Ok(Self { auth, stamp: session_stamp()? })
    }
    pub fn stamp(&self) -> &str {
        &self.stamp
    }
    pub fn is_current(&self) -> bool {
        session_stamp().is_ok_and(|s| s == self.stamp)
    }
    pub fn is_authenticated(&self) -> bool {
        self.is_current() && self.auth.is_authenticated()
    }
    pub fn principal(&self) -> CloudResult<candid::Principal> {
        if !self.is_authenticated() {
            return Err("Session changed or expired. Sign in again.".into());
        }
        self.auth.principal().map_err(|e| e.to_string())
    }
    pub async fn logout(&self) -> CloudResult<()> {
        if !self.is_current() {
            return Err("Session changed. Refresh your account first.".into());
        }
        self.auth.logout(None).await;
        Ok(())
    }
}

pub fn endpoint() -> CloudResult<ServiceEndpoint> {
    let window = web_sys::window().ok_or("Browser unavailable")?;
    let html = window.document().ok_or("Browser document unavailable")?.dyn_into::<web_sys::HtmlDocument>().map_err(|_| "HTML document unavailable")?;
    let cookies = html.cookie().map_err(|_| "Browser cookies unavailable")?;
    let origin = window.location().origin().map_err(|_| "Cannot read application origin")?;
    ServiceEndpoint::from_environment(&cookies, &origin)
}

pub async fn create_auth_client() -> CloudResult<AuthClient> {
    // The pinned upstream client panics on malformed persisted delegation JSON.
    // Recover a damaged/partial record before it reaches that parser.
    let store = storage()?;
    if let Some(chain) = store.get_item("ic-delegation").map_err(|_| "Cannot read identity storage")? {
        if chain.len() > 131_072 || serde_json::from_str::<ic_auth_client::delegation_chain::DelegationChain>(&chain).is_err() {
            forget_session()?;
        }
    }
    // An idle-page reload would destroy unsaved work in any editor.
    AuthClient::builder().disable_idle(true).build().await.map_err(|e| e.to_string())
}

pub fn login(session: &BrowserSession, success: impl Fn() + Send + 'static, error: impl Fn(String) + Send + 'static) -> CloudResult<()> {
    if !session.is_current() {
        return Err("Session changed. Try sign-in again.".into());
    }
    let config = endpoint()?;
    let mut options = AuthClientLoginOptions::builder()
        .identity_provider("https://id.ai")
        .max_time_to_live(8 * 60 * 60 * 1_000_000_000)
        .on_success(move |_| success())
        .on_error(move |e: Option<String>| error(e.unwrap_or_else(|| "Sign-in cancelled".into())))
        .build();
    options.derivation_origin = config.derivation_origin;
    session.auth.login_with_options(options);
    Ok(())
}

#[derive(Clone)]
pub struct CanisterTransport {
    agent: Agent,
    canister: candid::Principal,
    session: BrowserSession,
}
impl Transport for CanisterTransport {
    async fn update(&self, method: &str, args: Vec<u8>) -> CloudResult<Vec<u8>> {
        self.session.principal()?;
        self.agent.update(&self.canister, method).with_arg(args).call_and_wait().await.map_err(|e| e.to_string())
    }
    async fn query(&self, method: &str, args: Vec<u8>) -> CloudResult<Vec<u8>> {
        self.session.principal()?;
        self.agent.query(&self.canister, method).with_arg(args).call().await.map_err(|e| e.to_string())
    }
}

pub type BrowserClient = FileClient<CanisterTransport>;
impl BrowserClient {
    pub fn new(auth: &BrowserSession) -> CloudResult<Self> {
        Self::at_endpoint(auth, endpoint()?)
    }
    /// Future space/user-owned routing selects an endpoint here, without changing file operations.
    pub fn at_endpoint(auth: &BrowserSession, endpoint: ServiceEndpoint) -> CloudResult<Self> {
        if !auth.is_authenticated() {
            return Err("Sign in to Internet Identity first".into());
        }
        let agent = Agent::builder().with_url(endpoint.gateway).with_arc_identity(auth.auth.identity()).build().map_err(|e| e.to_string())?;
        agent.set_root_key(endpoint.root_key);
        Ok(Self::with_transport(CanisterTransport { agent, canister: endpoint.canister, session: auth.clone() }))
    }
}
