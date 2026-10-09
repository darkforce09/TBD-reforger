//! Unit tests for [`crate::route_drift`]: the route table parse over a crate-shaped `routes.rs`,
//! its refusals, and the live route source.

use super::*;

/// A route table as a born crate's `routes.rs` holds it: a module header, the `RouteDef`
/// definition, the table, and a later top-level array the parse must leave out.
const CRATE_ROUTES_RS: &str = r#"//! The route table.

use crate::prelude::*;

/// One route.
pub struct RouteDef {
    pub path: &'static str,
    pub component: &'static str,
    pub full_bleed: bool,
    pub chromeless: bool,
    pub auth: &'static str,
}

/// Every route.
pub static ROUTES: &[RouteDef] = &[
    RouteDef {
        path: "/login",
        component: "LoginPage",
        full_bleed: false,
        chromeless: true,
        auth: "none",
    },
    RouteDef { path: "/", component: "DashboardPage", full_bleed: true, chromeless: false, auth: "member" },
];

/// Paths the shell preloads.
pub static PRELOADED: &[RouteDef] = &[
    RouteDef {
        path: "/never",
        component: "NotInTheTable",
        full_bleed: false,
        chromeless: false,
        auth: "none",
    },
];
"#;

/// The parse reads the `static ROUTES` table of a crate's `routes.rs`, each field by name, and
/// stops at the table's closing bracket.
#[test]
fn a_crate_routes_file_parses_to_its_table_rows() {
    let rows = route_rows(CRATE_ROUTES_RS).expect("the crate-shaped table parses");
    let rendered: Vec<String> = rows.iter().map(|row| row.join(",")).collect();
    assert_eq!(
        rendered,
        [
            "/login,LoginPage,false,true,none",
            "/,DashboardPage,true,false,member",
        ]
    );
}

/// A source without the table is an error, never an empty route set.
#[test]
fn a_source_without_the_table_is_refused() {
    let refused = route_rows("pub struct RouteDef { pub path: &'static str }\n")
        .expect_err("no table is refused");
    assert!(refused.to_string().contains("static ROUTES"), "{refused}");
}

/// A table that holds no route is an error, never an empty route set.
#[test]
fn an_empty_table_is_refused() {
    let refused = route_rows("pub static ROUTES: &[RouteDef] = &[\n];\n")
        .expect_err("an empty table is refused");
    assert!(refused.to_string().contains("no `RouteDef`"), "{refused}");
}

/// The live route source exists, is a Rust file, and parses to routes that each name a path and a
/// component, the `/` root among them.
#[test]
fn the_live_route_source_parses() {
    let root = ::repository_root::find_repository_root_from(std::path::Path::new(env!(
        "CARGO_MANIFEST_DIR"
    )))
    .expect("repository root");
    assert!(ROUTE_TABLE_SOURCE.ends_with(".rs"), "{ROUTE_TABLE_SOURCE}");
    let source = std::fs::read_to_string(root.join(ROUTE_TABLE_SOURCE))
        .unwrap_or_else(|why| panic!("read {ROUTE_TABLE_SOURCE}: {why}"));
    let rows = route_rows(&source).expect("the live route table parses");
    assert!(
        rows.iter()
            .all(|row| !row[0].is_empty() && !row[1].is_empty()),
        "{rows:?}"
    );
    assert!(rows.iter().any(|row| row[0] == "/"), "{rows:?}");
}
