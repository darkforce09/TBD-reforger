# Enfusion MCP Automation Bridge (`mod/tbd_emcp/`)

The Enfusion Model Context Protocol (MCP) bridge (`TBD_EMCP`) enables programmatic automation of the Enfusion Workbench over a local NetAPI socket.

## Architecture
- `19 NetAPI Handlers`: Automate world entity spawning, component inspection, camera positioning, asset queries, and prefab compilation.
- `mcpd Daemon`: Local broker daemon (`tools_v2/developer-tools/src/bin/mcpd.rs`) interfacing AI agents with the Workbench NetAPI socket.
- `cargo xtask mcp call`: Command-line task runner for invoking Workbench automation handlers.

## Code Mapping
- Mod Scripts: `apps/mod/tbd-emcp/Scripts/WorkbenchGame/EnfusionMCP/`
- Broker Daemon: `tools_v2/developer-tools/src/bin/mcpd.rs`
