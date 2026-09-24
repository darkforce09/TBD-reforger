# Browser console logging

The console macros the map engine logs with in the browser: `log!`, `warn!` and `error!` format
their arguments and write the text to the browser console. They compile only for wasm32 with the
`render` feature.

## Contents

```text
apps/website/map-engine/src/diagnostics/platform/
├── console.rs  the `log!`, `warn!` and `error!` macros over `web_sys::console`
└── mod.rs      the module tree
```

## Boundaries

- Depends on: `web-sys` (`console::log_1`, `warn_1` and `error_1`).
- Used by: `crate::world::terrain::satellite::quadtree` (its bootstrap, retry, basemap, selection
  and download steps), `crate::streaming::host::preferences` and
  `crate::streaming::loaders::occluder_loader`.
- Rules: the macros are `pub(crate)`, so only this crate logs through them; callers spell them in
  full (`crate::diagnostics::platform::console::warn!`), and
  `apps/website/frontend/src/v2/apps/editor/tests/t629_satellite_resolution.rs` reads the satellite
  quadtree's source for those spellings (`no_call_site_may_guess_a_texture_limit`,
  `a_downscaled_basemap_warns_and_a_stuck_placeholder_warns`), so moving or renaming a macro breaks
  that suite.
