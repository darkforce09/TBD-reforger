> **SUPERSEDED 2026-09-12.** tbd-export is no longer a mirror of tbd-framework; it is a thin addon that depends on it (plus the new tbd-emcp bridge addon). The lockstep/ASCII/addon-order machinery this note describes was deleted from `xtask/src/gate_mod_compile.rs`; `mod compile` is framework-only. Kept as history.

# T-946.3 — the mod compile gate is blocked before it compiles anything

## The defect

`cargo xtask mod compile` is the only gate for Enfusion script. Before it boots the dedicated
server it runs `ascii_check_export` over every `.c` file under `apps/mod/tbd-export/Scripts`, and
that scan is a hard FAIL. Three files carried fifteen em-dashes in comments and operator-facing
`Print` strings, so the gate returned non-zero without compiling a line:

| file | em-dashes |
|---|---|
| `TBD_WorldFullExportPlugin.c` | 13 |
| `TBD_BlueprintReconPlugin.c` | 1 |
| `TBD_MapExportObjects.c` | 1 |

The scan itself is right and must stay. `Scripts/WorkbenchGame` is never read by the headless
server — a planted `Undefined function` there compiles clean — so the byte scan is the only
pre-restart guard for that module, and Workbench hard-rejects non-ASCII punctuation when it
eventually lexes those files.

What made this a blocker rather than a nit: the operator has ruled that agents may edit the mod's
`.c` scripts with this gate as the check. While the scan fails, no Enfusion slice can be gated at
all, and the wave-240 command-centre edit to both copies of `TBD_MissionValidator.c` shipped
UNCOMPILED for exactly that reason. Wave 241 dispatches T-676, a new Enfusion runtime.

## The repair

Transliterate the fifteen em-dashes to ASCII hyphens. No logic changes, no scan changes. The three
files are comments and log strings; `git diff` is fifteen characters wide.

`apps/mod/tbd-framework` stays exempt from the scan, as documented at `gate_mod_compile.rs:451` —
it predates the rule and is engine-green — but new framework code should be written ASCII anyway.

## Acceptance

`cargo xtask mod compile` reaches the compiler and reports the engine's own verdict rather than
failing its ASCII scan. `grep -rlP '[^\x00-\x7F]' apps/mod/tbd-export --include='*.c'` finds nothing.
