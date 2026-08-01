# Wave 100 adversarial verification — T-661 split + editor-capture port

**VERDICT: FINDINGS — 0 BLOCKER / 0 MAJOR / 5 MINOR / 5 NOTE.** Every load-bearing claim of the
wave holds: the split is a pure move (proven at symbol and line granularity), 36 tests relocated
1:1 with byte-identical assertions, 419/419 native tests pass, the shim is compile-proven complete
on wasm32, all five stubs name the right tickets, the five checked capture-parity behaviours match
the deleted sources, and both language gates are green at HEAD. The 33-vs-0 tbd-tools discrepancy
is two *reporting* defects (a transposed comment + a misread of multi-target cargo output), not a
lost suite — 51 tests run, unchanged across the port.

Verifier: Fable 5, 2026-08-02. Range verified: c2dac546..HEAD (payload = merge 64802289 with
branch commit b2297037, and 43a3f170; 35d714a2 touches only `.ai/`). Working tree during
verification: dirty only with `.ai/artifacts/editor_factory_run.md` (the command center's own
in-progress edit). Merge tree == branch tree (`git diff b2297037 64802289` empty), so the merge
introduced nothing beyond the slice.

---

## How the pure-move claim was tested (A) — method, then result

- **Line multiset:** old file (`git show 64802289^1:…/eden_chrome.rs`, 5,119 lines) vs the ten
  modules concatenated (5,237 lines), normalized (visibility qualifiers stripped, `use` lines /
  blanks / `//!` excluded), sorted, `comm -3`. Every residual line was read and classified. The
  complete residue: 2 `include_str!` retargets (claimed), regrouped multi-line import lists +
  their `};` closers, 9 replicated `#![allow(dead_code)]`, 7 added top-level
  `#[cfg(target_arch = "wasm32")]`, 4 added `#[cfg(test)] mod tests` wrappers + closers, 2
  dropped section-banner comments, 4 added/reworded comments, and the ORBAT re-export doc moved
  into the shim. **No body line missing, added, or altered.**
- **Symbol census:** top-level `fn/struct/enum/const/static/type/impl/trait/thread_local`
  extraction, visibility-normalized: **101 symbols old, 101 new, diff empty** — nothing lost,
  nothing duplicated.
- **cfg gates:** all 15 old top-level item gates map 1:1 to new item gates; the 7 added gates are
  all on `use` statements importing wasm-gated symbols (eden_settings ×2, eden_top_strip ×1,
  eden_vehicles_panel ×2, eden_zones ×2) — required mechanics, no item gained or lost a gate, so
  neither the wasm artifact nor the native test surface changed.

## Test census (B)

Old `mod tests`: 36 `#[test]`. New: eden_zones 8, eden_env 10, eden_top_strip 12,
eden_vehicles_panel 1, eden_dock_right 5 = **36, names identical** (sorted-name diff empty).
Assertion bodies byte-identical per the line multiset, except the two claimed `include_str!`
retargets. Suite run:
`cargo test -p website-frontend --quiet` → **`419 passed; 0 failed` in 5.60 s** (exact match to
the claim). wasm32: `cargo clippy -p website-frontend --target wasm32-unknown-unknown` rc=0.

## The 33-vs-0 discrepancy (C) — sourced

Measured now: `cargo test -p tbd-tools` runs **51** (lib target; the 6 bins have 0);
`cargo test -p xtask` runs **40**. Derivations: tbd-tools src has 48 `#[test]` + 3
`#[tokio::test]` (all three in `serve.rs`, added by T-361 — the "three serve tests" the comment
itself cites) = 51. xtask src has 42 `#[test]`, 2 of which don't compile into the harness → 40.
At the 2026-07-31 measurement commits (7b3630fa/d9c666e1): tbd-tools src = 48, xtask src = **35**
→ 33 running. Since then xtask gained exactly +4 (2f4244b6, T-611) and +3 (0104cf34, T-623):
33 + 7 = 40 ✓. And T-597's original "81" = 48 tbd-tools + 33 xtask ✓. Conclusion: **the gate
comment's figures are real but transposed** (F-2), and the port agent's "0 tests" is a misread
(F-1). The port itself changed no test code: 48 `#[test]` in tbd-tools src at 43a3f170^ and at
HEAD.

## Capture parity (D) — five claims vs `git show 43a3f170^:tools/editor-capture/*`

