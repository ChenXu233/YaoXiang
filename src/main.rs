//! YaoXiang Programming Language - CLI

use anyhow::{Context, Result};
use clap::{Parser, Subcommand, ValueEnum};
use serde::Serialize;
use serde_json::Value;
use std::io::IsTerminal;
use std::path::PathBuf;
use tracing::info;
use yaoxiang::{build_bytecode, dump_bytecode, run, NAME, VERSION};
use yaoxiang::util::logger::LogLevel;
use yaoxiang::util::i18n::set_lang_from_string;
use yaoxiang::util::diagnostic::{
    run_file_with_diagnostics, ErrorCodeDefinition, I18nRegistry, ErrorInfo,
    check_files_with_diagnostics, EmitterConfig, JsonEmitter, TextEmitter,
};
use yaoxiang::package;

/// Log level enum for CLI
#[derive(Debug, Clone, Copy, ValueEnum)]
enum LogLevelArg {
    Debug,
    Info,
    Warn,
    Error,
}

impl From<LogLevelArg> for LogLevel {
    fn from(level: LogLevelArg) -> Self {
        match level {
            LogLevelArg::Debug => LogLevel::Debug,
            LogLevelArg::Info => LogLevel::Info,
            LogLevelArg::Warn => LogLevel::Warn,
            LogLevelArg::Error => LogLevel::Error,
        }
    }
}

/// Language enum for CLI
#[derive(Debug, Clone, Copy, ValueEnum)]
enum LangArg {
    En,
    Zh,
    ZhMiao,
}

impl From<LangArg> for String {
    fn from(lang: LangArg) -> Self {
        match lang {
            LangArg::En => "en".to_string(),
            LangArg::Zh => "zh".to_string(),
            LangArg::ZhMiao => "zh-x-miao".to_string(),
        }
    }
}

/// Color output behavior for diagnostics
#[derive(Debug, Clone, Copy, ValueEnum)]
enum ColorChoice {
    Auto,
    Always,
    Never,
}

/// A high-performance programming language with "everything is type" philosophy
#[derive(Parser, Debug)]
#[command(name = "yaoxiang")]
#[command(author = "YaoXiang Team")]
#[command(version = VERSION)]
#[command(about = NAME, long_about = None)]
struct Args {
    #[command(subcommand)]
    command: Option<Commands>,

    /// Enable verbose output
    #[arg(short, long)]
    verbose: bool,

    /// Set log level (debug, info, warn, error)
    #[arg(short, long, value_enum)]
    log_level: Option<LogLevelArg>,

    /// Set language (en, zh, zh-miao)
    #[arg(short = 'L', long, value_enum)]
    lang: Option<LangArg>,
}

#[derive(Subcommand, Debug)]
enum Commands {
    /// Run a YaoXiang source file
    Run {
        /// Source file to run
        #[arg(value_name = "FILE")]
        file: PathBuf,
    },

    /// Evaluate YaoXiang code from command line (not supported well yet)
    Eval {
        /// Code to evaluate
        #[arg(value_name = "CODE")]
        code: String,
    },

    /// Check source file for errors (type checking) (unsupported yet)
    Check {
        /// Source file(s) or directory path(s) to check
        #[arg(value_name = "PATH", required = true, num_args = 1..)]
        paths: Vec<PathBuf>,

        /// Output diagnostics in JSON format
        #[arg(long)]
        json: bool,

        /// Watch input paths and re-check on file changes
        #[arg(short, long)]
        watch: bool,

        /// Control color output (auto, always, never)
        #[arg(long, value_enum, default_value = "auto")]
        color: ColorChoice,

        /// Suppress progress and summary messages
        #[arg(long)]
        no_progress: bool,
    },

    /// Format source file
    Format {
        /// Source file or directory to format
        #[arg(value_name = "PATH")]
        file: PathBuf,

        /// Check if files are formatted without modifying them
        #[arg(short, long)]
        check: bool,

        /// Write formatted output back to file(s) in place
        #[arg(short, long)]
        write: bool,

        /// Output to stdout (default when neither --write nor --check)
        #[arg(long)]
        stdout: bool,

        /// Override indent width
        #[arg(long)]
        indent: Option<usize>,

        /// Override max line width
        #[arg(long)]
        line_width: Option<usize>,

        /// Use tab indentation
        #[arg(long)]
        use_tabs: bool,

        /// Use single quotes for strings
        #[arg(long)]
        single_quote: bool,
    },

