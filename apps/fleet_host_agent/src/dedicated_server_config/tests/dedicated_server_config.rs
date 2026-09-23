use std::fs::Permissions;
use std::os::unix::fs::PermissionsExt;

use super::*;

const CAMPAIGN: &str = "{ECC61978EDCC2B5A}Missions/23_Campaign.conf";
const DEV_POC: &str = "{69A85365FC09E2CA}Missions/TBD_Dev_POC.conf";

/// A Reforger server config as an operator writes it: tabs, keys in their own order, numbers
/// spelt in several ways, and keys the agent knows nothing about.
const SERVER_CONFIG: &str = "{\n\
\t\"bindAddress\": \"\",\n\
\t\"bindPort\": 2001,\n\
\t\"a2s\": { \"address\": \"0.0.0.0\", \"port\": 17777 },\n\
\t\"rcon\": { \"address\": \"127.0.0.1\", \"port\": 19999, \"password\": \"range-master\" },\n\
\t\"game\": {\n\
\t\t\"name\": \"TBD Staging\",\n\
\t\t\"scenarioId\": \"{ECC61978EDCC2B5A}Missions/23_Campaign.conf\",\n\
\t\t\"maxPlayers\": 64,\n\
\t\t\"gameProperties\": { \"serverMaxViewDistance\": 2500, \"networkViewDistance\": 1500.0 },\n\
\t\t\"futureKey\": { \"unknownToTheAgent\": [1, 2.50, 1e3, -0.0, 18446744073709551616] }\n\
\t},\n\
\t\"operating\": { \"lobbyPlayerSynchronise\": true }\n\
}\n";

#[test]
fn only_the_scenario_id_changes_and_every_other_byte_stays() {
    let rewritten = with_scenario_id(SERVER_CONFIG, DEV_POC).unwrap();
    assert_eq!(rewritten, SERVER_CONFIG.replace(CAMPAIGN, DEV_POC));
}

#[test]
fn switching_rewrites_the_file_and_keeps_its_mode() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("server.json");
    fs::write(&path, SERVER_CONFIG).unwrap();
    fs::set_permissions(&path, Permissions::from_mode(0o640)).unwrap();
    DedicatedServerConfig::new(path.clone())
        .switch_scenario(DEV_POC)
        .unwrap();
    assert_eq!(
        fs::read_to_string(&path).unwrap(),
        SERVER_CONFIG.replace(CAMPAIGN, DEV_POC)
    );
    assert_eq!(
        fs::metadata(&path).unwrap().permissions().mode() & 0o777,
        0o640
    );
}

#[test]
fn malformed_configs_are_refused_and_left_unchanged() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("server.json");
    let config = DedicatedServerConfig::new(path.clone());
    let cases: [(&[u8], &str); 6] = [
        (b"{\"game\": {\"scenarioId\": \"x\"", "is not valid JSON"),
        (b"", "is not valid JSON"),
        (b"\xff\xfe{}", "is not valid JSON: it is not UTF-8 text"),
        (b"{\"game\": {}}", "has no single game.scenarioId string"),
        (
            b"{\"game\": {\"scenarioId\": 7}}",
            "game.scenarioId is not a string",
        ),
        (
            b"{\"game\": {\"scenarioId\": \"a\", \"scenarioId\": \"b\"}}",
            "game names scenarioId twice",
        ),
    ];
    for (contents, expected) in cases {
        fs::write(&path, contents).unwrap();
        let error = config.switch_scenario(DEV_POC).unwrap_err();
        assert!(error.to_string().contains(expected), "{error}");
        assert_eq!(fs::read(&path).unwrap(), contents, "the file is unchanged");
    }
}

#[test]
fn a_missing_or_oversized_config_is_refused() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("server.json");
    let config = DedicatedServerConfig::new(path.clone());
    assert!(matches!(
        config.switch_scenario(DEV_POC),
        Err(ServerConfigError::Unreadable(_))
    ));
    let oversized = format!("{}{SERVER_CONFIG}", " ".repeat(1 << 20));
    fs::write(&path, &oversized).unwrap();
    assert!(matches!(
        config.switch_scenario(DEV_POC),
        Err(ServerConfigError::TooLarge { .. })
    ));
    assert_eq!(fs::read_to_string(&path).unwrap(), oversized);
}
