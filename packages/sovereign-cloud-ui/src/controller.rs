//! Optional browser/cloud adapter. No changes to the upstream engine or document format.
#[path = "panel.rs"]
mod panel;
#[path = "session.rs"]
mod session;
use crate::{AppSpec, Editor, MenuHandle, Request, Snapshot, flow, presentation};
use candid::Principal;
use sovereign_cloud::protocol::*;
use sovereign_cloud::{
    TransferEvent, UploadState,
    browser::{self, BrowserSession},
};
use sovereign_cloud::{args, browser::BrowserClient as Api};
use std::{
    cell::RefCell,
    collections::BTreeMap,
    rc::Rc,
    sync::{Arc, mpsc},
};

#[derive(Clone)]
struct Attempt {
    owner: Principal,
    project: Option<Project>,
    upload: Option<u64>,
    expected: Option<u64>,
    bytes: Arc<Vec<u8>>,
    name: String,
    request: String,
    doc_id: u64,
    doc_revision: u64,
}
enum Event {
    RequestSave(bool),
    RequestProjects,
    SessionLoaded(CloudResult<BrowserSession>),
    Scoped(String, Box<Event>),
    SignedIn,
    SignedOut,
    Error(String),
    Progress(String, Option<f32>),
    Projects(AccountContext, Vec<Project>),
    Prepared(Attempt),
    Saved(Attempt, Revision),
    Opened(Project, Revision, Vec<u8>),
    Removed { file: Option<u64>, item: u64 },
}
#[derive(Clone, Copy, PartialEq)]
enum View {
    Save,
    Projects,
}
struct Binding {
    owner: Principal,
    file: u64,
    version: u64,
    document_revision: u64,
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
    pending_open: Option<(Project, Revision, Vec<u8>)>,
    auth: Rc<RefCell<Option<BrowserSession>>>,
    tx: mpsc::Sender<Event>,
    rx: mpsc::Receiver<Event>,
    visible: bool,
    view: View,
    busy: bool,
    signing_in: bool,
    status: String,
    progress: Option<f32>,
    notice: Option<Notice>,
    pending_save: Option<PendingSave>,
    copy_mode: bool,
    named_doc: Option<u64>,
    name: String,
    projects: Vec<Project>,
    bindings: BTreeMap<u64, Binding>,
    attempt: Option<Attempt>,
    delete: Option<(u64, String)>,
    session_loading: bool,
    session_owner: Option<Principal>,
    next_session_check: f64,
    account: Option<AccountContext>,
    launch: Option<sovereign_cloud::launch::CloudLaunch>,
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
        let auth = Rc::new(RefCell::new(None));
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
            auth,
            tx,
            rx,
            visible: launch_error.is_some() || launch.as_ref().is_ok_and(|v| v.is_some()),
            view: View::Projects,
            busy: false,
            signing_in: false,
            status: String::new(),
            progress: None,
            notice: launch_error.map(|message| Notice { message, error: true, expires: None }),
            pending_save: None,
            copy_mode: false,
            named_doc: None,
            name: "Untitled".into(),
            projects: vec![],
            bindings: BTreeMap::new(),
            attempt: None,
            delete: None,
            session_loading: true,
            session_owner: None,
            next_session_check: 0.0,
            account: None,
            launch: launch.ok().flatten(),
        }
    }
    pub fn menu_handle(&self) -> MenuHandle {
        self.menu.clone()
    }
    fn principal(&self) -> Option<Principal> {
        self.auth.borrow().as_ref().filter(|a| a.is_authenticated()).and_then(|a| a.principal().ok())
    }
    fn api(&self) -> CloudResult<Api> {
        Api::new(self.auth.borrow().as_ref().ok_or("Identity is still loading")?)
    }
    fn begin_operation(&mut self, message: impl Into<String>) {
        self.busy = true;
        self.status = message.into();
        self.progress = None;
        self.notice = None;
    }
    fn notify(&mut self, message: impl Into<String>, error: bool) {
        self.notice = Some(Notice { message: message.into(), error, expires: if error { None } else { Some(js_sys::Date::now() + 8000.0) } });
    }
    fn refresh(&mut self, ctx: &egui::Context) {
        let api = match self.api() {
            Ok(a) => a,
            Err(e) => {
                self.notify(e, true);
                return;
            }
        };
        self.begin_operation("Loading your files…");
        let stamp = self.auth.borrow().as_ref().map(|s| s.stamp().to_string()).unwrap_or_default();
        let (tx, ctx) = (self.tx.clone(), ctx.clone());
        let kind = self.spec.kind();
        wasm_bindgen_futures::spawn_local(async move {
            send_scoped(
                &tx,
                &ctx,
                &stamp,
                match async { Ok::<_, String>((api.account().await?, api.projects_for(&kind?).await?)) }.await {
                    Ok((account, p)) => Event::Projects(account, p),
                    Err(e) => Event::Error(e),
                },
            );
        });
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
    fn save(&mut self, app: &impl Editor, ctx: &egui::Context, copy: bool) {
        if self.busy || self.attempt.is_some() {
            return;
        }
        let Some(doc) = app.document() else {
            self.notify("Open or create a document first", true);
            return;
        };
        if !copy && self.bindings.get(&doc.id).is_some_and(|b| Some(b.owner) != self.principal()) {
            self.notify("This document belongs to another account. Sign back in, or choose File → Sovereign Cloud → Save As.", true);
            return;
        }
        let Some(owner) = self.principal() else {
            self.notify("Sign in before saving", true);
            return;
        };
        let binding = self.bindings.get(&doc.id).filter(|b| Some(b.owner) == self.principal());
        let project = if copy { None } else { binding.and_then(|b| self.projects.iter().find(|p| p.id == b.file)).cloned() };
        if !copy && binding.is_some() && project.is_none() {
            self.notify("This cloud file is unavailable. Refresh My files, or choose File → Sovereign Cloud → Save As.", true);
            return;
        }
        if flow::route(binding.map(|b| b.document_revision), doc.revision, copy) == flow::SaveRoute::AlreadySaved
            && project.as_ref().is_some_and(|p| binding.is_some_and(|b| p.revisions.iter().any(|r| r.id == b.version)))
        {
            self.view = View::Projects;
            self.notify("This file is already saved to cloud. No changes to upload.", false);
            return;
        }
        let name = project.as_ref().map_or_else(|| self.name.trim().to_string(), |p| p.name.clone());
        if name.is_empty() || name.len() > 160 {
            self.notify("Enter a file name of 1–160 bytes.", true);
            return;
        }
        let attempt = Attempt {
            owner,
            project,
            upload: None,
            expected: if copy { None } else { binding.map(|b| b.version) },
            bytes: Arc::new(Vec::new()),
            name,
            request: format!("{}-{}-{}", js_sys::Date::now(), doc.id, doc.revision),
            doc_id: doc.id,
            doc_revision: doc.revision,
        };
        // Capture the immutable document now; paint feedback before synchronous encoding.
        let snapshot = match app.capture() {
            Ok(snapshot) if snapshot.document.id == doc.id && snapshot.document.revision == doc.revision => snapshot,
            Ok(_) => {
                self.notify("The document changed while preparing. Try saving again.", true);
                return;
            }
            Err(e) => {
                self.notify(e, true);
                return;
            }
        };
        self.pending_save = Some(PendingSave { snapshot, attempt, ready_frame: ctx.cumulative_frame_nr().saturating_add(2) });
        self.view = View::Projects;
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
            Ok(export) if export.len() as u64 <= MAX_DOCUMENT_BYTES => {
                pending.attempt.bytes = Arc::new(export);
                self.attempt = Some(pending.attempt.clone());
                self.run_save(pending.attempt, ctx);
            }
            result => {
                self.busy = false;
                self.notify(
                    match result {
                        Ok(_) => "Cloud files can be up to 64 MiB".into(),
                        Err(e) => format!("Could not prepare your file: {e}"),
                    },
                    true,
                );
            }
        }
    }
    fn run_save(&mut self, mut attempt: Attempt, ctx: &egui::Context) {
        if Some(attempt.owner) != self.principal() {
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
            Ok(kind) => kind,
            Err(e) => {
                self.busy = false;
                self.notify(e, true);
                return;
            }
        };
        self.begin_operation("Connecting to your cloud…");
        let stamp = self.auth.borrow().as_ref().map(|s| s.stamp().to_string()).unwrap_or_default();
        let (tx, ctx) = (self.tx.clone(), ctx.clone());
        wasm_bindgen_futures::spawn_local(async move {
            let mut state = UploadState {
                file: attempt.project.clone(),
                upload: attempt.upload,
                expected_revision: attempt.expected,
                name: attempt.name.clone(),
                request_id: attempt.request.clone(),
            };
            let result = api
                .save_typed(&mut state, attempt.bytes.as_ref(), &kind, |event| match event {
                    TransferEvent::Checkpoint(state) => {
                        let mut checkpoint = attempt.clone();
                        checkpoint.project = state.file;
                        checkpoint.upload = state.upload;
                        send_scoped(&tx, &ctx, &stamp, Event::Prepared(checkpoint));
                    }
                    TransferEvent::Uploading { completed, total } => {
                        send_scoped(&tx, &ctx, &stamp, Event::Progress("Uploading your file…".into(), Some(completed as f32 / total.max(1) as f32)))
                    }
                    TransferEvent::Committing => send_scoped(&tx, &ctx, &stamp, Event::Progress("Finishing save…".into(), None)),
                    _ => {}
                })
                .await;
            attempt.project = state.file;
            attempt.upload = state.upload;
            send_scoped(
                &tx,
                &ctx,
                &stamp,
                match result {
                    Ok(r) => Event::Saved(attempt, r),
                    Err(e) => Event::Error(e),
                },
            );
        });
    }
    fn open(&mut self, project: Project, revision: Revision, ctx: &egui::Context) {
        let api = match self.api() {
            Ok(a) => a,
            Err(e) => {
                self.busy = false;
                self.notify(e, true);
                return;
            }
        };
        self.begin_operation(format!("Opening {}…", self.spec.file_name(&project.name)));
        let stamp = self.auth.borrow().as_ref().map(|s| s.stamp().to_string()).unwrap_or_default();
        let (tx, ctx) = (self.tx.clone(), ctx.clone());
        wasm_bindgen_futures::spawn_local(async move {
            let result = api
                .download(project.id, revision.id, |event| {
                    if let TransferEvent::Downloading { completed, total } = event {
                        send_scoped(&tx, &ctx, &stamp, Event::Progress("Downloading your file…".into(), Some(completed as f32 / total.max(1) as f32)));
                    }
                })
                .await;
            send_scoped(
                &tx,
                &ctx,
                &stamp,
                match result {
                    Ok((r, bytes)) => Event::Opened(project, r, bytes),
                    Err(e) => Event::Error(e),
                },
            );
        });
    }
    fn remove(&mut self, id: u64, method: &'static str, ctx: &egui::Context) {
        let api = match self.api() {
            Ok(a) => a,
            Err(e) => {
                self.notify(e, true);
                return;
            }
        };
        // Discarding an initial upload also removes its empty file record in one action.
        let empty_file = self.projects.iter().find(|p| p.pending_upload == Some(id) && p.revisions.is_empty()).map(|p| p.id);
        let file = if method == "delete_project" { Some(id) } else { empty_file };
        self.begin_operation("Removing cloud data…");
        let stamp = self.auth.borrow().as_ref().map(|s| s.stamp().to_string()).unwrap_or_default();
        let (tx, ctx) = (self.tx.clone(), ctx.clone());
        wasm_bindgen_futures::spawn_local(async move {
            let result: CloudResult<()> = async {
                api.update::<()>(method, args((id,))?).await?;
                if method != "delete_project"
                    && let Some(file) = empty_file
                {
                    api.update::<()>("delete_project", args((file,))?).await?;
                }
                Ok(())
            }
            .await;
            send_scoped(
                &tx,
                &ctx,
                &stamp,
                match result {
                    Ok(()) => Event::Removed { file, item: id },
                    Err(e) => Event::Error(e),
                },
            );
        });
    }
    fn poll(&mut self, app: &mut impl Editor, ctx: &egui::Context) {
        while let Some(request) = self.menu.pop() {
            let event = match request {
                Request::Save => Event::RequestSave(false),
                Request::SaveAs => Event::RequestSave(true),
                Request::Open => Event::RequestProjects,
            };
            let _ = self.tx.send(event);
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
                Event::RequestSave(copy) => {
                    self.visible = true;
                    if !self.busy && self.attempt.is_none() {
                        self.copy_mode = copy;
                        let binding = app.document().and_then(|d| self.bindings.get(&d.id));
                        if binding.is_some() && !copy {
                            self.save(app, ctx, false);
                        } else if let Some(doc) = app.document() {
                            self.view = View::Save;
                            let name = binding.and_then(|b| self.projects.iter().find(|p| p.id == b.file)).map_or(doc.name.as_str(), |p| p.name.as_str());
                            self.name = if copy { format!("{} copy", self.spec.stem(name)) } else { name.into() };
                            self.named_doc = Some(doc.id);
                        }
                    }
                }
                Event::RequestProjects => {
                    self.visible = true;
                    self.view = View::Projects;
                    if !self.busy && self.principal().is_some() {
                        self.refresh(ctx);
                    }
                }
                Event::SessionLoaded(result) => {
                    self.session_loading = false;
                    match result {
                        Ok(session) if session.is_current() => {
                            self.session_owner = session.principal().ok();
                            *self.auth.borrow_mut() = Some(session);
                            self.busy = false;
                            self.signing_in = false;
                            if self.principal().is_some() {
                                self.refresh(ctx);
                            }
                        }
                        Ok(_) => self.reload_session(ctx),
                        Err(e) => self.notify(format!("Identity initialization: {e}"), true),
                    }
                }
                Event::SignedIn => self.reload_session(ctx),
                Event::SignedOut => {
                    self.reload_session(ctx);
                    self.notify("Signed out. Your open files stay in the editor.", false);
                }
                Event::Scoped(_, _) => {}
                Event::Error(e) => {
                    self.busy = false;
                    self.signing_in = false;
                    self.notify(e.replace("Project", "File").replace("project", "file"), true);
                }
                Event::Progress(s, fraction) => {
                    self.status = s;
                    self.progress = fraction;
                }
                Event::Projects(account, p) => {
                    self.account = Some(account);
                    self.projects = p;
                    self.busy = false;
                    self.open_launch(ctx);
                }
                Event::Prepared(a) => self.attempt = Some(a),
                Event::Saved(a, r) => {
                    browser::notify_files_changed();
                    self.busy = false;
                    self.view = View::Projects;
                    self.attempt = None;
                    self.copy_mode = false;
                    if let Some(mut p) = a.project {
                        // A completed upload must not mark subsequent edits clean.
                        let saved_revision = match app.saved(a.doc_id, a.doc_revision, &a.bytes) {
                            Ok(revision) => revision.unwrap_or(a.doc_revision),
                            Err(error) => {
                                self.notify(format!("Cloud saved, but the editor could not update its saved state: {error}"), true);
                                a.doc_revision
                            }
                        };
                        self.bindings.insert(a.doc_id, Binding { owner: p.owner, file: p.id, version: r.id, document_revision: saved_revision });
                        self.notify(format!("Saved {} · {}", self.spec.file_name(&p.name), timestamp(r.created_at)), false);
                        p.pending_upload = None;
                        p.revisions.retain(|old| old.id != r.id);
                        p.revisions.push(r);
                        self.projects.retain(|old| old.id != p.id);
                        self.projects.push(p);
                    }
                }
                Event::Opened(p, r, bytes) => {
                    self.busy = false;
                    if app.replaces_unsaved() {
                        self.pending_open = Some((p, r, bytes));
                    } else {
                        self.finish_open(app, p, r, bytes);
                    }
                }
                Event::Removed { file, item } => {
                    browser::notify_files_changed();
                    self.busy = false;
                    if self.attempt.as_ref().is_some_and(|a| a.upload == Some(item) || (file.is_some() && a.project.as_ref().map(|p| p.id) == file)) {
                        self.attempt = None;
                    }
                    self.refresh(ctx);
                    self.notify("Cloud item removed", false);
                }
            }
        }
    }
    fn finish_open(&mut self, app: &mut impl Editor, project: Project, revision: Revision, bytes: Vec<u8>) {
        match app.open(&self.spec.file_name(&project.name), &bytes) {
            Ok(document) => {
                self.bindings
                    .insert(document.id, Binding { owner: project.owner, file: project.id, version: revision.id, document_revision: document.revision });
                self.copy_mode = false;
                self.notify(format!("Opened {} · saved {}", self.spec.file_name(&project.name), timestamp(revision.created_at)), false);
            }
            Err(e) => self.notify(format!("Could not open your file: {e}"), true),
        }
    }
}

/// Canister timestamps are nanoseconds; browser dates use milliseconds and the device timezone.
fn timestamp(nanos: u64) -> String {
    let date = js_sys::Date::new(&wasm_bindgen::JsValue::from_f64((nanos / 1_000_000) as f64));
    date.to_locale_string("en-GB", &wasm_bindgen::JsValue::UNDEFINED).into()
}
