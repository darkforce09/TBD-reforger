# Rounding

Float-to-integer rounding where truncation toward zero would bias a result.

## Contents

```text
apps/mod/tbd-framework/Scripts/Game/TBD/Core/Math/
└── TBD_Rounding.c  the nearest integer to a float, halves away from zero
```

## How it works

`TBD_Rounding.RoundToInt(value)` adds 0.5 to a non-negative value and subtracts 0.5 from a
negative one before the integer conversion truncates, so 2.5 becomes 3 and -2.5 becomes -3. It is
written without a ternary, which Enforce Script does not have.

## Authority

- Server: nothing; a pure function.
- Client: nothing.
- Owner: nothing.
- RPCs: none.
- Replicated properties: none.

## Boundaries

- Depends on: nothing.
- Used by: `TBD_TaskStateMachine` in `apps/mod/tbd-framework/Scripts/Game/TBD/Gamemode/Objectives/`;
  `TBD_MarkerService` and `TBD_RadioPlan.FreqKHz` in
  `apps/mod/tbd-framework/Scripts/Game/TBD/Systems/`.
- Rules: lines added stay ASCII; `cargo xtask mod compile` checks that the scripts compile.

## Related documentation

- [Framework core utilities](/apps/mod/tbd-framework/Scripts/Game/TBD/Core/README.md) — the rest of the core
