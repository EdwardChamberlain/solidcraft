//! Specifiers: the plane, vector and point inputs the Advanced approach's dialogs share.
//!
//! A specifier is one input of a dialog that can be answered in several ways. Its Type row starts
//! at **Inferred** (pick what you have and the way follows from it) and lists the explicit ways
//! after it: a plane from a plane or face, an offset, an angle, a bisector, three points…; a
//! vector along an edge, normal to a face, between two points, an axis (and always Reverse); a
//! point at a vertex, at an edge's midpoint, where you clicked, by coordinates, at the workpiece
//! origin. Every dialog that needs one embeds the same specifier, so the choices, the wording and
//! the picks work the same everywhere.
//!
//! The specifier's picks live in the dialog's own inputs (so the viewport routes clicks as it
//! always does); the specifier remembers how many it owns, and the choice, values and Reverse.
//! What a specifier makes is plain data the existing commands take: a point, a direction, or a
//! plane parameter, with the datum plane commands to run first when the plane is made rather than
//! picked.

use egui::Ui;
use serde_json::{Value, json};
use solidcraft_engine::Sel;
use solidcraft_engine::geom::Vec3;

use crate::SolveApp;
use crate::dialogs::{self, Dialog, Kind};
use crate::selection::{AXES, Accept, EDGES, FACES, PLANAR_FACES, PLANES, POINTS, SKETCH_POINTS, SelInput, VERTICES};

/// What a specifier answers.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Class {
    Plane,
    Vector,
    Point,
}

/// What a specifier stands for while nothing is picked and its Type is Inferred: the dialog's
/// default, shown in the empty selector.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Fallback {
    /// Nothing: the dialog wants a pick.
    #[default]
    Nothing,
    /// The x axis (a sketch's reference direction).
    XAxis,
    /// The workpiece origin.
    WorkpieceOrigin,
    /// The way the profile faces (an extrude's direction); no direction of its own.
    ProfileNormal,
}

impl Fallback {
    fn text(self) -> Option<&'static str> {
        match self {
            Fallback::Nothing => None,
            Fallback::XAxis => Some("X axis"),
            Fallback::WorkpieceOrigin => Some("Workpiece origin"),
            Fallback::ProfileNormal => Some("Profile normal"),
        }
    }
}

/// One specifier of a dialog.
#[derive(Clone, Debug)]
pub struct Specifier {
    pub class: Class,
    /// The Type: an index into [`choices`], 0 being Inferred.
    pub ty: usize,
    /// How many of the dialog's inputs are this specifier's (they follow the earlier specifiers').
    pub count: usize,
    /// Vectors: the opposite direction.
    pub reverse: bool,
    /// Typed values: a point's coordinates, or the axis of a vector.
    pub values: Vec<String>,
    /// Nothing picked is an answer (the dialog has a default): its inputs are not required.
    pub optional: bool,
    /// What it stands for while nothing is picked.
    pub fallback: Fallback,
    /// The icon beside the selector was clicked: the dialog of its Type is to open (the dialog
    /// that shows the specifier takes this and opens it).
    pub open: bool,
    /// What the dialog last mirrored of this specifier (a dialog that keeps its own copy of
    /// Reverse tells its own changes from the specifier's with it).
    pub memo: Option<bool>,
    /// The datum dialog a plane that is made (an offset, an angle…) takes its value and commands
    /// from. Its own inputs are not used: the specifier's are the dialog's.
    inner: Option<Box<Dialog>>,
}

/// The plane and vector choices, then the point's: (id, label). The datum plane choices are the
/// `construct.plane.*` commands, so every method the engine has is here.
pub fn choices(class: Class) -> Vec<(&'static str, &'static str)> {
    match class {
        Class::Plane => {
            let mut v = vec![("inferred", "Inferred"), ("plane", "Plane or Face")];
            v.extend(
                crate::dialogs_datum::types(crate::dialogs_datum::Class::Plane)
                    .into_iter()
                    .map(|(cmd, label)| (cmd, if cmd == "construct.plane.midplane" { "Bisector" } else { label })),
            );
            v
        }
        Class::Vector => vec![
            ("inferred", "Inferred"),
            ("edge", "Edge or Line"),
            ("normal", "Face or Plane Normal"),
            ("points", "Two Points"),
            ("axis", "Coordinate Axis"),
        ],
        Class::Point => vec![
            ("inferred", "Inferred"),
            ("existing", "Existing Point"),
            ("midpoint", "Midpoint of Edge"),
            ("pick", "Where Clicked"),
            ("coords", "Coordinates"),
            ("origin", "Workpiece Origin"),
        ],
    }
}

