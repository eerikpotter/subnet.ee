//! Shared browser controller; all file access uses the canonical authorized drive API.
#[path = "drafts.rs"]
mod drafts;
#[path = "panel.rs"]
mod panel;
#[path = "picker.rs"]
mod picker;
#[path = "recovery.rs"]
mod recovery;
#[path = "session.rs"]
mod session;
use crate::locale::tr;
use crate::{AppSpec, Editor, MenuHandle, Request, Snapshot, flow, presentation};
use candid::Principal;
use sovereign_cloud::{
    args,
    browser::{self, BrowserClient as Api, BrowserSession},
    drive::*,
    protocol::*,
};
use std::{
    cell::RefCell,
    collections::BTreeMap,
    rc::Rc,
    sync::{Arc, mpsc},
};

#[derive(Clone)]
struct Attempt {
    actor: Principal,
    target: UploadTarget,
    bytes: Arc<Vec<u8>>,
    name: String,
    request: String,
    doc_id: u64,
    doc_revision: u64,
    automatic: bool,
}
enum Event {
    SessionLoaded(CloudResult<BrowserSession>),
    Scoped(String, Box<Event>),
    SignedIn,
    Error(String),
    Progress(String, Option<f32>),
    Selection(u64, picker::Selection),
    Saved(Attempt, DriveUpload),
    Opened(DriveEntry, Revision, Vec<u8>),
}
struct Binding {
    /// The account that opened/saved, not the (potentially different) owner.
    actor: Principal,
    file: ObjectId,
    version: u64,
    name: String,
    document_revision: u64,
    editable: bool,
}
struct PendingSave {
    snapshot: Snapshot,
    attempt: Attempt,
    ready_frame: u64,
}
struct Notice {
    message: String,
    error: bool,
    expires: Option<f64>,
}
pub struct Cloud {
    spec: AppSpec,
    menu: MenuHandle,
    pending_open: Option<(DriveEntry, Revision, Vec<u8>)>,
    auth: Rc<RefCell<Option<BrowserSession>>>,
    tx: mpsc::Sender<Event>,
    rx: mpsc::Receiver<Event>,
    visible: bool,
    busy: bool,
    signing_in: bool,
    status: String,
    progress: Option<f32>,
    notice: Option<Notice>,
    pending_save: Option<PendingSave>,
    bindings: BTreeMap<u64, Binding>,
    attempt: Option<Attempt>,
    session_loading: bool,
    session_owner: Option<Principal>,
    next_session_check: f64,
    launch: Option<sovereign_cloud::launch::CloudLaunch>,
    after_login: Option<Request>,
    initial_destination: Option<(ObjectId, ObjectId)>,
    picker: Option<picker::Picker>,
    picker_id: u64,
    picker_document: Option<u64>,
    autosave: crate::autosave::Schedule,
    recovery: recovery::Recovery,
    background_save: bool,
}
fn send_scoped(tx: &mpsc::Sender<Event>, ctx: &egui::Context, stamp: &str, event: Event) {
    send(tx, ctx, Event::Scoped(stamp.to_string(), Box::new(event)));
}
fn send(tx: &mpsc::Sender<Event>, ctx: &egui::Context, event: Event) {
    let _ = tx.send(event);
    ctx.request_repaint();
}
impl Cloud {
    pub fn new(spec: AppSpec, ctx: egui::Context) -> Self {
        egui_extras::install_image_loaders(&ctx);
        let menu = MenuHandle::new(ctx.clone());
        let (tx, rx) = mpsc::channel();
        let sender = tx.clone();
        wasm_bindgen_futures::spawn_local(async move {
            send(&sender, &ctx, Event::SessionLoaded(BrowserSession::load().await));
        });
        let query = web_sys::window().and_then(|w| w.location().search().ok()).unwrap_or_default();
        let launch = sovereign_cloud::launch::CloudLaunch::parse(&query);
        let launch_error = launch.as_ref().err().cloned();
        Self {
            spec,
            menu,
            pending_open: None,
            auth: Rc::new(RefCell::new(None)),
            tx,
            rx,
            visible: launch_error.is_some() || launch.as_ref().is_ok_and(|v| v.is_some()),
            busy: false,
            signing_in: false,
            status: String::new(),
            progress: None,
            notice: launch_error.map(|message| Notice { message, error: true, expires: None }),
            pending_save: None,
            bindings: BTreeMap::new(),
            attempt: None,
            session_loading: true,
            session_owner: None,
            next_session_check: 0.0,
            launch: launch.ok().flatten(),
            after_login: None,
            initial_destination: None,
            picker: None,
            picker_id: 0,
            picker_document: None,
            autosave: crate::autosave::Schedule::default(),
            recovery: recovery::Recovery::new(),
            background_save: false,
        }
    }
    pub fn menu_handle(&self) -> MenuHandle {
        self.menu.clone()
    }
    fn principal(&self) -> Option<Principal> {
        self.auth.borrow().as_ref().filter(|a| a.is_authenticated()).and_then(|a| a.principal().ok())
    }
    fn stamp(&self) -> String {
        self.auth.borrow().as_ref().map(|s| s.stamp().to_string()).unwrap_or_default()
    }
    fn api(&self) -> CloudResult<Api> {
        Api::new(self.auth.borrow().as_ref().ok_or("Identity is still loading")?)
    }
    fn begin_operation(&mut self, message: impl Into<String>) {
        self.busy = true;
        if !self.background_save {
            self.visible = true;
        }
        self.status = message.into();
        self.progress = None;
        self.notice = None;
    }
    fn notify(&mut self, message: impl Into<String>, error: bool) {
        self.visible = true;
        self.notice = Some(Notice { message: message.into(), error, expires: if error { None } else { Some(js_sys::Date::now() + 8000.0) } });
    }
    fn login(&mut self, ctx: &egui::Context) {
        let (success_tx, error_tx, success_ctx, error_ctx) = (self.tx.clone(), self.tx.clone(), ctx.clone(), ctx.clone());
        let auth = self.auth.borrow().clone();
        if let Some(auth) = auth {
            self.begin_operation("Complete sign-in in the Internet Identity window…");
            self.signing_in = true;
            if let Err(e) =
                browser::login(&auth, move || send(&success_tx, &success_ctx, Event::SignedIn), move |e| send(&error_tx, &error_ctx, Event::Error(e)))
            {
                self.busy = false;
                self.signing_in = false;
                self.notify(e, true);
            }
        }
    }
    fn request(&mut self, request: Request, app: &impl Editor, ctx: &egui::Context) {
        if self.busy || self.attempt.is_some() || self.pending_open.is_some() || self.picker.is_some() {
            self.visible = self.picker.is_none();
            return;
        }
        if self.principal().is_none() {
            self.after_login = Some(request);
            self.visible = true;
            return;
        }
        if request != Request::Autosave {
            self.background_save = false;
        }
        if request == Request::Open {
            self.show_picker(None, None, ctx);
            return;
        }
        let Some(doc) = app.document() else {
            self.notify("Open or create a document first", true);
            return;
        };
        if matches!(request, Request::Save | Request::Autosave)
            && let Some(binding) = self.bindings.get(&doc.id)
        {
            if Some(binding.actor) != self.principal() {
                self.notify("This document was opened with another account. Sign back in, or use Save As to save a copy.", true);
                return;
            }
            if flow::route(Some(binding.document_revision), doc.revision, false) == flow::SaveRoute::AlreadySaved {
                if request != Request::Autosave {
                    self.notify("This file is already saved to cloud. No changes to upload.", false);
                }
                return;
            }
            let target = UploadTarget::Replace { file: binding.file, expected_revision: binding.version };
            let name = binding.name.clone();
            self.background_save = request == Request::Autosave;
            self.save(app, ctx, target, name);
            return;
        }
        if request == Request::Autosave {
            return;
        }
        self.background_save = false;
        let name = self.bindings.get(&doc.id).map_or(doc.name.as_str(), |b| b.name.as_str());
        let name = if request == Request::SaveAs { format!("{} copy", self.spec.stem(name)) } else { self.spec.stem(name) };
        self.show_picker(Some(&self.spec.file_name(&name)), Some(doc.id), ctx);
    }
    fn show_picker(&mut self, name: Option<&str>, document: Option<u64>, ctx: &egui::Context) {
        self.picker_id = self.picker_id.wrapping_add(1);
        let id = self.picker_id;
        let (tx, ctx) = (self.tx.clone(), ctx.clone());
        let stamp = self.stamp();
        let dark = ctx.global_style().visuals.dark_mode;
        match picker::Picker::open(self.spec, name, self.initial_destination, dark, move |selection| {
            send_scoped(&tx, &ctx, &stamp, Event::Selection(id, selection))
        }) {
            Ok(picker) => {
                self.picker = Some(picker);
                self.picker_document = document;
                self.visible = false;
                self.notice = None;
            }
            Err(e) => self.notify(e, true),
        }
    }
    fn save(&mut self, app: &impl Editor, ctx: &egui::Context, target: UploadTarget, name: String) {
        let Some(actor) = self.principal() else {
            self.notify("Sign in before saving", true);
            return;
        };
        let snapshot = match app.capture() {
            Ok(s) => s,
            Err(e) => {
                self.notify(e, true);
                return;
            }
        };
        let request = match web_sys::window().and_then(|w| w.crypto().ok()) {
            Some(crypto) => crypto.random_uuid(),
            None => {
                self.notify("Secure random IDs are unavailable", true);
                return;
            }
        };
        let attempt = Attempt {
            actor,
            target,
            bytes: Arc::new(Vec::new()),
            name,
            request,
            doc_id: snapshot.document.id,
            doc_revision: snapshot.document.revision,
            automatic: self.background_save,
        };
        self.pending_save = Some(PendingSave { snapshot, attempt, ready_frame: ctx.cumulative_frame_nr().saturating_add(2) });
        self.begin_operation("Preparing your file…");
        ctx.request_repaint();
    }
    fn prepare_save(&mut self, ctx: &egui::Context) {
        if let Some(pending) = &self.pending_save {
            if ctx.cumulative_frame_nr() < pending.ready_frame {
                ctx.request_repaint();
                return;
            }
        } else {
            return;
        }
        let Some(mut pending) = self.pending_save.take() else {
            return;
        };
        match (pending.snapshot.encode)() {
            Ok(bytes) if bytes.len() as u64 <= MAX_DOCUMENT_BYTES => {
                pending.attempt.bytes = Arc::new(bytes);
                self.attempt = Some(pending.attempt.clone());
                self.run_save(pending.attempt, ctx);
            }
            result => {
                self.busy = false;
                self.notify(
                    match result {
                        Ok(_) => "Cloud files can be up to 64 MiB".into(),
                        Err(e) => format!("{}: {e}", tr("Could not prepare your file")),
                    },
                    true,
                );
            }
        }
    }
    fn run_save(&mut self, attempt: Attempt, ctx: &egui::Context) {
        if Some(attempt.actor) != self.principal() {
            self.busy = false;
            self.notify("Account changed. The captured file was not sent.", true);
            return;
        }
        let api = match self.api() {
            Ok(a) => a,
            Err(e) => {
                self.busy = false;
                self.notify(e, true);
                return;
            }
        };
        let kind = match self.spec.kind() {
            Ok(k) => k,
            Err(e) => {
                self.busy = false;
                self.notify(e, true);
                return;
            }
        };
        self.begin_operation("Connecting to your cloud…");
        let stamp = self.stamp();
        let (tx, ctx) = (self.tx.clone(), ctx.clone());
        wasm_bindgen_futures::spawn_local(async move {
            let result = api
                .drive_save_craft_with_policy(attempt.target.clone(), &attempt.bytes, attempt.request.clone(), &kind, attempt.automatic, |done, total| {
                    send_scoped(
                        &tx,
                        &ctx,
                        &stamp,
                        Event::Progress(
                            if done == total { "Finishing save…" } else { "Uploading your file…" }.into(),
                            if done == total { None } else { Some(done as f32 / total.max(1) as f32) },
                        ),
                    );
                })
                .await;
            send_scoped(
                &tx,
                &ctx,
                &stamp,
                match result {
                    Ok(upload) => Event::Saved(attempt, upload),
                    Err(e) => Event::Error(e),
                },
            );
        });
    }
    fn open(&mut self, space: ObjectId, file: ObjectId, revision: Option<u64>, ctx: &egui::Context) {
        let api = match self.api() {
            Ok(a) => a,
            Err(e) => {
                self.notify(e, true);
                return;
            }
        };
        let kind = self.spec.kind();
        self.begin_operation("Opening your cloud file…");
        let stamp = self.stamp();
        let (tx, ctx) = (self.tx.clone(), ctx.clone());
        wasm_bindgen_futures::spawn_local(async move {
            let result: CloudResult<_> = async {
                let entry = api.drive_craft_file(space, file, &kind?).await?;
                let latest = entry.latest.clone().ok_or("File has no saved version")?;
                let revision = match revision {
                    Some(id) if id != latest.id => {
                        let history: Vec<Revision> = api.drive_call("drive_history", args((file,))?).await?;
                        history.into_iter().find(|r| r.id == id).ok_or("Revision unavailable")?
                    }
                    _ => latest,
                };
                let bytes = api.drive_download(file, revision.id).await?;
                Ok((entry, revision, bytes))
            }
            .await;
            send_scoped(
                &tx,
                &ctx,
                &stamp,
                match result {
                    Ok((entry, r, bytes)) => Event::Opened(entry, r, bytes),
                    Err(e) => Event::Error(e),
                },
            );
        });
    }
    fn poll(&mut self, app: &mut impl Editor, ctx: &egui::Context) {
        while let Some(request) = self.menu.pop() {
            self.request(request, app, ctx);
        }
        while let Ok(event) = self.rx.try_recv() {
            let event = match event {
                Event::Scoped(stamp, event) => {
                    if !self.auth.borrow().as_ref().is_some_and(|s| s.stamp() == stamp && s.is_authenticated()) {
                        continue;
                    }
                    *event
                }
                event => event,
            };
            match event {
                Event::SessionLoaded(result) => {
                    self.session_loading = false;
                    match result {
                        Ok(session) if session.is_current() => {
                            self.session_owner = session.principal().ok();
                            *self.auth.borrow_mut() = Some(session);
                            self.busy = false;
                            self.signing_in = false;
                            if self.principal().is_some() {
                                if let Some(link) = self.launch.take() {
                                    match link {
                                        sovereign_cloud::launch::CloudLaunch::File { space, file } => self.open(space, file, None, ctx),
                                        sovereign_cloud::launch::CloudLaunch::New { space, parent } => {
                                            self.initial_destination = Some((space, parent));
                                            self.visible = false;
                                        }
                                    }
                                } else if let Some(request) = self.after_login.take() {
                                    self.request(request, app, ctx);
                                }
                            }
                        }
                        Ok(_) => self.reload_session(ctx),
                        Err(e) => self.notify(format!("{}: {e}", tr("Identity initialization")), true),
                    }
                }
                Event::SignedIn => self.reload_session(ctx),
                Event::Scoped(_, _) => {}
                Event::Error(e) => {
                    self.background_save = false;
                    self.busy = false;
                    self.signing_in = false;
                    self.notify(e, true);
                }
                Event::Progress(message, fraction) => {
                    self.status = message;
                    self.progress = fraction;
                }
                Event::Selection(id, selection) => {
                    if id != self.picker_id || self.picker.is_none() {
                        continue;
                    }
                    self.picker = None;
                    match selection {
                        picker::Selection::Cancel => {}
                        picker::Selection::Open { principal, space, file, revision } => {
                            if self.principal().is_some_and(|p| p.to_text() == principal) {
                                self.open(space, file, revision, ctx);
                            }
                        }
                        picker::Selection::Save { principal, parent, name } => {
                            if self.principal().is_none_or(|p| p.to_text() != principal) {
                                continue;
                            }
                            if app.document().map(|d| d.id) != self.picker_document {
                                self.notify("The open document changed. Choose Save As again.", true);
                                continue;
                            }
                            match self.spec.kind() {
                                Ok(kind) => self.save(
                                    app,
                                    ctx,
                                    UploadTarget::New {
                                        parent,
                                        name: name.clone(),
                                        format: FileFormat {
                                            media_type: if self.spec.id == "pdfcraft" { "application/pdf" } else { "application/octet-stream" }.into(),
                                            craft: Some(kind),
                                        },
                                    },
                                    name,
                                ),
                                Err(e) => self.notify(e, true),
                            }
                        }
                    }
                    self.picker_document = None;
                }
                Event::Saved(attempt, upload) => {
                    self.busy = false;
                    self.background_save = false;
                    let Some(revision) = upload.committed else {
                        self.notify("The upload was not committed. Retry the captured file.", true);
                        continue;
                    };
                    self.recovery.saved(attempt.actor, attempt.doc_id, attempt.doc_revision);
                    browser::notify_files_changed();
                    self.attempt = None;
                    let saved_revision = match app.saved(attempt.doc_id, attempt.doc_revision, &attempt.bytes) {
                        Ok(revision) => revision.unwrap_or(attempt.doc_revision),
                        Err(e) => {
                            self.notify(format!("{}: {e}", tr("Cloud saved, but the editor could not update its saved state")), true);
                            attempt.doc_revision
                        }
                    };
                    self.bindings.insert(
                        attempt.doc_id,
                        Binding {
                            actor: attempt.actor,
                            file: upload.file,
                            version: revision.id,
                            name: attempt.name.clone(),
                            document_revision: saved_revision,
                            editable: true,
                        },
                    );
                    if !attempt.automatic && !self.notice.as_ref().is_some_and(|n| n.error) {
                        self.notify(format!("{} {} · {}", tr("Saved"), attempt.name, timestamp(revision.created_at)), false);
                    }
                }
                Event::Opened(entry, revision, bytes) => {
                    self.busy = false;
                    if app.replaces_unsaved() {
                        self.pending_open = Some((entry, revision, bytes));
                    } else {
                        self.finish_open(app, entry, revision, bytes);
                    }
                }
            }
        }
    }
    fn schedule_autosave(&mut self, app: &impl Editor, ctx: &egui::Context) {
        let Some(doc) = app.document() else {
            return;
        };
        let eligible = !self.busy
            && self.attempt.is_none()
            && self.pending_open.is_none()
            && self.picker.is_none()
            && !self.session_loading
            && self.bindings.get(&doc.id).is_some_and(|b| Some(b.actor) == self.principal() && b.editable && b.document_revision != doc.revision);
        if eligible {
            ctx.request_repaint_after(std::time::Duration::from_secs(1));
        }
        if self.autosave.due(doc.id, doc.revision, js_sys::Date::now(), eligible) {
            self.menu.request(Request::Autosave);
            ctx.request_repaint();
        }
    }
    fn finish_open(&mut self, app: &mut impl Editor, entry: DriveEntry, revision: Revision, bytes: Vec<u8>) {
        let Some(actor) = self.principal() else {
            return;
        };
        match app.open(&self.spec.file_name(&entry.object.name), &bytes) {
            Ok(document) => {
                // Only successful native decoding is a view. Personal activity
                // is best-effort and must never prevent access to the document.
                if let Ok(api) = self.api() {
                    let file = entry.object.id;
                    let seen = revision.id;
                    wasm_bindgen_futures::spawn_local(async move {
                        if let Ok(input) = args((file, Some(seen))) {
                            let _ = api.drive_call::<()>("drive_acknowledge_open", input).await;
                        }
                    });
                }
                self.bindings.insert(
                    document.id,
                    Binding {
                        actor,
                        file: entry.object.id,
                        version: revision.id,
                        name: entry.object.name.clone(),
                        document_revision: document.revision,
                        editable: entry.capabilities.edit,
                    },
                );
                if entry.capabilities.edit {
                    self.visible = false;
                    self.notice = None;
                } else {
                    self.notify("Opened read-only. You can edit locally and use Save As to save your own copy.", false);
                }
            }
            Err(e) => self.notify(format!("{}: {e}", tr("Could not open your file")), true),
        }
    }
}
fn timestamp(nanos: u64) -> String {
    js_sys::Date::new(&wasm_bindgen::JsValue::from_f64((nanos / 1_000_000) as f64))
        .to_locale_string(if crate::locale::language() == "et" { "et-EE" } else { "en-GB" }, &wasm_bindgen::JsValue::UNDEFINED)
        .into()
}
