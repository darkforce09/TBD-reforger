//! `meta.environment` authoring policy + the mission-flow block, split from
//! `eden_chrome.rs`.
//!
//! The gate ([`author_env`] → [`CARRIED_ENV_KEYS`] / [`AUTHORED_FLOW_KEYS`]) refuses any environment
//! key no surface reads back — the rule that stopped the View Distance / Thermals controls ()
//! and scopes the  flow block. Pure Rust + JSON; the doc-write helpers are wasm-only (they call
//! `editor_ops`, a wasm32-only module).
#![allow(dead_code)]

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
/// row has no `view_distance` / `thermals` column, so the  mirror cannot take them; the mod
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
    (
        "time",
        "compiled `environment.dateTime` + the `missions.time_of_day` column",
    ),
    (
        "weather",
        "compiled `environment.weatherPreset` + the `missions.weather` column",
    ),
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
/// The refusal is the whole point. A control wired straight at `editor_context::update_environment`
/// cannot tell whether its value will ever be read again, which is precisely how View Distance and
/// Thermals shipped looking functional. The check belongs on the one path every control takes.
#[cfg(target_arch = "wasm32")]
pub(crate) fn author_env(key: &str, value: serde_json::Value) {
    if !env_key_is_carried(key) {
        leptos::logging::error!(
            "refusing to author meta.environment.{key}: no surface reads it back (see CARRIED_ENV_KEYS)"
        );
        return;
    }
    let mut patch = serde_json::Map::new();
    patch.insert(key.to_string(), value);
    crate::v2::apps::editor::bridge::host_state::editor_context::update_environment(
        serde_json::Value::Object(patch).to_string(),
    );
}

/// What Mission Settings says where the View Distance field and the Thermals toggle used to be.
///
/// Pinned copy, because the blank is the problem: two controls vanishing from a dialog reads as a
/// regression unless the dialog says otherwise, and the one thing an author needs to know is that
/// the setting was never reaching the game — not that the UI got tidier.
pub const ENV_UNCARRIED_NOTE: &str =
    "View distance and thermals are not part of a compiled mission — it carries time and weather only.";

/// The four `flow` fields the editor authors: the key it writes into the document, the compiled
/// document path that key becomes, and the mod symbol that reads it there.
///
/// **Why these four and not the other four the ticket names.**  asks for six controls —
/// duration, respawn, spectator policy, NVG, tickets, JIP. Only two of those six reach a consumer
/// (duration = `flow.timeLimitSeconds`, and `jip`), so the block below is the two that do plus the
/// two remaining `flow` fields, which reach one for the same reason. The other four are refused, and
/// [`SETTINGS_UNREAD_NOTE`] is the dialog copy that says so. This is the  rule applied to a new
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
/// silence is its own ticket, ; this slice routes around it rather than depending on it.)
///
/// **The chain is closed end to end —  landed the last hop.** Every reader below is live in
/// the mod, the editor→mod chain is live for `meta.environment` up to the compiler (the saved
/// payload carries these keys out as top-level `environment`, and `mission_compile.rs` reads that
/// block for `time`/`weather`), and `ModFlow` no longer splices in four hardcoded constants:
/// `map_engine_core::mission::flatten::derive_flow` calls `authored_flow_seconds(env, key, default)`
/// per duration plus `authored_flow_jip(env)`, reading exactly the four key names in the first
/// column below, unprefixed, off the payload's top-level `environment`. So an authored 3600 is
/// stored, saved, reloaded, shown back AND compiled as 3600 — the old note here said it "still says
/// 5400", which was true when this file was written and stopped being true the day  shipped.
/// ( corrected it; the comment had outlived its ticket by several waves, and a stale comment
/// claiming a value is ignored is how a real drift gets waved through.)
///
/// The four constants remain as the fallback for a mission that authors nothing, and this module
/// re-exports the compiler's own ([`FLOW_DEFAULT_BRIEFING_S`] and friends) rather than mirroring
/// them — see that item for why the mirror was the bug.
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
///  (`settings`); tickets has no ticket because TBD events are one life by design and the
/// framework has no respawn pool for a ticket count to size.
pub const SETTINGS_UNREAD_NOTE: &str = "Respawn, spectator policy, night vision and per-faction tickets are not authored here — the mission document declares them and no mod script reads them. TBD events are one life.";

