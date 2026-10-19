//! What the Part Navigator's feature tree knows about each row beyond its name: which features
//! a feature depends on and which depend on it (shown by the colour of their names), and the
//! eye that hides what the feature made. Pure functions of the session, so they are tested alone.

use std::collections::BTreeSet;

use crate::SolveApp;

/// The features a selected feature depends on and the features that depend on it.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Related {
    pub parents: BTreeSet<u64>,
    pub children: BTreeSet<u64>,
}

/// The bodies feature `id` made or changed, by name: new in its result, or changed from the
/// result before it.
fn touched(app: &SolveApp, id: u64) -> Vec<String> {
    let results = &app.session.model.results;
    let Some(at) = results.iter().position(|r| r.id == id) else { return Vec::new() };
    let before = at.checked_sub(1).and_then(|i| results.get(i));
    let Some(after) = results.get(at) else { return Vec::new() };
    after
        .state
        .bodies
        .iter()
        .filter(|b| before.is_none_or(|p| p.state.body(&b.name).is_none_or(|old| old.content_key() != b.content_key())))
        .map(|b| b.name.clone())
        .collect()
}

/// The feature before `index` that last made or changed the body `name`.
fn last_to_touch(app: &SolveApp, index: usize, name: &str) -> Option<u64> {
    app.session.doc.features.iter().take(index).rev().map(|f| f.id).find(|id| touched(app, *id).iter().any(|b| b == name))
}

/// The features feature `id` is built on: its sketches, the features and bodies it names, and
/// the earlier features that made or changed the bodies it works on.
fn parents_of(app: &SolveApp, index: usize) -> BTreeSet<u64> {
    let doc = &app.session.doc;
    let mut out = BTreeSet::new();
    let Some(feature) = doc.features.get(index) else { return out };
    let (sketches, features, bodies) = feature.kind.references();
    out.extend(sketches.into_iter().filter(|s| doc.feature(*s).is_some()));
    out.extend(features.iter().filter_map(|n| doc.find_feature(n)).map(|f| f.id));
    let worked_on = touched(app, feature.id);
    for name in bodies.iter().chain(worked_on.iter()) {
        out.extend(last_to_touch(app, index, name));
    }
    out.remove(&feature.id);
    out
}

/// Parents and children of feature `id`, one step each way (a child of a parent is not a
/// child), so the tree reads as the recipe it is.
pub fn related(app: &SolveApp, id: u64) -> Related {
    let doc = &app.session.doc;
    let mut r = Related::default();
    let Some(index) = doc.features.iter().position(|f| f.id == id) else { return r };
    r.parents = parents_of(app, index);
    for (i, f) in doc.features.iter().enumerate() {
        if f.id != id && parents_of(app, i).contains(&id) {
            r.children.insert(f.id);
        }
    }
    r
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
    let bodies: Vec<String> = touched(app, id).into_iter().filter(|b| world.body(b).is_some()).collect();
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
}
