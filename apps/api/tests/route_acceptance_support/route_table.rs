//! The API's route table, read from the router source at test runtime.
//!
//! **Role:** turns `src/router.rs` and every route table it reaches into rows of
//! (method, full path, handler, development-only flag, route body limit), following every
//! `.merge(…)`, `.nest(prefix, …)`, `.nest_service(prefix, …)` and development-gated `if` block.
//!
//! **Position:** test support; the coverage binary compares the rows with the `@route` tags
//! ([`super::route_tags`]) and with the specs, and the dimension runner reads a row's body
//! limit and path parameters.
//!
//! **Signals & state:** the parsed table of the crate's own source is cached once per process
//! in a `OnceLock`; parsing any other source tree is uncached.
//!
//! **Invariants:** parsing starts at `fn router` and reaches only what it registers, so a table
//! nobody merges contributes no row; a registration shape the parser cannot read, an `if` whose
//! condition is not the development flag, or a route-level layer other than `DefaultBodyLimit`
//! is an error, never a skipped row; a row is development-only exactly when an enclosing `if`
//! tests the development flag.

use std::path::{Path, PathBuf};
use std::sync::OnceLock;

use super::rust_source_scanning::{
    Call, call_chain, file_scope_fn_body, find_outside_literals, identifiers, keyword_at,
    matching_close, module_directory, parent_dir_of_module, read_stripped_source, rust_files_under,
    split_top_level, string_literal_value,
};

/// The HTTP method routers axum builds a route from.
const METHOD_ROUTERS: [&str; 8] = [
    "get", "post", "put", "patch", "delete", "head", "options", "trace",
];
/// The registration calls whose presence makes an `if` block route-bearing.
const REGISTRATION_TOKENS: [&str; 4] = [".route(", ".merge(", ".nest(", ".nest_service("];

/// What serves a row.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum Handler {
    /// A named handler function; `module` is the module's file relative to `src/`, and a tag
    /// binds to it when it sits in that file or under the module's directory.
    Function { name: String, module: PathBuf },
    /// An inline closure (`/healthz`, `/metrics`); it carries no `@route` tag.
    Closure,
    /// A `ServeDir` mount; the row's path ends in the `{*path}` wildcard.
    StaticFiles,
}

/// One registered route.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct RouteRow {
    /// The upper-case method, e.g. `GET`.
    pub method: String,
    /// The full path in axum spelling, e.g. `/api/v1/missions/{id}`.
    pub path: String,
    /// What serves it.
    pub handler: Handler,
    /// True when an enclosing `if` gates it on the development flag.
    pub dev_only: bool,
    /// The argument of a route-level `DefaultBodyLimit::max(…)`, or `None` for the global limit.
    pub body_limit: Option<String>,
    /// The file that registers it, relative to `src/`.
    pub source: PathBuf,
}

impl RouteRow {
    /// `METHOD /path`, the key a [`super::spec::RouteSpec`] names.
    pub(crate) fn key(&self) -> String {
        format!("{} {}", self.method, self.path)
    }

    /// The handler function's name, when a named function serves the row.
    pub(crate) fn handler_fn(&self) -> Option<&str> {
        match &self.handler {
            Handler::Function { name, .. } => Some(name),
            _ => None,
        }
    }
}

/// The crate's `src/` directory.
pub(crate) fn crate_source_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("src")
}

/// The parsed route table of this crate, parsed once per process.
pub(crate) fn route_table() -> &'static [RouteRow] {
    static TABLE: OnceLock<Vec<RouteRow>> = OnceLock::new();
    TABLE.get_or_init(|| {
        parse_route_table(&crate_source_root())
            .unwrap_or_else(|error| panic!("the route table cannot be read from source: {error}"))
    })
}

/// The row registered under `key` (`METHOD /path`).
pub(crate) fn row(key: &str) -> Option<&'static RouteRow> {
    route_table().iter().find(|row| row.key() == key)
}

/// Parse the route table rooted at `src_root/router.rs`'s `fn router`.
pub(crate) fn parse_route_table(src_root: &Path) -> Result<Vec<RouteRow>, String> {
    let parser = Parser { src_root };
    let file = PathBuf::from("router.rs");
    let source = parser.read(&file)?;
    let body = file_scope_fn_body(&source, "router")?
        .ok_or_else(|| "router.rs has no `fn router`".to_string())?;
    let mut rows = Vec::new();
    let scope = Scope {
        file,
        prefix: String::new(),
        dev_only: false,
    };
    parser.walk(body, &scope, &mut rows)?;
    if rows.is_empty() {
        return Err("the router registers no route".into());
    }
    Ok(rows)
}

