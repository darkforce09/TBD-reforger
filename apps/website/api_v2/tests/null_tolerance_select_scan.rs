//! The enumerating half: every `SELECT` literal in `src/`, cross-referenced against
//! `information_schema` nullability.
//!
//! Needs no seed and no predicate, so it reaches read sites the behavioural sweep cannot.
//!
//! Skips without `TEST_DATABASE_URL`.

use std::collections::BTreeSet;

use website_api::core::database;

mod common;
mod null_tolerance_support;

use null_tolerance_support::database_fixtures::*;
use null_tolerance_support::source_scan::*;
use null_tolerance_support::*;

/// The enumerating half, and the check that catches the *fourth* instance.
///
/// A behavioural sweep only reaches code whose predicates the seed satisfies; this one needs no
/// predicate at all. For every `SELECT` literal handed to `query_as` / `QueryBuilder::new` in
/// `src/`, it cross-references the select list against `information_schema` nullability and
/// fails on:
///   * a bare `*` / `t.*` over a table that has nullable columns (the model
///     silently acquires whatever nullability the DDL has), or
///   * a nullable column selected without `COALESCE` and not in [`OPTION_FIELDS`].
///
/// Known limit: queries whose select list is assembled at runtime (`QueryBuilder::push`) are
/// only checked as far as their literal prefix.
#[tokio::test]
async fn no_query_as_reads_a_nullable_column_without_coalesce() {
    let Some(url) = common::require_test_database_url() else {
        eprintln!("skip: TEST_DATABASE_URL unset");
        return;
    };
    let pool = database::connect(&url).await.expect("connect");
    database::migrate(&pool).await.expect("migrate");
    let nullable = nullable_columns(&pool).await;
    let allow: BTreeSet<(&str, &str)> = OPTION_FIELDS.iter().copied().collect();

    let src_root = concat!(env!("CARGO_MANIFEST_DIR"), "/src");
    let mut files = Vec::new();
    collect_rs(std::path::Path::new(src_root), &mut files);
    assert!(
        files.len() > 20,
        "found only {} .rs files under {src_root}",
        files.len()
    );

    // (source file, produced column or `*`, human-readable finding) — the first two are the
    // KNOWN_OPEN key.
    let mut findings: Vec<(String, String, String)> = Vec::new();
    let mut statements = 0usize;
    for path in &files {
        let src = std::fs::read_to_string(path).expect("read source");
        let rel = path
            .strip_prefix(env!("CARGO_MANIFEST_DIR"))
            .unwrap_or(path)
            .display()
            .to_string();
        for (line, sql) in select_literals(&src) {
            statements += 1;
            let aliases = table_aliases(&sql);
            let live: Vec<&str> = aliases
                .values()
                .map(String::as_str)
                .filter(|t| nullable.contains_key(*t))
                .collect();
            if live.is_empty() {
                continue;
            }
            for item in select_items(&sql) {
                let it = item.trim();
                if it == "*" || (it.ends_with(".*") && !it.contains(' ')) {
                    findings.push((
                        rel.clone(),
                        "*".into(),
                        format!(
                            "{rel}:{line}  bare `{it}` over nullable table(s) {live:?} — spell \
                             the columns out and COALESCE the nullable ones"
                        ),
                    ));
                    continue;
                }
                let Some((produced, expr, qualifier)) = produced_column(it) else {
                    continue;
                };
                if expr.to_uppercase().contains("COALESCE") {
                    continue;
                }
                // Prefer the alias when the item is qualified — that names the table exactly.
                let candidates: Vec<&str> = match qualifier.and_then(|q| aliases.get(q)) {
                    Some(t) => vec![t.as_str()],
                    None => live
                        .iter()
                        .copied()
                        .filter(|t| nullable[*t].contains(&produced))
                        .collect(),
                };
                let offenders: Vec<&str> = candidates
                    .into_iter()
                    .filter(|t| {
                        nullable.get(*t).is_some_and(|cs| cs.contains(&produced))
                            && !allow.contains(&(*t, produced.as_str()))
                    })
                    .collect();
                if !offenders.is_empty() {
                    findings.push((
                        rel.clone(),
                        produced.clone(),
                        format!(
                            "{rel}:{line}  `{produced}` is nullable on {offenders:?} and is \
                             selected without COALESCE"
                        ),
                    ));
                }
            }
        }
    }
    assert!(
        statements > 60,
        "extracted only {statements} SELECT literals — the extractor has drifted"
    );
    findings.sort();
    findings.dedup();

    let stale: Vec<(&str, &str)> = KNOWN_OPEN
        .iter()
        .filter(|(f, c, _)| !findings.iter().any(|(rf, rc, _)| rf == f && rc == c))
        .map(|(f, c, _)| (*f, *c))
        .collect();
    if !stale.is_empty() {
        eprintln!(
            "note: KNOWN_OPEN names defects this scan no longer finds — they are fixed, so prune \
             them from tests/null_tolerance_support/mod.rs: {stale:?}"
        );
    }

    let new: Vec<&String> = findings
        .iter()
        .filter(|(f, c, _)| !KNOWN_OPEN.iter().any(|(kf, kc, _)| kf == f && kc == c))
        .map(|(_, _, msg)| msg)
        .collect();
    assert!(
        new.is_empty(),
        "nullable column(s) read into a non-Option field. Fix by adding COALESCE to the query \
         (NOT by making the model field Option — see match_telemetry::models::match_record::Match). If the field really \
         is Option<..>, add the pair to OPTION_FIELDS naming the model.\n  {}",
        new.iter()
            .map(|s| s.as_str())
            .collect::<Vec<_>>()
            .join("\n  ")
    );
}
