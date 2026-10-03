# API property evidence

The `api_property_evidence` crate: the property run recorder of the
[API](/documentation/glossary/a_to_f.md#api)'s property tests. It runs a proptest strategy with a
fixed seed and case count, counts the checks that completed, digests their inputs and prints the
run's record, so a property suite's log shows how many generated cases it really checked and from
which input stream. Dev-only: only `[dev-dependencies]` name it.

## Contents

```text
crates/api/api_property_evidence/
├── Cargo.toml  the package: `proptest`, `content_digest`, `serde_json`, layout tier 1
└── src/        the recorder and its negative controls
```

## How it works

`run_property(property_name, cases, strategy, check)` runs `check` over `cases` values of the
strategy with the ChaCha generator seeded from `PROPTEST_RNG_SEED`, or a fixed default when unset,
and prints one `property-run: <json>` line holding the property name, the requested and executed
case counts, the seed, the algorithm and the input digest. The digest is the SHA-256 of every
completed case's `Debug` text, each prefixed by its length as a little-endian `u64`, so the same
seed reproduces the same digest. A run that executes zero cases, fails a check or completes fewer
checks than requested panics instead of printing a record; rejected cases do not count.
`collect_property` returns the record instead of printing it, for the recorder's own controls.

## Getting started

Run from the repository root:

```bash
cargo test -p api_property_evidence   # the recorder's negative controls
```

## Configuration

`PROPTEST_RNG_SEED` (decimal digits) replaces the default seed; `PROPTEST_CASES` must be unset,
because each property owns its case count. No feature.

## Public surface

- `run_property`, `collect_property` and `PropertyRun` (`executed_cases`, `input_sha256`), at the
  crate root and in `prelude`.

## Boundaries

- Depends on: `content_digest` (`Sha256Hasher::update_length_framed`), `proptest`, `serde` and
  `serde_json`.
- Used by: the API's operations unit tests (event access, reservation planning, seat matching,
  quota selection) and its property suites under `apps/api/tests/`, all through
  `[dev-dependencies]`.
- Rules: never a normal dependency of any crate; a property's record is evidence only when its
  executed case count equals its requested count.

## Related documentation

- [API crates](/crates/api/README.md) — the category this crate belongs to and its rules.
- [Website API](/apps/api/README.md) — the property suites that run through the recorder.
