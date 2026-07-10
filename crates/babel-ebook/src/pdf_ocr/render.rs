//! Render PDF pages to images using the Poppler `pdftoppm` tool.

use std::path::{Path, PathBuf};
use std::process::Command;

use crate::core::BabelEbookError;

/// Locate the `pdftoppm` executable.
///
/// First checks `PATH`, then falls back to common Poppler installation
/// directories on Windows (winget / manual installs).
fn find_pdftoppm() -> Option<PathBuf> {
    if let Ok(path) = which::which("pdftoppm") {
        return Some(path);
    }

    #[cfg(target_os = "windows")]
    {
        let local_app_data = std::env::var_os("LOCALAPPDATA")?;
        let winget_base = Path::new(&local_app_data)
            .join("Microsoft")
            .join("WinGet")
            .join("Packages");

        // winget package id: oschwartz10612.Poppler_Microsoft.Winget.Source_8wekyb3d8bbwe
        if let Ok(entries) = std::fs::read_dir(&winget_base) {
            for entry in entries.flatten() {
                let name = entry.file_name();
                let name = name.to_string_lossy();
                if name.starts_with("oschwartz10612.Poppler") {
                    let candidate = entry
                        .path()
                        .join("poppler-25.07.0")
                        .join("Library")
                        .join("bin")
                        .join("pdftoppm.exe");
                    if candidate.is_file() {
                        return Some(candidate);
                    }
                }
            }
        }

        // Generic fallback: search Program Files for poppler-*/bin/pdftoppm.exe
        let program_files = std::env::var_os("ProgramFiles")?;
        if let Ok(entries) = std::fs::read_dir(&program_files) {
            for entry in entries.flatten() {
                let name = entry.file_name();
                let name = name.to_string_lossy();
                if name.starts_with("poppler-") {
                    let candidate = entry.path().join("bin").join("pdftoppm.exe");
                    if candidate.is_file() {
                        return Some(candidate);
                    }
                }
            }
        }
    }

    None
}

/// Render a range of PDF pages to PNG images.
///
/// Returns the paths to the generated images, one per page, in page order.
/// `dpi` controls the output resolution; higher values give sharper images but
/// use more tokens when sent to the OCR backend.
///
/// # Errors
///
/// Returns an error if `pdftoppm` is not on `PATH` or if it exits with an error.
pub fn render_pages(
    pdf_path: &Path,
    output_dir: &Path,
    dpi: u32,
) -> Result<Vec<PathBuf>, BabelEbookError> {
    let pdftoppm = find_pdftoppm().ok_or_else(|| {
        BabelEbookError::Configuration(
            "`pdftoppm` not found. Please install Poppler and ensure it is on PATH.".into(),
        )
    })?;

    let prefix = "page";
    let mut cmd = Command::new(pdftoppm);
    cmd.arg("-png")
        .arg("-r")
        .arg(dpi.to_string())
        .arg("-progress")
        .arg(pdf_path)
        .arg(output_dir.join(prefix));

    let output = cmd.output().map_err(|e| {
        BabelEbookError::Configuration(format!(
            "failed to run `pdftoppm`. Is Poppler installed and on PATH? {e}"
        ))
    })?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(BabelEbookError::Configuration(format!(
            "pdftoppm failed: {stderr}"
        )));
    }

    // pdftoppm writes files named <prefix>-<page>.png, zero-padded.
    let mut files: Vec<PathBuf> = Vec::new();
    for entry in std::fs::read_dir(output_dir).map_err(|e| {
        BabelEbookError::Anyhow(anyhow::anyhow!(
            "failed to read rendered page directory: {e}"
        ))
    })? {
        let entry = entry.map_err(|e| {
            BabelEbookError::Anyhow(anyhow::anyhow!("failed to read directory entry: {e}"))
        })?;
        let name = entry.file_name();
        let name = name.to_string_lossy();
        if name.starts_with(prefix) && name.ends_with(".png") {
            files.push(entry.path());
        }
    }

    files.sort_by(|a, b| {
        let a_name = a.file_stem().and_then(|s| s.to_str()).unwrap_or("");
        let b_name = b.file_stem().and_then(|s| s.to_str()).unwrap_or("");
        // Filenames like page-01, page-02 ... sort lexicographically.
        a_name.cmp(b_name)
    });

    Ok(files)
}

/// Render a single PDF page to a PNG image.
pub fn render_page(
    pdf_path: &Path,
    output_dir: &Path,
    page_number: usize,
    dpi: u32,
) -> Result<PathBuf, BabelEbookError> {
    let pdftoppm = find_pdftoppm().ok_or_else(|| {
        BabelEbookError::Configuration(
            "`pdftoppm` not found. Please install Poppler and ensure it is on PATH.".into(),
        )
    })?;

    let prefix = format!("page-{page_number:03}");
    let output_path = output_dir.join(format!("{prefix}.png"));

    let mut cmd = Command::new(pdftoppm);
    cmd.arg("-png")
        .arg("-r")
        .arg(dpi.to_string())
        .arg("-f")
        .arg(page_number.to_string())
        .arg("-l")
        .arg(page_number.to_string())
        .arg(pdf_path)
        .arg(output_dir.join(&prefix));

    let output = cmd.output().map_err(|e| {
        BabelEbookError::Configuration(format!(
            "failed to run `pdftoppm`. Is Poppler installed and on PATH? {e}"
        ))
    })?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(BabelEbookError::Configuration(format!(
            "pdftoppm failed: {stderr}"
        )));
    }

    Ok(output_path)
}
