# Tools V2 Phase Five Handoff

Phase five closes the tooling tree. Across the phases recorded below the repository root stops carrying a `scripts/` directory, `tools_v2/` stops carrying empty and pointer-only directories, every command spelling, task name, gate step, fixture directory, artifact file and printed label takes the name of the domain it serves, all four tooling crates hold production files under 500 lines with their unit tests in sibling files, each crate owns exactly one module that spells the repository paths it reads outside its own tree, every comment and document in the tree states what the code does now and why, and the root and agent documents name only files and commands that exist. Each phase lands as one commit on `main` and appends its own section here with the commands it ran and what they returned.

## Baseline

Measured on a clean working tree at HEAD `b76490e6d874d38df70f87d25fef34547abb1ae7`, with no edits. Row identifiers are the verification matrix rows of the closure plan, which holds the command texts; this document records identifiers and values, because the matrix searches this file too and its command texts contain the very tokens the matrix drives to zero.

| Row | What it counts | Measured | Value recorded in the plan |
|---|---|---|---|
| R1 | Ticket identifiers in `tools_v2` sources, manifests, documents and data, fixture trees excluded | 2959 lines: 2939 in `.rs`, 20 in `.md`/`.toml`/`.json`; 1887 open with a comment marker; of the 2018 lines in production `.rs` files, 407 are not comment lines | 2959 (1881 comment, 387 production non-comment, 20 in documents and data) |
| R1b | Ticket identifiers and node script names in the browser-oracle freeze manifest | 4, at the fixture tree's present name; the row's command names the tree's name after the rename, which does not exist yet | 4 |
| R2 | Dead names anywhere under `tools_v2` | 201 | 187 |
| R3 | Names of deleted shell, python and node scripts in `tools_v2` sources, documents and manifests | 597 | 602 |
| R4 | Empty directories under `tools_v2` | 138 | 138 |
| R5 | Root script tree present; tracked files under it; repository-wide references to it | present; 17 tracked files; 355 reference lines | present; 17; 363 |
| R5a | R5's references minus `tools_v2` documents and minus comment lines | 160 lines in 69 files: 92 in `.rs`, 62 in `.md`, 4 in service unit files, 2 in ignore files | 93 |
| R7 | Words that narrate a change rather than the present state | 112 | 112 |
| R8 | Repository path literals outside the crate layout modules | part one 344 lines over all files (ticket-engine 216, xtask 56, developer-tools 21, ticketboard 51), 159 of them in production files, which is what the row's command reports; part two 0, the destinations not existing yet | 347; — |
| R9 | Agent instructions and hub documents naming commands, files and crates that do not exist; tracked root ghost files | 56 lines; 3 tracked paths | 55; 3 |
| R10 | Root agent document and its mirror | identical; the mirror is untracked | identical |
| R17 | Verification functions named after a ticket; `AssetLayout` uses in the deployment preflight | 14 functions; 8 lines over 4 variants | 10; 4 variants |
| R18 | Distinct `.rs` basenames named in `tools_v2` production prose that exist nowhere in the workspace | 161 | 167 |

Where the measured value differs from the value recorded in the plan, the measured value is the baseline: both tools_v2 and the plan were read at the same commit, so the difference is in how each count was taken, and every row's target is zero regardless of where it starts.

### Build, test and gate state

| Command | Result |
|---|---|
| `cargo check --workspace --locked` | exit 0 in 32.09s; 244 warnings, all from `website-frontend` |
| `cargo test -p xtask -p developer-tools -p verification-core -p ticket-engine -p ticketboard` | exit 0; 1357 passed, 0 failed, 7 ignored |
| `cargo xtask verify file-length` | exit 0; scanned 2529 `.rs` files, 0 violations |
| `cargo xtask ci ci-local` | exit 0; every step green: editorconfig, the four language bans, engine layers, rust fmt, clippy and build, the wasm build, the backend integration suite of 294 tests against the running database, coding standards, document layout, the Leptos build, schema validation, contract citations, the staging compose-path check, the mission upload size-gate check and the schema-parity check. The size-gate check prints its own RED proofs of non-vacuity, which are part of its PASS |

Test totals per target: developer-tools library 254 passed and 4 ignored, its six binaries carrying no tests; ticket-engine library 213 passed plus 1 compile-failure test; ticketboard 170 passed and 3 ignored; verification-core library 68 passed plus 1 documentation test; xtask 650 passed. The names behind those totals are listed under `Test inventory` at the end of this document, so a later phase can show that a renamed test is the same test and that no live test disappeared; a phase that renames a test updates its line there.

### Environment

The shell is a Debian 12 container whose builds land in `target-glibc236`. The development database container `tbd_reforger_db` runs on the host and answers on port 5434; container-side commands that reach it resolve `podman` through a shim that forwards to the host. `cargo xtask db up` cannot run here because the host has no compose provider, and no step of this phase needs it: the container is already up and healthy.

### Found and fixed

This phase measures and edits no code, so it fixes nothing. Everything it found is listed below against the phase that owns the file.

### Found for P5

- `tools_v2/xtask/src/verifications/mod_scripts/results_reporter_identity_comments.rs:186`, `tools_v2/xtask/src/verifications/registry/object_registry_aliases.rs:112`, `tools_v2/xtask/src/verifications/mod_scripts/player_identity_comments.rs:110`, `tools_v2/xtask/src/verifications/mod_scripts/mission_rest_size_limits.rs:38`: four ticket-named verification functions the plan's R17 note does not list, which is why R17 measures 14 and not 10. The rename table already covers all four command spellings, so apply the same rule to the function names: function name equals `verify_` plus the module file name.
- `tools_v2/ticket-engine/src/registry/tests/typed_projection/typed_projection_tests.rs:45`: the test function is named after the ticket whose shipped commit it pins; rename it to `shipped_ticket_keeps_its_shipped_at_commit`. The ticket identifier the body parses is a string literal in a test file and stays; the function name is what R2 counts.
- `tools_v2/xtask/src/verifications/schemas/tests/checks/instance_kind_lockstep_tests.rs:13`: the test function name ends in a crate name that no longer exists; rename it to `instance_kinds_match_the_enums_schema_and_the_crate_array`. The doc comment above it cites a ticket as provenance, so replace it with what the guard asserts.
- Both function names above, and the eleven under the modules P7 renames, appear in the inventory at the end of this document. R2 counts that inventory, so each rename carries its inventory line with it: thirteen lines in all, and R2 stays above zero until they are updated.

### Found for P6

- `tools_v2/verification-core/src/lock.rs:240`: `fn interops_with_the_flock_command_used_by_wave_sh()` → `fn interops_with_the_flock_command()`, landing with the extraction of that inline test module.
- `tools_v2/verification-core/src/lock.rs` (lines 5, 12, 29, 45, 51, 53, 78, 205, 248), `src/lib.rs:38` and `src/verdict.rs:34` attribute the lock protocol and the log contract to a deleted shell driver. The live holder of that lock is `cargo xtask platform wave`, whose `wave_execution` module reads `gate_lock`, `gate_lock_poll` and `gate_lock_max` (`tools_v2/xtask/src/commands/platform/wave_execution/mod.rs:226-228,322-330`); name that command and keep the protocol facts the comments carry.

### Found for P7

- Eleven names in the inventory below sit under the two modules this phase renames: ten under the ticket-file storage module and one under the archived wave plan module. Update those lines when the modules move, so the inventory keeps describing the live test set.

### Found for P10

- The verification matrix searches `tools_v2` documents, this one included, so a section that pastes the R1, R2, R3, R5 or R7 command texts into this file puts those rows above zero by itself. Record row identifiers and results here and leave the command texts in the plan.

### Acceptance

| Command | Expected | Actual |
|---|---|---|
| `git grep -c -E '\bT-[0-9]' tools_v2/PHASE_FIVE_HANDOFF.md` | 0 | 0 |
| `git status --porcelain` before the commit | only the new handoff file | only `tools_v2/PHASE_FIVE_HANDOFF.md` (`?? ` unstaged, `A  ` staged) |
| `git status --porcelain` after the commit | empty | empty |

## Test inventory

The 1361 test functions the five tooling crates run, one per line as `target | test path`. A phase that renames, adds or deletes a test updates this list, so it keeps naming the tests that exist.

