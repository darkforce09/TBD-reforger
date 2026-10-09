//! The API packages the source scans read, each exactly once.
//!
//! **Role:** lists every package under `crates/api/` (each folder holding a `Cargo.toml`) together
//! with the application this test code is compiled in, deduplicated by manifest folder, and
//! refuses a list in which a package name repeats, a folder sits inside another, or the
//! application is missing or doubled.
//!
//! **Position:** shared by the layout and prose rules of `src/tests/` (mounted there with
//! `#[path]`) and by the integration suites through `common::api_packages`: the route-tag
//! collection and the route table's crate resolution, the raw `TEST_DATABASE_URL` read pin, the
//! nullable select scan and the debug route tag scan. Each scan picks its scope per package
//! (the application's `src/` or the whole folder of another API crate) from this one list.
//!
//! **Signals & state:** one process-wide [`OnceLock`] holding the list read on first use.
//!
//! **Invariants:** the application sits under `crates/api/` and is also named by
//! `CARGO_MANIFEST_DIR`; the two spellings collapse to one entry, so a scan that walks the
//! application and then every API crate reads no file twice. The list is sorted by package name.

use std::path::{Path, PathBuf};
use std::sync::OnceLock;

use repository_laws::cargo_manifest::read_manifest;

/// One API package: its name, its manifest folder, and whether it is the application.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ApiPackage {
    /// `[package] name` of its manifest.
    pub(crate) name: String,
    /// The folder holding its `Cargo.toml`, spelled as the walk of `crates/api/` under the
    /// repository root spells it.
    pub(crate) folder: PathBuf,
    /// Whether this is the application the scanning test is compiled in.
    pub(crate) is_application: bool,
}

impl ApiPackage {
    /// The package's `src/` folder.
    pub(crate) fn source_root(&self) -> PathBuf {
        self.folder.join("src")
    }
}

/// Every API package, the application included once, read on first use.
pub(crate) fn api_packages() -> Result<&'static [ApiPackage], String> {
    static PACKAGES: OnceLock<Result<Vec<ApiPackage>, String>> = OnceLock::new();
    PACKAGES
        .get_or_init(|| {
            let application = Path::new(env!("CARGO_MANIFEST_DIR"));
            let repository = repository_root::find_repository_root_from(application)
                .map_err(|e| format!("no repository root above {}: {e}", application.display()))?;
            api_packages_under(&repository.join("crates/api"), application)
        })
        .as_deref()
        .map_err(Clone::clone)
}

/// The packages of the folders under `api_folder` holding a `Cargo.toml`, plus the package at
/// `application`, each folder once, checked by [`check_each_package_once`].
pub(crate) fn api_packages_under(
    api_folder: &Path,
    application: &Path,
) -> Result<Vec<ApiPackage>, String> {
    let entries =
        std::fs::read_dir(api_folder).map_err(|e| format!("read {}: {e}", api_folder.display()))?;
    let mut folders = Vec::new();
    for entry in entries {
        let folder = entry
            .map_err(|e| format!("read {}: {e}", api_folder.display()))?
            .path();
        if folder.join("Cargo.toml").is_file() {
            folders.push((canonical(&folder)?, folder));
        }
    }
    let application_folder = canonical(application)?;
    folders.push((application_folder.clone(), application.to_path_buf()));
    // A stable sort keeps the `crates/api/` spelling of the application ahead of the
    // `CARGO_MANIFEST_DIR` spelling, and the deduplication keeps the first of each folder.
    folders.sort_by(|a, b| a.0.cmp(&b.0));
    folders.dedup_by(|later, earlier| later.0 == earlier.0);
    let mut packages = folders
        .into_iter()
        .map(|(resolved, folder)| {
            let manifest_path = folder.join("Cargo.toml");
            let manifest = read_manifest(&manifest_path)
                .map_err(|e| format!("read {}: {e}", manifest_path.display()))?;
            let name = manifest
                .package_name
                .ok_or_else(|| format!("{} declares no package", manifest_path.display()))?;
            Ok(ApiPackage {
                name,
                folder,
                is_application: resolved == application_folder,
            })
        })
        .collect::<Result<Vec<_>, String>>()?;
    packages.sort_by(|a, b| a.name.cmp(&b.name));
    check_each_package_once(&packages)?;
    Ok(packages)
}

/// Refuse a package list a scan would read some file of twice: a package name listed twice, a
/// folder equal to or inside another's, or not exactly one application.
pub(crate) fn check_each_package_once(packages: &[ApiPackage]) -> Result<(), String> {
    let mut problems = Vec::new();
    for (index, package) in packages.iter().enumerate() {
        for other in &packages[index + 1..] {
            if package.name == other.name {
                problems.push(format!("package `{}` is listed twice", package.name));
            }
            if package.folder.starts_with(&other.folder)
                || other.folder.starts_with(&package.folder)
            {
                problems.push(format!(
                    "the folders of `{}` ({}) and `{}` ({}) overlap",
                    package.name,
                    package.folder.display(),
                    other.name,
                    other.folder.display()
                ));
            }
        }
    }
    let applications = packages.iter().filter(|p| p.is_application).count();
    if applications != 1 {
        problems.push(format!(
            "{applications} application package(s) listed; exactly one is"
        ));
    }
    if problems.is_empty() {
        Ok(())
    } else {
        Err(problems.join("; "))
    }
}

/// `path` with every symbolic link and `..` resolved, so two spellings of a folder compare equal.
fn canonical(path: &Path) -> Result<PathBuf, String> {
    path.canonicalize()
        .map_err(|e| format!("resolve {}: {e}", path.display()))
}