/// What Inferred takes: anything that can answer the specifier.
fn inferred_accept(class: Class) -> Accept {
    match class {
        Class::Plane => PLANES | PLANAR_FACES | FACES | EDGES | AXES | selection_points() | crate::selection::CURVES,
        Class::Vector => AXES | EDGES | PLANES | PLANAR_FACES | selection_points(),
        Class::Point => selection_points() | EDGES | FACES,
    }
}

fn selection_points() -> Accept {
    VERTICES | POINTS | SKETCH_POINTS
}

fn placeholder(mut input: SelInput, fallback: Fallback) -> SelInput {
    input.placeholder = fallback.text();
    input
}

impl Specifier {
    /// A specifier starting at `ty` (0, Inferred, unless the dialog's default is another way);
    /// its inputs are made by [`Specifier::inputs`] and added to the dialog by the caller.
    pub fn new(class: Class, ty: usize) -> Specifier {
        let values = match class {
            Class::Point => vec!["0 mm".into(), "0 mm".into(), "0 mm".into()],
            _ => vec!["X".into()],
        };
        Specifier { class, ty, count: 0, reverse: false, values, optional: false, fallback: Fallback::Nothing, open: false, memo: None, inner: None }
    }

    /// A specifier starting at the way with this id (Inferred when there is none).
    pub fn with_id(class: Class, id: &str) -> Specifier {
        Specifier::new(class, choices(class).iter().position(|c| c.0 == id).unwrap_or(0))
    }

    /// The same specifier with its default being no pick at all.
    pub fn optional(mut self) -> Specifier {
        self.optional = true;
        self
    }

    /// The same specifier standing for `fallback` while nothing is picked (and so optional).
    pub fn with_fallback(mut self, fallback: Fallback) -> Specifier {
        self.fallback = fallback;
        self.optional = true;
        self
    }

    /// The typed value of a plane that is made (its distance, angle or position).
    pub fn inputs_value_mut(&mut self) -> Option<&mut String> {
        self.inner.as_mut().and_then(|d| d.primary()).map(|(_, _, v)| v)
    }

    /// The id of the chosen way.
    pub fn id(&self) -> &'static str {
        choices(self.class).get(self.ty).map_or("inferred", |c| c.0)
    }

    /// The dialog inputs the chosen way needs.
    pub fn inputs(&mut self, app: &SolveApp) -> Vec<SelInput> {
        self.inner = None;
        let id = self.id();
        match self.class {
            Class::Plane => match id {
                "inferred" => vec![SelInput::new("Geometry", inferred_accept(Class::Plane) & !crate::selection::BODIES, true)],
                "plane" => vec![SelInput::new("Plane", PLANES | PLANAR_FACES, false)],
                cmd => match Dialog::for_command(app, cmd) {
                    Some(d) => {
                        let inputs = d.inputs.clone();
                        self.inner = Some(Box::new(d));
                        inputs
                    }
                    None => Vec::new(),
                },
            },
            Class::Vector => match id {
                "inferred" => vec![placeholder(SelInput::new("Geometry", inferred_accept(Class::Vector), true), self.fallback)],
                "edge" => vec![SelInput::new("Edge or line", AXES | EDGES, false)],
                "normal" => vec![SelInput::new("Plane or face", PLANES | PLANAR_FACES, false)],
                "points" => vec![SelInput::new("From", selection_points(), false), SelInput::new("To", selection_points(), false)],
                _ => Vec::new(),
            },
            Class::Point => match id {
                "inferred" => vec![placeholder(SelInput::new("Geometry", inferred_accept(Class::Point), false), self.fallback)],
                "existing" => vec![SelInput::new("Point", selection_points(), false)],
                "midpoint" => vec![SelInput::new("Edge", EDGES, false)],
                "pick" => vec![SelInput::new("Edge or face", EDGES | FACES, false)],
                _ => Vec::new(),
            },
        }
    }
}

/// Add the specifier to the dialog: its inputs go after the ones already there, tagged as its own
/// so later inputs of the dialog (a To Object, say) never confuse them.
pub fn add(app: &SolveApp, d: &mut Dialog, mut spec: Specifier) {
    let owner = d.specs.len() + 1;
    let mut inputs = spec.inputs(app);
    for input in &mut inputs {
        input.owner = owner;
    }
    spec.count = inputs.len();
    d.inputs.extend(inputs);
    d.specs.push(spec);
}

