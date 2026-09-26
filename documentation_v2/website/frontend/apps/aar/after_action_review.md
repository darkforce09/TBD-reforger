**Status:** live

# After-action review

A planned workspace, not built: a replay of a finished match from server telemetry over a
read-only map, with a timeline scrubber, so members and leaders can review what happened in an
[event](/documentation_v2/glossary/a_to_f.md#event)'s [mission](/documentation_v2/glossary/g_to_m.md#mission)
after it ends.

## Where it lives

- Code: [`apps/website/frontend/src/v2/apps/aar/`](/apps/website/frontend/src/v2/apps/aar/README.md),
  a reserved folder that holds only its README; `apps/website/frontend/src/v2/apps/mod.rs` declares
  no `aar` module.
- Entry: none. `apps/website/frontend/src/app_routes.rs` and `apps/website/frontend/src/router.rs`
  have no replay route.
- Related features: the [deployments page](/documentation_v2/website/frontend/pages/operations/deployments/deployments_page.md),
  whose service record links each match's external replay; the
  [match telemetry domain](/apps/website/api_v2/src/match_telemetry/README.md), which takes in the
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
  `apps/website/api_v2/src/match_telemetry/handlers/match_results.rs`): upserts a match and its
  player results; an `aar_replay_url` that is not an absolute `http://` or `https://` URL is
  refused, and an absent one keeps the stored link, so a later post can attach it.
- The replay itself needs data the platform does not collect yet: timestamped unit positions and
  combat, medical and vehicle events per match, and a read that pages a match's timeline.

## Design

The design is at the idea stage, drawn from the product blueprint (archived at
`documentation_v2/archive/go_and_react_era_design/mission_creator_design.md`, section 5, "DCS-Style
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

- [T-136 — 3D AAR / OCAP-style replay](/documentation_v2/tickets/specs/t131_north_star_backlog.md)
  (ready, [plan](/documentation_v2/tickets/plans/t-136_plan.md)): a replay read that returns a
  match's timeline at 1 Hz, paged, and a map scrubber with play, pause and speed that the
  deployments page links to. Its plan places the page among the routed pages rather than in this
  workspace folder, and names code paths that no longer exist.
- [T-940.13 — Combat, medical and vehicle telemetry events](/documentation_v2/tickets/specs/t940_website_platform.md)
  (ready, [plan](/documentation_v2/tickets/plans/t-940_13_plan.md)): a telemetry-events schema
  and an ingest that stores the events, the data the replay plays.
- [T-096 — Live game-server telemetry bridge](/.ai/tickets/T-096.toml) (deferred, no plan): live
  game-server events bridged into the telemetry ingest.

## Decisions

- The folder is reserved and holds no code until the workspace is built: a workspace is added with
  its module line, its route and its README together, as the
  [workspaces README](/apps/website/frontend/src/v2/apps/README.md) rules require.
- Until the replay exists, the platform stores and shows only a link to a replay hosted elsewhere,
  checked as an `http(s)` URL on ingest and again when the page renders it.
