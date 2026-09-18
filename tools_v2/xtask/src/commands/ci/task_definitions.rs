#[macro_use]
#[path = "task_definitions/recipe_macros.rs"]
mod recipe_macros;

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
    citations, map_glyphs, map_object_enums, n6_sentence, n10_tile_budget, t090_specs,
    type_inventory, validate_all,
};

pub static TASKS: &[Task] = &[
    // ── composites ──────────────────────────────────────────────────────────────────────────
    Task {
        name: "ci-local",
        help: "Full CI gate locally — mirrors ci.yml (run `cargo xtask db up` first)",
        group: "CI",
        lane: Lane::Ci,
        // T-489/T-881: the last step is NOT `Step::Task("verify-t468")`. t468 is the tripwire that
        // pins other gates' recipe bodies against being hollowed, so routing it through the very
        // dispatch it polices would let a hollowed dispatcher green it. Direct call, as ci-local
        // and ci.yml already do for the same stated reason (Makefile:493).
        steps: &[
            Step::Task("verify-editorconfig"),
            Step::Task("verify-no-python"),
            Step::Task("verify-no-node"),
            Step::Task("verify-no-shell"),
            Step::Task("verify-ci-shell"),
            // Grouped with the language gates rather than the build lanes: like them it is a
            // seconds-long source scan, and it guards a wall (map-engine -> graphics-engine, one
            // way) that nothing in the compiler enforces. ENGINE_SPLIT_PROGRAM §5 requires it here
            // and in ci.yml — a rule nobody is stopped by is not a rule.
            Step::Task("verify-engine-layers"),
            Step::Task("rust-ci"),
            Step::Task("verify-coding-standards"),
            Step::Task("ci-local-leptos"),
            Step::Task("ci-local-schema"),
            Step::Task("verify-t438"),
            Step::Task("verify-t456"),
            xt!("cargo xtask verify t468", true, x_t468),
        ],
    },
    Task {
        name: "ci-local-schema",
        help: "CI gate: schema validate (TEST-3) + @contract citation verify",
        group: "CI",
        lane: Lane::Ci,
        steps: &[
            Step::Task("schema-validate"),
            Step::Task("verify-citations"),
        ],
    },
    Task {
        name: "schema-validate",
        help: "Validate golden missions + T-090 map-object contracts (enums + glyphs + spec consistency) + T-152.16 height labels",
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
            xt!("cargo xtask schema height-labels", false, x_height_labels),
            xt!(
                "cargo xtask schema map-object-enums",
                false,
                map_object_enums
            ),
            xt!("cargo xtask schema type-inventory", false, type_inventory),
            xt!("cargo xtask schema t090-specs", false, t090_specs),
            xt!("cargo xtask schema n6", false, n6_sentence),
            xt!("cargo xtask schema n10", false, n10_tile_budget),
        ],
    },
    Task {
        name: "schema-codegen",
        help: "Regenerate Rust contract types from contracts_v2/schema via typify (T-165.3; loadout.rs is hand-maintained)",
        group: "schema",
        lane: Lane::Ci,
        steps: &[xt!("cargo xtask schema codegen", false, codegen)],
    },
    Task {
        name: "verify-citations",
        help: "Verify @contract citations in apps/ packages/ tools_v2/ code — NOT docs/ prose (DOCUMENTATION_STANDARDS §10; T-165.1 Rust port, T-611 scope)",
        group: "schema",
        lane: Lane::Ci,
        steps: &[xt!("cargo xtask schema citations", false, citations)],
    },
    Task {
        name: "verify-coding-standards",
        help: "SIZE file length + doc layout + GO-7 @route/router match (CODING_STANDARDS §11)",
        group: "verify",
        lane: Lane::Ci,
        steps: &[
            Step::Task("verify-doc-layout"),
            xt!("cargo xtask verify file-length", true, verify_file_length),
            xt!("cargo xtask verify no-select-star", true, x_no_select_star),
            xt!("cargo xtask verify route-tags", true, x_route_tags),
        ],
    },
    Task {
        name: "verify-doc-layout",
        help: "DOCUMENTATION_STANDARDS §8.2: no markdown spec trees under apps/**/docs or packages/**/docs",
        group: "verify",
        lane: Lane::Ci,
        steps: &[Step::Native {
            run: verify_doc_layout,
        }],
    },
    Task {
        name: "verify-editorconfig",
        help: "FMT-2: run editorconfig-checker from repo root (CODING_STANDARDS §7)",
        group: "verify",
        lane: Lane::Ci,
        steps: &[Step::Native {
            run: crate::commands::ci::editor_api::verify_editorconfig,
        }],
    },
    Task {
        name: "verify-codegen-fresh",
        help: "Fail if apps/website/api_v2/src/missions/contract/generated is stale after schema-codegen",
        group: "schema",
        lane: Lane::Ci,
        steps: &[Step::Native {
            run: crate::commands::ci::editor_api::verify_codegen_fresh,
        }],
    },
    Task {
        name: "ci-chrome",
        help: "T-901: install pinned Chrome-for-Testing (editor-gates.yml)",
        group: "CI",
        lane: Lane::Ci,
        steps: &[Step::Native {
            run: crate::commands::ci::chromium_install::run,
        }],
    },
    Task {
        name: "editor-api-boot",
        help: "T-901: build+spawn website-api and wait on /healthz (editor-gates.yml)",
        group: "CI",
        lane: Lane::Ci,
        steps: &[Step::Native {
            run: crate::commands::ci::editor_api::run,
        }],
    },
    Task {
        name: "website-api-test",
        help: "T-901: cargo test in apps/website/api_v2 (honours TEST_DATABASE_URL)",
        group: "build",
        lane: Lane::Ci,
        steps: &[sh!("cd apps/website/api_v2 && cargo test")],
    },
    // The library suite runs without database, browser, or asset prerequisites.
    Task {
        name: "tbd-tools-test",
        help: "T-298: cargo test -p developer-tools --lib (density:: + world:: unit tests; no DB/LFS)",
        group: "build",
        lane: Lane::Ci,
        steps: &[sh!("cargo test -p developer-tools --lib")],
    },
    // ── map lane ────────────────────────────────────────────────────────────────────────────
    Task {
        name: "map-water-everon",
        help: "One-button Everon water composite: restore → mask → composite → bundle + pyramid → verify (T-090.1.2.5.2)",
        group: "map",
        lane: Lane::Ci,
        steps: &[
            // Coreutils `cp`, not `std::fs::copy`: the recipe's observable behaviour on a missing
            // source is cp's own "cannot stat" diagnostic, and assets_v2/terrains/**/staging is
            // gitignored scratch, so that miss is the COMMON path here, not the rare one.
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
        help: "One-button Everon Map view (stylized cartographic): staging ortho → pyramid → manifest patch → verify (T-090.1.1)",
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
        help: "Verify the Everon Map pyramid (complete z0–6 + manifest agreement, T-090.1.1)",
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
    // ── aliases: the make target was already a thin wrapper on an existing xtask command ─────
    Task {
        name: "verify-no-python",
        help: "T-904 hard zero — same TrackedLanguageBan table as verify-no-shell (.py / python3)",
        group: "verify",
        lane: Lane::Alias,
        steps: &[xt!("cargo xtask verify no-python", false, verify_no_python)],
    },
    Task {
        name: "verify-no-node",
        help: "T-165.10 hard gate — zero tracked .mjs/.cjs; node only as the enfusion-mcp floor",
        group: "verify",
        lane: Lane::Alias,
        steps: &[xt!("cargo xtask verify no-node", false, verify_no_node)],
    },
    Task {
        name: "verify-engine-layers",
        help: "ENGINE_SPLIT_PROGRAM §5 rules 1, 2, 3a, 3b, 4, 7 — graphics-engine imports no map engine and declares no map noun; map-engine names the frame vocabulary at one enumerated seam and no GPU module at all; data/scenario imports nothing outside itself; data/ and world/ name each other nowhere",
        group: "verify",
        lane: Lane::Alias,
        steps: &[xt!(
            "cargo xtask verify engine-layers",
            false,
            x_engine_layers
        )],
    },
    Task {
        name: "verify-no-shell",
        help: "T-904 hard zero — no tracked shell/Make/Python/Node-script paths (no inventory)",
        group: "verify",
        lane: Lane::Alias,
        steps: &[xt!("cargo xtask verify no-shell", false, verify_no_shell)],
    },
    Task {
        name: "verify-ci-shell",
        help: "T-901 — every GitHub Actions run: is cargo xtask or a short pre-cargo allowlist",
        group: "verify",
        lane: Lane::Alias,
        steps: &[xt!("cargo xtask verify ci-shell", false, verify_ci_shell)],
    },
    Task {
        name: "verify-t438",
        help: "T-438/T-461 deploy-staging compose path (website/, not api/)",
        group: "verify",
        lane: Lane::Alias,
        steps: &[xt!("cargo xtask verify t438", true, x_t438)],
    },
    Task {
        name: "verify-t456",
        help: "T-456/T-460 mission REST body size gate before ParseMissionJson",
        group: "verify",
        lane: Lane::Alias,
        steps: &[xt!("cargo xtask verify t456", true, x_t456)],
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
                x_terrain_manifest
            ),
            xt!(
                "cargo xtask schema terrain-alignment --terrain everon",
                false,
                x_terrain_alignment
            ),
        ],
    },
    Task {
        name: "verify-terrain-strict",
        help: "Full anchor alignment gate (T-091.0 GetSurfaceY DEM + anchors)",
        group: "verify",
        lane: Lane::Ci,
        steps: &[
            xt!(
                "cargo xtask schema terrain-manifest --terrain everon",
                false,
                x_terrain_manifest
            ),
            xt!(
                "cargo xtask schema terrain-alignment --terrain everon --strict",
                false,
                x_terrain_alignment_strict
            ),
        ],
    },
    // ── borrowed: T-895's build lane / T-894's db lane. See §2. ──────────────────────────────
    Task {
        name: "rust-ci",
        help: "Rust CI gate locally — fmt + clippy + build + test-it (mirrors the ci.yml rust-backend job)",
        group: "build",
        lane: Lane::Borrowed("T-895"),
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
        help: "Check Rust formatting (FMT-1 analog); workspace --all covers tools_v2/xtask/tbd-tools (T-297)",
        group: "build",
        lane: Lane::Borrowed("T-895"),
        steps: &[
            sh!("cd apps/website/api_v2 && cargo fmt --check"),
            sh!("cargo fmt --all --check"),
        ],
    },
    Task {
        name: "rust-clippy",
        help: "Lint Rust with clippy (deny warnings; GO-2..8 analog)",
        group: "build",
        lane: Lane::Borrowed("T-895"),
        steps: &[sh!(
            "cd apps/website/api_v2 && cargo clippy --all-targets -- -D warnings"
        )],
    },
    Task {
        name: "rust-build",
        help: "Build the Rust backend (all targets)",
        group: "build",
        lane: Lane::Borrowed("T-895"),
        steps: &[sh!("cd apps/website/api_v2 && cargo build --all-targets")],
    },
    Task {
        name: "rust-test",
        help: "Run Rust unit tests (no DB)",
        group: "build",
        lane: Lane::Borrowed("T-895"),
        steps: &[sh!("cd apps/website/api_v2 && cargo test --lib --bins")],
    },
    Task {
        name: "wasm-ci",
        help: "Fmt + clippy + test the map-engine core/render crates (T-145/T-151; T-418 dropped map-engine-wasm)",
        group: "build",
        lane: Lane::Borrowed("T-895"),
        // The engine split created `website-graphics-engine` and this lane did not learn about it,
        // so 54 files of renderer compiled in CI only as a transitive dependency of the frontend:
        // never fmt-checked, never clippied, and its 41 tests never run. A crate the pipeline does
        // not name is a crate the pipeline does not gate — it is added to all four steps, wasm32
        // included, because the browser half is where it actually ships.
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
        help: "CI gate: Leptos SPA fmt + clippy(wasm32 --all-targets) + native tests + trunk release build (mirrors ci.yml website-frontend clippy --all-targets; T-752)",
        group: "build",
        lane: Lane::Borrowed("T-895"),
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
        lane: Lane::Borrowed("T-895"),
        steps: &[sh!("cd apps/website/frontend && trunk build --release")],
    },
    Task {
        name: "rust-test-it",
        help: "Run Rust integration tests against a fresh dedicated DB (needs `cargo xtask db up` @ :5434)",
        group: "db",
        lane: Lane::Borrowed("T-894"),
        // T-894 owns the real port (the `while read -r db` reaper over psql output is the one
        // piece of genuinely non-trivial shell in the Makefile). Verbatim `/bin/sh -c` until then
        // — see Step::Shell. `ignore_err` on the DROP is make's leading `-`; the last line is
        // `@`-silenced. `\t` inside the reaper is where make's backslash-continuations were: sh
        // treats it as the separator it already was, so the pipeline is unchanged.
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

/* ─────────────────────────────── in-process leaf adapters ─────────────────────────────── */
// `fn` pointers cannot capture, and these four leaves take the repo root. One-liners rather than
// a boxed closure so the table stays a `static` and `help` needs no allocation.

#[path = "task_definitions/x_height_labels.rs"]
mod x_height_labels;
use x_height_labels::x_engine_layers;
use x_height_labels::x_height_labels;
use x_height_labels::x_no_select_star;
use x_height_labels::x_route_tags;
use x_height_labels::x_t438;
use x_height_labels::x_t456;
use x_height_labels::x_t468;
use x_height_labels::x_terrain_alignment;
use x_height_labels::x_terrain_alignment_strict;
use x_height_labels::x_terrain_manifest;
