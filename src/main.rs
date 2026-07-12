//! YaoXiang Programming Language - CLI

use anyhow::{Context, Result};
use clap::{Parser, Subcommand, ValueEnum};
use std::io::{IsTerminal, Read};
use std::path::PathBuf;
use tracing::info;
use yaoxiang::repl::Repl;
use yaoxiang::formatter::run_format_command;
use yaoxiang::{dump_bytecode, NAME, VERSION};
use yaoxiang::util::diagnostic::{
    render_explain_output, run_check_command_once, run_check_watch_command,
    run_file_with_diagnostics,
};
use yaoxiang::util::i18n::set_lang_from_string;
use yaoxiang::util::logger::LogLevel;
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

        /// Generate debug info for runtime errors (spans/source mapping)
        #[arg(long)]
        debug_info: bool,

        /// Runtime mode (embedded, standard, full)
        #[arg(long, default_value = "embedded")]
        runtime: String,

        /// Number of worker threads (0 = auto)
        #[arg(long, default_value = "0")]
        workers: usize,
    },

    /// Evaluate YaoXiang code (use '-' to read from stdin)
    Eval {
        /// Code to evaluate
        #[arg(value_name = "CODE")]
        code: String,
    },

    /// Check source file for errors (type checking)
    Check {
        /// Source file(s) or directory path(s) to check
        #[arg(value_name = "PATH", num_args = 0..)]
        paths: Vec<PathBuf>,

        /// Exclude file(s) or directory path(s) from check and watch
        #[arg(long = "exclude", value_name = "PATH", num_args = 1..)]
        exclude: Vec<PathBuf>,

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

        /// Dry-run mode: show formatting diff without writing
        #[arg(short = 'n', long)]
        dry_run: bool,

        /// Write formatted output back to file(s) in place
        #[arg(short = 'w', long)]
        write: bool,

        /// Skip post-format verification (performance optimization)
        #[arg(long)]
        no_verify: bool,

        /// Output to stdout (default when neither --dry-run nor --write)
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

        /// Embed debug section into .42 (sources + ip->span mapping)
        #[arg(long)]
        debug_info: bool,
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
        /// Project name (optional; uses current directory name if omitted)
        #[arg(value_name = "NAME")]
        name: Option<String>,

        /// Create a library project instead of a binary project
        #[arg(long)]
        lib: bool,
    },

    /// Create a new YaoXiang project directory
    New {
        /// Project name
        #[arg(value_name = "NAME")]
        name: String,

        /// Create a library project instead of a binary project
        #[arg(long)]
        lib: bool,
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
    Lsp {
        /// Enable debug mode (show debug! macro output)
        #[arg(long)]
        debug: bool,
    },
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
    match &command {
        Commands::Lsp { debug } => {
            if *debug {
                // LSP 模式下必须使用 stderr，debug 仅提升日志级别。
                yaoxiang::util::logger::init_lsp_with_level(LogLevel::Debug);
            } else {
                yaoxiang::util::logger::init_lsp();
            }
        }
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
        Commands::Run {
            file,
            debug_info,
            runtime,
            workers,
        } => {
            // Load project config for runtime settings
            let project_config = {
                let config_path = std::path::PathBuf::from("yaoxiang.toml");
                if config_path.exists() {
                    let content = std::fs::read_to_string(&config_path).unwrap_or_default();
                    toml::from_str::<yaoxiang::util::config::ProjectConfig>(&content)
                        .unwrap_or_default()
                } else {
                    yaoxiang::util::config::ProjectConfig::default()
                }
            };

            // CLI args override project config
            let runtime_mode = if runtime != "embedded" {
                runtime.clone()
            } else {
                project_config.runtime.mode.clone()
            };
            let workers = if workers > 0 {
                workers
            } else if project_config.runtime.workers > 0 {
                project_config.runtime.workers
            } else {
                0 // 0 = auto-detect
            };

            run_file_with_diagnostics(&file, debug_info, &runtime_mode, workers)?;
        }
        Commands::Eval { code } => {
            let source = if code == "-" {
                let mut buf = String::new();
                std::io::stdin()
                    .read_to_string(&mut buf)
                    .context("Failed to read from stdin")?;
                buf
            } else {
                code
            };
            yaoxiang::eval_code(&source).context("Failed to evaluate code")?;
        }
        Commands::Check {
            paths,
            exclude,
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
                run_check_watch_command(paths, exclude, json, use_colors, no_progress)?;
            } else {
                match run_check_command_once(&paths, &exclude, json, use_colors, no_progress) {
                    Ok(error_count) => {
                        if error_count > 0 {
                            ::std::process::exit(1);
                        }
                    }
                    Err(e) => {
                        eprintln!("{}", e);
                        if e.to_string().contains("No .yx files found") {
                            ::std::process::exit(2);
                        }
                        return Err(e);
                    }
                }
            }
        }
        Commands::Format {
            file,
            dry_run,
            write,
            no_verify,
            stdout: _,
            indent,
            line_width,
            use_tabs,
            single_quote,
        } => {
            // 1. Load user config
            let user_config = yaoxiang::util::config::load_user_config().unwrap_or_default();

            // 2. Load project config (current directory)
            let project_config = {
                let config_path = std::path::PathBuf::from("yaoxiang.toml");
                if config_path.exists() {
                    let content = std::fs::read_to_string(&config_path).unwrap_or_default();
                    toml::from_str::<yaoxiang::util::config::ProjectConfig>(&content)
                        .map(|c| c.fmt)
                        .unwrap_or_default()
                } else {
                    yaoxiang::util::config::FmtConfig::default()
                }
            };

            // 3. Start with user config as base
            let mut options = yaoxiang::formatter::FormatOptions::from(&user_config.fmt);

            // 4. Override with project config (only non-None values)
            if let Some(lw) = project_config.line_width {
                options.line_width = lw;
            }
            if let Some(w) = project_config.indent_width {
                options.indent_width = w;
            }
            if let Some(ut) = project_config.use_tabs {
                options.use_tabs = ut;
            }
            if let Some(sq) = project_config.single_quote {
                options.single_quote = sq;
            }
            if let Some(si) = project_config.sort_imports {
                options.sort_imports = si;
            }

            // 5. Override with CLI args (highest priority)
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

            // 6. Apply CLI overrides
            if no_verify {
                options.verify = false;
            }

            let result = match run_format_command(&file, &options, dry_run, write) {
                Ok(result) => result,
                Err(e) => {
                    eprintln!("{}", e);
                    if e.to_string().contains("No .yx files found") {
                        ::std::process::exit(2);
                    }
                    return Err(e);
                }
            };
            if dry_run && result.needs_formatting {
                ::std::process::exit(2);
            }
        }
        Commands::Dump { file } => {
            dump_bytecode(&file).with_context(|| format!("Failed to dump: {}", file.display()))?;
        }
        Commands::Build {
            file,
            output,
            debug_info,
        } => {
            let output_path = output.unwrap_or_else(|| {
                let mut path = file.clone();
                path.set_extension("42");
                path
            });
            yaoxiang::build_bytecode_with_options(&file, &output_path, debug_info)
                .with_context(|| format!("Failed to build: {}", file.display()))?;
        }
        Commands::Explain { code, json, lang } => {
            let lang_code = lang.map(Into::<String>::into);
            if let Some(output) = render_explain_output(&code, json, lang_code.as_deref())? {
                println!("{}", output);
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
                tracing::error!(
                    "TUI REPL mode is not available. Use 'yaoxiang repl' for the standard REPL."
                );
                std::process::exit(1);
            }
            let mut repl = Repl::new().context("Failed to initialize REPL")?;
            repl.run().context("REPL exited with error")?;
        }
        Commands::Init { name, lib } => {
            let options = package::commands::init::InitOptions { lib };
            match name {
                Some(name) => {
                    package::commands::init::exec(&options, &name)
                        .context("Failed to initialize project")?;
                }
                None => {
                    package::commands::init::exec_here(&options)
                        .context("Failed to initialize project")?;
                }
            }
        }
        Commands::New { name, lib } => {
            let options = package::commands::init::InitOptions { lib };
            package::commands::init::exec(&options, &name).context("Failed to create project")?;
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
        Commands::Lsp { .. } => {
            // LSP 服务器使用 stderr 记录日志（stdout 用于 JSON-RPC 通信）
            yaoxiang::lsp::run_lsp_server().context("LSP server error")?;
        }
    }

    Ok(())
}