| Claim | Old source | Port | Verdict |
|---|---|---|---|
| canvas-blank guard | `buf.length > 20000` (cdp2.mjs:124) | `CANVAS_MIN_BYTES = 20_000`, strict `>` (capture.rs:44,240) | ✓ exact |
| fallback chain | png fromSurface:true → png fromSurface:false → jpeg q80 (cdp2.mjs:134-136) | identical params, order, labels (capture.rs:350-370) | ✓ exact (timeout regime differs — F-3) |
| overlay poll | 25×1 s, selector, `i%10` logs, **dead `i === 89`** (cdp2.mjs:67-77) | 25×1 s, selector **verbatim**, `i%10`, **live `i == 24`** "after 25s" (capture.rs:39-40,160-179) | ✓, dead-branch fix as claimed |
| zoomsweep waits/args | 6000/15000 ms boots, 60 s overlay cap, `__editorCamSet(6400,6400,z)`, 3500 ms settle, `String(z).replace('.','p').replace('-','m')` (first occurrence) | identical; `replacen('.',"p",1).replacen('-',"m",1)` matches JS first-occurrence semantics (capture.rs:392-465) | ✓ (error tap dropped — F-4) |
| KB-002 XDG_CACHE_HOME | `export XDG_CACHE_HOME=…` in run_shot_gpu.sh:21 | `.env("XDG_CACHE_HOME", font_cache)` on the chromium child in the shared launch body (cdp.rs:323); `launch_with_gpu` uses it | ✓ (path differs — writable-cache semantic preserved) |

Also verified: `GpuBackend::Vulkan` flags verbatim from run_shot_gpu.sh's vulkan branch; the three
existing `cdp::launch` callers unchanged (launch delegates to `launch_with_gpu(_, Swiftshader, _)`);
the `__editorCamSet` panic carried as fn doc + README §caveat + existing artifact
`.ai/artifacts/parity/camset_panic_finding.md`, not filed — as claimed. `crop` = crop_imm +
resize_exact(Nearest) = ffmpeg `crop`+`scale=flags=neighbor`; >190,000 px warning preserved.

## Shim completeness (E)

Consumer census (grep `crate::eden_chrome::` outside eden_*): mission_editor (4 consts + 6
components), select_tool (4 consts), editor_ops (5 zone names, editor_ops.rs:2287-2289). All 15
paths are re-exported by the 46-line shim. The merge's diff on all three consumers is **empty**.
Compile-proof: wasm32 clippy rc=0 (the native 419 don't compile editor_ops/select_tool — the wasm
build is the binding check, and it passes).

## Stubs (F)

All five one-line stubs name the claimed filling tickets, and main.rs's comments agree:
context_menu→T-664, ruler_tool→T-642, los_tool→T-643, place_helpers→T-645,
validation_panel→T-655. Registry cross-check: each ticket's summary names that exact file and
says "wave 0 pre-declares the stub"; all five tickets sit in the run-log wave plan (102, 108,
109, 111×2).

## Gates and hygiene (G)

`verify no-node` OK (zero tracked .mjs/.cjs; tools/editor-capture holds only README.md).
`verify no-shell` OK — "59 shell scripts, none new" (58 by `.sh` extension + shebang census;
the commit message's 59 is the gate's own figure). `cargo clippy -p tbd-tools -- -D warnings`
rc=0. `cargo fmt -p tbd-tools -p website-frontend --check` rc=0. No conflict markers or TODOs in
any wave file.

---

## F-1 · MINOR — the port's "0 tests" figure is false; 51 run, none lost

**Evidence:** measured `cargo test -p tbd-tools --quiet` → `running 51 tests … 51 passed`,
followed by seven `running 0 tests` sections (6 test-less bins + doctests). 48 `#[test]` + 3
`#[tokio::test]` in src, identical at 43a3f170^ and HEAD.
**Impact:** the wave's handoff reported a number that reads as "the port lost the suite"; anyone
acting on it would chase a regression that does not exist. Mechanism: piping cargo's multi-target
output through `tail -N` shows only the final 0-test bin sections — the lib's 51 scroll past.
**Disposition:** record-only — correct the wave report; no tree change. When quoting cargo test,
quote the lib target's `test result:` line, not the tail.

## F-2 · MINOR (pre-existing, outside range) — wave.sh's gate comment transposes the measured split

**Evidence:** scripts/platform/wave.sh:2753 says "51 xtask + 33 tbd-tools; measured 2026-07-31".
Sourced above (§C): the true 2026-07-31 split was **51 tbd-tools + 33 xtask**; today it is 51/40.
The comment's own corroboration ("T-361's three serve tests") points at tbd-tools — serve.rs is a
tbd-tools module.
**Impact:** none on gate behaviour (the command under it is correct); it misleads humans doing
exactly the reconciliation this wave attempted, and it triggered question C. Introduced at
d9c666e1 (2026-07-31), before this wave's range.
**Disposition:** deferred-ticket — one-line comment fix ("51 tbd-tools + 33 xtask, now 51+40"),
must not be edited by this verifier.

## F-3 · MINOR — captureScreenshot hang-fallback latency grew 25 s → 130 s per attempt

