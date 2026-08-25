//! T-439 — the Objects-palette alias ↔ spawn-registry census (T-853 port of
//! `scripts/mod/verify-t439-objects-registry-aliases.sh`).
//!
//! ── WHAT THE GATE IS FOR ─────────────────────────────────────────────────────────────────────
//!
//! The 2D editor's Objects palette writes an alias into the mission payload; the mod's
//! `SpawnMissionEntities` looks it up in `apps/mod/tbd-framework/Data/registry.json` for the
//! prefab GUID to spawn. Nothing in the type system joins the two ends — the alias is *derived*
//! on the web side by `asset_catalog.rs::derive_object_alias` and *consumed by name* in Enfusion
//! — so only a census can hold the pairing, and breaking it costs the author a crate that never
//! appears: the spawner warn-skips a leaf the palette had offered, silently. So for every
//! Objects-eligible workbench kind this derives the alias the frontend would, and requires a
//! `prop:`/`comp:` row whose `guid` is exactly the workbench `resource_name`. Eligibility comes
//! from `packages/tbd-schema/registry/registry-items.workbench.json` (`kind` in {crate, other},
//! non-abstract) — the export the API imports, so no live Workbench needed.
//!
//! ── THE DERIVATION IS A MIRROR, AND MIRRORS DRIFT ────────────────────────────────────────────
//!
//! [`derive_object_alias`] and [`object_alias_slug`] hand-copy `asset_catalog.rs` with no compiler
//! joining the copies, deliberately: the point is to *independently* recompute what the frontend
//! computes, and importing `website-frontend` would make the gate agree by construction and check
//! nothing. The script had the same copy in Python for the same reason, guarded by the two
//! `grep -q` pins this port keeps — so the derivation cannot be renamed away while the mirror
//! here still describes the old one.
//!
//! ── WHAT THE PORT FIXES ──────────────────────────────────────────────────────────────────────
//!
//! 1. **The Python interpreter is gone.** The script was a 90-line `python3 - <<'PY'` heredoc and
//!    is line 34 of `scripts/python-inventory.txt` — the T-620 frozen debt list, whose header
//!    records that the gate meant to enforce it had itself been greping with `rg` (installed
//!    nowhere here) under `|| true`, so 127 read as "OK (none)" for four waves. Nothing is left
//!    here whose absence could exit this gate 127.
//! 2. **A structurally wrong registry is a verdict, not a traceback.** `wb["items"]`, `e["alias"]`
//!    and `i["resource_name"]` were bare subscripts; a renamed key produced a `KeyError` trace.
//!    Each is a [`NotRun`] — "could not run" is a different operator action from "found a
//!    mismatch".
//! 3. **`grep -q` misdiagnosis.** `if ! grep -q 'pub fn derive_object_alias' "$FE"` reads "pin
//!    absent" (1), "file unreadable" (2) and "grep not installed" (127) as one sentence: a true
//!    failure with a false cause, sending the next reader off to rewrite a function nobody
//!    touched. [`gate::require`] separates them.
//!
//! Preserved because the script got it right: `set -euo pipefail`, no `2>/dev/null`, no `|| true`
//! — one of the few `scripts/mod/` gates not born with the fail-open shape.
//!
//! ── OUTPUT IS A CONTRACT ─────────────────────────────────────────────────────────────────────
//!
//! `wave.sh:2555` and `:2826` run the script in the slice and cold gates and `tail -15` it on
//! failure, and T-853 accepts ports by diffing stdout. Two copied details look like mistakes and
//! are not: the **two-space sample indent** (Python's `print("  sample:", …)`, where [`Finding`]
//! would indent six, so those lines print raw), and **Python `repr()` of the sample lists** —
//! `['prop:x']`, tuples in parens, `None` for an absent `guid`, all in [`py_repr_value`]. A nicer
//! format would be a diff. Two deviations are deliberate:
//!
//! * **Exit 2, not 1, when the check did not run.** bash exits 1 both for "the registry is
//!   missing" and for "the registry disagrees". Both `run()` helpers in `wave.sh` test `rc -eq 0`
//!   / `rc -eq 124`, so any nonzero is still FAIL there — the resolution is free and a human can
//!   see it. The bash headline stays verbatim on line 1 so a grep for `FAIL: missing …` still hits.
//! * **`\A`/`\z` anchors instead of `^`/`$`.** [`Pattern`] forces `multi_line(true)` so ported
//!   `grep` patterns keep line anchors — right for file scans, wrong for one token, where `^…$`
//!   would let `"{DEADBEEF…}ok\nGARBAGE"` through. `\A…\z` is `re.fullmatch`, i.e. what
//!   `re.match(…$)` means for a newline-free subject. Python's `$` also tolerates ONE trailing
//!   newline; measured 2026-08-12 no eligible item contains one, so that gap is unreachable.
//!
//! No [`tbd_gate::Report`]: it prints every failure and the script does not — each check ends in
//! `sys.exit(1)`, load-bearing because once the census count drifts the alias diffs below it are
//! noise. Fail-fast order kept; the summary bash never printed stays out.

