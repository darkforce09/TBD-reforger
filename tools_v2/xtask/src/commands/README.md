# Domain Command Handlers (`xtask/src/commands`)

> Planned architecture scaffold. Phase one keeps the live Rust module layout; this directory does not yet implement the structure described below.

Domain-specific CLI command routers.

Each directory in this module defines its own typed Clap subcommand enum and execution method (`run(self, root: &Path) -> Result<u8>`), completely isolating command logic from the top-level `main.rs`.

---

## Command Groups

- **`ticket/`**: Forwards commands to `ticket-engine::cli`.
- **`wave/`**: Forwards commands to `ticket-engine::wave_lock`.
- **`map/`**: Forwards asset pipeline tasks to `developer-tools`.
- **`mod_ops/`**: Game mod compilation, dev-server launching, and playtest operations.
- **`db/`**: Docker PostgreSQL container lifecycle and database migration drills.
- **`deploy/`**: Staging server deployment, website deploy, and database backup routines.
- **`ci/`**: Continuous Integration suites and test runner entry points.
- **`setup/`**: Linux Workbench setup, symlinking, and server profile configuration.
- **`fetch/`**: Scrapes upstream Bohemia Script APIs and vanilla script references.
- **`mcp/`**: Model Context Protocol JSON-RPC daemon and Workbench NetAPI calls.
- **`mk/`**: Makefile target emulation for backward-compatible developer workflows.
- **`debug/`**: Direct-join networking diagnostics and A2S query probes.
- **`repro/`**: HTTP payload reproduction tools.
- **`ai/`**: Agent token usage estimation and output filtering.
