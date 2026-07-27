//! T-159.21 — Eden chrome scaffold for the Mission Creator (/missions/:id/edit).
//!
//! The docked shell React renders around the map: a Top Command Strip (title, Undo/Redo, the
//! T-159.20 Save/Export controls, a disabled Settings stub), a Bottom Toolbelt (Select + CUR/SEL/OBJ
//! readout), and left/right dock placeholders. This slice is the **scaffold**: the docks hold
//! placeholder text only — the outliner tree and asset palette land in T-159.22 (spec C4/C7).
//!
//! **Layering (React MissionCreatorPage:272):** the chrome overlays a full-bleed canvas; it never
//! shrinks it. Every `select_tool` probe builds its camera from the container's bounding rect, so a
//! resized container would silently invalidate the pan/select/marquee/move gates. The panels are
//! absolutely positioned inside the gesture container instead, and the host div stops `pointerdown`
//! from bubbling into the map handlers (see `mission_editor`'s view).
//!
//! **Not cfg-gated:** the components compile on the native target too (the `cargo check -p
//! website-frontend` shell). Nothing here touches a wasm-only type — the doc-driving `on:click` bodies
//! are `#[cfg(target_arch = "wasm32")]` inside the closure, the T-159.20 Save-button precedent.
#![allow(dead_code)]
use leptos::prelude::*;

// T-192 fix — the row mirror's debounce + single-flight state. Gated because only the wasm build has
// a `setTimeout` to hang a debounce on; the native view shell compiles the components without them.
#[cfg(target_arch = "wasm32")]
use std::cell::RefCell;
#[cfg(target_arch = "wasm32")]
use std::collections::HashMap;

use crate::asset_catalog::{CatalogNode, CatalogState};
use crate::outliner::{flatten_visible, FlatRow, NodeKind, OutlinerNode, VIRTUAL_SLOT_THRESHOLD};
use crate::ui::{badge_class, MaterialIcon};

// ── Chrome insets (CSS px) ───────────────────────────────────────────────────────────────────────
// These ARE the source the Tailwind utilities in `mission_editor`'s view are written from, and
// `select_tool::farthest_empty_px` insets its probe grid by them so a "guaranteed-empty" click px
// can never land under a panel that would swallow the pointerdown. Change a class → change the
// const (and vice versa) — they are one contract, verified by the select + marquee gates.

/// Top Command Strip height — `h-12` / the docks' `top-12`.
pub const STRIP_TOP_PX: f64 = 48.0;
/// Left dock width — `w-64`.
pub const DOCK_LEFT_PX: f64 = 256.0;
/// Right dock width — `w-80`.
pub const DOCK_RIGHT_PX: f64 = 320.0;
/// Bottom band reserved for the toolbelt. It floats (`bottom-5` ≈ 20 px + ~44 px tall) rather than
/// docking full-width, so this is a generous band, not an exact height.
pub const TOOLBELT_BAND_PX: f64 = 96.0;

// ── Class recipes ────────────────────────────────────────────────────────────────────────────────
// Ported from React `features/mission-creator/layout/overlay.ts`. The `cn(recipe, '…')` call sites
// are pre-merged into literals here (the `mortar.rs` idiom — `ui::cn` is a naive joiner and can't be
// `const`); each merge below is conflict-free, so the concatenation IS what tailwind-merge yields.

/// React `overlayPanel`, verbatim.
const OVERLAY_PANEL: &str = "pointer-events-auto rounded-xl border border-white/10 bg-surface-container-lowest/55 shadow-xl backdrop-blur-xl";
/// React `overlayDocked`, verbatim.
const OVERLAY_DOCKED: &str =
    "pointer-events-auto bg-surface-container-lowest/55 shadow-xl backdrop-blur-xl";

/// `cn(overlayDocked, 'flex h-full items-center gap-2 border-b border-white/10 px-3')`.
const STRIP: &str = "pointer-events-auto bg-surface-container-lowest/55 shadow-xl backdrop-blur-xl flex h-full items-center gap-2 border-b border-white/10 px-3";
/// `cn(overlayDocked, …)` + the dock's own edge border.
const DOCK_L: &str = "pointer-events-auto bg-surface-container-lowest/55 shadow-xl backdrop-blur-xl flex h-full flex-col overflow-y-auto border-r border-white/10 p-3";
const DOCK_R: &str = "pointer-events-auto bg-surface-container-lowest/55 shadow-xl backdrop-blur-xl h-full overflow-y-auto border-l border-white/10 p-3";
/// `cn(overlayPanel, 'flex items-center gap-1 px-1.5 py-1.5')`.
const TOOLBELT: &str = "pointer-events-auto rounded-xl border border-white/10 bg-surface-container-lowest/55 shadow-xl backdrop-blur-xl flex items-center gap-1 px-1.5 py-1.5";

/// The shared icon-button recipe (React TopCommandStrip:148).
const BTN_ICON: &str = "rounded-md p-1.5 text-on-surface-variant transition-colors hover:bg-white/10 disabled:opacity-30 disabled:hover:bg-transparent";
/// A vertical hairline divider (React `<span className="h-5 w-px bg-white/10" />`).
const DIVIDER: &str = "h-5 w-px bg-white/10";

/// A toolbelt tool button — active (Select) vs disabled stub (Ruler / LoS).
const TOOL_ACTIVE: &str =
    "flex items-center gap-1.5 rounded-lg px-2.5 py-1.5 text-label-md transition-colors bg-primary/20 text-primary";
const TOOL_DISABLED: &str = "flex items-center gap-1.5 rounded-lg px-2.5 py-1.5 text-label-md transition-colors text-on-surface-variant opacity-30 hover:bg-transparent";

/// Format a cursor axis for the mono readout. React `BottomToolbelt.fmtCoord`:
/// `n.toFixed(3).padStart(9, ' ')`, and the off-map cell is 7 spaces + an em dash. HTML collapses
/// the leading runs in both engines — `tabular-nums` does the real aligning — so this mirrors the
/// oracle rather than "fixing" it.
fn fmt_coord(v: Option<f64>) -> String {
    match v {
        Some(n) => format!("{n:>9.3}"),
        None => "       —".to_string(),
    }
}

// Top Command Strip (T-172 B9) — menu bar · editable title · time scrubber + weather ·
// History (disabled) · Undo/Redo · Save dialog · Export · Settings.

/// One top-strip menu (T-172 B9). React rendered File/Edit/View/Mission/Environment as dead
/// "(soon)" stubs; these open real dropdowns with the commands that exist. No DOM while closed.
struct MenuItem {
    label: &'static str,
    /// None = disabled row (rendered, not clickable — parity with genuinely-future features).
    action: Option<MenuAction>,
}

#[derive(Clone, Copy)]
enum MenuAction {
    Save,
    /// The editor SUPERSET envelope (`MissionExport`) — re-importable, not loadable by the mod.
    Export,
    /// T-243 — the compiled mod document, the bytes `GET /missions/:id/compiled` serves a game
    /// server. A separate action rather than a replacement for [`MenuAction::Export`]: the two
    /// files answer different questions and both have a caller.
    ExportCompiled,
    Undo,
    Redo,
    Settings,
}

const MENUS: [(&str, &[MenuItem]); 5] = [
    (
        "File",
        &[
            MenuItem {
                label: "Save Version…",
                action: Some(MenuAction::Save),
            },
            MenuItem {
                label: "Export JSON",
                action: Some(MenuAction::Export),
            },
            MenuItem {
                label: "Export Compiled Mission…",
                action: Some(MenuAction::ExportCompiled),
            },
        ],
    ),
    (
        "Edit",
        &[
            MenuItem {
                label: "Undo",
                action: Some(MenuAction::Undo),
            },
            MenuItem {
                label: "Redo",
                action: Some(MenuAction::Redo),
            },
        ],
    ),
    (
        "View",
        &[MenuItem {
            label: "Map layers — render host (T-159.28)",
            action: None,
        }],
    ),
    (
        "Mission",
        &[MenuItem {
            label: "Mission Settings…",
            action: Some(MenuAction::Settings),
        }],
    ),
    (
        "Environment",
        &[MenuItem {
            label: "Time & Weather (Mission Settings)…",
            action: Some(MenuAction::Settings),
        }],
    ),
];

/// Minutes-since-midnight ↔ `HH:MM` for the time scrubber (T-172 B9). Pure + tested.
pub fn minutes_to_hhmm(min: u32) -> String {
    format!("{:02}:{:02}", (min / 60) % 24, min % 60)
}

/// `HH:MM` → minutes since midnight; `None` when it is not a clock.
///
/// **T-192 — the trailing `:SS` is accepted on purpose.** `missions.time_of_day` is a Postgres
/// `time` selected as `::text`, so `mission_hydrate::apply_row_meta` writes `06:00:00` into
/// `meta.environment.time` on every load. The old `split_once` parse read the remainder as `00:00`,
/// failed, and silently parked the scrubber at the 06:00 default — so after a reload an author who
/// had set 21:45 saw the slider claim 06:00. That is the same "your setting was quietly reverted"
/// symptom this ticket exists to remove, on the same value.
pub fn hhmm_to_minutes(s: &str) -> Option<u32> {
    let mut parts = s.split(':');
    let h: u32 = parts.next()?.parse().ok()?;
    let m: u32 = parts.next()?.parse().ok()?;
    if let Some(sec) = parts.next() {
        let sec: u32 = sec.parse().ok()?;
        if sec > 59 {
            return None;
        }
    }
    if parts.next().is_some() || h > 23 || m > 59 {
        return None;
    }
    Some(h * 60 + m)
}

/// A clock from any of the controls (or from the row hydrate) → canonical `HH:MM`; `None` when it
/// is not one. The shape [`RowMirror::set_time`] sends to `PATCH /missions/{id}`.
pub fn normalize_clock(s: &str) -> Option<String> {
    hhmm_to_minutes(s).map(minutes_to_hhmm)
}

// ── meta.environment — the keys the editor is allowed to author (T-193) ──────────────────────────

/// Every `meta.environment` key the editor writes, paired with the surface that reads it back.
///
/// **Why a table with a gate on it, and not a comment.** Mission Settings shipped a View Distance
/// field and a Thermals toggle that wrote `meta.environment.{viewDistance,thermals}`. Both controls
/// *worked*: the value entered the document, took an undo step, survived a reload and came back when
/// the dialog reopened. Neither value ever left the editor.
///
/// The ticket filed this as a schema violation — `mission.schema.json` pins `environment` to
/// `dateTime` / `weatherPreset` / `windDirDeg` under `additionalProperties: false`. It is not one,
/// and that matters for the fix: `ModEnvironment` (`map-engine-core/src/mission/flatten.rs`) is a
/// fixed two-field struct built key by key, so the compiled document never carried the extra keys to
/// the schema in the first place. Nothing was ever rejected. The keys were dropped, in silence, on
/// the way out of the editor — which is the harder bug, because a rejection at least tells someone.
///
/// **Why they were removed rather than carried through.** There is no destination. The `missions`
/// row has no `view_distance` / `thermals` column, so the T-192 mirror cannot take them; the mod
/// document struct and the schema would both have to grow a field; and neither word appears anywhere
/// in `apps/mod` or `packages/tbd-schema` — the framework has no view-distance or thermals concept
/// to receive them, so even a widened schema would land the values in a document nothing reads. That
/// is a mod feature (`executor: workbench`), not an editor fix. Meanwhile the design corpus
/// (`engineering_plan.md`, `mission_creator_design.md`) has always described both as *auto-derived*
/// from the mission, never author-set. Two live controls for a setting nobody had planned to honour
/// is worse than no controls: the author sets a view distance, saves, and the mission runs at the
/// default with nothing said.
///
/// Every environment write in this file goes through [`author_env`], which refuses a key that is not
/// listed here. That is the part that makes this stay fixed — the next control cannot be wired to a
/// key with no reader without someone first adding the reader to this table.
const CARRIED_ENV_KEYS: &[(&str, &str)] = &[
    // Compiled AND mirrored: `mission_compile` prefers the saved payload's environment over the
    // row, and T-192 PATCHes the row so the library dossier cannot disagree with the editor.
    (
        "time",
        "compiled `environment.dateTime` + the `missions.time_of_day` column",
    ),
    (
        "weather",
        "compiled `environment.weatherPreset` + the `missions.weather` column",
    ),
    // Editor-local: per-mission render prefs applied live to the map host. These never compile, and
    // that is correct — they describe how the AUTHOR looks at the map, not how the mission runs.
    (
        "showHillshade",
        "the editor's map host (`world_assets::apply_hillshade`)",
    ),
    (
        "hillshadeOpacity",
        "the editor's map host (`world_assets::apply_hillshade`)",
    ),
    (
        "showGrid",
        "the editor's map host (`world_assets::apply_grid`)",
    ),
];

/// Does any surface read `key` back? See [`CARRIED_ENV_KEYS`] and [`AUTHORED_FLOW_KEYS`].
fn env_key_is_carried(key: &str) -> bool {
    CARRIED_ENV_KEYS.iter().any(|(k, _)| *k == key)
        || AUTHORED_FLOW_KEYS.iter().any(|(k, _, _)| *k == key)
}

/// Write one `meta.environment` key into the document — one undo step, exactly as the controls did
/// before — or refuse it and say so.
///
/// The refusal is the whole point. A control wired straight at `editor_ops::update_environment`
/// cannot tell whether its value will ever be read again, which is precisely how View Distance and
/// Thermals shipped looking functional. The check belongs on the one path every control takes.
#[cfg(target_arch = "wasm32")]
fn author_env(key: &str, value: serde_json::Value) {
    if !env_key_is_carried(key) {
        leptos::logging::error!(
            "refusing to author meta.environment.{key}: no surface reads it back (see CARRIED_ENV_KEYS)"
        );
        return;
    }
    let mut patch = serde_json::Map::new();
    patch.insert(key.to_string(), value);
    crate::editor_ops::update_environment(serde_json::Value::Object(patch).to_string());
}

/// What Mission Settings says where the View Distance field and the Thermals toggle used to be.
///
/// Pinned copy, because the blank is the problem: two controls vanishing from a dialog reads as a
/// regression unless the dialog says otherwise, and the one thing an author needs to know is that
/// the setting was never reaching the game — not that the UI got tidier.
pub const ENV_UNCARRIED_NOTE: &str =
    "View distance and thermals are not part of a compiled mission — it carries time and weather only.";

// ── The mission-flow block (T-224) ───────────────────────────────────────────────────────────────

/// The four `flow` fields the editor authors: the key it writes into the document, the compiled
/// document path that key becomes, and the mod symbol that reads it there.
///
/// **Why these four and not the other four the ticket names.** T-224 asks for six controls —
/// duration, respawn, spectator policy, NVG, tickets, JIP. Only two of those six reach a consumer
/// (duration = `flow.timeLimitSeconds`, and `jip`), so the block below is the two that do plus the
/// two remaining `flow` fields, which reach one for the same reason. The other four are refused, and
/// [`SETTINGS_UNREAD_NOTE`] is the dialog copy that says so. This is the T-193 rule applied to a new
/// block rather than a new exception to it: `mission.schema.json` declaring a field is not a reader,
/// and a control whose value stops at the editor boundary is worse than no control at all.
///
/// **Why the keys ride `meta.environment` and not a `meta.flow` sibling.** They are not the same
/// thing as the compiled document's `environment` block, and they are not meant to be — the third
/// column is what maps one to the other. `meta.environment` is the editor's per-mission settings
/// bag, and it already carries keys that compile elsewhere (`time` → `environment.dateTime`) or
/// nowhere at all (`showHillshade`, `showGrid` — editor-local render prefs, see
/// [`CARRIED_ENV_KEYS`]). The bag is the transport; the table is the contract.
///
/// A `meta.flow` sibling would read better and would not survive a reload. `compile_payload`
/// (`map-engine-core/src/mission/compile.rs`) builds the saved version out of exactly two meta
/// keys — `meta.terrain` and `meta.environment` — and `MissionDocCore::hydrate` restores exactly
/// those two. Anything written beside them is authored into the live document, dropped on Save, and
/// gone on the next load: a control that works until you reload, which is the shape of bug this file
/// has now spent three tickets removing. (That the compiler drops unrecognised top-level keys in
/// silence is its own ticket, T-219; this slice routes around it rather than depending on it.)
///
/// **The one hop that is still missing, stated plainly.** Every reader below is live in the mod
/// today, and the editor→mod chain is live for `meta.environment` up to the compiler: the saved
/// payload carries these keys out as top-level `environment` and `mission_compile.rs` already reads
/// that block for `time`/`weather`. What is NOT live is the last step — `ModFlow` in
/// `map-engine-core/src/mission/flatten.rs` splices in four hardcoded constants
/// ([`FLOW_DEFAULT_BRIEFING_S`] and friends are those constants, mirrored here) and never looks at
/// the payload. **`flatten.rs` is not this slice's file** — `docs/platform/wave_plan.tsv` hands it
/// to T-200 and T-204, and T-204 *is* this half's other half ("Emit mission flow and winConditions
/// instead of hardcoding them"). What it needs to read is the saved payload's top-level
/// `environment`, under exactly the four key names in the first column below. Until it lands, an
/// authored duration is stored, saved, reloaded and shown back correctly, and the compiled document
/// still says 5400. That is a partial, it is the half this file owns, and it is written down here
/// rather than discovered later.
const AUTHORED_FLOW_KEYS: &[(&str, &str, &str)] = &[
    (
        "briefingSeconds",
        "flow.briefingSeconds",
        "TBD_FrameworkManager.OnEnterBriefing — announces the briefing length on stage entry \
         (deliberately does not auto-advance the stage)",
    ),
    (
        "safeStartSeconds",
        "flow.safeStartSeconds",
        "TBD_FrameworkManager.ApplySafeStartSeconds → TBD_SafestartManager.AdminSetSeconds — the \
         real countdown length",
    ),
    (
        "timeLimitSeconds",
        "flow.timeLimitSeconds",
        "TBD_FrameworkManager.ArmRoundClock → SetStage(END); TBD_MissionValidator\
         .CheckTimeLimitReachable warns when a 'time_limit' win condition cannot fire without it",
    ),
    (
        "jip",
        "flow.jip",
        "TBD_FrameworkManager.JipPolicy → TBD_SpawnManager's JIP door \
         (TBD_MissionFlow.AllowsJoinAtStage)",
    ),
];

/// What Mission Settings says where a Respawn / Spectator / Night vision / Tickets control would be.
///
/// Pinned copy for the same reason as [`ENV_UNCARRIED_NOTE`]: an author who reads the ticket title,
/// opens the dialog and finds four of the six settings missing has to be told the difference between
/// "not built yet" and "the game does not read it". It is the second one.
/// `TBD_MissionDocumentStruct` (`TBD_MissionLoader.c`) declares no `settings` member and
/// `TBD_MissionFactionStruct` declares no `tickets`, and `JsonLoadContext` is a typed parser — a key
/// with no matching member is not rejected or logged, it is invisible. So all four would author
/// cleanly, validate cleanly, compile cleanly and change nothing about the round. The mod reader is
/// T-259 (`settings`); tickets has no ticket because TBD events are one life by design and the
/// framework has no respawn pool for a ticket count to size.
pub const SETTINGS_UNREAD_NOTE: &str = "Respawn, spectator policy, night vision and per-faction tickets are not authored here — the mission document declares them and no mod script reads them. TBD events are one life.";

/// What a mission runs with when nothing is authored. These four mirror the constants `ModFlow`
/// splices in today (`map-engine-core/src/mission/flatten.rs`), so an unauthored mission's dialog
/// shows the duration it will actually run with rather than a UI-invented zero. When the compiler
/// slice starts reading the authored keys, these stay as its fallback — if they ever disagree, the
/// dialog is lying about an unauthored mission.
pub const FLOW_DEFAULT_BRIEFING_S: i64 = 600;
/// See [`FLOW_DEFAULT_BRIEFING_S`].
pub const FLOW_DEFAULT_SAFESTART_S: i64 = 300;
/// See [`FLOW_DEFAULT_BRIEFING_S`].
pub const FLOW_DEFAULT_TIMELIMIT_S: i64 = 5400;
/// See [`FLOW_DEFAULT_BRIEFING_S`].
pub const FLOW_DEFAULT_JIP: &str = "until_safestart_end";

