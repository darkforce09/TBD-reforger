use anyhow::Result;
use regex::Regex;
use serde_json::Value;
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::LazyLock;
use walkdir::WalkDir;

use crate::constants::*;
use crate::gap::test_gap_analysis_round_trip;
use crate::registry::*;
use crate::root::gap_analysis_path;

static STRICT_RE: LazyLock<Regex> = LazyLock::new(|| Regex::new(STRICT_LEGACY).unwrap());
static PRIORITY_P: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"(?m)^\d+\.\s+\*\*P[0-3]").unwrap());

/// Cap schema-error spam so a broken registry still yields an actionable first page.
const SCHEMA_ERROR_CAP: usize = 100;

fn ticket_schema_path(root: &Path) -> PathBuf {
    root.join(".ai/tickets/schema.json")
}

/// Validate `registry` against Draft 2020-12 `.ai/tickets/schema.json`.
/// Missing/unreadable/uncompilable schema is itself a hard failure (never silent skip).
pub fn validate_registry_schema(root: &Path, registry: &Value) -> Vec<String> {
    let path = ticket_schema_path(root);
    if !path.is_file() {
        return vec![format!(
            "missing ticket schema (required for ticket check): {}",
            path.display()
        )];
    }
    let text = match fs::read_to_string(&path) {
        Ok(t) => t,
        Err(e) => {
            return vec![format!("read ticket schema {}: {e}", path.display())];
        }
    };
    let schema: Value = match serde_json::from_str(&text) {
        Ok(v) => v,
        Err(e) => {
            return vec![format!("parse ticket schema {}: {e}", path.display())];
        }
    };
    let validator = match jsonschema::validator_for(&schema) {
        Ok(v) => v,
        Err(e) => {
            return vec![format!("compile ticket schema {}: {e}", path.display())];
        }
    };
    let mut errors = Vec::new();
    for err in validator.iter_errors(registry) {
        let inst = err.instance_path().to_string();
        let loc = if inst.is_empty() {
            "/".to_string()
        } else {
            inst
        };
        // Use masked() so a root-type failure does not dump the entire registry JSON
        // into stderr (Display of ValidationError embeds the instance value).
        errors.push(format!("schema {loc}: {}", err.masked()));
        if errors.len() >= SCHEMA_ERROR_CAP {
            errors.push(format!(
                "schema: truncated after {SCHEMA_ERROR_CAP} errors (fix remaining silently)"
            ));
            break;
        }
    }
    errors
}

fn validate_row(row: &serde_json::Value) -> Vec<String> {
    let mut errors = vec![];
    let tid = opt_str(row, "id").unwrap_or("?");
    let status = opt_str(row, "status").unwrap_or("");
    let typed = is_truthy(row.get("kind"));
    let required = if typed {
        ["id", "title", "summary", "kind", "status"].as_slice()
    } else {
        [
            "id", "title", "summary", "program", "surfaces", "impact", "status",
        ]
        .as_slice()
    };
    for key in required {
        if !is_truthy(row.get(key)) {
            errors.push(format!("{tid}: missing {key}"));
        }
    }
    if status != "idea" && !order_truthy(row) {
        errors.push(format!(
            "{}: order required for status {status}",
            opt_str(row, "id").unwrap_or("?")
        ));
    }
    if typed && matches!(status, "ready" | "running" | "review") {
        if opt_str(row, "spec").unwrap_or("").trim().is_empty() {
            errors.push(format!("{tid}: ready-class requires spec"));
        }
        if opt_str(row, "user_story").unwrap_or("").trim().is_empty() {
            errors.push(format!("{tid}: ready-class requires user_story"));
        }
        let acc_ok = row
            .get("acceptance")
            .and_then(Value::as_array)
            .is_some_and(|a| {
                a.iter()
                    .any(|s| s.as_str().is_some_and(|x| !x.trim().is_empty()))
            });
        if !acc_ok {
            errors.push(format!("{tid}: ready-class requires acceptance"));
        }
    }
    if let Some(id) = opt_str(row, "id") {
        if FORBIDDEN_PHANTOM_IDS.contains(&id) {
            errors.push(format!("Forbidden phantom id {id}"));
        }
    }
    errors
}

fn validate_registry(registry: &serde_json::Value) -> Vec<String> {
    let mut errors = vec![];
    let mut ids = std::collections::HashSet::new();
    let mut live_orders: HashMap<i64, String> = HashMap::new();
    for row in tickets(registry) {
        errors.extend(validate_row(row));
        let tid = str_field(row, "id");
        if !tid.is_empty() {
            if ids.contains(&tid) {
                errors.push(format!("Duplicate id {tid}"));
            }
            ids.insert(tid.clone());
        }
        let status = opt_str(row, "status").unwrap_or("");
        if matches!(status, "queued" | "ready" | "running" | "review") {
            if let Some(order) = row.get("order").and_then(Value::as_i64) {
                if let Some(other) = live_orders.insert(order, tid.clone()) {
                    errors.push(format!("duplicate live order {order} on {other} and {tid}"));
                }
            }
        }
    }
    errors
}

/// T-912.1: every open work ticket must own its collision surface — the wave packer reads ticket
/// `owns` now, and an owns-empty ticket is invisible to every dispatch set it computes.
///
/// Reads EVERY `.ai/tickets/T-*.toml` through the shared typed corpus (T-916.2 — the store
/// replaced this fn's own glob). `tickets(registry)` walks the parents-only phase-2 view,
/// which would silently exempt children (T-181.16, T-912.2, …) from the rule. Fail-closed on
/// an unloadable corpus: the load error (naming the first offending file) is the finding — a
/// guard that cannot scan must not report clean.
fn check_open_work_owns(root: &Path) -> Vec<String> {
    let corpus = match tbd_tickets::Corpus::load(root) {
        Ok(c) => c,
        Err(e) => return vec![e],
    };
    let mut errors = Vec::new();
    for ticket in corpus.tickets.values() {
        if let tbd_tickets::Ticket::Work(w) = ticket {
            let status = w.status.name().as_str();
            if matches!(status, "queued" | "ready" | "running" | "review") && w.owns.is_empty() {
                errors.push(format!("{}: owns required for {status} work ticket", w.id));
            }
        }
    }
    errors
}

/// T-917.2 — class is REQUIRED on work tickets (spec Decisions log #4; the value set is
/// parse-validated in tbd-tickets, so only ABSENCE can red here). Corpus-wide: the v2
/// migrator triaged every historical work ticket and the minters classify at birth, so
/// no status tier is exempt. Fail-closed on an unloadable corpus, same as the owns rule.
fn check_work_class(root: &Path) -> Vec<String> {
    let corpus = match tbd_tickets::Corpus::load(root) {
        Ok(c) => c,
        Err(e) => return vec![e],
    };
    let mut errors = Vec::new();
    for ticket in corpus.tickets.values() {
        if let tbd_tickets::Ticket::Work(w) = ticket
            && w.class.is_none()
        {
            errors.push(format!(
                "{}: class required for work ticket (one of {})",
                w.id,
                tbd_tickets::CLASS_VALUES.join("|")
            ));
        }
    }
    errors
}

