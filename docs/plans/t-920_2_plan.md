# T-920.2 — main_goal on top, viewer beside detail: plan

## Context

main_goal landed in the schema (T-920.1) but renders in the old user_story slot; the
markdown viewer replaces the detail region instead of standing beside it.

## Approach

Move MainGoal out of the body section list into the detail header directly under the
title. Split the right pane into detail + optional viewer columns; Back collapses the
viewer; column width persists in eframe Storage. Card tooltip gains main_goal.

## Risks

- Narrow windows: two columns must degrade (minimum widths, viewer wins focus or
  collapses) rather than clip.
- Layout pin churn: the T-918.3 order test must move deliberately, not loosen.

## Verification

- Layout model test-pinned (main_goal in header, body starts at context, viewer-beside
  model); cargo test/build -p ticketboard green; porcelain unchanged after a session.
