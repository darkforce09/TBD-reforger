#[macro_use]
#[path = "task_definitions/recipe_macros.rs"]
mod recipe_macros;
#[path = "task_definitions/verification_dispatch.rs"]
mod verification_dispatch;

use super::{Lane, Step, Task, verify_doc_layout};
use crate::commands::generate::schema_types::codegen;
use crate::core::repository_root::find_repo_root;
use crate::verifications::ci::workflow_shell::verify_ci_shell;
use crate::verifications::language_bans::node_and_file_limits::{
    verify_file_length, verify_no_node,
};
use crate::verifications::language_bans::python_scripts::verify_no_python;
use crate::verifications::language_bans::shell_scripts::verify_no_shell;
use crate::verifications::map_assets::map_object_golden;
use crate::verifications::schemas::checks::{
    citations, map_glyphs, map_object_enums, type_inventory, validate_all,
};
use verification_dispatch::{
    run_ci_schema_parity, run_engine_layers, run_height_labels, run_mission_rest_size_limits,
    run_no_select_star, run_route_tags, run_staging_compose_paths, run_terrain_alignment,
    run_terrain_alignment_strict, run_terrain_manifest,
};

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
            // Grouped with the language gates rather than the build lanes: like them it is a
            // seconds-long source scan, and it guards a one-way wall (map-engine ->
            // graphics-engine) that nothing in the compiler enforces.
            // documentation_v2/standards/engine_boundary_rules.md §5 requires it here and in
            // ci.yml — a rule nobody is stopped by is not a rule.
            Step::Task("verify-engine-layers"),
            Step::Task("rust-ci"),
            Step::Task("verify-coding-standards"),
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
            xt!("cargo xtask schema validate", false, validate_all),
            xt!(
                "cargo xtask schema map-object-golden",
                false,
                map_object_golden
            ),
            xt!("cargo xtask schema map-glyphs", false, map_glyphs),
            xt!("cargo xtask schema height-labels", false, run_height_labels),
            xt!(
                "cargo xtask schema map-object-enums",
                false,
                map_object_enums
            ),
            xt!("cargo xtask schema type-inventory", false, type_inventory),
        ],
    },
    Task {
        name: "schema-codegen",
        help: "Regenerate Rust contract types from contracts_v2/definitions via typify (loadout_projection.rs is hand-maintained)",
        group: "schema",
        lane: Lane::Ci,
        steps: &[xt!("cargo xtask schema codegen", false, codegen)],
    },
    Task {
        name: "verify-citations",
        help: "Verify @contract citations in apps/ and tools_v2/ code — NOT documentation_v2/ prose (documentation_v2/standards/documentation_standards.md §10)",
        group: "schema",
        lane: Lane::Ci,
        steps: &[xt!("cargo xtask schema citations", false, citations)],
    },
    Task {
        name: "verify-coding-standards",
        help: "SIZE file length + doc layout + GO-7 @route/router match (documentation_v2/standards/coding_standards/README.md §11)",
        group: "verify",
        lane: Lane::Ci,
        steps: &[
            Step::Task("verify-doc-layout"),
            xt!("cargo xtask verify file-length", true, verify_file_length),
            xt!(
                "cargo xtask verify no-select-star",
                true,
                run_no_select_star
            ),
            xt!("cargo xtask verify route-tags", true, run_route_tags),
        ],
    },
    Task {
        name: "verify-doc-layout",
        help: "DOCUMENTATION_STANDARDS §8.2: no markdown spec trees under apps/**/docs, contracts_v2/**/docs or assets_v2/**/docs",
        group: "verify",
        lane: Lane::Ci,
        steps: &[Step::Native {
            run: verify_doc_layout,
        }],
    },
    Task {
        name: "verify-editorconfig",
        help: "FMT-2: run editorconfig-checker from repo root (documentation_v2/standards/coding_standards/README.md §7)",
        group: "verify",
        lane: Lane::Ci,
        steps: &[Step::Native {
            run: crate::commands::ci::editor_api::verify_editorconfig,
        }],
    },
    Task {
        name: "verify-codegen-fresh",
        help: "Compare generated files with expected schema bytes without writing files or using Git",
        group: "schema",
        lane: Lane::Ci,
        steps: &[Step::Native {
            run: crate::commands::ci::editor_api::verify_codegen_fresh,
        }],
    },
    Task {
        name: "ci-chrome",
        help: "Install the pinned Chrome-for-Testing build (editor-gates.yml)",
        group: "CI",
        lane: Lane::Ci,
        steps: &[Step::Native {
            run: crate::commands::ci::chromium_install::run,
        }],
    },
    Task {
        name: "editor-api-boot",
        help: "Build and spawn website-api, then wait on /healthz (editor-gates.yml)",
        group: "CI",
        lane: Lane::Ci,
        steps: &[Step::Native {
            run: crate::commands::ci::editor_api::run,
        }],
    },
    Task {
        name: "website-api-test",
        help: "cargo test in apps/website/api_v2 (honours TEST_DATABASE_URL)",
        group: "build",
        lane: Lane::Ci,
        steps: &[sh!("cd apps/website/api_v2 && cargo test")],
    },
    // The library suite runs without database, browser, or asset prerequisites.
    Task {
        name: "developer-tools-test",
        help: "cargo test -p developer-tools --lib (density:: + world:: unit tests; no DB/LFS)",
        group: "build",
        lane: Lane::Ci,
        steps: &[sh!("cargo test -p developer-tools --lib")],
    },
    // ── map lane ────────────────────────────────────────────────────────────────────────────
    Task {
        name: "map-water-everon",
        help: "One-button Everon water composite: restore → mask → composite → bundle + pyramid → verify",
        group: "map",
        lane: Lane::Ci,
        steps: &[
            // Coreutils `cp`, not `std::fs::copy`: the step's observable behaviour on a missing
            // source is cp's own "cannot stat" diagnostic, and assets_v2/scratch/ is gitignored,
            // so a miss is the COMMON path here.
            sh!(
                "cp assets_v2/scratch/everon/sap/everon-sap-ortho.pre-water.png assets_v2/scratch/everon/sap/everon-sap-ortho.png"
            ),
            sh!("cargo run -q -p developer-tools --bin map -- reset-water-meta --terrain everon"),
            sh!("cargo run -q -p developer-tools --bin map -- analyze-water"),
            sh!("cargo run -q -p developer-tools --bin map -- composite-water"),
            sh!(
                "cargo run -q -p developer-tools --bin map -- build-unified --input assets_v2/scratch/everon/sap/everon-sap-ortho.png --out assets_v2/terrains/everon/satellite/everon-sat.tbd-sat --terrain everon"
            ),
            sh!(
                "cargo run -q -p developer-tools --bin map -- patch-unified-bytes --terrain everon"
            ),
            sh!(
                "cargo run -q -p developer-tools --bin map -- build-pyramid --input assets_v2/scratch/everon/sap/everon-sap-ortho.png --out assets_v2/terrains/everon/tiles/satellite --minzoom 0 --maxzoom 6 --tilesize 256 --lossless"
            ),
            sh!("cargo run -q -p developer-tools --bin map -- verify-sap-ortho --terrain everon"),
            sh!("cargo run -q -p developer-tools --bin map -- verify-unified --terrain everon"),
            sh!(
                "cargo run -q -p developer-tools --bin map -- verify-pyramid --terrain everon --expect-lossless"
            ),
        ],
    },
    Task {
        name: "map-cartographic-everon",
        help: "One-button Everon Map view (stylized cartographic): staging ortho → pyramid → manifest patch → verify",
        group: "map",
        lane: Lane::Ci,
        steps: &[
            sh!("cargo run -q -p developer-tools --bin map -- build-cartographic --terrain everon"),
            sh!(
                "cargo run -q -p developer-tools --bin map -- build-pyramid --input assets_v2/scratch/everon/map/everon-map-ortho.png --out assets_v2/terrains/everon/tiles/map --minzoom 0 --maxzoom 6 --tilesize 256"
            ),
            sh!(
                "cargo run -q -p developer-tools --bin map -- patch-map-tiles-meta --terrain everon"
            ),
            Step::Task("map-cartographic-verify"),
        ],
    },
    Task {
        name: "map-cartographic-verify",
        help: "Verify the Everon Map pyramid (complete z0–6 + manifest agreement)",
        group: "map",
        lane: Lane::Ci,
        steps: &[sh!(
            "cargo run -q -p developer-tools --bin map -- verify-pyramid --terrain everon --view-map"
        )],
    },
    Task {
        name: "lfs-dem",
        help: "Pull the Everon DEM from LFS (72 MB — map-engine tests + hillshade)",
        group: "map",
        lane: Lane::Ci,
        steps: &[sh!(
            "git lfs pull --include assets_v2/terrains/everon/dem/everon-dem-16bit.png"
        )],
    },
    Task {
        name: "lfs-sat",
        help: "Pull the Everon satellite bundle from LFS (153 MB — full-res editor basemap)",
        group: "map",
        lane: Lane::Ci,
        steps: &[sh!(
            "git lfs pull --include assets_v2/terrains/everon/satellite/everon-sat.tbd-sat"
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
            sh!("cd apps/website/api_v2 && cargo build --release --bin api"),
            Step::Task("leptos-build"),
        ],
    },
    // ── aliases: one-line wrappers on an existing `cargo xtask verify …` command ────────────
    Task {
        name: "verify-no-python",
        help: "LANG-2 hard zero — same TrackedLanguageBan table as verify-no-shell (.py / python3)",
        group: "verify",
        lane: Lane::Alias,
        steps: &[xt!("cargo xtask verify no-python", false, verify_no_python)],
    },
    Task {
        name: "verify-no-node",
        help: "zero tracked Node script files (mjs/cjs); no node/npx invocation in a scanned file",
        group: "verify",
        lane: Lane::Alias,
        steps: &[xt!("cargo xtask verify no-node", false, verify_no_node)],
    },
    Task {
        name: "verify-engine-layers",
        help: "§5 rules 1, 2, 3a, 3b, 4, 7 of documentation_v2/standards/engine_boundary_rules.md — graphics-engine imports no map engine and declares no map noun; map-engine names the frame vocabulary at one enumerated seam and no GPU module at all; data/scenario imports nothing outside itself; data/ and world/ name each other nowhere",
        group: "verify",
        lane: Lane::Alias,
        steps: &[xt!(
            "cargo xtask verify engine-layers",
            false,
            run_engine_layers
        )],
    },
    Task {
        name: "verify-no-shell",
        help: "LANG-1 hard zero — no tracked shell/Make/Python/Node-script paths",
        group: "verify",
        lane: Lane::Alias,
        steps: &[xt!("cargo xtask verify no-shell", false, verify_no_shell)],
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
        help: "Check Rust formatting (FMT-1 analog); workspace --all covers every tools_v2 crate",
        group: "build",
        lane: Lane::Borrowed,
        steps: &[
            sh!("cd apps/website/api_v2 && cargo fmt --check"),
            sh!("cargo fmt --all --check"),
        ],
    },
    Task {
        name: "rust-clippy",
        help: "Lint Rust with clippy (deny warnings; GO-2..8 analog)",
        group: "build",
        lane: Lane::Borrowed,
        steps: &[sh!(
            "cd apps/website/api_v2 && cargo clippy --all-targets -- -D warnings"
        )],
    },
    Task {
        name: "rust-build",
        help: "Build the Rust backend (all targets)",
        group: "build",
        lane: Lane::Borrowed,
        steps: &[sh!("cd apps/website/api_v2 && cargo build --all-targets")],
    },
    Task {
        name: "rust-test",
        help: "Run Rust unit tests (no DB)",
        group: "build",
        lane: Lane::Borrowed,
        steps: &[sh!("cd apps/website/api_v2 && cargo test --lib --bins")],
    },
    Task {
        name: "wasm-ci",
        help: "Fmt + clippy + test the map-engine and graphics-engine crates",
        group: "build",
        lane: Lane::Borrowed,
        // A crate the pipeline does not name is a crate the pipeline does not gate: compiled in
        // CI only as a transitive dependency, never fmt-checked, never clippied, its tests never
        // run. Both engine crates are named in all four steps, wasm32 included, because the
        // browser half is where they ship.
        steps: &[
            sh!("cargo fmt --check -p website-map-engine -p website-graphics-engine"),
            sh!(
                "cargo clippy -p website-map-engine -p website-graphics-engine --all-targets --all-features -- -D warnings"
            ),
            sh!(
                "cargo clippy -p website-map-engine -p website-graphics-engine --target wasm32-unknown-unknown -- -D warnings"
            ),
            sh!("cargo test -p website-map-engine --all-features"),
            sh!("cargo test -p website-graphics-engine --all-features"),
        ],
    },
    Task {
        name: "ci-local-leptos",
        help: "CI gate: Leptos SPA fmt + clippy(wasm32 --all-targets) + native tests + trunk release build (mirrors the ci.yml website-frontend job)",
        group: "build",
        lane: Lane::Borrowed,
        steps: &[
            sh!("cargo fmt -p website-frontend --check"),
            sh!("cargo clippy -p website-frontend --target wasm32-unknown-unknown --all-targets"),
            sh!("cargo test -p website-frontend"),
            sh!("cd apps/website/frontend && trunk build --release"),
        ],
    },
    Task {
        name: "leptos-build",
        help: "Release-build the Leptos SPA into apps/website/frontend/dist",
        group: "build",
        lane: Lane::Borrowed,
        steps: &[sh!("cd apps/website/frontend && trunk build --release")],
    },
    Task {
        name: "rust-test-it",
        help: "Run Rust integration tests against a fresh dedicated DB (needs `cargo xtask db up` @ :5434)",
        group: "db",
        lane: Lane::Borrowed,
        // `/bin/sh -c`, because the `while read -r db` reaper over psql output is the one step
        // here whose shape is genuinely a shell pipeline. `ignore_err` on the DROP lets a missing
        // database pass.
        steps: &[
            Step::Shell {
                silent: false,
                ignore_err: true,
                script: "podman exec tbd_reforger_db psql -U tbd -d tbd_reforger -qc \"DROP DATABASE IF EXISTS rust_it WITH (FORCE);\"",
            },
            Step::Shell {
                silent: false,
                ignore_err: false,
                script: "podman exec tbd_reforger_db psql -U tbd -d tbd_reforger -qc \"CREATE DATABASE rust_it;\"",
            },
            Step::Shell {
                silent: false,
                ignore_err: false,
                script: "cd apps/website/api_v2 && TEST_DATABASE_URL=postgres://tbd:tbd@localhost:5434/rust_it?sslmode=disable cargo test",
            },
            Step::Shell {
                silent: true,
                ignore_err: false,
                script: "podman exec tbd_reforger_db psql -U tbd -d tbd_reforger -Atc \"SELECT datname FROM pg_database WHERE datname = 'rust_it' OR datname LIKE 'rust_it\\_%\\_it' ESCAPE '\\'\" | while read -r db; do \t[ -n \"$db\" ] || continue; \tpodman exec tbd_reforger_db psql -U tbd -d tbd_reforger -qc \"DROP DATABASE IF EXISTS $db WITH (FORCE);\" >/dev/null; done",
            },
        ],
    },
];