/// The `jip` enum, in schema order, with the words an author reads.
///
/// Pinned to `mission.schema.json#/$defs/flow/properties/jip` — three values, no others.
/// `TBD_MissionFlow.PolicyFromString` maps anything it does not recognise (including the empty
/// string an absent key decodes to) to `ALWAYS`, so a typo here would not fail loudly, it would
/// quietly hold the mission's door open for the whole round.
pub const JIP_OPTIONS: [(&str, &str); 3] = [
    ("disabled", "Disabled"),
    ("until_safestart_end", "Until safe start ends"),
    ("always", "Always"),
];

/// A duration box's committed value, or `None` when the box does not hold one.
///
/// **Refusing is the point.** `mission.schema.json` types every `flow` duration
/// `integer, minimum 0`, and a half-typed box passes through `""` and `"-"` on the way to `-1`.
/// Authoring those would put a non-integer or a negative in the document and turn one keystroke
/// into a schema-invalid compiled mission at `GET /missions/:id/compiled` — in front of a game
/// server rather than the author. Same contract as [`normalize_clock`]: commit a real value or
/// commit nothing at all.
///
/// `0` is deliberately accepted. It is a real authored value on every one of these fields, and on
/// `timeLimitSeconds` it is the ONLY way to say "no time limit" — `TBD_MissionValidator` reads a
/// `0` there as an explicit no-limit and warns about the `time_limit` win condition accordingly.
#[must_use]
pub fn parse_flow_seconds(s: &str) -> Option<i64> {
    let t = s.trim();
    if t.is_empty() {
        return None;
    }
    let n: i64 = t.parse().ok()?;
    (n >= 0).then_some(n)
}

/// `5400` → `"1 h 30 m"`. The human echo beside a seconds box.
///
/// **Why the box holds seconds and not minutes.** Seconds is the unit of the document, the schema
/// and every mod reader, so a seconds box is the only one that cannot round. A minutes box has to
/// divide on open, and an authored 5430 s (90.5 min) would come back as `90` or `91` — the dialog
/// silently rewriting a value the author never touched, which is the exact class of bug T-192 was
/// filed for. So the number in the box is the number in the document, and this renders what that
/// number means next to it.
#[must_use]
pub fn fmt_duration_secs(total: i64) -> String {
    if total < 0 {
        return String::new();
    }
    let (h, m, s) = (total / 3600, (total % 3600) / 60, total % 60);
    let mut out = String::new();
    if h > 0 {
        out.push_str(&format!("{h} h"));
    }
    if m > 0 {
        if !out.is_empty() {
            out.push(' ');
        }
        out.push_str(&format!("{m} m"));
    }
    if s > 0 || out.is_empty() {
        if !out.is_empty() {
            out.push(' ');
        }
        out.push_str(&format!("{s} s"));
    }
    out
}

/// One authored duration read back out of the document, or `default` when the mission has not
/// authored one. A key of the wrong type reads as unauthored rather than as `0` — the document is
/// shared and hydrated from a payload, so "someone wrote a string here" must not become "this
/// mission has no briefing".
#[cfg(target_arch = "wasm32")]
fn read_flow_seconds(key: &str, default: i64) -> i64 {
    crate::editor_ops::read_env_value(key)
        .as_ref()
        .and_then(serde_json::Value::as_i64)
        .filter(|n| *n >= 0)
        .unwrap_or(default)
}

/// The authored `jip` policy, or [`FLOW_DEFAULT_JIP`]. A value outside [`JIP_OPTIONS`] falls back to
/// the default rather than being shown: an unrecognised string in a `<select>` renders as no
/// selection at all, which reads as "unset" for a field that is very much set.
#[cfg(target_arch = "wasm32")]
fn read_flow_jip() -> String {
    crate::editor_ops::read_env_value("jip")
        .as_ref()
        .and_then(serde_json::Value::as_str)
        .filter(|v| JIP_OPTIONS.iter().any(|(k, _)| k == v))
        .unwrap_or(FLOW_DEFAULT_JIP)
        .to_string()
}

/// Is the route `:id` a real mission row? T-192.
///
/// The editor also mounts on synthetic ids — `mission_editor` falls back to `draft`, and the gate
/// route drives a smoke id — where a row PATCH is a guaranteed 400. Cheap shape check rather than a
/// `uuid` dependency the SPA does not otherwise carry.
fn is_mission_row_id(s: &str) -> bool {
    s.len() == 36
        && s.as_bytes().iter().enumerate().all(|(i, b)| match i {
            8 | 13 | 18 | 23 => *b == b'-',
            _ => b.is_ascii_hexdigit(),
        })
}

/// One `missions` column the editor mirrors, plus the name the author knows it by. T-192.
///
/// The label exists because a failure has to be reported in the user's vocabulary: they changed
/// "Time of day" on a slider, not `time_of_day` on a row.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
struct MirroredField {
    column: &'static str,
    label: &'static str,
}

const MIRROR_TIME: MirroredField = MirroredField {
    column: "time_of_day",
    label: "Time of day",
};
const MIRROR_WEATHER: MirroredField = MirroredField {
    column: "weather",
    label: "Weather",
};

/// How long a burst of authored values coalesces before the row PATCH goes out.
///
/// **Trailing edge only, on purpose.** A held time scrubber emits ~30 distinct values a second, and
/// the row only ever needs the one the author settles on; every intermediate value is already in the
/// document, and Save Version is the durable path. So the mirror waits for the hand to stop moving
/// rather than narrating the journey — one PATCH per settle instead of thirty per second, none of
/// which can then land out of order and leave the row holding a value the author scrubbed past.
const MIRROR_DEBOUNCE_MS: i32 = 400;

/// The toast a mirror PATCH raises when it does not land.
///
/// **Why a toast at all.** The shipped version only `warn!`ed. `PATCH /missions/{id}` gates on
/// *ownership* (`handlers/missions.rs` `can_edit` — author or admin) while the editor route gates on
/// *role* (`router.rs`, `mission_maker`), so a mission_maker who legitimately opens someone else's
/// **live** mission is refused every mirror PATCH. They watch the setting apply, reload, and
/// `mission_hydrate::apply_row_meta` writes the row back over the document — precisely the bug T-192
/// exists to remove, with the console as its only witness.
///
/// **Why two texts.** A 403 and a dropped connection ask different things of the user. The 403 is
/// structural: retrying cannot help, and neither can Save Version (`create_version` gates on the
/// same `can_edit`), so the only way to keep the change is to own the mission. Anything else — a
/// flaky connection, a restarting API — is worth another go, so it names what the server said.
///
/// Both say the setting will revert, because it will: the row still holds the old value, and the row
/// wins on the next hydrate.
fn mirror_failure_message(field: MirroredField, err: &crate::client::ApiErr) -> String {
    let label = field.label;
    if err.0 == 403 {
        return format!(
            "{label} was not saved — you are not this mission's author. It will revert when the editor reloads."
        );
    }
    format!(
        "Could not save {}: {}. It will revert when the editor reloads — try again.",
        label.to_lowercase(),
        crate::client::api_error_message(err, "the server did not respond")
    )
}

/// T-192 — mirrors an authored environment field onto the `missions` row.
///
/// **Why this exists.** The Mission Settings dialog and the top-strip scrubber below write
/// `meta.environment.{time,weather}` into the CRDT document and nowhere else. Nothing PATCHed the
/// row, and `mission_hydrate::apply_row_meta` re-applies the row over the document on every
/// hydrate — so the author's setting was dropped on the wire (the compile read the row) **and**
/// reverted locally on the next reload. The compile half is fixed in
/// `api/src/services/mission_compile.rs`, which now prefers the environment carried by the saved
/// payload; this is the half that stops the row from reverting it, and keeps the library dossier
/// (which renders the row's Weather/Time) telling the truth.
///
/// **It rides the same event that writes the document.** Not a later `change`: the Mission Settings
/// dialog re-runs its whole view closure on every `doc_tick`, and `update_environment` bumps
/// `doc_tick`, so a `change` handler on a control the rebuild may have re-created is exactly the
/// kind of thing that works until it doesn't — and its failure mode is this ticket's bug again,
/// silently. Mirroring from the doc-write handler makes "the row disagrees with the document"
/// unreachable rather than unlikely. The sequencing that keeps that flood sane lives in
/// [`MirrorState`], not here.
///
/// `Copy` so each control's handler can capture it. Built at component **setup**: `expect_context`,
/// `use_toasts` and `use_params_map` all resolve through the reactive owner, which a plain DOM event
/// handler does not have.
#[cfg(target_arch = "wasm32")]
#[derive(Clone, Copy)]
struct RowMirror {
    auth: crate::auth::AuthStore,
    mission_id: StoredValue<String>,
    /// Where a failed mirror is reported. Resolved at setup for the same reason `auth` is —
    /// `use_toasts()` is an `expect_context` and would panic from a DOM handler or a timer.
    toasts: crate::toast::Toasts,
}

/// Per-column mirror bookkeeping: the dedupe memory, the debounce queue, and the single-flight slot.
///
/// **Why it is a module singleton and not a field of [`RowMirror`].** `TopCommandStrip` and
/// `MissionSettingsDialog` each build their own handle and both write `time_of_day`. If the queue
/// lived in the handle those two would be independent sequencers and could put two PATCHes for the
/// same column on the wire at once — the exact out-of-order hazard this state exists to close. wasm
/// is single-threaded, so a `RefCell` map is the sound analogue of the shared mutable box (the
/// `yrs_persist` idiom).
///
/// **The three transitions below are the whole sequencer, and they are pure** — no timer, no
/// network, no `Toasts` — so the ordering guarantee is provable on the native `cargo test` shell
/// instead of only in a browser. [`RowMirror`] supplies the `setTimeout` and the PATCH around them.
#[derive(Default)]
struct MirrorState {
    /// Last value the row is believed to hold, so an unchanged commit costs nothing. Cleared on a
    /// failed PATCH so the next commit of the same value retries instead of assuming it landed.
    last: String,
    /// Newest authored value not yet on the wire. `None` = nothing waiting.
    pending: Option<String>,
    /// Bumped on every authored value. The response to generation `g` no longer speaks for the
    /// field once `generation != g` — the author has moved on and a successor is queued.
    generation: u64,
    /// The generation on the wire; `None` = idle. **Exactly one PATCH per column at a time**, which
    /// is what makes out-of-order landing unreachable rather than unlikely: a second value cannot
    /// start until the first has settled, so the row's last write is always the author's last edit.
    inflight: Option<u64>,
    /// The live debounce timer — the handle plus the `Closure` it fires, kept alive here so it is
    /// not leaked per-call. Dropped when re-armed, never from inside its own fire. wasm-only:
    /// the native view shell has no `setTimeout` to hang a debounce on.
    #[cfg(target_arch = "wasm32")]
    timer: Option<MirrorTimer>,
}

/// What a settled response is still allowed to do — see [`MirrorState::settle`].
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
struct Settled {
    /// The author has already moved past the value this response describes. It must not touch
    /// `last` and must not raise a toast — its successor is queued and will report its own outcome.
    stale: bool,
    /// Something is waiting for the wire; re-arm the debounce.
    queued: bool,
}

impl MirrorState {
    /// A control committed `value`. `true` when it is genuinely new, i.e. the debounce must (re)arm.
    fn queue(&mut self, value: String) -> bool {
        // Dedupe against the newest INTENT, not just what the row holds: while a burst is queued,
        // `pending` is what the row is about to hold, so a scrub that wanders back to the queued
        // value is still a no-op.
        if self.pending.as_deref().unwrap_or(self.last.as_str()) == value {
            return false;
        }
        self.generation += 1;
        self.pending = Some(value);
        true
    }

    /// The debounce window closed. `Some((generation, value))` when this column is idle and has
    /// something to send; `None` when a PATCH is already in flight (single flight — the completion
    /// re-arms) or nothing is queued.
    fn take_for_send(&mut self) -> Option<(u64, String)> {
        if self.inflight.is_some() {
            return None;
        }
        let value = self.pending.take()?;
        self.inflight = Some(self.generation);
        Some((self.generation, value))
    }

    /// A response for `generation` came back; `ok` is whether it landed. Frees the single-flight
    /// slot, then records the outcome only if this generation still speaks for the field.
    fn settle(&mut self, generation: u64, value: String, ok: bool) -> Settled {
        if self.inflight == Some(generation) {
            self.inflight = None;
        }
        let stale = self.generation != generation;
        if !stale {
            if ok {
                self.last = value;
            } else {
                self.last.clear(); // did not land — let the next commit of the same value retry
            }
        }
        Settled {
            stale,
            queued: self.pending.is_some(),
        }
    }
}

/// A live debounce timer. The `Closure` is owned here (not `.forget()`) so re-arming drops it.
#[cfg(target_arch = "wasm32")]
struct MirrorTimer {
    handle: i32,
    _closure: wasm_bindgen::closure::Closure<dyn FnMut()>,
}

#[cfg(target_arch = "wasm32")]
thread_local! {
    /// Keyed by `MirroredField::column` — two entries, shared by every `RowMirror` on the page.
    static MIRROR: RefCell<HashMap<&'static str, MirrorState>> = RefCell::new(HashMap::new());
}

/// Cancel and drop any armed timer for `column`. Called only from `arm` — never from inside a firing
/// timer, so this cannot drop a `Closure` that is currently running.
#[cfg(target_arch = "wasm32")]
fn clear_mirror_timer(column: &'static str) {
    let armed = MIRROR.with(|m| m.borrow_mut().get_mut(column).and_then(|f| f.timer.take()));
    if let Some(t) = armed {
        if let Some(win) = web_sys::window() {
            win.clear_timeout_with_handle(t.handle);
        }
    }
}

#[cfg(target_arch = "wasm32")]
impl RowMirror {
    fn from_route() -> Self {
        use leptos_router::hooks::use_params_map;
        let id = use_params_map()
            .get_untracked()
            .get("id")
            .map(|s| s.to_string())
            .unwrap_or_default();
        Self {
            auth: expect_context::<crate::auth::AuthStore>(),
            mission_id: StoredValue::new(id),
            toasts: crate::toast::use_toasts(),
        }
    }

    /// Queue one field of the row. Runs in the control's event handler, so this is also the only
    /// place the route id is read — everything downstream carries it, and none of it touches a
    /// `StoredValue` that may have been disposed by a navigation mid-debounce.
    fn commit(self, field: MirroredField, value: String) {
        let id = self.mission_id.get_value();
        if value.is_empty() || !is_mission_row_id(&id) {
            return;
        }
        let queued = MIRROR.with(|m| m.borrow_mut().entry(field.column).or_default().queue(value));
        if queued {
            self.arm(field, id);
        }
    }

    /// (Re)start the debounce window. Each commit restarts it, so a burst collapses to one PATCH.
    fn arm(self, field: MirroredField, id: String) {
        use wasm_bindgen::JsCast;
        clear_mirror_timer(field.column);
        let Some(win) = web_sys::window() else {
            return;
        };
        let closure = wasm_bindgen::closure::Closure::<dyn FnMut()>::new(move || {
            self.fire(field, id.clone());
        });
        let handle = win
            .set_timeout_with_callback_and_timeout_and_arguments_0(
                closure.as_ref().unchecked_ref(),
                MIRROR_DEBOUNCE_MS,
            )
            .unwrap_or(0);
        MIRROR.with(|m| {
            if let Some(f) = m.borrow_mut().get_mut(field.column) {
                f.timer = Some(MirrorTimer {
                    handle,
                    _closure: closure,
                });
            }
        });
    }

    /// The window closed: put the queued value on the wire — unless this column already has a PATCH
    /// in flight, in which case `pending` stays put and the in-flight completion re-arms. That is
    /// the single-flight rule; two PATCHes for one column are never open at the same time.
    ///
    /// Deliberately does **not** clear its own timer entry: that would drop the `Closure` currently
    /// running. The stale entry is harmless and is cleared by the next `arm`.
    fn fire(self, field: MirroredField, id: String) {
        let next = MIRROR.with(|m| {
            m.borrow_mut()
                .get_mut(field.column)
                .and_then(MirrorState::take_for_send)
        });
        if let Some((generation, value)) = next {
            self.send(field, id, generation, value);
        }
    }

    /// PATCH one field of the row. Fire-and-forget in the sense that matters — the document already
    /// holds the authored value, so a failed mirror never blocks or undoes the edit — but **not**
    /// silent: see [`mirror_failure_message`].
    fn send(self, field: MirroredField, id: String, generation: u64, value: String) {
        let auth = self.auth;
        let column = field.column;
        leptos::task::spawn_local(async move {
            let body = serde_json::json!({ column: value.clone() });
            let res = crate::client::api_patch::<serde_json::Value>(
                auth,
                &format!("/missions/{id}"),
                body,
            )
            .await;
            // Settle this generation, then ask whether it still speaks for the field.
            let settled = MIRROR.with(|m| {
                m.borrow_mut()
                    .get_mut(column)
                    .map(|f| f.settle(generation, value, res.is_ok()))
                    .unwrap_or(Settled {
                        stale: true,
                        queued: false,
                    })
            });
            if let Err(e) = &res {
                leptos::logging::warn!(
                    "T-192: could not mirror {} onto the mission row: {}",
                    column,
                    crate::client::api_error_message(e, "PATCH /missions/:id failed")
                );
                // A stale generation describes a value the author has already replaced, and its
                // successor is queued and will report its own outcome. Toasting here would stack one
                // per scrub tick and name a value nobody is looking at any more.
                if !settled.stale {
                    self.toasts.error(mirror_failure_message(field, e));
                }
            }
            // Either a value arrived while this one was on the wire, or `fire` found the slot busy
            // and left it queued. Re-arm rather than send now: it costs one debounce window on a
            // value that is only a mirror, and it keeps the PATCH rate bounded under a slow server.
            if settled.queued {
                self.arm(field, id);
            }
        });
    }

    /// A clock from a control (`HH:MM`) or a row hydrate (`HH:MM:SS`) → `missions.time_of_day`.
    /// `<input type="time">` reports a half-entered value as `""`, and [`normalize_clock`] rejects
    /// anything else that is not a whole clock, so a partial edit never leaves the tab.
    fn set_time(self, raw: &str) {
        if let Some(t) = normalize_clock(raw) {
            self.commit(MIRROR_TIME, t);
        }
    }

    /// The weather select value → `missions.weather` (the PATCH rejects anything off the enum).
    fn set_weather(self, raw: &str) {
        self.commit(MIRROR_WEATHER, raw.to_string());
    }
}

