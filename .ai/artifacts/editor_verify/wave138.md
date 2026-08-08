# Wave 138 — adversarial verify (T-742 · T-752 · T-739)

**Verified HEAD:** `73429adb708e2baabb9edc4a3a863003a903dd34`  
(`73429adb` T-739 merge tip after T-742/T-752)

**Wave base:** `89bfe0a8` (wave 114 CLOSED — editor wave 137) — ancestor of HEAD.  
**Merges in wave:** `17e51f07`/`b98b8488` (T-742), `efc8f842`/`a32549c6` (T-752),
`c25037f5`/`eff92dc1`/`73429adb` (T-739).

**Gate (orchestrator):** PASS (incl. clippy frontend `--all-targets` post T-742; no `-D`).  
**Method.** HOST cargo only (`~/.cargo/bin/cargo`, rustc 1.95.0). Private targets under
`~/.cache/tbd-target-wave138-{verify,perturb,sharedprobe,base,cleanmain}` (deleted after this
report). Mutations only in detached worktrees at HEAD / base (`~/.cache/tbd-verify138-{perturb,base}`
— deleted). Main checkout left byte-clean at HEAD (`git status` → only pre-existing
`?? tbd-target-T-*` plus this report file; HEAD unchanged). No fix / commit / ticket filing.

**PRIMARY ATTACK.** T-742 isolation (shared-dir collapse / foreign binary / reclaim live-delete /
HARD RULE bypass) first. Then T-739 Class-R hollow pins. Then T-752 residue + Makefile/CI
alignment honesty. Hollow-pin attacks on every new pin.

**NO-DEFERRAL note.** Every severity reported honestly — orchestrator fixes ALL in-wave. No
soft-pedal of hollow pins or found_not_fixed.

---

## Suite reconciliation

| measurement | value | how |
|---|---|---|
| HEAD frontend `--list` | **1006** | `cargo test -p website-frontend -- --list`, private dir |
| HEAD frontend run | **1006 passed / 0 failed** | same private dir; `--list` == run |
| base (`89bfe0a8`) frontend `--list` | **1003** | isolated worktree + private dir |
| Net frontend delta | **+3** | three T-739 pins; T-742/T-752 add no tests |

**New frontend pins this wave**

| test | ticket |
|---|---|
| `arsenal::tests::t739::editor_ops_must_not_reclaim_suppress_on_multi` | T-739 |
| `arsenal::tests::t739::gap_analysis_must_not_reclaim_suppress_on_multi` | T-739 |
| `arsenal::tests::t739::arsenal_cites_live_set_loadout_lines` | T-739 |

1003 + 3 = **1006**. Confirmed. List == run.

---

## FINDINGS (Evidence → Impact → Disposition; NO-DEFERRAL)

### F1 — MAJOR | T-742 `cmd_test` collapse check — **false-refuse when `CARGO_TARGET_DIR` already equals the private dir**

**Evidence.**  
`cmd_test` treats `${CARGO_TARGET_DIR:-$MAIN_ROOT/target}` as the “shared” path and refuses when
`priv_r` equals it (`scripts/platform/wave.sh` ~3486–3493).  
Executable proof:

```text
export CARGO_TARGET_DIR=$HOME/.cache/tbd-target-T-742
bash scripts/platform/wave.sh test --slice T-742 -p website-frontend -- --list
→ rc=2
→ REFUSING — private dir collapsed onto the shared CARGO_TARGET_DIR (.../tbd-target-T-742)
```

Control with `CARGO_TARGET_DIR` unset (script defaults to `$MAIN_ROOT/target`): same command
**accepts** and prints `CARGO_TARGET_DIR=.../tbd-target-T-742 (private — not the shared cache)`.

**Impact.** The sanctioned path self-blocks when an agent has already exported the correct
per-slice private dir (the natural habit from the brief’s `CARGO_TARGET_DIR=…` template). Agents
pushed off `wave.sh test` fall back to bare `cargo test` against the real shared cache — the
exact defect class T-742 exists to stop.

