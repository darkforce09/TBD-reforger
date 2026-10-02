//! `status_map_at_rev` over a scratch repository that carries both registry shapes: one
//! revision holding the single JSON file, one holding a ticket file per ticket, and one
//! holding neither.

use std::fs;

use crate::registry::ticket_file_storage::ticket_to_toml_string;
use crate::repository::TICKETS_DIR;

#[test]
fn dual_read_json_then_toml() {
    use std::process::Command;
    let tmp = crate::repository::find_repo_root()
        .expect("repository root")
        .join("target")
        .join("ticket-status-history-revisions");
    let _ = fs::remove_dir_all(&tmp);
    fs::create_dir_all(tmp.join(TICKETS_DIR)).unwrap();
    let git = |args: &[&str]| {
        Command::new("git")
            .args(args)
            .current_dir(&tmp)
            .env("GIT_AUTHOR_NAME", "tbd")
            .env("GIT_AUTHOR_EMAIL", "tbd@test")
            .env("GIT_COMMITTER_NAME", "tbd")
            .env("GIT_COMMITTER_EMAIL", "tbd@test")
            .status()
            .unwrap()
    };
    git(&["init", "-q"]);
    git(&["checkout", "-q", "-b", "main"]);
    let json =
        r#"{"tickets":[{"id":"T-AAA","status":"shipped"},{"id":"T-BBB","status":"queued"}]}"#;
    fs::write(tmp.join(super::historical_registry_json()), json).unwrap();
    git(&["add", "."]);
    git(&["commit", "-q", "-m", "A json"]);
    let sha_a = String::from_utf8(
        Command::new("git")
            .args(["rev-parse", "HEAD"])
            .current_dir(&tmp)
            .output()
            .unwrap()
            .stdout,
    )
    .unwrap()
    .trim()
    .to_string();

    fs::remove_file(tmp.join(super::historical_registry_json())).unwrap();
    fs::write(tmp.join(crate::repository::ROOT_MARKER), "#\n").unwrap();
    let aaa = serde_json::json!({"id":"T-AAA","status":"shipped"});
    let bbb = serde_json::json!({"id":"T-BBB","status":"shipped"});
    fs::write(
        tmp.join(TICKETS_DIR).join("T-AAA.toml"),
        ticket_to_toml_string(&aaa, 0).unwrap(),
    )
    .unwrap();
    fs::write(
        tmp.join(TICKETS_DIR).join("T-BBB.toml"),
        ticket_to_toml_string(&bbb, 1).unwrap(),
    )
    .unwrap();
    git(&["add", "-A"]);
    git(&["commit", "-q", "-m", "B toml"]);
    let sha_b = String::from_utf8(
        Command::new("git")
            .args(["rev-parse", "HEAD"])
            .current_dir(&tmp)
            .output()
            .unwrap()
            .stdout,
    )
    .unwrap()
    .trim()
    .to_string();

    let map_at = |rev: &str| -> Option<std::collections::HashMap<String, String>> {
        super::status_map_at_rev(&tmp, rev)
    };

    let a = map_at(&sha_a).expect("json rev");
    assert_eq!(a.get("T-AAA").map(String::as_str), Some("shipped"));
    assert_eq!(a.get("T-BBB").map(String::as_str), Some("queued"));
    let ids = ["T-AAA", "T-BBB"];
    let open_a: Vec<_> = ids
        .iter()
        .copied()
        .filter(|t| {
            !matches!(
                a.get(*t).map(String::as_str),
                Some("shipped") | Some("cancelled")
            )
        })
        .collect();
    assert_eq!(open_a, ["T-BBB"]);

    let b = map_at(&sha_b).expect("toml rev");
    let open_b: Vec<_> = ids
        .iter()
        .copied()
        .filter(|t| {
            !matches!(
                b.get(*t).map(String::as_str),
                Some("shipped") | Some("cancelled")
            )
        })
        .collect();
    assert!(open_b.is_empty(), "both shipped at B");

    fs::remove_dir_all(tmp.join(TICKETS_DIR)).unwrap();
    git(&["add", "-A"]);
    git(&["commit", "-q", "-m", "C empty"]);
    let sha_c = String::from_utf8(
        Command::new("git")
            .args(["rev-parse", "HEAD"])
            .current_dir(&tmp)
            .output()
            .unwrap()
            .stdout,
    )
    .unwrap()
    .trim()
    .to_string();
    assert!(
        map_at(&sha_c).is_none(),
        "neither form must refuse, not empty map"
    );
}
