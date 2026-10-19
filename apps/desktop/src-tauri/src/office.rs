//! Converting PowerPoint, Keynote and OpenDocument presentations to PDF
//! with LibreOffice, when it is installed. The PDF is then imported like
//! any other (see `commands::presentations`).

use std::path::{Path, PathBuf};
use std::process::Command;

use crate::error::{AppError, AppResult};

/// Where LibreOffice's command-line program usually is, per platform.
fn candidates() -> Vec<PathBuf> {
    let mut paths = vec![
        PathBuf::from("/Applications/LibreOffice.app/Contents/MacOS/soffice"),
        PathBuf::from(r"C:\Program Files\LibreOffice\program\soffice.exe"),
        PathBuf::from(r"C:\Program Files (x86)\LibreOffice\program\soffice.exe"),
        PathBuf::from("/usr/bin/soffice"),
        PathBuf::from("/usr/bin/libreoffice"),
        PathBuf::from("/usr/local/bin/soffice"),
        PathBuf::from("/snap/bin/libreoffice"),
    ];
    if let Some(home) = std::env::var_os("HOME") {
        paths.push(Path::new(&home).join("Applications/LibreOffice.app/Contents/MacOS/soffice"));
    }
    paths
}

/// LibreOffice's program, if it is installed in a usual place.
pub fn find_libreoffice() -> Option<PathBuf> {
    candidates().into_iter().find(|path| path.is_file())
}

/// Converts `file` to PDF in `out_folder` and returns the PDF's path.
/// Blocks (a large presentation takes a few seconds), so call it from a
/// background task.
pub fn convert_to_pdf(file: &Path, out_folder: &Path) -> AppResult<PathBuf> {
    let soffice = find_libreoffice().ok_or_else(|| {
        AppError::Conversion(
            "To import PowerPoint or Keynote files, install LibreOffice (free), or save the \
             presentation as PDF and import the PDF."
                .into(),
        )
    })?;

    let output = Command::new(soffice)
        .args(["--headless", "--convert-to", "pdf", "--outdir"])
        .arg(out_folder)
        .arg(file)
        .output()?;

    let stem = file.file_stem().unwrap_or_default();
    let pdf = out_folder.join(stem).with_extension("pdf");
    if output.status.success() && pdf.is_file() {
        Ok(pdf)
    } else {
        Err(AppError::Conversion(format!(
            "LibreOffice could not convert the presentation: {}",
            String::from_utf8_lossy(&output.stderr).trim()
        )))
    }
}