**Disposition — fix this wave.** Collapse-compare only against the true shared roots
(`$HOME/.cache/tbd-target` and `$MAIN_ROOT/target`), never against whatever
`CARGO_TARGET_DIR` currently holds. Allow `priv_r == default per-slice path` even if the env
already points there.

---

### F2 — MAJOR | T-742 `TBD_ADHOC_TARGET_DIR` — **foreign-slice override accepted; prints `rm -rf` of that live dir**

**Evidence.**  

```text
TBD_ADHOC_TARGET_DIR=$HOME/.cache/tbd-target-T-739 \
  bash scripts/platform/wave.sh test --slice T-999 -p website-frontend -- --list
→ ACCEPTS
→ CARGO_TARGET_DIR=.../tbd-target-T-739  (private — not the shared cache)
→ delete before report: rm -rf '.../tbd-target-T-739'
→ began Compiling website-frontend from main into T-739’s cache
```

Collapse check only equality-matches the three “shared” roots; any other existing slice cache is
treated as a fine private dir. Observed side effect: T-739 cache grew **1574 → 1719 MB** from this
verifier’s A6 contamination while T-739 worktree is still live.

**Impact.** Cross-slice artifact pollution (foreign binary class reintroduced between slices). The
banner actively instructs deleting a **sibling live** cache. Matches “can destroy
operator-authored work”.

**Disposition — fix this wave.** Either refuse `TBD_ADHOC_TARGET_DIR` unless it resolves to the
default `$HOME/.cache/tbd-target-$tid` (or a verifier-owned non-`T-*` path), or require the
basename token to match `--slice`. Never print `rm -rf` for a path whose ticket token differs
from `--slice` / is in the live worktree set.

---

### F3 — MAJOR | T-742 defect class still live on shared `CARGO_TARGET_DIR` — **foreign binary executed without rebuild**

**Evidence.** Cross-worktree shared-dir probe (private `~/.cache/tbd-target-wave138-sharedprobe`):

1. Built clean HEAD binary from main into a clean private dir (t739 pins GREEN).  
2. Contaminated shared probe from a detached HEAD worktree with
   `it suppresses on a multi-selection` restored in `editor_ops.rs` → t739 editor_ops pin RED;
   binary hash `2ce34f6f…`; `Compiling website-frontend` from the perturb path.  
3. Main tree confirmed clean (`rg` → no suppress phrase).  
4. From **main**, `CARGO_TARGET_DIR=<sharedprobe> cargo test … editor_ops_must_not_reclaim…`:

```text
Finished `test` profile … in 0.09s          # NO Compiling
Running …/website_frontend-1f1c05fc9a9eab73
test …editor_ops_must_not_reclaim_suppress_on_multi ... FAILED
```

Same artifact path/hash as the perturb build. Main’s fingerprint still said “fresh”; the binary
file had been overwritten by the sibling worktree.

**Impact.** Confirms the T-649/T-742 class is real and still reachable whenever agents skip
`wave.sh test` (or hit F1 and fall back). Private per-slice dirs are load-bearing, not advisory.

**Disposition — fix this wave (paired with F1/F2 + brief).** Keep mechanical enforcement; do not
treat “cargo will rebuild across worktrees” as a mitigator — measured false.

---

### F4 — MINOR | T-742 HARD RULE wording — **shared-dir cargo template still first; ban is prose-only**

**Evidence.** `docs/platform/EDITOR_SLICE_BRIEF.md` HARD RULE 3 still leads with:

```text
CARGO_TARGET_DIR=/home/Samuel/.cache/tbd-target cargo <cmd>
```

then FORBIDDEN bare `cargo test` against that shared dir + sanctioned `wave.sh test --slice`.
Nothing except agent discipline blocks
`… CARGO_TARGET_DIR=/home/Samuel/.cache/tbd-target cargo test`.

**Impact.** Wording remains bypassable by following the first template and substituting `test`
(exact highest-risk residual the ticket named). Mechanical path works when used (and when F1 does
not false-refuse).

**Disposition — fix this wave (NO-DEFERRAL).** Split rule 3: shared cache only for
check/clippy/build; test must name `wave.sh test --slice` with no shared-dir template on the same
line. Optional: make the brief’s paste block refuse to mention `cargo test` next to the shared
path at all.

