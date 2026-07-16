# T-159 — Leptos UI rewrite program

**Status:** program hub · **ACTIVE:** **T-159.22.1** (undo granularity hotfix) · **Latest:**
**T-159.22** @ `0154b4e9` (tag **T-159.22**) · **Worktree:**
`.ai/artifacts/worktrees/TBD-T-159/` @ `t-159-leptos-ui`

## Progress (tip `0154b4e9`)

| Milestone | Status |
|-----------|--------|
| Through **T-159.21** chrome + undo UI | shipped |
| **T-159.22** Outliner + Asset palette DnD | `0154b4e9` |
| **T-159.22.1** Undo step boundaries (map-engine-core) | **ACTIVE** |
| **T-159.23** Attributes modal | queued |
| **T-159.24+** ORBAT tree / Arsenal / cutover | queued |

### Verify logs (recent)

- [`.ai/artifacts/t159_21_verify_log.md`](../../.ai/artifacts/t159_21_verify_log.md) — chrome + undo UI
- [`.ai/artifacts/t159_22_verify_log.md`](../../.ai/artifacts/t159_22_verify_log.md) — docks live; wheel+CUR gates;
  **§Pre-existing defect** documents consecutive LOCAL txns merging into one undo step

### Next rationale (confirmed)

**T-159.22.1 before Attributes.** Found during .22, proven on untouched `54c8a4bd`: two drag-moves →
one Ctrl+Z reverts **both**. Contradicts `store.rs` / `mission_history` docs. User-visible; needs a
core unit test that exercises **step boundaries** (current undo smoke only does one mutation).

Then **T-159.23** Attributes (dbl-click modal). **ORBAT tree depth** is **T-159.24** — left dock still
has an ORBAT stub header; Attributes unblocks edit of placed units first.

## Slice index

| Slice | Status |
|-------|--------|
| **T-159.22** | shipped `0154b4e9` |
| **T-159.22.1** | **ready** — `t159_22_1_undo_granularity.md` |
| **T-159.23** | queued — `t159_23_attributes_modal.md` |
| **T-159.24+** | queued (ORBAT / Arsenal) |
