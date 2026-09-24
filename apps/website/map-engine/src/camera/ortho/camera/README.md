# Orthographic camera re-exports

A second path to the orthographic map camera: it re-exports `OrthoCamera` and its plane and zoom
constants from `apps/website/map-engine/src/camera/ortho/state.rs` under
`camera::ortho::camera`. Nothing in the repository imports that path; every caller names
`camera::ortho::state` directly.

## Contents

```text
apps/website/map-engine/src/camera/ortho/camera/
└── mod.rs  the module tree; re-exports `OrthoCamera`, `NEAR`, `FAR`, `MIN_ZOOM` and `MAX_ZOOM`
```

## Boundaries

- Depends on: `crate::camera::ortho::state`, which defines every item re-exported here.
- Used by: nothing; `git grep` finds no path through `camera::ortho::camera`.
- Rules: the folder holds re-exports only, so the camera and its constants keep one definition, in
  `state.rs` beside this folder.
