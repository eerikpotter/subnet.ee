//! Origin-local recovery snapshots. Separate metadata and bytes avoid reading
//! every document into memory when showing the recovery list.
use futures::channel::oneshot;
use serde::{Deserialize, Serialize};
use sovereign_cloud::CloudResult;
use std::{cell::RefCell, rc::Rc};
use wasm_bindgen::{JsCast, JsValue, closure::Closure};
use web_sys::{IdbDatabase, IdbKeyRange, IdbRequest, IdbTransaction, IdbTransactionMode};
#[derive(Clone, Debug, Serialize, Deserialize)]
pub(super) struct Header {
    pub key: String,
    pub name: String,
    pub revision: u64,
    pub bytes: u64,
    pub at: f64,
}
struct Database(IdbDatabase);
impl Drop for Database {
    fn drop(&mut self) {
        self.0.close();
    }
}
fn error(_: JsValue) -> String {
    "Local draft storage is unavailable or full".into()
}
async fn result(request: IdbRequest) -> CloudResult<JsValue> {
    let (tx, rx) = oneshot::channel();
    let tx = Rc::new(RefCell::new(Some(tx)));
    let ok_tx = tx.clone();
    let req = request.clone();
    let success = Closure::<dyn FnMut(web_sys::Event)>::new(move |_| {
        if let Some(tx) = ok_tx.borrow_mut().take() {
            let _ = tx.send(req.result().map_err(error));
        }
    });
    let fail_tx = tx.clone();
    let fail = Closure::<dyn FnMut(web_sys::Event)>::new(move |_| {
        if let Some(tx) = fail_tx.borrow_mut().take() {
            let _ = tx.send(Err("Local draft request failed".into()));
        }
    });
    request.set_onsuccess(Some(success.as_ref().unchecked_ref()));
    request.set_onerror(Some(fail.as_ref().unchecked_ref()));
    let result = rx.await.map_err(|_| "Local draft request interrupted")?;
    request.set_onsuccess(None);
    request.set_onerror(None);
    result
}
async fn completed(transaction: IdbTransaction) -> CloudResult<()> {
    let (tx, rx) = oneshot::channel();
    let tx = Rc::new(RefCell::new(Some(tx)));
    let ok_tx = tx.clone();
    let success = Closure::<dyn FnMut(web_sys::Event)>::new(move |_| {
        if let Some(tx) = ok_tx.borrow_mut().take() {
            let _ = tx.send(Ok(()));
        }
    });
    let fail_tx = tx.clone();
    let fail = Closure::<dyn FnMut(web_sys::Event)>::new(move |_| {
        if let Some(tx) = fail_tx.borrow_mut().take() {
            let _ = tx.send(Err("Local draft storage is unavailable or full".into()));
        }
    });
    transaction.set_oncomplete(Some(success.as_ref().unchecked_ref()));
    transaction.set_onabort(Some(fail.as_ref().unchecked_ref()));
    transaction.set_onerror(Some(fail.as_ref().unchecked_ref()));
    let result = rx.await.map_err(|_| "Local draft transaction interrupted")?;
    transaction.set_oncomplete(None);
    transaction.set_onabort(None);
    transaction.set_onerror(None);
    result
}
async fn open() -> CloudResult<Database> {
    let factory = web_sys::window().ok_or("Browser unavailable")?.indexed_db().map_err(error)?.ok_or("Local draft storage unavailable")?;
    let request = factory.open_with_u32("subnet.cloud.drafts", 1).map_err(error)?;
    let req = request.clone();
    let upgrade = Closure::<dyn FnMut(web_sys::Event)>::new(move |_| {
        if let Ok(db) = req.result().and_then(|v| v.dyn_into::<IdbDatabase>()) {
            for name in ["headers", "content"] {
                if !db.object_store_names().contains(name) {
                    let _ = db.create_object_store(name);
                }
            }
        }
    });
    request.set_onupgradeneeded(Some(upgrade.as_ref().unchecked_ref()));
    let value = result(request.clone().unchecked_into()).await;
    request.set_onupgradeneeded(None);
    Ok(Database(value?.dyn_into().map_err(error)?))
}
fn range(prefix: &str) -> CloudResult<IdbKeyRange> {
    IdbKeyRange::bound(&JsValue::from_str(prefix), &JsValue::from_str(&format!("{prefix}\u{ffff}"))).map_err(error)
}
fn headers(value: JsValue, prefix: &str) -> CloudResult<Vec<Header>> {
    js_sys::Array::from(&value)
        .iter()
        .map(|v| {
            let header: Header = serde_json::from_str(&v.as_string().ok_or("Invalid draft header")?).map_err(|_| "Invalid draft header")?;
            if !header.key.starts_with(prefix) || header.bytes > sovereign_cloud::protocol::MAX_DOCUMENT_BYTES {
                return Err("Invalid draft header".into());
            }
            Ok(header)
        })
        .collect()
}
pub(super) async fn list(prefix: &str) -> CloudResult<Vec<Header>> {
    let db = open().await?;
    let tx = db.0.transaction_with_str_and_mode("headers", IdbTransactionMode::Readonly).map_err(error)?;
    let request = tx.object_store("headers").map_err(error)?.get_all_with_key_and_limit(&JsValue::from(range(prefix)?), 21).map_err(error)?;
    let mut entries = headers(result(request).await?, prefix)?;
    entries.sort_by(|a, b| b.at.total_cmp(&a.at));
    Ok(entries)
}
fn transaction(db: &Database) -> CloudResult<IdbTransaction> {
    let stores = js_sys::Array::new();
    stores.push(&JsValue::from_str("headers"));
    stores.push(&JsValue::from_str("content"));
    db.0.transaction_with_str_sequence_and_mode(&stores, IdbTransactionMode::Readwrite).map_err(error)
}
pub(super) async fn write(prefix: &str, header: &Header, bytes: &[u8]) -> CloudResult<()> {
    if !header.key.starts_with(prefix) || bytes.len() as u64 != header.bytes || header.bytes > sovereign_cloud::protocol::MAX_DOCUMENT_BYTES {
        return Err("Invalid local draft".into());
    }
    let db = open().await?;
    let tx = transaction(&db)?;
    let store = tx.object_store("headers").map_err(error)?;
    let all = headers(result(store.get_all_with_key_and_limit(&JsValue::from(range(prefix)?), 21).map_err(error)?).await?, prefix)?;
    let others = all.iter().filter(|h| h.key != header.key).collect::<Vec<_>>();
    if others.len() >= 20 || others.iter().map(|h| h.bytes).sum::<u64>() + header.bytes > 128 * 1024 * 1024 {
        let _ = tx.abort();
        return Err("Local recovery storage is full. Remove old drafts from the Local drafts panel.".into());
    }
    let key = JsValue::from_str(&header.key);
    let data = js_sys::Uint8Array::from(bytes);
    let json = serde_json::to_string(header).map_err(|_| "Invalid local draft")?;
    store.put_with_key(&JsValue::from_str(&json), &key).map_err(error)?;
    tx.object_store("content").map_err(error)?.put_with_key(&data, &key).map_err(error)?;
    completed(tx).await
}
pub(super) async fn read(prefix: &str, header: &Header) -> CloudResult<Vec<u8>> {
    if !header.key.starts_with(prefix) {
        return Err("Draft belongs to another account".into());
    }
    let db = open().await?;
    let stores = js_sys::Array::new();
    stores.push(&JsValue::from_str("headers"));
    stores.push(&JsValue::from_str("content"));
    let tx = db.0.transaction_with_str_sequence_and_mode(&stores, IdbTransactionMode::Readonly).map_err(error)?;
    // Read metadata and bytes in one transaction so another tab cannot replace one half.
    let current_request = tx.object_store("headers").map_err(error)?.get(&JsValue::from_str(&header.key)).map_err(error)?;
    let bytes_request = tx.object_store("content").map_err(error)?.get(&JsValue::from_str(&header.key)).map_err(error)?;
    let (current, value) = futures::join!(result(current_request), result(bytes_request));
    let current: Header = serde_json::from_str(&current?.as_string().ok_or("Draft is no longer available")?).map_err(|_| "Invalid draft header")?;
    if current.at != header.at || current.revision != header.revision {
        return Err("Draft changed. Refresh the recovery list.".into());
    }
    let value = value?;
    if value.is_undefined() {
        return Err("Draft is no longer available".into());
    }
    let data = js_sys::Uint8Array::new(&value);
    if data.length() as u64 != header.bytes {
        return Err("Draft changed. Refresh the recovery list.".into());
    }
    Ok(data.to_vec())
}
pub(super) async fn remove(prefix: &str, key: &str, expected: f64) -> CloudResult<()> {
    if !key.starts_with(prefix) {
        return Err("Draft belongs to another account".into());
    }
    let db = open().await?;
    let tx = transaction(&db)?;
    let key = JsValue::from_str(key);
    let value = result(tx.object_store("headers").map_err(error)?.get(&key).map_err(error)?).await?;
    if value.is_undefined() {
        return Ok(());
    }
    let header: Header = serde_json::from_str(&value.as_string().ok_or("Invalid draft header")?).map_err(|_| "Invalid draft header")?;
    if header.at != expected {
        return Err("Draft changed. Refresh the recovery list.".into());
    }
    for name in ["headers", "content"] {
        tx.object_store(name).map_err(error)?.delete(&key).map_err(error)?;
    }
    completed(tx).await
}
