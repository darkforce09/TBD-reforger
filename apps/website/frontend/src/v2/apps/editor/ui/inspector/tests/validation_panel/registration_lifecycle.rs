//! Validation panel registration lifecycle tests.

use super::{
    publish_compile_findings, read_payload_source, register_panel_sink, register_payload_source,
    register_route_probe, register_select_by_id, route_select_by_subject_id, subject_id_routes,
    PanelFinding, PayloadSource,
};
use leptos::prelude::*;
use std::cell::RefCell;
use std::rc::Rc;
use website_map_engine::data::scenario::validate::Primitive;
use website_map_engine::data::scenario::validate::Severity;

thread_local! {
    static ANSWERED: RefCell<Vec<String>> = const { RefCell::new(Vec::new()) };

    static SINKS: RefCell<Vec<(&'static str, RwSignal<Vec<PanelFinding>>)>> =
        const { RefCell::new(Vec::new()) };
}

fn note(tag: &'static str) {
    ANSWERED.with(|l| l.borrow_mut().push(tag.to_string()));
}
fn answered() -> Vec<String> {
    ANSWERED.with(|l| l.borrow().clone())
}
fn forget_answers() {
    ANSWERED.with(|l| l.borrow_mut().clear());
}

fn row() -> PanelFinding {
    PanelFinding {
        rule_id: "F5-PROBE".into(),
        severity: Severity::Warning,
        primitive: Primitive::PerObjectInvariant,
        message: "f5".into(),
        subject: "f5".into(),
        subject_id: None,
    }
}

fn sink_for(tag: &'static str) -> RwSignal<Vec<PanelFinding>> {
    SINKS
        .with(|s| {
            s.borrow()
                .iter()
                .find(|(t, _)| *t == tag)
                .map(|(_, sig)| *sig)
        })
        .expect("prepare_sinks creates one signal per tag before anything is installed")
}

fn prepare_sinks(owner: &Owner) {
    let sinks = owner.with(|| {
        ["A", "B"]
            .map(|tag| (tag, RwSignal::new(Vec::<PanelFinding>::new())))
            .to_vec()
    });
    SINKS.with(|s| *s.borrow_mut() = sinks);
}

fn ask_sink() -> bool {
    SINKS.with(|s| {
        for (_, sig) in s.borrow().iter() {
            sig.set(Vec::new());
        }
    });
    publish_compile_findings(vec![row()]);
    let mut reached = false;
    SINKS.with(|s| {
        for (tag, sig) in s.borrow().iter() {
            if !sig.get_untracked().is_empty() {
                note(tag);
                reached = true;
            }
        }
    });
    reached
}

struct Seam {
    name: &'static str,
    install: fn(&'static str),
    ask: fn() -> bool,
}

fn seams() -> [Seam; 4] {
    [
        Seam {
            name: "PAYLOAD_SOURCE",
            install: |tag| {
                register_payload_source(Rc::new(move || {
                    note(tag);
                    Some(PayloadSource {
                        payload: serde_json::json!({}),
                        known_asset_ids: None,
                    })
                }));
            },
            ask: || read_payload_source().is_some(),
        },
        Seam {
            name: "SELECT_BY_ID",
            install: |tag| {
                register_select_by_id(Rc::new(move |_id: &str| {
                    note(tag);
                    true
                }));
            },
            ask: || route_select_by_subject_id("f5-subject"),
        },
        Seam {
            name: "ROUTE_PROBE",
            install: |tag| {
                register_route_probe(Rc::new(move |_id: &str| {
                    note(tag);
                    true
                }));
            },
            ask: || subject_id_routes("f5-subject"),
        },
        Seam {
            name: "PANEL_SINK",
            install: |tag| register_panel_sink(sink_for(tag)),
            ask: ask_sink,
        },
    ]
}

#[test]
fn a_seam_with_nothing_installed_reports_failure() {
    let root = Owner::new();
    prepare_sinks(&root);
    for seam in seams() {
        assert!(
            !(seam.ask)(),
            "F5 {}: nothing has ever been installed, so the seam must report failure",
            seam.name
        );
    }
    assert!(
        answered().is_empty(),
        "F5: no registration exists, so none can have answered — got {:?}",
        answered()
    );
}

#[test]
fn unmount_unregisters_every_seam_so_none_reports_success() {
    let root = Owner::new();
    prepare_sinks(&root);
    for seam in seams() {
        let mounted = root.child();
        mounted.with(|| (seam.install)("A"));

        forget_answers();
        assert!(
            (seam.ask)(),
            "F5 {} precondition: while mounted the seam really does answer",
            seam.name
        );
        assert_eq!(
            answered(),
            vec!["A".to_string()],
            "F5 {} precondition: the LIVE registration is the one that answered",
            seam.name
        );

        mounted.cleanup();

        forget_answers();
        assert!(
            !(seam.ask)(),
            "F5 {}: the installing owner is gone, so the seam must report FAILURE rather than \
                 success over a disposed no-op",
            seam.name
        );
        assert!(
            answered().is_empty(),
            "F5 {}: the stale registration must not be called at all after unmount — got {:?}",
            seam.name,
            answered()
        );
    }
}

#[test]
fn an_older_owners_cleanup_does_not_clobber_a_newer_registration() {
    let root = Owner::new();
    prepare_sinks(&root);
    for seam in seams() {
        let old = root.child();
        let new = root.child();
        old.with(|| (seam.install)("A"));
        new.with(|| (seam.install)("B"));

        old.cleanup();

        forget_answers();
        assert!(
            (seam.ask)(),
            "F5 {}: the NEW mount is live — the superseded owner's cleanup must not unregister \
                 it",
            seam.name
        );
        assert_eq!(
            answered(),
            vec!["B".to_string()],
            "F5 {}: the surviving registration must be the NEWER one, not a leftover that \
                 merely happens to answer",
            seam.name
        );

        new.cleanup();

        forget_answers();
        assert!(
            !(seam.ask)(),
            "F5 {}: the live mount's OWN cleanup does clear it — the guard skips losers, not \
                 everyone",
            seam.name
        );
    }
}

#[test]
fn the_seam_mechanism_is_defined_exactly_once_in_the_crate() {
    let needles = [
        ["fn ", "install", "_seam"].concat(),
        ["fn ", "unregister", "_seam"].concat(),
    ];

    fn walk(dir: &std::path::Path, out: &mut Vec<std::path::PathBuf>) {
        let entries = std::fs::read_dir(dir)
            .unwrap_or_else(|e| panic!("T-783: cannot read {}: {e}", dir.display()));
        for ent in entries {
            let path = ent.expect("read_dir entry").path();
            if path.is_dir() {
                walk(&path, out);
            } else if path.extension().and_then(|e| e.to_str()) == Some("rs") {
                out.push(path);
            }
        }
    }
    let src_root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
    let mut files = Vec::new();
    walk(&src_root, &mut files);
    files.sort();
    assert!(
        files.len() > 40,
        "T-783: the crate walk found only {} .rs files — the pin's input is wrong, so its \
             green would mean nothing",
        files.len()
    );

    let sources: Vec<(String, String, String)> = files
        .iter()
        .map(|path| {
            let raw = std::fs::read_to_string(path)
                .unwrap_or_else(|e| panic!("T-783: cannot read {}: {e}", path.display()));
            let name = path
                .strip_prefix(&src_root)
                .unwrap_or(path)
                .display()
                .to_string();
            let live = crate::v2::core::test_support::class_r_scrub::live_code(&raw);
            (name, raw, live)
        })
        .collect();

    for needle in &needles {
        let mut live_hits: Vec<String> = Vec::new();
        let mut raw_hits: Vec<String> = Vec::new();
        for (name, raw, live) in &sources {
            let n_raw = raw.matches(needle.as_str()).count();
            if n_raw > 0 {
                raw_hits.push(format!("{name} x{n_raw}"));
            }
            let n_live = live.matches(needle.as_str()).count();
            if n_live > 0 {
                live_hits.push(format!("{name} x{n_live}"));
            }
        }
        assert_eq!(
                live_hits,
                vec!["v2/apps/editor/ui/inspector/validation_panel/seam_registration.rs x1".to_string()],
                "T-783: `{needle}` must be defined exactly ONCE in live crate code, beside the \
                 SeamRegistration trait it depends on. Found: {live_hits:?}. Import it \
                 (`crate::v2::apps::editor::ui::inspector::validation_panel::install_seam`, or the `ruler_tool` re-export the \
                 wasm-only seams already use) instead of writing a second copy — one identity check \
                 with two mechanisms is how the remount guard drifts out of one of them."
            );
        assert_eq!(
            raw_hits,
            vec![
                "v2/apps/editor/ui/inspector/validation_panel/seam_registration.rs x1".to_string()
            ],
            "T-783: `{needle}` appears outside live code as well. Found: {raw_hits:?}. The raw \
                 count catches a copy the scrubber cannot see — it cuts from the first test-module \
                 attribute to end of file, so a definition parked below one would hide."
        );
    }
}
