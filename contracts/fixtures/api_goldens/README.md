# Recorded API responses

The [API](/documentation/glossary/a_to_f.md#api)'s answers to one route each, recorded over the
committed development seeds, with the request bodies of the writes that produced them. The
single-page app's typed DTOs, the API's contract tests and the headless browser gates all read the
same files, so a response shape cannot change on one side without a test on the other failing.

## Contents

```text
contracts/fixtures/api_goldens/
├── *.json      a response body, `<METHOD>__<path with / as __>.json`; a write's request body ends `.request.json`
├── *.sse.txt   the leading frames of an event stream, byte for byte
└── _index.tsv  one row per recording: status, route, file name, size; reads first, then writes
```

## Format

- `_index.tsv`: tab-separated, four columns (HTTP status, request path under `/api/v1`, the
  response file, its size), no header. The rows run in capture order: every read, then every
  write in the order it is sent, so a write's response reflects the writes before it.
- Responses: UTF-8 JSON as the API serialises it; identifiers and timestamps come from
  `apps/api/seeds/content_golden.sql`, which pins every one of them.
- Event streams: the raw `text/event-stream` bytes of the stream's first frames.
- Adding a recording: add its index row, the seed rows it reads, its request body when it writes,
  and capture it with the recipe that closes `content_golden.sql`.

## Producers and consumers

- Producers: the capture recipe at the end of `apps/api/seeds/content_golden.sql`, run against a
  fresh database seeded with `registry_dev.sql` and then `content_golden.sql`.
- Consumers: the API's `contract_parity_goldens` test binary (`apps/api/tests/`), which replays
  every row against the live router and checks each body against its route's schema in
  `contracts/definitions/`, and its other contract tests that embed single files; the frontend's
  DTO golden tests and page tests under `apps/frontend/src/`; the headless browser gates of
  `tools/developer_tools/`, which answer the app's requests from these files.

## Boundaries

- Depends on: `apps/api/seeds/content_golden.sql` and the schemas in `contracts/definitions/`.
- Used by: the API's contract tests, the frontend's DTO and page tests, and the browser gates.
- Rules: every recording reproduces from the seed through the capture recipe
  (`cargo xtask db test-it --test contract_parity_goldens`); a response shape change updates the
  recording, the frontend DTO and the schema in the same change.

## Related documentation

- [Contract fixtures](/contracts/fixtures/README.md) — every fixture folder and the gates that
  read it.
- [Development seeds](/apps/api/seeds/README.md) — the seed the recordings are captured from.