use std::collections::HashMap;
use std::io;
use std::path::Path;

use anyhow::Result;
use serde_json::Value;
use tbd_gate::{Finding, Kind, NotRun, Pattern, Verdict, gate};

/// The Objects-eligible census — the same export the API imports, so no live Workbench needed.
const WB_REL: &str = "packages/tbd-schema/registry/registry-items.workbench.json";
/// What `SpawnMissionEntities` actually reads at mission load.
const MOD_REL: &str = "apps/mod/tbd-framework/Data/registry.json";
/// The frontend derivation this file mirrors; pinned by two `require`s, never parsed.
const FE_REL: &str = "apps/website/frontend/src/editor/arsenal/asset_catalog.rs";
/// Kinds the Objects palette offers. Anything else is a character, vehicle or gear item, and
/// belongs to a different palette with a different alias namespace.
const OBJECT_KINDS: &[&str] = &["crate", "other"];
// Hard floors measured 2026-07-27: 333 Objects-eligible, 289 prop + 45 comp (incl. the POC
// checkpoint). `eligible` is an EQUALITY, not a floor: the count moving in either direction means
// the workbench census was re-exported, and every alias below it is then computed from a set
// nobody has reviewed. Re-measured 2026-08-12, unchanged.
const ELIGIBLE_EXACT: usize = 333;
const PROP_FLOOR: usize = 289;
const COMP_FLOOR: usize = 45;
/// The one composition wired end-to-end by hand, and the reverse-hit that proves the KNOWN table
/// in `derive_object_alias` is still honoured on both sides.
const POC_ALIAS: &str = "comp:checkpoint_small";
/// The single KNOWN reverse-hit, byte-for-byte from `asset_catalog.rs`. Without it this prefab
/// derives `comp:e_sandbag_barricade_us_04` from its display name, which is not the alias the mod
/// ships — the POC row was hand-authored before the derivation existed.
const KNOWN_CHECKPOINT_GUID: &str = "{E1D01D77D7F47EF3}PrefabsEditable/Auto/Compositions/Misc/SubCompositions/E_Sandbag_Barricade_US_04.et";
/// Enfusion ResourceName shape: `{16 uppercase hex}` then a prefab path. Module docs explain why
/// the anchors are `\A`/`\z` and not the script's `^`/`$`.
const GUID_RE: &str = r"\A\{[0-9A-F]{16}\}[A-Za-z0-9/_.\-]+\z";
/// `#/$defs/alias` shape from `packages/tbd-schema`. All seven namespaces are listed even though
/// only `prop:`/`comp:` can be produced here — the script validated against the schema's full
/// alternation, and narrowing it would be a behaviour change smuggled into a port.
const ALIAS_RE: &str = r"\A(kit|comp|veh|preset|layer|prop|item):[a-z0-9_]+\z";
/// The two `grep -q` pins on the frontend mirror, in the script's order.
#[rustfmt::skip]
const FE_PINS: &[(&str, &str)] = &[
    ("derive_object_alias missing from asset_catalog.rs", "pub fn derive_object_alias"),
    ("KNOWN comp:checkpoint_small reverse-hit missing from asset_catalog.rs", POC_ALIAS),
];