/// Replace the dialog's input `at` (a plane to pick, say) by a specifier's: same place, the same
/// picks kept as far as the specifier's first input takes them.
pub fn convert_input(app: &SolveApp, d: &mut Dialog, at: usize, mut spec: Specifier) {
    let owner = d.specs.len() + 1;
    let old = d.inputs.get(at).map(|i| i.items.clone()).unwrap_or_default();
    let mut inputs = spec.inputs(app);
    for input in &mut inputs {
        input.owner = owner;
    }
    if let Some(first) = inputs.first_mut() {
        first.items = old.into_iter().filter(|s| dialogs::fits(first.accept, s)).collect();
    }
    spec.count = inputs.len();
    if at < d.inputs.len() {
        d.inputs.remove(at);
    }
    for (n, input) in inputs.into_iter().enumerate() {
        d.inputs.insert((at + n).min(d.inputs.len()), input);
    }
    d.specs.push(spec);
}

/// The plane a Plane specifier stands for as a command's `plane` parameter takes it (a name, or an
/// origin and normal for a face), with the commands that make a plane the specifier defines
/// first (none for a plane or face that is picked).
pub fn plane_value(app: &SolveApp, d: &Dialog, index: usize) -> Result<(Vec<(String, Value)>, Value), String> {
    let out = plane(app, d, index)?;
    let value = match &out.sel {
        Some(sel) => dialogs::plane_value(&app.session, Some(sel)).ok_or("that is not a plane or a planar face")?,
        None => out.plane.clone(),
    };
    Ok((out.prefix, value))
}

/// The dialog's inputs that belong to specifier `index`.
fn indices(d: &Dialog, index: usize) -> Vec<usize> {
    d.inputs.iter().enumerate().filter(|(_, i)| i.owner == index + 1).map(|(k, _)| k).collect()
}

fn inputs_of(d: &Dialog, index: usize) -> Vec<&SelInput> {
    d.inputs.iter().filter(|i| i.owner == index + 1).collect()
}

fn picks(d: &Dialog, index: usize) -> Vec<&Sel> {
    inputs_of(d, index).into_iter().flat_map(|i| i.items.iter()).collect()
}

/// Change the way a specifier answers: its inputs are replaced, keeping the picks the new inputs
/// can still use.
fn set_type(app: &SolveApp, d: &mut Dialog, index: usize, ty: usize) {
    let Some(mut spec) = d.specs.get(index).cloned() else { return };
    let owned = indices(d, index);
    let old: Vec<Sel> = owned.iter().filter_map(|k| d.inputs.get(*k)).flat_map(|i| i.items.iter().cloned()).collect();
    spec.ty = ty;
    let mut inputs = spec.inputs(app);
    let mut keep = old.clone().into_iter();
    for input in &mut inputs {
        if let Some(pick) = keep.next().filter(|s| dialogs::fits(input.accept, s)) {
            input.items = vec![pick];
        }
    }
    if inputs.len() == 1 && inputs.first().is_some_and(|i| i.multi) {
        // Inferred takes every pick there was.
        if let Some(input) = inputs.first_mut() {
            input.items = old.into_iter().filter(|s| dialogs::fits(input.accept, s)).collect();
        }
    }
    for input in &mut inputs {
        input.owner = index + 1;
    }
    spec.count = inputs.len();
    let at = owned.first().copied().unwrap_or(d.inputs.len());
    let was_active = owned.contains(&d.active);
    for k in owned.iter().rev() {
        d.inputs.remove(*k);
    }
    for (n, input) in inputs.into_iter().enumerate() {
        d.inputs.insert((at + n).min(d.inputs.len()), input);
    }
    if let Some(slot) = d.specs.get_mut(index) {
        *slot = spec;
    }
    if was_active || d.active >= d.inputs.len() {
        d.active = at.min(d.inputs.len().saturating_sub(1));
    }
}

/// Choose a way by its id (`"offset"`-style ids are the command ids for planes).
pub fn choose(app: &SolveApp, d: &mut Dialog, index: usize, id: &str) -> bool {
    let Some(spec) = d.specs.get(index) else { return false };
    match choices(spec.class).iter().position(|c| c.0 == id) {
        Some(ty) => {
            set_type(app, d, index, ty);
            true
        }
        None => false,
    }
}

