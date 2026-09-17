# Schema Verifications (`verifications/schemas`)

> Planned architecture scaffold. Phase one keeps the live Rust module layout; this directory does not yet implement the structure described below.

Modularized JSON Schema and contract assertion suite.

Decomposed from the monolithic 4,764-line legacy `schema_gates.rs` file into clean, single-responsibility modules strictly under **500 lines of code** each.

---

## Submodules

- **`contract_citations.rs`** (<400 LOC): Validates RFC-6901 JSON pointer citations and `@idx` doc references across architecture specs and schemas.
- **`sentence_and_tile_budgets.rs`** (<200 LOC): Enforces sentence rules (N6) and satellite/terrain tile payload budget limits (N10).
- **`map_object_enums.rs`** (<200 LOC): Validates map object catalog enums against schema definitions.
- **`type_inventory.rs`** (<400 LOC): Runs type inventory checks I1–I7 between schemas and Rust/TypeScript DTOs.
- **`terrain_manifest.rs`** (<300 LOC): Validates binary chunk manifest headers and boundary alignment.
- **`terrain_binary_blocks.rs`** (<300 LOC): Validates binary chunk POD formats and elevation block limits.
- **`spec_consistency.rs`** (<450 LOC): Audits 12 spec-to-model consistency gates.
- **`kit_alias_spawn_registry.rs`** (<150 LOC): Cross-references kit aliases with the Enfusion spawn registry.
- **`schema_version_wire_fields.rs`** (<480 LOC): Schema version wire fields tripwire ensuring fields remain unread until readers land.
- **`validator/compiler.rs`** (<450 LOC) & **`validator/rules.rs`** (<450 LOC): JSON Schema validator compiler and rules.
- **`map_glyphs/manifest.rs`** (<400 LOC) & **`map_glyphs/rules.rs`** (<380 LOC): NATO MIL-STD-2525 map glyph catalog validation.