```text
developer_tools | blueprint::archive_emit::tests::archive_boot_splits_the_whole_corpus_and_never_censuses_a_blocking_prefab
developer_tools | blueprint::archive_emit::tests::archive_carries_every_committed_blueprint_level
developer_tools | blueprint::archive_emit::tests::archive_round_trips_every_committed_descriptor
developer_tools | blueprint::archive_emit::tests::report_names_the_buildings_still_missing_levels_and_the_command_that_fills_them
developer_tools | blueprint::batch::tests::cover_and_kind_heuristics
developer_tools | blueprint::batch::tests::layer_policy_keeps_fire_records_and_drops_physics_shells
developer_tools | blueprint::batch::tests::real_farmhouse_closure_counts_are_pinned
developer_tools | blueprint::batch::tests::walker_places_door_set_window_and_furniture_from_fixtures
developer_tools | blueprint::bvh::compound_tests::farmhouse_compound_door_parity_is_pinned
developer_tools | blueprint::bvh::tests::farmhouse_bvh_sidecar_parity_is_pinned
developer_tools | blueprint::emit::tests::box_room_blueprint_passes_the_schema_contract
developer_tools | blueprint::hull::tests::cube_hull_is_twelve_outward_triangles
developer_tools | blueprint::hull::tests::prism_like_trunk_hull_closes
developer_tools | blueprint::hull::tests::tetrahedron_and_degenerate_inputs
developer_tools | blueprint::library::tests::blas_dedup_by_stem_manifest_entries_and_hot_order
developer_tools | blueprint::library::tests::committed_farmhouse_descriptor_reproduces_its_instances_file
developer_tools | blueprint::library::tests::descriptors_carry_blocks_reasons_kinds_and_canopy
developer_tools | blueprint::library::tests::hull_sample_keeps_at_most_26_extreme_points
developer_tools | blueprint::library::tests::only_kind_and_limit_select_rows
developer_tools | blueprint::library::tests::write_is_schema_valid_and_deterministic
developer_tools | blueprint::mesh::tests::axes_remap_parses_and_applies
developer_tools | blueprint::mesh::tests::cube_matches_analytic_box
developer_tools | blueprint::mesh::tests::min_sep_merges_close_hits
developer_tools | blueprint::mesh::tests::open_sheet_is_one_sided
developer_tools | blueprint::mesh::tests::wedge_slope_registers_on_vertical_march
developer_tools | blueprint::mesh::tests::winding_flip_detected_and_corrected
developer_tools | blueprint::mesh::tests::written_dump_round_trips_through_strict_parser
developer_tools | blueprint::pair::tests::clean_wall_pairs
developer_tools | blueprint::pair::tests::consumed_closing_face_cannot_double_pair
developer_tools | blueprint::pair::tests::one_sided_forward_and_backward
developer_tools | blueprint::pair::tests::two_walls_with_doorway_do_not_bridge
developer_tools | blueprint::params::tests::defaults_match_live_pipeline_constants
developer_tools | blueprint::params::tests::partial_override_keeps_other_defaults
developer_tools | blueprint::params::tests::unknown_key_rejected
developer_tools | blueprint::parse::tests::descending_required_for_minus_runs
developer_tools | blueprint::parse::tests::march_order_violation_fails
developer_tools | blueprint::parse::tests::missing_end_line_is_truncation
developer_tools | blueprint::parse::tests::round_trip_minimal
developer_tools | blueprint::parse::tests::wrong_line_count_fails
developer_tools | blueprint::plate::tests::plate_keeps_topmost_in_window_entry_and_marks_occupancy
developer_tools | blueprint::prefab::tests::resolver_walks_inheritance_sockets_and_children
developer_tools | blueprint::prefab::tests::tokenizer_and_block_shapes
developer_tools | blueprint::rings::tests::diagonal_touch_stays_two_separate_rings
developer_tools | blueprint::rings::tests::disconnected_pieces_become_two_polygons
developer_tools | blueprint::rings::tests::donut_has_one_cw_hole
developer_tools | blueprint::rings::tests::full_rect_is_one_ccw_ring_of_four
developer_tools | blueprint::rings::tests::l_shape_traces_six_vertices
developer_tools | blueprint::rings::tests::min_area_drops_noise_rings_and_counts_them
developer_tools | blueprint::rings::tests::single_cell_is_a_four_vertex_ring
developer_tools | blueprint::rings::tests::trace_is_deterministic
developer_tools | blueprint::roof::tests::box_room_flat_roof_covers_plate
developer_tools | blueprint::roof::tests::coverage_ground_filter_and_erosion_guards
developer_tools | blueprint::roof::tests::gable_box_grid_min_biases_low
developer_tools | blueprint::rotation_pin::tests::garbage_container_lid_pins_y_x_z_with_negated_pitch_and_roll
developer_tools | blueprint::rotation_pin::tests::rigid_from_enfusion_is_the_y_x_z_hypothesis
developer_tools | blueprint::slabs::tests::box_room_yields_single_ground_slab
developer_tools | blueprint::slabs::tests::gable_slope_field_flags_roof_planes
developer_tools | blueprint::surface_kind::tests::gamemat_stems_classify
developer_tools | blueprint::surface_kind::tests::projectile_presets_follow_the_collision_layer_table
developer_tools | blueprint::tests::farmhouse_dump_matches_golden_blueprint
developer_tools | blueprint::tests::farmhouse_golden_parity_is_pinned
developer_tools | blueprint::tests::gable_mezzanine_bands_plate_and_knee_wall
developer_tools | blueprint::verify::tests::farmhouse_sockets_match_the_workbench_recon
developer_tools | blueprint::verify::tests::matches_through_the_building_yaw_and_skips_furniture_descendants
developer_tools | blueprint::verify::tests::recon_groups_from_class_and_components
developer_tools | blueprint::walls::tests::box_room_yields_four_walls_both_algos
developer_tools | blueprint::walls::tests::doorway_splits_wall_and_does_not_bridge
developer_tools | blueprint::walls::tests::gable_second_band_emits_gable_ends_and_zero_roof_phantoms
developer_tools | blueprint::walls::tests::segments_box_walls_are_centerline_accurate
developer_tools | blueprint::walls::tests::sparse_noise_fails_min_persist_rows_floor
developer_tools | blueprint::walls::tests::steep_graze_grid_phantoms_segments_clean
developer_tools | blueprint::world_row::tests::chunk_id_is_the_floor_partition
developer_tools | blueprint::world_row::tests::farmhouse_chunk_row_places_every_socket_child_within_2cm
developer_tools | blueprint::world_row::tests::garbage_container_row_carries_pitch_and_roll
developer_tools | blueprint::world_row::tests::pid_lookup_ignores_the_guid_prefix
developer_tools | blueprint::xob::tests::coll_box_record_becomes_twelve_triangles
developer_tools | blueprint::xob::tests::coll_convex_and_plain_trimesh_records_parse
developer_tools | blueprint::xob::tests::coll_grammar_drift_is_rejected
developer_tools | blueprint::xob::tests::coll_trimesh_record_parses_and_transforms
developer_tools | blueprint::xob::tests::lz4_literals_and_match_round_trip
developer_tools | blueprint::xob::tests::lz4_match_reaches_across_block_boundary
developer_tools | blueprint::xob::tests::lz4_overlapping_match_replicates
developer_tools | blueprint::xob::tests::tiny_xob_parses_to_quad
developer_tools | blueprint::xob::tests::wrong_magic_is_rejected
developer_tools | blueprint::xob_nodes::tests::hierarchy_less_models_keep_their_string_table
developer_tools | blueprint::xob_nodes::tests::node_table_decodes_sockets_and_the_name_space_starts_at_the_first_material
developer_tools | blueprint::xob_nodes::tests::real_farmhouse_nodes_sockets_and_materials
developer_tools | browser_testing::dom_oracle::routes::fixture_router::tests::a_json_fixture_is_served_minified
developer_tools | browser_testing::dom_oracle::routes::fixture_router::tests::a_non_api_request_passes_through_to_the_static_server
developer_tools | browser_testing::dom_oracle::routes::fixture_router::tests::a_path_becomes_a_method_prefixed_corpus_name
developer_tools | browser_testing::dom_oracle::routes::fixture_router::tests::a_query_string_does_not_change_which_fixture_answers
developer_tools | browser_testing::dom_oracle::routes::fixture_router::tests::an_event_stream_body_keeps_its_literal_frame_delimiter
developer_tools | browser_testing::dom_oracle::routes::fixture_router::tests::an_unanswered_api_call_is_reported_rather_than_filled_in
developer_tools | browser_testing::dom_oracle::routes::fixture_router::tests::the_method_selects_the_fixture_so_a_post_corpus_entry_is_reachable
developer_tools | browser_testing::dom_oracle::routes::fixture_router::tests::the_refusal_names_every_missing_file_and_the_url_that_wanted_it
developer_tools | browser_testing::dom_oracle::routes::fixture_router::tests::the_status_stream_resolves_to_an_event_stream_fixture
developer_tools | browser_testing::dom_oracle::routes::fixture_router::tests::the_token_endpoints_are_answered_without_a_fixture
developer_tools | browser_testing::dom_oracle::tests::accept_allows_plausible_object
developer_tools | browser_testing::dom_oracle::tests::accept_refuses_literal_null
developer_tools | browser_testing::dom_oracle::tests::accept_refuses_non_json
developer_tools | browser_testing::dom_oracle::tests::accept_refuses_undersized_object
developer_tools | browser_testing::editor_smoke_tests::assert_js_ok_tests::bare_diagnostic_object_fails
developer_tools | browser_testing::editor_smoke_tests::assert_js_ok_tests::diagnostic_string_is_echoable_not_pass
developer_tools | browser_testing::editor_smoke_tests::assert_js_ok_tests::literal_true_ok_literal_false_fails
developer_tools | browser_testing::editor_smoke_tests::assert_js_ok_tests::pass_false_object_fails
developer_tools | browser_testing::editor_smoke_tests::assert_js_ok_tests::pass_non_boolean_fails
developer_tools | browser_testing::editor_smoke_tests::assert_js_ok_tests::pass_true_object_ok
developer_tools | browser_testing::fixture_injection::tests::payloads_are_pinned
developer_tools | browser_testing::server::tests::close_does_not_hang_on_a_still_open_stream
developer_tools | browser_testing::server::tests::finite_responses_are_unchanged_by_streaming
developer_tools | browser_testing::server::tests::sse_frames_arrive_incrementally_not_buffered
developer_tools | content_digest::tests::an_unreadable_path_is_none_rather_than_an_empty_digest
developer_tools | content_digest::tests::hashes_whole_file_bytes_including_the_trailing_newline
developer_tools | content_digest::tests::matches_the_published_abc_vector
developer_tools | enfusion_pak::blueprint_source_tests::malformed_pak_images_are_rejected
developer_tools | enfusion_pak::blueprint_source_tests::real_pak_census
developer_tools | enfusion_pak::blueprint_source_tests::real_pak_farmhouse_xob_matches_extract
developer_tools | enfusion_pak::blueprint_source_tests::synthetic_pak_lists_and_reads_stored_and_zlib_files
developer_tools | enfusion_pak::policy_parity_tests::decompressed_length_checks_follow_the_consumer_policy
developer_tools | enfusion_pak::policy_parity_tests::lookup_and_duplicate_policies_remain_distinct
developer_tools | enfusion_pak::policy_parity_tests::malformed_archives_fail_blueprint_and_are_skipped_by_world
developer_tools | enfusion_pak::policy_parity_tests::payload_truncation_and_missing_loose_fallback_are_errors
developer_tools | enfusion_pak::policy_parity_tests::raw_deflate_is_world_only_and_metadata_is_preserved
developer_tools | enfusion_pak::world_source::tests::read_file_compressed_returns_inflated_bytes
developer_tools | enfusion_pak::world_source::tests::read_file_uncompressed_returns_stored_bytes
developer_tools | enfusion_pak::world_source::tests::read_raw_returns_stored_bytes
developer_tools | enfusion_pak::world_source::tests::synthetic_pak_parses_with_data_start_56
developer_tools | enfusion_tooling::apidoc::tests::build_refuses_header_only_classes_tsv
developer_tools | enfusion_tooling::apidoc::tests::build_refuses_header_only_members_tsv
developer_tools | enfusion_tooling::apidoc::tests::parses_index_rows
developer_tools | enfusion_tooling::apidoc::tests::strips_entities
developer_tools | enfusion_tooling::citations::tests::extracts_markers
developer_tools | enfusion_tooling::citations::tests::ignores_prose_without_markers
developer_tools | enfusion_tooling::enfusion_mcp_entrypoint::tests::a_root_without_an_installed_package_falls_through_to_the_download_tier
developer_tools | enfusion_tooling::enfusion_mcp_entrypoint::tests::an_installed_package_resolves_to_the_pinned_module
developer_tools | enfusion_tooling::enfusion_mcp_entrypoint::tests::every_source_has_a_self_describing_label
developer_tools | enfusion_tooling::enfusion_mcp_entrypoint::tests::the_process_pattern_is_the_escaped_installed_module_suffix
developer_tools | enfusion_tooling::refuse_empty_tests::refuse_empty_write_ok_when_nonempty
developer_tools | enfusion_tooling::refuse_empty_tests::refuse_empty_write_reds_on_empty
developer_tools | enfusion_tooling::source::tests::demangles_doxygen_names
developer_tools | enfusion_tooling::source::tests::parses_source_lines_and_drops_numbers
developer_tools | enfusion_tooling::symbols::tests::base_class_is_exact
developer_tools | enfusion_tooling::symbols::tests::captures_rplprop_with_callback
developer_tools | enfusion_tooling::symbols::tests::control_flow_is_not_a_method
developer_tools | enfusion_tooling::symbols::tests::does_not_invent_apis
developer_tools | enfusion_tooling::symbols::tests::finds_real_declarations
developer_tools | enfusion_tooling::symbols::tests::modded_class_kind_is_distinct
developer_tools | map_raster_pipeline::inland_water_archive::tests::a_depth_that_does_not_fit_the_container_is_refused_not_truncated
developer_tools | map_raster_pipeline::inland_water_archive::tests::a_meta_that_cannot_describe_a_grid_is_refused
developer_tools | map_raster_pipeline::inland_water_archive::tests::a_missing_terrain_or_staging_directory_exits_one
developer_tools | map_raster_pipeline::inland_water_archive::tests::a_raster_that_disagrees_with_the_meta_is_refused
developer_tools | map_raster_pipeline::inland_water_archive::tests::a_vector_export_the_archive_cannot_represent_is_refused
developer_tools | map_raster_pipeline::inland_water_archive::tests::an_export_with_no_water_at_all_is_refused
developer_tools | map_raster_pipeline::inland_water_archive::tests::bytes_are_deterministic_across_runs
developer_tools | map_raster_pipeline::inland_water_archive::tests::every_emitted_mip_level_agrees_with_level_zero
developer_tools | map_raster_pipeline::inland_water_archive::tests::mip_count_halves_to_one_texel
developer_tools | map_raster_pipeline::inland_water_archive::tests::odd_dimensions_emit_a_consistent_pyramid
developer_tools | map_raster_pipeline::inland_water_archive::tests::scratch_dir_pairs_with_terrain_dir_without_nesting_inside_it
developer_tools | map_raster_pipeline::inland_water_archive::tests::terrain_dir_takes_an_id_or_a_directory
developer_tools | map_raster_pipeline::inland_water_archive::tests::the_emitted_container_answers_the_placement_query
developer_tools | map_raster_pipeline::inland_water_archive::tests::the_vectors_archive_carries_every_lane_with_its_surface_height
developer_tools | map_raster_pipeline::map_label_archives::tests::everon_height_labels_parse_the_same_from_either_source
developer_tools | map_raster_pipeline::map_label_archives::tests::everon_override_is_load_bearing_below_the_secondary_gate
developer_tools | map_raster_pipeline::map_label_archives::tests::everon_road_labels_match_the_json_draw_set_at_every_zoom
developer_tools | map_raster_pipeline::map_label_archives::tests::everon_towns_parse_the_same_from_either_source
developer_tools | map_raster_pipeline::map_label_archives::tests::road_names_without_geometry_are_refused
developer_tools | map_raster_pipeline::map_label_archives::tests::terrain_dir_takes_an_id_or_a_directory
developer_tools | map_raster_pipeline::map_label_archives::tests::the_written_file_reads_back_through_access_checked
developer_tools | map_raster_pipeline::map_labels::map_labels_tests::height_labels_refuse_empty_contract
developer_tools | map_raster_pipeline::refuse_empty_tests::refuse_empty_write_ok_when_nonempty
developer_tools | map_raster_pipeline::refuse_empty_tests::refuse_empty_write_reds_on_empty
developer_tools | map_raster_pipeline::satellite_archive_container::container_tests::a_grid_that_disagrees_with_tile_px_is_rejected
developer_tools | map_raster_pipeline::satellite_archive_container::container_tests::an_index_len_one_byte_short_is_rejected
developer_tools | map_raster_pipeline::satellite_archive_container::container_tests::an_unknown_tile_format_is_rejected
developer_tools | map_raster_pipeline::satellite_archive_container::container_tests::the_v2_derivation_reproduces_the_committed_everon_tiling
developer_tools | map_raster_pipeline::satellite_archive_container::container_tests::v1_and_v2_carry_the_same_tiles_at_the_same_rects
developer_tools | map_verification::terrain_manifest::tests::a_chunks_dir_holding_no_bin_is_dangling
developer_tools | map_verification::terrain_manifest::tests::a_row_shape_this_build_cannot_read_is_refused
developer_tools | map_verification::terrain_manifest::tests::dangling_binary_paths_are_rejected_one_by_one
developer_tools | map_verification::terrain_manifest::tests::every_binary_block_is_accepted_when_its_paths_exist
developer_tools | map_verification::terrain_manifest::tests::live_pod_row_doc_matches_the_rust_pod
developer_tools | map_verification::terrain_manifest::tests::occluder_init_still_fetches_the_blas_manifest_for_hot_chunks
developer_tools | map_verification::terrain_manifest::tests::pod_row_doc_reds_on_a_shifted_offset_and_on_a_missing_block
developer_tools | map_verification::terrain_manifest::tests::the_live_everon_manifest_declares_the_cutover_blocks_and_passes
developer_tools | map_verification::world_line_of_sight::tests::cell_18_0_loads_with_no_proxy_rows_and_names_the_farmhouse
developer_tools | map_verification::world_line_of_sight::tests::farmhouse_descriptor_placed_at_a_yaw_replays_the_door_parity_fixture
developer_tools | map_verification::world_line_of_sight::tests::foliage_as_a_blocker_disagrees_with_the_projectile_trace
developer_tools | map_verification::world_line_of_sight::tests::world_parity_cell_18_0_is_pinned
developer_tools | map_verification::world_line_of_sight::tests::world_parity_forest_cell_is_pinned
developer_tools | map_verification::world_line_of_sight::tests::world_parity_world_column_clears_its_floor_when_the_dem_is_present
developer_tools | repository_layout::tests::every_declared_location_exists_in_the_checkout
developer_tools | repository_layout::tests::export_scratch_is_named_for_its_island_and_sits_outside_the_served_tree
developer_tools | repository_layout::tests::locations_resolve_against_the_given_root
developer_tools | repository_layout::tests::the_enfusion_mcp_entrypoint_sits_inside_its_npm_package_directory
developer_tools | repository_paths::tests::compiler_fixtures_resolve_from_root_crate_and_source_directory
developer_tools | repository_paths::tests::missing_repository_marker_is_an_error
developer_tools | world_export_pipeline::binary_emit::tests::empty_chunk_is_a_bare_header
developer_tools | world_export_pipeline::binary_emit::tests::every_everon_chunk_bin_decodes_to_the_json_columns
developer_tools | world_export_pipeline::binary_emit::tests::five_wide_row_takes_identity_trailers_and_positive_zero
developer_tools | world_export_pipeline::binary_emit::tests::out_of_range_chunk_index_is_an_error_not_a_wrap
developer_tools | world_export_pipeline::binary_emit::tests::rejected_rows_are_skipped_with_no_gap
developer_tools | world_export_pipeline::catalog_emit::tests::a_census_from_another_export_is_refused
developer_tools | world_export_pipeline::catalog_emit::tests::a_terrain_without_regions_emits_only_the_catalogue
developer_tools | world_export_pipeline::catalog_emit::tests::an_empty_catalogue_is_refused
developer_tools | world_export_pipeline::catalog_emit::tests::everon_catalog_archives_emit_and_read_back_as_their_json
developer_tools | world_export_pipeline::catalog_emit::tests::standalone_census_matches_the_catalogue
developer_tools | world_export_pipeline::catalog_emit::tests::the_manifest_block_names_the_paths_this_emitter_writes
developer_tools | world_export_pipeline::chunk_partitioner::tests::clear_density_skips_when_not_rebuilding
developer_tools | world_export_pipeline::chunk_partitioner::tests::clear_density_wipes_when_rebuilding
developer_tools | world_export_pipeline::chunk_partitioner::tests::non_density_phase_must_not_clear
developer_tools | world_export_pipeline::chunk_partitioner::tests::p5_admits_the_vehicle_lane
developer_tools | world_export_pipeline::chunk_partitioner::tests::refuse_empty_catalog_write_contract
developer_tools | world_export_pipeline::chunk_partitioner::tests::the_road_census_reads_the_committed_roads_file
developer_tools | world_export_pipeline::enfusion_texture_decoder::tests::bc7_decodes_known_block
developer_tools | world_export_pipeline::enfusion_texture_decoder::tests::bc7_rejects_bad_dims
developer_tools | world_export_pipeline::enfusion_texture_decoder::tests::lz4_roundtrip_simple
developer_tools | world_export_pipeline::export_preparation::elevation_dem_tests::elevation_dem_falls_back_to_the_v4_range_when_meta_omits_it
developer_tools | world_export_pipeline::export_preparation::elevation_dem_tests::everon_elevation_dem_matches_the_shipped_png
developer_tools | world_export_pipeline::export_preparation::elevation_dem_tests::raw_u16_to_dem_png_also_emits_elevation_dem_with_the_same_grid
developer_tools | world_export_pipeline::export_preparation::elevation_dem_tests::write_elevation_dem_frames_the_grid_and_round_trips
developer_tools | world_export_pipeline::export_preparation::elevation_dem_tests::write_elevation_dem_refuses_a_grid_that_is_not_width_times_height
developer_tools | world_export_pipeline::forest_smoothing::tests::a_degenerate_ring_does_not_produce_nan
developer_tools | world_export_pipeline::forest_smoothing::tests::a_hole_reads_the_canopy_the_other_way_round
developer_tools | world_export_pipeline::forest_smoothing::tests::a_hole_ring_keeps_its_sign_and_its_area
developer_tools | world_export_pipeline::forest_smoothing::tests::a_square_ring_holds_its_area_under_the_bound_at_every_scale
developer_tools | world_export_pipeline::forest_smoothing::tests::a_square_ring_rounds
developer_tools | world_export_pipeline::forest_smoothing::tests::a_three_point_ring_stays_valid
developer_tools | world_export_pipeline::forest_smoothing::tests::everon_smooths_within_the_area_bound_and_stops_being_a_staircase
developer_tools | world_export_pipeline::forest_smoothing::tests::the_area_solver_hits_its_target_in_closed_form
developer_tools | world_export_pipeline::forest_smoothing::tests::the_canopy_oracle_pins_corners_and_changes_the_ring
developer_tools | world_export_pipeline::forest_smoothing::tests::the_corner_probe_clears_the_vertexs_own_density_cell
developer_tools | world_export_pipeline::forest_smoothing::tests::the_emit_leaves_a_bare_four_vertex_square_alone
developer_tools | world_export_pipeline::forest_smoothing::tests::uncompensated_chaikin_would_blow_the_bound
developer_tools | world_export_pipeline::instance_kind_tests::instance_kinds_match_enums_schema
developer_tools | world_export_pipeline::json_number_formatting::transform_row_tests::any_nontrivial_trailer_writes_eight_wide
developer_tools | world_export_pipeline::json_number_formatting::transform_row_tests::negative_zero_angles_round_to_positive_zero
developer_tools | world_export_pipeline::json_number_formatting::transform_row_tests::round3_is_js_math_round_at_three_places
developer_tools | world_export_pipeline::json_number_formatting::transform_row_tests::trivial_trailers_write_five_wide
developer_tools | world_export_pipeline::reclassify::tests::appending_a_rule_reports_drift_and_a_new_census_bucket
developer_tools | world_export_pipeline::reclassify::tests::empty_catalogue_is_refused
developer_tools | world_export_pipeline::reclassify::tests::empty_rules_are_refused_not_reported_as_massive_drift
developer_tools | world_export_pipeline::reclassify::tests::measured_spatial_is_preserved_and_template_spatial_is_reclaimed
developer_tools | world_export_pipeline::reclassify::tests::no_drift_when_rules_match_the_catalogue
developer_tools | world_export_pipeline::reclassify::tests::rule_lookup_falls_back_when_no_rule_owns_the_pair
developer_tools | world_export_pipeline::refuse_empty_tests::refuse_empty_write_ok_when_nonempty
developer_tools | world_export_pipeline::refuse_empty_tests::refuse_empty_write_reds_on_empty
developer_tools | world_export_pipeline::roads_emit::tests::archive_stays_within_f32_of_the_json_metres
developer_tools | world_export_pipeline::roads_emit::tests::cli_reports_a_missing_terrain_directory
developer_tools | world_export_pipeline::roads_emit::tests::emit_writes_the_manifest_path_and_creates_its_directory
developer_tools | world_export_pipeline::roads_emit::tests::empty_road_json_is_refused
developer_tools | world_export_pipeline::roads_emit::tests::everon_archive_equals_the_json_road_network
developer_tools | world_export_pipeline::roads_emit::tests::store_accepts_both_sources_and_refuses_nothing_at_all
developer_tools | world_export_pipeline::roads_emit::tests::unknown_class_is_refused_at_write_time
developer_tools | world_export_pipeline::topo::tests::a_buffer_shorter_than_the_header_is_an_error
developer_tools | world_export_pipeline::topo::tests::a_record_outside_the_world_stops_the_parse_with_its_offset
developer_tools | world_export_pipeline::topo::tests::a_topo_declaring_zero_sections_is_an_error_not_a_panic
developer_tools | world_export_pipeline::topo::tests::one_section_with_one_record_decodes_end_to_end
developer_tools | world_export_pipeline::vegetation_density::tests::committed_everon_tiles_survive_decode_then_re_emit_byte_for_byte
developer_tools | world_export_pipeline::vegetation_density::tests::corner_partition_identity
developer_tools | world_export_pipeline::vegetation_density::tests::encode_decode_round_trip_and_fixture
developer_tools | world_export_pipeline::vegetation_density::tests::sample_corners_reads_the_corner_of_a_world_position
developer_tools | world_export_pipeline::vegetation_density::tests::seeded_random_corner_partition_identity
developer_tools | world_export_pipeline::vegetation_density::tests::synthetic_tile_emit_decode_round_trip
ticket_engine | cli::shipping::commit_subjects::tests::mine_subjects_live_repo_smoke
ticket_engine | cli::shipping::commit_subjects::tests::shape_predicates
ticket_engine | cli::shipping::commit_subjects::tests::subject_id_boundary_pins
ticket_engine | cli::shipping::commit_subjects::tests::utc_normalization
ticket_engine | cli::tests::command_mutation_tests::add_refuses_invalid_registry_without_write
ticket_engine | cli::tests::command_mutation_tests::advance_slice_refuses_invalid_registry_without_write
ticket_engine | cli::tests::command_mutation_tests::child_ship_end_to_end_typed_path
ticket_engine | cli::tests::command_mutation_tests::mark_ready_refuses_invalid_registry
ticket_engine | cli::tests::command_mutation_tests::remove_refuses_invalid_registry_without_write
ticket_engine | cli::tests::command_mutation_tests::require_check_ok_err_matches_set_status_gate
ticket_engine | cli::tests::command_mutation_tests::set_status_refuses_empty_without_write
ticket_engine | cli::tests::command_mutation_tests::set_status_refuses_invalid_enum_without_write
ticket_engine | cli::tests::command_mutation_tests::set_status_refuses_invalid_registry_without_write
ticket_engine | cli::tests::command_mutation_tests::ship_no_repack_leaves_the_lock_untouched_until_the_next_repack
ticket_engine | cli::tests::command_mutation_tests::ship_regenerates_docs_from_post_state_reload_pin
ticket_engine | cli::tests::command_mutation_tests::stamp_sha_end_to_end_scratch_cycle
ticket_engine | cli::tests::command_mutation_tests::the_batch_waiver_never_swallows_a_missing_lock
ticket_engine | cli::tests::command_mutation_tests::the_stale_lock_left_by_no_repack_is_waived_for_the_next_ship_and_nothing_else_is
ticket_engine | cli::tests::execution_boundaries_tests::batch_calls_executor_in_queue_order
ticket_engine | cli::tests::execution_boundaries_tests::cleanup_resolution_preserves_absolute_base_and_explicit_branch
ticket_engine | cli::tests::execution_boundaries_tests::cleanup_resolution_preserves_defaults_and_performs_no_deletion
ticket_engine | cli::tests::execution_boundaries_tests::dry_run_does_not_call_executor
ticket_engine | cli::tests::execution_boundaries_tests::executor_failure_stops_the_batch
ticket_engine | encoding::tests::encoding_roundtrip_tests::class_and_estimated_values_are_validated
ticket_engine | encoding::tests::encoding_roundtrip_tests::flat_scope_full_depth_roundtrip
ticket_engine | encoding::tests::encoding_roundtrip_tests::idea_rejects_order
ticket_engine | encoding::tests::encoding_roundtrip_tests::malformed_timestamp_is_parse_error_naming_ticket
ticket_engine | encoding::tests::encoding_roundtrip_tests::parse_render_work_queued
ticket_engine | encoding::tests::encoding_roundtrip_tests::program_timestamps_and_v2_fields_roundtrip
ticket_engine | encoding::tests::encoding_roundtrip_tests::surface_requires_component
ticket_engine | encoding::tests::encoding_roundtrip_tests::timestamps_roundtrip_in_canonical_slot
ticket_engine | encoding::tests::encoding_roundtrip_tests::user_story_alias_parses_and_emits_main_goal
ticket_engine | encoding::tests::encoding_roundtrip_tests::v1_nested_scope_refuses
ticket_engine | encoding::tests::encoding_roundtrip_tests::v2_keys_land_in_canonical_slots
ticket_engine | metrics::estimates::tests::estimate_provenance_tests::business_rules_red
ticket_engine | metrics::estimates::tests::estimate_provenance_tests::collect_numstat_live_repo_smoke
ticket_engine | metrics::estimates::tests::estimate_provenance_tests::estimates_schema_red_green
ticket_engine | metrics::estimates::tests::estimate_provenance_tests::excluded_paths_and_numstat_parse
ticket_engine | metrics::estimates::tests::estimate_provenance_tests::factor_constant_is_pinned_in_the_doc
ticket_engine | metrics::estimates::tests::estimate_provenance_tests::median_is_deterministic
ticket_engine | metrics::estimates::tests::estimate_provenance_tests::mutual_exclusion_and_marker_coherence
ticket_engine | metrics::estimates::tests::estimate_provenance_tests::planted_estimate_inside_metrics_reds_the_metrics_walker
ticket_engine | metrics::estimates::tests::estimate_provenance_tests::scratch_generator_cohorts_fallthrough_and_idempotence
ticket_engine | metrics::estimates::tests::estimate_provenance_tests::summarize_by_agent_on_mixed_tree_equals_receipts_only
ticket_engine | metrics::tests::receipt_validation_tests::by_agent_sums_elapsed_and_totals_over_real_files
ticket_engine | metrics::tests::receipt_validation_tests::drifted_reported_total_is_loud
ticket_engine | metrics::tests::receipt_validation_tests::factory_land_without_receipt_refuses_and_bookkeeping_proceeds
ticket_engine | metrics::tests::receipt_validation_tests::finished_before_started_is_red
ticket_engine | metrics::tests::receipt_validation_tests::land_stamp_two_tickets_touches_only_metrics_never_ticket_tomls
ticket_engine | metrics::tests::receipt_validation_tests::malformed_started_missing_id_missing_tokens_and_bad_sum_are_red
ticket_engine | metrics::tests::receipt_validation_tests::missing_usage_fails_closed_never_zero
ticket_engine | metrics::tests::receipt_validation_tests::reasoning_is_a_sibling_never_summed_into_total
ticket_engine | metrics::tests::receipt_validation_tests::recorded_claude_dialect_parses_and_total_is_the_sum
ticket_engine | metrics::tests::receipt_validation_tests::recorded_cursor_dialect_parses_and_total_is_the_sum
ticket_engine | metrics::tests::receipt_validation_tests::stamp_refuses_when_no_receipt_exists
ticket_engine | metrics::tests::receipt_validation_tests::total_sum_invariant_is_enforced
ticket_engine | metrics::tests::receipt_validation_tests::two_runs_in_one_second_yield_two_files
ticket_engine | model::tests::model_contract_tests::classify_work_is_token_boundary_and_ordered
ticket_engine | model::tests::model_contract_tests::domain_as_str_is_snake_case
ticket_engine | model::tests::model_contract_tests::frozen_unmappable_is_49
ticket_engine | model::tests::model_contract_tests::ready_constructor_rejects_empty_goal
ticket_engine | model::tests::model_contract_tests::status_name_roundtrip
ticket_engine | model::tests::model_contract_tests::title_debt_instrument
ticket_engine | ops::tests::hierarchy_and_body_tests::add_child_onto_program_appends_next_free_id
ticket_engine | ops::tests::hierarchy_and_body_tests::add_child_onto_work_refuses_then_promotes
ticket_engine | ops::tests::hierarchy_and_body_tests::add_refuses_wall_summary_pre_write
ticket_engine | ops::tests::hierarchy_and_body_tests::advance_slice_walks_and_refuses
ticket_engine | ops::tests::hierarchy_and_body_tests::duplicate_child_refuses_any_op
ticket_engine | ops::tests::hierarchy_and_body_tests::injected_clock_determinism
ticket_engine | ops::tests::hierarchy_and_body_tests::mirrored_keys_unrepresentable_4a2f3426_pin
ticket_engine | ops::tests::hierarchy_and_body_tests::preexisting_collision_is_not_retro_policed
ticket_engine | ops::tests::hierarchy_and_body_tests::promote_refuses_parented_and_stray_shipped_at
ticket_engine | ops::tests::hierarchy_and_body_tests::quarantined_ticket_is_exempt_from_summary_cap
ticket_engine | ops::tests::hierarchy_and_body_tests::remove_cascade_end_to_end_on_disk
ticket_engine | ops::tests::hierarchy_and_body_tests::remove_double_listed_child_refuses
ticket_engine | ops::tests::hierarchy_and_body_tests::remove_last_child_of_program_refuses
ticket_engine | ops::tests::hierarchy_and_body_tests::remove_program_refuses_then_force_cascades
ticket_engine | ops::tests::hierarchy_and_body_tests::remove_work_scrubs_parent_children
ticket_engine | ops::tests::hierarchy_and_body_tests::reorder_collision_refuses
ticket_engine | ops::tests::hierarchy_and_body_tests::reorder_flips_idea_and_requires_owns
ticket_engine | ops::tests::hierarchy_and_body_tests::reorder_unknown_anchor_message
ticket_engine | ops::tests::status_and_shipping_tests::add_mints_next_parent_id_and_stamps
ticket_engine | ops::tests::status_and_shipping_tests::made_live_component_without_surface_refuses
ticket_engine | ops::tests::status_and_shipping_tests::mark_ready_backfills_and_gates
ticket_engine | ops::tests::status_and_shipping_tests::mark_ready_plan_gate_refuses_and_resolves
ticket_engine | ops::tests::status_and_shipping_tests::mark_ready_refuses_empty_ready_tier_fields
ticket_engine | ops::tests::status_and_shipping_tests::mark_ready_refuses_missing_spec_and_missing_file
ticket_engine | ops::tests::status_and_shipping_tests::mark_ready_without_order_refuses
ticket_engine | ops::tests::status_and_shipping_tests::ops_refuse_malformed_clock
ticket_engine | ops::tests::status_and_shipping_tests::post_image_main_goal_gate_on_changed_live_work
ticket_engine | ops::tests::status_and_shipping_tests::post_image_title_gate_refuses_changed_debt_titles
ticket_engine | ops::tests::status_and_shipping_tests::set_status_cancelled_stamps_completed_at
ticket_engine | ops::tests::status_and_shipping_tests::set_status_idea_with_order_refuses
ticket_engine | ops::tests::status_and_shipping_tests::set_status_ready_without_order_refuses
ticket_engine | ops::tests::status_and_shipping_tests::set_status_refuses_empty_and_invalid
ticket_engine | ops::tests::status_and_shipping_tests::set_status_shipped_keeps_active_and_does_not_stamp
ticket_engine | ops::tests::status_and_shipping_tests::ship_dotted_child_clears_matching_parent_active
ticket_engine | ops::tests::status_and_shipping_tests::ship_leaves_unrelated_parent_active
ticket_engine | ops::tests::status_and_shipping_tests::ship_preserves_shipped_at_and_order
ticket_engine | ops::tests::status_and_shipping_tests::ship_refuses_created_at_less_pre_write
ticket_engine | ops::tests::status_and_shipping_tests::ship_refuses_empty_ready_tier_fields
ticket_engine | ops::tests::status_and_shipping_tests::stamp_sha_writes_noops_and_refuses
ticket_engine | proptest_roundtrip::parse_render_work_queued_roundtrip
ticket_engine | registry::shipping_status::tests::registry_poisons_on_a_ticket_without_an_id_tests::cancelled_counts_as_shipped
ticket_engine | registry::shipping_status::tests::registry_poisons_on_a_ticket_without_an_id_tests::registry_poisons_on_a_ticket_without_an_id
ticket_engine | registry::shipping_status::tests::registry_poisons_on_a_ticket_without_an_id_tests::registry_unreadable_is_not_shipped
ticket_engine | registry::ticket_file_storage::tests::ticket_file_storage_tests::derive_next_id_is_max_plus_one
ticket_engine | registry::ticket_file_storage::tests::ticket_file_storage_tests::frozen_27_matches_live_corpus
ticket_engine | registry::ticket_file_storage::tests::ticket_file_storage_tests::no_ticket_lost_set_equality
ticket_engine | registry::ticket_file_storage::tests::ticket_file_storage_tests::on_disk_keys_are_mapped_or_allowed_new
ticket_engine | registry::ticket_file_storage::tests::ticket_file_storage_tests::perturb_summary_makes_cmp_red
ticket_engine | registry::ticket_file_storage::tests::ticket_file_storage_tests::ticket_file_key_set_matches_consts
ticket_engine | registry::ticket_file_storage::tests::ticket_file_storage_tests::toml_roundtrip_is_byte_identical_to_the_registry_document
ticket_engine | registry::ticket_file_storage::tests::ticket_file_storage_tests::user_story_alias_maps_to_main_goal
ticket_engine | registry::ticket_status_history::tests::dual_read_json_then_toml
ticket_engine | registry::typed_projection::tests::typed_projection_tests::engine_scope_table_projects_to_the_engine_domain
ticket_engine | registry::typed_projection::tests::typed_projection_tests::mutators_never_reach_the_value_writer_pin
ticket_engine | registry::typed_projection::tests::typed_projection_tests::program_children_parse_as_work_and_their_parents_list_them
ticket_engine | registry::typed_projection::tests::typed_projection_tests::ready_class_tickets_carry_spec_main_goal_and_acceptance
ticket_engine | registry::typed_projection::tests::typed_projection_tests::shipped_ticket_keeps_its_shipped_at_commit
ticket_engine | registry::typed_projection::tests::typed_projection_tests::targets_from_scope_v2_outputs
ticket_engine | registry::typed_projection::tests::typed_projection_tests::value_to_ticket_accepts_ticket_to_value_output
ticket_engine | repository::tests::a_handoff_document_lands_in_the_artifact_tree
ticket_engine | repository::tests::a_plan_path_is_the_lowercased_id_under_the_plans_directory
ticket_engine | repository::tests::every_ticket_target_has_a_sparse_checkout_set
ticket_engine | repository::tests::the_root_sparse_set_carries_the_task_surface
ticket_engine | store::tests::corpus_storage_tests::corpus_roundtrip_real_tree_byte_identical
ticket_engine | store::tests::corpus_storage_tests::delete_files_refuses_live_ids
ticket_engine | store::tests::corpus_storage_tests::derive_next_parent_id_ignores_children
ticket_engine | store::tests::corpus_storage_tests::load_refuses_id_filename_mismatch
ticket_engine | store::tests::corpus_storage_tests::load_refuses_naming_broken_file
ticket_engine | store::tests::corpus_storage_tests::load_refuses_vocab_illegal_scope_and_missing_vocab
ticket_engine | store::tests::corpus_storage_tests::main_goal_debt_ratchet_pin
ticket_engine | store::tests::corpus_storage_tests::migration_legacy_ratchet_pin
ticket_engine | store::tests::corpus_storage_tests::next_child_id_direct_extensions_only
ticket_engine | store::tests::corpus_storage_tests::title_debt_ratchet_pin
ticket_engine | store::tests::corpus_storage_tests::write_back_is_surgical_and_clean
ticket_engine | sync::tests::empty_write_tests::inject_next_refuses_empty_tickets_bare_heading
ticket_engine | sync::tests::empty_write_tests::marker_inner_vacuous_detects_bare_heading
ticket_engine | sync::tests::empty_write_tests::refuse_empty_write_ok_when_nonempty
ticket_engine | sync::tests::empty_write_tests::refuse_empty_write_reds_on_empty
ticket_engine | timestamp::tests::utc_timestamp_tests::accepts_canonical_utc
ticket_engine | timestamp::tests::utc_timestamp_tests::now_is_canonical_and_validates
ticket_engine | timestamp::tests::utc_timestamp_tests::rejects_malformed_and_non_utc
ticket_engine | validation::tests::readiness_and_accounting_tests::children_integrity_red_green
ticket_engine | validation::tests::readiness_and_accounting_tests::debt_pin_growth_verdict
ticket_engine | validation::tests::readiness_and_accounting_tests::honesty_counters_fixture_math
ticket_engine | validation::tests::readiness_and_accounting_tests::malformed_timestamp_is_red
ticket_engine | validation::tests::readiness_and_accounting_tests::ready_tier_body_red_green_and_quarantine_exempt
ticket_engine | validation::tests::readiness_and_accounting_tests::require_check_ok_blocks_invalid_registry
ticket_engine | validation::tests::schema_and_integrity_tests::body_caps_red_green_and_warning_channel
ticket_engine | validation::tests::schema_and_integrity_tests::estimated_marker_without_field_is_red
ticket_engine | validation::tests::schema_and_integrity_tests::live_work_component_without_surface_is_red
ticket_engine | validation::tests::schema_and_integrity_tests::open_work_without_owns_is_red
ticket_engine | validation::tests::schema_and_integrity_tests::perturbed_schema_rejects_tip_registry
ticket_engine | validation::tests::schema_and_integrity_tests::perturbed_ticket_field_fails_schema
ticket_engine | validation::tests::schema_and_integrity_tests::plan_ready_gate_red_green
ticket_engine | validation::tests::schema_and_integrity_tests::quarantine_mint_past_cutover_is_red
ticket_engine | validation::tests::schema_and_integrity_tests::ship_gate_red_green_per_arm
ticket_engine | validation::tests::schema_and_integrity_tests::tip_registry_full_check_ok
ticket_engine | validation::tests::schema_and_integrity_tests::tip_registry_passes_schema
ticket_engine | validation::tests::schema_and_integrity_tests::work_title_nonempty_red_green
ticket_engine | validation::tests::schema_and_integrity_tests::work_without_class_is_red
ticket_engine | validation::vocabulary::tests::vocabulary_shape_tests::counted_shape_from_live_file
ticket_engine | validation::vocabulary::tests::vocabulary_shape_tests::duplicate_component_key_is_parse_red
ticket_engine | validation::vocabulary::tests::vocabulary_shape_tests::duplicate_surface_red_names_file_parent_value
ticket_engine | validation::vocabulary::tests::vocabulary_shape_tests::empty_values_are_red
ticket_engine | validation::vocabulary::tests::vocabulary_shape_tests::live_vocab_file_is_green
ticket_engine | validation::vocabulary::tests::vocabulary_shape_tests::missing_file_is_red_naming_path
ticket_engine | validation::vocabulary::tests::vocabulary_shape_tests::unknown_domain_is_red
ticket_engine | validation::vocabulary::tests::vocabulary_shape_tests::unsorted_keys_are_red_per_level
ticket_engine | validation::vocabulary::tests::vocabulary_shape_tests::wrong_value_shapes_are_red
ticket_engine | vocab::tests::vocabulary_resolution_tests::legality_walks_the_tree
ticket_engine | vocab::tests::vocabulary_resolution_tests::missing_file_refuses_naming_path
ticket_engine | wave_lock::archived_wave_plans::tests::archived_wave_plan_parsing_tests::parse_rows_drops_comments_header_blanks_and_short_lines
ticket_engine | wave_lock::collisions::tests::collision_source_tests::facts_come_from_ticket_files
ticket_engine | wave_lock::collisions::tests::collision_source_tests::hardcoded_dep_tables_stay_deleted
ticket_engine | wave_lock::tests::emptied_wave_tests::check_reds_on_a_perturbed_emptied_entry_until_restored
ticket_engine | wave_lock::tests::emptied_wave_tests::lock_without_an_emptied_section_parses_and_renders_without_one
ticket_engine | wave_lock::tests::emptied_wave_tests::two_emptied_waves_pend_ascending_and_open_waves_number_past_both
ticket_engine | wave_lock::tests::packing_and_history_tests::a_disavowed_marker_is_not_a_base
ticket_engine | wave_lock::tests::packing_and_history_tests::a_wave_dissolved_id_by_id_leaves_no_entry_and_its_label_is_reissued
ticket_engine | wave_lock::tests::packing_and_history_tests::a_wave_freezes_its_whole_set_only_when_repacked_after_the_last_ship
ticket_engine | wave_lock::tests::packing_and_history_tests::an_incidental_repack_keeps_the_locks_own_width
ticket_engine | wave_lock::tests::packing_and_history_tests::candidates_sort_by_order_then_id_never_glob_order
ticket_engine | wave_lock::tests::packing_and_history_tests::check_reds_on_perturbed_owns_and_edges_and_stale_membership
ticket_engine | wave_lock::tests::packing_and_history_tests::compile_render_is_deterministic_and_roundtrips
ticket_engine | wave_lock::tests::packing_and_history_tests::dependent_packs_strictly_after_unshipped_dependency
ticket_engine | wave_lock::tests::packing_and_history_tests::full_ship_freezes_the_wave_into_emptied_and_open_waves_number_past_it
ticket_engine | wave_lock::tests::packing_and_history_tests::head_itself_as_the_marker_counts
ticket_engine | wave_lock::tests::packing_and_history_tests::lock_without_a_wave_base_parses_as_zero
ticket_engine | wave_lock::tests::packing_and_history_tests::missing_lock_is_a_did_not_run_refusal
ticket_engine | wave_lock::tests::packing_and_history_tests::no_marker_tree_keeps_base_zero_and_waves_from_one
ticket_engine | wave_lock::tests::packing_and_history_tests::numbering_seats_on_the_highest_claim_not_the_newest_marker
ticket_engine | wave_lock::tests::packing_and_history_tests::overlapping_owns_never_share_a_wave
ticket_engine | wave_lock::tests::packing_and_history_tests::partial_ship_records_no_emptied_entry
ticket_engine | wave_lock::tests::packing_and_history_tests::reorder_changes_open_waves_only_never_wave_zero
ticket_engine | wave_lock::tests::packing_and_history_tests::repack_continues_the_close_marker_ledger
ticket_engine | wave_lock::tests::packing_and_history_tests::reserving_freezes_the_lost_set_and_the_open_wave_numbers_past_it
ticket_engine | wave_lock::tests::packing_and_history_tests::reserving_refuses_a_live_ticket_and_an_unknown_id
ticket_engine | wave_lock::tests::packing_and_history_tests::shallow_clone_refuses_base_derivation
ticket_engine | wave_lock::tests::packing_and_history_tests::stale_base_reds_check_until_repack
ticket_engine | wave_lock::tests::packing_and_history_tests::wave_zero_is_baseline_union_parked_minus_reopened
ticketboard | app::tests::back_collapses_the_viewer_column_only
ticketboard | app::tests::right_pane_gate_is_total_and_degrades_honestly
ticketboard | app::tests::viewer_width_persistence_model
ticketboard | board::tests::breadcrumb_assembly_variants
ticketboard | board::tests::breadcrumb_estimated_scope_marker
ticketboard | board::tests::bucketing_headers_and_chips
ticketboard | board::tests::cards_precompute_breadcrumb_and_class
ticketboard | board::tests::cards_precompute_main_goal_tooltip
ticketboard | board::tests::cards_sort_by_order_then_numeric_id
ticketboard | board::tests::child_ids_sort_numerically_within_ties
ticketboard | board::tests::class_parity_and_total_distinct_accents
ticketboard | board::tests::executor_chip_defaults_to_claude_code
ticketboard | board::tests::id_to_index_resolves_every_ticket
ticketboard | board::tests::ideas_sort_by_numeric_id_not_string
ticketboard | board::tests::order_label_precomputed
ticketboard | board::tests::status_label_forms
ticketboard | board::tests::status_order_matches_column_of
ticketboard | board::tests::truncate_chars_is_char_safe
ticketboard | board::tests::unparsable_ids_sort_last
ticketboard | board::tests::view_maps_both_kinds
ticketboard | corpus::tests::child_id_classification
ticketboard | corpus::tests::counts_match_the_scratch_corpus
ticketboard | corpus::tests::fail_closed_names_the_bad_file
ticketboard | corpus::tests::fail_closed_on_semantic_error_too
ticketboard | corpus::tests::live_corpus_loads_and_counts_sum
ticketboard | corpus::tests::missing_tickets_dir_refuses_with_the_path
ticketboard | detail::tests::absent_and_present_content_model
ticketboard | detail::tests::definitions_are_distinct_anti_blend_one_liners
ticketboard | detail::tests::header_model_is_pinned
ticketboard | detail::tests::legacy_collapse_threshold
ticketboard | detail::tests::numbered_lines_model
ticketboard | detail::tests::section_order_is_pinned_and_quarantine_is_last
ticketboard | detail::tests::triage_block_template_content
ticketboard | discovery::tests::ancestor_found_from_nested_cwd
ticketboard | discovery::tests::arg_wins_even_when_invalid
ticketboard | discovery::tests::arg_wins_over_ancestor
ticketboard | discovery::tests::nothing_found_is_none
ticketboard | discovery::tests::positional_arg_skips_flags
ticketboard | estimates::tests::absent_but_marked_shipped_at_renders_estimated_absent
ticketboard | estimates::tests::absent_estimates_dir_is_the_explicit_no_estimates_state
ticketboard | estimates::tests::checker_mirror_rules_each_produce_a_named_error_row
ticketboard | estimates::tests::empty_estimates_dir_is_no_estimates
ticketboard | estimates::tests::estimate_glyph_matches_the_scope_glyph
ticketboard | estimates::tests::hand_computed_sums_feature_62_500_bug_1_500_website_61_500_repo_2_500
ticketboard | estimates::tests::live_estimates_load_without_error_rows
ticketboard | estimates::tests::malformed_estimate_is_a_named_error_row_excluded_from_sums
ticketboard | estimates::tests::missing_ticket_classless_and_program_buckets_are_explicit
ticketboard | estimates::tests::sort_rows_by_each_key_with_direction_toggle_and_stable_tiebreak
ticketboard | estimates::tests::stamp_cell_glyph_predicate_and_verbatim_note_tip
ticketboard | estimates::tests::the_law_no_code_path_combines_measured_and_estimated
ticketboard | estimates::tests::tokens_cell_states
ticketboard | estimates::tests::tokens_detail_row_model_both_sources
ticketboard | estimates::tests::zero_valid_files_says_so_never_a_zeros_panel
ticketboard | facets::tests::broken_or_missing_vocab_is_none_never_a_crash
ticketboard | facets::tests::corpus_strays_are_offered_and_marked
ticketboard | facets::tests::missing_vocab_falls_back_to_corpus_values
ticketboard | facets::tests::stale_lower_selections_are_cleared_top_down
ticketboard | facets::tests::vocab_narrowing_walks_down_the_tree
ticketboard | filters::tests::class_filter_composes_and_clear_restores_everything
ticketboard | filters::tests::clear_restores_the_full_measured_count
ticketboard | filters::tests::facet_only_filters_are_active
ticketboard | filters::tests::filters_compose_as_intersection
ticketboard | filters::tests::index_precomputes_lowercase_haystacks_and_executors
ticketboard | filters::tests::index_precomputes_scope_and_class_facts
ticketboard | filters::tests::kind_filter_splits_work_and_program
ticketboard | filters::tests::no_status_toggle_means_all_statuses
ticketboard | filters::tests::parent_filter_matches_the_subtree_only
ticketboard | filters::tests::scope_facets_compose_with_existing_filters
ticketboard | gitstatus::tests::chip_states_from_exit
ticketboard | gitstatus::tests::porcelain_parse_counts_and_lists_entries_only
ticketboard | gitstatus::tests::shape_filter_edges
ticketboard | metrics::tests::absent_metrics_dir_is_the_explicit_no_receipts_state
ticketboard | metrics::tests::bad_sum_receipt_is_a_named_error_row_excluded_from_sums
ticketboard | metrics::tests::checker_mirror_rules_each_produce_a_named_error_row
ticketboard | metrics::tests::empty_metrics_dir_is_no_receipts_even_with_empty_id_subdirs
ticketboard | metrics::tests::format_elapsed_h_m_s
ticketboard | metrics::tests::format_tokens_groups_thousands
ticketboard | metrics::tests::hand_computed_sums_agent_a_150tok_180s_agent_b_20tok_30s
ticketboard | metrics::tests::min_max_use_parsed_instants_not_string_order
ticketboard | metrics::tests::missing_tokens_consumed_is_a_named_error_row_never_zero
ticketboard | metrics::tests::reasoning_is_excluded_from_the_total_check
ticketboard | metrics::tests::sort_rows_by_each_key_with_direction_toggle_and_stable_tiebreak
ticketboard | metrics::tests::stray_file_at_the_metrics_root_is_an_error_row
ticketboard | metrics::tests::ticket_id_and_sha_pattern_mirrors
ticketboard | metrics::tests::unfinished_run_counts_in_runs_and_unfinished_never_in_elapsed
ticketboard | metrics::tests::unknown_field_is_an_error_row_mirroring_the_schema
ticketboard | subproc::tests::bare_gui_path_falls_back_to_home_cargo_bin
ticketboard | subproc::tests::cargo_env_wins_when_it_exists
ticketboard | subproc::tests::empty_cargo_env_falls_through_to_path
ticketboard | subproc::tests::kill_delivers_a_signal_exit
ticketboard | subproc::tests::log_ring_keeps_the_last_cap_lines_and_counts_drops
ticketboard | subproc::tests::nothing_found_yields_literal_cargo
ticketboard | subproc::tests::path_scan_takes_the_first_hit_in_order
ticketboard | subproc::tests::spawn_failure_is_an_event_not_a_panic
ticketboard | subproc::tests::stale_cargo_env_falls_through_to_path
ticketboard | subproc::tests::streams_merged_lines_and_delivers_the_exit_code
ticketboard | tree::tests::explicit_parent_beats_dotted_prefix
ticketboard | tree::tests::flatten_collapsed_shows_roots_only
ticketboard | tree::tests::flatten_filter_force_expands_and_dims_ancestors
ticketboard | tree::tests::flatten_manual_expansion_descends
ticketboard | tree::tests::nests_program_child_of_program
ticketboard | tree::tests::orphan_dotted_child_roots
ticketboard | tree::tests::parent_cycle_rescued_as_root
ticketboard | tree::tests::siblings_sort_by_order_then_numeric_id
ticketboard | trust::tests::banner_states_walk_idle_building_checking_green
ticketboard | trust::tests::check_command_matches_its_expanded_argv
ticketboard | trust::tests::coalescer_burst_yields_one_run_and_one_followup
ticketboard | trust::tests::error_lines_counted_from_a_red_fixture_stream
ticketboard | trust::tests::killed_and_spawn_failed_are_red_and_honest
ticketboard | trust::tests::phase_split_fallback_on_check_output_without_cargo_lines
ticketboard | trust::tests::phase_split_on_a_full_build_stream
ticketboard | trust::tests::red_without_error_lines_points_at_the_output
ticketboard | trust::tests::utc_hms_formats_and_wraps_days
ticketboard | verbs::tests::add_builder_summary_flag_and_display_quoting
ticketboard | verbs::tests::add_child_flag_combos
ticketboard | verbs::tests::advanced_matrix_per_kind_and_status
ticketboard | verbs::tests::cas_guard_passes_unchanged_and_refuses_changed_or_deleted
ticketboard | verbs::tests::confirm_requests_map_to_exactly_one_verb_line
ticketboard | verbs::tests::descendants_closure_by_dotted_prefix
ticketboard | verbs::tests::display_quoting_escapes_embedded_single_quotes
ticketboard | verbs::tests::hash_is_content_sensitive
ticketboard | verbs::tests::mark_ready_with_and_without_spec
ticketboard | verbs::tests::offered_transitions_matrix_pinned
ticketboard | verbs::tests::queue_failure_drops_the_pending_tail_and_never_retries
ticketboard | verbs::tests::queue_is_single_flight_fifo
ticketboard | verbs::tests::queued_requests_keep_their_captured_fingerprints
ticketboard | verbs::tests::recovery_hint_applies_on_signal_kill_even_with_a_silent_log
ticketboard | verbs::tests::recovery_hint_triggers_on_the_wave_stale_signature_only
ticketboard | verbs::tests::remove_force_combo
ticketboard | verbs::tests::remove_type_to_confirm_gate_requires_the_exact_id
ticketboard | verbs::tests::reorder_and_advance_slice_builders
ticketboard | verbs::tests::running_is_never_a_dispatch_target_in_the_normal_ui
ticketboard | verbs::tests::set_status_builder_covers_the_whole_enum
ticketboard | verbs::tests::ship_builder_argv_and_display
ticketboard | verbs::tests::success_tail_skips_cargo_noise_and_blank_lines
ticketboard | viewer::tests::classify_non_utf8_goes_lossy
ticketboard | viewer::tests::classify_oversize_cut_is_char_safe
ticketboard | viewer::tests::classify_oversize_truncates_with_notice
ticketboard | viewer::tests::classify_within_cap_renders
ticketboard | viewer::tests::classify_within_cap_tail_corruption_is_lossy
ticketboard | viewer::tests::load_doc_capped_truncates_on_disk_file
ticketboard | viewer::tests::load_doc_missing_file_names_the_error
ticketboard | viewer::tests::load_doc_reads_markdown_inside_root
ticketboard | viewer::tests::load_doc_refuses_escape_without_reading
ticketboard | viewer::tests::load_doc_refuses_symlink_escape
ticketboard | viewer::tests::md_click_predicate_over_spec_plan_citations
ticketboard | viewer::tests::resolve_happy_paths
ticketboard | viewer::tests::resolve_refuses_escapes
ticketboard | viewer::tests::stale_results_are_dropped
ticketboard | viewer::tests::state_machine_transitions
ticketboard | viewer::tests::wants_viewer_predicate
ticketboard | watch::tests::burst_coalesces_to_one_fire
ticketboard | watch::tests::mixed_burst_across_the_trailing_edge_keeps_its_check
ticketboard | watch::tests::new_event_extends_the_quiet_window
ticketboard | watch::tests::relevance_filter_matches_the_watched_surfaces_only
ticketboard | watch::tests::resuppression_cancels_the_trailing_window
ticketboard | watch::tests::suppressed_fires_reload_only
ticketboard | watch::tests::suppression_at_fire_time_beats_a_checkworthy_event
ticketboard | watch::tests::ticket_doc_names
ticketboard | watch::tests::trailing_window_after_clear_stays_check_silent
ticketboard | wavelock::tests::collides_mirror_cases
ticketboard | wavelock::tests::colliding_pairs_lists_every_pair
ticketboard | wavelock::tests::live_lock_parses_verbatim
ticketboard | wavelock::tests::missing_lock_is_the_did_not_run_refusal
ticketboard | wavelock::tests::missing_required_field_refuses
ticketboard | wavelock::tests::parse_fixture_including_wave_base_and_pack_last
ticketboard | wavelock::tests::unparsable_lock_refuses_with_verbatim_error
ticketboard | wavelock::tests::wave_base_defaults_to_zero_for_pre_t914_locks
ticketboard | waves::tests::dispatchable_mirrors_the_xtask_rule
ticketboard | waves::tests::lane_tsv_format_is_n_tab_id_lines
ticketboard | waves::tests::lanes_render_the_lock_verbatim_never_sorted
ticketboard | waves::tests::lock_id_without_ticket_file_is_flagged
ticketboard | waves::tests::unplanned_is_pure_set_arithmetic
trybuild | compile_fail_mod_has_no_frontend_layer
verification_core (doc) | tools_v2/verification-core/src/lib.rs - (line 35)
verification_core | gate::tests::a_directory_target_is_missing_not_unreadable
verification_core | gate::tests::ban_fails_when_present
verification_core | gate::tests::ban_holds_when_absent
verification_core | gate::tests::missing_target_is_did_not_run_not_held
verification_core | gate::tests::missing_target_on_require_is_also_did_not_run
verification_core | gate::tests::one_missing_among_many_fails_the_whole_check
verification_core | gate::tests::patterns_cannot_match_across_a_file_boundary
verification_core | gate::tests::probe_propagates_did_not_run_instead_of_short_circuiting_clean
verification_core | gate::tests::probe_reports_both_true_and_false_when_it_ran
verification_core | gate::tests::require_fails_when_absent
verification_core | gate::tests::require_holds_when_present
verification_core | gate::tests::str_helpers_need_no_files
verification_core | lock::tests::a_second_holder_is_refused_not_granted
verification_core | lock::tests::acquires_when_free
verification_core | lock::tests::dropping_releases_for_the_next_holder
verification_core | lock::tests::exhaustion_is_did_not_run_never_a_pass
verification_core | lock::tests::heartbeat_fires_while_blocked
verification_core | lock::tests::interops_with_the_flock_command
verification_core | pattern::tests::caret_is_a_line_anchor
verification_core | pattern::tests::case_insensitive_folds
verification_core | pattern::tests::dollar_is_a_line_anchor
verification_core | pattern::tests::dot_does_not_cross_a_newline
verification_core | pattern::tests::invalid_regex_is_an_error_not_a_panic
verification_core | pattern::tests::literal_escapes_metacharacters
verification_core | pattern::tests::posix_classes_work_as_in_an_extended_regex
verification_core | pattern::tests::source_survives_escaping_for_diagnostics
verification_core | proc::tests::absent_tool_is_tool_absent_not_a_failure
verification_core | proc::tests::captures_stderr_separately
verification_core | proc::tests::captures_stdout_and_raw_code
verification_core | proc::tests::env_and_cwd_apply
verification_core | proc::tests::expect_code_wants_exactly_that_code
verification_core | proc::tests::expect_ok_maps_absent_tool_to_did_not_run
verification_core | proc::tests::large_output_does_not_deadlock
verification_core | proc::tests::merged_output_honours_cwd_env_and_stdin
verification_core | proc::tests::merged_output_keeps_the_raw_exit_code
verification_core | proc::tests::merged_output_preserves_interleaving
verification_core | proc::tests::merged_output_reports_absent_tools_and_signals_honestly
verification_core | proc::tests::merged_output_times_out_without_deadlocking_on_a_full_pipe
verification_core | proc::tests::retry_does_not_retry_an_absent_tool
verification_core | proc::tests::retry_gives_up_and_returns_the_last_error
verification_core | proc::tests::retry_succeeds_on_a_later_attempt
verification_core | proc::tests::signal_death_is_signalled_not_an_exit_code
verification_core | proc::tests::stdin_is_delivered
verification_core | proc::tests::timeout_kills_the_whole_process_group
verification_core | proc::tests::timeout_reports_timeout
verification_core | proc::tests::wait_for_returns_when_the_condition_holds
verification_core | proc::tests::wait_for_times_out_rather_than_reporting_success
verification_core | proc::tests::which_finds_and_misses
verification_core | report::tests::a_did_not_run_outranks_violations
verification_core | report::tests::all_held_is_clean_and_zero
verification_core | report::tests::an_empty_report_is_clean
verification_core | report::tests::did_not_run_alone_still_exits_two
verification_core | report::tests::violations_exit_one
verification_core | scan::tests::a_file_root_is_accepted_directly
verification_core | scan::tests::a_missing_root_is_did_not_run_not_zero_hits
verification_core | scan::tests::extension_filter_applies
verification_core | scan::tests::grep_lines_finds_every_occurrence
verification_core | scan::tests::grep_lines_on_a_missing_file_is_did_not_run
verification_core | scan::tests::grep_lines_reports_one_based_line_numbers
verification_core | scan::tests::non_utf8_bytes_do_not_abort_the_scan
verification_core | scan::tests::walks_recursively_and_deterministically
verification_core | verdict::tests::a_missing_target_names_the_file_and_the_six_space_continuation
verification_core | verdict::tests::ban_and_pin_differ_only_in_the_noun
verification_core | verdict::tests::exit_codes_separate_did_not_run_from_failed
verification_core | verdict::tests::held_renders_nothing
verification_core | verdict::tests::renders_a_bare_failure_as_one_headline
verification_core | verdict::tests::signal_death_is_did_not_run_never_failed
verification_core | verdict::tests::the_binary_exit_code_collapses_both_failure_kinds_to_one
xtask | commands::agent_context::guards::tests::bare_file_read_is_denied
xtask | commands::agent_context::guards::tests::capped_search_is_allowed
xtask | commands::agent_context::guards::tests::git_is_never_touched
xtask | commands::agent_context::guards::tests::grep_reading_from_a_pipe_is_allowed
xtask | commands::agent_context::guards::tests::head_as_a_cap_is_allowed
xtask | commands::agent_context::guards::tests::hook_registered_twice_does_not_deny_a_first_read
xtask | commands::agent_context::guards::tests::large_whole_file_read_is_denied_but_ranged_is_not
xtask | commands::agent_context::guards::tests::missing_file_fails_open
xtask | commands::agent_context::guards::tests::only_chatter_is_treated_as_noise
xtask | commands::agent_context::guards::tests::ranged_read_is_always_allowed_even_when_repeated
xtask | commands::agent_context::guards::tests::uncapped_grep_is_denied
xtask | commands::agent_context::guards::tests::verdict_and_failure_lines_always_survive
xtask | commands::agent_context::guards::tests::whole_read_of_a_path_read_in_an_earlier_turn_is_denied
xtask | commands::ci::task_runner::tests::a_failing_leaf_fails_the_composite
xtask | commands::ci::task_runner::tests::ci_local_runs_the_leaves_not_a_copy_of_them
xtask | commands::ci::task_runner::tests::ci_local_step_set_is_frozen
xtask | commands::ci::task_runner::tests::cmd_lines_are_shell_free
xtask | commands::ci::task_runner::tests::doc_layout_predicate_reproduces_finds_globs
xtask | commands::ci::task_runner::tests::every_composite_step_resolves
xtask | commands::ci::task_runner::tests::help_lists_every_task
xtask | commands::ci::task_runner::tests::list_gates_equals_the_wave_gate_constant
xtask | commands::db::milestone_announcement::tests::bad_database_url_forwards_psql_rc
xtask | commands::db::milestone_announcement::tests::missing_env_continues_then_no_psql
xtask | commands::db::milestone_announcement::tests::no_psql_no_container_exits_1
xtask | commands::db::milestone_announcement::tests::sql_matches_former_heredoc_len
xtask | commands::db::milestone_announcement::tests::the_api_directory_resolves_against_the_given_root
xtask | commands::db::operations::ab::tests::make_error_lines_are_told_from_recipe_output
xtask | commands::db::operations::ab::tests::norm_only_erases_ids
xtask | commands::db::operations::recipes::tests::expand_covers_web_and_compose_and_nothing_else
xtask | commands::db::operations::recipes::tests::recipe_body_stops_at_the_next_target
xtask | commands::db::operations::recipes::tests::recipe_body_strips_make_prefixes_and_comments
xtask | commands::db::operations::recipes::tests::seed_recipe_keeps_all_five_appliers_in_order
xtask | commands::db::operations::repair_migration_checksum::tests::a_changed_statement_is_never_comments_only
xtask | commands::db::operations::repair_migration_checksum::tests::a_double_dash_inside_a_string_literal_is_not_a_comment
xtask | commands::db::operations::repair_migration_checksum::tests::a_rewritten_comment_block_is_comments_only
xtask | commands::db::operations::repair_migration_checksum::tests::an_added_statement_is_ddl_changed
xtask | commands::db::operations::repair_migration_checksum::tests::identical_bytes_need_no_repair
xtask | commands::db::operations::repair_migration_checksum::tests::normalization_drops_comments_and_blank_lines_only
xtask | commands::db::operations::repair_migration_checksum::tests::the_diff_shows_both_sides_of_the_comment_change
xtask | commands::db::operations::repair_migration_checksum::tests::version_comes_from_the_filename_prefix
xtask | commands::db::operations::selftest::tests::baseline_covers_every_rendered_target
xtask | commands::db::operations::selftest::tests::frozen_baseline_matches_the_port
xtask | commands::db::operations::test_it::tests::a_failing_suite_still_reports_its_own_rc
xtask | commands::db::operations::test_it::tests::reap_select_is_the_makefile_pattern
xtask | commands::db::operations::test_it::tests::the_guard_refuses_the_live_database
xtask | commands::db::operations::tests::lane_commands_match_the_clap_enum
xtask | commands::debug::direct_join::tests::arm_logdir_exits_err
xtask | commands::debug::direct_join::tests::arm_nocursor_exits_err
xtask | commands::debug::direct_join::tests::clean_empty_home_writes_unknown_and_missing
xtask | commands::debug::direct_join::tests::steam_three_field_buildid_uses_awk_dollar3
xtask | commands::debug::direct_join::tests::steam_two_field_buildid_is_empty_not_unknown
xtask | commands::debug::remote_logs::tests::errors_present_fail
xtask | commands::debug::remote_logs::tests::healthy_is_partial
xtask | commands::debug::remote_logs::tests::missing_file_is_environment
xtask | commands::debug::remote_logs::tests::selftest_pass
xtask | commands::debug::remote_logs::tests::stale_fails
xtask | commands::deploy::database_backup::tests::parse_nonneg_accepts_digits
xtask | commands::deploy::database_backup::tests::parse_nonneg_uses_argument
xtask | commands::deploy::database_backup::tests::retention_sort_is_reverse_lexicographic
xtask | commands::deploy::database_operations::tests::count_copy_rows_counts_data_not_headings
xtask | commands::deploy::database_operations::tests::database_name_from_url_parses_ascii_path
xtask | commands::deploy::database_operations::tests::safe_scratch_allow_list_admits_scratch_names_and_refuses_the_live_database
xtask | commands::deploy::database_restore_drill::tests::mig_ver_strips_leading_zeros
xtask | commands::deploy::staging::agent::tests::agent_script_is_the_quoted_heredoc_verbatim
xtask | commands::deploy::staging::agent::tests::name_validation_fails_closed_on_injection
xtask | commands::deploy::staging::agent::tests::units_render_byte_for_byte
xtask | commands::deploy::staging::agent::tests::validate_rejects_a_tampered_agent
xtask | commands::deploy::staging::agent_selftest::tests::capture_reproduces_the_greedy_sed
xtask | commands::deploy::staging::agent_selftest::tests::contract_key_check_is_exact
xtask | commands::deploy::staging::agent_selftest::tests::stub_systemctl_can_express_a_zero_exit_over_a_dead_unit
xtask | commands::deploy::staging::boot::tests::addon_check_discriminates_on_path_not_guid
xtask | commands::deploy::staging::boot::tests::grep_after_resets_the_window_on_each_match
xtask | commands::deploy::staging::boot::tests::guid_reads_out_of_the_real_gproj
xtask | commands::deploy::staging::boot::tests::last_loaded_addons_block_wins
xtask | commands::deploy::staging::boot::tests::missing_log_is_not_a_pass
xtask | commands::deploy::staging::boot::tests::non_numeric_admin_count_takes_the_else_branch
xtask | commands::deploy::staging::boot::tests::rival_precedence_matches_the_bash_if_chain
xtask | commands::deploy::staging::config::tests::admin_id_schema_is_the_engines
xtask | commands::deploy::staging::config::tests::deploy_env_file_beats_the_process_environment
xtask | commands::deploy::staging::config::tests::dirname_matches_coreutils
xtask | commands::deploy::staging::config::tests::mod_source_label_names_the_actual_source
xtask | commands::deploy::staging::config::tests::mode_gate_matches_the_bash_case
xtask | commands::deploy::staging::config::tests::prairielearn_is_refused_anywhere_in_the_path
xtask | commands::deploy::staging::config::tests::source_no_longer_executes_the_env_file
xtask | commands::deploy::staging::config::tests::xargs_like_trims_and_collapses
xtask | commands::deploy::staging::payloads::tests::agent_install_payload_rereads_the_socket_state
xtask | commands::deploy::staging::payloads::tests::profile_payload_leaves_cfg_for_the_remote_shell
xtask | commands::deploy::staging::payloads::tests::smoke_payload_asserts_all_three_status_codes
xtask | commands::deploy::staging::payloads::tests::unit_payload_nests_a_quoted_heredoc
xtask | commands::deploy::staging::pycompat::tests::ensure_ascii_matches_json_dumps_default
xtask | commands::deploy::staging::pycompat::tests::json_repr_of_a_scalar
xtask | commands::deploy::staging::pycompat::tests::py_json_error_reproduces_the_measured_python_messages
xtask | commands::deploy::staging::pycompat::tests::py_repr_follows_pythons_quote_choice
xtask | commands::deploy::staging::pycompat::tests::py_str_or_empty_follows_pythons_truthiness
xtask | commands::deploy::staging::pycompat::tests::py_type_names_match
xtask | commands::deploy::staging::remote::tests::dry_run_never_spawns
xtask | commands::deploy::staging::remote::tests::exec_start_addons_mode_quotes_the_scenario
xtask | commands::deploy::staging::remote::tests::exec_start_config_mode_carries_both_flags
xtask | commands::deploy::staging::remote::tests::not_run_never_reads_as_success
xtask | commands::deploy::staging::remote::tests::rsync_argv_keeps_every_exclude_in_order
xtask | commands::deploy::staging::remote::tests::ssh_argv_plain_identity_and_sshpass
xtask | commands::deploy::staging::remote::tests::sshpass_wins_over_identity_file
xtask | commands::deploy::staging::remote::tests::v6_maps_all_four_outcomes_and_refuses_to_guess
xtask | commands::deploy::staging::render::tests::curl_argv_is_stable
xtask | commands::deploy::staging::render::tests::every_fail_closed_branch_fires
xtask | commands::deploy::staging::render::tests::legacy_mods_render_matches_the_captured_bytes
xtask | commands::deploy::staging::render::tests::modpack_url_without_a_token_fails_before_any_network_call
xtask | commands::deploy::staging::render::tests::raw_substitution_can_emit_non_json_and_the_validator_catches_it
xtask | commands::deploy::staging::render::tests::render_only_refuses_addons_mode
xtask | commands::deploy::staging::render::tests::validator_catches_the_truncated_scenario_and_the_port_clash
xtask | commands::deploy::staging::render::tests::version_is_emitted_only_when_non_empty_and_key_order_is_pinned
xtask | commands::deploy::staging::tests::empty_string_value_is_rejected_like_bash
xtask | commands::deploy::staging::tests::flags_accumulate
xtask | commands::deploy::staging::tests::missing_value_stops_with_two
xtask | commands::deploy::staging::tests::oddity_flag_is_eaten_as_a_value
xtask | commands::deploy::staging::tests::paths_resolve_against_the_running_checkout
xtask | commands::deploy::staging::tests::unknown_option_short_circuits_before_help
xtask | commands::deploy::staging::tests::usage_names_the_runnable_command_and_every_mode_flag
xtask | commands::deploy::website::tests::asset_probe_checks_the_registry_file_and_the_legacy_directory
xtask | commands::deploy::website::tests::asset_probe_distinguishes_the_three_layouts
xtask | commands::deploy::website::tests::only_a_legacy_or_unreadable_layout_refuses_the_deploy
xtask | commands::deploy::website::tests::prairielearn_case_insensitive
xtask | commands::deploy::website::tests::remote_prefix_rejects_escape_and_outside
xtask | commands::deploy::website::tests::rsync_argv_keeps_source_and_destination_last
xtask | commands::deploy::website::tests::rsync_excludes_the_secrets_asset_and_scratch_trees
xtask | commands::deploy::website::tests::the_checksum_repair_runs_in_the_remote_checkout_against_the_staging_container
xtask | commands::deploy::website::tests::the_remediation_names_every_directory_that_must_move
xtask | commands::deploy::website::tests::the_remote_plan_ends_with_the_checksum_repair_and_the_state_move
xtask | commands::deploy::website::tests::the_state_move_targets_the_unit_state_directory_and_is_idempotent
xtask | commands::deploy::website::tests::the_unit_install_command_renders_the_shipped_template_for_the_remote_dir
xtask | commands::deploy::website::tests::the_unit_template_declares_the_state_directory_the_deploy_moves_into
xtask | commands::deploy::website::tests::usage_mentions_dry_run
xtask | commands::fetch::vanilla_api::tests::cache_hit_index_only_rc0
xtask | commands::fetch::vanilla_api::tests::doxy_name_mangles_underscores
xtask | commands::fetch::vanilla_api::tests::from_file_missing_arg_exits_2
xtask | commands::fetch::vanilla_api::tests::from_file_nonexistent_continues_rc0
xtask | commands::fetch::vanilla_api::tests::from_file_usage_line_names_the_runnable_command
xtask | commands::fetch::vanilla_api::tests::index_miss_exits_1
xtask | commands::fetch::vanilla_source::tests::curated_list_holds_nineteen_entries
xtask | commands::fetch::vanilla_source::tests::empty_index_map_build_exits_1
xtask | commands::fetch::vanilla_source::tests::grep_empty_pattern_exits_2
xtask | commands::fetch::vanilla_source::tests::grep_missing_pattern_exits_2
xtask | commands::fetch::vanilla_source::tests::grep_usage_line_names_the_runnable_command
xtask | commands::fetch::vanilla_source::tests::help_is_filename_miss_rc0
xtask | commands::map::terrain_export::tests::parse_phase_and_default
xtask | commands::map::terrain_export::tests::parse_terrain_from_env_when_no_positional
xtask | commands::map::terrain_export::tests::parse_unknown_arg
xtask | commands::map::terrain_export::tests::parse_usage_when_no_terrain
xtask | commands::mcp::call::tests::emit_requests_embeds_tool_and_args
xtask | commands::mcp::call::tests::usage_when_tool_missing
xtask | commands::mcp::call_selftest::tests::trailing_newlines_are_stripped_so_a_blank_body_reads_as_empty
xtask | commands::mcp::daemon::tests::status_stopped_when_no_socket
xtask | commands::mcp::daemon::tests::usage_rejects_unknown_action
xtask | commands::mcp::netapi::tests::request_and_response_framing_round_trip
xtask | commands::mcp::smoke::tests::an_empty_body_with_a_zero_exit_code_is_a_failure
xtask | commands::mcp::smoke::tests::every_tool_answering_non_empty_is_the_only_green
xtask | commands::mcp::smoke::tests::every_tool_is_attempted_when_the_first_one_fails
xtask | commands::mcp::smoke::tests::trailing_newlines_are_stripped_so_a_blank_body_reads_as_empty
xtask | commands::mcp::workbench_logs::file_cli_tests::file_equals_empty_parses_via_clap
xtask | commands::mcp::workbench_logs::tests::errors_present_fail
xtask | commands::mcp::workbench_logs::tests::file_equals_empty_is_environment_rc3
xtask | commands::mcp::workbench_logs::tests::file_missing_sentinel_is_usage_rc3
xtask | commands::mcp::workbench_logs::tests::healthy_no_player_is_partial
xtask | commands::mcp::workbench_logs::tests::healthy_with_player_passes
xtask | commands::mcp::workbench_logs::tests::missing_file_is_environment
xtask | commands::mcp::workbench_logs::tests::preprocess_rewrites_file_empty_arg_to_missing_sentinel
xtask | commands::mcp::workbench_logs::tests::selftest_pass
xtask | commands::mcp::workbench_logs::tests::stale_fails
xtask | commands::mod_ops::backend_api_test::tests::clamp_code_passthrough
xtask | commands::mod_ops::backend_api_test::tests::paths_mirror_paths_sh
xtask | commands::mod_ops::compile::tests::count_game_scripts_counts_only_c_and_refuses_a_missing_tree
xtask | commands::mod_ops::compile::tests::missing_probe_is_rc2
xtask | commands::mod_ops::compile::tests::no_addon_is_rc3
xtask | commands::mod_ops::compile::tests::no_server_is_rc3
xtask | commands::mod_ops::compile::tests::workbench_tooling_guard_reports_the_dir_and_what_is_in_it
xtask | commands::mod_ops::development_bootstrap::tests::port_open_rejects_non_numeric_needle
xtask | commands::mod_ops::development_server::tests::no_args_is_rc2
xtask | commands::mod_ops::development_server::tests::usage_names_the_runnable_playtest_command
xtask | commands::mod_ops::mission_test::tests::backend_and_stage_round_trip
xtask | commands::mod_ops::mission_test::tests::missing_config_exits_1
xtask | commands::mod_ops::mission_test::tests::set_mission_id_no_trailing_newline
xtask | commands::mod_ops::mission_test::tests::unknown_golden_exits_1
xtask | commands::mod_ops::playtest_server::boot::tests::a_timeout_becomes_a_far_side_timeout_prefix
xtask | commands::mod_ops::playtest_server::boot::tests::the_join_details_are_scraped_out_of_the_engines_lines
xtask | commands::mod_ops::playtest_server::boot::tests::the_launcher_argv_is_the_one_the_engine_needs
xtask | commands::mod_ops::playtest_server::lifecycle::tests::a_broken_bridge_probes_unknown_not_dead
xtask | commands::mod_ops::playtest_server::lifecycle::tests::a_stale_lock_is_taken_over
xtask | commands::mod_ops::playtest_server::lifecycle::tests::an_empty_pgid_is_unknown_never_dead
xtask | commands::mod_ops::playtest_server::lifecycle::tests::an_empty_pidfile_clears_the_way
xtask | commands::mod_ops::playtest_server::lifecycle::tests::an_owner_file_that_never_fills_is_stale_not_a_refusal
xtask | commands::mod_ops::playtest_server::lifecycle::tests::kill_run_keeps_the_pidfile_when_it_cannot_confirm
xtask | commands::mod_ops::playtest_server::lifecycle::tests::local_liveness_handles_junk_owners
xtask | commands::mod_ops::playtest_server::lifecycle::tests::no_pidfile_is_a_clean_kill_run_but_not_a_claim_of_death
xtask | commands::mod_ops::playtest_server::lifecycle::tests::read_pgid_strips_all_whitespace
xtask | commands::mod_ops::playtest_server::lifecycle::tests::stray_warning_names_the_group_the_ports_and_the_pidfile
xtask | commands::mod_ops::playtest_server::lifecycle::tests::the_lock_is_exclusive_and_released_on_drop
xtask | commands::mod_ops::playtest_server::lifecycle::tests::unknown_is_not_dead_and_has_no_bool_shortcut
xtask | commands::mod_ops::playtest_server::lifecycle::tests::unreachable_bridge_refuses_to_stage_rather_than_guessing
xtask | commands::mod_ops::playtest_server::logread::tests::a_guid_outside_the_loaded_addons_window_does_not_count
xtask | commands::mod_ops::playtest_server::logread::tests::an_absent_log_is_a_phase_not_a_crash
xtask | commands::mod_ops::playtest_server::logread::tests::boot_phase_reports_the_furthest_milestone_not_the_first
xtask | commands::mod_ops::playtest_server::logread::tests::console_path_falls_back_to_the_literal_slash_console_log
xtask | commands::mod_ops::playtest_server::logread::tests::console_path_takes_the_newest_logs_dir
xtask | commands::mod_ops::playtest_server::logread::tests::grep_n_numbers_from_one_and_grep_c_counts_lines
xtask | commands::mod_ops::playtest_server::logread::tests::loaded_addon_line_takes_the_last_match_like_tail_1
xtask | commands::mod_ops::playtest_server::logread::tests::phases_below_the_world_do_not_claim_the_world_is_up
xtask | commands::mod_ops::playtest_server::logread::tests::the_hard_gate_holds_on_a_real_engine_boot
xtask | commands::mod_ops::playtest_server::logread::tests::the_lobby_marker_survives_a_reworded_arrow
xtask | commands::mod_ops::playtest_server::logread::tests::the_local_addon_must_win_or_the_gate_fails
xtask | commands::mod_ops::playtest_server::logread::tests::the_registered_marker_and_the_fatals_are_the_engines_own_strings
xtask | commands::mod_ops::playtest_server::logread::tests::the_vanilla_error_floor_is_demoted_not_the_real_cause
xtask | commands::mod_ops::playtest_server::render::tests::a2s_and_game_are_created_when_absent_and_land_at_the_end
xtask | commands::mod_ops::playtest_server::render::tests::a_non_object_a2s_is_an_error_not_a_silent_overwrite
xtask | commands::mod_ops::playtest_server::render::tests::admins_json_uses_pythons_comma_space_separator
xtask | commands::mod_ops::playtest_server::render::tests::backend_config_patch_sets_event_id_even_when_empty_and_keeps_the_token
xtask | commands::mod_ops::playtest_server::render::tests::empty_admins_are_dropped_by_the_line_pipeline
xtask | commands::mod_ops::playtest_server::render::tests::ensure_ascii_matches_cpython
xtask | commands::mod_ops::playtest_server::render::tests::int_like_reproduces_the_python_valueerror_text
xtask | commands::mod_ops::playtest_server::render::tests::newline_in_an_admin_splits_it_in_two
xtask | commands::mod_ops::playtest_server::render::tests::render_reproduces_the_measured_cpython_key_order
xtask | commands::mod_ops::playtest_server::tests::a_gproj_without_a_guid_yields_empty_not_a_guess
xtask | commands::mod_ops::playtest_server::tests::admin_newline_widening_is_preserved
xtask | commands::mod_ops::playtest_server::tests::admin_schema_matches_the_engines_two_patterns
xtask | commands::mod_ops::playtest_server::tests::admins_are_repeatable_and_ordered
xtask | commands::mod_ops::playtest_server::tests::bare_and_lone_dashes_are_unknown_arguments
xtask | commands::mod_ops::playtest_server::tests::defaults_match_the_bash_variable_block
xtask | commands::mod_ops::playtest_server::tests::empty_values_are_carried_not_dropped
xtask | commands::mod_ops::playtest_server::tests::guid_is_read_out_of_a_real_gproj_shape
xtask | commands::mod_ops::playtest_server::tests::help_opens_with_the_runnable_command_and_lists_every_option
xtask | commands::mod_ops::playtest_server::tests::help_position_decides_the_exit_code
xtask | commands::mod_ops::playtest_server::tests::help_text_matches_the_options_we_parse
xtask | commands::mod_ops::playtest_server::tests::scenario_extraction_stops_at_the_comma_and_the_quote
xtask | commands::mod_ops::playtest_server::tests::values_may_contain_equals_signs
xtask | commands::mod_ops::wave_execution::tests::land_refuses_dirty_worktree
xtask | commands::mod_ops::wave_execution::tests::missing_lock_is_a_refusal_not_all_shipped
xtask | commands::mod_ops::wave_execution::tests::parent_slice_normalises_sub_slices
xtask | commands::mod_ops::wave_execution::tests::prep_done_prints_nothing
xtask | commands::mod_ops::wave_execution::tests::status_absent_worktrees_on_scratch
xtask | commands::mod_ops::wave_execution::tests::unknown_command_prints_help_rc2
xtask | commands::mod_ops::world_boot_verdict::tests::bad_missing_rejects_exit_equiv_1
xtask | commands::mod_ops::world_boot_verdict::tests::bad_unknown_class_rejects
xtask | commands::mod_ops::world_boot_verdict::tests::good_log_passes
xtask | commands::mod_ops::world_boot_verdict::tests::selftest_harness_ok
xtask | commands::platform::preflight::run_target_tests::a_run_target_that_was_never_built_is_green
xtask | commands::platform::preflight::run_target_tests::a_stamp_from_a_worktree_at_the_same_sha_still_blocks_and_names_the_checkout
xtask | commands::platform::preflight::run_target_tests::a_stamp_that_disagrees_with_head_blocks_and_names_the_binary
xtask | commands::platform::preflight::run_target_tests::an_unresolvable_head_blocks_rather_than_certifies
xtask | commands::platform::preflight::run_target_tests::binaries_built_from_head_in_this_checkout_are_green
xtask | commands::platform::preflight::run_target_tests::binaries_with_no_stamp_block_rather_than_pass
xtask | commands::platform::preflight::run_target_tests::run_binaries_lists_executables_and_skips_the_stamp_and_depfiles
xtask | commands::platform::preflight::run_target_tests::the_release_profile_is_checked_too
xtask | commands::platform::preflight::run_target_tests::the_stamp_round_trips_and_an_absent_one_reads_as_unknown
xtask | commands::platform::slice_execution::tests::exit_zero_without_usage_fails_and_writes_no_file
xtask | commands::platform::slice_execution::tests::fixture_replay_writes_a_receipt_without_spawning
xtask | commands::platform::slice_execution::tests::malformed_started_override_is_refused
xtask | commands::platform::slice_execution::tests::non_claude_code_executor_is_refused
xtask | commands::platform::slice_execution::tests::slice_ids_resolve_through_the_parent_slice_plan
xtask | commands::platform::slice_execution::tests::stub_binary_claude_dialect_parses_too
xtask | commands::platform::slice_execution::tests::stub_binary_run_writes_a_receipt_with_the_recorded_tokens
xtask | commands::platform::slice_worktree::tests::ancestor_clause_is_inert
xtask | commands::platform::slice_worktree::tests::drop_refuses_a_dirty_tree_even_when_nothing_is_unmerged
xtask | commands::platform::slice_worktree::tests::drop_refuses_unmerged_commits_and_force_overrides
xtask | commands::platform::slice_worktree::tests::ln_sfn_replaces_a_symlink_but_descends_into_a_real_directory
xtask | commands::platform::slice_worktree::tests::merge_refuses_a_dirty_tree
xtask | commands::platform::slice_worktree::tests::merge_refuses_a_slice_no_gate_has_examined
xtask | commands::platform::slice_worktree::tests::merge_reports_an_absent_worktree_and_lands_a_clean_one
xtask | commands::platform::slice_worktree::tests::missing_slice_id_is_exit_2_on_every_arm
xtask | commands::platform::slice_worktree::tests::new_creates_a_tree_links_the_lanes_and_is_idempotent
xtask | commands::platform::slice_worktree::tests::new_refuses_when_a_required_oracle_is_missing
xtask | commands::platform::slice_worktree::tests::pins_the_sed_regex_oddities
xtask | commands::platform::slice_worktree::tests::reap_guards_every_destructive_case_in_one_pass
xtask | commands::platform::slice_worktree::tests::sub_slice_shares_the_parent_tree
xtask | commands::platform::slice_worktree::tests::unknown_and_empty_subcommands_print_usage_and_exit_2
xtask | commands::platform::slice_worktree::tests::usage_spells_every_subcommand_as_a_runnable_command
xtask | commands::platform::wave_execution::base::tests::the_four_accepted_suffixes_and_nothing_else
xtask | commands::platform::wave_execution::base::tests::the_plan_speaks_for_a_wave_that_has_already_emptied
xtask | commands::platform::wave_execution::base::tests::the_prefilter_and_the_authority_agree_on_the_delimiters
xtask | commands::platform::wave_execution::changed::tests::edition_falls_back_to_2021_when_nothing_says_otherwise
xtask | commands::platform::wave_execution::changed::tests::edition_is_read_from_the_nearest_manifest
xtask | commands::platform::wave_execution::changed::tests::join_rel_resolves_dotdot_and_refuses_to_climb_out
xtask | commands::platform::wave_execution::changed::tests::realpath_m_normalises_without_touching_the_disk
xtask | commands::platform::wave_execution::changed::tests::the_frontends_include_str_inputs_are_in_scope_and_the_apis_are_not
xtask | commands::platform::wave_execution::changed::tests::the_wasm_scope_follows_the_frontends_dependency_graph
xtask | commands::platform::wave_execution::changed::tests::workspace_members_parse_is_not_empty_on_the_real_manifest
xtask | commands::platform::wave_execution::gate::tests::both_gates_run_the_same_ten_class_r_verifies
xtask | commands::platform::wave_execution::gate::tests::the_step_runner_indents_the_last_fifteen_lines_on_failure
xtask | commands::platform::wave_execution::host::tests::bridged_hostrun_forwards_the_whitelist_because_distrobox_does_not
xtask | commands::platform::wave_execution::host::tests::checkrun_second_env_wins_over_the_baked_in_one
xtask | commands::platform::wave_execution::host::tests::native_hostrun_wraps_in_timeout_only
xtask | commands::platform::wave_execution::host::tests::timeout_rc_124_survives_as_124
xtask | commands::platform::wave_execution::land::tests::a_dirty_tree_refuses_before_any_commit_exists
xtask | commands::platform::wave_execution::land::tests::a_hostile_summary_cannot_change_the_marker_number
xtask | commands::platform::wave_execution::land::tests::an_oracle_refused_number_refuses_before_any_commit_exists
xtask | commands::platform::wave_execution::land::tests::close_ceremony_commits_the_marker_and_the_lock_refresh_and_ends_check_green
xtask | commands::platform::wave_execution::land::tests::close_targets_the_recorded_emptied_label_end_to_end
xtask | commands::platform::wave_execution::land::tests::close_tickets_flag_parses_and_still_refuses_an_unshipped_id
xtask | commands::platform::wave_execution::land::tests::close_tickets_refuses_a_label_that_is_still_an_open_wave
xtask | commands::platform::wave_execution::land::tests::dry_run_prints_the_subject_and_writes_nothing
xtask | commands::platform::wave_execution::land::tests::nothing_pending_refuses_with_zero_writes
xtask | commands::platform::wave_execution::land::tests::registry_view_reports_a_shipped_child_ticket_as_shipped
xtask | commands::platform::wave_execution::land::tests::the_allowlist_refuses_anything_that_is_not_wave_or_a_ticket
xtask | commands::platform::wave_execution::land::tests::the_close_argument_parser_is_an_allowlist
xtask | commands::platform::wave_execution::land::tests::the_subject_builder_sanitises_and_the_authority_accepts_every_product
xtask | commands::platform::wave_execution::land::tests::two_pending_labels_drain_oldest_first
xtask | commands::platform::wave_execution::lock::tests::a_fresh_state_is_neither_held_nor_degraded
xtask | commands::platform::wave_execution::lock::tests::bash_flock_and_this_port_contend_on_one_file
xtask | commands::platform::wave_execution::lock::tests::unserialised_verdict_cannot_look_like_a_clean_pass
xtask | commands::platform::wave_execution::migrate::tests::a_line_comment_inside_a_block_does_not_end_it
xtask | commands::platform::wave_execution::migrate::tests::comment_stripping_survives_multiline_blocks_and_line_comments
xtask | commands::platform::wave_execution::migrate::tests::migration_version_and_description_match_the_sed_pipeline
xtask | commands::platform::wave_execution::push::tests::a_path_list_larger_than_the_pipe_buffer_does_not_deadlock
xtask | commands::platform::wave_execution::push::tests::feed_and_capture_reports_a_child_that_exits_non_zero
xtask | commands::platform::wave_execution::push::tests::probe_ok_is_true_only_for_a_zero_exit
xtask | commands::platform::wave_execution::push::tests::push_plan_drops_no_verify_exactly_when_git_lfs_is_present
xtask | commands::platform::wave_execution::reclaim::tests::adhoc_pattern_is_uppercase_t_with_a_dash_only
xtask | commands::platform::wave_execution::reclaim::tests::key_matches_the_tr_pipeline
xtask | commands::platform::wave_execution::reclaim::tests::positive_identification_spares_everything_it_cannot_parse
xtask | commands::platform::wave_execution::run_target_tests::run_lane_refuses_a_worktree_and_names_both_checkouts
xtask | commands::platform::wave_execution::run_target_tests::run_lane_refuses_an_empty_argv_and_a_caller_supplied_target_dir
xtask | commands::platform::wave_execution::run_target_tests::run_target_is_one_child_of_the_shared_cache_and_never_the_cache_itself
xtask | commands::platform::wave_execution::run_target_tests::split_run_args_puts_program_arguments_only_on_run
xtask | commands::platform::wave_execution::run_target_tests::stamp_parse_refuses_what_it_cannot_read
xtask | commands::platform::wave_execution::schema::tests::cksum_matches_the_coreutils_tool
xtask | commands::platform::wave_execution::schema::tests::empty_input_matches_too
xtask | commands::platform::wave_execution::schema::tests::the_task_table_and_the_pinned_set_agree
xtask | commands::platform::wave_execution::test_cmd::tests::foreign_slice_tokens_are_recognised_so_the_banner_can_be_withheld
xtask | commands::platform::wave_execution::test_cmd::tests::slice_id_normalisation_matches_the_parameter_expansion
xtask | commands::platform::wave_execution::touch::tests::package_name_reads_only_the_package_table
xtask | commands::platform::wave_execution::verdict::tests::a_lost_ignore_file_is_rewritten_on_the_next_gate
xtask | commands::platform::wave_execution::verdict::tests::a_non_canonical_slice_id_is_told_the_truth_not_to_re_gate
xtask | commands::platform::wave_execution::verdict::tests::a_receipt_round_trips_through_disk
xtask | commands::platform::wave_execution::verdict::tests::a_receipt_without_a_sha_or_with_a_bogus_verdict_is_never_written
xtask | commands::platform::wave_execution::verdict::tests::a_slice_id_that_is_not_a_ticket_id_is_refused
xtask | commands::platform::wave_execution::verdict::tests::an_extra_field_is_refused_rather_than_ignored
xtask | commands::platform::wave_execution::verdict::tests::an_unreadable_receipt_is_a_refusal_not_an_absence
xtask | commands::platform::wave_execution::verdict::tests::an_unrecognised_verdict_string_is_not_green
xtask | commands::platform::wave_execution::verdict::tests::an_unresolvable_landing_head_is_refused
xtask | commands::platform::wave_execution::verdict::tests::cmd_land_refuses_before_it_merges
xtask | commands::platform::wave_execution::verdict::tests::gate_slice_writes_a_receipt_on_every_run
xtask | commands::platform::wave_execution::verdict::tests::land_accepts_a_green_receipt_at_the_landing_sha
xtask | commands::platform::wave_execution::verdict::tests::land_refuses_a_red_receipt
xtask | commands::platform::wave_execution::verdict::tests::land_refuses_a_stale_receipt_and_names_both_shas
xtask | commands::platform::wave_execution::verdict::tests::land_refuses_when_no_gate_has_run
xtask | commands::platform::wave_execution::verdict::tests::the_newest_gate_wins
xtask | commands::platform::wave_execution::verdict::tests::the_receipt_is_exactly_the_three_contract_fields
xtask | commands::platform::wave_execution::verdict::tests::the_receipts_directory_hides_itself_from_git
xtask | commands::reproduction::mission_version_upload::tests::extract_token_matches_sed
xtask | commands::schema::mission_flattening::tests::flatten_apply_preserves_schema_version_1_0_defense
xtask | commands::schema::mission_flattening::tests::flatten_in_place_preserves_loadout_uid_and_schema
xtask | commands::schema::mission_flattening::tests::flatten_in_place_preserves_schema_version_1_0
xtask | commands::schema::mission_flattening::tests::flatten_in_place_refuses_empty_slots_overwrite
xtask | commands::schema::mission_flattening::tests::flatten_in_place_refuses_lossy_loadout_drop
xtask | commands::schema::mission_flattening::tests::flatten_orbat_slots_no_post_apply_schema_reassign_source_ratchet
xtask | commands::schema::mission_flattening::tests::flatten_stdout_preserves_loadout_uid_and_schema
xtask | commands::schema::mission_flattening::tests::flatten_stdout_preserves_schema_version_1_0
xtask | commands::schema::mission_flattening::tests::flatten_stdout_refuses_empty_slots
xtask | commands::schema::mission_flattening::tests::flatten_stdout_refuses_lossy_loadout_drop
xtask | commands::setup::client_addons::tests::clean_tree_symlinks_and_prints
xtask | commands::setup::client_addons::tests::force_symlink_like_sfn_replaces_symlink
xtask | commands::setup::client_addons::tests::missing_framework_still_succeeds_dangling
xtask | commands::setup::client_addons::tests::run_reads_home_env
xtask | commands::setup::client_addons::tests::staging_is_file_exits_1
xtask | commands::setup::client_addons::tests::staging_not_writable_ln_exits_1
xtask | commands::setup::mcp_game_root::tests::addons_as_file_exits_1
xtask | commands::setup::mcp_game_root::tests::clean_tree_links_flattened_names
xtask | commands::setup::mcp_game_root::tests::empty_addons_links_zero
xtask | commands::setup::mcp_game_root::tests::flatten_replaces_every_slash
xtask | commands::setup::mcp_game_root::tests::is_pak_case_insensitive
xtask | commands::setup::mcp_game_root::tests::missing_addons_dir_exits_1
xtask | commands::setup::server_profile::tests::clean_tree_writes_modes_and_mission_id_name
xtask | commands::setup::server_profile::tests::missing_backend_exits_1
xtask | commands::setup::server_profile::tests::missing_golden_exits_1_with_bash_stderr
xtask | commands::setup::server_profile::tests::substitute_preserves_ampersand_and_pipe
xtask | commands::setup::server_profile::tests::token_from_env_first_line_wins
xtask | commands::setup::server_profile::tests::token_from_env_strips_quotes_and_cr
xtask | commands::setup::staging_server::tests::arm_missing_host_exits_1
xtask | commands::setup::staging_server::tests::arm_prairielearn_exits_1
xtask | commands::setup::staging_server::tests::defaults_fill_when_unset
xtask | commands::setup::staging_server::tests::the_deploy_file_resolves_against_the_given_root
xtask | commands::setup::staging_server::tests::the_prairielearn_refusal_is_case_sensitive
xtask | commands::setup::workbench_linux::tests::clean_tree_symlinks_and_prints
xtask | commands::setup::workbench_linux::tests::missing_gproj_exits_1
xtask | commands::setup::workbench_linux::tests::missing_steam_tree_exits_1
xtask | commands::setup::workbench_linux::tests::relink_replaces_existing_symlink
xtask | commands::setup::workbench_linux::tests::run_reads_home_and_steam_base_env
xtask | commands::ticket::execution::tests::cleanup_removes_an_unregistered_worktree_directory
xtask | commands::ticket::execution::tests::done_cleans_before_a_shipping_refusal
xtask | core::cargo_target_directory::tests::abi_guard_refuses_a_foreign_stamp
xtask | core::cargo_target_directory::tests::echo_matches_make
xtask | core::cargo_target_directory::tests::every_advertised_target_dispatches
xtask | core::cargo_target_directory::tests::leptos_gates_does_not_double_build
xtask | core::cargo_target_directory::tests::only_rust_api_sets_a_private_target_dir
xtask | core::cargo_target_directory::tests::pin_marker_is_present_in_this_file
xtask | core::cargo_target_directory::tests::pin_marker_verdict_fails_when_the_formula_is_gone
xtask | core::cargo_target_directory::tests::pin_points_at_the_primary_repo_not_the_worktree
xtask | core::cargo_target_directory::tests::private_target_dir_violation_bites
xtask | core::cargo_target_directory::tests::reclaim_never_touches_a_live_target_dir
xtask | core::cargo_target_directory::tests::reclaim_refusals_are_preserved
xtask | core::cargo_target_directory::tests::resolve_target_dir_follows_make_question_equals
xtask | core::cargo_target_directory::tests::worktree_local_pin_is_detected
xtask | core::host_execution::tests::a_broken_bridge_answers_nothing_even_when_one_exists
xtask | core::host_execution::tests::a_containerised_host_with_no_bridge_is_rc_127_and_silent
xtask | core::host_execution::tests::a_containerised_host_with_no_bridge_refuses_loudly_with_rc_127
xtask | core::host_execution::tests::bridge_is_never_used_on_the_metal
xtask | core::host_execution::tests::capture_trimmed_deletes_all_whitespace_like_tr_d
xtask | core::host_execution::tests::detect_agrees_with_the_two_marker_files
xtask | core::host_execution::tests::in_a_container_the_bridge_is_prepended
xtask | core::host_execution::tests::instruction_name_is_the_paste_ready_bridge_not_the_resolved_one
xtask | core::host_execution::tests::on_the_metal_commands_run_directly_and_need_no_bridge
xtask | core::host_execution::tests::the_bridge_really_crosses_the_container_wall
xtask | core::host_execution::tests::the_refusal_is_the_bash_heredoc
xtask | core::host_execution::tests::trailing_version_is_grep_o_anchored_at_end
xtask | core::repository_layout::tests::every_committed_location_exists_in_the_checkout
xtask | core::repository_layout::tests::every_file_sits_inside_the_directory_that_describes_it
xtask | core::repository_layout::tests::the_deploy_secrets_file_sits_beside_its_example
xtask | core::repository_root::tests::nested_tooling_directories_resolve_repository_and_fixtures
xtask | core::test_environment::tests::prepend_dir_keeps_usr_bin
xtask | tooling_dependency_boundaries::foundational_engines_have_no_workspace_dependencies
xtask | tooling_dependency_boundaries::inline_module_detection_handles_nested_syntax_without_matching_source_strings
xtask | tooling_dependency_boundaries::structural_limits_distinguish_scenarios_and_separate_tests
xtask | tooling_dependency_boundaries::the_tooling_tree_holds_its_executables_manifests_and_layout_modules
xtask | tooling_dependency_boundaries::ticket_implementations_have_one_owner
xtask | tooling_dependency_boundaries::tooling_crates_have_no_file_size_exemptions
xtask | tooling_dependency_boundaries::tooling_dependency_direction_is_enforced
xtask | tooling_dependency_boundaries::tooling_source_files_stay_below_their_structural_limits
xtask | tooling_dependency_boundaries::tooling_test_modules_live_in_separate_files
xtask | verifications::architecture::editor_orbat_coherency::tests::a_missing_target_never_reads_as_a_pass
xtask | verifications::architecture::editor_orbat_coherency::tests::every_cargo_pin_arm_can_go_red
xtask | verifications::architecture::editor_orbat_coherency::tests::every_static_arm_can_go_red
xtask | verifications::architecture::editor_orbat_coherency::tests::merged_capture_keeps_order_and_never_invents_an_exit_code
xtask | verifications::architecture::editor_orbat_coherency::tests::the_argv_rendering_and_the_pin_table_match_the_script
xtask | verifications::architecture::editor_orbat_coherency::tests::the_real_tree_holds
xtask | verifications::architecture::engine_layer_boundaries::tests::a_browser_name_inside_the_engines_editing_tree_fails
xtask | verifications::architecture::engine_layer_boundaries::tests::a_dependency_edge_is_a_breach_and_a_comment_is_not
xtask | verifications::architecture::engine_layer_boundaries::tests::a_missing_manifest_does_not_read_as_clean
xtask | verifications::architecture::engine_layer_boundaries::tests::a_new_gpu_module_import_in_the_map_engine_fails
xtask | verifications::architecture::engine_layer_boundaries::tests::a_pure_renderer_passes_and_prose_does_not_trip_it
xtask | verifications::architecture::engine_layer_boundaries::tests::an_absent_editing_tree_is_not_a_clean_rule_5
xtask | verifications::architecture::engine_layer_boundaries::tests::an_absent_frontend_is_not_a_clean_rule_6
xtask | verifications::architecture::engine_layer_boundaries::tests::an_absent_half_of_the_wall_is_not_a_clean_wall
xtask | verifications::architecture::engine_layer_boundaries::tests::build_output_is_pruned_but_only_below_the_root
xtask | verifications::architecture::engine_layer_boundaries::tests::importing_the_map_engine_fails
xtask | verifications::architecture::engine_layer_boundaries::tests::inputs_that_were_never_read_do_not_pass
xtask | verifications::architecture::engine_layer_boundaries::tests::map_nouns_in_declared_names_fail
xtask | verifications::architecture::engine_layer_boundaries::tests::naming_the_frame_vocabulary_outside_the_boundary_fails
xtask | verifications::architecture::engine_layer_boundaries::tests::rule_3a_matches_the_frame_path_and_not_the_crate_local_one
xtask | verifications::architecture::engine_layer_boundaries::tests::rule_3b_matches_the_five_gpu_modules_and_nothing_adjacent
xtask | verifications::architecture::engine_layer_boundaries::tests::rule_4_matches_only_what_is_outside_the_scenario_tree
xtask | verifications::architecture::engine_layer_boundaries::tests::rule_5_is_scoped_to_editing_and_not_the_whole_crate
xtask | verifications::architecture::engine_layer_boundaries::tests::rule_6_matches_imports_and_not_the_prose_that_describes_the_wall
xtask | verifications::architecture::engine_layer_boundaries::tests::rule_7_matches_the_wall_and_not_the_legal_traffic
xtask | verifications::architecture::engine_layer_boundaries::tests::rule_two_matches_declarations_and_not_mentions
xtask | verifications::architecture::engine_layer_boundaries::tests::the_frontend_importing_the_renderer_fails
xtask | verifications::architecture::engine_layer_boundaries::tests::the_rule_3a_pin_is_a_ratchet_in_both_directions
xtask | verifications::architecture::engine_layer_boundaries::tests::the_rule_3b_pin_is_a_ratchet_in_both_directions
xtask | verifications::architecture::engine_layer_boundaries::tests::the_rule_4_pin_is_a_ratchet_in_both_directions
xtask | verifications::architecture::engine_layer_boundaries::tests::the_scenario_tree_reaching_outside_itself_fails
xtask | verifications::architecture::engine_layer_boundaries::tests::the_world_data_wall_fails_in_either_direction
xtask | verifications::architecture::route_tags::tests::a_broken_router_shape_is_a_failure_not_a_pass
xtask | verifications::architecture::route_tags::tests::a_merge_without_a_route_file_fails
xtask | verifications::architecture::route_tags::tests::a_route_with_no_tag_fails
xtask | verifications::architecture::route_tags::tests::a_tag_pointing_at_no_route_fails
xtask | verifications::architecture::route_tags::tests::an_unmerged_route_file_fails
xtask | verifications::architecture::route_tags::tests::an_unreadable_parse_is_named_not_skipped
xtask | verifications::architecture::route_tags::tests::clean_tree_passes_and_counts_exactly
xtask | verifications::architecture::route_tags::tests::collation_reproduces_measured_glibc_order
xtask | verifications::architecture::route_tags::tests::inputs_that_were_never_read_do_not_pass
xtask | verifications::architecture::route_tags::tests::routes_spanning_two_files_are_unioned_and_sorted_stably
xtask | verifications::architecture::route_tags::tests::tag_parsing_edge_cases
xtask | verifications::architecture::route_tags::tests::the_merge_extractor_reads_only_the_merge_function
xtask | verifications::architecture::route_tags::tests::the_router_extractor_chains_and_marks
xtask | verifications::architecture::route_tags::tests::zero_route_files_is_not_a_pass
xtask | verifications::ci::schema_parity::tests::ci_run_accepts_the_alias_and_the_long_form_only
xtask | verifications::ci::schema_parity::tests::disconnected_or_conditionally_disabled_children_fail
xtask | verifications::ci::schema_parity::tests::each_hollowed_live_wave_body_fails_the_runtime_gate
xtask | verifications::ci::schema_parity::tests::each_missing_wave_child_fails_the_runtime_gate
xtask | verifications::ci::schema_parity::tests::facade_exports_cannot_replace_implementation_bodies
xtask | verifications::ci::schema_parity::tests::live_shaped_pins_hold
xtask | verifications::ci::schema_parity::tests::live_source_owners_satisfy_the_runtime_gate
xtask | verifications::ci::schema_parity::tests::missing_wave_fails
xtask | verifications::ci::schema_parity::tests::schema_job_without_ci_local_schema_fails
xtask | verifications::ci::schema_parity::tests::task_echoes_name_the_same_gates_as_the_wave_consts
xtask | verifications::ci::schema_parity::tests::the_live_task_table_satisfies_the_recipe_pins
xtask | verifications::ci::schema_parity::tests::the_make_spelling_does_not_satisfy_the_ci_pin
xtask | verifications::ci::schema_parity::tests::this_gate_stays_off_the_dispatch_table_it_polices
xtask | verifications::ci::schema_parity::tests::verify_consts_are_the_cargo_spelling
xtask | verifications::ci::schema_parity::tests::wave_commented_run_does_not_satisfy
xtask | verifications::ci::schema_parity::tests::wave_hollow_both_paths_required
xtask | verifications::ci::schema_parity::tests::wave_hollow_checkrun_argv_fails
xtask | verifications::ci::schema_parity::tests::wave_suffix_smuggles_fail_pin
xtask | verifications::ci::workflow_shell::tests::ampersand_background_is_red
xtask | verifications::ci::workflow_shell::tests::cargo_fmt_and_true_is_red
xtask | verifications::ci::workflow_shell::tests::cargo_xtask_verify_no_shell_holds
xtask | verifications::ci::workflow_shell::tests::defaults_run_only_no_steps_is_red
xtask | verifications::ci::workflow_shell::tests::defaults_run_without_step_run_does_not_false_fail
xtask | verifications::ci::workflow_shell::tests::echo_hi_is_red
xtask | verifications::ci::workflow_shell::tests::empty_workflows_dir_is_red
xtask | verifications::ci::workflow_shell::tests::git_lfs_pull_holds
xtask | verifications::ci::workflow_shell::tests::job_level_evil_uses_no_steps_is_red
xtask | verifications::ci::workflow_shell::tests::more_than_three_logical_lines_is_red
xtask | verifications::ci::workflow_shell::tests::multiline_run_with_if_and_set_dash_is_red
xtask | verifications::ci::workflow_shell::tests::planted_evil_composite_is_red
xtask | verifications::ci::workflow_shell::tests::production_parser_is_the_fixture_parser
xtask | verifications::ci::workflow_shell::tests::unreadable_yaml_is_fail_closed
xtask | verifications::ci::workflow_shell::tests::uses_plus_run_is_red
xtask | verifications::ci::workflow_shell_rules::tests::ampersand_background_is_red
xtask | verifications::ci::workflow_shell_rules::tests::cargo_xtask_is_allowlisted
xtask | verifications::ci::workflow_shell_rules::tests::double_ampersand_still_reports_and_and
xtask | verifications::ci::workflow_shell_rules::tests::git_lfs_pull_is_allowlisted
xtask | verifications::ci::workflow_shell_rules::tests::pipe_is_forbidden_even_on_xtask
xtask | verifications::ci::workflow_shell_rules::tests::unpinned_cargo_install_is_not_allowlisted
xtask | verifications::database::faction_library_seeds::tests::an_empty_seed_list_is_its_own_failure
xtask | verifications::database::faction_library_seeds::tests::both_wave_paths_are_pinned
xtask | verifications::database::faction_library_seeds::tests::comment_only_starter_name_is_not_a_seed
xtask | verifications::database::faction_library_seeds::tests::disconnected_wave_implementation_fails_closed
xtask | verifications::database::faction_library_seeds::tests::emptied_seed_fails_the_pin
xtask | verifications::database::faction_library_seeds::tests::fn_body_extraction_is_brace_balanced
xtask | verifications::database::faction_library_seeds::tests::hash_stripper_respects_quotes
xtask | verifications::database::faction_library_seeds::tests::hollowed_wave_implementation_cannot_borrow_the_other_paths_loop
xtask | verifications::database::faction_library_seeds::tests::linked_wave_inputs_preserve_universal_newline_reading
xtask | verifications::database::faction_library_seeds::tests::live_entrypoint_reads_both_linked_wave_implementations
xtask | verifications::database::faction_library_seeds::tests::live_inputs_hold
xtask | verifications::database::faction_library_seeds::tests::missing_wave_implementation_fails_closed
xtask | verifications::database::faction_library_seeds::tests::name_in_a_different_statement_is_not_a_seed
xtask | verifications::database::faction_library_seeds::tests::py_repr_matches_cpython
xtask | verifications::database::faction_library_seeds::tests::red_setup_refuses_a_list_it_does_not_recognise
xtask | verifications::database::faction_library_seeds::tests::red_setup_refuses_a_wave_it_does_not_recognise
xtask | verifications::database::faction_library_seeds::tests::sql_stripper_keeps_literals_and_kills_comments
xtask | verifications::database::faction_library_seeds::tests::the_seeder_must_apply_the_file_not_merely_name_it
xtask | verifications::database::sql_deserialization::tests::a_missing_api_tree_does_not_read_as_clean
xtask | verifications::database::sql_deserialization::tests::allowlisted_tables_are_exempt
xtask | verifications::database::sql_deserialization::tests::extracts_the_table_name
xtask | verifications::database::wiki_seeds::tests::a_missing_seed_file_does_not_read_as_pass
xtask | verifications::database::wiki_seeds::tests::a_nonexistent_repo_root_does_not_read_as_pass
xtask | verifications::database::wiki_seeds::tests::a_renamed_wiki_entry_does_not_satisfy_the_pin
xtask | verifications::database::wiki_seeds::tests::a_seed_list_missing_the_wiki_entry_is_caught
xtask | verifications::database::wiki_seeds::tests::a_seed_without_the_v_suite_slug_is_caught
xtask | verifications::database::wiki_seeds::tests::an_empty_seed_file_is_caught_before_the_slug_pin
xtask | verifications::database::wiki_seeds::tests::an_empty_seed_list_does_not_read_as_pass
xtask | verifications::database::wiki_seeds::tests::failure_text_is_pinned
xtask | verifications::database::wiki_seeds::tests::the_live_repo_contract_holds
xtask | verifications::database::wiki_seeds::tests::the_live_seed_list_holds
xtask | verifications::deployment::staging_compose_paths::tests::a_backslash_before_a_closing_single_quote_swallows_the_rest
xtask | verifications::deployment::staging_compose_paths::tests::a_correct_source_holds
xtask | verifications::deployment::staging_compose_paths::tests::a_missing_deploy_source_does_not_read_as_pass
xtask | verifications::deployment::staging_compose_paths::tests::a_transport_facade_cannot_substitute_for_the_compose_implementation
xtask | verifications::deployment::staging_compose_paths::tests::comments_go_and_quoted_hashes_stay
xtask | verifications::deployment::staging_compose_paths::tests::every_source_perturbation_bites
xtask | verifications::deployment::staging_compose_paths::tests::quoted_dash_f_arguments_lose_their_quotes
xtask | verifications::deployment::staging_compose_paths::tests::the_live_deploy_source_holds
xtask | verifications::deployment::staging_compose_paths::tests::the_on_disk_compose_pair_bites
xtask | verifications::language_bans::node_and_file_limits::file_length_tests::civil_ymd_pins_epoch_and_ticket_day
xtask | verifications::language_bans::node_and_file_limits::file_length_tests::empty_walk_is_not_ok
xtask | verifications::language_bans::node_and_file_limits::file_length_tests::mc_perf_only_exempts_size_2
xtask | verifications::language_bans::node_and_file_limits::file_length_tests::missing_walk_root_is_did_not_run
xtask | verifications::language_bans::node_and_file_limits::file_length_tests::orphan_allowlist_rows_fail_even_when_the_walk_is_otherwise_clean
xtask | verifications::language_bans::node_and_file_limits::file_length_tests::production_boundary_is_500_lines
xtask | verifications::language_bans::node_and_file_limits::file_length_tests::size2_without_reason_does_not_skip_size3
xtask | verifications::language_bans::node_and_file_limits::file_length_tests::size3_allowlist_quoted_empty_reason_does_not_exempt
xtask | verifications::language_bans::node_and_file_limits::file_length_tests::size3_allowlist_without_reason_does_not_exempt
xtask | verifications::language_bans::node_and_file_limits::file_length_tests::size3_allowlisted_with_reason_and_expires_holds
xtask | verifications::language_bans::node_and_file_limits::file_length_tests::size3_unallowlisted_fails
xtask | verifications::language_bans::node_and_file_limits::file_length_tests::test_boundary_is_1000_lines_for_directory_and_basename
xtask | verifications::language_bans::node_and_file_limits::file_length_tests::unreadable_file_is_did_not_run
xtask | verifications::language_bans::node_and_file_limits::file_length_tests::walk_is_nonempty_anti_vacuity
xtask | verifications::language_bans::node_and_file_limits::file_length_tests::website_test_roots_are_walked_and_generated_contracts_are_excluded
xtask | verifications::language_bans::python_scripts::tests::clean_fixture_passes
xtask | verifications::language_bans::python_scripts::tests::comment_only_python3_is_not_a_hit
xtask | verifications::language_bans::python_scripts::tests::leftover_py_file_fails
xtask | verifications::language_bans::python_scripts::tests::lowercase_makefile_is_banned
xtask | verifications::language_bans::python_scripts::tests::makefile_is_banned
xtask | verifications::language_bans::python_scripts::tests::new_python3_invocation_fails
xtask | verifications::language_bans::python_scripts::tests::planted_sh_fails
xtask | verifications::language_bans::python_scripts::tests::rust_comment_python3_is_not_a_hit
xtask | verifications::language_bans::python_scripts::tests::shebang_python_counts
xtask | verifications::language_bans::shell_scripts::tests::counts_real_shell_shebangs
xtask | verifications::language_bans::shell_scripts::tests::does_not_sweep_in_other_interpreters_or_prose
xtask | verifications::language_bans::shell_scripts::tests::does_not_sweep_in_rust_inner_attributes
xtask | verifications::language_bans::shell_scripts::tests::python3_command_position_catches_invocations
xtask | verifications::language_bans::shell_scripts::tests::python3_command_position_ignores_comments
xtask | verifications::language_bans::shell_scripts::tests::python_shebang_is_python_not_shell
xtask | verifications::licensing::upstream_code_leaks::tests::an_absent_mod_tree_does_not_read_as_clean
xtask | verifications::licensing::upstream_code_leaks::tests::asset_dirs_descend_through_symlinked_lanes
xtask | verifications::licensing::upstream_code_leaks::tests::identifier_leaks_are_caught_but_comments_and_longer_words_are_not
xtask | verifications::licensing::upstream_code_leaks::tests::reading_and_lane_resolution_match_the_script
xtask | verifications::licensing::upstream_code_leaks::tests::the_hit_list_truncates_and_the_mcp_bridge_is_exempt_from_arm_one
xtask | verifications::licensing::upstream_code_leaks::tests::the_three_no_finding_wordings_stay_distinct
xtask | verifications::licensing::upstream_code_leaks::tests::vanilla_paks_exempt_a_shared_guid_and_only_a_shared_guid
xtask | verifications::mod_scripts::destroy_target_diagnostics::tests::collapsed_returns_fail_registry_pins
xtask | verifications::mod_scripts::destroy_target_diagnostics::tests::live_tree_holds
xtask | verifications::mod_scripts::destroy_target_diagnostics::tests::paraphrase_injection_is_caught
xtask | verifications::mod_scripts::destroy_target_diagnostics::tests::strip_keeps_block_newlines
xtask | verifications::mod_scripts::mission_rest_size_limits::tests::live_shaped_helper_passes_order_and_length
xtask | verifications::mod_scripts::mission_rest_size_limits::tests::return_true_stub_fails
xtask | verifications::mod_scripts::mission_rest_size_limits::tests::strip_c_comments_drops_line_and_block
xtask | verifications::mod_scripts::player_identity_comments::tests::a_clean_source_holds
xtask | verifications::mod_scripts::player_identity_comments::tests::a_missing_target_does_not_read_as_pass
xtask | verifications::mod_scripts::player_identity_comments::tests::every_ban_is_discriminating
xtask | verifications::mod_scripts::player_identity_comments::tests::every_pin_is_discriminating
xtask | verifications::mod_scripts::results_reporter_identity_comments::tests::a_clean_source_holds
xtask | verifications::mod_scripts::results_reporter_identity_comments::tests::a_missing_target_does_not_read_as_pass
xtask | verifications::mod_scripts::results_reporter_identity_comments::tests::an_empty_source_fails_on_pins_not_bans
xtask | verifications::mod_scripts::results_reporter_identity_comments::tests::every_ban_is_discriminating
xtask | verifications::mod_scripts::results_reporter_identity_comments::tests::every_pin_is_discriminating
xtask | verifications::mod_scripts::spawn_determinism::tests::assess_census_mismatch_extracts_audit_only
xtask | verifications::mod_scripts::spawn_determinism::tests::assess_duplicate_binds_uniq_c_padding
xtask | verifications::mod_scripts::spawn_determinism::tests::assess_fallthrough_fails
xtask | verifications::mod_scripts::spawn_determinism::tests::assess_healthy_passes
xtask | verifications::mod_scripts::spawn_determinism::tests::assess_stale_fails
xtask | verifications::mod_scripts::spawn_determinism::tests::extract_drops_mission_load_failure
xtask | verifications::mod_scripts::spawn_determinism::tests::extract_keeps_healthy_mission_line
xtask | verifications::mod_scripts::spawn_verification::tests::default_pattern_matches_bash
xtask | verifications::mod_scripts::ui_layout_parser::tests::a_non_container_parent_needs_no_slot
xtask | verifications::mod_scripts::ui_layout_parser::tests::awk_number_conversion_matches_v_plus_zero
xtask | verifications::mod_scripts::ui_layout_parser::tests::awk_string_conversion_uses_percent_d_for_integers
xtask | verifications::mod_scripts::ui_layout_parser::tests::c1_stray_close_also_reports_the_eof_line
xtask | verifications::mod_scripts::ui_layout_parser::tests::c1_unbalanced_at_eof
xtask | verifications::mod_scripts::ui_layout_parser::tests::c2_unattested_slot_class
xtask | verifications::mod_scripts::ui_layout_parser::tests::c3_geometry_mirror
xtask | verifications::mod_scripts::ui_layout_parser::tests::c4_container_child_with_no_slot
xtask | verifications::mod_scripts::ui_layout_parser::tests::geometry_keyword_needs_a_separator
xtask | verifications::mod_scripts::ui_layout_parser::tests::guid_braces_do_not_desync_the_counter
xtask | verifications::mod_scripts::ui_layout_parser::tests::guid_desync_would_hide_a_c6
xtask | verifications::mod_scripts::ui_layout_parser::tests::multiple_c6_findings_come_out_in_ascending_line_order
xtask | verifications::mod_scripts::ui_layout_parser::tests::records_keep_a_trailing_carriage_return
xtask | verifications::mod_scripts::ui_layout_parser::tests::stale_geometry_leaks_across_slots
xtask | verifications::mod_scripts::ui_layout_parser::tests::widget_decl_predicate_matches_the_bash_regex
xtask | verifications::mod_scripts::ui_layouts::tests::declared_name_extraction_is_line_anchored
xtask | verifications::mod_scripts::ui_layouts::tests::finder_names_reads_every_occurrence_leftmost_longest
xtask | verifications::registry::object_registry_aliases::tests::inputs_that_were_never_examined_do_not_read_as_pass
xtask | verifications::registry::object_registry_aliases::tests::perturbing_the_registry_turns_the_pass_red
xtask | verifications::registry::object_registry_aliases::tests::python_semantics_are_reproduced
xtask | verifications::registry::object_registry_aliases::tests::shape_pins_reject_what_python_rejected
xtask | verifications::registry::object_registry_aliases::tests::the_mirror_matches_the_frontend
xtask | verifications::registry::object_registry_aliases::tests::the_real_registry_holds
xtask | verifications::schemas::checks::citation_scope_tests::empty_corpus_is_a_scope_failure_not_a_pass
xtask | verifications::schemas::checks::citation_scope_tests::missing_scan_root_is_a_scope_failure_not_a_pass
xtask | verifications::schemas::checks::citation_scope_tests::rust_under_apps_and_tooling_is_scanned_and_can_fail
xtask | verifications::schemas::checks::citation_scope_tests::scope_line_is_generated_from_the_constants
xtask | verifications::schemas::checks::instance_kind_lockstep_tests::instance_kinds_match_the_enums_schema_and_the_crate_array
xtask | verifications::schemas::checks::instance_kind_lockstep_tests::instance_kinds_order_is_the_emitted_bykind_order
xtask | verifications::schemas::checks::instance_kind_lockstep_tests::lockstep_reds_when_the_enum_and_the_array_disagree
xtask | verifications::schemas::checks::instance_kind_lockstep_tests::lockstep_reds_when_the_enums_are_unreadable
xtask | verifications::schemas::checks::objective_spine_tests::objective_spine_is_read_in_the_objectives_lane
xtask | verifications::schemas::checks::objective_spine_tests::the_lane_scan_can_still_report_zero
xtask | verifications::schemas::checks::side_fallback_tests::invalid_side_is_neutral_but_absent_and_valid_sides_keep_their_roles
xtask | verifications::schemas::checks::staged_golden_tests::the_staged_1_3_golden_objectives_row_binds_to_the_reader
xtask | verifications::schemas::checks::unread_wire_field_tests::all_1_3_fields_are_unread_on_the_live_tree
xtask | verifications::schemas::checks::unread_wire_field_tests::comments_and_string_literals_do_not_count_as_readers
xtask | verifications::schemas::checks::unread_wire_field_tests::nonzero_baselines_explain_the_pre_existing_identifier
xtask | verifications::schemas::checks::unread_wire_field_tests::stripper_removes_line_block_and_string_bodies
xtask | verifications::schemas::checks::unread_wire_field_tests::unread_gate_fires_when_a_reader_appears
```