#[derive(Clone)]
struct Scope {
    file: PathBuf,
    prefix: String,
    dev_only: bool,
}

struct Parser<'a> {
    src_root: &'a Path,
}

impl Parser<'_> {
    fn read(&self, file: &Path) -> Result<String, String> {
        read_stripped_source(&self.src_root.join(file))
    }

    /// Walk a router expression or function body, registering every route it reaches.
    fn walk(&self, text: &str, scope: &Scope, rows: &mut Vec<RouteRow>) -> Result<(), String> {
        let mut at = 0;
        while let Some(next) = next_event(text, at) {
            let (start, token) = next;
            if token == "if" {
                at = self.walk_if(text, start, scope, rows)?;
                continue;
            }
            let open = start + token.len() - 1;
            let close = matching_close(text, open)
                .ok_or_else(|| format!("{}: unclosed `{token}`", scope.file.display()))?;
            let args = &text[open + 1..close];
            match token {
                ".route(" => self.register(args, scope, rows)?,
                ".merge(" => self.walk_router_expression(args, scope, rows)?,
                ".nest(" => {
                    let [path, router] = two_arguments(args, scope)?;
                    let nested = Scope {
                        prefix: format!("{}{}", scope.prefix, self.path_value(path, scope)?),
                        ..scope.clone()
                    };
                    self.walk_router_expression(router, &nested, rows)?;
                }
                ".nest_service(" => {
                    let [path, _service] = two_arguments(args, scope)?;
                    rows.push(RouteRow {
                        method: "GET".into(),
                        path: format!(
                            "{}{}/{{*path}}",
                            scope.prefix,
                            self.path_value(path, scope)?
                        ),
                        handler: Handler::StaticFiles,
                        dev_only: scope.dev_only,
                        body_limit: None,
                        source: scope.file.clone(),
                    });
                }
                other => return Err(format!("{}: unsupported `{other}`", scope.file.display())),
            }
            at = close + 1;
        }
        Ok(())
    }

    /// An `if` block: walked as development-only when its condition is the development flag,
    /// skipped when it registers nothing, refused otherwise. Returns the index past the block
    /// (and past an `else` block that registers nothing).
    fn walk_if(
        &self,
        text: &str,
        start: usize,
        scope: &Scope,
        rows: &mut Vec<RouteRow>,
    ) -> Result<usize, String> {
        let open = find_outside_literals(text, start, "{")
            .ok_or_else(|| format!("{}: `if` without a block", scope.file.display()))?;
        let close = matching_close(text, open)
            .ok_or_else(|| format!("{}: unclosed `if` block", scope.file.display()))?;
        let condition = text[start + 2..open].trim();
        let block = &text[open + 1..close];
        if registers_routes(block) {
            if !is_development_condition(condition) {
                return Err(format!(
                    "{}: routes registered under `if {condition}`, which is not the development flag",
                    scope.file.display()
                ));
            }
            let gated = Scope {
                dev_only: true,
                ..scope.clone()
            };
            self.walk(block, &gated, rows)?;
        }
        let rest = text[close + 1..].trim_start();
        if rest.starts_with("else") {
            let else_at = text.len() - rest.len();
            let else_open = find_outside_literals(text, else_at, "{")
                .ok_or_else(|| format!("{}: `else` without a block", scope.file.display()))?;
            let else_close = matching_close(text, else_open)
                .ok_or_else(|| format!("{}: unclosed `else` block", scope.file.display()))?;
            if registers_routes(&text[else_open + 1..else_close]) {
                return Err(format!(
                    "{}: routes registered in an `else` block",
                    scope.file.display()
                ));
            }
            return Ok(else_close + 1);
        }
        Ok(close + 1)
    }

    /// A merged or nested router: an inline `Router::new()…` chain or a call to a function
    /// returning a router, resolved to its file and walked there.
    fn walk_router_expression(
        &self,
        expression: &str,
        scope: &Scope,
        rows: &mut Vec<RouteRow>,
    ) -> Result<(), String> {
        let expression = expression.trim();
        if expression.starts_with("Router::new()") {
            return self.walk(expression, scope, rows);
        }
        let calls = call_chain(expression).map_err(|e| format!("{}: {e}", scope.file.display()))?;
        let [callee] = calls.as_slice() else {
            return Err(format!(
                "{}: a merged router must be one call, found `{expression}`",
                scope.file.display()
            ));
        };
        let (file, body) = self.resolve_fn(&callee.path, &scope.file)?;
        let target = Scope {
            file,
            ..scope.clone()
        };
        self.walk(&body, &target, rows)
    }

    /// One `.route(path, method_router)` registration.
    fn register(&self, args: &str, scope: &Scope, rows: &mut Vec<RouteRow>) -> Result<(), String> {
        let [path, method_router] = two_arguments(args, scope)?;
        let path = string_literal_value(path).ok_or_else(|| {
            format!(
                "{}: route path `{path}` is not a literal",
                scope.file.display()
            )
        })?;
        let calls = call_chain(method_router)
            .map_err(|error| format!("{}: {error}", scope.file.display()))?;
        let mut body_limit = None;
        let mut registered = Vec::new();
        for call in &calls {
            if call.name() == "layer" {
                body_limit = Some(body_limit_argument(call, scope)?);
            } else if METHOD_ROUTERS.contains(&call.name()) {
                registered.push((call.name().to_uppercase(), self.handler(call, scope)?));
            } else {
                return Err(format!(
                    "{}: unsupported method router call `{}` on {path}",
                    scope.file.display(),
                    call.path
                ));
            }
        }
        if registered.is_empty() {
            return Err(format!(
                "{}: {path} registers no method",
                scope.file.display()
            ));
        }
        for (method, handler) in registered {
            rows.push(RouteRow {
                method,
                path: format!("{}{path}", scope.prefix),
                handler,
                dev_only: scope.dev_only,
                body_limit: body_limit.clone(),
                source: scope.file.clone(),
            });
        }
        Ok(())
    }

    fn handler(&self, call: &Call, scope: &Scope) -> Result<Handler, String> {
        let expression = call.args.trim();
        if expression.starts_with("move") || expression.starts_with('|') {
            return Ok(Handler::Closure);
        }
        let segments: Vec<&str> = expression.split("::").map(str::trim).collect();
        let (name, module_path) = segments
            .split_last()
            .filter(|(name, _)| identifiers(name).len() == 1)
            .ok_or_else(|| {
                format!(
                    "{}: unreadable handler `{expression}`",
                    scope.file.display()
                )
            })?;
        let module = if module_path.is_empty() {
            scope.file.clone()
        } else {
            self.resolve_module(module_path, &scope.file)?
        };
        Ok(Handler::Function {
            name: (*name).to_string(),
            module,
        })
    }

    /// A path argument: a literal, or a `&str` constant found in the module the path names.
    fn path_value(&self, expression: &str, scope: &Scope) -> Result<String, String> {
        if let Some(value) = string_literal_value(expression) {
            return Ok(value);
        }
        let segments: Vec<&str> = expression.split("::").map(str::trim).collect();
        let (name, module_path) = segments.split_last().expect("split yields one piece");
        let module = if module_path.is_empty() {
            scope.file.clone()
        } else {
            self.resolve_module(module_path, &scope.file)?
        };
        let needle = format!("const {name}: &str = ");
        let mut candidates = std::collections::BTreeSet::from([self.src_root.join(&module)]);
        if let Some(dir) = module_directory(&module) {
            candidates.extend(rust_files_under(&self.src_root.join(dir)));
        }
        let mut values = Vec::new();
        for file in candidates {
            let text = read_stripped_source(&file)?;
            if let Some(at) = find_outside_literals(&text, 0, &needle) {
                let rest = &text[at + needle.len()..];
                let end = rest.find(';').unwrap_or(rest.len());
                values.push(string_literal_value(&rest[..end]).ok_or_else(|| {
                    format!("{}: `{name}` is not a string literal", file.display())
                })?);
            }
        }
        match values.as_slice() {
            [only] => Ok(only.clone()),
            _ => Err(format!(
                "{}: path constant `{expression}` resolves to {} definitions",
                scope.file.display(),
                values.len()
            )),
        }
    }

    /// The file and body of the function `path` names, following a `pub use m::name;`
    /// re-export once per hop.
    fn resolve_fn(&self, path: &str, from: &Path) -> Result<(PathBuf, String), String> {
        let segments: Vec<&str> = path.split("::").map(str::trim).collect();
        let (name, module_path) = segments.split_last().expect("split yields one piece");
        let module = if module_path.is_empty() {
            from.to_path_buf()
        } else {
            self.resolve_module(module_path, from)?
        };
        let text = self.read(&module)?;
        if let Some(body) = file_scope_fn_body(&text, name)? {
            return Ok((module, body.to_string()));
        }
        let re_export = "pub use ";
        let mut at = 0;
        while let Some(found) = find_outside_literals(&text, at, re_export) {
            let end = text[found..].find(';').map_or(text.len(), |e| found + e);
            let used = text[found + re_export.len()..end].trim();
            if let Some(inner) = used.strip_suffix(&format!("::{name}")) {
                return self.resolve_fn(&format!("{inner}::{name}"), &module);
            }
            at = end;
        }
        Err(format!("{}: no `fn {name}` for `{path}`", module.display()))
    }

    /// The file of the module `segments` names, as seen from `from` (relative to `src/`).
    /// `crate::` starts at `src/`, `super::` at the parent module; a bare first segment is a
    /// child module of `from`, through a `use super::…` import a sibling of it, an API crate
    /// named directly (`api_<name>::…`, its root `src/lib.rs` when named alone) or, through a
    /// `use <crate>::…::<segment>;` import of an API crate, a module of that crate; an API
    /// crate's file is answered as an absolute path.
    fn resolve_module(&self, segments: &[&str], from: &Path) -> Result<PathBuf, String> {
        let own_dir = module_directory(from).unwrap_or_default();
        let parent_dir = from.parent().map(Path::to_path_buf).unwrap_or_default();
        let (bases, rest): (Vec<PathBuf>, &[&str]) = match segments.first() {
            Some(&"crate") => (vec![PathBuf::new()], &segments[1..]),
            Some(&"super") => (vec![parent_dir_of_module(from)], &segments[1..]),
            Some(&"self") => (vec![own_dir], &segments[1..]),
            _ => (vec![own_dir, parent_dir], segments),
        };
        for base in bases {
            if let Some(file) = module_file(self.src_root, base, rest) {
                return Ok(file);
            }
        }
        if let Some(file) = named_api_crate_module(segments)? {
            return Ok(file);
        }
        if let Some(file) = self.imported_api_crate_module(segments, from)? {
            return Ok(file);
        }
        Err(format!(
            "{}: module `{}` resolves to no file",
            from.display(),
            segments.join("::")
        ))
    }

    /// The absolute file of the module `segments` names when `from` imports its first segment
    /// from an API crate with a plain `use <crate>::…::<first>;`, or `None` when no such import
    /// reaches a module file of a crate under `crates/api/`.
    fn imported_api_crate_module(
        &self,
        segments: &[&str],
        from: &Path,
    ) -> Result<Option<PathBuf>, String> {
        let Some((first, inner)) = segments.split_first() else {
            return Ok(None);
        };
        let text = self.read(from)?;
        let api_crates = api_crates_root()?;
        let mut at = 0;
        while let Some(found) = find_outside_literals(&text, at, "use ") {
            let end = text[found..].find(';').map_or(text.len(), |e| found + e);
            at = end;
            let starts_a_word = text[..found]
                .chars()
                .next_back()
                .is_none_or(|c| !(c.is_ascii_alphanumeric() || c == '_'));
            let imported: Vec<&str> = text[found + "use ".len()..end]
                .split("::")
                .map(str::trim)
                .collect();
            let Some((krate, path)) = imported.split_first() else {
                continue;
            };
            if !starts_a_word || path.last() != Some(first) {
                continue;
            }
            let crate_source = api_crates.join(krate).join("src");
            let module_path: Vec<&str> = path.iter().chain(inner).copied().collect();
            if let Some(file) = module_file(&crate_source, PathBuf::new(), &module_path) {
                return Ok(Some(crate_source.join(file)));
            }
        }
        Ok(None)
    }
}

