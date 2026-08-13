//! T-864 — port of `scripts/mod/test-mission.sh` → `cargo xtask mod test-mission`.
//!
//! Switch the mission the Workbench client loads via profile
//! `$HOME/.../ArmaReforgerWorkbench/profile/TBD_BackendConfig.json` (+ optional golden stage).
//!
//! All four `python3` sites from the bash are in-process `serde_json` (show / set missionId /
//! golden `meta.id` extract). That removes the script from `scripts/python-inventory.txt`.
//!
//! Fail-opens closed vs bash:
//! - none that lied about having run — missing config / missing golden still hard-fail.
//!
//! Preserved oddities:
//! - Registry copy keeps bash's `cp … 2>/dev/null || true` (optional; silent on absence).
//! - `json.dump(..., indent=2)` shape: pretty JSON, **no** trailing newline after `}`.
//! - Golden lookup is `find … -name '<arg>.json' | head -1`; we walk + sort and take the first
//!   match (unique basenames under `packages/tbd-schema` today — sorted first == find first).

use std::collections::BTreeMap;
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use serde_json::Value;
use tbd_gate::scan;

use crate::root::find_repo_root;

/// Backend dev mission restored by `backend` (bash pin — not read from a .bak).
const BACKEND_MISSION: &str = "6d291619-8182-4164-866d-4e165a5516af";

const CFG_REL: &str = ".local/share/Steam/steamapps/compatdata/1874910/pfx/drive_c/users/steamuser/Documents/My Games/ArmaReforgerWorkbench/profile";

/// Entry for `xtask mod test-mission [TARGET]`.
pub fn run(target: Option<&str>) -> Result<u8> {
    let root = find_repo_root()?;
    run_with_root(&root, target)
}

/// Testable entry that does not walk for the repo root. Honours `$HOME` for the profile tree.
pub fn run_with_root(root: &Path, target: Option<&str>) -> Result<u8> {
    let home = match std::env::var_os("HOME") {
        Some(h) => PathBuf::from(h),
        None => {
            eprintln!("HOME is unset");
            return Ok(1);
        }
    };
    let prof = home.join(CFG_REL);
    let cfg = prof.join("TBD_BackendConfig.json");

    if !cfg.is_file() {
        eprintln!("no config at {} — has the mod ever run?", cfg.display());
        return Ok(1);
    }

    match target {
        None | Some("") => {
            println!("current:");
            show(&cfg, &prof)?;
            Ok(0)
        }
        Some("backend") => {
            set_mission_id(&cfg, BACKEND_MISSION)?;
            println!("switched to the backend mission:");
            show(&cfg, &prof)?;
            Ok(0)
        }
        Some(name) => stage_golden(root, &prof, &cfg, name),
    }
}

fn stage_golden(root: &Path, prof: &Path, cfg: &Path, name: &str) -> Result<u8> {
    let schema = root.join("packages/tbd-schema");
    let want = format!("{name}.json");
    let golden = match find_golden(&schema, &want) {
        Ok(Some(p)) => p,
        Ok(None) => {
            eprintln!("no golden named '{name}' under packages/tbd-schema");
            return Ok(1);
        }
        Err(e) => {
            // Missing schema tree: bash `find` prints to stderr and still yields empty → same
            // operator-facing message as "not found".
            eprintln!("no golden named '{name}' under packages/tbd-schema");
            let _ = e;
            return Ok(1);
        }
    };

    let mid = mission_id_from_golden(&golden)?;
    let missions = prof.join("missions");
    fs::create_dir_all(&missions).with_context(|| format!("mkdir -p {}", missions.display()))?;
    let dest = missions.join(format!("{mid}.json"));
    fs::copy(&golden, &dest)
        .with_context(|| format!("cp {} -> {}", golden.display(), dest.display()))?;

    // bash: `cp "$ROOT/apps/mod/tbd-framework/Data/registry.json" "$PROF/TBD_Registry.json" 2>/dev/null || true`
    let _ = fs::copy(
        root.join("apps/mod/tbd-framework/Data/registry.json"),
        prof.join("TBD_Registry.json"),
    );

    set_mission_id(cfg, &mid)?;
    println!("staged {name} and switched:");
    show(cfg, prof)?;
    Ok(0)
}

