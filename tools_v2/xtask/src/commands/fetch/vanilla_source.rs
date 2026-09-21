//! `cargo xtask fetch vanilla-source` — mirror vanilla Enfusion method bodies.
//!
//! Pages come from arexplorer.zeroy.com and are cached under
//! `apps/mod/vanilla_reference/source_html/`; a cached page that is non-empty is never
//! refetched. The default set is a curated spine of the classes the framework builds on;
//! `--all`, `--grep <pattern>` and explicit class names widen it.
//!
//! Three behaviours worth knowing before reading the code:
//! - `--help` is treated as a class name, and reports `MISS` like any other unknown name: clap's
//!   help is disabled on this subcommand so that a class actually called `help` stays fetchable.
//! - `--grep` with a missing or empty pattern prints the usage line and exits 2.
//! - An index with no matching links exits 1 after truncating `map.tsv`: an empty map is the
//!   one state that would make every later lookup silently miss.
//! - An HTTP miss counts as `miss` and leaves the exit code at 0.
//!
//! Curl runs through [`verification_core::proc::Run`]. Cache hits, an unknown name, `--grep`
//! usage and the empty-index refusal all work offline; a page that is absent is fetched with the
//! same recipe.

use std::fs::{self, File, OpenOptions};
use std::io::{self, BufRead, BufReader, Write};
use std::path::Path;
use std::thread;
use std::time::Duration;

use anyhow::{Context, Result, bail};
use regex::Regex;
use verification_core::proc::{self, Run};

const BASE: &str = "https://arexplorer.zeroy.com";
const UA: &str = "Mozilla/5.0 (X11; Linux x86_64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0 Safari/537.36";

/// The command named in the `--grep` usage line, so the refusal tells the operator exactly what
/// to retype.
const USAGE_COMMAND: &str = "cargo xtask fetch vanilla-source";

const CURATED: &[&str] = &[
    "SCR_BaseGameMode.c",
    "SCR_BaseGameModeComponent.c",
    "SCR_RespawnSystemComponent.c",
    "SCR_RespawnComponent.c",
    "SCR_SpawnPoint.c",
    "SCR_SpawnerRespawnComponent.c",
    "SCR_PossessSpawnPointComponent.c",
    "SCR_SpawnHandlerComponent.c",
    "SCR_SpawnRequestComponent.c",
    "SCR_PlayerController.c",
    "SCR_PlayerControllerGroupComponent.c",
    "ChimeraMenuBase.c",
    "SCR_MenuHelper.c",
    "SCR_FactionManager.c",
    "SCR_Faction.c",
    "SCR_GroupsManagerComponent.c",
    "SCR_AIGroup.c",
    "SCR_GameModeHealthSettings.c",
    "SCR_CharacterDamageManagerComponent.c",
];