#[component]
pub fn TopCommandStrip(
    /// Mission title fallback — the `:id` route param; the doc's `meta.title` wins once read.
    title: String,
    can_undo: RwSignal<bool>,
    can_redo: RwSignal<bool>,
    save_semver: RwSignal<String>,
    save_status: RwSignal<String>,
    /// T-159.26 — unsaved-changes flag; a `•` after the title marks dirty (React's `isDirty` dot).
    #[prop(optional)]
    dirty: Option<RwSignal<bool>>,
    /// T-159.26 — the Mission Settings dialog's open flag (gear button toggles it).
    #[prop(optional)]
    settings_open: Option<RwSignal<bool>>,
    /// T-172 B9 — doc revision; re-reads the env (scrubber/weather) + title after undo/redo.
    #[prop(optional)]
    doc_tick: Option<RwSignal<u64>>,
    /// T-172 B9 — obj count for the Save dialog's size line.
    #[prop(optional)]
    obj_count: Option<RwSignal<usize>>,
    /// T-177 B2 / T-071.0 — the ORBAT Manager modal's open flag (the top-strip button toggles it).
    /// Disabled in the scaffold-only case, like `settings_open`.
    #[prop(optional)]
    orbat_open: Option<RwSignal<bool>>,
) -> impl IntoView {
    let open_menu = RwSignal::new(None::<usize>);
    let save_open = RwSignal::new(false);
    let save_notes = RwSignal::new(String::new());
    // T-192 — row mirror for the inline scrubber / weather select. Setup-time, not handler-time.
    #[cfg(target_arch = "wasm32")]
    let row_mirror = RowMirror::from_route();
    // T-243 — where the server-truth Export reports its outcome. Resolved at setup for the same
    // reason `row_mirror` is: `use_toasts()` is an `expect_context` and a DOM click handler has no
    // reactive owner to resolve it through.
    #[cfg(target_arch = "wasm32")]
    let toasts = crate::toast::use_toasts();
    // T-181.44 — per-problem lines from a rejected Save (`details` from the 400). Local to the
    // strip because the Save dialog is the only place they are read; `save_status` stays a
    // one-liner because it also renders in the strip itself.
    let save_findings = RwSignal::new(Vec::<String>::new());
    #[cfg(target_arch = "wasm32")]
    {
        let esc = window_event_listener(leptos::ev::keydown, move |ev| {
            if ev.key() == "Escape" {
                if open_menu.get_untracked().is_some() {
                    open_menu.set(None);
                }
                if save_open.get_untracked() {
                    save_open.set(false);
                }
            }
        });
        on_cleanup(move || esc.remove());
    }
    // Env mirror for the inline scrubber/weather — re-read on every doc change.
    let env = Memo::new(move |_| {
        if let Some(t) = doc_tick {
            t.track();
        }
        #[cfg(target_arch = "wasm32")]
        {
            crate::editor_ops::read_env()
        }
        #[cfg(not(target_arch = "wasm32"))]
        {
            crate::dto::MissionEnv::default()
        }
    });
    let run_action = move |a: MenuAction| {
        open_menu.set(None);
        match a {
            MenuAction::Save => save_open.set(true),
            MenuAction::Export => {
                #[cfg(target_arch = "wasm32")]
                crate::mission_commands::export_now(&save_semver.get_untracked());
            }
            MenuAction::ExportCompiled => {
                #[cfg(target_arch = "wasm32")]
                crate::mission_commands::export_compiled_now(toasts);
            }
            MenuAction::Undo => {
                #[cfg(target_arch = "wasm32")]
                crate::mission_history::undo();
            }
            MenuAction::Redo => {
                #[cfg(target_arch = "wasm32")]
                crate::mission_history::redo();
            }
            MenuAction::Settings => {
                if let Some(s) = settings_open {
                    s.set(true);
                }
            }
        }
    };
    let title_fallback = StoredValue::new(title);
    view! {
        <div class=STRIP>
            // Menu bar (screen 05: File / Edit / View / Mission / Environment).
            <div class="flex items-center">
                {MENUS
                    .iter()
                    .enumerate()
                    .map(|(i, (name, items))| {
                        view! {
                            <div class="relative">
                                <button
                                    type="button"
                                    class=move || {
                                        if open_menu.get() == Some(i) {
                                            "rounded bg-white/10 px-2 py-1 text-label-sm text-on-surface"
                                        } else {
                                            "rounded px-2 py-1 text-label-sm text-on-surface-variant transition-colors hover:bg-white/10 hover:text-on-surface"
                                        }
                                    }
                                    on:click=move |_| {
                                        open_menu
                                            .update(|m| {
                                                *m = if *m == Some(i) { None } else { Some(i) };
                                            });
                                    }
                                >
                                    {*name}
                                </button>
                                {move || {
                                    (open_menu.get() == Some(i))
                                        .then(|| {
                                            view! {
                                                <div class="glass animate-menu-in absolute top-full left-0 z-50 mt-1 w-64 rounded-lg py-1 shadow-lg">
                                                    {items
                                                        .iter()
                                                        .map(|it| {
                                                            let label = it.label;
                                                            match it.action {
                                                                Some(a) => {
                                                                    let disabled = move || match a {
                                                                        MenuAction::Undo => !can_undo.get(),
                                                                        MenuAction::Redo => !can_redo.get(),
                                                                        _ => false,
                                                                    };
                                                                    view! {
                                                                        <button
                                                                            type="button"
                                                                            class="flex w-full items-center px-3 py-1.5 text-left text-label-sm text-on-surface transition-colors hover:bg-white/10 disabled:cursor-default disabled:text-outline disabled:hover:bg-transparent"
                                                                            disabled=disabled
                                                                            on:click=move |_| run_action(a)
                                                                        >
                                                                            {label}
                                                                        </button>
                                                                    }
                                                                        .into_any()
                                                                }
                                                                None => {
                                                                    view! {
                                                                        <span class="flex w-full items-center px-3 py-1.5 text-label-sm text-outline">
                                                                            {label}
                                                                        </span>
                                                                    }
                                                                        .into_any()
                                                                }
                                                            }
                                                        })
                                                        .collect_view()}
                                                </div>
                                            }
                                        })
                                }}
                            </div>
                        }
                    })
                    .collect_view()}
            </div>
            // T-177 B2 / T-071.0 — ORBAT Manager: opens the modal shell (browse/select the live
            // faction → squad → slot tree). Sits right of the Environment menu. Disabled in the
            // scaffold-only case (no `orbat_open` signal), mirroring the settings gear.
            <button
                type="button"
                aria-label="ORBAT Manager"
                class="rounded px-2 py-1 text-label-sm font-semibold text-primary transition-colors hover:bg-primary/15 disabled:opacity-30 disabled:hover:bg-transparent"
                disabled=orbat_open.is_none()
                on:click=move |_| {
                    if let Some(o) = orbat_open {
                        o.set(true);
                    }
                }
            >
                "ORBAT Manager"
            </button>
            // Click-away scrim for an open menu (below the dropdowns' z-50).
            {move || {
                open_menu
                    .get()
                    .is_some()
                    .then(|| {
                        view! {
                            <div
                                class="fixed inset-0 z-40"
                                on:click=move |_| open_menu.set(None)
                            ></div>
                        }
                    })
            }}
            <span class=DIVIDER></span>
            // Editable mission title (React setTitle) + the dirty dot.
            <div class="flex min-w-0 flex-1 items-center">
                {move || {
                    if let Some(t) = doc_tick {
                        t.track();
                    }
                    #[cfg(target_arch = "wasm32")]
                    let doc_title = {
                        let t = crate::editor_ops::read_title();
                        if t.is_empty() { title_fallback.get_value() } else { t }
                    };
                    #[cfg(not(target_arch = "wasm32"))]
                    let doc_title = title_fallback.get_value();
                    view! {
                        <input
                            type="text"
                            aria-label="Mission title"
                            class="w-full min-w-0 truncate rounded border border-transparent bg-transparent px-1.5 py-0.5 text-label-md font-semibold text-on-surface outline-none transition-colors focus:border-outline-variant/40 focus:bg-surface-container"
                            prop:value=doc_title
                            on:change=move |ev| {
                                #[cfg(target_arch = "wasm32")]
                                {
                                    let v = event_target_value(&ev);
                                    if !v.trim().is_empty() {
                                        crate::editor_ops::set_title(v.trim());
                                    }
                                }
                                #[cfg(not(target_arch = "wasm32"))]
                                let _ = &ev;
                            }
                        />
                    }
                }}
                {dirty
                    .map(|d| {
                        view! {
                            <span
                                class=move || if d.get() { "ml-1.5 text-primary" } else { "hidden" }
                                title="Unsaved changes"
                                aria-label="Unsaved changes"
                            >
                                "•"
                            </span>
                        }
                    })}
            </div>
            // Inline time scrubber + weather (screen 05 center) — same doc fields as the
            // Mission Settings dialog (`update_environment`, one undo step per commit), and
            // T-192 the same `missions` row mirror, so the two entry points cannot disagree.
            <div class="flex shrink-0 items-center gap-2">
                <input
                    type="range"
                    min="0"
                    max="1439"
                    step="1"
                    aria-label="Time of day"
                    class="w-28 accent-[--color-primary]"
                    prop:value=move || {
                        hhmm_to_minutes(&env.get().time).unwrap_or(360).to_string()
                    }
                    on:change=move |ev| {
                        #[cfg(target_arch = "wasm32")]
                        {
                            let v: u32 = event_target_value(&ev).parse().unwrap_or(0);
                            let hhmm = minutes_to_hhmm(v);
                            author_env("time", hhmm.as_str().into());
                            row_mirror.set_time(&hhmm);
                        }
                        #[cfg(not(target_arch = "wasm32"))]
                        let _ = &ev;
                    }
                />
                <span class="font-mono text-xs tabular-nums text-on-surface-variant">
                    {move || env.get().time}
                </span>
                <select
                    aria-label="Weather"
                    class="rounded border border-outline-variant/40 bg-surface-container px-1.5 py-0.5 text-xs text-on-surface"
                    prop:value=move || env.get().weather
                    on:change=move |ev| {
                        #[cfg(target_arch = "wasm32")]
                        {
                            let w = event_target_value(&ev);
                            author_env("weather", w.as_str().into());
                            row_mirror.set_weather(&w);
                        }
                        #[cfg(not(target_arch = "wasm32"))]
                        let _ = &ev;
                    }
                >
                    <option value="clear">"Clear"</option>
                    <option value="overcast">"Overcast"</option>
                    <option value="heavy_rain">"Heavy Rain"</option>
                    <option value="dense_fog">"Dense Fog"</option>
                </select>
            </div>
            <span class=DIVIDER></span>
            // History — present-but-disabled (React parity; version list lands with the history
            // lane).
            <button
                type="button"
                aria-label="History"
                title="Version history (soon)"
                class=BTN_ICON
                disabled=true
            >
                <MaterialIcon name="history" class="block text-base" />
            </button>
            // `aria-label` is the gate's DOM handle for the button path (smoke_undo_editor A3/A6) —
            // a real a11y name, not a test-only attribute.
            <button
                type="button"
                aria-label="Undo"
                title="Undo (Ctrl+Z)"
                class=BTN_ICON
                disabled=move || !can_undo.get()
                on:click=move |_| {
                    #[cfg(target_arch = "wasm32")]
                    {
                        crate::mission_history::undo();
                    }
                }
            >
                <MaterialIcon name="undo" class="block text-base" />
            </button>
            <button
                type="button"
                aria-label="Redo"
                title="Redo (Ctrl+Shift+Z)"
                class=BTN_ICON
                disabled=move || !can_redo.get()
                on:click=move |_| {
                    #[cfg(target_arch = "wasm32")]
                    {
                        crate::mission_history::redo();
                    }
                }
            >
                <MaterialIcon name="redo" class="block text-base" />
            </button>
            <span class=DIVIDER></span>
            <button
                type="button"
                class="rounded bg-primary px-3 py-1 text-xs font-medium text-on-primary"
                on:click=move |_| save_open.set(true)
            >
                "Save Version"
            </button>
            <button
                type="button"
                class="rounded border border-outline-variant/40 px-3 py-1 text-xs font-medium text-on-surface"
                on:click=move |_| {
                    #[cfg(target_arch = "wasm32")]
                    crate::mission_commands::export_now(&save_semver.get_untracked());
                }
            >
                "Export JSON"
            </button>
            // T-243 — the compiled mod document (what `/compiled` serves a game server), beside the
            // superset envelope above. `/compiled` is service-token-only, so this button is the
            // only way an author can see these bytes at all.
            <button
                type="button"
                title="Download the compiled mission document the game server receives"
                class="rounded border border-outline-variant/40 px-3 py-1 text-xs font-medium text-on-surface"
                on:click=move |_| {
                    #[cfg(target_arch = "wasm32")]
                    crate::mission_commands::export_compiled_now(toasts);
                }
            >
                "Export Compiled"
            </button>
            <span class="min-w-24 font-mono text-xs text-on-surface-variant">
                {move || save_status.get()}
            </span>
            // T-159.26 — Mission Settings (environment). Opens the dialog when a `settings_open`
            // signal is threaded (the editor); disabled in the scaffold-only case.
            <button
                type="button"
                aria-label="Mission settings"
                class=BTN_ICON
                disabled=settings_open.is_none()
                on:click=move |_| {
                    if let Some(s) = settings_open {
                        s.set(true);
                    }
                }
            >
                <MaterialIcon name="settings" class="block text-base" />
            </button>
            // Save Version dialog (React SaveVersionDialog: semver + notes + size estimate +
            // indeterminate bar while saving). Renders no DOM while closed.
            {move || {
                save_open
                    .get()
                    .then(|| {
                        let estimate = {
                            #[cfg(target_arch = "wasm32")]
                            {
                                crate::editor_ops::slots_json()
                                    .as_deref()
                                    .and_then(crate::mission_size::estimate_compiled_bytes)
                            }
                            #[cfg(not(target_arch = "wasm32"))]
                            {
                                None::<usize>
                            }
                        };
                        let obj = obj_count.map_or(0, |o| o.get());
                        let size_line = match estimate {
                            Some(b) => {
                                format!(
                                    "~{} · {} objects",
                                    crate::mission_size::format_bytes(b),
                                    obj,
                                )
                            }
                            None => format!("{obj} objects"),
                        };
                        let big = estimate.is_some_and(|b| b > 200_000_000);
                        view! {
                            <div
                                class="animate-overlay-fade fixed inset-0 z-50 bg-black/50 backdrop-blur-sm"
                                on:click=move |_| save_open.set(false)
                            ></div>
                            <div class="glass animate-dialog-in fixed top-1/2 left-1/2 z-50 flex max-h-[85vh] w-[92vw] max-w-md -translate-x-1/2 -translate-y-1/2 flex-col rounded-xl shadow-2xl outline-none">
                                <div class="flex items-start justify-between gap-4 border-b border-outline-variant/30 px-6 py-4">
                                    <div class="min-w-0">
                                        <h2 class="text-headline-sm text-on-surface">"Save Version"</h2>
                                        <p class="mt-1 text-label-md text-on-surface-variant">
                                            "Versions are immutable — pick a new semver."
                                        </p>
                                    </div>
                                    <button
                                        type="button"
                                        aria-label="Close"
                                        on:click=move |_| save_open.set(false)
                                        class="shrink-0 rounded-md p-1 text-outline transition-colors hover:bg-surface-variant/50 hover:text-on-surface"
                                    >
                                        <MaterialIcon name="close" />
                                    </button>
                                </div>
                                <div class="flex flex-col gap-3 px-6 py-5">
                                    <label class="flex flex-col gap-1">
                                        <span class="text-label-sm uppercase tracking-wider text-outline">
                                            "Version"
                                        </span>
                                        <input
                                            type="text"
                                            aria-label="Version"
                                            class="w-32 rounded border border-outline-variant/40 bg-surface-container px-2 py-1 font-mono text-xs text-on-surface"
                                            prop:value=move || save_semver.get()
                                            on:input=move |ev| save_semver.set(event_target_value(&ev))
                                        />
                                    </label>
                                    <label class="flex flex-col gap-1">
                                        <span class="text-label-sm uppercase tracking-wider text-outline">
                                            "Notes"
                                        </span>
                                        <textarea
                                            aria-label="Editor notes"
                                            rows="2"
                                            class="w-full resize-none rounded border border-outline-variant/40 bg-surface-container px-2 py-1 text-xs text-on-surface"
                                            prop:value=move || save_notes.get()
                                            on:input=move |ev| save_notes.set(event_target_value(&ev))
                                        ></textarea>
                                    </label>
                                    <p class=if big {
                                        "font-mono text-xs text-tactical-yellow"
                                    } else {
                                        "font-mono text-xs text-on-surface-variant"
                                    }>{size_line}</p>
                                    {move || {
                                        save_status
                                            .get()
                                            .starts_with("Saving")
                                            .then(|| {
                                                view! {
                                                    <div class="h-1 w-full overflow-hidden rounded-full bg-surface-variant/40">
                                                        <div class="animate-mc-load-bar h-full w-1/4 rounded-full bg-primary"></div>
                                                    </div>
                                                }
                                            })
                                    }}
                                    <p class="min-h-4 font-mono text-xs text-on-surface-variant">
                                        {move || save_status.get()}
                                    </p>
                                    // T-181.44 — the backend's `details`, one row each. Before
                                    // this the author saw "Save failed (400)" and nothing else,
                                    // and a control character in a callsign only ever surfaced as
                                    // a /compiled 500 in an API log they never read.
                                    {move || {
                                        let rows = save_findings.get();
                                        (!rows.is_empty())
                                            .then(|| {
                                                view! {
                                                    <ul class="max-h-32 list-disc space-y-1 overflow-y-auto rounded border border-error/40 bg-error/5 py-1 pl-5 pr-2 font-mono text-[11px] leading-snug text-error">
                                                        {rows
                                                            .into_iter()
                                                            .map(|r| view! { <li>{r}</li> })
                                                            .collect_view()}
                                                    </ul>
                                                }
                                            })
                                    }}
                                    <button
                                        type="button"
                                        class="self-end rounded bg-primary px-4 py-1.5 text-xs font-medium text-on-primary"
                                        on:click=move |_| {
                                            #[cfg(target_arch = "wasm32")]
                                            crate::mission_commands::save_now(
                                                save_semver.get_untracked(),
                                                save_notes.get_untracked(),
                                                save_status,
                                                save_findings,
                                            );
                                        }
                                    >
                                        "Save"
                                    </button>
                                </div>
                            </div>
                        }
                    })
            }}
        </div>
    }
}

// ── Tree rows (T-159.22 / T-172 B6+B7) ──────────────────────────────────────────────────────────
// Both trees collapse: container rows carry a chevron toggle (span, not a nested button — rows are
// `<button>`s) + open/closed folder icons, and depth renders as border-l guide-line runs instead of
// bare padding (the React `TreeView` look). The outliner/ORBAT collapsed sets start EMPTY (fully
// expanded — the T-169 windowing smoke's totals depend on it); the palette seeds from
// `CatalogNode::default_expanded` (only depth-0 faction folders open, `buildCatalogTree` rule 3).
//
// Rows are `<button>`s with a real `aria-label` — focusable, activatable, and the gates' DOM handle,
// the `aria-label="Undo"` precedent above (NOT a test-only attribute).

/// A tree row's shared recipe; depth renders as leading guide-line spans (see `guide_spans`).
const ROW: &str = "relative flex w-full items-center gap-1.5 rounded px-1.5 py-1 text-left text-label-sm text-on-surface-variant transition-colors hover:bg-white/10 hover:text-on-surface";
const ROW_ACTIVE: &str = "relative flex w-full items-center gap-1.5 rounded px-1.5 py-1 text-left text-label-sm transition-colors bg-primary/20 text-primary";
/// T-177 A2 — the palette-leaf variant of [`ROW`]: adds `cursor-grab` (→ `cursor-grabbing` while
/// pressed) so hovering a placeable role advertises the drag affordance. Folders keep `cursor-pointer`
/// and outliner slots keep the plain [`ROW`] default (only palette leaves are drag-to-place).
const PALETTE_LEAF: &str = "relative flex w-full items-center gap-1.5 rounded px-1.5 py-1 text-left text-label-sm text-on-surface-variant transition-colors hover:bg-white/10 hover:text-on-surface cursor-grab active:cursor-grabbing";

