//! The subset of Markdown the doctrine articles are written in, rendered to views.
//!
//! **Role:** turns an article body into block views — headings, bullet lists, callouts and
//! paragraphs — with bold, italic and code spans resolved inside each block.
//! **Position:** the reading half of the wiki's article surface; the edit half shows the raw
//! source instead and never calls in here.
//! **Signals & state:** none — every function here is a pure transformation of its input.
//! **Invariants:** an emphasis run never contains its own delimiter character, so an unmatched
//! `*` stays literal text; a run of `>` lines is one callout; consecutive non-blank lines join
//! into one paragraph with single spaces before inline spans are resolved.

use leptos::prelude::*;

/// Resolves the inline spans of `text` into a view per span.
///
/// `**bold**` becomes a `<strong>`, `*italic*` an `<em>` and `` `code` `` a monospace `<code>`;
/// everything else accumulates into plain text nodes. Returns the spans in source order.
fn render_inline(text: &str) -> Vec<AnyView> {
    let mut out: Vec<AnyView> = Vec::new();
    let mut plain = String::new();
    let mut rest = text;
    while !rest.is_empty() {
        let tok = if let Some(inner) = delim_token(rest, "**", '*') {
            Some(("b", inner.to_string(), 2 + inner.len() + 2))
        } else if rest.starts_with('*') {
            delim_token(rest, "*", '*').map(|inner| ("i", inner.to_string(), 1 + inner.len() + 1))
        } else if rest.starts_with('`') {
            delim_token(rest, "`", '`').map(|inner| ("c", inner.to_string(), 1 + inner.len() + 1))
        } else {
            None
        };
        match tok {
            Some((kind, inner, consumed)) => {
                if !plain.is_empty() {
                    out.push(view! { {plain.clone()} }.into_any());
                    plain.clear();
                }
                out.push(match kind {
                    "b" => view! { <strong class="font-semibold text-on-surface">{inner}</strong> }.into_any(),
                    "c" => view! { <code class="rounded bg-black/40 px-1.5 py-0.5 font-mono text-[0.85em] text-primary">{inner}</code> }.into_any(),
                    _ => view! { <em>{inner}</em> }.into_any(),
                });
                rest = &rest[consumed..];
            }
            None => {
                let ch = rest.chars().next().unwrap();
                plain.push(ch);
                rest = &rest[ch.len_utf8()..];
            }
        }
    }
    if !plain.is_empty() {
        out.push(view! { {plain} }.into_any());
    }
    out
}

/// The inner run of a delimited span at the start of `s`, if there is one.
///
/// `open` is the opening (and closing) delimiter and `bad` the character the inner run may not
/// contain. Returns `None` when `s` does not start with `open`, when the run would be empty, or
/// when the closing delimiter never arrives.
fn delim_token<'a>(s: &'a str, open: &str, bad: char) -> Option<&'a str> {
    let after = s.strip_prefix(open)?;
    // The run ends at the first forbidden character, or at the end of the string.
    let inner_end = after.find(bad).unwrap_or(after.len());
    if inner_end == 0 {
        return None;
    }
    // Only a run closed by the delimiter itself counts as a span.
    if after[inner_end..].starts_with(open) {
        Some(&after[..inner_end])
    } else {
        None
    }
}

/// Renders a whole article body as a sequence of block views.
///
/// Recognises `# ` and `## ` headings, `> ` callout runs and `- `/`* ` bullet runs; every other
/// non-blank run becomes a paragraph. Blank lines separate blocks and render nothing.
pub(super) fn render_markdown(source: &str) -> impl IntoView {
    let lines: Vec<&str> = source.split('\n').collect();
    let mut blocks: Vec<AnyView> = Vec::new();
    let mut i = 0;
    while i < lines.len() {
        let line = lines[i];
        if line.trim().is_empty() {
            i += 1;
            continue;
        }
        if let Some(rest) = line.strip_prefix("## ") {
            blocks.push(view! { <h2 class="mt-10 mb-3 border-b border-white/10 pb-2 text-xl font-bold tracking-tight text-white">{render_inline(rest)}</h2> }.into_any());
            i += 1;
            continue;
        }
        if let Some(rest) = line.strip_prefix("# ") {
            blocks.push(view! { <h1 class="mb-4 text-2xl font-bold tracking-tight text-white">{render_inline(rest)}</h1> }.into_any());
            i += 1;
            continue;
        }
        if line.starts_with('>') {
            let mut quoted: Vec<String> = Vec::new();
            while i < lines.len() && lines[i].starts_with('>') {
                // Drop the marker and, if it is there, the single space after it.
                let after = &lines[i][1..];
                let after = after.strip_prefix(' ').unwrap_or(after);
                quoted.push(after.to_string());
                i += 1;
            }
            blocks.push(callout(&quoted));
            continue;
        }
        if line.starts_with("- ") || line.starts_with("* ") {
            let mut items: Vec<String> = Vec::new();
            while i < lines.len() && (lines[i].starts_with("- ") || lines[i].starts_with("* ")) {
                items.push(lines[i][2..].to_string());
                i += 1;
            }
            blocks.push(view! {
                <ul class="mt-3 ml-1 space-y-2 text-body-md text-on-surface-variant">
                    {items.into_iter().map(|it| view! { <li>"• "{render_inline(&it)}</li> }).collect_view()}
                </ul>
            }.into_any());
            continue;
        }
        let mut para: Vec<&str> = Vec::new();
        while i < lines.len()
            && !lines[i].trim().is_empty()
            && !lines[i].starts_with('#')
            && !lines[i].starts_with('>')
            && !lines[i].starts_with("- ")
            && !lines[i].starts_with("* ")
        {
            para.push(lines[i]);
            i += 1;
        }
        blocks.push(view! { <p class="mt-3 text-body-md leading-relaxed text-on-surface-variant">{render_inline(&para.join(" "))}</p> }.into_any());
    }
    blocks
}

