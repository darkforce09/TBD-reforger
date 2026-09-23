//! Atomic evidence publication through pinned directories without following symlinks.

use std::{
    ffi::CString,
    fs::File,
    io::{self, Write},
    os::{
        fd::{AsRawFd, FromRawFd},
        unix::ffi::OsStrExt,
    },
    path::{Component, Path},
    sync::atomic::{AtomicU64, Ordering},
};

use anyhow::{Context, Result, ensure};

static TEMPORARY_SEQUENCE: AtomicU64 = AtomicU64::new(0);

fn evidence_name(name: &str) -> Result<CString> {
    let mut components = Path::new(name).components();
    ensure!(
        matches!(components.next(), Some(Component::Normal(_))) && components.next().is_none(),
        "evidence name must be a single relative filename"
    );
    ensure!(!name.contains(['/', '\\']), "invalid evidence filename");
    CString::new(name).context("invalid evidence filename")
}

fn open_directory(directory: &Path) -> Result<File> {
    let absolute = if directory.is_absolute() {
        directory.to_path_buf()
    } else {
        std::env::current_dir()?.join(directory)
    };
    let mut parent = File::open("/")?;
    for component in absolute.components() {
        let name = match component {
            Component::RootDir => continue,
            Component::Normal(name) => CString::new(name.as_bytes())?,
            _ => anyhow::bail!("unsafe evidence directory component"),
        };
        let mut descriptor = open_child_directory(&parent, &name);
        if descriptor
            .as_ref()
            .is_err_and(|error| error.raw_os_error() == Some(libc::ENOENT))
        {
            // SAFETY: parent is open, name is terminated and contains no separator.
            let created = unsafe { libc::mkdirat(parent.as_raw_fd(), name.as_ptr(), 0o755) };
            if created != 0 {
                let error = io::Error::last_os_error();
                if error.raw_os_error() != Some(libc::EEXIST) {
                    return Err(error.into());
                }
            }
            descriptor = open_child_directory(&parent, &name);
        }
        parent = descriptor.context("open evidence directory without following symlinks")?;
    }
    Ok(parent)
}

fn open_child_directory(parent: &File, name: &CString) -> io::Result<File> {
    // SAFETY: parent owns a live descriptor and name is a terminated C string.
    let descriptor = unsafe {
        libc::openat(
            parent.as_raw_fd(),
            name.as_ptr(),
            libc::O_RDONLY | libc::O_DIRECTORY | libc::O_NOFOLLOW | libc::O_CLOEXEC,
        )
    };
    if descriptor < 0 {
        return Err(io::Error::last_os_error());
    }
    // SAFETY: openat returned a new descriptor owned solely by this File.
    Ok(unsafe { File::from_raw_fd(descriptor) })
}

fn validate_destination(directory: &File, name: &CString) -> Result<()> {
    let mut metadata = std::mem::MaybeUninit::<libc::stat>::uninit();
    // SAFETY: the stat buffer is writable and the other arguments remain live.
    let status = unsafe {
        libc::fstatat(
            directory.as_raw_fd(),
            name.as_ptr(),
            metadata.as_mut_ptr(),
            libc::AT_SYMLINK_NOFOLLOW,
        )
    };
    if status != 0 {
        let error = io::Error::last_os_error();
        if error.raw_os_error() == Some(libc::ENOENT) {
            return Ok(());
        }
        return Err(error.into());
    }
    // SAFETY: successful fstatat initializes the stat buffer.
    let metadata = unsafe { metadata.assume_init() };
    ensure!(
        metadata.st_mode & libc::S_IFMT == libc::S_IFREG,
        "evidence destination is not a regular file"
    );
    Ok(())
}

struct TemporaryFile<'a> {
    directory: &'a File,
    name: CString,
    committed: bool,
}

impl Drop for TemporaryFile<'_> {
    fn drop(&mut self) {
        if !self.committed {
            // SAFETY: the borrowed directory outlives this guard; name is terminated.
            unsafe { libc::unlinkat(self.directory.as_raw_fd(), self.name.as_ptr(), 0) };
        }
    }
}

fn temporary_file(directory: &File) -> Result<(File, TemporaryFile<'_>)> {
    for _ in 0..128 {
        let sequence = TEMPORARY_SEQUENCE.fetch_add(1, Ordering::Relaxed);
        let name = CString::new(format!(
            ".api-evidence-{}-{sequence}.tmp",
            std::process::id()
        ))?;
        // O_CREAT | O_EXCL provides create_new semantics relative to the pinned parent.
        // SAFETY: the parent descriptor and terminated name remain live.
        let descriptor = unsafe {
            libc::openat(
                directory.as_raw_fd(),
                name.as_ptr(),
                libc::O_WRONLY | libc::O_CREAT | libc::O_EXCL | libc::O_NOFOLLOW | libc::O_CLOEXEC,
                0o600,
            )
        };
        if descriptor >= 0 {
            // SAFETY: openat created a new descriptor owned solely by this File.
            let file = unsafe { File::from_raw_fd(descriptor) };
            return Ok((
                file,
                TemporaryFile {
                    directory,
                    name,
                    committed: false,
                },
            ));
        }
        let error = io::Error::last_os_error();
        if error.raw_os_error() != Some(libc::EEXIST) {
            return Err(error.into());
        }
    }
    anyhow::bail!("could not allocate an exclusive evidence temporary file")
}

/// Publish one complete receipt or log. Parent descriptors fence directory replacement;
/// replacing a destination entry never writes through an existing symlink or hard link.
pub(super) fn write(directory: &Path, name: &str, bytes: &[u8]) -> Result<()> {
    let name = evidence_name(name)?;
    let directory = open_directory(directory)?;
    validate_destination(&directory, &name)?;
    let (mut file, mut temporary) = temporary_file(&directory)?;
    file.write_all(bytes)?;
    file.sync_all()?;
    drop(file);
    validate_destination(&directory, &name)?;
    // SAFETY: both directory descriptors are live and both names are terminated.
    let renamed = unsafe {
        libc::renameat(
            directory.as_raw_fd(),
            temporary.name.as_ptr(),
            directory.as_raw_fd(),
            name.as_ptr(),
        )
    };
    if renamed != 0 {
        return Err(io::Error::last_os_error().into());
    }
    temporary.committed = true;
    directory.sync_all()?;
    Ok(())
}
