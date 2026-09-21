# Central Task Runner (`tools_v2/xtask/`)

`xtask` is the central developer task dispatcher (`cargo xtask ...`) and repository integrity verification suite.

## Responsibilities
- Local database container management (`cargo xtask db up|down|seed|test-it`).
- Development server execution (`cargo xtask mk rust-api|leptos`).
- CI quality gate enforcement (`cargo xtask ci ci-local`, `cargo xtask mk ci-local-leptos`).
- Strict structural and boundary verification rules.
- Ticket validation, view synchronization, wave compilation, and the 3D blueprint compiler; these move in later Tools V2 phases.

## Code Mapping
- Source: `tools_v2/xtask/`