/// The specifier as a selector row: click it, then click geometry or features and what is picked
/// is inferred. The icon at its right opens the specifier's own dialog, where the way to say it
/// exactly is chosen (see [`open_sub`]); once a way other than Inferred is chosen, the row says
/// which and the dialog is what the row opens. `true` when Enter was pressed in a value.
pub fn show(app: &SolveApp, d: &mut Dialog, ui: &mut Ui, index: usize, title: &str, group: &str) -> bool {
    let Some(spec) = d.specs.get(index).cloned() else { return false };
    let owned = indices(d, index);
    let t = crate::theme::Tokens::get();
    let open = std::cell::Cell::new(false);
    let mut reset = false;
    let mut reverse = spec.reverse;
    let icon = match spec.class {
        Class::Plane => "plane",
        Class::Vector => "axis",
        Class::Point => "point",
    };
    let mut trailing = |ui: &mut Ui| {
        if spec.class == Class::Vector {
            let r = ui.add(egui::Button::new("").selected(reverse).min_size(egui::vec2(26.0, 22.0)));
            crate::icons::paint(ui.painter(), r.rect.shrink(4.0), "pattern_flip", t.icon, t.icon_fill, t.accent);
            crate::scenario::publish_handle(&format!("dialog:reverse:{group}"), r.rect.center());
            if r.on_hover_text(dialogs::tr("Reverse")).clicked() {
                reverse = !reverse;
            }
        }
        let r = ui.add(egui::Button::new("").min_size(egui::vec2(26.0, 22.0)));
        crate::icons::paint(ui.painter(), r.rect.shrink(3.0), icon, t.icon, t.icon_fill, t.accent);
        crate::scenario::publish_handle(&format!("dialog:specify:{group}"), r.rect.center());
        if r.on_hover_text(dialogs::tr(title)).clicked() {
            open.set(true);
        }
    };
    match owned.first() {
        // Inferred: the selector itself.
        Some(&first) if spec.ty == 0 => dialogs::input_rows_range_with(d, ui, &app.session, first..first + 1, Some(title), &mut trailing),
        // A way that was chosen in the dialog (or needs no pick): what it is, where the pick would be.
        _ => {
            dialogs::row_label(ui, title);
            ui.horizontal(|ui| {
                let text = way_text(&spec);
                let chip = ui.add(egui::Button::new(text).fill(t.chip).min_size(egui::vec2(0.0, 22.0)));
                if chip.on_hover_text(dialogs::tr("Edit")).clicked() {
                    open.set(true);
                }
                if spec.ty != 0 {
                    let clear = ui.small_button("×").on_hover_text(dialogs::tr("Clear the selection"));
                    crate::scenario::publish_handle(&format!("dialog:clear:{group}"), clear.rect.center());
                    if clear.clicked() {
                        // Back to the selector: Inferred, nothing picked.
                        reset = true;
                    }
                }
                trailing(ui);
            });
            ui.end_row();
        }
    }
    if let Some(slot) = d.specs.get_mut(index) {
        slot.reverse = reverse;
        slot.open |= open.get();
    }
    if let Some(p) = crate::scenario::handle_at(&format!("dialog:specify:{group}")) {
        crate::scenario::publish_handle(&format!("dialog:group:{group}"), p);
    }
    if reset {
        if let Some(slot) = d.specs.get_mut(index) {
            slot.reverse = false;
            slot.values = Specifier::new(slot.class, 0).values;
        }
        set_type(app, d, index, 0);
        // Inferred starts empty, whatever the way that was cleared had picked.
        for k in indices(d, index) {
            if let Some(input) = d.inputs.get_mut(k) {
                input.items.clear();
            }
        }
    }
    false
}

/// What a way that is not Inferred says in its selector row.
fn way_text(spec: &Specifier) -> String {
    let label = choices(spec.class).get(spec.ty).map_or("", |c| c.1);
    match (spec.class, spec.id()) {
        (Class::Vector, "axis") => format!("{} axis", spec.values.first().map_or("X", String::as_str)),
        (Class::Point, "coords") => spec.values.join(", "),
        (Class::Point, "origin") => "Workpiece origin".to_string(),
        _ => label.to_string(),
    }
}

/// The ways the specifier's own dialog offers: all but Inferred, which is the selector's.
fn dialog_choices(class: Class) -> Vec<(&'static str, &'static str)> {
    choices(class).into_iter().skip(1).collect()
}