pub fn verify_t439(repo_root: &Path) -> Result<u8> {
    let wb_path = repo_root.join(WB_REL);
    let mod_path = repo_root.join(MOD_REL);
    let fe_path = repo_root.join(FE_REL);

    // bash: `for f in "$WB" "$MOD" "$FE"; do [[ -f "$f" ]] || { echo "FAIL: missing $f"; exit 1; }`
    // Order and absolute paths preserved: `$ROOT` was absolute, so the printed path was too.
    for f in [&wb_path, &mod_path, &fe_path] {
        if !f.is_file() {
            return Ok(emit(missing_target(f)));
        }
    }

    // The mirror guard: not that the frontend derivation is *correct*, but that it has not been
    // renamed or deleted out from under the copy in this file.
    let fe = [fe_path.as_path()];
    for (msg, needle) in FE_PINS {
        let code = emit(gate::require(msg, &Pattern::literal(needle), &fe));
        if code != 0 {
            return Ok(code);
        }
    }

    match census(&wb_path, &mod_path) {
        Ok(code) => Ok(code),
        Err(verdict) => Ok(emit(verdict)),
    }
}

/// The Python half of the script. `Err` always means "the census could not run".
fn census(wb_path: &Path, mod_path: &Path) -> Result<u8, Verdict> {
    let guid_re = compile(GUID_RE)?;
    let alias_re = compile(ALIAS_RE)?;

    let wb = load_json(wb_path)?;
    let md = load_json(mod_path)?;
    let items = json_array(&wb, wb_path, "items")?;
    let entries = json_array(&md, mod_path, "entries")?;

    // python: `i.get("kind") in ("crate","other") and not i.get("abstract")` — `.get` on both, so
    // a missing key is a miss, not an error. Measured 2026-08-12 `abstract` is only ever absent
    // (1511) or `true` (346), never `false`; [`is_falsy`] still reproduces Python's full rule.
    let eligible: Vec<&Value> = items
        .iter()
        .filter(|i| {
            let kind = i.get("kind").and_then(Value::as_str);
            kind.is_some_and(|k| OBJECT_KINDS.contains(&k)) && is_falsy(i.get("abstract"))
        })
        .collect();

    // python: `{e["alias"]: e for e in mod["entries"]}` — a LATER duplicate alias overwrites an
    // earlier one and the counts below are over the deduplicated keys. Zero duplicates shipped
    // (2026-08-12), but `insert` keeps the script's rule for the day there is one.
    let mut by_alias: HashMap<&str, &Value> = HashMap::new();
    for (idx, e) in entries.iter().enumerate() {
        let alias = text_field(e, "alias", mod_path, &format!("entries[{idx}]"))?;
        by_alias.insert(alias, e);
    }
    let prop_n = by_alias.keys().filter(|a| a.starts_with("prop:")).count();
    let comp_n = by_alias.keys().filter(|a| a.starts_with("comp:")).count();

    let mut missing: Vec<String> = Vec::new();
    let mut guid_mismatch: Vec<(String, Option<Value>, String)> = Vec::new();
    let mut bad_shape: Vec<(&'static str, String, String)> = Vec::new();

    for (idx, i) in eligible.iter().enumerate() {
        let ctx = format!("items[{idx}] (Objects-eligible)");
        let resource_name = text_field(i, "resource_name", wb_path, &ctx)?;
        let display_name = text_field(i, "display_name", wb_path, &ctx)?;
        let alias = derive_object_alias(resource_name, display_name);

        // Shape violations are recorded and the item is STILL looked up — the script did not
        // `continue` here, so a malformed alias also counts toward `missing`. Preserved.
        if !shape_ok(&alias_re, &alias) {
            bad_shape.push(("alias", alias.clone(), display_name.to_string()));
        }
        if !shape_ok(&guid_re, resource_name) {
            bad_shape.push(("guid", resource_name.to_string(), display_name.to_string()));
        }

        let Some(ent) = by_alias.get(alias.as_str()) else {
            missing.push(alias);
            continue;
        };
        // python: `ent.get("guid") != i["resource_name"]`. Absent, null and non-string all
        // compare unequal to a str, so all three are mismatches, rendered `None` or their repr.
        if ent.get("guid").and_then(Value::as_str) != Some(resource_name) {
            guid_mismatch.push((alias, ent.get("guid").cloned(), resource_name.to_string()));
        }
    }

    // Fail-fast, in the script's order. See the module docs on why this is not a `Report`.
    let n = eligible.len();
    if n != ELIGIBLE_EXACT {
        let m = format!("Objects-eligible count {n} != {ELIGIBLE_EXACT} (workbench census drift)");
        return Ok(fail(m));
    }
    if prop_n < PROP_FLOOR {
        return Ok(fail(format!("prop: rows {prop_n} < {PROP_FLOOR}")));
    }
    if comp_n < COMP_FLOOR {
        return Ok(fail(format!("comp: rows {comp_n} < {COMP_FLOOR}")));
    }
    if !by_alias.contains_key(POC_ALIAS) {
        return Ok(fail(format!("POC {POC_ALIAS} missing from mod registry")));
    }
    if !missing.is_empty() {
        let k = missing.len();
        let head = format!("{k} Objects-eligible aliases missing from mod registry");
        return Ok(fail_with_sample(
            &head,
            &py_sample(&missing, 10, |a| py_repr_str(a)),
        ));
    }
    if !guid_mismatch.is_empty() {
        let head = format!("{} alias guid mismatches", guid_mismatch.len());
        let sample = py_sample(&guid_mismatch, 5, |(alias, got, want)| {
            let got = py_repr_value(got.as_ref());
            py_tuple(&[py_repr_str(alias), got, py_repr_str(want)])
        });
        return Ok(fail_with_sample(&head, &sample));
    }
    if !bad_shape.is_empty() {
        let head = format!("{} schema-shape violations", bad_shape.len());
        let sample = py_sample(&bad_shape, 5, |(kind, val, disp)| {
            py_tuple(&[py_repr_str(kind), py_repr_str(val), py_repr_str(disp)])
        });
        return Ok(fail_with_sample(&head, &sample));
    }

    let pass = format!("prop={prop_n} comp={comp_n} missing=0 guid_mismatch=0");
    println!("PASS: T-439 Objects aliases — eligible={n} {pass}");
    Ok(0)
}

/// Print a verdict, yield its code: [`tbd_gate::Report::check`] minus the summary bash never
/// printed. `Held` prints nothing, exactly as a passing `grep -q` printed nothing.
fn emit(verdict: Verdict) -> u8 {
    let (code, finding) = match verdict {
        Verdict::Held => return 0,
        Verdict::Failed(f) => (1, f),
        Verdict::DidNotRun(_, f) => (2, f),
    };
    println!("{finding}");
    code
}

/// python: `print(f"FAIL: …"); sys.exit(1)`.
fn fail(headline: String) -> u8 {
    emit(Verdict::failed(headline))
}
/// The same, plus Python's two-space `  sample: […]` — `print("  sample:", …)`, not [`Finding`]'s
/// six-space continuation, so line 2 bypasses the renderer and is printed raw.
fn fail_with_sample(headline: &str, sample: &str) -> u8 {
    let code = emit(Verdict::failed(headline));
    println!("  sample: {sample}");
    code
}

/// bash's `echo "FAIL: missing $f"; exit 1`, with the refusal underneath. Hand-built rather than
/// [`Verdict::did_not_run`], which would append ` — target file missing: <path>` to a headline
/// that already names it; the detail keeps the library's `TargetMissing` wording all the same.
fn missing_target(path: &Path) -> Verdict {
    let why = "The pin could not run. A moved or deleted file must not read as a clean result.";
    let headline = format!("missing {}", path.display());
    let detail = vec![why.to_string()];
    Verdict::DidNotRun(
        NotRun::TargetMissing(path.to_path_buf()),
        Finding { headline, detail },
    )
}

/// A malformed pattern constant is a bug in THIS file, and it must not read as "shape OK".
fn compile(src: &str) -> Result<Pattern, Verdict> {
    Pattern::regex(src).map_err(|e| {
        let stderr = format!("{src}: {e}");
        let cause = NotRun::ToolError {
            tool: "regex".into(),
            status: 1,
            stderr,
        };
        Verdict::did_not_run("T-439 pattern would not compile", Kind::Pin, cause)
    })
}

/// Does `subject` have the required shape? [`gate::probe_str`] is infallible today (the subject is
/// in hand) and returns `Result` only so a caller can move to `probe_files` without restructuring.
/// Should that change, a non-verdict must land on the failing side — for a shape PIN, `false`.
fn shape_ok(pattern: &Pattern, subject: &str) -> bool {
    gate::probe_str(pattern, subject).unwrap_or(false)
}

// Every structural surprise below is a DidNotRun, never a traceback.
fn unread(path: &Path, source: io::Error) -> Verdict {
    let msg = "the T-439 alias census could not read its input";
    let path = path.to_path_buf();
    Verdict::did_not_run(msg, Kind::Pin, NotRun::Unreadable { path, source })
}
fn malformed(path: &Path, why: String) -> Verdict {
    unread(path, io::Error::new(io::ErrorKind::InvalidData, why))
}
fn load_json(path: &Path) -> Result<Value, Verdict> {
    let text = std::fs::read_to_string(path).map_err(|e| unread(path, e))?;
    // `serde_json` (with `preserve_order`) rather than the script's `json.loads` — and rather than
    // regexing JSON, which would make an alias inside a comment or a string literal indexable.
    serde_json::from_str(&text).map_err(|e| malformed(path, format!("invalid JSON: {e}")))
}

fn json_array<'a>(doc: &'a Value, path: &Path, field: &str) -> Result<&'a Vec<Value>, Verdict> {
    let why = || malformed(path, format!("`{field}` is missing or not an array"));
    doc.get(field).and_then(Value::as_array).ok_or_else(why)
}
fn text_field<'a>(obj: &'a Value, field: &str, path: &Path, ctx: &str) -> Result<&'a str, Verdict> {
    obj.get(field)
        .and_then(Value::as_str)
        .ok_or_else(|| malformed(path, format!("{ctx}: `{field}` is missing or not a string")))
}

