use super::*;

/// A throwaway checkout holding `apps/mod/References/` (or not), removed on drop.
struct Scratch(PathBuf);

impl Scratch {
    fn new(name: &str, with_references: bool) -> Scratch {
        let root = std::env::temp_dir().join(format!("enf-ref-out-{}-{name}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(&root).unwrap();
        if with_references {
            std::fs::create_dir_all(root.join(crate::repository_layout::REFERENCES_DIR)).unwrap();
        }
        Scratch(root)
    }
    fn references(&self) -> PathBuf {
        self.0.join(crate::repository_layout::REFERENCES_DIR)
    }
}

impl Drop for Scratch {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

#[test]
fn a_lane_inside_the_references_folder_is_accepted() {
    let s = Scratch::new("inside", true);
    let lane = Path::new(crate::repository_layout::VANILLA_EXTRACTED_SCRIPTS);
    let got = reference_output_within(&s.references(), &s.0, lane).unwrap();
    assert_eq!(
        got,
        s.0.join(crate::repository_layout::VANILLA_EXTRACTED_SCRIPTS)
    );
    assert!(!got.exists(), "the guard checks; it creates nothing");
}

#[test]
fn an_output_outside_the_references_folder_is_refused() {
    let s = Scratch::new("outside", true);
    let refs = s.references();
    for out in [
        "apps/mod/vanilla_reference/Scripts",
        "apps/mod/References",
        "apps/mod/References/../vanilla_reference",
        "/tmp/elsewhere",
    ] {
        let err = reference_output_within(&refs, &s.0, Path::new(out));
        assert!(err.is_err(), "{out} must be refused");
    }
    assert!(!s.0.join("apps/mod/vanilla_reference").exists());
}

#[test]
fn a_checkout_without_the_references_folder_is_refused() {
    let s = Scratch::new("absent", false);
    let lane = Path::new(crate::repository_layout::VANILLA_REFERENCE);
    let err = reference_output_within(&s.references(), &s.0, lane).unwrap_err();
    assert!(format!("{err:#}").contains("is missing"), "{err:#}");
    assert!(!s.references().exists(), "the refusal creates nothing");
}

#[test]
fn a_previous_output_is_removed_only_on_request() {
    let s = Scratch::new("replace", true);
    let dir = s.references().join("vanilla_reference/Scripts");
    clear_previous_output(&dir, false).unwrap(); // absent: nothing to do
    std::fs::create_dir_all(&dir).unwrap();
    clear_previous_output(&dir, false).unwrap(); // empty: nothing to do
    std::fs::write(dir.join("Game.c"), "class A {}\n").unwrap();

    let err = clear_previous_output(&dir, false).unwrap_err();
    assert!(format!("{err:#}").contains("--replace"), "{err:#}");
    assert!(dir.join("Game.c").is_file(), "a refusal deletes nothing");

    clear_previous_output(&dir, true).unwrap();
    assert!(!dir.exists());
}
