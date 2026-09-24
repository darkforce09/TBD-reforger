# Mission artifacts

Compiles one version of a [mission](/documentation_v2/glossary.md#mission) into an immutable
[artifact](/documentation_v2/glossary.md#artifact), the compiled mission document together with
every input that determined it, and reads artifacts and their exact bytes back.

## Contents

```text
apps/website/api_v2/src/missions/services/mission_artifacts/
├── artifact_inputs.rs  what a compile reads: the catalog snapshot, the mission metadata, digests
├── artifact_store.rs   compile a version into an artifact or reuse the identical one; read it back
├── mod.rs              the module tree
└── tests/              unit tests for the catalogued compile, stored findings and refusal body
```

## How it works

`compile_artifact` reads the version payload and the current modpack's cargo catalog on the
caller's connection, so the inputs and the bytes they produce belong to one snapshot. It compiles
through `services::mission_compile::flatten_to_mod_document_with_catalog`, then checks the bytes
against `mission.schema.json` and the 8 MiB ceiling with
`contract::schema_validators::validate_mission_document`. A version that cannot become an artifact
answers 422 with a `code`:

| Code | Cause |
|---|---|
| `NO_PLACED_SLOTS` | the version places no [slot](/documentation_v2/glossary.md#slot) |
| `UNCOMPILABLE_VERSION` | the payload does not compile, or its cargo exceeds the catalog's capacity |
| `UNSUPPORTED_AUTHORED_DATA` | the version authors gameplay data the mission document cannot carry |
| `DOCUMENT_CONTRACT_VIOLATION` | the compiled bytes break `mission.schema.json` or pass 8 MiB |

The refusal body carries `finding_count`, the number of distinct findings, and the first
`MAX_REPORTED_FINDINGS` (20) of them; the full list goes to the log. The compile's own findings
never refuse: they are stored with the artifact and served beside its bytes.

The artifact digest is the SHA-256 of the canonical JSON (object keys sorted) of the compiler
version, the schema version, the version id, and the SHA-256 of the payload, the mission metadata
the compiler read, the catalog and the document, with the modpack id and version. The insert does
nothing on a digest that exists and returns that artifact instead, so compiling identical inputs
twice yields one artifact. The catalog rows are ordered by resource name in C collation, so its
digest never depends on storage order.

## Boundaries

- Depends on: `services::mission_compile` for the compile and `contract::schema_validators` for the
  document check; `models::mission::Mission`; `core` for errors and RFC 3339 timestamps;
  `website_map_engine::data::scenario` for `COMPILER_PACKAGE_VERSION`, the cargo catalog types and
  `unsupported_authored_data`; the `mission_artifacts`, `mission_versions`, `registry_items` and
  `modpacks` tables.
- Used by: `services::mission_reviews`, whose `open_review` compiles the artifact a submission puts
  under review; the handlers `mission_reviews.rs` (provenance, bytes and review workspace),
  `game_runtime_missions.rs` (a deployed artifact's bytes) and `artifact_document_response.rs` in
  `apps/website/api_v2/src/missions/handlers/`.
- Rules: an artifact is never changed or deleted: the `mission_artifacts_are_immutable` trigger of
  `apps/website/api_v2/migrations/0050_mission_artifacts.sql` refuses both, and the table's checks
  hold the document to 1 to 8,388,608 bytes; a compile always measures cargo against the current
  catalog (`artifacts_compile_through_the_catalogued_gate`), and its findings are stored, never
  refused (`compile_findings_are_stored_with_the_artifact_and_never_refuse`), both in
  `tests/artifact_store.rs`.

## Related documentation

- [Mission artifacts, reviews and deployment](/documentation_v2/website/api_v2/verification_evidence/mission_artifacts.md)
  — what an artifact records, and how reviews decide it and servers load it.
