# T-940 — Website platform: events, telemetry, admin, content

Owner: command center. Source: master audit S5 (2026-09-04), verified against main @ 072988d57 (README.md in this
directory). Scope: TBD-Reforger only. Executor: claude-code. API dir `apps/website/api/src/`; migrations from 0022 (next
free number at ship time); frontend mirrors in `apps/website/frontend/src/core/dto.rs` (R-api golden).

## 1. Related existing tickets — referenced, never re-minted
T-136 AAR replay · T-135 modset manager (Workshop sync) · T-311 leaderboard tie-breaker · T-301 briefing kit · T-302 equip verify · T-304 registry scan ·
T-139 lobby preview · T-393 (shipped) · T-227 (shipped) capacity · T-289 host agent · T-675.2 vehicle reader. False claims, no ticket (false_claims.md): capacity check; wiki parser limits.

## 2. Verified anchors (2026-09-04)
| Finding | Anchor | Verdict | Slice |
|---|---|---|---|
| waitlist promote slot_id NULL | handlers/events/events.rs:2042-2053 | TRUE | T-940.1 |
| withdrawn hard-delete; no_show dead | events.rs:2037-2041; models/event.rs:56 | TRUE | T-940.2 |
| reschedule leaves event_missions; delete orphans | events.rs:1338-1340; missions/missions.rs:816 soft delete | TRUE | T-940.3 |
| top-level deaths ignored | handlers/telemetry/telemetry.rs:594-598 (:579-592); Backend/TBD_ResultsReporter.c | TRUE | T-940.4 |
| pool 25 hardcoded | db.rs:31-34 | TRUE | T-940.5 |
| audit stream 2 s poll; missing rows | handlers/admin/audit.rs:161; app.rs:728 | TRUE | T-940.6 |
| list_users default 20 | handlers/mod.rs:42-45; admin.rs:60-61 | TRUE | T-940.7 |
| vehicles no PUT/PATCH/DELETE | app.rs:628; handlers/content/wiki.rs:47, :208 | TRUE | T-940.8 |
| wiki parser minimal | wiki.rs (features missing) | FALSE-as-worded | T-940.9 |
| mortar flat vacuum | services/mortar.rs:51-60 | TRUE | T-940.10 |
| rcon 503 kick/change_map | admin.rs:668-673, :765-771; services/game_agent.rs | PARTIAL | T-940.11 |
| no reservation tiers | events.rs:1768-1798 capacity only | TRUE | T-940.12 |
| no combat/medical/vehicle events | telemetry/deployments.rs:83 | TRUE | T-940.13 |

## 3. Design and wave packing
- .1 seat pick `FOR UPDATE SKIP LOCKED` + partial unique index (0022); .2 withdrawn_at tombstone, no_show writer, unlinked_attendance (0023); .3 delta cascade + hidden_at (0024); .12 tiers table (0027).
- .4 flat→nested fold, reporter emits nested; .13 telemetry-events schema + match_events (0028). .5 DbPoolConfig from TBD_DB_POOL_*.
- .6 trigger + pg_notify (0025) + `services/audit_notify.rs`; .7 page metadata + pager; .11 Kick/ChangeMap on `game_agent.rs`.
- .8 `content/vehicles.rs` PUT/PATCH/DELETE; .9 `services/wiki_markup.rs` + wiki_revisions (0026); .10 `crates/tbd-ballistics` for API and page.
Waves: A = .1 .4 .5 .6 .7 .10; B = .2, .13, .11; C = .3, .8; D = .9, .12. Shared files: events.rs (.1→.2→.3→.12),
telemetry.rs (.4→.13), admin.rs (.7→.11), wiki.rs (.8→.9), core/dto.rs (.2→.8→.12), services/mod.rs (.6→.9).

## 4. Rules every slice encodes
Defect verified on main first (red pasted verbatim, integration test via `cargo xtask db test-it`); perturbation with `touch` after restore;
no `git add -A`, no `git stash`, no `cargo xtask ci ci-local`; `skip:` = FAIL; no .py/.sh/.mjs committed; file-length allowlists never extended; agents never
merge/push/change status. Report schema: pwd_branch · defect_verified_on_main · changes · perturbation · gate_verdict_tail · files_outside_owns · found_not_fixed · deviations · commits.

