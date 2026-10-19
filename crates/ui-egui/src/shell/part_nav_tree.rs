//! What the Part Navigator's feature tree knows about each row beyond its name: which features
//! a feature depends on and which depend on it (shown by the colour of their names), and the
//! eye that hides what the feature made. Pure functions of the session, so they are tested alone.

use std::cell::RefCell;
use std::collections::{BTreeSet, HashMap};
use std::rc::Rc;

use crate::SolveApp;

/// The features a selected feature depends on and the features that depend on it.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Related {
    pub parents: BTreeSet<u64>,
    pub children: BTreeSet<u64>,
}

/// The design's dependency graph, worked out once per revision (a click must not wait for it).
#[derive(Default)]
struct Graph {
    /// The bodies each feature made or changed, by name: new in its result, or changed from the
    /// result before it.
    touched: HashMap<u64, Vec<String>>,
    /// The features each feature is built on: its sketches, the features and bodies it names, and
    /// the earlier features that made or changed the bodies it works on.
    parents: HashMap<u64, BTreeSet<u64>>,
}

type Key = (u64, usize, usize, usize);

thread_local! {
    static CACHE: RefCell<Option<(Key, Rc<Graph>)>> = const { RefCell::new(None) };
}

fn build(app: &SolveApp) -> Graph {
    let doc = &app.session.doc;
    let results = &app.session.model.results;
    let mut g = Graph::default();
    for (i, r) in results.iter().enumerate() {
        let before = i.checked_sub(1).and_then(|k| results.get(k));
        let names = r
            .state
            .bodies
            .iter()
            .filter(|b| before.is_none_or(|p| p.state.body(&b.name).is_none_or(|old| old.content_key() != b.content_key())))
            .map(|b| b.name.clone())
            .collect();
        g.touched.insert(r.id, names);
    }
    let by_name: HashMap<String, u64> = doc.features.iter().map(|f| (f.name.to_ascii_lowercase(), f.id)).collect();
    // The last feature to make or change each body, as the features go by in order.
    let mut last: HashMap<&str, u64> = HashMap::new();
    for f in &doc.features {
        let (sketches, features, bodies) = f.kind.references();
        let mut parents = BTreeSet::new();
        parents.extend(sketches.into_iter().filter(|s| doc.feature(*s).is_some()));
        parents.extend(features.iter().filter_map(|n| by_name.get(&n.to_ascii_lowercase()).copied()));
        let mine = g.touched.get(&f.id).map(Vec::as_slice).unwrap_or_default();
        for name in bodies.iter().map(String::as_str).chain(mine.iter().map(String::as_str)) {
            parents.extend(last.get(name).copied());
        }
        parents.remove(&f.id);
        g.parents.insert(f.id, parents);
        for name in mine {
            last.insert(name.as_str(), f.id);
        }
    }
    g
}

fn graph(app: &SolveApp) -> Rc<Graph> {
    let key: Key =
        (app.session.revision, app.session.doc.features.len(), app.session.doc.features.as_ptr() as usize, app.session.model.results.len());
    CACHE.with(|c| {
        if let Some((k, g)) = c.borrow().as_ref()
            && *k == key
        {
            return Rc::clone(g);
        }
        let g = Rc::new(build(app));
        *c.borrow_mut() = Some((key, Rc::clone(&g)));
        g
    })
}

/// Parents and children of feature `id`, one step each way (a child of a parent is not a
/// child), so the tree reads as the recipe it is.
pub fn related(app: &SolveApp, id: u64) -> Related {
    let g = graph(app);
    Related {
        parents: g.parents.get(&id).cloned().unwrap_or_default(),
        children: g.parents.iter().filter(|(_, p)| p.contains(&id)).map(|(c, _)| *c).filter(|c| *c != id).collect(),
    }
}

/// The names of the parents and the children of a feature, in the order of the tree.
pub fn names_of(app: &SolveApp, id: u64) -> (Vec<String>, Vec<String>) {
    let r = related(app, id);
    let pick =
        |set: &BTreeSet<u64>| -> Vec<String> { app.session.doc.features.iter().filter(|f| set.contains(&f.id)).map(|f| f.name.clone()).collect() };
    (pick(&r.parents), pick(&r.children))
}

