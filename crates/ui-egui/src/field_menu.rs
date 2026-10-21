//! The ▾ menu beside a value field in the Advanced approach's dialogs: **Reset** puts the field
//! back to what it was when the dialog opened, and **Measure** offers what the current selection
//! measures (a length, a diameter, the distance between two picks, an angle) as the field's
//! value. The values are the numbers `inspect.measure` gives; nothing is linked to the geometry.
//!
//! The dialog arms the menu for the frame it draws ([`arm`], [`disarm`]); the expression editor
//! of the kit draws the button where it finds the menu armed.

use egui::{Context, Id};
use solidcraft_engine::Sel;

use crate::SolveApp;
use crate::dialogs::Dialog;

/// What the editors of the dialog being drawn need to offer.
#[derive(Clone, Default)]
pub(crate) struct Armed {
    /// The dialog (a new one for each Apply and Reset).
    pub dialog: u64,
    /// What the selection measures, as (label, text).
    pub measures: Vec<(&'static str, String)>,
}

fn key() -> Id {
    Id::new("sc_field_menu")
}

fn open_key() -> Id {
    Id::new("sc_field_menu_open")
}

/// Arm the menu for the dialog drawn next (Advanced approach only).
pub(crate) fn arm(app: &SolveApp, ctx: &Context, d: &Dialog) {
    if app.preferences.modelling_approach != crate::prefs::ModellingApproach::Advanced {
        return disarm(ctx);
    }
    // Measuring costs a little: only while a menu is open.
    let open = ctx.data_mut(|data| data.remove_temp::<bool>(open_key())).unwrap_or(false);
    let measures = if open { measures(app) } else { Vec::new() };
    ctx.data_mut(|data| data.insert_temp(key(), Armed { dialog: d.uid, measures }));
}

pub(crate) fn disarm(ctx: &Context) {
    ctx.data_mut(|data| data.remove::<Armed>(key()));
}

pub(crate) fn armed(ctx: &Context) -> Option<Armed> {
    ctx.data(|data| data.get_temp::<Armed>(key()))
}

/// A menu is open this frame: the next frame measures.
pub(crate) fn set_open(ctx: &Context) {
    ctx.data_mut(|data| data.insert_temp(open_key(), true));
}

/// What the selection measures: one item gives its length, diameter, radius or circumference;
/// two give the distance between them, the farthest distance, the centre distance and the angle.
pub(crate) fn measures(app: &SolveApp) -> Vec<(&'static str, String)> {
    let items: Vec<Sel> = app
        .session
        .selection
        .iter()
        .filter(|s| matches!(s, Sel::Edge { .. } | Sel::Face { .. } | Sel::Vertex { .. } | Sel::SketchCurve { .. } | Sel::SketchPoint { .. }))
        .take(3)
        .cloned()
        .collect();
    if items.is_empty() || items.len() > 2 {
        return Vec::new();
    }
    let r = app.session.measure_items(&items);
    let num = |v: f64| {
        let t = format!("{v:.6}");
        t.trim_end_matches('0').trim_end_matches('.').to_string()
    };
    let mut out = Vec::new();
    let mm = |out: &mut Vec<(&'static str, String)>, label: &'static str, v: Option<f64>| {
        if let Some(v) = v.filter(|v| v.is_finite()) {
            out.push((label, format!("{} mm", num(v))));
        }
    };
    if let [only] = r["items"].as_array().map_or(&[][..], Vec::as_slice) {
        mm(&mut out, "Length", only["length_mm"].as_f64());
    }
    mm(&mut out, "Diameter", r["diameter"].as_f64());
    mm(&mut out, "Radius", r["radius"].as_f64());
    mm(&mut out, "Circumference", r["circumference"].as_f64());
    mm(&mut out, "Distance", r["distance_mm"].as_f64());
    mm(&mut out, "Center Distance", r["center_distance"].as_f64());
    mm(&mut out, "Farthest Distance", r["max"].as_f64());
    if let Some(a) = r["angle_deg"].as_f64().filter(|a| a.is_finite()) {
        out.push(("Angle", format!("{} deg", num(a))));
    }
    out
}