## Claude Code prompt — T-940.1

```text
═══ PREFLIGHT ═══
cd .ai/artifacts/worktrees/T-940.1 && export CARGO_TARGET_DIR=/home/Samuel/.cache/tbd-target && git branch --show-current
═══ READ ═══
Spec §3; docs/plans/t-940_1_plan.md; handlers/events/events.rs:1760-1800,2030-2060; migrations/0021_*.sql (format).
═══ PROBLEM ═══
Promotion (:2042-2053) runs UPDATE … SET state='registered' only: slot_id stays NULL and capacity
(T-227, :1768-1798) is never re-checked, so a promoted registrant holds no seat.
═══ SHIPPED ═══
T-227 capacity check at registration; waitlist state machine.
═══ LANGUAGE GATE ═══
Rust (Axum, sqlx); SQL migration.
═══ LOCKED ═══
- One transaction: pick a free slot FOR UPDATE SKIP LOCKED, set slot_id + state, else 409 EVENT_FULL.
- Migration 0022_waitlist_seat.sql adds the partial unique index; number = next free at ship time.
- Response carries the assigned slot.
═══ DO ═══
1. Verify on main: integration test promoting into a full mission succeeds with slot_id NULL; paste the red.
2. Write the migration; rewrite the promotion query; add the 409 path and the slot in the response.
3. Add a concurrent-promotion test (two promotions, one seat).
4. Perturbation: drop FOR UPDATE → concurrent test red; restore, touch, green.
═══ DO NOT ═══
No withdraw or reschedule changes (T-940.2/.3); no files outside owns.
═══ VERIFY ═══
cargo xtask db test-it ; cargo xtask platform wave gate --slice T-940.1
═══ MANUAL ═══
Fill an event, promote from the waitlist: 409; free a seat, promote: the row shows the seat.
═══ RETURN ═══
Report schema per spec §4. Ready for Cursor doc sync.
```

## Claude Code prompt — T-940.2

```text
═══ PREFLIGHT ═══
cd .ai/artifacts/worktrees/T-940.2 && export CARGO_TARGET_DIR=/home/Samuel/.cache/tbd-target && git branch --show-current
═══ READ ═══
Spec §3; docs/plans/t-940_2_plan.md; events.rs:2030-2045 (post T-940.1); models/event.rs:40-70; the results path in handlers/telemetry; core/dto.rs event section.
═══ PROBLEM ═══
Withdraw hard-deletes the registration (:2037-2041); no_show (models/event.rs:56) has no writer; result
players without a linked user leave no attendance, so history is lost.
═══ SHIPPED ═══
T-940.1 seat index; T-393 identity link; deployments.rs renders no_show.
═══ LANGUAGE GATE ═══
Rust (Axum, sqlx); SQL migration.
═══ LOCKED ═══
- Withdraw = state withdrawn + withdrawn_at; the seat index ignores withdrawn rows.
- no_show writer runs on the results path; unlinked players go to unlinked_attendance, reconciled on link (idempotent).
- models/event.rs and core/dto.rs expose the same fields.
═══ DO ═══
1. Verify on main: withdraw then list shows no row; paste the red.
2. Write 0023_registration_tombstones.sql; change the withdraw handler.
3. Add the no_show writer and the unlinked rows; mirror the DTOs.
4. Perturbation: writer skips absent players → no_show test red; restore, touch, green.
═══ DO NOT ═══
No reschedule or delete cascade (T-940.3); no files outside owns.
═══ VERIFY ═══
cargo xtask db test-it ; cargo xtask mk ci-local-leptos ; cargo xtask platform wave gate --slice T-940.2
═══ MANUAL ═══
Withdraw, then view the event: row shows withdrawn with a time; post results missing one registrant: no_show.
═══ RETURN ═══
Report schema per spec §4. Ready for Cursor doc sync.
```

