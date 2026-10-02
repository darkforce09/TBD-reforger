//! Contract parity of the frontend golden corpus with the seeded API and the published contracts.
//!
//! **Role:** proves the chain the frontend's typed DTOs rest on: every golden in
//! `apps/website/frontend/tests/fixtures/api/` is what the API answers over the committed seeds,
//! every golden satisfies its route's `contracts` schema, and every golden decodes into the
//! type generated from that schema; and the registry row definitions those goldens answer to
//! carry every constraint of the catalogue definitions they copy.
//!
//! **Position:** a database-backed integration binary (`cargo xtask db test-it --test
//! contract_parity_goldens`); support lives in `tests/contract_parity_support/`. `_index.tsv` is
//! read at runtime, so a golden added with its index row, seed rows and request body is covered
//! without a code change.
//!
//! **Signals & state:** the seeded capture is taken once per process
//! ([`contract_parity_support::seeded_capture`]) and shared by the reproduction and event-stream
//! cases; the other cases read committed files only.
//!
//! **Invariants:** a golden reproduces when its status matches and its JSON equals the live
//! answer as a `serde_json::Value`, or, for an event stream, when its frames equal the live
//! stream's leading frames byte for byte. The only exception is a field the normalisation table
//! ([`contract_parity_support::normalised_fields`]) names: its live value must pass its kind's
//! format check, and the golden holds the kind's placeholder there. Each failure names the golden
//! and the field or frame that moved.

mod common;
mod contract_parity_support;
mod contract_support;

use serde_json::Value;

use contract_parity_support::catalogue_row_constraints::{
    ROW_COPIES, ROW_SCHEMA_FILE, constraint_differences, read_schema,
};
use contract_parity_support::event_stream_frames::{complete_frames, frame_json};
use contract_parity_support::generated_type_decoders::decoder_for;
use contract_parity_support::golden_index::{
    self, EVENT_STREAM_SUFFIX, GoldenRow, JSON_SUFFIX, METHODS, REQUEST_BODY_SUFFIX,
};
use contract_parity_support::golden_normalisation::{
    normalise_live, table_problems as normalisation_table_problems,
};
use contract_parity_support::json_difference::differences;
use contract_parity_support::route_contracts::{
    ContractParts, contract_for, json_parts, stream_parts,
};
use contract_parity_support::seeded_capture::seeded_capture;

/// One report block per failing golden: its file, request and every problem line.
fn report_block(row: &GoldenRow, problems: &[String]) -> String {
    format!(
        "{} ({} {}):\n      {}",
        row.file,
        row.method(),
        row.path,
        problems.join("\n      ")
    )
}

fn assert_no_failures(what: &str, checked: usize, failures: &[String]) {
    assert!(
        failures.is_empty(),
        "{} of {checked} goldens {what}:\n  {}",
        failures.len(),
        failures.join("\n  ")
    );
}

/// Every JSON golden's index status and body equal the seeded API's answer, after each field the
/// normalisation table names passes its format check and takes its placeholder.
#[test]
fn contract_parity_every_frontend_golden_is_reproduced_by_the_seeded_api() {
    let capture = seeded_capture();
    let mut failures = Vec::new();
    let mut compared = 0;
    for (row, answer) in capture
        .answers
        .iter()
        .filter(|(row, _)| !row.is_event_stream())
    {
        compared += 1;
        let mut problems = Vec::new();
        if answer.status.as_u16() != row.status {
            problems.push(format!(
                "status: index {} but live {}",
                row.status, answer.status
            ));
        }
        let live = serde_json::from_slice::<Value>(&answer.body).map_err(|error| {
            format!(
                "the live answer is not JSON ({error}): {}",
                String::from_utf8_lossy(&answer.body[..answer.body.len().min(300)])
            )
        });
        match (row.read_json(), live) {
            (Ok(golden), Ok(live)) => {
                let (live, normalisation_problems) = normalise_live(row, live, &capture.window);
                problems.extend(normalisation_problems);
                problems.extend(differences(&golden, &live));
            }
            (Err(why), _) | (_, Err(why)) => problems.push(why),
        }
        if !problems.is_empty() {
            failures.push(report_block(row, &problems));
        }
    }
    assert!(compared > 0, "the index names no JSON golden");
    assert_no_failures("diverge from the seeded API", compared, &failures);
}