    /// Dump bytecode for debugging
    Dump {
        /// Source file to dump
        #[arg(value_name = "FILE")]
        file: PathBuf,
    },

    /// Build bytecode file
    Build {
        /// Source file to compile
        #[arg(value_name = "FILE")]
        file: PathBuf,

        /// Output file (optional, defaults to <input>.42)
        #[arg(short, long)]
        output: Option<PathBuf>,
    },

    /// Explain an error code
    Explain {
        /// Error code to explain (e.g., E1001)
        #[arg(value_name = "CODE")]
        code: String,

        /// Output in JSON format
        #[arg(short, long)]
        json: bool,

        /// Language for explanation (en, zh)
        #[arg(short, long, value_enum)]
        lang: Option<LangArg>,
    },

    /// Print version information
    Version,

    /// Start TUI REPL (default when no command is provided) (Experimental Feature)
    Repl {
        #[arg(short, long)]
        tui: bool,
    },

    /// Initialize a new YaoXiang project
    Init {
        /// Project name
        #[arg(value_name = "NAME")]
        name: String,
    },

    /// Add a dependency to the current project
    Add {
        /// Dependency name
        #[arg(value_name = "DEP")]
        dep: String,

        /// Version string (e.g. "1.0.0")
        #[arg(short, long)]
        version: Option<String>,

        /// Add as dev-dependency
        #[arg(short = 'D', long)]
        dev: bool,
    },

    /// Remove a dependency from the current project
    Rm {
        /// Dependency name to remove
        #[arg(value_name = "DEP")]
        dep: String,

        /// Remove from dev-dependencies
        #[arg(short = 'D', long)]
        dev: bool,
    },

    /// Update all dependencies (or a specific one)
    Update {
        /// Optional: specific package to update
        #[arg(value_name = "PKG")]
        pkg: Option<String>,
    },

    /// Install all dependencies
    Install,

    /// List all dependencies
    List,

    /// Start the Language Server Protocol (LSP) server
    Lsp,
}

