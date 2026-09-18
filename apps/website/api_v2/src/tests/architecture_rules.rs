//! Executable statements of this crate's layout rules, checked against the source text of `src/`.
//!
//! These are the constraints a type checker cannot express: a module tree shaped by domain rather
//! than by layer, a `core` that the domains depend on and that depends on none of them, test code
//! kept in sibling files, and prose that describes the code as it stands. Each rule reads the tree
//! itself, so a new file is covered the moment it is added — nothing has to be registered here.
//!
//! **Two scopes are excluded from the walk.**
//!
//! * `missions/contract/generated/` — emitted by `cargo xtask ci schema-codegen` from
//!   `contracts_v2`. Its prose belongs to the generator, so the prose rules would only ever
//!   report the generator's own habits at a file no one edits.
//! * this file — it holds every forbidden token as a literal needle, so scanning it would make
//!   each rule report itself.
//!
//! **The import rules read code lines only.** A rustdoc intra-doc link (`//!` / `///`) naming a
//! path creates no compile-time dependency: it is a pointer for a reader, and the crate uses them
//! deliberately to connect a service to the worker or handler at its other end. The rules here
//! constrain the dependency graph, so they skip comment lines and read what the compiler reads.

use std::path::{Path, PathBuf};

/// The eight domain modules `core::http_router` merges into `/api/v1`.
const DOMAINS: [&str; 8] = [
    "administration",
    "command_center",
    "community_content",
    "identity_and_access",
    "match_telemetry",
    "missions",
    "operations",
    "server_infrastructure",
];

/// Codegen output: exempt from the prose rules (see the module header).
const GENERATED_SUBTREE: &str = "missions/contract/generated";

/// This file, relative to `src/` — excluded from its own scans.
const THIS_FILE: &str = "tests/architecture_rules.rs";

/// Floor for a full-tree scan. The crate holds well over 200 source files; a walk that returns
/// fewer than this has lost the tree (wrong root, a silent read error) and every rule below it
/// would pass vacuously.
const FULL_TREE_FLOOR: usize = 100;

/// Floor for a `core/`-only scan.
const CORE_FLOOR: usize = 25;

/// `src/` of this crate, resolved from the manifest so the tests do not depend on the working
/// directory a test runner happens to use.
fn source_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("src")
}

/// Every `.rs` file under `root`, paired with its text, sorted by path so a failure message reads
/// the same on every machine. Skips the two excluded scopes.
fn rust_sources(root: &Path) -> Vec<(PathBuf, String)> {
    let mut files = Vec::new();
    collect_rust_sources(root, &mut files);
    files.sort_by(|a, b| a.0.cmp(&b.0));
    files
}

/// Depth-first half of [`rust_sources`].
fn collect_rust_sources(dir: &Path, files: &mut Vec<(PathBuf, String)>) {
    let entries = std::fs::read_dir(dir)
        .unwrap_or_else(|e| panic!("cannot read {}: {e}", dir.display()))
        .map(|entry| entry.expect("directory entry").path());
    for path in entries {
        if path.is_dir() {
            if !path.ends_with(GENERATED_SUBTREE) {
                collect_rust_sources(&path, files);
            }
        } else if path.extension().is_some_and(|ext| ext == "rs") && relative(&path) != THIS_FILE {
            let text = std::fs::read_to_string(&path)
                .unwrap_or_else(|e| panic!("cannot read {}: {e}", path.display()));
            files.push((path, text));
        }
    }
}

/// A path written the way the rules talk about it: relative to `src/`.
fn relative(path: &Path) -> String {
    path.strip_prefix(source_root())
        .unwrap_or(path)
        .display()
        .to_string()
}

/// Fail unless the walk saw enough files to make the rule meaningful.
fn assert_walk_is_not_vacuous(files: &[(PathBuf, String)], floor: usize, scope: &str) {
    assert!(
        files.len() >= floor,
        "the {scope} walk returned only {} file(s), fewer than the floor of {floor} — \
         the walker has lost the tree and every rule over it would pass vacuously",
        files.len()
    );
}

