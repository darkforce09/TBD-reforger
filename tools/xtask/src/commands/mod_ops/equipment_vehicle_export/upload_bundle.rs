use super::{FileDigest, files};
use anyhow::{Context, Result, ensure};
use flate2::{Compression, GzBuilder};
use std::{
    collections::BTreeMap,
    fs::{self, File, OpenOptions},
    path::Path,
};

pub(super) fn write(
    root: &Path,
    published: &Path,
    id: &str,
    expected: &BTreeMap<String, FileDigest>,
) -> Result<()> {
    let archives = files::child(root, "archives")?;
    fs::create_dir_all(&archives)?;
    let target = files::child(&archives, &format!("{id}.tar.gz"))?;
    let temporary = files::child(&archives, &format!(".{id}.tar.gz.tmp"))?;
    if temporary.exists() {
        fs::remove_file(&temporary)?;
    }
    let output = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&temporary)?;
    let gzip = GzBuilder::new()
        .mtime(0)
        .write(output, Compression::default());
    let mut archive = tar::Builder::new(gzip);
    for relative in expected
        .keys()
        .map(String::as_str)
        .chain(std::iter::once("manifest.json"))
    {
        let data = fs::read(files::child(published, relative)?)?;
        if let Some(digest) = expected.get(relative) {
            ensure!(
                &files::digest(&data) == digest,
                "published file changed while packaging: {relative}"
            );
        }
        let mut header = tar::Header::new_gnu();
        header.set_size(data.len() as u64);
        header.set_mode(0o644);
        header.set_uid(0);
        header.set_gid(0);
        header.set_mtime(0);
        header.set_cksum();
        archive.append_data(&mut header, relative, &data[..])?;
    }
    let output = archive.into_inner()?.finish()?;
    output.sync_all()?;
    let digest = files::digest(&fs::read(&temporary)?);
    if target.exists() {
        ensure!(
            files::digest(&fs::read(&target)?) == digest,
            "existing upload bundle differs"
        );
        fs::remove_file(&temporary)?;
    } else {
        fs::rename(&temporary, &target)?;
    }
    File::open(target.parent().context("bundle directory")?)?.sync_all()?;
    println!(
        "Upload bundle: {} ({} bytes; sha256 {})",
        target.display(),
        digest.bytes,
        digest.sha256
    );
    Ok(())
}