**Evidence:** cdp2.mjs:104 gave `Page.captureScreenshot` a 25,000 ms rpc timeout — the mechanism
behind "tries fromSurface:false when the surface capture hangs" (cdp2.mjs:2). capture.rs:267 uses
`page.send`, whose default is 130 s (cdp.rs:475-477).
**Impact:** chain order/params are exact parity, but in the documented failure mode (surface path
wedges under headless vulkan) the port waits 5.2× longer per step before falling back; worst case
all-three-hang goes 75 s → 390 s. Output parity preserved; latency parity is not.
**Disposition:** deferred — `shoot()` should call `send_with_timeout(…, Duration::from_secs(25))`;
one-line-per-call fix for a later wave.

## F-4 · MINOR — zoomsweep dropped the per-zoom console-error tap

**Evidence:** zoomsweep.mjs:38-46,80 collected console errors + exceptions and printed up to two
per zoom ("errors since last: …"). capture.rs `zoomsweep` (392-478) attaches no console tap
(`attach_console_capture` is only called from `shot`).
**Impact:** the documented `__editorCamSet` headless panic surfaces as a console exception — the
old tool showed the panic evidence next to each black frame; the ported one prints only
"CAPTURE FAILED"/byte counts, weakening the diagnostic the caveat itself relies on.
Diagnostics-only; capture output unaffected.
**Disposition:** deferred — reuse `attach_console_capture` in `zoomsweep` and print drained
errors per zoom.

## F-5 · MINOR — 657 ported lines, zero new tests

**Evidence:** 43a3f170 adds capture.rs (525 lines) + bin/capture.rs (132 lines); tbd-tools
`#[test]` count is 48 before and after. The commit's "cargo test … clean on tbd-tools" is true
but exercises none of the new code.
**Impact:** the pure fragments (`canvas_path`, the `ztag` formatting whose `replacen(…,1)`
mirrors JS single-replace semantics, crop bounds/px math, step-pair parsing with the 3000 ms
default) encode exactly the parity this verification had to re-derive from deleted sources; a
regression there is currently invisible to `test xtask+tbd-tools`.
**Disposition:** deferred-ticket — a handful of unit tests on the pure fns; no behaviour change.

## N-1 · NOTE — retargeted absence checks now scan one module, not the whole chrome

The two source-inspection tests moved with their needles into eden_dock_right.rs and still bind
(verified: the `begin_place_vehicle(payload.clone())` call :137, `PaletteKind::Object` :139, the
Marker stub prose :642 all live there; "Vehicle placement lands in T-070." and `OBJECTS_COMING…`
absent from all ten modules). But the *absence* assertions guarded 5,119 lines and now guard 827
— a reintroduction in another eden_* module would escape. The tests' own doc says "searches the
file it is written in", so the letter of the design is preserved. Record-only.

## N-2 · NOTE — lint-suppression shape: 1 blanket became 10 + a new `allow(unused_imports)`

Old file had file-wide `#![allow(dead_code)]` (line 17); each of the ten modules replicates it —
same net scope, now ten times harder to retire. The shim adds `#![allow(unused_imports)]`
(justified in-code: native builds cfg out the consumers). Record-only; a per-item
`#[cfg_attr]`-based retirement is future hygiene, not a wave defect.

## N-3 · NOTE — small claim inaccuracies in the wave's handoff

Shim is 46 lines, not 41. Claimed anchors `shot@:280 / zoomsweep@:377 / crop@:508` are actually
capture.rs:299/:392/:487. The functions exist and match their descriptions; the numbers drifted.
Record-only.

## N-4 · NOTE — sh-only launch details not carried (documented or improved)

Dropped chromium flags vs run_shot_gpu.sh: `--disable-dev-shm-usage`, `--no-first-run`,
`--no-default-browser-check`, `--window-size=1920,1080` (superseded by the Emulation override
1920×1080 set before navigation, as cdp2.mjs also did). `GPU_MODE=egl/x11` removed — **explicitly
documented in the README**. Old scripts always `exit 0` even on total failure; the port returns
1 when nothing was written (documented in the bin header) — an improvement, technically a parity
break. crop's CROPDIR/name convention + Arma-screenshots base dir dropped (they hardcoded a dead
session scratchpad). Record-only.

## N-5 · NOTE — stale prose left outside the slice's owns

arsenal.rs:31,42 still cite "eden_chrome.rs:1057/:1066" (now a 46-line shim); eden_zones.rs:931's
assertion message says "update eden_chrome::round_coord" (round_coord lives in eden_zones now);
two section banners (T-215, T-582) were not carried into the new modules. Consumers were
correctly untouched, so this is expected doc rot from a pure move. Record-only — belongs to the
Cursor doc pass, not a code wave.
