//! T-866 — port of `scripts/mod/fetch-vanilla-api.sh` → `cargo xtask fetch vanilla-api`.
//!
//! Mirrors Bohemia's official Arma Reforger Script API (Doxygen HTML) into
//! `apps/mod/vanilla_reference/apidoc/`. Index-only by default; optional class names or
//! `--from-file`. Pages are cached and never refetched when non-empty.
//!
//! Preserved oddities (byte-for-byte with bash):
//! - Browser UA required (wiki 403s curl's default).
//! - Class-page HTTP misses print `MISS` and are ignored (`|| true`); index miss exits 1.
//! - `--from-file` with a missing path prints usage and exits 2; a nonexistent file path
//!   prints grep's error and continues with an empty class list (rc 0).
//! - Doxygen mangling: `_` → `__` in filenames (`SCR_BaseGameMode` →
//!   `interfaceSCR__BaseGameMode.html`).
//!
//! Curl via [`tbd_gate::proc::Run`]. Offline arms prefer fixture/cache hits; live network
//! uses the same curl recipe when a page is absent (`TBD_FETCH_DELAY`, default 0.3 s).
//!
//! `TBD_FETCH_VANILLA_API_CURL` — optional absolute path to a curl binary, checked before
//! `proc::which("curl")`. Production leaves it unset. Tests use it instead of mutating `PATH`
//! (which races under `cargo test --test-threads=N`).

use std::fs::{self, OpenOptions};
use std::io::{self, BufRead, BufReader, Write};
use std::path::{Path, PathBuf};
use std::thread;
use std::time::Duration;

use anyhow::{Context, Result};
use tbd_gate::proc::{self, Run};

const BASE: &str = "https://community.bistudio.com/wikidata/external-data/arma-reforger/ArmaReforgerScriptAPIPublic";
const UA: &str = "Mozilla/5.0 (X11; Linux x86_64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0 Safari/537.36";

/// Historical `$0` when invoked as `bash scripts/mod/fetch-vanilla-api.sh`.
const USAGE_SELF: &str = "scripts/mod/fetch-vanilla-api.sh";

/// Entry for `xtask fetch vanilla-api` — `args` are tokens after the subcommand.
pub fn run(repo_root: &Path, args: &[String]) -> Result<u8> {
    let cache = repo_root.join("apps/mod/vanilla_reference/apidoc");
    fs::create_dir_all(&cache).with_context(|| format!("mkdir -p {}", cache.display()))?;

    out_line("==> class index")?;
    if !fetch(&cache, "annotated.html")? {
        err_line("fetch-vanilla-api: cannot reach the API docs")?;
        return Ok(1);
    }

    let classes = match resolve_classes(args)? {
        Classes::Usage => {
            err_line(&format!("usage: {USAGE_SELF} --from-file <path>"))?;
            return Ok(2);
        }
        Classes::List(c) => c,
    };

    if !classes.is_empty() {
        out_line(&format!("==> {} class page(s)", classes.len()))?;
        for c in &classes {
            // Class misses are ignored (`fetch … || true`). Sleep lives inside `fetch`
            // after a successful network get only (cache hits return early).
            let _ = fetch(&cache, &doxy_name(c))?;
        }
    }

    let n = count_html_pages(&cache)?;
    out_line(&format!("cache: {} ({} pages)", cache.display(), n))?;
    out_line("next:  cargo run -q -p tbd-tools --bin enf -- apidoc")?;
    Ok(0)
}

enum Classes {
    Usage,
    List(Vec<String>),
}

fn resolve_classes(args: &[String]) -> Result<Classes> {
    if args.first().map(String::as_str) == Some("--from-file") {
        let path = args.get(1).map(String::as_str).unwrap_or("");
        if path.is_empty() {
            return Ok(Classes::Usage);
        }
        return Ok(Classes::List(read_from_file(Path::new(path))?));
    }
    Ok(Classes::List(args.to_vec()))
}

/// `grep -v '^\s*#' "$2" | sed '/^\s*$/d'` — missing file → grep stderr + empty list (rc 0).
fn read_from_file(path: &Path) -> Result<Vec<String>> {
    if !path.is_file() {
        // GNU grep wording.
        err_line(&format!(
            "grep: {}: No such file or directory",
            path.display()
        ))?;
        return Ok(Vec::new());
    }
    let f = FileOpen::open(path)?;
    let mut out = Vec::new();
    for line in BufReader::new(f).lines() {
        let line = line?;
        if is_comment_line(&line) {
            continue;
        }
        if is_blank_line(&line) {
            continue;
        }
        out.push(line);
    }
    Ok(out)
}

/// GNU grep `-v '^\s*#'` — optional leading whitespace then `#`.
fn is_comment_line(line: &str) -> bool {
    let rest = line.trim_start_matches(|c: char| c.is_whitespace());
    rest.starts_with('#')
}

