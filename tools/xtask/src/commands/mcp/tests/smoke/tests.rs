use super::*;
use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use crate::core::test_environment::{PathGuard, lock_env};

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

fn capture(root: &Path) -> (i32, String) {
    let buf = SharedBuf(Arc::new(Mutex::new(Vec::new())));
    let mut out = buf.clone();
    let mut err = buf.clone();
    let rc = run_writers(root, &mut out, &mut err);
    let bytes = buf.0.lock().expect("buf").clone();
    (rc, String::from_utf8(bytes).expect("utf8"))
}

/// A scratch checkout root the stub `cargo` is invoked from.
fn scratch_root(tag: &str) -> PathBuf {
    let dir = PathBuf::from(format!(
        "/tmp/xtask-mcp-smoke/ut-{}-{}",
        tag,
        std::process::id()
    ));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();
    dir
}

/// Stub `cargo` that only handles `run -q -p xtask -- mcp call …`, prepended to PATH.
fn install_cargo_stub(bin_dir: &Path, body: &str) {
    write_exec(&bin_dir.join("cargo"), body);
}

#[test]
fn trailing_newlines_are_stripped_so_a_blank_body_reads_as_empty() {
    assert_eq!(strip_trailing_newlines("ok\n\n"), "ok");
    assert_eq!(strip_trailing_newlines("ok"), "ok");
    assert_eq!(strip_trailing_newlines("\n"), "");
    assert_eq!(strip_trailing_newlines(""), "");
}

#[test]
fn every_tool_is_attempted_when_the_first_one_fails() {
    let _g = lock_env();
    let root = scratch_root("both-fail");
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
    let (rc, text) = capture(&root);
    assert_eq!(rc, 1);
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
fn an_empty_body_with_a_zero_exit_code_is_a_failure() {
    let _g = lock_env();
    let root = scratch_root("one-empty");
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
    let (rc, text) = capture(&root);
    assert_eq!(rc, 1);
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
fn every_tool_answering_non_empty_is_the_only_green() {
    let _g = lock_env();
    let root = scratch_root("stub-green");
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
    let (rc, text) = capture(&root);
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