---

### F5 — MAJOR | T-752 Makefile claim — **`ci-local-leptos` does not mirror `ci.yml` (CI still blind to `--all-targets`)**

**Evidence.**  

| surface | frontend clippy |
|---|---|
| `Makefile` `ci-local-leptos` (T-752) | `--target wasm32-unknown-unknown --all-targets` |
| `wave.sh` `clippy frontend` / `clippy_changed` (T-742) | `--all-targets` |
| `.github/workflows/ci.yml:141` | `--target wasm32-unknown-unknown` **only** — no `--all-targets` |

Makefile comment still says “mirrors ci.yml website-frontend; T-752”. Live clippy
`--all-targets` on HEAD: **rc 0**, **111** warnings (advisory; no `-D` — matches ticket). Named
residue gone (`NodeKind` import, orphaned `#[test]` attribute placement, `title_id`/`cat_id`,
`int_plus_one` span bound). CI path still cannot see `#[cfg(test)]` lints.

**Impact.** Local/Makefile + wave gate gained the load-bearing flag; **GitHub CI did not**. Ticket
claim that test-target lints are “no longer invisible” is false for the CI surface the Makefile
says it mirrors. Hollow alignment.

**Disposition — fix this wave.** Add `--all-targets` to `ci.yml` website-frontend clippy (still no
`-D`, warn mode), or stop claiming CI mirror until that lands. Outside prior owns — NO-DEFERRAL
in-wave.

---

### F6 — MINOR | T-739 found_not_fixed — **`eden_top_strip` cite drift still live**

**Evidence.** Outside T-739 owns (ticket called this out). Live today:

| cite in `eden_top_strip.rs` | claimed symbol | actual |
|---|---|---|
| `editor_ops.rs:1055` (lines ~810, ~1623) | `refresh_docks` bump | L1055 is ATTR-MULTI rustdoc; `pub fn refresh_docks` is **L2660** |
| `editor_ops.rs:1735` (line ~1638) | `orbat_add_squad` guard | L1735 is unrelated `zip`/`find` in another fn; `pub fn orbat_add_squad` is **L4249** |

**Impact.** Same class T-739 fixed for arsenal `set_loadout` cites — stale line numbers teach the
wrong place. Does not break runtime; will mislead the next audit.

**Disposition — fix this wave (NO-DEFERRAL).** Update the two `eden_top_strip` cites to live lines
(or switch to symbol-only cites). Optionally extend the T-739 cite pin pattern to these sites.

---

### NIT | T-742 `cmd_test` — **`-p <crate>` required only in prose; any non-empty args accept**

**Evidence.** Empty args → REFUSE rc 2 (“at least `-p <crate>`”). Non-empty without `-p` (e.g.
subdir probe / help-shaped args) accepts. Comments overclaim enforcement.

**Impact.** Unbounded / mis-aimed invocations can still inflate a private dir. Low severity.

**Disposition — fix this wave (nit):** require an explicit `-p` / `--package` among args before
running.

---

### NIT | T-742 reclaim adhoc sweep — **live spare exact-match held; orphans would remove**

**Evidence.** Simulated spare map against current `git worktree list` (`t212 t654 t673 t674 t675
t702 t739 t742 t752`): live `tbd-target-T-739` SPARED; parked orphans
`T-741/745/749/751/757/761/772/776` would REMOVE. Shared `~/.cache/tbd-target` never touched.
Suffix dirs `tbd-target-T-742-extra…` tokenize as `T-742` and spare when live.  
**Failed to break:** reclaim did **not** delete a live slice dir under the current basename
convention. (Not a finding — register only.)

---

## Main safe to build the next wave on?

**no** — not until F1–F3 (T-742 isolation holes) and F5 (CI/`--all-targets` honesty) are fixed;
F4/F6/NIT in-wave under NO-DEFERRAL.  
T-739 production truth + Class-R pins held under hollow attack. T-752 residue cleanup is real on
Makefile/tree. T-742’s sanctioned path helps when it runs, but F1/F2 undermine it and F3 proves the
underlying shared-dir class is still lethal.

