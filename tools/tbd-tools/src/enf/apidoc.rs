//! T-181.3.1 — parse Bohemia's official Script API (Doxygen HTML) into the oracle index.
//!
//! Closes the gap `enf carve` cannot: the compressed-only classes (SCR_BaseGameMode,
//! SCR_PossessSpawnData, SCR_PossessSpawnRequestComponent, SCR_RespawnSystemComponent,
//! ChimeraMenuBase) are all present in the published docs — **7,990 classes** in one index
//! page. Signatures and inheritance, no bodies.
//!
//! Fetching is `scripts/mod/fetch-vanilla-api.sh` (curl; the wiki 403s a default UA). This
//! module only parses a local cache, so the index rebuild is offline and deterministic.
//!
//! Deliberately a small tag-stripping parser rather than a HTML crate: the Doxygen output is
//! machine-generated and uniform, and adding a scraper dependency to the workspace for four
//! regex-shaped extractions is not worth the lock churn.

use std::fmt::Write as _;
use std::path::Path;

use anyhow::{Context, Result};

#[derive(Debug)]
pub struct ApiStats {
    pub classes: usize,
    pub member_pages: usize,
    pub members: usize,
}

/// Strip HTML tags and decode the handful of entities Doxygen emits.
fn text_of(s: &str) -> String {
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
        .replace("&amp;", "&")
        .replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&quot;", "\"")
        .trim()
        .to_string()
}

/// Pull `href="X.html" ...>Name<` pairs plus the following `<td class="desc">` cell.
fn parse_index(html: &str) -> Vec<(String, String, String)> {
    let mut out = Vec::new();
    for row in html.split("<tr ").skip(1) {
        let Some(h0) = row.find("href=\"") else {
            continue;
        };
        let rest = &row[h0 + 6..];
        let Some(h1) = rest.find('"') else { continue };
        let doc = &rest[..h1];
        if !doc.ends_with(".html") {
            continue;
        }
        // Anchor text is the class name.
        let Some(a0) = rest.find('>') else { continue };
        let after = &rest[a0 + 1..];
        let Some(a1) = after.find("</a>") else {
            continue;
        };
        let name = text_of(&after[..a1]);
        if name.is_empty() || name.contains(' ') {
            continue;
        }
        let desc = match after.find("class=\"desc\"") {
            Some(d0) => {
                let d = &after[d0..];
                match (d.find('>'), d.find("</td>")) {
                    (Some(s), Some(e)) if e > s => text_of(&d[s + 1..e]),
                    _ => String::new(),
                }
            }
            None => String::new(),
        };
        out.push((name, doc.to_string(), desc));
    }
    out.sort();
    out.dedup();
    out
}

/// Member rows on a class page: `<td class="memItemLeft"...>ret</td><td class="memItemRight"...>sig</td>`
fn parse_members(html: &str) -> Vec<String> {
    let mut out = Vec::new();
    for chunk in html.split("memItemLeft").skip(1) {
        // Split lands *inside* the <td ...> tag, so skip to the end of that tag first —
        // otherwise the remaining attributes (align=/valign=) leak into the signature text.
        let Some(l_open) = chunk.find('>') else {
            continue;
        };
        let chunk = &chunk[l_open + 1..];
        let Some(l_end) = chunk.find("</td>") else {
            continue;
        };
        let ret = text_of(&chunk[..l_end]);
        let Some(r0) = chunk.find("memItemRight") else {
            continue;
        };
        let r = &chunk[r0..];
        let Some(r_open) = r.find('>') else { continue };
        let r = &r[r_open + 1..];
        let Some(r_end) = r.find("</td>") else {
            continue;
        };
        let sig = text_of(&r[..r_end]);
        if sig.is_empty() {
            continue;
        }
        let joined = format!("{ret} {sig}").trim().to_string();
        if !joined.is_empty() {
            out.push(joined);
        }
    }
    out.sort();
    out.dedup();
    out
}