/// Hierarchy guide lines — continuous YouTube spines (T-178 A3/A4; supersedes T-177 L-hooks).
/// `ancestors` / `guide_ids` both have `len == depth`. Continuous `w-px` stems + mid-row stub;
/// click toggles the column owner (`guide_ids[k]`).
fn guide_spans(
    ancestors: &[bool],
    guide_ids: &[String],
    collapsed: RwSignal<std::collections::HashSet<String>>,
) -> AnyView {
    let depth = ancestors.len();
    if depth == 0 {
        return ().into_any();
    }
    debug_assert_eq!(guide_ids.len(), depth);
    let col_left = |k: usize| format!("left:calc(0.375rem + {:.3}rem)", (k as f64) * 0.75 + 0.375);
    let mut lines: Vec<AnyView> = Vec::new();
    let make_toggle = |id: String, collapsed: RwSignal<std::collections::HashSet<String>>| {
        move |ev: web_sys::MouseEvent| {
            ev.stop_propagation();
            collapsed.update(|c| {
                if !c.remove(&id) {
                    c.insert(id.clone());
                }
            });
        }
    };
    // Ancestor spines: full-height hairline where the branch continues.
    for (k, cont) in ancestors.iter().enumerate().take(depth.saturating_sub(1)) {
        if *cont {
            let id = guide_ids.get(k).cloned().unwrap_or_default();
            let left = col_left(k);
            let on_click = make_toggle(id.clone(), collapsed);
            lines.push(
                view! {
                    <span
                        role="button"
                        tabindex="-1"
                        data-guide-toggle=id.clone()
                        aria-label=format!("Toggle {id}")
                        class="absolute inset-y-0 w-px cursor-pointer bg-white/25"
                        style=left
                        on:click=on_click
                    ></span>
                }
                .into_any(),
            );
        }
    }
    let last = depth - 1;
    let id = guide_ids.get(last).cloned().unwrap_or_default();
    let left = col_left(last);
    // Continuous stem: full height if sibling continues, else top-half only (last child).
    if ancestors[last] {
        let on_click = make_toggle(id.clone(), collapsed);
        lines.push(
            view! {
                <span
                    role="button"
                    tabindex="-1"
                    data-guide-toggle=id.clone()
                    aria-label=format!("Toggle {id}")
                    class="absolute inset-y-0 w-px cursor-pointer bg-white/25"
                    style=left.clone()
                    on:click=on_click
                ></span>
            }
            .into_any(),
        );
    } else {
        let on_click = make_toggle(id.clone(), collapsed);
        lines.push(
            view! {
                <span
                    role="button"
                    tabindex="-1"
                    data-guide-toggle=id.clone()
                    aria-label=format!("Toggle {id}")
                    class="absolute top-0 h-1/2 w-px cursor-pointer bg-white/25"
                    style=left.clone()
                    on:click=on_click
                ></span>
            }
            .into_any(),
        );
    }
    // Mid-row horizontal stub into the row content.
    let on_click = make_toggle(id.clone(), collapsed);
    lines.push(
        view! {
            <span
                role="button"
                tabindex="-1"
                data-guide-toggle=id.clone()
                aria-label=format!("Toggle {id}")
                class="absolute top-1/2 h-px w-2 cursor-pointer bg-white/25"
                style=left
                on:click=on_click
            ></span>
        }
        .into_any(),
    );
    let spacers = (0..depth)
        .map(|_| view! { <span class="w-3 shrink-0"></span> })
        .collect::<Vec<_>>();
    view! { {lines}{spacers} }.into_any()
}

/// Chevron toggle for container rows (`expand_more` open / `chevron_right` closed) — a
/// `role="button"` span so it can nest inside the row `<button>`; leaves get an alignment
/// spacer. Clicking toggles the id in `collapsed` without firing the row action.
fn chevron_or_spacer(
    has_children: bool,
    open: bool,
    id: &str,
    collapsed: RwSignal<std::collections::HashSet<String>>,
) -> AnyView {
    if !has_children {
        return view! { <span class="size-4 shrink-0"></span> }.into_any();
    }
    let cid = id.to_string();
    let icon = if open { "expand_more" } else { "chevron_right" };
    view! {
        <span
            role="button"
            tabindex="-1"
            aria-expanded=if open { "true" } else { "false" }
            class="flex size-4 shrink-0 cursor-pointer items-center justify-center rounded text-outline transition-colors hover:bg-white/10 hover:text-on-surface"
            on:click=move |ev| {
                ev.stop_propagation();
                collapsed
                    .update(|c| {
                        if !c.remove(&cid) {
                            c.insert(cid.clone());
                        }
                    });
            }
        >
            <MaterialIcon name=icon class="block text-sm" />
        </span>
    }
    .into_any()
}

/// T-169 — window geometry. `ROW_H` is the flow height of one row (`px-1.5 py-1 text-label-sm`);
/// the spacers use it to reserve the off-screen rows. `OVERSCAN` renders a few rows past the
/// viewport each way so a fast scroll never flashes blank.
const ROW_H: f64 = 24.0;
const CONTAINER_H: f64 = 420.0;
const OVERSCAN: usize = 6;

/// Render ONE flattened outliner row (no recursion — the windowed list draws a flat slice).
/// Header kinds (Unfiled / Faction) are inert; Squad is a refile drop target when `orbat_refile`;
/// Folder → active-layer; Slot → select + dbl-click→Attributes (SEL-ORBAT-DBL-001).
fn single_row(
    row: &FlatRow,
    selected: RwSignal<Vec<String>>,
    active_layer: RwSignal<Option<String>>,
    collapsed: RwSignal<std::collections::HashSet<String>>,
    // T-180.6 — when true, slot pointerdown arms refile; squad pointerup completes it.
    orbat_refile: bool,
) -> AnyView {
    let label = row.label.clone();
    let aria = row.label.clone();
    let id = row.id.clone();
    let is_leader = row.is_leader;
    // T-177/T-178 — per-row guide continuation + click-to-toggle owners.
    let ancestors: &[bool] = &row.ancestors;
    let guide_ids: &[String] = &row.guide_ids;
    // Static per build — a chevron toggle bumps `collapsed`, which re-flattens + re-renders
    // the slice (the virtual_tree Effect tracks it), so open state never goes stale.
    let open = !collapsed.with_untracked(|c| c.contains(&row.id));
    let toggle = chevron_or_spacer(row.has_children, open, &row.id, collapsed);
    let sl_badge = if is_leader {
        view! {
            <span class=badge_class("primary") data-sl-badge="true">"SL"</span>
        }
        .into_any()
    } else {
        ().into_any()
    };
    match row.kind {
        NodeKind::Unfiled => view! {
            <div class="relative flex items-center gap-1.5 px-1.5 py-1 text-label-sm text-outline">
                {guide_spans(ancestors, guide_ids, collapsed)}
                {toggle}
                <MaterialIcon name="inbox" class="block text-sm" />
                <span>{label}</span>
            </div>
        }
        .into_any(),
        NodeKind::Faction => view! {
            <div class="relative flex items-center gap-1.5 px-1.5 py-1 text-label-sm font-semibold uppercase tracking-wide text-on-surface-variant">
                {guide_spans(ancestors, guide_ids, collapsed)}
                {toggle}
                <MaterialIcon name="flag" class="block text-sm" />
                <span class="truncate">{label}</span>
            </div>
        }
        .into_any(),
        NodeKind::Squad => {
            let dest = id.clone();
            if orbat_refile {
                view! {
                    <div
                        class="relative flex items-center gap-1.5 px-1.5 py-1 text-label-sm text-on-surface-variant"
                        title="Drop a slot here to refile into this squad"
                        on:pointerup=move |ev| {
                            ev.stop_propagation();
                            #[cfg(target_arch = "wasm32")]
                            crate::editor_ops::complete_refile_onto_squad(dest.clone());
                            #[cfg(not(target_arch = "wasm32"))]
                            let _ = &dest;
                        }
                    >
                        {guide_spans(ancestors, guide_ids, collapsed)}
                        {toggle}
                        <MaterialIcon name="groups" class="block text-sm" />
                        <span class="truncate">{label}</span>
                    </div>
                }
                .into_any()
            } else {
                view! {
                    <div class="relative flex items-center gap-1.5 px-1.5 py-1 text-label-sm text-on-surface-variant">
                        {guide_spans(ancestors, guide_ids, collapsed)}
                        {toggle}
                        <MaterialIcon name="groups" class="block text-sm" />
                        <span class="truncate">{label}</span>
                    </div>
                }
                .into_any()
            }
        }
        NodeKind::Folder => {
            let is_active = {
                let id = id.clone();
                move || active_layer.get().as_deref() == Some(id.as_str())
            };
            let folder_icon = if open { "folder_open" } else { "folder" };
            view! {
                <button
                    type="button"
                    aria-label=aria
                    title="Make this the drop target"
                    class=move || if is_active() { ROW_ACTIVE } else { ROW }
                    on:click=move |_| {
                        #[cfg(target_arch = "wasm32")]
                        crate::editor_ops::set_active_layer(Some(id.clone()));
                    }
                >
                    {guide_spans(ancestors, guide_ids, collapsed)}
                    {toggle}
                    <MaterialIcon name=folder_icon class="block text-sm" />
                    <span class="truncate">{label}</span>
                </button>
            }
            .into_any()
        }
        NodeKind::Slot => {
            let is_sel = {
                let id = id.clone();
                move || selected.get().iter().any(|s| s == &id)
            };
            let id_dbl = id.clone();
            let id_refile = id.clone();
            view! {
                <button
                    type="button"
                    aria-label=aria
                    class=move || if is_sel() { ROW_ACTIVE } else { ROW }
                    on:click=move |_| {
                        #[cfg(target_arch = "wasm32")]
                        crate::editor_ops::select_slot(id.clone());
                    }
                    // T-159.26 A1 — outliner activate (native dblclick) opens Attributes,
                    // the SEL-ORBAT-DBL-001 contract.
                    on:dblclick=move |_| {
                        #[cfg(target_arch = "wasm32")]
                        crate::editor_ops::open_attributes(id_dbl.clone());
                        #[cfg(not(target_arch = "wasm32"))]
                        let _ = &id_dbl;
                    }
                    on:pointerdown=move |_| {
                        if orbat_refile {
                            #[cfg(target_arch = "wasm32")]
                            crate::editor_ops::begin_refile(id_refile.clone());
                            #[cfg(not(target_arch = "wasm32"))]
                            let _ = &id_refile;
                        }
                    }
                >
                    {guide_spans(ancestors, guide_ids, collapsed)}
                    {toggle}
                    <MaterialIcon name="person" class="block text-sm" />
                    <span class="truncate">{label}</span>
                    {sl_badge}
                </button>
            }
            .into_any()
        }
    }
}

/// T-169 — publish `window.__outlinerStats[key] = {total, rendered, threshold}` for the gate.
#[cfg(target_arch = "wasm32")]
fn set_outliner_stats(key: &str, total: usize, rendered: usize) {
    use wasm_bindgen::JsValue;
    let Some(win) = web_sys::window() else { return };
    let stats = match js_sys::Reflect::get(&win, &JsValue::from_str("__outlinerStats")) {
        Ok(v) if v.is_object() => v,
        _ => {
            let o = js_sys::Object::new();
            let _ = js_sys::Reflect::set(&win, &JsValue::from_str("__outlinerStats"), &o);
            o.into()
        }
    };
    let entry = js_sys::Object::new();
    let set = |k: &str, n: usize| {
        let _ = js_sys::Reflect::set(&entry, &JsValue::from_str(k), &JsValue::from_f64(n as f64));
    };
    set("total", total);
    set("rendered", rendered);
    set("threshold", VIRTUAL_SLOT_THRESHOLD);
    let _ = js_sys::Reflect::set(&stats, &JsValue::from_str(key), &entry);
}
#[cfg(not(target_arch = "wasm32"))]
fn set_outliner_stats(_key: &str, _total: usize, _rendered: usize) {}

/// T-169 — render a dock tree, windowed above [`VIRTUAL_SLOT_THRESHOLD`]. Below it the whole
/// flattened list renders eagerly; above it a fixed-height scroll container draws only the visible
/// slice (+ overscan) between two spacer divs, so a mission-scale tree never builds N DOM rows.
/// `stats_key` names this tree in `window.__outlinerStats`.
fn virtual_tree(
    nodes: RwSignal<Vec<OutlinerNode>>,
    selected: RwSignal<Vec<String>>,
    active_layer: RwSignal<Option<String>>,
    stats_key: &'static str,
    empty_msg: &'static str,
    // T-180.6 — enable ORBAT slot→squad pointer-refile in this tree.
    orbat_refile: bool,
) -> AnyView {
    // Per-tree collapse state (T-172 B6). Starts EMPTY = fully expanded, exactly the pre-collapse
    // render — the T-169 windowing smoke's totals depend on the default-expanded boot state.
    let collapsed = RwSignal::new(std::collections::HashSet::<String>::new());
    // Flatten once per doc/collapse change (O(n), like the mutation itself); the scroll path only
    // re-slices. Created ONCE per mount (this fn is called outside any reactive closure), so the
    // Effect never leaks — it re-runs on `nodes`/`collapsed` change, and the render `move ||`
    // re-slices on `rev`/scroll.
    let flat = StoredValue::new(Vec::<FlatRow>::new());
    let rev = RwSignal::new(0u64);
    Effect::new(move |_| {
        let f = collapsed.with(|c| flatten_visible(&nodes.get(), c));
        flat.set_value(f);
        rev.update(|r| *r = r.wrapping_add(1));
    });
    let scroll_top = RwSignal::new(0.0_f64);
    (move || {
        rev.track(); // re-render the slice when the tree changes
        let st = scroll_top.get();
        flat.with_value(|f| {
            let total = f.len();
            if total == 0 {
                set_outliner_stats(stats_key, 0, 0);
                return view! { <p class="text-label-sm text-outline">{empty_msg}</p> }.into_any();
            }
            if total <= VIRTUAL_SLOT_THRESHOLD {
                set_outliner_stats(stats_key, total, total);
                return view! {
                    <div>
                        {f
                            .iter()
                            .map(|r| single_row(r, selected, active_layer, collapsed, orbat_refile))
                            .collect::<Vec<_>>()}
                    </div>
                }
                .into_any();
            }
            let per_screen = (CONTAINER_H / ROW_H).ceil() as usize;
            let start = ((st / ROW_H).floor() as usize).saturating_sub(OVERSCAN);
            let end = (start + per_screen + 2 * OVERSCAN).min(total);
            set_outliner_stats(stats_key, total, end - start);
            let top = start as f64 * ROW_H;
            let bottom = (total - end) as f64 * ROW_H;
            let rows: Vec<AnyView> = f[start..end]
                .iter()
                .map(|r| single_row(r, selected, active_layer, collapsed, orbat_refile))
                .collect();
            view! {
                <div
                    class="overflow-y-auto"
                    style=format!("height:{CONTAINER_H}px")
                    on:scroll=move |ev| {
                        #[cfg(target_arch = "wasm32")]
                        {
                            use wasm_bindgen::JsCast;
                            if let Some(el) = ev.target().and_then(|t| t.dyn_into::<web_sys::Element>().ok()) {
                                scroll_top.set(el.scroll_top() as f64);
                            }
                        }
                        #[cfg(not(target_arch = "wasm32"))]
                        let _ = &ev;
                    }
                >
                    <div style=format!("height:{top}px")></div>
                    {rows}
                    <div style=format!("height:{bottom}px")></div>
                </div>
            }
            .into_any()
        })
    })
    .into_any()
}

/// T-215 — registry kinds the vehicle cargo picker offers.
///
/// A superset of `arsenal::CARGO_ADD_KINDS` (magazine / ammo / gear_item / throwable / explosive) on
/// purpose. That list answers "what goes inside a **worn garment**", so it excludes rifles, vests
/// and backpacks — things a person carries *on* rather than *in* themselves. A truck bed has no such
/// distinction: spare weapons, spare armour and spare packs are exactly what a resupply vehicle is
/// loaded with, and excluding them would make the feature useless for its main use.
///
/// Still excluded: `character` (a person is not cargo — crews are ORBAT slots), `vehicle` and
/// `vehicle_weapon` (nesting a vehicle inside a vehicle is not a thing the engine's storage does),
/// and `other` (the export's escape hatch, whose contents are by definition unclassified).
const VEHICLE_CARGO_KINDS: &[&str] = &[
    "magazine",
    "ammo",
    "gear_item",
    "gear_throwable",
    "gear_explosive",
    "gear_primary",
    "gear_handgun",
    "gear_launcher",
    "gear_binoculars",
    "gear_vest",
    "gear_armored_vest",
    "gear_backpack",
    "gear_helmet",
    "gear_jacket",
    "gear_pants",
    "gear_boots",
    "gear_gloves",
    "gear_glasses",
    "optic",
    "attachment",
    "crate",
];

/// T-215 — which palette a leaf belongs to. The tree machinery (guides, collapse, search) is
/// identical for both; only the glyph and which `editor_ops` arm the press calls differ, and those
/// are the two things that must not be shared — a Vehicles leaf that armed a character place would
/// silently write a `slots` row.
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum PaletteKind {
    Character,
    Vehicle,
    /// T-254 — Objects chip → `entitiesById`.
    Object,
}

impl PaletteKind {
    const fn leaf_icon(self) -> &'static str {
        match self {
            Self::Character => "person",
            Self::Vehicle => "directions_car",
            Self::Object => "inventory_2",
        }
    }

    const fn leaf_title(self) -> &'static str {
        match self {
            Self::Character => "Drag onto the map to place",
            Self::Vehicle => "Drag onto the map to place this vehicle",
            Self::Object => "Drag onto the map to place this object",
        }
    }
}

/// Render the palette recursively. A leaf (`payload.is_some()`) arms a place on `pointerdown` —
/// **pointer-drag, not HTML5 DnD**: the gates drive trusted `Input.dispatchMouseEvent`, which
/// synthesizes real pointer events into these handlers, where DnD would need `Input.setInterceptDrags`.
/// The chrome host stops `pointerdown` propagation, so this press cannot also open a map gesture; the
/// release is consumed by the container's `pointerup` (see `mission_editor`).
fn palette_rows(
    nodes: &[CatalogNode],
    depth: usize,
    // T-177 A1 — the parent row's guide-continuation vector (see `guide_spans`); `&[]` at the root.
    prefix: &[bool],
    // T-178 A4 — ancestor ids for guide click (`len == depth`).
    id_prefix: &[String],
    collapsed: RwSignal<std::collections::HashSet<String>>,
    // T-215 — Factions or Vehicles; picks the glyph and the `editor_ops` arm.
    kind: PaletteKind,
) -> AnyView {
    let len = nodes.len();
    nodes
        .iter()
        .enumerate()
        .map(|(i, n)| {
            let label = n.label.clone();
            let aria = n.label.clone();
            // T-177 A1 — same continuation rule as the outliner's `flatten_visible`: roots draw no
            // column; every deeper row extends its parent's vector with its own `!is_last` bit.
            let anc: Vec<bool> = if depth == 0 {
                Vec::new()
            } else {
                let mut v = Vec::with_capacity(depth);
                v.extend_from_slice(prefix);
                v.push(i + 1 != len);
                v
            };
            let gids = id_prefix.to_vec();
            match n.payload.clone() {
                None => {
                    // Folder — collapsible (T-172 B6): chevron + open/closed icon; kids render
                    // only while open. The whole palette re-renders on a toggle (the DockRight
                    // closure tracks `collapsed`), so open state is read untracked here.
                    let open = !collapsed.with_untracked(|c| c.contains(&n.id));
                    let toggle =
                        chevron_or_spacer(!n.children.is_empty(), open, &n.id, collapsed);
                    let folder_icon = if open { "folder_open" } else { "folder" };
                    let mut child_ids = gids.clone();
                    child_ids.push(n.id.clone());
                    let kids = if open {
                        palette_rows(&n.children, depth + 1, &anc, &child_ids, collapsed, kind)
                    } else {
                        ().into_any()
                    };
                    let cid = n.id.clone();
                    view! {
                        <div
                            role="button"
                            tabindex="-1"
                            aria-label=aria
                            class="relative flex cursor-pointer items-center gap-1.5 px-1.5 py-1 text-label-sm text-outline transition-colors hover:text-on-surface"
                            on:click=move |_| {
                                collapsed
                                    .update(|c| {
                                        if !c.remove(&cid) {
                                            c.insert(cid.clone());
                                        }
                                    });
                            }
                        >
                            {guide_spans(&anc, &gids, collapsed)}
                            {toggle}
                            <MaterialIcon name=folder_icon class="block text-sm" />
                            <span class="truncate">{label}</span>
                        </div>
                        {kids}
                    }
                    .into_any()
                }
                // T-177 A2 — a placeable role: PALETTE_LEAF adds `cursor-grab`/`active:cursor-grabbing`
                // over ROW so hovering shows the drag affordance (folders keep `cursor-pointer`).
                Some(payload) => view! {
                    <button
                        type="button"
                        aria-label=aria
                        title=kind.leaf_title()
                        class=PALETTE_LEAF
                        on:pointerdown=move |_| {
                            #[cfg(target_arch = "wasm32")]
                            match kind {
                                PaletteKind::Character => {
                                    crate::editor_ops::begin_place(payload.clone())
                                }
                                PaletteKind::Vehicle => {
                                    crate::editor_ops::begin_place_vehicle(payload.clone())
                                }
                                PaletteKind::Object => {
                                    crate::editor_ops::begin_place_object(payload.clone())
                                }
                            }
                            // `editor_ops` is wasm-only, so the native view shell would see an
                            // unused capture (the `announcements.rs` `let _ = store;` idiom).
                            #[cfg(not(target_arch = "wasm32"))]
                            let _ = &payload;
                        }
                    >
                        {guide_spans(&anc, &gids, collapsed)}
                        <span class="size-4 shrink-0"></span>
                        <MaterialIcon name=kind.leaf_icon() class="block text-sm" />
                        <span class="truncate">{label}</span>
                    </button>
                }
                .into_any(),
            }
        })
        .collect::<Vec<_>>()
        .into_any()
}

