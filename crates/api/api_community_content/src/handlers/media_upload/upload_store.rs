//! The upload directory writer: a file appears under its public name whole or not at all.
//!
//! **Role:** [`store_upload`] writes an accepted upload into the upload directory under a staging
//! name and renames it to its public name once every byte is on disk.
//! **Position:** called by the upload handler in [`super`] with `Config::upload_dir`, the directory
//! the API's router serves at `/uploads`; the only writer of that directory.
//! **Signals & state:** the upload directory on disk; nothing in memory.
//! **Invariants:**
//! - The public name exists only after the rename, and a rename within one directory is atomic,
//!   so a reader of `/uploads/<name>` sees the whole file or a 404, never a partial file.
//! - The staging name is a fresh random UUID opened with `create_new`, so no two writes share it
//!   and no existing file is truncated.
//! - The bytes are flushed with `sync_all` before the rename, so a crash never leaves a public
//!   name over missing data.
//! - A failed write removes its staging file (best effort) and returns the io error to the caller.

use std::io;
use std::path::Path;

use tokio::io::AsyncWriteExt;
use uuid::Uuid;

/// Writes `bytes` to `directory/file_name` through a staging file and an atomic rename, creating
/// `directory` when it is missing.
pub(super) async fn store_upload(
    directory: &Path,
    file_name: &str,
    bytes: &[u8],
) -> io::Result<()> {
    tokio::fs::create_dir_all(directory).await?;
    let staging = directory.join(format!(".{}.partial", Uuid::new_v4()));
    let stored = write_then_rename(&staging, &directory.join(file_name), bytes).await;
    if stored.is_err() {
        // The staging name is never served under a public name, so a file this cannot remove
        // costs only disk space; the caller still hears the original error.
        let _ = tokio::fs::remove_file(&staging).await;
    }
    stored
}

/// Creates `staging`, writes and flushes `bytes` into it, then renames it to `destination`.
async fn write_then_rename(staging: &Path, destination: &Path, bytes: &[u8]) -> io::Result<()> {
    let mut file = tokio::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(staging)
        .await?;
    file.write_all(bytes).await?;
    file.sync_all().await?;
    drop(file);
    tokio::fs::rename(staging, destination).await
}

#[cfg(test)]
#[path = "tests/upload_store.rs"]
mod tests;