/// Every event-stream golden's frames equal the live stream's leading frames.
#[test]
fn contract_parity_event_stream_goldens_match_the_live_leading_frames() {
    let capture = seeded_capture();
    let mut failures = Vec::new();
    let mut compared = 0;
    for (row, answer) in capture
        .answers
        .iter()
        .filter(|(row, _)| row.is_event_stream())
    {
        compared += 1;
        let mut problems = Vec::new();
        if answer.status.as_u16() != row.status {
            problems.push(format!(
                "status: index {} but live {}",
                row.status, answer.status
            ));
        }
        match row.read_bytes() {
            Ok(golden) => problems.extend(frame_differences(&golden, &answer.body)),
            Err(why) => problems.push(why),
        }
        if !problems.is_empty() {
            failures.push(report_block(row, &problems));
        }
    }
    assert!(compared > 0, "the index names no event-stream golden");
    assert_no_failures("differ from the live stream", compared, &failures);
}

/// Frame-by-frame differences; a differing frame whose data is JSON on both sides also names the
/// fields that moved.
fn frame_differences(golden: &[u8], live: &[u8]) -> Vec<String> {
    let golden_frames = complete_frames(golden);
    let live_frames = complete_frames(live);
    let mut problems = Vec::new();
    if golden_frames.is_empty() || !golden.ends_with(b"\n\n") {
        problems.push("the golden does not end on a complete frame".to_string());
    }
    if live_frames.len() < golden_frames.len() {
        problems.push(format!(
            "the live stream sent {} complete frames within the read limit, the golden holds {}",
            live_frames.len(),
            golden_frames.len()
        ));
    }
    for (index, (expected, actual)) in golden_frames.iter().zip(&live_frames).enumerate() {
        if expected == actual {
            continue;
        }
        problems.push(format!(
            "frame {index}: golden {expected:?} but live {actual:?}"
        ));
        if let (Ok(expected), Ok(actual)) = (frame_json(expected), frame_json(actual)) {
            problems.extend(
                differences(&expected, &actual)
                    .into_iter()
                    .map(|line| format!("frame {index} data{line}")),
            );
        }
    }
    problems
}

/// The golden split into the parts its route contract validates.
fn contract_parts(row: &GoldenRow) -> Result<ContractParts, String> {
    let contract = contract_for(row.method(), &row.path)?;
    if row.is_event_stream() {
        Ok(stream_parts(&contract, &row.read_bytes()?))
    } else {
        Ok(json_parts(&contract, &row.read_json()?))
    }
}

/// Every golden satisfies the schema its route's contract names.
#[test]
fn contract_parity_every_golden_validates_against_its_route_contract() {
    let rows = golden_index::read_index();
    let mut failures = Vec::new();
    let mut validated_parts = 0;
    for row in &rows {
        let problems = match contract_parts(row) {
            Ok(split) => {
                validated_parts += split.parts.len();
                split.violations()
            }
            Err(why) => vec![why],
        };
        if !problems.is_empty() {
            failures.push(report_block(row, &problems));
        }
    }
    assert!(
        validated_parts >= rows.len(),
        "{validated_parts} parts validated for {} goldens",
        rows.len()
    );
    assert_no_failures("break their route contract", rows.len(), &failures);
}