/// T-215 — the **Placed** section under the Vehicles palette: every `vehiclesById` row with its map
/// position, a delete, and an expandable `{item, qty}` cargo editor.
///
/// This is where authored vehicle cargo is entered. It lives in the Vehicles tab rather than in the
/// Attributes modal because Attributes is keyed on a **slot** id and reads the slot SoA — a vehicle
/// is deliberately off that SoA, so opening it there would need a second, parallel modal for a
/// two-field entity. Under the palette that produced them, the placed list is also the only surface
/// that shows an author what they have already put down.
///
/// Native builds render nothing: `editor_ops` is `#![cfg(target_arch = "wasm32")]`, so there is no
/// document to read (the same reason `CatalogState` never leaves `Loading` on the native shell).
#[cfg(target_arch = "wasm32")]
fn placed_vehicles_panel(
    doc_tick: RwSignal<u64>,
    registry_items: RwSignal<Option<Vec<crate::dto::RegistryItem>>>,
    expanded: RwSignal<std::collections::HashSet<String>>,
) -> AnyView {
    use crate::editor_ops::{VehicleCargoRow, VehicleRow};

    // Re-read the doc on every mutation — `MissionDocCore` has no change subscription, so this is
    // the same pull-mirror tick the Attributes modal uses.
    doc_tick.track();
    let rows: Vec<VehicleRow> = crate::editor_ops::vehicle_rows();
    if rows.is_empty() {
        return ().into_any();
    }

    let items = registry_items.get().unwrap_or_default();
    let names: HashMap<String, String> = items
        .iter()
        .map(|i| (i.resource_name.clone(), i.display_name.clone()))
        .collect();
    let mut addable: Vec<(String, String)> = items
        .iter()
        .filter(|i| VEHICLE_CARGO_KINDS.contains(&i.kind.as_str()))
        .filter(|i| !i.r#abstract.unwrap_or(false))
        .map(|i| (i.resource_name.clone(), i.display_name.clone()))
        .collect();
    addable.sort_by(|a, b| a.1.cmp(&b.1));
    let addable = StoredValue::new(addable);
    let names = StoredValue::new(names);

    let label_of =
        move |rn: &str| names.with_value(|n| n.get(rn).cloned().unwrap_or_else(|| rn.to_string()));

    let body = rows
        .into_iter()
        .map(|v| {
            let vid = v.id.clone();
            let open = expanded.with(|e| e.contains(&vid));
            let title = label_of(&v.resource_name);
            let pos = v.xy.map_or_else(
                // A vehicle added from the ORBAT Manager before it was ever dropped has no
                // position. Saying so is the point — it is the state this ticket exists to end.
                || "not placed".to_string(),
                |(x, y)| format!("{x:.1}, {y:.1}"),
            );
            let heading = v.rotation;
            let cargo = v.cargo.clone();
            let n_cargo = cargo.len();

            let (id_toggle, id_del) = (vid.clone(), vid.clone());
            let head = view! {
                <div class="flex items-center gap-1.5 rounded px-1.5 py-1 hover:bg-white/5">
                    <span
                        role="button"
                        tabindex="-1"
                        aria-label=format!("Toggle cargo for {title}")
                        aria-expanded=if open { "true" } else { "false" }
                        class="flex size-4 shrink-0 cursor-pointer items-center justify-center rounded text-outline hover:text-on-surface"
                        on:click=move |_| {
                            expanded
                                .update(|e| {
                                    if !e.remove(&id_toggle) {
                                        e.insert(id_toggle.clone());
                                    }
                                });
                        }
                    >
                        <MaterialIcon
                            name=if open { "expand_more" } else { "chevron_right" }
                            class="block text-sm"
                        />
                    </span>
                    <MaterialIcon name="directions_car" class="block shrink-0 text-sm" />
                    <span class="min-w-0 flex-1 truncate text-label-sm text-on-surface">{title}</span>
                    <span class="shrink-0 font-mono text-label-sm tabular-nums text-outline">
                        {pos}
                    </span>
                    <span class="shrink-0 font-mono text-label-sm tabular-nums text-outline">
                        {format!("{n_cargo}\u{a0}items")}
                    </span>
                    <button
                        type="button"
                        aria-label="Remove vehicle"
                        class="shrink-0 rounded p-0.5 text-on-surface-variant hover:text-error-alert"
                        on:click=move |_| {
                            crate::editor_ops::remove_vehicle(id_del.clone());
                        }
                    >
                        <MaterialIcon name="delete" class="block text-sm" />
                    </button>
                </div>
            };

            if !open {
                return head.into_any();
            }

            // T-425 — heading authoring. Placed vehicles defaulted to rotation 0.0 at drop; this
            // field is how the operator sets a real heading without delete-and-replace.
            let heading_row = if let Some(h) = heading {
                let id_h = vid.clone();
                view! {
                    <div class="flex items-center gap-1.5 py-0.5 pl-7 pr-1.5">
                        <span class="shrink-0 text-label-sm text-on-surface-variant">"Heading°"</span>
                        <input
                            type="number"
                            min="0"
                            max="360"
                            step="1"
                            aria-label="Vehicle heading degrees"
                            class="w-16 shrink-0 rounded border border-outline-variant/40 bg-surface-container-lowest/60 px-1 py-0.5 text-right font-mono text-label-sm tabular-nums text-on-surface outline-none focus:border-primary/60"
                            prop:value=format!("{h:.0}")
                            on:change=move |ev| {
                                let Ok(raw) = event_target_value(&ev).trim().parse::<f64>() else {
                                    return;
                                };
                                let deg = ((raw % 360.0) + 360.0) % 360.0;
                                crate::editor_ops::set_vehicle_heading(id_h.clone(), deg);
                            }
                        />
                    </div>
                }
                .into_any()
            } else {
                ().into_any()
            };

            let rows_for_edit = cargo.clone();
            let id_add = vid.clone();
            let editor = cargo
                .into_iter()
                .enumerate()
                .map(|(i, row)| {
                    let label = label_of(&row.item);
                    let (base_q, base_r) = (rows_for_edit.clone(), rows_for_edit.clone());
                    let (id_q, id_r) = (vid.clone(), vid.clone());
                    view! {
                        <div class="flex items-center gap-1.5 py-0.5 pl-7 pr-1.5">
                            <span class="min-w-0 flex-1 truncate text-label-sm text-on-surface-variant">
                                {label}
                            </span>
                            <input
                                type="number"
                                min="1"
                                aria-label="Quantity"
                                class="w-14 shrink-0 rounded border border-outline-variant/40 bg-surface-container-lowest/60 px-1 py-0.5 text-right font-mono text-label-sm tabular-nums text-on-surface outline-none focus:border-primary/60"
                                prop:value=row.qty.to_string()
                                on:change=move |ev| {
                                    let Ok(q) = event_target_value(&ev).trim().parse::<i64>() else {
                                        return;
                                    };
                                    let mut next = base_q.clone();
                                    if let Some(r) = next.get_mut(i) {
                                        r.qty = q;
                                    }
                                    crate::editor_ops::set_vehicle_cargo(id_q.clone(), next);
                                }
                            />
                            <button
                                type="button"
                                aria-label="Remove cargo row"
                                class="shrink-0 rounded p-0.5 text-on-surface-variant hover:text-error-alert"
                                on:click=move |_| {
                                    let mut next = base_r.clone();
                                    if i < next.len() {
                                        next.remove(i);
                                    }
                                    crate::editor_ops::set_vehicle_cargo(id_r.clone(), next);
                                }
                            >
                                <MaterialIcon name="close" class="block text-sm" />
                            </button>
                        </div>
                    }
                })
                .collect_view();

            let base_add = rows_for_edit;
            view! {
                {head}
                {heading_row}
                {editor}
                <div class="py-0.5 pl-7 pr-1.5">
                    <select
                        aria-label="Add cargo"
                        class="w-full rounded border border-outline-variant/40 bg-surface-container-lowest/60 px-1.5 py-0.5 text-label-sm text-on-surface outline-none focus:border-primary/60"
                        on:change=move |ev| {
                            let item = event_target_value(&ev);
                            if item.is_empty() {
                                return;
                            }
                            let mut next = base_add.clone();
                            // Adding an item already present bumps it rather than writing a second
                            // row for the same prefab: `qty` is a unit count, so two rows of 3 and
                            // one row of 6 are the same load, and the collapsed form is the one an
                            // author can read.
                            if let Some(r) = next.iter_mut().find(|r| r.item == item) {
                                r.qty = r.qty.saturating_add(1);
                            } else {
                                next.push(VehicleCargoRow { item, qty: 1 });
                            }
                            crate::editor_ops::set_vehicle_cargo(id_add.clone(), next);
                        }
                    >
                        <option value="">"Add cargo…"</option>
                        {addable
                            .get_value()
                            .into_iter()
                            .map(|(rn, label)| view! { <option value=rn>{label}</option> })
                            .collect_view()}
                    </select>
                </div>
            }
            .into_any()
        })
        .collect_view();

    view! {
        <div class="mt-3 border-t border-white/5 pt-2">
            <h3 class="text-label-md font-semibold text-on-surface">"Placed"</h3>
            <div class="mt-1">{body}</div>
        </div>
    }
    .into_any()
}

/// Native shell: no document, nothing to list. See the wasm sibling.
#[cfg(not(target_arch = "wasm32"))]
fn placed_vehicles_panel(
    doc_tick: RwSignal<u64>,
    registry_items: RwSignal<Option<Vec<crate::dto::RegistryItem>>>,
    expanded: RwSignal<std::collections::HashSet<String>>,
) -> AnyView {
    let _ = (doc_tick, registry_items, expanded);
    ().into_any()
}

/// Collect the folder ids whose `default_expanded` is false — the palette's initial collapsed
/// set (`buildCatalogTree` rule 3: only depth-0 faction folders start open). T-172 B6.
fn collapsed_seed(nodes: &[CatalogNode], out: &mut std::collections::HashSet<String>) {
    for n in nodes {
        if n.payload.is_none() && !n.children.is_empty() && !n.default_expanded {
            out.insert(n.id.clone());
        }
        collapsed_seed(&n.children, out);
    }
}

/// Left dock — the live **Editor Layers** outliner (spec O1). Click a folder to make it the drop
/// target, a slot to select it (no camera move — React parity).
///
/// T-177 B1 — the ORBAT browse/select tree moved OUT of this dock (the dual-tree split was bad UX)
/// into the top-strip **ORBAT Manager** modal ([`OrbatManagerDialog`], the T-071.0 cutover). Squad
/// MANAGEMENT (reparent/rename/delete) stays T-071.1+. This dock is now Editor Layers only.
#[component]
pub fn DockLeft(
    /// The Editor Layers tree, rebuilt from the doc at every mutation (`editor_ops::refresh_docks`).
    nodes: RwSignal<Vec<OutlinerNode>>,
    selected: RwSignal<Vec<String>>,
    active_layer: RwSignal<Option<String>>,
) -> impl IntoView {
    // T-172 B9 — screen-05 bottom icon strip: React's LeftSidebar BOTTOM_TABS were explicitly
    // visual-only (Hierarchy active), so present-but-disabled is the honest parity.
    let strip_btn = |icon: &'static str, label: &'static str, active: bool| {
        view! {
            <button
                type="button"
                disabled=true
                title=label
                aria-label=label
                class=if active {
                    "rounded-md p-1.5 text-primary"
                } else {
                    "rounded-md p-1.5 text-outline"
                }
            >
                <MaterialIcon name=icon class="block text-base" />
            </button>
        }
    };
    view! {
        <aside class=DOCK_L>
            <h2 class="text-label-sm font-semibold uppercase tracking-wide text-on-surface">
                "Editor Layers"
            </h2>
            <div class="mt-1">
                {virtual_tree(
                    nodes,
                    selected,
                    active_layer,
                    "editorLayers",
                    "No objects placed yet.",
                    false,
                )}
            </div>
            <div class="mt-auto flex items-center justify-between border-t border-outline-variant/20 pt-2">
                {strip_btn("account_tree", "Hierarchy (visual only)", true)}
                {strip_btn("layers", "Layers (visual only)", false)}
                {strip_btn("inventory_2", "Assets (visual only)", false)}
                {strip_btn("history", "History (visual only)", false)}
                {strip_btn("settings", "Settings (visual only)", false)}
            </div>
        </aside>
    }
}

/// T-180.7 — Stitch ORBAT Manager (near-fullscreen live graph). Implementation lives in
/// [`crate::orbat_manager`]; re-exported so `mission_editor` mount path stays stable.
pub use crate::orbat_manager::OrbatManagerDialog;

// ── T-180.5 — Eden side chips (no F1–F6, no CIV) ─────────────────────────────────────────────────

/// Ordered chip labels the DockRight row iterates. Gate E1/E5 pin this exact list.
pub const EDEN_SIDE_CHIPS: &[&str] = &["BLUFOR", "OPFOR", "INDFOR", "Objects"];

/// Which Eden chip is selected (side place vs Objects world-entity place).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EdenChip {
    Blufor,
    Opfor,
    Indfor,
    Objects,
}

impl EdenChip {
    /// Chip row label / `aria-label`.
    pub const fn label(self) -> &'static str {
        match self {
            Self::Blufor => "BLUFOR",
            Self::Opfor => "OPFOR",
            Self::Indfor => "INDFOR",
            Self::Objects => "Objects",
        }
    }

    /// Tailwind fill class (Aegis tokens matching map SIDE_* / tactical-yellow).
    pub const fn fill_class(self) -> &'static str {
        match self {
            Self::Blufor => "bg-primary",
            Self::Opfor => "bg-error-alert",
            Self::Indfor => "bg-success",
            Self::Objects => "bg-tactical-yellow",
        }
    }

    /// Parse a chip label from [`EDEN_SIDE_CHIPS`].
    pub fn from_label(label: &str) -> Option<Self> {
        match label {
            "BLUFOR" => Some(Self::Blufor),
            "OPFOR" => Some(Self::Opfor),
            "INDFOR" => Some(Self::Indfor),
            "Objects" => Some(Self::Objects),
            _ => None,
        }
    }
}

/// Apply a chip click to the shared place signals (same `active_side` OpsCtx / `place_at` read).
///
/// Side chips clear Objects mode and set the place side. Objects sets `objects_mode` only (leaves
/// `active_side` unchanged so flipping back restores the last side).
pub fn apply_eden_chip(
    chip: EdenChip,
    active_side: RwSignal<String>,
    objects_mode: RwSignal<bool>,
) {
    match chip {
        EdenChip::Objects => objects_mode.set(true),
        EdenChip::Blufor => {
            objects_mode.set(false);
            active_side.set(String::from("BLUFOR"));
        }
        EdenChip::Opfor => {
            objects_mode.set(false);
            active_side.set(String::from("OPFOR"));
        }
        EdenChip::Indfor => {
            objects_mode.set(false);
            active_side.set(String::from("INDFOR"));
        }
    }
}

/// Whether the chip row should show `chip` as selected given current side + objects mode.
pub fn eden_chip_selected(chip: EdenChip, active_side: &str, objects_mode: bool) -> bool {
    match chip {
        EdenChip::Objects => objects_mode,
        EdenChip::Blufor => !objects_mode && active_side == "BLUFOR",
        EdenChip::Opfor => !objects_mode && active_side == "OPFOR",
        EdenChip::Indfor => !objects_mode && active_side == "INDFOR",
    }
}

