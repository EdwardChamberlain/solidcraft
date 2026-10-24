//! Linked designs: a component whose bodies come from another design file (Insert Design).
//!
//! The linked design is opened and evaluated on its own, and its solid bodies, as placed in it,
//! join a new component through an Import feature (exact B-rep, kept in this design so it opens
//! without the part file). The link remembers the file, relative to this design's folder once it
//! is saved; Update Links reads the file again and replaces the Import feature's bodies, and
//! Break Link keeps the bodies and forgets the file.
//!
//! A part may link parts that link parts: a linked design is brought up to date from its own
//! links before its bodies are read, so editing a leaf reaches every design above it on the next
//! update. A design can never link one that already links it, however far down (`reaches`).
//! New Part (`component.new_part`) makes the part file and links it in one step. A part with no
//! solid bodies yet (a new one) links as an empty component; the bodies arrive on the update that
//! follows once something is modelled in it, and go again if they are all deleted.

use std::path::{Component as PathPart, Path, PathBuf};

use serde_json::{Value, json};
use solidcraft_doc::{DesignLink, Document, FeatureKind};

use super::CommandSpec;
use crate::params::{bad, str_, vec3};
use crate::{EngineError, Result, Session};

#[path = "component_extract.rs"]
mod extract;

#[cfg(test)]
#[path = "component_relink_tests.rs"]
#[allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
mod relink_tests;

pub static COMMANDS: &[CommandSpec] = &[
    CommandSpec::new("component.insert_design", "Insert Design", insert_design).at("SOLID", "ASSEMBLE").icon("component").params(
        "path: a .solidcraft design → a new component linked to it, holding its solid bodies as placed in that design; \
         name? (default: the design's name); parent?: component id or name (default: the active one); translate?: [x,y,z]; \
         reuse?: bool (default false): insert another occurrence of an existing link to the same file",
    ),
    CommandSpec::new("component.new_part", "New Part Component", new_part).at("SOLID", "ASSEMBLE").icon("component").params(
        "name: the part's name; path?: its .solidcraft file (default: <name>.solidcraft beside this design, which must be saved first); \
         parent?, translate?: as component.insert_design → a new empty part file, linked into a new component. Open it to model it",
    ),
    CommandSpec::new("component.update_links", "Update Links", update_links).icon("update").params(
        "component?: id or name (default: every linked component) — reads the linked designs again and replaces their bodies where they changed",
    ),
    CommandSpec::new("component.relink", "Relink Source", relink).icon("update").params(
        "component: id or name of a linked component; path: a saved .solidcraft design — replaces the source and linked bodies for all its occurrences, preserving identities and placements; returns path, previous_path, warnings, joint_errors and joint_conflicts",
    ),
    CommandSpec::new("component.break_link", "Break Link", break_link)
        .icon("break")
        .params("component: id or name of a linked component — keeps its bodies as they are and forgets the design file"),
    CommandSpec::new("component.extract", "Extract to File", extract::extract).icon("export").params(
        "component: id or name of a component that is not linked; path: the .solidcraft design to write → its history, subcomponents and their joints \
         move to that design, and the component becomes linked to it (as component.insert_design makes it); returns path, file, feature, bodies, \
         notices (what stays behind), warnings and copies (components that build the same bodies: see component.replace_with_link)",
    ),
    CommandSpec::new("component.copies", "Find Identical Copies", extract::copies).icon("inspect").noundo().params(
        "component: id or name of a linked component → copies: [{component, name}], the components not linked whose own history builds the same bodies",
    ),
    CommandSpec::new("component.replace_with_link", "Replace with Link", extract::replace_with_link).icon("chain").params(
        "component or components: [ids or names] of components that are not linked; with: a linked component, or path: a .solidcraft design \
         (its link here, else a new linked component) → each component's history goes and its placements become placements of the linked \
         component; refused when its bodies are not the same as the link's; returns component, name, replaced and warnings",
    ),
    CommandSpec::new("component.links", "List Links", list_links)
        .icon("chain")
        .noundo()
        .params("→ linked components: component, name, path, file (resolved), found"),
];

/// The folder relative link paths are kept against: the design's own, once it has a file.
fn base_dir(path: Option<&str>) -> Option<PathBuf> {
    path.and_then(|p| absolute(Path::new(p)).parent().map(Path::to_path_buf))
}

/// A path as given, made absolute when the platform can, with `.` and `..` worked out.
pub(crate) fn normalized(p: &Path) -> PathBuf {
    absolute(p)
}

fn absolute(p: &Path) -> PathBuf {
    let full = std::path::absolute(p).unwrap_or_else(|_| p.to_path_buf());
    let mut out = PathBuf::new();
    for c in full.components() {
        match c {
            PathPart::CurDir => {}
            PathPart::ParentDir if matches!(out.components().next_back(), Some(PathPart::Normal(_))) => {
                out.pop();
            }
            c => out.push(c.as_os_str()),
        }
    }
    out
}

/// `target` relative to the folder `base` (both absolute), when they share a root.
fn relative(base: &Path, target: &Path) -> Option<PathBuf> {
    let (b, t): (Vec<_>, Vec<_>) = (base.components().collect(), target.components().collect());
    let root = |c: &[PathPart]| c.iter().take_while(|x| matches!(x, PathPart::Prefix(_) | PathPart::RootDir)).count();
    if root(&b) == 0 || b.get(..root(&b)) != t.get(..root(&b)) {
        return None;
    }
    let common = b.iter().zip(&t).take_while(|(x, y)| x == y).count();
    let mut out = PathBuf::new();
    for _ in common..b.len() {
        out.push("..");
    }
    for c in t.iter().skip(common) {
        out.push(c.as_os_str());
    }
    Some(out)
}

/// How a link stores `file` for a design saved at `design` (`None`: not saved yet).
fn stored(design: Option<&str>, file: &Path) -> String {
    let file = absolute(file);
    match base_dir(design).and_then(|d| relative(&d, &file)) {
        // Relative paths are written with `/`, so a design moves between systems with its parts.
        Some(rel) => rel.components().map(|c| c.as_os_str().to_string_lossy().into_owned()).collect::<Vec<_>>().join("/"),
        None => file.to_string_lossy().into_owned(),
    }
}

/// The file a stored link path names, for a design saved at `design`.
fn resolve(design: Option<&str>, link: &str) -> PathBuf {
    let p = Path::new(link);
    match base_dir(design) {
        Some(d) if p.is_relative() => d.join(p),
        _ => p.to_path_buf(),
    }
}

/// The links of `doc` stored for a design saved at `to` instead of at `from`.
pub(super) fn rebased(doc: &Document, from: Option<&str>, to: &str) -> std::collections::BTreeMap<u64, DesignLink> {
    doc.links
        .iter()
        .map(|(c, l)| (*c, DesignLink { path: stored(Some(to), &resolve(from, &l.path)), feature: l.feature, id: l.id.clone() }))
        .collect()
}