## P1 — Agent instructions and tracked root ghosts

Every file that tells an agent what to do now names only commands, files and crates that exist, and the repository root carries no tracked build or checkpoint junk. The command texts for this phase's checks live in the closure plan's P1 block, not here: two of the four search for tokens the verification matrix drives to zero across `tools_v2` documents, and this document is inside that search.

### What changed

| Path | Change |
|---|---|
| `.cursor/rules/tbd-platform.mdc` | Ticket store is the per-ticket TOML files beside the `ROOT` marker; sync, check and run are `cargo xtask ticket` verbs; the absolute home-directory root becomes "the repository root"; the sentence about a status marker block in the root agent document goes, because no such marker exists; the layout line names the two engine crates; the five document links resolve from `.cursor/rules/`; the factory-mode paragraph drops its authorization date |
| `.cursor/rules/application-code-forbidden.mdc` | Application code ships through `cargo xtask ticket run`; the factory exception states the present rule instead of narrating how it came about |
| `.cursor/rules/class-r-plans.mdc` | The brief command is a `cargo xtask` verb reading the ticket's own file; census evidence is `assets_v2/terrains/`; the smoke-evidence pin is the editor smoke-test directory under developer-tools; the engine type authority is `website-map-engine` at `apps/website/map-engine`; research loop R7 is stated as parity with an implementation that lives only in git history, with no ticket identifiers standing in for surfaces |
| `.cursor/rules/claude-prompt-delivery.mdc` | Brief and prompt are `cargo xtask ticket` verbs; the prompt-standard link resolves from `.cursor/rules/`; the worked example uses the same identifier placeholders as the templates it points at |
| `.cursor/rules/cursor-agent-workflow.mdc` | The agent-split table names the ticket files and the `cargo xtask` verbs; the prompt-standard link resolves from `.cursor/rules/`; Mode F states the rule without narrating a budget that ran out; the audit section links the audit document at its real path; the file had the same pre-flight heading twice with two different bodies — they are now one pre-flight section and one thread-flow block |
| `.cursor/rules/acceptance-gates-reproducible.mdc`, `no-duplicate-slice-agents.mdc`, `platform-factory-mode.mdc` | The rules keep their reason and lose the incident narration and the ticket identifiers that carried it |
| `.ai/tickets/AI_PLAYBOOK.md` | Every pipeline verb is `cargo xtask ticket`; the source of truth is the per-ticket TOML files; the never-hand-edit list names the roadmap marker block that exists and drops the one that does not; the lifecycle table describes ticket files rather than rows of a deleted monolith; the ship recipe links the commit checklist at its real path |
| `.ai/tickets/CLAUDE_CODE_PROMPT.md` | Every pipeline verb is `cargo xtask ticket`; the prompt skeleton's language gate becomes a layer gate over the three live crates — engine primitives, map engine, Leptos frontend — replacing a gate written for a TypeScript view layer that is not in the tree; the do-not list names the ticket files |
| `.ai/tickets/SPEC_TEMPLATE.md`, `.ai/tickets/HANDOFF_TEMPLATE.md` | Same verb rewrite; the spec template's verify block runs the frontend check recipe instead of an npm build that has no package; the handoff template loses its closing historical note |
| `.ai/tickets/README.md` | States the live implementation — the `ticket` subcommand backed by the `ticket-engine` crate — and spells every verb as `cargo xtask ticket`; the monolith-cutover paragraph and the "Makefile" block (there is no Makefile, and its last line aliased a command to itself) are gone |
| `.ai/tickets/metrics.schema.json`, `estimates.schema.json`, `schema.json` | Descriptions name the validating modules under `tools_v2/ticket-engine/src/metrics/` and `.../metrics/estimates/`, and the engine crate by its real name; titles and descriptions carry no ticket identifiers |
| `README.md` | The root layout table gains a `tools_v2/` row naming the four crates and the task runner, the `.ai/` row names the ticket files and the wave lock, the website row names the two engine crates, and the closing section about archived upstream repositories is gone |
| `.gitignore` | New rule `node_modules/` under a present-tense comment; the comments on the target directories, the compile baselines, the per-slice target trees and the raster-tool rule state what the rule does instead of which ticket introduced it |
| `docs/platform/PLATFORM_FACTORY.md` | Two state-ownership sentences named the deleted ticket monolith; they name the per-ticket TOML files |
| `node_modules/.vite/vitest/da39a3ee5e6b4b0d3255bfef95601890afd80709/results.json`, `temp.json` | Removed from the index and from disk |

