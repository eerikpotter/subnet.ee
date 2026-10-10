//! Cloud file management is a browser-host view, separate from the editor workspace.
use super::*;
use presentation::{RevisionAction, SaveAction};

impl Cloud {
    pub fn ui(&mut self, ctx: &egui::Context, app: &mut impl Editor) {
        self.check_session(ctx);
        self.poll(app, ctx);
        self.prepare_save(ctx);
        let doc_id = app.document().map(|d| d.id);
        if self.named_doc != doc_id {
            self.named_doc = doc_id;
            self.copy_mode = false;
            self.name = app.document().map_or_else(|| "Untitled".into(), |d| d.name);
        }
        if self.notice.as_ref().and_then(|n| n.expires).is_some_and(|until| js_sys::Date::now() >= until) {
            self.notice = None;
        }
        let mut visible = self.visible;
        let t = presentation::palette(ctx);
        let title = match self.view {
            View::Save if self.copy_mode => "Save As to Cloud",
            View::Save => "Save to Cloud",
            View::Projects => "Open from Cloud",
        };
        egui::Window::new(title)
            .id(egui::Id::new("sovereign.cloud.library.window")).open(&mut visible)
            .frame(egui::Frame::window(&ctx.global_style()).fill(t.dock).stroke(egui::Stroke::new(1.0, t.accent_border)).corner_radius(t.radius_lg).inner_margin(16.0))
            .default_size([760.0, 560.0]).min_width(300.0).vscroll(true)
            .show(ctx, |ui| {
                presentation::style(ui);
                presentation::header(ui);
                ui.add_space(4.0);
                if let Some(principal) = self.principal() {
                    ui.horizontal(|ui| {
                        ui.weak("subnet.ee · My files");
                        ui.hyperlink_to("Open in subnet.ee", "/#failid");
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            ui.add_enabled_ui(!self.busy, |ui| {
                                ui.menu_button("Account", |ui| {
                                    ui.label("Internet Identity");
                                    ui.small(principal.to_text());
                                    ui.separator();
                                    if ui.button("Sign out").clicked() { self.sign_out(ctx); ui.close(); }
                                });
                            });
                        });
                    });
                    ui.add_space(8.0);
                    if self.busy { presentation::transfer(ui, &self.status, self.progress); ui.add_space(8.0); }
                    ui.add_enabled_ui(!self.busy, |ui| match self.view {
                        View::Save => { presentation::section(ui).show(ui, |ui| self.save_view(ui, app, ctx)); },
                        View::Projects => self.projects_view(ui, ctx),
                    });
                    if !self.busy { self.retry_view(ui, ctx); }
                } else {
                    ui.add_space(12.0);
                    presentation::section(ui).show(ui, |ui| {
                        ui.set_min_width(ui.available_width());
                        ui.heading("A home for your files");
                        ui.label("Save your files, keep earlier versions, and open them on another device.");
                        ui.add_space(12.0);
                        let label = if self.signing_in { "Reopen Internet Identity" } else { "Sign in with Internet Identity" };
                        if ui.add_enabled_ui((!self.busy || self.signing_in) && self.auth.borrow().is_some(), |ui| presentation::primary(ui, label)).inner.clicked() {
                            self.login(ctx);
                        }
                        if self.session_loading { ui.weak("Preparing sign-in…"); ctx.request_repaint(); }
                        else if self.auth.borrow().is_none() && ui.button("Retry sign-in initialization").clicked() { self.reload_session(ctx); }
                        if self.signing_in { ui.small("Allow the sign-in popup, or open this app in your regular browser."); }
                    });
                    if self.busy { ui.add_space(8.0); presentation::transfer(ui, &self.status, self.progress); }
                }
                if let Some(notice) = &self.notice {
                    ui.add_space(8.0);
                    let dismiss = presentation::notification(ui, &notice.message, notice.error);
                    if notice.expires.is_some() { ctx.request_repaint_after(std::time::Duration::from_secs(1)); }
                    if dismiss { self.notice = None; }
                }
                ui.add_space(12.0);
                ui.collapsing("How cloud saving works", |ui| {
                    ui.label("Editing runs on your device. ICP hosts the app and your saved files.");
                    ui.label("File → Sovereign Cloud → Save saves here. File → Save / Export downloads a local copy. Save before closing or reloading; cloud saving is manual.");
                    ui.label("Demo limits: 64 MiB per file; 256 MiB per account including versions; 20 files, 20 versions each; 2 GiB across the service.");
                    ui.label("Files are access-controlled, not end-to-end encrypted. The service controller can upgrade the backend.");
            ui.hyperlink_to("Source and architecture", "https://github.com/eerikpotter/subnet.ee");
                });
            });
        self.visible = visible;
        let save_enabled = !self.busy && self.attempt.is_none() && self.pending_open.is_none();
        self.menu.set_enabled(save_enabled);
        app.set_save_enabled(save_enabled);
        if self.pending_open.is_some() {
            egui::Window::new("Open cloud file?").collapsible(false).show(ctx, |ui| {
                presentation::style(ui);
                ui.label("The current document has unsaved changes. Opening this file replaces them.");
                ui.horizontal(|ui| {
                    if ui.button("Discard changes and open").clicked()
                        && let Some((project, revision, bytes)) = self.pending_open.take()
                    {
                        self.finish_open(app, project, revision, bytes);
                    }
                    if ui.button("Cancel").clicked() {
                        self.pending_open = None;
                    }
                });
            });
        }
        if let Some((id, method)) = self.delete.clone() {
            egui::Window::new("Delete cloud data?").collapsible(false).show(ctx, |ui| {
                presentation::style(ui);
                ui.label("This permanently removes the selected cloud file, version, or unfinished save. Open documents in the editor are unaffected.");
                ui.horizontal(|ui| {
                    if ui.add_enabled(!self.busy, egui::Button::new("Delete permanently")).clicked() {
                        self.delete = None;
                        self.remove(id, if method == "delete_project" { "delete_project" } else { "delete_revision_or_upload" }, ctx);
                    }
                    if ui.button("Cancel").clicked() {
                        self.delete = None;
                    }
                });
            });
        }
    }

    fn sign_out(&mut self, ctx: &egui::Context) {
        let auth = self.auth.borrow().clone();
        let (tx, ctx) = (self.tx.clone(), ctx.clone());
        self.begin_operation("Signing out…");
        wasm_bindgen_futures::spawn_local(async move {
            if let Some(a) = auth
                && let Err(e) = a.logout().await
            {
                send(&tx, &ctx, Event::Error(e));
                return;
            }
            send(&tx, &ctx, Event::SignedOut);
        });
    }

    fn save_view(&mut self, ui: &mut egui::Ui, app: &impl Editor, ctx: &egui::Context) {
        if app.document().is_some() {
            match presentation::save_form(ui, &mut self.name, self.copy_mode, self.attempt.is_none()) {
                Some(SaveAction::Save) => self.save(app, ctx, self.copy_mode),
                Some(SaveAction::Cancel) => {
                    self.view = View::Projects;
                    self.copy_mode = false;
                }
                None => {}
            }
        } else {
            ui.heading("No document is open");
            ui.label("Create or open a document in the editor, then choose File → Sovereign Cloud → Save.");
        }
    }

    fn retry_view(&mut self, ui: &mut egui::Ui, ctx: &egui::Context) {
        if let Some(a) = self.attempt.clone() {
            ui.add_space(12.0);
            ui.separator();
            ui.strong("Your save is ready to retry");
            ui.label(self.spec.file_name(&a.name));
            if !self.busy {
                ui.weak("Retry the captured file, or dismiss this attempt to save your current edits.");
                ui.horizontal(|ui| {
                    if ui.button("Retry save").clicked() {
                        self.run_save(a, ctx);
                    }
                    if ui.button("Dismiss attempt").clicked() {
                        self.attempt = None;
                        self.refresh(ctx);
                    }
                });
            }
        }
    }

    fn projects_view(&mut self, ui: &mut egui::Ui, ctx: &egui::Context) {
        ui.horizontal(|ui| {
            ui.heading("My files");
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if ui.button("Refresh").clicked() {
                    self.refresh(ctx);
                }
            });
        });
        ui.weak("Click a row for version history · Times shown in your local timezone");
        ui.add_space(12.0);
        let mut files = self.projects.clone();
        files.sort_by_key(|p| std::cmp::Reverse(p.revisions.last().map_or(p.created_at, |r| r.created_at)));
        if !files.iter().any(|p| !p.revisions.is_empty()) && !self.busy {
            ui.label("Your saved files will appear here.");
            ui.weak("Use File → Sovereign Cloud → Save in the editor to save your first file.");
        }
        if files.iter().any(|p| !p.revisions.is_empty()) {
            presentation::file_column_labels(ui);
        }
        // The window owns the only scrollbar; file rows and expanded history flow naturally.
        for p in files.iter().filter(|p| !p.revisions.is_empty()) {
            let Some(latest) = p.revisions.last() else {
                continue;
            };
            let open = presentation::file_row(ui, p.id, &self.spec.file_name(&p.name), &timestamp(latest.created_at), |ui| {
                ui.strong(format!("Version history ({})", p.revisions.len()));
                for r in p.revisions.iter().rev() {
                    ui.push_id(r.id, |ui| match presentation::revision_row(ui, &timestamp(r.created_at), r.bytes, r.id == latest.id) {
                        Some(RevisionAction::Open) => self.open(p.clone(), r.clone(), ctx),
                        Some(RevisionAction::Delete) => self.delete = Some((r.id, "delete_revision_or_upload".into())),
                        None => {}
                    });
                }
            });
            if open {
                self.open(p.clone(), latest.clone(), ctx);
            }
        }
        let unfinished: Vec<_> = files.iter().filter(|p| p.revisions.is_empty() || p.pending_upload.is_some()).collect();
        if !unfinished.is_empty() {
            ui.add_space(12.0);
            ui.separator();
            ui.collapsing(format!("Unfinished saves ({})", unfinished.len()), |ui| {
                ui.weak("These saves did not finish. They cannot be opened. Retry below while the attempt is available, or discard them here.");
                for p in unfinished {
                    ui.push_id(("unfinished", p.id), |ui| {
                        ui.strong(self.spec.file_name(&p.name));
                        ui.weak(if p.revisions.is_empty() { "No saved version yet" } else { "Your earlier saved versions are available above" });
                        if ui.button("Discard unfinished save…").clicked() {
                            self.delete =
                                Some(if let Some(id) = p.pending_upload { (id, "delete_revision_or_upload".into()) } else { (p.id, "delete_project".into()) });
                        }
                    });
                }
            });
        }
    }
}
