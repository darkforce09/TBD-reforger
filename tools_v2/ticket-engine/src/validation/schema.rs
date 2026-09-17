//! Schema.

use super::*;

pub(super) static STRICT_RE: LazyLock<Regex> = LazyLock::new(|| Regex::new(STRICT_LEGACY).unwrap());

pub(super) static PRIORITY_P: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"(?m)^\d+\.\s+\*\*P[0-3]").unwrap());

/// Cap schema-error spam so a broken registry still yields an actionable first page.
pub(super) const SCHEMA_ERROR_CAP: usize = 100;

pub(super) fn ticket_schema_path(root: &Path) -> PathBuf {
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

pub(super) fn validate_row(row: &serde_json::Value) -> Vec<String> {
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
        if opt_str(row, "main_goal").unwrap_or("").trim().is_empty() {
            errors.push(format!("{tid}: ready-class requires main_goal"));
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
    if let Some(id) = opt_str(row, "id")
        && FORBIDDEN_PHANTOM_IDS.contains(&id)
    {
        errors.push(format!("Forbidden phantom id {id}"));
    }
    errors
}

pub(super) fn validate_registry(registry: &serde_json::Value) -> Vec<String> {
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
        if matches!(status, "queued" | "ready" | "running" | "review")
            && let Some(order) = row.get("order").and_then(Value::as_i64)
            && let Some(other) = live_orders.insert(order, tid.clone())
        {
            errors.push(format!("duplicate live order {order} on {other} and {tid}"));
        }
    }
    errors
}
