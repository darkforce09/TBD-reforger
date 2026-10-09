# Mission validation loop

The live validation of the open [mission](/documentation/glossary/g_to_m.md#mission): the loop that
re-runs the map engine's validation rules after each document change, the findings list the top
strip's findings chip drops down, and the routes that select the subject a finding names. The
finding model and the timing constants live in the parent module,
`crates/frontend/workspaces/mission_creator_workspace/src/ui/inspector/validation_panel.rs`, which declares these
modules.

## Contents

```text
crates/frontend/workspaces/mission_creator_workspace/src/ui/inspector/validation_panel/
├── findings_dropdown.rs                    the chip's list: rule groups by severity, the legend
├── payload_source_and_selection_routes.rs  the payload source, subject router and route probe
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
- `evaluate_now` runs `default_registry()` of `mission_validation` over
  the compiled payload, with the asset ids the item [registry](/documentation/glossary/n_to_z.md#registry)
  knows, and appends the findings the latest compile published through `publish_compile_findings`
  (the editor page registers the panel as the session's compile-findings publisher at mount with
  `register_compile_findings_publisher`, so Export Compiled reaches it without naming it);
  `clear_compile_findings` empties them when another mission loads. A rule that panics yields no
  findings for that pass.
- `findings_dropdown` groups the findings by rule, worst severity first, shows "No issues" when
  there are none, and ends with the severity legend. A finding whose subject the route probe
  resolves selects it on click; any other renders inert and says why.
- Every hook here is installed with the state layer's `install_seam`
  (`crates/frontend/workspaces/mission_creator_state/src/seam_registration.rs`): the owner that installed a hook clears it when
  it unmounts, and never a newer owner's, so a remounted editor inherits no stale callback.

## Boundaries

- Depends on: `mission_validation` (`default_registry`, `EvalContext`,
  `Finding`, `Severity`, `Primitive`); the parent module's model; `derive_object_alias` from the
  state layer's `asset_catalog` in `crates/frontend/workspaces/mission_creator_state/src/` and its
  `install_seam`; the publisher slot `register_compile_findings_publisher` in
  `crates/frontend/workspaces/mission_creator_session/src/compile_findings_publisher.rs`; `MaterialIcon`
  and the `RegistryItem` DTO from the foundation crates.
- Used by: the modules below, through the parent module's re-exports:
  - `crates/frontend/workspaces/mission_creator_workspace/src/mission_editor.rs`, which mounts `ValidationPanel`,
    and, in `crates/frontend/workspaces/mission_creator_workspace/src/mission_editor/`, `canvas_mount.rs`,
    `canvas_mount/boot_tasks.rs` and `canvas_mount/review_restore.rs`, which register the hooks
    and clear the compile findings;
  - the export in `crates/frontend/workspaces/mission_creator_session/src/document_commands/imp/exports.rs`,
    which publishes compile findings through the registered publisher;
  - the top strip in `crates/frontend/workspaces/mission_creator_workspace/src/ui/docks/top_strip/`, for the chip
    and the dropdown; the left dock, the outliner rows and the All Settings dialog, which select
    subjects through `route_select_by_subject_id` and `subject_id_routes`.
- Rules: the seam mechanism is defined once in the crate, and an older owner's cleanup never clears
  a newer registration; a finding row is clickable exactly when the router resolves its subject
  (`a_finding_row_is_clickable_iff_the_router_resolves_its_subject` in
  `crates/frontend/workspaces/mission_creator_workspace/src/ui/inspector/tests/validation_panel/finding_route_probe.rs`).

## Related documentation

- [Mission Creator feature inventory: top command strip](/documentation/crates/frontend/workspaces/mission_creator_workspace/feature_inventory/top_command_strip.md) — the validation chip and its findings.
