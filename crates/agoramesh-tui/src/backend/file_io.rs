//! Atomic filesystem helpers for TUI local state and key backups.

use std::io::Write;
use std::path::Path;

use crate::error::Error;

/// Writes `bytes` to `path` atomically using a temporary file + rename.
pub(super) fn write_atomic(path: &Path, bytes: &[u8]) -> Result<(), Error> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(Error::StateIo)?;
    }
    let tmp_path = path.with_extension("tmp");
    write_temp_file(&tmp_path, bytes)?;
    std::fs::rename(tmp_path, path).map_err(Error::StateIo)
}

/// Writes `bytes` to `path` and fsyncs the file.
pub(super) fn write_temp_file(path: &Path, bytes: &[u8]) -> Result<(), Error> {
    let mut file = std::fs::File::create(path).map_err(Error::StateIo)?;
    file.write_all(bytes).map_err(Error::StateIo)?;
    file.sync_all().map_err(Error::StateIo)
}

/// Removes a temporary file, ignoring errors.
pub(super) fn remove_temp_file(path: &Path) {
    let _ = std::fs::remove_file(path);
}

/// Fsyncs the parent directory of `path`, ignoring errors.
pub(super) fn sync_parent_dir(path: &Path) {
    let Some(parent) = path.parent() else {
        return;
    };
    if let Ok(directory) = std::fs::File::open(parent) {
        let _ = directory.sync_all();
    }
}