/// Reads the design at `file`.
fn read_design_at(file: &Path) -> Result<Document> {
    // A part open in another tab counts as it is now, not as it was saved.
    if let Some(open) = crate::workspace::open_design(file) {
        return Ok(open);
    }
    let shown = file.display().to_string();
    let bytes = solidcraft_io::vfs::read(&shown, solidcraft_io::MAX_DESIGN_BYTES).map_err(|e| EngineError::Other(format!("{shown}: {e}")))?;
    solidcraft_io::read_design(&bytes).map_err(|e| EngineError::Other(format!("{shown}: {e}")))
}

/// How deep and how wide `reaches` looks before it gives up (and refuses).
const REACH_DEPTH: usize = 16;
const REACH_FILES: usize = 256;

/// Does the design at `file` link `target`, directly or through the designs it links? A file
/// that can't be read has no links to follow. Giving up (too deep or too many files) counts as
/// reaching it: a chain that long is refused rather than followed.
fn reaches(file: &Path, target: &Path) -> bool {
    fn walk(file: &Path, target: &Path, depth: usize, seen: &mut std::collections::BTreeSet<PathBuf>) -> bool {
        if depth > REACH_DEPTH || seen.len() > REACH_FILES {
            return true;
        }
        let file = absolute(file);
        if !seen.insert(file.clone()) {
            return false;
        }
        let Ok(doc) = read_design_at(&file) else { return false };
        let own = file.to_string_lossy().into_owned();
        doc.links.values().any(|l| {
            let linked = absolute(&resolve(Some(&own), &l.path));
            linked == target || walk(&linked, target, depth + 1, seen)
        })
    }
    walk(file, &absolute(target), 0, &mut std::collections::BTreeSet::new())
}

/// Every part file the design at `path` links, directly or through the parts it links, as the
/// files themselves (absolute). A host that fetches files before a command can read them (the
/// browser) asks this first, fetches what it lists, and only then opens the design. The design
/// itself must be readable; a part that is not is listed and its own links are not followed.
pub fn referenced_files(path: &str) -> Result<Vec<String>> {
    let root = absolute(Path::new(path));
    read_design_at(&root)?;
    let (mut out, mut seen, mut queue) = (Vec::new(), std::collections::BTreeSet::new(), vec![root]);
    while let Some(file) = queue.pop() {
        if !seen.insert(file.clone()) || seen.len() > REACH_FILES {
            continue;
        }
        let Ok(doc) = read_design_at(&file) else { continue };
        let own = file.to_string_lossy().into_owned();
        for link in doc.links.values() {
            let linked = absolute(&resolve(Some(&own), &link.path));
            let shown = linked.to_string_lossy().into_owned();
            if !out.contains(&shown) {
                out.push(shown);
                queue.push(linked);
            }
        }
    }
    Ok(out)
}

/// The solid bodies of the design at `file`, evaluated, as an Import feature: its name, its
/// identity and the feature.
fn linked_bodies(file: &Path, own: Option<&str>) -> Result<(String, String, Option<solidcraft_io::StepFeature>)> {
    let shown = file.display().to_string();
    if let Some(o) = own {
        if absolute(Path::new(o)) == absolute(file) {
            return Err(EngineError::Other(format!("{shown}: a design can't be linked into itself")));
        }
        if reaches(file, Path::new(o)) {
            return Err(EngineError::Other(format!(
                "{shown}: it already links this design (or links too many designs to follow), so it can't be linked here"
            )));
        }
    }
    let doc = read_design_at(file)?;
    let (name, id) = (doc.name.clone(), doc.id.clone());
    let mut part = Session::new(doc);
    part.path = Some(shown.clone());
    // The parts it links come up to date first. One that is missing keeps the bodies it has.
    if !part.doc.links.is_empty() {
        let _ = part.execute("component.update_links", &json!({}));
    }
    // Nothing modelled yet: the part links as an empty component.
    if !part.world_state().bodies.iter().any(|b| !b.body.is_mesh()) && part.model.results.iter().all(|r| r.error.is_none()) {
        return Ok((name, id, None));
    }
    let (_, feature) = session_bodies(&part, &shown)?;
    Ok((name, id, Some(feature)))
}

/// The solid bodies of `part`, the design saved as `shown`, as an Import feature.
fn session_bodies(part: &Session, shown: &str) -> Result<(String, solidcraft_io::StepFeature)> {
    let name = part.doc.name.clone();
    if let Some(e) = part.model.results.iter().find_map(|r| r.error.clone()) {
        return Err(EngineError::Other(format!("{shown}: the design has a feature that fails ({e}); fix it before linking it")));
    }
    let world = part.world_state();
    let solids: Vec<String> = world.bodies.iter().filter(|b| !b.body.is_mesh()).map(|b| b.name.clone()).collect();
    if solids.is_empty() {
        return Err(EngineError::Other(format!("{shown}: the design has no solid bodies to link")));
    }
    let step = solidcraft_io::export(&part.doc.painted(&world), &solids, solidcraft_io::Format::Step, &name)
        .map_err(|e| EngineError::Other(format!("{shown}: {e}")))?;
    let mut feature = solidcraft_io::step_import_feature(&step, shown).map_err(|e| EngineError::Other(format!("{shown}: {e}")))?;
    // The faces keep the names they have in the part, so what refers to one follows the part.
    let carried = carried_face_names(&world, &solids, &feature.import);
    if carried.iter().any(|names| names.iter().any(|n| !n.is_empty()))
        && let FeatureKind::Import { part_face_names, .. } = &mut feature.kind
    {
        *part_face_names = carried;
    }
    if world.bodies.len() > solids.len() {
        feature.warnings.push(format!("{} mesh bodies of {shown} are not linked", world.bodies.len() - solids.len()));
    }
    Ok((name, feature))
}

/// What the faces of the part's solid bodies are called in the part, laid over the faces of the
/// file read back from their export: for each body of the file (by index), each face's name in the
/// part, or empty where the face could not be matched. The file's faces come back in an order of
/// their own, so each is matched to the part's face it lies on (the same area, about the same
/// centre) rather than taken by position; a part face two file faces match is left to neither.
fn carried_face_names(world: &solidcraft_doc::ModelState, solids: &[String], imp: &solidcraft_kernel::StepImport) -> Vec<Vec<String>> {
    imp.bodies
        .iter()
        .enumerate()
        .map(|(k, file_body)| {
            let named = |n: &String| world.body(n).filter(|b| !b.body.is_mesh());
            let by_name = solids.iter().find(|n| **n == file_body.name).and_then(named);
            let Some(source) = by_name.or_else(|| solids.get(k).and_then(named)) else { return Vec::new() };
            let names = solidcraft_doc::naming::face_names(source);
            let tol = (source.body.size() * 1e-3).max(1e-3);
            let (Ok(part), Ok(file)) = (source.body.faces(tol), file_body.body.faces(tol)) else { return Vec::new() };
            let reach = (source.body.size() * 0.02).max(1e-3);
            let mut claimed: Vec<Option<usize>> = Vec::with_capacity(file.len());
            for f in &file {
                let near = part
                    .iter()
                    .filter(|p| (p.area - f.area).abs() <= 0.02 * p.area.max(f.area).max(1e-9) && p.centroid.dist(f.centroid) <= reach)
                    .min_by(|a, b| a.centroid.dist(f.centroid).total_cmp(&b.centroid.dist(f.centroid)));
                claimed.push(near.map(|p| p.index));
            }
            let mut count = std::collections::HashMap::new();
            for i in claimed.iter().flatten() {
                *count.entry(*i).or_insert(0usize) += 1;
            }
            file.iter()
                .zip(&claimed)
                .map(|(_, c)| c.filter(|i| count.get(i) == Some(&1)).and_then(|i| names.get(i)).cloned().unwrap_or_default())
                .collect()
        })
        .collect()
}

