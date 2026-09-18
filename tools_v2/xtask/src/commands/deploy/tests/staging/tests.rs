use super::*;

fn v(items: &[&str]) -> Vec<String> {
    items.iter().map(|s| (*s).to_string()).collect()
}

#[test]
fn usage_matches_the_captured_baseline() {
    // /tmp/t853/ds--help.old, three lines, rc=0.
    let lines: Vec<&str> = USAGE.lines().collect();
    assert_eq!(lines.len(), 3);
    assert_eq!(
        lines[0],
        "Usage: deploy-staging.sh [--dry-run] [--render-only <path>]"
    );
    assert!(lines[1].starts_with("                         [--render-agent <dir>]"));
    assert!(lines[2].ends_with("[--verify-boot-selftest]"));
}

#[test]
fn missing_value_stops_with_two() {
    for flag in [
        "--render-only",
        "--render-agent",
        "--agent-selftest",
        "--verify-boot",
    ] {
        assert_eq!(parse(&v(&[flag])), Parsed::Stop(2), "{flag}");
    }
}

#[test]
fn unknown_option_short_circuits_before_help() {
    // Bash `case` runs left to right and exits 2 on the first unknown word, so the later
    // --help is never reached. Pinned because "helpful" reordering would change the status.
    assert_eq!(parse(&v(&["--nope", "--help"])), Parsed::Stop(2));
    assert_eq!(parse(&v(&["--help", "--nope"])), Parsed::Help);
}

#[test]
fn oddity_flag_is_eaten_as_a_value() {
    // PRESERVED: `shift` then `${1:-}` has no lookahead, so --dry-run becomes the path.
    match parse(&v(&["--render-only", "--dry-run"])) {
        Parsed::Run(cli) => {
            assert_eq!(cli.render_only_out.as_deref(), Some("--dry-run"));
            assert!(
                !cli.dry_run,
                "--dry-run was consumed as a value, not a flag"
            );
        }
        other => panic!("{other:?}"),
    }
}

#[test]
fn empty_string_value_is_rejected_like_bash() {
    // `${1:-}` yields "" for an explicit empty arg too, and `[ -z ]` then fires.
    assert_eq!(parse(&v(&["--render-only", ""])), Parsed::Stop(2));
}

#[test]
fn flags_accumulate() {
    match parse(&v(&["--dry-run", "--verify-boot-selftest"])) {
        Parsed::Run(cli) => {
            assert!(cli.dry_run && cli.verify_boot_selftest);
        }
        other => panic!("{other:?}"),
    }
}

#[test]
fn paths_inline_the_three_fields_paths_sh_supplied() {
    let p = Paths::resolve().expect("repo root");
    assert!(
        p.mono_root.join(".ai/tickets/ROOT").is_file()
            || p.mono_root.join(".ai/tickets/registry.json").is_file()
    );
    assert!(p.schema.ends_with("packages/tbd-schema"));
    assert!(p.deploy_env.ends_with("scripts/deploy/deploy.env"));
}
