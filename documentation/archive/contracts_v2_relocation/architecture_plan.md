**Status:** archived

# Contract Pipeline & Evolution Policy (`contracts_v2`)

How Rust types are generated from the schemas, how a schema is allowed to change, and what CI checks.

---

## 1. Boundary Invariants

1. **The schema is the authority.** Neither a database model nor an Enfusion script class may put a property on the wire that the corresponding schema does not define. Where a schema closes its object (`additionalProperties: false`), the API rejects unknown keys rather than ignoring them.
2. **Optionality is meaningful.** `null` and absent are distinct states and each schema says which it accepts. A reader that conflates them will disagree with a writer that does not.
3. **Schemas carry no code.** Generation emits typed models into consumers; nothing executable lives here.

---

## 2. Code Generation

```bash
cargo xtask schema codegen        # regenerate, writing into the API crate
cargo xtask ci schema-codegen     # the same step as a CI task
cargo xtask ci verify-codegen-fresh
```

`typify` generates serde models from four schemas into `apps/website/api_v2/src/missions/contract/generated/`:

| Schema | Generated module |
|:---|:---|
| `mission-editor-payload.schema.json` | `mission_editor.rs` |
| `registry-items.schema.json` | `registry_items.rs` |
| `registry-compat.schema.json` | `registry_compat.rs` |
| `faction-library.schema.json` | `faction_library.rs` |

Generated files carry a `DO NOT EDIT` header naming their source schema, and CI fails if regenerating produces a diff.

**`loadout-export` is deliberately not generated.** Its versioned `oneOf` branches expand into empty structs under typify, which is silent and wrong rather than loud and broken. The model is hand-written in `apps/website/api_v2/src/missions/contract/loadout_projection.rs` and held to the schema by serde round-trip tests against the committed samples in `fixtures/registry/`.

Schemas consumed without generation are mapped by hand where they are used: `mission.schema.json` by the map engine's scenario document model, `terrain-manifest.schema.json` by its world loader, `bridge-messages.schema.json` by the voice bridge on both sides.

---

## 3. Evolution Policy

### Non-breaking additions
- A new optional property may be added to an open object.
- Adding one to a closed object (`additionalProperties: false`) requires updating every affected fixture in the same commit, because the closed objects are exactly the ones whose goldens would start failing.
- **Readers deploy before writers emit.** The game servers, the API and the SPA update independently, so a field emitted before its readers ship is a deserialization failure in production.

### Breaking changes
- Changing a property's type, removing one, or renaming one is a new major schema version (`mission.v2.schema.json`), not an edit.
- The API keeps parsing the previous version for as long as servers in the field speak it.
- Servers advertise their supported schema versions at boot, and the API refuses to stage a mission a target server cannot parse. That refusal is the point: a mission that fails to load has already wasted an operation.

---

## 4. CI Gates

```bash
cargo xtask schema validate       # full contract-validation suite over schemas and fixtures
cargo xtask schema citations      # every @contract pointer resolves
cargo xtask schema list-gates     # the sub-gate set, derived from the code that runs it
```

`@contract` tags in Rust and Enfusion sources name a schema and an RFC 6901 pointer into it, tying a struct or a script class to the clause it implements. The citation gate resolves all 125 of them across `apps/` and `tools_v2/` and fails closed: if the scan finds zero citations, or a scan root is missing, it reports that it did not run rather than reporting a pass.
