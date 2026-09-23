//! The platform flows against a scripted transport: every request they send, in order, and how
//! they read each answer.
use std::cell::RefCell;
use std::collections::VecDeque;
use std::time::Duration;

use serde_json::{Value, json};

use super::*;

/// Answers requests from a script and records them as `METHOD url [body]`.
#[derive(Default)]
struct ScriptedTransport {
    answers: RefCell<VecDeque<ApiAnswer>>,
    requests: RefCell<Vec<String>>,
}

impl ScriptedTransport {
    fn answering(answers: Vec<ApiAnswer>) -> Self {
        Self {
            answers: RefCell::new(answers.into()),
            requests: RefCell::default(),
        }
    }
    fn requests(&self) -> Vec<String> {
        self.requests.borrow().clone()
    }
}

impl ApiTransport for ScriptedTransport {
    fn exchange(
        &self,
        method: &str,
        url: &str,
        bearer: Option<&str>,
        body: Option<&Value>,
    ) -> anyhow::Result<ApiAnswer> {
        let path = url.strip_prefix("http://api").unwrap_or(url);
        let mut line = format!("{method} {path}");
        if bearer.is_none() {
            line.push_str(" (anonymous)");
        }
        if let Some(body) = body {
            line.push_str(&format!(" {body}"));
        }
        self.requests.borrow_mut().push(line);
        self.answers
            .borrow_mut()
            .pop_front()
            .ok_or_else(|| anyhow::anyhow!("unscripted request {method} {url}"))
    }
}

fn answer(status: u16, body: Value) -> ApiAnswer {
    ApiAnswer {
        status,
        headers: Vec::new(),
        body: serde_json::to_vec(&body).unwrap(),
    }
}

const PROVENANCE: &str = r#"{"terrain":"everon","modpack_id":"m-1"}"#;

#[test]
fn development_login_reads_the_token_from_the_redirect() {
    let transport = ScriptedTransport::answering(vec![ApiAnswer {
        status: 302,
        headers: vec![(
            "Location".into(),
            "http://ui/auth/callback#access_token=secret-token&expires_at=x".into(),
        )],
        body: Vec::new(),
    }]);
    let token = development_login(&transport, "http://api", "admin").unwrap();
    assert_eq!(token, "secret-token");
    assert_eq!(
        transport.requests(),
        ["GET /api/v1/auth/dev-login?role=admin (anonymous)"]
    );
    let refused = ScriptedTransport::answering(vec![answer(404, json!({"error": "not found"}))]);
    assert!(development_login(&refused, "http://api", "admin").is_err());
}

#[test]
fn approved_artifact_keeps_a_live_missions_decided_artifact() {
    let transport = ScriptedTransport::answering(vec![
        answer(
            200,
            json!({"status": "live", "approved_artifact_id": "a-1"}),
        ),
        answer(200, serde_json::from_str(PROVENANCE).unwrap()),
    ]);
    let client = ApiClient::new(&transport, "http://api", "t");
    let approved = approved_artifact(&client, "m").unwrap();
    assert_eq!(approved.artifact_id, "a-1");
    assert_eq!(approved.terrain, "everon");
    assert_eq!(approved.modpack_id.as_deref(), Some("m-1"));
    assert_eq!(
        transport.requests(),
        [
            "GET /api/v1/missions/m",
            "GET /api/v1/missions/m/artifacts/a-1"
        ]
    );
}