---

## Verified-clean register (claims re-proved)

- **T-739 production:** gap_analysis ATTR-OPEN/MULTI state T-649 open-on-multi (not suppress);
  `rotate_selection_to_face` doc no longer claims suppress-on-multi; arsenal production cites
  `editor_ops.rs:2037` / `:2051` matching live `set_loadout` / `after_local_edit`.  
- **T-739 pins:** H2 restore suppress phrase → editor_ops pin RED; H3 gap suppress → gap pin RED;
  H4/H6 stale cites → cite pin RED; H5 gut positive inversion prose → editor_ops pin RED; H7 blank
  line before `set_loadout` → cite pin RED; H8 decoy suppress phrase elsewhere in file → RED
  (file-wide contains — intentional).  
- **T-752 residue:** `NodeKind` import gone; `#[test]` attached to
  `no_row_glyph_carries_an_uncollapsed_line_box`; `title_id`/`cat_id` removed from composition row;
  sat span bound `end - start < SAT_CHUNK_BYTES` (integer-equivalent to old `+ 1 <=`); Makefile has
  `--all-targets`; clippy warn-mode rc 0 with ~111 remaining advisory warnings.  
- **T-742 refuses that held:** missing `--slice` rc 2; empty args rc 2; collapse onto
  `~/.cache/tbd-target` rc 2; symlink→shared rc 2; collapse onto `$MAIN_ROOT/target` rc 2; `/tmp`
  rc 2.  
- Suites: frontend **1006 == 1006** (base 1003, +3).  
- Main: `73429adb`, tracked tree untouched aside from this report.

---

## Attacked and FAILED to break

1. **T-742 collapse onto `~/.cache/tbd-target`** — REFUSE rc 2.  
2. **T-742 symlink collapse** (`tbd-target-T-999-linktest` → shared) — REFUSE rc 2 (`readlink -f`).  
3. **T-742 collapse onto `$MAIN_ROOT/target`** — REFUSE rc 2.  
4. **T-742 `/tmp` private dir** — REFUSE rc 2.  
5. **T-742 missing `--slice` / empty args** — REFUSE rc 2.  
6. **T-742 reclaim live-delete** — live `tbd-target-T-739` spared under current worktree basenames;
   shared cache never selected.  
7. **T-739 suppress phrase restore (H2)** — editor_ops pin RED.  
8. **T-739 gap suppress restore (H3)** — gap pin RED.  
9. **T-739 stale arsenal cite 2037→777 (H4)** — cite pin RED.  
10. **T-739 gut positive “OPENS the modal” prose (H5)** — editor_ops pin RED.  
11. **T-739 stale 2051 cite (H6)** — cite pin RED.  
12. **T-739 line-shift blank before `set_loadout` (H7)** — cite pin RED.  
13. **T-739 decoy suppress phrase elsewhere (H8)** — editor_ops pin RED.  
14. **T-752 named residue return** — absent under `--all-targets` clippy warn scan.  
15. **Frontend list↔run mismatch** — none (1006 == 1006).  
16. **Cross-worktree cargo rebuild when using private dirs** — perturb private dir correctly
    recompiled and went RED on defect (isolation works when dir is truly private).

---

## Safe? / register summary

| question | answer |
|---|---|
| **Main safe to build the next wave on?** | **no** (F1–F3 T-742 isolation; F5 CI flag gap; F4/F6/NIT in-wave) |
| **Attacked-and-FAILED-to-break** | See numbered register above (16 held attacks; F1/F2/F3/F5 are the misses) |

---

## Focused re-verify

**Post-fix HEAD:** `56b1e5b1`  
**Fix commits:** `caa9531b` (T-742 F1/F2/F3/NIT + F4 brief), `d2211dbe` (T-752 F5), `56b1e5b1` (T-739 F6)  
**Gate post-fix:** PASS (orchestrator). `wave.sh status` still runs (open 38/445; wave 121; land-ready T-702/T-212/T-654).  
**Method.** HOST cargo (`~/.cargo/bin/cargo`, rustc 1.95.0). Private probe dirs
`~/.cache/tbd-target-w138-{reverify,sharedprobe}` + detached worktree
`~/.cache/tbd-verify138-reverify` at `56b1e5b1` — deleted after this section. Main left
byte-identical except this append (tracked tree untouched; HEAD unchanged). No fix / commit /
ticket filing. **Skipped:** reclaim register-only NIT (as instructed).

