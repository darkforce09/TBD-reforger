# Audit 2026-09 — finding → ticket map

Source: master architectural audit (user-supplied 2026-09-04), every claim verified against main @ 072988d57.
Verdicts: TRUE / PARTIAL / FALSE / ALREADY-FIXED / UNVERIFIED. Programs minted from S1–S3: T-936 (mission logic),
T-937 (editor data layer), T-938 (engine and wasm performance). Specs: `t936_mission_logic.md`,
`t937_editor_data_layer.md`, `t938_engine_perf.md`. False claims: `false_claims.md`.

## S1 — mission flatten / contract (crates/map-engine-core/src/mission/flatten.rs)
| Finding | Anchor | Verdict | Ticket |
|---|---|---|---|
| vehicles DeclaredPendingEmit, stripped | flatten.rs:3252-3263 (emit_ticket "T-675") | TRUE | existing T-675/.1/.2 |
| editorTriggers DeclaredPendingEmit; no trigger runtime | flatten.rs:3271-3283; no TBD_TriggerRuntime.c | TRUE | existing T-676 |
| waypoints absent | zero logic in core; mod only icon aliases (TBD_MarkerIcons.c:352-388) | PARTIAL | existing T-677 |
| slot identity stripped | flatten.rs:2404-2419 | TRUE | existing T-674/.1/.2 |
| marker $defs closed | mission.schema.json:1171 | TRUE | existing T-673 |
| win/loss hardcoded | flatten.rs:2573-2574 mode "attrition", :2582 end_on; schema :8/:1132 already has winConditions | PARTIAL | T-936.1 |
| task state machine missing | nothing in schema/core/mod (schema :1208 enum value only) | TRUE | T-936.2 |
| radio synthesized, no UI | flatten.rs:1927-1960; $defs/radioPlan exists (:80, :481) | TRUE | T-936.3 |
| dynamic weather missing | schema :190 static weatherPreset | TRUE | T-936.4 |
| positional audio missing | — | TRUE | T-936.5 |
| dynamic spawn / garrison missing | — | TRUE | T-936.6 |
| tactical graphics missing | — | TRUE | T-936.7 |
| in-editor dry run missing | — | TRUE | deferred by operator 2026-09-04 |

## S2 — doc store / persist (crates/map-engine-core/src/doc/, editor/state/)
| Finding | Anchor | Verdict | Ticket |
|---|---|---|---|
| slotIds/entityIds Any::Array, O(N²), collab loss | store.rs:5162, :5171-5191 | TRUE | T-937.1 |
| ZeroClock, capture_timeout 0, no undo cap | store.rs:361-371 (deliberate :350-360, T-159.22.1) | TRUE | T-937.2 |
| materialize per-slot lookups | store.rs:745-764 cached; :772-812 per slot | PARTIAL | T-937.3 |
| existence via materialize | operations/entity.rs:1885 (warnings :1795/:1847) | TRUE | T-937.3 |
| QuotaExceeded silent | persist.rs:827-832 generic Err → console.warn | PARTIAL | T-937.4 |
| pagehide spawn_local | persist.rs:952-957 | TRUE | T-937.4 |
| note_unreadable lockout | persist.rs:252, :461, :815-826 | TRUE | T-937.4 |
| payload slots/editorLayers no item schema | mission-editor-payload.schema.json:42-43 | TRUE | T-937.5 |
| duplicate slot ids on one callsign | no guard anywhere in the editor | TRUE | T-937.5 |
| 8388608 not enforced in editor | mission.schema.json:6; api validate.rs:359,749; mission_library.rs:1452 = 64<<20 | TRUE | T-937.5 |

