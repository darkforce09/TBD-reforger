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

Waves are labelled **100–126** because the platform factory owns 0–99 in the same file. Column 1
must stay a bare integer.

## The shape

| | |
|---|---|
| Program tickets | **77 actionable** — 76 `claude-code`, 1 `human` |
| In the waves | **77**, across **27 waves**, 3 agents each |
| Not in the waves | T-146 (needs `owns`) · T-170 (`executor: human`) |
| Wave rows | 77, **0 `owns` collisions**, 0 waves over 3 agents |

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

**Will also do the mod half.** An earlier draft of this document said 17 tickets were
`executor: workbench` and undispatchable. **That was wrong**, and the operator corrected it.
`scripts/mod/compile.sh` compiles `tbd-framework` against the native Linux dedicated server
headlessly — verified 2026-08-02: `OK: compiled clean, 5707 files, 11182 classes, 832 ms, no
Workbench`. Editing a `.c` file is not the same as needing the Workbench GUI. All 16 mod tickets
are factory work; per the model-routing rule their `.c` portion goes to **Fable 5**.

**Waves 20–26 are the mod half.** T-706 widens `mission.schema.json` once for the whole program at
wave 20, then the 16 Enfusion runtime readers land at 21–26. The schema deliberately sits one wave
ahead of its consumers rather than at the start, to keep the contract-ahead-of-consumer window
short — `mission.schema.json:72` already carries a warning about exactly that failure for
`entities[]`. **T-706 must ship a test asserting each new field is currently unread**, so the day a
reader lands the test fails and forces the comment to be removed.

**Genuinely still out of scope**, and never in the research schema: mission diff/versioning,
real-time collaborative editing, a review workflow, and an editor→mod end-to-end test. They were in
the operator's original braindump and no artifact covers them. Everything else in the parity census
is either in a wave or explicitly `na`.

## Open items

1. ~~`__editorCamSet` panic~~ — **RESOLVED 2026-08-02, headless-only artifact.** The operator ran
   `window.__editorCamSet(6400, 6400, 0)` in a real browser: returns `undefined` (it is a void
   function) and the map renders normally at 147 FPS. **The 8 gate smokes in
   `tools/tbd-tools/src/smokes.rs` are sound.** The panic reproduces only under headless
   vulkan — record it in the capture harness, do not file it against the gate. T-641 is unblocked.
2. ~~T-687 loadout inheritance~~ — **CANCELLED 2026-08-02 by operator decision.** The synthesis
   ranked OFCRA's model highly and the operator rejected it: *"I don't really agree with the OFCRA
   loadout inheritance."* Filed as **rejected, not deferred**, so nobody revives it off the
   synthesis ranking without asking again. The arsenal stays as-is; **T-699** (loadout buffer —
   copy from one slot, apply to a selection) is the practical half and survives.
3. **T-146** (Asset Browser Data Wiring) is the one dispatchable `eden` row with no wave row —
   pre-existing; needs an `owns` before it can be picked up. **T-170** (prod default flip) is
   `executor: human` — the operator's own switch-flip, not factory work, listed here only so its
   absence from the plan is not read as an omission.

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
