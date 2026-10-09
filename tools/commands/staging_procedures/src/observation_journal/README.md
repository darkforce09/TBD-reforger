# Staging observation journal

What a recorded run keeps of what it saw: the JSONL journal with every raw artifact stored by its
SHA-256, and the browser inbox where the orchestrator saves page reads from the operator's Chrome.
Both live in the run folder `target/staging/<check>/<run>/`.

## Contents

```text
tools/commands/staging_procedures/src/observation_journal/
├── browser_inbox.rs  `browser_inbox/<step>.json` entries, accepted only inside the step's window
├── journal.rs        `journal.jsonl` lines and `artifacts/<sha256>.txt` files
└── mod.rs            the module tree
```

## How it works

`ObservationJournal::create` makes the run folder and opens `journal.jsonl` create-new, so no two
runs share one. `archive` writes the artifact as `artifacts/<sha256>.txt` (identical bytes share
one file), appends a line with the sequence number, step, observer, time, verdict, summary, digest
and size, and returns the digest the log's `observation:` line cites.

An inbox entry is `{"captured_at_unix_ms": <u64>, "output": <the Chrome tool's raw output>}`. The
runner reads it for a probe whose source is the inbox: absent or captured outside the window from
the step's start to its deadline, it is pending; accepted, its output is judged and the raw file is
archived as a `chrome page read`. An entry that names an authorization header, a bearer, an access
or refresh token, a `"token"` field or a `set-cookie` header is refused and never archived.

## Boundaries

- Depends on: `content_digest`, `serde_json`.
- Used by: `tools/commands/staging_procedures/src/procedure_runner/`.
- Rules: an artifact holds exactly the bytes its name digests; no credential is ever archived.
