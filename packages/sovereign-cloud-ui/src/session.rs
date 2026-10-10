//! Same-origin account changes never reload or discard the open editor document.
use super::*;

impl Cloud {
    pub(super) fn reload_session(&mut self, ctx: &egui::Context) {
        if self.session_loading {
            return;
        }
        self.session_loading = true;
        self.session_owner = None;
        *self.auth.borrow_mut() = None;
        self.busy = false;
        self.signing_in = false;
        self.projects.clear();
        self.account = None;
        self.pending_save = None;
        self.pending_open = None;
        self.attempt = None;
        self.delete = None;
        // Retain document bindings so a different account cannot silently save them as its own.
        let (tx, ctx) = (self.tx.clone(), ctx.clone());
        wasm_bindgen_futures::spawn_local(async move {
            send(&tx, &ctx, Event::SessionLoaded(BrowserSession::load().await));
        });
    }

    pub(super) fn check_session(&mut self, ctx: &egui::Context) {
        ctx.request_repaint_after(std::time::Duration::from_millis(500));
        if self.session_loading || js_sys::Date::now() < self.next_session_check {
            return;
        }
        self.next_session_check = js_sys::Date::now() + 500.0;
        let changed = self.auth.borrow().as_ref().is_some_and(|s| !s.is_current() || (self.session_owner.is_some() && !s.is_authenticated()));
        if changed {
            self.notify("Your account session changed. Open documents remain in the editor.", false);
            self.reload_session(ctx);
        }
    }

    pub(super) fn open_launch(&mut self, ctx: &egui::Context) {
        let Some(link) = self.launch.take() else { return };
        let result = self.api();
        let api = match result {
            Ok(api) => api,
            Err(e) => {
                self.notify(e, true);
                return;
            }
        };
        if self.account.as_ref().is_none_or(|a| a.account.personal_space != link.space) {
            self.notify("This file is not available to the signed-in account.", true);
            return;
        }
        let kind = self.spec.kind();
        self.begin_operation("Opening your cloud file…");
        let stamp = self.auth.borrow().as_ref().map(|s| s.stamp().to_string()).unwrap_or_default();
        let (tx, ctx) = (self.tx.clone(), ctx.clone());
        wasm_bindgen_futures::spawn_local(async move {
            let result: CloudResult<_> = async {
                let details = api.file(link.space, link.file).await?;
                if details.kind != kind? {
                    return Err("This file format is not supported by this editor".into());
                }
                let latest = details.project.revisions.last().ok_or("File has no saved version")?;
                let (revision, bytes) = api.download(link.file, latest.id, |_| {}).await?;
                Ok((details.project, revision, bytes))
            }
            .await;
            send_scoped(
                &tx,
                &ctx,
                &stamp,
                match result {
                    Ok((project, revision, bytes)) => Event::Opened(project, revision, bytes),
                    Err(e) => Event::Error(e),
                },
            );
        });
    }
}
