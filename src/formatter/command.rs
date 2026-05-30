use anyhow::{Context, Result};
use std::path::{Path, PathBuf};

use crate::formatter::{format_source, FormatOptions};

#[derive(Debug, Default, Clone, Copy)]
pub struct FormatRunResult {
    pub needs_formatting: bool,
}

pub fn run_format_command(
    path: &Path,
    options: &FormatOptions,
    check: bool,
    write: bool,
) -> Result<FormatRunResult> {
    let files = collect_yx_files(path)?;
    if files.is_empty() {
        anyhow::bail!("No .yx files found at: {}", path.display());
    }

    let mut needs_formatting = false;

    for file in &files {
        let source = std::fs::read_to_string(file)
            .with_context(|| format!("Failed to read: {}", file.display()))?;

        match format_source(&source, options) {
            Ok(formatted) => {
                if check {
                    if formatted != source {
                        eprintln!("Needs formatting: {}", file.display());
                        needs_formatting = true;
                    }
                } else if write {
                    if formatted != source {
                        std::fs::write(file, &formatted)
                            .with_context(|| format!("Failed to write: {}", file.display()))?;
                        eprintln!("Formatted: {}", file.display());
                    }
                } else {
                    print!("{}", formatted);
                }
            }
            Err(e) => {
                anyhow::bail!("Error formatting {}: {}", file.display(), e);
            }
        }
    }

    Ok(FormatRunResult { needs_formatting })
}

fn collect_yx_files(path: &Path) -> Result<Vec<PathBuf>> {
    let mut files = Vec::new();
    if path.is_file() {
        files.push(path.to_path_buf());
    } else if path.is_dir() {
        collect_yx_files_recursive(path, &mut files)?;
    }
    Ok(files)
}

fn collect_yx_files_recursive(
    dir: &Path,
    files: &mut Vec<PathBuf>,
) -> Result<()> {
    for entry in std::fs::read_dir(dir)
        .with_context(|| format!("Failed to read directory: {}", dir.display()))?
    {
        let entry = entry?;
        let path = entry.path();
        if path.is_dir() {
            collect_yx_files_recursive(&path, files)?;
        } else if path.extension().map(|e| e == "yx").unwrap_or(false) {
            files.push(path);
        }
    }
    Ok(())
}
