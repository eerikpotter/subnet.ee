//! Browser identity adapter. Never learns a trust key from an unauthenticated replica request.
use crate::{CloudResult, FileClient, Transport, config::ServiceEndpoint};
use ic_agent::Agent;
pub use ic_auth_client::AuthClient;
use ic_auth_client::AuthClientLoginOptions;
use wasm_bindgen::JsCast;

pub fn endpoint() -> CloudResult<ServiceEndpoint> {
    let window = web_sys::window().ok_or("Browser unavailable")?;
    let html = window.document().ok_or("Browser document unavailable")?.dyn_into::<web_sys::HtmlDocument>().map_err(|_| "HTML document unavailable")?;
    let cookies = html.cookie().map_err(|_| "Browser cookies unavailable")?;
    let origin = window.location().origin().map_err(|_| "Cannot read application origin")?;
    ServiceEndpoint::from_environment(&cookies, &origin)
}

pub async fn create_auth_client() -> CloudResult<AuthClient> {
    // An idle-page reload would destroy unsaved work in any editor.
    AuthClient::builder().disable_idle(true).build().await.map_err(|e| e.to_string())
}

pub fn login(auth: &AuthClient, success: impl Fn() + Send + 'static, error: impl Fn(String) + Send + 'static) -> CloudResult<()> {
    let config = endpoint()?;
    let mut options = AuthClientLoginOptions::builder()
        .identity_provider("https://id.ai")
        .max_time_to_live(8 * 60 * 60 * 1_000_000_000)
        .on_success(move |_| success())
        .on_error(move |e: Option<String>| error(e.unwrap_or_else(|| "Sign-in cancelled".into())))
        .build();
    options.derivation_origin = config.derivation_origin;
    auth.login_with_options(options);
    Ok(())
}

#[derive(Clone)]
pub struct CanisterTransport {
    agent: Agent,
    canister: candid::Principal,
}
impl Transport for CanisterTransport {
    async fn update(&self, method: &str, args: Vec<u8>) -> CloudResult<Vec<u8>> {
        self.agent.update(&self.canister, method).with_arg(args).call_and_wait().await.map_err(|e| e.to_string())
    }
    async fn query(&self, method: &str, args: Vec<u8>) -> CloudResult<Vec<u8>> {
        self.agent.query(&self.canister, method).with_arg(args).call().await.map_err(|e| e.to_string())
    }
}

pub type BrowserClient = FileClient<CanisterTransport>;
impl BrowserClient {
    pub fn new(auth: &AuthClient) -> CloudResult<Self> {
        Self::at_endpoint(auth, endpoint()?)
    }
    /// Future space/user-owned routing selects an endpoint here, without changing file operations.
    pub fn at_endpoint(auth: &AuthClient, endpoint: ServiceEndpoint) -> CloudResult<Self> {
        if !auth.is_authenticated() {
            return Err("Sign in to Internet Identity first".into());
        }
        let agent = Agent::builder().with_url(endpoint.gateway).with_arc_identity(auth.identity()).build().map_err(|e| e.to_string())?;
        agent.set_root_key(endpoint.root_key);
        Ok(Self::with_transport(CanisterTransport { agent, canister: endpoint.canister }))
    }
}
