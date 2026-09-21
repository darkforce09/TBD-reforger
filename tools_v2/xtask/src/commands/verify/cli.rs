use clap::Subcommand;

#[derive(Subcommand, Debug)]
pub(crate) enum VerifyCmd {
    /// SIZE-1/3 file-length gate (verify-file-length.mjs port)
    #[command(name = "file-length")]
    FileLength,
    /// T-090.12.2: the prefab BLAS library is complete and consistent — every catalogue pid has
    /// a schema-valid descriptor, every listed BLAS parses with the manifest's bytes / tris /
    /// kinds, the hot set is blocking pids by placement count, the farmhouse root BLAS is the
    /// T-090.11 shell.
    #[command(name = "blas-manifest")]
    BlasManifest,
    /// Zero tracked .mjs/.cjs; no node/npx invocation in a scanned file
    #[command(name = "no-node")]
    NoNode,
    /// T-904 LANG-1: tracked shell/Make hard zero (same TrackedLanguageBan table as no-python)
    #[command(name = "no-shell")]
    NoShell,
    /// T-901: every GitHub Actions `run:` is `cargo xtask` or a short pre-cargo allowlist
    #[command(name = "ci-shell")]
    CiShell,
    /// T-145 guard: no bare SELECT */RETURNING * on nullable-column tables
    /// (T-853 port of scripts/website/verify-no-select-star.sh)
    #[command(name = "no-select-star")]
    NoSelectStar,
    /// T-452 comment contract: TBD_PlayerIdentity must not claim `#tbd link` is unimplemented
    /// (T-853 port of scripts/mod/verify-t452-player-identity-link-comments.sh)
    #[command(name = "t452")]
    T452,
    /// T-296 comment contract: ResultsReporter identity (port of verify-t296-*.sh)
    #[command(name = "t296")]
    T296,
    /// T-439: Objects palette aliases pinned in the mod Data/registry.json
    #[command(name = "t439")]
    T439,
    /// T-444: `cargo xtask db seed` must apply seeds/wiki_pages.sql
    #[command(name = "t444")]
    T444,
    /// T-181.4/.52 oracle-leak guard: no CRF_/PS_ identifiers or oracle-only asset GUIDs
    #[command(name = "no-crf-leak")]
    NoCrfLeak,
    /// T-180.10 Class-R coherency for ORBAT + Eden locks
    #[command(name = "t180")]
    T180,
    /// GO-7: every @route tag resolves to a registered Axum route and back
    #[command(name = "route-tags")]
    RouteTags,
    /// T-181.51 Enfusion .layout structural gate (brace balance, slot classes, geometry)
    #[command(name = "ui-layouts")]
    UiLayouts,
    /// T-437: destroy-inert diagnostics must not claim entities[] never spawn
    #[command(name = "t437")]
    T437,
    /// T-438: deploy-staging must resolve the compose file by an absolute path
    #[command(name = "t438")]
    T438,
    /// T-440: faction library seed reaches the DB
    #[command(name = "t440")]
    T440,
    /// T-904 LANG-2: zero tracked .py + zero python3 in command position (alias of the language ban)
    #[command(name = "no-python")]
    NoPython,
    /// T-456/T-460: mission REST body size gate before ParseMissionJson
    #[command(name = "t456")]
    T456,
    /// T-468: CI schema parity + hollow recipe tripwire
    #[command(name = "t468")]
    T468,
    /// ENGINE_SPLIT_PROGRAM §5 rules 1, 2, 3a, 3b, 4 and 7: apps/website/graphics-engine may not
    /// import website_map_engine, and may not declare a type/fn/mod name containing terrain,
    /// symbology, mission, orbat or arma; and under apps/website/map-engine only the enumerated
    /// packet boundary may name website_graphics_engine::frame, nothing at all may name its
    /// device / pipeline / shaders / text::gpu / r#loop, data/scenario imports nothing outside
    /// itself, and data/ and world/ name each other nowhere. (§5 spells it `verify-engine-layers`;
    /// every sibling here is `verify <name>`, and the `verify-engine-layers` task row aliases both.)
    #[command(name = "engine-layers")]
    EngineLayers,
}
