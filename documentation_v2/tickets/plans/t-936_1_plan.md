# T-936.1 — Plan

## Context
flatten.rs:2573-2582 hardcodes `mode: "attrition"` and derives `end_on`; :2249 lists winConditions as
synthesized. mission.schema.json:8 already requires winConditions ($defs :1132: free mode string + endOn
enum). TBD_ObjectiveRegistry.c:40,239-247 drives endOn triggers. The editor has no card.

## Approach
1. Schema: mode enum attrition|objective|extraction|vip|timeout + params; golden; payload schema declares it.
2. `mission/extensions.rs` (new): AUTHORED_BLOCKS + validators; compile.rs:156 copies keys; flatten emits ExtensionBlocks; `mission/mod.rs` registers both.
3. `mission/win_conditions.rs` (new): parse/validate; flatten.rs:2573-2582 uses it when present.
4. `panels/win_conditions_card.rs` (new) from panels/mod.rs; `Gamemode/TBD_WinConditionEvaluator.c` (new).
5. Perturbation: invert the timeout comparison → red; restore, touch, green.

## Risks
- Absent block must emit today's bytes exactly — the Class-R parity suite is the guard.
- flatten.rs contention (T-674.1/T-675.1/T-290/T-291/T-936.3): different waves, no shared edits.

## Verification
- `cargo test -p map-engine-core --all-features`; `cargo xtask ci schema-validate`; `cargo xtask mk ci-local-leptos`; `cargo xtask mk leptos-gates`; `cargo xtask mod compile`; `cargo xtask platform wave gate --slice T-936.1`
