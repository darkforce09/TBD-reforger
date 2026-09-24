# Mission validation loop

The live validation of the open [mission](/documentation_v2/glossary.md#mission): the loop that
re-runs the map engine's validation rules after each document change, the findings list the top
strip's findings chip drops down, and the routes that select the subject a finding names. The
finding model and the timing constants live in the parent module,
`apps/website/frontend/src/v2/apps/editor/ui/inspector/validation_panel.rs`, which declares these
modules.

## Contents

```text
apps/website/frontend/src/v2/apps/editor/ui/inspector/validation_panel/
├── findings_dropdown.rs                    the chip's list: rule groups by severity, the legend
├── payload_source_and_selection_routes.rs  the payload source, subject router and route probe
├── seam_registration.rs                    `install_seam`: a hook that only its owner clears
├── validation_evaluation_and_findings.rs   runs the rules; keeps compile findings and the sink
└── validation_runtime.rs                   `ValidationPanel`: the debounced loop; draws nothing
```

## How it works

```text
canvas mount ──registers──> payload source, route probe, select router
document change ──doc_tick──> ValidationPanel ──250 ms after the last change──> evaluate_now
evaluate_now = map-engine rules over the payload + latest compile findings ──> sink signal
sink signal ──> top strip chip (Rollup) ──opens──> findings_dropdown ──click──> select router
```

- `ValidationPanel`, mounted by the editor page, registers its findings signal as the sink and
  evaluates as soon as the payload source answers (polled every `INITIAL_EVAL_TICK_MS`, 50 ms, at
  most `INITIAL_EVAL_MAX_TICKS`, 32 times), then `REEVAL_DEBOUNCE_MS`, 250 ms, after the last
  document change. It renders nothing itself.
- `evaluate_now` runs `default_registry()` of `website_map_engine::data::scenario::validate` over
  the compiled payload, with the asset ids the item [registry](/documentation_v2/glossary.md#registry)
  knows, and appends the findings the latest compile published through `publish_compile_findings`;
  `clear_compile_findings` empties them when another mission loads. A rule that panics yields no
  findings for that pass.
- `findings_dropdown` groups the findings by rule, worst severity first, shows "No issues" when
  there are none, and ends with the severity legend. A finding whose subject the route probe
  resolves selects it on click; any other renders inert and says why.
- Every hook here is installed with `install_seam`: the owner that installed a hook clears it when
  it unmounts, and never a newer owner's, so a remounted editor inherits no stale callback.

## Boundaries

- Depends on: `website_map_engine::data::scenario::validate` (`default_registry`, `EvalContext`,
  `Finding`, `Severity`, `Primitive`); the parent module's model; `derive_object_alias` from the
  [Arsenal](/documentation_v2/glossary.md#arsenal)'s `asset_catalog` in `apps/website/frontend/src/v2/apps/editor/arsenal/`; `MaterialIcon`
  and the `RegistryItem` DTO from `crate::v2::core`.
- Used by, through the parent module's re-exports:
  - `apps/website/frontend/src/v2/apps/editor/mission_editor.rs`, which mounts `ValidationPanel`,
    and, in `apps/website/frontend/src/v2/apps/editor/mission_editor/`, `canvas_mount.rs`,
    `canvas_mount/boot_tasks.rs` and `canvas_mount/review_restore.rs`, which register the hooks
    and clear the compile findings;
  - the export in `apps/website/frontend/src/v2/apps/editor/shell/document_commands/imp/exports.rs`,
    which publishes compile findings;
  - the top strip in `apps/website/frontend/src/v2/apps/editor/ui/docks/top_strip/`, for the chip
    and the dropdown; the left dock, the outliner rows and the All Settings dialog, which select
    subjects through `route_select_by_subject_id` and `subject_id_routes`;
  - `input/tools/ruler_tool.rs`, `input/tools/los_tool.rs` and `bridge/world_assets.rs` under
    `apps/website/frontend/src/v2/apps/editor/`, which install their own hooks with `install_seam`.
- Rules: the seam mechanism is defined once in the crate, and an older owner's cleanup never clears
  a newer registration (`the_seam_mechanism_is_defined_exactly_once_in_the_crate` and
  `an_older_owners_cleanup_does_not_clobber_a_newer_registration` in
  `apps/website/frontend/src/v2/apps/editor/ui/inspector/tests/validation_panel/registration_lifecycle.rs`);
  a finding row is clickable exactly when the router resolves its subject
  (`a_finding_row_is_clickable_iff_the_router_resolves_its_subject` in `finding_route_probe.rs`
  beside it).
