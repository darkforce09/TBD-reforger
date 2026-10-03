//! Executable statements of the API's layout rules, checked against the source text and the Cargo
//! manifests of the API application (`apps/api`) and of every API crate (`crates/api/*`).
//!
//! These are the constraints a type checker cannot express: an application that only assembles
//! the crates below it, a crate graph shaped by domain (kernel crates below every domain, the
//! domains along their one-way graph, the background workers above them, the application on
//! top), one route table per domain merged by the router, test code kept in sibling files, and
//! prose that describes the code as it stands. Each rule reads the tree itself, so a new crate or
//! file is covered the moment it is added — nothing has to be registered here.
//!
//! **The crate graph is read from the manifests.** A crate boundary makes an import of a crate
//! that is not a dependency a compile error, so the graph the rules judge is the dependency
//! tables of each `Cargo.toml` (normal and dev), not the source imports.
//!
//! **One scope is excluded from the source walk:** the rule files under `apps/api/src/tests/` —
//! this one and `prose_rules.rs` — which hold every forbidden token as a literal needle, so
//! scanning them would make each rule report itself.
//!
//! **The import rules read code lines only.** A rustdoc intra-doc link (`//!` / `///`) naming a
//! path creates no compile-time dependency: it is a pointer for a reader, and the crates use them
//! deliberately to connect a service to the worker or handler at its other end. The rules here
//! constrain the dependency graph, so they skip comment lines and read what the compiler reads.

use std::path::{Path, PathBuf};

use repository_laws::cargo_manifest::{CargoManifest, read_manifest};

/// The eight domain crates (`api_<domain>`) whose route tables `src/router.rs` merges into
/// `/api/v1`.
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

/// The domain graph: the domains each domain crate may depend on. A domain with an empty list
/// depends on no other domain.
const DOMAIN_DEPENDENCIES: [(&str, &[&str]); 8] = [
    ("community_content", &[]),
    ("identity_and_access", &[]),
    ("administration", &["identity_and_access"]),
    ("server_infrastructure", &["community_content"]),
    ("match_telemetry", &["server_infrastructure"]),
    ("missions", &["community_content", "server_infrastructure"]),
    (
        "operations",
        &[
            "identity_and_access",
            "match_telemetry",
            "missions",
            "server_infrastructure",
        ],
    ),
    (
        "command_center",
        &[
            "community_content",
            "identity_and_access",
            "missions",
            "operations",
            "server_infrastructure",
        ],
    ),
];

/// The package of the API application, which may depend on every API crate.
const APPLICATION: &str = "api";

/// The crate of the interval tasks, which sits above every domain and below the application.
const WORKERS: &str = "api_background_workers";

/// The one source file that names the workers crate: the binary that arms them.
const WORKERS_ARMED_BY: &str = "apps/api/src/bin/api.rs";

/// The rule files, relative to the repository root — excluded from the source walk, since each
/// holds every forbidden token as a literal needle.
const RULE_FILES: [&str; 2] = [
    "apps/api/src/tests/architecture_rules.rs",
    "apps/api/src/tests/prose_rules.rs",
];

/// The entries of the application's `src/`: the library root, the router, the composition root,
/// the binaries, the rule and router tests, and its README.
const APPLICATION_ENTRIES: [&str; 6] = [
    "README.md",
    "bin",
    "composition.rs",
    "lib.rs",
    "router.rs",
    "tests",
];

/// The entries of the application's `src/bin/`: the server, the registry import tool, the README.
const BINARY_ENTRIES: [&str; 3] = ["README.md", "api.rs", "import_registry.rs"];

/// The entries of the application's `src/tests/`: the two rule files and the router's tests.
const APPLICATION_TEST_ENTRIES: [&str; 3] =
    ["architecture_rules.rs", "prose_rules.rs", "router.rs"];

