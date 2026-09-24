# T-653 — Plan

## Context
Three headless-screenshot fixes (writable XDG_CACHE_HOME, --use-angle=vulkan only, canvas.toDataURL capture) live
only in a migration note; the Python capture scripts cannot be committed (verify-no-python), so prose is the artifact.

## Approach
1. `docs/website/EDITOR_GATE_RUNBOOK.md`: add "Headless screenshot prerequisites" after "Required environment" (:41)
   with the three findings, the exact flags, and the 3.7 MB vs 45 KB black-map tell.
2. `docs/platform/known-bugs/KB-002-editor-gate-boot-wedge.md`: one See-also line pointing at that section.

## Risks
- Drift if the gate harness changes flags; the section names the harness file so the next editor updates both.

## Verification
- `rg -n 'XDG_CACHE_HOME|use-angle=vulkan|toDataURL' docs/website/EDITOR_GATE_RUNBOOK.md` (three hits)
- `cargo xtask ticket check`
