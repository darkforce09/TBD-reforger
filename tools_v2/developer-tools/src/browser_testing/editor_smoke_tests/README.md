# Editor Smoke Test Suite (`browser_testing/editor_smoke_tests`)

> Planned architecture scaffold. Phase one keeps the live Rust module layout; this directory does not yet implement the structure described below.

Complete end-to-end regression test suite driving the 2D/3D Mission Creator editor.

Decomposed from the monolithic 4,071-line `smokes.rs` file into 6 focused modules strictly under **450 lines of code** each.

---

## Submodules

- **`harness.rs`** (<400 LOC): Page navigation, auth fixture seeding, and browser console panic listener.
- **`runner.rs`** (<250 LOC): Test runner loop and CLI subcommand dispatch.
- **`core_tests.rs`** (<450 LOC): Foundational editor tests: `selfcheck`, `editor`, `doc`, `cur`, `persist`.
- **`canvas_tests.rs`** (<450 LOC): GPU viewport tests: `fullmap`, `hillshade`, `pan`, `marquee_drag`, `select`.
- **`dock_widget_tests.rs`** (<450 LOC): CAD UI dock tests: `arsenal`, `attributes`, `outliner_palette`, `keyboard_settings`.
- **`mutation_tests.rs`** (<450 LOC): Transactional editing tests: `undo`, `save_export`, `hydrate`, `entrance_motion_rect`.
