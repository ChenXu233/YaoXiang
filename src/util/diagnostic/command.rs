use anyhow::{Context, Result};
use serde::Serialize;
use serde_json::Value;
use std::path::{Path, PathBuf};

/// Safely canonicalize a path, falling back to an absolute path derived
/// from the current directory when `canonicalize` fails (e.g. path does
/// not yet exist on disk).
fn safe_canonicalize(path: &Path) -> PathBuf {
    path.canonicalize().unwrap_or_else(|_| {
        if path.is_absolute() {
            path.to_path_buf()
        } else {
            std::env::current_dir()
                .unwrap_or_else(|_| PathBuf::from("."))
                .join(path)
        }
    })
}

use super::{
    check_files_with_diagnostics, CheckResult, EmitterConfig, ErrorCodeDefinition, ErrorInfo,
    I18nRegistry, JsonEmitter, TextEmitter,
};

pub fn run_check_command_once(
    paths: &[PathBuf],
    excludes: &[PathBuf],
    json: bool,
    use_colors: bool,
    no_progress: bool,
) -> Result<usize> {
    let paths = normalize_check_paths(paths)?;
    let files = collect_yx_files_from_paths(&paths, excludes)?;
    if files.is_empty() {
        return Err(anyhow::anyhow!("No .yx files found in provided paths"));
    }

    if !json && !no_progress {
        eprintln!("Checking {} file(s)...", files.len());
    }

    let result = check_files_with_diagnostics(&files)?;

    if json {
        output_check_json(&result)?;
    } else {
        let emitter = TextEmitter::with_config(EmitterConfig {
            use_colors,
            ..Default::default()
        });
        for entry in &result.diagnostics {
            let source_file = result.source_files.get(&entry.file);
            let output = emitter.render_with_source(&entry.diagnostic, source_file);
            eprintln!("\n{}", output);
        }

        if !no_progress {
            if result.error_count == 0 {
                eprintln!("Type check passed ({} file(s))", files.len());
            }
            eprintln!(
                "Summary: {} error(s), {} warning(s)",
                result.error_count, result.warning_count
            );
        }
    }

    Ok(result.error_count)
}

pub fn run_check_watch_command(
    paths: Vec<PathBuf>,
    excludes: Vec<PathBuf>,
    json: bool,
    use_colors: bool,
    no_progress: bool,
) -> Result<()> {
    use notify::{Config, RecommendedWatcher, RecursiveMode, Watcher};
    use std::sync::mpsc;
    use std::time::{Duration, Instant};

    let paths = normalize_check_paths(&paths)?;
    let excludes = normalize_exclude_paths(&excludes)?;

    run_check_command_once(&paths, &excludes, json, use_colors, no_progress)?;

    if !no_progress {
        eprintln!("Watching for changes... press Ctrl+C to stop");
    }

    let (tx, rx) = mpsc::channel();
    let mut watcher = RecommendedWatcher::new(
        move |res| {
            let _ = tx.send(res);
        },
        Config::default().with_poll_interval(Duration::from_millis(200)),
    )?;

    for path in &paths {
        if should_exclude_path(path, &excludes) {
            continue;
        }

        let mode = if path.is_dir() {
            RecursiveMode::Recursive
        } else {
            RecursiveMode::NonRecursive
        };
        watcher
            .watch(path, mode)
            .with_context(|| format!("Failed to watch path: {}", path.display()))?;
    }

    loop {
        let event = match rx.recv() {
            Ok(Ok(event)) => event,
            Ok(Err(err)) => {
                if !no_progress {
                    eprintln!("watch error: {}", err);
                }
                continue;
            }
            Err(_) => break,
        };

        if !is_yx_event(&event, &excludes) {
            continue;
        }

        // 简单防抖：窗口内持续接收事件，直到静默再触发一次检查。
        let mut deadline = Instant::now() + Duration::from_millis(250);
        while Instant::now() < deadline {
            match rx.recv_timeout(Duration::from_millis(50)) {
                Ok(Ok(next_event)) if is_yx_event(&next_event, &excludes) => {
                    deadline = Instant::now() + Duration::from_millis(250);
                }
                Ok(Ok(_)) => {}
                Ok(Err(_)) => {}
                Err(mpsc::RecvTimeoutError::Timeout) => {}
                Err(mpsc::RecvTimeoutError::Disconnected) => break,
            }
        }

        if !json && !no_progress && use_colors {
            eprint!("\x1B[2J\x1B[H");
        }

        let error_count = run_check_command_once(&paths, &excludes, json, use_colors, no_progress)?;
        if !no_progress {
            eprintln!("Last run: {} error(s)", error_count);
        }
    }

    Ok(())
}

pub fn render_explain_output(
    code: &str,
    json: bool,
    lang_code: Option<&str>,
) -> Result<Option<String>> {
    let Some(definition) = ErrorCodeDefinition::find(code) else {
        return Ok(None);
    };

    let lang_code = lang_code.unwrap_or("zh");
    let i18n = I18nRegistry::new(lang_code);
    let info = i18n.get_info(code).unwrap_or(ErrorInfo {
        title: "",
        help: "",
        example: None,
        error_output: None,
    });
    let template = i18n.get_template(code).unwrap_or("");

    if json {
        #[derive(Serialize)]
        struct ExplainOutput<'a> {
            code: &'static str,
            category: String,
            title: &'a str,
            template: &'a str,
            help: &'a str,
            example: Option<&'a str>,
            error_output: Option<&'a str>,
        }

        let output = ExplainOutput {
            code: definition.code,
            category: definition.category.to_string(),
            title: info.title,
            template,
            help: info.help,
            example: info.example,
            error_output: info.error_output,
        };

        Ok(Some(serde_json::to_string_pretty(&output)?))
    } else {
        let mut lines = vec![
            format!("Error {}", definition.code),
            format!("Category: {}", definition.category),
            format!("Title: {}", info.title),
            format!("Message Template: {}", template),
        ];
        if !info.help.is_empty() {
            lines.push(format!("Help: {}", info.help));
        }
        if let Some(example) = info.example {
            lines.push(format!("\nExample:\n{}", example));
        }
        if let Some(output) = info.error_output {
            lines.push(format!("\nExpected Output:\n{}", output));
        }
        Ok(Some(lines.join("\n")))
    }
}

