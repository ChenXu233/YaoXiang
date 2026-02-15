//! YaoXiang Programming Language - CLI

use anyhow::{Context, Result};
use clap::{Parser, Subcommand, ValueEnum};
use serde::Serialize;
use std::path::PathBuf;
use tracing::info;
use yaoxiang::{build_bytecode, dump_bytecode, run, NAME, VERSION};
use yaoxiang::util::logger::LogLevel;
use yaoxiang::util::i18n::{set_lang_from_string, t_cur_simple, MSG};
use yaoxiang::util::diagnostic::{
    run_file_with_diagnostics, check_file_with_diagnostics, ErrorCodeDefinition, I18nRegistry,
    ErrorInfo,
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
        /// Source file to check
        #[arg(value_name = "FILE")]
        file: PathBuf,
    },

    /// Format source file (unsupported yet)
    Format {
        /// Source file to format
        #[arg(value_name = "FILE")]
        file: PathBuf,

        /// Output to stdout instead of modifying file
        #[arg(short, long)]
        check: bool,
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

    // Initialize logger with specified level
    match args.log_level {
        Some(level) => yaoxiang::util::logger::init_with_level(level.into()),
        None => yaoxiang::util::logger::init_cli(),
    }

    if args.verbose {
        info!("YaoXiang version: {}", VERSION);
        info!("Host: {}", std::env::consts::OS);
    }

    // 如果没有提供子命令，启动 TUI REPL
    let command = args.command.unwrap_or(Commands::Repl { tui: false });
    match command {
        Commands::Run { file } => {
            run_file_with_diagnostics(&file)?;
        }
        Commands::Eval { code } => {
            run(&code).context("Failed to evaluate code")?;
        }
        Commands::Check { file } => {
            check_file_with_diagnostics(&file)?;
        }
        Commands::Format { file: _, check: _ } => {
            // TODO: Implement formatter
            tracing::warn!("{}", t_cur_simple(MSG::FormatterNotImplemented));
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
    }

    Ok(())
}
