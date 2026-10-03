# Mission compiler source

The source of `mission_compiler`: the game-document compiler, the editor-input structs it parses a
stored payload into, the error of a compile that cannot proceed, and the compiler identity every
[artifact](/documentation/glossary/a_to_f.md#artifact) records.

## Contents

```text
crates/mission/mission_compiler/src/
├── authoring.rs          `EditorPayload` and its rows: the authored input the compiler parses, crate-private
├── compiler_identity.rs  `COMPILER_PACKAGE_VERSION`: the stored compiler identity
├── error.rs              `Error` and `Result`: a payload that does not parse or places no slot
├── game_document/        the compile itself: one walk from payload to document, its findings and substitutions
├── lib.rs                the crate root; re-exports the entry points, the document, the ids of its findings
└── prelude.rs            the public items for glob import
```

## How it works

`authoring.rs` is the input side: `EditorPayload` with its `editor` graph of factions, squads and
[slots](/documentation/glossary/n_to_z.md#slot), and its zones, entities, vehicles and settings.
Every field has a default and unknown keys are dropped, so a missing key never fails a parse;
`environment` and the authored blocks stay raw JSON values so one wrong-typed key in a stored
payload becomes a finding rather than a refusal. Nothing outside the crate sees these structs.
`game_document/` reads them and writes the compiled rows of `mission_model::compiled`.

## Boundaries

- Depends on: `mission_model`, `mission_payload`, `mission_validation`, `mission_wire_safety`,
  `serde`, `serde_json` and `thiserror`.
- Used by: the API, the Mission Creator and the map engine, through the crate root.
- Rules: `authoring` and every item in it stay crate-private; `COMPILER_PACKAGE_VERSION` keeps
  its literal value whatever the crate is called, since every artifact stores and hashes it.
