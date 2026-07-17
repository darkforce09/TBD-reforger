# T-165.3 — codegen → typify + loadout hand-freeze — verify log

**Executor:** claude-code. **Result: PASS.**

## What shipped

- `xtask schema codegen` (`xtask/src/codegen_schema.rs`): typify 0.7 + schemars 0.8 + prettyplease
  + rustfmt generate the 4 tractable contracts (registry-items, registry-compat,
  mission-editor-payload, faction-library) into `apps/website/src/contract/generated/`.
  **Run-to-run hash-idempotent (sha256 equal across consecutive runs).** typify output validates
  string `pattern`s via `regress` (new backend dep) — an enforcement upgrade over quicktype, which
  ignored them.
- `loadout.rs` **hand-frozen**: the quicktype output was provably lossy (merged the versioned root
  `oneOf`; `Wear {}` / `Equipment {}` empty). New faithful model: `#[serde(tag = "loadoutVersion")]`
  V1/V2, patternProperties wear/equipment as `BTreeMap<String, Option<String>>`, double-Option
  null-vs-absent for optic/magazine, `deny_unknown_fields` matching `additionalProperties: false`.
  **Value-level round-trip tests against BOTH committed fixtures** (parse → serialize → parse:
  JSON values equal; null-vs-absent preserved — the grenade row's absent optic stays absent).
- `registry_import.rs` adapted to the typify shapes (TbdRegistryItems/Item/TbdRegistryCompat/Edge,
  newtype `.to_string()` at collection sites, `abstract_` field).
- Rewired: Makefile `schema-codegen` → xtask; **contracts.yml codegen-drift job Node-free**
  (rustfmt + cargo only). Deleted: `codegen.mjs`; `quicktype` devDep + `codegen` npm script
  dropped; gate-7 archival allowlist += `codegen`.

## Gates

| Gate | Result |
|------|--------|
| codegen idempotence | run-to-run sha256 EQUAL |
| `cargo test -p reforger-backend --lib` | **34 passed** (32 prior + 2 loadout round-trips) |
| backend clippy `--all-targets -D warnings` | clean |
| xtask clippy `-D warnings` | clean |
| `make schema-validate` | OK |
| t090 command gates | 12/12 |
| `ticket check` | exit 0 |

Generated-file delta vs the quicktype era: 5 files, +2,731/−538 (typify's validated newtypes +
the faithful loadout model) — committed as the new drift baseline for the contracts.yml job.
