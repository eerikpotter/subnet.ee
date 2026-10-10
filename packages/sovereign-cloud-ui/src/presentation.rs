//! Platform-independent cloud controls, shared by the browser and native UI tests.
use egui::Color32;

pub struct Palette {
    pub dock: Color32,
    pub card: Color32,
    pub card_border: Color32,
    pub field: Color32,
    pub field_border: Color32,
    pub hover: Color32,
    pub pressed: Color32,
    pub text: Color32,
    pub text_dim: Color32,
    pub accent: Color32,
    pub accent_soft: Color32,
    pub accent_border: Color32,
    pub accent_text: Color32,
    pub warning: Color32,
    pub radius_sm: f32,
    pub radius_lg: f32,
    dark: bool,
}
impl Palette {
    pub fn dark(&self) -> bool {
        self.dark
    }
}
pub fn palette(ctx: &egui::Context) -> Palette {
    let dark = ctx.global_style().visuals.dark_mode;
    let rgb = |a: [u8; 3], b: [u8; 3]| {
        let c = if dark { a } else { b };
        Color32::from_rgb(c[0], c[1], c[2])
    };
    Palette {
        dock: rgb([17, 17, 18], [240, 240, 243]),
        card: rgb([26, 26, 28], [252, 252, 253]),
        card_border: rgb([40, 40, 44], [222, 222, 228]),
        field: rgb([35, 35, 38], [242, 242, 245]),
        field_border: rgb([52, 52, 57], [214, 214, 220]),
        hover: rgb([44, 44, 48], [232, 232, 237]),
        pressed: rgb([56, 56, 62], [220, 220, 226]),
        text: rgb([236, 236, 240], [24, 24, 28]),
        text_dim: rgb([150, 150, 158], [96, 96, 106]),
        accent: rgb([139, 124, 246], [108, 92, 231]),
        accent_soft: rgb([46, 41, 67], [231, 228, 250]),
        accent_border: rgb([96, 86, 144], [174, 163, 239]),
        accent_text: rgb([214, 208, 255], [80, 64, 200]),
        warning: rgb([240, 190, 90], [190, 130, 20]),
        radius_sm: 6.0,
        radius_lg: 12.0,
        dark,
    }
}

/// Scope the cloud's visual identity to this UI. The editor's theme is untouched.
pub fn style(ui: &mut egui::Ui) {
    let t = palette(ui.ctx());
    let s = ui.style_mut();
    s.spacing.item_spacing = egui::vec2(10.0, 8.0);
    s.visuals = if t.dark() { egui::Visuals::dark() } else { egui::Visuals::light() };
    s.visuals.override_text_color = Some(t.text);
    s.visuals.weak_text_color = Some(t.text_dim);
    s.visuals.panel_fill = t.card;
    s.visuals.window_fill = t.card;
    s.visuals.extreme_bg_color = t.field;
    s.visuals.selection.bg_fill = t.accent_soft;
    s.visuals.selection.stroke = egui::Stroke::new(1.0, t.accent_text);
    s.visuals.widgets.inactive.bg_fill = t.field;
    s.visuals.widgets.inactive.weak_bg_fill = t.field;
    s.visuals.widgets.inactive.bg_stroke = egui::Stroke::new(1.0, t.field_border);
    s.visuals.widgets.hovered.bg_fill = t.hover;
    s.visuals.widgets.hovered.weak_bg_fill = t.hover;
    s.visuals.widgets.hovered.bg_stroke = egui::Stroke::new(1.0, t.accent_border);
    s.visuals.widgets.active.bg_fill = t.pressed;
    s.visuals.widgets.active.weak_bg_fill = t.pressed;
    for w in [&mut s.visuals.widgets.inactive, &mut s.visuals.widgets.hovered, &mut s.visuals.widgets.active] {
        w.corner_radius = egui::CornerRadius::same(t.radius_sm as u8);
    }
}

