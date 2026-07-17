# T-165.2 — validate suite → Rust — verify log

**Executor:** claude-code. **Result: PASS.** `validate.mjs` (315 LOC) + `validate-file.mjs`
(70 LOC) ported into `xtask schema validate` / `schema validate-file`.

## Port shape

- Cross-file `$ref`s: `referencing::Registry` keyed by each schema's `$id` (the `ajv.addSchema`
  equivalent) + `jsonschema::options().with_registry(...)`. 10 registered schemas (9 map-object +
  mission); ENF-4 pointer validators built as `{"$ref": "<mission $id>#/$defs/<name>"}`.
- All ~50 checks ported: golden missions (dir-sorted), registries, items (schema + addon
  provenance + variant_of FK walkers), compat (schema + edge referential integrity), faction
  library, loadout v1+v2, editor payload, bridge samples, terrain manifests (live + 3 goldens) +
  anchors, locations, height labels, ENF-4 fixtures, map-object prefabs/instances/chunk/regions/
  roads/catalog/resolved/terrain-registry/type-inventory.

## Parity (side-by-side, same tree)

| Metric | Node | Rust |
|--------|------|------|
| exit code | 0 | 0 |
| PASS lines | **130** | **130** |
| FAIL lines | 0 | 0 |
| PASS label set (sorted diff) | — | **empty diff** |
| Negative probe (golden mission `schemaVersion` mutated) | rc=1 | rc=1 → restored, both green |
| validate-file on a golden | rc=0 "ok" | rc=0 "ok" |

## Rewiring + deletions

- Makefile `schema-validate`: first step = `cargo run -q -p xtask -- schema validate` (npm chain
  keeps only golden/glyphs/height-labels until .4).
- **ci.yml schema job = Node-free** (setup-node + npm ci dropped; both steps cargo).
- **schema.yml workflow = Node-free** (cargo validate).
- `deploy-staging.sh` V1 mission gate → `xtask schema validate-file` (npm ci line dropped).
- Deleted: `validate.mjs`, `validate-file.mjs` (no remaining spawners — grep-verified).
  package.json `validate` script removed; gate-7 frozen allowlist covers the retired npm names
  (re-verified 12/12; the allowlist block also regression-hardened with an assert after an edit
  churn ate it once — caught by the gate itself).

## Gates

`make schema-validate` tail OK · t090 gates 12/12 · `ticket check` exit 0 ·
`cargo clippy -p xtask -- -D warnings` clean · fmt clean.

CI note: the T-165.0/.1 push (`9d97f05b`) CI result — watcher output was empty at close
(runs still pending when checked); the .2 push watch below supersedes it (all workflows on the
newer commit).
