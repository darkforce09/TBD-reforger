**Status:** live

# Ballistics agreement bench

The `/debug/ballistics-agreement` bench: it solves a seeded lattice of battery fire problems with
the fire-mission solver compiled to WebAssembly, in the browser, and writes every answer with the
bit pattern of every number. It exists so the gate `gate ballistics-agreement` can prove that the
mortar calculator's in-browser solutions equal the solutions the [API](/documentation/glossary/a_to_f.md#api)
re-solves natively when a fire mission is saved.

## Where it lives

- Code: the route component `BallisticsAgreementPage` and its defaults in
  `apps/frontend/src/v2/apps/debug/ballistics_agreement.rs`; the reading, the URL parser
  and the browser host in
  [`apps/frontend/src/v2/apps/debug/ballistics_agreement/`](/apps/frontend/src/v2/apps/debug/ballistics_agreement/),
  whose [README](/apps/frontend/src/v2/apps/debug/ballistics_agreement/README.md) describes
  each file.
- Entry: the route, its URL parameters and its layout are in the README's
  [Routes](/apps/frontend/src/v2/apps/debug/ballistics_agreement/README.md#routes). No
  navigation entry links it; it is opened by URL.
- Related: the gate in
  [`tools/developer_tools/src/browser_testing/ballistics_agreement/`](/tools/developer_tools/src/browser_testing/ballistics_agreement/README.md),
  run by `cargo xtask mk ballistics-wasm-agreement`; the solver in
  `legacy/map_engine/src/data/scenario/ballistics/`, whose `agreement_cases` draws the
  lattice.

## Behaviour

The bench reads the catalog list, chooses the version the URL names (or the newest version of the
lowest catalog id), reads that version's document and refuses one that names another version. It
draws `count` cases from `seed` over that catalog, solves each with the one fire-mission
assembler, yields to the browser between cases, and writes the whole reading into
`<pre data-ballistics-agreement>`. The state attribute `data-ballistics-agreement-state` goes from
`loading` to `ready`, or to `failed` with the cause on the status line. The texts are in the
README's [States](/apps/frontend/src/v2/apps/debug/ballistics_agreement/README.md#states).
The bench needs no sign-in.

## Data

The bench reads `GET /api/v1/ballistics-catalogs` and
`GET /api/v1/ballistics-catalogs/{catalogId}/versions/{version}` without credentials and writes
nothing. Under the gate both reads are answered with the captured goldens
`contracts/fixtures/api_goldens/GET__ballistics-catalogs.json` and
`GET__ballistics-catalogs__vanilla_mortars__versions__1.json`, which the gate first proves to be
the committed catalog `contracts/catalogs/ballistics/vanilla_mortars.v1.catalog.json`.

## Design

- Chromeless and full-bleed: a title, a status line and the reading as preformatted text.
- A verification instrument, not a product page: no visual reference set exists.

## Decisions

- Every number also travels as its IEEE 754 bit pattern, so the gate's exact comparison never
  depends on a decimal round trip.
- The gate passes a case within 1 weapon mil and 0.1 s of the native solution, the tolerance the
  API applies to a client solution, and reports separately how many cases agree bit for bit.
- The case lattice is the map engine's, drawn identically on every target, so the bench and the
  gate need only the seed and the count to solve the same problems.
