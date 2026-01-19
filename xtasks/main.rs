//! Minimal xtask for YaoXiang: bump-version and install-hooks
use std::fs;
use std::path::Path;
use std::process::Command;

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let command = args.get(1).map(|s| s.as_str()).unwrap_or("help");

    match command {
        "bump-version" => bump_version(),
        "install-hooks" => install_hooks(),
        "help" => show_help(),
        _ => show_help(),
    }
}

fn show_help() {
    println!(
        r#"Yaoxiang xtask commands:

    cargo xtask bump-version    - Bump the patch version in Cargo.toml
    cargo xtask install-hooks   - Write .githooks/* hooks and set core.hooksPath

"#
    );
}

fn bump_version() {
    // 防止循环触发：同一个 commit message 流程中多次调用
    if std::env::var("YAX_VERSION_BUMPED").is_ok() {
        println!("[SKIP] Version already bumped in this cycle");
        return;
    }
    std::env::set_var("YAX_VERSION_BUMPED", "1");

    // 获取项目根目录
    let root = Command::new("git")
        .args(["rev-parse", "--show-toplevel"])
        .output()
        .expect("Failed to run git rev-parse")
        .stdout;
    let root = String::from_utf8_lossy(&root).trim().to_string();

    // 检查是否有 .rs 文件被修改
    // 注意：pre-commit 运行时文件还未 staged，所以用 git diff 而不是 git diff --cached
    let rs_files = Command::new("git")
        .args(["diff", "--name-only"])
        .output()
        .expect("Failed to run git diff")
        .stdout;

    let has_rs_files = String::from_utf8_lossy(&rs_files)
        .lines()
        .any(|f| f.ends_with(".rs"));

    if !has_rs_files {
        println!("[SKIP] No .rs files changed, skipping version bump");
        return;
    }

    let cargo_path = Path::new(&root).join("Cargo.toml");
    let content = fs::read_to_string(&cargo_path).expect("Failed to read Cargo.toml");
    let mut lines: Vec<String> = Vec::new();
    let mut updated = false;

    for line in content.lines() {
        if line.trim_start().starts_with("version = ") && !updated {
            if let Some(start) = line.find('"') {
                if let Some(end) = line[start + 1..].find('"') {
                    let ver = &line[start + 1..start + 1 + end];
                    let parts: Vec<&str> = ver.split('.').collect();
                    if parts.len() >= 3 {
                        let major: u64 = parts[0].parse().unwrap_or(0);
                        let minor: u64 = parts[1].parse().unwrap_or(0);
                        let patch: u64 = parts[2].parse().unwrap_or(0);
                        let new = format!("version = \"{}.{}.{}\"", major, minor, patch + 1);
                        println!(
                            "Bumping version: {} -> {}.{}.{}",
                            ver,
                            major,
                            minor,
                            patch + 1
                        );
                        lines.push(new);
                        updated = true;
                        continue;
                    }
                }
            }
        }
        lines.push(line.to_string());
    }

    if updated {
        let new_content = lines.join("\n");
        fs::write(&cargo_path, new_content).expect("Failed to write Cargo.toml");
        // Stage the main Cargo.toml
        let cargo_to_add = Path::new(&root).join("Cargo.toml");
        let _ = Command::new("git")
            .args(["add", &cargo_to_add.to_string_lossy()])
            .status();
        println!("[OK] main Cargo.toml bumped and staged");
    } else {
        println!("[SKIP] No version line found or already updated");
    }
}

fn install_hooks() {
    // Write .githooks scripts and set core.hooksPath
    let root = Path::new(".");
    let hooks_dir = root.join(".githooks");
    if !hooks_dir.exists() {
        fs::create_dir_all(&hooks_dir).expect("Failed to create .githooks directory");
    }

    // Bash hook
    let bash = r#"#!/usr/bin/env bash
set -euo pipefail

# prepare-commit-msg: bump Cargo.toml when Rust files are staged
COMMIT_MSG_FILE="$1"
COMMIT_SOURCE="$2"
SHA1="$3"

if [ "$COMMIT_SOURCE" = "merge" ] || [ "$COMMIT_SOURCE" = "squash" ] || [ -n "$SHA1" ]; then
    exit 0
fi

cd "$(git rev-parse --show-toplevel)"
rs_files=$(git diff --cached --name-only | grep -E '\.rs$' || true)
if [ -n "$rs_files" ]; then
  echo "Rust files staged -> bumping Cargo.toml"
  export YAX_VERSION_BUMPED=1
  exec cargo xtask bump-version
fi
"#;

    fs::write(hooks_dir.join("prepare-commit-msg"), bash).expect("Failed to write bash hook");

    // PowerShell hook
    let ps = r#"Param($CommitMsgFile, $CommitSource, $Sha1)
# Skip merges/squashes/amends
if ($CommitSource -eq 'merge' -or $CommitSource -eq 'squash' -or -not [string]::IsNullOrEmpty($Sha1)) { exit 0 }
Set-Location (git rev-parse --show-toplevel)
$rs = git diff --cached --name-only | Select-String -Pattern '\.rs$' -Quiet
if ($rs) {
  Write-Host 'Rust files staged -> bumping Cargo.toml'
  $env:YAX_VERSION_BUMPED = "1"
  & cargo xtask bump-version
}
"#;

    fs::write(hooks_dir.join("prepare-commit-msg.ps1"), ps).expect("Failed to write ps1 hook");

    // CMD shim
    let bat = r#"@echo off
for /f "delims=" %%i in ('git rev-parse --show-toplevel') do set "ROOT=%%i"
pushd "%ROOT%"
set "COMMIT_SOURCE=%2"
set "SHA1=%3"
if "%COMMIT_SOURCE%"=="merge" goto :EOF
if "%COMMIT_SOURCE%"=="squash" goto :EOF
if not "%SHA1%"=="" goto :EOF
for /f "delims=" %%f in ('git diff --cached --name-only ^| findstr /r "\.rs$"') do set "RS_FOUND=1"
if defined RS_FOUND (
  set "YAX_VERSION_BUMPED=1"
  powershell -NoProfile -ExecutionPolicy Bypass -Command "& '%CD%\.githooks\prepare-commit-msg.ps1' %*"
)
popd
exit /b 0
"#;

    fs::write(hooks_dir.join("prepare-commit-msg.bat"), bat).expect("Failed to write bat hook");

    // Try to set executable on Unix
    let _ = Command::new("chmod")
        .args(["+x", ".githooks/prepare-commit-msg"])
        .status();

    // Configure core.hooksPath locally
    let status = Command::new("git")
        .args(["config", "core.hooksPath", ".githooks"])
        .status();
    match status {
        Ok(s) if s.success() => println!("[OK] core.hooksPath set to .githooks"),
        _ => println!("[WARN] failed to set core.hooksPath (you can run: git config core.hooksPath .githooks)")
    }

    println!("[OK] Hooks written to .githooks (prepare-commit-msg, .ps1, .bat)");
}
