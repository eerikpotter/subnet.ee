//! Only authentication, transfer and draft recovery live in egui; browsing is shared.
use super::*;
use crate::locale::tr;
impl Cloud {
    pub fn ui(&mut self, ctx: &egui::Context, app: &mut impl Editor) {
        self.check_session(ctx);
        self.poll(app, ctx);
        self.prepare_save(ctx);
        self.schedule_autosave(app, ctx);
        self.recovery.tick(self.principal(), self.spec, app, ctx, !self.busy && self.pending_open.is_none() && self.picker.is_none());
        if !presentation::host_fonts_ready(ctx) {
            ctx.request_repaint();
            return;
        }
        let status = if self.busy {
            &self.status
        } else if self.attempt.is_some() || self.notice.as_ref().is_some_and(|n| n.error) {
            "Save needs attention"
        } else if let Some(doc) = app.document() {
            match self.bindings.get(&doc.id).filter(|b| Some(b.actor) == self.principal()) {
                Some(b) if !b.editable => "Read-only cloud file",
                Some(b) if b.document_revision == doc.revision => "Saved",
                Some(_) => "Unsaved changes",
                None => "Save As to enable autosave",
            }
        } else {
            "Cloud space"
        };
        if self.recovery.ui(app, ctx, status, !self.busy && self.pending_open.is_none() && self.picker.is_none() && !self.session_loading) {
            self.visible = true;
        }
        if self.notice.as_ref().and_then(|n| n.expires).is_some_and(|until| js_sys::Date::now() >= until) {
            self.notice = None;
            if !self.busy && self.attempt.is_none() && self.principal().is_some() {
                self.visible = false;
            }
        }
        let mut visible = self.visible && self.picker.is_none();
        egui::Window::new(tr("Cloud space")).id(egui::Id::new("sovereign.cloud.status")).open(&mut visible).default_width(440.0).resizable(false).show(
            ctx,
            |ui| {
                presentation::style(ui);
                presentation::header(ui);
                ui.add_space(12.0);
                if self.principal().is_none() {
                    ui.label(tr("Sign in to open and save files in your folders and shared folders."));
                    let label = if self.signing_in { "Reopen Internet Identity" } else { "Sign in with Internet Identity" };
                    if ui
                        .add_enabled_ui((!self.busy || self.signing_in) && self.auth.borrow().is_some(), |ui| presentation::primary(ui, tr(label)))
                        .inner
                        .clicked()
                    {
                        self.login(ctx);
                    }
                    if self.session_loading {
                        ui.weak(tr("Preparing sign-in…"));
                        ctx.request_repaint();
                    } else if self.auth.borrow().is_none() && ui.button(tr("Retry sign-in initialization")).clicked() {
                        self.reload_session(ctx);
                    }
                }
                if self.busy {
                    presentation::transfer(ui, tr(&self.status), self.progress);
                }
                if let Some(notice) = &self.notice {
                    ui.add_space(8.0);
                    if presentation::notification(ui, tr(&notice.message), notice.error) {
                        self.notice = None;
                    }
                }
                if !self.busy
                    && let Some(attempt) = self.attempt.clone()
                {
                    ui.add_space(12.0);
                    ui.strong(tr("Your document is safe in the editor"));
                    ui.label(tr("Retry the captured save, or dismiss it and use Save As for a new copy."));
                    ui.horizontal(|ui| {
                        if ui.button(tr("Retry save")).clicked() {
                            self.run_save(attempt, ctx);
                        }
                        if ui.button(tr("Dismiss attempt")).clicked() {
                            self.attempt = None;
                            self.notice = None;
                        }
                    });
                }
                if !self.busy && self.attempt.is_none() && self.principal().is_some() {
                    ui.add_space(8.0);
                    ui.horizontal(|ui| {
                        if ui.button(tr("Open…")).clicked() {
                            self.request(Request::Open, app, ctx);
                        }
                        if ui.add_enabled(app.document().is_some(), egui::Button::new(tr("Save As…"))).clicked() {
                            self.request(Request::SaveAs, app, ctx);
                        }
                    });
                }
            },
        );
        self.visible = visible && self.picker.is_none();
        let enabled = !self.busy && self.attempt.is_none() && self.pending_open.is_none() && self.picker.is_none();
        self.menu.set_enabled(enabled);
        app.set_save_enabled(enabled);
        if self.pending_open.is_some() {
            egui::Window::new(tr("Open cloud file?")).collapsible(false).show(ctx, |ui| {
                presentation::style(ui);
                ui.label(tr("The current document has unsaved changes. Opening this file replaces them."));
                ui.horizontal(|ui| {
                    if ui.button(tr("Discard changes and open")).clicked()
                        && let Some((entry, revision, bytes)) = self.pending_open.take()
                    {
                        self.finish_open(app, entry, revision, bytes);
                    }
                    if ui.button(tr("Cancel")).clicked() {
                        self.pending_open = None;
                        self.visible = false;
                    }
                });
            });
        }
    }
}