## Claude Code prompt — T-940.3

```text
═══ PREFLIGHT ═══
cd .ai/artifacts/worktrees/T-940.3 && export CARGO_TARGET_DIR=/home/Samuel/.cache/tbd-target && git branch --show-current
═══ READ ═══
Spec §3; docs/plans/t-940_3_plan.md; events.rs:1320-1360; handlers/missions/missions.rs:800-840; schedule listing queries.
═══ PROBLEM ═══
Reschedule (:1338-1340) updates events only; delete_mission (missions.rs:816) soft-deletes and leaves
event_missions and registrations pointing at a hidden mission.
═══ SHIPPED ═══
Soft delete on missions; T-940.2 withdrawn tombstones.
═══ LANGUAGE GATE ═══
Rust (Axum, sqlx); SQL migration.
═══ LOCKED ═══
- Reschedule applies one delta to every event_missions row in the same transaction.
- delete_mission sets hidden_at and withdraws registrations with a system reason; listings exclude hidden rows.
- Migration 0024_event_missions_sync.sql: hidden_at + index on mission_id.
═══ DO ═══
1. Verify on main: reschedule by two hours leaves event_missions.start_time unchanged; paste the red.
2. Write the migration; cascade in the reschedule handler.
3. Cascade in delete_mission; filter listings.
4. Perturbation: delta on the first mission only → multi-mission test red; restore, touch, green.
═══ DO NOT ═══
No hard deletes; no tier logic (T-940.12); no files outside owns.
═══ VERIFY ═══
cargo xtask db test-it ; cargo xtask platform wave gate --slice T-940.3
═══ MANUAL ═══
Reschedule a three-mission event: all three shift; delete one mission: it leaves the schedule, registrants withdrawn.
═══ RETURN ═══
Report schema per spec §4. Ready for Cursor doc sync.
```

## Claude Code prompt — T-940.4

```text
═══ PREFLIGHT ═══
cd .ai/artifacts/worktrees/T-940.4 && export CARGO_TARGET_DIR=/home/Samuel/.cache/tbd-target && git branch --show-current
═══ READ ═══
Spec §3; docs/plans/t-940_4_plan.md; handlers/telemetry/telemetry.rs:570-600; apps/mod/…/Backend/TBD_ResultsReporter.c (emit); tests/deployments_combat.rs.
═══ PROBLEM ═══
telemetry.rs:594-598 ignores flat top-level counters (documented :579-592) while the mod emits only the
flat shape, so deaths and kills from every match are dropped.
═══ SHIPPED ═══
T-393 nested counters + golden; TBD_ResultsReporter.c flat emit.
═══ LANGUAGE GATE ═══
Rust (Axum, sqlx) + Enforce Script (gate: cargo xtask mod compile).
═══ LOCKED ═══
- Fold flat → nested only when the nested block is absent; nested wins when both exist (no double count).
- Reporter emits the nested block and keeps the flat fields for one release.
- Operator: agents may edit the Enfusion mod scripts; in-game behaviour goes on a human checklist.
═══ DO ═══
1. Verify on main: post the reporter's flat payload → deaths stored 0; paste the red.
2. Add the fold in telemetry.rs; extend deployments_combat.rs (flat golden, both-shapes-equal).
3. Emit the nested block in TBD_ResultsReporter.c; cargo xtask mod compile.
4. Perturbation: skip the kills fold → flat golden red; restore, touch, green.
═══ DO NOT ═══
No event-array ingest (T-940.13); no files outside owns.
═══ VERIFY ═══
cargo xtask db test-it ; cargo xtask mod compile ; cargo xtask platform wave gate --slice T-940.4
═══ MANUAL ═══
Run a dev-server match; the posted payload contains the nested block and the deployment shows kills and deaths.
═══ RETURN ═══
Report schema per spec §4. Ready for Cursor doc sync.
```

## Claude Code prompt — T-940.5

