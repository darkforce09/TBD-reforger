# T-159 — Leptos UI rewrite program

**Status:** program hub · **ACTIVE:** **T-159.17** (yrsPersist / editor session) · **Latest:**
**T-159.16** @ `f2cd6178` (tag **T-159.16**) · **Worktree:**
`.ai/artifacts/worktrees/TBD-T-159/` @ `t-159-leptos-ui` ·
[`.ai/tickets/registry.json`](../../.ai/tickets/registry.json)

## Progress (tip `f2cd6178`)

| Milestone | Status |
|-----------|--------|
| 24 page routes byte-identical | shipped |
| **T-159.15.0–.15.2** engine + loop + pan | shipped |
| **T-159.16** MissionDocCore host (same wasm, Class R encode) | `f2cd6178` |
| **T-159.17** yrsPersist IDB + warm editor session | **ACTIVE** |
| .18–.22 tools / save / outliner / Arsenal | queued |
| .23–.25 sweep / cutover | queued |

### Verify logs

- [`.ai/artifacts/t159_15_1_verify_log.md`](../../.ai/artifacts/t159_15_1_verify_log.md) — GpuTimer / `disable_frame_timing`
- [`.ai/artifacts/t159_15_2_verify_log.md`](../../.ai/artifacts/t159_15_2_verify_log.md) — pan Class R
- [`.ai/artifacts/t159_16_verify_log.md`](../../.ai/artifacts/t159_16_verify_log.md) — MissionDoc host; seed 8 slots / 1069 B

### Locked (carry forward)

L1–L11 as prior hub. MissionDoc = plain Rust `MissionDocCore` in same wasm (no JS shim). Mutator
port still deferred past .17.

## Slice index

| Slice | Goal | Status |
|-------|------|--------|
| **T-159.15.2** | Pan | shipped `ebcabe1d` |
| **T-159.16** | MissionDoc host | shipped `f2cd6178` |
| **T-159.17** | yrsPersist + editor session | **ready** — `t159_17_yrs_persist.md` |
| **T-159.18+** | Select tools → save → shell → Arsenal | queued |

## Ops

```bash
cd /home/Samuel/Projects/TBD-Reforger/.ai/artifacts/worktrees/TBD-T-159
# smokes: smoke_editor · selfcheck_editor · smoke_pan_editor · smoke_doc_editor
```
