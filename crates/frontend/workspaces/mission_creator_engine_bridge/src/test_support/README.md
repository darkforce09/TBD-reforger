# Mission Creator test support

Test-only helpers for the [Mission Creator](/documentation/glossary/g_to_m.md#mission-creator)'s
source pins: the production half of one source file, and the live entity operation sources the
structural pins read. Compiled only into the test build.

## Contents

```text
crates/frontend/workspaces/mission_creator_engine_bridge/src/test_support/
├── editor_operations.rs  the bridge's host state shards and the hosted commands, joined as source text
└── mod.rs                `production_half`: a source file up to its test-module declaration
```

## How it works

`production_half` returns the text of a source file before its test-module declaration (a
`#[cfg(test)]` whose item is a `mod`), so a pin that reads raw text never searches its own test
module. `editor_operations` joins, at compile time, the bridge's armed placement, editor context and
entity selection shards with the hosted command files of `mission_editing_commands` (`ENTITY`,
`CONTEXT`), and the entity operations of `mission_operations` (`DOMAIN_ENTITY`), keeping the domain
and the adapter sources distinct.

## Boundaries

- Depends on: the source files it embeds under `crates/frontend/workspaces/mission_creator_engine_bridge/src/bridge/host_state/`,
  `crates/mission_editing/mission_editing_commands/src/hosted_commands/` and
  `crates/mission/mission_operations/src/entity/`.
- Used by: the editor's test files under `crates/frontend/workspaces/mission_creator_workspace/src/`.
- Rules: each production shard is embedded once.
