//! Migration.

use super::*;
use anyhow::Context;

pub(super) fn ticket_paths(root: &Path) -> Result<Vec<PathBuf>> {
    let dir = crate::registry::legacy_storage::tickets_dir(root);
    let mut paths: Vec<PathBuf> = fs::read_dir(&dir)
        .with_context(|| format!("read {}", dir.display()))?
        .filter_map(|e| e.ok())
        .map(|e| e.path())
        .filter(|p| {
            p.file_name().is_some_and(|n| {
                let n = n.to_string_lossy();
                n.starts_with("T-") && n.ends_with(".toml")
            })
        })
        .collect();
    paths.sort();
    Ok(paths)
}

/// THE cutover. Reads every `.ai/tickets/T-*.toml` as `toml::Value`, transforms,
/// validates the WHOLE corpus (typed parse + vocab legality + byte-stable re-render)
/// before writing a single byte, then lands each file temp+rename and regenerates the
/// sync surface from the reloaded registry.
pub fn cmd_migrate_v2(root: &Path) -> Result<()> {
    let vocab = ScopeVocab::load(root).map_err(anyhow::Error::msg)?;
    let paths = ticket_paths(root)?;
    let n_files = paths.len();
    let mut staged: Vec<(PathBuf, String)> = Vec::with_capacity(n_files);
    let mut unmapped: Vec<String> = Vec::new();
    let mut layers_multi: Vec<String> = Vec::new();
    let mut owns_inferred_ids = 0usize;

    for path in &paths {
        let name = path.file_name().unwrap().to_string_lossy().to_string();
        let text = fs::read_to_string(path).with_context(|| format!("read {name}"))?;
        let mut doc: toml::Value = text.parse().with_context(|| format!("{name}: not TOML"))?;
        let table = doc
            .as_table_mut()
            .with_context(|| format!("{name}: root is not a table"))?;
        let id = table
            .get("id")
            .and_then(|v| v.as_str())
            .with_context(|| format!("{name}: missing id"))?
            .to_string();
        let kind = table
            .get("kind")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();

        if kind == "work" {
            let owns = str_array(table.get("owns"));
            let old_scope = table
                .remove("scope")
                .with_context(|| format!("{id}: work ticket without [scope]"))?;
            if old_scope
                .as_table()
                .is_some_and(|t| t.contains_key("domain"))
            {
                bail!(
                    "{id}: [scope] already carries `domain` — the tree is v2; migrate-v2 is one-shot"
                );
            }
            if str_array(
                old_scope
                    .as_table()
                    .and_then(|t| t.values().next())
                    .and_then(|d| d.as_table())
                    .and_then(|t| t.get("layers")),
            )
            .len()
                > 1
            {
                layers_multi.push(id.clone());
            }
            let mapped = match map_scope(&id, &old_scope, &owns) {
                Ok(m) => m,
                Err(e) => {
                    unmapped.push(format!("{e:#}"));
                    continue;
                }
            };
            let mut scope_table = toml::map::Map::new();
            scope_table.insert(
                "domain".into(),
                toml::Value::String(mapped.domain.to_string()),
            );
            scope_table.insert("layer".into(), toml::Value::String(mapped.layer.clone()));
            if let Some(c) = &mapped.component {
                scope_table.insert("component".into(), toml::Value::String(c.clone()));
            }
            if !mapped.surface.is_empty() {
                scope_table.insert(
                    "surface".into(),
                    toml::Value::Array(
                        mapped
                            .surface
                            .iter()
                            .map(|s| toml::Value::String(s.clone()))
                            .collect(),
                    ),
                );
            }
            table.insert("scope".into(), toml::Value::Table(scope_table));

            // class triage — v1 never carried the key (governance-pinned), so this is
            // an insert, never an overwrite.
            if table.contains_key("class") {
                bail!("{id}: v1 file already carries `class` — governance breach, refusing");
            }
            let title = table.get("title").and_then(|v| v.as_str()).unwrap_or("");
            let summary = table.get("summary").and_then(|v| v.as_str()).unwrap_or("");
            table.insert(
                "class".into(),
                toml::Value::String(
                    crate::classify_work(&format!("{title} {summary}")).to_string(),
                ),
            );

            if mapped.owns_inferred {
                owns_inferred_ids += 1;
                if table.contains_key("estimated") {
                    bail!(
                        "{id}: v1 file already carries `estimated` — governance breach, refusing"
                    );
                }
                table.insert(
                    "estimated".into(),
                    toml::Value::Array(vec![toml::Value::String("scope".into())]),
                );
            }
        } else if table.get("scope").is_some() {
            bail!("{id}: program carries [scope] — v1 tree is broken, refusing");
        }

        // Typed validation + canonical render + byte-stable re-render gate, per file.
        let file: TicketFile = doc
            .try_into()
            .with_context(|| format!("{id}: transformed value does not deserialize as v2"))?;
        let ticket = file
            .into_ticket()
            .map_err(|e| anyhow::anyhow!("{id}: {e}"))?;
        if let Ticket::Work(w) = &ticket {
            vocab
                .check_scope(&w.id, &w.scope)
                .map_err(anyhow::Error::msg)?;
        }
        let rendered = render_ticket_toml(&ticket).map_err(anyhow::Error::msg)?;
        let back = parse_ticket_toml(&rendered)
            .map_err(|e| anyhow::anyhow!("{id}: rendered v2 does not re-parse: {e}"))?;
        if back != ticket {
            bail!("{id}: render → re-parse does not round-trip to the same ticket");
        }
        let again = render_ticket_toml(&back).map_err(anyhow::Error::msg)?;
        if again != rendered {
            bail!("{id}: re-render is not byte-stable");
        }
        staged.push((path.clone(), rendered));
    }

    println!("unmapped-scope list ({}):", unmapped.len());
    for u in &unmapped {
        println!("  {u}");
    }
    if !unmapped.is_empty() {
        bail!(
            "{} ticket(s) have unmapped scope — nothing written; widen the mapping tables",
            unmapped.len()
        );
    }
    if staged.len() != n_files {
        bail!(
            "staged {} of {n_files} files — refusing partial migration",
            staged.len()
        );
    }

    // Land the bytes, temp+rename per file (the write_back pattern).
    for (path, text) in &staged {
        let tmp = path.with_file_name(format!(
            ".{}.tmp",
            path.file_name().unwrap().to_string_lossy()
        ));
        fs::write(&tmp, text).with_context(|| format!("write {}", tmp.display()))?;
        fs::rename(&tmp, path)
            .with_context(|| format!("rename {} -> {}", tmp.display(), path.display()))?;
    }
    println!("migrated {}/{n_files} files", staged.len());
    if !layers_multi.is_empty() {
        println!(
            "multi-layer v1 arrays took their FIRST layer ({}): {}",
            layers_multi.len(),
            layers_multi.join(", ")
        );
    }
    println!("owns-inference used on {owns_inferred_ids} tickets (estimated += \"scope\")");

    // Full-corpus re-parse gate: the typed load (vocab legality included) must accept
    // the tree we just wrote.
    let corpus = Corpus::load(root).map_err(anyhow::Error::msg)?;
    println!(
        "corpus reload: {} tickets parse v2-clean",
        corpus.tickets.len()
    );

    // Regenerate the sync surface from the reloaded registry (docs/TICKET_*.md,
    // queue.json, CLAUDE marker) — sync.rs copies summaries verbatim.
    let registry = crate::registry::load_registry(root)?;
    crate::sync::cmd_sync(root, &registry)?;

    print_scope_histogram(&corpus);
    Ok(())
}

