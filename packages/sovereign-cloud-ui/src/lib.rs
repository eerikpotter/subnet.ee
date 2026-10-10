//! Shared cloud UI. Editors own their document formats and supply a small adapter.
#![forbid(unsafe_code)]
#![deny(clippy::unwrap_used, clippy::expect_used, clippy::panic, clippy::unimplemented, clippy::todo, clippy::unreachable)]

#[cfg(target_arch = "wasm32")]
mod controller;
pub mod flow;
pub mod presentation;
#[cfg(target_arch = "wasm32")]
pub use controller::Cloud;

use sovereign_cloud::{CloudResult, protocol::FileKind};
use std::{
    cell::{Cell, RefCell},
    collections::VecDeque,
    rc::Rc,
};

#[derive(Clone, Copy)]
pub struct AppSpec {
    pub id: &'static str,
    pub name: &'static str,
    pub extension: &'static str,
}
impl AppSpec {
    pub const PHOTO: Self = Self { id: "photocraft", name: "PhotoCraft", extension: "pcraft" };
    pub const PDF: Self = Self { id: "pdfcraft", name: "PdfCraft", extension: "pdf" };
    pub const WORD: Self = Self { id: "wordcraft", name: "WordCraft", extension: "wcraft.json" };
    pub const DECK: Self = Self { id: "deckcraft", name: "DeckCraft", extension: "deckcraft" };
    pub const SOUND: Self = Self { id: "soundcraft", name: "SoundCraft", extension: "scraft.zip" };
    pub fn kind(self) -> CloudResult<FileKind> {
        FileKind::for_app(self.id).ok_or_else(|| "This editor is not registered for cloud files".into())
    }
    pub fn file_name(self, name: &str) -> String {
        let name = name.trim();
        let suffix = format!(".{}", self.extension);
        if name.to_ascii_lowercase().ends_with(&suffix) { name.into() } else { format!("{name}{suffix}") }
    }
    pub fn stem(self, name: &str) -> String {
        let suffix = format!(".{}", self.extension);
        if name.to_ascii_lowercase().ends_with(&suffix) { name.get(..name.len().saturating_sub(suffix.len())).unwrap_or(name).into() } else { name.into() }
    }
}

#[derive(Clone, Debug)]
pub struct DocumentState {
    /// Stable for the open document's lifetime, distinct after New/Open.
    pub id: u64,
    /// Changes whenever editable content changes, including undo and redo.
    pub revision: u64,
    pub name: String,
    pub dirty: bool,
}

pub struct Snapshot {
    pub document: DocumentState,
    /// Owns an immutable snapshot, so edits made during upload are never marked saved.
    pub encode: Box<dyn FnOnce() -> CloudResult<Vec<u8>>>,
}

pub trait Editor {
    fn document(&self) -> Option<DocumentState>;
    fn capture(&self) -> CloudResult<Snapshot>;
    /// Returns the resulting revision only when this exact snapshot is still current.
    fn saved(&mut self, id: u64, revision: u64, bytes: &[u8]) -> CloudResult<Option<u64>>;
    /// Parse before replacing anything; errors must preserve the user's existing work.
    fn open(&mut self, name: &str, bytes: &[u8]) -> CloudResult<DocumentState>;
    fn replaces_unsaved(&self) -> bool {
        false
    }
    fn set_save_enabled(&mut self, _enabled: bool) {}
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Request {
    Save,
    SaveAs,
    Open,
}

/// The same menu section is installed by each browser host into its editor's File UI.
#[derive(Clone)]
pub struct MenuHandle {
    queue: Rc<RefCell<VecDeque<Request>>>,
    enabled: Rc<Cell<bool>>,
    ctx: egui::Context,
}
impl MenuHandle {
    pub fn new(ctx: egui::Context) -> Self {
        Self { queue: Rc::new(RefCell::new(VecDeque::new())), enabled: Rc::new(Cell::new(true)), ctx }
    }
    pub fn request(&self, request: Request) {
        self.queue.borrow_mut().push_back(request);
        self.ctx.request_repaint();
    }
    pub fn ui(&self, ui: &mut egui::Ui, has_document: bool) {
        ui.separator();
        presentation::header(ui);
        for (label, request) in [("Save", Request::Save), ("Save As…", Request::SaveAs), ("Open…", Request::Open)] {
            let enabled = request == Request::Open || (has_document && self.enabled.get());
            if ui.add_enabled(enabled, egui::Button::new(label)).clicked() {
                self.request(request);
                ui.close();
            }
        }
    }
    #[cfg(target_arch = "wasm32")]
    fn pop(&self) -> Option<Request> {
        self.queue.borrow_mut().pop_front()
    }
    #[cfg(target_arch = "wasm32")]
    fn set_enabled(&self, enabled: bool) {
        self.enabled.set(enabled);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use egui_kittest::{Harness, kittest::Queryable};
    #[test]
    fn shared_menu_guards_saves_and_dispatches_all_three_actions() {
        let handle = MenuHandle::new(egui::Context::default());
        let mut h = Harness::builder().build_ui_state(|ui, has_document| handle.ui(ui, *has_document), false);
        h.get_by_label("Save").click();
        h.run_steps(3);
        assert!(handle.queue.borrow().is_empty());
        h.get_by_label("Open…").click();
        h.run_steps(3);
        assert_eq!(handle.queue.borrow_mut().pop_front(), Some(Request::Open));
        *h.state_mut() = true;
        h.run_steps(3);
        for (label, request) in [("Save", Request::Save), ("Save As…", Request::SaveAs)] {
            h.get_by_label(label).click();
            h.run_steps(3);
            assert_eq!(handle.queue.borrow_mut().pop_front(), Some(request));
        }
        handle.enabled.set(false);
        h.run_steps(3);
        h.get_by_label("Save As…").click();
        h.run_steps(3);
        assert!(handle.queue.borrow().is_empty());
    }
    #[test]
    #[allow(clippy::unwrap_used)]
    fn every_registered_editor_has_a_distinct_kind_and_native_file_name() {
        let mut kinds = std::collections::BTreeSet::new();
        for app in [AppSpec::PHOTO, AppSpec::PDF, AppSpec::WORD, AppSpec::DECK, AppSpec::SOUND] {
            let kind = app.kind().unwrap();
            assert!(kinds.insert(kind.format_id));
            let file = app.file_name("Test");
            assert_eq!(app.file_name(&file), file);
            assert_eq!(app.stem(&file), "Test");
        }
    }
}
