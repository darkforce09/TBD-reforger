//! T-661 — `meta.environment` authoring policy + the mission-flow block, split from
//! `eden_chrome.rs`.
//!
//! The gate ([`author_env`] → [`CARRIED_ENV_KEYS`] / [`AUTHORED_FLOW_KEYS`]) refuses any environment
//! key no surface reads back — the rule that stopped the View Distance / Thermals controls (T-193)
//! and scopes the T-224 flow block. Pure Rust + JSON; the doc-write helpers are wasm-only (they call
//! `editor_ops`, a wasm32-only module).
#![allow(dead_code)]

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
pub(crate) fn author_env(key: &str, value: serde_json::Value) {
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
pub(crate) fn read_flow_seconds(key: &str, default: i64) -> i64 {
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
pub(crate) fn read_flow_jip() -> String {
    crate::editor_ops::read_env_value("jip")
        .as_ref()
        .and_then(serde_json::Value::as_str)
        .filter(|v| JIP_OPTIONS.iter().any(|(k, _)| k == v))
        .unwrap_or(FLOW_DEFAULT_JIP)
        .to_string()
}

#[cfg(test)]
mod tests {
    use super::{
        env_key_is_carried, fmt_duration_secs, parse_flow_seconds, AUTHORED_FLOW_KEYS,
        CARRIED_ENV_KEYS, ENV_UNCARRIED_NOTE, FLOW_DEFAULT_BRIEFING_S, FLOW_DEFAULT_JIP,
        FLOW_DEFAULT_SAFESTART_S, FLOW_DEFAULT_TIMELIMIT_S, JIP_OPTIONS, SETTINGS_UNREAD_NOTE,
    };

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
}
