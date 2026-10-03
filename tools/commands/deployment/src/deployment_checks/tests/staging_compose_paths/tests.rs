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
        t.write(WEBSITE_DEPLOY_SOURCE, GOOD_WEBSITE_SOURCE);
        t.write(STAGING_DEPLOY_PIPELINE, GOOD_STAGING_SOURCE);
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

/// A compose helper shaped like the website deploy's, comments included — the comments are the
/// point: they name the good path, so a gate grepping the raw file would pass on them alone.
const GOOD_WEBSITE_SOURCE: &str = r##"/// Every step runs docker compose -f deploy/compose.staging.yml.
fn compose_session(remote_dir: &str) -> String {
    // podman compose -f deploy/compose.staging.yml when the host has no docker
    format!(
        "cd '{remote_dir}' && staging_compose() {{ if command -v docker >/dev/null 2>&1; then \
         docker compose -f deploy/compose.staging.yml \"$@\"; else \
         podman compose -f deploy/compose.staging.yml \"$@\"; fi; }}"
    )
}
"##;

/// A game server pipeline shaped like the real one: it mentions compose only in a comment, and
/// only in the comment may it.
const GOOD_STAGING_SOURCE: &str = r##"// The website deploy runs docker compose -f deploy/compose.staging.yml.
println!("==> website API ({health_url} on the host)");
"##;

