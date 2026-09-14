# `src/v2` migration log

The frontend is being reorganised, folder by folder, into the domain-driven tree described in
[`README.md`](README.md). Each phase moves a slice of the legacy tree into `src/v2`, decomposes the
files it touches to the size limits, evacuates inline tests into sibling `tests/` folders, and
rewrites the documentation from the code as it stands. **No behaviour changes.**

## Rules every phase follows

- **Move, never copy.** A file exists in exactly one place at any moment; when a legacy folder
  empties, it and its `mod.rs` are deleted.
- **Size limits.** Production files stay under 500 lines, test files under 1000. No allowlist rows
  are ever added.
- **No inline test modules.** A production file declares its tests with
  `#[cfg(test)] #[path = "tests/<file>.rs"] mod tests;`.
- **Fresh documentation.** Legacy comments are input for understanding only; every `//!` header and
  `///` block in `src/v2` is written from the code as it is today.
- **Gates green after every work item:** native `cargo check`, the `wasm32-unknown-unknown` check,
  `cargo test -p website-frontend --bin website-frontend`, and `cargo fmt`.

## Test-count floor

The native suite counted **1449 passed; 0 failed; 0 ignored** before the first move. Every later run
must be at or above that, with nothing newly failing or ignored.

## Phases

| Phase | commit | tests before→after | notes |
|-------|--------|--------------------|-------|
| Phase 1 | `4f67010bd` | 1449 → 1450 | `core/` retired: `api/`, `auth/`, `ui/`, `utils/`, `test_support/`. The OAuth callback page moved to `pages/account/`. One new test: the v2 documentation audit. |
| Phase 1 fix-up | follows `7a2abf8ed` | 1450 → 1450 | A stray `\(\s*\)` pass late in Phase 1 had emptied parentheses inside string literals; the compiler could not see those. Restored 26 weakened guard-test needles (SSE, client and scrubber batteries), rewrote six comment lines left with stripped-reference residue, and gated two wasm-only imports the split had exposed to the native build. |
| Phase 2 | this commit | 1450 → 1450 | `shell/` retired. The account tier ladder moved into `core/auth/role.rs`; the navigation frame became `pages/navigation/` (frame, top bar, sidebar, registry, not-found), and the sign-in page joined `pages/account/`. `app_routes.rs` now holds only the route list. |
| Phase 3A | this commit | 1450 → 1450 | The command centre hub opened: the landing dashboard, the live server panel and the announcement board each became a page folder under `pages/command_center/`. Three legacy page files became eighteen production files (four of them folder declarations) and three test files; the server panel's self-pins now read `pins::server_intel_source()`, and the stream teardown guard in `core/api` reads it too. Warning texts identical to the pre-phase set. |
| Phase 3B | this commit | 1450 → 1450 | The doctrine hub opened: the wiki, the vehicle index and the modpack manifests each became a page folder under `pages/doctrine_and_info/`. Three legacy page files became nineteen production files (four of them folder declarations) and three test files; the wiki's Markdown renderer and the modpack draft type became shards of their own, and both self-pins now read `pins::wiki_source()` and `pins::modpacks_source()`. Panels the folder READMEs named but the pages never had — a vehicle faction filter, an article contents rail, the modpack update and checksum chrome — are recorded as absent rather than invented. Warning texts identical to the pre-phase set. |
| Phase 3C | this commit | 1450 → 1450 | The field-tools hub opened with the mortar calculator, and the account group gained the settings page. Two legacy page files became ten production files (three of them folder declarations) and two test files; the calculator's grid maths, saved fire missions, pickers, coordinate fields and solution card each became a shard of their own, and the settings page's shared case table now includes through `CARGO_MANIFEST_DIR`. Panels the folder README named but the page never had — a propellant charge selector, a dispersion table, MGRS entry, terrain selection — are recorded as absent rather than invented. No new warning texts; the one the calculator's ungated imports used to raise is gone. |
| Phase 3D | this commit | 1450 → 1450 | The mission hub opened: the scenario library, the scenario dossier and the new-mission dialog each became a page folder under `pages/mission_hub/`. Three legacy files — the library among them the largest file of the phase — became thirty production files (four of them folder declarations) and four test files; the payload comparison, the upload guards, the version rail and the armory editor each became a shard of their own, and the library, overview and dialog self-pins now read `pins::mission_library_source()`, `pins::mission_overview_source()` and `pins::create_dialog_source()`. Panels the folder READMEs named but the pages never had — pagination, a map preview, a slot census, author attribution — are recorded as absent rather than invented. Warning texts identical to the pre-phase set, native and wasm. |
| Phase 3E | this commit | 1450 → 1450 | The operations hub opened: the schedule, the operation dossier, the standalone slotting view, the service record and the global ladders each became a page folder under `pages/operations/`, and `pages/public/` was retired. Five legacy page files became twenty-five production files (six of them folder declarations) and four test files; the dossier's slotting selector is now imported by the standalone route and its hub body by the schedule, so each still has one implementation. The two self-pins became `pins::event_schedule_source()`, `pins::event_hub_source()` and `pins::deployments_source()`, the hub golden reads through `fixtures::golden!`, and four shared-case-table includes now resolve through `CARGO_MANIFEST_DIR`. Panels the folder READMEs named but the pages never had — a calendar strip, an after-action archive, a squad roster of the standalone page's own, an AO preview, commander intent, loadout and vehicle rosters — are recorded as absent rather than invented. Warning texts identical to the pre-phase set. |