### F1 — MAJOR | false-refuse when `CARGO_TARGET_DIR` already equals private dir → **CLOSED**

**Disposition (original):** collapse-compare only true shared roots; allow `priv_r ==` default
per-slice even if env already points there.

**Re-probe (ACCEPT):**

```text
export CARGO_TARGET_DIR=$HOME/.cache/tbd-target-T-742
bash scripts/platform/wave.sh test --slice T-742 -p website-frontend -- --list
→ rc=0
→ ═══ ad-hoc test T-742 ═══
→ CARGO_TARGET_DIR=.../tbd-target-T-742  (private — not the shared cache)
→ 1006 tests, 0 benchmarks
```

**Verdict.** ACCEPT. Original false-refuse gone.

### F2 — MAJOR | foreign `TBD_ADHOC_TARGET_DIR` accepted + `rm -rf` banner → **CLOSED**

**Disposition (original):** refuse foreign-slice override; never print `rm -rf` for foreign token.

**Re-probe (REFUSE, no rm -rf):**

```text
TBD_ADHOC_TARGET_DIR=$HOME/.cache/tbd-target-T-739 \
  bash scripts/platform/wave.sh test --slice T-999 -p website-frontend -- --list
→ rc=2
→ test: REFUSING — TBD_ADHOC_TARGET_DIR is not the default per-slice path (.../tbd-target-T-739).
→         Foreign-slice token 'T-739' != --slice 'T-999'.
→ PASS: no rm -rf in output
```

**Verdict.** REFUSE. No `rm -rf` / delete banner. Foreign binary path blocked.

### F3 — MAJOR | shared-dir foreign binary without rebuild → **CLOSED** (class still real; mitigated)

**Disposition (original):** keep mechanical enforcement; do not treat “cargo rebuilds across
worktrees” as mitigator — measured false. Paired with F1/F2 + brief.

**Re-probe (original hollow still lethal on shared dir; private restores GREEN):**

1. Main → `CARGO_TARGET_DIR=~/.cache/tbd-target-w138-sharedprobe` build clean HEAD → pin GREEN;
   binary sha256 `74cbb547…`.
2. Detached worktree at HEAD: inject `/// (it suppresses on a multi-selection)` into
   `rotate_selection_to_face` doc → rebuild into **same** sharedprobe →
   `Compiling website-frontend` from worktree path; pin RED; binary sha256 `df94b17d…`.
3. Main tree confirmed clean (`rg` → no suppress phrase).
4. From **main**, same sharedprobe:

```text
Finished `test` profile … in 0.09s          # NO Compiling website-frontend
Running …/website_frontend-1f1c05fc9a9eab73
test …editor_ops_must_not_reclaim_suppress_on_multi ... FAILED
```

Same artifact path/hash as contaminating build (`df94b17d…`).

5. Main + **private** `~/.cache/tbd-target-w138-reverify`:

```text
Compiling website-frontend … (from main)
test …editor_ops_must_not_reclaim_suppress_on_multi ... ok
```

**Verdict.** Defect class still reachable if agents skip `wave.sh test` / use shared
`CARGO_TARGET_DIR`. Mechanical private path + F1/F2/F4 fixes close the sanctioned path. CLOSED
as mitigated (not “cargo rebuilt”).

### F4 — MINOR | brief templates `cargo test` next to shared dir → **CLOSED**

**Disposition (original):** split rule 3; no shared-dir `cargo test` template on one line.

**Re-probe:**

