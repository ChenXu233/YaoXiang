//! YaoXiang Programming Language - CLI

use anyhow::{Context, Result};
use clap::{Parser, Subcommand, ValueEnum};
use std::path::PathBuf;
use tracing::info;
use yaoxiang::{build_bytecode, dump_bytecode, run, run_file, NAME, VERSION};
use yaoxiang::util::logger::LogLevel;
use yaoxiang::util::i18n::Lang;

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
}

impl From<LangArg> for Lang {
    fn from(lang: LangArg) -> Self {
        match lang {
            LangArg::En => Lang::En,
            LangArg::Zh => Lang::Zh,
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
    command: Commands,

    /// Enable verbose output
    #[arg(short, long)]
    verbose: bool,

    /// Set log level (debug, info, warn, error)
    #[arg(short, long, value_enum)]
    log_level: Option<LogLevelArg>,

    /// Set language (en, zh)
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

    /// Evaluate YaoXiang code from command line
    Eval {
        /// Code to evaluate
        #[arg(value_name = "CODE")]
        code: String,
    },

    /// Check source file for errors (type checking)
    Check {
        /// Source file to check
        #[arg(value_name = "FILE")]
        file: PathBuf,
    },

    /// Format source file
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

    /// Print version information
    Version,
}

fn main() -> Result<()> {
    let args = Args::parse();

    // Set language first (before logger init)
    let lang = args.lang.map(Into::into).unwrap_or_else(|| {
        std::env::var("YAOXIANG_LANG")
            .ok()
            .as_deref()
            .map(Into::into)
            .unwrap_or(Lang::En)
    });
    yaoxiang::util::logger::set_lang(lang);

    // Initialize logger with specified level
    match args.log_level {
        Some(level) => yaoxiang::util::logger::init_with_level(level.into()),
        None => yaoxiang::util::logger::init_cli(),
    }

    if args.verbose {
        info!("YaoXiang version: {}", VERSION);
        info!("Host: {}", std::env::consts::OS);
    }

    match args.command {
        Commands::Run { file } => {
            info!("Run command received: {:?}", file);
            run_file(&file).with_context(|| format!("Failed to run: {}", file.display()))?;
            info!("Run command completed successfully!");
        }
        Commands::Eval { code } => {
            info!("Eval command received");
            run(&code).context("Failed to evaluate code")?;
            info!("Eval command completed successfully!");
        }
        Commands::Check { file } => {
            // TODO: Implement type checking without execution
            info!("Check command received: {:?}", file);
            run_file(&file).with_context(|| format!("Failed to check: {}", file.display()))?;
            info!("Check passed!");
        }
        Commands::Format { file: _, check: _ } => {
            // TODO: Implement formatter
            eprintln!("Formatter not implemented yet");
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
        Commands::Version => {
            println!("{} {}", NAME, VERSION);
        }
    }

    Ok(())
}
