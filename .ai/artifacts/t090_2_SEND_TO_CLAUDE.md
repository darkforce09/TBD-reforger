# Send-off — T-090.2 (map object taxonomy ship)

**CWD:** `.ai/artifacts/worktrees/TBD-T-090-2` · **Branch:** `ticket/T-090-2`

Runs **in parallel** with **T-090.1.2.5.1** on `main` — no satellite ortho overlap.

```bash
cd .ai/artifacts/worktrees/TBD-T-090-2
./scripts/ticket prompt T-090 --slice T-090.2
```

Handoff: [`.ai/artifacts/t090_2_claude_code_handoff.md`](t090_2_claude_code_handoff.md)  
Parallel playbook: [`.ai/artifacts/t090_2_parallel_setup.md`](t090_2_parallel_setup.md)  
Spec: [`docs/specs/Mission_Creator_Architecture/t090_2_map_object_taxonomy.md`](../../docs/specs/Mission_Creator_Architecture/t090_2_map_object_taxonomy.md)

Bootstrap parent: **T-090.0.2** (schemas + partial goldens already on `main` @ `0418d952`).
