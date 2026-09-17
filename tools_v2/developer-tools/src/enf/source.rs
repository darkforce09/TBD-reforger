//! T-181.3.3 — reconstruct vanilla `.c` source from Doxygen `*_source.html` pages.
//!
//! The AR Explorer (arexplorer.zeroy.com) is a Doxygen build of Arma Reforger 1.7.0.54 with
//! SOURCE_BROWSER enabled — 6,495 source pages, exactly matching the script count in the pak
//! file table, each carrying the **complete file including method bodies**.
//!
//! That is strictly better than every other lane we have:
//!   * the pak's compressed entries are an unidentified codec (T-181.3.3 negative results),
//!   * BI's official API docs give signatures but no bodies,
//!   * byte-carving only reaches the uncompressed minority.
//!
//! Doxygen wraps each line as `<div class="line" ...>…</div>` with syntax-highlighting spans
//! and a leading line number. Undoing that is mechanical: drop tags, unescape entities, strip
//! the line-number prefix. Fetching lives in `scripts/mod/fetch-vanilla-source.sh`; this parses
//! a local cache so rebuilds are offline and deterministic.

use std::fmt::Write as _;
use std::path::Path;

use anyhow::{Context, Result};

pub struct SourceStats {
    pub pages: usize,
    pub files: usize,
    pub lines: usize,
}

/// Strip tags and unescape the entities Doxygen emits.
fn detag(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut in_tag = false;
    for c in s.chars() {
        match c {
            '<' => in_tag = true,
            '>' => in_tag = false,
            _ if !in_tag => out.push(c),
            _ => {}
        }
    }
    out.replace("&#160;", " ")
        .replace("&nbsp;", " ")
        .replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&quot;", "\"")
        .replace("&#39;", "'")
        .replace("&amp;", "&")
}

/// Doxygen prefixes each line with its right-aligned number; drop it so the output is real
/// source whose line numbers match the original file.
fn strip_line_number(s: &str) -> &str {
    // `[' ', '\u{a0}']` is a `Pattern` over the char set — same two characters as the closure it
    // replaces (space and NBSP; Doxygen pads with the latter), no `\t` and no other whitespace,
    // so this is deliberately NOT `char::is_whitespace`.
    let t = s.trim_start_matches([' ', '\u{a0}']);
    let digits = t.len() - t.trim_start_matches(|c: char| c.is_ascii_digit()).len();
    if digits == 0 {
        return s;
    }
    &t[digits..]
}

/// Reconstruct one file's text from a source page.
pub fn parse_page(html: &str) -> Vec<String> {
    let mut out = Vec::new();
    for chunk in html.split("<div class=\"line\"").skip(1) {
        let Some(start) = chunk.find('>') else {
            continue;
        };
        let rest = &chunk[start + 1..];
        // Lines are flat divs; the first closing tag ends this line's content.
        let end = rest.find("</div>").unwrap_or(rest.len());
        let text = detag(&rest[..end]);
        out.push(strip_line_number(&text).to_string());
    }
    out
}

/// Recover the original filename from a Doxygen page name.
/// `_s_c_r___base_game_mode_8c_source.html` -> `SCR_BaseGameMode.c`
fn demangle(page: &str) -> Option<String> {
    let stem = page.strip_suffix("_source.html")?;
    let stem = stem.strip_suffix("_8c")?;
    let b = stem.as_bytes();
    let mut out = String::new();
    let mut i = 0;
    while i < b.len() {
        if b[i] == b'_' {
            if i + 1 < b.len() && b[i + 1] == b'_' {
                out.push('_');
                i += 2;
            } else if i + 1 < b.len() && b[i + 1].is_ascii_lowercase() {
                out.push(b[i + 1].to_ascii_uppercase() as char);
                i += 2;
            } else {
                out.push('_');
                i += 1;
            }
        } else {
            out.push(b[i] as char);
            i += 1;
        }
    }
    out.push_str(".c");
    Some(out)
}

/// Parse every cached page in `src` into `.c` files under `out`.
pub fn build(src: &Path, out: &Path) -> Result<SourceStats> {
    let rd = std::fs::read_dir(src).with_context(|| {
        format!(
            "reading {} — run scripts/mod/fetch-vanilla-source.sh first",
            src.display()
        )
    })?;
    let mut pages: Vec<_> = rd
        .flatten()
        .map(|e| e.path())
        .filter(|p| {
            p.file_name()
                .and_then(|s| s.to_str())
                .is_some_and(|s| s.ends_with("_source.html"))
        })
        .collect();
    pages.sort();

    std::fs::create_dir_all(out)?;
    let mut manifest = String::from("file\tlines\tpage\n");
    let mut st = SourceStats {
        pages: pages.len(),
        files: 0,
        lines: 0,
    };

    for p in &pages {
        let page_name = p.file_name().and_then(|s| s.to_str()).unwrap_or_default();
        let Some(fname) = demangle(page_name) else {
            continue;
        };
        let Ok(html) = std::fs::read_to_string(p) else {
            continue;
        };
        let lines = parse_page(&html);
        if lines.is_empty() {
            continue;
        }
        std::fs::write(out.join(&fname), lines.join("\n"))?;
        let _ = writeln!(manifest, "{}\t{}\t{}", fname, lines.len(), page_name);
        st.files += 1;
        st.lines += lines.len();
    }

    // T-537: refuse a header-only `_SOURCE_MANIFEST.tsv` overwrite when nothing demangled.
    super::refuse_empty_write(
        "enf source manifest",
        st.files == 0,
        "extracted zero source pages — refusing header-only _SOURCE_MANIFEST.tsv overwrite",
    )?;
    std::fs::write(out.join("_SOURCE_MANIFEST.tsv"), manifest)?;
    Ok(st)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn demangles_doxygen_names() {
        assert_eq!(
            demangle("_s_c_r___base_game_mode_8c_source.html").as_deref(),
            Some("SCR_BaseGameMode.c")
        );
        assert_eq!(
            demangle("_chimera_menu_base_8c_source.html").as_deref(),
            Some("ChimeraMenuBase.c")
        );
    }

    #[test]
    fn parses_source_lines_and_drops_numbers() {
        let html = r#"<div class="line"><a id="l00154" name="l00154"></a><span class="lineno">  154</span>    <span class="keyword">protected</span> void Foo()</div>
<div class="line"><span class="lineno">  155</span>    {</div>"#;
        let lines = parse_page(html);
        assert_eq!(lines.len(), 2);
        assert!(
            lines[0].contains("protected void Foo()"),
            "got {:?}",
            lines[0]
        );
        assert!(
            !lines[0].contains("154"),
            "line number leaked: {:?}",
            lines[0]
        );
        assert_eq!(lines[1].trim(), "{");
    }
}
