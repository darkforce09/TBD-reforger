**Status:** live — Documentation V2 move manifest summary: live targets

# Live targets of the Documentation V2 move

Every live target of [the manifest](/documentation_v2/refactor_move_manifest.tsv): 135 final
documents, each with the writer who rewrites it, its primary source (moved to the target path
in commit 2a) and its secondary sources (moved to `documentation_v2/pending_merge/<writer>/` and
merged by the writer in Phase 5). 56 targets have no primary source: the writer creates
them from the pending_merge sources. The folder index is [README.md](README.md).

## .ai/

| final target | writer | primary source | pending_merge sources |
|---|---|---|---|
| `tickets/plan_template.md` | G2 | `docs/plans/TEMPLATE.md` | - |

## documentation_v2/

| final target | writer | primary source | pending_merge sources |
|---|---|---|---|
| `README.md` | P3-2 | `docs/website/README.md` | `documentation_v2/README.md` |

## documentation_v2/contracts_v2/

| final target | writer | primary source | pending_merge sources |
|---|---|---|---|
| `definitions/bridge_messages.md` | F12 | `contracts_v2/definitions/bridge-messages.md` | - |

## documentation_v2/design_system/

| final target | writer | primary source | pending_merge sources |
|---|---|---|---|
| `README.md` | F17 | (writer creates) | `documentation_v2/design_system/README.md` |
| `design_tokens.md` | F17 | `docs/website/frontend/THEME.md` | `documentation_v2/design_system/design_tokens.md` |
| `military_symbology.md` | F17 | (writer creates) | `documentation_v2/design_system/military_symbology.md` |
| `token_exports/aegis_design_tokens.md` | F17 | `docs/specs/Mission_Creator_Mock_Up/aegis_tokens/DESIGN.md` (+5 collapsed copies) | - |
| `token_exports/dark_tactical_operations_design_tokens.md` | F17 | `docs/mod/ui/ui_stitch_mockup/stitch_tbd_reforger_ui_mod_ingame_hud/dark_tactical_operations/DESIGN.md` | - |
| `token_exports/reforger_dark_tactical_design_tokens.md` | F17 | `docs/mod/ui/ui_stitch_mockup/stitch_tbd_reforger_ui_mod_postgame/reforger_dark_tactical/DESIGN.md` | - |

## documentation_v2/known_bugs/

| final target | writer | primary source | pending_merge sources |
|---|---|---|---|
| `README.md` | F17 | `docs/platform/known-bugs/README.md` | - |
| `kb_001_mission_creator_selection_at_scale.md` | F17 | `docs/platform/known-bugs/KB-001-mission-creator-selection-at-scale.md` | - |
| `kb_002_editor_gate_boot_wedge.md` | F17 | `docs/platform/known-bugs/KB-002-editor-gate-boot-wedge.md` | - |

## documentation_v2/mod/

