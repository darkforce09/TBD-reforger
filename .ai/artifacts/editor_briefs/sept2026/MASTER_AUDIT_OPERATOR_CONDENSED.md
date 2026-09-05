# Master Architectural Audit (user-supplied 2026-09-04) — condensed with file anchors

## Critical path (user's own ordering)
1. T-675 vehicle roster + T-676 triggers emit in flatten.rs
2. Auto-deploy claimed slot holders on LOBBY->BRIEFING
3. Arm safestart / disable damage in LOBBY + BRIEFING
4. Telemetry ingest contract: record kills/deaths/combat stats (flat fields from mod vs nested counters)
5. Replace Any::Array on Yrs maps with native YArray (squad.slotIds, layer.entityIds)
6. Delete CPU linear scan in IconComputeCull::encode_cull (icon_cull_gpu.rs:226)
7. End-of-round scoreboard modal in mod
8. Postgres pool size configurable (db.rs:32) + audit-log stream LISTEN/NOTIFY instead of 2s poll

## S1 Mission creator parity (flatten.rs)
- Vehicles DeclaredPendingEmit flatten.rs:3254-3263 (T-675)
- editorTriggers DeclaredPendingEmit flatten.rs:3273-3283; no TBD_TriggerRuntime.c (T-676)
- Waypoints absent entirely (T-677)
- Slot identity stripped flatten.rs:2406-2419 (T-674)
- Markers $defs/marker additionalProperties:false drops color/fill/rot/shape (T-673)
- Missing: task state machine, positional audio, custom win/loss (flatten.rs:2574 hardcoded attrition), dynamic spawn/garrison, tactical graphics, radio freq UI (derive_radio_plan), dynamic weather, in-editor dry run

## S2 Data/CRDT (crates/map-engine-core/src/doc/)
- store.rs:5171-5191 squad.slotIds / layer.entityIds as Any::Array -> O(N^2) + collab loss on undo
- store.rs:350-371 ZeroClock, capture_timeout_millis 0 -> every txn undo step; no UndoOptions cap; GC lock-in
- store.rs:740-820 materialize() 2 lookups/slot, 12 vecs; operations/entity.rs:1885 existence check via materialize
- persist.rs:828-833 QuotaExceeded silent; :952-959 pagehide spawn_local aborted; note_unreadable lockout
- mission-editor-payload.schema.json slots no item schema; dup slot IDs on same callsign; no 8MB (8388608) payload limit

## S3 Engine/wasm
- DEM decode peak ~400MB (png_decode.rs, world_assets/mod.rs); satellite L0 655MB RGBA
- engine.rs:4024-4039, :4907-4942 upload_slot_role_lane create_buffer_init per gesture tick
- world_host.rs:454-525 10+ new buffers per chunk crossing
- engine.rs:1812 compute cull only WorldTrees; icon_cull_gpu.rs:226 count_icons_in_frustum CPU scan every frame; shader.wgsl:315 single atomicAdd
- building_section.rs:274-315 brute-force; HeightField::empty 67MB/level; building_viewshed.rs:155-206 31k raycasts sync; sample.rs:491-530 viewshed O((R/C)^2)

## S4 Editor UX
- attributes_modal.rs:1864 squad read-only, no faction selector
- outliner_tree.rs:1110 single-entity drag
- 50x Ctrl+Z (capture_timeout 0) -> batch txn grouping
- no Z gizmo (canvas/overlays.rs); alignment tools buried, no shortcuts
- no squad templates; win conditions raw JSON
- no canvas error badges; connection wires invisible overlays.rs:650-658; outliner re-flatten every change, vehicles bypass virtualization; Ctrl+F not captured

## S5 Website platform (apps/website/api)
- events.rs:2035-2055 waitlist promote slot_id NULL; register_for_event_mission no capacity check; no reservation tiers; withdrawn hard-deleted, no_show dead; reschedule desyncs event_missions.start_time; mission delete cascades
- telemetry.rs:594-598 top-level deaths ignored (T-393 nested counters vs flat mod fields)
- deployments.rs AAR = external link; no combat/medical/vehicle events
- no Workshop sync; wiki.rs parser minimal; vehicle DB no PUT/PATCH/DELETE; mortar calc flat vacuum, HTTP
- rcon 503 for kick/ban/change_map; list_users limit 20 no pagination; audit-logs/stream 2s poll + missing audit events; db.rs:32 pool 25 hardcoded

## S6 Enfusion mod (apps/mod/tbd-framework) — executor gate: mod paths need explicit claude-code assignment
- damage live in LOBBY/BRIEFING (TBD_SafestartManager arms only SAFE_START)
- TBD_LobbyScreen deploy lockout on BRIEFING
- END/DEBRIEF no UI; TickCountdown loops at 0 if SetStage(LIVE) refused
- chat-only HUD; radio NO_BACKBONE (no RadioManagerEntity); naked vehicles; spectator m_fHostMaxRangeM=0; `#tbd link <code>` in public chat
