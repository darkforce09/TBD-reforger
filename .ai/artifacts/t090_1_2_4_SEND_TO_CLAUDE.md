# Send-off — T-090.1.2.4 (engine render ortho)

**Paste the full prompt** (do not truncate):

```bash
./scripts/ticket prompt T-090
```

Or with file header paths:

```bash
./scripts/ticket prompt T-090 --header
```

Handoff (read first): [`.ai/artifacts/t090_1_2_4_claude_code_handoff.md`](t090_1_2_4_claude_code_handoff.md)  
Spec: [`docs/specs/Mission_Creator_Architecture/t090_1_2_4_engine_render_ortho_spike.md`](../docs/specs/Mission_Creator_Architecture/t090_1_2_4_engine_render_ortho_spike.md)  
Standard: [`.ai/tickets/CLAUDE_CODE_PROMPT.md`](../tickets/CLAUDE_CODE_PROMPT.md)

**Critical:** `MapDataExporter` = stylized cartographic (wrong). Target = continuous **terrain surface color** like Reforger in-game map — not SAP cell paste, not web tiles.