/// What a row's eye shows and hides.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Eye {
    Sketch(u64),
    /// The bodies the feature made or changed, still in the design: hiding a chamfer hides its
    /// body, and with it every feature that built that body.
    Bodies(Vec<String>),
    /// A datum, by name.
    Datum(String),
}

pub fn eye_of(app: &SolveApp, id: u64) -> Option<Eye> {
    use solidcraft_engine::doc::FeatureKind as K;
    let f = app.session.doc.feature(id)?;
    if matches!(f.kind, K::Sketch { .. }) {
        return Some(Eye::Sketch(id));
    }
    if matches!(f.kind, K::ConstructionPlane { .. } | K::ConstructionAxis { .. } | K::ConstructionPoint { .. }) {
        return Some(Eye::Datum(f.name.clone()));
    }
    let world = app.session.world_state();
    let g = graph(app);
    let bodies: Vec<String> = g.touched.get(&id).into_iter().flatten().filter(|b| world.body(b).is_some()).cloned().collect();
    (!bodies.is_empty()).then_some(Eye::Bodies(bodies))
}

/// Is what the eye covers drawn? Several bodies count as shown while any one is.
pub fn shown(app: &SolveApp, eye: &Eye) -> bool {
    let v = &app.session.visibility;
    match eye {
        Eye::Sketch(id) => crate::browser::sketch_visible(app, *id),
        Eye::Bodies(names) => names.iter().any(|b| !v.hidden_bodies.contains(b)),
        Eye::Datum(name) => !v.hidden_origin.iter().any(|h| h == name),
    }
}

pub fn set_shown(app: &mut SolveApp, eye: &Eye, on: bool) {
    match eye {
        Eye::Sketch(id) => crate::browser::set_sketch_visible(app, *id, on),
        Eye::Bodies(names) => {
            let v = &mut app.session.visibility;
            v.hidden_bodies.retain(|b| !names.contains(b));
            if !on {
                v.hidden_bodies.extend(names.iter().cloned());
            }
        }
        Eye::Datum(name) => {
            let v = &mut app.session.visibility;
            v.hidden_origin.retain(|h| h != name);
            if !on {
                v.hidden_origin.push(name.clone());
            }
        }
    }
}

