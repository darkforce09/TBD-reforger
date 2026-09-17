# Phase 3C M: toolbelt and context menu

Read `00_rules_every_agent_obeys.md` and the user-pasted Phase 3C plan. This brief overrides the copied Phase 3B-specific commit suffix and deferral text. The zero-behavior-change contract applies.

Own these files under `apps/website/frontend/src/v2/apps/editor/` (except `../debug` paths, under `v2/apps/debug/`): ui/docks/toolbelt.rs, ui/docks/context_menu.rs. Own adjacent newly extracted test files and named decomposition subfolders. Do not edit any other brief's production files.

1. Extract every inline `#[cfg(test)] mod` block from owned files into sibling `tests/` files. Split test subjects over 1000 lines into descriptive subject files. Put declarations after all production items. Record an extraction checkpoint for a separate commit.
2. Decompose every owned production file over 500 raw lines into coherent named modules, each below 500 lines. Preserve public interfaces, behavior, source-inspection pins, and WASM paths. Record a decomposition checkpoint for a separate commit.
3. Rewrite ticket/wave/history narration in comments into present-tense descriptions of current behavior and correct stale file names within owned files. Remove no behavior tests. Report cross-crate pin changes needed to the coordinator, who owns those shared files.
4. Report each owned file that is clean under doc-audit rules 2, 4, and 5. The coordinator alone removes its row from `v2/tests/doc_audit/allowlist.rs`.

Do not commit or run Cargo without a coordinator slot. Keep all existing dirty changes outside your ownership intact. Report the exact files edited, raw line counts, unresolved references, and the extraction/decomposition checkpoint when ready. At wave validation the coordinator runs frontend tests, wasm check, fmt check, and the file-length gate with `CARGO_TARGET_DIR=target-container`, one Cargo command at a time.
