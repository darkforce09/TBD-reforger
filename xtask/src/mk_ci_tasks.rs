//! The [`TASKS`] table: one row per Makefile target this lane absorbs (T-896).
//!
//! Split from `mk_ci.rs` at the data/behaviour seam so both stay inside the SIZE-1 600-line
//! guidance — the same shape as `gate_ui_layouts` / `gate_ui_layouts_awk`. Nothing here executes;
//! [`crate::mk_ci::run_task`] is the only interpreter of these rows.
//!
//! T-896 diffed every row against the Makefile recipe it reproduced. T-897 deleted that file, so
//! the successor pins are `ci_local_step_set_is_frozen` (`mk_ci_tests.rs`, the composite's step
//! list) and `gate_t468`'s `task_pins` (the three Class-R rows, at runtime).

use super::{Lane, Step, Task, verify_doc_layout};
use crate::codegen_schema::codegen;
use crate::gate_no_python::verify_no_python;
use crate::golden_gate::map_object_golden;
use crate::node_free::{verify_file_length, verify_no_node};
use crate::root::find_repo_root;
use crate::schema_gates::{
    citations, map_glyphs, map_object_enums, n6_sentence, n10_tile_budget, t090_specs,
    type_inventory, validate_all,
};
use crate::shell_free::verify_no_shell;

/// An echoed recipe line. The map lane stays a subprocess on purpose: `map` is a `tbd-tools`
/// binary, and reaching into another crate's clap wiring to save a fork would be drift.
macro_rules! sh {
    ($line:expr) => {
        Step::Cmd {
            line: $line,
            silent: false,
        }
    };
}

