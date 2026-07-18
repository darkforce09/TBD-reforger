//! SOPs & Manuals (/wiki) — ported from pages/doctrine.tsx `WikiPage`. `<AuthGate>` → a `GlassSplit`:
//! a category-grouped manual index (master) + a reading pane (detail) rendering the active manual's
//! Markdown. Fully client-MOCK-driven (`MANUALS`); the Markdown renderer is ported byte-for-byte.
//!
//! **Gate scope (this slice):** the default render (search empty, `plan-timeline` selected, read
//! mode) — the index + the article header + the full rendered Markdown body is byte-exact-verified.
//! Search + the edit mode (textarea) are behavior — a follow-up.
#![allow(dead_code)]
use crate::split_pane::{GlassSplit, ListDetailItem, SidebarSearch};
use leptos::prelude::*;

const BADGE_NEUTRAL: &str = "inline-flex items-center gap-1 rounded border px-2 py-0.5 uppercase whitespace-nowrap border-outline-variant/40 bg-surface-variant/40 text-on-surface-variant";

struct Manual {
    id: &'static str,
    category: &'static str,
    title: &'static str,
    updated: &'static str,
    body: &'static str,
}

const CATEGORY_ORDER: [&str; 4] = [
    "Leadership Fundamentals",
    "Timeline & Mission Planning",
    "Dynamic Communications Strategy",
    "Combat Formations & Maneuvers",
];

const PLAN_TIMELINE_BODY: &str = r#"In a 1-life PvP environment, the plan you make in the staging area decides the fight before a single shot is fired. You do not get a second attempt. This guide uses the Swedish squad leader methodology — a simple, repeatable loop for turning a vague objective into a clear, time-bound plan your squad can actually execute under pressure.

The whole process answers three questions, in order: **What is the problem? How much time do I have? How do we adapt when it breaks?**

## Phase 1 — Define the Problem

Before you talk about movement, get brutally clear on what you are actually being asked to do and what stands in the way. If you cannot state the objective in one sentence, you are not ready to brief it.

- **The objective.** What does winning look like — hold a grid, destroy an asset, break the enemy's main effort? Name it.
- **Enemy composition.** How many, how equipped, armor or air? Where are they likely strong, and where are they thin?
- **Terrain.** What covers our approach, what channels us into a kill zone, and what high ground matters?
- **Our assets.** Squad size, weapon teams, vehicles, and — critically — how much daylight and time you have.

> [!CRITICAL]
> Plan against what the enemy *can* do, not what you *hope* they do. Build your plan around their most dangerous option, then exploit the gaps.

## Phase 2 — Define the Timeline

This is the part most squad leads skip, and it is the part that wins matches. Work **backwards** from the moment you expect contact and assign hard times to every step. A plan without a clock is just a wish.

- Set your decisive moment — call it `H-Hour` (the assault, the ambush trigger, the objective seizure).
- Back-plan from it: `H-15` in support-by-fire position, `H-30` at the last covered rally, `H-45` step off from staging.
- Reserve time for the things that always run long: crossing danger areas, re-org after contact, and getting everyone on the same map.
- Give the squad a **time hack** so every watch matches. "We move in 5" means nothing if nobody agrees on now.

> [!TIP]
> Budget roughly a third of your available time for planning and rehearsal, and two-thirds for movement and execution. If you spend the whole window talking, you will be rushing — and loud — when it counts.

## Phase 3 — Execution & Adaptability

No plan survives first contact. The point of Phases 1 and 2 is not a rigid script — it is to give your squad enough shared understanding that they keep fighting *your intent* when the plan falls apart and you can't talk to them.

- Brief **intent**, not just instructions: "I want us holding the north ridge by `H+10`, even if Alpha gets pinned." Tell them the *why*.
- Push decisions down. A fireteam that understands the goal will make a good call faster than they can raise you on a (possibly looted) radio.
- Name your triggers and branches in advance: "If we take fire from the treeline, Bravo suppresses, Alpha flanks left — no further orders needed."
- Run a 60-second rehearsal or backbrief. Have a team lead repeat the plan back; you will catch the gaps before the enemy does.

> [!WARNING]
> When the plan breaks, the worst choice is to freeze and wait for perfect information. Make a decision, communicate it in one line, and keep the squad moving. Momentum beats hesitation in a 1-life fight."#;

