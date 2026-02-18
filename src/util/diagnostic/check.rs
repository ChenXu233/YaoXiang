//! Check command implementation
//!
//! Provides the `yaoxiang check` command with support for:
//! - Multiple files and directories
//! - JSON output format
//! - Color control
//! - Watch mode (optional)

use anyhow::{Context, Result};
use std::path::{Path, PathBuf};
use walkdir::WalkDir;

use crate::frontend::Compiler;
use crate::util::diagnostic::{Diagnostic, JsonEmitter, TextEmitter, EmitterConfig};
use crate::util::span::SourceFile;

/// Options for the check command
#[derive(Debug, Clone)]
pub struct CheckOptions<'a> {
    /// Output in JSON format
    pub json: bool,
    /// Color control: "auto", "always", or "never"
    pub color: &'a str,
    /// Watch mode
    pub watch: bool,
}

/// Check multiple paths (files or directories) with options
pub fn check_paths_with_options(paths: &[PathBuf], options: CheckOptions<'_>) -> Result<()> {
    if options.watch {
        check_with_watch(paths, options)
    } else {
        check_once(paths, options)
    }
}

/// Check paths once (without watch mode)
fn check_once(paths: &[PathBuf], options: CheckOptions<'_>) -> Result<()> {
    // Collect all .yx files from the given paths
    let files = collect_source_files(paths)?;
    
    if files.is_empty() {
        eprintln!("No .yx source files found in the specified paths");
        return Ok(());
    }

    // Check all files and collect diagnostics
    let mut all_diagnostics: Vec<(PathBuf, Vec<Diagnostic>)> = Vec::new();
    let mut has_errors = false;

    for file in &files {
        match check_single_file(file) {
            Ok(diagnostics) => {
                if !diagnostics.is_empty() {
                    has_errors = true;
                    all_diagnostics.push((file.clone(), diagnostics));
                }
            }
            Err(e) => {
                // File read error or other IO errors
                eprintln!("Error checking {}: {}", file.display(), e);
                has_errors = true;
            }
        }
    }

    // Output results
    if options.json {
        output_json(&all_diagnostics);
    } else {
        output_text(&all_diagnostics, options.color)?;
    }

    // Exit with error if any errors were found
    if has_errors {
        std::process::exit(1);
    } else if !options.json {
        // Only print success message in non-JSON mode
        println!("✓ All checks passed ({} file{})", 
                 files.len(), 
                 if files.len() == 1 { "" } else { "s" });
    }

    Ok(())
}

/// Check a single file and return diagnostics
fn check_single_file(file: &Path) -> Result<Vec<Diagnostic>> {
    let source = std::fs::read_to_string(file)
        .with_context(|| format!("Failed to read file {}", file.display()))?;

    let source_name = file.display().to_string();
    let mut compiler = Compiler::new();
    
    match compiler.compile(&source_name, &source) {
        Ok(_module) => {
            // Type check passed - no diagnostics
            Ok(Vec::new())
        }
        Err(e) => {
            // Extract diagnostic if available
            if let Some(diagnostic) = e.diagnostic() {
                Ok(vec![diagnostic.clone()])
            } else {
                // Create a generic diagnostic from the error message
                use crate::util::diagnostic::codes::ErrorCodeDefinition;
                let diagnostic = ErrorCodeDefinition::internal_error(e.message()).build();
                Ok(vec![diagnostic])
            }
        }
    }
}

/// Output diagnostics in JSON format
fn output_json(diagnostics: &[(PathBuf, Vec<Diagnostic>)]) {
    use serde::Serialize;
    
    #[derive(Serialize)]
    struct FileResult {
        file: String,
        diagnostics: Vec<serde_json::Value>,
    }

    let mut results = Vec::new();
    for (file, diags) in diagnostics {
        let file_result = FileResult {
            file: file.display().to_string(),
            diagnostics: diags
                .iter()
                .map(|d| {
                    // Parse the JSON from JsonEmitter
                    serde_json::from_str(&JsonEmitter::render(d))
                        .unwrap_or(serde_json::json!({}))
                })
                .collect(),
        };
        results.push(file_result);
    }

    // Output as JSON array
    if let Ok(json) = serde_json::to_string_pretty(&results) {
        println!("{}", json);
    }
}

/// Output diagnostics in text format
fn output_text(
    diagnostics: &[(PathBuf, Vec<Diagnostic>)],
    color: &str,
) -> Result<()> {
    // Determine if we should use color
    let use_color = match color {
        "always" => true,
        "never" => false,
        "auto" | _ => {
            // Auto: use color if stderr is a TTY
            atty::is(atty::Stream::Stderr)
        }
    };

    let config = EmitterConfig {
        use_colors: use_color,
        ..Default::default()
    };
    let emitter = TextEmitter::with_config(config);

    for (file, diags) in diagnostics {
        // Read the source file for rendering
        let source = std::fs::read_to_string(file)
            .with_context(|| format!("Failed to read file {}", file.display()))?;
        let source_file = SourceFile::new(file.display().to_string(), source);

        eprintln!(); // Add spacing
        for diagnostic in diags {
            let output = emitter.render_with_source(diagnostic, Some(&source_file));
            eprintln!("{}", output);
        }
    }

    Ok(())
}

