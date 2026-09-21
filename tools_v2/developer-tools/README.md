# Developer tools

`developer-tools` supplies the `developer_tools` Rust library and six executables: `enf`, `gate`, `mcpd`, `world`, `map`, and `capture`. Thin entrypoints in `src/bin/` call subsystem CLI modules; executable names do not depend on module names.

## Executables

- `enf`: Enfusion source extraction, indexing, symbol queries, API documentation, citations, and capability checks.
- `gate`: Chromium/CDP diagnostics, route checks, DOM verification, and editor smoke scenarios.
- `mcpd`: persistent Unix-socket broker for serialized Enfusion NetAPI requests.
- `world`: exported world preparation, object partitioning, catalogs, terrain, roads, and mathematical validation.
- `map`: aerial orthophotos, satellite archives, cartographic images, inland water, and labels.
- `capture`: browser-based editor image capture and zoom sweeps.

For example, build the executables or inspect their command syntax with:

```bash
cargo build --locked -p developer-tools --bins
cargo run --locked -p developer-tools --bin enf -- --help
cargo run --locked -p developer-tools --bin world -- --help
cargo run --locked -p developer-tools --bin map -- --help
```

## Source layout

- `src/browser_testing/` owns Chromium/CDP lifecycle, the static server and API proxy, browser diagnostics, DOM oracles, route drift, screen capture, fixture injection, and editor smoke tests. Shared harness helpers and individual scenarios live under `editor_smoke_tests/`.
- `src/enfusion_tooling/` owns source extraction, indexing, symbol scanning, API documentation, capability checks, citations, and the MCP broker implementation.
- `src/enfusion_pak/` implements shared Enfusion archive parsing and payload access with explicit filesystem policies for world exports and blueprint compilation.
- `src/map_raster_pipeline/` owns aerial orthophotos, satellite archive containers, cartographic rendering, image operations, inland-water classification and emission, and map-label generation.
- `src/world_export_pipeline/` owns export preparation and validation, object census and classification, chunk partitioning and emission, prefab catalogs, road networks, vegetation density, forest contours and smoothing, texture decoding, and mathematical verification.
- `src/blueprint/` groups mesh decoding, voxel processing, architectural analysis, BVH construction and batch processing, and archive emission. Ingestion and parity reporting provide the corresponding library entrypoints.
- `src/map_verification/` implements engine-backed object goldens, label checks, terrain and BLAS manifest checks, and world line-of-sight verification.
- `src/repository_paths.rs` resolves repository paths; `src/timestamp_formatting.rs` provides shared timestamp formatting.

## Interfaces and fixtures

The crate depends on `website-map-engine` for spatial and asset contracts and does not depend on `xtask`. Repository commands call the public blueprint and map-verification entrypoints. Command adapters pass the active repository path and retain each entrypoint's result and exit-code contract.

`test_fixtures/blueprint/` contains blueprint, prefab, and world-parity fixtures. Binary formats, schema versions, numeric constants, operation order, thresholds, and deterministic emitted bytes are compatibility contracts. Tests compare expected assets and invariants; module moves must preserve those checks and their fixture resolution.

## Validation and file limits

```bash
cargo test --locked -p developer-tools -- --test-threads=1
cargo check --locked -p developer-tools --all-targets
cargo fmt -p developer-tools --check
cargo xtask mk leptos-gates
```

The library suite exercises local fixtures and algorithms. Browser gates additionally require the browser environment and application services checked by the gate doctor; they are required separately from `cargo xtask ci ci-local`.

Unit tests live in separate sibling test files, declared with `#[cfg(test)]` and `#[path = "tests/…"]`. Production files must stay below 500 lines and test files below 1,000 lines. Binary entrypoints must stay below 250 lines; editor smoke scenario files must stay below 450 lines. Inline test modules are prohibited. Structural tests in `xtask` enforce these limits and the tooling dependency boundaries.