| final target | writer | primary source | pending_merge sources |
|---|---|---|---|
| `README.md` | F11 | `docs/mod/README.md` | `documentation_v2/mod/README.md` |
| `tbd-emcp/README.md` | F11 | (writer creates) | `documentation_v2/mod/tbd_emcp/README.md` |
| `tbd-export/README.md` | F11 | (writer creates) | `documentation_v2/mod/tbd_export/README.md` |
| `tbd-framework/README.md` | F10 | (writer creates) | `documentation_v2/mod/tbd_framework/README.md` |
| `tbd-framework/UI/README.md` | F10 | `docs/mod/ui/UI_STRUCTURE.md` | `docs/mod/ui/ui_referances/UI_REFERENCES.md` · `documentation_v2/mod/tbd_framework/ui_layouts/README.md` |
| `tbd-framework/UI/admin_help_ticket/admin_help_ticket_specification.md` | F10 | `docs/mod/ui/ui_referances/admin_help_ticket_spec.md` | - |
| `tbd-framework/UI/briefing/briefing_specification.md` | F10 | `docs/mod/ui/ui_referances/briefing_ui_spec.md` | - |
| `tbd-framework/UI/debrief_after_action_review/debrief_after_action_review_specification.md` | F10 | `docs/mod/ui/ui_referances/debrief_aar_ui_spec.md` | - |
| `tbd-framework/UI/discord_identity_link/discord_identity_link_specification.md` | F10 | `docs/mod/ui/ui_referances/discord_identity_link_spec.md` | - |
| `tbd-framework/UI/end_screen/end_screen_specification.md` | F10 | `docs/mod/ui/ui_referances/end_screen_ui_spec.md` | - |
| `tbd-framework/UI/in_game_menu/in_game_menu_specification.md` | F10 | `docs/mod/ui/ui_referances/ingame_menu_ui_spec.md` | - |
| `tbd-framework/UI/lobby/lobby_specification.md` | F10 | `docs/mod/ui/ui_referances/lobby_ui_spec.md` | - |
| `tbd-framework/UI/mission_selection/mission_selection_specification.md` | F10 | `docs/mod/ui/ui_referances/mission_selection_ui_spec.md` | - |
| `tbd-framework/UI/objective_capture_hud/objective_capture_hud_specification.md` | F10 | `docs/mod/ui/ui_referances/objective_capture_hud_spec.md` | - |
| `tbd-framework/UI/play_area_warning/play_area_warning_specification.md` | F10 | `docs/mod/ui/ui_referances/play_area_warning_spec.md` | - |
| `tbd-framework/UI/safe_start_hud/safe_start_hud_specification.md` | F10 | `docs/mod/ui/ui_referances/safestart_hud_spec.md` | - |
| `tbd-framework/UI/spectator/spectator_specification.md` | F10 | `docs/mod/ui/ui_referances/spectator_ui_spec.md` | - |
| `tbd-framework/UI/tactical_marker_palette/tactical_marker_palette_specification.md` | F10 | `docs/mod/ui/ui_referances/tactical_marker_palette_spec.md` | - |
| `tbd-framework/mod_design.md` | F10 | `docs/mod/TBD_MOD_DESIGN.md` | - |
| `tbd-framework/vanilla_source_coverage.md` | F10 | `docs/mod/vanilla_carve_coverage.md` | - |

## documentation_v2/runbooks/

| final target | writer | primary source | pending_merge sources |
|---|---|---|---|
| `README.md` | F18 | (writer creates) | `documentation_v2/runbooks/README.md` |
| `cursor_workspace_setup.md` | F16 | `docs/website/CURSOR_SETUP.md` | - |
| `database_operations.md` | F13 | (writer creates) | `documentation_v2/runbooks/database_operations.md` |
| `editor_capture.md` | F14 | `docs/tools/editor_capture.md` | - |
| `editor_gates.md` | F14 | `docs/website/EDITOR_GATE_RUNBOOK.md` | - |
| `enfusion_mcp_tooling.md` | F14 | `docs/mod/MCP_TOOLING.md` | - |
| `factory_waves/README.md` | F16 | `docs/platform/PLATFORM_FACTORY.md` | `docs/platform/EDITOR_FACTORY_FOR_CURSOR.md` · `docs/platform/EDITOR_FACTORY_START.md` · `docs/platform/EDITOR_SLICE_BRIEF.md` · `docs/platform/EDITOR_VERIFY_BRIEF.md` · `docs/platform/FACTORY_FOR_CURSOR.md` |
| `game_server_staging/README.md` | F14 | `docs/mod/STAGING-SERVER.md` | - |
| `local_development.md` | F13 | `docs/website/DEV_RUNBOOK.md` | `documentation_v2/runbooks/local_development.md` |
| `mod_slice_workflow.md` | F15 | `docs/mod/SLICE_WORKFLOW.md` | `docs/mod/CLAUDE-CODE-START.md` · `docs/mod/VERIFY_AGENT_PROMPT.md` |
| `spawn_determinism.md` | F14 | `docs/mod/SPAWN_DETERMINISM.md` | - |
| `testing_and_ci.md` | F13 | (writer creates) | `documentation_v2/runbooks/testing_and_ci.md` |
| `two_client_playtest/README.md` | F15 | `docs/platform/PLAYTEST_RUNBOOK.md` | - |
| `website_deployment.md` | F13 | `docs/website/HOME_SERVER.md` | `documentation_v2/runbooks/deployment.md` |

