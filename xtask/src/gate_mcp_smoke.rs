//! T-877 — port of `scripts/mod/mcp-smoke.sh` → `cargo xtask mcp smoke`.
//!
//! Live MCP smoke (T-090.0 gate S1): `wb_connect` + `wb_state` must both return
//! non-empty via in-process `cargo run -q -p xtask -- mcp call` (former
//! `lib/xtask-run.sh` parity — wave 226 option 2; libs stay on disk for OOS bash).
//!
//! Preserved bash shape (`set -uo pipefail`, **no** `-e`):
//! - a failed / empty tool call does not abort the loop
//! - command substitution captures **stdout only**; child stderr leaks to our stderr
//! - `$()` strips all trailing newlines before `[ -n "$out" ]`
//!
//! Exit: 0 all tools OK · 1 any tool FAIL.

use std::io::{self, Write};
use std::path::{Path, PathBuf};

use tbd_gate::NotRun;
use tbd_gate::proc::Run;

use crate::root::find_repo_root;

const TOOLS: &[&str] = &["wb_connect", "wb_state"];

/// Entry for `xtask mcp smoke`.
pub fn run() -> i32 {
    let root = match find_repo_root() {
        Ok(r) => r,
        Err(e) => {
            let _ = writeln!(io::stderr(), "mcp-smoke: FAIL (no repo root: {e})");
            return 1;
        }
    };
    run_at(&root.join("scripts/mod"))
}

/// Testable entry: `script_dir` is the former `SCRIPT_DIR` (`…/scripts/mod`).
pub fn run_at(script_dir: &Path) -> i32 {
    run_writers(script_dir, &mut io::stdout(), &mut io::stderr())
}

fn mono_root_from_script_dir(script_dir: &Path) -> PathBuf {
    // scripts/mod → ../../ = monorepo root (former xtask-run.sh dirname climb).
    script_dir
        .parent()
        .and_then(|p| p.parent())
        .map(Path::to_path_buf)
        .unwrap_or_else(|| script_dir.to_path_buf())
}

fn run_writers(script_dir: &Path, out: &mut dyn Write, err: &mut dyn Write) -> i32 {
    let root = mono_root_from_script_dir(script_dir);
    let mut fail = 0i32;

    for tool in TOOLS {
        match call_tool(&root, tool) {
            Ok((rc, body, child_err)) => {
                // Bash `$()` keeps child stderr on the smoke's stderr.
                if !child_err.is_empty() {
                    let _ = err.write_all(child_err.as_bytes());
                    let _ = err.flush();
                }
                if rc == 0 && !body.is_empty() {
                    let _ = writeln!(out, "mcp-smoke: {tool} OK");
                } else {
                    let _ = writeln!(err, "mcp-smoke: {tool} FAIL (rc={rc})");
                    fail = 1;
                }
            }
            Err(n) => {
                // No bool fold: DidNotRun must not look like Held. Surface as a tool FAIL
                // with a non-zero rc so the smoke stays red (bash would also fail the arm).
                let rc = match &n {
                    NotRun::ToolAbsent(_) => 127,
                    NotRun::Signalled { signal, .. } => 128 + signal,
                    NotRun::Timeout { .. } => 124,
                    NotRun::ToolError { status, .. } if *status > 0 => *status,
                    _ => 1,
                };
                let _ = writeln!(err, "mcp-smoke: DidNotRun ({n:?})");
                let _ = writeln!(err, "mcp-smoke: {tool} FAIL (rc={rc})");
                fail = 1;
            }
        }
    }

    if fail == 0 {
        let _ = writeln!(out, "mcp-smoke: OK");
        0
    } else {
        let _ = writeln!(err, "mcp-smoke: FAIL");
        1
    }
}

/// `(rc, bash-chomped stdout, raw stderr)`.
/// Former `lib/xtask-run.sh mcp call TOOL '{}'` ≡ `cargo run -q -p xtask -- mcp call …`.
fn call_tool(root: &Path, tool: &str) -> Result<(i32, String, String), NotRun> {
    let o = Run::new("cargo")
        .arg("run")
        .arg("-q")
        .arg("-p")
        .arg("xtask")
        .arg("--")
        .arg("mcp")
        .arg("call")
        .arg(tool)
        .arg("{}")
        .cwd(root)
        .output()?;
    Ok((o.code, bash_chomp(&o.stdout), o.stderr))
}

