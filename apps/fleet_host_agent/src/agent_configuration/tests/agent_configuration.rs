use std::fs::{self, Permissions};
use std::os::unix::fs::PermissionsExt;

use tempfile::TempDir;

use super::*;

const CREDENTIAL: &str = "tbdm_0123456789abcdef0123456789abcdef_\
     0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef";
const PASSWORD: &str = "range-master";
const SERVER_CONFIG: &str =
    "{\"game\": {\"scenarioId\": \"{ECC61978EDCC2B5A}Missions/23_Campaign.conf\"}}\n";

/// A directory holding both secret files with owner-only permissions and the dedicated
/// server's config.
struct SecretDirectory {
    directory: TempDir,
}

impl SecretDirectory {
    fn new() -> Self {
        let secrets = Self {
            directory: tempfile::tempdir().unwrap(),
        };
        secrets.write("machine-credential", &format!("{CREDENTIAL}\n"), 0o600);
        secrets.write("rcon-password", &format!("{PASSWORD}\n"), 0o400);
        secrets.write("server.json", SERVER_CONFIG, 0o644);
        secrets
    }

    fn path(&self, name: &str) -> PathBuf {
        self.directory.path().join(name)
    }

    /// Replaces the file: an earlier version may be read-only.
    fn write(&self, name: &str, contents: &str, mode: u32) {
        let path = self.path(name);
        if path.exists() {
            fs::remove_file(&path).unwrap();
        }
        fs::write(&path, contents).unwrap();
        fs::set_permissions(path, Permissions::from_mode(mode)).unwrap();
    }

    /// A complete configuration; `extra_rcon` lines are appended to the `[rcon]` table.
    fn configuration(&self, api_base_url: &str, extra_rcon: &str) -> String {
        format!(
            "api_base_url = \"{api_base_url}\"\n\
             credential_file = \"{}\"\n\
             [game_server]\n\
             systemd_user_unit = \"tbd-reforger.service\"\n\
             server_config_path = \"{}\"\n\
             [rcon]\n\
             address = \"127.0.0.1\"\n\
             password_file = \"{}\"\n\
             {extra_rcon}\n",
            self.path("machine-credential").display(),
            self.path("server.json").display(),
            self.path("rcon-password").display(),
        )
    }
}

fn parse(text: &str) -> Result<AgentConfiguration, ConfigurationError> {
    AgentConfiguration::parse(text, Path::new("/etc/fleet-host-agent/agent.toml"))
}

#[test]
fn a_minimal_configuration_takes_the_documented_defaults() {
    let secrets = SecretDirectory::new();
    let configuration = parse(&secrets.configuration("https://tbd.example.org", "")).unwrap();
    assert_eq!(
        configuration.api_base_url.as_str(),
        "https://tbd.example.org/"
    );
    assert_eq!(configuration.machine_credential.expose(), CREDENTIAL);
    assert_eq!(configuration.poll_interval, Duration::from_secs(5));
    let process_control = &configuration.process_control;
    assert_eq!(process_control.unit.as_str(), "tbd-reforger.service");
    assert_eq!(process_control.start_dwell, Duration::from_secs(8));
    assert_eq!(
        process_control.systemctl_program,
        PathBuf::from("/usr/bin/systemctl")
    );
    assert_eq!(
        configuration.rcon.server,
        "127.0.0.1:19999".parse().unwrap()
    );
    assert_eq!(configuration.rcon.password.expose(), PASSWORD);
    assert_eq!(
        configuration.server_config.path(),
        secrets.path("server.json")
    );
}

#[test]
fn every_key_can_be_set() {
    let secrets = SecretDirectory::new();
    let text = format!(
        "api_base_url = \"https://tbd.example.org/platform\"\n\
         credential_file = \"{}\"\n\
         poll_interval_seconds = 12\n\
         [game_server]\n\
         systemd_user_unit = \"reforger@eu-1.service\"\n\
         server_config_path = \"{}\"\n\
         start_dwell_seconds = 15\n\
         systemctl_program = \"/usr/local/bin/systemctl\"\n\
         [rcon]\n\
         address = \"::1\"\n\
         port = 20999\n\
         password_file = \"{}\"\n",
        secrets.path("machine-credential").display(),
        secrets.path("server.json").display(),
        secrets.path("rcon-password").display(),
    );
    let configuration = parse(&text).unwrap();
    assert_eq!(
        configuration.api_base_url.as_str(),
        "https://tbd.example.org/platform/"
    );
    assert_eq!(configuration.poll_interval, Duration::from_secs(12));
    assert_eq!(
        configuration.process_control.start_dwell,
        Duration::from_secs(15)
    );
    assert_eq!(configuration.rcon.server, "[::1]:20999".parse().unwrap());
    assert_eq!(
        configuration.server_config.path(),
        secrets.path("server.json")
    );
}

