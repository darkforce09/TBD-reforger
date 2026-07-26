//! Global Leaderboards (/leaderboards) — ported from pages/operations.tsx `LeaderboardsPage`.
//! `<AuthGate>` → a `/leaderboards` Resource keyed on (category, search) → 3-tier podium + roster.
//! Header + segmented category control + search input are always on and live **outside** the
//! `Transition`, so the input keeps focus across the refetch every keystroke fires.
//!
//! **T-195 — the mock is gone.** This page used to render an eight-row fabricated ladder
//! (`MOCK`: Reaper/Wraith/Havoc/Cobra/…) whenever the API returned no rows, and — through an
//! inverted branch, `if real_empty { ranked("kd") } else { Vec::new() }` — threw the *real* rows
//! away when there were any, so a populated ladder rendered "No operators match your search."
//! Both halves were wrong in the same direction: invented statistics displayed as live telemetry
//! under a subtitle that says "Real-time tactical performance metrics". There is no honest gate
//! for that (a "demo data" ribbon would still be a leaderboard of people who do not exist), so the
//! fixture is deleted outright and an empty board now says it is empty. The two empty states are
//! distinguished: a search with no hits vs. a ladder with nothing in it yet.
//!
//! **Server-side category + search.** `GET /leaderboards` already accepts `?category=` (whitelisted
//! → `ORDER BY`) and `?q=` (username `ILIKE`), so the five tabs and the search box re-key the
//! Resource rather than re-sorting/filtering a client array. Ordering therefore comes from the one
//! place that can see all the rows, not just the ≤50 the page was served.
//!
//! **Dossier.** `[ VIEW DOSSIER ]` and the roster rows open a slide-over backed by
//! `GET /users/{discord_id}/stats` — which until now had no caller anywhere in the SPA. It renders
//! in a `ui::Sheet`; no new route and no new shared component. The response is read as
//! `serde_json::Value` for the same reason the board rows are: `dto.rs` types the board as
//! `{category, data: Vec<Value>}` and typing the row is a `dto.rs` change (R-api golden), which is
//! not this file.
#![allow(dead_code)]
use crate::dto::Leaderboard;
use crate::ui::{cn, MaterialIcon, PageHeader, Sheet};
use leptos::prelude::*;
use serde_json::Value;

/// One ranked operator, as rendered. Mirrors the backend `LeaderboardRow` for the fields this page
/// shows; the dossier reads the full stat card straight off `/users/{id}/stats`.
#[derive(Clone)]
struct Row {
    rank: i64,
    discord_id: String,
    username: String,
    avatar_url: String,
    kills: i64,
    kd_ratio: f64,
    team_kills: i64,
    command_win_rate: f64,
    missions_played: i64,
    longest_kill_m: i64,
}

const LEADERBOARD_TABS: [(&str, &str); 5] = [
    ("K/D Ratio", "kd"),
    ("Command Win Rate", "command_win"),
    ("Missions Played", "missions"),
    ("Longest Kill", "longest_kill"),
    ("Wall of Shame", "team_kills"),
];

fn v_str(v: &Value, key: &str) -> String {
    v.get(key)
        .and_then(Value::as_str)
        .unwrap_or_default()
        .to_string()
}

fn v_i64(v: &Value, key: &str) -> i64 {
    v.get(key).and_then(Value::as_i64).unwrap_or(0)
}

fn v_f64(v: &Value, key: &str) -> f64 {
    v.get(key).and_then(Value::as_f64).unwrap_or(0.0)
}

/// Wire row → [`Row`]. The handler ranks server-side (`offset + i + 1`) and the board is already in
/// category order, so `rank` is taken from the payload; the positional fallback only covers a row
/// that arrived without one, which would otherwise render every operator as `#0`.
fn parse_row(v: &Value, index: usize) -> Row {
    let rank = v
        .get("rank")
        .and_then(Value::as_i64)
        .filter(|&r| r > 0)
        .unwrap_or(index as i64 + 1);
    Row {
        rank,
        discord_id: v_str(v, "discord_id"),
        username: v_str(v, "username"),
        avatar_url: v_str(v, "avatar_url"),
        kills: v_i64(v, "kills"),
        kd_ratio: v_f64(v, "kd_ratio"),
        team_kills: v_i64(v, "team_kills"),
        command_win_rate: v_f64(v, "command_win_rate"),
        missions_played: v_i64(v, "missions_played"),
        longest_kill_m: v_i64(v, "longest_kill_m"),
    }
}

