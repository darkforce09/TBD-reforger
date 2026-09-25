# Faction library seed pins

The parts of `cargo xtask verify faction-library-seeds` below its constants: the entry point that
runs the pin set against the live files and against four broken variants, the three pins, the
comment strippers, and the helpers that read inputs and print evidence. The parent file
`tools_v2/xtask/src/verifications/database/faction_library_seeds.rs` holds the pinned names and
paths.

## Contents

```text
tools_v2/xtask/src/verifications/database/faction_library_seeds/
├── extract_fn_body.rs  function-body extraction, the broken-variant builders, input reading, printing
└── source_audit.rs     the entry point, the three pins and the SQL and `#`/`//` comment strippers
```

## Boundaries

- Depends on: the parent's constants; `SEEDS` of `tools_v2/xtask/src/commands/db/operations.rs`,
  the list `cargo xtask db seed` applies; `WAVE_CHILDREN` and `wave_children_are_linked` of
  `tools_v2/xtask/src/verifications/architecture/wave_gate_sources.rs`; `verification_core`
  verdicts; the `regex` crate.
- Used by: the parent module, which re-exports `verify_faction_library_seeds`; its tests call
  `assert_faction_library_pins` and both strippers.
- Rules:
  - the broken variants are string transforms of text already read, so the gate never writes a
    file; the final live run re-reads the disk;
  - `gate.rs` and its two linked implementation files are read together, and an unlinked or
    missing implementation fails (`missing_wave_implementation_fails_closed`,
    `disconnected_wave_implementation_fails_closed`);
  - a variant that cannot be built exits 2 (`red_setup_refuses_a_list_it_does_not_recognise`,
    `red_setup_refuses_a_wave_it_does_not_recognise`); every other failure exits 1.