/// Right dock — the **Factions** palette (spec O2), off the live `GET /api/v1/registry`. Leaves drag
/// onto the map to place their slot. `fm_open` toggles the T-167 Faction Manager dialog.
///
/// T-180.5 — Eden side chips above search drive `active_side` / Objects stub.
///
/// T-215 — the **Vehicles** tab is a real palette off the same `/registry` fetch (`vehicle_catalog`,
/// built by `asset_catalog::build_vehicle_catalog_tree`), not the T-070 placeholder it was. Its
/// leaves arm `editor_ops::begin_place_vehicle`, so a release on the canvas writes a `vehiclesById`
/// row at that world point.
#[component]
pub fn DockRight(
    catalog: RwSignal<CatalogState>,
    /// T-215 — the `kind == "vehicle"` half of the same registry fetch.
    vehicle_catalog: RwSignal<CatalogState>,
    /// T-215 — the raw registry rows, for the placed-vehicle cargo picker's labels and options.
    registry_items: RwSignal<Option<Vec<crate::dto::RegistryItem>>>,
    /// T-215 — the doc-change tick the placed-vehicle list re-reads on.
    doc_tick: RwSignal<u64>,
    fm_open: RwSignal<bool>,
    active_side: RwSignal<String>,
    objects_mode: RwSignal<bool>,
) -> impl IntoView {
    // Palette collapse state (T-172 B6), seeded from `default_expanded` whenever the catalog
    // turns Ready or the Eden side chip rebuilds the tree (T-255). User toggles stick until the
    // next side-driven rebuild (NATO folders are meaningless under OPFOR).
    let palette_collapsed = RwSignal::new(std::collections::HashSet::<String>::new());
    Effect::new(move |_| {
        let _ = active_side.get(); // T-255 — re-seed when chips flip the filtered tree
        if let CatalogState::Ready(nodes) = catalog.get() {
            let mut set = std::collections::HashSet::new();
            collapsed_seed(&nodes, &mut set);
            palette_collapsed.set(set);
        }
    });
    // T-172 B9 — screen-05 palette chrome: FACTIONS / VEHICLES / MARKERS tabs + Asset Browser
    // search. Vehicles/Markers placement stays T-070/T-069 — React's tabs were stubs too, so the
    // panels say exactly that. Search filters the catalog (T-055 behavior) and force-expands
    // matches (an empty collapse set while a query is live).
    let tab = RwSignal::new(0usize);
    let search = RwSignal::new(String::new());
    let no_collapse = RwSignal::new(std::collections::HashSet::<String>::new());
    // T-215 — the Vehicles tab keeps its OWN collapse set and search box. Sharing either with the
    // Factions tab would mean a query typed against 178 vehicles silently filtering the roles the
    // author switches back to, and a folder id collision between two trees built from different
    // path vocabularies.
    let vehicle_collapsed = RwSignal::new(std::collections::HashSet::<String>::new());
    let vehicle_seeded = StoredValue::new(false);
    Effect::new(move |_| {
        if vehicle_seeded.get_value() {
            return;
        }
        if let CatalogState::Ready(nodes) = vehicle_catalog.get() {
            let mut set = std::collections::HashSet::new();
            collapsed_seed(&nodes, &mut set);
            vehicle_collapsed.set(set);
            vehicle_seeded.set_value(true);
        }
    });
    let vehicle_search = RwSignal::new(String::new());
    // T-254 — Objects chip palette (entities[]): own collapse + search, built from registry_items.
    let object_collapsed = RwSignal::new(std::collections::HashSet::<String>::new());
    let object_search = RwSignal::new(String::new());
    // T-215 — which placed vehicles have their cargo editor open.
    let vehicle_expanded = RwSignal::new(std::collections::HashSet::<String>::new());
    let tab_btn = move |i: usize, label: &'static str| {
        view! {
            <button
                type="button"
                class=move || {
                    if tab.get() == i {
                        "border-b-2 border-primary px-1.5 pb-1 text-label-sm font-semibold uppercase tracking-wide text-on-surface"
                    } else {
                        "border-b-2 border-transparent px-1.5 pb-1 text-label-sm font-semibold uppercase tracking-wide text-on-surface-variant transition-colors hover:text-on-surface"
                    }
                }
                on:click=move |_| tab.set(i)
            >
                {label}
            </button>
        }
    };
    view! {
        <aside class=DOCK_R>
            <div class="flex items-center justify-between">
                <div class="flex items-center gap-1">
                    {tab_btn(0, "Factions")}
                    {tab_btn(1, "Vehicles")}
                    {tab_btn(2, "Markers")}
                </div>
                <button
                    type="button"
                    aria-label="Manage factions"
                    on:click=move |_| fm_open.set(true)
                    class="rounded-md px-1.5 py-0.5 text-label-sm font-semibold uppercase tracking-wide text-primary transition-colors hover:bg-primary/15"
                >
                    "Manage"
                </button>
            </div>
            {move || match tab.get() {
                0 => view! {
                    <h3 class="mt-2 text-label-md font-semibold text-on-surface">"Asset Browser"</h3>
                    <p class="mt-0.5 text-label-sm normal-case text-outline">
                        "Drag a role onto the map to place its slot."
                    </p>
                    // T-180.5 — Eden side chips above search (E-L4). No F1–F6 row, no CIV.
                    <div
                        class="mt-2 flex items-center gap-1.5"
                        role="group"
                        aria-label="Eden side"
                    >
                        {EDEN_SIDE_CHIPS
                            .iter()
                            .filter_map(|label| EdenChip::from_label(label))
                            .map(|chip| {
                                let fill = chip.fill_class();
                                view! {
                                    <button
                                        type="button"
                                        aria-label=chip.label()
                                        aria-pressed=move || {
                                            eden_chip_selected(
                                                chip,
                                                &active_side.get(),
                                                objects_mode.get(),
                                            )
                                        }
                                        class=move || {
                                            let selected = eden_chip_selected(
                                                chip,
                                                &active_side.get(),
                                                objects_mode.get(),
                                            );
                                            if selected {
                                                format!(
                                                    "{fill} h-5 w-8 shrink-0 rounded-sm ring-2 ring-offset-1 ring-offset-surface-container-lowest ring-white/90 opacity-100"
                                                )
                                            } else {
                                                format!(
                                                    "{fill} h-5 w-8 shrink-0 rounded-sm opacity-45 transition-opacity hover:opacity-75"
                                                )
                                            }
                                        }
                                        on:click=move |_| {
                                            // T-255 — writes `active_side`; mission_editor's
                                            // Effect rebuilds `catalog` via build_catalog_tree(_, side).
                                            apply_eden_chip(chip, active_side, objects_mode)
                                        }
                                    />
                                }
                            })
                            .collect_view()}
                    </div>
                    <input
                        type="search"
                        aria-label=move || {
                            if objects_mode.get() {
                                "Search objects"
                            } else {
                                "Search assets"
                            }
                        }
                        placeholder=move || {
                            if objects_mode.get() {
                                "Search objects…"
                            } else {
                                "Search assets…"
                            }
                        }
                        class="mt-2 w-full rounded-md border border-outline-variant/40 bg-surface-container-lowest/60 px-2.5 py-1.5 text-label-sm text-on-surface outline-none transition-colors placeholder:text-outline focus:border-primary/60"
                        on:input=move |ev| {
                            let v = event_target_value(&ev);
                            if objects_mode.get_untracked() {
                                object_search.set(v);
                            } else {
                                search.set(v);
                            }
                        }
                    />
                    <div class="mt-2">
                        {move || {
                            if objects_mode.get() {
                                let items = registry_items.get().unwrap_or_default();
                                let nodes =
                                    crate::asset_catalog::build_object_catalog_tree(&items);
                                if nodes.is_empty() {
                                    return view! {
                                        <p class="text-label-sm text-outline">
                                            "No placeable objects in the registry."
                                        </p>
                                    }
                                    .into_any();
                                }
                                let q = object_search.get();
                                if q.trim().is_empty() {
                                    object_collapsed.track();
                                    return palette_rows(
                                        &nodes,
                                        0,
                                        &[],
                                        &[],
                                        object_collapsed,
                                        PaletteKind::Object,
                                    );
                                }
                                let filtered = crate::asset_catalog::filter_catalog(&nodes, &q);
                                if filtered.is_empty() {
                                    return view! {
                                        <p class="text-label-sm text-outline">
                                            "No objects match."
                                        </p>
                                    }
                                    .into_any();
                                }
                                return palette_rows(
                                    &filtered,
                                    0,
                                    &[],
                                    &[],
                                    no_collapse,
                                    PaletteKind::Object,
                                );
                            }
                            match catalog.get() {
                                CatalogState::Loading => {
                                    view! {
                                        <p class="text-label-sm text-outline">"Loading assets…"</p>
                                    }
                                        .into_any()
                                }
                                CatalogState::Failed => {
                                    view! {
                                        <p class="text-label-sm text-outline">
                                            "Could not load the catalog."
                                        </p>
                                    }
                                        .into_any()
                                }
                                CatalogState::Ready(nodes) if nodes.is_empty() => {
                                    view! {
                                        <p class="text-label-sm text-outline">"No placeable assets."</p>
                                    }
                                        .into_any()
                                }
                                CatalogState::Ready(nodes) => {
                                    let q = search.get();
                                    if q.trim().is_empty() {
                                        // Track the collapse set so a chevron toggle re-renders the
                                        // tree (palette_rows reads it untracked).
                                        palette_collapsed.track();
                                        palette_rows(
                                            &nodes,
                                            0,
                                            &[],
                                            &[],
                                            palette_collapsed,
                                            PaletteKind::Character,
                                        )
                                    } else {
                                        let filtered =
                                            crate::asset_catalog::filter_catalog(&nodes, &q);
                                        if filtered.is_empty() {
                                            view! {
                                                <p class="text-label-sm text-outline">
                                                    "No assets match."
                                                </p>
                                            }
                                                .into_any()
                                        } else {
                                            palette_rows(
                                                &filtered,
                                                0,
                                                &[],
                                                &[],
                                                no_collapse,
                                                PaletteKind::Character,
                                            )
                                        }
                                    }
                                }
                            }
                        }}
                    </div>
                }
                    .into_any(),
                // T-215 — Vehicles: the same tree machinery over the `kind == "vehicle"` rows.
                // A leaf drop writes a `vehiclesById` row at the world point, owned by whichever
                // Eden side the Factions tab's chips have selected (`active_side`) — the chips are
                // not repeated here because there is one active side per editor, not per tab.
                1 => view! {
                    <h3 class="mt-2 text-label-md font-semibold text-on-surface">"Vehicles"</h3>
                    <p class="mt-0.5 text-label-sm normal-case text-outline">
                        "Drag a vehicle onto the map to place it."
                    </p>
                    <input
                        type="search"
                        aria-label="Search vehicles"
                        placeholder="Search vehicles…"
                        class="mt-2 w-full rounded-md border border-outline-variant/40 bg-surface-container-lowest/60 px-2.5 py-1.5 text-label-sm text-on-surface outline-none transition-colors placeholder:text-outline focus:border-primary/60"
                        on:input=move |ev| vehicle_search.set(event_target_value(&ev))
                    />
                    <div class="mt-2">
                        {move || {
                            if objects_mode.get() {
                                return view! {
                                    <p class="text-label-sm text-outline">
                                        "Objects place from the Factions tab while the Objects chip is selected."
                                    </p>
                                }
                                    .into_any();
                            }
                            match vehicle_catalog.get() {
                                CatalogState::Loading => {
                                    view! {
                                        <p class="text-label-sm text-outline">"Loading vehicles…"</p>
                                    }
                                        .into_any()
                                }
                                CatalogState::Failed => {
                                    view! {
                                        <p class="text-label-sm text-outline">
                                            "Could not load the catalog."
                                        </p>
                                    }
                                        .into_any()
                                }
                                CatalogState::Ready(nodes) if nodes.is_empty() => {
                                    view! {
                                        <p class="text-label-sm text-outline">
                                            "No placeable vehicles."
                                        </p>
                                    }
                                        .into_any()
                                }
                                CatalogState::Ready(nodes) => {
                                    let q = vehicle_search.get();
                                    if q.trim().is_empty() {
                                        vehicle_collapsed.track();
                                        palette_rows(
                                            &nodes,
                                            0,
                                            &[],
                                            &[],
                                            vehicle_collapsed,
                                            PaletteKind::Vehicle,
                                        )
                                    } else {
                                        let filtered =
                                            crate::asset_catalog::filter_catalog(&nodes, &q);
                                        if filtered.is_empty() {
                                            view! {
                                                <p class="text-label-sm text-outline">
                                                    "No vehicles match."
                                                </p>
                                            }
                                                .into_any()
                                        } else {
                                            palette_rows(
                                                &filtered,
                                                0,
                                                &[],
                                                &[],
                                                no_collapse,
                                                PaletteKind::Vehicle,
                                            )
                                        }
                                    }
                                }
                            }
                        }}
                    </div>
                    {move || placed_vehicles_panel(doc_tick, registry_items, vehicle_expanded)}
                }
                    .into_any(),
                _ => view! {
                    <p class="mt-3 text-label-sm normal-case text-outline">
                        "Marker placement lands in T-069."
                    </p>
                }
                    .into_any(),
            }}
        </aside>
    }
}

/// Bottom Toolbelt — Select (active) + Ruler/LoS disabled stubs, then the mono CUR X/Y/Z +
/// SEL/OBJ readout.
///
/// T-172 B2/B9: Z is DEM-fed (em-dash until the grid publishes / off-coverage), and with exactly
/// one slot selected the readout swaps CUR→SEL and shows that slot's x/y/z (React parity). The
/// per-axis `title="Cursor …"` handles stay constant — they are the frozen cur-smoke's DOM hooks.
#[component]
pub fn BottomToolbelt(
    /// Cursor world position + DEM z, `None` when the pointer is off the map (em-dash cells).
    cursor: RwSignal<Option<(f64, f64, Option<f64>)>>,
    sel_count: RwSignal<usize>,
    obj_count: RwSignal<usize>,
    /// Live selection mirror — drives the CUR↔SEL swap.
    selected_ids: RwSignal<Vec<String>>,
    /// T-172 B9 — debounced compiled-payload estimate (None → `—`).
    #[prop(optional)]
    sz_bytes: Option<RwSignal<Option<usize>>>,
) -> impl IntoView {
    // Exactly-one-selected → that slot's x/y/z from the doc. Recomputes on selection change AND
    // on the post-mutation selected_ids re-set (drag commit), so it never shows a stale position.
    // (`editor_ops` is wasm-only; the native view shell always renders CUR.)
    let sel_xyz = Memo::new(move |_| -> Option<(f64, f64, f64)> {
        let ids = selected_ids.get();
        if ids.len() == 1 {
            #[cfg(target_arch = "wasm32")]
            {
                return crate::editor_ops::read_attrs(&ids[0]).map(|a| (a.x, a.y, a.z));
            }
        }
        let _ = ids;
        None
    });
    let axis_val = move |i: usize| match sel_xyz.get() {
        Some((x, y, z)) => fmt_coord(Some([x, y, z][i])),
        None => fmt_coord(cursor.get().and_then(|c| match i {
            0 => Some(c.0),
            1 => Some(c.1),
            _ => c.2,
        })),
    };
    view! {
        <div class=TOOLBELT>
            <button type="button" class=TOOL_ACTIVE aria-pressed="true" title="Select">
                <MaterialIcon name="arrow_selector_tool" class="block text-base" />
                <span class="hidden sm:inline">"Select"</span>
            </button>
            <button type="button" class=TOOL_DISABLED disabled=true title="Ruler (soon)">
                <MaterialIcon name="straighten" class="block text-base" />
                <span class="hidden sm:inline">"Ruler"</span>
            </button>
            <button type="button" class=TOOL_DISABLED disabled=true title="Line of sight (soon)">
                <MaterialIcon name="visibility" class="block text-base" />
                <span class="hidden sm:inline">"LoS"</span>
            </button>
            <span class="mx-1 h-5 w-px bg-white/10"></span>
            <div class="flex items-center gap-2 px-1 font-mono text-code-md text-on-surface-variant">
                <span class="text-outline" title="Cursor">
                    {move || if sel_xyz.get().is_some() { "SEL" } else { "CUR" }}
                </span>
                // T-159.22 — `title` (not `aria-label`): these are roleless `<span>`s, where an
                // `aria-label` is ignored by AT and would be a fake a11y name. `title` is a real
                // tooltip AND the CUR gate's DOM handle, matching the `title="Cursor"` idiom above.
                <span title="Cursor X">
                    "X"
                    <span class="ml-1 text-on-surface tabular-nums">{move || axis_val(0)}</span>
                </span>
                <span title="Cursor Y">
                    "Y"
                    <span class="ml-1 text-on-surface tabular-nums">{move || axis_val(1)}</span>
                </span>
                <span title="Cursor Z">
                    "Z"
                    <span class="ml-1 text-on-surface tabular-nums">{move || axis_val(2)}</span>
                </span>
            </div>
            <span class="mx-1 h-5 w-px bg-white/10"></span>
            <div
                class="flex items-center gap-2 px-1 font-mono text-code-md tabular-nums text-on-surface-variant"
                title="Placed slots on map / current selection"
            >
                <span>
                    "OBJ"
                    <span class="ml-1 text-on-surface">{move || obj_count.get()}</span>
                </span>
                <span>
                    "SEL"
                    <span class="ml-1 text-on-surface">{move || sel_count.get()}</span>
                </span>
                <span title="Estimated save payload">
                    "SZ"
                    <span class="ml-1 text-on-surface">
                        {move || {
                            sz_bytes
                                .and_then(|s| s.get())
                                .map_or_else(
                                    || "—".to_string(),
                                    crate::mission_size::format_bytes,
                                )
                        }}
                    </span>
                </span>
            </div>
        </div>
    }
}

/// Mission Settings dialog (MissionSettingsDialog.tsx — environment half). Terrain (readonly) +
/// time / weather flow through [`author_env`] (one undo step each); the render-pref controls (map
/// style, grid, hillshade, world-layer toggles) are live below them since T-173 P6. Renders no DOM
/// while closed. T-159.26.
///
/// **T-192:** time and weather additionally mirror to the `missions` row through [`RowMirror`] on
/// commit.
///
/// **T-193:** the View Distance field and the Thermals toggle are gone, and [`ENV_UNCARRIED_NOTE`]
/// stands where they were. They were fully working controls authoring keys that no compiled
/// document, no row column and no mod script has ever read — [`CARRIED_ENV_KEYS`] carries the whole
/// argument for deleting them rather than trying to carry them through.
///
/// **T-224:** [`render_flow_section`] adds the mission-flow block — duration, briefing, safe start
/// and join-in-progress — and [`SETTINGS_UNREAD_NOTE`] says why respawn, spectator policy, night
/// vision and tickets are not beside them.
#[component]
pub fn MissionSettingsDialog(open: RwSignal<bool>, doc_tick: RwSignal<u64>) -> impl IntoView {
    // Esc closes (the suite Dialog behavior).
    #[cfg(target_arch = "wasm32")]
    {
        let esc = window_event_listener(leptos::ev::keydown, move |ev| {
            if open.get_untracked() && ev.key() == "Escape" {
                open.set(false);
            }
        });
        on_cleanup(move || esc.remove());
    }
    // T-192 — read the route id + auth store here, in the component body: the reactive owner is
    // live at setup and gone by the time a control's `on:change` fires.
    #[cfg(target_arch = "wasm32")]
    let row_mirror = RowMirror::from_route();
    let ctrl = "w-full rounded-md border border-outline-variant/40 bg-surface-container-lowest/60 px-2.5 py-1.5 text-label-md text-on-surface outline-none transition-colors focus:border-primary/60";
    move || {
        if !open.get() {
            return None;
        }
        let _ = doc_tick.get(); // re-read env on undo/redo while open
        #[cfg(target_arch = "wasm32")]
        let env = crate::editor_ops::read_env();
        #[cfg(not(target_arch = "wasm32"))]
        let env = crate::dto::MissionEnv::default();
        Some(view! {
            <div
                class="animate-overlay-fade fixed inset-0 z-50 bg-black/50 backdrop-blur-sm transition-opacity duration-200"
                on:click=move |_| open.set(false)
            ></div>
            <div class="glass animate-dialog-in fixed top-1/2 left-1/2 z-50 flex max-h-[85vh] w-[92vw] max-w-lg -translate-x-1/2 -translate-y-1/2 flex-col rounded-xl shadow-2xl outline-none transition-all duration-200">
                <div class="flex items-start justify-between gap-4 border-b border-outline-variant/30 px-6 py-4">
                    <div class="min-w-0">
                        <h2 class="text-headline-sm text-on-surface">"Mission Settings"</h2>
                        <p class="mt-1 text-label-md text-on-surface-variant">
                            "Environment and flow for this mission."
                        </p>
                    </div>
                    <button
                        type="button"
                        aria-label="Close"
                        on:click=move |_| open.set(false)
                        class="shrink-0 rounded-md p-1 text-outline transition-colors hover:bg-surface-variant/50 hover:text-on-surface"
                    >
                        <MaterialIcon name="close" />
                    </button>
                </div>
                <div class="custom-scrollbar flex-1 overflow-y-auto px-6 py-5">
                    <div class="flex flex-col gap-4">
                        <label class="flex flex-col gap-1">
                            <span class="text-label-sm uppercase tracking-wider text-outline">
                                "Terrain"
                            </span>
                            <div class="rounded-md border border-outline-variant/20 bg-surface-container-lowest/30 px-2.5 py-1.5 font-mono text-code-md text-on-surface-variant">
                                {env.terrain.clone()}
                            </div>
                        </label>
                        <div class="grid grid-cols-2 gap-3">
                            <label class="flex flex-col gap-1">
                                <span class="text-label-sm uppercase tracking-wider text-outline">
                                    "Time"
                                </span>
                                <input
                                    type="time"
                                    value=env.time.clone()
                                    on:input=move |ev| {
                                        #[cfg(target_arch = "wasm32")]
                                        {
                                            let t = event_target_value(&ev);
                                            author_env("time", t.as_str().into());
                                            // T-192 — same handler as the doc write on purpose; a
                                            // `change` listener would not survive this dialog's
                                            // rebuild-per-doc-tick. Partial values arrive as "" and
                                            // repeats are absorbed by the mirror's dedupe.
                                            row_mirror.set_time(&t);
                                        }
                                        #[cfg(not(target_arch = "wasm32"))]
                                        let _ = &ev;
                                    }
                                    class=ctrl
                                />
                            </label>
                            // T-193 — Weather moved up beside Time. They are the two settings a
                            // compiled mission actually carries, and they were only ever apart
                            // because View Distance held this cell.
                            <label class="flex flex-col gap-1">
                                <span class="text-label-sm uppercase tracking-wider text-outline">
                                    "Weather"
                                </span>
                                <select
                                    prop:value=env.weather.clone()
                                    on:change=move |ev| {
                                        #[cfg(target_arch = "wasm32")]
                                        {
                                            let w = event_target_value(&ev);
                                            author_env("weather", w.as_str().into());
                                            row_mirror.set_weather(&w);
                                        }
                                        #[cfg(not(target_arch = "wasm32"))]
                                        let _ = &ev;
                                    }
                                    class=ctrl
                                >
                                    <option value="clear">"Clear"</option>
                                    <option value="overcast">"Overcast"</option>
                                    <option value="heavy_rain">"Heavy Rain"</option>
                                    <option value="dense_fog">"Dense Fog"</option>
                                </select>
                            </label>
                        </div>
                        <p class="text-label-sm normal-case text-outline">{ENV_UNCARRIED_NOTE}</p>
                        {render_flow_section(ctrl)}
                        {render_prefs_section(&env)}
                    </div>
                </div>
            </div>
        })
    }
}

