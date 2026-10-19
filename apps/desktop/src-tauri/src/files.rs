//! Small file-system helpers.

use std::fs;
use std::io;
use std::path::Path;

/// Writes `bytes` to `path` without ever leaving a half-written file.
///
/// The data goes to a temporary file next to the target first, which is
/// then renamed over it. A rename within one directory is atomic, so after
/// a crash or power cut mid-save the file is either the old version or the
/// new one, never a broken mix. That matters for a service schedule.
pub fn write_atomically(path: &Path, bytes: &[u8]) -> io::Result<()> {
    let mut temporary = path.as_os_str().to_owned();
    temporary.push(".saving");

    fs::write(&temporary, bytes)?;
    fs::rename(&temporary, path)
}
