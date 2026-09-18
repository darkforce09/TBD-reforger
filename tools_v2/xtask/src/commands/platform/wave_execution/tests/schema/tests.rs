use super::*;

#[test]
fn cksum_matches_the_coreutils_tool() {
    // If this drifts, the bash gate and this one fight over target-gate-schema and each pays a
    // cold rebuild. Compared against the real `cksum` so the interop claim is measured.
    use std::io::Write;
    let data = b"the quick brown fox\n";
    let mut child = std::process::Command::new("cksum")
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .spawn()
        .expect("cksum on PATH");
    child.stdin.take().unwrap().write_all(data).unwrap();
    let out = child.wait_with_output().unwrap();
    let want: String = String::from_utf8_lossy(&out.stdout)
        .trim_end()
        .chars()
        .filter(|c| !c.is_whitespace())
        .collect();
    assert_eq!(cksum(data), want);
}

#[test]
fn empty_input_matches_too() {
    assert_eq!(cksum(b""), format!("{}{}", 4294967295u32, 0));
}

/// T-420's awk read 3 of 9 and the one-way subset check stayed green over the hole, so the
/// set must match EXACTLY — an empty or partial read is a hard fail in `gate_schema`.
///
/// T-897 rebased this off the Makefile recipe onto the task table. Note there is no
/// `if …exists()` guard any more: the old one made the test vacuous the moment the file it
/// named went away, which is the same defect in miniature that this ticket exists to fix.
#[test]
fn the_task_table_and_the_pinned_set_agree() {
    let mut got = task_validate_gates();
    assert!(
        !got.is_empty(),
        "the schema-validate task row vanished — gate_schema would refuse, and so does this"
    );
    got.sort();
    let mut want: Vec<String> = VALIDATE_GATES.iter().map(|s| (*s).to_string()).collect();
    want.sort();
    assert_eq!(
        got, want,
        "`xtask schema list-gates` disagrees with GATE_SCHEMA_VALIDATE_GATES"
    );
}
