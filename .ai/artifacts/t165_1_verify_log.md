# T-165.1 — text/JSON schema gates → xtask — verify log

**Executor:** claude-code. **Result: PASS.** Port of 8 `packages/tbd-schema/scripts/*.mjs` gates
(~1,020 LOC JS) into `xtask/src/schema_gates.rs` (`cargo xtask schema <gate>`); `jsonschema` 0.46
(the crate already proven in `apps/website/src/contract/validate.rs`) replaces ajv for the two
schema-compiled gates.

## Side-by-side parity (Node vs Rust, same tree)

| Gate | Node (rc, verdict) | Rust (rc, verdict) | Match |
|------|--------------------|--------------------|-------|
| citations | 0 · "Checked **16** @contract citation(s)" · resolve | 0 · "Checked **16**" · resolve | ✓ (count parity) |
| t090-specs | 0 · OK (36 files, 12 gates) | 0 · OK (36 files, 12 gates) | ✓ |
| n6 | 0 · OK (5 locations) | 0 · OK (5 locations) | ✓ |
| n10 | 0 · OK | 0 · OK | ✓ |
| map-object-enums | 0 · OK | 0 · OK | ✓ |
| type-inventory | 0 · OK | 0 · OK | ✓ |
| terrain-manifest (everon) | 0 | 0 | ✓ |
| **Negative probes** | n6 with a mutated spec sentence: node rc=1 / rust rc=1 → restored green both. `--terrain bogus`: node rc=2 / rust rc=2 | | ✓ |

Retirements printed by the Rust citations gate (were silent no-ops in Node): TS-6 (React contract
layer deleted at T-159.29.3) + GO-7 (Go handlers deleted at T-145).

## Rewiring

- Makefile: `schema-validate` runs the 5 ported verify gates via xtask (validate.mjs +
  golden + glyphs + height-labels stay Node until .2/.4); `verify-citations` → xtask;
  `verify-terrain`/`verify-terrain-strict` split (manifest half → xtask, alignment stays Node
  until .4).
- ci.yml schema job: citations step → `cargo run -q -p xtask -- schema citations`
  (+ rust-toolchain + rust-cache; timeout 10→15 min). contracts.yml citations job → cargo
  (Node toolchain dropped from that job).
- package.json: the 7 retired gate scripts removed; `verify-terrain` = alignment only.
  Gate-7 (command existence) frozen allowlist extended with the retired npm script names so
  historical spec quotes stay archival-valid — re-verified 12/12.

## Deletions (reverse-dep edge list applied)

Deleted (nothing imports or spawns them): `verify-contract-citations.mjs`,
`verify-t090-spec-consistency.mjs`, `verify-n6-sentence.mjs`, `verify-n10-tile-budget.mjs`,
`verify-map-object-enums.mjs`, `verify-terrain-manifest.mjs`, `flatten-orbat-slots.mjs`.
**Kept:** `verify-type-inventory.mjs` — still spawned by path from `census-types.mjs:24` and
`validate-export-artifacts.mjs` (both Node until T-165.8); its Makefile invocation flipped to the
Rust twin.

## Gates

`make schema-validate` 0 FAIL/ERROR · `make verify-citations` green (resolve) ·
`cargo clippy -p xtask -- -D warnings` clean · fmt clean · `ticket check` exit 0 ·
t090 command-existence gate 12/12 post-rewire.