/// sed `/^\s*$/d`
fn is_blank_line(line: &str) -> bool {
    line.chars().all(|c| c.is_whitespace())
}

/// Doxygen: `SCR_BaseGameMode` → `interfaceSCR__BaseGameMode.html`.
fn doxy_name(class: &str) -> String {
    let mangled = class.replace('_', "__");
    format!("interface{mangled}.html")
}

/// Returns true on cache hit or HTTP 200; false on miss (caller may ignore for class pages).
fn fetch(cache: &Path, remote_name: &str) -> Result<bool> {
    let dest = cache.join(remote_name);
    if file_nonempty(&dest) {
        return Ok(true);
    }
    let url = format!("{BASE}/{remote_name}");
    let code = curl_fetch(&url, &dest)?;
    if code != "200" {
        let _ = fs::remove_file(&dest);
        err_line(&format!("  MISS {remote_name} (http {code})"))?;
        return Ok(false);
    }
    let bytes = fs::metadata(&dest).map(|m| m.len()).unwrap_or(0);
    out_line(&format!("  got  {remote_name} ({bytes} bytes)"))?;
    thread::sleep(fetch_delay());
    Ok(true)
}

fn fetch_delay() -> Duration {
    let secs: f64 = std::env::var("TBD_FETCH_DELAY")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(0.3);
    Duration::from_secs_f64(secs.max(0.0))
}

fn file_nonempty(path: &Path) -> bool {
    fs::metadata(path).map(|m| m.len() > 0).unwrap_or(false)
}

fn count_html_pages(cache: &Path) -> Result<usize> {
    // `find "$CACHE" -name '*.html' | wc -l` — recursive, unsorted (count only).
    let mut n = 0usize;
    fn walk(dir: &Path, n: &mut usize) -> Result<()> {
        let entries = match fs::read_dir(dir) {
            Ok(e) => e,
            Err(_) => return Ok(()),
        };
        for ent in entries {
            let ent = ent?;
            let path = ent.path();
            if path.is_dir() {
                walk(&path, n)?;
            } else if path
                .file_name()
                .and_then(|s| s.to_str())
                .is_some_and(|s| s.ends_with(".html"))
            {
                *n += 1;
            }
        }
        Ok(())
    }
    walk(cache, &mut n)?;
    Ok(n)
}

fn out_line(s: &str) -> Result<()> {
    let mut out = io::stdout().lock();
    writeln!(out, "{s}")?;
    out.flush()?;
    Ok(())
}

fn err_line(s: &str) -> Result<()> {
    let mut err = io::stderr().lock();
    writeln!(err, "{s}")?;
    err.flush()?;
    Ok(())
}

/// `code=$(curl -sSL -A UA -o dest -w '%{http_code}' url || echo 000)`.
/// Resolve curl: `TBD_FETCH_VANILLA_API_CURL` override, else `PATH` via [`proc::which`].
fn resolve_curl() -> Result<PathBuf, tbd_gate::NotRun> {
    if let Ok(override_path) = std::env::var("TBD_FETCH_VANILLA_API_CURL") {
        let trimmed = override_path.trim();
        if !trimmed.is_empty() {
            return Ok(PathBuf::from(trimmed));
        }
    }
    proc::which("curl")
}

fn curl_fetch(url: &str, dest: &Path) -> Result<String> {
    if let Some(parent) = dest.parent() {
        fs::create_dir_all(parent)?;
    }
    let _ = OpenOptions::new()
        .write(true)
        .create(true)
        .truncate(true)
        .open(dest);

    match resolve_curl() {
        Err(_) => {
            let _ = fs::remove_file(dest);
            Ok("000".to_string())
        }
        Ok(curl) => {
            let merged = Run::new(curl)
                .args([
                    "-sSL",
                    "-A",
                    UA,
                    "-o",
                    dest.to_str().unwrap_or("/dev/null"),
                    "-w",
                    "%{http_code}",
                    url,
                ])
                .merged_output();
            match merged {
                Ok(out) => {
                    if out.code != 0 {
                        let _ = fs::remove_file(dest);
                        return Ok("000".to_string());
                    }
                    let code = out.text.trim().to_string();
                    if code.is_empty() {
                        Ok("000".to_string())
                    } else {
                        Ok(code)
                    }
                }
                Err(_) => {
                    let _ = fs::remove_file(dest);
                    Ok("000".to_string())
                }
            }
        }
    }
}

