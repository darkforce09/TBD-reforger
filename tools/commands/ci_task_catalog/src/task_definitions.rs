#[macro_use]
#[path = "task_definitions/recipe_macros.rs"]
mod recipe_macros;
#[path = "task_definitions/map_asset_steps.rs"]
mod map_asset_steps;
#[path = "task_definitions/verification_dispatch.rs"]
mod verification_dispatch;
pub(crate) use verification_dispatch::run_database_test_suite;
#[path = "task_definitions/workspace_law_steps.rs"]
mod workspace_law_steps;

use super::{Lane, Step, Task};
use crate::workflow_checks::workflow_shell::verify_ci_shell;
use map_asset_steps::{MAP_CARTOGRAPHIC_EVERON_STEPS, MAP_WATER_EVERON_STEPS};
use repository_checks::language_bans::node_and_file_limits::{verify_file_length, verify_no_node};
use repository_checks::language_bans::python_scripts::verify_no_python;
use repository_checks::language_bans::shell_scripts::verify_no_shell;
use repository_layout::find_repository_root;
use schema_tooling::codegen;
use schema_tooling::{citations, map_glyphs, map_object_enums, type_inventory, validate_all};
use verification_dispatch::{
    run_ci_schema_parity, run_enfusion_comments, run_height_labels, run_link_check,
    run_map_object_golden, run_markdown_placement, run_mission_rest_size_limits,
    run_no_select_star, run_readme_coverage, run_route_tags, run_staging_compose_paths,
    run_terrain_alignment, run_terrain_alignment_strict, run_terrain_manifest,
};
use workspace_law_steps::WORKSPACE_LAW_STEPS;

