use super::*;

#[test]
fn extract_keeps_healthy_mission_line() {
    assert!(extract(HEALTHY).contains("loaded id="));
}

#[test]
fn extract_drops_mission_load_failure() {
    assert!(!extract(INVALID).contains("Mission loaded but invalid"));
}

#[test]
fn assess_healthy_passes() {
    let d = tempfile_dir("tbd-det-test-h");
    let p = d.join("h.log");
    fs::write(&p, HEALTHY).unwrap();
    assert_eq!(assess_run_silent(&p, "t"), 0);
    let _ = fs::remove_dir_all(&d);
}

#[test]
fn assess_stale_fails() {
    let d = tempfile_dir("tbd-det-test-s");
    let p = d.join("s.log");
    fs::write(&p, STALE).unwrap();
    assert_eq!(assess_run_silent(&p, "t"), 1);
    let _ = fs::remove_dir_all(&d);
}

#[test]
fn assess_fallthrough_fails() {
    let d = tempfile_dir("tbd-det-test-f");
    let p = d.join("f.log");
    let mut body = HEALTHY.to_string();
    body.push_str("SCRIPT       : path=vanilla-fallthrough\n");
    fs::write(&p, body).unwrap();
    assert_eq!(assess_run_silent(&p, "t"), 1);
    let _ = fs::remove_dir_all(&d);
}

/// Wave-218 REJECT pin: census mismatch must print grep -oE extract, not the full log line.
#[test]
fn assess_census_mismatch_extracts_audit_only() {
    let d = tempfile_dir("tbd-det-test-census");
    let p = d.join("c.log");
    let mut body = HEALTHY.to_string();
    body = body.replace(
        "[TBD][Audit] characters=2 bodies=2 players=1",
        "[TBD][Audit] characters=5 bodies=2 players=1",
    );
    fs::write(&p, &body).unwrap();
    let mut buf = Vec::new();
    assert_eq!(assess_run_to(&p, "novel", &mut buf), 1);
    let out = String::from_utf8(buf).unwrap();
    assert!(
        out.contains(
            "FAIL run novel: census mismatch [TBD][Audit] characters=5 bodies=2 players=1 (stray/missing bodies?)"
        ),
        "got: {out:?}"
    );
    assert!(
        !out.contains("21:12:03.000"),
        "timestamp prefix leaked into census mismatch: {out:?}"
    );
    let _ = fs::remove_dir_all(&d);
}

/// Wave-218 REJECT pin: duplicate binds must use GNU uniq -c 7-wide count field.
#[test]
fn assess_duplicate_binds_uniq_c_padding() {
    let d = tempfile_dir("tbd-det-test-dup");
    let p = d.join("d.log");
    let mut body = HEALTHY.to_string();
    body = body.replace(
        "bound player 1 to slot blufor:Alpha:SL:0 body (kit kit:rifleman_m16)\n",
        "bound player 1 to slot blufor:Alpha:SL:0 body (kit kit:rifleman_m16)\n21:12:02.200 SCRIPT       : [TBD] SpawnManager: bound player 1 to slot blufor:Alpha:SL:0 body (kit kit:rifleman_m16)\n",
    );
    fs::write(&p, &body).unwrap();
    let mut buf = Vec::new();
    assert_eq!(assess_run_to(&p, "novel", &mut buf), 1);
    let out = String::from_utf8(buf).unwrap();
    assert!(
        out.contains("FAIL run novel: duplicate binds:\n      2 bound player 1\n"),
        "uniq -c padding mismatch, got: {out:?}"
    );
    let _ = fs::remove_dir_all(&d);
}