/// T-917.2 — surface is REQUIRED on live work (spec Decisions log #3), the corpus-wide
/// mirror of the ops made-live gate. Binds exactly where a surface is *possible*: the
/// scope must name a component AND the vocabulary must OFFER surfaces for it — a rule
/// cannot require what `.ai/tickets/scope-vocab.toml` does not contain (component-free
/// layers and empty-surface components like `mod.scripts.backend` are exempt until the
/// vocabulary is widened). The escape is the migrator's honest `"scope" ∈ estimated[]`
/// marker: owns-uninferable history is recorded, not invented — a live ticket in a
/// surface-bearing component with neither surface nor marker is red.
fn check_live_work_surface(root: &Path) -> Vec<String> {
    let corpus = match tbd_tickets::Corpus::load(root) {
        Ok(c) => c,
        Err(e) => return vec![e],
    };
    // Corpus::load above already refused on a missing/unreadable vocabulary.
    let vocab = match tbd_tickets::ScopeVocab::load(root) {
        Ok(v) => v,
        Err(e) => return vec![e],
    };
    let mut errors = Vec::new();
    for ticket in corpus.tickets.values() {
        if let tbd_tickets::Ticket::Work(w) = ticket {
            let status = w.status.name().as_str();
            if matches!(status, "queued" | "ready" | "running" | "review")
                && let Some(component) = &w.scope.component
                && w.scope.surface.is_empty()
                && !w.estimated.iter().any(|e| e == "scope")
                && vocab
                    .surfaces_of(w.scope.domain.as_str(), &w.scope.layer, component)
                    .is_some_and(|s| !s.is_empty())
            {
                errors.push(format!(
                    "{}: surface required for {status} work ticket — scope names component {component} but surface is empty; set [scope] surface or record \"scope\" in estimated[]",
                    w.id
                ));
            }
        }
    }
    errors
}

/// T-917.3 quarantine cutover — the one-shot `ticket quarantine-walls` pass ran on
/// history created BEFORE this date; a work ticket carrying `migration_legacy` with a
/// later `created_at` is a NEW ticket minting the field, which is red (new tickets
/// never quarantine — they write the ten typed body fields). Bare-date string
/// comparison is sound: stamps are validated RFC 3339 UTC (`...T..:..:..Z`), which
/// sorts lexically, and any stamp on/after the cutover day compares greater than the
/// bare date by the prefix rule.
const QUARANTINE_CUTOVER: &str = "2026-08-15";

/// T-917.3 — body word caps, anti-blend rules and the quarantine-mint tripwire
/// (spec §Body + §Wall quarantine; Decisions log #6: CHECK-enforced, never
/// parse-enforced — old git revisions must stay readable). Warnings (the
/// command-shaped-acceptance rule) are eprinted, never errors. Fail-closed on an
/// unloadable corpus, same as every corpus rule here.
fn check_body_rules(root: &Path) -> Vec<String> {
    let corpus = match tbd_tickets::Corpus::load(root) {
        Ok(c) => c,
        Err(e) => return vec![e],
    };
    let (errors, warnings) = body_findings(&corpus);
    for w in &warnings {
        eprintln!("WARNING: {w}");
    }
    errors
}

/// The pure rule set over a loaded corpus → (errors, warnings). Split from
/// [`check_body_rules`] so tests can assert the warning channel.
///
/// Scoping decisions (measured against the live tree 2026-08-15 — zero nonempty
/// body-list fields existed, so every scoping choice starts live-green):
///
/// - **Caps bind on WORK tickets only** (prompt + spec §Wall quarantine: the pass is
///   work-only; program summaries stay uncapped and the verb reports over-cap ones as
///   a future note). Counting instrument: `split_whitespace().count()` on the
///   TOML-parsed string — the same instrument as the verb, ops gate and ratchet pin.
/// - **A nonempty `migration_legacy` exempts EXACTLY the summary cap** (quarantined
///   tickets carry `summary := title`, which may itself exceed the cap and must not
///   be truncated). Every other cap still binds on quarantined tickets.
/// - **Anti-blend rules bind on BOTH kinds** — they are field-relationship rules, not
///   caps, and programs carry `citations`/`owns`/`acceptance` too: a `citations[]`
///   entry duplicating an `owns[]` entry is red (ownership facts must not split
///   across fields); a command-shaped `acceptance[]` line (starts `cargo `/`$ `/`./`)
///   WARNS pointing at `verify[]`.
fn body_findings(corpus: &tbd_tickets::Corpus) -> (Vec<String>, Vec<String>) {
    use tbd_tickets::{BODY_LINE_WORD_CAP, CITATION_WORD_CAP, SUMMARY_WORD_CAP, Ticket};
    let mut errors = Vec::new();
    let mut warnings = Vec::new();
    let words = |s: &str| s.split_whitespace().count();
    for (id, ticket) in &corpus.tickets {
        let (citations, owns, acceptance) = match ticket {
            Ticket::Program(p) => (&p.citations, &p.owns, &p.acceptance),
            Ticket::Work(w) => (&w.citations, &w.owns, &w.acceptance),
        };
        for (i, c) in citations.iter().enumerate() {
            if owns.iter().any(|o| o == c) {
                errors.push(format!(
                    "{id}: citations[{i}] \"{c}\" duplicates an owns[] entry — ownership facts must not split across fields"
                ));
            }
        }
        for (i, a) in acceptance.iter().enumerate() {
            if a.starts_with("cargo ") || a.starts_with("$ ") || a.starts_with("./") {
                let first = a.split_whitespace().next().unwrap_or("");
                warnings.push(format!(
                    "{id}: acceptance[{i}] is command-shaped (starts \"{first}\") — commands to run belong in verify[]; acceptance states outcomes"
                ));
            }
        }
        let Ticket::Work(w) = ticket else { continue };
        if w.migration_legacy.is_empty() {
            let n = words(&w.summary);
            if n > SUMMARY_WORD_CAP {
                errors.push(format!(
                    "{id}: summary is {n} words (cap {SUMMARY_WORD_CAP})"
                ));
            }
        } else if let Some(created) = w.created_at.as_deref()
            && created > QUARANTINE_CUTOVER
        {
            errors.push(format!(
                "{id}: migration_legacy on a ticket created {created} — past the {QUARANTINE_CUTOVER} quarantine cutover; new tickets never quarantine, write the ten typed body fields instead"
            ));
        }
        for (field, lines) in [
            ("context", &w.context),
            ("requirement", &w.requirement),
            ("current_state", &w.current_state),
            ("approach", &w.approach),
            ("verify", &w.verify),
        ] {
            for (i, line) in lines.iter().enumerate() {
                let n = words(line);
                if n > BODY_LINE_WORD_CAP {
                    errors.push(format!(
                        "{id}: {field}[{i}] is {n} words (cap {BODY_LINE_WORD_CAP})"
                    ));
                }
            }
        }
        for (i, c) in w.citations.iter().enumerate() {
            let n = words(c);
            if n > CITATION_WORD_CAP {
                errors.push(format!(
                    "{id}: citations[{i}] is {n} words (cap {CITATION_WORD_CAP})"
                ));
            }
        }
    }
    (errors, warnings)
}