/// T-224 — the mission-flow half of Mission Settings: how long the round runs, how long the
/// briefing and safe start are, and who may join once it has started.
///
/// Every control here writes through [`author_env`], the same one gate the Time and Weather controls
/// take, so the "does anything read this back?" question is asked once per key rather than once per
/// control. [`AUTHORED_FLOW_KEYS`] holds the answers, the compiled path each key becomes, and the
/// one hop (`flatten.rs`, not this slice's file) that is still hardcoded.
///
/// The four settings this dialog deliberately does NOT grow — respawn, spectator policy, night
/// vision and per-faction tickets — are covered by [`SETTINGS_UNREAD_NOTE`], which renders below.
///
/// **`change`, not `input`.** A duration box authored per keystroke would file `5`, `54`, `540`,
/// `5400` as four undo steps, and each of those bumps `doc_tick`, which rebuilds this whole subtree
/// out from under the caret. `change` fires on blur/Enter — one undo step per settled value, and the
/// rebuild lands after the author has already left the field. (Weather already uses `change` for the
/// same reason; Time uses `input` because a `<input type="time">` has no half-typed state worth
/// suppressing and T-192's row mirror rides that handler.)
///
/// Inert on the native view shell (no document), exactly like [`render_prefs_section`].
fn render_flow_section(ctrl: &'static str) -> AnyView {
    #[cfg(not(target_arch = "wasm32"))]
    {
        let _ = ctrl;
        return ().into_any();
    }
    #[cfg(target_arch = "wasm32")]
    {
        let sect = "text-label-sm uppercase tracking-wider text-outline";
        let hint = "text-label-sm normal-case text-outline";

        // Mission duration leads, out of stage order (briefing → safe start → round), because it is
        // the one an author comes to this dialog to set; the other two are the stage lengths around
        // it. Each row is (key, label, committed value, the sentence the field needs or "").
        let rows: [(&str, &str, i64, &str); 3] = [
            (
                "timeLimitSeconds",
                "Mission duration",
                read_flow_seconds("timeLimitSeconds", FLOW_DEFAULT_TIMELIMIT_S),
                "0 means no time limit — the round will not end on the clock.",
            ),
            (
                "briefingSeconds",
                "Briefing",
                read_flow_seconds("briefingSeconds", FLOW_DEFAULT_BRIEFING_S),
                "Announced on entering the briefing; the stage is advanced by an admin, not by this \
                 clock.",
            ),
            (
                "safeStartSeconds",
                "Safe start",
                read_flow_seconds("safeStartSeconds", FLOW_DEFAULT_SAFESTART_S),
                "",
            ),
        ];

        let duration_rows = rows
            .into_iter()
            .map(|(key, label, committed, hint_text)| {
                view! {
                    <label class="flex flex-col gap-1">
                        <span class=sect>
                            {format!("{label} — {}", fmt_duration_secs(committed))}
                        </span>
                        <input
                            type="number"
                            min="0"
                            step="1"
                            value=committed.to_string()
                            on:change=move |ev| {
                                let raw = event_target_value(&ev);
                                if let Some(secs) = parse_flow_seconds(&raw) {
                                    author_env(key, secs.into());
                                } else {
                                    // A refused value must not stay on screen. Leaving "-1" in the
                                    // box while the document still holds 5400 is the editor showing
                                    // the author a setting they do not have — the same lie as a
                                    // silently reverted one, told the other way round.
                                    leptos::logging::warn!(
                                        "refusing meta.environment.{key} = {raw:?}: flow durations are whole seconds >= 0"
                                    );
                                    if let Some(input) = ev
                                        .target()
                                        .and_then(|t| {
                                            wasm_bindgen::JsCast::dyn_into::<
                                                web_sys::HtmlInputElement,
                                            >(t)
                                                .ok()
                                        })
                                    {
                                        input.set_value(&committed.to_string());
                                    }
                                }
                            }
                            class=ctrl
                        />
                        {(!hint_text.is_empty())
                            .then(|| view! { <span class=hint>{hint_text}</span> })}
                    </label>
                }
            })
            .collect::<Vec<_>>();

        let jip = read_flow_jip();
        let jip_options = JIP_OPTIONS
            .into_iter()
            .map(|(value, label)| view! { <option value=value>{label}</option> })
            .collect::<Vec<_>>();

        view! {
            <div class="mt-2 flex flex-col gap-4 border-t border-outline-variant/30 pt-4">
                <span class=sect>"Mission flow"</span>
                {duration_rows}
                <label class="flex flex-col gap-1">
                    <span class=sect>"Join in progress"</span>
                    <select
                        prop:value=jip
                        on:change=move |ev| {
                            let v = event_target_value(&ev);
                            // The <select> can only emit its own options, so this can only fail if
                            // JIP_OPTIONS ever drifts from the schema enum — in which case refusing
                            // is right: TBD_MissionFlow.PolicyFromString maps an unknown string to
                            // ALWAYS, so a bad value holds the door open all round instead of erroring.
                            if JIP_OPTIONS.iter().any(|(k, _)| *k == v) {
                                author_env("jip", v.as_str().into());
                            } else {
                                leptos::logging::error!("refusing meta.environment.jip = {v:?}");
                            }
                        }
                        class=ctrl
                    >
                        {jip_options}
                    </select>
                    <span class=hint>
                        "Whether a player who connects after the mission has started may deploy."
                    </span>
                </label>
                <p class=hint>{SETTINGS_UNREAD_NOTE}</p>
            </div>
        }
        .into_any()
    }
}

/// T-173 P6 — the render-pref half of Mission Settings, restored from the React
/// `MissionSettingsDialog`: basemap view (Satellite / Map), hillshade on/off + strength slider,
/// grid, and the 12 world-layer toggles. Per-mission prefs (hillshade / grid) persist to
/// `meta.environment`; per-user prefs (basemap view + layer toggles) persist to localStorage. Each
/// control applies live to the map host (no reload). On the native view-shell these are inert
/// (no engine), which is fine — the dialog is a wasm surface.
fn render_prefs_section(env: &crate::dto::MissionEnv) -> AnyView {
    #[cfg(not(target_arch = "wasm32"))]
    {
        let _ = env;
        return ().into_any();
    }
    #[cfg(target_arch = "wasm32")]
    {
        use crate::world_layer_prefs as wlp;
        let hillshade_on = env.show_hillshade;
        let hillshade_pct = (env.hillshade_opacity * 100.0).round() as i64;
        let grid_on = env.show_grid;
        let basemap = wlp::load_basemap_view();
        let prefs = wlp::load_prefs();
        let sect = "text-label-sm uppercase tracking-wider text-outline";

        let layer_rows = prefs
            .rows()
            .into_iter()
            .map(|(key, on, label)| {
                view! {
                    <div class="flex items-center justify-between py-0.5">
                        <span class="text-label-md text-on-surface-variant">{label}</span>
                        <input
                            type="checkbox"
                            prop:checked=on
                            on:change=move |ev| {
                                let checked = event_target_checked(&ev);
                                let mut p = wlp::load_prefs();
                                p.set(key, checked);
                                wlp::save_prefs(&p);
                                crate::world_assets::refresh_world_layers();
                            }
                            class="accent-primary"
                        />
                    </div>
                }
            })
            .collect::<Vec<_>>();

        view! {
            <div class="mt-2 flex flex-col gap-4 border-t border-outline-variant/30 pt-4">
                <span class=sect>"Basemap"</span>
                <div class="flex gap-2">
                    {["satellite", "map"]
                        .into_iter()
                        .map(|v| {
                            let active = basemap == v;
                            let label = if v == "satellite" { "Satellite" } else { "Map" };
                            view! {
                                <button
                                    type="button"
                                    class=if active {
                                        "flex-1 rounded-md border border-primary/60 bg-primary/20 px-2.5 py-1.5 text-label-md text-primary"
                                    } else {
                                        "flex-1 rounded-md border border-outline-variant/40 bg-surface-container-lowest/60 px-2.5 py-1.5 text-label-md text-on-surface-variant transition-colors hover:border-primary/40"
                                    }
                                    on:click=move |_| {
                                        wlp::save_basemap_view(v);
                                        crate::world_assets::apply_basemap_view(v);
                                    }
                                >
                                    {label}
                                </button>
                            }
                        })
                        .collect::<Vec<_>>()}
                </div>

                <div class="flex items-center justify-between py-0.5">
                    <span class="text-label-md text-on-surface-variant">"Show hillshade"</span>
                    <input
                        type="checkbox"
                        prop:checked=hillshade_on
                        on:change=move |ev| {
                            let on = event_target_checked(&ev);
                            author_env("showHillshade", on.into());
                            let op = crate::editor_ops::read_env().hillshade_opacity;
                            crate::world_assets::apply_hillshade(on, op);
                        }
                        class="accent-primary"
                    />
                </div>
                <label class="flex flex-col gap-1" class:opacity-40=move || !hillshade_on>
                    <span class=sect>{move || format!("Hillshade strength — {hillshade_pct}%")}</span>
                    <input
                        type="range"
                        min="0"
                        max="100"
                        step="1"
                        prop:disabled=!hillshade_on
                        value=hillshade_pct.to_string()
                        on:input=move |ev| {
                            let pct: f64 = event_target_value(&ev).parse().unwrap_or(40.0);
                            let op = (pct / 100.0).clamp(0.0, 1.0);
                            author_env("hillshadeOpacity", op.into());
                            crate::world_assets::apply_hillshade(true, op);
                        }
                        class="accent-primary"
                    />
                </label>

                <div class="flex items-center justify-between py-0.5">
                    <span class="text-label-md text-on-surface-variant">"Grid"</span>
                    <input
                        type="checkbox"
                        prop:checked=grid_on
                        on:change=move |ev| {
                            let on = event_target_checked(&ev);
                            author_env("showGrid", on.into());
                            crate::world_assets::apply_grid(on);
                        }
                        class="accent-primary"
                    />
                </div>

                <span class=sect>"World layers"</span>
                <div class="flex flex-col gap-1">{layer_rows}</div>
            </div>
        }
        .into_any()
    }
}

#[cfg(test)]
mod tests {
    use super::{
        apply_eden_chip, eden_chip_selected, env_key_is_carried, fmt_duration_secs,
        hhmm_to_minutes, is_mission_row_id, minutes_to_hhmm, mirror_failure_message,
        normalize_clock, parse_flow_seconds, EdenChip, MirrorState, AUTHORED_FLOW_KEYS,
        CARRIED_ENV_KEYS, EDEN_SIDE_CHIPS, ENV_UNCARRIED_NOTE, FLOW_DEFAULT_BRIEFING_S,
        FLOW_DEFAULT_JIP, FLOW_DEFAULT_SAFESTART_S, FLOW_DEFAULT_TIMELIMIT_S, JIP_OPTIONS,
        MIRROR_DEBOUNCE_MS, MIRROR_TIME, MIRROR_WEATHER, SETTINGS_UNREAD_NOTE, VEHICLE_CARGO_KINDS,
    };
    use leptos::prelude::*;

    // ── T-215 — the Vehicles tab ────────────────────────────────────────────────────────────────

    /// The tab was a one-line promise that placement would arrive in T-070, and the only vehicle
    /// path was the ORBAT Manager's derived position. Both halves of the replacement are pinned:
    /// the placeholder is gone, and a Vehicles leaf arms the **vehicle** place, not the character
    /// one.
    ///
    /// Source inspection, following `orbat_manager`'s precedent, because the thing under test is a
    /// Leptos view whose handlers are `#[cfg(target_arch = "wasm32")]` — a native test cannot mount
    /// it or fire the `pointerdown`. What it can do is fail loudly if the wiring is unpicked.
    ///
    /// **Every needle is assembled at run time, and must stay that way.** This test searches the
    /// file it is written in, so a needle spelled out contiguously — in an assertion, or in prose
    /// *about* an assertion — puts itself into the haystack: absence checks can then never pass and
    /// presence checks can never fail. That happened three times while writing this, and the third
    /// was caught only by perturbation: a bare-symbol `contains` for the vehicle arm stayed GREEN
    /// after the leaf was rewired to the character path, because the test's own literal satisfied
    /// it. This program's signature defect in miniature — a check reporting success over an input
    /// it never examined. The needle is therefore the whole call **expression**, which the file's
    /// prose (which names the bare function) never contains.
    #[test]
    fn vehicles_tab_places_instead_of_promising() {
        const SRC: &str = include_str!("eden_chrome.rs");
        let stub = |what: &str, ticket: &str| format!("{what} placement {} {ticket}.", "lands in");
        let arm = |f: &str| format!("editor_ops::{f}{}", "(payload.clone())");

        assert!(
            !SRC.contains(&stub("Vehicle", "T-070")),
            "the Vehicles tab placeholder must be gone"
        );
        assert!(
            SRC.contains(&arm("begin_place_vehicle")),
            "a Vehicles leaf must arm the vehicle place path"
        );
        // The Markers tab is deliberately still a stub (T-069) — if this ever stops being true the
        // assertion above stops proving that THIS tab is the one that got wired.
        assert!(
            SRC.contains(&stub("Marker", "T-069")),
            "the Markers stub is out of scope and must be untouched"
        );

        let ops = include_str!("editor_ops.rs");
        assert!(
            ops.contains("pub fn begin_place_vehicle"),
            "editor_ops must expose the vehicle arm"
        );
        assert!(
            ops.contains("core.add_vehicle("),
            "the vehicle place must reach the core mutator"
        );
    }

    /// A vehicle's cargo picker must never offer a person or another vehicle. `character` rows are
    /// crews (ORBAT slots, not freight) and nesting a vehicle inside a vehicle is not something the
    /// engine's storage does — either would author a document whose only failure mode is silence.
    #[test]
    fn vehicle_cargo_picker_excludes_people_and_vehicles() {
        for banned in ["character", "vehicle", "vehicle_weapon", "other"] {
            assert!(
                !VEHICLE_CARGO_KINDS.contains(&banned),
                "{banned} must not be offered as vehicle cargo"
            );
        }
        // …and it is a genuine superset of the worn-garment list, which is the whole reason it is a
        // separate constant rather than a reuse of `arsenal::CARGO_ADD_KINDS`.
        for expected in ["magazine", "ammo", "gear_primary", "gear_backpack"] {
            assert!(
                VEHICLE_CARGO_KINDS.contains(&expected),
                "{expected} is exactly what a resupply vehicle carries"
            );
        }
    }

    #[test]
    fn time_scrubber_roundtrip() {
        assert_eq!(minutes_to_hhmm(0), "00:00");
        assert_eq!(minutes_to_hhmm(360), "06:00");
        assert_eq!(minutes_to_hhmm(1439), "23:59");
        assert_eq!(hhmm_to_minutes("06:00"), Some(360));
        assert_eq!(hhmm_to_minutes("23:59"), Some(1439));
        assert_eq!(hhmm_to_minutes("24:00"), None);
        assert_eq!(hhmm_to_minutes("nope"), None);
        for m in [0u32, 1, 59, 60, 719, 1439] {
            assert_eq!(hhmm_to_minutes(&minutes_to_hhmm(m)), Some(m));
        }
    }

    /// T-192 — `missions.time_of_day` is a Postgres `time`, so the row hydrate puts `HH:MM:SS` in
    /// the document. The scrubber has to read it, or a reload silently shows 06:00 for a mission
    /// set to 21:45 — the same reverted-setting symptom the ticket removes.
    #[test]
    fn scrubber_reads_the_row_hydrate_clock() {
        assert_eq!(hhmm_to_minutes("21:45:00"), Some(1305));
        assert_eq!(hhmm_to_minutes("06:00:00"), Some(360));
        assert_eq!(hhmm_to_minutes("00:00:59"), Some(0));
        // Still a clock parser, not a "contains digits" parser.
        assert_eq!(hhmm_to_minutes("12:00:60"), None);
        assert_eq!(hhmm_to_minutes("12:00:00:00"), None);
        assert_eq!(hhmm_to_minutes("12"), None);
        assert_eq!(hhmm_to_minutes("12:0a"), None);
        assert_eq!(hhmm_to_minutes(""), None);
    }

    /// What [`super::RowMirror::set_time`] sends to `PATCH /missions/{id}`: a canonical `HH:MM`, or
    /// nothing at all. A half-typed clock must never reach the row.
    #[test]
    fn normalize_clock_is_the_patch_shape() {
        assert_eq!(normalize_clock("21:45").as_deref(), Some("21:45"));
        assert_eq!(normalize_clock("21:45:00").as_deref(), Some("21:45"));
        assert_eq!(normalize_clock("6:5").as_deref(), Some("06:05"));
        for bad in ["", "2", "24:00", "12:60", "noon", "12:00:00:00"] {
            assert_eq!(normalize_clock(bad), None, "{bad:?} must not be PATCHed");
        }
    }

    /// The mirror only fires on a real row. `mission_editor` falls back to `draft` and the editor
    /// gate mounts on a smoke id — a PATCH there is a guaranteed 400 and pure console noise.
    #[test]
    fn only_a_uuid_route_id_gets_mirrored() {
        assert!(is_mission_row_id("3f2504e0-4f89-11d3-9a0c-0305e82c3301"));
        assert!(is_mission_row_id("FFFFFFFF-FFFF-FFFF-FFFF-FFFFFFFFFFFF"));
        for bad in [
            "",
            "draft",
            "smoke",
            "3f2504e0-4f89-11d3-9a0c-0305e82c330",   // short
            "3f2504e0-4f89-11d3-9a0c-0305e82c33011", // long
            "3f2504e0x4f89-11d3-9a0c-0305e82c3301",  // dash in the wrong place
            "3f2504e0-4f89-11d3-9a0c-0305e82c330g",  // non-hex
        ] {
            assert!(!is_mission_row_id(bad), "{bad:?} is not a mission row id");
        }
    }

    /// The columns the mirror PATCHes, and the words the failure toast uses for them. Pinned
    /// because the column half is the API contract (`PatchMissionInput`) and the label half is what
    /// the author reads — `viewDistance` / `thermals` are absent because T-193 stopped the editor
    /// authoring them at all, not because the mirror declined to carry them.
    #[test]
    fn mirrored_fields_are_the_two_row_columns() {
        assert_eq!(MIRROR_TIME.column, "time_of_day");
        assert_eq!(MIRROR_WEATHER.column, "weather");
        assert_eq!(MIRROR_TIME.label, "Time of day");
        assert_eq!(MIRROR_WEATHER.label, "Weather");
        assert_ne!(MIRROR_TIME.column, MIRROR_WEATHER.column, "one queue each");
    }

    /// **T-193 — the two keys that must never come back.**
    ///
    /// `viewDistance` and `thermals` had working controls in Mission Settings for four waves. They
    /// wrote the document, took an undo step and read back correctly, and the value stopped dead at
    /// the editor boundary every single time: `ModEnvironment` is `dateTime` + `weatherPreset`, the
    /// `missions` row has no column for either, and neither word occurs anywhere in `apps/mod`. The
    /// controls were removed rather than wired through, because there is nothing on the far side to
    /// wire them to.
    ///
    /// `windDirDeg` is here as the other half of the lesson: the schema HAS a slot for it, and the
    /// editor still must not author it until something reads what it writes. A schema field is not
    /// a reader.
    #[test]
    fn keys_nothing_reads_are_not_authored() {
        for key in ["viewDistance", "thermals", "windDirDeg", "fog", "wind"] {
            assert!(
                !env_key_is_carried(key),
                "{key} has no reader — a control writing it would be dropped in silence"
            );
        }
    }