The removal is covered: the whole tracked node tree was that one file, a test-duration cache keyed by TypeScript test paths that no longer exist, and `temp.json` was zero bytes. A repository-wide search for either name outside the tree itself returns no consumer — the only other mentions of the cache's tool are notes inside shipped ticket files, which are records rather than readers.

### Acceptance

| Check | Expected | Actual |
|---|---|---|
| No agent instruction, playbook, schema or root readme invokes the deleted ticket shim or names the deleted ticket monolith (plan P1 acceptance 1, over `.cursor`, the ticket documents and data, and the root readme) | empty | empty, exit 1 |
| `git ls-files node_modules temp.json` | empty | empty, 0 paths |
| `git check-ignore -v node_modules` | names the new rule | `.gitignore:61:node_modules/	node_modules`, exit 0 |
| `cargo xtask ticket check --strict` | OK | `check OK`, exit 0; the ten command-shaped-acceptance warnings it prints are properties of ticket files this phase does not touch and match the baseline |
| No rule file names the deleted smoke module, the deleted engine crate spelling, the deleted asset tree or an absolute home-directory root (plan P1 acceptance 4, over `.cursor/rules`) | empty | empty, exit 1 |

Two further checks, run because this phase's goal is that these files name only things that exist:

| Check | Actual |
|---|---|
| Every path token of the form `dir/file.ext` in `.cursor/rules` and the ticket documents exists on disk | the only misses are template placeholders (`t0xx`, `tXXX`, `path/to/…`, `docs/specs/.../…`) and the search pattern's own truncation of the rule-file extension |
| Every relative markdown link in `.cursor/rules/*.mdc` and `.ai/tickets/*.md` resolves from its own directory | no dangling links |

