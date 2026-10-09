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
use map_asset_steps::{MAP_CARTOGRAPHIC_EVERON_STEPS, MAP_WATER_EVERON_STEPS};
use repository_checks::language_bans::node_and_file_limits::{verify_file_length, verify_no_node};
use repository_checks::language_bans::python_scripts::verify_no_python;
use repository_checks::language_bans::shell_scripts::verify_no_shell;
use schema_tooling::codegen;
use schema_tooling::{map_glyphs, map_object_enums, type_inventory, validate_all};
use verification_dispatch::{
    run_height_labels, run_map_object_golden, run_terrain_alignment, run_terrain_alignment_strict,
    run_terrain_manifest,
};
use workspace_law_steps::WORKSPACE_LAW_STEPS;

/// Every task `cargo xtask ci` runs: composites first, then leaves, aliases and borrowed rows.
pub static TASKS: &[Task] = &[
    // ── composites ──────────────────────────────────────────────────────────────────────────
    Task {
        name: "ci-local",
        help: "Full CI gate locally — the cargo, schema and repository-law jobs of ci.yml (run `cargo xtask db up` first)",
        group: "CI",
        lane: Lane::Ci,
        steps: &[
            Step::Task("rust-ci"),
            Step::Task("workspace-member-tests"),
            Step::Task("ci-local-leptos"),
            Step::Task("ci-local-schema"),
            Step::Task("verify-workspace-laws"),
            Step::Task("verify-language-bans"),
            Step::Task("verify-file-length"),
        ],
    },
    Task {
        name: "ci-local-schema",
        help: "CI gate: generated-byte freshness + schema validation (TEST-3)",
        group: "CI",
        lane: Lane::Ci,
        steps: &[
            Step::Task("verify-codegen-fresh"),
            Step::Task("schema-validate"),
        ],
    },
    Task {
        name: "schema-validate",
        help: "Validate golden missions + the map-object enums and type inventory",
        group: "schema",
        lane: Lane::Ci,
        steps: &[
            xt!("cargo xtask schema validate", false, || Ok(validate_all()?)),
            xt!("cargo xtask schema map-object-enums", false, || Ok(
                map_object_enums()?
            )),
            xt!("cargo xtask schema type-inventory", false, || Ok(
                type_inventory()?
            )),
        ],
    },
    Task {
        name: "schema-map-goldens",
        help: "On demand: the map-object golden, the glyph atlas and the height labels (needs the LFS Everon DEM: `cargo xtask ci lfs-dem`)",
        group: "schema",
        lane: Lane::Ci,
        steps: &[
            xt!(
                "cargo xtask schema map-object-golden",
                false,
                run_map_object_golden
            ),
            xt!("cargo xtask schema map-glyphs", false, || Ok(map_glyphs()?)),
            xt!("cargo xtask schema height-labels", false, run_height_labels),
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
        help: "Build and spawn api-server, then wait on /healthz (editor-gates.yml)",
        group: "CI",
        lane: Lane::Ci,
        steps: &[Step::Native {
            run: crate::editor_api::run,
        }],
    },
    Task {
        name: "api-test",
        help: "cargo test over api_server and every crates/api package (honours TEST_DATABASE_URL)",
        group: "build",
        lane: Lane::Ci,
        steps: &[Step::Native {
            run: crate::api_package_lane::run_api_test,
        }],
    },
    // Derived from the workspace: one `cargo test --workspace` excluding the families other rows test.
    Task {
        name: "workspace-member-tests",
        help: "One cargo test --workspace run, excluding the API and frontend families their own rows test",
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
        help: "Pull the Everon DEM from LFS (72 MB — height labels, terrain alignment, hillshade)",
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
            sh!("cd crates/api/api_server && cargo build --release --bin api-server"),
            Step::Task("leptos-build"),
        ],
    },
    // ── aliases: one-line wrappers on existing `cargo xtask verify …` commands ──────────────
    Task {
        name: "verify-language-bans",
        help: "LANG-1/2: no tracked Python, Node script or shell/Make path, and no node/npx or python3 invocation",
        group: "verify",
        lane: Lane::Alias,
        steps: &[
            xt!("cargo xtask verify no-python", false, || Ok(
                verify_no_python()?
            )),
            xt!(
                "cargo xtask verify no-node",
                false,
                || Ok(verify_no_node()?)
            ),
            xt!("cargo xtask verify no-shell", false, || Ok(
                verify_no_shell()?
            )),
        ],
    },
    Task {
        name: "verify-file-length",
        help: "File-length advice: warns about every production file over 500 lines, never fails on one",
        group: "verify",
        lane: Lane::Alias,
        steps: &[xt!("cargo xtask verify file-length", false, || Ok(
            verify_file_length()?
        ))],
    },
    Task {
        name: "verify-workspace-laws",
        help: "The enforced workspace laws: crate tiers, crate anatomy and Tailwind sources over the workspace members",
        group: "verify",
        lane: Lane::Alias,
        steps: WORKSPACE_LAW_STEPS,
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
        help: "Rust CI gate locally — fmt + clippy + wasm-ci + test-it (mirrors the ci.yml api job)",
        group: "build",
        lane: Lane::Borrowed,
        steps: &[
            Step::Task("rust-fmt"),
            Step::Task("rust-clippy"),
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
            sh!("cd crates/api/api_server && cargo fmt --check"),
            sh!("cargo fmt --all --check"),
        ],
    },
    Task {
        name: "rust-clippy",
        help: "Lint api_server and every crates/api package with clippy (deny warnings; GO-2..8 analog)",
        group: "build",
        lane: Lane::Borrowed,
        steps: &[Step::Native {
            run: crate::api_package_lane::run_api_clippy,
        }],
    },
    Task {
        name: "rust-test",
        help: "Run the unit tests of api_server and every crates/api package (no DB)",
        group: "build",
        lane: Lane::Borrowed,
        steps: &[Step::Native {
            run: crate::api_package_lane::run_api_unit_tests,
        }],
    },
    Task {
        name: "wasm-ci",
        help: "Clippy every wasm32 crate outside the frontend family for wasm32 (ci-local-leptos lints the family)",
        group: "build",
        lane: Lane::Borrowed,
        // A wasm32 crate this step does not name goes unlinted for the browser. The lint derives
        // its packages from the workspace (`crate::wasm32_lint_lane`), so every crate declaring
        // `targets = "wasm32"` outside the frontend family is linted; the family itself, the
        // single-page app and the offline service worker among it, is formatted, linted for both
        // targets and tested once, by `ci-local-leptos`.
        steps: &[Step::Native {
            run: crate::wasm32_lint_lane::run_wasm_ci_lint,
        }],
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
            sh!("cd crates/frontend/shell/frontend_application && trunk build --release"),
        ],
    },
    Task {
        name: "leptos-build",
        help: "Release-build the Leptos SPA into crates/frontend/shell/frontend_application/dist",
        group: "build",
        lane: Lane::Borrowed,
        steps: &[sh!(
            "cd crates/frontend/shell/frontend_application && trunk build --release"
        )],
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
