# T-177 — verify log (MC chrome UX + ORBAT dock cutover; + editor-gate root-cause & hardening)

**Branch:** `main` · **Base:** T-176 `a5940fad` · **Executor:** Claude Code

## Result

| Gate | Result |
|------|--------|
| `make ci-local` (full CI mirror, pinned 1.95.0) | **PASS** (exit 0) |
| `make leptos-gates` (18 editor smokes + v-suite + `gate doctor`) | **PASS** — 20/20, 0 fail |
| `cargo test -p website-frontend` (incl. new outliner test) | PASS — 74/74 |
| fmt + clippy (frontend wasm32 + tbd-tools) | clean |
| `gate doctor` (new fail-fast preflight) | OK — chromium/toolchain/RAM ✓, liveness ✓ |

## A/B matrix (the ticket)

| ID | Ask | Done | Verified by |
|----|-----|------|-------------|
| **A1** | YouTube-style tree connectors (Outliner + Asset Browser) | `guide_spans(&[bool])` — ancestor spines trim at last child + rounded elbow into each row; `FlatRow.ancestors` computed in `flatten_visible` (new unit test), threaded through `single_row` + `palette_rows` | `flatten_visible_computes_ancestor_continuation` PASS; manual G-A (operator) |
| **A2** | Grab cursor on placeable assets | new `PALETTE_LEAF` (= ROW + `cursor-grab active:cursor-grabbing`) on palette leaf `<button>`; folders/outliner slots unchanged | build/clippy; manual G-A |
| **A3** | Top menus paint above docks | strip wrapper `z-30`, docks `z-20` (`mission_editor.rs`) | manual G-A |
| **B1** | Remove ORBAT tree from left dock (= T-071.0) | `DockLeft` = Editor Layers only; `orbat` prop dropped; `smokes.rs` a6/o3/o4/o5/v5 re-pointed to the modal | `outliner-palette` + `virtual-outliner` smokes PASS |
| **B2** | Top-strip ORBAT Manager → modal shell (= T-071.0) | `OrbatManagerDialog` (ui::Dialog wrapping the live `virtual_tree` faction→squad→slot); top-strip button; select/dbl-click→Attributes work from the modal | `outliner-palette` o3/o4/o5 PASS |

A1–A3 are visual — no behavioral smoke; operator G-A confirms. B1/B2 verified by the re-pointed smokes.

## The blocker: `make leptos-gates` could not run (root-caused + fixed)

The editor gate wedged 130 s at boot all session. **Root cause (KB-002):** the harness resolved
playwright's `chrome-headless-shell`, whose stubbed Skia font manager FATAL-crashes on per-character
font fallback (`SkFontMgr_FontConfigInterface.cpp:163 "Not implemented"`) — the editor chrome triggers
it, the renderer core-dumps, and the harness saw only a dead-WS `Runtime.evaluate` timeout. Proven **not
my code / not T-177** (clean T-176 dist wedged identically; debug build too; chromium 1223 & 1228 both;
basic CDP + WebGL2 both worked). Decisive evidence: chromium's own stderr showed the Skia FATAL.

**Fix (`cdp.rs`):** `find_chromium` prefers the full `chrome` build (`chrome-linux64/chrome`) over the
shell; `launch` adds `--headless=new`. The full build has the real font backend. (Also fixed a latent
`chrome-linux` vs `chrome-linux64` path bug.)

Fixing it let the suite run past `selfcheck` for the first time, exposing two pre-existing drifts
(neither the wedge, neither T-177) — both fixed with operator approval:
- **fullmap** asserted `landcover_polygons === 36`; **T-176 intentionally removed the 32 m landcover
  wash** (CLAUDE.md T-176; `world_host.rs:336` drops forest-kind → empty lane) → `=== 0` (confirmed).
- **keyboard-settings**: full chrome natively intercepts trusted Ctrl+C/V → driven via JS `KeyboardEvent`.

## Part 2 — never-again hardening (operator-mandated)

