//! Tests the env subject.

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

    /// The unauthored-mission defaults are the constants `flatten.rs` splices in. If these drift
    /// from `ModFlow`, the dialog shows an author a duration their mission does not run with — the
    /// reverted-setting bug wearing a different hat.
    ///
    /// **T-753 — this test used to be unable to fail for that reason.** The names below resolved to
    /// this module's own `pub const`s, so it compared `flatten.rs`'s literals to a copy of
    /// `flatten.rs`'s literals and passed whatever the compiler actually held. It is now a real
    /// cross-crate pin by construction: the names are `pub use`d straight out of
    /// `map_engine_core::mission::flatten`, so these five assertions read the compiling crate's
    /// constants across the crate boundary. Editing `FLOW_DEFAULT_BRIEFING_S` in `flatten.rs` from
    /// 600 to 900 — the wave-115 verifier's experiment, which used to leave this suite fully green —
    /// now fails here with `left: 900, right: 600`.
    ///
    /// The literals stay spelled out on purpose. They are the whole point: a pin whose expectation
    /// is itself a variable pins nothing.
    #[test]
    fn flow_defaults_mirror_the_compiled_constants() {
        assert_eq!(
            FLOW_DEFAULT_BRIEFING_S,
            600,
            "map_engine_core::mission::flatten::FLOW_DEFAULT_BRIEFING_S moved; Mission Settings and \
             the compiled document must be changed together"
        );
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
