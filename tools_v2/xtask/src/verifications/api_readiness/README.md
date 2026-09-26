# API readiness verification

The check behind `cargo xtask verify api-readiness`: the [API](/documentation_v2/glossary/a_to_f.md#api)
counts as complete only when every requirement in the acceptance register has current, passing
evidence from every check it names. It judges receipts; with `--execute` it first runs the local
checks and writes their receipts.

## Contents

```text
tools_v2/xtask/src/verifications/api_readiness/
├── case_count.rs         counts distinct successful cases per cargo test binary or doc-test suite
├── evidence.rs           the receipt shape, and the rules that accept a receipt against its output
├── evidence_storage.rs   publishes receipts and logs atomically without following symlinks
├── fingerprint.rs        the source and configuration digests that bind evidence to one tree
├── mod.rs                `verify`: runs the local checks under `--execute`, then judges every receipt
├── operational.rs        the measured thresholds for the staging fleet, Discord and load checks
├── property_evidence.rs  the property-run records and each property's generated-case minimum
├── register.rs           reads and validates the acceptance register of requirements and checks
└── tests/                unit tests for case counting, receipts, property evidence and fingerprints
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
   (`git ls-files --cached --others --exclude-standard`) under `apps/`, `tools_v2/`,
   `contracts_v2/`, `.cargo/`, `.github/` and the evidence folder with a source extension (Markdown included), plus a fixed list of root
   files (the workspace manifest and lock, the toolchain, format and lint settings,
   `.editorconfig`, `.gitignore`); a symlink anywhere on an input's path fails the run. The configuration digest covers the three `.env`
   files and `deploy.env` (present or absent), every `PROPTEST_*` variable, and a fixed list of
   build and API environment variables; no value is printed.
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

`--execute` never runs an `operational` check or a check without a command: an external staging
runner writes those receipts into the evidence folder.

## Boundaries

- Depends on: `verification-core` (`Report`, `Verdict`, and `proc::Run` for git, the tool
  versions and the check commands); the parent's `property_test_configuration.rs` for the seed;
  `API_READINESS_REGISTER`, `API_READINESS_EVIDENCE_PREFIX` and `DEPLOY_ENV` from
  `tools_v2/xtask/src/core/repository_layout.rs`; `libc` for the directory-pinned writes; the
  `regex`, `serde_json` and `sha2` crates.
- Used by: `tools_v2/xtask/src/commands/verify/dispatch.rs`, for
  `cargo xtask verify api-readiness [--evidence <dir>] [--execute]`. Exit 0 when every check and
  requirement held, 1 on a rejected receipt or an unfulfilled requirement (and on an error that
  stops the run, such as an invalid register or a set `PROPTEST_CASES`), 2 when any receipt is
  missing.
- Rules:
  - evidence goes stale on any change to a fingerprinted file, Markdown included, or to the
    configuration, and after 24 hours
    (`stale_changed_failed_and_omitted_evidence_is_rejected` in `tests/evidence.rs`);
  - a receipt or log is published by an atomic rename inside a directory opened without
    following symlinks, and never writes through an existing link
    (`evidence_writes_do_not_follow_symlinks_or_modify_hardlinked_targets`);
  - an ignored or skipped test fails the check, and a repeated output line counts as one case
    (`ignored_tests_with_reasons_and_unreported_ignored_summaries_fail`,
    `duplicate_output_cannot_substitute_for_distinct_acceptance_cases`);
  - an operational receipt meets every recorded threshold
    (`operational_load_requires_all_recorded_acceptance_conditions`).

## Related documentation

- [Acceptance register](/documentation_v2/website/api_v2/verification_evidence/requirements.json)
  — every requirement, its implementation paths and the checks that prove it.
- [Property-test acceptance evidence](/documentation_v2/website/api_v2/verification_evidence/property_test_evidence.md)
  — why generated cases and test functions are counted apart.
- [API v2 completion and executable verification](/documentation_v2/website/api_v2/verification_evidence/completion_plan.md)
  — the acceptance contract this command enforces.
