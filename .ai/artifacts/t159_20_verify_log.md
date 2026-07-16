# T-159.20 — Save Version + Export (compile from MissionDoc) — verify log

Ports the React `compile.ts` → `MissionPayload` → `buildVersionBlob` → `POST /missions/:id/versions`
(Save) and the `exportSchema.ts` file download (Export) to the Leptos/Rust-WASM Mission Creator. The
compile is pure Rust in `map-engine-core`; the editor gains a minimal Save/Export strip, an authed
`api_post`, and a `window.__editorCommands` smoke bridge.

- **Worktree** `.ai/artifacts/worktrees/TBD-T-159/` · **branch** `t-159-leptos-ui` · **base** `6c5eaea0` (T-159.19 tip `f444b878`).
- **Executor:** claude-code.
- **Result: PASS.** `cargo test` (6 new) + `cargo check`/`clippy -D warnings` (my code clean) + `trunk build --release` green; new gate `editor-save-export-smoke` PASS; all 7 prior editor smokes PASS; **live Save 201 / dup 409 / invalid 400** against the running backend with the real browser-compiled payload.

## What shipped
- **`crates/map-engine-core/src/mission/compile.rs`** (new, 302 L) — pure `&str`/`Value` compile: `compile_payload` (Save = orbat omitted; Export = `derive_orbat_from_editor` injected), `compile_export` (camelCase `MissionExport` envelope, `exported_at` injected — core never reads the clock), `version_body` (`{semver, editor_notes, payload}`), `terrain_bounds` (everon 12800 / arland 4096, `coords/terrains.ts` mirror). 6 `#[cfg(test)]` Class R units (orbat faction→squad→index order, save-omits-orbat, null-meta defaults, arland bounds, envelope, body).
- **`crates/map-engine-core/src/mission/mod.rs`** (+1) — `pub mod compile;`.
- **`apps/website-leptos/src/mission_commands.rs`** (new, 209 L, wasm-only) — `EditorCtx` `thread_local` (doc/auth/mission-id, set from `on_load`); `export_now` (compile+download), `save_now` (compile+`api_post`, 409/413/401 → status), `download_json` (`Blob`→`createObjectURL`→`<a download>`→`revokeObjectURL`), `register_editor_commands` (`window.__editorCommands` = `compile_save_json`/`compile_export_json`, export path pins `exportedAt`/`missionId`/`version` for determinism).
- **`apps/website-leptos/src/client.rs`** (+52) — `api_post<T>(store, path, Value)`: `Request::post` + Bearer + `.json(&body)`, wrapped in the existing `send_with_refresh` (single-flight 401 refresh + one retry); maps 409/413 via the status. Exported.
- **`apps/website-leptos/src/mission_editor.rs`** (+50) — `save_semver`/`save_status` signals (both targets); `expect_context::<AuthStore>()` + `set_ctx`/`register_editor_commands` in the wasm `on_load`; a top-left glass Save/Export strip (semver input + two buttons + status; command bodies `#[cfg(wasm32)]`).
- **`apps/website-leptos/Cargo.toml`** (+11) — `map-engine-core` gains `"mission"`; web-sys `Blob`/`BlobPropertyBag`/`Url`/`HtmlAnchorElement`/`HtmlElement`/`HtmlInputElement`.
- **`apps/website-leptos/src/main.rs`** (+4) — `#[cfg(target_arch = "wasm32")] mod mission_commands;`.
- **`.ai/artifacts/t159_gates/driver/smoke_save_export_editor.mjs`** (new, 107 L) — the gate.

## Correctness note — Class R/S are SEMANTIC
Parity is on the compiled JSON's **shape + values**, not a byte-diff vs the React blob: the backend
`validate_payload` + `CreateVersionInput` are JSON-order-agnostic, and serde_json's default `Map` is a
BTreeMap → a given doc compiles **byte-deterministically** (sorted keys). The seed doc (8 slots, empty
factions/squads/layers, `meta` null) compiles to the exact golden below; the faction→squad→index orbat
order the seed can't exercise is proven by a hand-built two-faction unit test (`compile.rs`), mirroring
`orbat.rs::derives_from_editor_sorted_by_index`. **Deferral:** Export `orbat[].loadout` stays `""`
(the `derive_orbat_from_editor` contract) — the seed carries no Smart-Forge loadouts, so parity holds;
loadout summaries in export orbat are a later slice (T-068.11 boundary).

