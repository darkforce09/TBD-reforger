# REPORT T-937.4 — Persist: surface save errors, flush on hide

## pwd_branch
`/run/media/system/Disk_2/Projects/TBD-Reforger/.ai/artifacts/worktrees/T-937.4` · `slice/T-937.4`

## defect_verified
`run_save` turned every `save_state_as` `Err` (quota included) into `web_sys::console::warn_1` and `return`. No `SaveStatus`, no toast, no chip — forcing that `Err` was not observable in editor state.

**RED VERBATIM** (status test against the pre-fix persist Err arm):

```
thread 'editor::state::save_status::tests::run_save_err_arm_reports_into_save_status' (4114341) panicked at apps/website/frontend/src/editor/state/save_status.rs:298:9:
forcing save_state_as Err currently shows no observable state — persist must report every Err into SaveStatus (quota named via format_save_error). arm=if let Err(e) = save_state_as(&pending.owner, id, &bytes).await {
        web_sys::console::warn_1(&JsValue::from_str(&format!(

        )));
        return;
    }
note: run with `RUST_BACKTRACE=1` environment variable to display a backtrace
test editor::state::save_status::tests::run_save_err_arm_reports_into_save_status ... FAILED

failures:

failures:
    editor::state::save_status::tests::run_save_err_arm_reports_into_save_status

test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 1330 filtered out; finished in 0.08s
```

## changes
- New `apps/website/frontend/src/editor/state/save_status.rs` (registered in `state/mod.rs`): `SaveStatus { Saved, Saving, Failed(reason), Unreadable(retries) }`, status chip, one toast per Failed episode, `format_save_error` names quota.
- `persist.rs` `run_save` reports every `save_state_as` `Err` via `report_failed(format_save_error(...))`; success → `Saved`; IO start → `Saving`.
- Idle debounce is `IDLE_DEBOUNCE_MS` (1000). `visibilitychange` hidden still calls `flush_state` immediately; `pagehide` stays fire-and-forget `spawn_local`. `SAVE_IN_FLIGHT` in `run_save` is the shared in-flight guard.
- `note_unreadable` retries the read three times with `setTimeout` backoff, then lockout; chip Retry re-runs those keys.

## perturbation
Swallowed `report_failed` in the `run_save` `Err` arm (console.warn + return only); restored; `touch`; green.

**red VERBATIM:**

```
thread 'editor::state::save_status::tests::run_save_err_arm_reports_into_save_status' (4178618) panicked at apps/website/frontend/src/editor/state/save_status.rs:299:9:
forcing save_state_as Err currently shows no observable state — persist must report every Err into SaveStatus (quota named via format_save_error). arm=if let Err(e) = save_state_as(&pending.owner, id, &bytes).await {
        web_sys::console::warn_1(&JsValue::from_str(&format!(

        )));
        return;
    }
    save_status::report_saved();
test editor::state::save_status::tests::run_save_err_arm_reports_into_save_status ... FAILED

test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 1331 filtered out; finished in 0.08s
```

**restored_green:** `editor::state::save_status` 8 passed after restore + `touch`.

## gate_verdict_tail
```
  no-python (T-620)        PASS

  gate verdict PASS @ 9f4a8c7d1dff recorded: .ai/artifacts/verdicts/T-937.4.json
SLICE GATE: PASS
```

(First gate was on dirty HEAD `9f4a8c7d1dff`. Commit `01887527f219` follows; re-gate so land SHA matches.)

## files_outside_owns
- `.ai/artifacts/editor_briefs/sept2026/wave253/REPORT-T-937.4.md` (this report, required by the brief)

## found_not_fixed
- Chip is not wired into `top_strip.rs` / `mission_editor.rs` (sibling-owned). It self-mounts an overlay from `bind_runtime` / `report` on wasm; shell `Toasts` is used when a reactive owner has context, otherwise a fallback toast node.
- `pagehide` remains fire-and-forget by spec (does not await the write).
- Existing `console.warn` on T-221 orphans and T-374 unreadable *refuse* is still there in addition to the chip.

## deviations
- Brief `verify` listed `ci-local-leptos` and `leptos-gates`; the slice brief forbids both. Gate used is `cargo xtask platform wave gate --slice T-937.4` only.

## commits
- `01887527f219e012b79ab44949f9986cec31e011` T-937.4: surface persist save failures on chip and toast

## manual_checklist
Fill localStorage/IDB to quota, edit, observe toast + Failed chip naming quota; switch tabs, reload, edits present; unreadable store retries three times then Retry.
