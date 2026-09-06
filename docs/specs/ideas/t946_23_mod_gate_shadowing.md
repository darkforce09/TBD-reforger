# T-946.23 — the mod compile gate read the wrong copy of every shared script

## The defect

`cargo xtask mod compile` symlinks both mod trees into the run directory and launches the dedicated
server with:

```
-addons TBD_Framework,TBD_Export
```

The Enfusion VFS overlays addons **by path**, and the last one wins. 139 script paths exist in both
trees, so for every one of them the engine compiled the `tbd-export` copy and never read the
`tbd-framework` copy at all.

The shipping server loads only `TBD_Framework` (`scripts/mod/tbd-staging-server.config.json`). So
the gate was green over code that does not ship and silent about the code that does.

Measured 2026-09-06, on main, before anything was changed. Appending
`void f(){ThisSymbolDoesNotExist_CC();}` to
`apps/mod/tbd-framework/Scripts/Game/TBD/Backend/TBD_MissionSlotStruct.c`:

```
OK: compiled clean
    Module: Game; loaded 5740x files; 11314x classes
    0 warning(s) in TBD sources
PROBE_FW_EXIT=0
```

The same line in the `tbd-export` copy fails the gate. Found by the T-674.2 slice agent, reproduced
by the command centre before acting on it.

This is the signature defect in a gate: reported success on code it never examined. It covers every
Enfusion slice this factory has run. T-676's `TBD_TriggerRuntime.c` is framework-only, so it has no
mirror to be shadowed by and *was* compiled; the wave-240 edit to both copies of
`TBD_MissionValidator.c` was not, and the claim that it was compile-verified was wrong.

## The repair

Two halves, because the order alone would trade one blind spot for another.

**Framework last.** `-addons TBD_Export,TBD_Framework` makes the engine read the copies that ship.

**A lockstep gate over the mirrors.** With framework last, the export mirrors are no longer
compiled, so they are checked by comparison instead: every `Scripts/Game` script present in both
trees must be the same CODE. Comments and string literals are stripped from both sides before
comparing, because they are legitimately different — `tbd-export` is held to a pure-ASCII rule that
`tbd-framework` is exempt from, so the same sentence carries an em-dash in one tree and a hyphen in
the other. Whitespace is normalised for the same reason. What remains is code, and the code has no
reason to differ.

The two halves together mean a change to a shared script must be made in both trees and must
compile, which is what the repo's convention already said and nothing enforced.

## Acceptance

An error planted in a `tbd-framework` shared script fails the gate. A clean tree passes. Two mirrors
that disagree on code fail with the path named, while a punctuation-only difference passes.
