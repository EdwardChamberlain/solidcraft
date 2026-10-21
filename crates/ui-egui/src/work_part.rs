//! Editing a part in the context of its assembly (the Advanced approach's *work part*).
//!
//! A part is edited in its own tab, where every pick and command works on the part itself. Made
//! the work part from the Assembly Navigator, its tab also shows the rest of the assembly around
//! it, faded, in the part's own frame: the pin the bracket must clear, the lid it carries. That
//! context is drawn, and picked only for the places a plane, vector or point selector names (a
//! face gives its plane, a straight edge its direction, a vertex its place, by value: the part
//! does not follow the other component when that changes), and it follows the assembly as it stands in its tab: the part's own bodies are left out,
//! since the part is on show. Back in the assembly's tab the part's changes arrive as they do
//! for any edit of a part in another tab.

use std::sync::Arc;

use solidcraft_engine::doc::{ModelBody, kernel, mat_inverse};
use solidcraft_engine::geom::Vec3;

use crate::SolveApp;

/// What marks a context body's name, so nothing mistakes it for one of the part's own.
pub const CONTEXT_PREFIX: &str = "\u{1}context:";

/// The assembly a part tab is the work part of.
#[derive(Clone, Debug, PartialEq)]
pub struct WorkContext {
    /// The assembly design's file.
    pub assembly: String,
    /// The component of the assembly the part is.
    pub component: u64,
    /// Its name in the assembly.
    pub name: String,
    /// Draw the rest of the assembly around the part.
    pub show: bool,
    /// Just made: the window brings the Part Navigator, and its banner, forward once.
    pub fresh: bool,
}

/// The context bodies last made, and what they were made from.
pub(crate) struct Cached {
    key: (usize, u64, u64),
    bodies: Arc<Vec<ModelBody>>,
}

/// The assembly's design session, if its tab is open.
fn assembly_session<'a>(app: &'a SolveApp, work: &WorkContext) -> Option<&'a solidcraft_engine::Session> {
    let (sessions, _) = crate::documents::open_sessions(&app.docs, &app.session);
    sessions.into_iter().find(|s| s.path.as_deref().is_some_and(|p| solidcraft_engine::workspace::same_file(p, &work.assembly)))
}

/// The rest of the assembly in the work part's frame, faded bodies for the scene to draw. `None`
/// when this tab is no work part, the context is off, or the assembly is not open (or no longer
/// has the component).
pub fn context_bodies(app: &SolveApp) -> Option<Arc<Vec<ModelBody>>> {
    let work = app.work.as_ref().filter(|w| w.show)?;
    let assembly = assembly_session(app, work)?;
    let world = assembly.world_state();
    let key = (Arc::as_ptr(&world) as usize, work.component, assembly.revision);
    if let Ok(cache) = app.work_cache.lock()
        && let Some(c) = cache.as_ref().filter(|c| c.key == key)
    {
        return Some(c.bodies.clone());
    }
    // Where the component stands in the assembly; its frame is the part's.
    let placed = assembly.doc.placements().into_iter().find(|(c, _, _)| *c == work.component).map(|(_, _, m)| m)?;
    let to_part = mat_inverse(&placed)?;
    let bodies: Vec<ModelBody> = world
        .bodies
        .iter()
        .filter(|b| !assembly.doc.component_within(b.component, work.component) && assembly.body_visible(&b.name))
        .map(|b| {
            let body = kernel::transform_matrix(&b.body, to_part).unwrap_or_else(|_| b.body.clone());
            ModelBody::new(format!("{CONTEXT_PREFIX}{}", b.name), body, b.feature)
        })
        .collect();
    let bodies = Arc::new(bodies);
    if let Ok(mut cache) = app.work_cache.lock() {
        *cache = Some(Cached { key, bodies: bodies.clone() });
    }
    Some(bodies)
}

/// Is this body one of the assembly's, drawn around the work part?
pub fn is_context(name: &str) -> bool {
    name.starts_with(CONTEXT_PREFIX)
}

/// A context body by its (prefixed) name.
fn context_named(app: &SolveApp, name: &str) -> Option<ModelBody> {
    context_bodies(app)?.iter().find(|b| b.name == name).cloned()
}

/// The centre and normal of planar face `face` of a context body.
pub fn planar_face(app: &SolveApp, body: &str, face: usize) -> Option<(Vec3, Vec3)> {
    let b = context_named(app, body)?;
    let tol = (b.body.size() * 1e-3).max(1e-3);
    let f = b.body.faces(tol).ok()?.into_iter().find(|f| f.index == face)?;
    Some((f.centroid, f.plane_normal?))
}

/// A point on a context body's straight edge and its direction.
pub fn straight_edge(app: &SolveApp, body: &str, index: usize) -> Option<(Vec3, Vec3)> {
    let b = context_named(app, body)?;
    let m = b.mesh();
    let e = m.edges.get(index)?;
    let (a, z) = (*e.first()?, *e.last()?);
    let dir = (z - a).normalized()?;
    let tol = 1e-6 * a.dist(z).max(1.0);
    e.iter().all(|p| (*p - a).cross(dir).len() < tol).then_some((a, dir))
}

/// The context bodies a pick may land on now: only while a dialog's active input is one of a
/// plane, vector or point specifier's, and only for the places they name (never a selection to
/// edit or delete: the part has none of these).
pub fn pickable(app: &SolveApp) -> Option<Arc<Vec<ModelBody>>> {
    let input = app.dialog.as_ref()?.active_input().filter(|i| i.owner > 0)?;
    if input.accept & (crate::selection::FACES | crate::selection::EDGES | crate::selection::VERTICES) == 0 {
        return None;
    }
    context_bodies(app)
}

/// What the scene key hashes: changes when the context's bodies do.
pub fn scene_key(app: &SolveApp) -> usize {
    context_bodies(app).map_or(0, |b| Arc::as_ptr(&b) as usize)
}

/// Make component `component` of the displayed assembly the work part: open its part file (or go
/// to its tab) and show the assembly around it.
pub fn make_work_part(app: &mut SolveApp, component: u64) -> Result<(), String> {
    let assembly = app.session.path.clone().ok_or_else(|| app.tr("ui.work_part_unsaved").to_string())?;
    let rows = crate::shell::assembly_nav::rows(app);
    let row = rows.into_iter().find(|r| r.component == component).ok_or("no such component")?;
    let file = row.file.clone().filter(|_| row.found).ok_or_else(|| app.tr("ui.work_part_missing").to_string())?;
    match crate::documents::tab_of(app, &file) {
        Some(tab) => crate::documents::switch(app, tab),
        None => app.open_path(&file),
    }
    // The part's tab is the one on show now.
    if !app.session.path.as_deref().is_some_and(|p| solidcraft_engine::workspace::same_file(p, &file)) {
        return Err(format!("{file}: could not open the part"));
    }
    app.work = Some(WorkContext { assembly, component, name: row.name, show: true, fresh: true });
    Ok(())
}

/// Back to the assembly's tab. Whether it is open.
pub fn return_to_assembly(app: &mut SolveApp) -> bool {
    let Some(work) = app.work.clone() else { return false };
    match crate::documents::tab_of(app, &work.assembly) {
        Some(tab) => {
            crate::documents::switch(app, tab);
            true
        }
        None => false,
    }
}