pub fn section(ui: &egui::Ui) -> egui::Frame {
    let t = palette(ui.ctx());
    egui::Frame::new().fill(t.card).stroke(egui::Stroke::new(1.0, t.card_border)).corner_radius(t.radius_lg).inner_margin(16.0)
}

pub fn primary(ui: &mut egui::Ui, label: &str) -> egui::Response {
    ui.add(egui::Button::new(egui::RichText::new(label).color(Color32::WHITE)).fill(Color32::from_rgb(46, 110, 232)).min_size(egui::vec2(128.0, 34.0)))
}

pub fn header(ui: &mut egui::Ui) {
    ui.horizontal(|ui| {
        ui.add(egui::Image::new(egui::include_image!("../assets/icp/logo.svg")).fit_to_exact_size(egui::vec2(24.0, 14.0)));
        ui.weak("Sovereign Cloud");
    });
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum SaveAction {
    Save,
    Cancel,
}

pub fn size_label(bytes: u64) -> String {
    if bytes >= 1024 * 1024 { format!("{:.1} MiB", bytes as f64 / (1024.0 * 1024.0)) } else { format!("{} KiB", bytes.div_ceil(1024)) }
}

/// First-save/copy naming dialog, invoked only by a File-menu request.
pub fn save_form(ui: &mut egui::Ui, name: &mut String, copy: bool, enabled: bool) -> Option<SaveAction> {
    let mut action = None;
    ui.add_enabled_ui(enabled, |ui| {
        ui.heading(if copy { "Save As to Cloud" } else { "Save to Cloud" });
        ui.weak(if copy { "Save under a new name. Future cloud saves will update this new file." } else { "Name this file for your cloud storage." });
        ui.add_space(14.0);
        ui.label("File name");
        ui.add(egui::TextEdit::singleline(name).hint_text("Untitled").desired_width(f32::INFINITY));
        if name.trim().len() > 160 {
            ui.colored_label(palette(ui.ctx()).warning, "Choose a shorter file name (160 bytes maximum).");
        }
        ui.add_space(12.0);
        let valid = !name.trim().is_empty() && name.trim().len() <= 160;
        ui.horizontal(|ui| {
            if ui.add_enabled_ui(valid, |ui| primary(ui, "Save")).inner.clicked() {
                action = Some(SaveAction::Save);
            }
            if ui.button("Cancel").clicked() {
                action = Some(SaveAction::Cancel);
            }
        });
    });
    action
}

pub fn transfer(ui: &mut egui::Ui, message: &str, progress: Option<f32>) {
    let t = palette(ui.ctx());
    section(ui).fill(t.accent_soft).stroke(egui::Stroke::new(1.0, t.accent_border)).show(ui, |ui| {
        ui.set_min_width(ui.available_width());
        ui.horizontal(|ui| {
            ui.add(egui::Spinner::new().size(20.0).color(t.accent));
            ui.strong(message);
        });
        if let Some(fraction) = progress {
            ui.add(egui::ProgressBar::new(fraction.clamp(0.0, 1.0)).show_percentage().desired_width(ui.available_width()));
        }
    });
    ui.ctx().request_repaint_after(std::time::Duration::from_millis(50));
}

/// A labelled notification, never a file card. Returns true on dismiss.
pub fn notification(ui: &mut egui::Ui, message: &str, error: bool) -> bool {
    let t = palette(ui.ctx());
    let mut dismiss = false;
    ui.separator();
    ui.horizontal(|ui| {
        ui.colored_label(if error { t.warning } else { t.accent }, if error { "Needs attention" } else { "Activity" });
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            dismiss = ui.small_button("Dismiss").clicked();
        });
    });
    ui.label(message);
    dismiss
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum RevisionAction {
    Open,
    Delete,
}
pub fn revision_row(ui: &mut egui::Ui, timestamp: &str, bytes: u64, latest: bool) -> Option<RevisionAction> {
    let mut action = None;
    ui.horizontal(|ui| {
        ui.vertical(|ui| {
            ui.label(timestamp);
            ui.weak(format!("{}{}", if latest { "Latest version · " } else { "" }, size_label(bytes)));
        });
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            ui.menu_button("…", |ui| {
                if ui.button("Delete version…").clicked() {
                    action = Some(RevisionAction::Delete);
                    ui.close();
                }
            });
            if ui.button("Open").clicked() {
                action = Some(RevisionAction::Open);
            }
        });
    });
    action
}