## S3 — render / geometry perf
| Finding | Anchor | Verdict | Ticket |
|---|---|---|---|
| DEM decode peak ~400 MB | png_decode.rs:58/64/79; world_assets/mod.rs:620,626 | TRUE | existing T-935.4 |
| satellite L0 655 MB RGBA | satellite.rs | TRUE | T-938.6 |
| create_buffer_init per gesture tick | engine.rs:4907/4921/4924, 7 call sites (audit's :4024-4039 = cluster packing) | PARTIAL | T-938.1 |
| 10+ buffers per chunk crossing | world_host.rs:454-525 | UNVERIFIED | T-938.2 measures first |
| compute cull trees only | engine.rs:1812-1816 | TRUE | T-938.3 |
| CPU icon count every frame | icon_cull_gpu.rs:226; compute_cull.rs:79 | TRUE | T-938.3 |
| single atomicAdd counter | shader.wgsl:315 | TRUE | T-938.3 |
| section cut brute force | building_section.rs:292 | TRUE | T-938.4 |
| HeightField 67 MB per level | building_section.rs:46, :81 | TRUE | T-938.4 |
| 31k synchronous raycasts | building_viewshed.rs:251-268 (:156-195, defaults :37-41) | TRUE | T-938.5 |
| terrain viewshed O((R/C)²), no cap | dem/sample.rs:492-535 | TRUE | T-938.5 |

<!-- S4-S6 below -->
Programs minted from S4–S6: T-939 (editor usability), T-940 (website platform), T-941 (Enfusion mod lifecycle).
Specs: `t939_editor_usability.md`, `t940_website_platform.md`, `t941_mod_lifecycle.md`.

## S4 — editor UI (apps/website/frontend/src/editor/)
| Finding | Anchor | Verdict | Ticket |
|---|---|---|---|
| squad read-only, no faction selector | attributes_modal.rs:1864-1872 (:1546-1607, :1749-1842) | TRUE | T-939.2 |
| outliner single-entity drag | outliner_tree.rs:1110 (peers :1022, :664, :1101) | TRUE | T-939.1 |
| 50x Ctrl+Z after a batch | store.rs:361-371 capture_timeout 0 | TRUE | T-937.2 (T-939.2 consumes with_batch) |
| no Z gizmo | overlays.rs:172-198, :200-210 | TRUE | T-939.3 |
| alignment tools buried, no shortcuts | top_strip.rs:249-252 (T-645), :378-380; keymap = mission_editor.rs keydown | PARTIAL | T-939.4 |
| no squad templates | asset_catalog.rs:321/:460; Compositions dock_right.rs:63-92 (T-650) | PARTIAL | T-939.5 |
| win conditions raw JSON | flatten.rs:2573-2582 | PARTIAL | T-936.1 |
| no canvas error badges; connection wires invisible | overlays.rs:654-655, :679 | TRUE | T-939.6 |
| outliner re-flatten; vehicles bypass virtualization | outliner.rs:39; vehicles_panel.rs:231-276 | TRUE | T-939.7 |
| Ctrl+F not captured | dock_left.rs:134-144 (T-697) | PARTIAL | T-939.8 |

## S5 — website API (apps/website/api/src/)
| Finding | Anchor | Verdict | Ticket |
|---|---|---|---|
| waitlist promote slot_id NULL | events.rs:2042-2053 | TRUE | T-940.1 |
| register has no capacity check | events.rs:1768-1798 (T-227) | ALREADY-FIXED | none (false_claims.md) |
| no reservation tiers | — | TRUE | T-940.12 |
| withdrawn hard-deleted; no_show dead | events.rs:2037-2041; models/event.rs:56 | TRUE | T-940.2 |
| reschedule desyncs event_missions; mission delete orphans | events.rs:1338-1340; missions.rs:816 (soft delete) | TRUE | T-940.3 |
| top-level deaths ignored | telemetry.rs:594-598 (:579-592) | TRUE | T-940.4 |
| AAR external link; no combat/medical/vehicle events | deployments.rs:83 | TRUE | T-940.13 (+ existing T-136) |
| no Workshop sync | — | TRUE | existing T-135 |
| wiki parser limits | wiki.rs (no limits; features missing) | FALSE-as-worded | T-940.9 (features) |
| vehicles no PUT/PATCH/DELETE | app.rs:628; wiki.rs:47, :208 | TRUE | T-940.8 |
| mortar flat vacuum, HTTP-only | mortar.rs:51-60 | TRUE | T-940.10 |
| rcon 503 for kick/ban/change_map | admin.rs:668-673, :765-771 (ban = DB path :199-257) | PARTIAL | T-940.11 |
| list_users limit 20, no pagination | handlers/mod.rs:42-45; admin.rs:60-61 | TRUE | T-940.7 |
| audit stream 2 s poll; missing audit events | audit.rs:161; app.rs:728 | TRUE | T-940.6 |
| pool 25 hardcoded | db.rs:31-34 | TRUE | T-940.5 |

## S6 — Enfusion mod (apps/mod/tbd-framework/Scripts/Game/TBD/)
| Finding | Anchor | Verdict | Ticket |
|---|---|---|---|
| damage live in LOBBY/BRIEFING | TBD_SafestartManager.c:252-265 (:248-250) | TRUE | T-941.1 |
| lobby deploy lockout / race | TBD_LobbyScreen.c:211-212, :492-507 | PARTIAL | T-941.2 |
| END/DEBRIEF no UI | TBD_UILayouts.c:28,31; TBD_FrameworkManager.c:1022-1295 | TRUE | T-941.3 |
| TickCountdown loops at 0 | TBD_SafestartManager.c:389-417 (three exits) | FALSE | none (false_claims.md) |
| chat-only HUD | TBD_ObjectivesComponent.c:813 | TRUE | T-941.4 |
| radio NO_BACKBONE | TBD_RadioTuner.c:49-51, :142-143; TBD_RadioComponent.c:135 | TRUE | T-941.7 (world edit deferred by operator 2026-09-04) |
| naked vehicles | TBD_SpawnManager.c | TRUE | T-941.8 (crew: existing T-675.2) |
| spectator m_fHostMaxRangeM = 0 | TBD_SpectatorComponent.c:61-62 | TRUE | T-941.5 |
| link code in public chat | TBD_AdminCommands.c:41 | TRUE | T-941.6 |
