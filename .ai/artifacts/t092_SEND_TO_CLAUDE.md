# Send-off — T-092.1 + T-092.2 (same worktree)

**CWD:** `.ai/artifacts/worktrees/TBD-T-092` · **Branch:** `ticket/T-092`

```bash
cd .ai/artifacts/worktrees/TBD-T-092
git rebase main
./scripts/ticket prompt T-092 --slice T-092.1
```

After **T-092.1** ships (tag `T-092.1`):

```bash
./scripts/ticket prompt T-092 --slice T-092.2
```

Handoff: [`.ai/artifacts/t092_claude_code_handoff.md`](t092_claude_code_handoff.md)  
Hub: [`docs/specs/Mission_Creator_Architecture/t092_spawn_transform_program.md`](../../docs/specs/Mission_Creator_Architecture/t092_spawn_transform_program.md)

**Parallel:** `main` runs **T-090.1.1** Map view — no overlap with mod/schema/compiler work here.

**Order:** `.1` spawn policy **must** ship before `.2` flatten + `/compiled`.