/// The file, relative to `root`, of the module `rest` names below the folder `base`: each segment
/// a `<segment>.rs` or a `<segment>/mod.rs`; `None` when a segment names no file.
fn module_file(root: &Path, base: PathBuf, rest: &[&str]) -> Option<PathBuf> {
    let mut dir = base;
    let mut file = None;
    for segment in rest {
        if root.join(&dir).join(format!("{segment}.rs")).is_file() {
            file = Some(dir.join(format!("{segment}.rs")));
        } else if root.join(&dir).join(segment).join("mod.rs").is_file() {
            file = Some(dir.join(segment).join("mod.rs"));
        } else {
            return None;
        }
        dir = dir.join(segment);
    }
    file
}

/// The absolute file of the module `segments` names when its first segment is an API crate
/// (a folder of `crates/api/` holding `src/lib.rs`): the crate root for the crate alone, else the
/// module file below its `src/`; `None` when the first segment names no API crate.
fn named_api_crate_module(segments: &[&str]) -> Result<Option<PathBuf>, String> {
    let Some((krate, inner)) = segments.split_first() else {
        return Ok(None);
    };
    let crate_source = api_crates_root()?.join(krate).join("src");
    if !crate_source.join("lib.rs").is_file() {
        return Ok(None);
    }
    if inner.is_empty() {
        return Ok(Some(crate_source.join("lib.rs")));
    }
    Ok(module_file(&crate_source, PathBuf::new(), inner).map(|file| crate_source.join(file)))
}

