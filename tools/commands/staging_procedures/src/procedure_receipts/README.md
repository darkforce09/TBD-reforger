# Staging procedure receipts

The receipt of one recorded staging procedure run: the three recorded checks and their limits, the
log grammar, the measurements a passing run must meet, and the recorder that judges a run and
writes its receipt into `target/staging/receipts/`.

## Contents

```text
tools/commands/staging_procedures/src/procedure_receipts/
├── acceptance_thresholds.rs  `Observations` and the fleet, Discord and load thresholds a passing run meets
├── mod.rs                    the module tree and the re-exports the procedures import
├── receipt_log.rs            case names and outcomes, environment identities, observation lines, the log grammar
├── recording_session.rs      `RecordingSession`: begin, judge, and write the log, manifest and receipt
└── staging_check.rs          `StagingCheck`: the check ids, receipt file names, minimum passing cases, time limit
```

## How it works

`RecordingSession::begin` removes the check's earlier receipt and snapshots the start time and the
run id. `finish` takes the procedure's `RecordedOutcome` and collects the reasons a PASS is
refused: no declared case, a case named twice, a failed or not-run case, fewer passing cases than
the check's minimum (fleet 50, Discord 13, load 10), no environment identity, observations of the
wrong kind or below the thresholds, fleet or Discord observations citing another fixture digest,
or a run longer than two hours. It renders the log once with the verdict line `<check>: PASS
<ok>/<declared>` or `<check>: FAIL <ok>/<declared> (<reasons>)` and writes three files, each
through a temporary file renamed over its destination:

```text
target/staging/receipts/<check>.log           the escaped log; only a passing verdict carries `<check>: PASS`
target/staging/receipts/<check>.fixture.json  the fixture manifest, the bytes its digest covers
target/staging/receipts/<check>.json          check id, command, start, duration, exit code, log digest, environment, observations
```

The exit code is 0 for PASS and 1 for FAIL.

## Boundaries

- Depends on: `content_digest` (SHA-256), `serde`, `serde_json`, `time_source`.
- Used by: `procedure_runner/recording.rs`, which opens and finishes a recording; the three
  procedure modules, which build the cases, environment entries, observation records and
  measurements.
- Rules: no environment key that names a secret is recorded; only the verdict line of a passing
  run carries the success marker.
