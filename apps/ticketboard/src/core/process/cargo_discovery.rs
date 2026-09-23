use super::*;
// ---- cargo resolution ----

/// Resolve the cargo binary from real process env. Order: `$CARGO` (set by cargo
/// itself when it launched us) → `cargo` on `$PATH` → `$HOME/.cargo/bin/cargo`
/// (the rustup default a bare GUI PATH misses) → literal `cargo`, whose spawn
/// failure surfaces verbatim in the banner.
pub fn resolve_cargo() -> PathBuf {
    resolve_cargo_from(
        std::env::var_os("CARGO").as_deref(),
        std::env::var_os("PATH").as_deref(),
        std::env::var_os("HOME").map(PathBuf::from).as_deref(),
    )
}

/// The pure resolution order — env injected for tests.
pub fn resolve_cargo_from(
    cargo_env: Option<&OsStr>,
    path_env: Option<&OsStr>,
    home: Option<&Path>,
) -> PathBuf {
    if let Some(cargo) = cargo_env
        && !cargo.is_empty()
    {
        let candidate = PathBuf::from(cargo);
        if candidate.is_file() {
            return candidate;
        }
    }
    if let Some(path) = path_env {
        for dir in std::env::split_paths(path) {
            if dir.as_os_str().is_empty() {
                continue;
            }
            let candidate = dir.join("cargo");
            if candidate.is_file() {
                return candidate;
            }
        }
    }
    if let Some(home) = home {
        let candidate = home.join(".cargo").join("bin").join("cargo");
        if candidate.is_file() {
            return candidate;
        }
    }
    PathBuf::from("cargo")
}