```text
docs/platform/EDITOR_SLICE_BRIEF.md HARD RULE 3:
  L23: CARGO_TARGET_DIR=…/tbd-target cargo check|clippy|build …   # shared OK for non-test
  L24: Ad-hoc TEST → bash scripts/platform/wave.sh test --slice …  # no shared-dir template
  L24 prose names “shared-dir `cargo test`” as the forbidden class (warning, not a paste template)
rg 'CARGO_TARGET_DIR=/home/Samuel/.cache/tbd-target[^-].*cargo test|cargo test.*CARGO_TARGET_DIR=/home/Samuel/.cache/tbd-target[^-]'
→ (none — PASS)
```

**Verdict.** Shared cache is only templated with check|clippy|build. Test line is `wave.sh test`
only.

### F5 — MAJOR | `ci.yml` missing `--all-targets` → **CLOSED**

**Disposition (original):** add `--all-targets` to ci.yml website-frontend clippy (warn, no `-D`).

**Re-probe:**

```text
.github/workflows/ci.yml:141–142:
  # --all-targets: see #[cfg(test)] / native-shell lints (T-752; warn mode, no -D).
  run: cargo clippy -p website-frontend --target wasm32-unknown-unknown --all-targets
Makefile ci-local-leptos: mirrors ci.yml … clippy --all-targets; T-752
```

**Verdict.** CI flag present. Alignment claim no longer hollow.

### F6 — MINOR | `eden_top_strip` stale cites → **CLOSED**

**Disposition (original):** update cites to live lines (or symbol-only).

**Re-probe GREEN (main):**

| cite | claimed | live L2660 / L4249 |
|---|---|---|
| `editor_ops.rs:2660` (×3) | `refresh_docks` | `pub fn refresh_docks() {` |
| `editor_ops.rs:4249` (×1) | `orbat_add_squad` | `pub fn orbat_add_squad(...)` |
| `editor_ops.rs:1055` / `:1735` | — | **absent** on main |

**Re-probe RED (restore stale in detached worktree):** replace `2660→1055` (3×), `4249→1735` (1×):

```text
L1055 (claimed refresh_docks): /* ─── Attributes modal … */     → RED mismatch
L1735 (claimed orbat_add_squad): .zip(targets.iter())           → RED mismatch
L2660 actual: pub fn refresh_docks() {
L4249 actual: pub fn orbat_add_squad(side: String) -> Option<String> {
```

Main unchanged throughout (live cites remain). Restore of stale is detectable by line→symbol
resolve (ATTR-MULTI rustdoc / zip-find ≠ claimed fns).

**Verdict.** Stale cites gone; hollow restore detectable.

### NIT | `-p` / `--package` required only in prose → **CLOSED**

**Disposition (original):** require explicit `-p` / `--package` among args.

**Re-probe (REFUSE):**

```text
bash scripts/platform/wave.sh test --slice T-742
→ rc=2  REFUSING — pass cargo test args (at least -p <crate>).

bash scripts/platform/wave.sh test --slice T-742 --list
→ rc=2  REFUSING — cargo test args must include -p / --package <crate>.

bash scripts/platform/wave.sh test --slice T-742 website-frontend -- --list
→ rc=2  REFUSING — cargo test args must include -p / --package <crate>.
```

**Verdict.** Non-empty args without `-p` refuse. Empty args still refuse.

### Skipped

- Reclaim register-only NIT (operator instruction).

---

## Focused re-verify summary

| finding | post-fix | evidence |
|---|---|---|
| F1 | **CLOSED** | ACCEPT when env already private (rc 0, list 1006) |
| F2 | **CLOSED** | REFUSE foreign TBD_ADHOC; no `rm -rf` |
| F3 | **CLOSED** | shared-dir foreign binary still RED/no-compile; private GREEN |
| F4 | **CLOSED** | no shared-dir + `cargo test` template |
| F5 | **CLOSED** | `ci.yml` has `--all-targets` |
| F6 | **CLOSED** | live cites; restore stale → RED mismatch |
| NIT `-p` | **CLOSED** | no `-p` → REFUSE rc 2 |

**CLOSED count:** 7 / 7 in scope (F1–F6 + `-p` NIT).  
**list/run (F1 ad-hoc):** **1006** listed.  
**Main safe to build the next wave on?** **yes** — focused set closed; reclaim NIT skipped
(register-only). No wave close performed.
