# T-653 — Preserve the three headless editor-screenshot findings

Owner: command center. Docs ticket (executor cursor-docs). Source: migration_legacy note on the ticket.

## Claude Code prompt — T-653

```text
═══ PREFLIGHT ═══
cd .ai/artifacts/worktrees/T-653 && pwd && git branch --show-current   # must be slice/T-653
═══ READ ═══
.ai/tickets/T-653.toml notes (verbatim findings), docs/website/EDITOR_GATE_RUNBOOK.md, docs/platform/known-bugs/KB-002-editor-gate-boot-wedge.md
═══ PROBLEM ═══
The three headless screenshot fixes exist only in a ticket note; the runbook that people read has none of them.
═══ SHIPPED ═══
EDITOR_GATE_RUNBOOK.md structure (Run it / Required environment / Known wedge modes / Debug recipe).
═══ LANGUAGE GATE ═══
Markdown only. No scripts of any kind (verify-no-python is a hard gate).
═══ LOCKED ═══
- Keep the three findings verbatim in substance: XDG_CACHE_HOME writable; --use-angle=vulkan only; canvas.toDataURL.
- Include the black-map tell (3.7 MB vs 45 KB).
═══ DO ═══
1. Add the section to the runbook. 2. Add the See-also line to KB-002. 3. Run the rg check and ticket check.
═══ DO NOT ═══
No code or script files; no edits outside the two owned docs; no git add -A.
═══ VERIFY ═══
rg -n 'XDG_CACHE_HOME|use-angle=vulkan|toDataURL' docs/website/EDITOR_GATE_RUNBOOK.md ; cargo xtask ticket check
═══ MANUAL ═══
None.
═══ RETURN ═══
Report schema per brief. Ready for Cursor doc sync.
```
