use super::*;
use std::path::PathBuf;

/// A scratch repo root that cleans itself up: a handful of tests do not justify a
/// dev-dependency. `good()` lays down a tree that
/// satisfies the contract, so every test below perturbs exactly one thing.
struct Tree(PathBuf);

impl Drop for Tree {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

impl Tree {
    fn new(name: &str) -> Tree {
        let p = std::env::temp_dir().join(format!(
            "staging-compose-paths-{}-{name}",
            std::process::id()
        ));
        let _ = std::fs::remove_dir_all(&p);
        std::fs::create_dir_all(&p).unwrap();
        Tree(p)
    }

    fn good(name: &str) -> Tree {
        let t = Tree::new(name);
        t.write(DEPLOY_SOURCE, GOOD_SOURCE);
        t.write(GOOD_PATH, "services: {}\n");
        t
    }

    fn write(&self, rel: &str, body: &str) {
        let path = self.0.join(rel);
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(&path, body).unwrap();
    }

    /// Exactly what [`verify_staging_compose_paths`] prints above its summary line.
    fn text(&self) -> String {
        audit(&self.0)
            .unwrap()
            .iter()
            .filter(|v| !matches!(v, Verdict::Held))
            .map(|v| format!("{v}\n"))
            .collect()
    }
}

/// A compose block shaped like the audited source, comment included — the comment is the point:
/// it names the good path, so a gate grepping the raw file would pass on the comment alone.
const GOOD_SOURCE: &str = r#"echo "==> docker compose (API + Postgres)"
# compose file lives at apps/website/docker-compose.staging.yml,
# not under apps/website/api_v2/. Match `cargo xtask deploy website`.
if [ "$DRY_RUN" -eq 1 ]; then
  echo "[dry-run] cd \$TBD_REMOTE_DIR && docker compose -f apps/website/docker-compose.staging.yml up -d --build"
else
  ssh_cmd "cd '$TBD_REMOTE_DIR' && docker compose -f apps/website/docker-compose.staging.yml up -d --build"
fi
"#;

/// Rewrite the audited source in a good tree, then require every `want` substring in the report
/// AND exit 1.
///
/// Anti-vacuity: one green line is no evidence, so what makes this gate worth having is that each
/// arm still bites on the perturbation it names.
fn bites(name: &str, script: &str, want: &[&str]) {
    let t = Tree::good(name);
    t.write(DEPLOY_SOURCE, script);
    let text = t.text();
    for w in want {
        assert!(text.contains(w), "[{name}] missing {w:?} in:\n{text}");
    }
    assert_eq!(
        verify_staging_compose_paths(&t.0).unwrap(),
        1,
        "[{name}] must exit 1"
    );
}

/// The live tree must satisfy the gate. Moving the deploy driver turns this red first, which is
/// the intended alarm: the pin needs repointing, not deleting.
#[test]
fn the_live_deploy_source_holds() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("tools_v2/xtask has a parent")
        .parent()
        .unwrap();
    assert_eq!(verify_staging_compose_paths(root).unwrap(), 0);
}

#[test]
fn a_correct_source_holds() {
    assert_eq!(Tree::good("ok").text(), "");
    assert_eq!(
        verify_staging_compose_paths(&Tree::good("ok2").0).unwrap(),
        0
    );
}

/// Six source-level perturbations, each the gate must catch.
#[test]
fn every_source_perturbation_bites() {
    // The compose path goes relative — the perturbation the gate's title names.
    bites(
        "relative",
        &GOOD_SOURCE.replace(GOOD_PATH, "docker-compose.staging.yml"),
        &[
            "FAIL: dry-run -f path must be apps/website/docker-compose.staging.yml \
                 (got: docker-compose.staging.yml)",
            &format!(
                "FAIL: live {LIVE_KEY} -f path must be apps/website/docker-compose.staging.yml \
                 (got: docker-compose.staging.yml)"
            ),
        ],
    );
    // Live regresses to api/ while the dry-run stays clean. Three separate messages must fire —
    // wrong path, divergence, stale reference.
    let live = format!("ssh_cmd \"cd '$TBD_REMOTE_DIR' && docker compose -f {GOOD_PATH}");
    bites(
        "live-api",
        &GOOD_SOURCE.replace(&live, &live.replace(GOOD_PATH, BAD_PATH)),
        &[
            &format!(
                "FAIL: live {LIVE_KEY} -f path must be apps/website/docker-compose.staging.yml \
                 (got: apps/website/api_v2/docker-compose.staging.yml)"
            ),
            "FAIL: dry-run and live compose -f paths diverge:",
            "FAIL: live compose line still references \
                 apps/website/api_v2/docker-compose.staging.yml",
        ],
    );
    // The false-green this gate exists for: the good path present ONLY in `#` and `//` comments.
    bites(
        "comment-only",
        &format!("# docker compose -f {GOOD_PATH}\n// docker compose -f {GOOD_PATH}\n"),
        &[
            "FAIL: no dry-run docker compose -f line after comment strip",
            &format!("FAIL: no live {LIVE_KEY} docker compose -f line after comment strip"),
        ],
    );
    // The banned `cd`, each quoting on its own.
    bites(
        "cd-sq",
        &format!("{GOOD_SOURCE}{CD_INTO_API_SQ}\n"),
        &[&format!(
            "FAIL: {} still cds into apps/website/api_v2 (compose must not)",
            source_basename()
        )],
    );
    bites(
        "cd-dq",
        &format!("{GOOD_SOURCE}{CD_INTO_API_DQ}\n"),
        &[&format!(
            "FAIL: {} still cds into apps/website/api_v2 (double-quoted form)",
            source_basename()
        )],
    );
    // `-f` with no argument: its own message, echoing the line at the report's six-space indent.
    bites(
        "no-arg",
        "  echo \"[dry-run] docker compose -f\"\n  ssh_cmd \"docker compose -f\"\n",
        &[
            "FAIL: dry-run compose line has no parseable -f path:\n      \
                 echo \"[dry-run] docker compose -f\"",
            "FAIL: live compose line has no parseable -f path:\n      \
                 ssh_cmd \"docker compose -f\"",
        ],
    );
}