```text
═══ PREFLIGHT ═══
cd .ai/artifacts/worktrees/T-940.5 && export CARGO_TARGET_DIR=/home/Samuel/.cache/tbd-target && git branch --show-current
═══ READ ═══
Spec §3; docs/plans/t-940_5_plan.md; db.rs:20-40; config.rs (env parsing pattern); .env.example.
═══ PROBLEM ═══
db.rs:31 max_connections(25) and :32-34 timeouts are literals; operators cannot tune the pool without a rebuild.
═══ SHIPPED ═══
config.rs env-backed settings; .env.example.
═══ LANGUAGE GATE ═══
Rust (sqlx).
═══ LOCKED ═══
- DbPoolConfig from TBD_DB_POOL_* with today's literals as defaults; test-it timing unchanged.
- Invalid values fail startup naming the variable.
═══ DO ═══
1. Verify on main: TBD_DB_POOL_MAX_CONNECTIONS=3 still yields 25; paste the red.
2. Add DbPoolConfig with parse tests; build the pool from it.
3. Document the four variables in .env.example.
4. Perturbation: ignore the override → parse test red; restore, touch, green.
═══ DO NOT ═══
No default changes; no files outside owns.
═══ VERIFY ═══
cargo xtask db test-it ; cargo xtask platform wave gate --slice T-940.5
═══ MANUAL ═══
Start the API with TBD_DB_POOL_MAX_CONNECTIONS=abc: startup error names the variable.
═══ RETURN ═══
Report schema per spec §4. Ready for Cursor doc sync.
```

## Claude Code prompt — T-940.6

```text
═══ PREFLIGHT ═══
cd .ai/artifacts/worktrees/T-940.6 && export CARGO_TARGET_DIR=/home/Samuel/.cache/tbd-target && git branch --show-current
═══ READ ═══
Spec §3; docs/plans/t-940_6_plan.md; handlers/admin/audit.rs:140-200; services/mod.rs; the audit_log table migration; app.rs:728.
═══ PROBLEM ═══
audit.rs:161 polls every 2 s; event create, mission delete and slot kick write no audit row, so the
admin log is late and incomplete.
═══ SHIPPED ═══
audit_log table and stream route; sqlx PgListener available.
═══ LANGUAGE GATE ═══
Rust (Axum, sqlx); SQL migration (trigger function).
═══ LOCKED ═══
- events.rs is not edited: rows come from the 0025 trigger; pg_notify('audit_log') per row.
- services/audit_notify.rs: one listener per process, reconnect with backoff; poll stays as fallback.
═══ DO ═══
1. Verify on main: creating an event yields no audit row; paste the red.
2. Write 0025_audit_notify.sql (trigger on events insert, missions deleted_at, registrations kicked).
3. Write audit_notify.rs; register in services/mod.rs; switch audit.rs to the stream.
4. Perturbation: drop the mission-delete branch → test red; restore, touch, green.
═══ DO NOT ═══
No handler edits under handlers/events or handlers/missions; no files outside owns.
═══ VERIFY ═══
cargo xtask db test-it ; cargo xtask platform wave gate --slice T-940.6
═══ MANUAL ═══
Open the admin audit stream, create an event in another tab: the row appears within a second.
═══ RETURN ═══
Report schema per spec §4. Ready for Cursor doc sync.
```

## Claude Code prompt — T-940.7

```text
═══ PREFLIGHT ═══
cd .ai/artifacts/worktrees/T-940.7 && export CARGO_TARGET_DIR=/home/Samuel/.cache/tbd-target && git branch --show-current
═══ READ ═══
Spec §3; docs/plans/t-940_7_plan.md; handlers/mod.rs:40-50 (read only); handlers/admin/admin.rs:50-80; pages/admin/personnel.rs.
═══ PROBLEM ═══
list_users returns a bare array bounded by PageParams (default 20, max 100) with no metadata, and the
personnel page shows that first page only.
═══ SHIPPED ═══
PageParams::bounds; the personnel page.
═══ LANGUAGE GATE ═══
Rust (Axum, sqlx; Leptos for the page).
═══ LOCKED ═══
- handlers/mod.rs is shared and untouched; the response type change is confined to admin.rs.
- Response = {items, page, per_page, total}; per_page above 100 clamps.
- Pager keeps its state in the URL query.
═══ DO ═══
1. Verify on main: 25 users → response carries no total; paste the red.
2. Add the metadata in admin.rs with a test.
3. Build the pager and per-page selector in personnel.rs.
4. Perturbation: total = items.len() → 25-user test red; restore, touch, green.
═══ DO NOT ═══
No rcon changes (T-940.11); no files outside owns.
═══ VERIFY ═══
cargo xtask db test-it ; cargo xtask mk ci-local-leptos ; cargo xtask mk leptos-gates ; cargo xtask platform wave gate --slice T-940.7
═══ MANUAL ═══
With 25 users, page to the last one; reload keeps the page.
═══ RETURN ═══
Report schema per spec §4. Ready for Cursor doc sync.
```