The plan's row R9 counted 56 lines at the baseline. After this phase it counts 16, all of them in files that later phases own; they are listed below.

### Found and fixed

- `.cursor/rules/cursor-agent-workflow.mdc:87` and `:103` — the same heading twice with two different pre-flight bodies, so which one bound was undefined. Merged into one pre-flight section; the numbered thread flow that sat under the first heading is now its own section.
- `.cursor/rules/cursor-agent-workflow.mdc:117` — the audit document was named without a path and does not sit at the repository root. It now links `docs/platform/CODEBASE_AUDIT_2026.md`.
- `.ai/tickets/AI_PLAYBOOK.md:37` and `.ai/tickets/SPEC_TEMPLATE.md:65` — the link label read `docs/AGENT_COMMIT_CHECKLIST.md`, which does not exist; the link target was already the real `docs/website/AGENT_COMMIT_CHECKLIST.md`. Labels corrected.
- `.ai/tickets/AI_PLAYBOOK.md:9` — the never-hand-edit list named a marker block in the root agent document. No such marker exists anywhere in the tree; the roadmap marker it also named does exist. The list now names only the roadmap block, with its path.
- `.ai/tickets/SPEC_TEMPLATE.md:6` — the authority line linked a file in this directory that does not exist. It now names the ticket's own TOML file.
- `.ai/tickets/SPEC_TEMPLATE.md:54` — the verify block ran an npm build and lint in a `frontend` directory that has no package manifest. It runs the frontend check recipe.
- `.ai/tickets/README.md:38-46` — a "Makefile" section (the repository has no Makefile) whose last line described a command as an alias for itself.
- `.ai/tickets/CLAUDE_CODE_PROMPT.md:53-60,110-123` — the mandatory prompt gate described a React and TypeScript view layer, a state library and a deck oracle, none of which are in the tree, and pointed work at a crate path that does not exist. Rewritten as the layer gate the repository laws actually state, over the three crates that exist.
- `docs/platform/PLATFORM_FACTORY.md:148,453` — the live factory runbook named the deleted ticket monolith as the thing the command center owns. No phase of the plan lists these two lines, and the matrix requires them at zero, so they are fixed here.

### Found for P4

- `.gitignore:10-11` and `:24-25` — the secrets rule and the node-dependency rule still point into the root script tree. They are correct until the files move, so they move with them: rewrite both to the new destinations in the same commit as the move. The new bare `node_modules/` rule added here already covers a node dependency tree at any depth, so the second of those two rules can simply be deleted rather than repointed, and the node dependency directory beside the relocated package manifest needs no rule of its own.
- `CLAUDE.md:212` — the deployment comment names the secrets file at its current path; repoint it with the move.

### Found for P5

- `.github/workflows/ci.yml:103` and `:105` — the step name and the task it runs still carry the old crate spelling, and the step name also carries a ticket identifier. Both change with the task rename; the step name becomes a plain description.

### Found for P9

- `CLAUDE.md:153-160` — the atlas block describes a top-level `tools/` directory that does not exist on disk at all.
- `.github/workflows/ci.yml:78,90,92` — three comments name the old crate spelling; the one at `:90` also carries a ticket identifier and narrates what CI used to miss.
- `apps/ticketboard/Cargo.toml:55` — the comment names a module path that does not exist; the parser lives under `tools_v2/ticket-engine/src/metrics/`.
- `docs/tools/editor_capture.md:7` — narrates a port, carries a ticket identifier, names the old crate spelling and names the deleted smoke module. `:43` names the old crate spelling. The plan's P9 block lists this file at lines 7-8 and 44; the live lines are 7 and 43.
- `docs/platform/token_estimate_factor.md:1` carries a ticket identifier in its title and `:11` names a module path that does not exist; the constant lives under `tools_v2/ticket-engine/src/metrics/estimates/`. The plan's P9 block lists lines 4 and 11; line 4 holds the command and spec link, which are correct, and the title on line 1 is the second line to change.
- `documentation_v2/tools/README.md:3`, `documentation_v2/tools/developer_tools/README.md:3,14`, `documentation_v2/runbooks/testing_and_ci.md:30` — all four name the old crate spelling or a source path under a directory that does not exist; `documentation_v2/tools/README.md:3` also narrates the move.

### Commands that could not run

None. Every check of this phase ran in this environment.

## P2 — Ghost directories, husk directories, ticket-named test files

The tooling tree now holds no empty directory, no directory whose only content is a README
describing files that are not there, and no test file or test directory named after a ticket. Git
tracks no directories, so the empty-directory sweep is local hygiene: a fresh clone never has them,
and the count is restated here as evidence rather than as a durable property of the repository.

### What changed

| Change | Paths |
|---|---|
| Empty directories deleted | 138 directories under `tools_v2/`, all untracked, none holding a tracked file, none named by any `#[path]` attribute |
| Build residue deleted | `tools_v2/ticket-engine/wip/` (trybuild stderr output; the `wip/` rule stays in `tools_v2/ticket-engine/.gitignore` so it is ignored again the moment a stderr mismatch regenerates it) |
| External tool logs deleted | `enfusion_unpacker.log`, `scripts/mod/enfusion_unpacker.log` — output of an unpacker that runs outside this repository; `git grep enfusion_unpacker tools_v2` returns nothing. Their two lines in the local `.git/info/exclude` are gone with them |
| README-only directories removed from the index and disk | `tools_v2/developer-tools/tests/`, `tools_v2/developer-tools/test_fixtures/mcp/`, `tools_v2/developer-tools/src/enfusion_tooling/mcp_node_bridge/`, `tools_v2/verification-core/tests/` — `git ls-files` showed exactly one `README.md` in each before removal |
| Crate constant deleted | `tools_v2/developer-tools/src/lib.rs` — the `PROGRAM` constant carried a ticket identifier as its value and had no reader; the module list is one alphabetical block under a crate doc comment naming the six binaries it serves |
| Test file moved and renamed | one ticket-numbered file under `tools_v2/xtask/src/tests/main/` becomes `tools_v2/xtask/src/commands/mcp/tests/workbench_logs/file_cli_tests.rs`, declared from `commands/mcp/workbench_logs.rs` beside that module's other test file; `tools_v2/xtask/src/tests/main/` is gone and `main.rs` no longer declares it |
| Test files renamed | three ticket-numbered files become `verifications/schemas/tests/checks/objective_spine_tests.rs`, `side_fallback_tests.rs` and `staged_golden_tests.rs`, declared from `verifications/schemas/checks.rs` |
| Test file moved and renamed | `developer-tools/src/map_raster_pipeline/tests/map_labels/map_labels_tests.rs`; `.../tests/satellite_archive_container/container_tests.rs`, with the emptied directory it left behind gone |
| Test directories renamed | `world_export_pipeline/tests/enfusion_texture_decoder/`, `world_export_pipeline/tests/json_number_formatting/`, each named for the module that declares it |
| Documents corrected | `tools_v2/verification-core/README.md` described a `tests/` directory holding a file that never existed; it now describes the layout that exists. The `Test inventory` in this document carries the eleven renamed test paths |

Removing the four README-only directories removes no behaviour: no source file, test, fixture
loader or build script reads any of the four paths. The only other mentions in the tree are three
destination cells in `tools_v2/ANALYSIS_AND_INVENTORY.md`, recorded below.

### Acceptance

| Command | Expected | Actual |
|---|---|---|
| `find tools_v2 -type d -empty \| wc -l` | 0 | 0 |
| `git ls-files tools_v2 \| grep -E '/[^/]*t[0-9]{3}' \| grep -v 'fixtures/'` | empty | empty, exit 1 |
| A `#[path]` grep for the four retired test-directory spellings over `tools_v2` | empty | empty, exit 1 |
| `test ! -e tools_v2/developer-tools/tests && test ! -e tools_v2/verification-core/tests && test ! -e tools_v2/developer-tools/src/enfusion_tooling/mcp_node_bridge && test ! -e tools_v2/developer-tools/test_fixtures/mcp` | exit 0 | exit 0 |
| `grep -c PROGRAM tools_v2/developer-tools/src/lib.rs` | 0 | 0 |
| `cargo test -p xtask -p developer-tools` | green, no live test lost | exit 0; developer-tools 254 passed, 0 failed, 4 ignored; xtask 650 passed, 0 failed — both equal to the baseline |

Test identity, proved mechanically rather than asserted: `cargo test -p xtask -p developer-tools --
--list` reports 908 test paths and the baseline inventory holds 908 for the same two crates.
Comparing the two sorted lists, exactly eleven paths differ on each side, and they pair one to one
by function name — every moved test keeps its `fn` name, and nothing else moved. After updating
those eleven lines, the inventory and the live list are identical with no remaining difference.

Two further checks over the crates this phase edits, both clean: `cargo fmt -p xtask -p
developer-tools --check` and `cargo clippy -p xtask -p developer-tools --all-targets -- -D
warnings`.

### Found and fixed

- `tools_v2/xtask/src/commands/mcp/workbench_logs.rs:1-16,28,37-46,48-55,92,124,179,211,216,237,323,391` — the module doc, the printed usage banner and eight inline comments named a deleted shell script and narrated that script's behaviour. The banner is the command's own `--help` text, so it told every reader to run a file that does not exist; it now spells `cargo xtask mcp wb-logs`. No test pins that text. Two printed failure lines claimed `grep` exited with a status although the command runs no `grep` process; they now say the probe errored. One user-visible line dated a stale build by calendar month and now names what the data shows: flat tags without subsystem tags.
- `tools_v2/xtask/src/commands/mcp/cli.rs:19,26,30,33,43,49,66` — seven `--help` strings, six carrying a ticket identifier and five naming a deleted shell script as the thing the subcommand is a port of. They are the text `cargo xtask mcp --help` prints.
- `tools_v2/xtask/src/verifications/schemas/checks.rs` — the module doc named eight deleted Node scripts; twenty-three comment lines carried ticket identifiers; one doc comment named the crate by a spelling that no longer exists and claimed neither the wave gate nor CI runs its tests, which is no longer true; another named the module path `tools_v2/developer-tools/src/world/INSTANCE_KINDS`, which is nowhere in the workspace — the twin is `developer_tools::world_export_pipeline::INSTANCE_KINDS`; one named a deleted shell script as the discipline it follows. Every invariant, measurement and refusal reason is kept; only the narrative around them is gone.
- `tools_v2/developer-tools/src/map_raster_pipeline/map_labels.rs:1-5` — the header named three deleted Node exporters and resolved elevation through a crate spelling that does not exist; the live path is `website_map_engine::world::environment::locations::peaks`.
- `tools_v2/developer-tools/src/map_raster_pipeline/satellite_archive_container.rs:1,148`, `world_export_pipeline/enfusion_texture_decoder.rs:1-6,74,149`, `world_export_pipeline/json_number_formatting.rs:1-3,60,65` — ticket identifiers and references to deleted Node sources, including a header that told the reader to consult one of them for the container format. The format is now described where the decoder is.
- `tools_v2/xtask/src/verifications/schemas/tests/checks/objective_spine_tests.rs:47` — a ticket identifier inside an assertion message. `side_fallback_tests.rs:140` — a ticket identifier inside the printed banner of the simulated program the test compiles and runs.
- Every moved test file gained a module doc comment saying what it proves, since its file name no longer carries that meaning implicitly.
- `tools_v2/verification-core/README.md` — the document framed the crate as a phase-one implementation with a target architecture still to come, described a `proc/` directory that does not exist, described a `tests/` directory holding `proc_tests.rs` that never existed, and named the crate by a path under a directory that does not exist. It now describes the layout, the four-outcome verdict and the exit contract as they are.
- `.git/info/exclude` — besides the two unpacker log lines, the entry for the zero-byte checkpoint file removed in the previous phase named a path with no writer anywhere in the tree and nothing on disk. All three lines are gone. The file is local to this checkout and is not part of the commit.

### Found for P6

- `tools_v2/verification-core/README.md` §1 — the layout block lists one process module because that is what exists. When it splits into a `proc/` directory of four modules and the seven inline test modules move to `src/tests/`, extend that block with those files; the rest of the document needs no change.

### Found for P8

- `tools_v2/xtask/src/commands/debug/remote_logs/execution.rs:104-105,143` — the hand-synced twin of the workbench-log vocabulary still carries the comment `probe_str is infallible today; keep the bash "did not execute" arm` and prints `grep exited ?` on a path that runs no `grep` process. Replace both with the wording now at `tools_v2/xtask/src/commands/mcp/workbench_logs.rs:178-179,211`, so the two stay hand-synced.
- `tools_v2/xtask/src/commands/mcp/call.rs:29` and `tools_v2/xtask/src/commands/mcp/daemon.rs:30` — each `USAGE` constant tells the user to run a deleted shell script. Replace with `usage: cargo xtask mcp call <tool> '<json-args>'` and `usage: cargo xtask mcp daemon {start|stop|status|restart|stop-all}`; no test pins either string.
- `tools_v2/ANALYSIS_AND_INVENTORY.md:182,184` — the two destination cells name `tools_v2/developer-tools/test_fixtures/mcp/` and `tools_v2/developer-tools/src/enfusion_tooling/mcp_node_bridge/package.json`, both removed in this phase; the settled destinations are `tools_v2/xtask/fixtures/mcp/` and `tools_v2/enfusion_mcp_node_package/`. `:189` repeats the first of the two. `:191` states the repository root retains `tools/` and `packages/`; neither directory exists. These four lines are inside the document this phase rewrites wholesale, and the rewrite has to state the destinations that the move actually used, so they are listed rather than patched ahead of it.
- `tools_v2/developer-tools/src/world_export_pipeline/mod.rs:26-35` — the doc comment for `INSTANCE_KINDS` carries two ticket identifiers, narrates a three-copy history, and names two Rust files that exist nowhere in this crate. The live invariant to keep is that one const holds the census bucket order and that a classified prefab with no bucket is a hard failure rather than a missing row.

### Commands that could not run

None. Every check of this phase ran in this environment.

## P3 — Dead code

This phase deletes code whose only reason to exist was a shell driver that no longer exists or a
one-shot corpus migration that has already run. Every deletion carries a measurement taken on this
checkout, and every capability that had a live caller was relocated before its neighbourhood went.

### The verdict-diff harness

`cargo xtask platform wave diff` compared this driver's stdout, stderr and exit code against a
shell implementation. No shell file is in the checkout (`git ls-files scripts` listed eight
deployment files, five MCP transcripts, two node manifests and two server profiles, and nothing
executable), so every comparison arm refused before comparing. The two internal probes it
carried, `base-probe` and `hold-lock`, were read by the harness's own noise-floor arm and by
nothing else: a grep for the two probe names over `tools_v2` named only the comparison
dispatcher, its noise-floor arm at lines 142 and 358, and the pre-`Ctx::enter` special case in
`flush.rs`.

Deleted, with the line count each carried:

Five files under `tools_v2/xtask/src/commands/platform/wave_execution/` carried it: the dispatcher
(288 lines), the arm module (27), the two arms (450 and 351) and the reclaim comparison (238) —
1,354 lines in total.

No test file declared any of them (the `#[path = "tests/` declarations under `wave_execution` number
fourteen, and none named a comparison module), so no test disappeared with them. The three `pub mod`
lines at `wave_execution/mod.rs`, the `"diff"` dispatch arm and the pre-`Ctx::enter` `base-probe`
special case in `flush.rs`, and the `diff <arm>` spelling in the `platform/cli.rs` argument
documentation went in the same commit. `reclaim` stays: `reclaim::cmd_reclaim` is dispatched from
`flush.rs` and keeps its tests. `UNKNOWN_HELP` names no arm of the harness, so nothing was removed
from it; the one line that matched a `diff` substring search was the English word "different",
reworded so the help text carries no such token.

Three symbols the harness borrowed stay because other callers hold them: `ledger::LFS_NEUTRAL`
(`land/close_ceremony.rs`, `ledger.rs`), `host::status_code` (six callers) and
`base::prev_wave_close` (`gate/gate_dispatch.rs`, `base/demand_base_confirmation.rs`).

### The bash bridge

`cargo xtask deploy db emit-bash-fns` printed bash function definitions for wrappers to `eval`.
`git grep -n 'emit-bash-fns\|emit_bash_fns' -- . ':!tools_v2'` is empty: nothing outside the crate
named it, and the repository admits no shell. Deleted: `verify_dump.rs::emit_bash_fns` (31 lines),
the `EmitBashFns` variant and its documentation line in `database_operations.rs`, the
`pub use verify_dump::emit_bash_fns` re-export, and the dispatch arm in
`database_operations/execution.rs`. No test called it
(`git grep -rn 'emit_bash' tools_v2/xtask/src/commands/deploy/tests` is empty), so the deploy test
count is unchanged. The three comments that described the bridge from a distance
(`database_backup.rs:4,211`, `database_restore_drill.rs:7`) now describe what the code does.

### Tests that compared against a deleted script

Each of these copied a `.sh` file into a scratch tree and returned early when the file was absent.
Since no `.sh` file exists in the checkout, each examined nothing on every run. Every one is
replaced by a test of the live command's own usage text, so the surface each claimed to cover is
still asserted.

| Removed test | Replaced by |
|---|---|
| `commands::fetch::vanilla_api::tests::bash_index_miss_goes_red_first` | the Rust index-miss arm is already pinned by `index_miss_exits_1` |
| `commands::fetch::vanilla_api::tests::bash_from_file_usage_goes_red_first` | `commands::fetch::vanilla_api::tests::from_file_usage_line_names_the_runnable_command` |
| `commands::fetch::vanilla_source::tests::bash_empty_index_goes_red_first` | the Rust empty-index arm is already pinned by `empty_index_map_build_exits_1`; the usage line is pinned by `grep_usage_line_names_the_runnable_command` |
| `commands::mod_ops::development_server::tests::the_missing_launcher_arm_is_discharged_not_deleted` | `commands::mod_ops::development_server::tests::usage_names_the_runnable_playtest_command` |

Two tests keep their assertions under a name that describes them:

| Old name | New name |
|---|---|
| `commands::fetch::vanilla_source::tests::curated_list_len_matches_bash` | `curated_list_holds_nineteen_entries` |
| `commands::platform::slice_worktree::tests::usage_matches_the_bash_header` | `usage_spells_every_subcommand_as_a_runnable_command` |

Making those replacements honest meant the usage strings themselves had to name a runnable
command, so three production strings changed with them: `fetch/vanilla_api.rs` and
`fetch/vanilla_source.rs` print `cargo xtask fetch vanilla-api` / `cargo xtask fetch
vanilla-source` in their usage lines (the constant is now `USAGE_COMMAND`), and
`platform/slice_worktree.rs::USAGE` spells its five subcommands through the live command instead
of reproducing a shell header with a `set -euo pipefail` line and a trailing blank that a `sed`
range once overshot into. `mod_ops/development_server.rs` loses the comment that claimed to retain
an `is_executable` helper (no such function exists in that module), and its usage block and module
documentation now describe the two outcomes the gate actually has.

`git grep -n 'is_executable' tools_v2/xtask/src/commands/mod_ops` still reports nine lines. All
nine are live checks on real executables — the Enfusion compile host (`compile_host.rs` and its two
call sites), the playtest launcher (`playtest_server/usage_fail.rs`) and the world-boot service
token resolver (`world_boot/resolve_service_token.rs` and its two call sites). None is in
`development_server`, which is the module this phase was measuring; deleting the other three
implementations would remove live behaviour, so they stay.

### Finished ticket migrations

Each verb was run on this checkout and the tree inspected afterwards.

| Verb | Output | `git status --porcelain .ai docs` | Verdict |
|---|---|---|---|
| `ticket migrate-v2` | refuses on the first ticket file: `[scope] already carries domain — the tree is v2; migrate-v2 is one-shot` (exit 1) | empty | no-op; deleted |
| `ticket quarantine-walls` | `0 summaries over cap; nothing to do` (exit 0) | empty | no-op; deleted |
| `ticket backfill-stamps` | `0 tickets missing stamps; nothing to do` (exit 0) | empty | no-op; deleted |
| `ticket estimate-tokens` | `0 shipped tickets missing token estimates; nothing to do` (exit 0) | empty | no-op; deleted |
| `ticket migrate-main-goal` | `nothing to write — migration already ran (0 raw user_story carriers, 0 empty fill targets)` (exit 0) | empty | no-op; deleted |

Relocated before the delete, because each has a live caller:

- `SubjectCommit`, `mine_subjects`, `subject_ids` and `to_utc_z` (with the `format_utc_z` helper
  they share) now live in `tools_v2/ticket-engine/src/cli/shipping/commit_subjects.rs`, re-exported
  as `ticket_engine::cli::commit_subjects`. `cli/shipping.rs::cmd_stamp_sha` and
  `metrics/estimates` are the callers.
- `cmd_scope_histogram` is a read-only query and now lives in
  `tools_v2/ticket-engine/src/cli/queries.rs`; the `scope-histogram` verb keeps working and its
  help line describes the census instead of the migration it used to tail.
- The shrink-only debt pins the plan expected inside the quarantine tests are not there:
  `MIGRATION_LEGACY_PIN` lives at `tools_v2/ticket-engine/src/tests/store/mod.rs:90` and
  `TITLE_DEBT_PIN` / `MAIN_GOAL_DEBT_PIN` at `model/scope.rs:128,139`, all read by
  `validation/debt.rs`, all untouched by this phase. Nothing had to move.