/// Open the dialog of specifier `index` of `parent`: its own dialog, with the ways to say a plane
/// (an axis, a point) exactly. The parent waits under it; OK returns what was chosen to the
/// parent's selector, Cancel leaves the parent as it was.
pub fn open_sub(app: &mut SolveApp, parent: Dialog, index: usize) {
    let Some(spec) = parent.specs.get(index).cloned() else {
        app.dialog = Some(parent);
        return;
    };
    // Inferred is the selector's: the dialog starts at the first way that is not.
    let ty = if spec.ty == 0 { 1 } else { spec.ty };
    let mut chosen = Specifier::new(spec.class, ty);
    chosen.reverse = spec.reverse;
    if spec.ty != 0 {
        chosen.values = spec.values.clone();
    }
    let mut sub = Dialog::new(Kind::Specify { index }, Vec::new());
    add(app, &mut sub, chosen);
    // What the parent holds comes along as far as the way can use it.
    let old: Vec<Sel> = indices(&parent, index).iter().filter_map(|k| parent.inputs.get(*k)).flat_map(|i| i.items.iter().cloned()).collect();
    let mut keep = old.into_iter();
    for input in sub.inputs.iter_mut() {
        if let Some(pick) = keep.next().filter(|s| dialogs::fits(input.accept, s)) {
            input.items = vec![pick];
        }
    }
    if spec.ty != 0
        && let Some(inner) = spec.inner
        && let Some(slot) = sub.specs.first_mut()
    {
        slot.inner = Some(inner);
    }
    sub.active = sub.inputs.iter().position(|i| i.items.is_empty()).unwrap_or(0);
    app.dialog_stack.push(parent);
    app.dialog = Some(sub);
}

/// The specifier dialog was closed: with OK its choice goes to the parent's selector.
pub fn close_sub(app: &mut SolveApp, sub: Dialog, applied: bool) {
    let Some(mut parent) = app.dialog_stack.pop() else { return };
    if let (true, Kind::Specify { index }) = (applied, &sub.kind)
        && let Some(chosen) = sub.specs.first().cloned()
        && parent.specs.get(*index).is_some()
    {
        let index = *index;
        let mut inputs: Vec<SelInput> = sub.inputs.clone();
        for input in &mut inputs {
            input.owner = index + 1;
        }
        let owned = indices(&parent, index);
        let at = owned.first().copied().unwrap_or(parent.inputs.len());
        for k in owned.iter().rev() {
            parent.inputs.remove(*k);
        }
        let mut spec = chosen;
        spec.count = inputs.len();
        spec.open = false;
        spec.optional = parent.specs[index].optional;
        spec.fallback = parent.specs[index].fallback;
        for (n, input) in inputs.into_iter().enumerate() {
            parent.inputs.insert((at + n).min(parent.inputs.len()), input);
        }
        parent.specs[index] = spec;
        // The next pick the parent wants, not one of the specifier's own.
        parent.active = (0..parent.inputs.len())
            .find(|k| parent.inputs.get(*k).is_some_and(|i| i.items.is_empty()) && !is_optional(&parent, *k))
            .unwrap_or_else(|| parent.active.min(parent.inputs.len().saturating_sub(1)));
    }
    app.dialog = Some(parent);
}

/// The heading of a specifier's dialog.
pub fn sub_title(d: &Dialog) -> &'static str {
    match d.specs.first().map(|s| s.class) {
        Some(Class::Plane) => "PLANE",
        Some(Class::Vector) => "AXIS",
        Some(Class::Point) => "POINT",
        None => "SPECIFY",
    }
}

/// Can the specifier dialog's choice be handed back: is what its way needs given?
pub fn sub_valid(app: &SolveApp, d: &Dialog) -> bool {
    let Some(spec) = d.specs.first() else { return false };
    match spec.class {
        Class::Plane => plane(app, d, 0).is_ok(),
        Class::Vector => matches!(vector_raw(app, d, 0), Ok(Some(_))),
        Class::Point => matches!(point(app, d, 0), Ok(Some(_))),
    }
}