/// The bodies of a linked part, as an Import feature in component `comp`.
fn attach_bodies(s: &mut Session, comp: u64, name: &str, sf: solidcraft_io::StepFeature) -> Result<u64> {
    let id = s.doc_mut().add_feature(sf.kind, Some(name))?;
    if let Some(f) = s.doc_mut().feature_mut(id) {
        f.component = comp;
        f.body_names = sf.body_names;
    }
    Ok(id)
}

fn component_key(s: &Session, p: &Value, key: &str, cmd: &str) -> Result<u64> {
    let k = match p.get(key) {
        Some(Value::Number(n)) => n.to_string(),
        Some(Value::String(x)) => x.clone(),
        _ => return Err(bad(cmd, format!("`{key}` must be a component id or name"))),
    };
    s.doc.find_component(&k).ok_or_else(|| bad(cmd, format!("no component `{k}`")))
}

fn insert_design(s: &mut Session, p: &Value) -> Result<Value> {
    let cmd = "component.insert_design";
    let given = str_(p, "path").filter(|x| !x.trim().is_empty() && x.len() < 4096).ok_or_else(|| bad(cmd, "`path` is required"))?;
    let translate = match p.get("translate") {
        None => solidcraft_geom::Vec3::ZERO,
        Some(v) => vec3(v).filter(|t| t.is_finite() && t.len() < 1e9).ok_or_else(|| bad(cmd, "`translate` must be [x, y, z] in mm"))?,
    };
    let file = absolute(Path::new(given));
    if s.path.as_deref().is_some_and(|own| absolute(Path::new(own)) == file) {
        return Err(bad(cmd, "a design can't be linked into itself"));
    }
    if p.get("reuse").and_then(Value::as_bool).unwrap_or(false) {
        let existing = s.doc.links.iter().find(|(_, link)| absolute(&resolve(s.path.as_deref(), &link.path)) == file).map(|(id, _)| *id);
        if let Some(component) = existing {
            let parent = p.get("parent").cloned().unwrap_or_else(|| json!(s.active_component));
            return super::component::copy_occurrence(s, &json!({"component": component, "parent": parent, "translate": translate}));
        }
    }
    let (design_name, design_id, sf) = linked_bodies(&file, s.path.as_deref())?;
    let name = str_(p, "name").filter(|n| !n.trim().is_empty()).map(str::to_string).unwrap_or(design_name);
    let mut create = json!({"name": name, "activate": false});
    if let Some(parent) = p.get("parent") {
        create["parent"] = parent.clone();
    }
    let made = super::component::new_component(s, &create)?;
    let comp = made["component"].as_u64().ok_or_else(|| bad(cmd, "the component was not created"))?;
    let occ = made["occurrence"].as_u64();
    // A part with nothing modelled yet has no Import feature (id 0) until it has bodies.
    let (id, warnings) = match sf {
        Some(sf) => {
            let warnings = sf.warnings.clone();
            (attach_bodies(s, comp, &name, sf)?, warnings)
        }
        None => (0, Vec::new()),
    };
    if let Some(o) = s.doc_mut().occurrences.iter_mut().find(|o| Some(o.id) == occ) {
        o.transform[3] = [translate.x, translate.y, translate.z, 1.0];
    }
    let path = stored(s.path.as_deref(), &file);
    s.doc_mut().links.insert(comp, DesignLink { path: path.clone(), feature: id, id: design_id });
    s.refresh();
    if id != 0
        && let Some(e) = s.model.result(id).and_then(|r| r.error.clone())
    {
        return Err(EngineError::Other(e));
    }
    let st = s.model.state();
    let bodies: Vec<&str> = st.bodies.iter().filter(|b| id != 0 && b.feature == id).map(|b| b.name.as_str()).collect();
    Ok(
        json!({"component": comp, "occurrence": occ, "name": name, "feature": id, "path": path, "bodies": bodies, "created_bodies": bodies, "warnings": warnings}),
    )
}

fn new_part(s: &mut Session, p: &Value) -> Result<Value> {
    let cmd = "component.new_part";
    let name = str_(p, "name")
        .map(str::trim)
        .filter(|n| !n.is_empty() && n.len() < 256 && !n.contains(['/', '\\']))
        .ok_or_else(|| bad(cmd, "`name` is required"))?;
    let file = match str_(p, "path").filter(|x| !x.trim().is_empty() && x.len() < 4096) {
        Some(given) => absolute(Path::new(given)),
        None => base_dir(s.path.as_deref())
            .map(|d| d.join(format!("{name}.solidcraft")))
            .ok_or_else(|| bad(cmd, "save this design first (the part is made beside it), or give `path`"))?,
    };
    let shown = file.display().to_string();
    if file.extension().is_none_or(|e| !e.eq_ignore_ascii_case("solidcraft")) {
        return Err(bad(cmd, "`path` must end in .solidcraft"));
    }
    if solidcraft_io::vfs::exists(&shown) {
        return Err(bad(cmd, format!("{shown} exists already; give another name or use component.insert_design")));
    }
    // The new part: an empty design of the same approach. It links as an empty component; its
    // bodies come in once something is modelled in it.
    let mut part = Session::new(Document::new(name));
    part.doc_mut().approach = s.doc.approach;
    part.execute("file.save", &json!({"path": shown}))?;
    let mut insert = json!({"path": shown, "name": name});
    for key in ["parent", "translate"] {
        if let Some(v) = p.get(key) {
            insert[key] = v.clone();
        }
    }
    let mut r = insert_design(s, &insert)?;
    r["file"] = json!(shown);
    Ok(r)
}

