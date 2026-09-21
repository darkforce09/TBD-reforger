use super::*;
use std::sync::atomic::{AtomicU32, Ordering};

const SHA: &str = "2e52e37c40f5476272e8381ee3dd3601203718ae";
const OTHER: &str = "33776e3e2aa1b0c9d4e5f60718293a4b5c6d7e8f";
const AT: &str = "2026-09-06T09:00:00Z";

static N: AtomicU32 = AtomicU32::new(0);

/// A scratch "main checkout". Same idiom as [`ticket_engine::metrics`]'s tests, plus a counter so two
/// tests in the same process never share a directory.
fn scratch(tag: &str) -> PathBuf {
    let n = N.fetch_add(1, Ordering::Relaxed);
    let tmp = std::env::temp_dir().join(format!("tbd-verdict-{tag}-{}-{n}", std::process::id()));
    let _ = std::fs::remove_dir_all(&tmp);
    std::fs::create_dir_all(&tmp).expect("mk scratch");
    tmp
}

/// T-946 — a non-canonical slice id must not be told to do the impossible.
///
/// `slice/T-247-hotfix` is a branch shape this repo really creates. `path_for` refuses it, so
/// `read` errors — and the old message said "re-gate", which cannot help: `record_slice_gate`
/// refuses the same shape in the same place. The refusal now says so and names the two things
/// that DO work.
#[test]
fn a_non_canonical_slice_id_is_told_the_truth_not_to_re_gate() {
    let dir = std::env::temp_dir().join(format!("t946-badid-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    let msg = land_refusal(
        &dir,
        "T-247-hotfix",
        "deadbeefdeadbeefdeadbeefdeadbeefdeadbeef",
    )
    .expect("must refuse");
    println!("── refusal ──\n{msg}");
    assert!(
        msg.contains("not a ticket id"),
        "names the real cause: {msg}"
    );
    assert!(
        msg.contains("Re-gating CANNOT help"),
        "and does not send the operator round a loop with no exit: {msg}"
    );
    // A canonical id with no receipt still gets the ordinary re-gate advice.
    let ok = land_refusal(&dir, "T-999", "deadbeefdeadbeefdeadbeefdeadbeefdeadbeef")
        .expect("must refuse");
    assert!(
        ok.contains("no gate has run on it") && !ok.contains("Re-gating CANNOT help"),
        "the ordinary arm is unchanged: {ok}"
    );
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn a_receipt_round_trips_through_disk() {
    let root = scratch("roundtrip");
    let path = write_at(&root, "T-924", SHA, PASS, AT).expect("write");
    assert_eq!(path, root.join(VERDICTS_DIR).join("T-924.json"));
    let got = read(&root, "T-924").expect("read").expect("present");
    assert_eq!(
        got,
        Verdict {
            sha: SHA.into(),
            verdict: PASS.into(),
            at: AT.into(),
        }
    );
    assert!(got.is_green());
}

#[test]
fn the_receipt_is_exactly_the_three_contract_fields() {
    // {sha, verdict, at} is the ticket's own wire shape. A field added here silently changes
    // what every reader of the directory is looking at, so the JSON is pinned, not the struct.
    let root = scratch("fields");
    write_at(&root, "T-924", SHA, PASS, AT).expect("write");
    let text =
        std::fs::read_to_string(root.join(VERDICTS_DIR).join("T-924.json")).expect("read raw");
    let v: serde_json::Value = serde_json::from_str(&text).expect("parse");
    let obj = v.as_object().expect("object");
    let mut keys: Vec<&str> = obj.keys().map(String::as_str).collect();
    keys.sort_unstable();
    assert_eq!(keys, ["at", "sha", "verdict"]);
}

#[test]
fn the_receipts_directory_hides_itself_from_git() {
    // Note 2 in the module header. Without this the gate makes the slice worktree dirty and
    // `ledger::tree_state` — a `git status --porcelain`, which lists UNTRACKED files — reports
    // "dirty", so the slice can never land. The `*` matches `.gitignore` itself.
    let root = scratch("gitignore");
    write_at(&root, "T-924", SHA, PASS, AT).expect("write");
    let ignore = root.join(VERDICTS_DIR).join(".gitignore");
    assert_eq!(
        std::fs::read_to_string(&ignore).expect("ignore file"),
        "*\n"
    );
}

#[test]
fn a_lost_ignore_file_is_rewritten_on_the_next_gate() {
    let root = scratch("reignore");
    write_at(&root, "T-924", SHA, PASS, AT).expect("write");
    let ignore = root.join(VERDICTS_DIR).join(".gitignore");
    std::fs::remove_file(&ignore).expect("rm ignore");
    write_at(&root, "T-924", SHA, PASS, AT).expect("rewrite");
    assert_eq!(
        std::fs::read_to_string(&ignore).expect("ignore file"),
        "*\n"
    );
}

#[test]
fn land_accepts_a_green_receipt_at_the_landing_sha() {
    let root = scratch("green");
    write_at(&root, "T-924", SHA, PASS, AT).expect("write");
    assert_eq!(land_refusal(&root, "T-924", SHA), None);
}

#[test]
fn land_refuses_when_no_gate_has_run() {
    // The pre-T-924 behaviour of `land` in one assertion: with NO receipt on disk it used to
    // merge anyway. This is the arm that closes the 2026-08-14 incident.
    let root = scratch("missing");
    let r = land_refusal(&root, "T-924", SHA).expect("must refuse");
    assert!(r.contains("no gate has run"), "{r}");
    assert!(r.contains(&regate_hint("T-924")), "fix not named: {r}");
}

#[test]
fn land_refuses_a_stale_receipt_and_names_both_shas() {
    // The slice committed (or rebased) after gating: the verdict describes a tree that is not
    // the one about to reach main.
    let root = scratch("stale");
    write_at(&root, "T-924", SHA, PASS, AT).expect("write");
    let r = land_refusal(&root, "T-924", OTHER).expect("must refuse");
    assert!(r.contains("STALE"), "{r}");
    assert!(r.contains(&short12(SHA)), "gated sha absent: {r}");
    assert!(r.contains(&short12(OTHER)), "landing sha absent: {r}");
    assert!(r.contains(&regate_hint("T-924")), "fix not named: {r}");
}

#[test]
fn land_refuses_a_red_receipt() {
    let root = scratch("red");
    write_at(&root, "T-924", SHA, FAIL, AT).expect("write");
    let r = land_refusal(&root, "T-924", SHA).expect("must refuse");
    assert!(r.contains("not PASS"), "{r}");
    assert!(r.contains(&regate_hint("T-924")), "fix not named: {r}");
}

#[test]
fn an_unrecognised_verdict_string_is_not_green() {
    // "not FAIL" is not "green". A receipt hand-edited to `"verdict": "ok"` must refuse.
    let root = scratch("bogus");
    let dir = root.join(VERDICTS_DIR);
    std::fs::create_dir_all(&dir).expect("mkdir");
    std::fs::write(
        dir.join("T-924.json"),
        format!("{{\"sha\":\"{SHA}\",\"verdict\":\"ok\",\"at\":\"{AT}\"}}"),
    )
    .expect("write");
    let r = land_refusal(&root, "T-924", SHA).expect("must refuse");
    assert!(r.contains("not PASS"), "{r}");
}

#[test]
fn an_unreadable_receipt_is_a_refusal_not_an_absence() {
    // Flattening a parse error into "no receipt" would be honest here but not everywhere: the
    // point is that a receipt this build cannot understand never reads as green.
    let root = scratch("corrupt");
    let dir = root.join(VERDICTS_DIR);
    std::fs::create_dir_all(&dir).expect("mkdir");
    std::fs::write(dir.join("T-924.json"), "{ not json").expect("write");
    let r = land_refusal(&root, "T-924", SHA).expect("must refuse");
    assert!(r.contains("unreadable"), "{r}");
    assert!(r.contains(&regate_hint("T-924")), "fix not named: {r}");
}

#[test]
fn an_extra_field_is_refused_rather_than_ignored() {
    let root = scratch("unknown-field");
    let dir = root.join(VERDICTS_DIR);
    std::fs::create_dir_all(&dir).expect("mkdir");
    std::fs::write(
        dir.join("T-924.json"),
        format!("{{\"sha\":\"{SHA}\",\"verdict\":\"PASS\",\"at\":\"{AT}\",\"ok\":true}}"),
    )
    .expect("write");
    assert!(land_refusal(&root, "T-924", SHA).is_some());
}

#[test]
fn an_unresolvable_landing_head_is_refused() {
    let root = scratch("nohead");
    write_at(&root, "T-924", SHA, PASS, AT).expect("write");
    let r = land_refusal(&root, "T-924", "").expect("must refuse");
    assert!(r.contains("cannot resolve the landing HEAD"), "{r}");
}

#[test]
fn a_slice_id_that_is_not_a_ticket_id_is_refused() {
    // `<slice>.json` is interpolated into a path; argv is not trusted to stay inside it.
    for bad in [
        "../../etc/passwd",
        "T-924/../../x",
        "",
        "T-",
        "main",
        "T-92a",
        "T-924.",
    ] {
        assert!(
            path_for(Path::new("/nowhere"), bad).is_err(),
            "accepted {bad:?}"
        );
    }
    for good in ["T-924", "T-159.29.3", "T-1"] {
        assert!(
            path_for(Path::new("/nowhere"), good).is_ok(),
            "rejected {good:?}"
        );
    }
}

#[test]
fn a_receipt_without_a_sha_or_with_a_bogus_verdict_is_never_written() {
    let root = scratch("refuse-write");
    assert!(write_at(&root, "T-924", "   ", PASS, AT).is_err());
    assert!(write_at(&root, "T-924", SHA, "green", AT).is_err());
    assert!(write_at(&root, "T-924", SHA, PASS, "2026-09-06 09:00:00").is_err());
    assert!(read(&root, "T-924").expect("read").is_none());
}

#[test]
fn the_newest_gate_wins() {
    let root = scratch("overwrite");
    write_at(&root, "T-924", SHA, FAIL, AT).expect("write");
    write_at(&root, "T-924", OTHER, PASS, "2026-09-06T10:00:00Z").expect("rewrite");
    let got = read(&root, "T-924").expect("read").expect("present");
    assert_eq!(got.sha, OTHER);
    assert_eq!(got.verdict, PASS);
}

// ── Class-R: the gate really calls the writer, and `land` really calls the refusal ──────────
//
// The behavioural tests above prove the ORACLE is right. They cannot prove it is WIRED — a
// correct refusal that nothing invokes is this program's most expensive recurring defect
// (T-462, T-463, T-556: verify scripts that existed, were correct, and were called by nothing).
// These read the production source of the two call sites. The haystack is a single function
// BODY extracted by brace matching, so this test module can never satisfy its own pin.

const GATE_SRC: &str = include_str!("../../gate/checkrun.rs");
const LAND_SRC: &str = include_str!("../../land/merge_execution.rs");

/// The body of `fn <name>` — from its opening brace to the matching close.
fn fn_body(src: &str, name: &str) -> String {
    let sig = format!("fn {name}(");
    let at = src
        .find(&sig)
        .unwrap_or_else(|| panic!("no fn {name} in source"));
    let open = src[at..].find('{').expect("fn body opens") + at;
    let bytes = src.as_bytes();
    let mut depth = 0usize;
    for (i, b) in bytes.iter().enumerate().skip(open) {
        match b {
            b'{' => depth += 1,
            b'}' => {
                depth -= 1;
                if depth == 0 {
                    return src[open..=i].to_string();
                }
            }
            _ => {}
        }
    }
    panic!("unbalanced body for {name}");
}

#[test]
fn gate_slice_writes_a_receipt_on_every_run() {
    // "on every run" = pass AND fail. If only PASS wrote one, `land`'s "not green" arm would be
    // unreachable and a red gate would be indistinguishable from a gate that never ran.
    let body = fn_body(GATE_SRC, "gate_slice");
    assert!(
        body.contains("verdict::record_slice_gate("),
        "gate_slice no longer writes a T-924 verdict receipt"
    );
    // Written BEFORE the FAIL early return, which is what puts both exits behind one call.
    let write_at_idx = body
        .find("verdict::record_slice_gate(")
        .expect("call present");
    let fail_return = body
        .find("state.verdict(\"FAIL\"")
        .expect("FAIL arm present");
    assert!(
        write_at_idx < fail_return,
        "the receipt write must precede the FAIL return, or a red gate leaves no receipt"
    );
}

#[test]
fn cmd_land_refuses_before_it_merges() {
    // The refusal has to be upstream of `git merge`: refusing after the merge is a report, not
    // a gate. `land` merges in a loop, so the check lives in its own pass over `ready` first.
    let body = fn_body(LAND_SRC, "cmd_land");
    let check = body
        .find("verdict::land_refusal(")
        .expect("cmd_land no longer consults the T-924 gate verdict");
    let merge = body.find("\"merge\",").expect("the merge call is present");
    assert!(
        check < merge,
        "the verdict check must run BEFORE the merge loop"
    );
}
