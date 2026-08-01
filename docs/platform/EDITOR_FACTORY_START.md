# Editor factory — cold start

**Written 2026-08-02.** Start a **fresh chat** with this file. The planning session that produced
the program was long and heavily compacted; do not try to reconstruct it. The repo is the handoff.

Preflight is **PASS**, the program is filed, and nothing has been dispatched.

---

## Start here

```bash
make db-up && make api && make leptos-debug     # api :8080, spa :3000
bash scripts/platform/preflight.sh              # must say PASS before anything
export CARGO_TARGET_DIR=/home/Samuel/.cache/tbd-target   # NOT /tmp — 16 GB tmpfs, a build will not fit
./scripts/platform/wave.sh status
```

**Wave 100 is `T-661` — split `eden_chrome.rs` into ten modules. It runs alone.** Every later wave
assumes the post-split module names in its `owns`, so this cannot be skipped or reordered.

Waves are labelled **100–121** because the platform factory owns 0–99 in the same file. Column 1
must stay a bare integer.

## The shape

| | |
|---|---|
| Program tickets | **75** — 58 `claude-code`, 16 `workbench`, 1 `cursor-docs` |
| In the waves | **55**, across **22 waves**, 3 agents each |
| Not in the waves | **19** — 17 workbench, 1 docs, 1 stray |
| Wave rows | 60, **0 `owns` collisions**, 0 waves over 3 agents |

**Waves run 3 agents with a barrier** — all three report, all three merge, then the wave gate, then
an adversarial verifier. This overrides `PLATFORM_FACTORY.md` rule 3, which says land each slice as
it goes green; that was traded away deliberately because tokens and commander attention are the
constraint, not wall clock. Slice agents must **not** spawn sub-agents.

**Model routing:** slice/coder agents → **Opus** (`model: "opus"`). Adversarial verifiers → **Fable
5** (`model: "fable"`). Enfusion `.c` work under `apps/mod/tbd-framework/` → **Fable 5** even when
it is a coder slice. Route on the language of the files in `owns`, not the ticket's subject. Pass
the model **explicitly on every dispatch** — never rely on inheritance.

## What the waves will and will not deliver

**Will:** the editor front-end. Eden's layout and state vocabulary, its menus and dialogs,
placement and selection, live validation, the ruler, line of sight, contour refinement, the markers
UI, layers and outliner authoring.

**Will not — and this is the important half.** 17 tickets need `packages/tbd-schema/schema/mission.schema.json`
widened plus Enfusion readers, so they are `executor: workbench` and the factory **cannot** dispatch
them. They include the deepest work in the program:

- **T-212** objectives as typed per-side entities (WOG's `WMT_Task_Point` spine, incl. min/max
  height so a zone is a volume)
- **T-685** zone volume + force counts · **T-687** loadout templates + inheritance (OFCRA's model —
  faction default + per-role deep merge + explicit-null as "remove inherited")
- **T-677** waypoints · **T-678** group AI state · trigger runtime · the full marker field set

Also outside everything, and never in the research schema: mission diff/versioning, real-time
collaborative editing, a review workflow, and an editor→mod end-to-end test. They were in the
operator's original braindump and no artifact covers them.

**So: complete editor front-end, not a complete Mission Creator.** Say so before anyone assumes
otherwise.

## Three open items

1. **`window.__editorCamSet(6400, 6400, 0)` in a real browser.** Headless, this panics the renderer
   (`wgpu webgpu.rs:2697`) and every canvas read afterwards is a 44 KB black rectangle instead of
   ~3.7 MB of map. **8 gate smokes in `tools/tbd-tools/src/smokes.rs` drive the camera with it.** If
   it reproduces in a real browser those smokes have been asserting against a dead engine. If it
   does not, it is a headless artifact — note it and move on. **T-641 sits at wave 3 and cannot be
   properly scoped until this is answered.** See
   [`camset_panic_finding.md`](../../.ai/artifacts/parity/camset_panic_finding.md).
2. **T-687 is invisible to every queue view.** `xtask/src/sync.rs:254-266` gates the mod queue on
   `targets ∋ "mod"`. Left honest rather than mislabelled to force it to appear.
3. **T-146** is the one dispatchable `eden` row with no wave row. Pre-existing. Needs an `owns` or a
   supersede before it can be picked up.

## Where the evidence lives

`.ai/artifacts/` — read the README in each directory first; they carry corrections that no single
file inside them has.

| Path | What |
|---|---|
| `parity/` | The five sweeps: attributes (93 ids), interactions (83), screenshots (374 rows), 3den (245), plus `owns`/wave derivation and a coverage audit |
| `eden_screenshots/` | 8 batch docs over 75 operator screenshots + the reconciliation README |
| `frameworks/` | WOG · FNF v3 · FNF v4 · FNF tooling · OFCRA, 6,264 lines |
| `framework_synthesis.md` | Best-of-breed, five decisions, 16 ranked items — all filed |
| `adversarial/` | Three Fable 5 passes: claims-vs-source, coverage, reasoning |
| `editor_chrome_direction.md` | **Read before any chrome ticket.** Eden's layout, Aegis's colours |
| `../specs/…/eden/gap_analysis.md` | The parity census — 191 rows, all 176 Eden ids, with `build_class` |

## Traps that cost this program time

- **The repo bans `.py` files** (`scripts/verify-no-python.sh`, runs in CI). Keep any scripting in a
  scratchpad. This was tripped once and broke `main` across two commits.
- **`xtask` is a host binary** — `./scripts/ticket check` fails with a GLIBC error inside the
  container. Route through `distrobox-host-exec sh -c 'cd <repo> && …'`.
- **`git add <dir>` while agents are writing into it** captures mid-write snapshots. An 885-line
  snapshot of a 968-line file got committed twice. Stage explicit paths.
- **A grep answering a different question than the one asked** is the recurring defect here, found
  eight times. `grep -w` is a narrowing step, not a verdict — `grep -riw snap` returns 37 apparent
  hits, all `let snap = read_snapshot()`. **Read the matches before reporting a count.**
- **An agent's chat summary is not an artifact.** A quotation, two statistics and a whole ticket
  scope entered the record from summaries that said things their files did not.

## The habit that matters

The recurring defect in this codebase is **a tool reporting success over an input it never actually
examined**. This program found it three more times in the wild: FNF's mission validator runs 14 of
its 27 checks and has not verified that objectives exist for years; WOG's slot tagger has a regex
that may never match; `make verify-t180` stayed green while six authored values were silently
dropped at compile.

Treat any green you did not watch fail first as unproven. Every validation ticket in this program
(T-655–T-660) ships a test that **makes the rule fire**, not one that watches it pass.
