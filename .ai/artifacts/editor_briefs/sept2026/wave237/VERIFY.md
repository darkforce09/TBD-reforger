You are the adversarial verifier for wave 237 of the TBD-Reforger factory. Your job is to find what the three slice agents and the command center got WRONG, not to confirm they were right. Do NOT fix. Do NOT commit. Do NOT file tickets. Leave main exactly as you found it (restore anything you mutate; `git status --porcelain` must be empty when you finish). No subagents. Token economy: read only what you attack; ranges for files over 200 lines.

Repo: /run/media/system/Disk_2/Projects/TBD-Reforger, branch main (do not create branches).

ENVIRONMENT. You are in the `claude-desktop` container (glibc 2.36); the shared cache is host-stamped (glibc 2.43), so run EVERY cargo/rustfmt command on the host through the wrappers, which pass cwd, stdin and exit code through:
  /home/Samuel/.cache/tbd-bin/hcargo <args>     /home/Samuel/.cache/tbd-bin/hrustfmt <args>
`podman` is host-only: `distrobox-host-exec podman exec tbd_reforger_db psql -U tbd -d postgres -c '<SQL>'`.
DB-backed tests need `tbd_verify_w237_it`; drop it when done. Never `cargo xtask ci ci-local`. A test printing `skip:` is a FAIL. `map-engine-core` is ALWAYS `--all-features`.
KNOWN ENVIRONMENTAL RED, not a finding: in a slice worktree `packages/map-assets` LFS payloads are pointer files, so `dem::peaks::everon_peaks_max_above_350` and ~7 xtask map tests fail with `bad magic [118, 101, 114, 115]` / `Invalid PNG signature`. On MAIN they should pass — if they do not, that IS a finding.

THE SPAN. Base marker `b6a3cfd89` (wave 235 CLOSED). 48 commits. It covers TWO waves' work, because wave 236's marker was disavowed (see below).

Slices landed:
- `87e5663a1` T-300 — `xtask/src/wave/mod.rs` + `platform_preflight.rs`: a `run-main` target dir, a `tbd-built-from` stamp, `wave run`, and a preflight probe.
- `7fbf92e18` T-935.1 — `crates/map-engine-core/src/world/binary/*` + `manifest.rs` + `Cargo.toml`: `ObjectInstancePod`, TBDC/TBDE/TBDB/TBDS headers, seven rkyv archives, `access_checked`.
- `eb779cc48` T-277 — `packages/tbd-schema/rules/prefab-classify.json` (+1361 lines), the rebuilt `prefabs.json.gz` / `type-inventory.json`, and `density_ladder.rs` / `residency.rs`.
Earlier in the span (wave 236's work, already verified once at its own gate): T-305, T-298, T-943.

COMMAND-CENTER CODE IN THE SPAN — attack it exactly as hard, it is unreviewed by anyone else. All T-946:
- `compile_with_cap` + the repack inheriting the lock's width; `carry_emptied` re-seating pending labels.
- `check::require_check_ok_deferring_repack` and `ticket ship --no-repack`.
- `wave --close --tickets`, and its new refusal when the label is still an open wave.
- `base::wave_plan_tickets_at` now reading `[[emptied]]` when `[[waves]]` has no row for the label.
- The disavowal `36f462d3f` of marker `35328a7b1`, written by hand (`git commit --allow-empty` with the revert trailer) because a close marker carries no diff and `git revert` refuses it.
- The T-277 completion commit `d6cd724e4`: the catalogue refresh plus `exact_tree_count` taking a zoom.

Highest-risk claims to attack:
1. T-300. (a) The stamp is written next to the binary — can a stale binary survive a stamp rewrite, or a stamp survive a binary rebuild, so preflight reads green over the wrong pair? (b) `run_lane_refusal` refuses a worktree outright; does any legitimate flow now break? (c) The ticket's requirement 1 says run lanes build into `run-main`; NONE were rerouted (filed as T-959) — confirm the requirement is genuinely unmet on main and that nothing claims otherwise.
2. T-935.1. (a) `access_checked` must validate: prove a bit-flipped relative pointer returns `Err` and does not abort, for at least two different archive types. (b) `ObjectInstancePod` is 32 bytes with `_pad: u8` — is the padding actually zeroed on the wire, and does a non-zero pad round-trip or silently differ? (c) rkyv's pointer width is left at 32-bit for wasm parity: write the same archive on x86_64 and prove a wasm32 build reads the SAME bytes, or say precisely why you cannot. (d) `manifest.rs` gained optional blocks — does the committed everon manifest still parse byte-identically, and does an unknown field anywhere silently vanish?
3. T-277 + the command-center completion. (a) The catalogue was rebuilt by `world reclassify --write`; is it REPRODUCIBLE — run it again and diff, byte for byte. (b) `exact_tree_count` now takes a zoom and gates vegetation on `class_visible`. Find a zoom where the counter and the packer STILL disagree (try the exact boundary 1.5, and negative zooms), and check `exact_tree_count_chunk`, which was NOT given the same gate. (c) The group-0 glyph pin moved 51 → 84 — verify from the artifact, not the comment. (d) 38 prefabs still fall through; confirm they are all non-spatial engine entities as claimed and that none is a real map object.
4. T-946 (command center). (a) `wave_plan_tickets_at` reading `[[emptied]]`: can a stale or hand-edited pending entry now make a BAD marker corroborate that would previously have been refused as silent? That is a weakening if so. (b) The `--tickets` open-label guard: find a label collision it still lets through (an emptied entry whose label equals an open wave's, say). (c) The disavowal: `wave_close_disavowed_in` greps `This reverts commit <full sha>.` in `<full>..HEAD` — confirm `36f462d3f` really disavows `35328a7b1` by that rule, and that no OTHER marker is accidentally disavowed by the same body text. (d) `require_check_ok_deferring_repack` excludes the missing-lock error by string identity — construct a lock path where that identity comparison fails (a different root, a symlink) and the refusal is swallowed again.
5. Bookkeeping. T-300 / T-935.1 / T-277 must each read `status = "shipped"` with `shipped_at` equal to its landing merge (`87e5663a1`, `7fbf92e18`, `eb779cc48`). `cargo xtask ticket check` exit 0. `wave.lock` must show `wave_base = 235`, pending `[[emptied]]` 236 holding exactly those three, and open waves starting at 237 — no label collision.
6. Gate vacuity. The full gate PASSed on this span. Pick the three steps most likely to pass without examining the new code and prove for each whether it really ran over it. `--list` versus run totals for every crate you test. In particular: does any gate step actually execute `map-engine-core --all-features`, and did `clippy` see the new `#[cfg(test)]` modules?

Severity table (also the command center's triage table):
- BLOCKER: main is broken, data at risk, or a gate reported success on code it never examined.
- MAJOR: a shipped ticket does not do what it claims, or can destroy operator-authored work.
- MINOR / NIT: everything else.

Report (≤ 60 lines): findings first, each with severity, file:line, the exact command + verbatim output that proves it, and the fix shape (one line). Then an explicit list of what you attacked and FAILED to break. Then confirm main is untouched (`git status --porcelain` empty, HEAD sha).