// All five bodies ported from React doctrine.tsx (T-172 A4 — selection needs real articles).
const MANUALS: &[Manual] = &[
    Manual {
        id: "plan-timeline",
        category: "Timeline & Mission Planning",
        title: "Timeline & Mission Planning",
        updated: "2026-06-18",
        body: PLAN_TIMELINE_BODY,
    },
    Manual {
        id: "lead-role",
        category: "Leadership Fundamentals",
        title: "The Squad Leader Mindset",
        updated: "2026-06-15",
        body: r#"Your job is not to be the best shooter — it is to make the rest of the squad more effective than they would be alone. You fight with your radio and your map first, your rifle second. A squad lead who is heads-down in a firefight is a squad lead who has stopped leading.

## What You Own

- **The plan** — and making sure everyone understands the intent behind it.
- **Tempo** — knowing when to push hard and when to slow down and reset.
- **Information** — building the picture and pushing the relevant parts down.

> [!TIP]
> Position yourself where you can *see and influence*, not where the action is hottest. Usually that is just behind your lead element, with eyes on the objective.

> [!CRITICAL]
> Calm is contagious, and so is panic. The squad takes its emotional temperature from you — if you keep your voice level under fire, they will too."#,
    },
    Manual {
        id: "lead-decisions",
        category: "Leadership Fundamentals",
        title: "Decision-Making Under Pressure",
        updated: "2026-06-10",
        body: r#"In a 1-life fight you will rarely have complete information, and waiting for it is itself a decision — usually a bad one. Train yourself to act on a good-enough read of the situation.

## A Fast Decision Loop

- **Read** — what just changed, and what is the biggest threat right now?
- **Decide** — pick the option that keeps initiative and protects the squad.
- **Act** — give one clear order and commit; correct on the move.

> [!WARNING]
> A decent decision made now beats a perfect decision made too late. Indecision gets people killed faster than a wrong call you correct quickly."#,
    },
    Manual {
        id: "comms-dynamic",
        category: "Dynamic Communications Strategy",
        title: "Operating With Looted Radios",
        updated: "2026-06-16",
        body: r#"We do not use fixed frequencies. The enemy can loot a radio off a body and listen to everything you say — so our comms plan assumes the net is compromised from the start. Frequencies are randomized each match and treated as throwaway.

## Assume You Are Being Heard

- Distribute the match frequency in the staging area, never over an open channel.
- If a member goes down in enemy territory, treat that frequency as **burned** and jump to your pre-agreed fallback.
- Reference locations by features or a private grid-shift, not raw map grids the enemy can also read.
- Keep transmissions short. Long, chatty traffic gives away your strength, intent, and rough position.

> [!CRITICAL]
> The moment a radio is lost behind enemy lines, every callsign and reference on that net is assumed compromised. Switch frequency and stop using any code words tied to it.

> [!TIP]
> Agree on a one-word **flash** signal before the op that means "the net is blown, jump to fallback now." One word, everyone moves, no debate on the radio."#,
    },
    Manual {
        id: "combat-formations",
        category: "Combat Formations & Maneuvers",
        title: "Fire & Movement",
        updated: "2026-06-05",
        body: r#"Everything in a gunfight comes down to one principle: one element fixes the enemy with fire while the other moves. If nobody is shooting, nobody should be moving in the open.

## Bounding

- Split into a **base of fire** and a **maneuver element** before you make contact, not during.
- Short bounds between hard cover — stay up only as long as your buddy can realistically cover you.
- The flank, not the frontal push, wins the position. Use fire to pin them in place while you get to their side.

> [!WARNING]
> Stay dispersed. In a 1-life fight, two operators caught in the same blast or burst is two permanent losses for the rest of the match.

> [!TIP]
> Read the terrain backwards from the objective: pick your support-by-fire position and your assault lane *before* you move, and the formation almost chooses itself."#,
    },
];

/* ───────────────────────── Markdown renderer (ports renderInline + Markdown) ───────────────────────── */

/// Inline: `**bold**` → strong, `*italic*` → em, `` `code` `` → Mono, else plain text. Mirrors the
/// JS regex /(\*\*[^*]+\*\*|\*[^*]+\*|`[^`]+`)/g: each token's inner content has no delimiter char.
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

/// If `s` opens with `open`, return the inner run up to the closing `open` — where the inner run
/// contains no `bad` char (the regex `[^delim]+`) and is non-empty. Else None.
fn delim_token<'a>(s: &'a str, open: &str, bad: char) -> Option<&'a str> {
    let after = s.strip_prefix(open)?;
    // inner = longest prefix with no `bad`
    let inner_end = after.find(bad).unwrap_or(after.len());
    if inner_end == 0 {
        return None;
    }
    // the char run must be immediately followed by the closing delimiter
    if after[inner_end..].starts_with(open) {
        Some(&after[..inner_end])
    } else {
        None
    }
}