/// Collect all .yx source files from the given paths
fn collect_source_files(paths: &[PathBuf]) -> Result<Vec<PathBuf>> {
    let mut files = Vec::new();

    for path in paths {
        if !path.exists() {
            return Err(anyhow::anyhow!("Path does not exist: {}", path.display()));
        }

        if path.is_file() {
            // Check if it's a .yx file
            if path.extension().and_then(|s| s.to_str()) == Some("yx") {
                files.push(path.clone());
            } else {
                eprintln!("Warning: Skipping non-.yx file: {}", path.display());
            }
        } else if path.is_dir() {
            // Walk the directory and collect all .yx files
            for entry in WalkDir::new(path).follow_links(true) {
                let entry = entry.with_context(|| {
                    format!("Failed to read directory entry in {}", path.display())
                })?;
                
                if entry.file_type().is_file() {
                    if entry.path().extension().and_then(|s| s.to_str()) == Some("yx") {
                        files.push(entry.path().to_path_buf());
                    }
                }
            }
        }
    }

    Ok(files)
}

/// Watch mode implementation
#[cfg(not(feature = "watch"))]
fn check_with_watch(_paths: &[PathBuf], _options: CheckOptions<'_>) -> Result<()> {
    eprintln!("Watch mode is not available in this build.");
    eprintln!("Please rebuild with the 'watch' feature enabled.");
    std::process::exit(1);
}

/// Watch mode implementation with notify crate
#[cfg(feature = "watch")]
fn check_with_watch(paths: &[PathBuf], options: CheckOptions<'_>) -> Result<()> {
    use notify::{Watcher, RecursiveMode, Event};
    use std::sync::mpsc::channel;
    use std::time::Duration;

    println!("Watching for changes... (Press Ctrl+C to stop)");
    
    // Initial check
    let check_options = CheckOptions {
        watch: false, // Don't recurse into watch mode
        ..options.clone()
    };
    let _ = check_once(paths, check_options.clone());

    // Set up file watcher
    let (tx, rx) = channel();
    let mut watcher = notify::recommended_watcher(move |res: Result<Event, notify::Error>| {
        if let Ok(_event) = res {
            let _ = tx.send(());
        }
    })?;

    // Watch all paths
    for path in paths {
        watcher.watch(path, RecursiveMode::Recursive)?;
    }

    // Wait for file changes
    loop {
        if let Ok(_) = rx.recv_timeout(Duration::from_millis(100)) {
            // Debounce: wait a bit for multiple rapid changes
            std::thread::sleep(Duration::from_millis(100));
            
            // Clear previous output
            print!("\x1B[2J\x1B[1;1H"); // Clear screen and move cursor to top
            println!("Files changed, re-checking...\n");
            
            // Re-check
            let _ = check_once(paths, check_options.clone());
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    #[test]
    fn test_collect_single_file() {
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("test.yx");
        fs::write(&file_path, "main = () => {}").unwrap();

        let files = collect_source_files(&[file_path.clone()]).unwrap();
        assert_eq!(files.len(), 1);
        assert_eq!(files[0], file_path);
    }

    #[test]
    fn test_collect_directory() {
        let temp_dir = TempDir::new().unwrap();
        let file1 = temp_dir.path().join("test1.yx");
        let file2 = temp_dir.path().join("test2.yx");
        let file3 = temp_dir.path().join("test.txt");
        
        fs::write(&file1, "main = () => {}").unwrap();
        fs::write(&file2, "main = () => {}").unwrap();
        fs::write(&file3, "not a yx file").unwrap();

        let files = collect_source_files(&[temp_dir.path().to_path_buf()]).unwrap();
        assert_eq!(files.len(), 2);
    }

    #[test]
    fn test_collect_multiple_paths() {
        let temp_dir = TempDir::new().unwrap();
        let file1 = temp_dir.path().join("test1.yx");
        let subdir = temp_dir.path().join("subdir");
        fs::create_dir(&subdir).unwrap();
        let file2 = subdir.join("test2.yx");
        
        fs::write(&file1, "main = () => {}").unwrap();
        fs::write(&file2, "main = () => {}").unwrap();

        let files = collect_source_files(&[file1.clone(), subdir.clone()]).unwrap();
        assert_eq!(files.len(), 2);
    }

    #[test]
    fn test_check_valid_file() {
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("test.yx");
        fs::write(&file_path, "use std.io\n\nmain = () => {\n  print(\"test\\n\")\n}").unwrap();

        let diagnostics = check_single_file(&file_path).unwrap();
        assert_eq!(diagnostics.len(), 0);
    }

    #[test]
    fn test_check_invalid_file() {
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("test.yx");
        fs::write(&file_path, "use std.io\n\nmain = () => {\n  print(unknown_var)\n}").unwrap();

        let diagnostics = check_single_file(&file_path).unwrap();
        assert!(diagnostics.len() > 0);
    }
}