/// (primary, secondary, accent) per category — mirrors statFor().
fn stat_for(r: &Row, category: &str) -> (String, String, &'static str) {
    match category {
        "command_win" => (
            format!("{:.0}%", r.command_win_rate),
            format!("{} Ops", r.missions_played),
            "text-success",
        ),
        "missions" => (
            format!("{}", r.missions_played),
            format!("{} Kills", r.kills),
            "text-primary",
        ),
        "longest_kill" => (
            format!("{}m", r.longest_kill_m),
            format!("{} Kills", r.kills),
            "text-tactical-yellow",
        ),
        "team_kills" => (
            format!("{}", r.team_kills),
            format!("{} Ops", r.missions_played),
            "text-error-alert",
        ),
        _ => (
            format!("{:.2}", r.kd_ratio),
            format!("{} Kills", r.kills),
            "text-success",
        ),
    }
}

/// Podium tier styling (avatar, ring, badge, score, order) for ranks 1/2/3.
fn tier(
    rank: i64,
) -> (
    &'static str,
    &'static str,
    &'static str,
    &'static str,
    &'static str,
) {
    match rank {
        1 => (
            "h-32 w-32",
            "border-tactical-yellow shadow-[0_0_50px_rgba(250,204,21,0.5)]",
            "bg-tactical-yellow text-black",
            "text-4xl text-tactical-yellow",
            "order-2",
        ),
        2 => (
            "h-24 w-24",
            "border-slate-300 shadow-[0_0_35px_rgba(203,213,225,0.45)]",
            "bg-slate-300 text-black",
            "text-2xl text-slate-200",
            "order-1",
        ),
        _ => (
            "h-20 w-20",
            "border-orange-400 shadow-[0_0_30px_rgba(251,146,60,0.45)]",
            "bg-orange-400 text-black",
            "text-xl text-orange-300",
            "order-3",
        ),
    }
}

/// First letters of the first two words — the personnel-roster fallback glyph.
fn initials(name: &str) -> String {
    let mut out = String::new();
    for word in name.split_whitespace().take(2) {
        if let Some(c) = word.chars().next() {
            out.extend(c.to_uppercase());
        }
    }
    if out.is_empty() {
        "?".to_string()
    } else {
        out
    }
}

/// The operator's Discord avatar, or their initials when the row has none. The handler `COALESCE`s
/// a missing `users.avatar_url` to `""` — the mock never did, so `<img src="">` (a broken-image
/// glyph on every such row) only became reachable once real rows started rendering.
fn avatar(url: &str, username: &str, class: &str) -> impl IntoView {
    if url.is_empty() {
        let c = cn(&[
            "flex items-center justify-center bg-gradient-to-br from-primary/40 to-tertiary/30 font-semibold text-on-surface",
            class,
        ]);
        view! { <span class=c>{initials(username)}</span> }.into_any()
    } else {
        view! { <img src=url.to_string() alt="" class=class.to_string() /> }.into_any()
    }
}

#[component]
pub fn LeaderboardsPage() -> impl IntoView {
    view! {
        <crate::ui::AuthGate>
            <LeaderboardsInner />
        </crate::ui::AuthGate>
    }
}