/// The `src/` folder of every API crate (each folder of `crates/api/` holding `src/lib.rs`),
/// sorted; an unreadable `crates/api/` is an error, never an empty list.
pub(crate) fn api_crate_source_roots() -> Result<Vec<PathBuf>, String> {
    let root = api_crates_root()?;
    let entries = std::fs::read_dir(&root).map_err(|e| format!("read {}: {e}", root.display()))?;
    let mut roots: Vec<PathBuf> = entries
        .filter_map(|entry| entry.ok().map(|entry| entry.path().join("src")))
        .filter(|source| source.join("lib.rs").is_file())
        .collect();
    roots.sort();
    if roots.is_empty() {
        return Err(format!("no API crate under {}", root.display()));
    }
    Ok(roots)
}

/// The folder of the API crates (`crates/api`), found above this package's manifest folder.
fn api_crates_root() -> Result<PathBuf, String> {
    let manifest = Path::new(env!("CARGO_MANIFEST_DIR"));
    repository_root::find_repository_root_from(manifest)
        .map(|root| root.join("crates/api"))
        .map_err(|error| format!("no repository root above {}: {error}", manifest.display()))
}

/// The next registration call or `if` keyword in `text` at or after `from`.
fn next_event(text: &str, from: usize) -> Option<(usize, &'static str)> {
    let mut best: Option<(usize, &'static str)> = None;
    for token in REGISTRATION_TOKENS {
        if let Some(at) = find_outside_literals(text, from, token) {
            best = best.filter(|(b, _)| *b < at).or(Some((at, token)));
        }
    }
    let mut at = from;
    while let Some(found) = find_outside_literals(text, at, "if") {
        if best.is_some_and(|(b, _)| b < found) {
            break;
        }
        if keyword_at(text, found, "if") {
            best = Some((found, "if"));
            break;
        }
        at = found + 2;
    }
    best
}

fn registers_routes(block: &str) -> bool {
    REGISTRATION_TOKENS
        .iter()
        .any(|token| find_outside_literals(block, 0, token).is_some())
}

/// The development flag: a condition naming `dev`, `development` or `is_development` and
/// not negated.
fn is_development_condition(condition: &str) -> bool {
    !condition.starts_with('!')
        && identifiers(condition)
            .iter()
            .any(|word| matches!(*word, "dev" | "development" | "is_development"))
}

fn two_arguments<'a>(args: &'a str, scope: &Scope) -> Result<[&'a str; 2], String> {
    match split_top_level(args, b',').as_slice() {
        [first, second] => Ok([first, second]),
        other => Err(format!(
            "{}: expected two arguments, found {} in `{args}`",
            scope.file.display(),
            other.len()
        )),
    }
}

/// The limit expression of a `DefaultBodyLimit::max(..)` layer, without the trailing comma a
/// formatter leaves on an argument it wraps onto its own line.
fn body_limit_argument(call: &Call, scope: &Scope) -> Result<String, String> {
    let calls = call_chain(&call.args).map_err(|e| format!("{}: {e}", scope.file.display()))?;
    match calls.as_slice() {
        [limit] if limit.path.ends_with("DefaultBodyLimit::max") => {
            Ok(limit.args.trim().trim_end_matches(',').trim_end().into())
        }
        _ => Err(format!(
            "{}: unsupported route layer `{}`",
            scope.file.display(),
            call.args.trim()
        )),
    }
}
