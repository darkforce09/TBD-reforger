# Outliner drag smoke

The parts of the `outliner-drag` smoke declared in
`tools_v2/developer-tools/src/browser_testing/editor_smoke_tests/outliner_drag.rs`: it drives the
[Mission Creator](/documentation_v2/glossary.md#mission-creator)'s real drag handlers over four
controlled [missions](/documentation_v2/glossary.md#mission) and reads every verdict from the
editor's own compiled save payload and undo history.

## Contents

```text
tools_v2/developer-tools/src/browser_testing/editor_smoke_tests/outliner_drag/
├── execution.rs           `run`: the smoke's steps and checks, from the first mission to the verdict
├── mission.rs             the fixture missions, the API interception, and the drag and read helpers
└── vehicle_snap_cases.rs  the vehicle snap, outside-drop, height-drag and ORBAT case groups
```

## How it works

`run` starts the shared `Harness` on ports 5396 and 9496 with the map assets and no API proxy.
`mission::intercept` answers every `/api/v1/` request itself: the fixture user and registry from
`apps/website/frontend/tests/fixtures/api/`, a token refresh, one of four missions by id (five slots
with a vehicle, the same with a duplicated slot id, a large one, and a mixed one), an empty list
for anything else, and a 400 for a version save, which it counts. It also accepts any unload
dialog so the next mission can load.

Each check reads `window.__editorCommands.compile_save_json()` and the undo depth before and after
a gesture made with trusted DevTools input at coordinates taken from the rendered widget:

- the height drag of a mixed slot and vehicle selection previews without writing, keeps each
  height offset, commits as one undo step and releases pointer capture;
- a cancelled pointer, or a release by another pointer id, never commits;
- five-row outliner and [ORBAT](/documentation_v2/glossary.md#orbat) drops move all five as one
  undo step, and undo restores the authored structure;
- tactical-graphic drawing, picking, deletion and cancel behave, and a stale tactical selection
  does not swallow a delete;
- a mission with a duplicated slot id is refused before any save request leaves;
- the large mission repeats the outside-drop cases with the windowed outliner;
- no panic reaches the console.

The verdict is one JSON object of named checks; the smoke exits 0 only when every check passed.

## Boundaries

- Depends on: the parent's `Harness`, readiness expressions and input and assertion helpers;
  `gate_access_token` from `tools_v2/developer-tools/src/browser_testing/session_tokens.rs`;
  `crate::repository_layout`; the fixtures `GET__me.json` and `GET__registry.json`; the editor's
  `window.__editorCommands`, `__editorSelection`, `__editorHistory` and `__missionDoc` hooks.
- Used by: `outliner_drag.rs`, which re-exports `run`; `run_smoke` in
  `tools_v2/developer-tools/src/browser_testing/editor_smoke_tests/run_smoke.rs`, as
  `gate smoke outliner-drag` and as part of `gate editor-suite`.
- Rules: gestures go through the editor's real handlers with trusted input, never a copied
  gesture model; a held mouse button always travels with a matching `buttons` mask, or Chromium
  drops pointer capture on the next move.