fn render_markdown(source: &str) -> impl IntoView {
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
                // strip /^>\s?/ — the '>' then an optional single whitespace
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
        // paragraph
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

/// A `> [!TYPE]` callout block. Ports the CALLOUT_TAGS + CALLOUT_STYLES mapping.
fn callout(quoted: &[String]) -> AnyView {
    // (variant box class, label class, default title) + optional explicit title
    let (mut box_cls, mut label_cls, mut default_title) =
        ("bg-primary/10 border-primary", "text-primary", "NOTE"); // info default
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
    let outer = crate::ui::cn(&[
        "my-6 rounded-2xl border border-l-4 p-4 shadow-lg backdrop-blur-md",
        box_cls,
    ]);
    let label = crate::ui::cn(&[
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

/// Match `^\[!([A-Za-z-]+)\]\s*(.*)$` → (tag, rest).
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

/* ───────────────────────────────── page ───────────────────────────────── */

/// Resolve the active manual from the `/wiki/:slug` route param — unknown/absent slug falls back
/// to the first manual (the `/wiki` default, `plan-timeline`). T-172 A4 + H11.
fn resolve_wiki_selection(slug: Option<&str>) -> &'static Manual {
    slug.and_then(|s| MANUALS.iter().find(|m| m.id == s))
        .unwrap_or(&MANUALS[0])
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum WikiMode {
    Read,
    Edit,
}

#[component]
pub fn WikiPage() -> impl IntoView {
    // Selection derives from the route (deep-linkable; back/forward work); rows navigate.
    let params = leptos_router::hooks::use_params_map();
    let selected =
        Memo::new(move |_| resolve_wiki_selection(params.read().get("slug").as_deref()).id);
    let search = RwSignal::new(String::new());
    // READ/EDIT mode + session-local per-manual Markdown edits (React parity; no backend until
    // the wiki CMS ships). Switching manuals drops back to read mode, like React.
    let mode = RwSignal::new(WikiMode::Read);
    let edits = RwSignal::new(std::collections::HashMap::<&'static str, String>::new());
    Effect::new(move |prev: Option<&'static str>| {
        let id = selected.get();
        if prev.is_some_and(|p| p != id) {
            mode.set(WikiMode::Read);
        }
        id
    });
    view! {
        <crate::ui::AuthGate>
            <GlassSplit
                master_width="17rem"
                master_header=master_header(search).into_any()
                master=view! { {move || manual_index(selected.get(), &search.get())} }.into_any()
                detail=view! {
                    {move || {
                        article(
                            MANUALS.iter().find(|m| m.id == selected.get()).unwrap_or(&MANUALS[0]),
                            mode,
                            edits,
                        )
                    }}
                }
                    .into_any()
            />
        </crate::ui::AuthGate>
    }
}

fn master_header(search: RwSignal<String>) -> impl IntoView {
    view! {
        <div class="w-full space-y-3">
            <p class="font-mono text-xs font-bold tracking-widest text-on-surface-variant uppercase">
                "SOPs & Manuals"
            </p>
            <SidebarSearch placeholder="Search manuals..." bind=search />
        </div>
    }
}

fn manual_index(active_id: &'static str, query: &str) -> impl IntoView {
    let query = query.to_string();
    CATEGORY_ORDER
        .iter()
        .filter_map(move |category| {
            // React filters on `${title} ${category}` (case-insensitive substring).
            let rows: Vec<&Manual> = MANUALS
                .iter()
                .filter(|m| m.category == *category)
                .filter(|m| {
                    crate::split_pane::search_matches(
                        &query,
                        &format!("{} {}", m.title, m.category),
                    )
                })
                .collect();
            if rows.is_empty() {
                return None;
            }
            Some(view! {
                <div class="mb-3">
                    <p class="px-1 py-1 font-mono text-[11px] tracking-widest text-outline uppercase">
                        {*category}
                    </p>
                    <div class="mt-1 flex flex-col gap-1">
                        {rows
                            .into_iter()
                            .map(|m| {
                                let id = m.id;
                                // use_navigate is captured at view-build time (Router context is
                                // live here); the callback just invokes the stored navigator.
                                let navigate = leptos_router::hooks::use_navigate();
                                view! {
                                    <ListDetailItem
                                        active=m.id == active_id
                                        title=view! { {m.title} }.into_any()
                                        on_click=Callback::new(move |()| {
                                            navigate(&format!("/wiki/{id}"), Default::default());
                                        })
                                    />
                                }
                            })
                            .collect_view()}
                    </div>
                </div>
            })
        })
        .collect_view()
}

fn article(
    m: &'static Manual,
    mode: RwSignal<WikiMode>,
    edits: RwSignal<std::collections::HashMap<&'static str, String>>,
) -> impl IntoView {
    let id = m.id;
    let body = m.body;
    view! {
        <section class="flex h-full min-w-0 flex-1 flex-col overflow-hidden">
            <header class="flex shrink-0 items-start justify-between gap-4 border-b border-white/10 px-8 pt-8 pb-5 md:px-12">
                <div class="min-w-0">
                    <div class="mb-3 flex items-center gap-2">
                        <span class=BADGE_NEUTRAL>
                            <span class="material-symbols-outlined text-[14px]">"schedule"</span>
                            "Last updated "
                            {m.updated}
                        </span>
                        <span class="font-mono text-xs tracking-widest text-outline uppercase">
                            {m.category}
                        </span>
                    </div>
                    <h1 class="text-4xl font-bold tracking-tight text-white">{m.title}</h1>
                </div>
                {read_edit_toggle(mode)}
            </header>
            {move || {
                if mode.get() == WikiMode::Edit {
                    // Distraction-free Markdown editor (React parity, T-172 H7): session-local
                    // per-manual edits. Initial value read untracked so keystrokes never rebuild
                    // the textarea (focus survives); the read branch tracks edits and re-renders.
                    let initial = edits
                        .with_untracked(|e| e.get(id).cloned())
                        .unwrap_or_else(|| body.to_string());
                    view! {
                        <textarea
                            prop:value=initial
                            spellcheck="false"
                            on:input=move |ev| {
                                edits
                                    .update(|e| {
                                        e.insert(id, event_target_value(&ev));
                                    })
                            }
                            class="h-full w-full flex-1 resize-none border-none bg-transparent p-8 font-mono text-sm leading-relaxed text-on-surface-variant outline-none focus:ring-0 md:p-12"
                        ></textarea>
                    }
                        .into_any()
                } else {
                    let source = edits
                        .with(|e| e.get(id).cloned())
                        .unwrap_or_else(|| body.to_string());
                    view! {
                        <article class="custom-scrollbar flex-1 overflow-y-auto p-8 md:p-12">
                            <div class="max-w-3xl">{render_markdown(&source)}</div>
                        </article>
                    }
                        .into_any()
                }
            }}
        </section>
    }
}

/// The `[ READ ]` / `[ EDIT ]` pill toggle — live (T-172 H7). Class strings per state are
/// byte-identical to the gate-era static render (read active by default).
fn read_edit_toggle(mode: RwSignal<WikiMode>) -> impl IntoView {
    let btn = |m: WikiMode, label: &'static str| {
        view! {
            <button
                type="button"
                class=move || {
                    if mode.get() == m {
                        "rounded-full px-3 py-1 font-medium transition-all bg-surface-glass text-on-surface shadow-md"
                    } else {
                        "rounded-full px-3 py-1 font-medium transition-all text-on-surface-variant hover:text-on-surface"
                    }
                }
                on:click=move |_| mode.set(m)
            >
                {label}
            </button>
        }
    };
    view! {
        <div class="inline-flex shrink-0 gap-1 rounded-full border border-white/5 bg-black/20 p-1 font-mono text-xs">
            {btn(WikiMode::Read, "[ READ ]")}
            {btn(WikiMode::Edit, "[ EDIT ]")}
        </div>
    }
}

#[cfg(test)]
mod tests {
    use super::{resolve_wiki_selection, MANUALS};

    #[test]
    fn slug_resolution() {
        assert_eq!(resolve_wiki_selection(None).id, "plan-timeline");
        assert_eq!(resolve_wiki_selection(Some("lead-role")).id, "lead-role");
        assert_eq!(resolve_wiki_selection(Some("nope")).id, "plan-timeline");
    }

    #[test]
    fn all_manuals_have_bodies() {
        // T-172 A4: selection is only honest if every manual renders a real article.
        for m in MANUALS {
            assert!(!m.body.is_empty(), "{} body empty", m.id);
        }
    }
}
