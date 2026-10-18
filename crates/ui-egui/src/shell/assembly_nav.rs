//! The Assembly Navigator (Advanced approach): the components of the part on show, each one a
//! part file linked in (or a component kept inside this file), with where its file is, whether
//! the file is there, how many times it is placed, and what to do with it.

use egui::{RichText, vec2};
use serde_json::json;

use crate::SolveApp;
use crate::theme::Tokens;

/// One component as the navigator lists it.
#[derive(Clone, Debug, PartialEq)]
pub struct Row {
    pub component: u64,
    pub name: String,
    pub depth: usize,
    /// The part file's address as the link stores it, for a linked component.
    pub path: Option<String>,
    /// The part file, resolved against this design's folder.
    pub file: Option<String>,
    /// The part file exists (always true for a component kept inside this file).
    pub found: bool,
    /// Placements of the component.
    pub count: usize,
    pub grounded: bool,
}

/// The components of the design, parents before their children.
pub fn rows(app: &SolveApp) -> Vec<Row> {
    let doc = &app.session.doc;
    // A link's path is relative to this design's folder once the design has a file.
    let folder = app.session.path.as_deref().and_then(|p| std::path::Path::new(p).parent().map(std::path::Path::to_path_buf));
    let linked = |component: u64| -> Option<(String, String, bool)> {
        let link = doc.links.get(&component)?;
        let given = std::path::Path::new(&link.path);
        let file = match &folder {
            Some(dir) if given.is_relative() => dir.join(given),
            _ => given.to_path_buf(),
        };
        Some((link.path.clone(), file.display().to_string(), file.is_file()))
    };
    // Depth first, each component before its own children, siblings in file order. Bounded, as a
    // design file is hostile input: a parent cycle can't loop it.
    let mut out = Vec::new();
    let mut stack: Vec<(u64, usize)> = doc.components.iter().filter(|c| c.parent == 0).rev().map(|c| (c.id, 0)).collect();
    while let Some((id, depth)) = stack.pop() {
        if out.len() >= 4096 || depth > 32 {
            continue;
        }
        let Some(c) = doc.components.iter().find(|c| c.id == id) else { continue };
        let link = linked(id);
        let placements: Vec<_> = doc.occurrences.iter().filter(|o| o.component == id).collect();
        out.push(Row {
            component: id,
            name: c.name.clone(),
            depth,
            found: link.as_ref().is_none_or(|(_, _, found)| *found),
            path: link.as_ref().map(|(path, _, _)| path.clone()),
            file: link.map(|(_, file, _)| file),
            count: placements.len(),
            grounded: placements.iter().any(|o| o.grounded),
        });
        stack.extend(doc.components.iter().filter(|k| k.parent == id).rev().map(|k| (k.id, depth + 1)));
    }
    out
}

pub fn show(app: &mut SolveApp, ui: &mut egui::Ui) {
    let t = Tokens::get();
    egui::Panel::left("sc_assembly_navigator")
        .exact_size(300.0)
        .resizable(false)
        .frame(egui::Frame::NONE.fill(t.panel).inner_margin(egui::Margin::symmetric(6, 6)))
        .show(ui, |ui| {
            ui.label(RichText::new(app.tr("ui.assembly_nav_heading")).size(11.0).strong().color(t.text_dim));
            ui.horizontal(|ui| {
                if ui.button(app.tr("ui.add_component")).clicked() {
                    app.start("component.insert_design");
                }
                if ui.button(app.tr("ui.new_component")).clicked() {
                    app.start("component.new_part");
                }
                let update = ui.add_enabled(!app.session.doc.links.is_empty(), egui::Button::new(app.tr("ui.update_parts")));
                crate::scenario::publish_rect("assembly_nav:update", update.rect);
                if update.clicked() {
                    match app.run("component.update_links", json!({})) {
                        Ok(v) => {
                            let n = v.get("updated").and_then(|u| u.as_array()).map_or(0, Vec::len);
                            app.set_status(crate::i18n::format("ui.parts_updated", &[("count", &n.to_string())]), false);
                        }
                        Err(e) => app.set_status(e, true),
                    }
                }
            });
            ui.separator();
            let rows = rows(app);
            let design = app.session.doc.name.clone();
            ui.label(RichText::new(design).strong());
            if rows.is_empty() {
                ui.label(RichText::new(app.tr("ui.assembly_nav_empty")).color(t.text_dim));
            }
            egui::ScrollArea::vertical().id_salt("sc_assembly_nav_rows").auto_shrink([false, false]).show(ui, |ui| {
                for row in &rows {
                    one_row(app, ui, row);
                }
            });
        });
}

fn one_row(app: &mut SolveApp, ui: &mut egui::Ui, row: &Row) {
    let t = Tokens::get();
    ui.horizontal(|ui| {
        ui.add_space(8.0 + row.depth as f32 * 14.0);
        let status = match (&row.path, row.found) {
            (None, _) => "ui.component_inside",
            (Some(_), true) => "ui.component_linked",
            (Some(_), false) => "ui.component_missing",
        };
        let label = ui.label(RichText::new(&row.name).color(if row.found { t.text } else { t.error }));
        crate::scenario::publish_rect(&format!("assembly_nav:row:{}", row.name), label.rect);
        let mut hover = app.tr(status).to_string();
        if let Some(path) = &row.path {
            hover = format!("{hover}: {path}");
        }
        label.on_hover_text(hover);
        ui.label(RichText::new(format!("×{}", row.count)).size(11.0).color(t.text_dim));
        if row.grounded {
            ui.label(RichText::new(app.tr("ui.component_fixed")).size(11.0).color(t.text_dim));
        }
        if let Some(file) = &row.file {
            if row.found {
                let open = ui.small_button(app.tr("ui.open_part"));
                crate::scenario::publish_rect(&format!("assembly_nav:open:{}", row.name), open.rect);
                if open.clicked() {
                    app.open_path(file);
                }
            }
            if ui.small_button(app.tr("ui.break_link")).clicked()
                && let Err(e) = app.run("component.break_link", json!({"component": row.component}))
            {
                app.set_status(e, true);
            }
        }
    });
    ui.add_space(vec2(0.0, 2.0).y);
}
