# T-159.1 — Claude Code handoff (Leptos scaffold)

**CWD:** `/home/Samuel/Projects/TBD-Reforger/.ai/artifacts/worktrees/TBD-T-159`  
**Branch:** `t-159-leptos-ui` (standing worktree — linear commits)  
**Spec:** [`docs/platform/t159_1_leptos_scaffold.md`](../../docs/platform/t159_1_leptos_scaffold.md)  
**Hub:** [`docs/platform/t159_leptos_ui_program.md`](../../docs/platform/t159_leptos_ui_program.md)

## Context

Operator overruled “keep React”: full UI rewrite to **Leptos**, developed like T-151 on a
standing worktree so `main` / Arsenal work can continue. This slice is **scaffold only** —
prove the crate builds and serves a branded hello page.

## File map (expected touches)

| Path | Action |
|------|--------|
| `apps/website-leptos/**` | Create (Leptos app + assets + README) |
| `Cargo.toml` | Add workspace member |
| `Cargo.lock` | Update |
| `Makefile` | Optional `web-leptos` / `leptos-dev` |
| `.ai/artifacts/t159_1_verify_log.md` | Create |

## Return contract

Verify log + tag **T-159.1** + “Ready for Cursor doc sync.”
