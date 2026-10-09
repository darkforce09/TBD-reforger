# API readiness checks

The `api_readiness_checks` crate, behind `cargo xtask verify api-readiness`: the
[API](/documentation/glossary/a_to_f.md#api) counts as complete only when every requirement in the
acceptance register has current, passing evidence from every check it names. The crate judges the
receipts, runs the local checks first when asked, records the receipts of the staging checks, and
fixes the property-test seed every readiness run and `cargo xtask db test-it` use.

## Contents

```text
tools/commands/api_readiness_checks/
├── Cargo.toml  the `api_readiness_checks` library package: `verification_core`, `process_runner`, `content_digest`, `deploy_settings`, `repository_layout`, layout tier 3
└── src/        the register, the receipts and their judge, the fingerprints, the evidence writer, the staging recorder and the seed
```

## How it works

```text
verify(root, evidence, execute)
  ├── PropertyTestConfiguration::from_environment   PROPTEST_CASES unset, PROPTEST_RNG_SEED decimal
  ├── register ──▶ fingerprints (source, configuration)
  ├── [execute: run each local command ──▶ <id>.log + <id>.json]
  ├── per check: <id>.json + its log ──▶ evidence judge ──▶ one verdict
  ├── per requirement: every named check held?
  └── fingerprints unchanged? ──▶ the report's exit status

RecordingSession::begin ──▶ the staging procedure runs ──▶ finish(RecordedOutcome)
  └── judged log, fixture manifest, receipt (PASS exit 0, FAIL exit 1)
```

`src/README.md` describes each step, the staging log grammar and the rules the tests pin.

## Getting started

Run from the repository root:

```bash
cargo test -p api_readiness_checks   # register, receipts, fingerprints on throwaway Git trees, recordings, the seed
```

## Configuration

No feature. The crate reads `PROPTEST_CASES` (refused when set), `PROPTEST_RNG_SEED` (default
`2026092201`), the configuration files and variables the configuration fingerprint covers, and,
for a staging recording, the environment the harness hands it, never `std::env`.

## Public surface

- At the crate root: `verify`, `PropertyTestConfiguration`, `Error` and `Result`.
- `operational_recording`: `RecordingSession`, `RecordedOutcome`, `RecordedReceipt`,
  `StagingCheck`, `FixtureManifest`, `current_fingerprints`, and the log values `CaseName`,
  `CaseStatus`, `RecordedCase`, `EnvironmentEntry`, `ObservationRecord` and `Observations`.
- `prelude`: `verify`, `PropertyTestConfiguration` and the recorder types the staging procedures
  build.

## Boundaries

- Depends on: `verification_core` (`Report`, `Verdict`), `process_runner` (`git`, the tool
  versions, the check commands), `content_digest` (SHA-256), `deploy_settings`
  (`DEPLOY_ENV_OVERRIDE_VARIABLE`), `repository_layout` (the register, the evidence prefix,
  `deploy.env`, the workspace folders), `time_source` (the receipts' wall clock), `libc` (the directory-pinned writes), `regex`, `serde`, `serde_json` and
  `thiserror`; `tool_test_support` in tests.
- Used by: the `verify api-readiness` command of `xtask`, the `staging` procedures (the recorder
  and `staging fingerprints`), and `xtask db test-it` (the seed).
- Rules: tier 3 of `tools/commands` (`cargo xtask verify crate-tiers`); a receipt counts only
  against both current fingerprints and for 24 hours; `--execute` never runs an `operational`
  check; only a passing staging recording carries the success marker. The tests that pin each
  rule are named in `src/README.md`.

## Related documentation

- [Command crates](/tools/commands/README.md) — the command crates and their tiers.
- [Verification evidence](/documentation/crates/api/api_server/verification_evidence/README.md) — the
  acceptance register, the receipts and the staging procedures this crate judges.
- [Staging verification](/documentation/crates/api/api_server/verification_evidence/staging.md) — how the
  staging runner records its operational receipts.
