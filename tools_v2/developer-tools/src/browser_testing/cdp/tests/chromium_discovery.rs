//! Unit tests for Chromium discovery over scratch Playwright folder trees.

use super::*;

/// A scratch folder unique to one test and this process, emptied first.
fn scratch(name: &str) -> PathBuf {
    let dir =
        std::env::temp_dir().join(format!("chromium-discovery-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

/// Create an executable stand-in at `root/relative` and return its path.
fn plant(root: &Path, relative: &str) -> PathBuf {
    let path = root.join(relative);
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    std::fs::write(&path, "#!/bin/sh\n").unwrap();
    path
}

fn roots(
    explicit: Option<&Path>,
    browsers_path: Option<&Path>,
    cache: Option<&Path>,
) -> ChromiumSearchRoots {
    ChromiumSearchRoots {
        explicit_executable: explicit.map(Path::to_path_buf),
        playwright_browsers_path: browsers_path.map(Path::to_path_buf),
        playwright_cache: cache.map(Path::to_path_buf),
    }
}

/// The `chrome-linux/chrome` layout (the container's `/opt/pw-browsers/chromium-1194`) is found
/// under `PLAYWRIGHT_BROWSERS_PATH`, and that folder is searched before the Playwright cache.
#[test]
fn chromium_under_playwright_browsers_path_is_found_before_the_cache() {
    let base = scratch("browsers-path");
    let browsers = base.join("pw-browsers");
    let cache = base.join("ms-playwright");
    let wanted = plant(&browsers, "chromium-1194/chrome-linux/chrome");
    plant(&cache, "chromium-9999/chrome-linux64/chrome");
    assert_eq!(
        resolve_chromium(&roots(None, Some(&browsers), Some(&cache))),
        Some(wanted)
    );
    let _ = std::fs::remove_dir_all(&base);
}

/// `CHROME_HEADLESS_SHELL` wins when it names an existing file, and is skipped when it does not.
#[test]
fn chromium_explicit_executable_wins_only_when_it_exists() {
    let base = scratch("explicit");
    let browsers = base.join("pw-browsers");
    let explicit = plant(&base, "custom/chrome");
    let from_folder = plant(&browsers, "chromium-1194/chrome-linux/chrome");
    assert_eq!(
        resolve_chromium(&roots(Some(&explicit), Some(&browsers), None)),
        Some(explicit)
    );
    let missing = base.join("missing/chrome");
    assert_eq!(
        resolve_chromium(&roots(Some(&missing), Some(&browsers), None)),
        Some(from_folder)
    );
    let _ = std::fs::remove_dir_all(&base);
}

/// The Playwright cache is the last root, and it accepts the `chrome-linux/` layout too.
#[test]
fn chromium_cache_is_searched_last_and_accepts_both_full_layouts() {
    let base = scratch("cache");
    let cache = base.join("ms-playwright");
    let empty_browsers = base.join("empty");
    std::fs::create_dir_all(&empty_browsers).unwrap();
    let wanted = plant(&cache, "chromium-1100/chrome-linux/chrome");
    assert_eq!(
        resolve_chromium(&roots(None, Some(&empty_browsers), Some(&cache))),
        Some(wanted.clone())
    );
    // A missing PLAYWRIGHT_BROWSERS_PATH folder is skipped the same way.
    assert_eq!(
        resolve_chromium(&roots(None, Some(&base.join("absent")), Some(&cache))),
        Some(wanted)
    );
    let modern = plant(&cache, "chromium-1100/chrome-linux64/chrome");
    assert_eq!(best_playwright_build(&cache), Some(modern));
    let _ = std::fs::remove_dir_all(&base);
}

/// Builds are ranked by number, not by name: `chromium-1194` beats `chromium-999`, although
/// `"999" > "1194"` as text.
#[test]
fn chromium_newest_build_number_wins() {
    let base = scratch("newest");
    plant(&base, "chromium-999/chrome-linux/chrome");
    let newest = plant(&base, "chromium-1194/chrome-linux/chrome");
    plant(&base, "chromium-1000/chrome-linux64/chrome");
    assert_eq!(best_playwright_build(&base), Some(newest));
    let _ = std::fs::remove_dir_all(&base);
}

/// A full Chrome build of any number beats a headless shell; the shell's `chrome-linux/` layout is
/// used when no full build exists, and it reads as a headless shell.
#[test]
fn chromium_full_build_beats_headless_shell_layouts() {
    let base = scratch("shell");
    let shell = plant(
        &base,
        "chromium_headless_shell-1194/chrome-linux/headless_shell",
    );
    assert_eq!(best_playwright_build(&base), Some(shell.clone()));
    assert!(is_headless_shell(&shell));
    let full = plant(&base, "chromium-1000/chrome-linux/chrome");
    assert_eq!(best_playwright_build(&base), Some(full.clone()));
    assert!(!is_headless_shell(&full));
    let modern_shell = base
        .join("chromium_headless_shell-1194/chrome-headless-shell-linux64/chrome-headless-shell");
    assert!(is_headless_shell(&modern_shell));
    let _ = std::fs::remove_dir_all(&base);
}

/// No root holding a known layout yields `None`, never a guessed path.
#[test]
fn chromium_absent_everywhere_is_none() {
    let base = scratch("absent");
    plant(&base, "chromium-1194/README");
    plant(&base, "firefox-1500/firefox/firefox");
    assert_eq!(
        resolve_chromium(&roots(None, Some(&base), Some(&base))),
        None
    );
    assert_eq!(resolve_chromium(&ChromiumSearchRoots::default()), None);
    let _ = std::fs::remove_dir_all(&base);
}
