//! T-181.2 — Enfusion `.c` symbol scanner.
//!
//! Produces provable `symbol -> file:line` rows for the CRF oracle and (T-181.3) the carved
//! vanilla tree. Both lanes share this one scanner so there is a single correctness bar.
//!
//! WHY THIS IS MECHANICAL AND NOT AN LLM SUMMARY
//! ---------------------------------------------
//! An agent asked to summarise `CRF_SlottingManager.c` produced four APIs that do not exist
//! (`RequestSlotChange`, `ReleaseSlot`, `GetInstance`, and a wrong base class) and missed
//! `RplSave`/`RplLoad` entirely. Line numbers in prose are therefore never trustworthy.
//! Everything in `docs/mod/oracle/**` cites rows emitted here, and `verify-oracle` fails the
//! build when a citation does not resolve.
//!
//! SCOPE — this is a deliberate line/brace scanner, not a full Enfusion parser.
//! It is accurate for the shapes Enfusion actually uses (declarations and members live on
//! their own lines) and it never guesses: anything it cannot classify is simply not emitted.
//! Known limits, stated rather than hidden:
//!   * declarations split across lines are missed,
//!   * `//`-commented and `/* */` bodies are skipped only for brace counting on `//` lines,
//!   * a method-like line inside a nested block at depth > 1 is not emitted.

use std::path::Path;

/// What kind of declaration a row describes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    Class,
    ModdedClass,
    SealedClass,
    Enum,
    Method,
}

impl Kind {
    pub fn as_str(self) -> &'static str {
        match self {
            Kind::Class => "class",
            Kind::ModdedClass => "modded_class",
            Kind::SealedClass => "sealed_class",
            Kind::Enum => "enum",
            Kind::Method => "method",
        }
    }
}

/// One declaration, with the coordinates a citation resolves against.
#[derive(Debug, Clone)]
pub struct Symbol {
    pub name: String,
    pub kind: Kind,
    /// Repo-relative (or carve-relative) path — the citation key.
    pub file: String,
    /// 1-based line of the declaration.
    pub line: usize,
    pub end_line: usize,
    /// Base class after `:`, empty when none.
    pub base: String,
    /// Owning class for methods, empty for top-level declarations.
    pub parent: String,
}

/// A replicated property — the surface that governs authority/proxy behaviour.
///
/// Worth its own table because of a landmine CRF documents in `CRF_Gamemode.c:8`:
/// an `onRplName` callback fires automatically only on the PROXY, so authority must invoke
/// its own handler. Knowing every `[RplProp]` up front is how that stops being relearned.
#[derive(Debug, Clone)]
pub struct RplProp {
    pub class: String,
    pub prop: String,
    pub on_rpl_name: String,
    pub file: String,
    pub line: usize,
}

#[derive(Debug, Default)]
pub struct FileScan {
    pub symbols: Vec<Symbol>,
    pub rpl_props: Vec<RplProp>,
    pub loc: usize,
}

/// True for an identifier character in Enfusion.
fn is_ident(c: char) -> bool {
    c.is_alphanumeric() || c == '_'
}

/// Strip a trailing `//` comment; good enough for brace accounting.
fn strip_line_comment(s: &str) -> &str {
    match s.find("//") {
        Some(i) => &s[..i],
        None => s,
    }
}

/// Parse `Name : Base` / `Name: Base` / `Name` from a declaration remainder.
fn split_name_base(rest: &str) -> (String, String) {
    let rest = rest.trim();
    let head: &str = rest.split(['{', '<']).next().unwrap_or(rest);
    match head.split_once(':') {
        Some((n, b)) => (
            n.trim().to_string(),
            b.trim().trim_end_matches(';').to_string(),
        ),
        None => (head.trim().trim_end_matches(';').to_string(), String::new()),
    }
}

