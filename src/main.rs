//! YaoXiang Programming Language - CLI

use anyhow::{Context, Result};
use clap::{Parser, Subcommand};
use std::path::PathBuf;
use yaoxiang::{run, run_file, NAME, VERSION};

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

    /// Print version information
    Version,
}

fn main() -> Result<()> {
    let args = Args::parse();

    if args.verbose {
        eprintln!("YaoXiang version: {}", VERSION);
        eprintln!("Host: {}", std::env::consts::OS);
    }

    match args.command {
        Commands::Run { file } => {
            run_file(&file).with_context(|| format!("Failed to run: {}", file.display()))?;
        }
        Commands::Eval { code } => {
            run(&code).context("Failed to evaluate code")?;
        }
        Commands::Check { file } => {
            // TODO: Implement type checking without execution
            run_file(&file).with_context(|| format!("Failed to check: {}", file.display()))?;
            eprintln!("Check passed!");
        }
        Commands::Format { file: _, check: _ } => {
            // TODO: Implement formatter
            eprintln!("Formatter not implemented yet");
        }
        Commands::Version => {
            println!("{} {}", NAME, VERSION);
        }
    }

    Ok(())
}