/// The Rust files of the application's `src/` outside the rule files: `lib.rs`, `router.rs`,
/// `composition.rs`, the two binaries and the router's tests. Fewer means the walk lost the tree.
const APPLICATION_SOURCE_FLOOR: usize = 6;

/// The API crates under `crates/api/`: the kernel crates, the eight domains and the workers.
const API_CRATE_FLOOR: usize = 23;

/// The Rust files of every API crate's `src/`, tests included (550 when the floor was set).
const API_CRATE_SOURCE_FLOOR: usize = 500;

/// The domain-to-domain edges of the domain crates' manifests (14 when the floor was set, every
/// edge of [`DOMAIN_DEPENDENCIES`]); fewer means the manifest reader lost them.
const DOMAIN_EDGE_FLOOR: usize = 14;

/// The application's folder, resolved from the manifest so the tests do not depend on the working
/// directory a test runner happens to use.
fn application_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

/// The repository root, found above the application's manifest folder.
fn repository_root() -> PathBuf {
    repository_layout::find_repository_root_from(&application_root())
        .expect("the repository root above the API package")
}

/// Every API crate under `crates/api/` (each folder holding a `Cargo.toml`), sorted by name.
fn api_crates() -> Vec<String> {
    let root = repository_root().join("crates/api");
    let crates: Vec<String> = entries_of(&root)
        .into_iter()
        .filter(|name| root.join(name).join("Cargo.toml").is_file())
        .collect();
    assert!(
        crates.len() >= API_CRATE_FLOOR,
        "found {} API crate(s) under crates/api/, fewer than the floor of {API_CRATE_FLOOR} — the \
         crate walk has lost the tree and every rule over it would pass vacuously",
        crates.len()
    );
    crates
}

/// The domain an API package holds, or `None` for a kernel crate, the workers or the application.
fn domain_of_crate(name: &str) -> Option<&'static str> {
    let domain = name.strip_prefix("api_")?;
    DOMAINS.iter().copied().find(|known| *known == domain)
}

/// The manifest of `package` (the application's or an API crate's), with its repository-relative
/// path.
fn manifest_of(package: &str) -> (String, CargoManifest) {
    let relative = if package == APPLICATION {
        "apps/api/Cargo.toml".to_string()
    } else {
        format!("crates/api/{package}/Cargo.toml")
    };
    let manifest = read_manifest(&repository_root().join(&relative))
        .unwrap_or_else(|e| panic!("cannot read {relative}: {e:?}"));
    (relative, manifest)
}

/// Every edge of `package`'s manifest, normal or dev, to another API package, as `(line, target)`.
fn api_edges(package: &str) -> (String, Vec<(usize, String)>) {
    let (relative, manifest) = manifest_of(package);
    let edges = manifest
        .dependencies
        .iter()
        .filter(|edge| edge.package.starts_with("api_") || edge.package == APPLICATION)
        .map(|edge| (edge.line_no, edge.package.clone()))
        .collect();
    (relative, edges)
}

/// Why an edge from the API package `source` to the API package `target` breaks the crate graph,
/// or `None` when it is allowed. The application may depend on every API crate; the workers on
/// every domain and kernel crate; a domain on a kernel crate and on the domains
/// [`DOMAIN_DEPENDENCIES`] lists for it; a kernel crate on kernel crates only; nothing on the
/// application, and nothing but the application on the workers.
fn crate_edge_violation(source: &str, target: &str) -> Option<String> {
    if source == target {
        return None;
    }
    if target == APPLICATION {
        return Some(format!(
            "`{source}` depends on the application `{APPLICATION}`; nothing depends on it"
        ));
    }
    if source == APPLICATION {
        return None;
    }
    if target == WORKERS {
        return Some(format!(
            "`{source}` depends on `{WORKERS}`; only the application arms the workers"
        ));
    }
    let target_domain = domain_of_crate(target)?;
    if source == WORKERS {
        return None;
    }
    let Some(source_domain) = domain_of_crate(source) else {
        return Some(format!(
            "kernel crate `{source}` depends on the domain crate `{target}`; the kernel crates \
             sit below every domain"
        ));
    };
    let allowed = DOMAIN_DEPENDENCIES
        .iter()
        .find(|(domain, _)| *domain == source_domain)
        .is_some_and(|(_, dependencies)| dependencies.contains(&target_domain));
    (!allowed).then(|| {
        format!(
            "domain `{source_domain}` depends on domain `{target_domain}`, an edge the domain \
             graph (`DOMAIN_DEPENDENCIES`) does not hold"
        )
    })
}