/// Bash command-substitution strips every trailing newline.
fn bash_chomp(s: &str) -> String {
    let mut t = s.to_string();
    while t.ends_with('\n') {
        t.pop();
    }
    t
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::os::unix::fs::PermissionsExt;
    use std::path::PathBuf;
    use std::sync::{Arc, Mutex};

    use crate::test_env::{PathGuard, lock_env};

    /// Shared buffer so stdout+stderr writes stay interleaved like `>file 2>&1`.
    #[derive(Clone)]
    struct SharedBuf(Arc<Mutex<Vec<u8>>>);

    impl Write for SharedBuf {
        fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
            self.0.lock().expect("shared buf").extend_from_slice(buf);
            Ok(buf.len())
        }

        fn flush(&mut self) -> io::Result<()> {
            Ok(())
        }
    }

    fn write_exec(path: &Path, body: &str) {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).unwrap();
        }
        fs::write(path, body).unwrap();
        let mut perms = fs::metadata(path).unwrap().permissions();
        perms.set_mode(0o755);
        fs::set_permissions(path, perms).unwrap();
    }

    fn capture(script_dir: &Path) -> (i32, String) {
        let buf = SharedBuf(Arc::new(Mutex::new(Vec::new())));
        let mut out = buf.clone();
        let mut err = buf.clone();
        let rc = run_writers(script_dir, &mut out, &mut err);
        let bytes = buf.0.lock().expect("buf").clone();
        (rc, String::from_utf8(bytes).expect("utf8"))
    }

    fn fixture(tag: &str) -> PathBuf {
        let dir = PathBuf::from(format!(
            "/tmp/t853/w226/t877/ut-{}-{}",
            tag,
            std::process::id()
        ));
        let _ = fs::remove_dir_all(&dir);
        // scripts/mod layout so mono_root_from_script_dir → fixture root
        fs::create_dir_all(dir.join("scripts/mod")).unwrap();
        dir
    }

    /// Stub `cargo` that only handles `run -q -p xtask -- mcp call …` (PATH-prepended).
    fn install_cargo_stub(bin_dir: &Path, body: &str) {
        write_exec(&bin_dir.join("cargo"), body);
    }

    #[test]
    fn bash_chomp_strips_all_trailing_newlines() {
        assert_eq!(bash_chomp("ok\n\n"), "ok");
        assert_eq!(bash_chomp("ok"), "ok");
        assert_eq!(bash_chomp("\n"), "");
        assert_eq!(bash_chomp(""), "");
    }

    #[test]
    fn both_tools_fail_arm_matches_bash() {
        let _g = lock_env();
        let root = fixture("both-fail");
        let bin = root.join("bin");
        fs::create_dir_all(&bin).unwrap();
        install_cargo_stub(
            &bin,
            "#!/usr/bin/env bash\n\
# peel args after --\n\
args=()\n\
seen=\n\
for a in \"$@\"; do\n\
  if [ -n \"$seen\" ]; then args+=(\"$a\"); continue; fi\n\
  [ \"$a\" = \"--\" ] && seen=1\n\
done\n\
echo \"stub-fail: ${args[*]}\" >&2\n\
exit 1\n",
        );
        let _path = PathGuard::prepend_dir(&bin);
        let script_dir = root.join("scripts/mod");
        let (rc, text) = capture(&script_dir);
        assert_eq!(rc, 1, "bash went red first on both-fail");
        assert_eq!(
            text,
            "\
stub-fail: mcp call wb_connect {}
mcp-smoke: wb_connect FAIL (rc=1)
stub-fail: mcp call wb_state {}
mcp-smoke: wb_state FAIL (rc=1)
mcp-smoke: FAIL
"
        );
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn one_tool_empty_arm_matches_bash() {
        let _g = lock_env();
        let root = fixture("one-empty");
        let bin = root.join("bin");
        fs::create_dir_all(&bin).unwrap();
        install_cargo_stub(
            &bin,
            "#!/usr/bin/env bash\n\
args=()\n\
seen=\n\
for a in \"$@\"; do\n\
  if [ -n \"$seen\" ]; then args+=(\"$a\"); continue; fi\n\
  [ \"$a\" = \"--\" ] && seen=1\n\
done\n\
tool=\"${args[2]:-}\"\n\
if [ \"$tool\" = \"wb_connect\" ]; then\n\
  printf '%s\\n' '{\"ok\":true}'\n\
  exit 0\n\
fi\n\
if [ \"$tool\" = \"wb_state\" ]; then\n\
  exit 0\n\
fi\n\
echo \"unexpected: ${args[*]}\" >&2\n\
exit 1\n",
        );
        let _path = PathGuard::prepend_dir(&bin);
        let script_dir = root.join("scripts/mod");
        let (rc, text) = capture(&script_dir);
        assert_eq!(rc, 1, "bash went red first on one-empty");
        assert_eq!(
            text,
            "\
mcp-smoke: wb_connect OK
mcp-smoke: wb_state FAIL (rc=0)
mcp-smoke: FAIL
"
        );
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn stub_green_arm_matches_bash() {
        let _g = lock_env();
        let root = fixture("stub-green");
        let bin = root.join("bin");
        fs::create_dir_all(&bin).unwrap();
        install_cargo_stub(
            &bin,
            "#!/usr/bin/env bash\n\
args=()\n\
seen=\n\
for a in \"$@\"; do\n\
  if [ -n \"$seen\" ]; then args+=(\"$a\"); continue; fi\n\
  [ \"$a\" = \"--\" ] && seen=1\n\
done\n\
tool=\"${args[2]:-}\"\n\
printf '%s\\n' \"STUB-OK $tool\"\n\
exit 0\n",
        );
        let _path = PathGuard::prepend_dir(&bin);
        let script_dir = root.join("scripts/mod");
        let (rc, text) = capture(&script_dir);
        assert_eq!(rc, 0);
        assert_eq!(
            text,
            "\
mcp-smoke: wb_connect OK
mcp-smoke: wb_state OK
mcp-smoke: OK
"
        );
        let _ = fs::remove_dir_all(&root);
    }
}
