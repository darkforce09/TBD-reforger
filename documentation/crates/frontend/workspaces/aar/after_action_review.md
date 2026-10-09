**Status:** live

# After-action review

A planned workspace, not built: a replay of a finished match from server telemetry over a
read-only map, with a timeline scrubber, so members and leaders can review what happened in an
[event](/documentation/glossary/a_to_f.md#event)'s [mission](/documentation/glossary/g_to_m.md#mission)
after it ends.

## Where it lives

- Code: none. The workspace is built as its own crate under `crates/frontend/workspaces/`
  ([frontend workspace crates](/crates/frontend/workspaces/README.md)); no crate, module or code
  folder exists for it yet.
- Entry: none. `crates/frontend/shell/frontend_application/src/app_routes.rs` and `crates/frontend/foundation/frontend_route_table/src/routes.rs`
  have no replay route.
- Related features: the [deployments page](/documentation/crates/frontend/pages/operations_pages/deployments/deployments_page.md),
  whose service record links each match's external replay; the
  [match telemetry domain](/crates/api/api_match_telemetry/src/README.md), which takes in the
  match results.

## Behaviour

None: no code replays a match. What exists around it:

1. A game server reports a finished match to `POST /api/v1/ingest/match-results` with its service
   token. The result may carry an `aar_replay_url`, an absolute `http://` or `https://` link to a
   replay hosted elsewhere, often attached by a later post once the replay is uploaded.
2. The `/deployments` page's service record shows "View Replay" beside a match whose link is an
   `http(s)` URL, and opens it in a new tab; any other value shows "—".
3. The platform stores no positions, shots or casualties over time: the telemetry holds match
   results, per-player statistics and runtime-session heartbeats, nothing a timeline can play.

## Data

- `POST /api/v1/ingest/match-results` (`ingest_match_results` in
  `crates/api/api_match_telemetry/src/handlers/match_results.rs`): upserts a match and its
  player results; an `aar_replay_url` that is not an absolute `http://` or `https://` URL is
  refused, and an absent one keeps the stored link, so a later post can attach it.
- The replay itself needs data the platform does not collect yet: timestamped unit positions and
  combat, medical and vehicle events per match, and a read that pages a match's timeline.

## Design

The design is at the idea stage, drawn from the product blueprint (archived at
`documentation/archive/go_and_react_era_design/mission_creator_design.md`, section 5, "DCS-Style
AAR") and the workspace's first design draft. No visual reference set exists.

### Design notes

- The replay plays server telemetry over the 2D map of the match's terrain, read-only.
- A multi-channel timeline scrubber plays at 1x, 2x, 5x and 10x.
- The map shows player positions, movement traces, engagement lines and casualty markers.
- Objective capture history and the combat event log stay in step with the timeline.
- The target is a high-fidelity, high-throughput telemetry viewer that replays engagements,
  trajectories and movements accurately, beyond a simple 2D OCAP-style replay; a 3D view is a
  stretch goal, outside the first cut.
- The replay replaces the external link: each match's `aar_replay_url` and its "View Replay" link
  lead to the platform's own replay.

## Open work

- [T-136 — 3D AAR / OCAP-style replay](/documentation/tickets/specs/t131_north_star_backlog.md)
  (ready, [plan](/documentation/tickets/plans/t-136_plan.md)): a replay read that returns a
  match's timeline at 1 Hz, paged, and a map scrubber with play, pause and speed that the
  deployments page links to. Its plan places the page among the routed pages rather than in a
  workspace crate, and names code paths that no longer exist.
- [T-940.13 — Combat, medical and vehicle telemetry events](/documentation/tickets/specs/t940_website_platform.md)
  (ready, [plan](/documentation/tickets/plans/t-940_13_plan.md)): a telemetry-events schema
  and an ingest that stores the events, the data the replay plays.
- [T-096 — Live game-server telemetry bridge](/.ai/tickets/T-096.toml) (deferred, no plan): live
  game-server events bridged into the telemetry ingest.

## Decisions

- No code folder is reserved: the workspace is added as one crate under
  `crates/frontend/workspaces/`, with its manifest, its README, its route component, a row in
  `crates/frontend/shell/frontend_application/src/app_routes.rs` and one in
  `crates/frontend/foundation/frontend_route_table/src/routes.rs`, together; the crate depends on
  the foundation and feature crates and the map crates, never on a page crate or another
  workspace's crates, and its documentation then moves to
  `documentation/crates/frontend/workspaces/<crate>/`.
- Until the replay exists, the platform stores and shows only a link to a replay hosted elsewhere,
  checked as an `http(s)` URL on ingest and again when the page renders it.