/// The on-disk pair: the compose file moved away (`-f`), and a leftover restored at the stale
/// path (`-e`). Neither uses [`bites`] — one needs a tree that is deliberately not good.
#[test]
fn the_on_disk_compose_pair_bites() {
    let gone = Tree::new("no-compose");
    gone.write(DEPLOY_SOURCE, GOOD_SOURCE);
    assert_eq!(gone.text(), format!("FAIL: missing {GOOD_PATH}\n"));
    assert_eq!(verify_staging_compose_paths(&gone.0).unwrap(), 1);

    let stale = Tree::good("stale");
    stale.write(BAD_PATH, "services: {}\n");
    assert_eq!(
        stale.text(),
        format!("FAIL: unexpected {BAD_PATH} (stale path)\n")
    );
}

/// A missing audited source is a check that did not run — never a pass — and prints WITHOUT the
/// trailing summary line, because there is nothing to summarise.
#[test]
fn a_missing_deploy_source_does_not_read_as_pass() {
    assert_eq!(
        verify_staging_compose_paths(&Tree::new("no-source").0).unwrap(),
        1
    );
    assert_eq!(
        verify_staging_compose_paths(Path::new("/nonexistent/staging-compose-paths")).unwrap(),
        1
    );
}

#[test]
fn quoted_dash_f_arguments_lose_their_quotes() {
    let re = f_regex().unwrap();
    for (line, want) in [
        ("docker compose -f 'a b.yml' up", Some("a b.yml")),
        ("docker compose -f \"a b.yml\" up", Some("a b.yml")),
        ("docker compose -f a.yml up", Some("a.yml")),
        ("docker compose -f", None),
        // ODDITY PIN, not an endorsement (see [`f_path`]): the first `-f` on the line wins,
        // even when it belongs to some other command entirely.
        ("rm -f /tmp/x && docker compose -f good.yml", Some("/tmp/x")),
    ] {
        assert_eq!(f_path(&re, line), want, "{line}");
    }
}

#[test]
fn comments_go_and_quoted_hashes_stay() {
    assert_eq!(strip_comments("a # b\nc\n"), "a \nc\n");
    assert_eq!(strip_comments("e '# no'\n"), "e '# no'\n");
    assert_eq!(strip_comments("e \"# no\"\n"), "e \"# no\"\n");
    // The C-style arm — deliberate, and the unquoted-URL hazard it implies.
    assert_eq!(strip_comments("x // y\nz\n"), "x \nz\n");
    assert_eq!(strip_comments("curl https://h/p\n"), "curl https:\n");
}

/// ODDITY PIN, not an endorsement. See `strip_comments`: POSIX says a backslash inside
/// single quotes is literal, so the quote closes and `# gone` should be stripped. This machine
/// believes the quote is still open and strips nothing after it. A future fix turns this red.
#[test]
fn a_backslash_before_a_closing_single_quote_swallows_the_rest() {
    assert_eq!(strip_comments("a='x\\' # gone\n"), "a='x\\' # gone\n");
}

#[test]
fn a_transport_facade_cannot_substitute_for_the_compose_implementation() {
    let tree = Tree::new("facade-only");
    tree.write(
        "tools_v2/xtask/src/commands/deploy/staging/remote.rs",
        GOOD_SOURCE,
    );
    tree.write(GOOD_PATH, "services: {}\n");
    assert_eq!(verify_staging_compose_paths(&tree.0).unwrap(), 1);
    assert!(tree.text().contains(DEPLOY_SOURCE));
}