fn main() -> Result<()> {
    let args = Args::parse();

    // Set language first (before logger init)
    let lang = args.lang.map(Into::<String>::into).unwrap_or_else(|| {
        std::env::var("YAOXIANG_LANG")
            .ok()
            .and_then(|s| {
                // Only use if it's a valid language
                if ["en", "zh", "zh-x-miao", "zh-miao"].contains(&s.as_str()) {
                    Some(s)
                } else {
                    None
                }
            })
            .unwrap_or_else(|| "en".to_string())
    });
    set_lang_from_string(lang);

    // 如果没有提供子命令，启动 TUI REPL
    let command = args.command.unwrap_or(Commands::Repl { tui: false });

    // Initialize logger
    // LSP 模式必须写 stderr，避免污染 stdout 的 JSON-RPC 通道
    match command {
        Commands::Lsp => yaoxiang::util::logger::init_lsp(),
        _ => match args.log_level {
            Some(level) => yaoxiang::util::logger::init_with_level(level.into()),
            None => yaoxiang::util::logger::init_cli(),
        },
    }

    if args.verbose {
        info!("YaoXiang version: {}", VERSION);
        info!("Host: {}", std::env::consts::OS);
    }

    match command {
        Commands::Run { file } => {
            run_file_with_diagnostics(&file)?;
        }
        Commands::Eval { code } => {
            run(&code).context("Failed to evaluate code")?;
        }
        Commands::Check {
            paths,
            json,
            watch,
            color,
            no_progress,
        } => {
            let use_colors = match color {
                ColorChoice::Always => true,
                ColorChoice::Never => false,
                ColorChoice::Auto => std::io::stderr().is_terminal(),
            };

            if watch {
                run_check_watch(paths, json, use_colors, no_progress)?;
            } else {
                let error_count = run_check_once(&paths, json, use_colors, no_progress)?;
                if error_count > 0 {
                    ::std::process::exit(1);
                }
            }
        }
        Commands::Format {
            file,
            check,
            write,
            stdout: _,
            indent,
            line_width,
            use_tabs,
            single_quote,
        } => {
            // 构建格式化选项
            let mut options = yaoxiang::formatter::FormatOptions::default();
            if let Some(w) = indent {
                options.indent_width = w;
            }
            if let Some(lw) = line_width {
                options.line_width = lw;
            }
            if use_tabs {
                options.use_tabs = true;
            }
            if single_quote {
                options.single_quote = true;
            }

            // 收集需要格式化的文件
            let files = collect_yx_files(&file)?;
            if files.is_empty() {
                eprintln!("No .yx files found at: {}", file.display());
                ::std::process::exit(2);
            }

            let mut needs_formatting = false;

            for f in &files {
                let source = ::std::fs::read_to_string(f)
                    .with_context(|| format!("Failed to read: {}", f.display()))?;
                match yaoxiang::formatter::format_source(&source, &options) {
                    Ok(formatted) => {
                        if check {
                            if formatted != source {
                                eprintln!("Needs formatting: {}", f.display());
                                needs_formatting = true;
                            }
                        } else if write {
                            if formatted != source {
                                ::std::fs::write(f, &formatted)
                                    .with_context(|| format!("Failed to write: {}", f.display()))?;
                                eprintln!("Formatted: {}", f.display());
                            }
                        } else {
                            // 默认输出到 stdout
                            print!("{}", formatted);
                        }
                    }
                    Err(e) => {
                        eprintln!("Error formatting {}: {}", f.display(), e);
                        ::std::process::exit(2);
                    }
                }
            }

            if check && needs_formatting {
                ::std::process::exit(1);
            }
        }
        Commands::Dump { file } => {
            dump_bytecode(&file).with_context(|| format!("Failed to dump: {}", file.display()))?;
        }
        Commands::Build { file, output } => {
            let output_path = output.unwrap_or_else(|| {
                let mut path = file.clone();
                path.set_extension("42");
                path
            });
            build_bytecode(&file, &output_path)
                .with_context(|| format!("Failed to build: {}", file.display()))?;
        }
        Commands::Explain { code, json, lang } => {
            if let Some(definition) = ErrorCodeDefinition::find(&code) {
                let lang_code = lang
                    .map(Into::<String>::into)
                    .unwrap_or_else(|| "zh".to_string());
                let i18n = I18nRegistry::new(&lang_code);
                let info = i18n.get_info(&code).unwrap_or(ErrorInfo {
                    title: "",
                    help: "",
                    example: None,
                    error_output: None,
                });

                if json {
                    // JSON output
                    #[derive(Serialize)]
                    struct ExplainOutput<'a> {
                        code: &'static str,
                        category: String,
                        title: &'a str,
                        template: &'static str,
                        help: &'a str,
                    }
                    let output = ExplainOutput {
                        code: definition.code,
                        category: definition.category.to_string(),
                        title: info.title,
                        template: definition.message_template,
                        help: info.help,
                    };
                    println!("{}", serde_json::to_string_pretty(&output).unwrap());
                } else {
                    // Human-readable output
                    println!("Error {}", definition.code);
                    println!("Category: {}", definition.category);
                    println!("Title: {}", info.title);
                    println!("Message Template: {}", definition.message_template);
                    if !info.help.is_empty() {
                        println!("Help: {}", info.help);
                    }
                }
            } else {
                eprintln!("Unknown error code: {}", code);
                std::process::exit(1);
            }
        }
        Commands::Version => {
            info!("{} {}", NAME, VERSION);
        }
        Commands::Repl { tui } => {
            if tui {
                // TUI REPL mode explicitly requested but not available
                tracing::error!("TUI REPL mode (`--tui`) is not implemented in this build.");
                tracing::info!("You can run non-interactive programs with 'yaoxiang run <file>'.");
            } else {
                // Non-TUI REPL mode not available
                tracing::error!("REPL mode is currently not available in this build.");
                tracing::info!("Use 'yaoxiang run <file>' to execute a YaoXiang source file.");
            }
            // Exit with a non-zero status so callers know the command failed.
            std::process::exit(1);
        }
        Commands::Init { name } => {
            package::commands::init::exec(&name).context("Failed to initialize project")?;
        }
        Commands::Add { dep, version, dev } => {
            package::commands::add::exec(&dep, version.as_deref(), dev)
                .context("Failed to add dependency")?;
        }
        Commands::Rm { dep, dev } => {
            package::commands::rm::exec(&dep, dev).context("Failed to remove dependency")?;
        }
        Commands::Update { pkg } => {
            if let Some(name) = pkg {
                package::commands::update::exec_single_in(
                    &std::env::current_dir().context("Failed to get current directory")?,
                    &name,
                )
                .context("Failed to update dependency")?;
            } else {
                package::commands::update::exec().context("Failed to update dependencies")?;
            }
        }
        Commands::Install => {
            package::commands::install::exec().context("Failed to install dependencies")?;
        }
        Commands::List => {
            package::commands::list::exec().context("Failed to list dependencies")?;
        }
        Commands::Lsp => {
            // LSP 服务器使用 stderr 记录日志（stdout 用于 JSON-RPC 通信）
            yaoxiang::lsp::run_lsp_server().context("LSP server error")?;
        }
    }

    Ok(())
}

