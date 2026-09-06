You are the adversarial verifier for wave 238 of the TBD-Reforger factory. Your job is to find what the three slice agents and the command center got WRONG, not to confirm they were right. Do NOT fix. Do NOT commit. Do NOT file tickets. Leave main exactly as you found it (restore anything you mutate; `git status --porcelain` must be empty when you finish). No subagents. Token economy: read only what you attack; ranges for files over 200 lines.

Repo: /run/media/system/Disk_2/Projects/TBD-Reforger, branch main (do not create branches).

ENVIRONMENT. You are in the `claude-desktop` container (glibc 2.36); the shared cache is host-stamped (glibc 2.43), so run EVERY cargo/rustfmt command on the host through the wrappers, which pass cwd, stdin and exit code through:
  /home/Samuel/.cache/tbd-bin/hcargo <args>     /home/Samuel/.cache/tbd-bin/hrustfmt <args>
`podman` is host-only: `distrobox-host-exec podman exec tbd_reforger_db psql -U tbd -d postgres -c '<SQL>'`.
DB-backed tests need `tbd_verify_w238_it`; drop it when done. Never `cargo xtask ci ci-local`. A test printing `skip:` is a FAIL. `map-engine-core` is ALWAYS `--all-features`.
TWO ENVIRONMENT TRAPS BOTH HIT THIS WAVE — do not let either fool you, and do not report either as new:
 - The shared target dir served two different slices a STALE or FOREIGN artifact (a `map-engine-core` test binary holding 4 of a file's 8 tests; an `unresolved import` with no compile line). Cross-check `--list` counts against `grep -c '#\[test\]'`, and use a private `CARGO_TARGET_DIR` if they disagree. Filed as T-979.
 - `dem::peaks::everon_peaks_max_above_350` fails in a slice worktree because the DEM PNG is an LFS pointer there. ON MAIN it must PASS. Filed as T-972.

THE SPAN. Base marker `ad9b22890` (wave 237 CLOSED). Landed:
- `38e89e47f` T-935.4 — `crates/map-engine-core/src/dem/raw.rs` (new), `dem/mod.rs`, `tools/tbd-tools/src/world/aux.rs`, `apps/website/frontend/src/editor/world_assets/dem_load.rs` (new) + `mod.rs`: a TBDE raw u16 DEM, dual-emitted beside the PNG and streamed into one `Vec<u16>`.
- `efc03d036` T-935.7 — `tools/tbd-tools/src/map/labels_emit.rs` (new) + `map/mod.rs` + `bin/map.rs`, `crates/map-engine-core/src/world/{locations,road_labels,mod}.rs`, `apps/website/frontend/src/editor/world_assets/labels.rs`: town/height/road labels into one rkyv archive.
- `aaa0b86d9` T-935.8 — `xtask/src/map_blueprint/archive_emit.rs` (new) + `library_cli.rs` + `mod.rs`, `crates/map-engine-core/src/{building_blueprint.rs,world/occluder/descriptor.rs}`, `apps/website/frontend/src/editor/world_assets/occluder_host.rs`: a building-blueprint archive and an occluder boot path.
All three inherited commits from predecessors killed by a session limit; the successors were told they own that code.

Highest-risk claims to attack:
1. T-935.8, hardest — it sits under line-of-sight, so a wrong answer is a silently wrong sightline, not a crash. (a) The slice seeds ONLY the 301 non-blocking descriptors and leaves 1322 blocking ones on JSON, because the archived descriptor has no instance records (its own finding, filed T-981). VERIFY THAT IS TRUE: can a blocking prefab reach `insert_descriptor` from the archive by any path — a pid in both sets, a race between the archive seed and a JSON arrival, a fallback that runs after a partial seed? (b) `with_archive_blas` caps at `WANT_PER_PASS`; can a sidecar be dropped permanently rather than deferred? (c) Compare a real LOS trace with the archive branch forced on against the JSON branch.
2. T-935.4. (a) The central claim is ONE allocation. Prove it end to end, not just in `RawDemSink`: count full-grid buffers from fetch to the retained grid, and check the f32 grid the slice admits to (T-978). (b) `Budget` moved after `resp.body()?` — does the boot bar still total correctly when the raw arm falls back mid-stream? (c) Feed a header whose `width * height` overflows a 32-bit `usize` and prove `LengthMismatch`, on wasm32 if you can build it.
3. T-935.7. (a) The archive lane must render IDENTICALLY to JSON. Attack the road-name path: order carries priority and the class byte carries a visibility floor, so check a curated override that no class floor expresses. (b) `map_labels_from_bytes` does an aligned copy — prove the read survives every byte offset and that a truncated or version-bumped buffer errors rather than yielding partial labels. (c) The slice says only TWO label fetches exist, not three; confirm, and confirm the height lane genuinely did not change hands.
4. Command centre. (a) Three tickets shipped with `shipped_at` = `38e89e47f` / `efc03d036` / `aaa0b86d9`; `wave.lock` pending `[[emptied]]` 238 must hold exactly those three with no open-wave label collision. (b) `cargo xtask ticket check` exit 0. (c) The gate receipts under `.ai/artifacts/verdicts/` must match each slice's landed tip — and check whether a receipt can outlive the branch it describes.
5. Gate vacuity. The gate PASSed with `wasm32 (frontend) PASS` and `trunk build PASS` — last wave those were vacuous, and the fix was to derive the scope from the dependency graph. Prove they really compiled this wave's code. Then pick two more steps most likely to pass without reading the new code and prove them. `--list` versus run totals for every crate you test.

Severity table (also the command center's triage table):
- BLOCKER: main is broken, data at risk, or a gate reported success on code it never examined.
- MAJOR: a shipped ticket does not do what it claims, or can destroy operator-authored work.
- MINOR / NIT: everything else.

Report (≤ 60 lines): findings first, each with severity, file:line, the exact command + verbatim output that proves it, and the fix shape (one line). Then an explicit list of what you attacked and FAILED to break. Then confirm main is untouched (`git status --porcelain` empty, HEAD sha).
