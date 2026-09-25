# Verifications

The repository checks behind `cargo xtask verify`, grouped by the invariant they enforce, one
folder per group. Each check reads the tree and reports a verdict; the operational commands that
change things live in `tools_v2/xtask/src/commands/`.

## Contents

```text
tools_v2/xtask/src/verifications/
├── api_readiness/                   API completion evidence for every requirement in the acceptance register
├── architecture/                    engine layer walls, API route tags, Mission Creator ORBAT coherency
├── ci/                              workflow `run:` step rules and CI schema parity
├── database/                        faction library and wiki seed pins, and the API's SELECT * ban
├── deployment/                      the staging deploy's compose file path
├── documentation/                   the readme-coverage, markdown-placement and link-check gates
├── language_bans/                   the shell, Python and Node bans and the file length limits
├── licensing/                       upstream framework identifiers and asset GUIDs kept out of the addons
├── map_assets/                      adapters that forward the map asset checks to developer-tools
├── mod.rs                           the module tree
├── mod_scripts/                     mod script pins, the UI layout gate and the Workbench spawn checks
├── property_test_configuration.rs   the property-test seed, and the refusal of a case-count override
├── registry/                        the Objects palette's rows in the mod's spawn registry
├── schemas/                         the contract schema, fixture and `@contract` citation gates
└── tests/                           unit tests for the property-test configuration
```

## How it works

Three callers reach the same verification functions: the `verify` group
(`tools_v2/xtask/src/commands/verify/dispatch.rs`) and the `schema` group
(`tools_v2/xtask/src/commands/schema/dispatch.rs`) call them directly; the CI task table
(`tools_v2/xtask/src/commands/ci/task_definitions.rs`) calls them in process as `ci-local` and
other task steps; and the platform wave gate runs `cargo run -q -p xtask -- verify <name>` for
each row of its `VERIFY_STEPS`. A few folders also serve other groups: `mod_scripts/` holds the
`cargo xtask mod spawn-determinism` and `cargo xtask mod spawn-verify` runs, and
`language_bans/` the font table that `cargo xtask gen font-table` writes.

Every check takes its paths from `tools_v2/xtask/src/core/repository_layout.rs` or its own named
constants, and builds its verdicts from `verification-core`; source matchers are compiled in
(`verification_core::Pattern` or the `regex` crate), and an external tool runs only where a check
needs one, such as `git`, `cargo` or `grep`. An input a check could not read is "did not run",
never a pass.
Most gates exit 0 when every check held, 1 on a violation and 2 when a check did not run. Some
keep a binary 0 or 1 on purpose and name a missing input in their output instead, because their
callers record only pass or fail: among them `wiki-seeds`, `staging-compose-paths`,
`ci-schema-parity` and `editor-orbat-coherency`; `faction-library-seeds` exits 2 only when it
cannot build its broken variants. Each group's README gives its gates' codes.

`property_test_configuration.rs` fixes how property tests are seeded for
`cargo xtask verify api-readiness` and `cargo xtask db test-it`: `PROPTEST_RNG_SEED` must be
decimal digits within `u64` (default `2026092201`), and a set `PROPTEST_CASES` is refused, since
each test suite owns its case count. It renders the seed as the output marker and the receipt
environment that `api_readiness/` checks.

## Public surface

- `map_assets`, the one `pub` group, and every other group as `pub(crate)`: each gate is a
  function that takes the repository root (or reads it) and returns its exit status as `u8`.
- `property_test_configuration::PropertyTestConfiguration::from_environment`, read by
  `tools_v2/xtask/src/commands/db/operations/test_it.rs`.

## Boundaries

- Depends on: `verification-core` (verdicts, reports, patterns, scans, process runs);
  `developer-tools` for the map asset checks; the layout constants in
  `tools_v2/xtask/src/core/repository_layout.rs`; the CI task table and the seed list in
  `tools_v2/xtask/src/commands/`, which some pins read in process.
- Used by: the `verify`, `schema`, `ci`, `db`, `mod` and `gen` command groups under
  `tools_v2/xtask/src/commands/`; the wave gate in
  `tools_v2/xtask/src/commands/platform/wave_execution/gate.rs`; and the jobs of
  `.github/workflows/ci.yml` that run `cargo xtask verify` verbs.
- Rules:
  - a check never reports a pass over an input it did not read (each group's tests hold this for
    its gates);
  - a verification reads and reports; only `api_readiness/` (its evidence folder, under
    `--execute`) and the spawn determinism run in `mod_scripts/` write files;
  - `PROPTEST_CASES` stays unset and the seed is decimal
    (`tools_v2/xtask/src/verifications/tests/property_test_configuration.rs`).

## Related documentation

- [Local development](/documentation_v2/runbooks/local_development.md) — running `ci-local` and
  its prerequisites.
