//! Finding the Chromium build every browser gate launches.
//!
//! **Role:** resolves the browser executable: `CHROME_HEADLESS_SHELL` when it names an existing
//! file, else the newest build under `PLAYWRIGHT_BROWSERS_PATH`, else the newest build under
//! `$HOME/.cache/ms-playwright`; and tells a minimal headless shell from a full Chrome build.
//!
//! **Position:** called by `launch_with_gpu` in `browser_launch.rs` and by `gate doctor`'s
//! font checks in `browser_gate_suites::diagnostics`; re-exported at the crate root.
//!
//! **Signals & state:** none; reads three environment variables and the folder listings.
//!
//! **Invariants:** the search roots are tried in the order above and the first root holding any
//! known layout wins. Inside a root a full `chrome` build always beats a headless shell, and among
//! builds of one kind the highest build number (the digits after `chromium-` or
//! `chromium_headless_shell-`, compared as numbers) wins. A full build is preferred because the
//! minimal headless shell ships a stubbed `SkFontMgr_FontConfigInterface` whose per-character font
//! fallback is a `FATAL: … "Not implemented"`: the first page needing a fallback glyph kills the
//! renderer, and the harness sees only a `Runtime.evaluate` timeout.

use std::path::{Path, PathBuf};

/// The executable layouts Playwright installs, per build-folder prefix, full Chrome first.
///
/// A full build is `chrome-linux64/chrome` in current Playwright releases and `chrome-linux/chrome`
/// in older ones and in the browser bundles some containers ship; a headless shell is
/// `chrome-headless-shell-linux64/chrome-headless-shell` or `chrome-linux/headless_shell`.
const PLAYWRIGHT_BUILD_LAYOUTS: &[(&str, &[&str])] = &[
    (
        "chromium-",
        &["chrome-linux64/chrome", "chrome-linux/chrome"],
    ),
    (
        "chromium_headless_shell-",
        &[
            "chrome-headless-shell-linux64/chrome-headless-shell",
            "chrome-linux/headless_shell",
        ],
    ),
];

/// The places a Chromium build can come from, in the order [`resolve_chromium`] tries them.
#[derive(Debug, Default, Clone)]
pub(super) struct ChromiumSearchRoots {
    /// `CHROME_HEADLESS_SHELL`: one executable, taken when it exists.
    pub explicit_executable: Option<PathBuf>,
    /// `PLAYWRIGHT_BROWSERS_PATH`: a Playwright browser folder holding `chromium-<build>/` folders.
    pub playwright_browsers_path: Option<PathBuf>,
    /// `$HOME/.cache/ms-playwright`: Playwright's default browser folder.
    pub playwright_cache: Option<PathBuf>,
}

impl ChromiumSearchRoots {
    /// The roots this process's environment names; an empty variable counts as unset.
    pub(super) fn from_environment() -> Self {
        let non_empty = |key: &str| {
            std::env::var_os(key)
                .filter(|v| !v.is_empty())
                .map(PathBuf::from)
        };
        Self {
            explicit_executable: non_empty("CHROME_HEADLESS_SHELL"),
            playwright_browsers_path: non_empty("PLAYWRIGHT_BROWSERS_PATH"),
            playwright_cache: non_empty("HOME").map(|home| home.join(".cache/ms-playwright")),
        }
    }
}

/// The Chromium executable the gates launch, from this process's environment; `None` when no
/// search root holds one.
pub fn find_chromium() -> Option<PathBuf> {
    resolve_chromium(&ChromiumSearchRoots::from_environment())
}

/// The Chromium executable `roots` yield: the explicit executable when it exists, else the best
/// build of the first Playwright folder that holds one.
pub(super) fn resolve_chromium(roots: &ChromiumSearchRoots) -> Option<PathBuf> {
    if let Some(explicit) = &roots.explicit_executable
        && explicit.exists()
    {
        return Some(explicit.clone());
    }
    [&roots.playwright_browsers_path, &roots.playwright_cache]
        .into_iter()
        .flatten()
        .find_map(|folder| best_playwright_build(folder))
}

/// The best executable in one Playwright browser folder: full Chrome before a headless shell,
/// highest build number first, first matching layout of that build.
pub(super) fn best_playwright_build(folder: &Path) -> Option<PathBuf> {
    let entries: Vec<String> = std::fs::read_dir(folder)
        .ok()?
        .filter_map(Result::ok)
        .filter_map(|e| e.file_name().into_string().ok())
        .collect();
    for (prefix, layouts) in PLAYWRIGHT_BUILD_LAYOUTS {
        let mut builds: Vec<(u64, &String)> = entries
            .iter()
            .filter_map(|name| {
                let build = name.strip_prefix(prefix)?;
                Some((build.parse().unwrap_or(0), name))
            })
            .collect();
        // Highest build number first; the name breaks a tie so the order never depends on the
        // order the folder listing happened to return.
        builds.sort_by(|a, b| b.cmp(a));
        for (_, name) in builds {
            for layout in *layouts {
                let executable = folder.join(name).join(layout);
                if executable.is_file() {
                    return Some(executable);
                }
            }
        }
    }
    None
}

/// True when the resolved Chromium is the minimal headless shell, which is always headless and
/// ignores `--headless`; the full `chrome` build needs an explicit `--headless=new` (`launch`).
pub fn is_headless_shell(bin: &Path) -> bool {
    bin.to_string_lossy().contains("chrome-headless-shell")
        || bin.file_name().is_some_and(|name| name == "headless_shell")
}
