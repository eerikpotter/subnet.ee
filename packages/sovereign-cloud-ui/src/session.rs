//! Account changes preserve drafts while discarding stale private UI and responses.
use super::*;
impl Cloud {
    pub(super) fn reload_session(&mut self, ctx: &egui::Context) {
        if self.session_loading {
            return;
        }
        self.session_loading = true;
        self.session_owner = None;
        self.profile_name = None;
        self.profile_loading = false;
        self.next_profile_check = 0.0;
        self.initial_destination = None;
        self.creation = None;
        self.naming = None;
        *self.auth.borrow_mut() = None;
        self.busy = false;
        self.signing_in = false;
        self.pending_save = None;
        self.background_save = false;
        self.autosave.reset();
        self.pending_open = None;
        self.attempt = None;
        self.picker = None;
        self.picker_document = None;
        // Retain actor bindings: another account must explicitly choose Save As.
        let (tx, ctx) = (self.tx.clone(), ctx.clone());
        wasm_bindgen_futures::spawn_local(async move {
            send(&tx, &ctx, Event::SessionLoaded(BrowserSession::load().await));
        });
    }
    pub(super) fn refresh_profile(&mut self, ctx: &egui::Context) {
        if self.principal().is_none() || self.profile_loading || js_sys::Date::now() < self.next_profile_check {
            return;
        }
        let Ok(api) = self.api() else { return };
        let marker = self.profile_marker.clone();
        self.profile_loading = true;
        self.next_profile_check = js_sys::Date::now() + 30_000.0;
        let (tx, ctx, stamp) = (self.tx.clone(), ctx.clone(), self.stamp());
        wasm_bindgen_futures::spawn_local(async move {
            send_scoped(&tx, &ctx, &stamp, Event::Profile(marker, api.account().await));
        });
    }
    pub(super) fn check_session(&mut self, ctx: &egui::Context) {
        ctx.request_repaint_after(std::time::Duration::from_millis(500));
        if self.session_loading || js_sys::Date::now() < self.next_session_check {
            return;
        }
        self.next_session_check = js_sys::Date::now() + 500.0;
        let marker = web_sys::window().and_then(|w| w.local_storage().ok().flatten()).and_then(|s| s.get_item("subnet.profile.changed").ok().flatten());
        if marker != self.profile_marker {
            self.profile_marker = marker;
            self.next_profile_check = 0.0;
        }
        if self.auth.borrow().as_ref().is_some_and(|s| !s.is_current() || (self.session_owner.is_some() && !s.is_authenticated())) {
            self.after_login = None;
            self.notify("Your account session changed. Open documents remain in the editor.", false);
            self.reload_session(ctx);
        }
    }
}
