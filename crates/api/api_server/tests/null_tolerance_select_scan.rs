//! The enumerating half: every `SELECT` literal in `src/` and in the API crates under
//! `crates/api/`, cross-referenced against `information_schema` nullability.
//!
//! Needs no seed and no predicate, so it reaches read sites the behavioural sweep cannot.
//!
//! Skips without `TEST_DATABASE_URL`.

use std::collections::BTreeSet;

mod common;
mod null_tolerance_support;

use null_tolerance_support::database_fixtures::*;
use null_tolerance_support::source_scan::*;
use null_tolerance_support::*;

/// The enumerating half, and the check that catches the *fourth* instance.
///
/// A behavioural sweep only reaches code whose predicates the seed satisfies; this one needs no
/// predicate at all. For every `SELECT` literal handed to `query_as` / `QueryBuilder::new` in
/// `src/` and `crates/api/`, a `concat!` of literals and column-list macros read whole, it cross-references the select list against `information_schema` nullability and
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
    let pool = api_database::connect(&url).await.expect("connect");
    api_database::migrate(&pool).await.expect("migrate");
    let nullable = nullable_columns(&pool).await;
    let allow: BTreeSet<(&str, &str)> = OPTION_FIELDS.iter().copied().collect();

    // This package's `src/` and every other API crate under `crates/api/`, where the domains'
    // reads live; this package sits there too and is read once, through its `src/`.
    let manifest = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let repository_root = repository_root::find_repository_root_from(manifest)
        .expect("the repository root above the API package");
    let mut files = Vec::new();
    collect_rs(&manifest.join("src"), &mut files);
    let packages = common::api_packages().unwrap_or_else(|e| panic!("the API package list: {e}"));
    for package in packages.iter().filter(|package| !package.is_application) {
        collect_rs(&package.folder, &mut files);
    }
    assert!(
        files.len() > 20,
        "found only {} .rs files under src/ and crates/api/",
        files.len()
    );

    // (source file, produced column or `*`, human-readable finding) — the first two are the
    // KNOWN_OPEN key.
    let mut findings: Vec<(String, String, String)> = Vec::new();
    let mut statements = 0usize;
    let sources: Vec<String> = files
        .iter()
        .map(|path| std::fs::read_to_string(path).expect("read source"))
        .collect();
    // Column-list macros are shared across files, so `concat!` parts expand against the tree.
    let macro_source = sources.concat();
    for (path, src) in files.iter().zip(&sources) {
        let rel = path
            .strip_prefix(manifest)
            .or_else(|_| path.strip_prefix(&repository_root))
            .unwrap_or(path)
            .display()
            .to_string();
        for (line, sql) in select_literals(src, &macro_source) {
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
         (NOT by making the model field Option — see api_match_telemetry::models::match_record::Match). If the field really \
         is Option<..>, add the pair to OPTION_FIELDS naming the model.\n  {}",
        new.iter()
            .map(|s| s.as_str())
            .collect::<Vec<_>>()
            .join("\n  ")
    );
}

/// A `SELECT` assembled with `concat!` from literals and column-list macros is read whole, so the
/// scan above inspects its select list rather than a bare `SELECT ` prefix.
#[test]
fn game_ballistics_select_scan_reads_concat_built_queries() {
    let src = r##"
macro_rules! row_columns {
    () => {
        "id, \
         nullable_note"
    };
}
async fn read() {
    let rows: Vec<Row> = sqlx::query_as(concat!(
        "SELECT ",
        row_columns!(),
        " FROM notes WHERE id = $1"
    ));
}
"##;
    let statements = select_literals(src, src);
    assert_eq!(
        statements,
        vec![(
            9,
            "SELECT id, nullable_note FROM notes WHERE id = $1".to_owned()
        )]
    );
    assert_eq!(
        select_items(&statements[0].1),
        vec!["id".to_owned(), "nullable_note".to_owned()]
    );

    // The shipped fire-mission store builds its reads this way; both are scanned whole.
    let store = include_str!("../../api_operations/src/services/fire_mission_store.rs");
    let tables: Vec<String> = select_literals(store, store)
        .iter()
        .flat_map(|(_, sql)| table_aliases(sql).into_values())
        .collect();
    assert!(
        tables.contains(&"fire_missions".to_owned())
            && tables.contains(&"fire_mission_guns".to_owned()),
        "the store's concat!-built SELECTs are scanned: {tables:?}"
    );
}
