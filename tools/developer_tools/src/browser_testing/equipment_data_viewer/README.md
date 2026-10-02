# Equipment data viewer live check

`gate equipment-data-viewer`: an acceptance check that drives the real equipment data viewer at
`/debug/data-viewer` through a normally running website and its imported generation, and records
screenshots and measurements of what it saw.

## Contents

```text
tools/developer_tools/src/browser_testing/equipment_data_viewer/
├── grid_navigation.rs    catalog search and selection, inline expansion, history and a large vehicle
├── mod.rs                `run`: the status read, the overview, the resource inspector and the report
├── polling.rs            the background status poll keeps the visible document through success and failure
└── polling_stability.js  the page script that watches the main panel while the poll runs
```

## How it works

`run` launches headless Chromium, reads `GET /api/v1/debug/equipment-data/status` from the website
to learn the imported generation, opens `/debug/data-viewer`, and checks in turn that the overview
renders anonymously in a full window (no navigation, no stored access token), that the resource
inspector renders, that catalog search, selection, inline expansion and back navigation keep the
reading context (`grid_navigation.rs`), and that a status poll, answered or failed, leaves the
visible document in place (`polling.rs` with `polling_stability.js`). Screenshots and
`polling-stability.json` go to the output folder; the command prints `PASS` and exits 0, and any
failed check ends it with an error.

## Boundaries

- Depends on: `crate::browser_testing::cdp` (launch, pages, evaluation, screenshots); `reqwest` for
  the status read; a website already serving the app and the API, by default
  `http://localhost:3000`, with an imported equipment generation.
- Used by: `gate equipment-data-viewer` in `tools/developer_tools/src/browser_testing/cli.rs`,
  run by hand with `--website` and `--output` (default
  `assets/scratch/equipment-data-viewer-verification`); no gate suite runs it.
- Rules: the check reads only; it never imports or publishes a generation, and it needs the
  imported data it names (the large combat field pack and a large vehicle) to be present.