/// Parse the cached docs at `src` into TSVs under `out`.
pub fn build(src: &Path, out: &Path) -> Result<ApiStats> {
    let index_path = src.join("annotated.html");
    let index_html = std::fs::read_to_string(&index_path).with_context(|| {
        format!(
            "reading {} — run scripts/mod/fetch-vanilla-api.sh first",
            index_path.display()
        )
    })?;

    let rows = parse_index(&index_html);
    let mut classes_tsv = String::from("class\tdoc_page\n");
    for (name, doc, desc) in &rows {
        // Names + coordinates only. BI's description prose is THEIR content; the committed
        // index deliberately carries facts about the API, not text copied from their docs.
        let _ = desc;
        let _ = writeln!(classes_tsv, "{name}\t{doc}");
    }

    let mut members_tsv = String::from("class\tsignature\n");
    let mut member_pages = 0usize;
    let mut members = 0usize;
    if let Ok(rd) = std::fs::read_dir(src) {
        let mut files: Vec<_> = rd
            .flatten()
            .map(|e| e.path())
            .filter(|p| {
                p.extension().and_then(|s| s.to_str()) == Some("html")
                    && p.file_name().and_then(|s| s.to_str()) != Some("annotated.html")
            })
            .collect();
        files.sort();
        for p in files {
            let Ok(html) = std::fs::read_to_string(&p) else {
                continue;
            };
            // interfaceSCR__BaseGameMode.html -> SCR_BaseGameMode
            //
            // T-603: a `.replace('_', "_")` used to sit between the two lines below. Clippy's
            // `no_effect_replace` is right that it is a no-op — `str::replace` returns a new
            // String with each match swapped, and swapping "_" for "_" swaps nothing. It read as
            // "and leave single underscores alone", which the sentinel already guarantees:
            // Doxygen escapes a real `_` in a class name as `__`, so `__` is mapped out of the
            // way first and only mapped back after. Deleted, not silenced.
            let stem = p.file_stem().and_then(|s| s.to_str()).unwrap_or("");
            let class = stem
                .trim_start_matches("interface")
                .trim_start_matches("class")
                .trim_start_matches("struct")
                .replace("__", "\u{1}")
                .replace('\u{1}', "_");
            for sig in parse_members(&html) {
                let _ = writeln!(members_tsv, "{class}\t{sig}");
                members += 1;
            }
            member_pages += 1;
        }
    }

    // T-537: refuse header-only TSV overwrite of the committed enf-index (T-383 residual).
    // The bin used to write first and only then exit 1 on classes==0 — damage already done.
    super::refuse_empty_write(
        "enf apidoc classes TSV",
        rows.is_empty(),
        "parsed zero classes — refusing header-only overwrite of vanilla_api_classes.tsv",
    )?;
    super::refuse_empty_write(
        "enf apidoc members TSV",
        members == 0,
        "parsed zero member signatures — refusing header-only overwrite of vanilla_api_members.tsv",
    )?;

    std::fs::create_dir_all(out)?;
    std::fs::write(out.join("vanilla_api_classes.tsv"), classes_tsv)?;
    std::fs::write(out.join("vanilla_api_members.tsv"), members_tsv)?;

    Ok(ApiStats {
        classes: rows.len(),
        member_pages,
        members,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    // Verbatim shape from the real annotated.html.
    const ROW: &str = r#"<tr id="row_0_" class="even"><td class="entry"><span class="icona"><span class="icon">C</span></span><a class="el" href="interfaceAABGridMap.html" target="_self">AABGridMap</a></td><td class="desc">Represent a grid map </td></tr>"#;

    #[test]
    fn parses_index_rows() {
        let rows = parse_index(ROW);
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].0, "AABGridMap");
        assert_eq!(rows[0].1, "interfaceAABGridMap.html");
        assert_eq!(rows[0].2, "Represent a grid map");
    }

    #[test]
    fn strips_entities() {
        assert_eq!(text_of("<b>a&#160;b</b>"), "a b");
    }

    #[test]
    fn build_refuses_header_only_classes_tsv() {
        // T-537 Class-R: empty annotated.html must not overwrite committed vanilla_api_*.tsv.
        let dir = std::env::temp_dir().join(format!("t537-apidoc-empty-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        let src = dir.join("src");
        let out = dir.join("out");
        std::fs::create_dir_all(&src).unwrap();
        std::fs::write(src.join("annotated.html"), "<html><body></body></html>").unwrap();
        // Seed a non-empty committed-shaped out so a silent overwrite would be observable.
        std::fs::create_dir_all(&out).unwrap();
        let classes_path = out.join("vanilla_api_classes.tsv");
        let members_path = out.join("vanilla_api_members.tsv");
        std::fs::write(&classes_path, "class\tdoc_page\nKeepMe\tkeep.html\n").unwrap();
        std::fs::write(&members_path, "class\tsignature\nKeepMe\tvoid Keep()\n").unwrap();
        let before_c = std::fs::read_to_string(&classes_path).unwrap();
        let before_m = std::fs::read_to_string(&members_path).unwrap();

        let err = build(&src, &out).expect_err("must refuse empty apidoc write");
        let msg = format!("{err:#}");
        assert!(
            msg.contains("refusing empty write") && msg.contains("zero classes"),
            "{msg}"
        );
        assert_eq!(
            std::fs::read_to_string(&classes_path).unwrap(),
            before_c,
            "classes TSV must be untouched on refuse"
        );
        assert_eq!(
            std::fs::read_to_string(&members_path).unwrap(),
            before_m,
            "members TSV must be untouched on refuse"
        );
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn build_refuses_header_only_members_tsv() {
        // Classes parse, but no member pages → members TSV would be header-only.
        let dir =
            std::env::temp_dir().join(format!("t537-apidoc-nomembers-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        let src = dir.join("src");
        let out = dir.join("out");
        std::fs::create_dir_all(&src).unwrap();
        std::fs::write(
            src.join("annotated.html"),
            r#"<tr id="row_0_" class="even"><td class="entry"><a class="el" href="interfaceAABGridMap.html" target="_self">AABGridMap</a></td><td class="desc">grid</td></tr>"#,
        )
        .unwrap();
        std::fs::create_dir_all(&out).unwrap();
        let members_path = out.join("vanilla_api_members.tsv");
        std::fs::write(&members_path, "class\tsignature\nKeepMe\tvoid Keep()\n").unwrap();
        let before_m = std::fs::read_to_string(&members_path).unwrap();

        let err = build(&src, &out).expect_err("must refuse empty members write");
        let msg = format!("{err:#}");
        assert!(
            msg.contains("refusing empty write") && msg.contains("zero member"),
            "{msg}"
        );
        assert_eq!(
            std::fs::read_to_string(&members_path).unwrap(),
            before_m,
            "members TSV must be untouched on refuse"
        );
        let _ = std::fs::remove_dir_all(&dir);
    }
}