#[test]
fn plain_http_is_accepted_only_for_loopback() {
    let secrets = SecretDirectory::new();
    for url in [
        "http://127.0.0.1:8080",
        "http://localhost:8080/",
        "http://[::1]:8080",
    ] {
        assert!(parse(&secrets.configuration(url, "")).is_ok(), "{url}");
    }
    for (url, problem) in [
        ("http://tbd.example.org", "must use https"),
        ("http://10.0.0.5:8080", "must use https"),
        ("ftp://tbd.example.org", "must use https"),
        ("tbd.example.org", "is not an absolute URL"),
        (
            "https://user:pass@tbd.example.org",
            "must not carry credentials",
        ),
        ("https://tbd.example.org/?token=1", "must not carry a query"),
    ] {
        let error = parse(&secrets.configuration(url, "")).unwrap_err();
        assert!(
            matches!(&error, ConfigurationError::ApiBaseUrlInvalid { problem: found, .. } if found.contains(problem)),
            "{url}: {error}"
        );
    }
}

#[test]
fn out_of_range_and_malformed_values_are_named() {
    let secrets = SecretDirectory::new();
    let base = secrets.configuration("https://tbd.example.org", "");
    let with = |from: &str, to: &str| parse(&base.replacen(from, to, 1)).unwrap_err();
    assert!(matches!(
        with(
            "[game_server]\n",
            "poll_interval_seconds = 0\n[game_server]\n"
        ),
        ConfigurationError::PollIntervalOutOfRange(0)
    ));
    assert!(matches!(
        with(
            "[game_server]\n",
            "[game_server]\nstart_dwell_seconds = 31\n"
        ),
        ConfigurationError::StartDwellOutOfRange(31)
    ));
    assert!(matches!(
        with("tbd-reforger.service", "tbd-reforger.timer"),
        ConfigurationError::UnitNameInvalid { .. }
    ));
    assert!(matches!(
        with(
            "[game_server]\n",
            "[game_server]\nsystemctl_program = \"systemctl\"\n"
        ),
        ConfigurationError::SystemctlProgramNotAbsolute(_)
    ));
    assert!(matches!(
        with("\"127.0.0.1\"", "\"game-host.local\""),
        ConfigurationError::RconAddressInvalid(_)
    ));
    assert!(matches!(
        with("[rcon]\n", "[rcon]\nport = 0\n"),
        ConfigurationError::RconPortInvalid
    ));
    assert!(matches!(
        with("[rcon]\n", "[rcon]\nbroadcast_command = \"#say\"\n"),
        ConfigurationError::FileMalformed { .. }
    ));
    assert!(matches!(
        with("[rcon]\n", "[rcon]\npasword_file = \"/typo\"\n"),
        ConfigurationError::FileMalformed { .. }
    ));
}

#[test]
fn the_server_config_must_be_an_existing_file_the_agent_can_write() {
    let secrets = SecretDirectory::new();
    let base = secrets.configuration("https://tbd.example.org", "");
    let server_config = secrets.path("server.json").display().to_string();
    let naming = |replacement: &str| parse(&base.replacen(&server_config, replacement, 1));
    let rejected = |result: Result<AgentConfiguration, ConfigurationError>| match result {
        Err(ConfigurationError::ServerConfigFileRejected { problem, .. }) => problem,
        other => panic!("a rejected server config, not {other:?}"),
    };
    assert!(matches!(
        rejected(naming("server.json")),
        ServerConfigFileProblem::RelativePath
    ));
    assert!(matches!(
        rejected(naming(&secrets.path("absent.json").display().to_string())),
        ServerConfigFileProblem::Unreadable(_)
    ));
    assert!(matches!(
        rejected(naming(&secrets.directory.path().display().to_string())),
        ServerConfigFileProblem::NotRegularFile
    ));
    secrets.write("server.json", SERVER_CONFIG, 0o444);
    // A privileged user writes a read-only file anyway; the refusal applies to everyone else.
    if OpenOptions::new()
        .write(true)
        .open(secrets.path("server.json"))
        .is_err()
    {
        assert!(matches!(
            rejected(parse(&base)),
            ServerConfigFileProblem::NotWritable(_)
        ));
    }
    let without_key = base.replacen(
        &format!("server_config_path = \"{server_config}\"\n"),
        "",
        1,
    );
    assert!(
        matches!(
            parse(&without_key),
            Err(ConfigurationError::FileMalformed { .. })
        ),
        "the key is required"
    );
}

