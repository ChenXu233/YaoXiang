use anyhow::{Context, Result};
use std::path::{Path, PathBuf};

use crate::formatter::{format_source, FormatError, FormatOptions};

#[derive(Debug, Default, Clone, Copy)]
pub struct FormatRunResult {
    pub needs_formatting: bool,
}

pub fn run_format_command(
    path: &Path,
    options: &FormatOptions,
    dry_run: bool,
    write: bool,
) -> Result<FormatRunResult> {
    let files = collect_yx_files(path)?;
    if files.is_empty() {
        anyhow::bail!("No .yx files found at: {}", path.display());
    }

    let mut needs_formatting = false;
    let mut errors: Vec<String> = Vec::new();

    for file in &files {
        let source = match std::fs::read_to_string(file) {
            Ok(s) => s,
            Err(e) => {
                errors.push(format!("Failed to read {}: {}", file.display(), e));
                continue;
            }
        };

        match format_source(&source, options) {
            Ok(formatted) => {
                if dry_run {
                    if formatted != source {
                        eprintln!("Would format: {}", file.display());
                        needs_formatting = true;
                    }
                } else if write {
                    if formatted != source {
                        if let Err(e) = std::fs::write(file, &formatted) {
                            errors.push(format!("Failed to write {}: {}", file.display(), e));
                        } else {
                            eprintln!("Formatted: {}", file.display());
                        }
                    }
                } else {
                    print!("{}", formatted);
                }
            }
            Err(FormatError::Semantic(diags)) => {
                eprintln!("{}:", file.display());
                for d in diags {
                    eprintln!("  error[{}]: {}", d.code, d.message);
                }
            }
            Err(FormatError::FormatterBug { .. }) => {
                eprintln!(
                    "{}: Formatter internal error — this is a bug, please report it at \
                     https://github.com/ChenXu233/YaoXiang/issues/new",
                    file.display()
                );
            }
        }
    }

    if !errors.is_empty() {
        for err in &errors {
            eprintln!("error: {}", err);
        }
        anyhow::bail!("{} error(s) occurred during formatting", errors.len());
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