## documentation_v2/standards/

| final target | writer | primary source | pending_merge sources |
|---|---|---|---|
| `coding_standards/README.md` | F17 | `docs/platform/CODING_STANDARDS.md` | - |
| `commit_checklist.md` | F17 | `docs/website/AGENT_COMMIT_CHECKLIST.md` | - |
| `documentation_standards.md` | P3-2 | `docs/platform/DOCUMENTATION_STANDARDS.md` | `docs/website/_doc_header.md` |
| `ticket_identifiers.md` | F17 | `docs/website/TAGS.md` | - |
| `where_does_x_go.md` | F17 | `docs/platform/WHERE_DOES_X_GO.md` | - |

## documentation_v2/ticketboard/

| final target | writer | primary source | pending_merge sources |
|---|---|---|---|
| `README.md` | F12 | (writer creates) | `documentation_v2/tools/ticketboard/README.md` |

## documentation_v2/tickets/

| final target | writer | primary source | pending_merge sources |
|---|---|---|---|
| `README.md` | F18 | (writer creates) | `documentation_v2/tickets/README.md` |
| `plans/README.md` | F18 | (writer creates) | `documentation_v2/tickets/plans/README.md` |
| `specs/README.md` | F18 | (writer creates) | `documentation_v2/tickets/specs/README.md` |

## documentation_v2/tools_v2/

| final target | writer | primary source | pending_merge sources |
|---|---|---|---|
| `README.md` | F12 | (writer creates) | `documentation_v2/tools/README.md` |
| `developer-tools/README.md` | F12 | (writer creates) | `documentation_v2/tools/developer_tools/README.md` |
| `ticket-engine/README.md` | F12 | (writer creates) | `documentation_v2/tools/ticket_engine/README.md` |
| `ticket-engine/token_estimate_factor.md` | F12 | `docs/platform/token_estimate_factor.md` | - |
| `verification-core/README.md` | F12 | (writer creates) | `documentation_v2/tools/verification_core/README.md` |
| `xtask/README.md` | F12 | (writer creates) | `documentation_v2/tools/xtask/README.md` |

## documentation_v2/website/

| final target | writer | primary source | pending_merge sources |
|---|---|---|---|
| `README.md` | F09 | (writer creates) | `documentation_v2/website/README.md` |

## documentation_v2/website/api_v2/

| final target | writer | primary source | pending_merge sources |
|---|---|---|---|
| `README.md` | F09 | `docs/website/backend/README.md` | `documentation_v2/website/api/README.md` |
| `api_overview.md` | F09 | `docs/website/backend/ROADMAP.md` | - |

## documentation_v2/website/frontend/

| final target | writer | primary source | pending_merge sources |
|---|---|---|---|
| `README.md` | F04 | `docs/website/frontend/ROADMAP.md` | `documentation_v2/website/frontend/README.md` · `docs/website/frontend/INDEX.md` · `docs/website/frontend/README.md` · `docs/website/frontend/TRACKING.md` |

## documentation_v2/website/frontend/apps/