## Claude Code prompt — T-940.8

```text
═══ PREFLIGHT ═══
cd .ai/artifacts/worktrees/T-940.8 && export CARGO_TARGET_DIR=/home/Samuel/.cache/tbd-target && git branch --show-current
═══ READ ═══
Spec §3; docs/plans/t-940_8_plan.md; app.rs:620-640; handlers/content/wiki.rs:40-60,200-230; handlers/content/mod.rs; core/dto.rs vehicle section.
═══ PROBLEM ═══
app.rs:628 routes list and create only; the vehicle handlers live inside wiki.rs (:47, :208); a vehicle
record can never be corrected or removed.
═══ SHIPPED ═══
list_vehicles, create_vehicle; vehicle DTOs.
═══ LANGUAGE GATE ═══
Rust (Axum, sqlx).
═══ LOCKED ═══
- Handlers move to content/vehicles.rs; wiki.rs keeps only wiki code.
- DELETE is a soft delete hidden from list; PATCH rejects unknown fields.
- core/dto.rs mirrors VehicleUpdate and VehiclePatch.
═══ DO ═══
1. Verify on main: PUT /vehicles/{id} → 405; paste the red.
2. Create vehicles.rs; register in content/mod.rs; move list and create.
3. Add PUT, PATCH, DELETE with tests; route in app.rs; mirror the DTOs.
4. Perturbation: PATCH ignores an unknown field → reject test red; restore, touch, green.
═══ DO NOT ═══
No wiki parser changes (T-940.9); no files outside owns.
═══ VERIFY ═══
cargo xtask db test-it ; cargo xtask mk ci-local-leptos ; cargo xtask platform wave gate --slice T-940.8
═══ MANUAL ═══
Edit a vehicle's name via PUT, patch one field, delete it: list reflects each step.
═══ RETURN ═══
Report schema per spec §4. Ready for Cursor doc sync.
```

## Claude Code prompt — T-940.9

```text
═══ PREFLIGHT ═══
cd .ai/artifacts/worktrees/T-940.9 && export CARGO_TARGET_DIR=/home/Samuel/.cache/tbd-target && git branch --show-current
═══ READ ═══
Spec §3; docs/plans/t-940_9_plan.md; handlers/content/wiki.rs (post T-940.8); services/mod.rs; pages/public/wiki.rs.
═══ PROBLEM ═══
The wiki parser handles a minimal subset: no H3+, links, images, tables or checklists, and saves
overwrite with no history.
═══ SHIPPED ═══
Minimal parser in wiki.rs; wiki page; T-940.6 services/mod.rs registration pattern.
═══ LANGUAGE GATE ═══
Rust (Axum, sqlx; Leptos for the page); SQL migration.
═══ LOCKED ═══
- Parser lives in services/wiki_markup.rs with a typed AST and goldens; links and images are sanitized.
- 0026_wiki_revisions.sql; every save inserts one revision; GET revisions route.
═══ DO ═══
1. Verify on main: a level-three heading renders as a paragraph; paste the red.
2. Write wiki_markup.rs with goldens; register in services/mod.rs.
3. Write the migration; wire the handler and the revisions route; render blocks and history in the page.
4. Perturbation: drop table parsing → golden red; restore, touch, green.
═══ DO NOT ═══
No raw HTML passthrough; no files outside owns.
═══ VERIFY ═══
cargo xtask db test-it ; cargo xtask mk ci-local-leptos ; cargo xtask mk leptos-gates ; cargo xtask platform wave gate --slice T-940.9
═══ MANUAL ═══
Save a fixture page with every block type: all render; save twice: two revisions listed.
═══ RETURN ═══
Report schema per spec §4. Ready for Cursor doc sync.
```