#[test]
fn secret_files_readable_by_other_users_are_refused() {
    let secrets = SecretDirectory::new();
    secrets.write("machine-credential", CREDENTIAL, 0o644);
    let error = parse(&secrets.configuration("https://tbd.example.org", "")).unwrap_err();
    assert!(
        matches!(
            &error,
            ConfigurationError::SecretFileRejected {
                key: "credential_file",
                problem: SecretFileProblem::AccessibleToOthers { mode: 0o644 },
                ..
            }
        ),
        "{error}"
    );
    secrets.write("machine-credential", CREDENTIAL, 0o600);
    secrets.write("rcon-password", PASSWORD, 0o640);
    let error = parse(&secrets.configuration("https://tbd.example.org", "")).unwrap_err();
    assert!(
        matches!(
            &error,
            ConfigurationError::SecretFileRejected {
                key: "rcon.password_file",
                problem: SecretFileProblem::AccessibleToOthers { .. },
                ..
            }
        ),
        "{error}"
    );
}

#[test]
fn malformed_secrets_are_refused_without_being_quoted() {
    let secrets = SecretDirectory::new();
    let leaked = "tbdm_not-a-real-credential";
    secrets.write("machine-credential", leaked, 0o600);
    let error = parse(&secrets.configuration("https://tbd.example.org", "")).unwrap_err();
    assert!(
        matches!(
            &error,
            ConfigurationError::SecretFileRejected {
                problem: SecretFileProblem::InvalidContent(_),
                ..
            }
        ),
        "{error}"
    );
    assert!(!error.to_string().contains(leaked), "{error}");

    secrets.write("machine-credential", CREDENTIAL, 0o600);
    secrets.write("rcon-password", "two words", 0o600);
    let error = parse(&secrets.configuration("https://tbd.example.org", "")).unwrap_err();
    assert!(!error.to_string().contains("two words"), "{error}");
}

#[test]
fn relative_and_missing_secret_files_are_refused() {
    let secrets = SecretDirectory::new();
    let relative = secrets
        .configuration("https://tbd.example.org", "")
        .replacen(
            &secrets.path("machine-credential").display().to_string(),
            "machine-credential",
            1,
        );
    assert!(matches!(
        parse(&relative).unwrap_err(),
        ConfigurationError::SecretFileRejected {
            problem: SecretFileProblem::RelativePath,
            ..
        }
    ));
    fs::remove_file(secrets.path("rcon-password")).unwrap();
    assert!(matches!(
        parse(&secrets.configuration("https://tbd.example.org", "")).unwrap_err(),
        ConfigurationError::SecretFileRejected {
            key: "rcon.password_file",
            problem: SecretFileProblem::Unreadable(_),
            ..
        }
    ));
}

#[test]
fn credential_and_password_formats_follow_the_contracts() {
    assert!(machine_credential_format(CREDENTIAL).is_ok());
    let uppercase = CREDENTIAL.to_uppercase();
    let wrong_prefix = CREDENTIAL.replacen("tbdm_", "tbdx_", 1);
    let one_digit_more = format!("{CREDENTIAL}0");
    let with_scheme = format!("Bearer {CREDENTIAL}");
    for invalid in [
        "",
        "tbdm_",
        uppercase.as_str(),
        wrong_prefix.as_str(),
        &CREDENTIAL[..CREDENTIAL.len() - 1],
        one_digit_more.as_str(),
        with_scheme.as_str(),
    ] {
        assert!(machine_credential_format(invalid).is_err(), "{invalid:?}");
    }
    assert!(rcon_password_format("abc").is_ok());
    assert!(rcon_password_format("pässwört").is_ok());
    let too_long = "x".repeat(257);
    for invalid in ["", "ab", "two words", "tab\there", too_long.as_str()] {
        assert!(rcon_password_format(invalid).is_err(), "{invalid:?}");
    }
}

#[test]
fn the_loaded_configuration_never_prints_a_secret() {
    let secrets = SecretDirectory::new();
    let configuration = parse(&secrets.configuration("https://tbd.example.org", "")).unwrap();
    let printed = format!("{configuration:?}");
    assert!(!printed.contains(CREDENTIAL));
    assert!(!printed.contains(PASSWORD));
}

#[test]
fn load_reads_the_file_it_names() {
    let secrets = SecretDirectory::new();
    let path = secrets.path("agent.toml");
    fs::write(&path, secrets.configuration("https://tbd.example.org", "")).unwrap();
    assert!(AgentConfiguration::load(&path).is_ok());
    assert!(matches!(
        AgentConfiguration::load(&secrets.path("absent.toml")).unwrap_err(),
        ConfigurationError::FileUnreadable { .. }
    ));
}
