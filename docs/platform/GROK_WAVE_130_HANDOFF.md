# Editor factory remediation — Grok handoff, waves 130–141

**Paste the block below into a fresh Grok session.** Everything it needs is in the repo; this file
is the entry point. Written 2026-08-08 after waves 127–129 closed under Opus/Fable.

---

Run the TBD-Reforger editor factory remediation program, **waves 130–141 of
`docs/platform/wave_plan.tsv` (35 tickets)**, in plan order, start to finish, in
`/home/Samuel/Projects/TBD-Reforger`.

**Read first, in this order — they are the authority and the process lives in them, not in this
prompt:**
1. `docs/platform/EDITOR_FACTORY_START.md`
2. `.ai/artifacts/editor_factory_run.md` — the **continuation recipe** (§"Continuation recipe"), the
   **z = 0.0 family** section, the **T-742 harness** section, and **forward constraints**
3. `docs/platform/EDITOR_SLICE_BRIEF.md` — the standing HARD RULES block + required report schema
4. `docs/platform/EDITOR_VERIFY_BRIEF.md` — the verifier brief + severity table

## Facts a fresh chat cannot derive

**Model routing.** ALL coder agents AND the adversarial verifier → **Grok**, explicit model on every
dispatch. (Waves 100–129 used Opus coders + Fable 5 verifiers; the operator changed this at wave 130.
Do not resurrect the old routing from older copies of the docs.) Never downgrade a rate-limited
agent — park and resume.

**Close markers.** Derive, never type:
`BASE=$(git rev-list --extended-regexp --grep='^wave [0-9]+ CLOSED' -1 HEAD)`.
Waves 127/128/129 closed as markers 104/105/106. Wave 130 closes as **107**, and so on to **118** at
wave 141. The remediation wave label rides in the free text after the dash.
**No gate has needed `TBD_GATE_BASE_CONFIRM` in this program** (oracle 2 corroborates on its own,
verified live at waves 127–129). **If a gate demands the hatch anyway, STOP AND READ IT — that is a
signal, not a formality.**

**Standing env on every `wave.sh` / `cargo` invocation:**
`CARGO_TARGET_DIR=/home/Samuel/.cache/tbd-target` (**never /tmp** — 16 GB tmpfs) ·
`TBD_WAVE_GENERATION_FLOOR=100`.

**Mod wave A is PARKED mid-barrier — do not touch it.** Six worktrees: T-702, T-212, T-654, T-673,
T-674 committed-and-unmerged, plus T-675 stacked on T-674 with no work in it. Do not reap, drop or
merge them. Preflight WARNs about stale worktrees — expected, not a block. **`wave.sh status` will
report the parked mod half with three slices "READY TO LAND" — NEVER run
`wave.sh land`.** The remediation gates take `TBD_GATE_WAVE=<L>` explicitly.

**Before wave 130:** the stack should already be up (`api` on :8080, `trunk` on :3000, postgres on
:5434). If not: `cargo xtask db up && cargo xtask mk rust-api && cargo xtask mk leptos` — **`cargo` is HOST-only, route it through
`distrobox-host-exec`**. Then `cargo xtask platform preflight` must PASS (worktree warn aside).

**`cargo` is a host binary** (and there is no `make` any more — T-897 deleted the Makefile). This container has no `cc`/`gcc`; the host does. Route
everything through
`distrobox-host-exec sh -c 'cd /home/Samuel/Projects/TBD-Reforger && CARGO_TARGET_DIR=<dir> cargo <cmd>'`.

## NO-DEFERRAL REGIME — operator instruction, overrides EDITOR_VERIFY_BRIEF.md §Severity

**Every finding the wave verifier files gets FIXED in that wave — BLOCKER, MAJOR, MINOR and NIT
alike. Nothing is filed `deferred`.** Waves 127–129 fixed 15 findings this way.

Per wave, after the verifier reports: triage into *fixable* and *reserved* (a genuine operator
decision, or a finding that refutes a slice claim rather than naming a defect). Reserved should be
near-empty. Then dispatch fix agents, re-run the wave gate, and run a **focused re-verify scoped to
the fix commits only**. Loop until the re-verify is clean. If a wave needs more than 3 fix rounds,
quarantine the specific unconverged fix, file it with the full diagnosis, close the wave with the
rest, and keep moving.