const FILE_ROW_HEIGHT: f32 = 44.0;
const FILE_DATE_WIDTH: f32 = 150.0;
const FILE_OPEN_WIDTH: f32 = 82.0;

fn file_columns(rect: egui::Rect) -> (egui::Rect, egui::Rect, egui::Rect) {
    let open = egui::Rect::from_min_max(
        egui::pos2((rect.right() - FILE_OPEN_WIDTH - 8.0).max(rect.left()), rect.top() + 7.0),
        egui::pos2(rect.right() - 8.0, rect.bottom() - 7.0),
    );
    let date = egui::Rect::from_min_max(
        egui::pos2((open.left() - 10.0 - FILE_DATE_WIDTH).max(rect.left() + 32.0), rect.top()),
        egui::pos2(open.left() - 10.0, rect.bottom()),
    );
    let name = egui::Rect::from_min_max(egui::pos2(rect.left() + 32.0, rect.top()), egui::pos2((date.left() - 10.0).max(rect.left() + 32.0), rect.bottom()));
    (name, date, open)
}

pub fn file_column_labels(ui: &mut egui::Ui) {
    let (rect, _) = ui.allocate_exact_size(egui::vec2(ui.available_width(), 20.0), egui::Sense::hover());
    let (name, date, _) = file_columns(rect);
    file_cell(ui, name, egui::RichText::new("Name").small().weak());
    file_cell(ui, date, egui::RichText::new("Last saved").small().weak());
}

fn file_cell(ui: &mut egui::Ui, rect: egui::Rect, text: egui::RichText) {
    ui.new_child(egui::UiBuilder::new().max_rect(rect).layout(egui::Layout::left_to_right(egui::Align::Center)))
        .add(egui::Label::new(text).truncate().selectable(false));
}

