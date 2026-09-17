# Developer-tool executables

The package is `developer-tools`; the six binary names and their existing arguments remain stable:

- `enf`: Enfusion source and API tools.
- `gate`: browser tests and diagnostics.
- `mcpd`: Workbench MCP broker and offline stub.
- `world`: world export and asset processing.
- `map`: raster and cartographic processing.
- `capture`: editor screenshots.

Example: `cargo run -p developer-tools --bin gate -- --help`.

Executable renaming and broad entrypoint decomposition are not part of Phase 2.