/// T-917.4 — estimated[]-vs-field coherence (the S.6 gate builds on this rule). An
/// `estimated[]` stamp entry must correspond to a PRESENT field — a marked estimate
/// with no value is a hole wearing a provenance badge — with exactly one legal
/// asymmetry: `shipped_at` may be absent+marked WHEN `estimate_note` names the gap
/// (spec §Estimation ladder shipped_at row: a SHA is never invented — a
/// present-but-fake SHA would point at a real commit that is NOT the ticket's work,
/// worse than absence). `created_at`/`completed_at` listed with the field absent are
/// red; `tokens`/`scope` coherence belongs to the S.5 estimates check and the
/// surface rule respectively, not here. `shipped_at` reads through BOTH arms (work
/// field / program status). Fail-closed on an unloadable corpus, like every corpus
/// rule in this file.
fn check_estimated_stamp_coherence(root: &Path) -> Vec<String> {
    let corpus = match tbd_tickets::Corpus::load(root) {
        Ok(c) => c,
        Err(e) => return vec![e],
    };
    let mut errors = Vec::new();
    for (id, ticket) in &corpus.tickets {
        let (created, completed, shipped, estimated, note) = match ticket {
            tbd_tickets::Ticket::Program(p) => {
                let shipped = match &p.status {
                    tbd_tickets::Status::Shipped { shipped_at, .. } => shipped_at.as_deref(),
                    _ => None,
                };
                (
                    p.created_at.as_deref(),
                    p.completed_at.as_deref(),
                    shipped,
                    &p.estimated,
                    p.estimate_note.as_deref(),
                )
            }
            tbd_tickets::Ticket::Work(w) => (
                w.created_at.as_deref(),
                w.completed_at.as_deref(),
                w.shipped_at.as_deref(),
                &w.estimated,
                w.estimate_note.as_deref(),
            ),
        };
        for e in estimated {
            match e.as_str() {
                "created_at" if created.is_none() => errors.push(format!(
                    "{id}: estimated[] lists created_at but the field is absent — dates must be present when marked (only shipped_at may be legally absent-marked)"
                )),
                "completed_at" if completed.is_none() => errors.push(format!(
                    "{id}: estimated[] lists completed_at but the field is absent — dates must be present when marked (only shipped_at may be legally absent-marked)"
                )),
                "shipped_at"
                    if shipped.is_none()
                        && note.is_none_or(|n| n.trim().is_empty()) =>
                {
                    errors.push(format!(
                        "{id}: estimated[] lists shipped_at with the field absent and no estimate_note naming the gap — absent-marked is legal only with the gap named"
                    ));
                }
                _ => {}
            }
        }
    }
    errors
}

/// T-916.2 — parent↔child referential integrity over EVERY `.ai/tickets/T-*.toml` (the typed
/// corpus; parents-only walks cannot see either half of the relation). Two rules, both naming
/// the pair:
///
/// - every `children[]` entry must have an on-disk `T-<child>.toml` — with `save_tree`'s
///   delete pass gone (T-916.2 demoted it to migration/test duty) a mangled `children[]` can
///   no longer mass-delete files, but a listing without a file was previously INVISIBLE:
///   nothing checked parent↔child at all;
/// - every child file's `parent` must exist on disk — a removed parent would otherwise strand
///   its children as permanently unreachable rows.
///
/// Measured against the live tree 2026-08-14: ZERO violations, so no allowlist. The one
/// pre-known oddity — T-111 (frozen-unmappable parking) cross-listing T-067.1 — satisfies both
/// rules because the file and its parent both exist; only a dotted-extension SHAPE rule would
/// red it, and that rule deliberately lives in the ops post-image gate (changed programs only),
/// not here, exactly so frozen history stays green.
///
/// Fail-closed: a corpus that cannot load reports the load error — a guard that cannot scan
/// must not report clean (the fossil-guard precedent).
fn check_children_integrity(root: &Path) -> Vec<String> {
    let corpus = match tbd_tickets::Corpus::load(root) {
        Ok(c) => c,
        Err(e) => return vec![e],
    };
    let mut errors = Vec::new();
    for (id, ticket) in &corpus.tickets {
        match ticket {
            tbd_tickets::Ticket::Program(p) => {
                for child in &p.children {
                    if !corpus.tickets.contains_key(child) {
                        errors.push(format!(
                            "{id}: children[] names {child}, which has no .ai/tickets/{child}.toml on disk"
                        ));
                    }
                }
            }
            tbd_tickets::Ticket::Work(w) => {
                if let Some(parent) = &w.parent {
                    if !corpus.tickets.contains_key(parent) {
                        errors.push(format!(
                            "{id}: parent {parent} has no .ai/tickets/{parent}.toml on disk"
                        ));
                    }
                }
            }
        }
    }
    errors
}

/// T-912.2 fossil-path guard: the wave-plan TSVs and their env knobs are dead, and any LIVE
/// mention of them is a regression vector — a reader quietly retargeted at a file that no longer
/// exists is exactly the false-green class this program killed. Greps the tracked tree (working
/// contents, so an uncommitted plant is caught) minus a tight historical allowlist.
///
/// Needles are assembled at runtime, same trick as the T-912.1 `const DEPS` tripwire, so this
/// file's own source cannot satisfy the scan it performs.
fn fossil_needles() -> [String; 3] {
    [
        format!("wave_plan{}", ".tsv"),
        format!("TBD_WAVE{}", "_PLAN"),
        format!("TBD_WAVE_GENERATION{}", "_FLOOR"),
    ]
}