/// Every task `cargo xtask ci` runs: composites first, then leaves, aliases and borrowed rows.
pub static TASKS: &[Task] = &[
    // ── composites ──────────────────────────────────────────────────────────────────────────
    Task {
        name: "ci-local",
        help: "Full CI gate locally — mirrors ci.yml (run `cargo xtask db up` first)",
        group: "CI",
        lane: Lane::Ci,
        // Invoke parity verification directly: it checks dispatcher bodies and cannot rely
        // on the dispatcher it checks to reach its own validation. ci.yml does the same.
        steps: &[
            Step::Task("verify-editorconfig"),
            Step::Task("verify-no-python"),
            Step::Task("verify-no-node"),
            Step::Task("verify-no-shell"),
            Step::Task("verify-ci-shell"),
            Step::Task("verify-workspace-laws"),
            Step::Task("rust-ci"),
            Step::Task("workspace-member-tests"),
            Step::Task("verify-coding-standards"),
            Step::Task("verify-documentation"),
            Step::Task("ci-local-leptos"),
            Step::Task("ci-local-schema"),
            Step::Task("verify-staging-compose-paths"),
            Step::Task("verify-mission-rest-size-limits"),
            xt!(
                "cargo xtask verify ci-schema-parity",
                true,
                run_ci_schema_parity
            ),
        ],
    },
    Task {
        name: "ci-local-schema",
        help: "CI gate: generated-byte freshness + schema validation (TEST-3) + contract citations",
        group: "CI",
        lane: Lane::Ci,
        steps: &[
            Step::Task("verify-codegen-fresh"),
            Step::Task("schema-validate"),
            Step::Task("verify-citations"),
        ],
    },
    Task {
        name: "schema-validate",
        help: "Validate golden missions + map-object contracts (enums + glyphs + type inventory) + height labels",
        group: "schema",
        lane: Lane::Ci,
        steps: &[
            xt!("cargo xtask schema validate", false, || Ok(validate_all()?)),
            xt!(
                "cargo xtask schema map-object-golden",
                false,
                run_map_object_golden
            ),
            xt!("cargo xtask schema map-glyphs", false, || Ok(map_glyphs()?)),
            xt!("cargo xtask schema height-labels", false, run_height_labels),
            xt!("cargo xtask schema map-object-enums", false, || Ok(
                map_object_enums()?
            )),
            xt!("cargo xtask schema type-inventory", false, || Ok(
                type_inventory()?
            )),
        ],
    },
    Task {
        name: "schema-codegen",
        help: "Regenerate the contract_schema_types crate from contracts/definitions via typify (loadout_projection.rs is hand-maintained)",
        group: "schema",
        lane: Lane::Ci,
        steps: &[xt!("cargo xtask schema codegen", false, || Ok(codegen()?))],
    },
    Task {
        name: "verify-citations",
        help: "Verify @contract citations in code under every workspace member's top-level folder (apps/, crates/, tools/) and apps/mod/ — NOT documentation/ prose (documentation/standards/documentation_standards.md §10)",
        group: "schema",
        lane: Lane::Ci,
        steps: &[xt!("cargo xtask schema citations", false, || Ok(
            citations()?
        ))],
    },
    Task {
        name: "verify-coding-standards",
        help: "SIZE file length + Enfusion comment card + no SELECT * + GO-7 @route/router match (documentation/standards/coding_standards/README.md §11)",
        group: "verify",
        lane: Lane::Ci,
        steps: &[
            xt!("cargo xtask verify file-length", true, || Ok(
                verify_file_length()?
            )),
            xt!(
                "cargo xtask verify enfusion-comments",
                true,
                run_enfusion_comments
            ),
            xt!(
                "cargo xtask verify no-select-star",
                true,
                run_no_select_star
            ),
            xt!("cargo xtask verify route-tags", true, run_route_tags),
        ],
    },
    Task {
        name: "verify-documentation",
        help: "README coverage + Contents, links + cited paths and commands, Markdown placement + size — over the committed tree",
        group: "verify",
        lane: Lane::Ci,
        steps: &[
            xt!(
                "cargo xtask verify readme-coverage",
                false,
                run_readme_coverage
            ),
            xt!("cargo xtask verify link-check", false, run_link_check),
            xt!(
                "cargo xtask verify markdown-placement",
                false,
                run_markdown_placement
            ),
        ],
    },
    Task {
        name: "verify-editorconfig",
        help: "FMT-2: run editorconfig-checker from repo root (documentation/standards/coding_standards/README.md §7)",
        group: "verify",
        lane: Lane::Ci,
        steps: &[Step::Native {
            run: crate::editor_api::verify_editorconfig,
        }],
    },
    Task {
        name: "verify-codegen-fresh",
        help: "Compare generated files with expected schema bytes without writing files or using Git",
        group: "schema",
        lane: Lane::Ci,
        steps: &[Step::Native {
            run: crate::editor_api::verify_codegen_fresh,
        }],
    },
    Task {
        name: "ci-chrome",
        help: "Install the pinned Chrome-for-Testing build (editor-gates.yml)",
        group: "CI",
        lane: Lane::Ci,
        steps: &[Step::Native {
            run: crate::chromium_install::run,
        }],
    },
    Task {
        name: "editor-api-boot",
        help: "Build and spawn api, then wait on /healthz (editor-gates.yml)",
        group: "CI",
        lane: Lane::Ci,
        steps: &[Step::Native {
            run: crate::editor_api::run,
        }],
    },
    Task {
        name: "api-test",
        help: "cargo test over api and every crates/api package (honours TEST_DATABASE_URL)",
        group: "build",
        lane: Lane::Ci,
        steps: &[Step::Native {
            run: crate::api_package_lane::run_api_test,
        }],
    },
    // Derived from the workspace: every member no task above tests, one `cargo test -p` each.
    Task {
        name: "workspace-member-tests",
        help: "cargo test -p <package> for every workspace member no dedicated task tests (derived from Cargo.toml)",
        group: "build",
        lane: Lane::Ci,
        steps: &[Step::Native {
            run: crate::workspace_member_tests::run,
        }],
    },
    // ── map lane ────────────────────────────────────────────────────────────────────────────
    Task {
        name: "map-water-everon",
        help: "One-button Everon water composite: restore → mask → composite → bundle + pyramid → verify",
        group: "map",
        lane: Lane::Ci,
        steps: MAP_WATER_EVERON_STEPS,
    },
    Task {
        name: "map-cartographic-everon",
        help: "One-button Everon Map view (stylized cartographic): staging ortho → pyramid → manifest patch → verify",
        group: "map",
        lane: Lane::Ci,
        steps: MAP_CARTOGRAPHIC_EVERON_STEPS,
    },
    Task {
        name: "map-cartographic-verify",
        help: "Verify the Everon Map pyramid (complete z0–6 + manifest agreement)",
        group: "map",
        lane: Lane::Ci,
        steps: &[sh!(
            "cargo run -q -p developer_tools --bin map -- verify-pyramid --terrain everon --view-map"
        )],
    },
    Task {
        name: "lfs-dem",
        help: "Pull the Everon DEM from LFS (72 MB — terrain and world-object tests + hillshade)",
        group: "map",
        lane: Lane::Ci,
        steps: &[sh!(
            "git lfs pull --include assets/terrains/everon/dem/everon-dem-16bit.png"
        )],
    },
    Task {
        name: "lfs-sat",
        help: "Pull the Everon satellite bundle from LFS (153 MB — full-res editor basemap)",
        group: "map",
        lane: Lane::Ci,
        steps: &[sh!(
            "git lfs pull --include assets/terrains/everon/satellite/everon-sat.tbd-sat"
        )],
    },
    // ── build / test entry points ───────────────────────────────────────────────────────────
    Task {
        name: "test",
        help: "Run backend unit tests",
        group: "build",
        lane: Lane::Ci,
        steps: &[Step::Task("rust-test")],
    },
    Task {
        name: "build",
        help: "Build the backend + the Leptos SPA",
        group: "build",
        lane: Lane::Ci,
        steps: &[
            sh!("cd apps/api && cargo build --release --bin api"),
            Step::Task("leptos-build"),
        ],
    },
    // ── aliases: one-line wrappers on an existing `cargo xtask verify …` command ────────────
    Task {
        name: "verify-no-python",
        help: "LANG-2 hard zero — same TrackedLanguageBan table as verify-no-shell (.py / python3)",
        group: "verify",
        lane: Lane::Alias,
        steps: &[xt!("cargo xtask verify no-python", false, || Ok(
            verify_no_python()?
        ))],
    },
    Task {
        name: "verify-no-node",
        help: "zero tracked Node script files (mjs/cjs); no node/npx invocation in a scanned file",
        group: "verify",
        lane: Lane::Alias,
        steps: &[xt!("cargo xtask verify no-node", false, || Ok(
            verify_no_node()?
        ))],
    },
    Task {
        name: "verify-workspace-laws",
        help: "WS-1 to WS-5, the workspace laws — crate tiers, crate anatomy, the strangler rule, frontend layering (hard at zero: any violation fails) and Tailwind sources over the workspace members",
        group: "verify",
        lane: Lane::Alias,
        steps: WORKSPACE_LAW_STEPS,
    },
    Task {
        name: "verify-no-shell",
        help: "LANG-1 hard zero — no tracked shell/Make/Python/Node-script paths",
        group: "verify",
        lane: Lane::Alias,
        steps: &[xt!("cargo xtask verify no-shell", false, || Ok(
            verify_no_shell()?
        ))],
    },
    Task {
        name: "verify-ci-shell",
        help: "Every GitHub Actions run: is cargo xtask or a short pre-cargo allowlist",
        group: "verify",
        lane: Lane::Alias,
        steps: &[xt!("cargo xtask verify ci-shell", false, verify_ci_shell)],
    },
    Task {
        name: "verify-staging-compose-paths",
        help: "deploy staging resolves the compose file under website/, not api/",
        group: "verify",
        lane: Lane::Alias,
        steps: &[xt!(
            "cargo xtask verify staging-compose-paths",
            true,
            run_staging_compose_paths
        )],
    },
    Task {
        name: "verify-mission-rest-size-limits",
        help: "mission REST body size gate runs before ParseMissionJson",
        group: "verify",
        lane: Lane::Alias,
        steps: &[xt!(
            "cargo xtask verify mission-rest-size-limits",
            true,
            run_mission_rest_size_limits
        )],
    },
    Task {
        name: "verify-terrain",
        help: "Manifest + anchor verify (stub mode OK for Arland-only)",
        group: "verify",
        lane: Lane::Ci,
        steps: &[
            xt!(
                "cargo xtask schema terrain-manifest --terrain everon",
                false,
                run_terrain_manifest
            ),
            xt!(
                "cargo xtask schema terrain-alignment --terrain everon",
                false,
                run_terrain_alignment
            ),
        ],
    },
    Task {
        name: "verify-terrain-strict",
        help: "Full anchor alignment gate (GetSurfaceY DEM + anchors)",
        group: "verify",
        lane: Lane::Ci,
        steps: &[
            xt!(
                "cargo xtask schema terrain-manifest --terrain everon",
                false,
                run_terrain_manifest
            ),
            xt!(
                "cargo xtask schema terrain-alignment --terrain everon --strict",
                false,
                run_terrain_alignment_strict
            ),
        ],
    },
    // ── borrowed rows: the build lane and the db lane, carried so the composites above run. ──
    Task {
        name: "rust-ci",
        help: "Rust CI gate locally — fmt + clippy + build + test-it (mirrors the ci.yml rust-backend job)",
        group: "build",
        lane: Lane::Borrowed,
        steps: &[
            Step::Task("rust-fmt"),
            Step::Task("rust-clippy"),
            Step::Task("rust-build"),
            Step::Task("wasm-ci"),
            Step::Task("rust-test-it"),
        ],
    },
    Task {
        name: "rust-fmt",
        help: "Check Rust formatting (FMT-1 analog); workspace --all covers every tools crate",
        group: "build",
        lane: Lane::Borrowed,
        steps: &[
            sh!("cd apps/api && cargo fmt --check"),
            sh!("cargo fmt --all --check"),
        ],
    },
    Task {
        name: "rust-clippy",
        help: "Lint api and every crates/api package with clippy (deny warnings; GO-2..8 analog)",
        group: "build",
        lane: Lane::Borrowed,
        steps: &[Step::Native {
            run: crate::api_package_lane::run_api_clippy,
        }],
    },
    Task {
        name: "rust-build",
        help: "Build api and every crates/api package (all targets)",
        group: "build",
        lane: Lane::Borrowed,
        steps: &[Step::Native {
            run: crate::api_package_lane::run_api_build,
        }],
    },
    Task {
        name: "rust-test",
        help: "Run the unit tests of api and every crates/api package (no DB)",
        group: "build",
        lane: Lane::Borrowed,
        steps: &[Step::Native {
            run: crate::api_package_lane::run_api_unit_tests,
        }],
    },
    Task {
        name: "wasm-ci",
        help: "Fmt + clippy + test the offline service worker crate; clippy every wasm32 crate for wasm32",
        group: "build",
        lane: Lane::Borrowed,
        // A crate these steps do not name goes ungated: built only as a dependency, never
        // formatted, linted or tested. The offline service worker is named in every step
        // because the browser half is where it ships; the wasm32 lint
        // derives its packages from the workspace (`crate::wasm32_lint_lane`), so every crate
        // declaring `targets = "wasm32"` is linted for the browser too.
        steps: &[
            sh!("cargo fmt --check -p offline_service_worker"),
            sh!(
                "cargo clippy -p offline_service_worker --all-targets --all-features -- -D warnings"
            ),
            Step::Native {
                run: crate::wasm32_lint_lane::run_wasm_ci_lint,
            },
            sh!("cargo test -p offline_service_worker --all-features"),
        ],
    },
    Task {
        name: "ci-local-leptos",
        help: "CI gate: Leptos SPA and every crates/frontend crate fmt + clippy -D warnings (wasm32 and native, --all-targets) + native tests, then the trunk release build (mirrors the ci.yml frontend job)",
        group: "build",
        lane: Lane::Borrowed,
        // The four cargo lines name the frontend family, derived from the workspace
        // (`crate::frontend_package_lane`), so a frontend crate is gated from its first commit.
        steps: &[
            Step::Native {
                run: crate::frontend_package_lane::run_frontend_format,
            },
            Step::Native {
                run: crate::frontend_package_lane::run_frontend_wasm32_clippy,
            },
            Step::Native {
                run: crate::frontend_package_lane::run_frontend_native_clippy,
            },
            Step::Native {
                run: crate::frontend_package_lane::run_frontend_tests,
            },
            sh!("cd apps/frontend && trunk build --release"),
        ],
    },
    Task {
        name: "leptos-build",
        help: "Release-build the Leptos SPA into apps/frontend/dist",
        group: "build",
        lane: Lane::Borrowed,
        steps: &[sh!("cd apps/frontend && trunk build --release")],
    },
    Task {
        name: "rust-test-it",
        help: "Run Rust integration tests against a fresh dedicated DB (needs `cargo xtask db up` @ :5434)",
        group: "db",
        lane: Lane::Borrowed,
        // The database lane's own command, in process: a fresh database per run, the container
        // reached through the runtime the lane resolves, and the cleanup after every outcome.
        steps: &[xt!(
            "cargo xtask db test-it",
            false,
            verification_dispatch::run_database_test_suite
        )],
    },
];