#[doc = " What a mission runs with when nothing is authored — **the compiler's own constants, not a copy**."]
#[doc = ""]
#[doc = " These are what `ModFlow` splices in when the payload authors nothing"]
#[doc = " (`map_engine_core::mission::flatten::derive_flow`), so an unauthored mission's dialog shows the"]
#[doc = " duration it will actually run with rather than a UI-invented zero. They are also the fallback the"]
#[doc = " compiler keeps now that it reads the authored keys, which is why the dialog and the compiled"]
#[doc = " document have to agree about them: if they disagree, the dialog is lying about an unauthored"]
#[doc = " mission."]
#[doc = ""]
#[doc = " **T-753 — why this is a `pub use` and not four `pub const`s.** It used to be four `pub const`s"]
#[doc = " here holding literals identical to `flatten.rs`'s, with nothing anywhere comparing the two sets."]
#[doc = " The only guard was `flow_defaults_mirror_the_compiled_constants` below, which restated"]
#[doc = " the literals against THIS module's own copy — so it agreed with itself no matter what the"]
#[doc = " compiler held. The wave-115 verifier proved the hole rather than arguing it: editing"]
#[doc = " `FLOW_DEFAULT_BRIEFING_S` in `flatten.rs` from 600 to 900 left `cargo test -p website-frontend`"]
#[doc = " at 800 passed / 0 failed while the compiler emitted 900-second briefings and every editor surface"]
#[doc = " kept displaying 600. That is exactly the defect class T-688 exists to prevent — a view showing a"]
#[doc = " value the authority does not hold — one layer beneath the surface T-688 audited."]
#[doc = ""]
#[doc = " A cross-crate `assert_eq!` would have closed it. Re-exporting closes it harder: there is now ONE"]
#[doc = " definition in the workspace, so \"the two disagree\" is not a bug that can be written. The literal"]
#[doc = " guard below is kept and is no longer circular — it now reads the compiler's constant, so the"]
#[doc = " verifier's 600 → 900 edit turns it red. This crate already depended on `map-engine-core` with the"]
#[doc = " `mission` feature (`Cargo.toml`), so this costs nothing but the deletion."]
pub use website_map_engine::data::scenario::flatten::FLOW_DEFAULT_BRIEFING_S;
#[doc = " What a mission runs with when nothing is authored — **the compiler's own constants, not a copy**."]
#[doc = ""]
#[doc = " These are what `ModFlow` splices in when the payload authors nothing"]
#[doc = " (`map_engine_core::mission::flatten::derive_flow`), so an unauthored mission's dialog shows the"]
#[doc = " duration it will actually run with rather than a UI-invented zero. They are also the fallback the"]
#[doc = " compiler keeps now that it reads the authored keys, which is why the dialog and the compiled"]
#[doc = " document have to agree about them: if they disagree, the dialog is lying about an unauthored"]
#[doc = " mission."]
#[doc = ""]
#[doc = " **T-753 — why this is a `pub use` and not four `pub const`s.** It used to be four `pub const`s"]
#[doc = " here holding literals identical to `flatten.rs`'s, with nothing anywhere comparing the two sets."]
#[doc = " The only guard was `flow_defaults_mirror_the_compiled_constants` below, which restated"]
#[doc = " the literals against THIS module's own copy — so it agreed with itself no matter what the"]
#[doc = " compiler held. The wave-115 verifier proved the hole rather than arguing it: editing"]
#[doc = " `FLOW_DEFAULT_BRIEFING_S` in `flatten.rs` from 600 to 900 left `cargo test -p website-frontend`"]
#[doc = " at 800 passed / 0 failed while the compiler emitted 900-second briefings and every editor surface"]
#[doc = " kept displaying 600. That is exactly the defect class T-688 exists to prevent — a view showing a"]
#[doc = " value the authority does not hold — one layer beneath the surface T-688 audited."]
#[doc = ""]
#[doc = " A cross-crate `assert_eq!` would have closed it. Re-exporting closes it harder: there is now ONE"]
#[doc = " definition in the workspace, so \"the two disagree\" is not a bug that can be written. The literal"]
#[doc = " guard below is kept and is no longer circular — it now reads the compiler's constant, so the"]
#[doc = " verifier's 600 → 900 edit turns it red. This crate already depended on `map-engine-core` with the"]
#[doc = " `mission` feature (`Cargo.toml`), so this costs nothing but the deletion."]
pub use website_map_engine::data::scenario::flatten::FLOW_DEFAULT_JIP;
#[doc = " What a mission runs with when nothing is authored — **the compiler's own constants, not a copy**."]
#[doc = ""]
#[doc = " These are what `ModFlow` splices in when the payload authors nothing"]
#[doc = " (`map_engine_core::mission::flatten::derive_flow`), so an unauthored mission's dialog shows the"]
#[doc = " duration it will actually run with rather than a UI-invented zero. They are also the fallback the"]
#[doc = " compiler keeps now that it reads the authored keys, which is why the dialog and the compiled"]
#[doc = " document have to agree about them: if they disagree, the dialog is lying about an unauthored"]
#[doc = " mission."]
#[doc = ""]
#[doc = " **T-753 — why this is a `pub use` and not four `pub const`s.** It used to be four `pub const`s"]
#[doc = " here holding literals identical to `flatten.rs`'s, with nothing anywhere comparing the two sets."]
#[doc = " The only guard was `flow_defaults_mirror_the_compiled_constants` below, which restated"]
#[doc = " the literals against THIS module's own copy — so it agreed with itself no matter what the"]
#[doc = " compiler held. The wave-115 verifier proved the hole rather than arguing it: editing"]
#[doc = " `FLOW_DEFAULT_BRIEFING_S` in `flatten.rs` from 600 to 900 left `cargo test -p website-frontend`"]
#[doc = " at 800 passed / 0 failed while the compiler emitted 900-second briefings and every editor surface"]
#[doc = " kept displaying 600. That is exactly the defect class T-688 exists to prevent — a view showing a"]
#[doc = " value the authority does not hold — one layer beneath the surface T-688 audited."]
#[doc = ""]
#[doc = " A cross-crate `assert_eq!` would have closed it. Re-exporting closes it harder: there is now ONE"]
#[doc = " definition in the workspace, so \"the two disagree\" is not a bug that can be written. The literal"]
#[doc = " guard below is kept and is no longer circular — it now reads the compiler's constant, so the"]
#[doc = " verifier's 600 → 900 edit turns it red. This crate already depended on `map-engine-core` with the"]
#[doc = " `mission` feature (`Cargo.toml`), so this costs nothing but the deletion."]
pub use website_map_engine::data::scenario::flatten::FLOW_DEFAULT_SAFESTART_S;
#[doc = " What a mission runs with when nothing is authored — **the compiler's own constants, not a copy**."]
#[doc = ""]
#[doc = " These are what `ModFlow` splices in when the payload authors nothing"]
#[doc = " (`map_engine_core::mission::flatten::derive_flow`), so an unauthored mission's dialog shows the"]
#[doc = " duration it will actually run with rather than a UI-invented zero. They are also the fallback the"]
#[doc = " compiler keeps now that it reads the authored keys, which is why the dialog and the compiled"]
#[doc = " document have to agree about them: if they disagree, the dialog is lying about an unauthored"]
#[doc = " mission."]
#[doc = ""]
#[doc = " **T-753 — why this is a `pub use` and not four `pub const`s.** It used to be four `pub const`s"]
#[doc = " here holding literals identical to `flatten.rs`'s, with nothing anywhere comparing the two sets."]
#[doc = " The only guard was `flow_defaults_mirror_the_compiled_constants` below, which restated"]
#[doc = " the literals against THIS module's own copy — so it agreed with itself no matter what the"]
#[doc = " compiler held. The wave-115 verifier proved the hole rather than arguing it: editing"]
#[doc = " `FLOW_DEFAULT_BRIEFING_S` in `flatten.rs` from 600 to 900 left `cargo test -p website-frontend`"]
#[doc = " at 800 passed / 0 failed while the compiler emitted 900-second briefings and every editor surface"]
#[doc = " kept displaying 600. That is exactly the defect class T-688 exists to prevent — a view showing a"]
#[doc = " value the authority does not hold — one layer beneath the surface T-688 audited."]
#[doc = ""]
#[doc = " A cross-crate `assert_eq!` would have closed it. Re-exporting closes it harder: there is now ONE"]
#[doc = " definition in the workspace, so \"the two disagree\" is not a bug that can be written. The literal"]
#[doc = " guard below is kept and is no longer circular — it now reads the compiler's constant, so the"]
#[doc = " verifier's 600 → 900 edit turns it red. This crate already depended on `map-engine-core` with the"]
#[doc = " `mission` feature (`Cargo.toml`), so this costs nothing but the deletion."]
pub use website_map_engine::data::scenario::flatten::FLOW_DEFAULT_TIMELIMIT_S;

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
/// silently rewriting a value the author never touched, which is the exact class of bug  was
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
pub(crate) fn read_flow_seconds(key: &str, default: i64) -> i64 {
    crate::v2::apps::editor::bridge::host_state::editor_context::read_env_value(key)
        .as_ref()
        .and_then(serde_json::Value::as_i64)
        .filter(|n| *n >= 0)
        .unwrap_or(default)
}

/// The authored `jip` policy, or [`FLOW_DEFAULT_JIP`]. A value outside [`JIP_OPTIONS`] falls back to
/// the default rather than being shown: an unrecognised string in a `<select>` renders as no
/// selection at all, which reads as "unset" for a field that is very much set.
#[cfg(target_arch = "wasm32")]
pub(crate) fn read_flow_jip() -> String {
    crate::v2::apps::editor::bridge::host_state::editor_context::read_env_value("jip")
        .as_ref()
        .and_then(serde_json::Value::as_str)
        .filter(|v| JIP_OPTIONS.iter().any(|(k, _)| k == v))
        .unwrap_or(FLOW_DEFAULT_JIP)
        .to_string()
}

#[cfg(test)]
#[path = "tests/env/environment_flow_contract.rs"]
mod tests;