/// Python truthiness, for `not i.get("abstract")`.
fn is_falsy(v: Option<&Value>) -> bool {
    match v {
        None | Some(Value::Null) => true,
        Some(Value::Bool(b)) => !b,
        Some(Value::Number(n)) => n.as_f64() == Some(0.0),
        Some(Value::String(s)) => s.is_empty(),
        Some(Value::Array(a)) => a.is_empty(),
        Some(Value::Object(o)) => o.is_empty(),
    }
}

// The frontend mirror — duplicated ON PURPOSE; see the module docs.
/// Mirror of `asset_catalog.rs::derive_object_alias`.
fn derive_object_alias(resource_name: &str, display_name: &str) -> String {
    if resource_name == KNOWN_CHECKPOINT_GUID {
        return POC_ALIAS.to_string();
    }
    // `|| contains("Compositions")` is redundant — "Compositions" contains "Composition" — and is
    // kept because both the frontend and the script spell it this way. A port is not the place to
    // tidy a mirror; that has to happen on the frontend side first or the two stop matching.
    let comp = resource_name.contains("Composition") || resource_name.contains("Compositions");
    let prefix = if comp { "comp" } else { "prop" };
    format!("{prefix}:{}", object_alias_slug(display_name))
}

/// Mirror of `asset_catalog.rs::object_alias_slug`: lowercase, keep `[a-z0-9]`, collapse every
/// other run to a single `_`, trim `_` from both ends, fall back to `object` if nothing survives.
fn object_alias_slug(raw: &str) -> String {
    let mut out = String::with_capacity(raw.len());
    let mut prev_repl = false;
    for c in raw.to_lowercase().chars() {
        // python: `c.isascii() and (c.islower() or c.isdigit())` — for ASCII exactly `[a-z0-9]`,
        // which is what the frontend writes directly.
        if c.is_ascii_lowercase() || c.is_ascii_digit() {
            out.push(c);
            prev_repl = false;
        } else if !prev_repl {
            out.push('_');
            prev_repl = true;
        }
    }
    match out.trim_matches('_') {
        "" => "object".to_string(),
        trimmed => trimmed.to_string(),
    }
}