/// `find "$ROOT/packages/tbd-schema" -name "$1.json" | head -1` — sorted walk, first match.
fn find_golden(schema: &Path, want_name: &str) -> Result<Option<PathBuf>, tbd_gate::NotRun> {
    let files = scan::walk_files(&[schema], |p| {
        p.file_name()
            .and_then(|n| n.to_str())
            .is_some_and(|n| n == want_name)
    })?;
    Ok(files.into_iter().next())
}

fn mission_id_from_golden(path: &Path) -> Result<String> {
    let text = fs::read_to_string(path).with_context(|| format!("read {}", path.display()))?;
    let v: Value =
        serde_json::from_str(&text).with_context(|| format!("parse {}", path.display()))?;
    let mid = v
        .pointer("/meta/id")
        .and_then(|x| x.as_str())
        .with_context(|| format!("meta.id missing in {}", path.display()))?;
    Ok(mid.to_string())
}

fn set_mission_id(cfg: &Path, mission_id: &str) -> Result<()> {
    let text = fs::read_to_string(cfg).with_context(|| format!("read {}", cfg.display()))?;
    let mut v: Value =
        serde_json::from_str(&text).with_context(|| format!("parse {}", cfg.display()))?;
    let obj = v
        .as_object_mut()
        .with_context(|| format!("config root is not an object: {}", cfg.display()))?;
    obj.insert(
        "missionId".to_string(),
        Value::String(mission_id.to_string()),
    );
    // Python `json.dump(..., indent=2)` — pretty, no trailing newline.
    let body = serde_json::to_string_pretty(&v).context("serialize config")?;
    let mut f = fs::File::create(cfg).with_context(|| format!("open {}", cfg.display()))?;
    f.write_all(body.as_bytes())
        .with_context(|| format!("write {}", cfg.display()))?;
    Ok(())
}

