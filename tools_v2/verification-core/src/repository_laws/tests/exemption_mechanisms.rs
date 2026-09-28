//! Tests for [`super`] — exemption files, comment directives and exemption tables.
//!
//! A fixture that would itself be a finding in this file is assembled with `concat!`, so the
//! scanned text of this file never spells one.

use super::*;
use crate::repository_laws::temporary_checkout::{TemporaryCheckout, this_repository};

fn matches(pattern: &str, subject: &str) -> bool {
    Pattern::regex(pattern).unwrap().is_match(subject)
}

#[test]
fn an_exemption_list_is_recognised_by_its_file_name() {
    for name in [
        "allowlist.rs",
        ".coding-standards-allowlist.yaml",
        "allow_list.toml",
        "file_length_exemptions.json",
        "test-exemptions.txt",
        concat!("grand", "fathered_rows.rs"),
    ] {
        assert!(
            matches(EXEMPTION_FILE_NAME_RE, name),
            "{name} is an exemption list"
        );
    }
    for name in [
        "map_assets_rate_limit_exemption.rs",
        "exemption_mechanisms.rs",
        "url_allowlist_cases.rs",
        "allowed_hosts.rs",
        "README.md",
    ] {
        assert!(
            !matches(EXEMPTION_FILE_NAME_RE, name),
            "{name} is not an exemption list"
        );
    }
}

#[test]
fn a_directive_names_a_structural_rule_and_an_off_switch() {
    for line in [
        concat!("// file", "-length: allow"),
        concat!("//! SIZE", "-3 = exempt"),
        concat!("/* inline", "-tests: skip */"),
        concat!("// allow", "-long-file"),
    ] {
        assert!(matches(EXEMPTION_DIRECTIVE_RE, line), "{line}");
        assert!(is_comment_line(line), "{line}");
    }
    for line in [
        "// the file-length gate has no exemption list",
        "// hosts on the allowlist: example.org",
        "// skip the header when the file is empty",
    ] {
        assert!(!matches(EXEMPTION_DIRECTIVE_RE, line), "{line}");
    }
}

#[test]
fn an_exemption_table_is_a_declared_name() {
    for line in [
        concat!("struct Grand", "fatherRow {"),
        concat!("pub(super) const ROWS_ALLOW", "LIST: &[Row] = &[];"),
        concat!("static FILE_ALLOW", "_LIST: [&str; 0] = [];"),
    ] {
        assert!(matches(EXEMPTION_TABLE_RE, line), "{line}");
    }
    for line in [
        "const ALLOWED_HOSTS: &[&str] = &[];",
        "fn the_allow_list_refuses_anything_else() {}",
        "pub const RATE_LIMIT_EXEMPT_MOUNT: &str = \"/map-assets\";",
    ] {
        assert!(!matches(EXEMPTION_TABLE_RE, line), "{line}");
    }
}

#[test]
fn the_scan_finds_every_shape_and_nothing_else() {
    let checkout = TemporaryCheckout::with_pinned_roots("exemptions");
    checkout.write(
        "apps/website/frontend/src/v2/tests/doc_audit/allowlist.rs",
        "//! Rows.\n",
    );
    checkout.write(".coding-standards-allowlist.yaml", "- path: a.rs\n");
    checkout.write(
        "tools_v2/xtask/src/gate.rs",
        concat!(
            "fn gate() {}\n",
            "// file",
            "-length: allow\n",
            "const NOTE: &str = \"// file",
            "-length: allow\";\n",
            "struct Grand",
            "fatherRow { path: &'static str }\n",
            "const ALLOWED_HOSTS: &[&str] = &[];\n",
        ),
    );
    let scan = scan_exemption_mechanisms(checkout.root()).unwrap();
    let found: Vec<(ExemptionKind, String, Option<usize>)> = scan
        .findings
        .iter()
        .map(|finding| (finding.kind, finding.path.clone(), finding.line_no))
        .collect();
    assert_eq!(
        found,
        [
            (
                ExemptionKind::ExemptionFile,
                "apps/website/frontend/src/v2/tests/doc_audit/allowlist.rs".to_string(),
                None
            ),
            (
                ExemptionKind::ExemptionFile,
                ".coding-standards-allowlist.yaml".to_string(),
                None
            ),
            (
                ExemptionKind::CommentDirective,
                "tools_v2/xtask/src/gate.rs".to_string(),
                Some(2)
            ),
            (
                ExemptionKind::ExemptionTable,
                "tools_v2/xtask/src/gate.rs".to_string(),
                Some(4)
            ),
        ]
    );
    assert!(
        scan.findings[2]
            .rendered()
            .starts_with("tools_v2/xtask/src/gate.rs:2: CommentDirective")
    );
}

#[test]
fn a_missing_root_is_a_scan_that_did_not_run() {
    let checkout = TemporaryCheckout::empty("exemptions-missing");
    assert!(matches!(
        scan_exemption_mechanisms(checkout.root()),
        Err(NotRun::TargetMissing(_))
    ));
}

#[test]
fn the_scan_over_this_repository_examines_files() {
    let scan = scan_exemption_mechanisms(&this_repository()).expect("the scan runs");
    assert!(scan.files_examined > 1000, "{}", scan.files_examined);
}