/// Recognise a member function line at class scope, returning its name.
///
/// Enfusion methods look like `void Foo()`, `override protected bool Bar(int x)`,
/// `static CRF_X GetInstance()`, `ref array<ref T> Baz()`. We require a `(` before any `=`
/// so field initialisers are not mistaken for methods.
fn method_name(line: &str) -> Option<String> {
    let l = line.trim();
    if l.is_empty() || l.starts_with('#') || l.starts_with('[') || l.starts_with("//") {
        return None;
    }
    let paren = l.find('(')?;
    if let Some(eq) = l.find('=')
        && eq < paren
    {
        return None;
    }
    // Reject control flow that also has parentheses.
    let head = &l[..paren];
    let last = head
        .rsplit(|c: char| !is_ident(c))
        .find(|s| !s.is_empty())?;
    if matches!(
        last,
        "if" | "for" | "while" | "switch" | "return" | "foreach" | "catch"
    ) {
        return None;
    }
    // A declaration needs a return type (or ctor): at least one token before the name,
    // or the name itself repeated as a constructor. Require the head to have >= 1 token.
    if head.trim().is_empty() {
        return None;
    }
    Some(last.to_string())
}

/// Scan one Enfusion source file.
pub fn scan(path: &Path, rel: &str) -> anyhow::Result<FileScan> {
    let text = std::fs::read_to_string(path)?;
    Ok(scan_str(&text, rel))
}

