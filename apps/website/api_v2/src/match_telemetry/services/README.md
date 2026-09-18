# Match Telemetry Services (`match_telemetry/services/`)

High-throughput buffering pipelines and compression engines supporting live telemetry ingest and spatial replay generation.

---

## 1. Domain Services

### `tick_batcher.rs` (<380 LOC)
- **Purpose**: Decouples incoming HTTP tick requests from database write latency using an asynchronous ring buffer.
- **Key Functions**:
  - `enqueue_ticks(ticks: Vec<TelemetryTick>) -> Result<(), BufferError>`: Appends incoming tick batches to the lockless staging ring.
  - `flush_buffer_to_postgres(pool: &PgPool) -> Result<usize, DbError>`: Dispatches staged ticks into Postgres using bulk `COPY` or parameterized multi-row statements.
- **Invariants**:
  - High-load resilience: Drops non-critical interpolations before blocking HTTP ingest threads if database queue exceeds capacity.

### `replay_indexer.rs` (<420 LOC)
- **Purpose**: Prepares spatial tick archives for lightweight client streaming in the After-Action Review (AAR) player.
- **Key Functions**:
  - `build_keyframes(session_id: Uuid, interval_ms: u64) -> Result<Vec<ReplayKeyframe>, IndexerError>`: Samples full world states at fixed intervals.
  - `encode_delta_stream(session_id: Uuid) -> Result<Vec<u8>, CompressionError>`: Encodes vector displacements between keyframes using compact varint encoding.
