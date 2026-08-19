# Editor factory — handoff to Cursor (Grok)

**Written 2026-08-11 by the Claude Code command center, on operator instruction.** Claude Code ran
editor waves 200–207 (close markers 122–130). **Cursor now runs the rest.** This file is the whole
handoff: state, loop, mechanics, and the traps that cost this program real time. Read it end to end
before dispatching anything.

You are the **command center**: you dispatch, integrate, gate, verify, and close. **You never
implement** — if you find yourself editing a `.rs`/`.c`/`.css` file under `apps/`, `crates/`,
`packages/` or `tools/` in the main checkout, stop and dispatch a slice agent instead. The files you
may edit yourself: `.ai/tickets/` (ticket TOMLs; `wave.lock` only via `cargo xtask wave repack`),
`docs/**`, and the run log `.ai/artifacts/editor_factory_run.md`.

Process ancestor: [`FACTORY_FOR_CURSOR.md`](FACTORY_FOR_CURSOR.md) (the platform factory's runbook —
worktrees, reject conditions, phase-0 cold start). Where it and this file disagree on procedure,
**this file wins for the editor band.**

---

## 1. Where things stand

| | |
|---|---|
| Last close | `bc627304` — **wave 130 CLOSED — editor wave 207** |
| **Next close marker** | **131** — but DERIVE IT, never assume: `git log --grep='^wave [0-9]\+ CLOSED' --format='%s' -1`, then +1 |
| Waves shipped by Claude Code | 200, 209, 201, 202, 203, 204, 205, 206, 207 (markers 122–130) |
| **Waves queued for you** | **208, 210, 211, 212, 213, 214, 215, 216, 217** — 26 tickets, all in `.ai/tickets/wave.lock` (relabeled 1..N at the T-912.2 cutover), **all verified file-disjoint within each wave** |
| Not dispatchable | **T-825** — the outliner-first design program; it is an operator design session, not a slice. Do not dispatch it. |
| Parked, do not touch | six mod worktrees (T-702, T-212, T-654, T-673, T-674, T-675) — preflight WARNs about them and that warn is expected |

Every ticket's `summary` **is** its brief — written to be pasted verbatim into a slice dispatch.
They carry measured values, mechanism, traps and an `ACCEPTANCE:` clause. Read
`.ai/artifacts/editor_verify/wave2*.md` for what previous verifiers proved and refuted; the
"verified-clean register" in each is the must-not-break list for the files it names.

### The queue, in order

- **208** — T-801 (tether lines follow the drag), T-805 (editor route gating for non-editors). Last
  wave of the original UX band.
