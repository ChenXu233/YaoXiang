//! Cargo xtask for Yaoxiang project tooling
//!
//! Run with: cargo xtask <command>

use std::fs;
use std::path::Path;
use std::process::Command;

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let command = args.get(1).map(|s| s.as_str()).unwrap_or("help");

    match command {
        "bump-version" => bump_version(),
        "install-hook" => install_git_hook(),
        "help" | _ => show_help(),
    }
}

fn bump_version() {
    // Read Cargo.toml
    let cargo_content = fs::read_to_string("Cargo.toml").expect("Failed to read Cargo.toml");

    // Parse and update version
    let lines = cargo_content.lines();
    let mut new_lines: Vec<String> = Vec::new();
    let mut version_updated = false;

    for line in lines {
        if line.trim_start().starts_with("version = ") {
            let version = line
                .trim_start()
                .strip_prefix("version = \"")
                .and_then(|s| s.strip_suffix('\"'))
                .expect("Failed to parse version");

            let parts: Vec<u32> = version.split('.').map(|s| s.parse().unwrap_or(0)).collect();

            if parts.len() >= 3 {
                let new_version = format!("{}.{}.{}", parts[0], parts[1], parts[2] + 1);
                println!("Version: {} -> {}", version, new_version);
                new_lines.push(format!("version = \"{}\"", new_version));
                version_updated = true;
            } else {
                new_lines.push(line.to_string());
            }
        } else {
            new_lines.push(line.to_string());
        }
    }

    if version_updated {
        let new_content = new_lines.join("\n");
        fs::write("Cargo.toml", &new_content).expect("Failed to write Cargo.toml");

        // Stage the change
        Command::new("git")
            .args(&["add", "Cargo.toml"])
            .status()
            .expect("Failed to stage Cargo.toml");

        println!("[OK] Version bumped and staged!");
    }
}

fn install_git_hook() {
    let is_windows = cfg!(target_os = "windows");

    if is_windows {
        // Windows batch script
        let hook_content = r#"@echo off
REM Yaoxiang auto-version-bump hook (Windows version)
REM Installed by cargo xtask install-hook

setlocal EnableDelayedExpansion

set COMMIT_MSG_FILE=%1
set COMMIT_SOURCE=%2
set SHA1=%3

REM Skip merges and amends
if "%COMMIT_SOURCE%"=="merge" goto :eof
if "%COMMIT_SOURCE%"=="squash" goto :eof
if not "%SHA1%"=="" goto :eof

REM Change to project root directory
for /f "delims=" %%i in ('git rev-parse --show-toplevel') do set "PROJECT_ROOT=%%i"
cd /d "%PROJECT_ROOT%"

REM Only bump version if Rust files (.rs) are modified
set "rs_files="
for /f "delims=" %%i in ('git diff --cached --name-only ^| findstr /r "\.rs$"') do set "rs_files=%%i"

if defined rs_files (
    echo Rust files modified, bumping version...
    cargo xtask bump-version
) else (
    echo No Rust files modified, skipping version bump.
)

endlocal
"#;

        let hook_path = Path::new(".git/hooks/prepare-commit-msg.bat");
        fs::write(hook_path, hook_content).expect("Failed to write Windows hook");

        // Also create bash version for Git Bash users
        let bash_hook_content = r#"#!/bin/bash
# Yaoxiang auto-version-bump hook
# Installed by cargo xtask install-hook

set -e

COMMIT_MSG_FILE="$1"
COMMIT_SOURCE="$2"
SHA1="$3"

# Skip merges and amends
if [ "$COMMIT_SOURCE" = "merge" ] || [ "$COMMIT_SOURCE" = "squash" ] || [ -n "$SHA1" ]; then
    exit 0
fi

# Change to project root directory
cd "$(git rev-parse --show-toplevel)"

# Only bump version if Rust files (.rs) are modified
rs_files=$(git diff --cached --name-only | grep -E '\.rs$' || true)

if [ -n "$rs_files" ]; then
    echo "Rust files modified, bumping version..."
    exec cargo xtask bump-version
else
    echo "No Rust files modified, skipping version bump."
fi
"#;

        let bash_hook_path = Path::new(".git/hooks/prepare-commit-msg");
        fs::write(bash_hook_path, bash_hook_content).expect("Failed to write bash hook");

        Command::new("chmod")
            .args(&["+x", ".git/hooks/prepare-commit-msg"])
            .status()
            .ok();

        println!("[OK] Git hook installed at .git/hooks/prepare-commit-msg.bat (Windows CMD)");
        println!("[OK] Also installed .git/hooks/prepare-commit-msg (Git Bash)");
    } else {
        // Unix/Linux/macOS bash script
        let hook_content = r#"#!/bin/bash
# Yaoxiang auto-version-bump hook
# Installed by cargo xtask install-hook

set -e

COMMIT_MSG_FILE="$1"
COMMIT_SOURCE="$2"
SHA1="$3"

# Skip merges and amends
if [ "$COMMIT_SOURCE" = "merge" ] || [ "$COMMIT_SOURCE" = "squash" ] || [ -n "$SHA1" ]; then
    exit 0
fi

# Change to project root directory
cd "$(git rev-parse --show-toplevel)"

# Only bump version if Rust files (.rs) are modified
rs_files=$(git diff --cached --name-only | grep -E '\.rs$' || true)

if [ -n "$rs_files" ]; then
    echo "Rust files modified, bumping version..."
    exec cargo xtask bump-version
else
    echo "No Rust files modified, skipping version bump."
fi
"#;

        let hook_path = Path::new(".git/hooks/prepare-commit-msg");
        fs::write(hook_path, hook_content).expect("Failed to write hook");

        Command::new("chmod")
            .args(&["+x", ".git/hooks/prepare-commit-msg"])
            .status()
            .ok();

        println!("[OK] Git hook installed at .git/hooks/prepare-commit-msg");
    }

    println!("  Version will be bumped automatically when Rust files are committed!");
}

fn show_help() {
    println!(
        r#"Yaoxiang xtask commands:
    
    cargo xtask bump-version    - Bump the patch version in Cargo.toml
    cargo xtask install-hook    - Install git prepare-commit-msg hook
    cargo xtask help            - Show this help message

Git Hook Usage:
    After running 'cargo xtask install-hook', every commit will
    automatically bump the patch version (0.2.6 -> 0.2.7 -> 0.2.8)
    and stage the Cargo.toml change.
    
    Version bump only happens when Rust files (.rs) are modified.
"#
    );
}
