# MCP Test Fixtures (`developer-tools/test_fixtures/mcp`)

> Planned architecture scaffold. Phase one keeps the live Rust module layout; this directory does not yet implement the structure described below.

JSON-RPC test fixtures for the Model Context Protocol (MCP) daemon and Workbench NetAPI callers.

Relocated from the legacy `scripts/mod/fixtures/` directory to live with the developer tooling test fixtures.

---

## Fixture Inventory

- **`mcp-empty.jsonl`**: Empty responses test fixture.
- **`mcp-init-fail.jsonl`**: Initialization failure handling test fixture.
- **`mcp-tool-error.jsonl`**: Tool execution error handling test fixture.
- **`mcp-tool-iserror.jsonl`**: Tool `isError` flag verification fixture.
- **`mcp-wb-state-success.jsonl`**: Successful Workbench NetAPI state payload fixture.