/// Thin wrapper so `read_from_file` can name the open helper without clashing `fs::File`.
struct FileOpen;
impl FileOpen {
    fn open(path: &Path) -> Result<fs::File> {
        fs::File::open(path).with_context(|| format!("open {}", path.display()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;
    use std::process::Command;

    fn scratch(name: &str) -> PathBuf {
        let p = std::env::temp_dir().join(format!(
            "t866-{name}-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let _ = fs::remove_dir_all(&p);
        fs::create_dir_all(p.join("apps/mod/vanilla_reference/apidoc")).unwrap();
        fs::create_dir_all(p.join(".ai/tickets")).unwrap();
        fs::write(p.join(".ai/tickets/registry.json"), "{}").unwrap();
        p
    }

    fn seed_index(root: &Path) {
        fs::write(
            root.join("apps/mod/vanilla_reference/apidoc/annotated.html"),
            "<html>index</html>\n",
        )
        .unwrap();
    }

    #[test]
    fn doxy_name_mangles_underscores() {
        assert_eq!(
            doxy_name("SCR_BaseGameMode"),
            "interfaceSCR__BaseGameMode.html"
        );
        assert_eq!(doxy_name("NoSuchClass"), "interfaceNoSuchClass.html");
    }

    #[test]
    fn from_file_missing_arg_exits_2() {
        let root = scratch("fromfile-miss");
        seed_index(&root);
        let code = run(&root, &["--from-file".into()]).unwrap();
        assert_eq!(code, 2);
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn index_miss_exits_1() {
        let root = scratch("index-miss");
        // Empty cache + stub curl via TBD_FETCH_VANILLA_API_CURL (not PATH — PATH races
        // under cargo test parallel threads and let real curl win → false green rc=0).
        let bin = root.join("bin");
        fs::create_dir_all(&bin).unwrap();
        let stub = bin.join("curl");
        fs::write(&stub, "#!/bin/bash\nexit 1\n").unwrap();
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mut perms = fs::metadata(&stub).unwrap().permissions();
            perms.set_mode(0o755);
            fs::set_permissions(&stub, perms).unwrap();
        }
        let old = std::env::var_os("TBD_FETCH_VANILLA_API_CURL");
        // SAFETY: restored below; only this test sets the override.
        unsafe { std::env::set_var("TBD_FETCH_VANILLA_API_CURL", &stub) };
        let code = run(&root, &[]).unwrap();
        match old {
            Some(v) => unsafe { std::env::set_var("TBD_FETCH_VANILLA_API_CURL", v) },
            None => unsafe { std::env::remove_var("TBD_FETCH_VANILLA_API_CURL") },
        }
        assert_eq!(code, 1);
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn from_file_nonexistent_continues_rc0() {
        let root = scratch("fromfile-nofile");
        seed_index(&root);
        let code = run(
            &root,
            &[
                "--from-file".into(),
                "/tmp/t866-does-not-exist-xyz.txt".into(),
            ],
        )
        .unwrap();
        assert_eq!(code, 0);
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn cache_hit_index_only_rc0() {
        let root = scratch("cache-hit");
        seed_index(&root);
        let code = run(&root, &[]).unwrap();
        assert_eq!(code, 0);
        let _ = fs::remove_dir_all(root);
    }

    /// Anti-vacuity: bash must go red on index miss before we trust parity.
    #[test]
    fn bash_index_miss_goes_red_first() {
        let sh =
            PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../scripts/mod/fetch-vanilla-api.sh");
        if !sh.exists() {
            return;
        }
        let root = scratch("bash-red");
        fs::create_dir_all(root.join("scripts/mod")).unwrap();
        fs::copy(&sh, root.join("scripts/mod/fetch-vanilla-api.sh")).unwrap();
        let bin = root.join("bin");
        fs::create_dir_all(&bin).unwrap();
        let stub = bin.join("curl");
        fs::write(&stub, "#!/bin/bash\nexit 1\n").unwrap();
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mut perms = fs::metadata(&stub).unwrap().permissions();
            perms.set_mode(0o755);
            fs::set_permissions(&stub, perms).unwrap();
        }
        let path = format!("{}:/usr/bin:/bin", bin.display());
        let out = Command::new("bash")
            .arg("scripts/mod/fetch-vanilla-api.sh")
            .current_dir(&root)
            .env("PATH", &path)
            .output()
            .expect("bash");
        assert_ne!(out.status.code(), Some(0), "bash must go red on index miss");
        assert_eq!(out.status.code(), Some(1));
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn bash_from_file_usage_goes_red_first() {
        let sh =
            PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../scripts/mod/fetch-vanilla-api.sh");
        if !sh.exists() {
            return;
        }
        let root = scratch("bash-usage");
        seed_index(&root);
        fs::create_dir_all(root.join("scripts/mod")).unwrap();
        fs::copy(&sh, root.join("scripts/mod/fetch-vanilla-api.sh")).unwrap();
        let out = Command::new("bash")
            .arg("scripts/mod/fetch-vanilla-api.sh")
            .arg("--from-file")
            .current_dir(&root)
            .output()
            .expect("bash");
        assert_eq!(out.status.code(), Some(2));
        let _ = fs::remove_dir_all(root);
    }
}