#[component]
fn LeaderboardsInner() -> impl IntoView {
    let store = expect_context::<crate::auth::AuthStore>();
    let category = RwSignal::new("kd");
    let query = RwSignal::new(String::new());
    // Slide-over dossier: the clicked row (for the header) + the Sheet's open flag.
    let sheet_open = RwSignal::new(false);
    let selected = RwSignal::new(None::<Row>);

    // Keyed on both controls: a tab click re-orders server-side (`ORDER BY` off the whitelist) and
    // a keystroke re-filters server-side (`username ILIKE`). Neither is a client-side re-sort, so
    // the ranking stays correct across the handler's page limit instead of only within it.
    let board = LocalResource::new(move || {
        let cat = category.get();
        let q = query.get();
        async move {
            #[cfg(target_arch = "wasm32")]
            {
                let mut path = format!("/leaderboards?category={cat}");
                let trimmed = q.trim();
                if !trimmed.is_empty() {
                    path.push_str("&q=");
                    path.push_str(
                        &js_sys::encode_uri_component(trimmed)
                            .as_string()
                            .unwrap_or_default(),
                    );
                }
                crate::client::api_get::<Leaderboard>(store, &path)
                    .await
                    .ok()
            }
            #[cfg(not(target_arch = "wasm32"))]
            {
                let _ = (store, cat, q);
                None::<Leaderboard>
            }
        }
    });

    let open_dossier = move |row: Row| {
        selected.set(Some(row));
        sheet_open.set(true);
    };

    view! {
        <div class="bg-topo-map bg-grid-overlay h-full w-full overflow-hidden">
            <div class="custom-scrollbar flex h-full w-full flex-col overflow-y-auto bg-surface-glass p-6 backdrop-blur-xl md:p-10">
                <PageHeader
                    title="Global Leaderboards"
                    subtitle="Real-time tactical performance metrics across all active theaters."
                />
                <div class="flex flex-wrap items-center justify-between gap-4">
                    <div class="flex w-max rounded-full border border-white/5 bg-black/20 p-1">
                        {LEADERBOARD_TABS
                            .iter()
                            .map(|(label, cat)| {
                                let cat = *cat;
                                // cn(): twMerge drops the base `text-label-md` against the trailing
                                // text-{color}, so it's omitted here (unlike plain-string classes).
                                let class = move || {
                                    if category.get() == cat {
                                        "rounded-full px-6 py-1.5 transition-colors bg-white/10 text-white shadow-sm"
                                    } else {
                                        "rounded-full px-6 py-1.5 transition-colors text-on-surface-variant hover:bg-white/5 hover:text-on-surface"
                                    }
                                };
                                view! {
                                    <button
                                        type="button"
                                        on:click=move |_| category.set(cat)
                                        class=class
                                    >
                                        {*label}
                                    </button>
                                }
                            })
                            .collect_view()}
                    </div>
                    <input
                        type="search"
                        placeholder="Search operators..."
                        // value="" attribute at rest = React controlled-input parity (frozen V).
                        value=""
                        prop:value=move || query.get()
                        on:input=move |ev| query.set(event_target_value(&ev))
                        class="w-full max-w-xs rounded-full border border-white/10 bg-black/20 px-4 py-2 text-sm text-on-surface placeholder:text-on-surface-variant"
                    />
                </div>
                // Transition, not Suspense: a refetch keeps the resolved board on screen instead of
                // collapsing the page to "Loading…" on every keystroke.
                <Transition fallback=move || {
                    view! { <p class="mt-8 text-on-surface-variant">"Loading…"</p> }
                }>
                    {move || {
                        board
                            .get()
                            .map(|opt| match opt {
                                Some(board) => {
                                    let rows: Vec<Row> = board
                                        .data
                                        .iter()
                                        .enumerate()
                                        .map(|(i, v)| parse_row(v, i))
                                        .collect();
                                    board_body(
                                            rows,
                                            category.get(),
                                            query.get().trim().is_empty(),
                                            open_dossier,
                                        )
                                        .into_any()
                                }
                                None => {
                                    view! {
                                        <p class="mt-8 text-error">"Failed to load the leaderboard."</p>
                                    }
                                        .into_any()
                                }
                            })
                    }}
                </Transition>
            </div>
        </div>

        // Slide-over operator dossier (no full-page navigation; no new route).
        <Sheet open=sheet_open title="Operator Dossier">
            {move || { selected.get().map(|row| view! { <OperatorDossier row=row /> }) }}
        </Sheet>
    }
}

/// Podium + roster, or the matching empty state. `searchless` separates "your query matched nobody"
/// from "the ladder has nothing in it" — the pre-T-195 page showed the search message for both, and
/// (via the inverted branch) for a fully populated board as well.
fn board_body(
    rows: Vec<Row>,
    category: &'static str,
    searchless: bool,
    open: impl Fn(Row) + Copy + 'static,
) -> impl IntoView {
    if rows.is_empty() {
        let msg = if searchless {
            "No ranked operators yet — telemetry has not reported any tracked matches."
        } else {
            "No operators match your search."
        };
        return view! { <p class="mt-8 text-on-surface-variant">{msg}</p> }.into_any();
    }
    let podium: Vec<Row> = rows.iter().take(3).cloned().collect();
    let rest: Vec<Row> = rows.iter().skip(3).cloned().collect();
    view! {
        <>
            <div class="flex flex-row items-end justify-center gap-8 pt-16 pb-12 md:gap-16">
                {podium.iter().map(|p| podium_place(p, category, open)).collect_view()}
            </div>
            {(!rest.is_empty())
                .then(|| {
                    view! {
                        <div class="mt-2 flex flex-col gap-0.5 border-t border-white/5 pt-4">
                            {rest.iter().map(|r| roster_row(r, category, open)).collect_view()}
                        </div>
                    }
                })}
        </>
    }
    .into_any()
}

