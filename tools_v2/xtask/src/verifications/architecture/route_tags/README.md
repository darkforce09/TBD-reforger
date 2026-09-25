# Route tag gate parts

The parts of `cargo xtask verify route-tags` below its constants: reading the API's route tables
and `@route` doc tags out of source, the fixed sort order of the report, and the runner that
compares the two sides. The parent file `tools_v2/xtask/src/verifications/architecture/route_tags.rs`
holds the paths, patterns, sentinels and report text.

## Contents

```text
tools_v2/xtask/src/verifications/architecture/route_tags/
├── collate_cmp.rs               a locale-free comparison that fixes the report's sort order
├── route_and_tag_extraction.rs  discovers the domain route tables, parses registrations and tags
└── verify_route_tags.rs         the entry point: probes, router shape pins, both directions, summary
```

## How it works

`verify_route_tags.rs` probes its literal matcher, then pins the shape of
`apps/website/api_v2/src/core/http_router.rs`: it defines `fn api_v1_routes` and nests it at
`/api/v1`. `route_and_tag_extraction.rs` finds every `apps/website/api_v2/src/<domain>/routes.rs`
holding exactly one column-0 `pub fn routes(`, reads each `.route(` registration into
`METHOD PATH HANDLER` rows, reads the `.merge(crate::<domain>::routes(` lines of
`api_v1_routes`, and sweeps every `.rs` file under `apps/website/api_v2/src` for column-0
`/// @route METHOD PATH` tags on the `pub fn` below them. A line it cannot read becomes an
`UNPARSED` or `ORPHAN` row instead of disappearing.

The runner then fails on any unparsed or orphan row, any route table not merged or merge without a
table, and any of the four sentinel routes missing from either side, before it compares
direction A (every tag names a registered method, path and handler) and direction B (every
registered route carries a matching tag). Both lists print in `collate_cmp` order, the same bytes
on every machine.

## Boundaries

- Depends on: the parent's constants; `verification_core` (`scan::walk_files`, `Pattern`,
  `gate::probe_str`, `gate::require_str`, `Verdict`); the `regex` crate. It reads the working tree,
  so an untracked handler file counts.
- Used by: the parent module, which re-exports `verify_route_tags` for
  `tools_v2/xtask/src/commands/verify/dispatch.rs`, for the `verify-coding-standards` row of
  `tools_v2/xtask/src/commands/ci/task_definitions.rs`, and for the `route tags` row of the wave
  gate's `VERIFY_STEPS` in `tools_v2/xtask/src/commands/platform/wave_execution/gate.rs`; the
  parent's tests call `run` and `extract_tags`.
- Rules:
  - output goes into a line buffer, so the tests compare exact lines
    (`clean_tree_passes_and_counts_exactly` in
    `tools_v2/xtask/src/verifications/architecture/tests/route_tags/tests.rs`);
  - exit 1 for a failed probe or pin, an unread registration or tag, a mount mismatch or a
    tag and route that do not match; exit 2 when `http_router.rs` or the source tree could not be
    read (`inputs_that_were_never_read_do_not_pass`);
  - zero route tables is a failure, never a pass (`zero_route_files_is_not_a_pass`);
  - the sort order reproduces the measured glibc `en_AU.UTF-8` order with no locale input
    (`collation_reproduces_measured_glibc_order`).
