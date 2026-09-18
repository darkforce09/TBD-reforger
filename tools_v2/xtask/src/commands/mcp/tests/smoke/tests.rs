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