/// The rows of a specifier's own dialog: the Type, what it asks for, its values.
pub fn sub_rows(app: &SolveApp, d: &mut Dialog, ui: &mut Ui) -> bool {
    let mut enter = false;
    let Some(spec) = d.specs.first().cloned() else { return enter };
    let all = dialog_choices(spec.class);
    let labels: Vec<&str> = all.iter().map(|c| c.1).collect();
    let mut at = spec.ty.saturating_sub(1);
    dialogs::row_label(ui, "Type");
    dialogs::combo(ui, "specifier_sub_type", &labels, &mut at);
    ui.end_row();
    if at + 1 != spec.ty {
        set_type(app, d, 0, at + 1);
    }
    for k in indices(d, 0) {
        dialogs::input_rows_range(d, ui, &app.session, k..k + 1);
    }
    let Some(mut spec) = d.specs.first().cloned() else { return enter };
    match (spec.class, spec.id()) {
        (Class::Plane, _) => {
            if let Some(inner) = spec.inner.as_mut()
                && let Some((label, _, value)) = inner.primary()
            {
                dialogs::row_label(ui, label);
                enter |= dialogs::field(ui, value);
                ui.end_row();
            }
        }
        (Class::Vector, "axis") => {
            dialogs::row_label(ui, "Axis");
            let mut axis = ["X", "Y", "Z"].iter().position(|a| spec.values.first().is_some_and(|v| v == a)).unwrap_or(0);
            dialogs::combo(ui, "specifier_sub_axis", &["X", "Y", "Z"], &mut axis);
            spec.values = vec![["X", "Y", "Z"][axis.min(2)].to_string()];
            ui.end_row();
        }
        (Class::Point, "coords") => {
            for (k, label) in ["X", "Y", "Z"].iter().enumerate() {
                dialogs::row_label(ui, label);
                if let Some(v) = spec.values.get_mut(k) {
                    enter |= dialogs::field(ui, v);
                }
                ui.end_row();
            }
        }
        _ => {}
    }
    if spec.class == Class::Vector {
        dialogs::row_label(ui, "Reverse");
        let r = ui.checkbox(&mut spec.reverse, "");
        crate::scenario::publish_handle("dialog:reverse:sub", r.rect.center());
        ui.end_row();
    }
    if let Some(slot) = d.specs.first_mut() {
        *slot = spec;
    }
    enter
}

/// A pick as a point in the world: a vertex, a sketch point, where an edge or face was picked.
pub fn point_of(app: &SolveApp, sel: &Sel) -> Option<Vec3> {
    match sel {
        Sel::Vertex { point, .. } | Sel::Edge { point, .. } | Sel::Face { point, .. } => Some(*point),
        Sel::SketchPoint { id } => {
            let (sketch, point) = match id.split_once(':') {
                Some((sketch, point)) if sketch.parse::<u64>().is_ok() => (sketch.parse::<u64>().ok()?, point),
                _ => (app.session.active_sketch?, id.as_str()),
            };
            let st = app.session.world_state();
            let ss = st.sketch(sketch)?;
            let p = ss.sketch.point(ss.sketch.resolve_point(point)?)?;
            Some(ss.plane.to_world(p))
        }
        _ => None,
    }
}

/// A pick as a direction: along an edge, line or axis; the normal of a plane or planar face.
fn direction_of(app: &SolveApp, sel: &Sel) -> Option<Vec3> {
    match sel {
        Sel::Plane { name } => dialogs::plane_by_name(app, name).and_then(|p| p.normal().normalized()),
        other => crate::dialogs_specify::vector_of(app, other).map(|(_, d)| d),
    }
}

/// The point a Point specifier stands for, or `None` when nothing is specified (Inferred with
/// no pick yet).
pub fn point(app: &SolveApp, d: &Dialog, index: usize) -> Result<Option<Vec3>, String> {
    let spec = d.specs.get(index).ok_or("no such input")?;
    let first = picks(d, index).into_iter().next();
    let at = |sel: Option<&Sel>, what: &str| -> Result<Option<Vec3>, String> {
        match sel {
            Some(s) => point_of(app, s).map(Some).ok_or_else(|| format!("that is not {what}")),
            None => Ok(None),
        }
    };
    match spec.id() {
        "inferred" if first.is_none() && spec.fallback == Fallback::WorkpieceOrigin => Ok(Some(work_axes(app).0)),
        "inferred" => at(first, "a point"),
        "existing" => at(first, "a point").and_then(|p| p.ok_or_else(|| "select a point first".to_string()).map(Some)),
        "midpoint" => at(first, "an edge").and_then(|p| p.ok_or_else(|| "select an edge first".to_string()).map(Some)),
        "pick" => at(first, "an edge or a face").and_then(|p| p.ok_or_else(|| "select an edge or a face first".to_string()).map(Some)),
        "coords" => {
            let value = |k: usize| -> Result<f64, String> {
                let text = spec.values.get(k).ok_or("missing coordinate")?;
                app.session.doc.eval(text, solidcraft_engine::doc::expr::Kind::Length).map_err(|e| e.to_string())
            };
            Ok(Some(Vec3::new(value(0)?, value(1)?, value(2)?)))
        }
        "origin" => Ok(Some(work_axes(app).0)),
        _ => Ok(None),
    }
}

