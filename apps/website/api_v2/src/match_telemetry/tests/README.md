# Match Telemetry Tests (`match_telemetry/tests/`)

Sibling unit and performance benchmark test specifications for telemetry buffering, tick ingest validation, and replay stream serialization.

---

## 1. Test Modules

Declared via Monorepo Law #7 (`#[cfg(test)] #[path = "tests/<file>.rs"] mod tests;`).

### `tick_ingest.rs`
- **Coverage**: Batch payload parsing, coordinate clamping, timestamp monotonicity checks, and authentication secret rejection.

### `combat_events.rs`
- **Coverage**: Combat event schema validation, handling null instigators (environmental/suicide deaths), and event broadcaster emission.

### `session_tokens.rs`
- **Coverage**: Ephemeral token generation entropy, TTL expiration enforcement, single-use invalidation, and slot reservation linking.

### `aar_replay.rs`
- **Coverage**: Range header handling, missing session 404 responses, and stream chunk framing.

### `tick_batcher.rs`
- **Coverage**: Ring buffer overflow strategies, flush latency timers, and graceful teardown drain.

### `replay_indexer.rs`
- **Coverage**: Keyframe reconstruction fidelity, delta decompression integrity, and compression ratio benchmarks.
