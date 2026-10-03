# Enfusion tool crates

The libraries that read and drive the Enfusion engine's own formats and tools: the pak archive
reader, the script symbol index over the mod and reference scripts, and the broker that keeps one
Enfusion MCP server session for the Workbench automation.

## Contents

```text
tools/enfusion/
├── enfusion_mcp_broker/    `enfusion_mcp_broker`: the `mcpd` broker over one enfusion-mcp server behind a Unix socket, and its offline stub
├── enfusion_pak/           `enfusion_pak`: the `.pak` archive reader, its virtual file system and the loose and layered sources
└── enfusion_script_index/  `enfusion_script_index`: the script oracle behind `enf` and the vanilla page mirrors behind `cargo xtask fetch`
```