/// The work coordinate system as the specifiers use it: the origin, and the x, y and z axes, in
/// the active component's frame.
fn work_axes(app: &SolveApp) -> (Vec3, Vec3, Vec3, Vec3) {
    let w = solidcraft_engine::view::work_frame(&app.session);
    let m = app.session.active_component_frame();
    let (o, x, y, z) = (w.origin, w.x, w.y, w.normal());
    (
        solidcraft_engine::doc::apply_point(&m, o),
        solidcraft_engine::doc::apply_vector(&m, x),
        solidcraft_engine::doc::apply_vector(&m, y),
        solidcraft_engine::doc::apply_vector(&m, z),
    )
}

/// The direction a Vector specifier stands for (Reverse applied), or `None` when nothing is
/// specified.
pub fn vector(app: &SolveApp, d: &Dialog, index: usize) -> Result<Option<Vec3>, String> {
    let reverse = d.specs.get(index).is_some_and(|s| s.reverse);
    Ok(vector_raw(app, d, index)?.map(|v| if reverse { -v } else { v }))
}

/// The direction before Reverse.
pub fn vector_raw(app: &SolveApp, d: &Dialog, index: usize) -> Result<Option<Vec3>, String> {
    let spec = d.specs.get(index).ok_or("no such input")?;
    let items = picks(d, index);
    let dir = match spec.id() {
        // The work coordinate system's axes (the world's until one is named).
        "axis" => {
            let (_, x, y, z) = work_axes(app);
            match spec.values.first().map(String::as_str) {
                Some("Y") => Some(y),
                Some("Z") => Some(z),
                _ => Some(x),
            }
        }
        "points" => {
            let inputs = inputs_of(d, index);
            let at = |i: usize| inputs.get(i).and_then(|inp| inp.items.first()).and_then(|s| point_of(app, s));
            match (at(0), at(1)) {
                (Some(a), Some(b)) => Some((b - a).normalized().ok_or("the two points are the same")?),
                _ => return Err("select both points first".into()),
            }
        }
        "inferred" => match items.as_slice() {
            [] => match spec.fallback {
                Fallback::XAxis => Some(work_axes(app).1),
                _ => None,
            },
            [one] => Some(direction_of(app, one).ok_or("that has no direction")?),
            [a, b] => match (point_of(app, a), point_of(app, b)) {
                (Some(a), Some(b)) => Some((b - a).normalized().ok_or("the two points are the same")?),
                _ => return Err("pick one edge, axis or face, or two points".into()),
            },
            _ => return Err("pick one edge, axis or face, or two points".into()),
        },
        _ => match items.first() {
            Some(s) => Some(direction_of(app, s).ok_or("that has no direction")?),
            None => return Err("select what gives the direction first".into()),
        },
    };
    Ok(dir)
}

/// What a Plane specifier makes: the commands to run first (datum planes made for it), the
/// plane as `sketch.create` and the other plane parameters take it, and the pick it stands for
/// when it is one.
#[derive(Clone, Debug, Default)]
pub struct PlaneOut {
    pub prefix: Vec<(String, Value)>,
    pub plane: Value,
    pub sel: Option<Sel>,
}

/// A name for a datum plane the dialog makes, not yet used in the design or by an earlier one.
fn fresh_name(app: &SolveApp, used: &[String]) -> String {
    (1..)
        .map(|k| format!("Datum Plane {k}"))
        .find(|n| app.session.doc.find_feature(n).is_none() && !used.iter().any(|u| u.eq_ignore_ascii_case(n)))
        .unwrap_or_else(|| "Sketch Plane".to_string())
}

