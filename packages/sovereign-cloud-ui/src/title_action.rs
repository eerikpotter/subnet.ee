//! Shell intent only; the controller rechecks document and account authority.
use crate::{MenuHandle, Request};
use wasm_bindgen::{JsCast, closure::Closure};
const EVENT: &str = "subnet-rename-document";
pub(crate) struct TitleAction {
    root: web_sys::Element,
    listener: Closure<dyn FnMut(web_sys::Event)>,
}
impl TitleAction {
    pub fn new(menu: MenuHandle) -> Option<Self> {
        let root = web_sys::window()?.document()?.document_element()?;
        let listener = Closure::wrap(Box::new(move |_: web_sys::Event| menu.request(Request::Rename)) as Box<dyn FnMut(web_sys::Event)>);
        root.add_event_listener_with_callback(EVENT, listener.as_ref().unchecked_ref()).ok()?;
        Some(Self { root, listener })
    }
}
impl Drop for TitleAction {
    fn drop(&mut self) {
        let _ = self.root.remove_event_listener_with_callback(EVENT, self.listener.as_ref().unchecked_ref());
    }
}