## Claude Code prompt — T-940.10

```text
═══ PREFLIGHT ═══
cd .ai/artifacts/worktrees/T-940.10 && export CARGO_TARGET_DIR=/home/Samuel/.cache/tbd-target && git branch --show-current
═══ READ ═══
Spec §3; docs/plans/t-940_10_plan.md; services/mortar.rs; pages/public/mortar.rs; root Cargo.toml members; both app Cargo.toml files.
═══ PROBLEM ═══
mortar.rs:51-60 is a vacuum model (GRAVITY, per-charge muzzle table) and the page needs the API for
every solution, so solutions ignore elevation, drag and wind and fail offline.
═══ SHIPPED ═══
mortar.rs error handling (:20-49); the mortar page; the workspace layout under crates/.
═══ LANGUAGE GATE ═══
Rust (workspace crate, wasm-clean; Axum; Leptos).
═══ LOCKED ═══
- All physics in crates/tbd-ballistics with unit tests against a published range table.
- API response shape unchanged, extended with dispersion; page computes locally.
- Cargo.lock: one dependency addition, no version bumps.
═══ DO ═══
1. Verify on main: a 100 m elevation delta returns the flat solution; paste the red.
2. Create the crate; add it to root Cargo.toml members and both app Cargo.toml files.
3. Delegate from mortar.rs; rebuild the page on the crate with dispersion and battery rows.
4. Perturbation: zero the drag term → table test red; restore, touch, green.
═══ DO NOT ═══
No changes to saved fire-mission storage; no files outside owns.
═══ VERIFY ═══
cargo test -p tbd-ballistics ; cargo xtask db test-it ; cargo xtask mk ci-local-leptos ; cargo xtask mk leptos-gates ; cargo xtask platform wave gate --slice T-940.10
═══ MANUAL ═══
Disable the network, open the mortar page, compute: a solution with dispersion appears.
═══ RETURN ═══
Report schema per spec §4. Ready for Cursor doc sync.
```

## Claude Code prompt — T-940.11

```text
═══ PREFLIGHT ═══
cd .ai/artifacts/worktrees/T-940.11 && export CARGO_TARGET_DIR=/home/Samuel/.cache/tbd-target && git branch --show-current
═══ READ ═══
Spec §3; docs/plans/t-940_11_plan.md; handlers/admin/admin.rs:490-560,660-680,760-780 (post T-940.7); services/game_agent.rs.
═══ PROBLEM ═══
agent_action_for (:668-673) maps only Restart; Kick and ChangeMap return 503 RCON_ACTION_UNSUPPORTED
(:765-771), so admins cannot act on a live server from the website.
═══ SHIPPED ═══
T-289 host agent bridge (services/game_agent.rs); admin.rs safety model (:537-559); ban via the DB path.
═══ LANGUAGE GATE ═══
Rust (Axum).
═══ LOCKED ═══
- Kick{player_id} numeric only; ChangeMap{scenario_id} from the server's known list; Custom stays 503.
- Every dispatch audited with admin id and arguments.
- Host-side filter grammar is on the plan's operator checklist (not in this repo).
═══ DO ═══
1. Verify on main: rcon kick → 503; paste the red.
2. Extend AgentAction and its wire encoding with tests.
3. Map the commands in admin.rs; audit; keep Custom at 503.
4. Perturbation: accept a non-numeric id → validation test red; restore, touch, green.
═══ DO NOT ═══
No ban changes; no shell strings built from user input; no files outside owns.
═══ VERIFY ═══
cargo xtask db test-it ; cargo xtask platform wave gate --slice T-940.11
═══ MANUAL ═══
After the operator extends the agent filter: kick a test player and change map from the admin page.
═══ RETURN ═══
Report schema per spec §4. Ready for Cursor doc sync.
```

