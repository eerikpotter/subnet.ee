//! Small browser facade: all identity and file operations use the same Rust SDK as Craft.
#![forbid(unsafe_code)]
#![deny(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
use futures::channel::oneshot;
use sovereign_cloud::browser::{self, BrowserClient, BrowserSession};
use std::{
    cell::{Cell, RefCell},
    rc::Rc,
    sync::{Arc, Mutex},
};
use wasm_bindgen::prelude::*;

fn main() {}
fn js_error(error: impl ToString) -> JsValue {
    js_sys::Error::new(&error.to_string()).into()
}
fn json(value: serde_json::Value) -> Result<JsValue, JsValue> {
    js_sys::JSON::parse(&value.to_string())
}
fn id(value: &str) -> Result<u64, JsValue> {
    value
        .parse()
        .map_err(|_| js_error("Invalid file reference"))
}

#[wasm_bindgen]
pub struct CloudPortal {
    session: Rc<RefCell<Option<BrowserSession>>>,
    signed_in: Cell<bool>,
}
#[wasm_bindgen]
impl CloudPortal {
    #[wasm_bindgen(constructor)]
    pub fn new() -> Self {
        Self {
            session: Rc::new(RefCell::new(None)),
            signed_in: Cell::new(false),
        }
    }

    /// Rehydrate from the shared adapter, never from independently interpreted JS credentials.
    pub async fn restore(&self) -> Result<JsValue, JsValue> {
        let session = BrowserSession::load().await.map_err(js_error)?;
        let signed_in = session.is_authenticated();
        let state = if signed_in {
            let account = BrowserClient::new(&session)
                .map_err(js_error)?
                .account()
                .await
                .map_err(js_error)?;
            if !session.is_authenticated() {
                return Err(js_error("Session changed during account loading"));
            }
            serde_json::json!({"authenticated":true, "principal":account.account.owner.to_text(), "accountId":account.account.id.to_string(), "spaceId":account.account.personal_space.to_string(), "usedBytes":account.used_bytes, "quotaBytes":account.quota_bytes})
        } else {
            serde_json::json!({"authenticated":false})
        };
        self.signed_in.set(signed_in);
        *self.session.borrow_mut() = Some(session);
        json(state)
    }
    pub fn needs_refresh(&self) -> bool {
        self.session
            .borrow()
            .as_ref()
            .is_none_or(|s| !s.is_current() || s.is_authenticated() != self.signed_in.get())
    }
    /// Starts the popup synchronously inside the browser's click gesture.
    pub fn sign_in(&self) -> Result<js_sys::Promise, JsValue> {
        let session = self
            .session
            .borrow()
            .clone()
            .ok_or_else(|| js_error("Sign-in is still loading"))?;
        let (sender, receiver) = oneshot::channel::<Result<(), String>>();
        let success = Arc::new(Mutex::new(Some(sender)));
        let failure = success.clone();
        browser::login(
            &session,
            move || {
                if let Some(sender) = success
                    .lock()
                    .unwrap_or_else(std::sync::PoisonError::into_inner)
                    .take()
                {
                    let _ = sender.send(Ok(()));
                }
            },
            move |e| {
                if let Some(sender) = failure
                    .lock()
                    .unwrap_or_else(std::sync::PoisonError::into_inner)
                    .take()
                {
                    let _ = sender.send(Err(e));
                }
            },
        )
        .map_err(js_error)?;
        Ok(wasm_bindgen_futures::future_to_promise(async move {
            receiver.await.map_err(js_error)?.map_err(js_error)?;
            Ok(JsValue::UNDEFINED)
        }))
    }
    pub async fn sign_out(&self) -> Result<(), JsValue> {
        let session = self.session.borrow().clone();
        if let Some(session) = session {
            session.logout().await.map_err(js_error)?;
        }
        self.signed_in.set(false);
        Ok(())
    }
    pub async fn files(&self, space: String, after: Option<String>) -> Result<JsValue, JsValue> {
        let session = self
            .session
            .borrow()
            .clone()
            .ok_or_else(|| js_error("Sign in first"))?;
        let page = BrowserClient::new(&session)
            .map_err(js_error)?
            .files(id(&space)?, after.map(|s| id(&s)).transpose()?, 20)
            .await
            .map_err(js_error)?;
        if !session.is_authenticated() {
            return Err(js_error("Session changed during file loading"));
        }
        let files: Vec<_> = page
            .files
            .into_iter()
            .map(|file| {
                serde_json::json!({
                    "id":file.id.to_string(), "spaceId":file.space_id.to_string(), "name":file.name,
                    "formatId":file.kind.format_id, "formatVersion":file.kind.format_version,
                    "bytes":file.latest.bytes, "updatedMs":file.latest.created_at / 1_000_000,
                    "revision":file.latest.id.to_string(), "versions":file.version_count
                })
            })
            .collect();
        json(
            serde_json::json!({"files":files,"nextCursor":page.next_cursor.map(|id| id.to_string())}),
        )
    }
}
impl Default for CloudPortal {
    fn default() -> Self {
        Self::new()
    }
}