#[test]
fn approved_artifact_approves_a_pending_review_and_submits_a_draft() {
    let pending = ScriptedTransport::answering(vec![
        answer(200, json!({"status": "pending_approval"})),
        answer(
            200,
            json!({"reviews": [{"state": "rejected", "artifact_id": "old"}, {"state": "pending", "artifact_id": "a-2"}]}),
        ),
        answer(200, json!({"id": "m"})),
        answer(200, serde_json::from_str(PROVENANCE).unwrap()),
    ]);
    let client = ApiClient::new(&pending, "http://api", "t");
    assert_eq!(approved_artifact(&client, "m").unwrap().artifact_id, "a-2");
    assert_eq!(
        pending.requests()[2],
        r#"POST /api/v1/approvals/m/approve {"artifact_id":"a-2"}"#
    );

    let draft = ScriptedTransport::answering(vec![
        answer(200, json!({"status": "draft"})),
        answer(200, json!({"id": "m"})),
        answer(
            200,
            json!({"reviews": [{"state": "pending", "artifact_id": "a-3"}]}),
        ),
        answer(200, json!({"id": "m"})),
        answer(200, serde_json::from_str(PROVENANCE).unwrap()),
    ]);
    let client = ApiClient::new(&draft, "http://api", "t");
    assert_eq!(approved_artifact(&client, "m").unwrap().artifact_id, "a-3");
    assert_eq!(draft.requests()[1], "POST /api/v1/missions/m/submit");
    assert_eq!(
        draft.requests()[3],
        r#"POST /api/v1/approvals/m/approve {"artifact_id":"a-3"}"#
    );
}

#[test]
fn a_refused_submission_names_its_code() {
    let transport = ScriptedTransport::answering(vec![
        answer(200, json!({"status": "draft"})),
        answer(
            422,
            json!({"error": "no placed slots", "details": {"code": "NO_PLACED_SLOTS"}}),
        ),
    ]);
    let client = ApiClient::new(&transport, "http://api", "t");
    let error = approved_artifact(&client, "m").unwrap_err().to_string();
    assert!(error.contains("422 NO_PLACED_SLOTS"), "{error}");
}

#[test]
fn artifact_document_must_hash_to_its_entity_tag() {
    let bytes = br#"{"meta":{}}"#.to_vec();
    let digest = sha256_hex(&bytes);
    let tagged = |tag: &str| ApiAnswer {
        status: 200,
        headers: vec![("ETag".into(), format!("\"{tag}\""))],
        body: bytes.clone(),
    };
    let transport = ScriptedTransport::answering(vec![tagged(&digest), tagged(&"0".repeat(64))]);
    let client = ApiClient::new(&transport, "http://api", "t");
    let document = artifact_document(&client, "m", "a").unwrap();
    assert_eq!((document.bytes, document.sha256), (bytes.clone(), digest));
    assert!(artifact_document(&client, "m", "a").is_err());
}

#[test]
fn fleet_scenario_registration_keeps_an_existing_scenario() {
    let transport = ScriptedTransport::answering(vec![
        answer(
            200,
            json!({"items": [{"terrain_key": "everon", "scenario_id": "{A}x.conf"}]}),
        ),
        answer(200, json!({"items": []})),
        answer(200, json!({"terrain_key": "arland"})),
    ]);
    let client = ApiClient::new(&transport, "http://api", "t");
    assert_eq!(
        ensure_fleet_scenario(&client, "everon", "{B}y.conf", "Everon").unwrap(),
        "{A}x.conf"
    );
    assert_eq!(
        ensure_fleet_scenario(&client, "arland", "{C}z.conf", "Arland").unwrap(),
        "{C}z.conf"
    );
    assert_eq!(
        transport.requests()[2],
        r#"PUT /api/v1/fleet/scenarios/arland {"scenario_id":"{C}z.conf","display_name":"Arland"}"#
    );
}

#[test]
fn server_registration_reuses_by_name_and_aligns_the_modpack() {
    let transport = ScriptedTransport::answering(vec![
        answer(
            200,
            json!({"data": [{"id": "s-1", "name": "Playtest", "required_modpack_id": null}]}),
        ),
        answer(200, json!({"id": "s-1"})),
        answer(200, json!({"data": []})),
        answer(201, json!({"id": "s-2"})),
    ]);
    let client = ApiClient::new(&transport, "http://api", "t");
    assert_eq!(
        ensure_server(&client, "Playtest", 2001, Some("m-1")).unwrap(),
        "s-1"
    );
    assert_eq!(
        transport.requests()[1],
        r#"PATCH /api/v1/servers/s-1 {"required_modpack_id":"m-1"}"#
    );
    assert_eq!(
        ensure_server(&client, "Playtest", 2001, None).unwrap(),
        "s-2"
    );
}