fn collect_yx_files_from_paths(
    paths: &[PathBuf],
    excludes: &[PathBuf],
) -> Result<Vec<PathBuf>> {
    let mut files = Vec::new();
    let excludes = normalize_exclude_paths(excludes)?;

    for path in paths {
        if !path.exists() {
            return Err(anyhow::anyhow!("Path does not exist: {}", path.display()));
        }
        if should_exclude_path(path, &excludes) {
            continue;
        }

        if path.is_file() {
            if path.extension().map(|e| e == "yx").unwrap_or(false) {
                files.push(safe_canonicalize(path));
            }
        } else if path.is_dir() {
            collect_yx_files_recursive_with_excludes(path, &excludes, &mut files)?;
        }
    }

    files.sort();
    files.dedup();
    Ok(files)
}

fn collect_yx_files_recursive_with_excludes(
    dir: &Path,
    excludes: &[PathBuf],
    files: &mut Vec<PathBuf>,
) -> Result<()> {
    if should_exclude_path(dir, excludes) {
        return Ok(());
    }

    for entry in std::fs::read_dir(dir)
        .with_context(|| format!("Failed to read directory: {}", dir.display()))?
    {
        let entry = entry?;
        let path = entry.path();

        if should_exclude_path(&path, excludes) {
            continue;
        }

        if path.is_dir() {
            collect_yx_files_recursive_with_excludes(&path, excludes, files)?;
        } else if path.extension().map(|e| e == "yx").unwrap_or(false) {
            files.push(safe_canonicalize(&path));
        }
    }

    Ok(())
}

fn default_check_path() -> Result<PathBuf> {
    let cwd = std::env::current_dir().context("Failed to get current directory")?;

    for dir in cwd.ancestors() {
        if dir.join(crate::package::manifest::MANIFEST_FILE).exists() {
            return Ok(dir.to_path_buf());
        }
    }

    Ok(cwd)
}

fn normalize_check_paths(paths: &[PathBuf]) -> Result<Vec<PathBuf>> {
    if paths.is_empty() {
        return Ok(vec![default_check_path()?]);
    }

    let cwd = std::env::current_dir().context("Failed to get current directory")?;
    Ok(paths
        .iter()
        .map(|p| {
            if p.is_absolute() {
                p.clone()
            } else {
                cwd.join(p)
            }
        })
        .collect())
}

fn normalize_exclude_paths(excludes: &[PathBuf]) -> Result<Vec<PathBuf>> {
    let cwd = std::env::current_dir().context("Failed to get current directory")?;
    Ok(excludes
        .iter()
        .map(|p| {
            if p.is_absolute() {
                p.clone()
            } else {
                cwd.join(p)
            }
        })
        .collect())
}

fn is_default_excluded_name(path: &Path) -> bool {
    path.components().any(|component| {
        let name = component.as_os_str().to_string_lossy();
        name == ".git" || name == ".yaoxiang" || name == "target"
    })
}

fn should_exclude_path(
    path: &Path,
    excludes: &[PathBuf],
) -> bool {
    if is_default_excluded_name(path) {
        return true;
    }

    let absolute = safe_canonicalize(path);

    excludes
        .iter()
        .any(|excluded| absolute.starts_with(excluded))
}

fn is_yx_event(
    event: &notify::Event,
    excludes: &[PathBuf],
) -> bool {
    event.paths.iter().any(|p| {
        p.extension().map(|ext| ext == "yx").unwrap_or(false) && !should_exclude_path(p, excludes)
    })
}

#[derive(Serialize)]
struct CheckJsonDiagnostic {
    file: String,
    severity: String,
    code: String,
    message: String,
    line: usize,
    column: usize,
    end_line: usize,
    end_column: usize,
    lsp: Value,
}

#[derive(Serialize)]
struct CheckJsonOutput {
    error_count: usize,
    warning_count: usize,
    diagnostics: Vec<CheckJsonDiagnostic>,
}

fn output_check_json(result: &CheckResult) -> Result<()> {
    let diagnostics = result
        .diagnostics
        .iter()
        .map(|entry| {
            let (line, column, end_line, end_column) = entry
                .diagnostic
                .span
                .map(|span| {
                    (
                        span.start.line,
                        span.start.column,
                        span.end.line,
                        span.end.column,
                    )
                })
                .unwrap_or((0, 0, 0, 0));

            let lsp: Value = serde_json::from_str(&JsonEmitter::render(&entry.diagnostic))
                .unwrap_or_else(|e| {
                    tracing::warn!("Failed to serialize diagnostic to JSON: {}", e);
                    serde_json::json!({})
                });

            CheckJsonDiagnostic {
                file: entry.file.clone(),
                severity: entry.diagnostic.severity.to_string(),
                code: entry.diagnostic.code.clone(),
                message: entry.diagnostic.message.clone(),
                line,
                column,
                end_line,
                end_column,
                lsp,
            }
        })
        .collect();

    let payload = CheckJsonOutput {
        error_count: result.error_count,
        warning_count: result.warning_count,
        diagnostics,
    };

    println!("{}", serde_json::to_string_pretty(&payload)?);
    Ok(())
}