fn update_links(s: &mut Session, p: &Value) -> Result<Value> {
    let cmd = "component.update_links";
    let only = if p.get("component").is_some() { Some(component_key(s, p, "component", cmd)?) } else { None };
    if let Some(c) = only
        && !s.doc.links.contains_key(&c)
    {
        return Err(bad(cmd, "that component is not linked to a design"));
    }
    let links: Vec<(u64, DesignLink)> = s.doc.links.iter().filter(|(c, _)| only.is_none_or(|o| o == **c)).map(|(c, l)| (*c, l.clone())).collect();
    let (mut updated, mut unchanged, mut warnings) = (Vec::new(), Vec::new(), Vec::new());
    for (comp, link) in links {
        let cname = s.doc.components.iter().find(|c| c.id == comp).map(|c| c.name.clone());
        let Some(cname) = cname else {
            // The component was deleted: the link goes with it.
            s.doc_mut().links.remove(&comp);
            continue;
        };
        // The Import feature holding the part's bodies (none while the part has no solids).
        let held = if link.feature == 0 {
            None
        } else {
            match s.doc.feature(link.feature).filter(|f| f.component == comp).map(|f| &f.kind) {
                Some(FeatureKind::Import { step, part_face_names, .. }) => Some((step.clone(), part_face_names.clone())),
                _ => {
                    return Err(bad(
                        cmd,
                        format!("{cname}: its linked bodies (feature {}) are gone; break the link or insert the design again", link.feature),
                    ));
                }
            }
        };
        let file = resolve(s.path.as_deref(), &link.path);
        let (_, design_id, found) = linked_bodies(&file, s.path.as_deref()).map_err(|e| bad(cmd, format!("{cname}: {e}")))?;
        if let Some(sf) = &found {
            warnings.extend(sf.warnings.iter().cloned());
        }
        if !link.id.is_empty() && !design_id.is_empty() && link.id != design_id {
            warnings.push(format!("{cname}: the file {} is a different design now than the one that was linked", link.path));
        }
        if link.id != design_id
            && let Some(l) = s.doc_mut().links.get_mut(&comp)
        {
            l.id = design_id;
        }
        match (held, found) {
            // Still nothing modelled in it.
            (None, None) => unchanged.push(cname),
            // Its first bodies.
            (None, Some(sf)) => {
                let id = attach_bodies(s, comp, &cname, sf)?;
                if let Some(l) = s.doc_mut().links.get_mut(&comp) {
                    l.feature = id;
                }
                updated.push(cname);
            }
            // Every body deleted from it.
            (Some(_), None) => {
                s.doc_mut().delete_feature(link.feature)?;
                if let Some(l) = s.doc_mut().links.get_mut(&comp) {
                    l.feature = 0;
                }
                updated.push(cname);
            }
            (Some(old), Some(sf)) => {
                if matches!(&sf.kind, FeatureKind::Import { step, part_face_names, .. } if (step, part_face_names) == (&old.0, &old.1)) {
                    unchanged.push(cname);
                    continue;
                }
                if let Some(f) = s.doc_mut().feature_mut(link.feature) {
                    // Bodies keep their names where the design still has as many.
                    if f.body_names.len() != sf.body_names.len() {
                        f.body_names = sf.body_names;
                    }
                    f.kind = sf.kind;
                }
                updated.push(cname);
            }
        }
    }
    s.refresh();
    if let Some(e) = s.doc.links.values().filter(|l| l.feature != 0).find_map(|l| s.model.result(l.feature).and_then(|r| r.error.clone())) {
        return Err(EngineError::Other(e));
    }
    Ok(json!({"updated": updated, "unchanged": unchanged, "warnings": warnings}))
}

fn relink(s: &mut Session, p: &Value) -> Result<Value> {
    let cmd = "component.relink";
    let comp = component_key(s, p, "component", cmd)?;
    let link = s.doc.links.get(&comp).cloned().ok_or_else(|| bad(cmd, "that component is not linked to a design"))?;
    if !matches!(s.doc.feature(link.feature), Some(f) if f.component == comp && matches!(f.kind, FeatureKind::Import { .. })) {
        return Err(bad(cmd, "the linked bodies are gone; break the link or insert the design again"));
    }
    let given = str_(p, "path").filter(|x| !x.trim().is_empty() && x.len() < 4096).ok_or_else(|| bad(cmd, "`path` is required"))?;
    let file = absolute(Path::new(given));
    if !file.extension().and_then(|e| e.to_str()).is_some_and(|e| e.eq_ignore_ascii_case("solidcraft")) {
        return Err(bad(cmd, "choose a saved .solidcraft design"));
    }
    // The ordinary lexical check in linked_bodies also works in the browser VFS. On native
    // hosts, reject an alias of this assembly too (a symlink or a path with different case).
    if let (Ok(candidate), Some(Ok(own))) = (file.canonicalize(), s.path.as_deref().map(std::fs::canonicalize))
        && candidate == own
    {
        return Err(bad(cmd, "a design can't be linked into itself"));
    }
    // Read/evaluate the candidate before changing anything. Session::execute owns the single
    // undo step and restores both the document and evaluated state on any later failure.
    let (_, design_id, sf) = linked_bodies(&file, s.path.as_deref()).map_err(|e| bad(cmd, e.to_string()))?;
    let Some(sf) = sf else { return Err(bad(cmd, "that design has no solid bodies to link yet")) };
    let geometry_changed = match (s.doc.feature(link.feature).map(|f| &f.kind), &sf.kind) {
        (Some(FeatureKind::Import { step: old, .. }), FeatureKind::Import { step, .. }) => old != step,
        _ => true,
    };
    let path = stored(s.path.as_deref(), &file);
    let mut warnings = sf.warnings;
    if geometry_changed
        && s.doc
            .assembly
            .joints
            .iter()
            .any(|j| s.doc.occurrences.iter().any(|o| o.component == comp && (j.a.occurrence == o.id || j.b.occurrence == o.id)))
    {
        warnings.push("the linked geometry changed; check this component's joints, even when the solver reports no error".to_string());
    }
    if let Some(f) = s.doc_mut().feature_mut(link.feature) {
        if f.body_names.len() != sf.body_names.len() {
            f.body_names = sf.body_names;
        }
        f.kind = sf.kind;
    }
    s.doc_mut().links.insert(comp, DesignLink { path: path.clone(), feature: link.feature, id: design_id });
    s.refresh();
    if let Some(error) = s.model.result(link.feature).and_then(|r| r.error.clone()) {
        return Err(bad(cmd, error));
    }
    // Report the solver's diagnostics. A resolved joint can still describe the wrong
    // connection after a source edit; the reminder above does not depend on solve errors.
    let sol = solidcraft_doc::joints::solve(&s.doc, &s.model.state());
    let diagnostics =
        |rows: &[(u64, String, String)]| rows.iter().map(|(id, name, why)| json!({"joint": id, "name": name, "why": why})).collect::<Vec<_>>();
    for (_, name, why) in sol.errors.iter().chain(&sol.conflicts) {
        warnings.push(format!("joint {name}: {why}"));
    }
    Ok(json!({"component": comp, "feature": link.feature, "path": path, "previous_path": link.path, "geometry_changed": geometry_changed,
        "warnings": warnings, "joint_errors": diagnostics(&sol.errors), "joint_conflicts": diagnostics(&sol.conflicts)}))
}

fn break_link(s: &mut Session, p: &Value) -> Result<Value> {
    let cmd = "component.break_link";
    let comp = component_key(s, p, "component", cmd)?;
    let link = s.doc_mut().links.remove(&comp).ok_or_else(|| bad(cmd, "that component is not linked to a design"))?;
    Ok(json!({"component": comp, "path": link.path}))
}