    /// The other direction: every key in the table is genuinely reachable, and every entry says who
    /// reads it. The "who" is the load-bearing half — it is the question nobody asked before adding
    /// a View Distance field, and an entry that cannot answer it does not belong in the table.
    #[test]
    fn every_carried_key_names_its_reader() {
        for (key, reader) in CARRIED_ENV_KEYS {
            assert!(
                env_key_is_carried(key),
                "{key} must resolve through the gate"
            );
            assert!(
                !reader.is_empty(),
                "{key} must name the surface that reads it"
            );
        }
        // The compiled pair and the editor-local trio — nothing else authors an environment key.
        assert_eq!(
            CARRIED_ENV_KEYS.len(),
            5,
            "adding a key means adding its reader first"
        );
        for key in [
            "time",
            "weather",
            "showHillshade",
            "hillshadeOpacity",
            "showGrid",
        ] {
            assert!(env_key_is_carried(key), "{key} is still authored");
        }
        // Two entries for the same key would make the table lie about ownership.
        let mut keys: Vec<&str> = CARRIED_ENV_KEYS.iter().map(|(k, _)| *k).collect();
        keys.sort_unstable();
        let n = keys.len();
        keys.dedup();
        assert_eq!(keys.len(), n, "one entry per key");
    }

    /* ───────────────────────── T-224 — the mission-flow block ───────────────────────── */

    /// **The four fields T-224 asks for that must NOT get a control.**
    ///
    /// `respawn`, `spectatorPolicy` and `nightVision` are declared in `mission.schema.json` and read
    /// by nothing: `TBD_MissionDocumentStruct` has no `settings` member at all, and `flatten` emits
    /// no `settings` block for it to miss. `tickets` is the same story one level down —
    /// `TBD_MissionFactionStruct` declares `key`/`displayName`/`presetId` and no `tickets`, so the
    /// hardcoded `0` the compiler emits is read by nobody either.
    ///
    /// The reason this is a test and not a comment is that all four would look like they worked.
    /// `JsonLoadContext` is a typed parser — a JSON key with no matching class member is not
    /// rejected and not logged, it is invisible — so a Respawn dropdown would author cleanly,
    /// validate cleanly, compile cleanly, survive a reload, and change nothing whatsoever about the
    /// round. That is exactly the View Distance failure T-193 removed two controls for, and the mod
    /// reader for `settings` is its own ticket (T-259).
    #[test]
    fn fields_with_no_mod_reader_get_no_control() {
        for key in ["respawn", "spectatorPolicy", "nightVision", "tickets"] {
            assert!(
                !env_key_is_carried(key),
                "{key} has no reader in the mod — a control writing it would change nothing"
            );
        }
    }

    /// The flow keys, their compiled destinations and the promise that each names a live reader.
    /// Pinned as a set because the key IS the contract: the compiler slice reads these names out of
    /// the saved payload's `environment`, so a rename here is a silent disconnection there.
    #[test]
    fn the_flow_block_is_the_four_schema_fields() {
        let keys: Vec<&str> = AUTHORED_FLOW_KEYS.iter().map(|(k, _, _)| *k).collect();
        assert_eq!(
            keys,
            [
                "briefingSeconds",
                "safeStartSeconds",
                "timeLimitSeconds",
                "jip"
            ],
            "mission.schema.json#/$defs/flow has exactly these four properties"
        );
        for (key, path, reader) in AUTHORED_FLOW_KEYS {
            assert!(
                env_key_is_carried(key),
                "{key} must resolve through the gate"
            );
            assert_eq!(
                *path,
                format!("flow.{key}"),
                "the bag key and the document path must stay one rename apart"
            );
            assert!(
                !reader.is_empty(),
                "{key} must name the mod symbol that reads it"
            );
        }
        // The T-193 table is untouched: these are a second block, not five more environment keys.
        assert_eq!(CARRIED_ENV_KEYS.len(), 5);
    }

    /// The unauthored-mission defaults are the constants `flatten.rs` splices in today. If these
    /// ever drift from `ModFlow`, the dialog shows an author a duration their mission does not run
    /// with — which is the reverted-setting bug wearing a different hat.
    #[test]
    fn flow_defaults_mirror_the_compiled_constants() {
        assert_eq!(FLOW_DEFAULT_BRIEFING_S, 600);
        assert_eq!(FLOW_DEFAULT_SAFESTART_S, 300);
        assert_eq!(FLOW_DEFAULT_TIMELIMIT_S, 5400);
        assert_eq!(FLOW_DEFAULT_JIP, "until_safestart_end");
        assert!(
            JIP_OPTIONS.iter().any(|(k, _)| *k == FLOW_DEFAULT_JIP),
            "the default must be a value the <select> can actually show"
        );
    }

    /// The `jip` enum, verbatim from the schema. `TBD_MissionFlow.PolicyFromString` falls through to
    /// `ALWAYS` on anything it does not recognise, so a value that drifts out of this list does not
    /// fail — it silently holds the mission's door open for the whole round.
    #[test]
    fn jip_options_are_the_schema_enum() {
        let values: Vec<&str> = JIP_OPTIONS.iter().map(|(v, _)| *v).collect();
        assert_eq!(values, ["disabled", "until_safestart_end", "always"]);
        for (_, label) in JIP_OPTIONS {
            assert!(
                !label.is_empty(),
                "every option needs words an author reads"
            );
        }
    }

    /// What a duration box is allowed to put in the document. `mission.schema.json` types every
    /// `flow` duration `integer, minimum 0`, and `flatten` splices these into a document a game
    /// server fetches — so a half-typed box must commit nothing rather than commit garbage.
    ///
    /// `0` is accepted on purpose: on `timeLimitSeconds` it is the only way to author "no time
    /// limit", and `TBD_MissionValidator` reads it as exactly that.
    #[test]
    fn only_whole_non_negative_seconds_are_authored() {
        assert_eq!(parse_flow_seconds("5400"), Some(5400));
        assert_eq!(
            parse_flow_seconds("0"),
            Some(0),
            "0 = no limit, a real value"
        );
        assert_eq!(parse_flow_seconds("  90  "), Some(90));
        for bad in ["", "  ", "-", "-1", "90.5", "1e3", "nine", "5400s", "+"] {
            assert_eq!(
                parse_flow_seconds(bad),
                None,
                "{bad:?} must not reach the document"
            );
        }
    }

    /// The box holds seconds because seconds is what the document, the schema and every mod reader
    /// use — a minutes box would have to round an authored 5430 on open and hand the author back a
    /// value they never set. This is the echo that makes the raw number readable instead.
    #[test]
    fn the_duration_echo_reads_as_a_duration() {
        assert_eq!(fmt_duration_secs(5400), "1 h 30 m");
        assert_eq!(fmt_duration_secs(600), "10 m");
        assert_eq!(fmt_duration_secs(300), "5 m");
        assert_eq!(fmt_duration_secs(90), "1 m 30 s");
        assert_eq!(fmt_duration_secs(3600), "1 h");
        assert_eq!(fmt_duration_secs(45), "45 s");
        assert_eq!(
            fmt_duration_secs(0),
            "0 s",
            "zero is a duration, not a blank"
        );
        assert_eq!(fmt_duration_secs(-1), "", "never authored, never rendered");
    }

    /// Four settings missing from a dialog whose ticket names all six reads as unfinished work
    /// unless the dialog says otherwise, and the thing an author needs to know is that the game
    /// does not read them — not that someone ran out of time.
    #[test]
    fn the_settings_note_names_all_four_refusals() {
        let note = SETTINGS_UNREAD_NOTE.to_lowercase();
        for word in ["respawn", "spectator", "night vision", "tickets"] {
            assert!(note.contains(word), "the note must name {word}: {note}");
        }
        assert!(
            note.contains("read"),
            "the note must say WHY they are absent: {note}"
        );
    }

    /// Two controls disappearing from a dialog is indistinguishable from a regression unless the
    /// dialog says why, so the replacement copy has to name both of them and say what a compiled
    /// mission does carry.
    #[test]
    fn the_note_names_what_it_replaced() {
        let note = ENV_UNCARRIED_NOTE.to_lowercase();
        for word in ["view distance", "thermals", "time", "weather"] {
            assert!(note.contains(word), "the note must name {word}: {note}");
        }
    }

    /// A failed mirror must SAY so — the shipped version only `warn!`ed, which is how a
    /// mission_maker editing someone else's live mission watched the setting apply and revert with
    /// no feedback at all. Every failure names the setting and says it will revert.
    #[test]
    fn every_mirror_failure_names_the_setting_and_the_revert() {
        for field in [MIRROR_TIME, MIRROR_WEATHER] {
            for err in [
                (403u16, Some("not your mission".to_string())),
                (400, Some("invalid weather".to_string())),
                (500, None),
                (0, None), // transport: no response at all
            ] {
                let msg = mirror_failure_message(field, &err);
                assert!(
                    msg.to_lowercase().contains(&field.label.to_lowercase()),
                    "{err:?} must name the setting: {msg}"
                );
                assert!(
                    msg.contains("revert"),
                    "{err:?} must warn of the revert: {msg}"
                );
            }
        }
    }

    /// 403 and "the server fell over" call for different action, so they must not read the same.
    /// The ownership refusal is structural — `PATCH /missions/:id` gates on `can_edit` (author or
    /// admin) while the editor route gates on role — so retrying cannot help and the text must not
    /// suggest it. Everything else is worth another go and carries what the server said.
    #[test]
    fn forbidden_and_transport_failures_read_differently() {
        let denied = mirror_failure_message(MIRROR_TIME, &(403, Some("not your mission".into())));
        let dropped = mirror_failure_message(MIRROR_TIME, &(0, None));
        assert_ne!(denied, dropped);
        assert!(
            denied.contains("author"),
            "403 must name the cause: {denied}"
        );
        assert!(
            !denied.contains("try again"),
            "403 is not retryable: {denied}"
        );
        assert!(
            dropped.contains("try again"),
            "a transport failure is: {dropped}"
        );
        assert!(
            dropped.contains("the server did not respond"),
            "a bodyless failure still says what happened: {dropped}"
        );
        // A backend message is surfaced verbatim (capitalized), not flattened to one house string.
        let bad = mirror_failure_message(MIRROR_WEATHER, &(400, Some("invalid weather".into())));
        assert!(bad.contains("Invalid weather"), "{bad}");
    }

    /// The rate bound. A held scrubber emits ~30 distinct values a second; the window has to be long
    /// enough to swallow that burst and short enough that a settled value lands while the author is
    /// still looking at the dialog.
    #[test]
    fn mirror_debounce_bounds_the_patch_rate() {
        assert!(
            MIRROR_DEBOUNCE_MS >= 200,
            "a 30 Hz scrub must collapse to one PATCH"
        );
        assert!(MIRROR_DEBOUNCE_MS <= 1000, "a settle must feel immediate");
    }

    /// A held scrubber: 30 distinct values inside one debounce window must reach the wire as ONE
    /// PATCH carrying the value the author stopped on. Every intermediate value is in the document
    /// already, so none of them is worth a round trip — and 30 of them racing is how the row ends up
    /// holding one the author scrubbed past.
    #[test]
    fn a_burst_collapses_to_the_settled_value() {
        let mut f = MirrorState::default();
        let mut rearms = 0;
        for m in 0..30u32 {
            if f.queue(minutes_to_hhmm(360 + m)) {
                rearms += 1; // each commit restarts the window; the timer fires once, at the end
            }
        }
        assert_eq!(rearms, 30, "every distinct value re-arms");
        let (generation, value) = f
            .take_for_send()
            .expect("the window closed with work queued");
        assert_eq!(
            value, "06:29",
            "the wire gets the settled value, not the first"
        );
        assert_eq!(f.take_for_send(), None, "and only that one");
        assert!(
            !f.settle(generation, value, true).queued,
            "nothing left over"
        );
        assert_eq!(f.last, "06:29");
    }

    /// The single-flight rule, which is what actually makes out-of-order landing unreachable: while
    /// one PATCH is on the wire, a newer value cannot start a second one. It waits, and the
    /// completion hands it the slot — so the row's last write is always the author's last edit.
    #[test]
    fn a_second_patch_cannot_start_while_one_is_in_flight() {
        let mut f = MirrorState::default();
        assert!(f.queue("06:00".into()));
        let (first, first_value) = f.take_for_send().expect("first goes out");

        assert!(f.queue("21:45".into()), "the author moves on mid-flight");
        assert_eq!(
            f.take_for_send(),
            None,
            "the window may close again, but the slot is busy"
        );

        // The first response comes back. It no longer speaks for the field, so it must not write
        // `last` — otherwise the losing value is what the next hydrate believes.
        let settled = f.settle(first, first_value, true);
        assert!(settled.stale, "the author has moved past 06:00");
        assert!(settled.queued, "21:45 is still waiting");
        assert_eq!(f.last, "", "a stale response must not record a value");

        let (second, second_value) = f.take_for_send().expect("the slot is free now");
        assert_eq!(second_value, "21:45");
        assert!(second > first, "generations are monotonic");
        assert!(!f.settle(second, second_value, true).stale);
        assert_eq!(f.last, "21:45", "the row ends on the author's last edit");
    }

    /// A stale FAILURE is just as silent as a stale success: it must not clear `last` (which
    /// describes a different generation) and must not toast (its successor will). The newest
    /// failure always speaks — that is the whole MAJOR.
    #[test]
    fn only_the_newest_generation_reports_a_failure() {
        let mut f = MirrorState::default();
        assert!(f.queue("clear".into()));
        let (first, first_value) = f.take_for_send().unwrap();
        assert!(!f.settle(first, first_value, true).stale);
        assert_eq!(f.last, "clear");

        assert!(f.queue("overcast".into()));
        let (second, second_value) = f.take_for_send().unwrap();
        assert!(f.queue("dense_fog".into()), "author moves on mid-flight");
        let stale_failure = f.settle(second, second_value, false);
        assert!(stale_failure.stale, "no toast for a value already replaced");
        assert_eq!(f.last, "clear", "a stale failure must not rewrite last");

        let (third, third_value) = f.take_for_send().unwrap();
        let live_failure = f.settle(third, third_value, false);
        assert!(!live_failure.stale, "THIS one must reach the user");
        assert!(!live_failure.queued);
        // Cleared so the very next commit of "dense_fog" retries rather than deduping away.
        assert_eq!(f.last, "");
        assert!(f.queue("dense_fog".into()), "the retry is not deduped away");
    }

    /// The T-192 dedupe, extended to cover the queue: a rebuild replaying a value, or a scrub that
    /// wanders back to what is already queued, must not cost a PATCH or bump the generation.
    #[test]
    fn an_unchanged_value_costs_nothing() {
        let mut f = MirrorState::default();
        assert!(f.queue("06:00".into()));
        assert!(!f.queue("06:00".into()), "same value, already queued");
        assert_eq!(f.generation, 1, "a no-op must not bump the generation");

        let (g, v) = f.take_for_send().unwrap();
        f.settle(g, v, true);
        assert!(!f.queue("06:00".into()), "same value, already on the row");
        assert_eq!(f.take_for_send(), None, "and nothing to send");
    }

    /// E1 + E5 — exact chip list; no CIV; no F-key labels in the chip row source of truth.
    #[test]
    fn eden_side_chips_labels_no_civ() {
        assert_eq!(EDEN_SIDE_CHIPS, &["BLUFOR", "OPFOR", "INDFOR", "Objects"]);
        assert_eq!(EDEN_SIDE_CHIPS.len(), 4);
        assert!(!EDEN_SIDE_CHIPS.iter().any(|c| *c == "CIV"));
        for label in EDEN_SIDE_CHIPS {
            assert!(
                !label.starts_with('F') || label == &"Objects",
                "F1–F6 mode row banned: {label}"
            );
            // F1…F6 are two-char labels like "F1" — none of our chips match.
            assert!(!matches!(*label, "F1" | "F2" | "F3" | "F4" | "F5" | "F6"));
        }
    }

    /// E2 — OPFOR chip writes the same side string `place_at` / OpsCtx read.
    #[test]
    fn apply_eden_chip_opfor_sets_active_side() {
        let active_side = RwSignal::new(String::from("BLUFOR"));
        let objects_mode = RwSignal::new(true);
        apply_eden_chip(EdenChip::Opfor, active_side, objects_mode);
        assert_eq!(active_side.get_untracked(), "OPFOR");
        assert!(!objects_mode.get_untracked());
        assert!(eden_chip_selected(
            EdenChip::Opfor,
            &active_side.get_untracked(),
            objects_mode.get_untracked()
        ));
    }

    /// E3 — Objects chip flips objects_mode without clobbering side; coming-soon stub is gone.
    #[test]
    fn objects_chip_enables_mode_without_clobbering_side() {
        // T-254 — stub constant name must not remain (split so this assert's own source cannot
        // false-fail the contains check).
        let src = include_str!("eden_chrome.rs");
        let stub_const = ["OBJECTS_", "COMING_", "SOON"].concat();
        assert!(
            !src.contains(&stub_const),
            "Objects stub constant must be removed"
        );
        assert!(
            src.contains("begin_place_object") || src.contains("PaletteKind::Object"),
            "Objects palette must arm object places"
        );
        let active_side = RwSignal::new(String::from("OPFOR"));
        let objects_mode = RwSignal::new(false);
        apply_eden_chip(EdenChip::Objects, active_side, objects_mode);
        assert!(objects_mode.get_untracked());
        assert_eq!(
            active_side.get_untracked(),
            "OPFOR",
            "Objects must leave last side intact"
        );
        assert!(eden_chip_selected(
            EdenChip::Objects,
            &active_side.get_untracked(),
            objects_mode.get_untracked()
        ));
        assert!(!eden_chip_selected(
            EdenChip::Opfor,
            &active_side.get_untracked(),
            objects_mode.get_untracked()
        ));
    }

    /// T-255 — chip write + `build_catalog_tree(_, side)` is the dock rebuild contract: OPFOR
    /// after a BLUFOR default must drop NATO leaves and keep only the USSR perturbation row.
    #[test]
    fn eden_chip_side_rebuilds_filtered_catalog() {
        use crate::asset_catalog::build_catalog_tree;
        use crate::dto::RegistryResponse;

        let golden: RegistryResponse =
            serde_json::from_str(include_str!("../tests/fixtures/api/GET__registry.json"))
                .expect("golden");
        let mut items = golden.data;
        items.push(
            serde_json::from_value(serde_json::json!({
                "id": "ussr",
                "modpack_id": "mp",
                "resource_name": "{DCB41B3746FDD1BE}Prefabs/Characters/Factions/OPFOR/USSR_Army/Character_USSR_Rifleman.et",
                "display_name": "USSR Rifleman",
                "category": "ArmaReforger/Characters/Factions/OPFOR/USSR_Army/Rifleman",
                "kind": "character",
                "sort_order": 99,
                "created_at": "2026-07-26T00:00:00Z",
                "updated_at": "2026-07-26T00:00:00Z",
            }))
            .expect("ussr row"),
        );

        let active_side = RwSignal::new(String::from("BLUFOR"));
        let objects_mode = RwSignal::new(false);
        let blufor_tree = build_catalog_tree(&items, &active_side.get_untracked());
        assert!(
            blufor_tree.iter().any(|n| n.id == "NATO"),
            "default BLUFOR chip keeps NATO"
        );

        apply_eden_chip(EdenChip::Opfor, active_side, objects_mode);
        let opfor_tree = build_catalog_tree(&items, &active_side.get_untracked());
        assert_eq!(active_side.get_untracked(), "OPFOR");
        assert_eq!(opfor_tree.len(), 1);
        assert!(
            !opfor_tree.iter().any(|n| n.id == "NATO"),
            "OPFOR chip must rebuild without NATO — got {:?}",
            opfor_tree.iter().map(|n| n.id.as_str()).collect::<Vec<_>>()
        );
        fn has_leaf(nodes: &[crate::asset_catalog::CatalogNode], label: &str) -> bool {
            nodes
                .iter()
                .any(|n| (n.payload.is_some() && n.label == label) || has_leaf(&n.children, label))
        }
        assert!(
            has_leaf(&opfor_tree, "USSR Rifleman"),
            "OPFOR rebuild must keep USSR Rifleman"
        );
        assert!(!has_leaf(&opfor_tree, "US Rifleman"));
    }
}