fn podium_place(r: &Row, category: &str, open: impl Fn(Row) + Copy + 'static) -> impl IntoView {
    let (avatar_size, ring, badge, score, order) = tier(r.rank);
    let (primary, secondary, _accent) = stat_for(r, category);
    let outer = cn(&["flex flex-col items-center", order]);
    let img_class = cn(&["rounded-xl border-2 object-cover", avatar_size, ring]);
    let badge_class = cn(&[
        "absolute -bottom-3 left-1/2 -translate-x-1/2 rounded-full px-3 py-0.5 text-xs font-bold",
        badge,
    ]);
    let score_class = cn(&["mt-1 font-bold drop-shadow-md", score]);
    let is_first = r.rank == 1;
    let glyph = avatar(&r.avatar_url, &r.username, &img_class);
    let dossier_row = r.clone();
    view! {
        <div class=outer>
            <div class="relative">
                {glyph} <span class=badge_class>"#"{r.rank}</span>
            </div>
            <p class="mt-6 text-label-md font-semibold text-on-surface">{r.username.clone()}</p>
            <p class=score_class>{primary}</p>
            <span class="text-label-sm text-on-surface-variant">{secondary}</span>
            {is_first
                .then(|| {
                    view! {
                        <button
                            type="button"
                            on:click=move |_| open(dossier_row.clone())
                            class="mt-3 font-mono text-[11px] tracking-widest text-tactical-yellow/80 transition-colors hover:text-tactical-yellow"
                        >
                            "[ VIEW DOSSIER ]"
                        </button>
                    }
                })}
        </div>
    }
}

fn roster_row(r: &Row, category: &str, open: impl Fn(Row) + Copy + 'static) -> impl IntoView {
    let (primary, secondary, accent) = stat_for(r, category);
    let primary_class = cn(&["w-16 text-right font-mono font-semibold", accent]);
    let glyph = avatar(
        &r.avatar_url,
        &r.username,
        "h-8 w-8 shrink-0 rounded-full object-cover",
    );
    // The row already advertised itself as clickable (cursor-pointer + a trailing chevron) while
    // doing nothing; it opens the same dossier the podium button does.
    let dossier_row = r.clone();
    view! {
        <div
            class="group flex cursor-pointer items-center gap-4 rounded-lg px-2 py-3 transition-colors hover:bg-white/5"
            on:click=move |_| open(dossier_row.clone())
        >
            <span class="w-8 shrink-0 font-mono text-sm text-on-surface-variant">
                {format!("{:02}", r.rank)}
            </span>
            {glyph}
            <span class="flex-1 truncate text-label-md font-medium text-on-surface">
                {r.username.clone()}
            </span>
            <span class="hidden text-sm text-on-surface-variant sm:inline">{secondary}</span>
            <span class=primary_class>{primary}</span>
            <MaterialIcon name="chevron_right" class="text-on-surface-variant group-hover:text-white" />
        </div>
    }
}

/* ───────────── Operator dossier (GET /users/{discord_id}/stats) ───────────── */

/// The stat card behind `[ VIEW DOSSIER ]`. Read as `Value`: the response is
/// `{stats: LeaderboardRow, total_operations, attendance_rate}` and there is no DTO for it (adding
/// one is a `dto.rs` + R-api-golden change, i.e. a different file than this slice owns).
#[component]
fn OperatorDossier(row: Row) -> impl IntoView {
    let store = expect_context::<crate::auth::AuthStore>();
    let id = StoredValue::new(row.discord_id.clone());
    let stats = LocalResource::new(move || {
        let id = id.get_value();
        async move {
            #[cfg(target_arch = "wasm32")]
            {
                let path = format!("/users/{id}/stats");
                crate::client::api_get::<Value>(store, &path).await.ok()
            }
            #[cfg(not(target_arch = "wasm32"))]
            {
                let _ = (store, id);
                None::<Value>
            }
        }
    });
    let header_glyph = avatar(
        &row.avatar_url,
        &row.username,
        "h-14 w-14 shrink-0 rounded-full object-cover",
    );
    let name = row.username.clone();
    let rank = row.rank;
    view! {
        <div class="flex items-center gap-4">
            {header_glyph}
            <div class="min-w-0">
                <p class="truncate text-headline-sm text-on-surface">{name}</p>
                <p class="font-mono text-label-sm text-on-surface-variant">
                    {format!("RANK #{rank}")}
                </p>
            </div>
        </div>
        <Suspense fallback=move || {
            view! { <p class="mt-6 text-on-surface-variant">"Loading…"</p> }
        }>
            {move || {
                stats
                    .get()
                    .map(|opt| match opt {
                        Some(body) => dossier_stats(&body).into_any(),
                        None => {
                            view! {
                                <p class="mt-6 text-error">"Failed to load this operator's record."</p>
                            }
                                .into_any()
                        }
                    })
            }}
        </Suspense>
    }
}