| final target | writer | primary source | pending_merge sources |
|---|---|---|---|
| `README.md` | F08 | (writer creates) | `documentation_v2/website/frontend/apps/README.md` |
| `aar/README.md` | F08 | (writer creates) | `documentation_v2/website/frontend/apps/aar/README.md` |
| `debug/README.md` | F03 | (writer creates) | `documentation_v2/website/frontend/apps/debug/README.md` |
| `debug/building_viewer_page.md` | F03 | `docs/website/frontend/pages/debug-building-viewer.md` | - |
| `editor/README.md` | F08 | `docs/specs/Mission_Creator_Architecture/README.md` | `documentation_v2/website/frontend/apps/editor/README.md` |
| `editor/arsenal/README.md` | F08 | (writer creates) | `documentation_v2/website/frontend/apps/editor/arsenal/README.md` |
| `editor/decisions.md` | F07 | `docs/specs/Mission_Creator_Architecture/agent_execution.md` | - |
| `editor/eden_editor_reference/attributes.md` | F08 | `docs/specs/Mission_Creator_Architecture/eden/attributes.md` | - |
| `editor/eden_editor_reference/eden_gap_analysis.md` | F07 | `docs/specs/Mission_Creator_Architecture/eden/gap_analysis.md` | - |
| `editor/eden_editor_reference/interactions/README.md` | F08 | `docs/specs/Mission_Creator_Architecture/eden/interactions.md` | - |
| `editor/eden_editor_reference/ui_anatomy.md` | F08 | `docs/specs/Mission_Creator_Architecture/eden/ui_anatomy.md` | - |
| `editor/feature_inventory/README.md` | F05 | `docs/specs/Mission_Creator_Architecture/feature_inventory.md` | - |
| `editor/feature_inventory/feds_schema.md` | F05 | `docs/specs/Mission_Creator_Architecture/reference/feds_schema.md` | - |
| `editor/mission_creator_roadmap.md` | F07 | `docs/specs/Mission_Creator_Architecture/ROADMAP.md` | - |
| `editor/ux_spec.md` | F07 | `docs/specs/Mission_Creator_Architecture/ux_spec.md` | `docs/website/frontend/pages/mission-editor.md` |
| `planner/README.md` | F08 | (writer creates) | `documentation_v2/website/frontend/apps/planner/README.md` |

## documentation_v2/website/frontend/core/

| final target | writer | primary source | pending_merge sources |
|---|---|---|---|
| `README.md` | F04 | (writer creates) | `documentation_v2/website/frontend/core/README.md` |
| `ui/README.md` | F04 | (writer creates) | `documentation_v2/website/frontend/core/ui/README.md` |

## documentation_v2/website/frontend/pages/

