use super::*;
/// The explicit empty state — rendered INSTEAD of a zeros panel.
pub fn no_estimates_text() -> String {
    format!(
        "no estimates yet — {}/ has no files; estimate files are generated for shipped tickets \
         that never got a run receipt",
        ticket_engine::repository::ESTIMATES_DIR
    )
}

/// The panel-level provenance banner (acceptance): estimated figures
/// are historical reconstruction and never enter a measured sum.
pub const NEVER_COMBINED_NOTE: &str =
    "estimated — historical reconstruction from recorded inputs; never combined with measured";

/// The provenance glyph — ONE glyph language with the scope breadcrumb
/// (`board::SCOPE_ESTIMATED_GLYPH`); parity is test-pinned.
pub const ESTIMATE_GLYPH: &str = "~";

/// Tooltip fallback when a stamp is marked estimated but the ticket carries no
/// `estimate_note` — explicit, never an invented method description.
pub const NOTE_ABSENT_TIP: &str = "listed in estimated[] — this ticket carries no estimate_note";

/// The absent-but-marked stamp marker (`shipped_at` mined nowhere: a SHA is
/// never invented, so the field is listed in `estimated[]` with the note naming
/// the gap).
pub const ABSENT_ESTIMATED_MARKER: &str = "— (estimated absent)";

pub fn estimates_dir(repo_root: &Path) -> PathBuf {
    repo_root.join(ticket_engine::repository::ESTIMATES_DIR)
}

pub(super) fn rel_of(repo_root: &Path, path: &Path) -> String {
    path.strip_prefix(repo_root)
        .unwrap_or(path)
        .display()
        .to_string()
}

/// Read + parse + validate ONE estimate file, mirroring the checker's per-file
/// rules — including filename-stem-must-equal-id.
pub(super) fn load_one(path: &Path) -> Result<ValidEstimate, String> {
    let text = fs::read_to_string(path).map_err(|e| format!("unreadable ({e})"))?;
    let rec: EstimateFile = serde_json::from_str(&text).map_err(|e| e.to_string())?;
    let source = validate_file(&rec)?;
    let stem = path
        .file_stem()
        .map(|s| s.to_string_lossy().into_owned())
        .unwrap_or_default();
    if rec.id != stem {
        return Err(format!(
            "estimate id {} does not match its filename stem {stem}",
            rec.id
        ));
    }
    Ok(ValidEstimate {
        id: rec.id,
        factor: rec.factor,
        tokens_estimated: rec.tokens_estimated,
        generated_at: rec.generated_at,
        source,
    })
}

/// Scan `repo_root/.ai/tickets/estimates/` (flat by contract — a subdirectory
/// is an error row, mirroring the checker's flat-tree rule). Every file is
/// either a validated record or a named [`ErrorRow`] — no third bucket.
pub fn load_raw(repo_root: &Path) -> RawEstimates {
    let dir = estimates_dir(repo_root);
    if !dir.is_dir() {
        return RawEstimates::default();
    }
    let mut raw = RawEstimates {
        present: true,
        ..RawEstimates::default()
    };
    let entries = match fs::read_dir(&dir) {
        Ok(rd) => rd,
        Err(e) => {
            raw.errors.push(ErrorRow {
                rel: rel_of(repo_root, &dir),
                reason: format!("unreadable directory ({e})"),
            });
            return raw;
        }
    };
    let mut paths: Vec<PathBuf> = entries.filter_map(|e| e.ok().map(|e| e.path())).collect();
    paths.sort();
    for path in paths {
        if path.is_dir() {
            raw.errors.push(ErrorRow {
                rel: rel_of(repo_root, &path),
                reason: format!(
                    "estimate files live flat at {}/<id>.json — unexpected subdirectory",
                    ticket_engine::repository::ESTIMATES_DIR
                ),
            });
            continue;
        }
        match load_one(&path) {
            Ok(rec) => raw.records.push(rec),
            Err(reason) => raw.errors.push(ErrorRow {
                rel: rel_of(repo_root, &path),
                reason,
            }),
        }
    }
    raw
}
