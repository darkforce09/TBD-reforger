//! The game-runtime smoke against scripted answers: each check passes or fails on exactly what
//! the platform answered.
use std::cell::RefCell;
use std::collections::VecDeque;

use serde_json::json;

use super::*;

struct Scripted(RefCell<VecDeque<ApiAnswer>>);

impl ApiTransport for Scripted {
    fn exchange(&self, _: &str, _: &str, _: Option<&str>, _: Option<&Value>) -> Result<ApiAnswer> {
        self.0
            .borrow_mut()
            .pop_front()
            .ok_or_else(|| anyhow::anyhow!("unscripted request"))
    }
}

fn scripted(answers: Vec<ApiAnswer>) -> Scripted {
    Scripted(RefCell::new(answers.into()))
}

fn answer(status: u16, body: Value) -> ApiAnswer {
    ApiAnswer {
        status,
        headers: Vec::new(),
        body: serde_json::to_vec(&body).unwrap(),
    }
}

fn artifact(bytes: &[u8], tag: &str) -> ApiAnswer {
    ApiAnswer {
        status: 200,
        headers: vec![("ETag".into(), format!("\"{tag}\""))],
        body: bytes.to_vec(),
    }
}

#[test]
fn no_deployment_is_a_pass_when_the_refusal_is_named() {
    let transport = scripted(vec![
        answer(404, json!({"details": {"code": "NO_DEPLOYMENT"}})),
        answer(401, json!({})),
    ]);
    assert_eq!(smoke(&transport, "http://api", "tbdm_x").unwrap(), 0);
    let unnamed = scripted(vec![answer(404, json!({})), answer(401, json!({}))]);
    assert_eq!(smoke(&unnamed, "http://api", "tbdm_x").unwrap(), 1);
}

#[test]
fn a_deployed_artifact_must_hash_to_its_recorded_digest_and_tag() {
    let bytes = br#"{"meta":{"id":"m"}}"#;
    let digest = sha256_hex(bytes);
    let deployment = json!({"deployment_id": "d", "artifact_id": "a", "artifact_sha256": digest,
                            "event_id": "e"});
    let intact = scripted(vec![
        answer(200, deployment.clone()),
        artifact(bytes, &digest),
        answer(200, json!({"version": 2})),
        answer(401, json!({})),
    ]);
    assert_eq!(smoke(&intact, "http://api", "tbdm_x").unwrap(), 0);
    let altered = scripted(vec![
        answer(200, deployment.clone()),
        artifact(b"{}", &digest),
        answer(200, json!({"version": 1})),
        answer(401, json!({})),
    ]);
    assert_eq!(
        smoke(&altered, "http://api", "tbdm_x").unwrap(),
        2,
        "bytes and roster version"
    );
}

#[test]
fn an_open_deployment_read_fails_the_smoke() {
    let transport = scripted(vec![
        answer(404, json!({"details": {"code": "NO_DEPLOYMENT"}})),
        answer(404, json!({"details": {"code": "NO_DEPLOYMENT"}})),
    ]);
    assert_eq!(smoke(&transport, "http://api", "tbdm_x").unwrap(), 1);
}

#[test]
fn no_answer_at_all_is_an_environment_error() {
    assert!(smoke(&scripted(Vec::new()), "http://api", "tbdm_x").is_err());
}