/// `cargo run -q -p xtask -- <cmd>` in the Makefile; an in-process call here.
macro_rules! xt {
    ($echo:expr, $silent:expr, $run:expr) => {
        Step::Xtask {
            echo: $echo,
            silent: $silent,
            run: $run,
        }
    };
}

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
        help: "Regenerate Rust contract types from packages/tbd-schema/schema via typify (T-165.3; loadout.rs is hand-maintained)",
        group: "schema",
        lane: Lane::Ci,
        steps: &[xt!("cargo xtask schema codegen", false, codegen)],
    },
    Task {
        name: "verify-citations",
        help: "Verify @contract citations in apps/ crates/ packages/ code — NOT docs/ prose (DOCUMENTATION_STANDARDS §10; T-165.1 Rust port, T-611 scope)",
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
        steps: &[sh!("editorconfig-checker")],
    },
    // ── map lane ────────────────────────────────────────────────────────────────────────────
    Task {
        name: "map-water-everon",
        help: "One-button Everon water composite: restore → mask → composite → bundle + pyramid → verify (T-090.1.2.5.2)",
        group: "map",
        lane: Lane::Ci,
        steps: &[
            // Coreutils `cp`, not `std::fs::copy`: the recipe's observable behaviour on a missing
            // source is cp's own "cannot stat" diagnostic, and packages/map-assets/**/staging is
            // gitignored scratch, so that miss is the COMMON path here, not the rare one.
            sh!(
                "cp packages/map-assets/everon/staging/sap/everon-sap-ortho.pre-water.png packages/map-assets/everon/staging/sap/everon-sap-ortho.png"
            ),
            sh!("cargo run -q -p tbd-tools --bin map -- reset-water-meta --terrain everon"),
            sh!("cargo run -q -p tbd-tools --bin map -- analyze-water"),
            sh!("cargo run -q -p tbd-tools --bin map -- composite-water"),
            sh!(
                "cargo run -q -p tbd-tools --bin map -- build-unified --input packages/map-assets/everon/staging/sap/everon-sap-ortho.png --out packages/map-assets/everon/satellite/everon-sat.tbd-sat --terrain everon"
            ),
            sh!("cargo run -q -p tbd-tools --bin map -- patch-unified-bytes --terrain everon"),
            sh!(
                "cargo run -q -p tbd-tools --bin map -- build-pyramid --input packages/map-assets/everon/staging/sap/everon-sap-ortho.png --out packages/map-assets/everon/tiles/satellite --minzoom 0 --maxzoom 6 --tilesize 256 --lossless"
            ),
            sh!("cargo run -q -p tbd-tools --bin map -- verify-sap-ortho --terrain everon"),
            sh!("cargo run -q -p tbd-tools --bin map -- verify-unified --terrain everon"),
            sh!(
                "cargo run -q -p tbd-tools --bin map -- verify-pyramid --terrain everon --expect-lossless"
            ),
        ],
    },
    Task {
        name: "map-cartographic-everon",
        help: "One-button Everon Map view (stylized cartographic): staging ortho → pyramid → manifest patch → verify (T-090.1.1)",
        group: "map",
        lane: Lane::Ci,
        steps: &[
            sh!("cargo run -q -p tbd-tools --bin map -- build-cartographic --terrain everon"),
            sh!(
                "cargo run -q -p tbd-tools --bin map -- build-pyramid --input packages/map-assets/everon/staging/map/everon-map-ortho.png --out packages/map-assets/everon/tiles/map --minzoom 0 --maxzoom 6 --tilesize 256"
            ),
            sh!("cargo run -q -p tbd-tools --bin map -- patch-map-tiles-meta --terrain everon"),
            Step::Task("map-cartographic-verify"),
        ],
    },
    Task {
        name: "map-cartographic-verify",
        help: "Verify the Everon Map pyramid (complete z0–6 + manifest agreement, T-090.1.1)",
        group: "map",
        lane: Lane::Ci,
        steps: &[sh!(
            "cargo run -q -p tbd-tools --bin map -- verify-pyramid --terrain everon --view-map"
        )],
    },
    Task {
        name: "lfs-dem",
        help: "Pull the Everon DEM from LFS (72 MB — map-engine tests + hillshade)",
        group: "map",
        lane: Lane::Ci,
        steps: &[sh!(
            "git lfs pull --include packages/map-assets/everon/dem/everon-dem-16bit.png"
        )],
    },
    Task {
        name: "lfs-sat",
        help: "Pull the Everon satellite bundle from LFS (153 MB — full-res editor basemap)",
        group: "map",
        lane: Lane::Ci,
        steps: &[sh!(
            "git lfs pull --include packages/map-assets/everon/satellite/everon-sat.tbd-sat"
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
            sh!("cd apps/website/api && cargo build --release --bin api"),
            Step::Task("leptos-build"),
        ],
    },
    // ── aliases: the make target was already a thin wrapper on an existing xtask command ─────
    Task {
        name: "verify-no-python",
        help: "T-162 hard gate — zero .py files / no Python interpreter in scripts",
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
        name: "verify-no-shell",
        help: "T-621 ratchet — no NEW .sh outside scripts/shell-inventory.txt (list may only shrink)",
        group: "verify",
        lane: Lane::Alias,
        steps: &[xt!("cargo xtask verify no-shell", false, verify_no_shell)],
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
    // T-897. These two were the only former make targets with no 1:1 xtask successor: two-line
    // recipes with no composite behind them. Deleting the Makefile without them would have left
    // ~20 T-090 spec citations pointing at a command nobody could run, so the composite moves
    // here rather than being dissolved into "go run these two things yourself".
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
        help: "Check Rust formatting (FMT-1 analog); workspace --all covers xtask/tbd-tools (T-297)",
        group: "build",
        lane: Lane::Borrowed("T-895"),
        steps: &[
            sh!("cd apps/website/api && cargo fmt --check"),
            sh!("cargo fmt --all --check"),
        ],
    },
    Task {
        name: "rust-clippy",
        help: "Lint Rust with clippy (deny warnings; GO-2..8 analog)",
        group: "build",
        lane: Lane::Borrowed("T-895"),
        steps: &[sh!(
            "cd apps/website/api && cargo clippy --all-targets -- -D warnings"
        )],
    },
    Task {
        name: "rust-build",
        help: "Build the Rust backend (all targets)",
        group: "build",
        lane: Lane::Borrowed("T-895"),
        steps: &[sh!("cd apps/website/api && cargo build --all-targets")],
    },
    Task {
        name: "rust-test",
        help: "Run Rust unit tests (no DB)",
        group: "build",
        lane: Lane::Borrowed("T-895"),
        steps: &[sh!("cd apps/website/api && cargo test --lib --bins")],
    },
    Task {
        name: "wasm-ci",
        help: "Fmt + clippy + test the map-engine core/render crates (T-145/T-151; T-418 dropped map-engine-wasm)",
        group: "build",
        lane: Lane::Borrowed("T-895"),
        steps: &[
            sh!("cargo fmt --check -p map-engine-core -p map-engine-render"),
            sh!("cargo clippy -p map-engine-core --all-targets --all-features -- -D warnings"),
            sh!("cargo clippy -p map-engine-render --target wasm32-unknown-unknown -- -D warnings"),
            sh!("cargo test -p map-engine-core --all-features"),
            sh!("cargo test -p map-engine-render"),
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
                script: "cd apps/website/api && TEST_DATABASE_URL=postgres://tbd:tbd@localhost:5434/rust_it?sslmode=disable cargo test",
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

fn x_height_labels() -> anyhow::Result<u8> {
    crate::label_gates::height_labels("everon")
}
fn x_terrain_manifest() -> anyhow::Result<u8> {
    crate::schema_gates::terrain_manifest("everon")
}
fn x_terrain_alignment() -> anyhow::Result<u8> {
    crate::label_gates::terrain_alignment("everon", false)
}
fn x_terrain_alignment_strict() -> anyhow::Result<u8> {
    crate::label_gates::terrain_alignment("everon", true)
}
fn x_no_select_star() -> anyhow::Result<u8> {
    crate::sql_gates::verify_no_select_star(&find_repo_root()?)
}
fn x_route_tags() -> anyhow::Result<u8> {
    crate::gate_route_tags::verify_route_tags(&find_repo_root()?)
}
fn x_t438() -> anyhow::Result<u8> {
    crate::gate_t438::verify_t438(&find_repo_root()?)
}
fn x_t456() -> anyhow::Result<u8> {
    crate::gate_t456::verify_t456(&find_repo_root()?)
}
fn x_t468() -> anyhow::Result<u8> {
    crate::gate_t468::verify_t468(&find_repo_root()?)
}
