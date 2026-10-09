# API readiness checks source

The check behind `cargo xtask verify api-readiness`: the [API](/documentation/glossary/a_to_f.md#api)
counts as complete only when every requirement in the acceptance register has current, passing
evidence from every check it names. It judges receipts; with `--execute` it first runs the local
checks and writes their receipts.

## Contents

```text
tools/commands/api_readiness_checks/src/
├── case_count.rs                   counts distinct successful cases per cargo test binary or doc-test suite
├── error.rs                        `Error` and `Result`; the crate-private refusal macros and context trait
├── evidence.rs                     the receipt shape, and the rules that accept a receipt against its output
├── evidence_storage.rs             publishes receipts and logs atomically without following symlinks
├── fingerprint.rs                  the source and configuration digests that bind evidence to one tree
├── lib.rs                          the crate root: module header, `mod` lines and the re-exports
├── operational.rs                  the measured thresholds for the staging fleet, Discord and load checks
├── operational_log.rs              the staging log grammar: its line values, their validation and escaping
├── operational_recording.rs        records one staging run and writes its judged receipt, log and manifest
├── prelude.rs                      `verify`, the property-test seed and the staging recorder for glob import
├── property_evidence.rs            the property-run records and each property's generated-case minimum
├── property_test_configuration.rs  the property-test seed, and the refusal of a case-count override
├── readiness_verification.rs       `verify`: runs the local checks under `--execute`, then judges every receipt
├── register.rs                     reads and validates the acceptance register of requirements and checks
├── tool_identity.rs                the rustc, cargo and git version lines every receipt records
└── tests/                          unit tests: case counting, receipts, recordings, property evidence, fingerprints, the seed
```

## How it works

```text
register.rs ──▶ fingerprint.rs (source, configuration) ──▶ [--execute: run each local command,
                                                             write <id>.log and <id>.json]
            ──▶ per check: read <id>.json + its log ──▶ evidence.rs ──▶ one verdict per check
            ──▶ per requirement: every named check held? ──▶ fingerprints unchanged? ──▶ exit
```

1. `register.rs` reads `API_READINESS_REGISTER` (the `requirements.json` under
   `API_READINESS_EVIDENCE_PREFIX`) with unknown fields refused, and validates it: version 1,
   unique identifiers, an existing repository path for every implementation entry, a positive
   timeout and case minimum, a success marker and a compiling case pattern for every check, a
   property list exactly on `property` checks, and every check named by some requirement.
2. `fingerprint.rs` hashes the source: every tracked or untracked, not ignored file
   (`git ls-files --cached --others --exclude-standard`) under `mod/`, `crates/`,
   `tools/`, `contracts/`, `.cargo/`, `.github/` and the evidence folder with a source extension
   (Markdown included), plus a fixed list of root files (the workspace manifest and lock, the
   toolchain, format and lint settings, `AGENTS.md`, `.editorconfig`, `.gitignore`). A symlink
   that Git tracks (index mode 120000) is hashed as its link text under its own tag, and only
   when it resolves to an existing entry inside the repository; any other symlink on an input's
   path (one Git does not track as a symlink, or a symlinked ancestor directory) fails the run.
   The configuration digest covers the root `.env`, the API server's `crates/api/api_server/.env`
   and `deploy.env` (each path with its presence and contents, so a moved file changes it),
   every `PROPTEST_*` variable, and a fixed list of build and API environment variables, and
   refuses a symlinked configuration file; no value is printed.
3. With `--execute`, every check that carries a command and is not `operational` runs once per
   distinct command, from the repository root, under its timeout, with `PROPTEST_RNG_SEED` set
   and `PROPTEST_CASES` removed. The merged output becomes `<id>.log` and the receipt `<id>.json`
   in the evidence folder (default `target/api-readiness`), both written through
   `evidence_storage.rs`. A command that cannot start deletes the older receipts of every check
   that shares it, so no earlier green receipt survives.
4. Each check is judged from its files: a missing receipt is "did not run"; otherwise
   `evidence.rs` requires the right identity and command, both current fingerprints, a start at
   most 24 hours ago, a duration within the timeout, exit 0, the output's SHA-256, the success
   marker, no skipped database run and no ignored test, and at least the minimum of distinct
   case-pattern matches (`case_count.rs` counts a match once per test binary). A `property` check
   also needs one `property-run:` record per required property, with the configured seed, the
   `ChaCha` algorithm and every requested case executed (`property_evidence.rs`); an
   `operational` check needs an environment identity and measurements that meet the thresholds in
   `operational.rs` (five servers and two clients for the fleet, 30 minutes, 1000 accounts, 100
   concurrent clients, 20 requests a second, no unexpected error and p95 JSON reads within 500 ms
   and writes within 1000 ms for load).
5. A requirement fails when any check it names did not hold. The run ends by recomputing both
   fingerprints and refusing a result whose tree or configuration changed while it ran.

`--execute` never runs an `operational` check or a check without a command: the staging harness
records those receipts into the evidence folder through `operational_recording.rs`.

## Recording staging receipts

The three `operational` checks (`staging_fleet`, `staging_discord`, `staging_load`) have no local
command. The staging harness records each one while its procedure runs:

1. `RecordingSession::begin(root, evidence_dir, check, argv, environment)` first refuses an
   `environment` that sets `TEST_DATABASE_URL`, `DEPLOY_ENV` or any `PROPTEST_*` (the refusal names
   the variable, never its value). This run discipline reads only the variables it is given: the
   harness hands it its own process environment and a test a clean one, so the environment a test
   happens to run in (such as `--execute`'s, which sets `PROPTEST_RNG_SEED`) never decides it.
   `begin` then refuses a check the register does not declare `operational` with a null command
   and the success marker `<check>: PASS`, snapshots both fingerprints, the start time and the
   tool versions (`tool_identity.rs`, which `--execute` shares), picks the run id the harness
   journal reuses, and deletes the check's earlier receipt, so no older PASS outlives a newer
   attempt; a refusal leaves that receipt in place.
2. The procedure ends, a partial run included, by handing `finish` a `RecordedOutcome`: every
   declared case (`Ok`, `Failed(reason)` or `NotRun { missing }`), the environment identities,
   the `Observations` that `operational.rs` judges, the fixture manifest, and the journaled
   observations.
3. `finish` recomputes both fingerprints; drift fails the run, and the receipt keeps the start
   digests. The run is a candidate PASS only when every case is ok and named once, the fleet or
   Discord observations cite the manifest's SHA-256, `operational.rs` accepts the measurements and
   the duration is within the check's timeout. `evidence::validate` then judges the exact receipt
   and log, and a rejection rewrites the run as a FAIL with the judge's reason.
4. It writes `<check>.log`, `<check>.fixture.json` and, last, `<check>.json` through
   `evidence_storage.rs`, and returns exit code 0 only on PASS. A FAIL exits 1 and keeps the real
   observations.

The log (`operational_log.rs`) reads, one record per line:

```text
staging-run: <check> run=<id> started=<unix> command=<argv>
environment: <key>=<value>
fixture: sha256=<hex> manifest=<check>.fixture.json
observation: <step> <observer> <summary> sha256=<raw artifact digest>
case <check>_<name> ... ok | FAILED (<why>) | NOT RUN (missing: <dependency>)
missing: <dependency>
<check>: PASS <ok>/<declared>                  (full acceptance only)
<check>: FAIL <ok>/<declared> (<reasons>)
```

- Case names and environment keys match `[a-z0-9_]+`. A key that contains `token`, `secret`,
  `password`, `credential`, `authorization` or `cookie` is refused, so no secret is recorded.
- Every line but a passing verdict is escaped as a whole: a backslash doubles, and a control
  character, a Unicode line or paragraph separator, or the first character of the success marker
  becomes `\u{hex}`. No value can therefore start a line of its own, and a failing log never
  carries the marker.
- The fixture digest is the SHA-256 of the manifest bytes written beside the receipt
  (`FixtureManifest::sha256`), the value fleet and Discord observations cite.

## Boundaries

- Depends on: `verification_core` (`Report`, `Verdict`); `process_runner::Run` for git, the tool
  versions and the check commands; `property_test_configuration.rs` for the seed;
  `API_READINESS_REGISTER` and `API_READINESS_EVIDENCE_PREFIX` from
  `tools/foundation/repository_layout/src/documentation_locations.rs` and `DEPLOY_ENV` from `tools/foundation/repository_layout/src/deployment.rs`; `libc` for the directory-pinned writes; the
  `regex`, `serde_json` and `content_digest` crates.
- Used by: the crate root's re-exports; `tools/xtask/src/commands/verify/dispatch.rs` calls
  `verify` for
  `cargo xtask verify api-readiness [--evidence <dir>] [--execute]`. Exit 0 when every check and
  requirement held, 1 on a rejected receipt or an unfulfilled requirement (and on an error that
  stops the run, such as an invalid register or a set `PROPTEST_CASES`), 2 when any receipt is
  missing. The staging harness records the `operational` checks through
  `operational_recording.rs`, whose `Observations`, case, environment and observation types it
  builds, and whose `current_fingerprints` `cargo xtask staging fingerprints` prints.
- Rules:
  - evidence goes stale on any change to a fingerprinted file, Markdown included, or to the
    configuration, and after 24 hours
    (`stale_changed_failed_and_omitted_evidence_is_rejected` in `tests/evidence.rs`);
  - a tracked symlink binds its link text, so retargeting it invalidates evidence, and it never
    shares a hash input with a regular file holding the same bytes; an untracked, escaping or
    dangling symlink fails the fingerprint
    (`tracked_symlink_is_fingerprinted_stably_by_its_tagged_link_text`,
    `retargeting_a_tracked_symlink_changes_the_fingerprint`,
    `untracked_symlink_beside_an_accepted_tracked_symlink_is_refused`,
    `tracked_symlink_resolving_outside_the_repository_or_nowhere_is_refused` in
    `tests/source_fingerprint.rs`);
  - a receipt or log is published by an atomic rename inside a directory opened without
    following symlinks, and never writes through an existing link
    (`evidence_writes_do_not_follow_symlinks_or_modify_hardlinked_targets`);
  - an ignored or skipped test fails the check, and a repeated output line counts as one case
    (`ignored_tests_with_reasons_and_unreported_ignored_summaries_fail`,
    `duplicate_output_cannot_substitute_for_distinct_acceptance_cases`);
  - an operational receipt meets every recorded threshold
    (`operational_load_requires_all_recorded_acceptance_conditions`,
    `operational_load_refuses_more_reads_and_writes_than_completed_requests_and_a_blank_network`,
    `operational_fleet_requires_five_servers_two_clients_every_scenario_and_a_fixture_digest`,
    `operational_discord_requires_every_scenario_and_a_fixture_digest`);
  - a staging recording passes only when the judge holds its exact receipt and log
    (`a_passing_recording_is_judged_held` in `tests/operational_recording.rs`); a failed or
    not-run case, a rejected measurement, drift or a judge rejection writes a FAIL that exits 1
    with the real observations and without the success marker, which the judge refuses
    (`a_failed_case_fails_the_run`,
    `a_case_not_run_fails_the_run_and_names_the_missing_dependency`,
    `measurements_the_operational_thresholds_reject_fail_with_the_real_observations`,
    `drift_fails_the_run_and_the_receipt_keeps_the_start_digests`,
    `a_judge_rejection_rewrites_a_candidate_pass_as_a_failure`);
  - free text in a recording cannot forge a case line or the success marker
    (`free_text_cannot_forge_case_lines_or_the_success_marker`);
  - a tool that exits non-zero or prints no version line fails the tool identity instead of
    recording a blank one (`tool_identity_refuses_a_tool_that_fails_or_prints_no_version`).

## Related documentation

- [Acceptance register](/documentation/crates/api/api_server/verification_evidence/requirements.json)
  — every requirement, its implementation paths and the checks that prove it.
- [Property-test acceptance evidence](/documentation/crates/api/api_server/verification_evidence/property_test_evidence.md)
  — why generated cases and test functions are counted apart.
- [API v2 completion and executable verification](/documentation/crates/api/api_server/verification_evidence/completion_plan.md)
  — the acceptance contract this command enforces.
