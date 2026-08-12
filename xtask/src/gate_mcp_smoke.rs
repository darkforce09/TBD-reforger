//! T-877 — port of `scripts/mod/mcp-smoke.sh` → `cargo xtask mcp smoke`.
//!
//! Live MCP smoke (T-090.0 gate S1): `wb_connect` + `wb_state` must both return
//! non-empty via `scripts/mod/lib/xtask-run.sh mcp call` (do **not** delete
//! xtask-run.sh — T-879). Prefer that helper over inventing a second call path.
//!
//! Preserved bash shape (`set -uo pipefail`, **no** `-e`):
//! - a failed / empty tool call does not abort the loop
//! - command substitution captures **stdout only**; child stderr leaks to our stderr
//! - `$()` strips all trailing newlines before `[ -n "$out" ]`
//!
//! Exit: 0 all tools OK · 1 any tool FAIL.

use std::io::{self, Write};
use std::path::Path;

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

fn run_writers(script_dir: &Path, out: &mut dyn Write, err: &mut dyn Write) -> i32 {
    let xtask_run = script_dir.join("lib/xtask-run.sh");
    let mut fail = 0i32;

    for tool in TOOLS {
        match call_tool(&xtask_run, tool) {
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
fn call_tool(xtask_run: &Path, tool: &str) -> Result<(i32, String, String), NotRun> {
    // Same helper bash used: `"$SCRIPT_DIR/lib/xtask-run.sh" mcp call "$tool" '{}'`
    let o = Run::new(xtask_run)
        .arg("mcp")
        .arg("call")
        .arg(tool)
        .arg("{}")
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
            "/tmp/t853/w225/t877/ut-{}-{}",
            tag,
            std::process::id()
        ));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(dir.join("lib")).unwrap();
        dir
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
        let dir = fixture("both-fail");
        write_exec(
            &dir.join("lib/xtask-run.sh"),
            "#!/usr/bin/env bash\necho \"stub-fail: $*\" >&2\nexit 1\n",
        );
        let (rc, text) = capture(&dir);
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
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn one_tool_empty_arm_matches_bash() {
        let dir = fixture("one-empty");
        write_exec(
            &dir.join("lib/xtask-run.sh"),
            "#!/usr/bin/env bash\n\
tool=\"${3:-}\"\n\
if [ \"$tool\" = \"wb_connect\" ]; then\n\
  printf '%s\\n' '{\"ok\":true}'\n\
  exit 0\n\
fi\n\
if [ \"$tool\" = \"wb_state\" ]; then\n\
  exit 0\n\
fi\n\
echo \"unexpected: $*\" >&2\n\
exit 1\n",
        );
        let (rc, text) = capture(&dir);
        assert_eq!(rc, 1, "bash went red first on one-empty");
        assert_eq!(
            text,
            "\
mcp-smoke: wb_connect OK
mcp-smoke: wb_state FAIL (rc=0)
mcp-smoke: FAIL
"
        );
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn stub_green_arm_matches_bash() {
        let dir = fixture("stub-green");
        write_exec(
            &dir.join("lib/xtask-run.sh"),
            "#!/usr/bin/env bash\ntool=\"${3:-}\"\nprintf '%s\\n' \"STUB-OK $tool\"\nexit 0\n",
        );
        let (rc, text) = capture(&dir);
        assert_eq!(rc, 0);
        assert_eq!(
            text,
            "\
mcp-smoke: wb_connect OK
mcp-smoke: wb_state OK
mcp-smoke: OK
"
        );
        let _ = fs::remove_dir_all(&dir);
    }
}