So a whole verification lane can't silently break against a floating, undocumented gate again:
- **`gate doctor`** (`tools/tbd-tools/src/doctor.rs`, new) — a prerequisite of `make leptos-gates`:
  validates the resolved chromium + toolchain vs the pins, checks free RAM + orphaned chrome, and runs a
  ~15 s editor liveness probe that FAILS with a diagnosis + a `gdb` hint instead of the 130 s hang.
  Backed by new `cdp.rs` `send_with_timeout` / `evaluate_with_timeout` (short-timeout, server-side in
  lockstep). `make gate-doctor` + wired into `leptos-gates`.
- **Pins:** `tools/tbd-tools/gate-env.json` (chromium build/version + toolchain + limits) + a new **root
  `rust-toolchain.toml`** (1.95.0 + wasm32 — the frontend/harness had been on the floating rustup
  default). `ci.yml`'s three floating `@stable` jobs pinned to 1.95.0.
- **CI coverage:** `.github/workflows/editor-gates.yml` (nightly + dispatch + gate/editor-path PRs) —
  Postgres + Node-free curl-install of the pinned full chrome + `gate doctor` + `leptos-gates`. Closes
  the ci.yml "run locally per stream" gap. *(New workflow — validate on a first `workflow_dispatch`.)*
- **Docs/rule:** `docs/website/EDITOR_GATE_RUNBOOK.md`, `docs/platform/known-bugs/KB-002-editor-gate-boot-wedge.md`,
  `.cursor/rules/acceptance-gates-reproducible.mdc` (alwaysApply).

### Doc-write override (quoted per `no-silent-deferrals.mdc`)

CLAUDE.md assigns doc writes to Cursor and the T-177 handoff banned editing docs/registry. The operator
**directly overrode** this for the hardening docs/rule: *"Additionally, make sure that the issues that
are happening now never happen again. Add rules, add workflows and like the Claude MD, so what's
happening now doesn't happen again … what's happening now shouldn't be happening at all."* The new
runbook / KB-002 / `.mdc` are authored under that authorization. Pointer lines for the **already-dirty**
CLAUDE.md + DEV_RUNBOOK.md are left to Cursor (see doc list) to avoid entangling this commit with
Cursor's uncommitted doc pass.

## Commit hygiene

Committed **only** my files (explicit `git add`): `eden_chrome.rs`, `outliner.rs`, `mission_editor.rs`,
`cdp.rs`, `smokes.rs`, `gate.rs`, `lib.rs`, `doctor.rs`, `gate-env.json`, `Makefile`, `rust-toolchain.toml`,
`ci.yml`, `editor-gates.yml`, `EDITOR_GATE_RUNBOOK.md`, `KB-002…md`, `acceptance-gates-reproducible.mdc`,
this log. Cursor's pre-existing dirty docs/registry/artifacts left untouched.

## Remaining for T-071.1+ (NOT this slice — per the program ladder)

T-071.0 (modal shell + left → Editor Layers only) shipped via T-177. **T-071.1** squad CRUD + move slot
between squads · **T-071.2** slot numbering/order in export · **T-071.3–.4** logos / standardizations /
per-slot Arsenal link — remain on the T-071 program. The ORBAT Manager modal is a browse/select shell;
squad management is stubbed (a note in the modal points to T-071.1).

## Cursor doc list

- **CLAUDE.md** §Status: T-177 shipped bullet (chrome UX A1–A3 + ORBAT cutover T-071.0 + editor-gate
  root-cause/hardening); a §Run-it pointer to `EDITOR_GATE_RUNBOOK.md`; note the root
  `rust-toolchain.toml` pin.
- **`docs/website/DEV_RUNBOOK.md`** §Notes: add the `EDITOR_GATE_RUNBOOK.md` pointer on the
  `make leptos-gates` line (I reverted my edit to avoid entangling Cursor's dirty copy).
- **`t071_orbat_manager_program.md`**: mark T-071.0 shipped via T-177; T-071.1+ remain ready/queued.
- **`.ai/tickets/registry.json`**: T-177 status + T-071.0 note (`./scripts/ticket sync`).
- MC ROADMAP / `mission-editor.md`: YouTube connectors + ORBAT Manager entry point.
