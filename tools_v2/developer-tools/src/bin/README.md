# Binary Entrypoints (`developer-tools/src/bin`)

> Planned architecture scaffold. Phase one keeps the live Rust module layout; this directory does not yet implement the structure described below.

Houses the 6 dedicated CLI binary entrypoints for `developer-tools`.

In accordance with **Law 7 (File Size Limits)**, each binary in this directory is a thin, declarative CLI argument parser strictly under **250 lines of code** that delegates execution to the corresponding library modules.

---

## Binary Inventory

1. **`browser_test_runner.rs`** (<250 LOC):
   - Replaces `bin/gate.rs`.
   - Entry point for headless Chrome CDP testing: `smoke`, `editor-suite`, `doctor`, `r-auth`, `render-check`.
2. **`screen_capture.rs`** (<150 LOC):
   - Replaces `bin/capture.rs`.
   - Entry point for live editor screenshot capture (`shot`, `zoomsweep`, `crop`).
3. **`enfusion_oracle.rs`** (<250 LOC):
   - Replaces `bin/enf.rs`.
   - Entry point for Enfusion script reverse-engineering (`index`, `carve`, `apidoc`, `citations`).
4. **`mcp_broker.rs`** (<250 LOC):
   - Replaces `bin/mcpd.rs`.
   - Entry point for the persistent AF_UNIX socket broker and mock offline testing stub.
5. **`map_pipeline.rs`** (<250 LOC):
   - Replaces `bin/map.rs`.
   - Entry point for the 2D imagery pipeline (`build-cartographic`, `water`, `stitch-sap-ortho`, `build-unified`).
6. **`world_pipeline.rs`** (<250 LOC):
   - Replaces `bin/world.rs`.
   - Entry point for terrain chunking and world export (`build-objects`, `build-roads`, `validate-exports`, `phase-gate`).