/// `ticket scope-histogram` — standing read-only verb (also the tail of the migration
/// report): per-domain/layer/component counts, per-surface counts, the
/// "U surface-empty (scope ∈ estimated: E)" honesty counters per component bucket,
/// and the work-ticket class distribution.
pub fn cmd_scope_histogram(root: &Path) -> Result<()> {
    let corpus = Corpus::load(root).map_err(anyhow::Error::msg)?;
    print_scope_histogram(&corpus);
    Ok(())
}

pub(super) fn print_scope_histogram(corpus: &Corpus) {
    let mut works = 0usize;
    let mut programs = 0usize;
    let mut buckets: BTreeMap<String, Vec<&crate::WorkTicket>> = BTreeMap::new();
    let mut classes: BTreeMap<String, usize> = BTreeMap::new();
    for t in corpus.tickets.values() {
        match t {
            Ticket::Program(_) => programs += 1,
            Ticket::Work(w) => {
                works += 1;
                let key = match &w.scope.component {
                    Some(c) => format!("{}/{}/{c}", w.scope.domain.as_str(), w.scope.layer),
                    None => format!("{}/{}", w.scope.domain.as_str(), w.scope.layer),
                };
                buckets.entry(key).or_default().push(w);
                *classes
                    .entry(w.class.clone().unwrap_or_else(|| "(none)".into()))
                    .or_default() += 1;
            }
        }
    }
    println!("scope histogram — {works} work tickets, {programs} programs");
    for (key, tickets) in &buckets {
        println!("{key}: {}", tickets.len());
        let mut surfaces: BTreeMap<&str, usize> = BTreeMap::new();
        let mut empty = 0usize;
        let mut empty_marked = 0usize;
        for w in tickets {
            if w.scope.surface.is_empty() {
                empty += 1;
                if w.estimated.iter().any(|e| e == "scope") {
                    empty_marked += 1;
                }
            }
            for s in &w.scope.surface {
                *surfaces.entry(s.as_str()).or_default() += 1;
            }
        }
        if !surfaces.is_empty() {
            let list: Vec<String> = surfaces.iter().map(|(s, n)| format!("{s} {n}")).collect();
            println!("  surfaces: {}", list.join(", "));
        }
        if empty > 0 {
            println!("  {empty} surface-empty (scope ∈ estimated: {empty_marked})");
        }
    }
    let class_line: Vec<String> = classes.iter().map(|(c, n)| format!("{c} {n}")).collect();
    println!("class distribution (work): {}", class_line.join(", "));
}
