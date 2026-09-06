# T-946.24 — four holes in the mirror-lockstep guard, and one real drift it found

## Why

T-946.23 fixed the mod gate compiling the wrong copy of every shared script, and added
`mirror_lockstep` to cover the mirrors it stopped compiling. The wave-242 adversarial verifier went
at that fix first and broke it four ways. A guard that is wrong is silent, so all four are recorded
here with the measurement that proved them.

## 1 — BLOCKER: a missing framework file was invisible

`mirror_lockstep` walked the export tree and skipped any path with no framework twin. The engine
reads the framework copy of a shared path and falls through to export only when framework has none,
so "framework lost a file" and "this is legitimately export-only" look identical to the compiler.

```
$ mv apps/mod/tbd-framework/Scripts/Game/TBD/Core/TBD_Log.c /tmp/
$ cargo xtask mod compile
OK: compiled clean
    Module: Game; loaded 5740x files; 11314x classes      # unchanged
```

The shipping mod is missing a script and the gate is green on export's copy. This is T-946.23's own
defect class reintroduced in the opposite direction.

**Fix:** name the 13 legitimately export-only scripts. Any other export-only script fails the gate.

## 2 — MAJOR: the stripper truncated at `//` inside a string literal

`strip_enfusion_comments_and_strings` found `//` before blanking literals, so everything after a
`"https://…"` on a line was discarded. A mirror whose export copy called an undefined function after
such a literal passed.

**Fix:** blank literals first, then look for the comment. This also corrects the identifier counts
the unread-wire-field gate depends on.

## 3 — MAJOR: literal contents were erased, so behaviour could diverge

Literals were replaced wholesale with `""`. A resource GUID is a string literal and it decides which
UI layout an addon instantiates, so two mirrors could load different screens and pass.

**Fix:** keep literals and ASCII-fold them, which is all the pure-ASCII rule actually requires.

## 4 — MAJOR: the flip did not change only scripts

Three shared NON-script paths differ byte-for-byte while carrying the same resource GUID, so the
addon order decides which body resolves: `Prefabs/Systems/TBD_GameMode.et` (the export copy carries
`TBD_RoadExportComponent`), `resourceDatabase.rdb` and `addon.gproj`. Nothing covered them.

**Fix:** compare every shared non-script path byte-for-byte against a three-entry allowlist, each
entry carrying its reason.

## What the repaired gate found immediately

The framework and export copies of `TBD_RegistryItemsExportPlugin.c` called `TBD_ExportJson.Escape`
and `TBD_MapExportJson.Escape` — the alias and the class it aliases. Both compile, so nothing had
ever noticed. `TBD_MapExportPaths.c:164` documents the alias as existing for this exact caller, so
they are aligned on the alias.

The lockstep also grew from `Scripts/Game` to all of `Scripts`, which brings the 84 shared
`Scripts/WorkbenchGame` scripts under a check for the first time — the headless server never reads
that module, so neither tree's copy was ever compiled.

## Acceptance

Each of the four attacks above fails the gate, naming the path. A clean tree passes.