### Golden (`/missions/smoke/edit`, seed `SEED_N=8`)
Save payload (`compile_save_json`, **1323 B**): `schemaVersion` integer `1`; `map.terrain "everon"`;
`map.bounds [0,0,12800,12800]`; **no `orbat` key**; `editor.slots.length 8` (`__missionDoc.slot_count()`);
`editor.{factions,squads,editorLayers} []`; `loadouts`/`environment` objects; `objectives`/`vehicles`/`markers`
arrays. Export doc (`compile_export_json`, **1587 B**): `exportFormatVersion 1`; `payload.orbat []`.
Both byte-identical across two calls (Export `exportedAt` pinned in the bridge).

## Live Save proof (running backend @ :8080, dev-login admin)
The backend route is unchanged by this slice, so the real browser-compiled bytes were POSTed to it:
- **POST** the actual `compile_save_json()` output (1323 B, wrapped `{semver:"0.2.0", editor_notes, payload}`) → **201**.
- **Re-POST** same semver → **409** `{"error":"version already exists"}`.
- **Negative control** (`payload.schemaVersion` as a string) → **400** `{"error":"invalid mission payload","details":["/schemaVersion: … is not of type \"integer\""]}` — confirms `validate_payload` truly schema-checks, and that this slice's **integer** `schemaVersion` (which got 201) is what the validator requires.
- Test mission cleaned up (DELETE 204, GET 404) — no dev-DB footprint.
`CreateVersionInput { semver, payload, editor_notes }` == this slice's `version_body` shape (confirmed in `handlers/missions.rs:551`).

## GPU-preview lanes
The compile bridge is registered synchronously in `on_load` (GPU-independent, like `__missionDoc`), so
the new smoke needs no `?force=webgl`. The two GPU-readback smokes keep `?force=webgl` (`selfcheck`,
`marquee-drag`) — unchanged. The Save/Export overlay sits top-left, clear of the canvas-centre region
the pan/select/marquee smokes drive; the editor smokes are bridge/behaviour-based (no DOM-diff).

## Gate results (all exit 0)
**New — `editor-save-export-smoke`:** `pass:true`, `slotCount:8`, all **16** checks true (saveDeterministic,
exportDeterministic, saveParsed, exportParsed, schemaVersionInt, terrainEveron, boundsExact, saveOmitsOrbat,
editorObj, slotsMatchDoc, emptyGraph, objectShapes, arrayShapes, exportFormatVersion, exportOrbatEmpty,
exportWrapsPayload), `panics:[]`.

**Regression — 7 prior editor smokes (exit 0, pass:true):** `editor-smoke` · `editor-selfcheck` (`?force=webgl`) ·
`editor-pan-smoke` · `editor-doc-smoke` · `editor-persist-smoke` · `editor-select-smoke` · `editor-marquee-drag-smoke` (`?force=webgl`).

**Rust units — `cargo test -p map-engine-core --features mission compile`:** 6/6 pass.

## Build
- `cargo test -p map-engine-core --features mission compile` — 6/6.
- `cargo check -p map-engine-core --features doc,mission` — clean; `cargo clippy … -D warnings` — clean.
- `cargo check -p website-leptos --target wasm32-unknown-unknown` — clean.
- `cargo clippy -p website-leptos --target wasm32-unknown-unknown -- -D warnings` — the 4 T-159.20 files (`compile.rs`, `mission_commands.rs`, `client.rs` `api_post`, `mission_editor.rs` additions) are **clean**. 11 pre-existing lints remain in unrelated ported pages (`event_manager`/`leaderboards`/`personnel`) + `mission_editor.rs`'s pre-existing module doc/code — **byte-identical set before vs after this slice** (verified by `git stash -u` diff: same per-file counts event_manager 2 / leaderboards 3 / mission_editor 5 / personnel 1), i.e. rust-1.95.0 toolchain drift (`doc_lazy_continuation` / `manual_pattern_char_comparison` / `type_complexity`), not a T-159.20 regression.
- `cargo check -p reforger-backend` — clean (additive `pub mod compile`; the backend also enables `mission`).
- `cargo check -p website-leptos` (native shell) — compiles (8 pre-existing native dead-code warnings).
- `trunk build --release` — `✅ success`; wasm dist `website-leptos_bg.wasm` = **5,423,269 B**.

## Non-goals held (spec E6)
Full TopCommandStrip / Eden chrome · undo UI · conflict dialog · 360k progress polish · compiled-mod
loadout (`/compiled`, T-068.11) · Comlink worker (sync compile of 8 slots is instant). Export orbat
loadout summaries deferred (above).

## Next
Ready for Cursor → **T-159.21** (Eden chrome / undo — hub will specify). Return: SHA + tag **T-159.20** + this log.
