//! Shared native-format recovery lifecycle. Session changes never relabel bytes.
use super::{AppSpec, Editor, Principal, drafts};
use crate::locale::tr;
use drafts::Header;
use sovereign_cloud::CloudResult;
use std::{collections::BTreeMap, sync::mpsc};
enum Event {
    Listed(String, bool, CloudResult<Vec<Header>>),
    Written(String, u64, CloudResult<Header>),
    Read(String, Header, CloudResult<Vec<u8>>),
    Removed(String, CloudResult<()>),
}
pub(super) struct Recovery {
    prefix: Option<String>,
    actor: Option<Principal>,
    nonce: String,
    owners: BTreeMap<u64, Principal>,
    keys: BTreeMap<(Principal, u64), String>,
    captured: BTreeMap<(Principal, u64), Header>,
    saved: BTreeMap<(Principal, u64), u64>,
    origins: BTreeMap<(Principal, u64), Header>,
    observed: Option<(u64, u64)>,
    changed: f64,
    last: f64,
    busy: bool,
    entries: Vec<Header>,
    ready: Option<(Header, Vec<u8>)>,
    deleting: Option<Header>,
    remove: Vec<Header>,
    visible: bool,
    pub(super) error: Option<String>,
    tx: mpsc::Sender<Event>,
    rx: mpsc::Receiver<Event>,
}
impl Recovery {
    pub(super) fn new() -> Self {
        let (tx, rx) = mpsc::channel();
        let nonce = web_sys::window().and_then(|w| w.crypto().ok()).map(|c| c.random_uuid()).unwrap_or_default();
        Self {
            prefix: None,
            actor: None,
            nonce,
            owners: BTreeMap::new(),
            keys: BTreeMap::new(),
            captured: BTreeMap::new(),
            origins: BTreeMap::new(),
            saved: BTreeMap::new(),
            observed: None,
            changed: 0.,
            last: 0.,
            busy: false,
            entries: vec![],
            ready: None,
            deleting: None,
            remove: vec![],
            visible: false,
            error: None,
            tx,
            rx,
        }
    }
    fn list(&self, ctx: &egui::Context, offer: bool) {
        if let Some(prefix) = self.prefix.clone() {
            let (tx, ctx) = (self.tx.clone(), ctx.clone());
            wasm_bindgen_futures::spawn_local(async move {
                let result = drafts::list(&prefix).await;
                let _ = tx.send(Event::Listed(prefix, offer, result));
                ctx.request_repaint();
            });
        }
    }
    pub(super) fn saved(&mut self, actor: Principal, doc: u64, revision: u64) {
        self.owners.insert(doc, actor);
        self.saved.insert((actor, doc), revision);
        if let Some(header) = self.captured.get(&(actor, doc)).filter(|h| h.revision == revision) {
            self.remove.push(header.clone());
        }
        if let Some(header) = self.origins.remove(&(actor, doc)) {
            self.remove.push(header);
        }
    }
    pub(super) fn tick(&mut self, actor: Option<Principal>, spec: AppSpec, app: &impl Editor, ctx: &egui::Context, allow: bool) {
        if self.actor != actor {
            self.actor = actor;
            self.prefix = actor.and_then(|a| sovereign_cloud::browser::endpoint().ok().map(|e| format!("{}:{a}:{}:", e.canister, spec.id)));
            self.entries.clear();
            self.ready = None;
            self.deleting = None;
            self.remove.clear();
            self.visible = false;
            self.error = None;
            self.observed = None;
            self.list(ctx, true);
        }
        while let Ok(event) = self.rx.try_recv() {
            match event {
                Event::Listed(prefix, offer, result) if self.prefix.as_ref() == Some(&prefix) => match result {
                    Ok(entries) => {
                        self.visible |= offer && !entries.is_empty();
                        self.entries = entries;
                    }
                    Err(e) => self.error = Some(e),
                },
                Event::Written(prefix, doc, result) => {
                    self.busy = false;
                    if self.prefix.as_ref() == Some(&prefix) {
                        match result {
                            Ok(header) => {
                                if let Some(actor) = actor {
                                    self.captured.insert((actor, doc), header.clone());
                                    if self.saved.get(&(actor, doc)) == Some(&header.revision) {
                                        self.remove.push(header.clone());
                                    }
                                }
                                self.entries.retain(|h| h.key != header.key);
                                self.entries.insert(0, header);
                                self.error = None;
                            }
                            Err(e) => self.error = Some(e),
                        }
                    }
                }
                Event::Read(prefix, header, result) => {
                    self.busy = false;
                    if self.prefix.as_ref() == Some(&prefix) {
                        match result {
                            Ok(bytes) => self.ready = Some((header, bytes)),
                            Err(e) => self.error = Some(e),
                        }
                    }
                }
                Event::Removed(prefix, result) => {
                    self.busy = false;
                    if self.prefix.as_ref() == Some(&prefix) {
                        if let Err(e) = result {
                            self.error = Some(e);
                        }
                        self.list(ctx, false);
                    }
                }
                _ => {}
            }
        }
        if self.busy {
            return;
        }
        if let (Some(header), Some(prefix)) = (self.remove.pop(), self.prefix.clone()) {
            self.busy = true;
            let (tx, ctx) = (self.tx.clone(), ctx.clone());
            wasm_bindgen_futures::spawn_local(async move {
                let result = drafts::remove(&prefix, &header.key, header.at).await;
                let _ = tx.send(Event::Removed(prefix, result));
                ctx.request_repaint();
            });
            return;
        }
        let (Some(actor), Some(prefix), Some(doc)) = (actor, self.prefix.clone(), app.document()) else {
            return;
        };
        let owner = self.owners.entry(doc.id).or_insert(actor);
        if *owner != actor || self.nonce.is_empty() || !allow || self.ready.is_some() || !doc.dirty {
            return;
        }
        ctx.request_repaint_after(std::time::Duration::from_secs(1));
        let now = js_sys::Date::now();
        if self.observed != Some((doc.id, doc.revision)) {
            self.observed = Some((doc.id, doc.revision));
            self.changed = now;
        }
        if now - self.changed < 3000. || now - self.last < 10000. || self.captured.get(&(actor, doc.id)).is_some_and(|h| h.revision == doc.revision) {
            return;
        }
        self.last = now;
        let snapshot = match app.capture() {
            Ok(s) => s,
            Err(e) => {
                self.error = Some(e);
                return;
            }
        };
        let doc = snapshot.document;
        let bytes = match (snapshot.encode)() {
            Ok(b) => b,
            Err(e) => {
                self.error = Some(e);
                return;
            }
        };
        let key = self.keys.entry((actor, doc.id)).or_insert_with(|| format!("{prefix}{}:{}", self.nonce, doc.id)).clone();
        let header = Header { key, name: spec.file_name(&doc.name), revision: doc.revision, bytes: bytes.len() as u64, at: now };
        self.busy = true;
        let (tx, ctx) = (self.tx.clone(), ctx.clone());
        wasm_bindgen_futures::spawn_local(async move {
            let result = drafts::write(&prefix, &header, &bytes).await.map(|()| header);
            let _ = tx.send(Event::Written(prefix, doc.id, result));
            ctx.request_repaint();
        });
    }
    pub(super) fn ui(&mut self, app: &mut impl Editor, ctx: &egui::Context, status: &str, allow: bool) -> bool {
        let mut show_cloud = false;
        if self.actor.is_none() {
            return false;
        }
        egui::Area::new(egui::Id::new("cloud.recovery.available")).anchor(egui::Align2::RIGHT_BOTTOM, egui::vec2(-16., -16.)).show(ctx, |ui| {
            egui::Frame::popup(ui.style()).show(ui, |ui| {
                ui.horizontal(|ui| {
                    if ui.small_button(tr(status)).clicked() {
                        show_cloud = true;
                    }
                    if !self.entries.is_empty() || self.error.is_some() {
                        if ui.small_button(tr(if self.error.is_some() { "Local recovery needs attention" } else { "Local drafts" })).clicked() {
                            self.visible = true;
                        }
                    }
                });
            });
        });
        let mut visible = self.visible;
        egui::Window::new(tr("Local drafts")).id(egui::Id::new("cloud.recovery")).open(&mut visible).resizable(true).default_width(430.).show(ctx, |ui| {
            ui.label(tr("Drafts stay in this browser for this account. Restore as a new copy, then choose Save As. Clearing browser storage removes them."));
            if let Some(error) = &self.error {
                ui.colored_label(ui.visuals().error_fg_color, tr(error));
            }
            if self.busy {
                ui.spinner();
            } else if ui.button(tr("Refresh")).clicked() {
                self.list(ctx, false);
            }
            for header in self.entries.clone() {
                ui.horizontal_wrapped(|ui| {
                    ui.label(&header.name);
                    ui.weak(format!("{} · {} KiB", super::timestamp((header.at * 1_000_000.) as u64), header.bytes.div_ceil(1024)));
                    if ui.add_enabled(allow && !self.busy, egui::Button::new(tr("Restore copy"))).clicked() {
                        if let Some(prefix) = self.prefix.clone() {
                            self.busy = true;
                            let (tx, ctx) = (self.tx.clone(), ctx.clone());
                            wasm_bindgen_futures::spawn_local(async move {
                                let result = drafts::read(&prefix, &header).await;
                                let _ = tx.send(Event::Read(prefix, header, result));
                                ctx.request_repaint();
                            });
                        }
                    } else if ui.add_enabled(allow && !self.busy, egui::Button::new(tr("Discard draft"))).clicked() {
                        self.deleting = Some(header);
                    }
                });
            }
        });
        self.visible = visible;
        if self.ready.is_some() {
            egui::Window::new(tr("Restore draft?")).collapsible(false).show(ctx, |ui| {
                ui.label(tr("Restoring replaces the open document. Save or export its unsaved work first. The cloud file will not be overwritten."));
                if ui.add_enabled(allow, egui::Button::new(tr("Restore as a copy"))).clicked() {
                    if let Some((header, bytes)) = self.ready.take() {
                        match app.open(&header.name, &bytes) {
                            Ok(doc) => {
                                if let Some(actor) = self.actor {
                                    self.owners.insert(doc.id, actor);
                                    self.origins.insert((actor, doc.id), header);
                                }
                                self.visible = false;
                            }
                            Err(e) => self.error = Some(e),
                        }
                    }
                }
                if ui.button(tr("Cancel")).clicked() {
                    self.ready = None;
                }
            });
        }
        if let Some(header) = self.deleting.clone() {
            egui::Window::new(tr("Discard local draft?")).collapsible(false).show(ctx, |ui| {
                ui.label(&header.name);
                ui.label(tr("This permanently removes this local recovery copy. Cloud files are unchanged."));
                if ui.button(tr("Discard draft")).clicked() {
                    self.remove.push(header);
                    self.deleting = None;
                }
                if ui.button(tr("Cancel")).clicked() {
                    self.deleting = None;
                }
            });
        }
        show_cloud
    }
}
