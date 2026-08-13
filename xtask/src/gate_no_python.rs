//! T-904 — LANG-2 is the same `TrackedLanguageBan` table as LANG-1.
//!
//! `cargo xtask verify no-python` stays as a CI alias so job names in `ci.yml` / `mk_ci_tasks`
//! / the wave gate do not break. It does **not** keep a second ratchet that can disagree with
//! `verify no-shell`. Inventories are gone.
//!
//! Historical: T-882 ported `scripts/verify-no-python.sh`; T-620 made that script fail-closed
//! after four waves of `rg || true`. T-904 folds the `.py` / `python3` command-position ban into
//! `xtask/src/shell_free.rs`.

use anyhow::Result;

use crate::shell_free;

/// Entry for `xtask verify no-python`. Same walk as [`shell_free::verify_no_shell`].
pub fn verify_no_python() -> Result<u8> {
    shell_free::verify_no_python()
}

/// Testable / fixture entry that does not walk for the repo root.
#[cfg(test)]
pub fn run_with_root(root: &std::path::Path) -> Result<u8> {
    shell_free::run_with_root(root)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::path::PathBuf;
    use std::process::Command;

    fn throwaway(name: &str) -> PathBuf {
        let p = std::env::temp_dir().join(format!(
            "t904-no-python-{}-{}-{}",
            name,
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let _ = fs::remove_dir_all(&p);
        fs::create_dir_all(p.join("src")).unwrap();
        p
    }

    fn write_min_tree(root: &std::path::Path) {
        // One tracked Rust file so the walk examines >0 paths (anti-vacuity). No inventory.
        fs::write(
            root.join("src/lib.rs"),
            "#![allow(clippy::collapsible_if)]\npub fn f() {}\n",
        )
        .unwrap();
        let _ = Command::new("git")
            .args(["init", "-q"])
            .current_dir(root)
            .status();
        let _ = Command::new("git")
            .args(["add", "-A"])
            .current_dir(root)
            .status();
    }

    #[test]
    fn clean_fixture_passes() {
        let root = throwaway("clean");
        write_min_tree(&root);
        let code = run_with_root(&root).unwrap();
        assert_eq!(code, 0, "clean fixture must PASS");
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn leftover_py_file_fails() {
        let root = throwaway("py");
        write_min_tree(&root);
        fs::create_dir_all(root.join("scripts")).unwrap();
        fs::write(root.join("scripts/evil.py"), "print('x')\n").unwrap();
        let _ = Command::new("git")
            .args(["add", "scripts/evil.py"])
            .current_dir(&root)
            .status();
        let code = run_with_root(&root).unwrap();
        assert_eq!(code, 1, "tracked .py must FAIL");
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn new_python3_invocation_fails() {
        let root = throwaway("new");
        write_min_tree(&root);
        fs::create_dir_all(root.join("scripts")).unwrap();
        // Extensionless on purpose: `*.sh` already fails the table; this proves command-position.
        fs::write(
            root.join("scripts/sneaky"),
            "echo start\npython3 -c 'import os'\n",
        )
        .unwrap();
        let _ = Command::new("git")
            .args(["add", "scripts/sneaky"])
            .current_dir(&root)
            .status();
        let code = run_with_root(&root).unwrap();
        assert_eq!(code, 1, "python3 in command position must FAIL");
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn comment_only_python3_is_not_a_hit() {
        let root = throwaway("comment");
        write_min_tree(&root);
        fs::create_dir_all(root.join("scripts")).unwrap();
        // Not `*.sh` — the combined table bans that extension regardless of python3.
        fs::write(
            root.join("scripts/note.txt"),
            "# deliberately no python3 here\necho ok\n",
        )
        .unwrap();
        let _ = Command::new("git")
            .args(["add", "scripts/note.txt"])
            .current_dir(&root)
            .status();
        let code = run_with_root(&root).unwrap();
        assert_eq!(code, 0, "comment-only python3 must not count");
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn rust_comment_python3_is_not_a_hit() {
        let root = throwaway("rscomment");
        write_min_tree(&root);
        fs::write(
            root.join("src/lib.rs"),
            "#![allow(clippy::collapsible_if)]\n// python3 -c 'print(1)'\npub fn f() {}\n",
        )
        .unwrap();
        let _ = Command::new("git")
            .args(["add", "src/lib.rs"])
            .current_dir(&root)
            .status();
        let code = run_with_root(&root).unwrap();
        assert_eq!(code, 0, "python3 in a Rust comment must not count");
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn shebang_python_counts() {
        let root = throwaway("shebang");
        write_min_tree(&root);
        fs::create_dir_all(root.join("scripts")).unwrap();
        fs::write(
            root.join("scripts/tool"),
            "#!/usr/bin/env python3\nprint(1)\n",
        )
        .unwrap();
        let _ = Command::new("git")
            .args(["add", "scripts/tool"])
            .current_dir(&root)
            .status();
        let code = run_with_root(&root).unwrap();
        assert_eq!(code, 1, "python shebang must FAIL");
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn makefile_is_banned() {
        let root = throwaway("make");
        write_min_tree(&root);
        fs::write(root.join("Makefile"), "all:\n\t@echo no\n").unwrap();
        let _ = Command::new("git")
            .args(["add", "Makefile"])
            .current_dir(&root)
            .status();
        let code = run_with_root(&root).unwrap();
        assert_eq!(code, 1, "Makefile must FAIL");
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn planted_sh_fails() {
        let root = throwaway("sh");
        write_min_tree(&root);
        fs::create_dir_all(root.join("scripts")).unwrap();
        fs::write(
            root.join("scripts/_t904_plant.sh"),
            "#!/usr/bin/env bash\necho x\n",
        )
        .unwrap();
        let _ = Command::new("git")
            .args(["add", "scripts/_t904_plant.sh"])
            .current_dir(&root)
            .status();
        let code = run_with_root(&root).unwrap();
        assert_eq!(code, 1, "tracked .sh must FAIL");
        let _ = fs::remove_dir_all(&root);
    }
}