fn list_links(s: &mut Session, _p: &Value) -> Result<Value> {
    let links: Vec<Value> = s
        .doc
        .links
        .iter()
        .map(|(c, l)| {
            let file = resolve(s.path.as_deref(), &l.path);
            let name = s.doc.components.iter().find(|x| x.id == *c).map(|x| x.name.clone());
            json!({"component": c, "name": name, "path": l.path, "file": file.display().to_string(), "found": solidcraft_io::vfs::exists(&file.display().to_string())})
        })
        .collect();
    Ok(json!({"links": links}))
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
mod tests {
    use super::*;

    #[test]
    fn link_paths_are_kept_relative_to_the_design_and_follow_save_as() {
        let root = absolute(Path::new("projects"));
        let part = root.join("parts").join("plate.solidcraft");
        let design = root.join("assembly").join("case.solidcraft");
        let design = design.to_string_lossy().into_owned();
        assert_eq!(stored(Some(&design), &part), "../parts/plate.solidcraft");
        assert_eq!(absolute(&resolve(Some(&design), "../parts/plate.solidcraft")), part);
        // A design path given relative to the working folder works the same.
        assert_eq!(stored(Some("case.solidcraft"), &absolute(Path::new("parts/plate.solidcraft"))), "parts/plate.solidcraft");
        // Before the design has a file the path is kept whole; saving it makes it relative.
        let unsaved = stored(None, &part);
        assert!(Path::new(&unsaved).is_absolute(), "{unsaved}");
        let mut doc = Document::default();
        doc.links.insert(7, DesignLink { path: unsaved, feature: 3, id: String::new() });
        let moved = rebased(&doc, None, &design);
        assert_eq!(moved.get(&7).map(|l| l.path.as_str()), Some("../parts/plate.solidcraft"));
        let beside = root.join("parts").join("asm.solidcraft").to_string_lossy().into_owned();
        assert_eq!(
            rebased(&Document { links: moved, ..Document::default() }, Some(&design), &beside).get(&7).map(|l| l.path.as_str()),
            Some("plate.solidcraft")
        );
    }

    fn run(s: &mut Session, id: &str, p: Value) -> Value {
        s.execute(id, &p).unwrap_or_else(|e| panic!("{id} {p}: {e}"))
    }

    fn save_box(path: &Path, height: f64) {
        let mut part = Session::default();
        run(&mut part, "solid.box", json!({"length": 10, "width": 20, "height": height, "body_name": "Plate"}));
        run(&mut part, "file.save", json!({"path": path}));
    }

    fn volume(s: &mut Session) -> f64 {
        run(s, "inspect.measure", json!({}))["total"]["volume_mm3"].as_f64().unwrap_or(f64::NAN)
    }

    #[test]
    fn a_saved_assembly_reopens_with_its_link_updates_it_and_breaks_it() {
        let dir = std::env::temp_dir().join(format!("solidcraft-link-reopen-{}", std::process::id()));
        std::fs::create_dir_all(dir.join("parts")).unwrap();
        std::fs::create_dir_all(dir.join("asm")).unwrap();
        let part = dir.join("parts").join("plate.solidcraft");
        let case = dir.join("asm").join("case.solidcraft");
        save_box(&part, 30.0);
        // Given an assembly holding the part, saved in a folder beside the part's.
        let mut a = Session::default();
        let inserted = run(&mut a, "component.insert_design", json!({"path": part}));
        assert_eq!(inserted["name"], "plate");
        run(&mut a, "file.save", json!({"path": case}));
        assert!((volume(&mut a) - 6000.0).abs() < 1e-6);
        // When it is opened again, the link is kept relative to it and finds the part.
        let mut b = Session::default();
        run(&mut b, "doc.open", json!({"path": case}));
        let links = run(&mut b, "component.links", json!({}));
        assert_eq!(links["links"][0]["path"], "../parts/plate.solidcraft", "{links}");
        assert_eq!(links["links"][0]["found"], true, "{links}");
        assert!((volume(&mut b) - 6000.0).abs() < 1e-6);
        // An unchanged part leaves the assembly as it is; an edited one comes in on update.
        assert_eq!(run(&mut b, "component.update_links", json!({}))["unchanged"], json!(["plate"]));
        save_box(&part, 45.0);
        assert_eq!(run(&mut b, "component.update_links", json!({}))["updated"], json!(["plate"]));
        assert!((volume(&mut b) - 9000.0).abs() < 1e-6, "{}", volume(&mut b));
        // A part file that has gone is reported, and the assembly keeps the bodies it had.
        std::fs::remove_file(&part).unwrap();
        let err = b.execute("component.update_links", &json!({})).unwrap_err().to_string();
        assert!(err.contains("plate"), "{err}");
        assert!((volume(&mut b) - 9000.0).abs() < 1e-6);
        // Break Link keeps the bodies and forgets the file.
        run(&mut b, "component.break_link", json!({"component": "plate"}));
        assert_eq!(run(&mut b, "component.links", json!({}))["links"], json!([]));
        assert!((volume(&mut b) - 9000.0).abs() < 1e-6);
        assert!(b.execute("component.update_links", &json!({"component": "plate"})).is_err());
        let _ = std::fs::remove_dir_all(&dir);
    }

    fn temp_dir(tag: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("solidcraft-{tag}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn a_design_gets_an_identity_when_saved_and_keeps_it_through_undo_and_save_as() {
        let dir = temp_dir("identity");
        let (one, two) = (dir.join("one.solidcraft"), dir.join("two.solidcraft"));
        let mut s = Session::default();
        assert!(s.doc.id.is_empty());
        run(&mut s, "solid.box", json!({"length": 5, "width": 5, "height": 5}));
        run(&mut s, "file.save", json!({"path": one}));
        let id = s.doc.id.clone();
        assert_eq!(id.len(), 32);
        run(&mut s, "solid.box", json!({"length": 1, "width": 1, "height": 1, "body_name": "Other"}));
        run(&mut s, "edit.undo", json!({}));
        assert_eq!(s.doc.id, id, "undo keeps the identity");
        run(&mut s, "file.save", json!({"path": one}));
        assert_eq!(s.doc.id, id, "saving again keeps it");
        run(&mut s, "file.save_as", json!({"path": two}));
        assert_eq!(s.doc.id, id, "Save As keeps the design's identity");
        let mut reopened = Session::default();
        run(&mut reopened, "doc.open", json!({"path": one}));
        assert_eq!(reopened.doc.id, id);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn a_design_cannot_link_one_that_links_it_however_far_down() {
        let dir = temp_dir("cycle");
        let (a, b, c) = (dir.join("a.solidcraft"), dir.join("b.solidcraft"), dir.join("c.solidcraft"));
        save_box(&c, 10.0);
        // b links c; a links b.
        let mut sb = Session::default();
        run(&mut sb, "component.insert_design", json!({"path": c}));
        run(&mut sb, "file.save", json!({"path": b}));
        let mut sa = Session::default();
        run(&mut sa, "component.insert_design", json!({"path": b}));
        run(&mut sa, "file.save", json!({"path": a}));
        // c can't take a (a reaches c through b), and b can't take a either.
        let mut sc = Session::default();
        run(&mut sc, "file.save", json!({"path": c}));
        let err = sc.execute("component.insert_design", &json!({"path": a})).unwrap_err().to_string();
        assert!(err.contains("already links"), "{err}");
        let mut sb2 = Session::default();
        run(&mut sb2, "doc.open", json!({"path": b}));
        assert!(sb2.execute("component.insert_design", &json!({"path": a})).is_err());
        assert!(sb2.doc.links.len() == 1 && sb2.doc.components.len() == 1, "nothing changed");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn editing_a_leaf_part_reaches_the_assembly_two_levels_up() {
        let dir = temp_dir("nested");
        let (leaf, mid, top) = (dir.join("leaf.solidcraft"), dir.join("mid.solidcraft"), dir.join("top.solidcraft"));
        save_box(&leaf, 30.0);
        let mut m = Session::default();
        run(&mut m, "component.insert_design", json!({"path": leaf}));
        run(&mut m, "file.save", json!({"path": mid}));
        let mut t = Session::default();
        run(&mut t, "component.insert_design", json!({"path": mid}));
        run(&mut t, "file.save", json!({"path": top}));
        assert!((volume(&mut t) - 6000.0).abs() < 1e-6);
        // The leaf changes; the middle part is not opened or saved, yet the top follows.
        save_box(&leaf, 45.0);
        assert_eq!(run(&mut t, "component.update_links", json!({}))["updated"], json!(["mid"]));
        assert!((volume(&mut t) - 9000.0).abs() < 1e-6, "{}", volume(&mut t));
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn a_new_part_is_a_file_of_its_own_linked_in_and_reusable() {
        let dir = temp_dir("new-part");
        let case = dir.join("case.solidcraft");
        let mut s = Session::default();
        s.doc_mut().approach = solidcraft_doc::Approach::Advanced;
        // Unsaved: nowhere to put the part.
        assert!(s.execute("component.new_part", &json!({"name": "Lid"})).is_err());
        run(&mut s, "file.save", json!({"path": case}));
        let made = run(&mut s, "component.new_part", json!({"name": "Lid"}));
        assert!(dir.join("Lid.solidcraft").is_file(), "{made}");
        assert_eq!(s.doc.links.len(), 1);
        assert_eq!(run(&mut s, "component.links", json!({}))["links"][0]["path"], "Lid.solidcraft");
        // It opens on its own as an Advanced part with an identity, and can be used again.
        let mut part = Session::default();
        run(&mut part, "doc.open", json!({"path": dir.join("Lid.solidcraft")}));
        assert_eq!(part.doc.approach, solidcraft_doc::Approach::Advanced);
        assert_eq!(part.doc.id.len(), 32);
        assert_eq!(s.doc.links.values().next().map(|l| l.id.clone()), Some(part.doc.id.clone()));
        run(&mut s, "component.insert_design", json!({"path": dir.join("Lid.solidcraft"), "name": "Lid 2", "translate": [50, 0, 0]}));
        assert_eq!(s.doc.links.len(), 2);
        // Names that would escape the folder, and existing files, are refused.
        assert!(s.execute("component.new_part", &json!({"name": "../Out"})).is_err());
        assert!(s.execute("component.new_part", &json!({"name": "Lid"})).is_err());
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn a_file_replaced_by_another_design_is_reported_on_update() {
        let dir = temp_dir("replaced");
        let (part, case) = (dir.join("p.solidcraft"), dir.join("case.solidcraft"));
        save_box(&part, 30.0);
        let mut s = Session::default();
        run(&mut s, "component.insert_design", json!({"path": part}));
        run(&mut s, "file.save", json!({"path": case}));
        // Another design is saved over the part's file (new design, new identity).
        std::fs::remove_file(&part).unwrap();
        save_box(&part, 40.0);
        let r = run(&mut s, "component.update_links", json!({}));
        let warnings = r["warnings"].to_string();
        assert!(warnings.contains("different design"), "{warnings}");
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// A store that keeps files in memory, as a PDM or database backend would.
    #[derive(Default)]
    struct MemoryStore(std::sync::Mutex<std::collections::BTreeMap<String, Vec<u8>>>);

    impl solidcraft_io::vfs::DesignStore for MemoryStore {
        fn read(&self, path: &str, max: usize) -> std::io::Result<Vec<u8>> {
            let bytes = self.0.lock().unwrap().get(path).cloned().ok_or_else(|| std::io::Error::from(std::io::ErrorKind::NotFound))?;
            if bytes.len() > max { Err(std::io::Error::from(std::io::ErrorKind::InvalidData)) } else { Ok(bytes) }
        }
        fn write(&self, path: &str, bytes: &[u8]) -> std::io::Result<()> {
            self.0.lock().unwrap().insert(path.to_string(), bytes.to_vec());
            Ok(())
        }
        fn len(&self, path: &str) -> std::io::Result<u64> {
            self.0.lock().unwrap().get(path).map(|b| b.len() as u64).ok_or_else(|| std::io::Error::from(std::io::ErrorKind::NotFound))
        }
        fn exists(&self, path: &str) -> bool {
            self.0.lock().unwrap().contains_key(path)
        }
        fn remove(&self, path: &str) -> std::io::Result<()> {
            self.0.lock().unwrap().remove(path).map(|_| ()).ok_or_else(|| std::io::Error::from(std::io::ErrorKind::NotFound))
        }
        fn create_dir_all(&self, _path: &str) -> std::io::Result<()> {
            Ok(())
        }
    }

    #[test]
    fn parts_link_save_open_and_update_through_an_installed_store_without_touching_the_disk() {
        let store = std::sync::Arc::new(MemoryStore::default());
        let root = std::env::temp_dir().join(format!("solidcraft-store-{}-never-created", std::process::id()));
        let (case, lid) = (root.join("case.solidcraft"), root.join("Lid.solidcraft"));
        solidcraft_io::vfs::with_store(store.clone(), || {
            let mut s = Session::default();
            run(&mut s, "file.save", json!({"path": case}));
            run(&mut s, "component.new_part", json!({"name": "Lid"}));
            run(&mut s, "file.save", json!({}));
            assert!(solidcraft_io::vfs::exists(&lid.to_string_lossy()));
            assert_eq!(run(&mut s, "component.links", json!({}))["links"][0]["found"], true);
            // The part is opened and changed through the same store; the assembly updates from it.
            let mut part = Session::default();
            run(&mut part, "doc.open", json!({"path": lid}));
            run(&mut part, "solid.box", json!({"length": 10, "width": 10, "height": 10, "body_name": "Extra", "corner": [50, 0, 0]}));
            run(&mut part, "file.save", json!({}));
            assert_eq!(run(&mut s, "component.update_links", json!({}))["updated"], json!(["Lid"]));
            assert!((volume(&mut s) - 1000.0).abs() < 1e-6, "the part's one body");
            // A third session opens the assembly from the store.
            let mut again = Session::default();
            run(&mut again, "doc.open", json!({"path": case}));
            assert_eq!(again.doc.links.len(), 1);
        });
        assert!(store.0.lock().unwrap().len() >= 2);
        assert!(!root.exists(), "nothing was written to the disk");
        // Outside the store's scope the disk answers again.
        assert!(!solidcraft_io::vfs::exists(&lid.to_string_lossy()));
    }

    #[test]
    fn referenced_files_lists_every_part_below_a_design_for_a_host_that_fetches_first() {
        let dir = temp_dir("references");
        let (leaf, mid, top) = (dir.join("leaf.solidcraft"), dir.join("mid.solidcraft"), dir.join("top.solidcraft"));
        save_box(&leaf, 30.0);
        let mut m = Session::default();
        run(&mut m, "component.insert_design", json!({"path": leaf}));
        run(&mut m, "file.save", json!({"path": mid}));
        let mut t = Session::default();
        run(&mut t, "component.insert_design", json!({"path": mid}));
        run(&mut t, "file.save", json!({"path": top}));
        let listed = referenced_files(&top.to_string_lossy()).unwrap();
        let names: Vec<_> = listed.iter().map(|p| Path::new(p).file_name().unwrap().to_string_lossy().into_owned()).collect();
        assert_eq!(names, ["mid.solidcraft", "leaf.solidcraft"], "{listed:?}");
        assert!(referenced_files(&leaf.to_string_lossy()).unwrap().is_empty());
        // A part that is gone is still listed (the host may be able to fetch it); the design must exist.
        std::fs::remove_file(&leaf).unwrap();
        assert_eq!(referenced_files(&top.to_string_lossy()).unwrap().len(), 2);
        assert!(referenced_files(&dir.join("nothing.solidcraft").to_string_lossy()).is_err());
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn a_part_open_in_the_window_reaches_its_assembly_before_it_is_saved() {
        let dir = temp_dir("open-part");
        let (part_file, case_file) = (dir.join("plate.solidcraft"), dir.join("case.solidcraft"));
        save_box(&part_file, 30.0);
        let mut case = Session::default();
        run(&mut case, "component.insert_design", json!({"path": part_file}));
        run(&mut case, "file.save", json!({"path": case_file}));
        // The part is open and changed, not saved.
        let mut part = Session::default();
        run(&mut part, "doc.open", json!({"path": part_file}));
        run(&mut part, "solid.box", json!({"length": 10, "width": 10, "height": 10, "body_name": "Extra", "corner": [50, 0, 0]}));
        assert!(part.is_dirty());
        // With nothing registered the file is read, and it has not changed.
        assert_eq!(run(&mut case, "component.update_links", json!({}))["unchanged"], json!(["plate"]));
        // With the part registered as open, its current state is what the assembly takes.
        let open = vec![(part_file.to_string_lossy().into_owned(), part.doc.clone())];
        let r = crate::workspace::with_open_designs(open, || run(&mut case, "component.update_links", json!({})));
        assert_eq!(r["updated"], json!(["plate"]));
        assert!((volume(&mut case) - 7000.0).abs() < 1e-6, "{}", volume(&mut case));
        // The registration ended with the call: the file is read again, and the saved part wins.
        assert_eq!(run(&mut case, "component.update_links", json!({}))["updated"], json!(["plate"]));
        assert!((volume(&mut case) - 6000.0).abs() < 1e-6);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn a_new_part_links_empty_gets_its_bodies_once_modelled_and_loses_them_when_they_go() {
        let dir = temp_dir("empty-part");
        let case_file = dir.join("case.solidcraft");
        let mut case = Session::default();
        run(&mut case, "file.save", json!({"path": case_file}));
        let made = run(&mut case, "component.new_part", json!({"name": "Lid"}));
        assert_eq!(made["bodies"], json!([]), "{made}");
        assert_eq!(case.doc.links.values().next().map(|l| l.feature), Some(0), "no Import feature yet");
        assert!(case.world_state().bodies.is_empty());
        assert_eq!(run(&mut case, "component.update_links", json!({}))["unchanged"], json!(["Lid"]), "still nothing modelled");
        // Something is modelled in the part and saved.
        let lid_file = dir.join("Lid.solidcraft");
        let mut part = Session::default();
        run(&mut part, "doc.open", json!({"path": lid_file}));
        run(&mut part, "solid.box", json!({"length": 10, "width": 20, "height": 30, "body_name": "Plate"}));
        run(&mut part, "file.save", json!({}));
        assert_eq!(run(&mut case, "component.update_links", json!({}))["updated"], json!(["Lid"]));
        assert!((volume(&mut case) - 6000.0).abs() < 1e-6);
        let feature = case.doc.links.values().next().map(|l| l.feature).unwrap();
        assert_ne!(feature, 0);
        assert!(case.doc.feature(feature).is_some_and(|f| matches!(f.kind, FeatureKind::Import { .. })));
        // The body is deleted from the part: the component is empty again.
        run(&mut part, "timeline.delete", json!({"feature": "Box1"}));
        run(&mut part, "file.save", json!({}));
        assert_eq!(run(&mut case, "component.update_links", json!({}))["updated"], json!(["Lid"]));
        assert!(case.world_state().bodies.is_empty());
        assert_eq!(case.doc.links.values().next().map(|l| l.feature), Some(0));
        assert!(case.doc.feature(feature).is_none(), "the Import feature went with its bodies");
        // It saves and reopens as it is, with its link.
        run(&mut case, "file.save", json!({}));
        let mut again = Session::default();
        run(&mut again, "doc.open", json!({"path": case_file}));
        assert_eq!(again.doc.links.len(), 1);
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// The name and height of the face of `body` (of `s`'s model) that looks up and stands highest.
    fn top_face(s: &Session, body: &str) -> (String, f64) {
        let st = s.model.state();
        let b = st.body(body).unwrap_or_else(|| panic!("no body {body}"));
        let faces = b.body.faces((b.body.size() * 1e-3).max(1e-3)).unwrap();
        let top = faces
            .iter()
            .filter(|f| f.plane_normal.is_some_and(|n| n.z > 0.9))
            .max_by(|a, c| a.centroid.z.total_cmp(&c.centroid.z))
            .expect("a face looking up");
        (solidcraft_doc::naming::face_names(b)[top.index].clone(), top.centroid.z)
    }

    fn save_prism(path: &Path, distance: &str) {
        let mut part = Session::default();
        run(&mut part, "sketch.create", json!({"plane": "XY", "name": "Base"}));
        run(&mut part, "sketch.rectangle.two_point", json!({"p0": [0, 0], "p1": [40, 25]}));
        run(&mut part, "sketch.finish", json!({}));
        run(&mut part, "solid.extrude", json!({"sketch": "Base", "distance": distance, "operation": "new", "name": "Boss", "body_name": "Boss"}));
        run(&mut part, "file.save", json!({"path": path}));
    }

    /// The prism with a corner chamfered in its sketch (one more side, so its top face comes later
    /// in the file) and taller.
    fn change_prism(part: &mut Session) {
        run(part, "sketch.edit", json!({"sketch": "Base"}));
        run(part, "sketch.chamfer.equal_distance", json!({"a": "l2", "b": "l3", "distance": 6}));
        run(part, "sketch.finish", json!({}));
        run(part, "timeline.edit", json!({"feature": "Boss", "set": {"extent": {"distance": "25 mm"}}}));
    }

    #[test]
    fn a_linked_parts_faces_keep_the_names_they_have_in_the_part() {
        let dir = temp_dir("carried-names");
        let part_file = dir.join("boss.solidcraft");
        save_prism(&part_file, "10 mm");
        let mut part = Session::default();
        run(&mut part, "doc.open", json!({"path": part_file}));
        let (in_part, _) = top_face(&part, "Boss");
        let mut case = Session::default();
        let made = run(&mut case, "component.insert_design", json!({"path": part_file}));
        let import = made["feature"].as_u64().unwrap();
        let body = made["bodies"][0].as_str().unwrap().to_string();
        let (in_case, _) = top_face(&case, &body);
        assert_eq!(in_case, format!("F{import}:{in_part}"), "the part's own name, under the import's feature");
        // Its place in the file stays an alias, so older references still find it.
        let st = case.model.state();
        let b = st.body(&body).unwrap();
        let index = solidcraft_doc::naming::face_names(b).iter().position(|n| *n == in_case).unwrap();
        assert!(solidcraft_doc::naming::face_has_name(b, index, &format!("F{import}:face:{index}")));
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn what_names_a_face_of_a_linked_part_follows_it_when_the_part_changes_shape() {
        let dir = temp_dir("follows");
        let part_file = dir.join("boss.solidcraft");
        save_prism(&part_file, "10 mm");
        let mut case = Session::default();
        let made = run(&mut case, "component.insert_design", json!({"path": part_file}));
        let body = made["bodies"][0].as_str().unwrap().to_string();
        let (name, height) = top_face(&case, &body);
        assert!((height - 10.0).abs() < 1e-6);
        // The part's corner is chamfered, which puts a new side among the others in the file (the top
        // face moves from the sixth place to the seventh), and the part grows taller: its top face is
        // the same face, though neither in the same place nor in the same position.
        let mut part = Session::default();
        run(&mut part, "doc.open", json!({"path": part_file}));
        change_prism(&mut part);
        run(&mut part, "file.save", json!({}));
        run(&mut case, "component.update_links", json!({}));
        let st = case.model.state();
        let hit = solidcraft_doc::naming::locate_one(&st, Some(&body), &name).expect("the name still finds a face");
        let b = st.body(&body).unwrap();
        let faces = b.body.faces((b.body.size() * 1e-3).max(1e-3)).unwrap();
        let found = faces.iter().find(|f| f.index == hit.face).unwrap();
        assert!(found.plane_normal.is_some_and(|n| n.z > 0.9) && (found.centroid.z - 25.0).abs() < 1e-6, "{:?}", found);
        // And the name is still the one it is under in the part itself.
        assert_eq!(top_face(&case, &body).0, name);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn a_joint_origin_on_a_linked_parts_face_follows_the_face_when_the_part_changes_shape() {
        let dir = temp_dir("joint-follows");
        let part_file = dir.join("boss.solidcraft");
        save_prism(&part_file, "10 mm");
        let mut case = Session::default();
        run(&mut case, "component.insert_design", json!({"path": part_file}));
        // A joint origin at the centre of the top face.
        run(&mut case, "joint.origin", json!({"name": "Top", "face": [20, 12.5, 10]}));
        let origin = |s: &Session| {
            let o = s.doc.assembly.origins.iter().find(|o| o.name == "Top").expect("the joint origin").origin.clone();
            let frame = solidcraft_doc::joints::resolve_origin(&s.doc, &s.model.state(), &o).expect("the origin still resolves");
            (frame[3][2], frame[2][2])
        };
        let (z, up) = origin(&case);
        assert!((z - 10.0).abs() < 1e-6 && up > 0.99, "{z} {up}");
        // The part's corner is chamfered (a new side among the others in the file) and it grows.
        let mut part = Session::default();
        run(&mut part, "doc.open", json!({"path": part_file}));
        change_prism(&mut part);
        run(&mut part, "file.save", json!({}));
        run(&mut case, "component.update_links", json!({}));
        let (z, up) = origin(&case);
        assert!((z - 25.0).abs() < 1e-6 && up > 0.99, "the origin followed the top face: z {z}, normal {up}");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn a_name_survives_two_levels_of_linking_and_follows_the_leaf() {
        let dir = temp_dir("names-nested");
        let (leaf, mid) = (dir.join("boss.solidcraft"), dir.join("mid.solidcraft"));
        save_prism(&leaf, "10 mm");
        let mut m = Session::default();
        run(&mut m, "component.insert_design", json!({"path": leaf}));
        run(&mut m, "file.save", json!({"path": mid}));
        let mut t = Session::default();
        let made = run(&mut t, "component.insert_design", json!({"path": mid}));
        let body = made["bodies"][0].as_str().unwrap().to_string();
        let (name, height) = top_face(&t, &body);
        assert!((height - 10.0).abs() < 1e-6);
        // The leaf changes; the middle part is not opened or saved.
        let mut part = Session::default();
        run(&mut part, "doc.open", json!({"path": leaf}));
        change_prism(&mut part);
        run(&mut part, "file.save", json!({}));
        run(&mut t, "component.update_links", json!({}));
        let st = t.model.state();
        let hit = solidcraft_doc::naming::locate_one(&st, Some(&body), &name).expect("the name still finds a face two levels up");
        let b = st.body(&body).unwrap();
        let faces = b.body.faces((b.body.size() * 1e-3).max(1e-3)).unwrap();
        let found = faces.iter().find(|f| f.index == hit.face).unwrap();
        assert!(found.plane_normal.is_some_and(|n| n.z > 0.9) && (found.centroid.z - 25.0).abs() < 1e-6, "{found:?}");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn inserting_a_missing_file_or_bad_parameters_changes_nothing() {
        let mut s = Session::default();
        let missing = std::env::temp_dir().join(format!("solidcraft-no-such-part-{}.solidcraft", std::process::id()));
        assert!(s.execute("component.insert_design", &json!({"path": missing})).is_err());
        assert!(s.execute("component.insert_design", &json!({})).is_err());
        assert!(s.execute("component.insert_design", &json!({"path": "x.solidcraft", "translate": [1, "a", 2]})).is_err());
        assert!(s.doc.components.is_empty() && s.doc.links.is_empty() && s.doc.features.is_empty());
    }
}
