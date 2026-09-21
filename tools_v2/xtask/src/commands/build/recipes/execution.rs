use super::*;

/// `cargo xtask mk <target> [--dry-run]`.
///
/// `--dry-run` is the analog of `make -n`: it prints the recipe lines without running them. It is
/// not a convenience — it is the only deterministic acceptance surface for the targets that never
/// terminate (`leptos`, `leptos-debug`) or that start a server (`rust-api`).
pub(crate) fn run(args: &[String]) -> Result<u8> {
    let dry = args.iter().any(|a| a == "--dry-run" || a == "-n");
    let target = args.iter().find(|a| !a.starts_with('-')).cloned();
    let Some(target) = target else {
        println!("usage: cargo xtask mk <target> [--dry-run]");
        for t in TARGETS {
            println!("  {t}");
        }
        return Ok(if args.iter().any(|a| a == "--list") {
            0
        } else {
            2
        });
    };

    // Asked and answered once, so the advertised list and the dispatch below cannot disagree —
    // and so `handles`, the chaining seam, is the same predicate callers get.
    if !handles(&target) {
        return unknown_target(&target);
    }

    // The three non-recipe targets: they compute or delete, they do not spawn a build.
    match target.as_str() {
        "print-cargo-target-dir" => {
            println!("{}", resolve_target_dir(env_pin().as_deref()));
            return Ok(0);
        }
        "verify-cargo-target" => return verify_cargo_target(&cwd_root()),
        "reclaim-target-ci" => return reclaim_target_ci(&primary_root()),
        _ => {}
    }

    let steps = match target.as_str() {
        "rust-api" => rust_api(),
        "rust-build" => rust_build(),
        "rust-test" => rust_test(),
        "rust-fmt" => rust_fmt(),
        "rust-clippy" => rust_clippy(),
        "rust-sqlx-prepare" => rust_sqlx_prepare(),
        "wasm-ci" => wasm_ci(),
        "leptos" => leptos(),
        "leptos-debug" => leptos_debug(),
        "leptos-build" => leptos_build(),
        "gate-doctor" => gate_doctor(),
        "leptos-gates" => leptos_gates(),
        "ci-local-leptos" => ci_local_leptos(),
        "rust-ci" => {
            if dry {
                let mut all = Vec::new();
                for s in [
                    rust_fmt(),
                    rust_clippy(),
                    rust_build(),
                    wasm_ci(),
                    rust_test_it(),
                ] {
                    all.extend(s);
                }
                all
            } else {
                return rust_ci();
            }
        }
        // Reachable only if TARGETS advertises something with no recipe — which
        // `tests::every_advertised_target_dispatches` forbids. Reported, never panicked.
        other => return unknown_target(other),
    };

    if dry {
        for s in &steps {
            println!("{}", s.echo());
        }
        return Ok(0);
    }
    run_steps(&steps)
}
