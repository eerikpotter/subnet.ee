//! Only authentication, transfer and draft recovery live in egui; browsing is shared.
use super::*;
impl Cloud {
    pub fn ui(&mut self, ctx: &egui::Context, app: &mut impl Editor) {
        self.check_session(ctx);
        self.poll(app, ctx);
        self.prepare_save(ctx);
        if !presentation::host_fonts_ready(ctx) {
            ctx.request_repaint();
            return;
        }
        if self.notice.as_ref().and_then(|n| n.expires).is_some_and(|until| js_sys::Date::now() >= until) {
            self.notice = None;
            if !self.busy && self.attempt.is_none() && self.principal().is_some() {
                self.visible = false;
            }
        }
        let mut visible = self.visible && self.picker.is_none();
        egui::Window::new("Cloud space").id(egui::Id::new("sovereign.cloud.status")).open(&mut visible).default_width(440.0).resizable(false).show(ctx, |ui| {
            presentation::style(ui);
            presentation::header(ui);
            ui.add_space(12.0);
            if self.principal().is_none() {
                ui.label("Sign in to open and save files in your folders and shared folders.");
                let label = if self.signing_in { "Reopen Internet Identity" } else { "Sign in with Internet Identity" };
                if ui.add_enabled_ui((!self.busy || self.signing_in) && self.auth.borrow().is_some(), |ui| presentation::primary(ui, label)).inner.clicked() {
                    self.login(ctx);
                }
                if self.session_loading {
                    ui.weak("Preparing sign-in…");
                    ctx.request_repaint();
                } else if self.auth.borrow().is_none() && ui.button("Retry sign-in initialization").clicked() {
                    self.reload_session(ctx);
                }
            }
            if self.busy {
                presentation::transfer(ui, &self.status, self.progress);
            }
            if let Some(notice) = &self.notice {
                ui.add_space(8.0);
                if presentation::notification(ui, &notice.message, notice.error) {
                    self.notice = None;
                }
            }
            if !self.busy
                && let Some(attempt) = self.attempt.clone()
            {
                ui.add_space(12.0);
                ui.strong("Your document is safe in the editor");
                ui.label("Retry the captured save, or dismiss it and use Save As for a new copy.");
                ui.horizontal(|ui| {
                    if ui.button("Retry save").clicked() {
                        self.run_save(attempt, ctx);
                    }
                    if ui.button("Dismiss attempt").clicked() {
                        self.attempt = None;
                        self.notice = None;
                    }
                });
            }
            if !self.busy && self.attempt.is_none() && self.principal().is_some() {
                ui.add_space(8.0);
                ui.horizontal(|ui| {
                    if ui.button("Open…").clicked() {
                        self.request(Request::Open, app, ctx);
                    }
                    if ui.add_enabled(app.document().is_some(), egui::Button::new("Save As…")).clicked() {
                        self.request(Request::SaveAs, app, ctx);
                    }
                });
            }
        });
        self.visible = visible && self.picker.is_none();
        let enabled = !self.busy && self.attempt.is_none() && self.pending_open.is_none() && self.picker.is_none();
        self.menu.set_enabled(enabled);
        app.set_save_enabled(enabled);
        if self.pending_open.is_some() {
            egui::Window::new("Open cloud file?").collapsible(false).show(ctx, |ui| {
                presentation::style(ui);
                ui.label("The current document has unsaved changes. Opening this file replaces them.");
                ui.horizontal(|ui| {
                    if ui.button("Discard changes and open").clicked()
                        && let Some((entry, revision, bytes)) = self.pending_open.take()
                    {
                        self.finish_open(app, entry, revision, bytes);
                    }
                    if ui.button("Cancel").clicked() {
                        self.pending_open = None;
                        self.visible = false;
                    }
                });
            });
        }
    }
}