/// Scan already-loaded source. Split out so it is unit-testable without touching disk.
pub fn scan_str(text: &str, rel: &str) -> FileScan {
    let mut out = FileScan {
        loc: text.lines().count(),
        ..Default::default()
    };

    // Open declarations awaiting their closing brace: (index into out.symbols, depth).
    let mut open: Vec<(usize, i32)> = Vec::new();
    let mut depth: i32 = 0;
    let mut pending_rpl: Option<String> = None; // onRplName from a [RplProp(...)] attribute
    let mut saw_rpl_attr = false;

    for (i, raw) in text.lines().enumerate() {
        let lineno = i + 1;
        let code = strip_line_comment(raw);
        let t = code.trim();

        // ── attributes ───────────────────────────────────────────────────────────────
        // Attributes never change depth in practice; this arm sets `pending_rpl` and falls through
        // to brace counting either way, so collapsing the two `if`s changes nothing — the outer
        // one had no `else` and no body beyond the inner one.
        if t.starts_with('[')
            && let Some(p) = t.find("RplProp")
        {
            saw_rpl_attr = true;
            pending_rpl = t[p..]
                .find("onRplName")
                .and_then(|o| t[p + o..].find('"').map(|q| p + o + q + 1))
                .and_then(|s| t[s..].find('"').map(|e| t[s..s + e].to_string()));
        }

        let enclosing_class = open
            .iter()
            .rev()
            .find(|(idx, _)| {
                matches!(
                    out.symbols[*idx].kind,
                    Kind::Class | Kind::ModdedClass | Kind::SealedClass
                )
            })
            .map(|(idx, _)| out.symbols[*idx].name.clone())
            .unwrap_or_default();

        // ── declarations ─────────────────────────────────────────────────────────────
        if depth == 0 {
            let (kind, rest) = if let Some(r) = t.strip_prefix("modded class ") {
                (Some(Kind::ModdedClass), r)
            } else if let Some(r) = t.strip_prefix("sealed class ") {
                (Some(Kind::SealedClass), r)
            } else if let Some(r) = t.strip_prefix("class ") {
                (Some(Kind::Class), r)
            } else if let Some(r) = t.strip_prefix("enum ") {
                (Some(Kind::Enum), r)
            } else {
                (None, "")
            };

            if let Some(k) = kind {
                let (name, base) = split_name_base(rest);
                if !name.is_empty() {
                    out.symbols.push(Symbol {
                        name,
                        kind: k,
                        file: rel.to_string(),
                        line: lineno,
                        end_line: lineno,
                        base,
                        parent: String::new(),
                    });
                    open.push((out.symbols.len() - 1, depth));
                }
            }
        } else if depth == 1 && !enclosing_class.is_empty() {
            // Members of a class.
            if saw_rpl_attr && !t.is_empty() && !t.starts_with('[') {
                // The declaration line following the attribute carries the property name:
                // `int m_GamemodeState = CRF_EGamemodeState.BRIEFING;`
                let decl = t.split(['=', ';']).next().unwrap_or(t).trim();
                if let Some(prop) = decl.rsplit(|c: char| !is_ident(c)).find(|s| !s.is_empty()) {
                    out.rpl_props.push(RplProp {
                        class: enclosing_class.clone(),
                        prop: prop.to_string(),
                        on_rpl_name: pending_rpl.clone().unwrap_or_default(),
                        file: rel.to_string(),
                        line: lineno,
                    });
                }
                saw_rpl_attr = false;
                pending_rpl = None;
            } else if let Some(m) = method_name(t) {
                out.symbols.push(Symbol {
                    name: m,
                    kind: Kind::Method,
                    file: rel.to_string(),
                    line: lineno,
                    end_line: lineno,
                    base: String::new(),
                    parent: enclosing_class.clone(),
                });
            }
        }

        // ── brace accounting (last, so a decl line's own `{` is counted) ─────────────
        for c in code.chars() {
            match c {
                '{' => depth += 1,
                '}' => {
                    // Clamp at 0. Carved vanilla blobs (T-181.3) start mid-file, so their
                    // first braces are unbalanced; without this, depth goes negative and
                    // every subsequent top-level declaration is missed.
                    depth = (depth - 1).max(0);
                    while let Some(&(idx, d)) = open.last() {
                        if depth <= d {
                            out.symbols[idx].end_line = lineno;
                            open.pop();
                        } else {
                            break;
                        }
                    }
                }
                _ => {}
            }
        }
    }

    out
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Shapes taken verbatim from real CRF/vanilla sources read this session.
    const SAMPLE: &str = r#"
class CRF_SlottingManagerClass : ScriptComponentClass {}

class CRF_SlottingManager : ScriptComponent
{
	[Attribute("0", UIWidgets.Hidden), RplProp(onRplName: "OnGamemodeStateChanged")]
	int m_GamemodeState = CRF_EGamemodeState.BRIEFING;

	override void OnPostInit(IEntity owner)
	{
		if (true) { }
	}

	void UpdateSlotPlayerID(int slotId, int playerId = -1)
	{
	}

	protected bool IsValidGroupInSlot(CRF_SlotData slotData)
	{
	}
}

modded class SCR_ChatComponent
{
}

enum CRF_EGamemodeState
{
	BRIEFING,
}
"#;

    #[test]
    fn finds_real_declarations() {
        let s = scan_str(SAMPLE, "CRF_SlottingManager.c");
        let names: Vec<_> = s.symbols.iter().map(|x| x.name.as_str()).collect();
        assert!(names.contains(&"CRF_SlottingManager"));
        assert!(names.contains(&"CRF_SlottingManagerClass"));
        assert!(names.contains(&"SCR_ChatComponent"));
        assert!(names.contains(&"CRF_EGamemodeState"));
        // The methods that actually exist.
        assert!(names.contains(&"UpdateSlotPlayerID"));
        assert!(names.contains(&"OnPostInit"));
        assert!(names.contains(&"IsValidGroupInSlot"));
    }

    /// The regression that motivates the whole mechanical index.
    #[test]
    fn does_not_invent_apis() {
        let s = scan_str(SAMPLE, "CRF_SlottingManager.c");
        let names: Vec<_> = s.symbols.iter().map(|x| x.name.as_str()).collect();
        for hallucinated in ["RequestSlotChange", "ReleaseSlot", "GetInstance"] {
            assert!(
                !names.contains(&hallucinated),
                "scanner invented {hallucinated}"
            );
        }
    }

    #[test]
    fn base_class_is_exact() {
        let s = scan_str(SAMPLE, "x.c");
        let m = s
            .symbols
            .iter()
            .find(|x| x.name == "CRF_SlottingManager")
            .unwrap();
        // NOT SCR_BaseGameModeComponent, which is what the LLM claimed.
        assert_eq!(m.base, "ScriptComponent");
        assert_eq!(m.kind, Kind::Class);
    }

    #[test]
    fn control_flow_is_not_a_method() {
        let s = scan_str(SAMPLE, "x.c");
        assert!(!s.symbols.iter().any(|x| x.name == "if"));
    }

    #[test]
    fn captures_rplprop_with_callback() {
        let s = scan_str(SAMPLE, "x.c");
        let p = s.rpl_props.first().expect("no RplProp captured");
        assert_eq!(p.prop, "m_GamemodeState");
        assert_eq!(p.on_rpl_name, "OnGamemodeStateChanged");
        assert_eq!(p.class, "CRF_SlottingManager");
    }

    #[test]
    fn modded_class_kind_is_distinct() {
        let s = scan_str(SAMPLE, "x.c");
        let m = s
            .symbols
            .iter()
            .find(|x| x.name == "SCR_ChatComponent")
            .unwrap();
        assert_eq!(m.kind, Kind::ModdedClass);
    }
}
