# T-933 — Plan

## Context

Wave-211 F2 MAJOR: `mk leptos-gates` = doctor + editor-suite + v-suite verify (`xtask/src/mk_build.rs`); the v-suite SPA goldens mass-fail independently, so the composite exit is non-zero after editor-suite 20/20 — the T-843 pre-close recipe lies about the editor half.

## Approach

1. Verify on main: run `cargo xtask mk leptos-gates`; paste the tail showing editor 20/20 then v-suite red.
2. Either refresh the SPA goldens or split the recipe so the rect pre-close is editor-suite-only (`mk_build.rs`); pick the honest one and say why.
3. `EDITOR_FACTORY_FOR_CURSOR.md` + `EDITOR_GATE_RUNBOOK.md`: name the live command.

## Risks

- Refreshing goldens can hide real SPA regressions; if goldens change, list every diff in the report.

## Verification

- `cargo xtask mk leptos-gates` exits 0 on main with editor-suite 20/20 · `cargo xtask platform wave gate --slice T-933`
