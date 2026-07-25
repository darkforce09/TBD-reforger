# Negative goldens — fixtures that MUST be rejected

`golden-missions/` holds documents that **must validate**. Every file in it is fed to
`mission.schema.json` by `xtask schema validate` and a failure is a failure.

That is only half a gate. A vocabulary nobody tests is not enforced: you can delete an
`enum` from the schema and every positive golden still passes. These fixtures are the
other half — each one is a document that the gate is **required to reject**, and for a
**named reason**. Delete the `container` enum and `cargo-container-typo.json` starts
passing, which fails the run.

They live in a separate directory on purpose. `validate_all()` walks
`golden-missions/*.json` and asserts each file PASSES, so a must-fail fixture cannot
simply sit alongside them — it would be reported as a broken golden. `world-boot.sh`,
`deploy-staging.sh` and `setup-server-profile.sh` also resolve missions out of
`golden-missions/` by name, and none of these documents is bootable.

## Format

A file here is **not** a mission document. It is a wrapper the gate reads:

```json
{
  "$comment": "why this exists",
  "mustFail": {
    "gate": "schema",
    "at": "/slots/0/loadout/cargo/0/container",
    "because": "one line naming the runtime consequence of NOT catching it"
  },
  "document": { "...the mission..." }
}
```

The wrapper shape is deliberate: dropping one of these into `golden-missions/` by
mistake fails loudly and immediately (it is not a mission at all), rather than quietly
becoming a "passing" golden.

| field | meaning |
|-------|---------|
| `gate` | `schema` — `mission.schema.json` must reject it. `registry` — it must be schema-VALID and rejected by the kit-alias registry cross-reference. |
| `at` | RFC-6901 pointer into `document`. **Every** finding must be at or below it, and there must be at least one. That is what pins the fixture to its reason: a fixture that failed for an unrelated typo would be a false green. |
| `because` | Prose for the human reading the gate output. |

## Adding one

Copy the nearest fixture, change **one** thing, and set `at` to the pointer of the thing
you changed. Keep the rest of the document valid — a fixture with two defects cannot
prove which check caught it.