/// Paths where a fossil mention is genuinely historical. Every entry carries its reason; keep
/// this list TIGHT — a live doc that names the TSV as current truth gets UPDATED, not listed.
const FOSSIL_ALLOWLIST: &[(&str, &str)] = &[
    (
        ".ai/artifacts/",
        "pipeline output — frozen run reports and verify logs",
    ),
    (
        ".ai/tickets/",
        "ticket notes/summaries narrate the TSV era; owns cells may name deleted paths",
    ),
    (
        "docs/TICKET_",
        "generated views (ticket sync) — they quote ticket prose verbatim",
    ),
    (
        "docs/platform/SHIPPED_HISTORY.md",
        "the shipped-history archive describes past states in past commits",
    ),
    (
        "docs/platform/t911_ticket_registry_redesign.md",
        "T-911 program spec — approved design text, written while the TSVs lived",
    ),
    (
        "docs/platform/t912_wave_lockfile.md",
        "this program's own spec names the files it deletes",
    ),
    (
        "docs/platform/GROK_WAVE_130_HANDOFF.md",
        "past kickoff doc for a finished wave — a snapshot, not a runbook",
    ),
    (
        "docs/platform/WAVE209_GROK_KICKOFF.md",
        "past kickoff doc for a finished wave — a snapshot, not a runbook",
    ),
    (
        "xtask/src/wave/legacy_plan.rs",
        "the ONE module allowed to name the dead files: git-show history reads for pre-cutover \
         wave-close corroboration plus the one-shot migration",
    ),
    (
        "apps/mod/tbd-framework/Scripts/Game/TBD/Backend/TBD_MissionValidator.c",
        "T-181-era lane note in an Enfusion comment; mod scripts are workbench-gated (D5), not \
         agent-editable from a platform slice",
    ),
    (
        "apps/website/api/migrations/0011_events_server_modpack.sql",
        "committed migrations are checksum-frozen (db_migrate persist audits them); editing one \
         to reword a comment is the a843905f incident",
    ),
];

fn fossil_paths_check(root: &Path) -> Vec<String> {
    let needles = fossil_needles();
    let mut cmd = std::process::Command::new("git");
    cmd.arg("-C")
        .arg(root)
        .args(["grep", "-l", "-I", "--fixed-strings"]);
    for n in &needles {
        cmd.args(["-e", n]);
    }
    cmd.args(["--", "."]);
    let out = match cmd.output() {
        Ok(o) => o,
        Err(e) => return vec![format!("fossil-path guard could not run git grep: {e}")],
    };
    // git grep: 0 = matches, 1 = no matches, anything else = failure. Fail closed — a guard
    // that cannot scan must not report clean.
    match out.status.code() {
        Some(0) | Some(1) => {}
        other => {
            return vec![format!(
                "fossil-path guard: git grep failed (rc {other:?}): {}",
                String::from_utf8_lossy(&out.stderr).trim()
            )];
        }
    }
    let mut errors = Vec::new();
    for path in String::from_utf8_lossy(&out.stdout).lines() {
        let path = path.trim();
        if path.is_empty() {
            continue;
        }
        if FOSSIL_ALLOWLIST.iter().any(|(p, _)| path.starts_with(p)) {
            continue;
        }
        errors.push(format!(
            "dead wave-plan reference in {path} — the TSVs and their env knobs died at T-912.2; \
             read .ai/tickets/wave.lock (historical mentions belong on the allowlist in \
             xtask/src/check.rs, with a reason)"
        ));
    }
    errors
}

fn scan_legacy_ids(root: &Path) -> HashMap<String, Vec<String>> {
    let mut hits: HashMap<String, Vec<String>> = HashMap::new();
    let scan_roots: Vec<PathBuf> = vec![
        root.join("docs"),
        root.join("docs/specs"),
        root.join(".ai/tickets/queue.json"),
        root.join("CLAUDE.md"),
        root.join("README.md"),
    ];
    for base in scan_roots {
        let files: Vec<PathBuf> = if base.is_file() {
            vec![base]
        } else if base.is_dir() {
            WalkDir::new(&base)
                .into_iter()
                .filter_map(|e| e.ok())
                .filter(|e| e.file_type().is_file())
                .map(|e| e.path().to_path_buf())
                .collect()
        } else {
            continue;
        };
        for f in files {
            let rel = match f.strip_prefix(root) {
                Ok(r) => r.to_string_lossy().replace('\\', "/"),
                Err(_) => continue,
            };
            if EXEMPT_SCAN_PREFIXES
                .iter()
                .any(|p| rel.starts_with(p) || rel.contains(p))
            {
                continue;
            }
            if rel.ends_with("REORG_CHANGELOG.md") {
                continue;
            }
            let text = match fs::read_to_string(&f) {
                Ok(t) => t,
                Err(_) => continue,
            };
            let matches: Vec<String> = STRICT_RE
                .find_iter(&text)
                .map(|m| m.as_str().to_string())
                .collect();
            if !matches.is_empty() {
                hits.insert(rel, matches);
            }
        }
    }
    hits
}

pub fn check(root: &Path, registry: &serde_json::Value, strict: bool) -> Vec<String> {
    // Schema first: structural/enum contract from .ai/tickets/schema.json (T-237 / T-273).
    // Hand-rolled checks below add business rules (order, phantoms, on-disk specs, markers).
    let mut errors = validate_registry_schema(root, registry);
    errors.extend(validate_registry(registry));
    errors.extend(check_open_work_owns(root));
    // T-917.2: class required on every work ticket; surface required on live work with
    // a component (the estimated-marker escape documented on the fns).
    errors.extend(check_work_class(root));
    errors.extend(check_live_work_surface(root));
    // T-917.3: body word caps (summary ≤40 on work, migration_legacy-exempt; body
    // lines ≤30; citations ≤8), anti-blend rules, and the post-cutover
    // quarantine-mint tripwire.
    errors.extend(check_body_rules(root));
    // T-917.4: an estimated[] stamp entry must have a present field — except
    // shipped_at, which may be absent+marked with the gap named in estimate_note.
    errors.extend(check_estimated_stamp_coherence(root));
    // T-916.2: children[] must reference on-disk files and child files must reference on-disk
    // parents — the referential rule the save_tree delete-pass retirement makes load-bearing.
    errors.extend(check_children_integrity(root));
    // T-912.2: the committed wave.lock must match the tickets it was compiled from, and a
    // MISSING lock is a DidNotRun refusal — wired into the base check so every registry mutator
    // preflight and CI's `ticket check --strict` cover it.
    errors.extend(crate::wave_lock::check_as_errors(root));
    // T-913.2: every run receipt under .ai/tickets/metrics/ must satisfy
    // .ai/tickets/metrics.schema.json plus the token-sum / RFC 3339 UTC invariants —
    // a malformed receipt is red, named by file.
    errors.extend(crate::metrics::check_as_errors(root));
    // T-917.1/.2: the scope vocabulary (.ai/tickets/scope-vocab.toml) must EXIST and be
    // shape-valid — BASE tier since the S.2 cutover (S.1 parked existence at --strict
    // only while pre-v2 scratch registries still lacked the file): scope legality now
    // rides every corpus load, so a missing vocabulary is red wherever check runs.
    errors.extend(crate::vocab_check::check_as_errors(root));
    errors.extend(fossil_paths_check(root));

    for row in tickets(registry) {
        let tid = str_field(row, "id");
        if let Some(targets) = row.get("targets").and_then(|t| t.as_array()) {
            for tgt in targets {
                if let Some(s) = tgt.as_str() {
                    if !VALID_TARGETS.contains(&s) {
                        errors.push(format!("{tid}: invalid target '{s}'"));
                    }
                }
            }
        }
        if let Some(ex) = opt_str(row, "executor") {
            if !VALID_EXECUTORS.contains(&ex) {
                errors.push(format!("{tid}: invalid executor '{ex}'"));
            }
        }
        if let Some(stream) = opt_str(row, "stream") {
            if !VALID_STREAMS.contains(&stream) {
                errors.push(format!("{tid}: invalid stream '{stream}'"));
            }
        }
        if let Some(plan) = row.get("slice_plan").and_then(|p| p.as_object()) {
            for (sid, meta) in plan {
                if let Some(targets) = meta.get("targets").and_then(|t| t.as_array()) {
                    for tgt in targets {
                        if let Some(s) = tgt.as_str() {
                            if !VALID_TARGETS.contains(&s) {
                                errors.push(format!("{tid} slice {sid}: invalid target '{s}'"));
                            }
                        }
                    }
                }
                let ex_ok = meta
                    .get("executor")
                    .and_then(|e| e.as_str())
                    .map(|e| VALID_EXECUTORS.contains(&e))
                    .unwrap_or(false);
                if !ex_ok {
                    errors.push(format!("{tid} slice {sid}: invalid executor"));
                }
            }
        }
    }

    for tid in FORBIDDEN_PHANTOM_IDS {
        if ticket_by_id(registry, tid).is_some() {
            errors.push(format!("Forbidden phantom ticket row: {tid}"));
        }
    }

    for row in tickets(registry) {
        let tid = str_field(row, "id");
        let spec = opt_str(row, "spec").unwrap_or("").trim().to_string();
        let status = opt_str(row, "status").unwrap_or("");
        if !spec.is_empty() && status != "idea" && status != "cancelled" {
            if !root.join(&spec).is_file() {
                errors.push(format!("{tid}: spec missing on disk: {spec}"));
            }
        }
    }

    let claude = root.join("CLAUDE.md");
    let roadmap = root.join("docs/specs/Mission_Creator_Architecture/ROADMAP.md");
    for (p, start, end) in [
        (&claude as &Path, STATUS_MARKER_START, STATUS_MARKER_END),
        (&roadmap, NEXT_MARKER_START, NEXT_MARKER_END),
    ] {
        if p.is_file() {
            let text = fs::read_to_string(p).unwrap_or_default();
            if !text.contains(start) || !text.contains(end) {
                let rel = p.strip_prefix(root).unwrap_or(p);
                errors.push(format!("Missing markers in {}", rel.display()));
            }
        }
    }

    if let Err(e) = test_gap_analysis_round_trip(root) {
        errors.push(e.to_string());
    }

    if strict {
        let hits = scan_legacy_ids(root);
        for (path, matches) in hits {
            errors.push(format!("Legacy ID in {path}: {} match(es)", matches.len()));
        }
        let gap = gap_analysis_path(root);
        if gap.is_file() {
            let text = fs::read_to_string(&gap).unwrap_or_default();
            if text.contains("| priority |") || PRIORITY_P.is_match(&text) {
                errors.push("gap_analysis still has priority column or numbered P backlog".into());
            }
        }
    }

    errors
}

