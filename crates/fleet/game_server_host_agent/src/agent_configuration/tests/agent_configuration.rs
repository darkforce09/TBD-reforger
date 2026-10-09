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
    AgentConfiguration::parse(text, Path::new("/etc/game_server_host_agent/agent.toml"))
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
fn the_loaded_configuration_never_prints_a_secret() {
    let secrets = SecretDirectory::new();
    let configuration = parse(&secrets.configuration("https://tbd.example.org", "")).unwrap();
    let printed = format!("{configuration:?}");
    assert!(!printed.contains(CREDENTIAL));
    assert!(!printed.contains(PASSWORD));
}