fn show(cfg: &Path, prof: &Path) -> Result<()> {
    let text = fs::read_to_string(cfg).with_context(|| format!("read {}", cfg.display()))?;
    let v: Value =
        serde_json::from_str(&text).with_context(|| format!("parse {}", cfg.display()))?;
    let mid = v
        .get("missionId")
        .and_then(|x| x.as_str())
        .with_context(|| format!("missionId missing in {}", cfg.display()))?;

    let local = prof.join("missions").join(format!("{mid}.json"));
    let looks_golden = mid.starts_with("msn_");
    println!("  missionId = {mid}");
    if looks_golden {
        println!("  loads via: profile fallback (backend answers 400 invalid-id — expected)");
    } else {
        println!("  loads via: backend /compiled (schema-validated)");
    }

    if local.is_file() {
        let local_text =
            fs::read_to_string(&local).with_context(|| format!("read {}", local.display()))?;
        let d: Value = serde_json::from_str(&local_text)
            .with_context(|| format!("parse {}", local.display()))?;
        let slots = d.get("slots").and_then(|s| s.as_array());
        let mut fac: BTreeMap<String, u32> = BTreeMap::new();
        if let Some(slots) = slots {
            for s in slots {
                let faction = s
                    .get("faction")
                    .and_then(|x| x.as_str())
                    .with_context(|| format!("slot missing faction in {}", local.display()))?;
                *fac.entry(faction.to_string()).or_insert(0) += 1;
            }
        }
        let n = slots.map(|a| a.len()).unwrap_or(0);
        let seats: String = fac
            .iter()
            .map(|(k, v)| format!("{k} {v}"))
            .collect::<Vec<_>>()
            .join(", ");
        let what = if looks_golden {
            "staged"
        } else {
            "last cached"
        };
        println!("  {n} seats — {seats}   ({what})");
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Mutex;

    /// Serialise HOME mutations across tests in this process.
    static HOME_LOCK: Mutex<()> = Mutex::new(());

    #[test]
    fn missing_config_exits_1() {
        let _g = HOME_LOCK.lock().unwrap();
        let home = tempfile_dir("nocfg");
        let root = throwaway_root("nocfg-root", true);
        unsafe { std::env::set_var("HOME", &home) };
        let code = run_with_root(&root, None).unwrap();
        assert_eq!(code, 1);
    }

    #[test]
    fn unknown_golden_exits_1() {
        let _g = HOME_LOCK.lock().unwrap();
        let (home, _prof, root) = primed_home("nogolden");
        unsafe { std::env::set_var("HOME", &home) };
        let code = run_with_root(&root, Some("does-not-exist-xyz")).unwrap();
        assert_eq!(code, 1);
    }

    #[test]
    fn backend_and_stage_round_trip() {
        let _g = HOME_LOCK.lock().unwrap();
        let (home, prof, root) = primed_home("roundtrip");
        unsafe { std::env::set_var("HOME", &home) };

        let code = run_with_root(&root, Some("bridgehead-at-levie")).unwrap();
        assert_eq!(code, 0);
        let cfg: Value =
            serde_json::from_str(&fs::read_to_string(prof.join("TBD_BackendConfig.json")).unwrap())
                .unwrap();
        assert_eq!(cfg["missionId"], "msn_8f3a2c");
        assert!(prof.join("missions/msn_8f3a2c.json").is_file());
        assert!(prof.join("TBD_Registry.json").is_file());

        let code = run_with_root(&root, Some("backend")).unwrap();
        assert_eq!(code, 0);
        let cfg: Value =
            serde_json::from_str(&fs::read_to_string(prof.join("TBD_BackendConfig.json")).unwrap())
                .unwrap();
        assert_eq!(cfg["missionId"], BACKEND_MISSION);
    }

    #[test]
    fn set_mission_id_no_trailing_newline() {
        let dir = tempfile_dir("json-nl");
        let cfg = dir.join("TBD_BackendConfig.json");
        fs::write(
            &cfg,
            r#"{"backendUrl":"http://x","serverToken":"t","missionId":"old","eventId":"e"}"#,
        )
        .unwrap();
        set_mission_id(&cfg, "msn_x").unwrap();
        let body = fs::read_to_string(&cfg).unwrap();
        assert!(
            !body.ends_with('\n'),
            "python json.dump has no trailing newline"
        );
        assert!(body.contains("\"missionId\": \"msn_x\""));
    }

    fn tempfile_dir(tag: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("t864-{tag}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    fn throwaway_root(tag: &str, with_golden: bool) -> PathBuf {
        let root = tempfile_dir(tag);
        fs::create_dir_all(root.join(".ai/tickets")).unwrap();
        fs::write(root.join(".ai/tickets/ROOT"), "{}").unwrap();
        fs::create_dir_all(root.join("apps/mod/tbd-framework/Data")).unwrap();
        fs::create_dir_all(root.join("packages/tbd-schema/golden-missions")).unwrap();
        fs::write(
            root.join("apps/mod/tbd-framework/Data/registry.json"),
            "{\"ok\":true}\n",
        )
        .unwrap();
        if with_golden {
            fs::write(
                root.join("packages/tbd-schema/golden-missions/bridgehead-at-levie.json"),
                r#"{"meta":{"id":"msn_8f3a2c"},"slots":[{"faction":"blufor"},{"faction":"blufor"},{"faction":"opfor"}]}"#,
            )
            .unwrap();
        }
        root
    }

    fn primed_home(tag: &str) -> (PathBuf, PathBuf, PathBuf) {
        let home = tempfile_dir(&format!("{tag}-home"));
        let root = throwaway_root(&format!("{tag}-root"), true);
        let prof = home.join(CFG_REL);
        fs::create_dir_all(prof.join("missions")).unwrap();
        fs::write(
            prof.join("TBD_BackendConfig.json"),
            format!(
                "{{\n  \"backendUrl\": \"http://127.0.0.1:8080\",\n  \"serverToken\": \"tok\",\n  \"missionId\": \"{BACKEND_MISSION}\",\n  \"eventId\": \"b0000000-0000-4000-8000-000000000001\"\n}}"
            ),
        )
        .unwrap();
        (home, prof, root)
    }
}