**⚠️ SERIALIZE THE FIX PASS.** Slice agents get isolated worktrees; fix agents run directly on
`main` and therefore share ONE working tree. Disjoint *files* is not enough — the crate must compile
as a whole. In wave 129 three concurrent fix agents broke each other's builds and one nearly reported
a sibling's red as its own. **Run fix agents one at a time**, or give each its own `git worktree`.

## THE THREE DEFECT FAMILIES FOUND IN 127–129 — carry these into every brief

**1. `z = None` on an x/y write flattens an authored Z.** `update_slot_position`
(`crates/map-engine-core/src/doc/store.rs:2779-2783`) writes `pz = 0.0`, excused everywhere by a
"DEM re-sampled JS-side" comment. **That sampler does not exist** — `terrainZ` died with the React
deletion. Four sites found and fixed; family proven closed. **Rules:** never fix this in
`map-engine-core` (byte-parity with the JS oracle is load-bearing) — fix at the frontend caller;
reuse `keep_z_rows()` / `slot_z()` (`editor_ops.rs`, `pub(crate)`); read z off `slots_json` (exact
f64), **not** the SoA (f32, omits hidden-layer slots per T-665); **any new
`update_slot_position` / `move_entities_and_vehicles` caller must pass a real z.** Vehicles keeping
their z while slots lose theirs is the reliable tell.
**Still open:** `paste_at_cursor` and `place_composition` create at z = 0.0 — see T-777, reserved for
the operator alongside T-743.

**2. Hollow source pins.** A bare `include_str!` SRC includes the test module, so every positive
needle matches the assertion searching for it — delete the production code and the pin stays green.
Worse, a needle named in the file's own **comments** is green off the prose alone. Use
`class_r_scrub`'s `live_code` (literals blanked) / `live_source` (literals kept); both cut the test
module first. **Never scope a NEGATIVE pin** — for "must NOT appear", the widest unscrubbed haystack
is strongest. `#[cfg(target_arch = "wasm32")]` code is never compiled natively, so there the source
pin is the ONLY guarantee. **Acceptance test: delete the production code and watch the pin go RED.**
Directly relevant to T-736, T-755, T-776 (w134), T-751 (w135), T-763 (w136), T-738 (w141).

**3. Stale `thread_local` hooks — the most infectious of the three.** A seam registered at mount and
never unregistered stays callable after Backspace hide-chrome unmounts the panel (`mission_editor.rs`,
**no modal guard**, and dialogs explicitly survive the hide). It returns `true` while every `set`
writes to DISPOSED signals — a silent no-op in `reactive_graph` 0.2.14 — so a click reports success
and selects nothing.

**Two failure modes; a naive fix closes only the first:** (1) no unregister at all → the stale hook
answers forever; (2) an *unconditional* unregister → a REMOUNT that installs before the dead
component's cleanup runs gets clobbered by it.

