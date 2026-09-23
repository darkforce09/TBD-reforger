//! Atomic replacement of a file's contents.
//!
//! The new contents go to a file created beside the original, in the same directory so the
//! final rename never crosses file systems. That file is written, given the original's
//! permission bits, flushed to disk, and renamed over the original. A rename within one file
//! system is atomic: the path names the complete old file until it names the complete new one.
//! On any failure before the rename the new file is removed and the original is untouched. A
//! symbolic link is followed, so the file it names is replaced and the link itself survives.

use std::fs::{self, File, OpenOptions, Permissions};
use std::io::{self, Write};
use std::os::unix::fs::{OpenOptionsExt, PermissionsExt};
use std::path::Path;

use tracing::warn;

/// Permission bits carried over from the original, special bits included.
const PERMISSION_BITS: u32 = 0o7777;
/// The new file stays private to its owner until it takes the original's bits.
const PRIVATE_WHILE_WRITTEN: u32 = 0o600;

pub(super) fn replace_atomically(path: &Path, contents: &[u8]) -> io::Result<()> {
    let target = fs::canonicalize(path)?;
    let (Some(directory), Some(name)) = (target.parent(), target.file_name()) else {
        return Err(io::Error::other("the path names no file in a directory"));
    };
    let mode = fs::metadata(&target)?.permissions().mode() & PERMISSION_BITS;
    let temporary = directory.join(format!(
        ".{}.{}.{:016x}.tmp",
        name.to_string_lossy(),
        std::process::id(),
        rand::random::<u64>()
    ));
    match write_then_rename(&temporary, &target, contents, mode) {
        Ok(()) => {
            flush_directory(directory);
            Ok(())
        }
        Err(error) => {
            // The new file may not exist yet; either way nothing of it may remain.
            let _ = fs::remove_file(&temporary);
            Err(error)
        }
    }
}

fn write_then_rename(
    temporary: &Path,
    target: &Path,
    contents: &[u8],
    mode: u32,
) -> io::Result<()> {
    let mut file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .mode(PRIVATE_WHILE_WRITTEN)
        .open(temporary)?;
    file.write_all(contents)?;
    file.set_permissions(Permissions::from_mode(mode))?;
    file.sync_all()?;
    drop(file);
    fs::rename(temporary, target)
}

/// Makes the rename itself durable. The file already holds the new contents, so a failure here
/// is logged rather than reported as a failed replacement.
fn flush_directory(directory: &Path) {
    if let Err(error) = File::open(directory).and_then(|directory| directory.sync_all()) {
        warn!(%error, directory = %directory.display(), "the directory was not flushed after a rename");
    }
}

#[cfg(test)]
#[path = "tests/atomic_replacement.rs"]
mod tests;