pub fn plane(app: &SolveApp, d: &Dialog, index: usize) -> Result<PlaneOut, String> {
    let spec = d.specs.get(index).ok_or("no such input")?;
    let items: Vec<Sel> = picks(d, index).into_iter().cloned().collect();
    let on = |sel: Option<&Sel>| -> Result<PlaneOut, String> {
        Ok(PlaneOut { prefix: Vec::new(), plane: dialogs::plane_arg(app, sel)?, sel: sel.cloned() })
    };
    let made = |cmd: &str, mut params: Value| -> PlaneOut {
        let name = fresh_name(app, &[]);
        params["name"] = json!(name);
        PlaneOut { prefix: vec![(cmd.to_string(), params)], plane: json!(name), sel: None }
    };
    match spec.id() {
        "plane" => on(items.first()),
        "inferred" => {
            if items.is_empty() {
                return Err("select a plane or a face, or what defines one".into());
            }
            // One plane or planar face is the plane itself.
            if let [one] = items.as_slice()
                && matches!(one, Sel::Plane { .. } | Sel::Face { .. })
                && (matches!(one, Sel::Plane { .. }) || dialogs::plane_arg(app, Some(one)).is_ok())
            {
                return on(Some(one));
            }
            let mut tmp = crate::dialogs_datum::inferred(crate::dialogs_datum::Class::Plane);
            tmp.inputs[0].items = items;
            let (cmd, params) = crate::dialogs_datum::infer(app, &tmp)?;
            Ok(made(cmd, params))
        }
        cmd => {
            let mut tmp = match spec.inner.as_deref() {
                Some(inner) => inner.clone(),
                None => Dialog::for_command(app, cmd).ok_or("that plane type is not available")?,
            };
            tmp.inputs = inputs_of(d, index)
                .into_iter()
                .cloned()
                .map(|mut i| {
                    i.owner = 0;
                    i
                })
                .collect();
            let commands = dialogs::apply_commands(app, &tmp)?;
            let (made_cmd, params) = commands.into_iter().next().ok_or("nothing to make")?;
            Ok(made(&made_cmd, params))
        }
    }
}

/// Is the dialog one whose specifiers decide a sketch (so Create Sketch's own rows apply)?
pub fn is_sketch(d: &Dialog) -> bool {
    matches!(d.kind, Kind::Sketch) && !d.specs.is_empty()
}

/// Is input `k` of the dialog one that need not be picked?
pub fn is_optional(d: &Dialog, k: usize) -> bool {
    d.inputs.get(k).is_some_and(|i| i.owner > 0 && d.specs.get(i.owner - 1).is_some_and(|s| s.optional))
}

/// A pick as a line: a point on it and its direction (an axis, an edge or sketch line, the normal
/// of a plane or face through the picked point).
fn line_of(app: &SolveApp, sel: &Sel) -> Option<(Vec3, Vec3)> {
    match sel {
        Sel::Plane { name } => dialogs::plane_by_name(app, name).and_then(|p| p.normal().normalized().map(|n| (p.origin, n))),
        other => crate::dialogs_specify::vector_of(app, other),
    }
}

/// The line a Vector specifier stands for (Reverse applied): a point on it and its direction, for
/// an input that is an axis (a revolve's) rather than a direction. `None` when nothing is
/// specified.
pub fn line(app: &SolveApp, d: &Dialog, index: usize) -> Result<Option<(Vec3, Vec3)>, String> {
    let spec = d.specs.get(index).ok_or("no such input")?;
    let items = picks(d, index);
    let found = match spec.id() {
        "axis" => {
            let dir = vector_raw(app, d, index)?.ok_or("no direction")?;
            Some((work_axes(app).0, dir))
        }
        "points" => {
            let dir = vector_raw(app, d, index)?.ok_or("select both points first")?;
            let inputs = inputs_of(d, index);
            let from = inputs.first().and_then(|i| i.items.first()).and_then(|s| point_of(app, s)).ok_or("select both points first")?;
            Some((from, dir))
        }
        "inferred" => match items.as_slice() {
            [] => None,
            [one] => Some(line_of(app, one).ok_or("that is not a line, an edge or a face")?),
            [a, b] => match (point_of(app, a), point_of(app, b)) {
                (Some(a), Some(b)) => Some((a, (b - a).normalized().ok_or("the two points are the same")?)),
                _ => return Err("pick one edge, axis or face, or two points".into()),
            },
            _ => return Err("pick one edge, axis or face, or two points".into()),
        },
        _ => match items.first() {
            Some(s) => Some(line_of(app, s).ok_or("that is not a line, an edge or a face")?),
            None => return Err("select what gives the axis first".into()),
        },
    };
    Ok(found.map(|(p, v)| (p, if spec.reverse { -v } else { v })))
}