/// One file, one row. The name/date area expands details; Open is a separate hit target.
/// Persistent numeric IDs keep expansion attached to the file across refreshes and sorting.
pub fn file_row(ui: &mut egui::Ui, id: u64, name: &str, timestamp: &str, add_details: impl FnOnce(&mut egui::Ui)) -> bool {
    ui.push_id(("cloud.file", id), |ui| {
        let t = palette(ui.ctx());
        let mut state = egui::collapsing_header::CollapsingState::load_with_default_open(ui.ctx(), ui.make_persistent_id("details"), false);
        let (rect, _) = ui.allocate_exact_size(egui::vec2(ui.available_width(), FILE_ROW_HEIGHT), egui::Sense::hover());
        let (name_rect, date_rect, open_rect) = file_columns(rect);
        let details_rect = egui::Rect::from_min_max(rect.min, egui::pos2(open_rect.left() - 4.0, rect.bottom()));
        let mut details = ui.interact(details_rect, state.id(), egui::Sense::click());
        if details.clicked() {
            details.request_focus();
            state.toggle(ui);
            details.mark_changed();
        }
        details.widget_info(|| egui::WidgetInfo::selected(egui::WidgetType::CollapsingHeader, ui.is_enabled(), state.is_open(), format!("Details for {name}")));
        let highlighted = state.is_open() || details.has_focus();
        let fill = if details.hovered() {
            t.hover
        } else if highlighted {
            t.accent_soft
        } else {
            t.field
        };
        ui.painter().rect_filled(rect, t.radius_sm, fill);
        ui.painter().rect_stroke(
            rect,
            t.radius_sm,
            egui::Stroke::new(1.0, if highlighted { t.accent_border } else { t.field_border }),
            egui::StrokeKind::Inside,
        );
        let icon = egui::Rect::from_center_size(egui::pos2(rect.left() + 16.0, rect.center().y), egui::vec2(12.0, 12.0));
        egui::collapsing_header::paint_default_icon(ui, state.openness(ui.ctx()), &details.clone().with_new_rect(icon));
        file_cell(ui, name_rect, egui::RichText::new(name).strong());
        file_cell(ui, date_rect, egui::RichText::new(timestamp).small().weak());
        details
            .on_hover_cursor(egui::CursorIcon::PointingHand)
            .on_hover_text(format!("{name}\nLast saved {timestamp}\nClick to {} version history", if state.is_open() { "hide" } else { "show" }));
        let open = ui.put(open_rect, egui::Button::new("Open file")).on_hover_text(format!("Open the latest saved version of {name}")).clicked();
        state.show_body_unindented(ui, |ui| {
            egui::Frame::new().fill(t.card).stroke(egui::Stroke::new(1.0, t.card_border)).corner_radius(t.radius_sm).inner_margin(14.0).show(ui, |ui| {
                ui.set_min_width(ui.available_width());
                add_details(ui);
            });
        });
        open
    })
    .inner
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;
    use egui_kittest::{Harness, kittest::Queryable};

    #[test]
    fn naming_dialog_confirms_once_and_can_be_cancelled() {
        for copy in [false, true] {
            let mut h = Harness::builder().with_size(egui::vec2(460.0, 280.0)).build_ui_state(
                |ui, action: &mut Option<SaveAction>| {
                    if let Some(a) = save_form(ui, &mut "Poster".to_string(), copy, true) {
                        *action = Some(a);
                    }
                },
                None,
            );
            h.get_by_label("Save").click();
            h.run_steps(3);
            assert_eq!(*h.state(), Some(SaveAction::Save));
            h.get_by_label("Cancel").click();
            h.run_steps(3);
            assert_eq!(*h.state(), Some(SaveAction::Cancel));
        }
    }

    #[test]
    fn saving_and_invalid_names_cannot_start_another_save() {
        for (name, enabled) in [("Poster", false), ("   ", true), (&"a".repeat(161), true)] {
            let mut h = Harness::builder().build_ui_state(
                |ui, action: &mut Option<SaveAction>| {
                    *action = save_form(ui, &mut name.to_string(), false, enabled);
                },
                None,
            );
            h.get_by_label("Save").click();
            h.run_steps(3);
            assert!(h.state().is_none());
        }
    }

    #[test]
    fn file_row_expands_from_date_or_keyboard_and_open_does_not_toggle_it() {
        let mut h = Harness::builder().with_size(egui::vec2(720.0, 360.0)).build_ui_state(
            |ui, opens: &mut usize| {
                style(ui);
                if file_row(ui, 7, "Poster", "07/10/2026, 10:25:00", |ui| {
                    ui.label("Version history");
                }) {
                    *opens += 1;
                }
            },
            0,
        );
        h.run_steps(3);
        assert!(h.query_by_label("Version history").is_none());
        h.get_by_label("Open file").click();
        h.run_steps(3);
        assert_eq!(*h.state(), 1);
        assert!(h.query_by_label("Version history").is_none());
        // The date is part of the expansion target, not a separate inert cell.
        h.get_by_label("07/10/2026, 10:25:00").click();
        h.run_steps(20);
        assert!(h.query_by_label("Version history").is_some());
        assert_eq!(*h.state(), 1);
        h.get_by_label("Details for Poster").click();
        h.run_steps(20);
        assert!(h.query_by_label("Version history").is_none());
        // Clicking the header also gives it keyboard focus.
        h.key_press(egui::Key::Enter);
        h.run_steps(20);
        assert!(h.query_by_label("Version history").is_some());
        h.get_by_label("Open file").click();
        h.run_steps(3);
        assert_eq!(*h.state(), 2);
        assert!(h.query_by_label("Version history").is_some());
    }

    #[test]
    fn file_expansion_follows_id_when_rows_are_reordered() {
        let mut h = Harness::builder().with_size(egui::vec2(720.0, 360.0)).build_ui_state(
            |ui, reverse: &mut bool| {
                let mut rows = [(1, "Poster"), (2, "Landscape")];
                if *reverse {
                    rows.reverse();
                }
                for (id, name) in rows {
                    file_row(ui, id, name, "07/10/2026, 10:25:00", |ui| {
                        ui.label(format!("History of {name}"));
                    });
                }
            },
            false,
        );
        h.get_by_label("Details for Poster").click();
        h.run_steps(20);
        assert!(h.query_by_label("History of Poster").is_some());
        assert!(h.query_by_label("History of Landscape").is_none());
        *h.state_mut() = true;
        h.run_steps(3);
        assert!(h.query_by_label("History of Poster").is_some());
        assert!(h.query_by_label("History of Landscape").is_none());
    }

    #[test]
    fn cloud_views_render_in_all_themes() {
        let output = std::env::var("CRAFT_CLOUD_PREVIEWS").ok();
        for (index, theme) in [egui::Visuals::dark(), egui::Visuals::light()].into_iter().enumerate() {
            for width in [420.0, 760.0] {
                for view in ["files", "saving"] {
                    let builder = Harness::builder().with_size(egui::vec2(width, 560.0));
                    let builder = if output.is_some() { builder.wgpu() } else { builder };
                    let mut h = builder.build_ui(|ui| {
                        style(ui);
                        header(ui);
                        ui.add_space(8.0);
                        if view == "files" {
                            ui.heading("My files");
                            ui.weak("4 files · Times shown in your local timezone");
                            file_column_labels(ui);
                            for (id, name, time) in [
                                (1, "Poster", "07/10/2026, 10:25:00"),
                                (2, "Landscape study", "06/10/2026, 18:10:00"),
                                (3, "Brand exploration — a deliberately long file name", "05/10/2026, 09:45:00"),
                                (4, "色彩 — värvid", "04/10/2026, 12:30:00"),
                            ] {
                                file_row(ui, id, name, time, |ui| {
                                    ui.strong("Version history (2)");
                                    revision_row(ui, time, 3 * 1024 * 1024, true);
                                });
                            }
                        } else {
                            ui.heading("My files");
                            ui.weak("Your saved files will appear here.");
                        }
                        ui.add_space(12.0);
                        if view == "saving" {
                            transfer(ui, "Uploading your file…", Some(0.625));
                        } else {
                            notification(ui, "Opened Poster.pcraft · saved 07/10/2026, 01:25:00", false);
                        }
                    });
                    h.ctx.set_visuals(theme.clone());
                    h.run_steps(3);
                    if view == "files" {
                        assert!(h.query_by_label("Details for Poster").is_some());
                        assert!(h.query_by_label("Version history (2)").is_none());
                    } else {
                        assert!(h.query_by_label("Save").is_none());
                        assert!(h.query_by_label("Save file").is_none());
                    }
                    if let Some(dir) = &output {
                        h.render().unwrap().save(format!("{dir}/cloud-{view}-{index}-{width}.png")).unwrap();
                    }
                    if view == "files" {
                        h.get_by_label("Details for Landscape study").click();
                        h.run_steps(20);
                        assert!(h.query_by_label("Version history (2)").is_some());
                        if let Some(dir) = &output {
                            h.render().unwrap().save(format!("{dir}/cloud-expanded-{index}-{width}.png")).unwrap();
                        }
                    }
                }
            }
        }
    }

    #[test]
    fn file_names_do_not_duplicate_native_extension() {
        assert_eq!(crate::AppSpec::PHOTO.file_name(" Poster "), "Poster.pcraft");
        assert_eq!(crate::AppSpec::PHOTO.file_name("Poster.pcraft"), "Poster.pcraft");
        assert_eq!(crate::AppSpec::PHOTO.file_name("Poster.PCRAFT"), "Poster.PCRAFT");
    }
}
