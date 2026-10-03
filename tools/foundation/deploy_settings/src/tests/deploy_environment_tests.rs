use super::*;

const PATH: &str = "/home/deploy/checkout/deploy/deploy.env";

fn pairs(list: &[(&str, &str)]) -> Vec<(String, String)> {
    list.iter()
        .map(|(key, value)| (key.to_string(), value.to_string()))
        .collect()
}

fn settings(file: Option<&str>, process: &[(&str, &str)]) -> DeployEnvironment {
    DeployEnvironment::from_text(Path::new(PATH), file, pairs(process)).expect("parses")
}

#[test]
fn an_empty_assignment_in_the_file_beats_the_process_environment() {
    let environment = settings(
        Some("TBD_SSH_PASS=\n"),
        &[("TBD_SSH_PASS", "exported-secret")],
    );
    assert_eq!(environment.value("TBD_SSH_PASS"), None);
    assert!(environment.required("TBD_SSH_PASS").is_err());
}

#[test]
fn a_file_value_beats_a_stale_exported_one() {
    let environment = settings(
        Some("TBD_SSH_HOST=deploy@192.0.2.10\n"),
        &[("TBD_SSH_HOST", "deploy@198.51.100.7")],
    );
    assert_eq!(environment.value("TBD_SSH_HOST"), Some("deploy@192.0.2.10"));
}

#[test]
fn the_process_environment_fills_only_keys_the_file_never_assigns() {
    let environment = settings(
        Some("TBD_SSH_HOST=deploy@192.0.2.10\n"),
        &[("TBD_MODPACK_JSON", "/tmp/pack.json"), ("TBD_EMPTY", "")],
    );
    assert_eq!(
        environment.value("TBD_MODPACK_JSON"),
        Some("/tmp/pack.json")
    );
    assert_eq!(
        environment.value("TBD_EMPTY"),
        None,
        "empty counts as unset"
    );
    assert_eq!(environment.value_or("TBD_GAME_PORT", "2001"), "2001");
    let without_file = settings(None, &[("TBD_SSH_HOST", "deploy@192.0.2.10")]);
    assert_eq!(
        without_file.value("TBD_SSH_HOST"),
        Some("deploy@192.0.2.10")
    );
}

#[test]
fn the_override_path_is_made_absolute_and_an_empty_one_is_ignored() {
    let root = Path::new("/home/deploy/checkout");
    let working = Path::new("/home/deploy/elsewhere");
    let default = root.join(repository_layout::DEPLOY_ENV);
    assert_eq!(
        resolve_deploy_environment_path(root, None, working),
        default
    );
    assert_eq!(
        resolve_deploy_environment_path(root, Some(OsStr::new("")), working),
        default
    );
    assert_eq!(
        resolve_deploy_environment_path(root, Some(OsStr::new("other.env")), working),
        working.join("other.env")
    );
    assert_eq!(
        resolve_deploy_environment_path(root, Some(OsStr::new("/srv/deploy.env")), working),
        PathBuf::from("/srv/deploy.env")
    );
}

#[test]
fn the_load_errors_name_the_file_and_the_example() {
    let missing = DeployEnvironment::load_required(Path::new("/nonexistent/deploy.env"))
        .expect_err("no file");
    assert_eq!(
        missing.to_string(),
        "Missing /nonexistent/deploy.env — copy from deploy/deploy.env.example"
    );
    let absent = DeployEnvironment::load_if_present(Path::new("/nonexistent/deploy.env"))
        .expect("an absent file is allowed here");
    assert_eq!(absent.path(), Path::new("/nonexistent/deploy.env"));
    let syntax =
        DeployEnvironment::from_text(Path::new(PATH), Some("A=1\nnot an assignment\n"), [])
            .expect_err("line 2 has no `=`");
    assert_eq!(
        syntax.to_string(),
        format!("{PATH}:2: expected KEY=VALUE, and this line has no `=`")
    );
}

#[test]
fn a_setting_error_points_at_the_line_or_names_the_file() {
    let environment = settings(
        Some("# header\nTBD_SSH_HOST=two@at@signs\n"),
        &[("TBD_PUBLIC_ADDRESS", "not-an-address")],
    );
    assert_eq!(
        environment.deploy_host().unwrap_err().to_string(),
        format!("{PATH}:2: TBD_SSH_HOST: holds a second `@`; write user@host or host")
    );
    assert_eq!(
        environment
            .invalid("TBD_PUBLIC_ADDRESS", "not IPv4")
            .to_string(),
        "TBD_PUBLIC_ADDRESS (process environment): not IPv4"
    );
    assert_eq!(
        environment
            .required("TBD_REMOTE_DIR")
            .unwrap_err()
            .to_string(),
        format!("TBD_REMOTE_DIR is not set: add it to {PATH}")
    );
    assert_eq!(
        environment.invalid("TBD_UNSET", "anything"),
        environment.missing("TBD_UNSET")
    );
    assert_eq!(
        environment
            .not_derivable(
                "TBD_PUBLIC_ADDRESS",
                "h has no IPv4 address from here (x)",
                "fix it"
            )
            .to_string(),
        format!(
            "TBD_PUBLIC_ADDRESS is unset and h has no IPv4 address from here (x): fix it or set TBD_PUBLIC_ADDRESS in {PATH}"
        )
    );
}

#[test]
fn the_deploy_host_is_required() {
    let environment = settings(
        Some("TBD_SSH_HOST=\n"),
        &[("TBD_SSH_HOST", "deploy@192.0.2.10")],
    );
    assert_eq!(
        environment.deploy_host().unwrap_err(),
        environment.missing(DEPLOY_HOST_KEY)
    );
    let named = settings(
        Some("export TBD_SSH_HOST = 'deploy@192.0.2.10' # host\n"),
        &[],
    );
    assert_eq!(
        named.deploy_host().expect("parses").ssh_destination(),
        "deploy@192.0.2.10"
    );
}

/// The committed template loads under the grammar, names its host in the one key, and assigns no
/// optional key empty, which would mask the process environment.
#[test]
fn the_committed_example_loads_and_masks_nothing() {
    const EXAMPLE: &str = include_str!("../../../../../deploy/deploy.env.example");
    let example = settings(Some(EXAMPLE), &[("TBD_SSH_PASS", "exported")]);
    let host = example.deploy_host().expect("the example names a host");
    assert!(host.user().is_some(), "the example names the deploy user");
    assert_eq!(example.value("TBD_SSH_PASS"), Some("exported"));
    assert_eq!(example.value("TBD_PUBLIC_ADDRESS"), None);
    for key in [
        "TBD_REMOTE_DIR",
        "TBD_PROFILE_DIR",
        "TBD_ADDONS_STAGING",
        "TBD_SERVER_DIR",
    ] {
        assert_eq!(
            example.value(key),
            None,
            "{key} defaults under the user's home"
        );
    }
}

#[test]
fn secure_shell_transport_reads_the_password_before_the_identity_file() {
    assert_eq!(
        settings(Some("TBD_SSH_PASS=pw\nTBD_SSH_IDENTITY_FILE=/k/id\n"), &[]).ssh_base(),
        SshBase::Pass("pw".into())
    );
    assert_eq!(
        settings(Some("TBD_SSH_IDENTITY_FILE=/k/id\n"), &[]).ssh_base(),
        SshBase::Identity("/k/id".into())
    );
    assert_eq!(
        settings(Some("TBD_SSH_PASS=\n"), &[]).ssh_base(),
        SshBase::Plain
    );
}
