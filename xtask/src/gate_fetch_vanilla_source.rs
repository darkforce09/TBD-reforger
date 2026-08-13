//! T-862 — port of `scripts/mod/fetch-vanilla-source.sh` → `cargo xtask fetch vanilla-source`.
//!
//! Mirrors vanilla Enfusion SOURCE (method bodies) from arexplorer.zeroy.com. Default is the
//! curated T-181 spine; `--all` / `--grep` / explicit names match bash. Pages are cached under
//! `apps/mod/vanilla_reference/source_html/` and never refetched when non-empty.
//!
//! Preserved oddities (byte-for-byte with bash):
//! - `--help` is a *filename* target (MISS), not usage — clap help is disabled on this subcommand.
//! - `--grep` with missing or empty pattern prints historical `$0` usage and exits 2.
//! - Building `map.tsv` from an index with zero href matches exits 1 (grep pipefail), after
//!   truncating the map file empty.
//! - HTTP misses count as `miss` and do not change the process exit code (still 0).
//!
//! Curl is invoked via [`tbd_gate::proc::Run`] (same flags as bash). Offline arms that never
//! need the network (cache hits, MISS-not-in-index, `--grep` usage, empty-index map build)
//! are the acceptance surface; live fetch is the same curl recipe when a page is absent.

use std::fs::{self, File, OpenOptions};
use std::io::{self, BufRead, BufReader, Write};
use std::path::Path;
use std::thread;
use std::time::Duration;

use anyhow::{Context, Result, bail};
use regex::Regex;
use tbd_gate::proc::{self, Run};

const BASE: &str = "https://arexplorer.zeroy.com";
const UA: &str = "Mozilla/5.0 (X11; Linux x86_64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0 Safari/537.36";

/// Historical `$0` in the bash `--grep` usage line (`bash scripts/mod/fetch-vanilla-source.sh`).
const USAGE_SELF: &str = "scripts/mod/fetch-vanilla-source.sh";

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
    // Line-flush so `>out 2>&1` interleaves stdout/stderr like bash `echo` (T-853 byte diff).
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
    out_line("next:  cargo run -q -p tbd-tools --bin enf -- source")?;
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

/// `--grep` with missing or empty pattern → usage on stderr, exit 2 (bash `[ -n "${2:-}" ]`).
fn early_usage(args: &[String]) -> Option<u8> {
    if args.first().map(String::as_str) == Some("--grep") {
        let pat = args.get(1).map(String::as_str).unwrap_or("");
        if pat.is_empty() {
            let _ = err_line(&format!("usage: {USAGE_SELF} --grep <pattern>"));
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
    // bash: `curl -sSL -A UA -o INDEX URL` with set -e (no `|| echo 000`, no http_code).
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
    // bash: grep exits 1 on no match → pipefail kills the script before `echo mapped`.
    if rows.is_empty() {
        // `> "$MAP"` truncates even when the pipeline fails.
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

/// Index fetch: `curl -sSL -A UA -o dest url` — curl failure is hard (bash `set -e`).
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
                    // Non-zero curl still may print an http_code; bash `|| echo 000` only on
                    // failure of the curl command itself.
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
mod tests {
    use super::*;
    use std::path::PathBuf;
    use std::process::Command;

    fn scratch(name: &str) -> PathBuf {
        let p = std::env::temp_dir().join(format!(
            "t862-{name}-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let _ = fs::remove_dir_all(&p);
        fs::create_dir_all(p.join("apps/mod/vanilla_reference/source_html")).unwrap();
        fs::create_dir_all(p.join(".ai/tickets")).unwrap();
        fs::write(p.join(".ai/tickets/ROOT"), "{}").unwrap();
        p
    }

    #[test]
    fn grep_missing_pattern_exits_2() {
        let root = scratch("grep-miss");
        let code = run(&root, &["--grep".into()]).unwrap();
        assert_eq!(code, 2);
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn grep_empty_pattern_exits_2() {
        let root = scratch("grep-empty");
        let code = run(&root, &["--grep".into(), "".into()]).unwrap();
        assert_eq!(code, 2);
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn empty_index_map_build_exits_1() {
        let root = scratch("empty-idx");
        let cache = root.join("apps/mod/vanilla_reference/source_html");
        fs::write(cache.join("files.html"), "no hrefs here\n").unwrap();
        // no map.tsv
        let code = run(&root, &["NoSuch.c".into()]).unwrap();
        assert_eq!(code, 1);
        // bash truncates map on failed pipeline
        assert!(cache.join("map.tsv").exists());
        assert_eq!(fs::metadata(cache.join("map.tsv")).unwrap().len(), 0);
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn help_is_filename_miss_rc0() {
        let root = scratch("help-miss");
        let cache = root.join("apps/mod/vanilla_reference/source_html");
        // minimal valid map so we don't hit empty-index
        fs::write(cache.join("files.html"), "x").unwrap();
        fs::write(
            cache.join("map.tsv"),
            "SCR_BaseGameMode.c\t_s_c_r___base_game_mode_8c_source.html\n",
        )
        .unwrap();
        let code = run(&root, &["--help".into()]).unwrap();
        assert_eq!(code, 0);
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn curated_list_len_matches_bash() {
        assert_eq!(CURATED.len(), 19);
    }

    /// Anti-vacuity: bash side must go red on the empty-index fixture before we trust parity.
    #[test]
    fn bash_empty_index_goes_red_first() {
        let sh = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../scripts/mod/fetch-vanilla-source.sh");
        // Script may already be deleted after land — skip if absent (post-delete unit run).
        if !sh.exists() {
            return;
        }
        let root = scratch("bash-red");
        let cache = root.join("apps/mod/vanilla_reference/source_html");
        fs::create_dir_all(root.join("scripts/mod")).unwrap();
        fs::copy(&sh, root.join("scripts/mod/fetch-vanilla-source.sh")).unwrap();
        fs::write(cache.join("files.html"), "no hrefs here\n").unwrap();
        let out = Command::new("bash")
            .arg(root.join("scripts/mod/fetch-vanilla-source.sh"))
            .arg("NoSuch.c")
            .output()
            .expect("bash");
        assert_ne!(
            out.status.code(),
            Some(0),
            "bash must go red on empty index"
        );
        assert_eq!(out.status.code(), Some(1));
        let _ = fs::remove_dir_all(root);
    }
}
