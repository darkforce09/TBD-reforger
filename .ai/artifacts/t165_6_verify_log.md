# T-165.6 verify log — smokes → Rust; Node driver deleted

Scope: port the 19 remaining driver scripts to `tbd_tools::smokes`, flip `make leptos-gates`
to cargo, delete the driver. The V-suite core shipped at T-165.5 (25/25 byte-identical).

## Ported (dispositions — all 19 driver scripts accounted for)

| Node script | Rust | Disposition |
|---|---|---|
| selfcheck_editor.mjs | `gate smoke selfcheck` | ported (GPU readback, ?force=webgl) |
| smoke_editor.mjs | `gate smoke editor` | ported (canvas + wheel screenshot-hash) |
| smoke_pan_editor.mjs | `gate smoke pan` | ported (RMB pan + mid-pan wheel rebase) |
| smoke_doc_editor.mjs | `gate smoke doc` | ported (SEED_N/roundtrip/encode-stable) |
| smoke_persist_editor.mjs | `gate smoke persist` | ported (COLD→WARM IDB, semantic digest) |
| smoke_select_editor.mjs | `gate smoke select` | ported (pick selfcheck + 4-click battery) |
| smoke_marquee_drag_editor.mjs | `gate smoke marquee-drag` | ported (?force=webgl, oracle set-eq) |
| smoke_save_export_editor.mjs | `gate smoke save-export` | ported (16 schema-shape checks) |
| smoke_undo_editor.mjs | `gate smoke undo` | ported (21 checks incl. A7 keydown guard) |
| smoke_outliner_palette_editor.mjs | `gate smoke outliner-palette` | ported (15 checks, f32-bit-exact D2) |
| smoke_cur_editor.mjs | `gate smoke cur` | ported (C0–C3 exact-string readouts) |
| smoke_attributes_editor.mjs | `gate smoke attributes` | ported (9 checks) |
| smoke_keyboard_settings_editor.mjs | `gate smoke keyboard-settings` | ported (7 checks) |
| smoke_arsenal_editor.mjs | `gate smoke arsenal` | ported (registry-golden interception, R1–R5) |
| smoke_hillshade_editor.mjs | `gate smoke hillshade` | ported (map-assets passthrough lane) |
| smoke_hydrate_editor.mjs | `gate smoke hydrate` | ported (LIVE backend: dev-login → create → save → hydrate → delete) |
| smoke_mutations.mjs | `gate smoke mutations` | ported (live lane, TOKEN/REFRESH envs) |
| gate_r_auth.mjs | `gate r-auth` | ported (single-flight bootstrap pin) |
| render-check.mjs | `gate render-check` | ported (generic CLI, --assert-js kept) |

`gate editor-suite` runs the 16 editor smokes in the Makefile glob's shell-sort order with the
`set -e` first-failure-stops semantics. Key-chord contract preserved: rawKeyDown+keyUp ONLY
(T-159.22.1); mouse moves carry `button:none` + held `buttons` bits.

## Acceptance (side-by-side, same release dist, live backend on :8080)

- **Editor suite:** Node glob run rc=0, `gate editor-suite` rc=0 — **16/16 gates PASS on
  both**, identical gate-name sets, 18 `"pass": true` markers each.
- **r-auth:** rc=0/0, stdout **byte-identical**.
- **render-check** (`--expect "COMMAND CENTER"`): rc=0/0, stdout byte-identical (mod the
  identical url line).
- **mutations** (fresh dev-login TOKEN/REFRESH per run): rc=0/0, stdout byte-identical.
- **V-suite:** re-run green post-flip via `make leptos-gates` (25/25).
- **Negative probe:** both harnesses against an app-less dist (`index.html` = `<p>empty</p>`)
  → node rc=1 / rust rc=1 (canvas never appears; not vacuously green).
- `cargo test -p tbd-tools` 3/3 — the inject byte-parity test now exercises its
  driver-deleted skip path (the Rust consts are the single source).
- `cargo clippy -p tbd-tools --all-targets -- -D warnings` rc=0 · fmt clean ·
  `xtask schema t090-specs` 12/12 · `./scripts/ticket check` OK.

## Stale gate contracts fixed (identically on BOTH sides, pre-parity-run)

Both auxiliary gates were red on current main — in BOTH harnesses, byte-identically — from
T-159-era contract rot, not port defects:

1. **gate_r_auth**: the mock's catch-all `401 {}` on every non-/me endpoint × the post-.25
   dashboard's boot queries drove a refresh loop (`refreshCount: 326`, flow itself correct —
   `authedUsername` landed). The gate pins the BOOTSTRAP single-flight; catch-all → `200 {}`
   (the /me 401→refresh→retry lane is untouched). Applied to the .mjs AND the Rust port,
   then both green rc=0 byte-identical.
2. **render-check README invocation**: the T-159.1 expect string "TBD Reforger — Leptos"
   predates the real shell (brand renders line-broken `TBD\nReforger`). README now checks
   `--expect "COMMAND CENTER"`.

## Deleted (this slice)

`.ai/artifacts/t159_gates/driver/` — all 24 files (22 .mjs + freeze.js + dom.js) and
`manifests/extract-leptos-routes.mjs`. Tracked non-mod `.mjs`: 66 → 43. `make leptos-gates`
recipe is Node-free; freeze/dom payloads survive as provenance-headed verbatim consts in
`tools/tbd-tools/src/inject.rs`; harness docs updated (`t159_gates/README.md` layout,
`apps/website-leptos/README.md` render proof → cargo).
