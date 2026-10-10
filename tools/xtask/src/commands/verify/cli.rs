use clap::{Args, Subcommand};

/// The arguments the link check takes: which folders it judges, and whether the untracked files
/// git does not ignore count.
#[derive(Args, Debug)]
pub(crate) struct DocumentationGateArgs {
    /// Judge only what lies at or under this repository-relative folder (repeatable)
    #[arg(long = "path", value_name = "DIR")]
    pub(crate) paths: Vec<String>,
    /// Also judge the untracked files git does not ignore, exactly like tracked ones: a check of
    /// new files before they are committed
    #[arg(long = "with-untracked")]
    pub(crate) with_untracked: bool,
}

#[derive(Subcommand, Debug)]
pub(crate) enum VerifyCmd {
    /// File-length advice: warns about every production file over 500 lines; never fails on one
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
    /// Tracked shell/Make hard zero (same TrackedLanguageBan table as no-python)
    #[command(name = "no-shell")]
    NoShell,
    /// No bare SELECT */RETURNING * on nullable-column tables
    #[command(name = "no-select-star")]
    NoSelectStar,
    /// Objects palette aliases pinned in the mod Data/registry.json
    #[command(name = "object-registry-aliases")]
    ObjectRegistryAliases,
    /// Oracle-leak guard: no CRF_/PS_ identifiers or oracle-only asset GUIDs
    #[command(name = "no-crf-leak")]
    NoCrfLeak,
    /// Enfusion .layout structural gate (brace balance, slot classes, geometry)
    #[command(name = "ui-layouts")]
    UiLayouts,
    /// Zero tracked .py and zero python3 in command position (alias of the language ban)
    #[command(name = "no-python")]
    NoPython,
    /// Crate-tier law: no member depends on an application package, in any table, and every
    /// application package is a member; the external-crate firewalls hold (the wasm_bindgen
    /// attribute only in the two shell crates and browser_platform; no rendering-stack crate or
    /// wgpu in the offline service worker or an API crate, in any table; no tokio, axum,
    /// reqwest, resvg or image in the xtask closure)
    #[command(name = "crate-tiers")]
    CrateTiers,
    /// Crate-anatomy law: every judged library crate keeps a lib.rs of at most 80 lines of
    /// module lines, a prelude, a thiserror error.rs when fallible, workspace-inherited
    /// edition, rust-version, lints and dependencies, only dev-only features, typed ids, and no
    /// re-export of another workspace crate outside its prelude
    #[command(name = "crate-anatomy")]
    CrateAnatomy,
    /// Test-file reachability law: every .rs file in a test folder of a workspace member (under
    /// src/ or the member's tests/ folder) is loaded by a mod declaration (with its path
    /// attribute) from one of the member's targets, or is itself an integration target
    #[command(name = "test-file-reachability")]
    TestFileReachability,
    /// Frontend-layering law: a lower frontend layer never imports a higher one, pages and
    /// workspaces never import each other, one page area never imports another, a crate imports
    /// only the crates before it in its folder's declared order, and the two shell crates (the
    /// app and the offline service worker) never name each other; any edge fails
    #[command(name = "frontend-layering")]
    FrontendLayering,
    /// Tailwind-sources law: every member that depends on leptos has an @source line in the app
    /// stylesheet covering its src/**/*.rs
    #[command(name = "tailwind-sources")]
    TailwindSources,
    /// Every link in the documentation root, the READMEs, the project instructions and the
    /// Cursor rules reaches a tracked file or folder, a heading or
    /// line anchor, a defined reference, or a sha permalink of this repository; every repository
    /// path a live document writes in backticks names a tracked or ignored file or folder; and
    /// every `cargo xtask` command a live document cites exists, with the value of its first
    /// argument when that argument has a closed set of values
    #[command(name = "link-check")]
    LinkCheck {
        /// Print every break as `path:line: rule: message` instead of the first ones
        #[arg(long)]
        report: bool,
        #[command(flatten)]
        arguments: DocumentationGateArgs,
    },
}
