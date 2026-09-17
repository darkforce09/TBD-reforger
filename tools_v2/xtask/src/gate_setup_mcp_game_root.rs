//! T-876 — port of `scripts/mod/setup-mcp-game-root.sh` → `cargo xtask setup mcp-game-root`.
//!
//! Builds a flattened pak symlink farm so enfusion-mcp's VFS (which only scans
//! `<gamePath>/addons/*.pak` directly) can see nested `addons/data/` + `addons/core/` paks.
//!
//! Acceptance is bash/port stdout+stderr+rc (+ symlink names/targets) on a clean throwaway
//! tree and ≥2 broken arms — not a green run alone (T-556 / T-853).
//!
//! Preserved oddities:
//! - Flatten naming is bash `${rel//\//_}` (every `/` → `_`), including preserving `.PAK` case
//!   from `-iname "*.pak"`.
//! - Default GAME is the hardcoded Steam path from the former script (not `$HOME`-relative).
//! - Default FAKE is `$HOME/.cache/enfusion-mcp-root` (falls back to `/home/Samuel` if HOME unset,
//!   matching common shell behaviour when HOME is empty — we use the same hard-coded user home
//!   only when `HOME` is unset via `std::env::var` Err; empty HOME still joins `.cache/...`).
//! - Success line is exactly `Linked N pak files into <fake>/addons/` (trailing slash on addons).

use std::fs;
use std::os::unix::fs::symlink;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};

/// Hardcoded default from former `setup-mcp-game-root.sh` line 11.
const DEFAULT_GAME: &str = "/home/Samuel/.local/share/Steam/steamapps/common/Arma Reforger";

/// Entry for `xtask setup mcp-game-root [GAME] [FAKE]`.
pub fn run(game: Option<&Path>, fake: Option<&Path>) -> Result<u8> {
    let game = game
        .map(Path::to_path_buf)
        .unwrap_or_else(|| PathBuf::from(DEFAULT_GAME));
    let fake = fake.map(Path::to_path_buf).unwrap_or_else(default_fake);
    run_with_paths(&game, &fake)
}

fn default_fake() -> PathBuf {
    let home = std::env::var("HOME").unwrap_or_else(|_| "/home/Samuel".into());
    PathBuf::from(home).join(".cache/enfusion-mcp-root")
}

/// Testable entry with explicit GAME + FAKE roots (throwaways under `/tmp`).
pub fn run_with_paths(game: &Path, fake: &Path) -> Result<u8> {
    let addons = game.join("addons");
    // bash: `if [ ! -d "$GAME/addons" ]; then echo … >&2; exit 1; fi`
    if !addons.is_dir() {
        eprintln!("Game addons dir not found: {}", addons.display());
        return Ok(1);
    }

    // bash: `rm -rf "$FAKE"` then `mkdir -p "$FAKE/addons/data"`
    if fake.exists() {
        fs::remove_dir_all(fake)
            .or_else(|_| {
                // FAKE may be a plain file (bash `rm -rf` removes files too).
                fs::remove_file(fake)
            })
            .with_context(|| format!("rm -rf {}", fake.display()))?;
    }
    let fake_addons = fake.join("addons");
    let fake_data = fake_addons.join("data");
    fs::create_dir_all(&fake_data).with_context(|| format!("mkdir -p {}", fake_data.display()))?;

    let paks = collect_paks(&addons)?;
    let mut count: u64 = 0;
    for p in &paks {
        let rel = p
            .strip_prefix(&addons)
            .with_context(|| format!("strip prefix {} from {}", addons.display(), p.display()))?;
        let flat = flatten_rel(rel);
        let link = fake_addons.join(&flat);
        symlink(p, &link).with_context(|| format!("ln -s {} {}", p.display(), link.display()))?;
        count += 1;
    }

    println!("Linked {count} pak files into {}/", fake_addons.display());
    Ok(0)
}

/// bash `${rel//\//_}` — replace every path separator with `_`.
fn flatten_rel(rel: &Path) -> String {
    // Path::display uses platform separators; we built from Unix find paths with `/`.
    let s = rel.to_string_lossy();
    s.replace('/', "_")
}

/// `find "$GAME/addons" -iname "*.pak" -print0 | sort -z` — case-insensitive extension, sorted.
fn collect_paks(addons: &Path) -> Result<Vec<PathBuf>> {
    let mut out = Vec::new();
    collect_paks_rec(addons, &mut out)?;
    out.sort();
    Ok(out)
}