Deleted, with the line count each carried:

| Subtree of `tools_v2/ticket-engine/src/maintenance/` | Files | Lines |
|---|---:|---:|
| the module root | 1 | 5 |
| the main-goal migration | 1 | 260 |
| the body quarantine, with its tests | 3 | 368 |
| the scope migration, with its tests | 5 | 644 |
| the timestamp backfill, with its tests | 8 | 1,274 |

With the tree gone, `pub mod maintenance;` leaves `ticket-engine/src/lib.rs`, the five clap variants
leave `xtask/src/commands/ticket/cli.rs`, the five dispatch arms leave
`xtask/src/commands/ticket/dispatch.rs`, and the whole `ticket_engine::maintenance` re-export block
plus the `cmd_estimate_tokens` re-export leave `xtask/src/commands/ticket/mod.rs`.
`metrics/estimates/storage.rs` loses `cmd_estimate_tokens` and the `print_report` it alone called
(56 lines); the module's writeable core `run_estimates` and its planner stay, unchanged and still
tested, as `ticket_engine::metrics::estimates` API.

Five stale module names left `xtask/src/tests/tooling_dependency_boundaries.rs`'s negative list
(`estimate_tokens`, `backfill_stamps`, `migrate_v2`, `migrate_main_goal`, `quarantine_walls`); the
test still asserts that both ticket adapters delegate to `ticket_engine::`.

Four refusal and comment sites told an operator to run a verb this phase deletes, so they were
rewritten to name what actually repairs the ticket: `ops/transitions.rs:139,158` and
`validation/shipping.rs:146,155` now say to hand-stamp a defensible date or to run
`ticket stamp-sha <id> <sha>`. Three further comments describing the quarantine cutover
(`ops/validation.rs`, `tests/store/mod.rs`, `validation/body.rs`) state the invariant without naming
the deleted pass.

### Tests removed, renamed and added

Seventeen `ticket-engine` tests went with the migration tree; four of them assert behaviour that
survives and were re-seated verbatim in
`tools_v2/ticket-engine/src/cli/shipping/tests/commit_subjects_tests.rs`.

| P0 inventory name | Outcome |
|---|---|
| `maintenance::body_quarantine::tests::body_quarantine_tests::parked_ticket_renders_canonically` | removed with the quarantine pass |
| `maintenance::body_quarantine::tests::body_quarantine_tests::quarantine_moves_walls_reversibly_then_second_run_is_empty` | removed with the quarantine pass |
| `maintenance::scope_migration::tests::scope_mapping_tests::chrome_map_is_deterministic_no_marker` | removed with the v1 scope mapper |
| `maintenance::scope_migration::tests::scope_mapping_tests::chromeless_editor_is_owns_inferred_marked` | removed with the v1 scope mapper |
| `maintenance::scope_migration::tests::scope_mapping_tests::editor_owns_inference_dominant_component` | removed with the v1 scope mapper |
| `maintenance::scope_migration::tests::scope_mapping_tests::mod_feature_infers_from_enfusion_segments` | removed with the v1 scope mapper |
| `maintenance::scope_migration::tests::scope_mapping_tests::multi_layer_takes_first` | removed with the v1 scope mapper |
| `maintenance::scope_migration::tests::scope_mapping_tests::repo_xtask_component_prefix_rules` | removed with the v1 scope mapper |
| `maintenance::scope_migration::tests::scope_mapping_tests::unmapped_shapes_refuse_naming_ticket` | removed with the v1 scope mapper |
| `maintenance::timestamp_backfill::tests::timestamp_provenance_tests::method2_descriptions_match_the_derivation` | removed with the stamp interpolator |
| `maintenance::timestamp_backfill::tests::timestamp_provenance_tests::odd_shipped_at_is_untouched_and_reported` | removed with the stamp backfill |
| `maintenance::timestamp_backfill::tests::timestamp_provenance_tests::scratch_backfill_mines_interpolates_and_is_idempotent` | removed with the stamp backfill |
| `maintenance::timestamp_backfill::tests::timestamp_provenance_tests::stray_date_shaped_shipped_at_resolves` | removed with the stamp backfill |
| `maintenance::timestamp_backfill::tests::timestamp_provenance_tests::mine_subjects_live_repo_smoke` | re-seated at `cli::shipping::commit_subjects::tests::mine_subjects_live_repo_smoke` |
| `maintenance::timestamp_backfill::tests::timestamp_provenance_tests::subject_id_boundary_pins` | re-seated at `cli::shipping::commit_subjects::tests::subject_id_boundary_pins` |
| `maintenance::timestamp_backfill::tests::timestamp_provenance_tests::utc_normalization` | re-seated at `cli::shipping::commit_subjects::tests::utc_normalization`, minus the two day-floor assertions whose subject went with the interpolator |
| `maintenance::timestamp_backfill::tests::timestamp_provenance_tests::shape_predicates` | re-seated at `cli::shipping::commit_subjects::tests::shape_predicates`, minus the date-shape assertions whose predicate went with the backfill |

`ticket-engine` therefore runs 200 library tests plus the compile-failure test, against a P0
baseline of 213 plus one: 213 − 17 + 4 = 200. `xtask` runs 649 against a baseline of 650:
650 − 4 removed + 3 added = 649, with two renames carrying their assertions across.

### Acceptance

| Command | Expected | Actual |
|---|---|---|
| `cargo xtask platform wave --help 2>&1 \| grep -c diff` | 0 | 0 |
| `git grep -n '"diff"' tools_v2/xtask/src/commands/platform` | empty | five lines, all the `git` subcommand inside `git_stdout_lossy(&["diff", "--name-only", …])` (`base/demand_base_confirmation.rs:183`, `changed/changed_rs.rs:30,222,311`, `gate/gate_dispatch.rs:329`); zero are the wave verb, whose dispatch arm is gone |
| `git grep -n 'diff_arms\|diff_reclaim\|cmd_diff' tools_v2` | empty | six lines, every one of them inside this document — the deletion table above and this row; adding `':!tools_v2/PHASE_FIVE_HANDOFF.md'`, or scoping to `'tools_v2/**/*.rs'`, is empty (exit 1) |
| `cargo xtask ticket --help 2>&1 \| grep -c -E 'migrate-v2\|quarantine-walls\|backfill-stamps\|migrate-main-goal'` | 0 | 0 |
| `cargo xtask ticket --help 2>&1 \| grep -c estimate-tokens` | 0 (its proof was empty) | 0 |
| `cargo xtask deploy db --help 2>&1 \| grep -c emit-bash` | 0 | 0 |
| `git grep -n -E 'exists\(\) *\{ *return' -- 'tools_v2/**/tests/*.rs' 'tools_v2/**/tests.rs'` | empty | empty (exit 1) |
| `git grep -n 'is_executable' tools_v2/xtask/src/commands/mod_ops` | empty | nine lines, all live executable checks in `compile`, `compile_host`, `playtest_server` and `world_boot`; zero in `development_server` |
| `cargo test -p xtask -p ticket-engine` | green | exit 0; xtask 649 passed, ticket-engine 200 passed plus 1 compile-failure test, 0 failed, 0 ignored |
| `cargo clippy -p xtask -p ticket-engine --all-targets -- -D warnings` | clean | exit 0, no output |
| `git status --porcelain .ai docs` | empty | empty |

Three of those rows do not reach the value the plan predicted. Two cannot without deleting live
code: `"diff"` is how every `git diff --name-only` call in the wave driver spells its subcommand,
and `is_executable` is how four unrelated modules ask whether a binary on disk can be run. Both
subjects the phase was aimed at — the wave `diff` verb and a vestigial helper in
`development_server` — are gone, and the rows above record the measured remainder. The third is
this document: the bulk-delete rule requires the deleted file names to be written down, so the
record of a deletion is what the search now finds. Sources carry none of the three names.

