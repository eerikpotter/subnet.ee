//! A same-origin portal picker. Messages select IDs only; the SDK reauthorizes them.
use crate::AppSpec;
use sovereign_cloud::CloudResult;
use std::{cell::Cell, rc::Rc};
use wasm_bindgen::{JsCast, JsValue, closure::Closure};

pub(super) enum Selection {
    Open { principal: String, space: u64, file: u64, revision: Option<u64> },
    Save { principal: String, parent: u64, name: String },
    Cancel,
}

pub(super) struct Picker {
    window: web_sys::Window,
    dialog: web_sys::HtmlDialogElement,
    close: web_sys::HtmlElement,
    previous_focus: Option<web_sys::HtmlElement>,
    message: Closure<dyn FnMut(web_sys::MessageEvent)>,
    cancel: Closure<dyn FnMut(web_sys::Event)>,
}
fn error(value: impl Into<JsValue>) -> String {
    value.into().as_string().unwrap_or_else(|| "Could not open the cloud picker".into())
}
fn string(value: &JsValue, key: &str) -> Option<String> {
    js_sys::Reflect::get(value, &key.into()).ok()?.as_string()
}
fn id(value: &JsValue, key: &str) -> Option<u64> {
    let text = string(value, key)?;
    if !text.bytes().all(|b| b.is_ascii_digit()) {
        return None;
    }
    text.parse().ok().filter(|id| *id > 0)
}

impl Picker {
    pub fn open(spec: AppSpec, save_name: Option<&str>, dark: bool, selected: impl Fn(Selection) + 'static) -> CloudResult<Self> {
        let window = web_sys::window().ok_or("Browser unavailable")?;
        let document = window.document().ok_or("Document unavailable")?;
        let previous_focus = document.active_element().and_then(|e| e.dyn_into().ok());
        let origin = window.location().origin().map_err(error)?;
        let channel = window.crypto().map_err(error)?.random_uuid();
        let params = web_sys::UrlSearchParams::new().map_err(error)?;
        let saving = save_name.is_some();
        params.append("picker", if saving { "save" } else { "open" });
        params.append("app", spec.id);
        params.append("channel", &channel);
        params.append("theme", if dark { "dark" } else { "light" });
        if let Some(name) = save_name {
            params.append("name", name);
        }
        let current = web_sys::UrlSearchParams::new_with_str(&window.location().search().map_err(error)?).map_err(error)?;
        let language = current.get("lang").unwrap_or_else(|| "en".into());
        let path = if language == "et" { "/pilveruum/" } else { "/en/pilveruum/" };
        let dialog = document.create_element("dialog").map_err(error)?.dyn_into::<web_sys::HtmlDialogElement>().map_err(error)?;
        dialog.set_attribute("aria-label", if language == "et" { "Pilveruum" } else { "Cloud drive" }).map_err(error)?;
        let background = if dark { "#24272d" } else { "#f4f7fb" };
        dialog.set_attribute("style", &format!("background:{background};padding:0;border:1px solid #64748b;border-radius:16px;width:min(1080px,calc(100vw - 24px));height:min(780px,calc(100dvh - 24px));max-width:none;max-height:none;overflow:hidden;box-shadow:0 24px 100px #0008")).map_err(error)?;
        let close = document.create_element("button").map_err(error)?.dyn_into::<web_sys::HtmlElement>().map_err(error)?;
        close.set_text_content(Some(if language == "et" { "Sulge ×" } else { "Close ×" }));
        close.set_attribute("type", "button").map_err(error)?;
        close.set_attribute("aria-label", if language == "et" { "Sulge" } else { "Close" }).map_err(error)?;
        close.set_attribute("style", "position:absolute;right:12px;top:8px;z-index:2;padding:6px 12px;border:1px solid #64748b;border-radius:8px;background:#f4f7fb;color:#0f2747;cursor:pointer;font:13px system-ui").map_err(error)?;
        let frame = document.create_element("iframe").map_err(error)?.dyn_into::<web_sys::HtmlIFrameElement>().map_err(error)?;
        frame.set_title("subnet.ee cloud drive");
        frame.set_attribute("style", "display:block;width:100%;height:calc(100% - 44px);margin-top:44px;border:0;background:#f4f7fb").map_err(error)?;
        if dark {
            close.set_attribute("style","position:absolute;right:12px;top:8px;z-index:2;padding:6px 12px;border:1px solid #454b56;border-radius:8px;background:#24272d;color:#f0f2f6;cursor:pointer;font:13px system-ui").map_err(error)?;
            frame.set_attribute("style", "display:block;width:100%;height:calc(100% - 44px);margin-top:44px;border:0;background:#24272d").map_err(error)?;
        }
        frame.set_src(&format!("{path}?{}", params.to_string()));
        dialog.append_child(&close).map_err(error)?;
        dialog.append_child(&frame).map_err(error)?;
        let done = Cell::new(false);
        let selected: Rc<dyn Fn(Selection)> = Rc::new(move |selection| {
            if !done.replace(true) {
                selected(selection);
            }
        });
        let receive = selected.clone();
        let message = Closure::new(move |event: web_sys::MessageEvent| {
            if event.origin() != origin {
                return;
            }
            let Some(source) = event.source() else { return };
            let Some(expected) = frame.content_window() else { return };
            if AsRef::<JsValue>::as_ref(&source) != AsRef::<JsValue>::as_ref(&expected) {
                return;
            }
            let data = event.data();
            if string(&data, "type").as_deref() != Some("subnet-cloud-selection") || string(&data, "channel").as_deref() != Some(channel.as_str()) {
                return;
            }
            if string(&data, "action").as_deref() == Some("cancel") {
                receive(Selection::Cancel);
                return;
            }
            let Some(principal) = string(&data, "principal") else { return };
            match string(&data, "action").as_deref() {
                Some("open") if !saving => {
                    if let (Some(space), Some(file)) = (id(&data, "space"), id(&data, "file")) {
                        receive(Selection::Open { principal, space, file, revision: id(&data, "revision") });
                    }
                }
                Some("save") if saving => {
                    if let (Some(parent), Some(name)) = (id(&data, "parent"), string(&data, "name"))
                        && !name.is_empty()
                        && name.len() <= 160
                    {
                        receive(Selection::Save { principal, parent, name });
                    }
                }
                _ => {}
            }
        });
        let cancel = Closure::new(move |event: web_sys::Event| {
            event.prevent_default();
            selected(Selection::Cancel);
        });
        document.body().ok_or("Document body unavailable")?.append_child(&dialog).map_err(error)?;
        let picker = Self { window, dialog, close, previous_focus, message, cancel };
        picker.window.add_event_listener_with_callback("message", picker.message.as_ref().unchecked_ref()).map_err(error)?;
        picker.dialog.add_event_listener_with_callback("cancel", picker.cancel.as_ref().unchecked_ref()).map_err(error)?;
        picker.close.add_event_listener_with_callback("click", picker.cancel.as_ref().unchecked_ref()).map_err(error)?;
        picker.dialog.show_modal().map_err(error)?;
        Ok(picker)
    }
}
impl Drop for Picker {
    fn drop(&mut self) {
        let _ = self.window.remove_event_listener_with_callback("message", self.message.as_ref().unchecked_ref());
        let _ = self.dialog.remove_event_listener_with_callback("cancel", self.cancel.as_ref().unchecked_ref());
        let _ = self.close.remove_event_listener_with_callback("click", self.cancel.as_ref().unchecked_ref());
        self.dialog.close();
        self.dialog.remove();
        if let Some(element) = &self.previous_focus {
            let _ = element.focus();
        }
    }
}