| final target | writer | primary source | pending_merge sources |
|---|---|---|---|
| `README.md` | F04 | (writer creates) | `documentation_v2/website/frontend/pages/README.md` |
| `account/README.md` | F04 | (writer creates) | `documentation_v2/website/frontend/pages/account/README.md` |
| `account/account_pages.md` | F04 | `docs/website/frontend/pages/settings.md` | `docs/website/frontend/auth/auth-callback.md` · `docs/website/frontend/auth/login.md` |
| `administration/README.md` | F01 | (writer creates) | `documentation_v2/website/frontend/pages/administration/README.md` |
| `administration/approvals/README.md` | F01 | (writer creates) | `documentation_v2/website/frontend/pages/administration/approvals/README.md` |
| `administration/approvals/mission_approvals_page.md` | F01 | `docs/website/frontend/pages/mission-approvals.md` | `apps/website/frontend/src/v2/pages/administration/approvals/page.md` |
| `administration/audit_logs/README.md` | F01 | (writer creates) | `documentation_v2/website/frontend/pages/administration/audit_logs/README.md` |
| `administration/audit_logs/audit_logs_page.md` | F01 | `docs/website/frontend/pages/audit-logs.md` | `apps/website/frontend/src/v2/pages/administration/audit_logs/page.md` |
| `administration/content_manager/README.md` | F01 | (writer creates) | `documentation_v2/website/frontend/pages/administration/content_manager/README.md` |
| `administration/content_manager/content_manager_page.md` | F01 | `docs/website/frontend/pages/content-manager.md` | `apps/website/frontend/src/v2/pages/administration/content_manager/page.md` |
| `administration/event_manager/README.md` | F01 | (writer creates) | `documentation_v2/website/frontend/pages/administration/event_manager/README.md` |
| `administration/event_manager/event_manager_page.md` | F01 | `docs/website/frontend/pages/event-manager.md` | `apps/website/frontend/src/v2/pages/administration/event_manager/page.md` |
| `administration/personnel/README.md` | F01 | (writer creates) | `documentation_v2/website/frontend/pages/administration/personnel/README.md` |
| `administration/personnel/personnel_roster_page.md` | F01 | `docs/website/frontend/pages/personnel-roster.md` | `apps/website/frontend/src/v2/pages/administration/personnel/page.md` |
| `administration/server_control/README.md` | F01 | (writer creates) | `documentation_v2/website/frontend/pages/administration/server_control/README.md` |
| `administration/server_control/server_control_page.md` | F01 | `docs/website/frontend/pages/server-control.md` | `apps/website/frontend/src/v2/pages/administration/server_control/page.md` |
| `command_center/README.md` | F02 | (writer creates) | `documentation_v2/website/frontend/pages/command_center/README.md` |
| `command_center/announcements/announcements_page.md` | F02 | `docs/website/frontend/pages/announcements.md` | `apps/website/frontend/src/v2/pages/command_center/announcements/page.md` |
| `command_center/dashboard/dashboard_page.md` | F02 | `docs/website/frontend/pages/dashboard.md` | `apps/website/frontend/src/v2/pages/command_center/dashboard/page.md` |
| `command_center/server_intel/server_intel_page.md` | F02 | `docs/website/frontend/pages/server-intel.md` | `apps/website/frontend/src/v2/pages/command_center/server_intel/page.md` |
| `doctrine_and_info/README.md` | F03 | (writer creates) | `documentation_v2/website/frontend/pages/doctrine_and_info/README.md` |
| `doctrine_and_info/modpacks/README.md` | F03 | (writer creates) | `documentation_v2/website/frontend/pages/doctrine_and_info/modpacks/README.md` |
| `doctrine_and_info/modpacks/modpacks_page.md` | F03 | `docs/website/frontend/pages/modpacks.md` | `apps/website/frontend/src/v2/pages/doctrine_and_info/modpacks/page.md` |
| `doctrine_and_info/vehicles/README.md` | F03 | (writer creates) | `documentation_v2/website/frontend/pages/doctrine_and_info/vehicles/README.md` |
| `doctrine_and_info/vehicles/vehicle_database_page.md` | F03 | `docs/website/frontend/pages/vehicle-database.md` | `apps/website/frontend/src/v2/pages/doctrine_and_info/vehicles/page.md` |
| `doctrine_and_info/wiki/README.md` | F03 | (writer creates) | `documentation_v2/website/frontend/pages/doctrine_and_info/wiki/README.md` |
| `doctrine_and_info/wiki/wiki_page.md` | F03 | `docs/website/frontend/pages/wiki.md` | `apps/website/frontend/src/v2/pages/doctrine_and_info/wiki/page.md` |
| `field_tools/README.md` | F03 | (writer creates) | `documentation_v2/website/frontend/pages/field_tools/README.md` |
| `field_tools/mortar/README.md` | F03 | (writer creates) | `documentation_v2/website/frontend/pages/field_tools/mortar/README.md` |
| `field_tools/mortar/mortar_calculator_page.md` | F03 | `docs/website/frontend/pages/mortar-calculator.md` | `apps/website/frontend/src/v2/pages/field_tools/mortar/page.md` |
| `mission_hub/README.md` | F03 | (writer creates) | `documentation_v2/website/frontend/pages/mission_hub/README.md` |
| `mission_hub/create_dialog/README.md` | F03 | (writer creates) | `documentation_v2/website/frontend/pages/mission_hub/create_dialog/README.md` |
| `mission_hub/library/README.md` | F03 | (writer creates) | `documentation_v2/website/frontend/pages/mission_hub/library/README.md` |
| `mission_hub/library/mission_library_page.md` | F03 | `docs/website/frontend/pages/mission-library.md` | `apps/website/frontend/src/v2/pages/mission_hub/library/page.md` |
| `mission_hub/overview/README.md` | F03 | (writer creates) | `documentation_v2/website/frontend/pages/mission_hub/overview/README.md` |
| `mission_hub/overview/mission_overview_page.md` | F03 | `docs/website/frontend/pages/mission-overview.md` | `apps/website/frontend/src/v2/pages/mission_hub/overview/page.md` |
| `mission_hub/review_workspace/review_workspace_page.md` | F03 | (writer creates) | `apps/website/frontend/src/v2/pages/mission_hub/review_workspace/page.md` |
| `navigation/README.md` | F04 | (writer creates) | `documentation_v2/website/frontend/pages/navigation/README.md` |
| `navigation/app_layout_and_navigation.md` | F04 | `docs/website/frontend/shell/sidebar.md` | `apps/website/frontend/src/v2/pages/navigation/layout.md` · `apps/website/frontend/src/v2/pages/navigation/nav_config.md` · `docs/website/frontend/pages/not-found.md` · `docs/website/frontend/shell/app-layout.md` · `docs/website/frontend/shell/topnav.md` |
| `operations/README.md` | F02 | (writer creates) | `documentation_v2/website/frontend/pages/operations/README.md` |
| `operations/deployments/README.md` | F02 | (writer creates) | `documentation_v2/website/frontend/pages/operations/deployments/README.md` |
| `operations/deployments/deployments_page.md` | F02 | `docs/website/frontend/pages/deployments.md` | - |
| `operations/event_detail/README.md` | F02 | (writer creates) | `documentation_v2/website/frontend/pages/operations/event_detail/README.md` |
| `operations/event_detail/event_hub_page.md` | F02 | `docs/website/frontend/pages/event-hub.md` | `apps/website/frontend/src/v2/pages/operations/event_detail/page.md` |
| `operations/leaderboards/README.md` | F02 | (writer creates) | `documentation_v2/website/frontend/pages/operations/leaderboards/README.md` |
| `operations/leaderboards/leaderboards_page.md` | F02 | `docs/website/frontend/pages/leaderboards.md` | - |
| `operations/orbat_selection/README.md` | F02 | (writer creates) | `documentation_v2/website/frontend/pages/operations/orbat_selection/README.md` |
| `operations/orbat_selection/orbat_selection_page.md` | F02 | (writer creates) | `apps/website/frontend/src/v2/pages/operations/orbat_selection/page.md` |
| `operations/schedule/README.md` | F02 | (writer creates) | `documentation_v2/website/frontend/pages/operations/schedule/README.md` |
| `operations/schedule/event_schedule_page.md` | F02 | `docs/website/frontend/pages/event-schedule.md` | `apps/website/frontend/src/v2/pages/operations/schedule/page.md` |

## documentation_v2/website/graphics-engine/

| final target | writer | primary source | pending_merge sources |
|---|---|---|---|
| `README.md` | F09 | (writer creates) | `documentation_v2/website/graphics_engine/README.md` |

## documentation_v2/website/map-engine/

| final target | writer | primary source | pending_merge sources |
|---|---|---|---|
| `README.md` | F09 | (writer creates) | `documentation_v2/website/map_engine/README.md` |

## Targets with no source row

The plan names these documents; no file feeds them, so no manifest row exists and the writer
creates each one: `runbooks/ticket_run_pipeline.md` (F16), `standards/engine_boundary_rules.md`
(F09), `product_roadmap.md` (F19), `glossary.md` (P3-2), the `create_dialog/` feature doc and
the `/debug/world-los` page doc (F03), the aar and planner feature docs (F08), and the api_v2
environment reference and domain docs (F09). All paths are under `documentation_v2/`.