/// Entry for `xtask fetch vanilla-source` — `args` are the tokens after the subcommand
/// (hyphen values allowed so `--help` / `--all` / `--grep` reach us, not clap).
pub fn run(repo_root: &Path, args: &[String]) -> Result<u8> {
    if let Some(code) = early_usage(args) {
        return Ok(code);
    }

    let cache = repo_root.join("apps/mod/vanilla_reference/source_html");
    fs::create_dir_all(&cache).with_context(|| format!("mkdir -p {}", cache.display()))?;

    let index = cache.join("files.html");
    ensure_index(&index)?;

    let map_path = cache.join("map.tsv");
    if let Err(code) = ensure_map(&index, &map_path) {
        return Ok(code);
    }

    let targets = resolve_targets(args, &map_path)?;
    // Line-flushed, so a `>out 2>&1` capture interleaves stdout and stderr in real order.
    out_line(&format!("==> {} source page(s)", targets.len()))?;

    let delay = fetch_delay();
    let mut got: u32 = 0;
    let mut miss: u32 = 0;

    for name in &targets {
        if name.is_empty() {
            continue;
        }
        let page = lookup_page(&map_path, name)?;
        let Some(page) = page else {
            err_line(&format!("  MISS {name} (not in index)"))?;
            miss += 1;
            continue;
        };
        let dest = cache.join(&page);
        if file_nonempty(&dest) {
            got += 1;
            continue;
        }
        let code = curl_fetch(&format!("{BASE}/{page}"), &dest)?;
        if code == "200" {
            got += 1;
            out_line(&format!("  got  {name}"))?;
        } else {
            let _ = fs::remove_file(&dest);
            miss += 1;
            err_line(&format!("  FAIL {name} (http {code})"))?;
        }
        thread::sleep(delay);
    }

    out_line(&format!(
        "cached {got} page(s), {miss} missing -> {}",
        cache.display()
    ))?;
    out_line("next:  cargo run -q -p developer-tools --bin enf -- source")?;
    Ok(0)
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

/// `--grep` with a missing or empty pattern: usage on stderr, exit 2.
fn early_usage(args: &[String]) -> Option<u8> {
    if args.first().map(String::as_str) == Some("--grep") {
        let pat = args.get(1).map(String::as_str).unwrap_or("");
        if pat.is_empty() {
            let _ = err_line(&format!("usage: {USAGE_COMMAND} --grep <pattern>"));
            return Some(2);
        }
    }
    None
}

fn fetch_delay() -> Duration {
    let secs: f64 = std::env::var("TBD_FETCH_DELAY")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(0.4);
    Duration::from_secs_f64(secs.max(0.0))
}

fn file_nonempty(path: &Path) -> bool {
    fs::metadata(path).map(|m| m.len() > 0).unwrap_or(false)
}

fn ensure_index(index: &Path) -> Result<()> {
    if file_nonempty(index) {
        return Ok(());
    }
    out_line("==> file index")?;
    // The index is not optional: a failed fetch stops the command.
    curl_download(&format!("{BASE}/files.html"), index)?;
    let bytes = fs::metadata(index).map(|m| m.len()).unwrap_or(0);
    out_line(&format!("  got files.html ({bytes} bytes)"))?;
    Ok(())
}

/// Build `map.tsv` when missing/empty. Zero href matches → truncate map + exit 1 (grep pipefail).
fn ensure_map(index: &Path, map_path: &Path) -> Result<(), u8> {
    if file_nonempty(map_path) {
        return Ok(());
    }
    let body = fs::read_to_string(index).map_err(|_| 1u8)?;
    let re = Regex::new(r#"href="([a-z0-9_]*)_8c\.html" target="_self">([^<]*\.c)<"#)
        .expect("index href regex");
    let mut rows: Vec<(String, String)> = re
        .captures_iter(&body)
        .map(|c| {
            let stem = c.get(1).unwrap().as_str();
            let name = c.get(2).unwrap().as_str();
            (name.to_string(), format!("{stem}_8c_source.html"))
        })
        .collect();
    // An index with no matching links yields no map. Truncate it rather than leaving a stale
    // one, and refuse: an empty map makes every later lookup miss silently.
    if rows.is_empty() {
        let _ = File::create(map_path);
        return Err(1);
    }
    rows.sort();
    rows.dedup();
    let mut out = File::create(map_path).map_err(|_| 1u8)?;
    for (name, page) in &rows {
        writeln!(out, "{name}\t{page}").map_err(|_| 1u8)?;
    }
    let n = rows.len();
    // `wc -l < "$MAP"` — no leading pad on this platform when stdin-redirected.
    let _ = out_line(&format!("  mapped {n} source pages"));
    Ok(())
}

fn resolve_targets(args: &[String], map_path: &Path) -> Result<Vec<String>> {
    match args.first().map(String::as_str) {
        Some("--all") => map_names(map_path),
        Some("--grep") => {
            let pat = args.get(1).expect("early_usage guards missing pattern");
            let names = map_names(map_path)?;
            let pat_l = pat.to_lowercase();
            Ok(names
                .into_iter()
                .filter(|n| n.to_lowercase().contains(&pat_l))
                .collect())
        }
        None => Ok(CURATED.iter().map(|s| (*s).to_string()).collect()),
        Some(_) => Ok(args.to_vec()),
    }
}

fn map_names(map_path: &Path) -> Result<Vec<String>> {
    let f = File::open(map_path).with_context(|| format!("open {}", map_path.display()))?;
    let mut names = Vec::new();
    for line in BufReader::new(f).lines() {
        let line = line?;
        if line.is_empty() {
            continue;
        }
        let name = line.split('\t').next().unwrap_or("");
        if !name.is_empty() {
            names.push(name.to_string());
        }
    }
    Ok(names)
}

fn lookup_page(map_path: &Path, name: &str) -> Result<Option<String>> {
    let f = File::open(map_path).with_context(|| format!("open {}", map_path.display()))?;
    for line in BufReader::new(f).lines() {
        let line = line?;
        let mut parts = line.splitn(2, '\t');
        let n = parts.next().unwrap_or("");
        if n == name {
            return Ok(parts.next().map(str::to_string));
        }
    }
    Ok(None)
}

/// Fetch one document to `dest`. A curl failure here stops the command.
fn curl_download(url: &str, dest: &Path) -> Result<()> {
    let curl = proc::which("curl").map_err(|_| anyhow::anyhow!("curl: command not found"))?;
    let out = Run::new(curl)
        .args([
            "-sSL",
            "-A",
            UA,
            "-o",
            dest.to_str().unwrap_or("/dev/null"),
            url,
        ])
        .status()
        .map_err(|e| anyhow::anyhow!("curl: {e:?}"))?;
    if out != 0 {
        bail!("curl exited {out} fetching {url}");
    }
    Ok(())
}

/// Page loop: `code=$(curl -sSL -A UA -o dest -w '%{http_code}' url || echo 000)`.
fn curl_fetch(url: &str, dest: &Path) -> Result<String> {
    if let Some(parent) = dest.parent() {
        fs::create_dir_all(parent)?;
    }
    let _ = OpenOptions::new()
        .write(true)
        .create(true)
        .truncate(true)
        .open(dest);

    match proc::which("curl") {
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
                    // curl failing as a process is reported as `000`, distinct from any HTTP
                    // status it would otherwise have printed.
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

#[cfg(test)]
#[path = "tests/vanilla_source/tests.rs"]
mod tests;