/// Fail with every offending `path:line` listed, one per line.
fn assert_no_offenders(rule: &str, offenders: &[String]) {
    assert!(
        offenders.is_empty(),
        "{rule}: {} violation(s)\n{}",
        offenders.len(),
        offenders.join("\n")
    );
}

/// Walk every line of every file, collecting `path:line — reason` for each hit.
///
/// `read_comments` chooses the scope: `true` scans the whole text (the prose rules), `false` skips
/// comment-only lines so the import rules read what the compiler reads.
fn offenders(
    files: &[(PathBuf, String)],
    read_comments: bool,
    mut hit: impl FnMut(&Path, &str) -> Option<String>,
) -> Vec<String> {
    let mut found = Vec::new();
    for (path, text) in files {
        for (index, line) in text.lines().enumerate() {
            if !read_comments && line.trim_start().starts_with("//") {
                continue;
            }
            if let Some(reason) = hit(path, line) {
                found.push(format!("{}:{} — {reason}", relative(path), index + 1));
            }
        }
    }
    found
}

/// The domain a file belongs to, or `None` for `core`, `background_workers` and the binary.
fn domain_of(path: &Path) -> Option<&'static str> {
    let relative = relative(path);
    DOMAINS
        .iter()
        .copied()
        .find(|domain| relative.starts_with(&format!("{domain}/")))
}

/// The name in `mod <name> {` — an inline module body, as opposed to a `mod <name>;` declaration.
fn inline_module_name(trimmed_line: &str) -> Option<&str> {
    let (name, tail) = trimmed_line.strip_prefix("mod ")?.split_once(' ')?;
    let is_identifier = !name.is_empty()
        && name
            .bytes()
            .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'_');
    (tail == "{" && is_identifier).then_some(name)
}

/* ══════════ prose and test placement ══════════ */

/// Unit tests live in sibling files under `tests/`, declared with `#[cfg(test)] #[path = …] mod`.
/// An inline `mod tests { … }` body buries the tests inside the production file and pushes it over
/// the 500-line ceiling from the inside.
#[test]
fn no_inline_test_modules() {
    let files = rust_sources(&source_root());
    assert_walk_is_not_vacuous(&files, FULL_TREE_FLOOR, "src/");

    let mut found = Vec::new();
    for (path, text) in &files {
        let mut under_cfg_test = false;
        for (index, line) in text.lines().enumerate() {
            let trimmed = line.trim();
            if trimmed.is_empty() || trimmed.starts_with("//") {
                continue;
            }
            if trimmed == "#[cfg(test)]" {
                under_cfg_test = true;
                continue;
            }
            if let Some(name) = inline_module_name(trimmed)
                && (name == "tests" || under_cfg_test)
            {
                found.push(format!(
                    "{}:{} — inline `mod {name} {{`; move the body to a sibling file declared \
                     with #[cfg(test)] #[path = \"tests/<file>.rs\"]",
                    relative(path),
                    index + 1
                ));
            }
            if !trimmed.starts_with("#[") {
                under_cfg_test = false;
            }
        }
    }
    assert_no_offenders("no_inline_test_modules", &found);
}

/// Comments describe the code as it stands. A ticket id is a pointer into a registry the reader of
/// this crate does not have, and it outlives the work it named.
#[test]
fn no_ticket_references_in_source() {
    let files = rust_sources(&source_root());
    assert_walk_is_not_vacuous(&files, FULL_TREE_FLOOR, "src/");

    let found = offenders(&files, true, |_, line| {
        let bytes = line.as_bytes();
        let is_ticket = bytes
            .windows(3)
            .any(|window| window[0] == b'T' && window[1] == b'-' && window[2].is_ascii_digit());
        is_ticket.then(|| {
            "ticket reference; say what the code does, not which ticket changed it".to_string()
        })
    });
    assert_no_offenders("no_ticket_references_in_source", &found);
}

