You are the adversarial verifier for wave 240 of the TBD-Reforger factory. Your job is to find what the three slice agents and the command center got WRONG, not to confirm they were right. Do NOT fix. Do NOT commit. Do NOT file tickets. Leave main exactly as you found it (restore anything you mutate; `git status --porcelain` must be empty when you finish). No subagents. Token economy: read only what you attack; ranges for files over 200 lines.

Repo: /run/media/system/Disk_2/Projects/TBD-Reforger, branch main (do not create branches).

ENVIRONMENT. You are in the `claude-desktop` container (glibc 2.36); the shared cache is host-stamped (glibc 2.43), so run EVERY cargo/rustfmt command on the host through the wrappers, which pass cwd, stdin and exit code through:
  /home/Samuel/.cache/tbd-bin/hcargo <args>     /home/Samuel/.cache/tbd-bin/hrustfmt <args>
`podman` is host-only: `distrobox-host-exec podman exec tbd_reforger_db psql -U tbd -d postgres -c '<SQL>'`.
Never `cargo xtask ci ci-local`. A test printing `skip:` is a FAIL. `map-engine-core` is ALWAYS `--all-features`.
KNOWN AND ALREADY FILED — do not re-report: the shared cache can serve a stale/foreign artifact (T-979); `dem::peaks` fails on an LFS-pointer DEM in a worktree but must PASS on main (T-972); `v-suite verify` fails 22/25 on drifted oracles (T-986); frontend wasm clippy has 76 pre-existing errors (T-987); `verify file-length` has 8 pre-existing SIZE-3 violations; the mod compile gate is blocked by three pre-existing non-ASCII WorkbenchGame files (T-946.3).

THE SPAN. Base marker `b0257e946` (wave 239 CLOSED). Landed:
- `ab7f900b8` T-935.6 — `tools/tbd-tools/src/world/roads_emit.rs` (new), `world/mod.rs`, `bin/world.rs`, `crates/map-engine-core/src/world/{roads.rs,store.rs}`: road network to rkyv, with a gzip-vs-rkyv sniff.
- `fe14f8d0f` T-935.9 — `crates/map-engine-core/src/world/water.rs` (new), `world/mod.rs`, `tools/tbd-tools/src/map/water_emit.rs` (new), `map/mod.rs`, `bin/map.rs`, `apps/website/frontend/src/editor/world_assets/{water.rs (new),mod.rs}`: water vectors + a TBDB bathymetry pyramid and a placement-guard mask.
- `8647ad1d6` T-674.1 — `crates/map-engine-core/src/mission/flatten.rs`: slot identity and squad leader on the wire at schema 1.3.

COMMAND-CENTRE work in the span, and attack it as hard as the slices:
- `e5f46e4fc` — BOTH copies of `TBD_MissionValidator.c` (tbd-framework AND tbd-export) now accept `"1.3"`. This was done because T-674.1 emits 1.3 the moment a mission authors a callsign, and the validator rejects any version it cannot name, which parks the server in LOADING. The mod compile gate could NOT run (T-946.3), so this change is UNCOMPILED. Read it as Enfusion script: is the constant declared correctly, is the comparison right, can any other version check still refuse 1.3, and did the two copies stay consistent with each other and with their own ASCII rules?
- `26f7a785a` — `a_mission_with_findings_still_serves_a_schema_valid_document` was re-aimed: the old seed authored a rank and stance that are now representable, so it asserted a loss that no longer happens. The new seed authors an off-ladder rank. Check the replacement still proves what its name claims, and that it did not quietly weaken the boundary.
- `.ai/tickets/T-674.1.toml` gained a hand-written `created_at` taken from the file's first commit. Verify that date against git, and that nothing else in the file changed.

Highest-risk claims to attack:
1. T-935.9's water mask, hardest — a placement guard will call it, so a wrong answer puts a unit in a lake or refuses good ground. (a) The table claims `Unknown` off the map, with `is_water` and `is_known_dry_land` BOTH false, and false when the file is absent. Prove each, including NaN and infinities, and find any input where `is_known_dry_land` returns true without real evidence. (b) The browser Range-fetches a coarse suffix; prove a suffix answers identically to the whole file at the levels it holds, and that a server ignoring Range cannot hand the tab the whole 655 MB. (c) The mip fold is any-water/max-depth, claimed safe because coarse over-reports water. Find a case where it under-reports.
2. T-935.6. (a) The gzip-vs-rkyv sniff: find a buffer that lands in the wrong parser, or a truncated file that is accepted. (b) An unnameable road-class byte must error at BOTH writer and reader — prove neither invents a fallback. (c) The JSON path must be untouched: the committed roads file has 887 segments across five classes.
3. T-674.1. (a) It emits 1.3 only when an identity key actually reaches the wire. Find an input where the version bumps but no key emits, or the reverse. (b) Values that cannot be carried must drop WHOLE, never be trimmed or coerced — try control characters, blanks, and case variants. (c) `leaderSlotId` must name a seat in its own squad; find a dangling one that survives.
4. Command centre. (a) Three tickets shipped with `shipped_at` = `ab7f900b8` / `fe14f8d0f` / `8647ad1d6`; `ticket check` exit 0; each receipt's sha is its slice's landed tip. (b) This wave has NO pending `[[emptied]]` entry — the batch broke mid-way and it will be closed with `--close --tickets`. Confirm that is the true state and that no label collides.
5. Gate vacuity. The gate PASSed with `test api`, `wasm32` and `trunk build` all PASS. Prove each compiled or executed this wave's code. Then pick the two steps most likely to pass without reading it and prove them. `--list` versus run totals for every crate you test.

Severity table (also the command center's triage table):
- BLOCKER: main is broken, data at risk, or a gate reported success on code it never examined.
- MAJOR: a shipped ticket does not do what it claims, or can destroy operator-authored work.
- MINOR / NIT: everything else.

Report (≤ 60 lines): findings first, each with severity, file:line, the exact command + verbatim output that proves it, and the fix shape (one line). Then an explicit list of what you attacked and FAILED to break. Then confirm main is untouched (`git status --porcelain` empty, HEAD sha).
