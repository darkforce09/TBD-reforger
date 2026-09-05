# Master audit — claims verified against live code, 2026-09-04
Verdicts: TRUE / PARTIAL / FALSE / ALREADY-FIXED. Anchors are current file:line on main @ 072988d57.
Related ticket status: T-673 queued, T-674(.1,.2) queued, T-675(.1,.2) queued, T-676 queued, T-677 queued,
T-393 SHIPPED, T-227 SHIPPED, T-090.9 ready. API handlers now nest: apps/website/api/src/handlers/{events,telemetry,admin,content}/.

## S1 mission flatten / contract (crates/map-engine-core/src/mission/flatten.rs)
- TRUE vehicles DeclaredPendingEmit, stripped — flatten.rs:3252-3263 (fate :3258, emit_ticket "T-675")
- TRUE editorTriggers DeclaredPendingEmit — flatten.rs:3271-3283 (:3277, emit_ticket "T-676")
- TRUE no TBD_TriggerRuntime.c — apps/mod/tbd-framework/Scripts/Game/TBD/{Backend,Core,Gamemode,Markers,Objectives,Radio,Registry,Spectator,UI,Zones}; zero trigger runtime
- PARTIAL waypoints: zero logic in core; mod hits are marker-icon aliases only (tbd-export TBD_MarkerIcons.c:352-388); frontend dead comments panels/context_menu.rs:170,188,293 → no movement-order logic anywhere
- TRUE slot identity stripped — flatten.rs:2404-2419 (SLOT_IDENTITY_DROPS loop only files diagnostics)
- TRUE $defs/marker additionalProperties:false — packages/tbd-schema/schema/mission.schema.json:1171 (required x,z,icon,label; icon closed 64-key enum)
- PARTIAL win conditions — flatten.rs:2573-2574 mode "attrition" hardcoded; end_on at :2582 conditional (2+ factions holding slots)
- TRUE radio synthesized — flatten.rs:1927-1960 (NET_FREQ_BASE_MHZ + STEP * index); zero radio UI in apps/website/frontend/src/editor/panels/

## S2 doc store / persist
- TRUE slotIds/entityIds as Any::Array — crates/map-engine-core/src/doc/store.rs:5162 (retain_ids) + :5171-5191 (append_id) linear scan + clone-rewrite per append
- TRUE ZeroClock + capture_timeout 0, no undo cap — store.rs:361-371 UndoOptions{capture_timeout_millis:0, timestamp:ZeroClock}; deliberate per :350-360 comment (Yjs parity, T-159.22.1) — decision must be recorded when changing
- PARTIAL materialize — store.rs:740-820: layer/hidden cached (745-764) but per-slot read_position/read_str×3/read_stance/resolve_slot_side_key at 772-812
- TRUE existence via materialize — apps/website/frontend/src/editor/state/operations/entity.rs:1885 slot_attrs_exists (same file warns at 1795/1847 that materialize drops hidden slots — must NOT be used for existence)
- PARTIAL QuotaExceeded — persist.rs:827-832 generic save_state_as Err → console.warn → return; no quota-specific code anywhere. Silent-drop effect real.
- TRUE pagehide spawn_local — apps/website/frontend/src/editor/state/persist.rs:952-957 (on_hide closure), fire-and-forget
- TRUE note_unreadable lockout — persist.rs:252 (def), :461 (set), :815-826 (run_save refuses forever; only "Reload to retry")
- TRUE payload schema slots no items — packages/tbd-schema/schema/mission-editor-payload.schema.json:42 "slots":{"type":"array"}; :43 editorLayers same
- TRUE 8388608 not enforced in editor — pin mission.schema.json:6; enforced only apps/website/api/src/contract/validate.rs:359,749; editor ceiling library/mission_library.rs:1452 UPLOAD_MAX_BYTES = 64<<20

## S3 render / geometry perf
- TRUE DEM decode buffers — crates/map-engine-core/src/dem/png_decode.rs:58 (u8), :64 (u16), :79 meters_cache f32 + frontend world_assets/mod.rs:620 fetched bytes, :626 hillshade; sync on wasm thread (mod.rs:609-611 admits)
- PARTIAL cited 4024-4039 = cluster packing (engine.rs:4025-4039) → upload_cluster_lane (4626) → upload_slot_role_lane; alloc is at 4924
- TRUE upload_slot_role_lane new buffer — crates/map-engine-render/src/engine.rs:4907 fn, :4924 create_buffer_init + :4921 bytes.to_vec() per call; 7 call sites (4565/4577/4610/4627/4642/4693/4870); no pool
- UNVERIFIED world_host.rs:454-525 chunk-crossing buffer burst — apps/website/frontend/src/editor/world_assets/world_host.rs exists; slice must verify first
- TRUE compute cull WorldTrees only — engine.rs:1812-1816 do_compute_trees requires !tree_icons_20.is_empty()
- TRUE CPU scan per frame — crates/map-engine-render/src/icon_cull_gpu.rs:226 count_icons_in_frustum before early-outs, every encode_cull (compute_cull.rs:79 linear)
- TRUE atomicAdd single counter — crates/map-engine-render/src/shader.wgsl:315
- TRUE section cut brute force — crates/map-engine-core/src/building_section.rs:292 section_at_owned iterates every tri; occl.bvh unused
- TRUE HeightField Option<f64> 2048² — building_section.rs:46 MAX_PLAN_DIM=2048; :66-83, alloc :81 vec![None; cols*rows] (16 B each → 67 MB)
- TRUE viewshed 31k sync raycasts — crates/map-engine-core/src/building_viewshed.rs:251-268 wash_band via level_wash:156-175 / level_wash_compound:180-195; defaults :37-41 r=25 m cell 0.25; per level (198-206)
- TRUE terrain viewshed O((R/C)²) no cap — crates/map-engine-core/src/dem/sample.rs:492-500 OVERSAMPLE=2.0, loop :500-535; no radius clamp