pub fn cmd_check(root: &Path, registry: &serde_json::Value, strict: bool) -> Result<()> {
    let errors = check(root, registry, strict);
    if !errors.is_empty() {
        for e in &errors {
            eprintln!("ERROR: {e}");
        }
        std::process::exit(1);
    }
    println!("check OK");
    Ok(())
}

/// Schema + structural preflight shared by registry mutators
/// (`ship`/`done` — T-237; `set-status`/`mark-ready`/`reorder` — T-451;
/// `add`/`remove` — T-455).
///
/// Returns `Ok(())` when `check` is green; `Err` with a refuse message when red.
/// Callers must not mutate the registry on `Err`. Prefer this over `process::exit`
/// so unit tests can assert refusal without killing the test process.
pub fn require_check_ok(root: &Path, registry: &Value, context: &str) -> Result<()> {
    let errors = check(root, registry, false);
    if errors.is_empty() {
        return Ok(());
    }
    for e in &errors {
        eprintln!("ERROR: {e}");
    }
    anyhow::bail!(
        "refusing {context}: ticket check failed ({} error(s))",
        errors.len()
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use std::path::PathBuf;

    fn worktree_root() -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .expect("xtask parent = repo/worktree root")
            .to_path_buf()
    }

    #[test]
    fn tip_registry_passes_schema() {
        let root = worktree_root();
        let registry = load_registry(&root).expect("load tip registry");
        let errs = validate_registry_schema(&root, &registry);
        assert!(
            errs.is_empty(),
            "tip registry must PASS schema; got:\n{}",
            errs.join("\n")
        );
    }

    #[test]
    fn tip_registry_full_check_ok() {
        let root = worktree_root();
        let registry = load_registry(&root).expect("load tip registry");
        let errs = check(&root, &registry, false);
        assert!(
            errs.is_empty(),
            "tip registry must PASS full check; got:\n{}",
            errs.join("\n")
        );
    }

    #[test]
    fn perturbed_ticket_field_fails_schema() {
        let root = worktree_root();
        let mut registry = load_registry(&root).expect("load tip registry");
        let tickets = registry
            .get_mut("tickets")
            .and_then(|t| t.as_array_mut())
            .expect("tickets array");
        let first = tickets.first_mut().expect("at least one ticket");
        first
            .as_object_mut()
            .expect("ticket object")
            .remove("title");
        let errs = validate_registry_schema(&root, &registry);
        assert!(
            !errs.is_empty(),
            "removing required title must make schema check RED"
        );
        assert!(
            errs.iter().any(|e| e.contains("schema")),
            "errors should be schema-tagged: {errs:?}"
        );
    }

    #[test]
    fn perturbed_schema_rejects_tip_registry() {
        let root = worktree_root();
        let registry = load_registry(&root).expect("load tip registry");
        let schema_path = ticket_schema_path(&root);
        let schema_text = fs::read_to_string(&schema_path).expect("read schema");
        let mut schema: Value = serde_json::from_str(&schema_text).expect("parse schema");
        // Narrow root type to array — tip registry is an object → must fail.
        schema
            .as_object_mut()
            .expect("schema object")
            .insert("type".into(), json!("array"));
        let validator =
            jsonschema::validator_for(&schema).expect("perturbed schema still compiles");
        let errs: Vec<_> = validator.iter_errors(&registry).collect();
        assert!(
            !errs.is_empty(),
            "type=array schema must reject object registry"
        );
    }

    /// Scratch tickets dir carrying the minimal vocabulary the fail-closed corpus load
    /// requires since T-917.2 (Corpus::load resolves scope legality on every load).
    fn scratch_tickets_dir(tag: &str) -> (PathBuf, PathBuf) {
        let tmp = std::env::temp_dir().join(format!("{tag}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&tmp);
        let dir = tmp.join(".ai/tickets");
        fs::create_dir_all(&dir).unwrap();
        fs::write(
            dir.join("scope-vocab.toml"),
            "[mod.scripts]\nbackend = []\n\n[repo.docs]\n\n[website.frontend]\nmission_creator = [\"map_canvas\"]\n",
        )
        .unwrap();
        (tmp, dir)
    }

    /// T-912.1: the owns rule sees CHILD ticket files. The live tree must be green, and an
    /// owns-empty queued work ticket dropped into a synthetic tickets dir must go red — including
    /// a dotted child id the parents-only registry view never loads.
    #[test]
    fn open_work_without_owns_is_red() {
        let root = worktree_root();
        let errs = check_open_work_owns(&root);
        assert!(
            errs.is_empty(),
            "live tree must have owns on every open work ticket; got:\n{}",
            errs.join("\n")
        );

        let (tmp, dir) = scratch_tickets_dir("t912-owns-check");
        let bad = r#"id = "T-001.1"
kind = "work"
title = "x"
summary = "x"
class = "chore"
status = "queued"
order = 10

[scope]
domain = "repo"
layer = "docs"
"#;
        fs::write(dir.join("T-001.1.toml"), bad).unwrap();
        let errs = check_open_work_owns(&tmp);
        assert_eq!(
            errs,
            vec!["T-001.1: owns required for queued work ticket".to_string()],
            "owns-empty queued child must be red"
        );

        let good = bad.replace("order = 10\n", "order = 10\nowns = [\"docs/README.md\"]\n");
        fs::write(dir.join("T-001.1.toml"), good).unwrap();
        assert!(
            check_open_work_owns(&tmp).is_empty(),
            "nonempty owns must restore green"
        );
        fs::remove_dir_all(&tmp).unwrap();
    }

    /// T-917.2: class is required on every work ticket — the live tree is green (the
    /// migrator triaged all of history), and a planted class-less work ticket reds
    /// naming ticket + the legal set; restoring class restores green.
    #[test]
    fn work_without_class_is_red() {
        let root = worktree_root();
        let errs = check_work_class(&root);
        assert!(
            errs.is_empty(),
            "live tree must carry class on every work ticket; got:\n{}",
            errs.join("\n")
        );

        let (tmp, dir) = scratch_tickets_dir("t917-class-check");
        let bare = r#"id = "T-001"
kind = "work"
title = "x"
summary = "x"
status = "idea"

[scope]
domain = "repo"
layer = "docs"
"#;
        fs::write(dir.join("T-001.toml"), bare).unwrap();
        let errs = check_work_class(&tmp);
        assert_eq!(errs.len(), 1, "{errs:?}");
        assert!(
            errs[0].contains("T-001")
                && errs[0].contains("class required")
                && errs[0].contains("bug|feature|chore|audit|docs"),
            "{}",
            errs[0]
        );
        fs::write(
            dir.join("T-001.toml"),
            bare.replace("summary = \"x\"\n", "summary = \"x\"\nclass = \"chore\"\n"),
        )
        .unwrap();
        assert!(check_work_class(&tmp).is_empty(), "class restores green");
        fs::remove_dir_all(&tmp).unwrap();
    }

    /// T-917.2: live work with a component but no surface is red unless the migrator's
    /// `"scope" ∈ estimated[]` escape is recorded; component-free scope is exempt.
    #[test]
    fn live_work_component_without_surface_is_red() {
        let root = worktree_root();
        let errs = check_live_work_surface(&root);
        assert!(
            errs.is_empty(),
            "live tree must satisfy the surface rule; got:\n{}",
            errs.join("\n")
        );

        let (tmp, dir) = scratch_tickets_dir("t917-surface-check");
        let bare = r#"id = "T-001"
kind = "work"
title = "x"
summary = "x"
class = "feature"
status = "queued"
order = 10
owns = ["a.rs"]

[scope]
domain = "website"
layer = "frontend"
component = "mission_creator"
"#;
        fs::write(dir.join("T-001.toml"), bare).unwrap();
        let errs = check_live_work_surface(&tmp);
        assert_eq!(errs.len(), 1, "{errs:?}");
        assert!(
            errs[0].contains("T-001")
                && errs[0].contains("surface required")
                && errs[0].contains("mission_creator"),
            "{}",
            errs[0]
        );
        // The migrator's honest escape…
        fs::write(
            dir.join("T-001.toml"),
            bare.replace(
                "owns = [\"a.rs\"]\n",
                "owns = [\"a.rs\"]\nestimated = [\"scope\"]\n",
            ),
        )
        .unwrap();
        assert!(
            check_live_work_surface(&tmp).is_empty(),
            "scope ∈ estimated must pass"
        );
        // …a real surface…
        fs::write(
            dir.join("T-001.toml"),
            bare.replace(
                "component = \"mission_creator\"\n",
                "component = \"mission_creator\"\nsurface = [\"map_canvas\"]\n",
            ),
        )
        .unwrap();
        assert!(
            check_live_work_surface(&tmp).is_empty(),
            "nonempty surface must pass"
        );
        // …component-free scope…
        fs::write(
            dir.join("T-001.toml"),
            bare.replace(
                "[scope]\ndomain = \"website\"\nlayer = \"frontend\"\ncomponent = \"mission_creator\"\n",
                "[scope]\ndomain = \"repo\"\nlayer = \"docs\"\n",
            ),
        )
        .unwrap();
        assert!(
            check_live_work_surface(&tmp).is_empty(),
            "component-free scope is exempt"
        );
        // …and a component whose vocabulary surface list is EMPTY (mod.scripts.backend
        // — the live T-674.2/T-675.2 shape) are all green: the rule cannot require a
        // surface the vocabulary does not offer.
        fs::write(
            dir.join("T-001.toml"),
            bare.replace(
                "[scope]\ndomain = \"website\"\nlayer = \"frontend\"\ncomponent = \"mission_creator\"\n",
                "[scope]\ndomain = \"mod\"\nlayer = \"scripts\"\ncomponent = \"backend\"\n",
            ),
        )
        .unwrap();
        assert!(
            check_live_work_surface(&tmp).is_empty(),
            "empty vocab surface list is exempt until widened"
        );
        fs::remove_dir_all(&tmp).unwrap();
    }

    /// T-917.3: the body cap rules. The live tree is green (post-quarantine); a
    /// planted 41-word summary reds naming ticket, field, count and cap; a 31-word
    /// context line, a 9-word citation and an owns-duplicating citation each red; a
    /// command-shaped acceptance line WARNS (never errors); nonempty
    /// migration_legacy exempts the summary cap ONLY.
    #[test]
    fn body_caps_red_green_and_warning_channel() {
        let root = worktree_root();
        let errs = check_body_rules(&root);
        assert!(
            errs.is_empty(),
            "live tree must satisfy the body caps; got:\n{}",
            errs.join("\n")
        );

        let (tmp, dir) = scratch_tickets_dir("t917-body-caps");
        let with_summary = |summary: &str, extra: &str| {
            format!(
                "id = \"T-001\"\nkind = \"work\"\ntitle = \"short title\"\nsummary = \"{summary}\"\nclass = \"chore\"\nstatus = \"idea\"\n{extra}\n[scope]\ndomain = \"repo\"\nlayer = \"docs\"\n"
            )
        };
        let wall41 = (1..=41)
            .map(|i| format!("w{i}"))
            .collect::<Vec<_>>()
            .join(" ");
        let load = |tmp: &Path| tbd_tickets::Corpus::load(tmp).expect("scratch corpus loads");

        // 41-word summary → red naming ticket, field, count, cap.
        fs::write(dir.join("T-001.toml"), with_summary(&wall41, "")).unwrap();
        let (errs, warns) = body_findings(&load(&tmp));
        assert_eq!(errs.len(), 1, "{errs:?}");
        assert!(
            errs[0].contains("T-001") && errs[0].contains("summary is 41 words (cap 40)"),
            "{}",
            errs[0]
        );
        assert!(warns.is_empty(), "{warns:?}");

        // Nonempty migration_legacy exempts the summary cap — and ONLY the summary
        // cap: a 31-word context line on the same ticket still reds.
        let line31 = (1..=31)
            .map(|i| format!("c{i}"))
            .collect::<Vec<_>>()
            .join(" ");
        fs::write(
            dir.join("T-001.toml"),
            with_summary(
                &wall41,
                &format!(
                    "migration_legacy = [\"parked wall\"]\ncontext = [\"why now\", \"{line31}\"]\n"
                ),
            ),
        )
        .unwrap();
        let (errs, _) = body_findings(&load(&tmp));
        assert_eq!(
            errs.len(),
            1,
            "summary exempt, context line still red: {errs:?}"
        );
        assert!(
            errs[0].contains("T-001") && errs[0].contains("context[1] is 31 words (cap 30)"),
            "{}",
            errs[0]
        );

        // 9-word citation and an owns-duplicating citation each red.
        fs::write(
            dir.join("T-001.toml"),
            with_summary(
                "fine",
                "owns = [\"docs/README.md\"]\ncitations = [\"one two three four five six seven eight nine\", \"docs/README.md\"]\n",
            ),
        )
        .unwrap();
        let (errs, _) = body_findings(&load(&tmp));
        assert_eq!(errs.len(), 2, "{errs:?}");
        assert!(
            errs.iter()
                .any(|e| e.contains("citations[0] is 9 words (cap 8)")),
            "{errs:?}"
        );
        assert!(
            errs.iter()
                .any(|e| e.contains("citations[1]") && e.contains("duplicates an owns[] entry")),
            "{errs:?}"
        );

        // Command-shaped acceptance → WARNING pointing at verify[], never an error.
        fs::write(
            dir.join("T-001.toml"),
            with_summary(
                "fine",
                "acceptance = [\"cargo xtask ticket check prints check OK\", \"board renders ten fields\"]\n",
            ),
        )
        .unwrap();
        let (errs, warns) = body_findings(&load(&tmp));
        assert!(errs.is_empty(), "warning must not be an error: {errs:?}");
        assert_eq!(warns.len(), 1, "{warns:?}");
        assert!(
            warns[0].contains("T-001")
                && warns[0].contains("acceptance[0]")
                && warns[0].contains("verify[]"),
            "{}",
            warns[0]
        );
        fs::remove_dir_all(&tmp).unwrap();
    }

    /// T-917.3: the quarantine is one-shot history migration — a work ticket carrying
    /// migration_legacy with created_at past the 2026-08-15 cutover is red (new
    /// tickets never quarantine); a pre-cutover stamp (or no stamp) stays green.
    #[test]
    fn quarantine_mint_past_cutover_is_red() {
        let (tmp, dir) = scratch_tickets_dir("t917-quarantine-mint");
        let quarantined = |created: &str| {
            format!(
                "id = \"T-001\"\nkind = \"work\"\ntitle = \"short title\"\nsummary = \"short title\"\nclass = \"chore\"\nstatus = \"idea\"\n{created}migration_legacy = [\"parked wall\"]\n\n[scope]\ndomain = \"repo\"\nlayer = \"docs\"\n"
            )
        };
        fs::write(
            dir.join("T-001.toml"),
            quarantined("created_at = \"2026-08-16T00:00:00Z\"\n"),
        )
        .unwrap();
        let (errs, _) = body_findings(&tbd_tickets::Corpus::load(&tmp).unwrap());
        assert_eq!(errs.len(), 1, "{errs:?}");
        assert!(
            errs[0].contains("T-001")
                && errs[0].contains("2026-08-16T00:00:00Z")
                && errs[0].contains("cutover"),
            "{}",
            errs[0]
        );
        for green in [
            "created_at = \"2026-08-14T23:59:59Z\"\n",
            "", // stampless history is exempt (988 shipped lack created_at until S.4)
        ] {
            fs::write(dir.join("T-001.toml"), quarantined(green)).unwrap();
            let (errs, _) = body_findings(&tbd_tickets::Corpus::load(&tmp).unwrap());
            assert!(errs.is_empty(), "{green:?} must be green: {errs:?}");
        }
        fs::remove_dir_all(&tmp).unwrap();
    }

    /// T-917.4: the estimated[]-vs-field coherence rule. Live tree green; a ticket
    /// listing created_at/completed_at in estimated[] with the field ABSENT is red
    /// naming ticket + field; shipped_at absent+marked is legal ONLY with an
    /// estimate_note naming the gap; present fields restore green.
    #[test]
    fn estimated_marker_without_field_is_red() {
        let root = worktree_root();
        let errs = check_estimated_stamp_coherence(&root);
        assert!(
            errs.is_empty(),
            "live tree must satisfy estimated[] coherence; got:\n{}",
            errs.join("\n")
        );

        let (tmp, dir) = scratch_tickets_dir("t917-estimated-coherence");
        let with = |extra: &str| {
            format!(
                "id = \"T-001\"\nkind = \"work\"\ntitle = \"x\"\nsummary = \"x\"\nclass = \"chore\"\nstatus = \"shipped\"\norder = 10\n{extra}\n[scope]\ndomain = \"repo\"\nlayer = \"docs\"\n"
            )
        };
        // created_at marked but absent → red naming ticket + field.
        fs::write(
            dir.join("T-001.toml"),
            with("estimated = [\"created_at\"]\n"),
        )
        .unwrap();
        let errs = check_estimated_stamp_coherence(&tmp);
        assert_eq!(errs.len(), 1, "{errs:?}");
        assert!(
            errs[0].contains("T-001")
                && errs[0].contains("created_at")
                && errs[0].contains("absent"),
            "{}",
            errs[0]
        );
        // completed_at marked but absent → red.
        fs::write(
            dir.join("T-001.toml"),
            with("estimated = [\"completed_at\"]\n"),
        )
        .unwrap();
        let errs = check_estimated_stamp_coherence(&tmp);
        assert!(
            errs.len() == 1 && errs[0].contains("completed_at"),
            "{errs:?}"
        );
        // shipped_at absent+marked WITHOUT a note → red; WITH the gap named → green.
        fs::write(
            dir.join("T-001.toml"),
            with("estimated = [\"shipped_at\"]\n"),
        )
        .unwrap();
        let errs = check_estimated_stamp_coherence(&tmp);
        assert!(
            errs.len() == 1 && errs[0].contains("shipped_at") && errs[0].contains("estimate_note"),
            "{errs:?}"
        );
        fs::write(
            dir.join("T-001.toml"),
            with("estimated = [\"shipped_at\"]\nestimate_note = \"no subject commits; no SHA mined\"\n"),
        )
        .unwrap();
        assert!(
            check_estimated_stamp_coherence(&tmp).is_empty(),
            "absent-marked shipped_at with the gap named is the legal asymmetry"
        );
        // All three present + marked → green (the backfill's normal output shape).
        fs::write(
            dir.join("T-001.toml"),
            with(
                "shipped_at = \"abcd1234\"\ncreated_at = \"2026-07-01T10:00:00Z\"\ncompleted_at = \"2026-07-02T10:00:00Z\"\nestimated = [\"created_at\", \"completed_at\", \"shipped_at\"]\nestimate_note = \"mined git_subject\"\n",
            ),
        )
        .unwrap();
        assert!(check_estimated_stamp_coherence(&tmp).is_empty());
        fs::remove_dir_all(&tmp).unwrap();
    }

    /// T-916.2: the referential-integrity rule. The live tree must be green (measured: zero
    /// violations, T-111→T-067.1 included — file and parent both exist); a children[] entry
    /// without a file and a child whose parent file is missing must each go red naming BOTH
    /// ids; restoring the files restores green.
    #[test]
    fn children_integrity_red_green() {
        let root = worktree_root();
        let errs = check_children_integrity(&root);
        assert!(
            errs.is_empty(),
            "live tree must be referentially intact; got:\n{}",
            errs.join("\n")
        );

        let (tmp, dir) = scratch_tickets_dir("t916-refint-check");
        let program = r#"id = "T-009"
kind = "program"
title = "x"
summary = "x"
status = "idea"
children = [
    "T-009.1",
    "T-009.2",
]
"#;
        let child = |id: &str, parent: &str| {
            format!(
                "id = \"{id}\"\nkind = \"work\"\ntitle = \"x\"\nsummary = \"x\"\nclass = \"chore\"\nstatus = \"idea\"\nparent = \"{parent}\"\n\n[scope]\ndomain = \"repo\"\nlayer = \"docs\"\n"
            )
        };
        fs::write(dir.join("T-009.toml"), program).unwrap();
        fs::write(dir.join("T-009.1.toml"), child("T-009.1", "T-009")).unwrap();
        // T-009.2 listed but missing on disk → red naming lister and child.
        let errs = check_children_integrity(&tmp);
        assert_eq!(
            errs,
            vec![
                "T-009: children[] names T-009.2, which has no .ai/tickets/T-009.2.toml on disk"
                    .to_string()
            ],
            "dangling children[] entry must be red"
        );
        fs::write(dir.join("T-009.2.toml"), child("T-009.2", "T-009")).unwrap();
        assert!(
            check_children_integrity(&tmp).is_empty(),
            "restored child file must be green"
        );

        // A child whose parent file is absent → red naming child and parent.
        fs::write(dir.join("T-010.4.toml"), child("T-010.4", "T-010")).unwrap();
        let errs = check_children_integrity(&tmp);
        assert_eq!(
            errs,
            vec!["T-010.4: parent T-010 has no .ai/tickets/T-010.toml on disk".to_string()],
            "orphaned child must be red"
        );
        fs::remove_file(dir.join("T-010.4.toml")).unwrap();
        assert!(check_children_integrity(&tmp).is_empty());

        // Fail-closed: an unparseable corpus reports the load error, never a clean scan.
        fs::write(dir.join("T-011.toml"), "id = \"T-011\"\nkind = \"nope\"\n").unwrap();
        let errs = check_children_integrity(&tmp);
        assert_eq!(errs.len(), 1, "one load refusal: {errs:?}");
        assert!(errs[0].contains("T-011"), "must name the file: {}", errs[0]);
        fs::remove_dir_all(&tmp).unwrap();
    }

    /// T-913.1: a malformed lifecycle stamp is a parse error that names the ticket — the
    /// every-file walk (`check_open_work_owns` reuses `parse_ticket_toml`) goes red, and
    /// nothing coerces the value to now. Valid stamps restore green.
    #[test]
    fn malformed_timestamp_is_red() {
        let (tmp, dir) = scratch_tickets_dir("t913-timestamp-check");
        let bad = r#"id = "T-001.1"
kind = "work"
title = "x"
summary = "x"
class = "chore"
status = "queued"
order = 10
created_at = "2026-13-99T25:61:00Z"
owns = ["docs/README.md"]

[scope]
domain = "repo"
layer = "docs"
"#;
        fs::write(dir.join("T-001.1.toml"), bad).unwrap();
        let errs = check_open_work_owns(&tmp);
        assert_eq!(errs.len(), 1, "exactly one parse error: {errs:?}");
        assert!(
            errs[0].contains("T-001.1") && errs[0].contains("created_at"),
            "error must name ticket and field: {}",
            errs[0]
        );

        let naive = bad.replace("2026-13-99T25:61:00Z", "2026-08-14 10:00");
        fs::write(dir.join("T-001.1.toml"), naive).unwrap();
        let errs = check_open_work_owns(&tmp);
        assert!(
            errs.len() == 1 && errs[0].contains("created_at"),
            "naive datetime must be red: {errs:?}"
        );

        let good = bad.replace("2026-13-99T25:61:00Z", "2026-08-14T10:00:00Z");
        fs::write(dir.join("T-001.1.toml"), good).unwrap();
        assert!(
            check_open_work_owns(&tmp).is_empty(),
            "valid RFC 3339 UTC must restore green"
        );
        fs::remove_dir_all(&tmp).unwrap();
    }

    #[test]
    fn require_check_ok_blocks_invalid_registry() {
        let root = worktree_root();
        let mut registry = load_registry(&root).expect("load tip registry");
        registry
            .get_mut("tickets")
            .and_then(|t| t.as_array_mut())
            .expect("tickets")
            .first_mut()
            .expect("ticket")
            .as_object_mut()
            .expect("obj")
            .insert("status".into(), json!("not-a-real-status"));
        let errs = check(&root, &registry, false);
        assert!(
            !errs.is_empty(),
            "invalid status must fail check (ship/set-status preflight relies on this)"
        );
        assert!(
            errs.iter().any(|e| e.contains("schema")),
            "expected schema error for bogus status: {errs:?}"
        );
        let refuse = require_check_ok(&root, &registry, "set-status T-001");
        assert!(
            refuse.is_err(),
            "require_check_ok must Err on red registry (T-451)"
        );
        let msg = format!("{:#}", refuse.unwrap_err());
        assert!(
            msg.contains("refusing set-status T-001"),
            "refuse message missing: {msg}"
        );
    }
}
