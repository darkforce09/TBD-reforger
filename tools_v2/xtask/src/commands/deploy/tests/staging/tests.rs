use super::*;

fn v(items: &[&str]) -> Vec<String> {
    items.iter().map(|s| (*s).to_string()).collect()
}

#[test]
fn usage_names_the_runnable_command_and_every_mode_flag() {
    let lines: Vec<&str> = USAGE.lines().collect();
    assert_eq!(lines.len(), 3);
    assert_eq!(
        lines[0],
        "Usage: cargo xtask deploy staging [--dry-run] [--render-only <path>]"
    );
    assert!(lines[1].trim_start().starts_with("[--render-agent <dir>]"));
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
fn paths_resolve_against_the_running_checkout() {
    let p = Paths::resolve().expect("repo root");
    assert!(ticket_engine::repository::is_repo_root(&p.mono_root));
    assert!(p.schema.ends_with("contracts_v2"));
    assert!(
        p.deploy_env
            .ends_with(crate::core::repository_layout::DEPLOY_ENV)
    );
}
