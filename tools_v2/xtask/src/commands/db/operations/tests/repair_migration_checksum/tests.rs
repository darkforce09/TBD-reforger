use super::*;

const APPLIED: &str = "\
-- T-578 — the table the durable rate limiter needs.
--
-- It shipped instead as `const RATE_LIMIT_BUCKETS_DDL` in `src/app.rs`.

CREATE TABLE IF NOT EXISTS rate_limit_buckets (
    key TEXT PRIMARY KEY,
    tokens INTEGER NOT NULL
);
";

/// The real shape of the drift this command exists for: the header comment was rewritten to fix
/// stale module paths, and the statements were not touched.
const CURRENT: &str = "\
-- The table the durable rate limiter needs.
--
-- `core::middleware::durable_ratelimit::PgRateLimiter` binds this shape.

CREATE TABLE IF NOT EXISTS rate_limit_buckets (
    key TEXT PRIMARY KEY,
    tokens INTEGER NOT NULL
);
";

#[test]
fn a_rewritten_comment_block_is_comments_only() {
    assert_eq!(classify(CURRENT, APPLIED), Change::CommentsOnly);
}

#[test]
fn identical_bytes_need_no_repair() {
    assert_eq!(classify(APPLIED, APPLIED), Change::Identical);
}

/// The refusal that matters: a changed column type is not a comment edit, however similar the
/// surrounding file looks.
#[test]
fn a_changed_statement_is_never_comments_only() {
    let altered = CURRENT.replace("tokens INTEGER NOT NULL", "tokens BIGINT NOT NULL");
    assert_eq!(classify(&altered, APPLIED), Change::DdlChanged);
}

#[test]
fn an_added_statement_is_ddl_changed() {
    let altered = format!("{CURRENT}CREATE INDEX ON rate_limit_buckets (tokens);\n");
    assert_eq!(classify(&altered, APPLIED), Change::DdlChanged);
}

/// A `--` inside a string literal is content. Stripping it as a comment could make two different
/// statements normalise to the same text, which is the one way this tool could do harm.
#[test]
fn a_double_dash_inside_a_string_literal_is_not_a_comment() {
    let a = "INSERT INTO t (v) VALUES ('a -- b');\n";
    let b = "INSERT INTO t (v) VALUES ('a -- c');\n";
    assert_eq!(
        normalize_sql(a).trim(),
        "INSERT INTO t (v) VALUES ('a -- b');"
    );
    assert_eq!(classify(a, b), Change::DdlChanged);
}

#[test]
fn normalization_drops_comments_and_blank_lines_only() {
    let sql = "-- lead\n\nSELECT 1;  -- trailing\n\n";
    assert_eq!(normalize_sql(sql), "SELECT 1;\n");
}

#[test]
fn version_comes_from_the_filename_prefix() {
    assert_eq!(version_of("0021_rate_limit_buckets.sql"), Some(21));
    assert_eq!(version_of("0025_audit_notify.sql"), Some(25));
    assert_eq!(version_of("README.md"), None);
}

#[test]
fn the_diff_shows_both_sides_of_the_comment_change() {
    let diff = comment_diff(CURRENT, APPLIED);
    assert!(diff.contains("- -- T-578"), "{diff}");
    assert!(diff.contains("+ -- The table"), "{diff}");
    // The unchanged DDL must not appear as a change on either side.
    assert!(!diff.contains("CREATE TABLE"), "{diff}");
}
