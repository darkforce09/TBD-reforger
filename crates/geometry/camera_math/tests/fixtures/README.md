# Camera fixtures

The deck.gl captures the orthographic camera's parity suite compares against.

## Contents

```text
crates/geometry/camera_math/tests/fixtures/
└── deckgl_ortho_goldens.json  300 orthographic viewport captures from deck.gl 9.3.5: matrices, projections, round trips
```

## Boundaries

- Depends on: deck.gl 9.3.5, through the capture script the file's `meta` block names.
- Used by: `crates/geometry/camera_math/tests/deckgl_ortho_parity.rs`, which compiles the file in.
- Rules: the captured coordinates and the suite's tolerances are never edited to make a camera
  change pass; a drift is a camera defect.
