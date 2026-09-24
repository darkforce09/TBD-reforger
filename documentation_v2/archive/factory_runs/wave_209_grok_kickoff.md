# Editor wave 209 — kickoff for Grok (Cursor factory command center)

**Written 2026-08-09 by the wave-200 orchestrator, on operator instruction.** You are running ONE
wave that fixes the nine adversarial-verifier findings from editor wave 200. Process authority:
[`FACTORY_FOR_CURSOR.md`](FACTORY_FOR_CURSOR.md) (phase 0 cold start, worktree lifecycle, reject
conditions) — this file only adds the wave-209 specifics. **You never implement; slice agents in
worktrees do.**

## What this wave is

Wave 200 shipped T-785/786/787 (gate PASS 30/30) and its Fable adversarial verifier then re-ran
the acceptance measurements with real per-char CDP input and found 4 MAJOR / 3 MINOR / 2 NIT.
**Read the report first: [`.ai/artifacts/editor_verify/wave200.md`](../../.ai/artifacts/editor_verify/wave200.md).**
Its "verified-clean register" is your MUST-NOT-BREAK list; its findings F1–F8 are these tickets:

| Ticket | Findings | Owns (touch ONLY) |
|---|---|---|
| T-811 | F1 (MAJOR) — the real layer rename (eden_tree.rs) never focuses; wave 200 fixed the wrong widget | `apps/website/frontend/src/eden_tree.rs` |
| T-812 | F2 (MAJOR) + F7 — bookmark rename truncates to last char; bookmark ADD input never focuses | `apps/website/frontend/src/eden_dock_left.rs` |
| T-813 | F3 (MAJOR) + F6 — multi-edit differing-field focus+blur with zero typing wipes the selection; field Esc also closes the modal | `apps/website/frontend/src/attributes.rs` |
| T-814 | F4 (MAJOR) + F5 — transients survive non-strip dialog opens; one Esc closes two layers; Esc dismisses the hidden dialog | `apps/website/frontend/src/ui.rs; apps/website/frontend/src/eden_top_strip.rs` |
| T-815 | F8 both NITs — ORBAT squad-rename opens unfocused; plus delete the two `T787 *Probe` dev missions (ids in the ticket) | `apps/website/frontend/src/orbat_manager.rs` |

Briefs: the registry `summary` **is** the brief — paste it verbatim into each slice dispatch
(`./scripts/ticket brief T-8xx`). Owns are disjoint (verified via `cargo xtask slice-collisions`).
Plan rows are filed at wave 209 in `wave_plan.tsv`.

## Wave shape

- Dispatch ≤3 slices at once (your runbook's cap); **one wave**: all five report → merge ALL five
  (`git merge --no-ff slice/T-8xx -m "T-8xx: <title>"`) → ONE gate → stop.
- Slice rules: paste the HARD RULES block from
  [`EDITOR_SLICE_BRIEF.md`](EDITOR_SLICE_BRIEF.md) verbatim (rule 9's tool names are
  Claude-harness-specific — your agents follow your runbook's discipline instead; everything else
  binds, especially the per-slice private test dir and `SLICE GATE: PASS`). The FINAL REPORT
  schema there is mandatory per slice.
- Gate: `TBD_WAVE_GENERATION_FLOOR=100 TBD_GATE_WAVE=209 bash scripts/platform/wave.sh gate`.
  Close markers 122+ have no plan rows, so the gate WILL demand
  `TBD_GATE_BASE_CONFIRM=<prev close sha>` — verify that sha equals
  `git log --grep='^wave [0-9]\+ CLOSED' -1 --format=%H` (the wave-122 close), then pass it. That
  demand is documented-benign here, not a signal.

## Verification bar (the wave-200 lesson)

The F1 defect shipped because a slice with no browser wrote source-pins that pinned the WRONG
widget and stayed green. Therefore:
- Every ACCEPTANCE clause in these tickets cites the verifier's exact measured values — slices
  must re-run the **measurement** with real-input-shaped probes (per-char key events, real
  clicks), not synthetic-only events, against the live SPA on :3000.
- Reference harness: `tools/tbd-tools/src/smokes.rs` idioms (the gate's own CDP driver). A
  throwaway driver + probe specs from the wave-200 passes exist at
  `/tmp/claude-1000/-home-Samuel/f79c73b6-e03c-49f1-8e14-6b61c7dc9af4/scratchpad/` (`cdp.py`,
  `785*.json`, `786*.json`, `esc.json`; also `fixpass_round1_partial.diff` — an UNVERIFIED partial
  diff from a dead fix attempt: consult freely, trust nothing, `/tmp` may vanish on reboot).
- T-813 and T-814 are two halves of one Esc ladder (field-level consume vs modal-level
  consume-aware). Their combined acceptance (each Esc press closes exactly one layer, field draft
  first, visual-top dialog next) can only be probed AFTER BOTH merge — do that before the gate.
- If a slice changes a commit seam the a2i smoke asserts (`smokes.rs` — blur-commit since
  bef0a071), update the smoke in the same slice and say so; `smokes.rs` is unowned this wave.

## Hard boundaries

- **STOP after GATE PASS.** Do NOT write a close commit, do NOT touch the marker ledger, do NOT
  flip registry rows to shipped, do NOT dispatch wave 201. The Fable adversarial verifier, the
  operator eye-pass, and the close run from the Claude Code command center (band amendment,
  `EDITOR_FACTORY_START.md` §UX remediation band). Report the merged HEAD sha + gate tail back to
  the operator.
- Never `--repack` the wave plan. No `.py` committed. Stage explicit paths. The six parked mod
  worktrees (T-702/T-212/T-654/T-673/T-674/T-675) are load-bearing — never reap/drop/merge them.
- `yrs_persist.rs` and every file not in your ticket's owns: read-only.