Also run: `cargo check --workspace --locked --all-targets` exits 0 (`ticketboard` consumes
`ticket_engine`, so the engine's changed surface is proved across the workspace);
`cargo xtask ticket check --strict` exits 0 with `check OK`, `TITLE_DEBT_PIN 0 == measured 0` and
`MAIN_GOAL_DEBT_PIN 0 == measured 0`; `cargo xtask ticket sync` run twice leaves the second run with
nothing further to write.

### Found and fixed

- `tools_v2/ticket-engine/src/ops/transitions.rs:139,158` and
  `tools_v2/ticket-engine/src/validation/shipping.rs:146,155` — four live refusal messages told the
  operator to run `ticket backfill-stamps`, the verb this phase deletes. Rewritten to name the
  repair that exists: a deliberate hand-stamp, or `ticket stamp-sha <id> <sha>`.
- `tools_v2/ticket-engine/src/ops/validation.rs:48`,
  `tools_v2/ticket-engine/src/tests/store/mod.rs:75`,
  `tools_v2/ticket-engine/src/validation/body.rs:5` — three comments explained a live invariant by
  naming the one-shot quarantine verb. Each now states the invariant on its own terms.
- `tools_v2/ticket-engine/src/ops/tests/status_and_shipping_tests.rs:184` — the assertion behind
  `ship_refuses_created_at_less_pre_write` pinned the refusal's fix clause by the word `backfill`.
  It now pins `hand-stamp`, the fix the message names, so the test still proves the refusal tells
  the operator what to do rather than merely that it refused.
- `tools_v2/xtask/src/commands/mod_ops/development_server.rs:65` — a comment claimed the module
  retained `is_executable` "only for the tests that still pin the old shape"; the module contains no
  such function. Comment removed with the tests it referred to.
- `tools_v2/xtask/src/commands/mod_ops/development_server.rs:8-11` — the module documented an exit
  code 3 for a missing launcher, an outcome the module cannot produce. The documented codes are now
  the two it has.
- `tools_v2/xtask/src/commands/mod_ops/cli.rs:50` — the `dev-server` help line described the verb as
  a shim to a deleted `.sh` file; it now says what the verb does.
- `tools_v2/xtask/src/commands/platform/wave_execution/flush.rs:77` — a doc comment justified an
  empty-string return "because several refusal messages are asserted byte-for-byte by the diff
  harness". The harness is deleted; the comment now states the invariant the empty string serves.
- `tools_v2/xtask/src/commands/ci/tests/task_runner.rs:12` — the module comment spelled a skip guard
  in code form while asserting that no test uses one, which made a mechanical search for such guards
  report the sentence that forbids them. Same statement, prose form.
- `docs/platform/token_estimate_factor.md:4,11` — the document pointed at
  `cargo xtask ticket estimate-tokens` and at a deleted xtask module. The first is deleted
  here and the second has not existed since the engine split; both now name the live generator
  (`ticket stamp-sha`) and the live constant
  (`tools_v2/ticket-engine/src/metrics/estimates/model.rs::TOKENS_PER_LOC`). The test that pins the
  factor value in this document still passes.
- `docs/TICKET_REGISTRY.md:820` — the generated view carried a program title that the ticket file has
  not held since it was minted; `cargo xtask ticket sync` regenerates the line, and the refreshed
  view lands with this commit so the generated surface and the corpus agree again.
- `tools_v2/xtask/src/commands/fetch/tests/vanilla_api/tests.rs` and `…/vanilla_source/tests.rs` —
  scratch directories were named after ticket identifiers; they are now named after the command
  under test.

### Found for P4

- `tools_v2/xtask/src/commands/fetch/vanilla_api.rs:34-35` and
  `tools_v2/xtask/src/commands/fetch/vanilla_source.rs:31-32` — the consumer list for these two
  lines is already discharged: the constant is `USAGE_COMMAND` and prints the live `cargo xtask
  fetch …` spelling. Only the module doc comment on line 1 of each file still names the old script.
- `tools_v2/xtask/src/commands/mod_ops/development_server.rs:30-37` — likewise discharged; the usage
  block names `cargo xtask mod playtest`.
- `tools_v2/xtask/src/commands/platform/slice_worktree.rs:67-71` — likewise discharged; `USAGE`
  spells the five subcommands through `cargo xtask platform slice-worktree --`.

### Found for P8

- `tools_v2/ticket-engine/src/metrics/estimates/storage.rs:50` (`run_estimates`) and
  `tools_v2/ticket-engine/src/metrics/estimates/planning.rs:26` (`plan_estimates`, with
  `EstimateReport`) have no in-tree caller once the `estimate-tokens` verb is gone. They remain
  public `ticket_engine::metrics::estimates` API and are exercised by
  `scratch_generator_cohorts_fallthrough_and_idempotence`,
  `mutual_exclusion_and_marker_coherence` and
  `summarize_by_agent_on_mixed_tree_equals_receipts_only`. If the prose-and-dead-code rules require
  every public engine item to have an in-tree caller, the edit is: delete `planning.rs` and
  `storage.rs::run_estimates`, then re-seat those three tests on `plan_estimate_for_id` plus
  `write_estimate_file` (the incremental path `ticket stamp-sha` uses), which reaches the same
  cohort widening, marker coherence and mutual-exclusion behaviour one ticket at a time.
- `tools_v2/xtask/src/commands/platform/wave_execution/mod.rs:84-87` — the doc comment above
  `UNKNOWN_HELP` describes the constant as a `sed` range over a script "deleted at the end of this
  port". Replace with what it is: the help text an unknown wave subcommand prints.
- `tools_v2/xtask/src/commands/platform/wave_execution/mod.rs:88-135` — `UNKNOWN_HELP` itself is
  operator-facing text that still names three deleted shell and Python drivers, and dates its three
  corrections against a past program. The command list in its tail is accurate and must survive the
  rewrite.
- `tools_v2/xtask/src/commands/platform/slice_worktree.rs:44-51` — the `PROG` doc comment explains
  the constant by narrating a deleted script. The invariant to keep is that usage and every guard
  refusal name one command, through this constant.

### Commands that could not run

None. Every check of this phase ran in this environment.

## P4 — scripts/ elimination

Git tracks no `scripts/` directory. Every file it held now sits beside the command that reads it,
every consumer resolves that location through one named constant, and the three duplicated
`enfusion-mcp` resolvers are one function whose installed-module path is spelled exactly once in
the workspace.

### Destinations

| From | To | Who reads it |
|---|---|---|
| `scripts/deploy/deploy.env.example` | `tools_v2/xtask/deploy/deploy.env.example` | An operator copies it to `deploy.env` beside it |
| `scripts/deploy/Caddyfile.website` | `tools_v2/xtask/deploy/Caddyfile.website` | Caddy on the server; `cargo xtask deploy website` prints its reload command; `apps/website/api_v2/tests/forwarded_for_trust.rs` pins its loopback upstream |
| `scripts/deploy/tbd-website-api.service` | `tools_v2/xtask/deploy/systemd/tbd-website-api.service` | `cargo xtask deploy website` renders and restarts it |
| `scripts/deploy/tbd-reforger.service` | `tools_v2/xtask/deploy/systemd/tbd-reforger.service` | `cargo xtask deploy staging` installs it |
| `scripts/deploy/tbd-website-backup.service` and `.timer` | `tools_v2/xtask/deploy/systemd/` | An operator installs them; the service runs `cargo xtask deploy db backup` |
| `scripts/deploy/tbd-website-backup-drill.service` and `.timer` | `tools_v2/xtask/deploy/systemd/` | An operator installs them; the service runs `cargo xtask deploy db drill` |
| `scripts/mod/tbd-dev-server.config.json` | `tools_v2/xtask/dedicated_server_profiles/tbd-dev-server.config.json` | `cargo xtask mod playtest` and `cargo xtask mod world-boot` |
| `scripts/mod/fixtures/mcp-*.jsonl` (5) | `tools_v2/xtask/fixtures/mcp/` | `cargo xtask mcp selftest` replays them through `cargo xtask mcp consume` |
| `scripts/mod/package.json`, `package-lock.json`, root `.nvmrc` | `tools_v2/enfusion_mcp_node_package/` | `npm ci` there installs the pinned `enfusion-mcp` server |
| `scripts/mod/tbd-staging-server.config.json` | deleted | No code consumer: `git grep -n tbd-staging-server` named one comment, and `cargo xtask deploy staging` renders `server.config.json` itself in `deploy/staging/render.rs`. That comment is gone with it. |

The npm package sits outside every crate root on purpose. `FILE_LENGTH_PINS` pins whole crate
directories and `cargo xtask verify file-length` walks them in full, so a vendored `.rs` inside an
installed dependency tree would become a subject of the size gate.

### Operator step, before deleting the last directory on disk

`scripts/mod/node_modules/` is left in place, untracked and gitignored, because the running Claude
Code and Cursor sessions still execute `enfusion-mcp` from it. The three machine-local MCP configs
(`.mcp.json`, `.cursor/mcp.json`, `apps/mod/.cursor/mcp.json`, all gitignored) now point at the
server module installed under
`/run/media/system/Disk_2/Projects/TBD-Reforger/tools_v2/enfusion_mcp_node_package/node_modules/enfusion-mcp/`,
which is installed and verified present. So:

1. Restart Claude Code and Cursor, so each connects to the server at the new path.
2. `rm -rf scripts`

### One entrypoint resolver

`tools_v2/developer-tools/src/enfusion_tooling/enfusion_mcp_entrypoint.rs` holds
`resolve(repo_root) -> EnfusionMcpCommand` and `process_pattern()`. The tiers are unchanged and
now named rather than numbered: an `ENFUSION_MCP_BIN` that exists, the module installed by
`npm ci`, a copy in npm's download cache, then `npx -y enfusion-mcp`. The three resolvers it
replaces are:

- `tools_v2/xtask/src/commands/mcp/call.rs` — `resolve_runner` and its `scripts_mod()` root,
- `tools_v2/xtask/src/commands/mcp/daemon.rs` — `resolve_bin`, its cache walk and its `pkill`
  pattern, which is now `process_pattern()` derived from the same constant,
- `tools_v2/developer-tools/src/enfusion_tooling/mcp_broker.rs` — `resolve_runner`.

The daemon's cache tier now sorts its hits, as the call path already did, so two machines with the
same cache resolve the same file. The broker gains the cache tier it lacked; it is the tier the
daemon would have handed it through `ENFUSION_MCP_BIN` anyway.

### Single sources of truth

`tools_v2/xtask/src/core/repository_layout.rs` (new, declared from `core/mod.rs`): `DEPLOY_DIR`,
`DEPLOY_ENV`, `DEPLOY_ENV_EXAMPLE`, `CADDYFILE`, `SYSTEMD_UNITS_DIR`, `WEBSITE_API_UNIT`,
`DEDICATED_SERVER_PROFILES_DIR`, `DEV_SERVER_PROFILE`, `MCP_TRANSCRIPT_FIXTURES_DIR`. Its tests
(`tools_v2/xtask/src/tests/repository_layout_tests.rs`) assert every committed location exists in
a checkout, that the secrets file is the example minus `.example`, and that each file constant
lies inside the directory constant describing its kind.

`tools_v2/developer-tools/src/repository_layout.rs` gains `ENFUSION_MCP_NODE_PACKAGE_DIR`,
`ENFUSION_MCP_ENTRYPOINT` and the two functions that resolve them against a root. A test asserts
the entrypoint lies under the package directory, so a half-landed relocation cannot leave `npm ci`
installing where no runner looks.

Two derivations removed a duplicated literal each:
`deploy/website/systemd_unit.rs::default_unit_name()` derives the default unit name from
`WEBSITE_API_UNIT`'s file name, and `template_for(unit)` derives a unit's template from
`SYSTEMD_UNITS_DIR` plus that unit's own name — so a deploy pointed at another unit through
`TBD_WEBSITE_SYSTEMD_UNIT` now prints that unit's template rather than the API's.

### What changed, by area

- xtask commands: `deploy/website.rs` (+ new `deploy/website/help_text.rs`, `website/systemd_unit.rs`,
  `website/rsync_argv.rs`), `deploy/staging.rs`, `deploy/staging/{config,remote/ssh_argv,boot/read_addon_guid}.rs`,
  `debug/{cli,probes,direct_join}.rs`, `debug/remote_logs.rs` + `remote_logs/execution.rs`,
  `setup/staging_server.rs`, `mcp/{call,daemon,smoke,call_selftest}.rs`,
  `mod_ops/{development_bootstrap,playtest_server,compile/execution,world_boot/execution}.rs`,
  `mod_ops/playtest_server/usage_fail.rs`, `fetch/{vanilla_api,vanilla_source}.rs`,
  `db/milestone_announcement.rs`, `platform/cli.rs`, `platform/wave_execution/mod.rs`,
  `verifications/language_bans/node_and_file_limits.rs` + `verify_no_node.rs`,
  `commands/{ci/task_definitions,verify/cli}.rs`.
- xtask tests: `tests/{repository_root_tests,repository_layout_tests}.rs`,
  `deploy/tests/{website,staging}/tests.rs`, `deploy/staging/tests/remote/tests.rs`,
  `debug/tests/direct_join/tests.rs`, `setup/tests/staging_server/tests.rs`,
  `db/tests/milestone_announcement/tests.rs`, `mcp/tests/{smoke,call_selftest}/tests.rs`,
  `mod_ops/tests/{wave_execution,playtest_server/tests}.rs`,
  `verifications/language_bans/tests/python_scripts/tests.rs`.
- developer-tools: `enfusion_tooling/{mod,cli,mcp_broker,source}.rs`, the new
  `enfusion_tooling/enfusion_mcp_entrypoint.rs` and its tests, `repository_layout.rs` and its tests.
- verification-core: `src/verdict.rs` (one synthetic test subject renamed off a deleted-script shape).
- apps: `apps/website/api_v2/{tests/forwarded_for_trust.rs,.env.example,PHASE_1_HANDOFF.md}`,
  `api_v2/src/core/{configuration/mod,middleware/durable_ratelimit,observability/health_probe}.rs`,
  `apps/website/{Dockerfile,docker-compose.staging.yml}`,
  `apps/website/map-engine/src/data/scenario/compiler/flatten/tests/cases_3.rs`,
  `apps/mod/{.gitignore,README.md}`, `apps/mod/tbd-emcp/README.md`,
  `apps/mod/tbd-framework/README.md`, and five EnfScript comments under
  `apps/mod/tbd-framework/Scripts/Game/TBD/` (pure ASCII edits).
- root and configuration: `.gitignore`, `.mcp.json`, `.cursor/mcp.json`, `apps/mod/.cursor/mcp.json`,
  `.world-boot-warning-baseline`, `CLAUDE.md`, `.github/workflows/{ci,mod-gates}.yml`.
- documents: `tools_v2/{README.md,xtask/deploy/README.md}`, `assets_v2/MIGRATION_HANDOFF.md`,
  `documentation_v2/{ANALYSIS_AND_INVENTORY.md,runbooks/deployment.md}`,
  `docs/website/{HOME_SERVER,DEV_RUNBOOK}.md`,
  `docs/platform/{PLAYTEST_RUNBOOK,PLATFORM_FACTORY,FACTORY_FOR_CURSOR,EDITOR_FACTORY_FOR_CURSOR,EDITOR_FACTORY_START,CODING_STANDARDS,MONOREPO_MIGRATION,tbd_north_star_backlog}.md`,
  `docs/mod/{STAGING-SERVER,MCP_TOOLING,CLAUDE-CODE-START,SPAWN_DETERMINISM,SLICE_WORKFLOW,vanilla_carve_coverage,MILESTONES}.md`.

### Acceptance

| Command | Result |
|---|---|
| `git ls-files scripts \| wc -l` | 0 |
| `find scripts -type f -not -path '*/node_modules/*' \| wc -l` | 0 |
| R5a grep (comment lines and `tools_v2/*.md` excluded) | 0 lines |
| `git check-ignore -v tools_v2/xtask/deploy/deploy.env` | `.gitignore:11:tools_v2/xtask/deploy/deploy.env` |
| `git check-ignore -v tools_v2/enfusion_mcp_node_package/node_modules` | `.gitignore:58:node_modules/` |
| the three MCP configs resolve to an existing entrypoint | three `ok` lines |
| `git grep -c -F` over the installed-module path, scoped to `tools_v2` | 1 — `tools_v2/developer-tools/src/repository_layout.rs`, the only file in the workspace that spells it. This document deliberately does not, so the count stays exact. |
| layout-literal grep over `tools_v2 apps` | 4 lines, each accounted for below |
| `cargo check --workspace --locked` | exit 0 |
| `cargo test -p xtask -p developer-tools -p verification-core` | exit 0 — xtask 652 passed, developer-tools 259 passed (4 ignored), verification-core 68 passed; 0 failed |
| `cargo test -p website-api --test forwarded_for_trust` | exit 0 — 8 passed, 0 failed |
| `cargo xtask deploy website --dry-run` | exit 0 with `DEPLOY_ENV` pointed at a scratch file (no `deploy.env` exists on this machine). Prints `==> unit: tools_v2/xtask/deploy/systemd/tbd-website-api.service is installed by hand`, `--exclude=tools_v2/xtask/deploy/deploy.env`, and the Caddy reload against `tools_v2/xtask/deploy/Caddyfile.website` |
| `cargo xtask mcp selftest` | `mcp-call-selftest: ALL PASS (20)`, exit 0 |
| `cargo xtask verify no-node` | exit 0 — `OK (none)` for each of the three checks; the walked root is `.github` |
| `cargo xtask verify file-length` | exit 0 — scanned 2513 `.rs` files, 0 violations. The baseline was 2529 at P0 and 2508 after P3's deletions; P4 adds exactly the five `.rs` files listed above and deletes none |
| `cargo xtask mod world-boot --selftest` | `SELFTEST OK`, exit 0; every negative fixture still rejected |
| `cargo xtask mod compile` | exit 0 — `OK: compiled clean`, 5804 files, 11643 classes, 0 warnings in TBD sources |

The four lines the layout-literal grep still reports are not repository-path duplicates:

- `apps/website/api_v2/tests/forwarded_for_trust.rs:398` — the Caddyfile path, spelled once in that
  test and used by both its `include_str!` neighbour and its failure message. `website-api` cannot
  depend on xtask, so this is the one place outside the two layout modules that may name it.
- `tools_v2/xtask/src/tests/repository_root_tests.rs:15` — the fixture the repository-root walk is
  pinned against, which is by definition a path and not a constant.
- `tools_v2/developer-tools/src/enfusion_tooling/cli.rs:69` — the Enfusion pak's own internal
  `scripts/` prefix, where every vanilla `.c` lives inside the archive. Not a checkout path.
- `tools_v2/xtask/src/commands/mod_ops/world_boot_verdict.rs:389` — a captured Enfusion server log
  line; `scripts/game/tbd/…` is the engine's VFS path, and editing it would falsify the fixture.

### Found and fixed

- `tools_v2/xtask/src/commands/mod_ops/compile/execution.rs:189` — the comment named the deleted
  staging server profile, then narrated a 2026-09-12 addon split and carried a ticket identifier.
  Rewritten as the invariant it protects: the gate compiles `TBD_Framework` because that is what a
  dedicated server loads, and tbd-export's road exporter is what could be added.
- `tools_v2/enfusion_mcp_node_package/package.json` — the package was named `tbd-mod-scripts`,
  after a directory that no longer exists, and its description named an internal tier number.
  Renamed to `enfusion-mcp-node-package`, with a description saying what the pin buys. The
  lockfile's two `name` fields follow, so `npm ci` stays in sync.
- `tools_v2/xtask/src/commands/deploy/website.rs` — the required-variable error printed a shell
  script path and line as its prefix. It now names the deploy file and the line to edit.
  `tools_v2/xtask/src/commands/deploy/staging/config.rs:190` carried the same prefix and is fixed
  the same way.
- `tools_v2/xtask/src/commands/deploy/website/rsync_argv.rs:30-38` — the exclusion comments dated
  an asset relocation and called the old tree "pre-relocation". They now say what each exclusion
  protects.
- `tools_v2/xtask/src/commands/deploy/website.rs` was 505 lines after the help text became a
  function. Split: the `--help` block is now `deploy/website/help_text.rs` (43 lines), leaving
  `website.rs` at 468.
- `tools_v2/xtask/src/commands/deploy/website/systemd_unit.rs` rendered the API unit's template
  regardless of which unit `TBD_WEBSITE_SYSTEMD_UNIT` named. `template_for(unit)` fixes that, and
  `default_unit_name()` removes the duplicated `tbd-website-api.service` literal.
- `tools_v2/xtask/src/commands/debug/probes.rs:74,159` — the emitted JSON `"location"` named a
  deleted shell script. It now names `cargo xtask debug direct-join`, the command that writes the
  row. `debug/cli.rs:45` and `debug/probes.rs:4` named it too.
- `tools_v2/xtask/src/verifications/language_bans/tests/python_scripts/tests.rs` — the synthetic
  offenders were planted under a directory the repository does not have, and one was named after a
  ticket. They now sit under `tooling/`, and the planted shell file carries a neutral name.
- `tools_v2/verification-core/src/verdict.rs:243-253` — the missing-target test used a deleted
  shell helper as its subject and named another in its comment. The subject is now
  `etc/socket.conf` and the test is named for the behaviour it pins.
- `tools_v2/xtask/src/commands/platform/wave_execution/mod.rs` — `UNKNOWN_HELP` and the doc comment
  above it (the P3 handoff's two `Found for P8` entries) named three deleted drivers and dated
  their corrections against a past program. Rewritten as the three decisions the lifecycle rests
  on, with the command list intact. `COLLIDE`'s doc comment named a deleted Python file.
- `tools_v2/xtask/src/commands/platform/cli.rs:6,12,18-21` — four clap doc comments, which are
  operator-facing `--help` text, named deleted scripts.
- `tools_v2/xtask/src/verifications/language_bans/node_and_file_limits.rs:49` — `SCAN_DIRS` is
  `[".github"]`. A declared-but-absent root fails closed, so leaving `scripts` there would have
  turned `cargo xtask verify no-node` red the moment the directory went. The node allowlist is now
  empty with a comment saying why: the only place that invokes node is the Rust entrypoint
  resolver, which this gate does not walk.
- `apps/website/api_v2/src/core/observability/health_probe.rs:71,79` — named a deleted preflight
  script as a `/healthz` caller. The caller is `cargo xtask platform preflight`.
- Present-tense rewrites of the module documentation in every xtask module this phase touched:
  each one opened by naming a deleted shell script as its origin, and several carried a
  "preserved oddity" list written against that script rather than against the behaviour. The
  invariants are kept; the comparisons to a program that no longer exists are gone.
- The moved deployment files carried their own history: `deploy.env.example` listed a deleted
  staging script and a ticket identifier, `tbd-website-backup.service` explained its absolute
  placeholder by narrating an earlier shape, and four files named ticket identifiers in their first
  line. All rewritten to describe what they are.

### Found for P5

- `tools_v2/xtask/src/commands/deploy/staging/agent.rs:77` — the rendered agent file's header names
  a deleted staging script as its renderer and carries a ticket identifier. The renderer is
  `cargo xtask deploy staging`. `:94` and `:104` name the same deleted script in the rendered
  body. The R5a grep does not see these: the lines are inside a raw string and begin with `#`.
- `tools_v2/xtask/src/commands/deploy/staging.rs:142-144` — `USAGE` is repointed, but the rest of
  the staging tree still names the deleted driver in comments (`staging/pycompat.rs:6`,
  `staging/config.rs:185`, `verifications/deployment/staging_compose_paths*`), and
  `verifications/deployment/staging_compose_paths.rs` still describes itself as gating a shell
  script that no longer exists. Its subject today is `apps/website/docker-compose.staging.yml`
  plus the Rust render path.

### Found for P8

- `tools_v2/xtask/src/commands/mod_ops/wave_execution.rs:36-56` — `UNKNOWN_HELP` for the mod wave
  driver is described as "the historical bash header, retargeted at the lock". It names no deleted
  script, so it did not block this phase, but the doc comment is history.
- `tools_v2/xtask/src/commands/deploy/cli.rs:11,14` and `deploy/database_*.rs:1` — six clap and
  module doc comments still open with "port of scripts/deploy/…". They are comment lines, so the
  R5a grep excludes them; R3 will not.
- `tools_v2/xtask/src/commands/setup/{client_addons,mcp_game_root,server_profile,workbench_linux}.rs`
  — four module doc comments open by naming a deleted setup script, and three of them instruct the
  reader to preserve a path-pin shell file that is not in the repository.

### Commands that could not run

`cargo xtask deploy website --dry-run` cannot read a real `deploy.env`: the file holds host credentials and exists on no development machine in this repository (the preflight check `test ! -e scripts/deploy/deploy.env` confirmed that before the move). The dry-run was therefore driven through the command's own `DEPLOY_ENV` override against a three-line scratch file in the session scratchpad, which exercises the same code path and the same printed paths. Without it the command exits 1 with `Missing /run/media/system/Disk_2/Projects/TBD-Reforger/tools_v2/xtask/deploy/deploy.env — copy from tools_v2/xtask/deploy/deploy.env.example`, which is itself evidence that the relocated path is the one the command reads.

Every other check of this phase ran unmodified in this environment.

## P5 — Public-surface renames

Every command spelling, task name, gate step label, printed gate name, entry function, fixture
directory, analysis artifact and emitted metadata field now carries the name of what it checks.
`cargo xtask verify <name>` is spelled after the module that implements it — the module file name
with underscores written as hyphens — and its entry function is `verify_` plus that module name.

The retired spellings are not reproduced in this document. This phase's acceptance filters and the
closure plan's verification matrix both search `tools_v2`, this file included, so writing a retired
ticket number, dead crate name, old lock path or old constant name here would put those rows above
zero by itself. Each row below therefore names the domain the check serves, and every retired
spelling of it is recoverable from this commit's diff.

### Verifications: command, module and entry function

| What it checks | Command | Module and entry function |
|---|---|---|
| ORBAT and Eden placement coherency | `verify editor-orbat-coherency` | `verifications/architecture/editor_orbat_coherency.rs` · `verify_editor_orbat_coherency` |
| The results reporter's identity-link comment contract | `verify results-reporter-identity-comments` | `verifications/mod_scripts/results_reporter_identity_comments.rs` · `verify_results_reporter_identity_comments` |
| Destroy-target inert diagnostics | `verify destroy-target-diagnostics` | `verifications/mod_scripts/destroy_target_diagnostics.rs` · `verify_destroy_target_diagnostics` |
| The staging deploy's compose path | `verify staging-compose-paths` | `verifications/deployment/staging_compose_paths.rs` · `verify_staging_compose_paths` |
| Objects-palette alias to spawn-registry census | `verify object-registry-aliases` | `verifications/registry/object_registry_aliases.rs` · `verify_object_registry_aliases` |
| The faction-library seed reaching the database | `verify faction-library-seeds` | `verifications/database/faction_library_seeds.rs` · `verify_faction_library_seeds` |
| The wiki seed reaching the database | `verify wiki-seeds` | `verifications/database/wiki_seeds.rs` · `verify_wiki_seeds` |
| The player identity comment contract | `verify player-identity-comments` | `verifications/mod_scripts/player_identity_comments.rs` · `verify_player_identity_comments` |
| The mission REST body size gate | `verify mission-rest-size-limits` | `verifications/mod_scripts/mission_rest_size_limits.rs` · `verify_mission_rest_size_limits` |
| CI schema parity and the hollow-task tripwire | `verify ci-schema-parity` | `verifications/ci/schema_parity.rs` · `verify_ci_schema_parity` |
| Specification-corpus consistency, gates 1-12 | `schema specification-consistency` | `verifications/schemas/checks/specification_consistency.rs` · `specification_consistency` |
| The program-wide cartographic aggregator | `map verify-cartographic` | `map_raster_pipeline/cli.rs` `Cmd::VerifyCartographic` · `verify_cartographic` |

The clap variants moved with the spellings: `VerifyCmd::{PlayerIdentityComments,
ResultsReporterIdentityComments, ObjectRegistryAliases, WikiSeeds, EditorOrbatCoherency,
DestroyTargetDiagnostics, StagingComposePaths, FactionLibrarySeeds, MissionRestSizeLimits,
CiSchemaParity}` and `SchemaCmd::SpecificationConsistency`. Each verification's printed PASS/FAIL
headline is now its command spelling, so an operator log and the command that produced it read the
same.

### Tasks, gate steps and dispatch

| Surface | Name now |
|---|---|
| The two Class-R alias tasks | `verify-staging-compose-paths`, `verify-mission-rest-size-limits` |
| The developer-tools library test task | `developer-tools-test` |
| The two wave gate steps that build and lint the tooling crates | `test xtask+developer-tools`, `clippy xtask+developer-tools` |
| The world-catalogue reclassification step | `catalogue drift` |
| The four language-ban steps | `no-python`, `no-node`, `no-shell`, `ci-shell` |
| The eleven in-process leaf adapters, in one file named for what they all are | `commands/ci/task_definitions/verification_dispatch.rs`; the three renamed adapters are `x_staging_compose_paths`, `x_mission_rest_size_limits`, `x_ci_schema_parity` |
| The lane marker on a borrowed task row | `Lane::Borrowed`, with no payload; `cargo xtask help` renders ` [borrowed]` |

`gate.rs`'s `VERIFY_STEPS` rows lost their ticket prefixes and now read
`("object registry aliases", "object-registry-aliases")` and so on, one row per verification above.
Two files pin those rows as source text and moved in the same commit:
`verifications/ci/schema_parity.rs` (`ROW_MISSION_REST_SIZE_LIMITS`, `ROW_CI_SCHEMA_PARITY`,
`VERIFY_MISSION_REST_SIZE_LIMITS`, `VERIFY_CI_SCHEMA_PARITY`,
`TASK_ECHO_MISSION_REST_SIZE_LIMITS`, `TASK_ECHO_CI_SCHEMA_PARITY`) and
`verifications/database/faction_library_seeds.rs` (`VERIFY_REL`, `WAVE_RUN_LINE`). The negative pin
holds: there is still no `verify-ci-schema-parity` row in `TASKS`, and `ci-local` reaches that gate
through a direct `Step::Xtask`, so a hollowed dispatch table cannot green the check that polices
dispatch. The step-label column widened from 24 to 28 characters in `gate.rs` and
`gate/gate_dispatch.rs` so the longer names stay aligned.

### Other renamed surfaces

| Surface | Name now |
|---|---|
| The mod wave status banner | `═══ mod wave status ═══` |
| The staging host-agent banner | `==> host control agent` |
| The editor-gate preflight banner | `== gate doctor (editor-gate preflight)` |
| The four-weapon equip assertion, its constant and its self-test | `assert_four_weapon_equip`, `EXPECTED_EQUIP_OK`, `four_weapon_equip_selftest`, printed as `four-weapon equip` |
| The compiled-boot fixture title | `compiled-boot fixture` |
| The slice merge commit subject | `merge <branch>` |
| The database self-test's arm 3 and its scratch base for arm 4 | `arm_live_database_refusal`; `tbd_gate_selftest_arm4`, still inside the `tbd_gate*` drop allow-list |
| The staging compose gate's audited subject, its basename helper and its comment stripper | `DEPLOY_SOURCE` (the Rust render path), `source_basename`, `strip_comments` |
| The file-length gate's scratch fixture | temp roots named `xtask-file-length-*`, holding a `fn placeholder()` body |
| The outliner drag-and-drop smoke suite entry | `outliner-drag`; `EDITOR_SUITE` is re-sorted so it sits in name order |
| The browser-oracle fixture tree | `tools_v2/developer-tools/fixtures/dom_oracle/` |
| The freeze marker attribute and the serializer global it injects | `data-dom-oracle-freeze`, `window.__domOracleSerialize` |
| The deployment asset-layout variant and its probe exit code | `AssetLayout::OldPackagesTree`, `const OLD_PACKAGES_TREE` |
| The two-outcome exit conversion | `Verdict::into_binary_exit_code` |
| The single-mod deploy fallback label | `single-mod env fallback (TBD_WORKSHOP_MOD_ID)` |
| The four road-section constants of the terrain topology decoder | `TOPO_MAIN_HIGHWAY`, `TOPO_SECONDARY_ASPHALT`, `TOPO_GRAVEL_COUNTRY_ROAD`, `TOPO_FARM_TRACK`; the values are unchanged, because they are on-disk section codes |
| The wave context's ticket-ledger display label | `.ai/tickets` |
| The shared gate lock | `verification_core::lock::GATE_LOCK_RELPATH` = `target/.repository-verification.lock`; `wave_execution/mod.rs` joins the const instead of its own literal |
| The verification-core test scratch prefixes | `verification-core-*` |
| The faction-library pin set | `assert_faction_library_pins` |

The browser-oracle payloads changed with the freeze marker and the serializer global, so their
pinned sha256 values in `browser_testing/tests/fixture_injection/tests.rs` are re-pinned to
`6ca42b7e360f8884a55fbc3328d1521b3ead232a86b84608d0add7f12618e305` (freeze) and
`8f2ab7e44f410d83d5d5c362a54fe0fadadb7ba6e4ab7869191c2d0dedf2ee5f` (serializer). No golden was
re-accepted and none needed to be: the marker only ever lands on the `<style>` element the freeze
script injects, and `STYLE` is already in the serializer's skip set, so it never reaches a golden.
The fixture tree contains no occurrence of the retired directory name.

### Analysis artifacts

Five committed decision records are live inputs of the map lane — the inland-water classifier
writes one and reads another, the seam verifier writes a third, and two more are cited by name in
the metadata the land-cover builder and the water compositor emit — so they move to domain
directories and the code resolves them through `developer-tools/src/repository_layout.rs`:

| To |
|---|
| `.ai/artifacts/inland_water/refine_spike.json` |
| `.ai/artifacts/inland_water/source_spike.json` |
| `.ai/artifacts/inland_water/water_source_spike.json` |
| `.ai/artifacts/aerial_orthophoto/seam_analysis.json` |
| `.ai/artifacts/cartographic_rendering/landcover_source_spike.json` |

`repository_layout.rs` holds `INLAND_WATER_ARTIFACTS_DIR`, `AERIAL_ORTHOPHOTO_ARTIFACTS_DIR` and
`CARTOGRAPHIC_RENDERING_ARTIFACTS_DIR` with their `inland_water_artifacts_dir(root)`,
`aerial_orthophoto_artifacts_dir(root)` and `cartographic_rendering_artifacts_dir(root)`
accessors, following the pair already in that module for the node package directory. The reads and
writes go through the accessors; the three emitted metadata strings that CITE an artifact
(`cartographic_rendering/build_landcover_masks.rs` `spikeArtifact`, `inland_water/sap_dir.rs`
`spikeArtifact` and `refineSpikeArtifact`, `inland_water/analyze_water_sources.rs` `parent`) are
`format!`ed from the same constants, so an emitted path cannot drift from the file it names.

Four analysis records under `.ai/artifacts` keep the names they were written under, because
nothing executable reads them: `git grep` over the whole repository finds each named only by frozen
specification documents, frozen `.ai/artifacts/*.md` handoffs and ticket bodies — none of which
this phase may edit. They are historical records, not pipeline inputs.

Three committed records under `.ai/artifacts` — `inland_water/refine_spike.json`,
`inland_water/source_spike.json` and `map_export_everon.json` — cite a renamed file by its previous
path inside a captured output field. Those files are pipeline output, not source: the classifier
rewrites its citation from the constants above on its next run, and the other two are frozen
records of runs that happened. Nothing resolves those strings at runtime — the classifier reads the
refine spike's measured bodies, never its provenance line — so the citations are stale prose in
captured data, not a broken path any code follows.

### Emitted metadata

The orthophoto stitcher now writes `"decoder": "developer-tools enfusion_texture_decoder (BC7 +
LZ4)"` and `"seamRepair": true`, on both the emit path and the in-place seam-bridge path. The
landcover builder drops a trailing ticket citation from three source strings, and the inland-water
classifier drops one from the `automation` string of its decision block. The `"slice"` key in the
five map-lane meta blocks becomes `"lane"`, carrying the pipeline lane's name, and the two
`"provenance"` strings describe the input rather than citing a ticket.

**The committed satellite artifact carries the old strings.**
`assets_v2/terrains/everon/satellite/everon-sat.tbd-sat` was produced by the previous code and is
not regenerated here, because regenerating it needs a Workbench export. Nothing reads those fields:
`git grep -n seamRepair contracts_v2 apps` is empty, and no schema under `contracts_v2/definitions`
names `decoder`, `seamRepair` or `slice`. The divergence is metadata only and resolves the next
time the bundle is rebuilt.

### Landing note for the gate lock

The shared gate lock's repo-relative path changes, and the lock serialises every worktree of this
checkout. Land this commit with no platform wave in flight: a worktree still on the older code
takes the old path while a worktree on this code takes the new one, and for that overlap the two
gates do not serialise against each other.

### Baseline test renames

Twelve test functions were named after a ticket number or after a driver that no longer exists.
Each is renamed to say what it asserts, and each carries its line in `Test inventory`
above, edited in place so the inventory stays a complete roster:

| Crate and module | Name now |
|---|---|
| `developer_tools` `blueprint::library::tests` | `committed_farmhouse_descriptor_reproduces_its_instances_file` |
| `developer_tools` `map_verification::terrain_manifest::tests` | `occluder_init_still_fetches_the_blas_manifest_for_hot_chunks` |
| `ticket_engine` `registry::typed_projection::tests::typed_projection_tests` | `ready_class_tickets_carry_spec_main_goal_and_acceptance` |
| `ticket_engine` `registry::typed_projection::tests::typed_projection_tests` | `program_children_parse_as_work_and_their_parents_list_them` |
| `ticket_engine` `registry::typed_projection::tests::typed_projection_tests` | `engine_scope_table_projects_to_the_engine_domain` |
| `ticket_engine` `wave_lock::tests::emptied_wave_tests` | `lock_without_an_emptied_section_parses_and_renders_without_one` |
| `ticket_engine` `wave_lock::tests::packing_and_history_tests` | `lock_without_a_wave_base_parses_as_zero` |
| `xtask` `commands::deploy::database_operations::tests` | `safe_scratch_allow_list_admits_scratch_names_and_refuses_the_live_database` |
| `xtask` `verifications::deployment::staging_compose_paths::tests` | `a_correct_source_holds` |
| `xtask` `verifications::deployment::staging_compose_paths::tests` | `a_missing_deploy_source_does_not_read_as_pass` |
| `xtask` `verifications::deployment::staging_compose_paths::tests` | `every_source_perturbation_bites` |
| `xtask` `verifications::deployment::staging_compose_paths::tests` | `the_live_deploy_source_holds` |

The earlier phase's two renames — `shipped_ticket_keeps_its_shipped_at_commit` and
`instance_kinds_match_the_enums_schema_and_the_crate_array` — land in the same commit with their
inventory lines. Test totals are unchanged: 259 + 200 + 1 + 68 + 652 + 1 = 1181 passed, 0 failed,
4 ignored, exactly the baseline count, so no live test vanished behind a rename.

### Acceptance

| Command | Expected | Actual |
|---|---|---|
| `cargo xtask verify ci-schema-parity` | PASS | `ci-schema-parity: PASS`, exit 0 |
| `cargo fmt --check` | clean | exit 0, no output |
| ticket-named verification and adapter functions under `tools_v2` | empty | empty, grep exit 1 |
| any ticket-numbered function or test name under `tools_v2` | empty | empty, grep exit 1 |
| the eleven-adapter shim file under its old name | absent | absent |
| the seven `cargo xtask … --help` surfaces through the ticket, script and dead-name filter | empty | empty, grep exit 1 |
| `cargo run -p developer-tools --bin map -- --help` through the ticket-number filter | empty | empty, grep exit 1 |
| `.github/workflows` through the ticket-number and dead-crate filter | empty | empty, grep exit 1 |
| `AssetLayout::` uses in the deployment preflight | four variants present | 8 lines over `Ready`, `OldPackagesTree`, `Absent`, `Indeterminate` |
| browser-oracle fixture and page-global filter over `tools_v2` and `apps` | empty | empty, grep exit 1 |
| ticket identifiers and node script names in the browser-oracle freeze manifest | 0 | 0 |
| browser-oracle DOM goldens carrying a content edit | none | none: every entry is an `R` rename, zero `M` |
| the renamed-artifact literal filter over `tools_v2` | empty | empty, grep exit 1 |
| ticket-named `.json` artifacts under `.ai/artifacts` | the four historical records | exactly four, each with no executable reader |
| gate-lock, road-section, binary-exit and workshop-label filter over `tools_v2` | empty | empty, grep exit 1 |
| production strings naming a deleted script | empty | five lines remain, all one live rendered filename (below) |
| `cargo test -p xtask -p developer-tools -p verification-core -p ticket-engine` | green | exit 0; 1181 passed, 0 failed, 4 ignored |
| `cargo clippy -p xtask -p developer-tools -p verification-core -p ticket-engine --all-targets -- -D warnings` | clean | exit 0 |
| `cargo xtask ci ci-local` | green | exit 0; every step green, ending `ci-schema-parity: PASS`. The mission-rest-size-limits step prints its three RED proofs of non-vacuity, which are part of its PASS |
| `cargo xtask ci editor-api-boot` then `cargo xtask mk leptos-gates` | green | exit 0 for both; `gate doctor: OK — 0 warning(s)`, 21/21 editor smokes pass with zero panics, and the browser-oracle verify reports 25/25 routes matching the frozen oracle with `diffs=0` and identical byte counts on every route |
| `git status --porcelain` after the commit | empty | empty |

Two filters do not reach zero, and each names something this phase must not rewrite:

- The deleted-script filter matches the host control agent's file name at
  `commands/deploy/staging/agent.rs:307`, `staging/agent/render_agent_files.rs:10,58`,
  `staging/agent_selftest.rs:122` and `staging/remote/ssh_argv.rs:421`. That name is not a deleted
  script: `render_agent_files.rs` WRITES that file on every staging deploy, systemd socket-activates
  it on the game host, `apps/website/api_v2/tests/game_agent_rcon.rs:102,178` asserts the filename,
  and `docs/platform/PLAYTEST_RUNBOOK.md` documents it for operators. Renaming it changes a live
  operational contract across two crates and a deployed host, which is the same reason the closure
  plan keeps the deployment preflight's remote `packages/map-assets` probe.
- The ticket-identifier filter over production comment lines is still non-zero across the files this
  phase edited only for a label or a single identifier. Those are the subject of the single prose
  pass that follows; the entries are listed under `Found for P8`.

### Found and fixed

- `tools_v2/developer-tools/src/map_raster_pipeline/cartographic_rendering/build_tile_pyramid.rs`
  — the cartographic aggregator spawned three `make` targets. There is no Makefile, so all three
  could only ever fail. Repointed at the live successors: `cargo xtask schema map-glyphs`, and
  `cargo run -p developer-tools --bin world -- validate-exports` /
  `… verify-phase --terrain everon --phase P5_props` through a new `run_world` closure.
- `tools_v2/xtask/src/commands/db/operations/selftest.rs:158` — the same missing Makefile: arm 2's
  skip message asserted that a deletion had removed the file and cited a ticket for it. It now says
  the checkout carries no Makefile and that arm 1 holds the pin, which is what the code does.
- `tools_v2/xtask/src/commands/db/operations/test_it.rs:75,208` — the two REFUSING messages an
  operator sees when a database name falls outside the allow-list cited a ticket instead of naming
  the rule. They now say `scratch allow-list`, and the three consumers of that text moved with
  them: the third arm of `db/operations/selftest.rs` (now `arm_live_database_refusal`), the guard
  test in `db/operations/tests/test_it/tests.rs`, now `the_guard_refuses_the_live_database`, and
  the allow-list test in `deploy/tests/database_operations/tests.rs`.
- `tools_v2/xtask/src/commands/platform/wave_execution/schema.rs` — the "found no stamp inputs"
  refusal named three directories, two of which do not exist. It now formats the real `stamp_roots`
  array, so the message cannot drift from the list again.
- `tools_v2/xtask/src/commands/ci/task_definitions.rs` — reformatting the renamed rows pushed the
  file to 507 lines, over the 500-line limit. Brought back to 499 by collapsing the ten dispatch
  imports into one braced `use` and tightening four comment blocks.
- `.github/workflows/ci.yml` — the tooling test step, the language-gate job, the schema job's
  sub-gate list and the mod-gates step names all named tickets or the dead crate; each now names
  what the step runs.
- Every `cargo xtask` help surface reached by the acceptance loop (`verify`, `schema`, `ticket`,
  `platform wave`, `deploy db`, `help`, the root parser, plus `mod`, `map`, `deploy`, the `world`
  binary and the `gate` binary) lost its ticket citations and its deleted-script names. Two were
  caught only by running the filter: the `deploy db` backup, restore and drill help lines still
  named deleted shell scripts, and `ticket stamp-sha`'s help contained the word "different", whose
  first four letters the acceptance filter's bare `diff` token matches.
- `tools_v2/developer-tools/src/browser_testing/cli.rs` — dropped the `as smokes` import alias, so
  the eight call sites name `editor_smoke_tests` directly.
- `tools_v2/developer-tools/src/map_raster_pipeline/satellite_archive/build_unified_satellite.rs`
  — the unified-satellite builder copied the water-composite meta block's lane key under the
  spelling the compositor stopped writing when that key was renamed, so the field would have been
  JSON null in every bundle built from here on. It reads the key the compositor writes. It is the
  only reader of that block's contents in the repository.
- `tools_v2/developer-tools/src/browser_testing/dom_oracle.rs` and `fixture_injection.rs` — their
  module headers named four driver files that do not exist and narrated two retirements. They now
  say what the gate captures, what `verify` and `accept` do, why there is no whole-tree re-freeze
  (the goldens are not regenerable from any dist this repository builds, so a bulk overwrite
  destroys the oracle), and why the two injected payloads must never be reimplemented natively
  (their exact bytes are what serialized every golden).
- `tools_v2/developer-tools/src/map_raster_pipeline/cartographic_rendering/build_landcover_masks.rs`
  — the entry point's doc comment introduced itself as a port of a deleted Node script; it now
  describes the classification it performs.
- `tools_v2/xtask/src/commands/ci/task_definitions.rs` — the node gate's help named its two banned
  extensions with leading dots, which reads as two filenames to the deleted-script filter. It names
  them without.
- `tools_v2/xtask/src/commands/setup/client_addons.rs:51` — a measurement note quoted the tooling
  test step under its old spelling; it now quotes `test xtask+developer-tools`.
- `tools_v2/ticket-engine/src/cli/brief.rs` — twelve printed guidance lines named deleted Node and
  shell scripts and a Makefile target that does not exist. Each now names the live command: the
  `map` binary's `stitch-sap-ortho`, `composite-water`, `build-pyramid --lossless`,
  `verify-sap-ortho` and `verify-pyramid --expect-lossless` verbs, `cargo xtask schema validate`,
  `cargo xtask verify route-tags`, and the DEM sampler at
  `apps/website/map-engine/src/world/terrain/dem/sampling.rs`.
- `tools_v2/developer-tools/src/map_verification/labels/terrain_alignment.rs:145` — the anchor
  check read a pixel coordinate through two field accesses whose spelling collides with the
  deleted-script filter. It destructures `PixelCoord` instead, which is also what the two following
  lines read as.
- `tools_v2/xtask/src/tests/node_free_tests.rs` — the file-length fixture named its temp roots,
  its synthetic function and its synthetic allow-list comment after a ticket. All three now say
  what they are.
- `tools_v2/xtask/src/verifications/deployment/staging_compose_paths.rs` and its
  `source_audit.rs` and tests — the gate audits a Rust render path, so its subject constant, its
  basename helper, its comment stripper and every doc comment now say so. The prose that narrated
  a Python and shell predecessor is replaced by the invariants it carried: comments are stripped
  before matching so a described contract cannot pass for an honoured one, an unreadable or absent
  subject is `DidNotRun` rather than a pass, the findings accumulate rather than stopping at the
  first, and the three carried oddities (bare `#`, `//` before an unquoted URL, a backslash before
  a closing single quote) are still named with the test that pins each.
- `tools_v2/ticket-engine/src/registry/tests/typed_projection/typed_projection_tests.rs` — the
  module's doc comments cited tickets as provenance and named a module file that does not exist;
  they now state the invariant each test holds, and the refusal message names the CLI tree and the
  typed ops surface rather than a deleted module and a dead crate.
- Prose, in every file whose subject this phase renamed: the nine verification modules with their
  `source_audit` and `extract_fn_body` children, both wave gate drivers, `wave_execution/mod.rs`
  and `wave_execution/schema.rs`, the task table and the task runner, the editor smoke harness, the
  map and world CLIs, `verification-core/lock.rs` and `verdict.rs`, and the CI workflow. Ticket
  identifiers, deleted script names, dead crate names and change narration are gone from all of
  them; the measurements and refusal reasons they carried stay, in the present tense.
- `tools_v2/xtask/README.md` — the sentence promising that ticket-number command spellings
  remain supported is replaced by the naming rule this phase enforces.
- `docs/platform/EDITOR_FACTORY_START.md:130` and `tools_v2/PHASE_TWO_HANDOFF.md:13` named a
  verification command and a task alias that this commit removes; both now name the live spelling.
- The two test renames the previous phase left for this one are done, with their inventory lines:
  `ticket-engine/src/registry/tests/typed_projection/typed_projection_tests.rs` is
  `shipped_ticket_keeps_its_shipped_at_commit`, and
  `xtask/src/verifications/schemas/tests/checks/instance_kind_lockstep_tests.rs:13` is
  `instance_kinds_match_the_enums_schema_and_the_crate_array`.
- The previous phase's rendered-agent entry is done here: the staging agent's file header named a
  deleted shell driver and a ticket on three lines inside its raw string. It now names
  `cargo xtask deploy staging` as the renderer and points at the renderer's own module header for
  the scope note.
- The second `Found for P6` entry of the previous phase — `verification-core/src/lock.rs`
  attributing the lock protocol to a deleted shell driver — is done here, because this phase edits
  that file: the module header, the three public constants and the interop test now name
  `cargo xtask platform wave` and the environment overrides. Its first entry, the inline test
  function name, is still P6's and lands with the test-module extraction; the function is already
  `interops_with_the_flock_command`.

### Found for P6

- `tools_v2/verification-core/src/lib.rs:10` quotes a deleted shell library's own header, ticket
  identifier and script name included. The live fact to keep is the one the quote carries: one
  implementation of the four-outcome verdict, shared by every gate, so the next gate cannot be born
  with the "a search that did not run reads as a pass" hole. P6 already rewrites `lib.rs:3,28,63`;
  this is the same block.
- `tools_v2/verification-core`'s process module at four lines, `src/gate.rs:1`, `src/scan.rs:6,17` and
  `src/verdict.rs:5,20,28,34,41,64,145` name deleted shell scripts. The invariants to keep: a
  merged-output drain must never collapse an exit code, because a self-test that passes only on
  exit 1 is meaningless otherwise; and the four-outcome verdict exists because a boolean cannot
  carry "did not run".
- `tools_v2/xtask/src/tests/tooling_dependency_boundaries.rs:61` still asserts on a manifest path
  under the deleted tools tree. P6 replaces those negative asserts with positive layout asserts.

### Found for P7

- `tools_v2/ticket-engine/src/cli/brief.rs` — the per-ticket hardcoded switch is now free of
  deleted script names, but it is still a hardcoded switch: forty printed lines of per-ticket
  guidance that belong in the tickets' own `spec`, `plan` and `owns` fields. P7 deletes it, which
  also removes the frozen `.ai/artifacts/*.md` handoff pointers and the `docs/specs/` literals it
  prints.
- `tools_v2/xtask/src/commands/platform/wave_execution/mod.rs:59` still declares the archived
  wave-plan module under its old name. P7 renames that module.
- The eleven inventory lines at `tools_v2/PHASE_FIVE_HANDOFF.md:453-462,523` sit under the two
  ticket-engine modules P7 renames and move with them, as the previous phase recorded.

### Found for P8

- Comment lines across the files this phase edited still carry a ticket identifier. They are
  provenance prose in files whose subject this phase did not rename — a label change or a single
  identifier in an otherwise untouched file — and they are exactly what P8's single prose pass is
  for. Reproduce the list with the plan's R1 grep restricted to production `.rs` comment lines. The
  heaviest are `commands/db/operations.rs`, `commands/platform/slice_worktree/git_plain.rs`,
  `browser_testing/diagnostics/ensure_gate_font_cache.rs` and `commands/deploy/staging/config.rs`.
- `tools_v2/developer-tools/src/map_raster_pipeline/aerial_orthophoto.rs:3`,
  `aerial_orthophoto/stitch_sap_ortho.rs:3`, `cartographic_rendering.rs:4`,
  `cartographic_rendering/build_tile_pyramid.rs:5`, `inland_water.rs:3,21` and
  `inland_water_archive.rs:10` describe each stage as a port of a deleted Node or shell script.
  The live fact under each is what the stage does; the file names go.
- The plan's R3 row targets zero shell, Python and Node file spellings in `tools_v2`. Five of them
  name the host control agent the file `commands/deploy/staging/agent/render_agent_files.rs` writes
  onto the game host and `apps/website/api_v2/tests/game_agent_rcon.rs` asserts by name. It is a
  live remote artifact, not a deleted script, so R3 needs the same explicit retained-name carve-out
  the plan's decision 8 gives the deployment preflight's `packages/map-assets` probe — not a
  rename.
- `tools_v2/developer-tools/src/browser_testing/editor_smoke_tests/outliner_drag/execution.rs:18`
  writes a ticket-named page global that `outliner_drag/vehicle_snap_cases.rs:40` reads back. The
  closure plan's scope section calls that global the frontend's, but it is not: a grep for it over
  `apps` is empty, so both ends are inside developer-tools and the rename is one-sided. Suggested
  name `window.__outlinerDragEvents`. Left as the plan directs, recorded because the measurement
  disagrees with it.
- `tools_v2/xtask/src/verifications/mod_scripts/mission_rest_size_limits.rs:369` pins the literal
  text of a comment in
  `apps/mod/tbd-framework/Scripts/Game/TBD/Systems/Mission/Loaders/TBD_MissionLoader.c:1041`, which
  opens with a ticket identifier. The pin is true today; renaming it means editing the mod source
  in the same commit and re-running `cargo xtask mod compile`.
- `tools_v2/ANALYSIS_AND_INVENTORY.md`, `ARCHITECTURE_PLAN.md`, `PHASE_ONE_HANDOFF.md` and
  `PHASE_THREE_HANDOFF.md` still name deleted gate files, the deleted tools tree, and three
  decomposed modules. Every one of those documents is on P8's rewrite list.

### Commands that could not run

None. Every command of this phase ran unmodified in this environment.

## P6 — verification-core and structural rules

This phase brings `verification-core` under the same structural rules as the rest of the tooling
and widens the rules themselves to every crate under `tools_v2`. The file-length gate refuses an
allowlist row whose file is not scanned, so splitting the oversized module and removing its
exemption are one commit or neither.

### What changed

The crate's single process module (786 lines) becomes a module directory:

- `src/proc/mod.rs` — the vocabulary: `Run` and its builder, `Output`, `Merged`, and the
  re-exports of `which`, `retry` and `wait_for`. The public API is unchanged: callers still write
  `verification_core::proc::{Run, Output, Merged, which, retry, wait_for}`.
- `src/proc/runner.rs` — `output`, `merged_output`, `status`, `expect_ok`, `expect_code`, the
  process-group isolation (`setsid`, with `setpgid` as the fallback), the deadline loop and the
  `killpg` that ends a whole tree.
- `src/proc/stream.rs` — the pipe drains: two threads for separated streams, one for the shared
  pipe, each reading to EOF for the child's whole life so a full buffer cannot deadlock it.
- `src/proc/lookup.rs` — `which`, `retry` and `wait_for`.
- `src/proc/README.md` — described a file that no longer existed; it now describes the four
  modules of the directory it sits in.

Two duplications the split removed rather than copied: both capture paths now build their
`Command` through one `Run::command`, and both reap through one `wait_within`. The behaviour is
unchanged on every path, the joins on the timeout path included.

The seven inline test modules move to one file each under `src/tests/`, named for the production
module they test, each declared from that module with a `#[cfg(test)]` `#[path]` declaration
(`proc` declares its own from `proc/mod.rs`, one directory up). Test bodies and assertions are
unchanged; the eight renamed functions are listed below and their lines in the baseline inventory
above now name them.

`.coding-standards-allowlist.yaml` loses its last `tools_v2/` row, the SIZE-3 exemption that
process module held. Every file in the crate is now inside the ordinary limits.

`tools_v2/xtask/src/tests/tooling_dependency_boundaries.rs`:

- `TOOLING_CRATES` lists all four crates, so the line limits, the no-inline-tests rule and the
  no-size-exemptions rule scan `verification-core` and `ticket-engine` as well.
- The executable test asserts what the tree holds instead of what it does not: the six
  executables, the four crate manifests, the two repository-layout modules and the node package
  manifest. It is named for what it checks.

`tools_v2/xtask/src/main.rs` loses its three crate-wide `#![allow(clippy::…)]`. Clippy then
reported 52 `collapsible_if` sites in 34 files, every one a nested `if` that becomes a let chain;
all are collapsed and no `#[allow]` was added anywhere. `unnecessary_sort_by` and
`unnecessary_unwrap` had no remaining sites.

Manifests: `verification-core/Cargo.toml` gains `description`, `rust-version = "1.95"` and
`license = "UNLICENSED"`, and its header comment now states the dependency policy without naming
a crate and a script that do not exist; `developer-tools/Cargo.toml` gains `rust-version` and
`license`; `xtask/Cargo.toml` describes what the binary is.

Prose across the crate now describes the code as it stands. The module documents of `lib.rs`,
`gate.rs`, `pattern.rs`, `proc/mod.rs`, `report.rs`, `scan.rs` and `verdict.rs` kept every
invariant they carried — the four outcomes, the line-anchor semantics, the fail-closed walk, the
raw exit code, the process group, the pipe drains — and lost the attributions to deleted shell
drivers, the ticket identifiers and the broken rustdoc link to a file outside the workspace.
`README.md` lists the module tree as it now is and documents the lock path and its overrides.

### The renamed tests

| Before | After | Why |
|---|---|---|
| `lock::tests::interops_with_the_flock_command_used_by_wave_sh` | `interops_with_the_flock_command` | The name pointed at a deleted driver; the function was renamed in the previous phase and only its inventory line was outstanding. |
| `verdict::tests::legacy_binary_exit_matches_gate_grep` | `the_binary_exit_code_collapses_both_failure_kinds_to_one` | Named a deleted library and used a word the prose rules ban; the new name states the contract. |
| `verdict::tests::renders_bare_failure_like_bash` | `renders_a_bare_failure_as_one_headline` | Described the rendering by comparison to something not in the repository. |
| `verdict::tests::renders_missing_target_like_bash` | `a_missing_target_names_the_file_and_the_six_space_continuation` | The function carried this name already; the inventory line was stale. |
| `pattern::tests::caret_is_a_line_anchor_like_grep` | `caret_is_a_line_anchor` | Same comparison-to-an-absent-thing shape. |
| `pattern::tests::dollar_is_a_line_anchor_like_grep` | `dollar_is_a_line_anchor` | As above. |
| `pattern::tests::posix_classes_work_as_in_ere` | `posix_classes_work_as_in_an_extended_regex` | An abbreviation that needs outside context. |
| `tooling_dependency_boundaries::heavy_package_has_one_owner_and_preserves_executable_names` | `the_tooling_tree_holds_its_executables_manifests_and_layout_modules` | The old name described the negative asserts this phase replaced. |

The crate runs 68 test functions, the same 68 as the baseline, plus the crate-level documentation
example the inventory does not list.

### Acceptance

| Command | Expected | Actual |
|---|---|---|
| `find tools_v2/verification-core/src -name '*.rs' -exec wc -l {} + \| awk '$1 >= 500 && $2 !~ /tests/'` | empty | one line, ` 2282 total` — `wc` prints an aggregate row for a multi-file argument list and 2282 passes both conditions. No file row is printed; adding `&& $2 != "total"` yields empty. The largest production file is `proc/runner.rs` at 283 lines. |
| `git grep -c 'mod tests {' tools_v2/verification-core/src` | 0 | no output, exit 1 (no file matches) |
| `grep -c 'tools_v2/' .coding-standards-allowlist.yaml` | 0 | 0 |
| `grep -c '#!\[allow' tools_v2/xtask/src/main.rs` | 0 | 0 |
| `git grep -n 'TOOLING_CRATES' tools_v2/xtask/src/tests/tooling_dependency_boundaries.rs` | four entries | `:134` declares `[&str; 4]` with `xtask`, `developer-tools`, `verification-core`, `ticket-engine`; `:143` and `:324` iterate it |
| `grep -c -E '^(description\|rust-version\|license)' <the four manifests>` | 3 each | verification-core 3, developer-tools 3, xtask 3, ticket-engine 3 |
| `cargo test -p verification-core` | count = P0 | ok — 68 passed, 0 failed, plus 1 documentation test |
| `cargo test -p xtask tooling_` | green, 4 crates | ok — 12 passed, 0 failed |
| `cargo xtask verify file-length` | OK, no orphan row | exit 0 — scanned 2523 `.rs` files, 0 violations (2513 after the previous phase, plus the ten files this split adds) |
| `cargo clippy -p xtask -p verification-core --all-targets -- -D warnings` | clean | exit 0, no diagnostics |
| `cargo check --workspace --locked` | clean | exit 0 |
| `cargo fmt --check` | clean | exit 0 |
| `cargo doc -p verification-core --no-deps` | links valid | exit 0, zero warnings |
| `cargo xtask verify ci-schema-parity` | PASS | `ci-schema-parity: PASS`, exit 0 |
| `cargo test -p xtask -p verification-core -p ticket-engine -p developer-tools` | green | exit 0 — 652, 68, 259 and 200 passed, 0 failed, 4 ignored |

### Found and fixed

- `tools_v2/verification-core/src/lib.rs:3-63` — the crate document quoted a deleted shell
  library's header, carried three ticket identifiers, and ended in a rustdoc link to a path
  outside the workspace. The facts it carried are kept: one implementation of the four outcomes
  shared by every gate, the matcher compiled in, and `?` propagating "did not run" out of a
  compound condition.
- `tools_v2/verification-core/src/proc/mod.rs:25,29`, `runner.rs` (`expect_code`) and
  `tests/proc_tests.rs` — the raw-exit-code rule named two deleted scripts. The live commands with
  that contract are `cargo xtask mod compile --selftest`, which passes only on exactly 1
  (`tools_v2/xtask/src/commands/mod_ops/compile.rs:2`), and `cargo xtask map export-terrain`,
  which exits 2 when the staged export is missing
  (`tools_v2/xtask/src/commands/map/terrain_export.rs:4,7`).
- `tools_v2/verification-core/src/gate.rs:1-34`, `scan.rs:1-26`, `verdict.rs:1-146`,
  `pattern.rs:1-31`, `report.rs:1-11,83` — deleted script names, a bash-idiom narrative and the
  word the history rules ban. Each invariant stays and now stands on its own: a compound condition
  must not short-circuit clean, a silenced search error must not read as zero violations, a
  boolean cannot carry four outcomes, `^` and `$` are line anchors, and the two failure kinds are
  counted separately.
- `tools_v2/verification-core/src/tests/lock_tests.rs` — the interop assertion said Rust had
  failed to contend with bash's lock; it names `flock(1)`, which is what the test actually starts.
- `tools_v2/xtask/src/verifications/language_bans/node_and_file_limits/repository_access.rs:47` —
  the vacuous-pass refusal printed a ticket identifier to the operator. The sibling gate in
  `verify_engine_layers.rs:318` prints the same sentence without one; they now match.
- Six directories under `tools_v2` held nothing but a signpost `README.md` pointing at an
  implementation that lives elsewhere: `ticket-engine/src/{core,store,operations}`,
  `developer-tools/src/map_raster_pipeline/satellite_container`,
  `developer-tools/src/browser_testing/server` and
  `developer-tools/src/world_export_pipeline/mathematical_gates`. Coverage check before deleting:
  each directory contained exactly one tracked file, that file was its own `README.md`, none of
  the six holds a `.rs` file, so no `mod` or `#[path]` declaration can resolve into one; and each
  target the README named still exists (`ticket-engine/src/model/`, `src/store.rs`, `src/ops/`,
  `map_raster_pipeline/satellite_archive_container.rs`, `browser_testing/server.rs`,
  `world_export_pipeline/mathematical_verification.rs`). All six are deleted;
  `find tools_v2 -type d -empty` outside the node package is empty afterwards.
- The baseline inventory lines for the eight renamed tests, one of which
  (`verdict::tests::renders_missing_target_like_bash`) had been stale since the phase that moved
  the deployment files.

### Found for P7

- `tools_v2/xtask/src/verifications/schemas/checks/specification_consistency.rs:326` reads the
  registry through `ticket_engine::registry::load_registry` and falls back to a hardcoded slice
  identifier when that read fails. The fallback is a literal ticket identifier in production code,
  so it cannot survive P8's rule; the path it stands in is the one P7 reworks when the registry
  loader loses its JSON branch. Decide there whether an unreadable registry should refuse the gate
  (a `DidNotRun`) rather than silently compare the hub header against a frozen identifier.

### Found for P8

- `tools_v2/verification-core/src/scan.rs:95` exports `grep_lines`, whose name borrows an external
  tool for a function that runs no process. It is public API used across `xtask` and
  `developer-tools`; renaming it is a public-surface change, which belongs with the other renames
  rather than inside a prose pass — recorded so the decision is taken deliberately.
- `tools_v2/xtask/src/commands/platform/wave_execution/changed/changed_rs.rs:57-59` explains the
  edition walk by quoting a shell pipeline. The invariant to keep is that the FIRST line starting
  with `edition` is read, reduced to its digits, so `edition.workspace = true` yields an empty
  string and the walk continues upward.
- `tools_v2/xtask/src/commands/mcp/json_rpc.rs:80` ends a return arm with the comment
  `// BrokenPipe → 0 (Python)`, naming a language this repository bans from its tooling.

### Found for P10

- This document is itself inside the R2, R3 and R7 scopes (they read `tools_v2` and
  `tools_v2/*.md`). Measured on this commit: R2 matches 15 lines, R3 10 lines, R7 3 lines, all of
  them in sections written by earlier phases — twelve are baseline-inventory lines naming the two
  ticket-engine modules P7 renames, and the rest are prose that names a deleted driver while
  explaining why something was removed. The final matrix will report them unless the phase that
  rewrites this document's earlier sections states the same facts without the names, so run the
  three rows with and without `':!tools_v2/PHASE_FIVE_HANDOFF.md'` and record both numbers.

### Commands that could not run

None. Every command of this phase ran unmodified in this environment.

## P7 — Repository path modules

Each of the three tooling crates now owns exactly one module that spells the repository paths it
reads outside its own tree, and `xtask` and `ticketboard` consume the ticket domain's rather than
declaring their own. No production file in `tools_v2/` or `apps/ticketboard/` spells a `.ai/`,
`docs/` or `documentation_v2/` path anywhere else.

### The three modules

| Module | What it owns |
|---|---|
| `tools_v2/ticket-engine/src/repository.rs` | The registry directory and its root marker, the ticket schema, the scope vocabulary, the wave lock, the dispatch queue, the receipt and estimate trees with their schemas, the artifact tree, the worktree base, the last-verified marker, the verdict directory, the handoff document name, the sparse-checkout sets, and a `documentation` submodule holding every path under the documentation tree |
| `tools_v2/xtask/src/core/repository_layout.rs` | The deployment and server-profile trees it already owned, plus a re-export of the five ticket-domain locations xtask reads, plus a `documentation` submodule: the wave-packing marker, the specification directory, the four authority documents, the document-layout target, and the six runbooks printed in messages |
| `tools_v2/developer-tools/src/repository_layout.rs` | The contract and asset trees it already owned, plus the checkout root marker, the Enfusion symbol index and its upstream symbol table, the export operation-log directory with a function per artifact it holds, and a `documentation` submodule: the mod documentation directory, the capability verdict table and the editor-gate runbook |

### What changed

**`tools_v2/ticket-engine/`**

- `src/repository.rs` rewritten as above. It gains `is_repo_root`, so a caller holding a directory
  confirms it without a second spelling of the marker, and loses `registry_path` and
  `gap_analysis_path`.
- Every path literal in the crate — production and the scratch trees the tests build — now reads
  from those items. Synthetic sample paths in test data (`docs/spec.md`, `docs/x.md`) stay
  literals: they name files that do not exist and are not repository locations.
- `validation/vocabulary.rs` and `vocab.rs` each declared the vocabulary path; both now read
  `repository::SCOPE_VOCAB`. The same collapse applied to the wave lock, the metrics and estimate
  trees and their schemas, and the token-estimate factor document.
- The ticket-file storage module and its test file now stand at `registry/ticket_file_storage/`,
  and the status-at-a-revision reader it contained at `registry/ticket_status_history.rs`, with its
  own sibling test file. The archived-wave-plan reader stands at `wave_lock/archived_wave_plans.rs`
  with its test directory. Each moved with `git mv`, so history follows.
- The JSON-monolith read path is gone: `registry::load_json_monolith` and the `json.is_file()` arm
  of `load_registry`, and the root walk's second marker. `save_registry` keeps its refusal and
  drops the file removal that followed it. The one remaining reader of that file reads git
  revisions, and derives its path from the ticket directory rather than spelling it.
- `cli/brief.rs` loses the per-ticket switch — forty printed lines of guidance keyed on ticket
  identifiers. The brief now prints the ticket's own `spec`, `plan`, `owns`, `main_goal`, the five
  body fields, its citations and its acceptance.
- `cli/queries.rs` reads the sparse-checkout sets from `repository::SPARSE_CHECKOUT_SETS`; the
  `root` set is the registry, the artifact tree, the documentation tree, `tools_v2`, `.cargo`,
  `README.md` and `CLAUDE.md`.
- `validation/references.rs` reads its scan roots, its exempt prefixes and its archived-wave-plan
  reader list from the documentation submodule.
- New sibling test file `src/tests/repository_layout_tests.rs`: four pins over the layout module,
  including that every ticket target vocabulary word has a sparse-checkout set.

**`tools_v2/xtask/`**

- `const BASE` in `commands/mod_ops/wave_execution.rs` and `commands/platform/slice_worktree.rs`,
  and the literals in `platform/preflight/ok.rs`, `platform/slice_execution.rs` and
  `wave_execution/mod.rs`, all read `repository_layout::WORKTREES_DIR`. Same for the last-verified
  marker and the verdict directory.
- The three root probes in `commands/fetch/dispatch.rs`, `platform/preflight/ok.rs` and
  `commands/deploy/tests/staging/tests.rs` call `ticket_engine::repository::is_repo_root`.
- Help and refusal text that names a document is built from the constant: the two wave-lifecycle
  help blocks, the slice-worktree usage, the dev-server usage, the document-layout refusal, the
  deployment and staging messages, the upstream-leak advice and the spawn-determinism preflight.
- The archived-wave-plan shim stands at `wave_execution/archived_wave_plans.rs`, and the module
  list it sits in is sorted again.
- `verifications/schemas/checks/specification_consistency.rs` gate 10 answered from a hardcoded
  slice identifier whenever the registry read failed. It now propagates an unreadable registry as a
  refusal, and reports having nothing to check when the program records no active slice — which is
  the live state, so the gate had been comparing the hub header against a frozen identifier.
- `src/tests/tooling_dependency_boundaries.rs` asserts three layout modules exist, not two.

**`tools_v2/developer-tools/`**

- `repository_paths.rs` loses the second root marker and reads the one in `repository_layout.rs`.
  Its header states why the walk is deliberately the second implementation of the ticket domain's.
- The `enf` defaults, the operation-log writers and readers, and the font diagnostic's runbook
  pointer all resolve through the layout module. `enf --help` and every subcommand default are
  byte-identical to before.

**`apps/ticketboard/`**

- Its six local path constants — the ticket directory, the scope vocabulary, the roadmap, the
  lock file name, and the estimate and receipt subdirectories — are deleted; every production path
  reads `ticket_engine::repository`. The two
  empty-state strings that named a directory became functions that name it from the constant.
- The eighteen inline `#[cfg(test)] mod tests { … }` blocks are extracted to
  one file per module under `apps/ticketboard/src/tests/`, declared with `#[path]`, as Law 7 requires. Test
  module paths are unchanged, so every test keeps its name: 173 before, 173 after.
- `detail.rs`, `verbs.rs` and `viewer.rs` fall under 500 lines once their tests move out, so their
  three SIZE-3 rows leave `.coding-standards-allowlist.yaml`. `app.rs`, `board.rs`, `estimates.rs`,
  `metrics.rs` and `mutate.rs` keep theirs.

**`documentation_v2/ARCHITECTURE_PLAN.md`** §4 is no longer a per-file pin table. It names the
three `documentation` submodules and what each owns, adds `docs/platform/factory_pack_wave` to the
retire-or-move list, and states that Phase 1 of that blueprint is three module edits plus the
`.ai/tickets` citation rewrite.

### Acceptance

| Command | Expected | Actual |
|---|---|---|
| Path literals outside the three layout modules, production files only | empty | 0 lines |
| Tooling-tree literals outside the two layout modules | the api_v2 Caddyfile pin and the xtask repository-root test pin | 3 lines: those two plus the structural-rules assertion that the npm package directory exists |
| The historical registry file name under `tools_v2` and `apps/ticketboard` | only the module that reads git history | 28 lines verbatim, none of them the ticket registry: restricted to the ticket directory's own path the count is 0, and the only module naming that file is `registry/ticket_status_history.rs`, which derives the path from the ticket directory rather than spelling it. The 28 name three live and correctly named files — the mod's object registry, the terrain registry, and a frontend API golden |
| Renamed module names anywhere | empty | 0 lines |
| Ticket identifiers in `cli/brief.rs` | 0 | 0 |
| `cargo test -p ticket-engine -p xtask -p developer-tools -p ticketboard` | green | exit 0; ticket-engine 203 plus 1 compile-failure test, xtask 652, developer-tools 259 and 4 ignored, ticketboard 170 and 3 ignored |
| `cargo xtask ticket check --strict` | OK | exit 0, `check OK` |
| `cargo xtask ticket sync` twice, then `git status --porcelain docs .ai` | empty | exit 0 both times; status empty after each |
| `cargo xtask wave check` | OK | exit 0; 78 open tickets in 19 waves, 1247 parked at wave 0 |
| `cargo run -p developer-tools --bin enf -- --help` | defaults unchanged | exit 0; `citations` still defaults to `docs/mod` and `.ai/artifacts/enf-index` |
| `cargo clippy -p ticket-engine -p xtask -p developer-tools -p verification-core -p ticketboard --all-targets -- -D warnings` | clean | exit 0 |
| `cargo fmt --check` | clean | exit 0 |
| `cargo check --workspace --locked` | clean | exit 0 |
| `cargo xtask verify file-length` | OK | exit 0; scanned 2543 `.rs` files, 0 violations |
| `cargo xtask schema specification-consistency` | OK | exit 0; 11 of 12 gates pass, gate 10 reports it had nothing to check |
| `cargo xtask ticket sparse-paths <a root-target ticket>` | names `tools_v2` and `.cargo`, never `scripts` or `xtask` | No ticket in the registry carries a `targets` field, so every ticket resolves to the default `website` set: `sparse-paths` on a live ticket prints `.github` and `apps/website`. The `root` set is asserted directly by `repository::tests::the_root_sparse_set_carries_the_task_surface`, and reading `SPARSE_CHECKOUT_SETS` shows `tools_v2` and `.cargo` present and neither `scripts` nor `xtask` |

### Found and fixed

- `tools_v2/ticket-engine/src/registry/typed_projection.rs:1-13` — the header described a deleted
  one-shot migrator and pointed at a file that no longer exists. It now says what the module does.
- `tools_v2/ticket-engine/src/registry/mod.rs:174-176` — a comment whose whole content was the
  removal of two functions. Deleted.
- `tools_v2/ticket-engine/src/registry/tests/shipping_status/…` wrote its scratch fixture as
  a file named like the deleted ticket monolith. Renamed to `shipping_status.json`.
- `tools_v2/xtask/src/commands/platform/slice_worktree.rs:1-45` — the header and the command
  constant's documentation described a byte-for-byte port of a deleted shell script, named two
  more deleted scripts, and stated a condition about that script's deletion that has been
  false since the script went. Rewritten to what the module does and why its guards exist.
- `tools_v2/xtask/src/commands/mod_ops/wave_execution.rs:1-21` — same class: a header describing a
  port, a deleted script and a deleted plan format. Rewritten.
- `tools_v2/xtask/src/commands/mod_ops/playtest_server/usage_fail.rs:145-147` — a comment claiming
  the root walk looks for the deleted registry file. It looks for the root marker.
- `.coding-standards-allowlist.yaml:6` — said the frontend's oversized files "are split in Phase
  3C", a phase name that means nothing in this tree. It now states the fact.
- `tools_v2/PHASE_FIVE_HANDOFF.md` — the test inventory had drifted from the tree: tests deleted
  with the finished migrations, tests relocated out of them, tests added by the script-elimination
  pass, renames from the public-surface pass, and one documentation-test line number. It is
  regenerated from the live list, and the section is named `Test inventory`, since its stated
  purpose is to keep naming the tests that exist.
- `apps/ticketboard/` carried eighteen inline test modules — the Law 7 violation named above, fixed
  with the extraction.

### Found for P8

- `tools_v2/xtask/src/commands/deploy/staging/agent.rs:203-216` — `API_SLICE_SPEC` is an
  `#[allow(dead_code)]` constant holding a specification for work in another crate, kept only so a
  text search finds it, and its documentation attributes it to a deleted shell script by line
  number. It also names `docs/website/HOME_SERVER.md:282`, a documentation pin with a line number.
  Either delete the constant and let the specification live in a ticket, or move it to one; the
  `#[allow(dead_code)]` says plainly that nothing reads it.
- `tools_v2/xtask/src/verifications/schemas/checks/specification_consistency.rs` names one program
  identifier (`MAP_TERRAIN_PROGRAM`) and reads specification files whose names carry that
  identifier. The constant is the subject of the check rather than provenance, so the prose rule
  needs the same explicit carve-out the retained names get, or the check needs to resolve the
  program from the registry by another key.
- `tools_v2/developer-tools/src/map_raster_pipeline/cartographic_rendering/build_tile_pyramid.rs:151`
  reads committed verify logs named after a ticket (`t152_<n>_verify_log.md`). The logs are frozen
  records; the identifier in the production `format!` is not.
- `tools_v2/ticket-engine/src/cli/mutations.rs:121,153,155` and `cli/readiness.rs:21` qualify the
  refusal strings with an adjective the prose rule forbids. The live fact is that those exact
  strings are the refusals; the adjective is the only thing that has to go.
- `tools_v2/ticket-engine/src/registry/typed_projection.rs:126-132,196-205` — the documentation on
  the Value write path still carries ticket identifiers as provenance and a measured incident
  narrated in the past tense. The invariants under both (the write path refuses, the stale-file
  pass is gone) are what to keep.

### Found for P9

- `apps/ticketboard/Cargo.toml:1-11` and `src/main.rs` name the crate's design document and its
  shipping ticket in their headers, which the root-document pass covers.

### Found for P10

- The verification matrix's historical-registry-file row cannot be read verbatim: three live files carry
  that name (the mod's object registry, the terrain registry and a frontend API golden). Restricting it to the ticket directory's own file name gives the
  ticket-registry answer, which is 0.
- `cargo xtask verify file-length` now scans 2543 `.rs` files against the 2529 recorded at the
  baseline. The rise is the test files the last two phases extracted from inline modules, not a
  widened walk: the pinned directories are unchanged.

---

## P8 — Present-tense prose

Every comment, doc comment, help string, README and landing record under `tools_v2` now states
what the code does now and why. A structural test keeps it that way.

### What changed

**The sources.** 1,489 lines across 349 files carried a ticket identifier: 1,307 comment lines,
165 production lines that were not comments, and 17 lines of documents and manifests. Each was
rewritten to carry the fact the identifier was cited for — the measured number, the rule, the
refusal reason — or deleted where the identifier was the whole content. The heaviest files were
the wave-execution lifecycle (`touch`, `db`, `lock`, `migrate`, `push`, `schema`, `base`,
`ledger`, `flush`, `land` and `reclaim`), `slice_worktree`, `schemas/checks/wire_field_readers.rs`,
`ci/task_definitions.rs` and `task_runner.rs`, every `commands/*/cli.rs`, `verifications/mod_scripts`,
`verifications/language_bans`, `commands/deploy`, `commands/mcp`, `commands/mod_ops`,
`commands/setup`, `commands/db`, the browser-testing diagnostics and capture lanes, the world
export and map raster pipelines, the blueprint lane, and the ticket engine's validation, cli,
wave-lock and registry modules.

**Corpus data left the source.** Three ticket-identifier tables were data about this repository's
ticket corpus rather than code, and they now live beside the corpus they describe, in
`.ai/tickets/corpus-pins.toml`, read fail-closed by `ticket_engine::corpus_pins`:

| Pin | Read by |
|---|---|
| `game_mod_programme_ticket` | `cargo xtask mod wave`, which filters the shared lock to that programme's dotted children. A missing or malformed pin file is a refusal, because an empty programme id would claim every other programme's rows. |
| `map_terrain_programme_ticket` | `cargo xtask schema specification-consistency`, for the specification file prefix, the first-slice claim and the hub header's active slice. |
| `never_minted` | `cargo xtask ticket check`, which reds on a ticket row carrying one of those ids. |
| `gap_implementations` | `cargo xtask ticket sync`, for the gap rows no ticket claims through its own `implements` list. |

`FROZEN_UNMAPPABLE` went with them, deleted rather than moved: it had no reader in the workspace
beyond a test asserting its own length.

**Names that were ticket-shaped became names.** The ten in-process CI adapters are `run_<check>`
rather than `x_<check>`. The `UnreadField` rows lost their `ticket` field; the message now names
the baseline's reason, which is the actionable half. The satellite export's `slice` stamp carries
the stage that produced it (`spike-subregion-export`, `world-object-build`, `density-grid-build`,
`density-redensify`, `aerial-cell-catalog`) in the writers, in the validator and in the committed
`map_export_everon.json`. A contract fixture is `terrain-manifest-everon-tile-only-satellite.json`,
after what it holds.

**Three mod comment contracts moved with their sources.** `TBD_PlayerIdentity.c`,
`TBD_ResultsReporter.c` and `TBD_MissionLoader.c` carried ticket identifiers in the exact comment
text three verifications pin. Source and pin were rewritten in one commit: the bans now name the
retired phrasing rather than a ticket, the truth pins name the shipped behaviour, and
`cargo xtask mod compile` is clean.

**The documents.** `ANALYSIS_AND_INVENTORY.md` is an inventory of what exists — crate, module,
responsibility — instead of a migration mapping. `ARCHITECTURE_PLAN.md` is the architecture: the
four crates and the node package, the dependency direction with the rule that asserts each edge,
the invariants, the layout modules and the verification surface. The four landing records keep
their measurements and lose the phase narrative, the ticket identifiers and the dead names. This
document was rewritten under the same rules: where an earlier section quoted a retired spelling to
show what changed, it now states the live name and the count of retired spellings.

**The test.** `tools_v2/xtask/src/tests/tooling_prose_rules.rs`, declared from `main.rs` (28
lines), walks `git ls-files tools_v2` and asserts eight rules, printing every offending
`path:line` on failure:

| Rule | Subject |
|---|---|
| Production `.rs` files carry no ticket identifier | anywhere in the file |
| Test `.rs` files carry none on comment lines | string literals may carry synthetic ids — that is the ticket domain's own test data |
| `.md`, `.toml` and `.json` carry none | outside the four fixture trees |
| No file names a retired spelling | the whole tracked tree |
| No file names a shell, Python or Node source | outside the language-ban tests, which synthesise the offenders their gates catch, and outside the host control agent's own file name |
| Only a layout module spells a repository path | production `.rs` only; a `.pak` archive's internal script tree and an Enfusion diagnostic are named as what they are, not repository paths |
| Nothing narrates its own history | the whole tracked tree |
| Every `.rs` file named in prose exists | production `.rs` and `.md` |

The retired spellings and the history words are held in halves and joined at runtime, so the rules
file does not match its own needles — the discipline `ticket_engine::validation::references`
already uses for its fossil-path guard. `every_rule_fires_on_a_line_that_breaks_it` assembles a
fixture the same way and asserts each pattern matches exactly the line that breaks it, so a green
suite means the rules looked.

### Acceptance

| Row | What it counts | Before | After |
|---|---|---|---|
| R1 | Ticket identifiers in `tools_v2` sources, manifests and documents, fixture trees excluded | 2,087 lines: 1,307 comment lines, 165 production non-comment lines, 17 in documents and manifests, 598 test-file string literals | 596 lines, every one a string literal in a test file: **0** comment lines, **0** production lines, **0** in documents and manifests |
| R1b | Ticket identifiers and node script names in the browser-oracle freeze manifest | 0 | 0 |
| R2 | Dead names anywhere under `tools_v2` | 84 | 0 |
| R3 | Shell, Python and Node file names in `tools_v2` sources, documents and manifests, language-ban tests excluded | 257 | 9, every one the host control agent the staging deploy renders onto the game host and `apps/website/api_v2/tests/game_agent_rcon.rs` asserts by name; **0** otherwise |
| R7 | Words that narrate a change rather than the present state | 83 | 0 |
| R18 | Distinct `.rs` basenames named in production prose that exist nowhere in the workspace | 171 | 0 |

| Command | Expected | Actual |
|---|---|---|
| `cargo test -p xtask tooling_prose_rules` | green | exit 0; 9 passed |
| `cargo test -p xtask -p developer-tools -p verification-core -p ticket-engine -p ticketboard` | green | exit 0; xtask 661, developer-tools 259 and 4 ignored, verification-core 68, ticket-engine 206 plus 1 compile-failure test, ticketboard 170 and 3 ignored |
| `cargo clippy -p xtask -p developer-tools -p verification-core -p ticket-engine -p ticketboard --all-targets -- -D warnings` | clean | exit 0 |
| `cargo fmt --all --check` | clean | exit 0 |
| `cargo doc -p verification-core -p developer-tools --no-deps` | builds without warnings | exit 0; zero warnings after one unclosed-HTML-tag doc comment was fenced |
| `cargo check --workspace --locked` | passes | exit 0 |
| `cargo xtask verify ci-schema-parity` | PASS | `ci-schema-parity: PASS` |
| `cargo xtask verify file-length` | OK | `scanned 2546 .rs file(s), 0 violation(s)` |
| `cargo xtask verify no-shell` / `verify no-node` | OK | hard zero over 12,868 tracked paths; Node exists solely as the MCP runtime |
| `cargo xtask schema validate` | all contracts valid | `All contracts valid.` |
| `cargo xtask ticket check` | OK | `check OK` |
| `cargo xtask mod compile` | green | `OK: compiled clean`; 5,804 files, 11,643 classes, 0 warnings in TBD sources |
| `cargo xtask verify player-identity-comments`, `verify results-reporter-identity-comments`, `verify mission-rest-size-limits`, `verify destroy-target-diagnostics` | PASS with their RED proofs | all four PASS, every reintroduced-lie and removed-pin proof failing as expected |

### Found and fixed

- `tools_v2/ticket-engine/src/model/tickets.rs:123` — `FROZEN_UNMAPPABLE` had no reader in the
  workspace: one re-export and one test asserting its own length. Deleted with both.
- `tools_v2/xtask/src/commands/ci/task_definitions/verification_dispatch.rs` — the ten adapters
  were `x_<check>`; one of them carried the name of the file the previous phase retired. All ten
  are `run_<check>` now, and the module's two `#[path]` declarations sit together at the top,
  which keeps the table file under the line limit.
- `tools_v2/xtask/src/verifications/language_bans/node_and_file_limits/repository_access.rs:144`
  — `is_test_file` matched a string suffix. It reads the path's components and file stem instead,
  which is both the correct test and free of the literal.
- `tools_v2/developer-tools/src/world_export_pipeline/export_preparation/export_validation/operations_log.rs`
  — the spike-log validator compared against a literal the in-tree writers never produce. The
  stage name is a named constant, and the committed operations log carries it.
- `tools_v2/developer-tools/src/world_export_pipeline/mod.rs:26` — the `INSTANCE_KINDS` doc
  narrated a three-copy history and named two Rust files absent from the crate. It states the
  invariant: one const per census bucket order, a classified prefab with no bucket is a hard
  failure, and the second copy is the schema check's, which may not read this one.
- `tools_v2/developer-tools/src/world_export_pipeline/export_preparation/aerial_cell_catalog.rs:4`
  — an unfenced `<terrain>` placeholder was the crate's only rustdoc warning.
- `tools_v2/xtask/src/commands/deploy/website/remote_steps.rs:68` — the remote state migration
  looped over a variable named for an era rather than its contents; it iterates `tree` now, and
  the test that pins the rendered command moves with it.
- One map contract fixture was named for an era rather than its content. It is
  `contracts_v2/fixtures/map/terrain-manifest-everon-tile-only-satellite.json`, after the manifest
  shape it holds, and its two readers moved with it.
- `tools_v2/xtask/src/commands/deploy/staging/tests/agent/tests.rs` — the byte-for-byte unit
  render pinned a ticket identifier inside a systemd `Description=`. The units and the pin lost it
  together.
- `apps/mod/tbd-framework/Scripts/Game/TBD/API/TBD_PlayerIdentity.c`,
  `TBD_ResultsReporter.c` and `Systems/Mission/Loaders/TBD_MissionLoader.c` — six comment lines
  carried ticket identifiers that three verifications pin verbatim. Source and pins rewritten
  together; `cargo xtask mod compile` re-run.

### Found for P9

- `tools_v2/xtask/src/commands/deploy/staging/agent.rs:203-216` — `API_SLICE_SPEC` is an
  `#[allow(dead_code)]` constant holding a specification for work in another crate, kept only so a
  text search finds it. Its prose is now present-tense and identifier-free, but the constant still
  has no reader; the `#[allow(dead_code)]` says so. Either delete it and let the specification live
  in a ticket, or move it to one.
- `tools_v2/verification-core/src/scan.rs:95` exports `grep_lines`, whose name borrows an external
  tool for a function that runs no process. It is public API across `xtask` and `developer-tools`,
  so renaming it is a public-surface change rather than a prose one.
- `TBD_RUN_T092_SMOKE` is a live operator-facing environment variable and a matching field name in
  `deploy/staging/config.rs`, documented in `docs/mod/STAGING-SERVER.md`. It is ticket-shaped but
  matches no rule here — `T092` carries no hyphen — so renaming it belongs with the other
  operator-surface renames, in one commit with the runbook.
- `tools_v2/developer-tools/src/browser_testing/editor_smoke_tests/outliner_drag/execution.rs:18`
  writes a ticket-named page global that `outliner_drag/vehicle_snap_cases.rs:40` reads back. Both
  ends are inside `developer-tools`, so the rename is one-sided; `window.__outlinerDragEvents` is
  the name the rest of the lane would use.
- `docs/mod/STAGING-SERVER.md:202,373` and `.ai/artifacts/t128_doc_link_repair_log.md:56` name a
  deleted staging script and the environment variable above. The first is a live runbook.