/// Every golden part whose schema is a codegen target decodes into the generated type.
#[test]
fn contract_parity_goldens_decode_into_their_generated_types() {
    let rows = golden_index::read_index();
    let mut failures = Vec::new();
    let mut decoded = 0;
    for row in &rows {
        let problems = match contract_parts(row) {
            Ok(split) => split
                .parts
                .iter()
                .filter_map(|part| decoder_for(part.schema).map(|decode| (part, decode)))
                .filter_map(|(part, decode)| {
                    decoded += 1;
                    decode(&part.value).err().map(|error| {
                        format!(
                            "{}: the type generated from {} rejects it: {error}",
                            if part.at.is_empty() { "/" } else { &part.at },
                            part.schema.describe()
                        )
                    })
                })
                .collect(),
            Err(why) => vec![why],
        };
        if !problems.is_empty() {
            failures.push(report_block(row, &problems));
        }
    }
    assert!(decoded > 0, "no golden part has a generated type");
    assert_no_failures("do not decode", rows.len(), &failures);
}

/// The index and the corpus directory describe the same goldens, each row is well formed, and
/// every normalisation table row names a placeholder its indexed golden holds.
#[test]
fn contract_parity_every_index_row_names_an_existing_golden_and_every_golden_is_indexed() {
    let rows = golden_index::read_index();
    let on_disk = golden_index::corpus_goldens();
    let mut problems = Vec::new();
    let mut indexed = std::collections::BTreeSet::new();
    for row in &rows {
        let at = format!("_index.tsv:{} ({})", row.line, row.file);
        if !indexed.insert(row.file.clone()) {
            problems.push(format!("{at}: indexed twice"));
        }
        if !METHODS.contains(&row.method()) {
            problems.push(format!("{at}: the file name starts with no HTTP method"));
        }
        if !row.path.starts_with('/') {
            problems.push(format!("{at}: path {:?} is not absolute", row.path));
        }
        if !(100..600).contains(&row.status) {
            problems.push(format!("{at}: status {} is not an HTTP status", row.status));
        }
        match row.read_bytes() {
            Ok(bytes) => {
                let size = format!("{}B", bytes.len());
                if row.size_column != size {
                    problems.push(format!(
                        "{at}: the size column says {} but the golden holds {size}",
                        row.size_column
                    ));
                }
            }
            Err(_) => problems.push(format!("{at}: no such golden in the corpus directory")),
        }
    }
    for file in on_disk.difference(&indexed) {
        problems.push(format!("{file}: in the corpus directory but not indexed"));
    }
    for file in &on_disk {
        if !file.ends_with(JSON_SUFFIX) && !file.ends_with(EVENT_STREAM_SUFFIX) {
            problems.push(format!("{file}: neither a JSON nor an event-stream golden"));
        }
    }
    for body in golden_index::stored_request_bodies() {
        let owner = rows
            .iter()
            .find(|row| row.request_body_file() == body && row.method() != "GET");
        if owner.is_none() {
            problems.push(format!(
                "{body}: no indexed write golden sends it (a `{REQUEST_BODY_SUFFIX}` belongs to \
                 one POST, PUT, PATCH or DELETE row)"
            ));
        }
    }
    problems.extend(normalisation_table_problems(&rows));
    assert!(!rows.is_empty(), "the index names no golden");
    assert!(
        problems.is_empty(),
        "the golden index and corpus disagree:\n  {}",
        problems.join("\n  ")
    );
}

/// The registry row definitions of `arsenal-envelopes.schema.json` carry every constraint of the
/// catalogue definitions they copy (`registry-items.schema.json#/$defs/item`,
/// `registry-compat.schema.json#/$defs/edge`): the `kind` and `edge_type` vocabularies, patterns,
/// minimums and required names.
#[test]
fn contract_parity_registry_row_constraints_match_the_catalogue_schemas() {
    let row_document = read_schema(ROW_SCHEMA_FILE);
    let mut failures = Vec::new();
    for copy in ROW_COPIES {
        let catalogue_document = read_schema(copy.catalogue_file);
        failures.extend(constraint_differences(
            copy,
            &row_document,
            &catalogue_document,
        ));
    }
    assert!(
        failures.is_empty(),
        "{} catalogue constraint(s) differ in their row copies:\n  {}",
        failures.len(),
        failures.join("\n  ")
    );
}
