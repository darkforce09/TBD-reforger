use clap::Subcommand;

#[derive(Subcommand, Debug)]
pub(crate) enum VerifyCmd {
    /// Verify every API requirement against current Rust tests and staging evidence
    #[command(name = "api-readiness")]
    ApiReadiness {
        /// Evidence directory containing receipts and their complete check output
        #[arg(long, default_value = "target/api-readiness")]
        evidence: std::path::PathBuf,
        /// Execute registered local checks before evaluating all required evidence
        #[arg(long)]
        execute: bool,
    },
    /// SIZE-1/3 file-length gate: production files stay under 500 lines, test files under 1000
    #[command(name = "file-length")]
    FileLength,
    /// The prefab BLAS library is complete and consistent — every catalogue pid has a
    /// schema-valid descriptor, every listed BLAS parses with the manifest's bytes / tris /
    /// kinds, the hot set is blocking pids by placement count, and the farmhouse root BLAS is
    /// the shell the catalogue records.
    #[command(name = "blas-manifest")]
    BlasManifest,
    /// Zero tracked Node sources; no node/npx invocation in a scanned file
    #[command(name = "no-node")]
    NoNode,
    /// LANG-1: tracked shell/Make hard zero (same TrackedLanguageBan table as no-python)
    #[command(name = "no-shell")]
    NoShell,
    /// Every GitHub Actions `run:` is `cargo xtask` or a short pre-cargo allowlist
    #[command(name = "ci-shell")]
    CiShell,
    /// No bare SELECT */RETURNING * on nullable-column tables
    #[command(name = "no-select-star")]
    NoSelectStar,
    /// Comment contract: TBD_PlayerIdentity must not claim `#tbd link` is unimplemented
    #[command(name = "player-identity-comments")]
    PlayerIdentityComments,
    /// Comment contract: ResultsReporter must describe the identity it actually reports
    #[command(name = "results-reporter-identity-comments")]
    ResultsReporterIdentityComments,
    /// Objects palette aliases pinned in the mod Data/registry.json
    #[command(name = "object-registry-aliases")]
    ObjectRegistryAliases,
    /// `cargo xtask db seed` must apply seeds/wiki_pages.sql
    #[command(name = "wiki-seeds")]
    WikiSeeds,
    /// Oracle-leak guard: no CRF_/PS_ identifiers or oracle-only asset GUIDs
    #[command(name = "no-crf-leak")]
    NoCrfLeak,
    /// Class-R coherency for ORBAT + Eden locks
    #[command(name = "editor-orbat-coherency")]
    EditorOrbatCoherency,
    /// GO-7: every @route tag resolves to a registered Axum route and back
    #[command(name = "route-tags")]
    RouteTags,
    /// Enfusion .layout structural gate (brace balance, slot classes, geometry)
    #[command(name = "ui-layouts")]
    UiLayouts,
    /// Destroy-inert diagnostics must not claim entities[] never spawn
    #[command(name = "destroy-target-diagnostics")]
    DestroyTargetDiagnostics,
    /// `deploy staging` must resolve the compose file by an absolute path
    #[command(name = "staging-compose-paths")]
    StagingComposePaths,
    /// Faction library seed reaches the DB
    #[command(name = "faction-library-seeds")]
    FactionLibrarySeeds,
    /// LANG-2: zero tracked .py + zero python3 in command position (alias of the language ban)
    #[command(name = "no-python")]
    NoPython,
    /// Mission REST body size gate before ParseMissionJson
    #[command(name = "mission-rest-size-limits")]
    MissionRestSizeLimits,
    /// CI schema parity + hollow recipe tripwire
    #[command(name = "ci-schema-parity")]
    CiSchemaParity,
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