/// Every manifest edge from a package `sources` selects to a package `targets` selects that
/// [`crate_edge_violation`] refuses, as `manifest:line — reason`.
fn graph_offenders(sources: impl Fn(&str) -> bool, targets: impl Fn(&str) -> bool) -> Vec<String> {
    let mut packages = api_crates();
    packages.push(APPLICATION.to_string());
    let mut found = Vec::new();
    for package in packages.iter().filter(|package| sources(package)) {
        let (manifest, edges) = api_edges(package);
        for (line, target) in edges.iter().filter(|(_, target)| targets(target)) {
            if let Some(reason) = crate_edge_violation(package, target) {
                found.push(format!("{manifest}:{line} — {reason}"));
            }
        }
    }
    found
}

/// Every `.rs` file of the application's `src/` and of every API crate's `src/`, paired with its
/// text and sorted by path, the rule files skipped. Each root must hold its floor of files.
fn every_api_source() -> Vec<(PathBuf, String)> {
    let mut files = rust_sources(&application_root().join("src"));
    assert_walk_is_not_vacuous(files.len(), APPLICATION_SOURCE_FLOOR, "apps/api/src/");
    let application_files = files.len();
    for name in api_crates() {
        files.extend(rust_sources(
            &repository_root().join("crates/api").join(name).join("src"),
        ));
    }
    assert_walk_is_not_vacuous(
        files.len() - application_files,
        API_CRATE_SOURCE_FLOOR,
        "crates/api/*/src/",
    );
    files
}

/// Every `.rs` file under `root`, paired with its text, sorted by path. Skips the rule files.
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
            collect_rust_sources(&path, files);
        } else if path.extension().is_some_and(|ext| ext == "rs")
            && !RULE_FILES.contains(&relative(&path).as_str())
        {
            let text = std::fs::read_to_string(&path)
                .unwrap_or_else(|e| panic!("cannot read {}: {e}", path.display()));
            files.push((path, text));
        }
    }
}

/// A path relative to the repository root, the way the rules report it.
fn relative(path: &Path) -> String {
    path.strip_prefix(repository_root())
        .unwrap_or(path)
        .display()
        .to_string()
}

/// The API package a source file belongs to: the application, or the API crate whose folder
/// holds it.
fn package_of(path: &Path) -> String {
    let relative = relative(path);
    match relative.strip_prefix("crates/api/") {
        Some(rest) => rest.split('/').next().unwrap_or_default().to_string(),
        None => APPLICATION.to_string(),
    }
}

/// The sorted names of the entries of `dir`.
fn entries_of(dir: &Path) -> Vec<String> {
    let mut names: Vec<String> = std::fs::read_dir(dir)
        .unwrap_or_else(|e| panic!("cannot read {}: {e}", dir.display()))
        .map(|entry| {
            entry
                .expect("directory entry")
                .file_name()
                .to_string_lossy()
                .into_owned()
        })
        .collect();
    names.sort();
    names
}