fn collect_paks_rec(dir: &Path, out: &mut Vec<PathBuf>) -> Result<()> {
    let entries = fs::read_dir(dir).with_context(|| format!("read_dir {}", dir.display()))?;
    for entry in entries {
        let entry = entry.with_context(|| format!("read_dir entry under {}", dir.display()))?;
        let path = entry.path();
        let ty = entry
            .file_type()
            .with_context(|| format!("file_type {}", path.display()))?;
        if ty.is_dir() {
            // GNU find does not follow dir symlinks by default — same here.
            collect_paks_rec(&path, out)?;
        } else if (ty.is_file() || ty.is_symlink()) && is_pak_iname(&path) {
            // bash `find -iname` without `-type f` matches regular files and file symlinks.
            out.push(path);
        }
    }
    Ok(())
}

/// bash `-iname "*.pak"` against the basename.
fn is_pak_iname(path: &Path) -> bool {
    path.extension()
        .and_then(|e| e.to_str())
        .is_some_and(|e| e.eq_ignore_ascii_case("pak"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::os::unix::fs::PermissionsExt;

    #[test]
    fn flatten_replaces_every_slash() {
        assert_eq!(
            flatten_rel(Path::new("data/sub/Nested.PAK")),
            "data_sub_Nested.PAK"
        );
        assert_eq!(flatten_rel(Path::new("core/Engine.pak")), "core_Engine.pak");
    }

    #[test]
    fn is_pak_case_insensitive() {
        assert!(is_pak_iname(Path::new("x.pak")));
        assert!(is_pak_iname(Path::new("x.PAK")));
        assert!(is_pak_iname(Path::new("x.Pak")));
        assert!(!is_pak_iname(Path::new("x.txt")));
    }

    #[test]
    fn clean_tree_links_flattened_names() {
        let base = tempfile_dir("clean");
        let game = base.join("game");
        let fake = base.join("fake");
        seed_game(&game, true);
        let code = run_with_paths(&game, &fake).unwrap();
        assert_eq!(code, 0);
        assert!(fake.join("addons/data").is_dir());
        assert_eq!(
            fs::read_link(fake.join("addons/core_Engine.pak")).unwrap(),
            game.join("addons/core/Engine.pak")
        );
        assert_eq!(
            fs::read_link(fake.join("addons/data_Base.pak")).unwrap(),
            game.join("addons/data/Base.pak")
        );
        assert_eq!(
            fs::read_link(fake.join("addons/data_sub_Nested.PAK")).unwrap(),
            game.join("addons/data/sub/Nested.PAK")
        );
        // non-pak ignored
        assert!(!fake.join("addons/data_readme.txt").exists());
    }

    #[test]
    fn missing_addons_dir_exits_1() {
        let base = tempfile_dir("no-addons");
        let game = base.join("game");
        let fake = base.join("fake");
        fs::create_dir_all(&game).unwrap();
        let code = run_with_paths(&game, &fake).unwrap();
        assert_eq!(code, 1);
        assert!(!fake.exists(), "must not create FAKE on broken arm");
    }

    #[test]
    fn addons_as_file_exits_1() {
        let base = tempfile_dir("addons-file");
        let game = base.join("game");
        let fake = base.join("fake");
        fs::create_dir_all(&game).unwrap();
        fs::write(game.join("addons"), b"not a dir\n").unwrap();
        let code = run_with_paths(&game, &fake).unwrap();
        assert_eq!(code, 1);
    }

    #[test]
    fn empty_addons_links_zero() {
        let base = tempfile_dir("empty");
        let game = base.join("game");
        let fake = base.join("fake");
        fs::create_dir_all(game.join("addons")).unwrap();
        let code = run_with_paths(&game, &fake).unwrap();
        assert_eq!(code, 0);
        assert!(fake.join("addons/data").is_dir());
    }

    fn seed_game(game: &Path, with_paks: bool) {
        fs::create_dir_all(game.join("addons/data/sub")).unwrap();
        fs::create_dir_all(game.join("addons/core")).unwrap();
        if with_paks {
            fs::write(game.join("addons/data/Base.pak"), b"pak1\n").unwrap();
            fs::write(game.join("addons/core/Engine.pak"), b"pak2\n").unwrap();
            fs::write(game.join("addons/data/sub/Nested.PAK"), b"pak3\n").unwrap();
            fs::write(game.join("addons/data/readme.txt"), b"ignore\n").unwrap();
        }
        // make sure modes are ordinary files
        let _ = fs::set_permissions(
            game.join("addons/data/Base.pak"),
            fs::Permissions::from_mode(0o644),
        );
    }

    fn tempfile_dir(tag: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!(
            "t876-{tag}-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        dir
    }
}