## Claude Code prompt — T-940.12

```text
═══ PREFLIGHT ═══
cd .ai/artifacts/worktrees/T-940.12 && export CARGO_TARGET_DIR=/home/Samuel/.cache/tbd-target && git branch --show-current
═══ READ ═══
Spec §3; docs/plans/t-940_12_plan.md; events.rs register handler (post T-940.3); models/event.rs; core/dto.rs; pages/operations/event_hub.rs.
═══ PROBLEM ═══
Registration is first-come with a capacity check only; members get no priority window and the hub
cannot explain why a seat is unavailable.
═══ SHIPPED ═══
T-940.1 seats, T-940.2 tombstones, T-940.3 cascades; role membership on users.
═══ LANGUAGE GATE ═══
Rust (Axum, sqlx; Leptos for the hub); SQL migration.
═══ LOCKED ═══
- 0027_reservation_tiers.sql: event_reservation_tiers(event_id, tier, seats, opens_at), tier check constraint.
- Events without tiers keep today's behaviour; overflow to open seats only once the open tier is open.
═══ DO ═══
1. Verify on main: a guest registers before the guest tier opens; paste the red.
2. Write the migration; add tier resolution, opens_at refusal and per-tier counting to the register handler.
3. Mirror the model and DTO; render tiers, remaining seats and opening times in event_hub.rs.
4. Perturbation: skip the opens_at check → test red; restore, touch, green.
═══ DO NOT ═══
No changes to promotion or withdraw; no files outside owns.
═══ VERIFY ═══
cargo xtask db test-it ; cargo xtask mk ci-local-leptos ; cargo xtask mk leptos-gates ; cargo xtask platform wave gate --slice T-940.12
═══ MANUAL ═══
Create tiers member/guest/open with staggered times; sign up as each: refusals name the tier and time.
═══ RETURN ═══
Report schema per spec §4. Ready for Cursor doc sync.
```

## Claude Code prompt — T-940.13

```text
═══ PREFLIGHT ═══
cd .ai/artifacts/worktrees/T-940.13 && export CARGO_TARGET_DIR=/home/Samuel/.cache/tbd-target && git branch --show-current
═══ READ ═══
Spec §3; docs/plans/t-940_13_plan.md; handlers/telemetry/telemetry.rs (post T-940.4); models/telemetry.rs; packages/tbd-schema/schema/ (sibling schema + golden layout).
═══ PROBLEM ═══
Ingest stores per-player counters only; there is no per-event combat, medical or vehicle record, so
the AAR replay (T-136) has nothing to play.
═══ SHIPPED ═══
T-940.4 counters ingest; schema gates (cargo xtask ci schema-validate); T-136 owns the in-game capture.
═══ LANGUAGE GATE ═══
Rust (Axum, sqlx); JSON Schema; SQL migration.
═══ LOCKED ═══
- telemetry-events.schema.json envelope {t, kind, actor, target, pos, data}; kinds combat.hit/kill, medical.down/revive/heal, vehicle.enter/exit/destroy.
- 0028_telemetry_events.sql match_events with (match_id, t) index; bulk insert in chunks.
- Unknown kind → reject naming the index; nothing stored.
═══ DO ═══
1. Verify on main: results with an events array store zero events; paste the red.
2. Write the schema and golden; run schema-validate.
3. Write the migration; add validation + insert with tests; add the row type.
4. Perturbation: accept an unknown kind → reject test red; restore, touch, green.
═══ DO NOT ═══
No mod emitter changes; no files outside owns.
═══ VERIFY ═══
cargo xtask ci schema-validate ; cargo xtask db test-it ; cargo xtask platform wave gate --slice T-940.13
═══ MANUAL ═══
Post a fixture with fifty events: fifty rows in match_events; post one bad kind: 4xx naming the index.
═══ RETURN ═══
Report schema per spec §4. Ready for Cursor doc sync.
```
