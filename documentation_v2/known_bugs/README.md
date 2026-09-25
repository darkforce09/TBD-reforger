**Status:** live

# Known bugs

The registry of recorded, triaged defects: one file per bug, saying what it is, why it happens,
how to live with it and what fixes it, so no one derives the analysis again. Developers and agents
read it before chasing a symptom that may already be understood.

## Contents

```text
documentation_v2/known_bugs/
└── kb_*.md  one known bug per file, named kb_<NNN>_<subject>.md and headed KB-NNN
```

## How it works

Each entry follows the [known bug template](/documentation_v2/standards/templates/known_bug.md):
its number and title, then Status, Symptom, Cause, Workaround, Fix and Related tickets. An entry
records a defect someone reproduced; a suspected defect, or one found by reading code, is a
[ticket](/documentation_v2/glossary.md#ticket) until it is reproduced. A new entry takes the next
free number and gets a row below. A resolved entry stays in the folder with its status set to
resolved and the proof of what resolved it.

| Entry | Area | Status |
|---|---|---|
| [KB-001](/documentation_v2/known_bugs/kb_001_mission_creator_selection_at_scale.md): selection and copy-paste break at extreme slot counts | Mission Creator selection | resolved: not reproducible, the code it lived in is deleted |
| [KB-002](/documentation_v2/known_bugs/kb_002_editor_gate_boot_wedge.md): the editor gate wedges at boot on a font-fallback crash | browser gate harness | resolved: fixed |

## Code

- [Mission Creator](/apps/website/frontend/src/v2/apps/editor/) — KB-001: the selection and paste
  paths that replaced the code the defect was seen in.
- [Browser gate harness](/tools_v2/developer-tools/src/browser_testing/) — KB-002: the Chromium
  build and font cache it launches with.

## Boundaries

- Depends on: the known bug template, which fixes each entry's sections, and the ticket registry
  in `.ai/tickets/` for Related tickets.
- Used by: the [editor gates runbook](/documentation_v2/runbooks/editor_gates.md), which cites
  KB-002; the Cursor rule `.cursor/rules/acceptance-gates-reproducible.mdc`, which sends a recorded
  gate defect here; comments in the gate's screen capture
  (`tools_v2/developer-tools/src/browser_testing/screen_capture.rs`) that name KB-002; the ticket
  registry, whose tickets cite the entries; and the known bug and documentation folder templates,
  whose samples are written from these entries.
- Rules: one bug per file; a number is never reused, so a dropped number stays unused; a resolved
  entry stays with its status set to resolved; an entry needs a reproduced defect.

## Related documentation

- [Editor gates](/documentation_v2/runbooks/editor_gates.md) — the gate's wedge modes and debug
  recipe.
- [Performance at scale](/documentation_v2/website/frontend/apps/editor/feature_inventory/performance_at_scale.md)
  — what the Mission Creator does at large slot counts.