// Python `repr()` below, because the sample lines are a diffed contract.
/// `repr()` of a Python 3 `str`. Quote selection is Python's: `'` unless the value contains `'`
/// and no `"`. Escapes cover `\`, the active quote, `\t`/`\n`/`\r` and the ASCII control range.
/// Python also escapes non-ASCII *non-printables* (Cc/Cf/Cs/Co/Cn/Zl/Zp/Zs) as `\uXXXX`; measured
/// 2026-08-12 all 333 eligible display and resource names are printable ASCII, so that branch is
/// unreachable on real data and is left out rather than implemented wrong.
fn py_repr_str(s: &str) -> String {
    let needs_dq = s.contains('\'') && !s.contains('"');
    let quote = if needs_dq { '"' } else { '\'' };
    let mut out = String::with_capacity(s.len() + 2);
    out.push(quote);
    for c in s.chars() {
        match c {
            '\\' => out.push_str("\\\\"),
            '\t' => out.push_str("\\t"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            c if c == quote => {
                out.push('\\');
                out.push(c);
            }
            c if (c as u32) < 0x20 || c as u32 == 0x7f => {
                out.push_str(&format!("\\x{:02x}", c as u32));
            }
            c => out.push(c),
        }
    }
    out.push(quote);
    out
}

/// `repr()` of a decoded JSON value, for the `ent.get("guid")` slot.
fn py_repr_value(v: Option<&Value>) -> String {
    match v {
        // Both an absent key and an explicit `null` reach Python's `.get` as `None`.
        None | Some(Value::Null) => "None".to_string(),
        Some(Value::Bool(true)) => "True".to_string(),
        Some(Value::Bool(false)) => "False".to_string(),
        Some(Value::Number(n)) => n.to_string(),
        Some(Value::String(s)) => py_repr_str(s),
        // A list/dict here is corruption no schema permits; the JSON form is close enough to
        // Python's nested repr and, unlike a panic, still lets the FAIL land.
        Some(other) => other.to_string(),
    }
}

/// python: `print("  sample:", xs[:n])` — the repr of a truncated list.
fn py_sample<T>(items: &[T], n: usize, render: impl Fn(&T) -> String) -> String {
    let rendered: Vec<String> = items.iter().take(n).map(render).collect();
    format!("[{}]", rendered.join(", "))
}
fn py_tuple(items: &[String]) -> String {
    format!("({})", items.join(", "))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    /// `xtask/` -> repo root. The gate's whole job is the committed data, so the real tree is the
    /// fixture; no synthetic input reaches the alias checks past the `eligible == 333` equality.
    fn repo() -> PathBuf {
        let here = Path::new(env!("CARGO_MANIFEST_DIR"));
        here.parent().expect("xtask has a parent").to_path_buf()
    }

    /// A scratch repo root: real WB/MOD/FE, with the mod registry optionally rewritten.
    struct Fixture(PathBuf);
    impl Fixture {
        fn new(name: &str, mod_json: Option<String>) -> Fixture {
            let root = std::env::temp_dir().join(format!("tbd-t439-{}-{name}", std::process::id()));
            let _ = std::fs::remove_dir_all(&root);
            for rel in [WB_REL, MOD_REL, FE_REL] {
                let dst = root.join(rel);
                std::fs::create_dir_all(dst.parent().unwrap()).unwrap();
                std::fs::copy(repo().join(rel), &dst).unwrap();
            }
            if let Some(body) = mod_json {
                std::fs::write(root.join(MOD_REL), body).unwrap();
            }
            Fixture(root)
        }
        fn drop_file(&self, rel: &str) {
            std::fs::remove_file(self.0.join(rel)).unwrap();
        }
    }
    impl Drop for Fixture {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }

    /// One perturbation of the shipped registry's `entries` array.
    type Edit = fn(&mut Vec<Value>);
    /// Mutate the shipped registry's entries with `f`, then run the gate over the result.
    fn perturb(name: &str, f: Edit) -> u8 {
        let raw = std::fs::read_to_string(repo().join(MOD_REL)).unwrap();
        let mut doc: Value = serde_json::from_str(&raw).unwrap();
        f(doc["entries"].as_array_mut().unwrap());
        let fx = Fixture::new(name, Some(doc.to_string()));
        verify_t439(&fx.0).unwrap()
    }

    fn row(es: &[Value], pred: impl Fn(&str) -> bool) -> usize {
        let hit = es
            .iter()
            .position(|e| e["alias"].as_str().is_some_and(&pred));
        hit.expect("the shipped registry has the row this perturbation edits")
    }

    #[test]
    fn the_real_registry_holds() {
        assert_eq!(verify_t439(&repo()).unwrap(), 0);
    }

    /// T-556 anti-vacuity: a gate that cannot fail is indistinguishable from one that checks
    /// nothing, and this one prints a single PASS line on a clean tree. Each case aims at a
    /// different one of the script's ordered checks and must turn that PASS into a 1.
    #[test]
    fn perturbing_the_registry_turns_the_pass_red() {
        let cases: [(&str, Edit); 4] = [
            // guid_mismatch — the row exists and points at the wrong prefab.
            ("guid", |es| {
                let i = row(es, |a| a.starts_with("prop:"));
                es[i]["guid"] = Value::String("{0000000000000000}Prefabs/Wrong.et".into());
            }),
            // missing — RENAMED not deleted, so prop: stays at its floor and this reaches the
            // alias lookup instead of tripping the count check above it.
            ("rename", |es| {
                let i = row(es, |a| a.starts_with("prop:"));
                let to = format!("{}_zz", es[i]["alias"].as_str().unwrap());
                es[i]["alias"] = Value::String(to);
            }),
            // the prop floor — one row fewer than the 2026-07-27 measurement.
            ("delete", |es| {
                let i = row(es, |a| a.starts_with("prop:"));
                es.remove(i);
            }),
            // the POC check — retargeted, so comp: still meets its floor.
            ("poc", |es| {
                let i = row(es, |a| a == POC_ALIAS);
                es[i]["alias"] = Value::String("comp:not_the_poc".into());
            }),
        ];
        for (name, f) in cases {
            assert_eq!(perturb(name, f), 1, "perturbation `{name}` went green");
        }
        // The frontend mirror pins are violations (1) too, not did-not-runs.
        let gutted = Fixture::new("gutfe", None);
        std::fs::write(gutted.0.join(FE_REL), "// nothing to see here\n").unwrap();
        assert_eq!(
            verify_t439(&gutted.0).unwrap(),
            1,
            "a gutted mirror went green"
        );
    }

    /// THE DEFECT THE CRATE EXISTS FOR. Inputs nobody read are not clean inputs — and these are 2,
    /// not the 1 a real drift returns, so CI can tell a broken checkout from a broken registry.
    #[test]
    fn inputs_that_were_never_examined_do_not_read_as_pass() {
        let absent = Fixture::new("absent", None);
        absent.drop_file(MOD_REL);
        assert_eq!(verify_t439(&absent.0).unwrap(), 2, "absent registry");
        let no_fe = Fixture::new("nofe", None);
        no_fe.drop_file(FE_REL);
        assert_eq!(verify_t439(&no_fe.0).unwrap(), 2, "absent frontend mirror");
        let garbage = Fixture::new("garbage", Some("{ this is not json".into()));
        assert_eq!(verify_t439(&garbage.0).unwrap(), 2, "unparseable registry");
        // The Python `mod["entries"]` KeyError path, now a verdict rather than a stack trace.
        let bare = Fixture::new("noentries", Some(r#"{"registryVersion": 1}"#.into()));
        assert_eq!(verify_t439(&bare.0).unwrap(), 2, "no entries array");
    }

    #[test]
    fn the_mirror_matches_the_frontend() {
        assert_eq!(object_alias_slug("Ammo Box US 01"), "ammo_box_us_01");
        assert_eq!(object_alias_slug("  -- Weird -- "), "weird");
        assert_eq!(object_alias_slug("!!!"), "object", "empty falls back");
        let known = derive_object_alias(KNOWN_CHECKPOINT_GUID, "E Sandbag Barricade US 04");
        assert_eq!(known, POC_ALIAS, "the KNOWN reverse-hit beats the slug");
        let path = "{A}PrefabsEditable/Compositions/X.et";
        assert_eq!(derive_object_alias(path, "Fuel Depot"), "comp:fuel_depot");
        let p2 = "{A}Prefabs/P.et";
        assert_eq!(derive_object_alias(p2, "Fuel Depot"), "prop:fuel_depot");
    }

    #[test]
    fn shape_pins_reject_what_python_rejected() {
        let guid = compile(GUID_RE).unwrap();
        let alias = compile(ALIAS_RE).unwrap();
        assert!(shape_ok(&guid, "{0123456789ABCDEF}Prefabs/A-b_c.et"));
        assert!(!shape_ok(&guid, "{0123456789abcdef}P.et"), "lowercase hex");
        // The `^`/`$` trap: line anchors would let the second line smuggle anything through.
        assert!(!shape_ok(&guid, "{0123456789ABCDEF}Prefabs/A.et\nGARBAGE"));
        assert!(shape_ok(&alias, "prop:ammo_box_us_01"));
        assert!(!shape_ok(&alias, "prop:Ammo"), "uppercase");
        assert!(!shape_ok(&alias, "thing:x"), "namespace not in $defs");
    }

    /// The Python behaviours the sample lines and the eligible filter depend on.
    #[test]
    fn python_semantics_are_reproduced() {
        assert_eq!(py_repr_str("prop:x"), "'prop:x'");
        assert_eq!(py_repr_str("it's"), "\"it's\"");
        assert_eq!(py_repr_str("both ' and \""), "'both \\' and \"'");
        assert_eq!(py_repr_str("a\\b\nc"), "'a\\\\b\\nc'");
        assert_eq!(py_repr_value(None), "None", "an absent guid renders None");
        assert_eq!(
            py_sample(&["a", "b", "c"], 2, |s| py_repr_str(s)),
            "['a', 'b']"
        );
        let tup = py_tuple(&[py_repr_str("alias"), py_repr_value(None), py_repr_str("x")]);
        assert_eq!(tup, "('alias', None, 'x')");
        assert!(is_falsy(None), "an absent `abstract` is falsy");
        assert!(is_falsy(Some(&Value::Bool(false))));
        assert!(is_falsy(Some(&serde_json::json!(0))));
        assert!(!is_falsy(Some(&Value::Bool(true))));
    }
}
