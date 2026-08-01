# Mission Creator — editor UI/UX work, handoff

**Written 2026-08-01, at the end of the platform-factory run.** This is the kickoff for a fresh
chat that works on the *editor* rather than the platform. Read this, then the authority docs it
points at. Do not try to reconstruct the previous session — the repo is the handoff.

---

## What this work is

Four asks, in the operator's words:

1. **Eden parity** — "add all the things to the editor that should be there", against the parity
   document listing what Arma 3's Eden editor has.
2. **Clean up the UI** — broad, aesthetic and flow-level. This is design work, not bug-fixing.
3. **Make Line of Sight actually work.**
4. **Make the Ruler actually work.**

Plus, explicitly *not urgent*: rocks are missing from the map render ("doesn't matter if they're
rocks, that's fine").

---

## The authority documents

`docs/specs/Mission_Creator_Architecture/`

| File | Lines | What it is |
|---|---|---|
| [`eden/gap_analysis.md`](../specs/Mission_Creator_Architecture/eden/gap_analysis.md) | 136 | **The parity table.** 87 rows, `eden_id → tbd_id`, a `parity` column (`match` / `partial` / `missing` / `deferred` / `na` / `tbd_only`) and a ticket column. **32 rows read `missing`.** Start here. |
| [`eden/ui_anatomy.md`](../specs/Mission_Creator_Architecture/eden/ui_anatomy.md) | 273 | What Eden's screen is actually made of |
| [`eden/interactions.md`](../specs/Mission_Creator_Architecture/eden/interactions.md) | 560 | Eden's input model — clicks, modifiers, drags |
| [`eden/attributes.md`](../specs/Mission_Creator_Architecture/eden/attributes.md) | 250 | Eden's per-entity attribute catalogue |
| [`feature_inventory.md`](../specs/Mission_Creator_Architecture/feature_inventory.md) | 1797 | TBD's own inventory, with the `RIGHT-*` / `PLACE-*` / `SEL-*` ids the gap table joins on |

The numbered files (`07_…`, `08_…`) at the parent level are **stubs** that redirect into `eden/`.
Do not edit those.

---

## What I measured about each ask, so you do not re-derive it

### Ruler and Line of Sight

**Both are disabled placeholder buttons, not broken features.**
`apps/website/frontend/src/eden_chrome.rs:3711-3717`:

```rust
<button type="button" class=TOOL_DISABLED disabled=true title="Ruler (soon)">
```

`eden_chrome.rs:71` calls them what they are: *"active (Select) vs disabled stub (Ruler / LoS)."*
There is no half-finished implementation to repair — this is greenfield.

Note they are **not rows in the parity table**. Eden has no ruler; these are TBD additions for a
2D top-down planner, so their design is yours to decide rather than something to match. Line of
sight in particular needs a decision the parity docs cannot answer: it is a *terrain* query, and
the DEM is already loaded (`crates/map-engine-core`, 6400×6400 uint16, ±0.204 m verified against
11 survey anchors at T-091.0). A viewshed is achievable; what it should *look* like is a design
question.

### Rocks

**The data already ships.** `packages/map-assets/everon/manifest.json` lists `P4_rocks` in
`importPhaseShipped` and carries a `rockLarge` entry in the type inventory. So this is a *render*
gap, not an export gap — the chunks contain rocks and nothing draws them. Cheaper than it sounds.

### UI cleanup

The Aegis design system was applied in a **sweep** (T-011 → T-025) across ~30 pages built at
different times, rather than designed per-flow. Expect drift. There are skills for this work —
`design-critique`, `design-system`, `ux-copy`, `accessibility-review`.

---

## Running the stack

```bash
make db-up        # Postgres :5434
make api          # Axum API :8080 — applies migrations on boot
make leptos-debug # SPA :3000 — FAST rebuilds, use this for UI iteration
```

**Use `leptos-debug`, not `leptos`.** Release takes ~30 s per rebuild, which makes design iteration
miserable. The documented caveat is that **editor FPS in debug is not representative** — judge
layout, spacing, flow and copy on debug; switch to `make leptos` before judging map performance.

**Log in without Discord:** open `http://localhost:8080/api/v1/auth/dev-login?role=admin`
(also `mission_maker`, `leader`, `enlisted`). It mints a real session and redirects into the SPA.

**If you start a long-lived service from an agent, it dies when the agent finishes.** This bit the
previous session twice — an agent reported "API restarted, healthz 200", which was true when
written and false a minute later. Verify liveness yourself after any agent claims to have started
something.

---

## Traps that will cost you time

- **`rg` does not exist.** It is a harness-injected shell function; `bash -c 'command -v rg'` finds
  nothing. Use `grep`. Never bake `rg` into a script — a gate did that and silently reported OK for
  months because `|| true` swallowed `command not found`.
- **`grep` is ugrep 7.5.0 in an agent shell but GNU grep 3.8 inside `bash script.sh`.** They
  disagree on bare `{}` in an ERE, and every API route path contains `{id}`. Test patterns both ways.
- **The API log's status fields carry ANSI escapes**, so `grep 'status=429'` returns zero hits on a
  log containing thousands. Pipe through `sed 's/\x1b\[[0-9;]*m//g'` first. This defeated two
  investigations in the previous session.
- **`/tmp` is a 16 GB tmpfs.** Never put a `CARGO_TARGET_DIR` there — a frontend build will not fit,
  and filling it makes *every* `bash` call return exit 1 with empty output, which looks exactly like
  a wedged harness. Use `/home/Samuel/.cache/…`.
- **`cargo check` can replay a cached PASS over source that does not compile**, and `--quiet` hides
  the line that would reveal it. A sub-2-second `Finished` is a replay; prove liveness by injecting
  a type error and watching it fail.
- **cargo / make / the Arma binary are host binaries** — route through
  `distrobox-host-exec sh -c '...'`.
- **git-lfs is absent.** Prefix git with
  `-c filter.lfs.process= -c filter.lfs.smudge=cat -c filter.lfs.clean=cat -c filter.lfs.required=false`.

---

## What NOT to do

**Do not restart the platform factory.** It stopped after wave 82 with no agent-actionable backlog
left — see [`PLATFORM_FACTORY.md`](PLATFORM_FACTORY.md). The 13 open platform tickets are features
nobody started, two Workbench-only items, and the playtest. Editor UI work is a different program;
run it as ordinary interactive development, not as waves.

**Do not use a source-grep Class-R pin as a test.** Five rounds of the previous session went into
proving those defeatable by dead code, comments and shadow copies. If you need one, use the
fail-closed scrubber at `apps/website/frontend/src/arsenal.rs` (`class_r_scrub`) — but prefer a
behavioural test that runs the code.

**Do not judge the editor's performance on a debug build.**

---

## The one habit worth carrying over

The recurring defect in this codebase has a single shape: **a tool reports success over an input it
never actually examined.** In the last session alone it appeared as a gate step that never ran, a
test greping a string in its own assertion, a launcher that printed SERVER UP and launched nothing,
a health check that passed *only* when the mod was stale, and a loading bar that animated
identically at 1%, 99% and stalled.

That last one is the version that matters here: **it is the same defect pointed at the user.** A
spinner that cannot fail, a preview that shows a move it will not perform, a dialog that promises a
cascade that never runs — all of it is the UI claiming knowledge it does not have. Treat any green
you did not watch fail first as unproven, and treat any UI state you cannot make *wrong* on demand
as undesigned.