## S4 editor UI (apps/website/frontend/src/editor/)
- TRUE squad read-only, no faction selector — panels/attributes_modal.rs:1864-1872; faction only as asset-id text (1546-1607, 1749-1842)
- TRUE outliner single drag — panels/outliner_tree.rs:1110 begin_layer_slot_drag(one String); peers 1022, 664, 1101
- TRUE no Z gizmo — canvas/overlays.rs:172-198 Translate X/Y only; Rotate 200-210 flat ring; enum total
- TRUE connections not drawn — overlays.rs:654-655 "no map glyph"; ConnectionsPanelOverlay :679
- PARTIAL Ctrl+F — no "f" handler; document search exists panels/dock_left.rs:134-144 (search_document, T-697), keyboard-unreachable
- PARTIAL placement tools menu is "Arrange" — panels/top_strip.rs:249-252 (T-645), gated selection ≥1 (:378-380); menu-only true
- PARTIAL squad templates — arsenal/asset_catalog.rs zero squad rows (:321,:460 drop *_base.et); Compositions palette exists panels/dock_right.rs:63-92 (T-650)
- TRUE vehicles bypass virtualization — panels/outliner.rs:39 VIRTUAL_SLOT_THRESHOLD=50 gates only virtual_tree (outliner_tree.rs:1245,1369); panels/vehicles_panel.rs:231-276 .map().collect_view()

## S5 website API (apps/website/api/src/)
- TRUE waitlist promote slot_id NULL — handlers/events/events.rs:2042-2053 UPDATE … SET state='registered' only
- ALREADY-FIXED capacity check — T-227: events.rs:1768-1772 counts orbat_slots; :1794-1798 409 at capacity==0; :1781-1793 comment
- TRUE withdrawn hard-delete — events.rs:2037-2041 DELETE FROM event_registrations
- TRUE no_show dead — models/event.rs:56 defined; frontend pages/public/deployments.rs:55 rendered; no writer
- TRUE reschedule leaves event_missions.start_time — events.rs:1338-1340; zero "UPDATE event_missions" in events.rs
- TRUE top-level deaths ignored — handlers/telemetry/telemetry.rs:594-598 (deliberate per :579-592, documented, still data loss)
- TRUE AAR external link — handlers/telemetry/deployments.rs:83 aar_replay_url String (:217,:233,:244)
- FALSE-as-worded wiki parser limits — handlers/content/wiki.rs has no limits; real gap = missing features (H3+, links, images, tables, checklists, revisions)
- TRUE vehicles no PUT/PATCH/DELETE — app.rs:628 get(list_vehicles).post(create_vehicle); handlers in wiki.rs:47,:208
- TRUE mortar flat — services/mortar.rs:51-60 GRAVITY, per-charge muzzle table, vacuum; error handling :20-49 fine
- PARTIAL rcon — handlers/admin/admin.rs:668-673 agent_action_for Restart→Some, ChangeMap|Kick|Custom→None → 503 RCON_ACTION_UNSUPPORTED (:765-771); "ban" not an rcon action (parse_rcon_command:498-510 400s it; user ban separate DB path :199-257)
- TRUE list_users default 20 — handlers/mod.rs:42-45 PageParams::bounds unwrap_or(20), max 100; admin.rs:60-61
- TRUE audit stream 2s poll — handlers/admin/audit.rs:161 interval 2s; route app.rs:728
- TRUE pool 25 hardcoded — db.rs:31 max_connections(25); :32-34 idle/lifetime/acquire fixed

## S6 Enfusion mod (apps/mod/tbd-framework/Scripts/Game/TBD/)
- TRUE Safestart arms only SAFE_START — Gamemode/TBD_SafestartManager.c:252-265 (deliberate :248-250); LOBBY auto-deploy (~250 ms, TBD_SpawnManager m_bAutoDeploy) puts bodies in world during that window
- PARTIAL deploy lockout — UI/Lobby/TBD_LobbyScreen.c:211-212 race acknowledged; DEPLOY gating :492-507 (ShouldStandDown + IsDeployPending); screen closes rather than locks
- TRUE END/DEBRIEF no layouts — UI/TBD_UILayouts.c:28,31 only SCREEN_SHELL, LIST_ROW; stages exist Core TBD_GameStage.c:1, TBD_FrameworkManager.c:1022,1123,1162,1295
- FALSE TickCountdown loop — TBD_SafestartManager.c:389-417 next<=0 → GoLive; !m_bArmed guard :391-395; stage-drift lift :397-405; three exits
- TRUE radio NO_BACKBONE — Radio/TBD_RadioTuner.c:49-51 enum, :142-143 set when GetBackbone() null; TBD_RadioComponent.c:135 boot warning; world TBD_Dev_POC.ent lacks RadioManagerEntity (fix is Workbench world edit — DEFERRED by operator; script-side fallback + checklist only)
- TRUE spectator range 0 = unlimited — Spectator/TBD_SpectatorComponent.c:61-62 attribute; :56-60 comment
- TRUE link code in public chat — Gamemode/TBD_AdminCommands.c:41

Scorecard: 28 TRUE · 9 PARTIAL · 3 FALSE (S5 capacity ALREADY-FIXED T-227; S6 countdown; S5 wiki inverted) · 1 UNVERIFIED (S3 world_host.rs:454-525)
