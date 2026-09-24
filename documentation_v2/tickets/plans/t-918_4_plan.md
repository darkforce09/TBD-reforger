# T-918.4 — In-app markdown viewer: plan

## Context

Spec, plan and citation paths in the detail panel currently open via xdg-open — an
external hop. The T-917.6 plan documents make in-board reading the primary flow: the
operator should click a plan and read it without leaving the ticketboard.

## Approach

New `egui_commonmark` dependency (named supply-chain event, version + lock delta
reported like eframe at T-915.1). A viewer pane opens on spec/plan/citation click,
rendering the markdown file read-only; raw-text fallback with a note when parsing or
reading misbehaves; external-open stays available as the secondary action. Repo-root-
relative resolution through the existing discovery plumbing; loads on a worker thread
like every other IO path.

## Risks

- Dependency tree size/duplication from egui_commonmark — report the resolved delta;
  reject the slice if it drags a second egui version.
- Large or pathological markdown files freezing the UI thread — read + parse off-thread,
  cap rendered size with an explicit truncation notice.

## Verification

- `cargo test -p ticketboard` green; viewer model unit-tested (fallback predicate,
  path resolution, truncation cap).
- Clicking a plan doc renders headings/lists/code legibly; broken file shows raw text
  with a note, never a crash; external-open still offered.
- `git status --porcelain` unchanged after a full session; dependency delta pasted in
  the slice report.
