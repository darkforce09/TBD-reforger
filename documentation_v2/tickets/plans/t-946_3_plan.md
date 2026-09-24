# T-946.3 — Plan

## Context

Wave 241 dispatches T-676, a new Enfusion runtime in `tbd-framework`, and its gate is
`cargo xtask mod compile`. That gate currently fails its pre-compile ASCII scan on three
`tbd-export` WorkbenchGame files, so no Enfusion slice can be gated. The same blockage is why the
wave-240 validator edit shipped uncompiled.

## Approach

Replace the fifteen em-dashes in `TBD_WorldFullExportPlugin.c`, `TBD_BlueprintReconPlugin.c` and
`TBD_MapExportObjects.c` with ASCII hyphens. Do not touch `ascii_check_export` — the scan is the
guard, not the defect.

## Risks

The affected strings include operator-facing `Print` output, so the replacement must read correctly
in a log line, not just satisfy the byte scan. A spaced hyphen does; a doubled hyphen would look
like a flag. Nothing else in the files changes, so the engine verdict is the only thing left to
prove, and that is what the gate reports.

## Verification

- `grep -rlP '[^\x00-\x7F]' apps/mod/tbd-export --include='*.c'` returns nothing.
- `cargo xtask mod compile` gets past the scan and reports the engine's verdict.