/// The crate is not described in terms of a system it is not. Prose that compares this code to
/// another implementation dates on the day that implementation stops existing, and sends the
/// reader looking for files this repository does not contain.
#[test]
fn no_go_port_narrative() {
    let files = rust_sources(&source_root());
    assert_walk_is_not_vacuous(&files, FULL_TREE_FLOOR, "src/");

    const PHRASES: [&str; 5] = ["Rust port", "Go API", "internal/", "GORM", "parsePage"];
    let found = offenders(&files, true, |_, line| {
        if let Some(phrase) = PHRASES.iter().find(|phrase| line.contains(**phrase)) {
            return Some(format!("`{phrase}` — describe this crate, not another one"));
        }
        names_a_go_file(line).then(|| "names a `.go` file — this crate has none".to_string())
    });
    assert_no_offenders("no_go_port_narrative", &found);
}

/// Whether the line mentions a `.go` file: `.go` not continued by another word character, so
/// `.google` and the like do not read as a filename.
fn names_a_go_file(line: &str) -> bool {
    let mut rest = line;
    while let Some(at) = rest.find(".go") {
        let after = &rest[at + 3..];
        let continues = after
            .chars()
            .next()
            .is_some_and(|c| c.is_alphanumeric() || c == '_');
        if !continues {
            return true;
        }
        rest = after;
    }
    false
}

/* ══════════ the dependency graph ══════════ */

/// `core` is the foundation every domain is allowed to depend on, so it must depend on none of
/// them — otherwise the two halves are mutually recursive and neither can be read, moved or tested
/// without the other. The composition root is the exception by definition: `application_state`
/// constructs the domain services the state carries, and `http_router` merges the domain route
/// tables. Both are the places where the wiring is supposed to be visible.
#[test]
fn core_imports_no_domain_except_composition_root() {
    const COMPOSITION_ROOT: [&str; 2] = ["core/application_state.rs", "core/http_router.rs"];
    let files = rust_sources(&source_root().join("core"));
    assert_walk_is_not_vacuous(&files, CORE_FLOOR, "src/core/");

    let found = offenders(&files, false, |path, line| {
        if COMPOSITION_ROOT.contains(&relative(path).as_str()) {
            return None;
        }
        let domain = DOMAINS
            .iter()
            .find(|domain| line.contains(&format!("crate::{domain}::")))?;
        Some(format!(
            "`crate::{domain}::` — core must not depend on a domain; the composition root \
             ({}) is the only place that wires one in",
            COMPOSITION_ROOT.join(", ")
        ))
    });
    assert_no_offenders("core_imports_no_domain_except_composition_root", &found);
}

/// A handler is one domain's HTTP surface: its extractors, its status codes, its wire shapes.
/// Calling another domain's handler borrows all of that along with the logic, and couples two
/// route tables that should only ever meet in the router. Cross-domain reuse goes through the
/// owning domain's `models` (a shape) or `services` (a behaviour), both of which are free of HTTP.
#[test]
fn domain_handlers_import_no_foreign_handlers() {
    let files = rust_sources(&source_root());
    assert_walk_is_not_vacuous(&files, FULL_TREE_FLOOR, "src/");

    let found = offenders(&files, false, |path, line| {
        let own = domain_of(path)?;
        let relative = relative(path);
        let in_domain_layer = ["/handlers/", "/services/", "/models/"]
            .iter()
            .any(|layer| relative.contains(layer));
        if !in_domain_layer {
            return None;
        }
        let foreign = DOMAINS
            .iter()
            .find(|d| **d != own && line.contains(&format!("crate::{d}::handlers")))?;
        Some(format!(
            "`{own}` reaches into `crate::{foreign}::handlers` — cross-domain access goes \
             through `{foreign}::models` or `{foreign}::services`"
        ))
    });
    assert_no_offenders("domain_handlers_import_no_foreign_handlers", &found);
}

