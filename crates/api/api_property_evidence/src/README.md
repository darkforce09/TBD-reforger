# API property evidence source

The source of `api_property_evidence`: the recorder module, its tests and the crate root.

## Contents

```text
crates/api/api_property_evidence/src/
├── lib.rs           the crate root: module header, `mod` lines and the re-exports
├── prelude.rs       `run_property`, `collect_property` and `PropertyRun` for glob import
├── property_run.rs  `run_property`, `collect_property` and `PropertyRun`: the run and its record
└── tests/           the negative controls: case counting, seed reproduction, failure and rejection
```

## How it works

`property_run.rs` drives a `proptest` `TestRunner` with a fixed configuration (no failure
persistence, no fork, no timeout) and records each completed case into a `Sha256Hasher`.

## Boundaries

- Depends on: `content_digest`, `proptest`, `serde` and `serde_json`.
- Used by: the crate root.
- Rules: the record's JSON keys are `version`, `id`, `requested_cases`, `executed_cases`, `seed`,
  `algorithm` and `input_sha256`.