/// Renders one `>` run as a callout box.
///
/// `quoted` holds the run with its `>` markers already stripped. A leading `[!TYPE]` tag selects
/// the colour and the default label and is dropped from the body; any text after the tag becomes
/// the label instead. An unrecognised or absent tag renders the informational variant.
fn callout(quoted: &[String]) -> AnyView {
    // Box class, label class and default label; the informational variant until a tag says otherwise.
    let (mut box_cls, mut label_cls, mut default_title) =
        ("bg-primary/10 border-primary", "text-primary", "NOTE");
    let mut title: Option<String> = None;
    let mut body_lines: &[String] = quoted;
    if let Some(first) = quoted.first() {
        if let Some(tag) = parse_tag(first) {
            let mapped = match tag.0.to_uppercase().as_str() {
                "CRITICAL" | "CAUTION" => Some(("critical", None::<&str>)),
                "WARNING" => Some(("warning", None)),
                "TIP" => Some(("info", Some("PRO-TIP"))),
                "NOTE" | "INFO" => Some(("info", None)),
                _ => None,
            };
            if let Some((variant, tag_title)) = mapped {
                let styles = match variant {
                    "critical" => (
                        "bg-error/10 border-error",
                        "text-error-alert",
                        "CRITICAL RULE",
                    ),
                    "warning" => (
                        "bg-tactical-yellow/10 border-tactical-yellow",
                        "text-tactical-yellow",
                        "WARNING",
                    ),
                    _ => ("bg-primary/10 border-primary", "text-primary", "NOTE"),
                };
                box_cls = styles.0;
                label_cls = styles.1;
                default_title = tag_title.unwrap_or(styles.2);
                let explicit = tag.1.trim();
                title = if explicit.is_empty() {
                    None
                } else {
                    Some(explicit.to_string())
                };
                body_lines = &quoted[1..];
            }
        }
    }
    let shown_title = title.unwrap_or_else(|| default_title.to_string());
    let body = body_lines
        .iter()
        .map(|s| s.as_str())
        .collect::<Vec<_>>()
        .join(" ");
    let outer = crate::v2::core::ui::cn(&[
        "my-6 rounded-2xl border border-l-4 p-4 shadow-lg backdrop-blur-md",
        box_cls,
    ]);
    let label = crate::v2::core::ui::cn(&[
        "mb-1 font-mono text-xs font-bold tracking-widest uppercase",
        label_cls,
    ]);
    view! {
        <div class=outer>
            <p class=label>{shown_title}</p>
            <div class="text-body-md leading-relaxed text-on-surface-variant">
                {render_inline(&body)}
            </div>
        </div>
    }
    .into_any()
}

/// Splits a leading `[!TAG]` marker into the tag and the text that follows it.
///
/// Returns `None` unless the line opens with `[!`, closes the bracket, and spells the tag with
/// ASCII letters and hyphens only.
fn parse_tag(line: &str) -> Option<(&str, &str)> {
    let inner = line.strip_prefix("[!")?;
    let close = inner.find(']')?;
    let tag = &inner[..close];
    if tag.is_empty() || !tag.chars().all(|c| c.is_ascii_alphabetic() || c == '-') {
        return None;
    }
    let rest = inner[close + 1..].trim_start();
    Some((tag, rest))
}
