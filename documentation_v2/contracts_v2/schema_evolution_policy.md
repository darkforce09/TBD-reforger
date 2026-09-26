**Status:** live

# Schema evolution policy

How a contract schema in `contracts_v2/definitions/` may change, and what a change must carry with
it. The [API](/documentation_v2/glossary/a_to_f.md#api), the single-page app, the game servers running the
[mod](/documentation_v2/glossary/g_to_m.md#mod) and the external voice bridge update independently, so a schema change that one reader does not
yet understand fails in production, not in a test. Developers and AI agents read this before
editing a schema.

## Where it lives

- Code: the schemas in
  [`contracts_v2/definitions/`](/contracts_v2/definitions/README.md), their fixtures in
  [`contracts_v2/fixtures/`](/contracts_v2/fixtures/README.md), and the typify codegen in
  [`tools_v2/xtask/src/commands/generate/`](/tools_v2/xtask/src/commands/generate/) (`TARGETS` in
  `schema_types.rs`, 24 schemas).
- Entry: `cargo xtask schema codegen`, `cargo xtask schema validate` and
  `cargo xtask schema citations`, and the CI tasks `schema-codegen`, `verify-codegen-fresh` and
  `ci-local-schema`, which the
  [contracts README](/contracts_v2/README.md#getting-started) lists.
- Related features: the [voice bridge contract](/documentation_v2/contracts_v2/definitions/bridge_messages.md),
  which versions its envelope by the same rules.

## Behaviour

### Boundary invariants

1. The schema is the authority. Neither a database model nor an Enfusion script class puts a
   property on the wire that its schema does not define. Where a schema closes an object
   (`additionalProperties: false`), the API's embedded validators reject an unknown key instead of
   dropping it (`apps/website/api_v2/src/missions/contract/schema_validators.rs`).
2. Optionality means something. `null` and absent are distinct states, and each schema says which
   it accepts; a reader that conflates them disagrees with a writer that does not.
3. The tree holds data only. Code generation writes typed models into the consumers; nothing
   executable lives in `contracts_v2/`.

### How a schema reaches code

- Generated types: `cargo xtask schema codegen` runs typify over each schema in `TARGETS` and
  writes one module folder into the `models/generated/` or `contract/generated/` folder of the
  owning API domain. Every generated file opens with a `DO NOT EDIT` header naming its source
  schema, and `cargo xtask ci verify-codegen-fresh` fails on any missing, stale or stray file.
- Hand-written models: `loadout-export.schema.json` stays out of codegen, because typify expands
  its versioned `oneOf` branches into empty structs, which is silent and wrong rather than loud and
  broken. Its model is `apps/website/api_v2/src/missions/contract/loadout_projection.rs`, held to
  the schema by the round-trip tests over both committed sample exports
  (`apps/website/api_v2/src/missions/contract/tests/loadout_projection.rs`).
- Hand mappings: the map engine maps `mission.schema.json` into its mission document model and
  `terrain-manifest.schema.json` into its world loader; the voice bridge's two ends map
  `bridge-messages.schema.json`. The [definitions README](/contracts_v2/definitions/README.md#how-it-works)
  lists every consumer.

### Additive changes

1. A new optional property may join an open object.
2. A new property on a closed object updates every fixture it affects in the same change,
   because the closed objects are exactly the ones whose samples start failing.
3. Readers ship before writers emit. A field lands in its schema and in every reader before any
   writer puts it on the wire; a field emitted first is a deserialisation failure on the game
   server, the API or the app that has not updated.

The [mission](/documentation_v2/glossary/g_to_m.md#mission) schema shows the pattern. Its `schemaVersion` is
`1.0` to `1.3` in one file, each minor version additive, selected by `if`/`then` blocks. The compiler emits the lowest version whose
keys actually reach the wire: a document with no `1.3` key still goes out as `1.1` or `1.2`, so an
older server keeps loading it (`mission_compile_flatten.rs` in
`apps/website/api_v2/src/missions/services/tests/` pins the bump to `1.2` on the first slot
height). The mod's `TBD_MissionValidator.CheckSchemaVersion` admits `1.1`, `1.2` and `1.3`. A
`1.3` field that no shipped reader handles yet is listed by the unread-wire-field check
(`tools_v2/xtask/src/verifications/schemas/checks/wire_field_readers.rs`), and a field loses its
row when its reader lands.

### Breaking changes

1. Changing a property's type, removing a property or renaming one is a new major version in a new
   file (for example `mission.v2.schema.json`), never an edit of the old one.
2. The API keeps parsing the previous version for as long as servers in the field speak it.
3. Servers advertise the schema versions they read at boot, and the API refuses to stage a mission
   a target server cannot parse: a mission that fails to load has already wasted an
   [event](/documentation_v2/glossary/a_to_f.md#event).

No schema has had a major version yet. Rule 3 has no implementation: no game-runtime route
carries a server's schema versions, and today the protection is the compiler's lowest-version
emission and the mod's own version check.

### A change, step by step

1. Edit the schema, keeping the rules above.
2. For a generated schema, run `cargo xtask schema codegen` and commit the regenerated modules.
3. Update every fixture the change touches, then run `cargo xtask schema validate`.
4. Update every `@contract <schema>#<pointer>` citation of a renamed or moved definition, then run
   `cargo xtask schema citations`.
5. Run `cargo xtask ci ci-local-schema`, the schema lane of `ci-local`.

### Known discrepancies

- The mission schema's description names `GET /missions/:id/compiled` as the enforcer of the
  8 MiB document ceiling (`contracts_v2/definitions/mission.schema.json:5`) — no such route
  exists; the API validates a compiled [artifact](/documentation_v2/glossary/a_to_f.md#artifact) when it
  compiles it, and game servers fetch it from `GET /api/v1/game-runtime/artifacts/{artifactId}`
  (`apps/website/api_v2/src/missions/routes.rs`).

## Data

- `contracts_v2/definitions/*.schema.json`: the web API, fleet, game-runtime, mission-review and
  mission-deployment contracts use draft-07 without an `$id`; the others use draft 2020-12 with
  an `$id`, through which the gates resolve cross-file references. The [definitions README](/contracts_v2/definitions/README.md#format)
  gives the naming and encoding rules.
- The generated modules under `apps/website/api_v2/src/*/models/generated/` and
  `apps/website/api_v2/src/missions/contract/generated/`.
- A published mission artifact is validated once, when it is compiled, and its stored bytes are
  never reinterpreted under a newer schema.

## Design

The policy puts the cost of a change on the writer: a reader can always accept more than any
writer sends, so a mixed fleet of old and new servers stays safe while each side updates on its
own schedule. The version-advertising refusal is the design target for breaking changes; until
it exists, a breaking change is not safe to ship.

## Open work

- [T-946.22 — The unread-wire-field gate is one-directional](/.ai/tickets/T-946.22.toml) (idea,
  no plan): a schema description that says nothing reads a field must keep a matching unread
  row, so retiring the row forces the stale wording out.
- [T-946.15 — No CI gate validates a roster-carrying compiled document](/.ai/tickets/T-946.15.toml)
  (idea, no plan): the compiled-document schema check seeds a `vehicles[]` roster, so a document
  that carries one is validated.
- [T-1026 — Add @contract tags to three untagged api_v2 schema-projecting models](/.ai/tickets/T-1026.toml),
  [T-1049 — Add @contract tags to map-engine compiled mission document structs](/.ai/tickets/T-1049.toml)
  and [T-1088 — Add @contract tags to EnfScript JSON wire structs](/.ai/tickets/T-1088.toml)
  (idea, no plan): the citation gate starts covering these hand-written models, so a schema change
  fails where they still cite the old shape.
- [T-1080 — Clean up contract schema descriptions, $id hosts and workflow name](/.ai/tickets/T-1080.toml)
  (idea, no plan): one `$id` host for every schema, and descriptions that name live code.

## Decisions

- Minor versions live in one file and are additive; a breaking change is a new file: an old reader
  never meets a shape it cannot parse under a version it claims to read.
- `loadout-export` is hand-written rather than generated: a silently empty generated struct is
  worse than a hand model pinned by round-trip tests.
- Readers ship before writers emit: the platform's parts deploy independently, and only this
  order keeps a mixed fleet working.