/// The interval tasks are armed once, by the process that owns a lifetime long enough to run them.
/// A handler or service that reaches for `background_workers` is either spawning a second copy of
/// a worker per request or calling a worker's body outside the schedule that makes it safe; either
/// way the work belongs in a service both the worker and the caller can share.
#[test]
fn background_workers_used_only_by_the_binary() {
    const ARMED_BY: [&str; 2] = ["bin/api.rs", "lib.rs"];
    let files = rust_sources(&source_root());
    assert_walk_is_not_vacuous(&files, FULL_TREE_FLOOR, "src/");

    let found = offenders(&files, false, |path, line| {
        if ARMED_BY.contains(&relative(path).as_str()) {
            return None;
        }
        let names_workers = line.contains("crate::background_workers")
            || line.contains("website_api::background_workers");
        names_workers.then(|| {
            format!(
                "names `background_workers` — only the binary arms them ({})",
                ARMED_BY.join(", ")
            )
        })
    });
    assert_no_offenders("background_workers_used_only_by_the_binary", &found);
}

/* ══════════ the shape of a domain ══════════ */

/// Every domain owns exactly one route table, and the router merges all eight. A domain whose
/// table is not merged compiles, passes its own tests, and answers 404 on the wire — so both
/// halves are pinned: the table exists, and the router names it.
#[test]
fn every_domain_exports_a_route_table() {
    let files = rust_sources(&source_root());
    assert_walk_is_not_vacuous(&files, FULL_TREE_FLOOR, "src/");

    let router_path = source_root().join("core/http_router.rs");
    let router = std::fs::read_to_string(&router_path)
        .unwrap_or_else(|e| panic!("cannot read {}: {e}", router_path.display()));

    let mut found = Vec::new();
    for domain in DOMAINS {
        let table = source_root().join(domain).join("routes.rs");
        match std::fs::read_to_string(&table) {
            Ok(text) if text.contains("pub fn routes(") => {}
            Ok(_) => found.push(format!(
                "{domain}/routes.rs:1 — no `pub fn routes(`; every domain exports one table"
            )),
            Err(e) => found.push(format!("{domain}/routes.rs:1 — cannot read: {e}")),
        }
        let merged = format!("crate::{domain}::routes(");
        if !router.contains(&merged) {
            found.push(format!(
                "core/http_router.rs:1 — does not merge `{merged}`; the domain would answer 404"
            ));
        }
    }
    assert_no_offenders("every_domain_exports_a_route_table", &found);
}

/// The crate is organised by domain, not by layer. A top-level `handlers/`, `services/` or
/// `models/` re-opens the layer split, and each of the named files is a second home for something
/// that now lives in `core` — two homes meaning two versions of the same truth.
#[test]
fn no_legacy_top_level_modules() {
    let files = rust_sources(&source_root());
    assert_walk_is_not_vacuous(&files, FULL_TREE_FLOOR, "src/");

    const LAYER_DIRECTORIES: [&str; 5] = ["handlers", "services", "models", "contract", "auth"];
    const DISPLACED_FILES: [&str; 5] = ["app.rs", "state.rs", "db.rs", "config.rs", "realtime.rs"];

    let mut found = Vec::new();
    for name in LAYER_DIRECTORIES {
        if source_root().join(name).is_dir() {
            found.push(format!(
                "{name}/:1 — top-level layer directory; this crate splits by domain, and each \
                 domain owns its own {name}"
            ));
        }
    }
    for name in DISPLACED_FILES {
        if source_root().join(name).is_file() {
            found.push(format!(
                "{name}:1 — top-level module; its home is under `core/`"
            ));
        }
    }
    assert_no_offenders("no_legacy_top_level_modules", &found);
}
