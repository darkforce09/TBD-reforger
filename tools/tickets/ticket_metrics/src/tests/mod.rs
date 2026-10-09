use super::*;
use repository_layout::METRICS_SCHEMA;
use serde_json::Value;
use std::fs;
use std::path::{Path, PathBuf};
use ticket_model::TicketId;

use serde_json::json;

fn fixture(name: &str) -> Value {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/execution_receipts")
        .join(name);
    serde_json::from_str(&fs::read_to_string(&path).expect("read fixture")).expect("parse fixture")
}

fn scratch(tag: &str) -> PathBuf {
    let tmp = std::env::temp_dir().join(format!("tbd-metrics-{tag}-{}", std::process::id()));
    let _ = fs::remove_dir_all(&tmp);
    fs::create_dir_all(tmp.join(repository_layout::TICKETS_DIR)).expect("mk scratch");
    // The real committed schema, so scratch trees validate exactly like the repo.
    let schema = repository_root::find_repository_root()
        .expect("repository root")
        .join(METRICS_SCHEMA);
    fs::copy(&schema, tmp.join(METRICS_SCHEMA)).expect("copy schema");
    tmp
}

fn rec(id: &str, agent: &str, input: u64, started: &str, finished: &str) -> RunRecord {
    RunRecord {
        id: id.into(),
        agent: agent.into(),
        started: started.into(),
        finished: Some(finished.into()),
        outcome: Some("ran".into()),
        git_sha: Some("0123456789abcdef0123456789abcdef01234567".into()),
        tokens_consumed: TokensConsumed {
            input,
            output: 0,
            cache_read: 0,
            cache_write: 0,
            total: input,
            reasoning: None,
        },
    }
}

mod receipt_validation_tests;
