# World-layer preference tests

Unit tests of the world-layer switches: which layers start on, and the JSON spelling the
Mission Creator's preference store keeps them in.

## Contents

```text
crates/streaming/map_streaming_model/src/tests/
└── world_layer_preferences_tests.rs  defaults and the stored camelCase label keys
```

## Boundaries

- Depends on: `crate::world_layer_preferences` through `use super::*`; `serde_json` (a
  dev-dependency).
- Used by: nothing outside the folder; `src/world_layer_preferences.rs` compiles it only in test
  builds.
- Rules:
  - every layer starts on except props (`every_world_layer_starts_on_except_props`);
  - the label switches are stored as `townLabels` and `roadNames`, one key per switch, and read
    back unchanged (`the_label_switches_are_stored_in_camel_case`).