fn stat_tile(label: &'static str, value: String) -> impl IntoView {
    view! {
        <div class="rounded-lg border border-white/5 bg-black/20 px-4 py-3">
            <p class="text-label-sm text-on-surface-variant">{label}</p>
            <p class="mt-1 font-mono text-lg font-semibold text-on-surface">{value}</p>
        </div>
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The exact row `GET /api/v1/leaderboards` served off the dev stack (T-195 verify, seeded
    /// `leaderboard_totals`). It pins [`parse_row`] to the handler's `LeaderboardRow` field names:
    /// the populated board had no coverage of any kind before this slice, because the page threw
    /// real rows away before they reached a renderer.
    const WIRE_ROW: &str = r#"{"avatar_url":"","command_win_rate":0.0,"command_wins":0,
        "deaths":0,"discord_id":"000000000000000001","kd_ratio":1.5,"kills":3,"longest_kill_m":412,
        "missions_played":1,"rank":1,"team_kills":2,"username":"Dev Operator",
        "vehicles_destroyed":0}"#;

    fn wire() -> Value {
        serde_json::from_str(WIRE_ROW).expect("fixture parses")
    }

    #[test]
    fn parse_row_reads_every_rendered_field() {
        let r = parse_row(&wire(), 0);
        assert_eq!(r.rank, 1);
        assert_eq!(r.discord_id, "000000000000000001");
        assert_eq!(r.username, "Dev Operator");
        assert_eq!(r.avatar_url, "");
        assert_eq!(r.kills, 3);
        assert_eq!(r.kd_ratio, 1.5);
        assert_eq!(r.team_kills, 2);
        assert_eq!(r.missions_played, 1);
        assert_eq!(r.longest_kill_m, 412);
    }

    #[test]
    fn parse_row_prefers_the_server_rank_over_position() {
        // Page 2 of the board: the handler ranks `offset + i + 1`, so row 0 is #21, not #1.
        let mut v = wire();
        v["rank"] = Value::from(21);
        assert_eq!(parse_row(&v, 0).rank, 21);
    }

    #[test]
    fn parse_row_falls_back_to_position_when_rank_is_missing_or_zero() {
        // Without the fallback every operator renders as "#0" / "00".
        let mut absent = wire();
        absent.as_object_mut().expect("object").remove("rank");
        assert_eq!(parse_row(&absent, 2).rank, 3);
        let mut zero = wire();
        zero["rank"] = Value::from(0);
        assert_eq!(parse_row(&zero, 2).rank, 3);
    }

    #[test]
    fn parse_row_survives_a_row_missing_its_stats() {
        let r = parse_row(&Value::Null, 0);
        assert_eq!(r.rank, 1);
        assert_eq!(r.username, "");
        assert_eq!(r.kills, 0);
    }

    #[test]
    fn initials_cover_the_empty_avatar_url_the_api_actually_sends() {
        assert_eq!(initials("Dev Operator"), "DO");
        assert_eq!(initials("Reaper"), "R");
        assert_eq!(initials("a b c"), "AB");
        assert_eq!(initials(""), "?");
    }
}

fn dossier_stats(body: &Value) -> impl IntoView {
    let s = body.get("stats").cloned().unwrap_or(Value::Null);
    let tiles = vec![
        stat_tile("Kills", v_i64(&s, "kills").to_string()),
        stat_tile("Deaths", v_i64(&s, "deaths").to_string()),
        stat_tile("K/D Ratio", format!("{:.2}", v_f64(&s, "kd_ratio"))),
        stat_tile("Team Kills", v_i64(&s, "team_kills").to_string()),
        stat_tile("Longest Kill", format!("{}m", v_i64(&s, "longest_kill_m"))),
        stat_tile(
            "Vehicles Destroyed",
            v_i64(&s, "vehicles_destroyed").to_string(),
        ),
        stat_tile("Missions Played", v_i64(&s, "missions_played").to_string()),
        stat_tile("Command Wins", v_i64(&s, "command_wins").to_string()),
        stat_tile(
            "Command Win Rate",
            format!("{:.0}%", v_f64(&s, "command_win_rate")),
        ),
        stat_tile(
            "Total Operations",
            v_i64(body, "total_operations").to_string(),
        ),
        stat_tile(
            "Attendance",
            format!("{:.0}%", v_f64(body, "attendance_rate")),
        ),
    ];
    view! { <div class="mt-6 grid grid-cols-2 gap-3">{tiles}</div> }
}