/// Fail unless the walk saw enough files to make the rule meaningful.
fn assert_walk_is_not_vacuous(count: usize, floor: usize, scope: &str) {
    assert!(
        count >= floor,
        "the {scope} walk returned only {count} file(s), fewer than the floor of {floor} — \
         the walker has lost the tree and every rule over it would pass vacuously"
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

/// Whether the code line names the path `prefix` (`api_missions::handlers`, say) where it does not
/// continue a longer identifier, so `xapi_missions::handlers` does not read as the crate.
fn names_path(line: &str, prefix: &str) -> bool {
    line.match_indices(prefix).any(|(at, _)| {
        !line[..at]
            .chars()
            .next_back()
            .is_some_and(|c| c.is_ascii_alphanumeric() || c == '_')
    })
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

/* ══════════ prose and test placement ══════════ */

/// Unit tests live in sibling files under `tests/`, declared with `#[cfg(test)] #[path = …] mod`.
/// An inline `mod tests { … }` body buries the tests inside the production file and pushes it over
/// the 500-line ceiling from the inside.
#[test]
fn no_inline_test_modules() {
    let files = every_api_source();
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
/// these crates does not have, and it outlives the work it named.
#[test]
fn no_ticket_references_in_source() {
    let files = every_api_source();
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

/// The crates are not described in terms of a system they are not. Prose that compares this code
/// to another implementation dates on the day that implementation stops existing, and sends the
/// reader looking for files this repository does not contain.
#[test]
fn no_other_implementation_narrative() {
    let files = every_api_source();
    const PHRASES: [&str; 5] = ["Rust port", "Go API", "internal/", "GORM", "parsePage"];
    let found = offenders(&files, true, |_, line| {
        if let Some(phrase) = PHRASES.iter().find(|phrase| line.contains(**phrase)) {
            return Some(format!("`{phrase}` — describe this crate, not another one"));
        }
        names_a_go_file(line).then(|| "names a `.go` file — this crate has none".to_string())
    });
    assert_no_offenders("no_other_implementation_narrative", &found);
}

/* ══════════ the crate graph ══════════ */

/// The kernel crates (every API crate that is neither a domain nor the workers) are the floor
/// every domain stands on, so they depend on no domain: otherwise the two halves are mutually
/// recursive and neither can be read, moved or tested without the other. The application state
/// takes its services injected; the application's composition root builds them.
#[test]
fn kernel_crates_depend_on_no_domain() {
    let found = graph_offenders(
        |package| {
            package != APPLICATION && package != WORKERS && domain_of_crate(package).is_none()
        },
        |_| true,
    );
    assert_no_offenders("kernel_crates_depend_on_no_domain", &found);
}

/// The domain crates depend on one another only along the one-way domain graph
/// ([`DOMAIN_DEPENDENCIES`]), read from their manifests' normal and dev dependency tables. An edge
/// outside it, or one towards the workers or the application, is refused.
#[test]
fn domain_crates_depend_only_along_the_domain_graph() {
    let mut domain_edges = 0;
    for domain in DOMAINS {
        let (_, edges) = api_edges(&format!("api_{domain}"));
        domain_edges += edges
            .iter()
            .filter(|(_, target)| domain_of_crate(target).is_some_and(|t| t != domain))
            .count();
    }
    assert!(
        domain_edges >= DOMAIN_EDGE_FLOOR,
        "the domain manifests hold only {domain_edges} domain edge(s), fewer than the floor of \
         {DOMAIN_EDGE_FLOOR} — the manifest reader has lost the graph"
    );
    let found = graph_offenders(|package| domain_of_crate(package).is_some(), |_| true);
    assert_no_offenders("domain_crates_depend_only_along_the_domain_graph", &found);
}

/// The interval tasks are armed once, by the process that owns a lifetime long enough to run them.
/// A crate that reaches for the workers is either spawning a second copy of a worker per request or
/// calling a worker's body outside the schedule that makes it safe; either way the work belongs in
/// a service both the worker and the caller can share. So only the application's manifest names
/// the workers crate, and in the application's source only the `api` binary does.
#[test]
fn background_workers_used_only_by_the_binary() {
    let mut found = graph_offenders(
        |package| package != APPLICATION && package != WORKERS,
        |target| target == WORKERS,
    );
    let files = every_api_source();
    found.extend(offenders(&files, false, |path, line| {
        let armed_here = relative(path) == WORKERS_ARMED_BY;
        let names_workers = names_path(line, &format!("{WORKERS}::"));
        (names_workers && !armed_here && package_of(path) != WORKERS)
            .then(|| format!("names `{WORKERS}` — only the binary arms them ({WORKERS_ARMED_BY})"))
    }));
    assert_no_offenders("background_workers_used_only_by_the_binary", &found);
}

/// A handler is one domain's HTTP surface: its extractors, its status codes, its wire shapes.
/// Calling another domain's handler borrows all of that along with the logic, and couples two
/// route tables that should only ever meet in the router. Cross-domain reuse goes through the
/// owning domain's `models` (a shape) or `services` (a behaviour), both of which are free of HTTP,
/// so no source outside a domain crate names its `handlers`.
#[test]
fn no_crate_imports_a_foreign_domains_handlers() {
    let files = every_api_source();
    let found = offenders(&files, false, |path, line| {
        let own = package_of(path);
        let foreign = DOMAINS.iter().find(|domain| {
            own != format!("api_{domain}") && names_path(line, &format!("api_{domain}::handlers"))
        })?;
        Some(format!(
            "`{own}` reaches into the handlers of `api_{foreign}` — cross-domain access goes \
             through `api_{foreign}::models` or `api_{foreign}::services`"
        ))
    });
    assert_no_offenders("no_crate_imports_a_foreign_domains_handlers", &found);
}

/* ══════════ the shape of the application ══════════ */

/// Every domain crate owns exactly one route table, and the router merges all eight. A domain
/// whose table is not merged compiles, passes its own tests, and answers 404 on the wire — so
/// both halves are pinned: the crate's `routes.rs` defines the one `pub fn routes(` of the crate,
/// and `src/router.rs` merges it as `api_<domain>::routes(`.
#[test]
fn every_domain_crate_exports_one_route_table_the_router_merges() {
    let router_path = application_root().join("src/router.rs");
    let router = std::fs::read_to_string(&router_path)
        .unwrap_or_else(|e| panic!("cannot read {}: {e}", router_path.display()));

    let mut found = Vec::new();
    for domain in DOMAINS {
        let source = repository_root().join(format!("crates/api/api_{domain}/src"));
        let tables: Vec<String> = rust_sources(&source)
            .iter()
            .flat_map(|(path, text)| {
                text.lines()
                    .filter(|line| line.trim_start().starts_with("pub fn routes("))
                    .map(|_| relative(path))
                    .collect::<Vec<_>>()
            })
            .collect();
        let table = format!("crates/api/api_{domain}/src/routes.rs");
        if tables != [table.clone()] {
            found.push(format!(
                "{table}:1 — the crate defines `pub fn routes(` in {tables:?}; a domain owns \
                 exactly one route table, in its `routes.rs`"
            ));
        }
        let merged = format!("api_{domain}::routes(");
        if !router.contains(&merged) {
            found.push(format!(
                "apps/api/src/router.rs:1 — does not merge `{merged}`; the domain would answer 404"
            ));
        }
    }
    assert_no_offenders(
        "every_domain_crate_exports_one_route_table_the_router_merges",
        &found,
    );
}

/// The application is thin: its `src/` holds the library root, the router, the composition root,
/// the two binaries and the rule and router tests, and nothing else. A domain folder, a `core/`, a
/// top-level `handlers/` or a second home for the state would put code here that belongs in an API
/// crate, where the crate graph judges it.
#[test]
fn the_application_source_holds_only_the_thin_app() {
    let source = application_root().join("src");
    let mut found = Vec::new();
    for (folder, expected) in [
        ("", &APPLICATION_ENTRIES[..]),
        ("bin", &BINARY_ENTRIES[..]),
        ("tests", &APPLICATION_TEST_ENTRIES[..]),
    ] {
        let present = entries_of(&source.join(folder));
        let shown = if folder.is_empty() {
            "apps/api/src".to_string()
        } else {
            format!("apps/api/src/{folder}")
        };
        for name in present
            .iter()
            .filter(|name| !expected.contains(&name.as_str()))
        {
            found.push(format!(
                "{shown}/{name}:1 — not part of the thin application; it belongs \
                 in an API crate under crates/api/"
            ));
        }
        for name in expected
            .iter()
            .filter(|name| !present.iter().any(|p| p == *name))
        {
            found.push(format!("{shown}/{name}:1 — missing"));
        }
    }
    assert_no_offenders("the_application_source_holds_only_the_thin_app", &found);
}

/* ══════════ the judgements themselves ══════════ */

/// The verdicts the crate graph hands out: the application above everything, the workers above
/// the domains, the domains along their graph, the kernel below every domain; an edge into the
/// application, a foreign edge into the workers, an off-graph domain edge and a kernel edge into a
/// domain fail.
#[test]
fn the_crate_graph_refuses_upward_off_graph_and_kernel_to_domain_edges() {
    assert_eq!(crate_edge_violation("api", "api_operations"), None);
    assert_eq!(crate_edge_violation("api", WORKERS), None);
    assert_eq!(crate_edge_violation(WORKERS, "api_operations"), None);
    assert_eq!(crate_edge_violation(WORKERS, "api_state"), None);
    assert_eq!(crate_edge_violation("api_operations", "api_missions"), None);
    assert_eq!(crate_edge_violation("api_operations", "api_state"), None);
    assert_eq!(crate_edge_violation("api_state", "api_foundation"), None);
    assert!(
        crate_edge_violation("api_missions", "api_operations")
            .is_some_and(|reason| reason.contains("domain graph"))
    );
    assert!(crate_edge_violation("api_community_content", "api_identity_and_access").is_some());
    assert!(
        crate_edge_violation("api_state", "api_missions")
            .is_some_and(|reason| reason.contains("kernel crate"))
    );
    assert!(crate_edge_violation("api_operations", WORKERS).is_some());
    assert!(crate_edge_violation(WORKERS, "api").is_some());
}

/// The domain graph names each domain once, names only domains, and has no cycle: a cycle would
/// be a pair of crates Cargo cannot build, and a missing domain would leave its edges unjudged.
#[test]
fn the_domain_graph_is_acyclic_over_the_eight_domains() {
    let mut listed: Vec<&str> = DOMAIN_DEPENDENCIES
        .iter()
        .map(|(domain, _)| *domain)
        .collect();
    listed.sort_unstable();
    assert_eq!(listed, DOMAINS, "every domain is listed exactly once");
    // Kahn's order: a graph is acyclic when every domain can be placed after its dependencies.
    let mut placed: Vec<&str> = Vec::new();
    while placed.len() < DOMAINS.len() {
        let next = DOMAIN_DEPENDENCIES.iter().find(|(domain, dependencies)| {
            !placed.contains(domain)
                && dependencies.iter().all(|dependency| {
                    assert!(
                        DOMAINS.contains(dependency),
                        "`{dependency}` is not a domain"
                    );
                    placed.contains(dependency)
                })
        });
        let Some((domain, _)) = next else {
            panic!("the domain graph has a cycle among the domains not in {placed:?}");
        };
        placed.push(domain);
    }
}

/// The path reader matches a crate path on its own, never as the tail of a longer identifier.
#[test]
fn the_path_reader_matches_whole_crate_names() {
    assert!(names_path(
        "use api_missions::handlers::x;",
        "api_missions::handlers"
    ));
    assert!(names_path(
        "f(api_missions::handlers::x)",
        "api_missions::handlers"
    ));
    assert!(!names_path(
        "use xapi_missions::handlers::x;",
        "api_missions::handlers"
    ));
    assert!(!names_path(
        "use api_missions::services::x;",
        "api_missions::handlers"
    ));
}