/// Rewrite one audited source in a good tree, then require every `want` substring in the report
/// AND exit 1.
///
/// Anti-vacuity: one green line is no evidence, so what makes this gate worth having is that each
/// arm still bites on the perturbation it names.
fn bites(name: &str, source: &str, body: &str, want: &[&str]) {
    let t = Tree::good(name);
    t.write(source, body);
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

/// The live tree must satisfy the gate. Moving the deploy code turns this red first, which is
/// the intended alarm: the pin needs repointing, not deleting.
#[test]
fn the_live_deploy_sources_hold() {
    let root = repository_layout::find_repository_root_from(Path::new(env!("CARGO_MANIFEST_DIR")))
        .expect("repository root");
    assert_eq!(
        audit(&root)
            .unwrap()
            .iter()
            .filter(|v| !matches!(v, Verdict::Held))
            .count(),
        0
    );
    assert_eq!(verify_staging_compose_paths(&root).unwrap(), 0);
}

/// The development stack beside the staging file.
const DEVELOPMENT_COMPOSE: &str = "deploy/compose.dev.yml";

#[test]
fn a_correct_tree_holds() {
    assert_eq!(Tree::good("ok").text(), "");
    // The development stack belongs in the compose folder too; only naming it on a staging line
    // is wrong.
    let with_development = Tree::good("ok-development");
    with_development.write(DEVELOPMENT_COMPOSE, "services: {}\n");
    assert_eq!(with_development.text(), "");
    assert_eq!(
        verify_staging_compose_paths(&Tree::good("ok2").0).unwrap(),
        0
    );
}

/// Every perturbation of the website deploy's source that the gate must catch.
#[test]
fn every_website_source_perturbation_bites() {
    // The compose path goes relative — the perturbation the gate's title names.
    bites(
        "relative",
        WEBSITE_DEPLOY_SOURCE,
        &GOOD_WEBSITE_SOURCE.replace(GOOD_PATH, "docker-compose.staging.yml"),
        &["FAIL: compose -f path must be deploy/compose.staging.yml \
             (got: docker-compose.staging.yml)"],
    );
    // One provider's line regresses to the development stack beside the staging file while the
    // other stays clean. Both messages must fire — wrong path and the reference itself.
    let podman = format!("podman compose -f {GOOD_PATH}");
    bites(
        "development-stack",
        WEBSITE_DEPLOY_SOURCE,
        &GOOD_WEBSITE_SOURCE.replace(&podman, &podman.replace(GOOD_PATH, DEVELOPMENT_COMPOSE)),
        &[
            "FAIL: compose -f path must be deploy/compose.staging.yml \
                 (got: deploy/compose.dev.yml)",
            "FAIL: compose line names deploy/compose.dev.yml, which is not \
                 deploy/compose.staging.yml:\n      podman compose -f deploy/compose.dev.yml",
        ],
    );
    // The right first `-f`, then an overlay or an env file from outside the compose folder: the
    // `-f` pin holds, the name ban does not.
    let docker = format!("docker compose -f {GOOD_PATH}");
    for (name, extra, wrong) in [
        (
            "overlay-outside",
            " -f apps/api/docker-compose.staging.yml",
            "apps/api/docker-compose.staging.yml",
        ),
        ("overlay-root", " -f 'compose.yml'", "compose.yml"),
        (
            "env-file-development",
            " --env-file=deploy/compose.dev.yml",
            "deploy/compose.dev.yml",
        ),
    ] {
        bites(
            name,
            WEBSITE_DEPLOY_SOURCE,
            &GOOD_WEBSITE_SOURCE.replace(&docker, &format!("{docker}{extra}")),
            &[&format!(
                "FAIL: compose line names {wrong}, which is not deploy/compose.staging.yml:"
            )],
        );
    }
    // The false-green this gate exists for: the good path present ONLY in `#` and `//` comments.
    bites(
        "comment-only",
        WEBSITE_DEPLOY_SOURCE,
        &format!("# docker compose -f {GOOD_PATH}\n// podman compose -f {GOOD_PATH}\n"),
        &[&format!(
            "FAIL: no compose command in {} after comment strip",
            source_basename(WEBSITE_DEPLOY_SOURCE)
        )],
    );
    // `-f` with no argument, and a compose command with no `-f` at all: their own message,
    // echoing the line at the report's six-space indent.
    bites(
        "no-arg",
        WEBSITE_DEPLOY_SOURCE,
        "  let c = \"docker compose -f\";\n  let d = \"podman-compose up -d caddy\";\n",
        &[
            "FAIL: compose line has no parseable -f path:\n      \
                 let c = \"docker compose -f\";",
            "FAIL: compose line has no parseable -f path:\n      \
                 let d = \"podman-compose up -d caddy\";",
        ],
    );
}

/// The `cd` into the compose folder, under every quoting the Rust source can spell it with.
#[test]
fn a_cd_into_the_compose_folder_bites_under_every_quoting() {
    let message = format!(
        "FAIL: {} cds into deploy (compose runs from the checkout root)",
        source_basename(WEBSITE_DEPLOY_SOURCE)
    );
    for (name, line) in [
        ("cd-sq", "let c = \"cd '{remote_dir}/deploy' && true\";"),
        (
            "cd-dq",
            "let c = \"cd \\\"{remote_dir}/deploy\\\" && true\";",
        ),
        ("cd-bare", "let c = \"cd $TBD_REMOTE_DIR/deploy && true\";"),
        ("cd-relative", "let c = \"cd deploy/systemd && true\";"),
    ] {
        bites(
            name,
            WEBSITE_DEPLOY_SOURCE,
            &format!("{GOOD_WEBSITE_SOURCE}{line}\n"),
            &[&message],
        );
    }
    // Another folder of the checkout, and a path that only passes through a folder of the same
    // name, are not a cd into the compose folder.
    for (name, line) in [
        (
            "cd-elsewhere",
            "let c = \"cd '{remote_dir}/apps/frontend' && true\";",
        ),
        ("cd-home", "let c = \"cd /home/deploy/tbd/repo && true\";"),
    ] {
        let t = Tree::good(name);
        t.write(
            WEBSITE_DEPLOY_SOURCE,
            &format!("{GOOD_WEBSITE_SOURCE}{line}\n"),
        );
        assert_eq!(t.text(), "", "[{name}]");
    }
}

/// The game server deploy may name compose in a comment and nowhere else, under either provider
/// and either spelling.
#[test]
fn a_compose_command_in_the_game_server_deploy_bites() {
    let header = format!(
        "FAIL: {} runs compose; the staging compose stack belongs to cargo xtask deploy website:",
        STAGING_DEPLOY_PIPELINE
    );
    for (name, line) in [
        (
            "staging-docker",
            "&[format!(\"cd '{}' && docker compose -f deploy/compose.staging.yml up -d\", dir)],",
        ),
        ("staging-podman-hyphen", "let c = \"podman-compose up -d\";"),
    ] {
        bites(
            name,
            STAGING_DEPLOY_PIPELINE,
            &format!("{GOOD_STAGING_SOURCE}{line}\n"),
            &[&format!("{header}\n      {line}")],
        );
    }
}

/// Every compose file a line names besides the staging one, as whole words under any quoting;
/// the staging file itself, other YAML and a longer word that merely contains it are not.
#[test]
fn every_other_compose_file_on_a_line_is_named_whole() {
    let line = "docker compose -f deploy/compose.staging.yml -f \\\"deploy/compose.dev.yml\\\" \
                --env-file='../compose.yaml' cat x/docker-compose.yml.bak deploy/caddy.yml \
                deploy/compose.staging.yml";
    assert_eq!(
        wrong_compose_files(line).unwrap(),
        vec!["deploy/compose.dev.yml", "../compose.yaml"]
    );
}

/// The compose file's own name is not a compose command, and neither is the helper the website
/// deploy calls its steps through.
#[test]
fn the_compose_recognizer_takes_commands_and_not_file_names() {
    let text = "docker compose -f a.yml\npodman compose up\ndocker-compose up\npodman-compose\n\
                cat deploy/compose.staging.yml\nstaging_compose up -d caddy\n";
    assert_eq!(
        compose_lines(text).unwrap(),
        vec![
            "docker compose -f a.yml",
            "podman compose up",
            "docker-compose up",
            "podman-compose"
        ]
    );
}

/// The on-disk checks: the compose file moved away, and a compose file — or a folder or a dangling
/// symlink of that name — in the checkout root. Neither uses [`bites`] — one needs a tree that is
/// deliberately not good.
#[test]
fn the_on_disk_compose_pair_bites() {
    let gone = Tree::new("no-compose");
    gone.write(WEBSITE_DEPLOY_SOURCE, GOOD_WEBSITE_SOURCE);
    gone.write(STAGING_DEPLOY_PIPELINE, GOOD_STAGING_SOURCE);
    assert_eq!(gone.text(), format!("FAIL: missing {GOOD_PATH}\n"));
    assert_eq!(verify_staging_compose_paths(&gone.0).unwrap(), 1);

    for (name, root_entry) in [
        ("root-file", "compose.yml"),
        ("root-docker-file", "docker-compose.staging.yaml"),
        ("root-folder", "compose.staging.yml/"),
    ] {
        let second = Tree::good(name);
        if let Some(folder) = root_entry.strip_suffix('/') {
            std::fs::create_dir_all(second.0.join(folder)).unwrap();
        } else {
            second.write(root_entry, "services: {}\n");
        }
        let entry = root_entry.trim_end_matches('/');
        assert_eq!(
            second.text(),
            format!(
                "FAIL: unexpected {entry} in the checkout root: compose runs there, and the \
                 staging compose file is {GOOD_PATH} alone\n"
            ),
            "[{name}]"
        );
        assert_eq!(verify_staging_compose_paths(&second.0).unwrap(), 1);
    }
    let dangling = Tree::good("root-dangling-symlink");
    std::os::unix::fs::symlink("nowhere/compose.yml", dangling.0.join("compose.yml")).unwrap();
    assert!(
        dangling
            .text()
            .contains("FAIL: unexpected compose.yml in the checkout root")
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
    let website_only = Tree::new("website-only");
    website_only.write(WEBSITE_DEPLOY_SOURCE, GOOD_WEBSITE_SOURCE);
    website_only.write(GOOD_PATH, "services: {}\n");
    assert_eq!(verify_staging_compose_paths(&website_only.0).unwrap(), 1);
    assert!(website_only.text().contains(STAGING_DEPLOY_PIPELINE));
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

/// The pinned source is the one that builds the compose commands, not the step runner in front
/// of it: the same helper in the runner's file leaves the gate blind, and it says so.
#[test]
fn the_step_runner_cannot_substitute_for_the_compose_implementation() {
    let tree = Tree::new("runner-only");
    tree.write(
        "tools/commands/deployment/src/website.rs",
        GOOD_WEBSITE_SOURCE,
    );
    tree.write(STAGING_DEPLOY_PIPELINE, GOOD_STAGING_SOURCE);
    tree.write(GOOD_PATH, "services: {}\n");
    assert_eq!(verify_staging_compose_paths(&tree.0).unwrap(), 1);
    assert!(tree.text().contains(WEBSITE_DEPLOY_SOURCE));
}

/// The ban reads every production source of the game server deploy — the fleet pipeline, each
/// payload module and the module file — and no test file. The paths are literal, so a pin left on
/// a retired file cannot pass by agreeing with itself.
#[test]
fn a_compose_command_in_any_game_server_deploy_source_bites() {
    let line = "let c = \"docker compose up -d\";";
    for (index, source) in [
        "tools/commands/deployment/src/staging/remote/fleet_deploy.rs",
        "tools/commands/deployment/src/staging/fleet_units.rs",
        "tools/commands/deployment/src/staging.rs",
    ]
    .into_iter()
    .enumerate()
    {
        bites(
            &format!("any-staging-source-{index}"),
            source,
            &format!("{GOOD_STAGING_SOURCE}{line}\n"),
            &[&format!(
                "FAIL: {source} runs compose; the staging compose stack belongs to cargo xtask \
                 deploy website:\n      {line}"
            )],
        );
    }
    let tests_only = Tree::good("staging-tests-only");
    tests_only.write(
        "tools/commands/deployment/src/staging/tests/fleet_units/tests.rs",
        &format!("{line}\n"),
    );
    assert_eq!(tests_only.text(), "");
}

/// A tree that still holds the retired pin, `remote/ssh_argv.rs`, but no fleet pipeline does not
/// read as a pass: the gate names the pipeline it could not find.
#[test]
fn a_missing_fleet_pipeline_does_not_read_as_pass() {
    let tree = Tree::new("no-fleet-pipeline");
    tree.write(WEBSITE_DEPLOY_SOURCE, GOOD_WEBSITE_SOURCE);
    tree.write(
        "tools/commands/deployment/src/staging/remote/ssh_argv.rs",
        GOOD_STAGING_SOURCE,
    );
    tree.write(GOOD_PATH, "services: {}\n");
    assert_eq!(verify_staging_compose_paths(&tree.0).unwrap(), 1);
    let text = tree.text();
    assert!(
        text.contains("missing ")
            && text.contains("tools/commands/deployment/src/staging/remote/fleet_deploy.rs"),
        "{text}"
    );
}