#[test]
fn deployments_are_requested_and_followed_to_their_settlement() {
    let transport = ScriptedTransport::answering(vec![
        answer(202, json!({"id": "d-1", "state": "requested"})),
        answer(200, json!({"state": "requested"})),
        answer(
            200,
            json!({"state": "confirmed", "confirmed_runtime_session_id": "rs-1"}),
        ),
        answer(
            200,
            json!({"state": "failed", "failure_reason": "the load_mission command ended expired"}),
        ),
    ]);
    let client = ApiClient::new(&transport, "http://api", "t");
    let requested = request_deployment(&client, "s", "m", "a", Some("em")).unwrap();
    assert_eq!(requested["id"], "d-1");
    assert_eq!(
        transport.requests()[0],
        r#"POST /api/v1/servers/s/deployments {"mission_id":"m","artifact_id":"a","event_mission_id":"em"}"#
    );
    let wait = |client: &ApiClient<'_>| {
        wait_for_deployment(client, "s", "d-1", Duration::from_secs(5), Duration::ZERO).unwrap()
    };
    assert_eq!(
        wait(&client),
        DeploymentSettlement::Confirmed {
            runtime_session_id: "rs-1".into()
        }
    );
    assert_eq!(
        wait(&client),
        DeploymentSettlement::Failed {
            state: "failed".into(),
            reason: "the load_mission command ended expired".into()
        }
    );
}

#[test]
fn credentials_are_issued_and_revoked_with_an_encoded_reason() {
    let transport = ScriptedTransport::answering(vec![
        answer(
            201,
            json!({"credential": {"id": "c-1"}, "secret": "tbdm_x"}),
        ),
        answer(200, json!({"id": "c-1"})),
        answer(200, json!({})),
        answer(409, json!({"details": {"code": "COMMAND_NOT_CANCELLABLE"}})),
    ]);
    let client = ApiClient::new(&transport, "http://api", "t");
    let issued = issue_credential(&client, "s", "mod_runtime", "Playtest runtime").unwrap();
    assert_eq!(
        (issued.credential_id.as_str(), issued.secret.as_str()),
        ("c-1", "tbdm_x")
    );
    revoke_credential(&client, "s", "c-1", "playtest ended").unwrap();
    assert_eq!(
        transport.requests()[1],
        "DELETE /api/v1/servers/s/credentials/c-1?reason=playtest%20ended"
    );
    assert!(cancel_unclaimed_command(&client, "s", "k").unwrap());
    assert!(!cancel_unclaimed_command(&client, "s", "k").unwrap());
}

#[test]
fn staged_artifact_cache_holds_the_bytes_and_their_identity() {
    let directory = std::env::temp_dir().join(format!("tbd-artifact-cache-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&directory);
    let document = br#"{"meta":{"id":"m"}}"#;
    let staged = StagedArtifact {
        artifact_id: "a-1",
        mission_id: "m",
        terrain_key: "everon",
    };
    let sha256 = stage_artifact_cache(&directory, document, &staged).unwrap();
    let cache = directory.join(ARTIFACT_CACHE_DIRECTORY);
    assert_eq!(
        std::fs::read(cache.join("document.json")).unwrap(),
        document
    );
    let identity: Value =
        serde_json::from_str(&std::fs::read_to_string(cache.join("identity.json")).unwrap())
            .unwrap();
    assert_eq!(identity["artifact_sha256"], sha256);
    assert_eq!(identity["artifact_id"], "a-1");
    assert_eq!(identity["artifact_bytes"], document.len());
    assert!(clear_artifact_cache(&directory).unwrap());
    assert!(!cache.exists());
    std::fs::remove_dir_all(&directory).unwrap();
}
