**Status:** live

# Capability verdicts

The index of [capability_verdicts.tsv](/documentation_v2/mod/tbd-framework/capability_verdicts.tsv),
the hand-authored table that gives every CRF source file a TBD verdict: build it, replace it, leave
it for later, skip it, or defer it by operator word. Reforger ships no lobby, briefing, slotting,
respawn, spectator or admin tooling, and CRF covers that ground, so a CRF file with no verdict is a
subsystem the [mod](/documentation_v2/glossary/g_to_m.md#mod) could forget; the check turns that into a
build error.

## Format

- File: tab-separated text. Lines starting with `#` are comments: the table's header note and the
  section rules (`# ── spine: the event loop ──…`) that group the rows.
- Columns: `prefix` (a path under `apps/mod/crf_framework/`, a folder or a file), `capability` (the
  name the matrix groups by), `verdict`, `note` (why, or where TBD's version lives).
- Matching: the longest prefix wins, so a rule for one file overrides the rule for its folder;
  prefixes of equal length keep file order.
- Verdicts, the only legal values (`VERDICTS` in
  `tools_v2/developer-tools/src/enfusion_tooling/capability.rs`; anything else fails the load):

| Verdict | Meaning |
|---|---|
| `BUILD` | TBD must implement this |
| `HAVE` | already exists in `tbd-framework` |
| `PARTIAL` | partly covered, tracked elsewhere |
| `REPLACE` | CRF's mechanism swapped for the mission JSON |
| `LATER` | wanted, not on the critical path |
| `SKIP` | deliberately out of scope |
| `DEFERRED` | out of scope by operator word; §6 of the [mod design](/documentation_v2/mod/tbd-framework/mod_design.md) records it |

The table holds 93 rules over 57 capabilities: 37 `BUILD`, 22 `LATER`, 21 `SKIP`, 9 `PARTIAL`,
3 `DEFERRED` and 1 `REPLACE` rule, and no `HAVE` rule. A verdict records the triage judgement,
not build progress: the lobby, briefing, spectator and admin rows still read `BUILD` although
those screens ship.

## Producers and consumers

- Written by hand and reviewed: the verdict is product judgement, the aggregation is mechanical.
  A new CRF file that no prefix covers needs a new row.
- `cargo run -q -p developer-tools --bin enf -- capability` reads the table from
  `CAPABILITY_VERDICTS` (`tools_v2/developer-tools/src/repository_layout.rs`), joins it against
  the CRF index (`crf_files.tsv` and `crf_symbols.tsv` in `.ai/artifacts/enf-index/`), prints one
  row per capability (verdict, files, lines, symbols, note, heaviest first) and writes the same
  matrix to `.ai/artifacts/enf-index/capability_matrix.tsv`. Each matrix row carries the verdict
  and note of the rule that matched the capability's last CRF file. A CRF file that matches no
  rule is reported as `UNTRIAGED` and the command exits 1.
- The capabilities by matrix verdict, as the command prints them:

| Verdict | Capabilities |
|---|---|
| `BUILD` (22) | Admin menu; Admin permissions; Briefing screen; Core entities; Deploy / respawn UI; Enums / config containers; Faction / groups; Gamemode state machine; Gamemode timers; Helpers layer; Lobby / slot picker UI; Lobby / slotting authority; Logging; Mission validation; Player controller / RPC; Replication backbone; Safestart / warmup; Spawn / respawn; Spectator; UI framework (listbox); Vanilla overrides (damage/groups); Vanilla overrides (spawn) |
| `LATER` (14) | AO / play area / zones + markers; Character overrides; Environment (time/weather); Garbage collection; HUD elements; Inventory overrides; Map staging; Radio / VON frequency plan; Server settings enforcement; UI framework (menu stack); Vanilla UI overrides; Vehicle depot; Vehicle overrides; Vehicle spawning |
| `SKIP` (18) | Airdrop; Battle royale zone; Community tags; CRF documentation samples; CRF editor tooling; CRF game modes; CRF Workbench plugins; Forward deploy; Misc components; Misc world entities; Mortar strike controller; Nuke; Parachutes; Persistence / save-load; Rally points; Slot lottery; Third-party mod bridges; World actions |
| `PARTIAL` (1) | Arsenal / supply |
| `REPLACE` (1) | Loadouts |
| `DEFERRED` (1) | AAR / statistics |

## Related documentation

- [Mod design](/documentation_v2/mod/tbd-framework/mod_design.md) — §4 names the spine the
  `BUILD` rows serve, §6 the deferrals
- [Enfusion tooling](/tools_v2/developer-tools/src/enfusion_tooling/README.md) — the `enf`
  commands that build the CRF index and run the check