/// 收集目录或文件下的所有 .yx 文件
fn collect_yx_files(path: &std::path::Path) -> Result<Vec<PathBuf>> {
    let mut files = Vec::new();
    if path.is_file() {
        files.push(path.to_path_buf());
    } else if path.is_dir() {
        collect_yx_files_recursive(path, &mut files)?;
    }
    Ok(files)
}

/// 递归收集 .yx 文件
fn collect_yx_files_recursive(
    dir: &std::path::Path,
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

/// 从多个输入路径收集并去重所有 .yx 文件
fn collect_yx_files_from_paths(paths: &[PathBuf]) -> Result<Vec<PathBuf>> {
    let mut files = Vec::new();

    for path in paths {
        if !path.exists() {
            return Err(anyhow::anyhow!("Path does not exist: {}", path.display()));
        }
        files.extend(collect_yx_files(path)?);
    }

    files.sort();
    files.dedup();
    Ok(files)
}

/// 单次执行 check 流程，返回错误数量
fn run_check_once(
    paths: &[PathBuf],
    json: bool,
    use_colors: bool,
    no_progress: bool,
) -> Result<usize> {
    let files = collect_yx_files_from_paths(paths)?;
    if files.is_empty() {
        eprintln!("No .yx files found in provided paths");
        ::std::process::exit(2);
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

fn output_check_json(result: &yaoxiang::util::diagnostic::CheckResult) -> Result<()> {
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
                .unwrap_or_else(|_| serde_json::json!({}));

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

fn is_yx_event(event: &notify::Event) -> bool {
    event
        .paths
        .iter()
        .any(|p| p.extension().map(|ext| ext == "yx").unwrap_or(false))
}

fn run_check_watch(
    paths: Vec<PathBuf>,
    json: bool,
    use_colors: bool,
    no_progress: bool,
) -> Result<()> {
    use notify::{Config, RecommendedWatcher, RecursiveMode, Watcher};
    use std::sync::mpsc;
    use std::time::{Duration, Instant};

    run_check_once(&paths, json, use_colors, no_progress)?;

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

        if !is_yx_event(&event) {
            continue;
        }

        // 简单防抖：窗口内持续接收事件，直到静默再触发一次检查。
        let mut deadline = Instant::now() + Duration::from_millis(250);
        while Instant::now() < deadline {
            match rx.recv_timeout(Duration::from_millis(50)) {
                Ok(Ok(next_event)) if is_yx_event(&next_event) => {
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

        let error_count = run_check_once(&paths, json, use_colors, no_progress)?;
        if !no_progress {
            eprintln!("Last run: {} error(s)", error_count);
        }
    }

    Ok(())
}
