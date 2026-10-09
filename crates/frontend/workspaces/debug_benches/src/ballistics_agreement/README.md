# Ballistics agreement bench

The `/debug/ballistics-agreement` bench: it solves a seeded lattice of battery fire problems with
the fire-mission solver compiled to WebAssembly, so the native agreement gate can prove the
browser build gives the same answers as the native one. It reads the public ballistics catalog
routes, solves in the page and writes one JSON reading; it has no canvas and no controls.

## Contents

```text
crates/frontend/workspaces/debug_benches/src/ballistics_agreement/
├── agreement_report.rs  `AgreementReport`, `case_report`, `assemble_report`: one solved case and the whole reading
├── bench_query.rs       `parse_bench_query` and `choose_catalog_version`: the URL and the catalog version
└── live.rs              the browser half: catalog reads, one solve per macrotask, the finished reading
```

## How it works

`BallisticsAgreementPage`, in the module root
`crates/frontend/workspaces/debug_benches/src/ballistics_agreement.rs`, renders the state attribute,
the status line and `<pre data-ballistics-agreement>`, and hands its signals to `live::run`.
`live.rs` parses the URL with `parse_bench_query`, reads `GET /api/v1/ballistics-catalogs`, picks
the version with `choose_catalog_version`, reads that version's document and refuses one that
names another catalog or version. It draws the cases with `ballistics_agreement_cases::agreement_cases`
(the same seed and count give the same cases on every target) and solves each with
`solve_fire_mission` through `case_report`, yielding to the event loop between cases.
`case_report` maps the case with `fire_mission_inputs` of the same crate, restates the lead gun
with `lead_summary` and records the bits with `case_bit_patterns`, the same functions the gate
calls. The
finished reading is serialised into the `<pre>`.

Each case of the reading holds its `case_id`, the `inputs` it was solved from, the `solution`
(or the assembler's `refusal`), the lead gun's recommended rings and time of flight, and
`bit_patterns`: every `f64` of `{"inputs", "solution"}` by JSON pointer, as the 16 hexadecimal
digits of its IEEE 754 bits. The gate `gate ballistics-agreement` of
`tools/browser_testing/browser_gate_suites/src/ballistics_agreement/` compares those with its own
native solves.

## Routes

| Route | Component | Access | Layout |
|---|---|---|---|
| `/debug/ballistics-agreement` | `BallisticsAgreementPage`, in the module root | route tier `none`; no sign-in needed | full-bleed, chromeless; no navigation entry |

The URL parameters are `?seed=` (decimal, default 1), `&count=` (1 to 512, default 32),
`&catalog=` (a catalog slug; default the lowest catalog id listed) and `&version=` (default the
newest listed version of that catalog; needs `catalog`). A malformed parameter fails the run.

## Data

- `GET /api/v1/ballistics-catalogs`, the public catalog list.
- `GET /api/v1/ballistics-catalogs/{catalogId}/versions/{version}`, the chosen catalog document.
- Both are read without credentials; the bench writes nothing and reads no storage.

## States

| `data-ballistics-agreement-state` | What the viewer sees |
|---|---|
| `loading` | "reading the catalog list…", "reading <catalog> v<n>…", then "solving case <i> of <n>…" |
| `ready` | "done", and the whole reading in the `<pre>` |
| `failed` | the cause on the status line (a malformed parameter, an unreadable or unlisted catalog, a mismatched document); the `<pre>` stays empty |

## Boundaries

- Depends on: `ballistics_agreement_cases` (the draw, the case-to-inputs mapping, the lead
  summary and the bit walk), `fire_mission_planning` (`fire_mission`) and `ballistics_model`
  (`catalog`); `frontend_transport::client::public_reads::public_get` and the
  catalog DTOs of `frontend_api_dtos::ballistics_catalogs`; `gloo_timers`, `serde_json`
  and `web_sys` in the browser build.
- Used by: the `/debug/ballistics-agreement` route in `crates/frontend/shell/frontend_application/src/app_routes.rs`,
  with its row in `crates/frontend/foundation/frontend_route_table/src/routes.rs`; the gate
  `tools/browser_testing/browser_gate_suites/src/ballistics_agreement/`, which mirrors the
  reading's shape.
- Rules: the case-to-inputs mapping, the lead summary and the bit walk live once, in the map
  engine's `agreement_cases.rs`, and are tested there; the reading's shape must stay equal to its
  decoder in the gate, whose strict decoding fails on a divergence. The bench imports no page and
  never persists.

## Related documentation

- [Ballistics agreement bench](/documentation/crates/frontend/workspaces/debug_benches/ballistics_agreement_page.md) —
  the bench's purpose and behaviour.
