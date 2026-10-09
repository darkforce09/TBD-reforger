# Ballistics WebAssembly agreement gate

`gate ballistics-agreement`: proves that the fire-mission solver compiled to WebAssembly and
running in the browser gives the same answers as the native build. The browser bench
`/debug/ballistics-agreement` solves a seeded lattice of battery fire problems from the served
catalog; the gate solves the same lattice natively from the committed catalog and compares them
case by case.

## Contents

```text
tools/browser_testing/browser_gate_suites/src/ballistics_agreement/
├── bench_reading.rs      `BenchReading`, `BenchCase`: the bench's JSON reading and its strict decoder
├── browser_session.rs    `read_bench`: the server, Chromium, the catalog reads answered, the `<pre>` read
├── case_verdict.rs       `judge_reading`, `AgreementVerdict`: the per-case judgement and the printed lines
├── golden_provenance.rs  `check_served_goldens`: the catalog goldens proved the committed catalog
├── mod.rs                `run` and `AgreementArgs`: the steps in order and the exit code
└── native_reference.rs   `native_cases`: the same cases drawn, mapped and solved on the host
```

## How it works

```text
contracts/catalogs/ballistics/vanilla_mortars.v1.catalog.json ──▶ native_cases(seed, count)
contracts/fixtures/api_goldens/GET__ballistics-catalogs*.json
   └─ check_served_goldens ──▶ Fetch.requestPaused answers ──▶ /debug/ballistics-agreement
                                                                 └─ <pre data-ballistics-agreement>
judge_reading(native, reading) ──▶ case ballistics_wasm_agreement_<id> ... ok|FAILED
```

1. `run` reads and decodes the committed catalog
   `contracts/catalogs/ballistics/vanilla_mortars.v1.catalog.json`; its `catalog_id` and
   `catalog_version` name the run.
2. `check_served_goldens` reads `GET__ballistics-catalogs.json` and
   `GET__ballistics-catalogs__<catalog_id>__versions__<version>.json` from
   `contracts/fixtures/api_goldens/`: the list must name the version with the SHA-256 of
   the committed file's bytes, and the document must decode to the committed catalog (the API
   re-serialises the stored document, so it is compared decoded).
3. `native_cases` draws the agreement cases with `ballistics_agreement_cases::agreement_cases`, turns each
   into fire-mission inputs with its `fire_mission_inputs` and solves them with
   `solve_fire_mission`, recording every `f64` of the inputs and the solution by JSON pointer and
   bit pattern with its `case_bit_patterns`: the functions the bench calls.
4. `read_bench` serves the built app, launches headless Chromium, bypasses the offline service
   worker, answers the two catalog reads with the golden bytes and every other `/api/v1/` read
   with 404, opens `/debug/ballistics-agreement?seed=&count=&catalog=&version=` and waits for
   `data-ballistics-agreement-state` to leave `loading`.
5. `judge_reading` holds the reading to the run (seed, count, catalog, version, solver revision,
   case count) and each case to the native case at its index: the same id, the same inputs bit
   for bit, bit patterns and a lead-gun summary that restate its own values, and a solution within
   1 weapon mil and 0.1 s of the native one (`compare_solutions`), or the same refusal.
6. The gate prints one `case ballistics_wasm_agreement_<id> ... ok|FAILED` line per case, with
   the causes indented, then `bit-identical cases: <k>/<n>` and
   `ballistics-wasm-agreement: PASS <n>/<n>` or `FAIL <agreeing>/<n>`.

| Option | Default | Meaning |
|---|---|---|
| `--dist` | `crates/frontend/shell/frontend_application/dist` | the built app; relative paths resolve against the repository root |
| `--seed` | 1 | seed of the case lattice |
| `--count` | 32 | number of cases |
| `--catalog` | `contracts/catalogs/ballistics/vanilla_mortars.v1.catalog.json` | the committed catalog |
| `--api-goldens` | `contracts/fixtures/api_goldens` | the folder of captured API goldens the catalog reads are answered from |
| `--port`, `--debug-port` | 5407, 9407 | the static server and Chromium's debugging port |
| `--timeout-s` | 600 | longest wait for the bench |

Exit codes: 0 every case agrees and at least one ran; 1 a golden is missing or is not the
committed catalog, the bench failed or timed out, the reading does not decode, the run differs,
or a case disagrees; 3 the committed catalog is unreadable or a driver error.

## Boundaries

- Depends on: `ballistics_agreement_cases`, `ballistics_model` (`catalog`) and
  `fire_mission_planning` (`fire_mission`, `fire_mission_comparison`); the crate's `server` and
  `Error`, and `chrome_devtools_protocol`; `content_digest`, `serde`, `serde_json`, `tokio`.
- Used by: `command_lines::gate` (`gate ballistics-agreement`) and
  `cargo xtask mk ballistics-wasm-agreement`, which runs `trunk build --release` first.
- Rules: the case-to-inputs mapping, the lead summary and the bit walk are the
  `ballistics_agreement_cases` crate's (`crates/ballistics/ballistics_agreement_cases/`, tested there),
  called by both the gate and the bench, so they cannot drift; `bench_reading.rs` mirrors the
  reading of `crates/frontend/workspaces/debug_benches/src/ballistics_agreement/agreement_report.rs`
  and decodes it strictly. A missing golden fails the gate; nothing is skipped.

## Related documentation

- [Ballistics agreement bench](/crates/frontend/workspaces/debug_benches/src/ballistics_agreement/README.md) —
  the browser half and its reading.
- [Build and development-server commands](/tools/commands/ci_task_catalog/src/build_lane/README.md) —
  `cargo xtask mk ballistics-wasm-agreement`.
