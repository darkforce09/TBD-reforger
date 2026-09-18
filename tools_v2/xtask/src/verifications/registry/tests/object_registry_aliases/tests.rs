use super::*;
use std::path::PathBuf;

/// `tools_v2/xtask/` -> repo root. The gate's whole job is the committed data, so the real tree is the
/// fixture; no synthetic input reaches the alias checks past the `eligible == 333` equality.
fn repo() -> PathBuf {
    let here = Path::new(env!("CARGO_MANIFEST_DIR"));
    here.parent()
        .expect("tools_v2")
        .parent()
        .expect("repository root")
        .to_path_buf()
}

/// A scratch repo root: real WB/MOD/FE, with the mod registry optionally rewritten.
struct Fixture(PathBuf);
impl Fixture {
    fn new(name: &str, mod_json: Option<String>) -> Fixture {
        let root = std::env::temp_dir().join(format!("tbd-t439-{}-{name}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        let src_root = repo();
        let copies = [
            (
                registry_items_catalog_path(&src_root),
                registry_items_catalog_path(&root),
            ),
            (src_root.join(MOD_REL), root.join(MOD_REL)),
            (src_root.join(FE_REL), root.join(FE_REL)),
        ];
        for (src, dst) in copies {
            std::fs::create_dir_all(dst.parent().unwrap()).unwrap();
            std::fs::copy(src, &dst).unwrap();
        }
        if let Some(body) = mod_json {
            std::fs::write(root.join(MOD_REL), body).unwrap();
        }
        Fixture(root)
    }
    fn drop_file(&self, rel: &str) {
        std::fs::remove_file(self.0.join(rel)).unwrap();
    }
}
impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

/// One perturbation of the shipped registry's `entries` array.
type Edit = fn(&mut Vec<Value>);
/// Mutate the shipped registry's entries with `f`, then run the gate over the result.
fn perturb(name: &str, f: Edit) -> u8 {
    let raw = std::fs::read_to_string(repo().join(MOD_REL)).unwrap();
    let mut doc: Value = serde_json::from_str(&raw).unwrap();
    f(doc["entries"].as_array_mut().unwrap());
    let fx = Fixture::new(name, Some(doc.to_string()));
    verify_t439(&fx.0).unwrap()
}

fn row(es: &[Value], pred: impl Fn(&str) -> bool) -> usize {
    let hit = es
        .iter()
        .position(|e| e["alias"].as_str().is_some_and(&pred));
    hit.expect("the shipped registry has the row this perturbation edits")
}

#[test]
fn the_real_registry_holds() {
    assert_eq!(verify_t439(&repo()).unwrap(), 0);
}

/// T-556 anti-vacuity: a gate that cannot fail is indistinguishable from one that checks
/// nothing, and this one prints a single PASS line on a clean tree. Each case aims at a
/// different one of the script's ordered checks and must turn that PASS into a 1.
#[test]
fn perturbing_the_registry_turns_the_pass_red() {
    let cases: [(&str, Edit); 4] = [
        // guid_mismatch — the row exists and points at the wrong prefab.
        ("guid", |es| {
            let i = row(es, |a| a.starts_with("prop:"));
            es[i]["guid"] = Value::String("{0000000000000000}Prefabs/Wrong.et".into());
        }),
        // missing — RENAMED not deleted, so prop: stays at its floor and this reaches the
        // alias lookup instead of tripping the count check above it.
        ("rename", |es| {
            let i = row(es, |a| a.starts_with("prop:"));
            let to = format!("{}_zz", es[i]["alias"].as_str().unwrap());
            es[i]["alias"] = Value::String(to);
        }),
        // the prop floor — one row fewer than the 2026-07-27 measurement.
        ("delete", |es| {
            let i = row(es, |a| a.starts_with("prop:"));
            es.remove(i);
        }),
        // the POC check — retargeted, so comp: still meets its floor.
        ("poc", |es| {
            let i = row(es, |a| a == POC_ALIAS);
            es[i]["alias"] = Value::String("comp:not_the_poc".into());
        }),
    ];
    for (name, f) in cases {
        assert_eq!(perturb(name, f), 1, "perturbation `{name}` went green");
    }
    // The frontend mirror pins are violations (1) too, not did-not-runs.
    let gutted = Fixture::new("gutfe", None);
    std::fs::write(gutted.0.join(FE_REL), "// nothing to see here\n").unwrap();
    assert_eq!(
        verify_t439(&gutted.0).unwrap(),
        1,
        "a gutted mirror went green"
    );
}

/// THE DEFECT THE CRATE EXISTS FOR. Inputs nobody read are not clean inputs — and these are 2,
/// not the 1 a real drift returns, so CI can tell a broken checkout from a broken registry.
#[test]
fn inputs_that_were_never_examined_do_not_read_as_pass() {
    let absent = Fixture::new("absent", None);
    absent.drop_file(MOD_REL);
    assert_eq!(verify_t439(&absent.0).unwrap(), 2, "absent registry");
    let no_fe = Fixture::new("nofe", None);
    no_fe.drop_file(FE_REL);
    assert_eq!(verify_t439(&no_fe.0).unwrap(), 2, "absent frontend mirror");
    let garbage = Fixture::new("garbage", Some("{ this is not json".into()));
    assert_eq!(verify_t439(&garbage.0).unwrap(), 2, "unparseable registry");
    // The Python `mod["entries"]` KeyError path, now a verdict rather than a stack trace.
    let bare = Fixture::new("noentries", Some(r#"{"registryVersion": 1}"#.into()));
    assert_eq!(verify_t439(&bare.0).unwrap(), 2, "no entries array");
}

#[test]
fn the_mirror_matches_the_frontend() {
    assert_eq!(object_alias_slug("Ammo Box US 01"), "ammo_box_us_01");
    assert_eq!(object_alias_slug("  -- Weird -- "), "weird");
    assert_eq!(object_alias_slug("!!!"), "object", "empty falls back");
    let known = derive_object_alias(KNOWN_CHECKPOINT_GUID, "E Sandbag Barricade US 04");
    assert_eq!(known, POC_ALIAS, "the KNOWN reverse-hit beats the slug");
    let path = "{A}PrefabsEditable/Compositions/X.et";
    assert_eq!(derive_object_alias(path, "Fuel Depot"), "comp:fuel_depot");
    let p2 = "{A}Prefabs/P.et";
    assert_eq!(derive_object_alias(p2, "Fuel Depot"), "prop:fuel_depot");
}

#[test]
fn shape_pins_reject_what_python_rejected() {
    let guid = compile(GUID_RE).unwrap();
    let alias = compile(ALIAS_RE).unwrap();
    assert!(shape_ok(&guid, "{0123456789ABCDEF}Prefabs/A-b_c.et"));
    assert!(!shape_ok(&guid, "{0123456789abcdef}P.et"), "lowercase hex");
    // The `^`/`$` trap: line anchors would let the second line smuggle anything through.
    assert!(!shape_ok(&guid, "{0123456789ABCDEF}Prefabs/A.et\nGARBAGE"));
    assert!(shape_ok(&alias, "prop:ammo_box_us_01"));
    assert!(!shape_ok(&alias, "prop:Ammo"), "uppercase");
    assert!(!shape_ok(&alias, "thing:x"), "namespace not in $defs");
}

/// The Python behaviours the sample lines and the eligible filter depend on.
#[test]
fn python_semantics_are_reproduced() {
    assert_eq!(py_repr_str("prop:x"), "'prop:x'");
    assert_eq!(py_repr_str("it's"), "\"it's\"");
    assert_eq!(py_repr_str("both ' and \""), "'both \\' and \"'");
    assert_eq!(py_repr_str("a\\b\nc"), "'a\\\\b\\nc'");
    assert_eq!(py_repr_value(None), "None", "an absent guid renders None");
    assert_eq!(
        py_sample(&["a", "b", "c"], 2, |s| py_repr_str(s)),
        "['a', 'b']"
    );
    let tup = py_tuple(&[py_repr_str("alias"), py_repr_value(None), py_repr_str("x")]);
    assert_eq!(tup, "('alias', None, 'x')");
    assert!(is_falsy(None), "an absent `abstract` is falsy");
    assert!(is_falsy(Some(&Value::Bool(false))));
    assert!(is_falsy(Some(&serde_json::json!(0))));
    assert!(!is_falsy(Some(&Value::Bool(true))));
}