- **210** — the **vehicle wave**, mostly operator-found: T-818 (crew/heading/cargo editor moves into
  vehicle Attributes; the right-dock Placed strip dies), T-819 (crewed slots render inside the
  vehicle), T-836 (seeded vehicles can't compile — missing `veh:` aliases).
- **211 — RUN THIS BEFORE THE REST.** T-843 (suite re-green + **`cargo xtask mk leptos-gates` required
  editor pre-close**; wave gate stays chromium-free — see §5), T-842 (the clamp-not-wrap heading
  defect, third instance, on the world lanes), T-826 (markers stop declaring factions — operator
  decision already recorded).
- **212** — T-838 (markers selectable on the map, listed in the outliner, edited in Attributes — the
  first real instalment of the operator's unification rule), T-841 (opaque Type picker), T-827 (chip
  contrast measured live, not on paper).
- **213** — T-830 (outliner density), T-833 (**rotation: relative drag delta + live preview** — the
  operator revised this decision at the wave-207 eye-pass; read the ⟡ section of its summary),
  T-820 (catalog failure cause probe).
- **214** — T-837 (vehicle delete), T-845 (selected vehicles look identical to unselected), T-824
  (placed zones are invisible).
- **215** — T-839 (the floating Select/Ruler/LoS pill finally dies), T-828 (marker captions drift at
  close zoom), T-823 (OBJ readout ignores vehicles).
- **216** — T-816 (Esc clears hint + arm in one press), T-821 (Save prefill never bumps → 409),
  T-822 (outliner dblclick bubbles into the map container).
- **217** — T-817 (grid labels lag on wheel-zoom), T-834 (strip cleanup; absorbs T-835 + T-840),
  T-831 (per-side markers — audit first, the model is already side-scoped).

---

## 2. The loop — one wave per session, the operator's eye gates the next

This is a **band amendment the operator made deliberately** (`EDITOR_FACTORY_START.md` §UX
remediation band): instrumented verification proved insufficient for UI work — the operator's
hands-on pass repeatedly found things scripted probes could not. So:

1. Dispatch **one wave**. 3 slice agents in worktrees, barrier, merge all, gate, one adversarial
   verifier, close.
2. **STOP after the close commit.** Post an **operator eye-pass checklist**: one human-runnable step
   per ticket, derived from its `ACCEPTANCE:` clause, on `cargo xtask mk leptos` (**release**) at 1920×1080,
   plus "anything that feels wrong is a finding."
3. The operator's verdict gates the next wave. Eye-pass findings are **filed as tickets**, never
   fixed ad hoc in the closed wave.

Findings the operator reports are often *design corrections*, not bugs. Record the decision verbatim
in the ticket (the ⟡ convention: `⟡ DECISION RECORDED (operator, <date>)`) and say what it
supersedes. Three of this band's best outcomes came from exactly that.

---

## 3. Mechanics — copy these

```bash
cd /home/Samuel/Projects/TBD-Reforger
export CARGO_TARGET_DIR=/home/Samuel/.cache/tbd-target      # never /tmp (16 GB tmpfs)
```

**Environment.** `cargo xtask db up`; `cargo xtask mk rust-api`; `cargo xtask mk leptos-debug` during a wave (`cargo xtask mk leptos` —
release — only for the operator's eye-pass). Preflight must say PASS:

```bash
cargo xtask platform preflight        # 2 warns are normal: CARGO_TARGET_DIR unset, 6 parked worktrees
```

**Per wave L:**

```bash
cargo xtask slice-collisions                            # the open waves: tickets + their owns
                                                        # (plan = .ai/tickets/wave.lock since T-912.2)
bash scripts/mod/slice-worktree.sh new T-xxx            # one per ticket
# ... dispatch 3 slice agents (see §4) ... barrier: ALL report ...
git merge --no-ff slice/T-xxx -m "T-xxx: <title>"       # each
BASE=$(git rev-list --extended-regexp --grep='^wave [0-9]+ CLOSED' -1 HEAD)
CARGO_TARGET_DIR=/home/Samuel/.cache/tbd-target \
  TBD_GATE_WAVE=L TBD_GATE_BASE_CONFIRM=$BASE cargo xtask platform wave gate
# (the generation-floor env died at T-912.2 — landed generations live in the lock's wave 0)
```

The gate **will** demand `TBD_GATE_BASE_CONFIRM` — markers past 121 have no corroborating plan rows.
That is documented-benign. **Derive the sha in-shell as above; never type one from memory** (the gate
refuses an abbreviated or misremembered sha, and has).

**Close:**

```bash
cargo xtask platform wave verified $(git rev-parse HEAD)   # no commit may land after the verifier ran
# registry: wave tickets -> shipped (with an honest note); verifier findings filed at the next free id
./scripts/ticket sync
# add the ledger row to .ai/artifacts/editor_factory_run.md ; echo L > docs/platform/factory_pack_wave
git add <EXPLICIT PATHS>    # never `git add <dir>` — mid-write snapshots have been committed twice
git commit -m "wave M CLOSED — editor wave L: <one-liner>; GATE PASS n/n"
bash scripts/mod/slice-worktree.sh drop T-xxx                  # each
git diff --name-only origin/main..HEAD | grep map-assets       # must be empty
git -c core.hooksPath=/dev/null push origin main               # plain push dies on the absent-LFS hook
```

**Environment note:** this box runs the repo inside a container but `cargo xtask platform wave`, `cargo`, and
`./scripts/ticket` are **host** binaries. From the container, prefix with
`distrobox-host-exec sh -c 'cd /home/Samuel/Projects/TBD-Reforger && PATH=$HOME/.cargo/bin:$PATH …'`.
Run natively if your shell is already the host — the wave driver detects and says so.

---

## 4. Dispatching a slice

Model: **Opus-class for coders, a different strong model for the adversarial verifier** — the value
of the verifier comes from it not sharing the coder's blind spots. Claude Code used Opus slices +
Fable 5 verifiers all band; keep the *two-model* shape whatever you route it to.

Each brief = **the registry `summary` verbatim** + `OWNS (touch ONLY these files)` from the plan +
the sibling owns (so it knows what it must not touch) + the standing rules from
[`EDITOR_SLICE_BRIEF.md`](EDITOR_SLICE_BRIEF.md) + the report schema. Add, per ticket, whatever
protected machinery lives in its files — the verify reports name it.

**The report schema is not optional.** A missing field is a structural failure:

```
pwd_branch · defect_verified_on_main [{claim, path:line}] · changes [{path,line,why}]
perturbation {red_output VERBATIM, restored_green} · gate_verdict_tail (must end SLICE GATE: PASS)
files_outside_owns [] · found_not_fixed [{path:line, repro}] · deviations [...] · commits [sha]
```

**The verifier** runs once per wave on merged main, documents and never fixes, and reports to
`.ai/artifacts/editor_verify/wave<L>.md`. Brief it with the base sha, the merge shas, **each slice's
highest-risk claim — especially anything it admitted was untested** — the severity table from
[`EDITOR_VERIFY_BRIEF.md`](EDITOR_VERIFY_BRIEF.md), and an instruction to restore anything it
mutates. A verifier that finds nothing is not wasted.

Triage: **BLOCKER** → fix in-wave, the wave does not close. **MAJOR** → fix in-wave if it can lose
authored work or blocks a feature, else file. **MINOR/NIT** → file. When a MAJOR is a small
mechanical fix with the mechanism already diagnosed, fixing it in-wave and running a **focused
re-verify** of just that commit is cheaper than a follow-up wave; that pattern closed four MAJORs
this band.

### The completion-pass pattern — you will need it

Owns boundaries stop a slice from finishing work that genuinely belongs to its ticket. The rule is
**disclose, don't silently defer**: the slice reports the gap in `found_not_fixed` with a precise
recipe, and after the barrier — when the sibling files are free — you dispatch a **completion agent**
on merged main to close it. Used five times this band; wave 207 needed three. Completion agents
**commit checkpoints as they go** (a harness restart destroyed an uncommitted fix pass once).

---

## 5. Required editor pre-close — `cargo xtask mk leptos-gates` (T-843 option b)

**Decision (wave 211, operator-approved):** keep chromium **OUT** of every
`cargo xtask platform wave gate` run. The wave gate stays the cheap Class-R / verify lane.
Editor CDP smokes — including the rect guards `save-dialog-rect` and `entrance-motion-rect` —
run only through **`cargo xtask mk leptos-gates`** (`gate doctor` → `gate editor-suite` →
`gate v-suite verify`).

**Required pre-close for every editor factory wave:** after the barrier merge and the wave gate
PASS, and **before** `cargo xtask platform wave verified` / registry flip / CLOSE commit, run:

```bash
cargo xtask mk leptos-gates   # trunk release → gate doctor → editor-suite (incl. rect smokes) → v-suite
```

A wave that skips this leaves the rect/geometry guards unexecuted — the wave-203/207 dead-guard
class. Do **not** add `gate editor-suite` (or any chromium smoke) to `cargo xtask platform wave gate`
to "fix" that; option (b) is deliberate cost control.

T-843 also re-greened the suite: `cur` pins the Eden ` m` suffix (T-793), `undo` pins `Layers` /
`Locations` + `[aria-label=Factions]` (T-637/T-696), and `virtual-outliner` `v5_orbatWindowed` is
deterministic under gate and verifier harness configs (T-829 absorption).

---

## 6. The traps that cost this program time

**Measure; do not read.** Three times a slice declared a defect "not reproducing" from source and was
wrong — the Save dialog really was off-screen (a `backdrop-filter` ancestor was hijacking
`position:fixed`), and slots really did render no heading (the lane hardcoded yaw 0). A
did-not-reproduce verdict is only as good as its measurement. Equally, one premise genuinely *was*
stale (the composition stamp had already shipped) — so check, don't assume either way.

**Hollow pins.** A test that does `include_str!("thisfile.rs")` and greps for a literal **matches its
own assertion string** and passes forever, including after you delete the production code. Two
shipped this band and were caught. Always scrub the test module out of the haystack
(`class_r_scrub::live_source` / `live_code`), and **prove every new pin by perturbation**: break the
production code, capture the real red, restore, `touch` the file, re-run green. A `git checkout`
restore does not reliably re-trigger a rebuild.

**A pin on classes is not a pin on geometry.** `is_clamped_on_screen_by_construction` was green while
the dialog rendered at y=−22. If the claim is about pixels, the guard must measure pixels.

**The shared target dir lies.** Concurrent worktrees sharing `CARGO_TARGET_DIR` can execute each
other's test binary — six false verdicts in two waves. Ad-hoc tests go through
`cargo xtask platform wave test --slice T-xxx -p <pkg>` (private dir; delete it after), and
**cross-check the `--list` total against the run total every time**.

**`cargo test -p map-engine-core` without `--all-features` is a vacuous pass** — feature-gated modules
never compile. The `--slice` flag isolates the dir but does **not** inject features.

**Column alignment.** When you build parallel arrays (ids, positions, headings, tints), build them in
**one pass over one sorted source**. `vehicle_rows()` is id-sorted; `vehicle_xy_flat()` uses map
iteration order; mixing them gives every vehicle someone else's heading — silent, and worse than
missing data. Same family as the `zs[i]`/`ids[i]` mismatch rule.

**The z = 0.0 family.** `update_slot_position` writes `pz = 0.0` when z is None and x/y are Some.
Never fix it in `map-engine-core` (byte-parity with a JS oracle); fix at the frontend caller, reuse
`keep_z_rows()`/`slot_z()`, and read z from `slots_json` (exact f64), never the SoA.

**Editions differ.** `tools/*` and `crates/map-engine-render` are edition 2024; import sort order
differs from 2021 and the gate's `fmt` step will catch it. Run `rustfmt --edition <the crate's>`.

**Other standing rules.** Never `--repack` the wave plan while this band is live (it renumbers and
drops edges). No `.py` files committed (CI scans for them; scratchpad use is fine). Stage explicit
paths. `cargo xtask ci schema-validate` does not work in worktrees (`xtask schema validate` does). Gate lock
`WAITING` is serialisation, not a hang.

**Headless probing.** Editor probes run through the `tools/tbd-tools` CDP harness (`smokes.rs`,
`cdp.rs`) — playwright chromium, `--headless=new`, SwiftShader, `?force=webgl`. Two known artifacts,
neither a defect: a **second engine boot in one session** crashes under software WebGPU (use a fresh
page/profile per probe), and `__editorCamSet` panics under headless vulkan.

---

## 7. Your stop rule

Stop and hand back to the operator when: a wave closes (always — post the eye-pass checklist), real
data loss is possible, or a decision is the operator's to make rather than yours. **Quarantine, don't
stop**: a second red on the same ticket means revert that slice (`cargo xtask platform wave revert` keeps the branch),
defer its ticket with the full diagnosis, close the wave with the rest, continue.

Do not dispatch the next wave on your own initiative. The operator's eye-pass is the gate, and it has
caught things every instrumented pass missed.