**Correct idiom** (shipped at `eden_dock_right.rs` `install_select_zone`, commit f6a2b687, and ported
to `validation_panel.rs`'s four seams): `install_*` = register + `on_cleanup` unregister guarded by
**`Rc::ptr_eq`**, so only the losing registration is cleared. `on_cleanup` is `Send + Sync`-bound and
`Rc<dyn Fn>` is `!Send`, so park the Rc in a `StoredValue::new_local` and read it back inside the
cleanup. **Never compare a bare `usize` address** — the old Rc drops on re-register, so a later hook
can land on the freed address and a stale cleanup would wrongly clear it (ABA).

**⚠️ This family was found FIVE times in one wave, and one instance was INTRODUCED by a fix agent
during that same wave** — F1 added `register_route_probe` with no cleanup while F2 was fixing the
identical shape twelve lines away, and F1 then made row clickability depend on that probe. **So:
audit every `thread_local` + register-at-mount seam you touch, and every one you ADD.** `grep -n
'thread_local\|fn register_\|on_cleanup\|ptr_eq'` per file; a file with registrations and zero
`ptr_eq` is the smell.

## THE AFFORDANCE INVARIANT — wave 129's hardest-won rule, and it is not finished

**A row is clickable IFF clicking it does something.** Wave 129 enforced this on FOUR surfaces and
each one was a separate defect: the validation panel, the probe/click pair, aggregated settings, and
dock-left search hits. **Two of those four were created by wave 129's own fixes**, because the same
question — "can this subject be clicked?" — was answered independently in several places.

**The rule for all later waves: ONE decision, ONE place.** The single answer is
`validation_panel::subject_id_routes` — an `Rc::clone` of the very resolver the click runs. Any
surface deciding clickability must call it. **Never re-ask `route_target` directly, never hardcode a
kind list, and never add a fallback "just in case" — that fallback IS the bug.** When nothing is
registered it returns false and the row renders inert; that is the CORRECT answer, not a degradation.

**The two polarities are NOT equally severe. Do not treat them as one rule — the orchestrator did,
and the operator was right to call it out:**
- **affordance says yes, click does nothing → a dead click. MAJOR, FIX IT.** It lies to the operator
  and wastes their action.
- **affordance says no, click would have worked → an un-taken capability. FILE IT, do not fix it
  in-wave.** Nothing breaks, no data is at risk, nobody is misled into acting. Dock-left shipped
  exactly this (telling the operator the router "resolves slots and vehicles only" long after it
  resolved zones and objects) and it was fixed in wave 129 as scope creep — it should have been a
  ticket. The "IFF" framing makes the two sound equivalent; they are not.

**Pins must assert the CORRESPONDENCE, two-directionally, over every kind — never a kind list.**
The dock-left pin was green precisely because it hardcoded the same stale list it was meant to guard.
Every correspondence pin must also carry a non-vacuity assert proving it saw BOTH live and inert rows.

## THE HARNESS LIES — this is the single most important operational fact

The shared `CARGO_TARGET_DIR` lets concurrent worktrees execute each other's test binary (T-742).
**Six sightings across waves 127–128: three FALSE PASSES, one false fail, one green pin over a
phrase that could not possibly match (which hid two real defects), one phantom "flake".**
**`touch` does NOT prevent it.**

**MANDATORY in every slice/fix/verifier brief:**
- Do ad-hoc verification in a private `CARGO_TARGET_DIR=/home/Samuel/.cache/tbd-target-<TICKET>`
  (**never /tmp**), delete it before reporting.
- Run the mandated slice gate on the SHARED dir as normal.
- **Cross-check the `--list` total against the run total EVERY time and report BOTH.** Disagreement
  means the binary is not yours. Baseline at the end of wave 129: see `cargo test -p website-frontend`
  on a clean tree — do not trust a number quoted from an earlier wave.

## Compensating for single-model verification

The operator has chosen Grok for both coding and verification. The cross-model independence that
caught several defects in 127–129 is therefore gone, so the verifier brief must lean harder on
**executable proof**:
- The verifier must **prove by running**, not by reading — build a rig, run the repro, capture output.
  Waves 117 and 128's best finds came from extracting an engine into a native harness and driving it.
- It must **attack each slice claim by name**, especially anything the slice admitted was untested.
- It must **restore anything it mutates** and leave `main` byte-identical.
- It must close with an explicit **"attacked and FAILED to break"** register, naming the falsification
  attempts made in any category where it found nothing. Vague reassurance is a failed verification.
- **An agent's chat summary is not an artifact.** State slice claims as claims until they are checked.
  Two wave-118 slice claims were refuted exactly this way, and a wave-129 slice claim was confirmed
  only after a repro test.

## Other standing traps

- **Registry and doc line cites DRIFT CONSTANTLY** — five confirmed in three waves (a spec said
  `:4399`, truth was `:4439`; another said `:1031`, truth was `:1368`). **Locate by symbol; treat
  every cite as a hint.**
- **`cargo test -p map-engine-core` without `--all-features` is a VACUOUS PASS** — the `doc` module
  is feature-gated (this is T-747, w139).
- `dem::peaks::tests::everon_peaks_max_above_350` fails with "Invalid PNG signature" **in every
  worktree** — unresolved Git-LFS pointer, `git-lfs` on no PATH. **On `main` it passes.** Expected,
  environmental, not a defect. Do not chase it; do not let it mask a real red.
- The repo **bans committed `.py` files** (`scripts/verify-no-python.sh`, in CI). An inline heredoc
  that creates no file is fine.
- **Stage explicit paths.** `git add <dir>` while agents are writing captures mid-write snapshots.
- Push with `git -c core.hooksPath=/dev/null push origin main` (the LFS pre-push hook dies without
  git-lfs). Verify the range is LFS-free first:
  `git diff --name-only origin/main..HEAD | grep map-assets` must be empty.
- **Watch disk.** ~66 GB free, shared target dir is 57 GB, in-repo gate dirs ~41 GB. Check `df` at
  every wave close; if free < 25 GB, delete stray `tbd-target-T-*` dirs then `target-gate-api`
  (27 GB, rebuilds on demand). Never delete the shared dir mid-run.

## Load-bearing sequencing (all satisfied by plain plan order — do not reorder)

- **T-723 (w130) before T-768 (w140)** — T-768 is the connect gesture's pointer half; building it on
  the un-fixed armed-pointerup path inherits every one of that path's defects.
- **T-732 (w139) before T-770 (w140)** — T-770's receipt cannot measure the document until the write
  path returns an acknowledgement.
- **T-760 (w130) before T-748 (w141)** — both need the same `draw_order.rs` lane and the same
  `mission_history.rs` rebind tail.

## Reserved for the operator — DO NOT invent an answer

- **T-743 + T-777** — paste lands 20 m off (`PASTE_NUDGE`) *and* drops Z. Both are byte-parity with
  the JS oracle; the operator decides them **together**.
- **T-742's approach (w138)** — the ticket's disk figures are WRONG and this re-opens the decision.
  Measured live: shared dir **57 GB** (not ~4), a slice-private *frontend test* dir **2.7 GB** (not
  ~44 — that figure is a full workspace build). Three concurrent private dirs cost ~8 GB. Per-slice
  test dirs are therefore affordable, which the ticket assumed they were not. **Give the operator
  these numbers; do not choose for them.** The orchestrator's prior recommendation was: a HARD RULE
  in the slice brief forbidding bare ad-hoc `cargo test`, plus a sanctioned `wave.sh` test path
  reusing the existing `target-gate-*` + gate-lock + mtime-bump pattern.
- **T-752's approach (w138)** — the operator already decided: **clean the lints and add
  `--all-targets` to `ci-local-leptos`**. Re-measure on a quiet tree first (slice-report counts are
  inflated by T-742; T-633's five plus T-695's `title_id`/`cat_id` are the credible residue).
  **Scope widened during this run:** `scripts/platform/wave.sh:912`'s `clippy_changed`
  website-frontend arm also omits `--all-targets`, while that function's own header calls it "the
  load-bearing flag" — a THIRD blind spot beyond the two the ticket names. Note T-742 owns `wave.sh`
  in the same wave; coordinate rather than reaching outside owns.
- **The player-cap question** from T-694 — slot count vs filled players vs the 128 server cap.

## Rule-7 carve-outs (HARD RULE 7 says "no doc/ticket/plan edits")

Five tickets own a non-code file as their deliverable. State the exception in those dispatches;
everywhere else rule 7 stands:
`T-767` → `packages/tbd-schema/schema/mission.schema.json` ·
`T-742` → `docs/platform/EDITOR_SLICE_BRIEF.md` and `scripts/platform/wave.sh` ·
`T-752` → `Makefile` · `T-739` → `docs/specs/Mission_Creator_Architecture/eden/gap_analysis.md` ·
`T-747` → `docs/platform/EDITOR_FACTORY_START.md`.

**T-742 edits `wave.sh` — the gate script itself.** After merging it, re-run `wave.sh status` before
the wave-138 gate; a broken gate stops the run.

## When wave 141 closes

Update `.ai/artifacts/editor_factory_run.md`, sync the registry
(`distrobox-host-exec sh -c './scripts/ticket sync'`), push, and **stop with a summary** — shipped,
findings fixed in-wave, anything reserved, and what needs the operator's eyes.
**Do not start the mod half (plan waves 150–155, renumbered from 121–126 on 2026-08-08 — see the run log for why), finalization, or any playtest.**
