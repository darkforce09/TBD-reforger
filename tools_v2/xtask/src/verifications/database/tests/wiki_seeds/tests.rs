use super::*;
use std::path::PathBuf;

/// A scratch repo tree that cleans itself up. Same shape as `verification_core::scan`'s, and for the
/// same reason: a handful of tests do not justify a dev-dependency.
struct Tree(PathBuf);

impl Tree {
    fn new(name: &str) -> Tree {
        let mut p = std::env::temp_dir();
        p.push(format!("xtask-t444-{}-{name}", std::process::id()));
        let _ = std::fs::remove_dir_all(&p);
        std::fs::create_dir_all(&p).unwrap();
        Tree(p)
    }

    fn write(&self, rel: &str, body: &str) -> &Tree {
        let path = self.0.join(rel);
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(&path, body).unwrap();
        self
    }

    /// A tree that satisfies the contract, so each test perturbs exactly one thing.
    fn good(name: &str) -> Tree {
        let t = Tree::new(name);
        t.write(
            SEED_FILE,
            "INSERT INTO wiki_pages (slug) VALUES ('field-manual');\n",
        );
        t
    }

    /// The live seed list — the tree half of the contract against the real const.
    fn verdict(&self) -> Verdict {
        first_failure(&self.0, SEEDS).unwrap()
    }

    /// The same tree against a PERTURBED seed list.
    fn verdict_with(&self, seeds: &[&str]) -> Verdict {
        first_failure(&self.0, seeds).unwrap()
    }
}

impl Drop for Tree {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

/// EVERY fixture below is DERIVED from [`SEEDS`], never transcribed. A hand-written copy of
/// the const is how `gate_t440`'s fixtures drifted and cost seven test failures earlier in
/// this program: the copy stayed green while the thing it claimed to mirror had moved.
fn seeds_without(entry: &str) -> Vec<&'static str> {
    SEEDS.iter().copied().filter(|s| *s != entry).collect()
}

fn text(v: &Verdict) -> String {
    v.to_string()
}

#[test]
fn the_live_seed_list_holds() {
    assert!(matches!(Tree::good("ok").verdict(), Verdict::Held));
}

/// THE WAVE 24 DEFECT, in its post-Makefile shape. Dropping the wiki seed from the list the
/// seeder walks greened the cold gate; it must not.
#[test]
fn a_seed_list_missing_the_wiki_entry_is_caught() {
    let t = Tree::good("no-wiki-entry");
    let v = t.verdict_with(&seeds_without(SEED_ENTRY));
    assert!(matches!(v, Verdict::Failed(_)), "{}", text(&v));
    assert!(
        text(&v).starts_with("FAIL: cargo xtask db seed does not apply wiki_pages.sql"),
        "{}",
        text(&v)
    );
}

/// Membership is by EQUALITY. `wiki_pages.sql.disabled` is a plausible way to park the seed
/// without applying it, and a substring test would have accepted it.
#[test]
fn a_renamed_wiki_entry_does_not_satisfy_the_pin() {
    let t = Tree::good("renamed-entry");
    let mut seeds = seeds_without(SEED_ENTRY);
    seeds.push("wiki_pages.sql.disabled");
    assert!(matches!(t.verdict_with(&seeds), Verdict::Failed(_)));
}

/// A gutted list is the successor to "the Makefile is gone": it is reported before the file
/// checks, because the operator action is different.
#[test]
fn an_empty_seed_list_does_not_read_as_pass() {
    let t = Tree::good("empty-list");
    let v = t.verdict_with(&[]);
    assert!(matches!(v, Verdict::Failed(_)));
    assert!(text(&v).contains("is empty"), "{}", text(&v));
    assert!(text(&v).contains(RECIPE_CONST), "{}", text(&v));
}

#[test]
fn a_missing_seed_file_does_not_read_as_pass() {
    let t = Tree::new("no-seed");
    let v = t.verdict();
    assert!(matches!(v, Verdict::DidNotRun(NotRun::TargetMissing(_), _)));
    assert!(text(&v).contains("T-444 requires apps/website/api/seeds/wiki_pages.sql"));
    assert_eq!(verify_t444(&t.0).unwrap(), 1);
}

/// A whole repo root that does not exist at all: still not a pass.
#[test]
fn a_nonexistent_repo_root_does_not_read_as_pass() {
    assert_eq!(verify_t444(Path::new("/nonexistent/tbd-t444")).unwrap(), 1);
}

#[test]
fn an_empty_seed_file_is_caught_before_the_slug_pin() {
    let t = Tree::good("empty-seed");
    t.write(SEED_FILE, "");
    let v = t.verdict();
    assert!(matches!(v, Verdict::Failed(_)));
    assert!(text(&v).contains("is empty"), "{}", text(&v));
}

#[test]
fn a_seed_without_the_v_suite_slug_is_caught() {
    let t = Tree::good("no-slug");
    t.write(SEED_FILE, "INSERT INTO wiki_pages (slug) VALUES ('sop');\n");
    let v = t.verdict();
    assert!(matches!(v, Verdict::Failed(_)));
    assert!(text(&v).contains("does not contain 'field-manual'"));
}

/// The stdout contract. `wave.sh` prints `tail -15` of a failed step, so the failure body is
/// operator-facing evidence and pinned here. Re-baselined at T-897 when the subject moved off
/// the Makefile recipe onto `crate::commands::db::operations::SEEDS`.
#[test]
fn failure_text_is_pinned() {
    let t = Tree::good("bytes");
    assert_eq!(
        text(&t.verdict_with(&seeds_without(SEED_ENTRY))),
        "FAIL: cargo xtask db seed does not apply wiki_pages.sql\n      \
         Add to tools_v2/xtask/src/commands/db/operations.rs SEEDS:\n        \
         \"wiki_pages.sql\",\n      \
         Without this entry, cargo xtask db seed never loads doctrine wiki pages."
    );
}

/// The live tree must satisfy the gate — const AND seed file together.
#[test]
fn the_live_repo_contract_holds() {
    let repo_root = Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("tools_v2/xtask has a parent")
        .parent()
        .unwrap();
    let v = first_failure(repo_root, SEEDS).unwrap();
    assert!(matches!(v, Verdict::Held), "{}", text(&v));
}