pub fn toggle(app: &mut SolveApp, id: u64) {
    if let Some(eye) = eye_of(app, id) {
        let on = shown(app, &eye);
        set_shown(app, &eye, !on);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::prefs::ModellingApproach;
    use crate::scenario::Harness;
    use serde_json::json;

    fn id_of(h: &Harness, name: &str) -> u64 {
        h.app.session.doc.find_feature(name).map(|f| f.id).unwrap_or_default()
    }

    /// A sketch, an extrude of it into Main, a chamfer of Main, and a second sketch that nothing uses.
    fn part() -> Harness {
        let mut h = Harness::new();
        h.app.preferences.modelling_approach = ModellingApproach::Advanced;
        h.app.run("sketch.create", json!({"plane":"XY","name":"Base"})).unwrap();
        h.app.run("sketch.rectangle.two_point", json!({"p0":[0,0],"p1":[20,10]})).unwrap();
        h.app.run("sketch.finish", json!({})).unwrap();
        h.app.run("solid.extrude", json!({"sketch":"Base","distance":"10 mm","operation":"new","body_name":"Main"})).unwrap();
        h.app.run("solid.chamfer", json!({"body":"Main","edges":[[0,0,10]],"distance":"1 mm"})).unwrap();
        h.app.run("sketch.create", json!({"plane":"XZ","name":"Spare"})).unwrap();
        h.app.run("sketch.finish", json!({})).unwrap();
        h.frames(2);
        h
    }

    #[test]
    fn a_feature_depends_on_its_sketch_and_the_features_that_built_its_body() {
        let h = part();
        let (base, extrude, spare) = (id_of(&h, "Base"), id_of(&h, "Extrude1"), id_of(&h, "Spare"));
        let r = related(&h.app, extrude);
        assert!(r.parents.contains(&base), "the extrude is built on its sketch: {r:?}");
        assert!(!r.parents.contains(&spare) && !r.children.contains(&spare), "an unrelated sketch is neither");
        let r = related(&h.app, base);
        assert!(r.children.contains(&extrude) && r.parents.is_empty(), "the sketch's child is the extrude: {r:?}");
    }

    #[test]
    fn the_eye_hides_a_sketch_and_the_body_a_feature_made() {
        let mut h = part();
        let (base, extrude) = (id_of(&h, "Base"), id_of(&h, "Extrude1"));
        assert_eq!(eye_of(&h.app, base), Some(Eye::Sketch(base)));
        let Some(Eye::Bodies(bodies)) = eye_of(&h.app, extrude) else { panic!("an extrude has an eye for its body") };
        assert_eq!(bodies, ["Main"]);
        assert!(shown(&h.app, &Eye::Bodies(bodies.clone())));
        toggle(&mut h.app, extrude);
        assert!(h.app.session.visibility.hidden_bodies.contains(&"Main".to_string()));
        toggle(&mut h.app, extrude);
        assert!(h.app.session.visibility.hidden_bodies.is_empty());
        // A sketch an extrude uses is already out of the way; its eye brings it back and hides it again.
        assert!(!crate::browser::sketch_visible(&h.app, base));
        toggle(&mut h.app, base);
        assert!(crate::browser::sketch_visible(&h.app, base));
        toggle(&mut h.app, base);
        assert!(!crate::browser::sketch_visible(&h.app, base));
    }

    #[test]
    fn hiding_one_feature_of_a_body_hides_every_feature_that_built_it() {
        let mut h = part();
        let chamfer =
            h.app.session.doc.features.iter().find(|f| f.kind.type_name().contains("Chamfer")).map(|f| f.id).expect("the part has a chamfer");
        let extrude = id_of(&h, "Extrude1");
        toggle(&mut h.app, chamfer);
        for id in [chamfer, extrude] {
            let eye = eye_of(&h.app, id).expect("an eye");
            assert!(!shown(&h.app, &eye), "feature {id} follows its body");
        }
    }

    #[test]
    fn a_chamfer_depends_on_the_extrude_that_made_its_body() {
        let h = part();
        let chamfer = h.app.session.doc.features.iter().find(|f| f.kind.type_name().contains("Chamfer")).map(|f| f.id).expect("a chamfer");
        let extrude = id_of(&h, "Extrude1");
        assert!(related(&h.app, chamfer).parents.contains(&extrude));
        assert!(related(&h.app, extrude).children.contains(&chamfer));
    }

    #[test]
    fn clicking_a_rows_eye_hides_it_without_selecting_the_row() {
        let mut h = part();
        h.frames(4);
        let extrude = id_of(&h, "Extrude1");
        if crate::scenario::handle_at("eye:Extrude1").is_none() {
            // The tree is in the Part Navigator, which the Advanced shell shows.
            h.frames(8);
        }
        h.step(&json!({"click":{"handle":"eye:Extrude1"}})).unwrap();
        h.frames(2);
        assert!(h.app.session.visibility.hidden_bodies.contains(&"Main".to_string()), "the click hid the body");
        assert!(!h.app.session.selection.iter().any(|s| matches!(s, solidcraft_engine::Sel::Feature { id } if *id == extrude)));
        let _ = extrude;
    }

    #[test]
    fn selecting_a_feature_never_evaluates_the_design() {
        // The sample plate has fillets and cuts: evaluating without a feature takes tens to
        // hundreds of milliseconds, so a click that did it would hang. Parents and children come
        // from a graph worked out once per revision.
        let mut h = Harness::new();
        h.app.preferences.modelling_approach = ModellingApproach::Advanced;
        h.app.session.run_script(&solidcraft_engine::sample::script()).unwrap();
        h.frames(6);
        let ids: Vec<u64> = h.app.session.doc.features.iter().map(|f| f.id).collect();
        let revision = h.app.session.revision;
        let started = std::time::Instant::now();
        for id in &ids {
            let _ = crate::shell::part_nav::facts(&h.app, *id);
            let _ = related(&h.app, *id);
        }
        assert!(started.elapsed() < std::time::Duration::from_millis(50), "{:?} for {} features", started.elapsed(), ids.len());
        assert_eq!(h.app.session.revision, revision, "nothing was evaluated or changed");
        let base = id_of(&h, "Base");
        let r = related(&h.app, base);
        assert!(!r.children.is_empty() && r.parents.is_empty(), "{r:?}");
    }
}
