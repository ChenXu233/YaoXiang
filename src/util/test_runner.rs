//! `yaoxiang test` 运行器 — RFC-036 Phase 1
//!
//! 测试文件是普通 `.yx` 文件：发现 → 逐文件子进程 `yaoxiang run --debug-info` →
//! exit code 判定 → 汇总报告。进程级隔离，零编译器改动。
//!
//! 标记约定（见 tests/yaoxiang/TEST_STANDARDS.md）：
//! - `// [test:error]: <原因>` — 本文件应编译失败：run 退出码非 0 = PASS
//!   （06-compile-errors 目录用，验证编译器正确报错）
//! - 无标记 — run 退出码 0 = PASS
//!
//! 规范来源：docs/src/design/rfc/accepted/036-test-framework.md §1 CLI 设计 / §5 发现与执行

use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::Instant;

use crate::util::config::ProjectConfig;
use crate::util::diagnostic::emitter::ansi::strip_ansi;

/// 运行测试命令，返回失败数（0 = 全部通过或没有发现测试）。
///
/// 发现顺序：显式 paths → `./yaoxiang.toml` 的 `[tool.test].patterns` → 默认 `tests/**/*.yx`。
pub fn run_test_command(paths: &[PathBuf]) -> anyhow::Result<usize> {
    let files = discover(paths);
    if files.is_empty() {
        println!("No tests found.");
        return Ok(0);
    }

    println!("Running {} test file(s)...\n", files.len());

    let exe = std::env::current_exe()
        .map_err(|e| anyhow::anyhow!("Failed to locate current executable: {}", e))?;
    let cwd = std::env::current_dir().unwrap_or_default();

    let mut passed = 0usize;
    let mut failed = 0usize;
    let total_start = Instant::now();

    for file in &files {
        let display = file
            .strip_prefix(&cwd)
            .unwrap_or(file)
            .display()
            .to_string();

        let start = Instant::now();
        let expect_error = file_marks_expected_error(file);
        let output = Command::new(&exe)
            .arg("run")
            .arg(file)
            .arg("--debug-info")
            .output();
        let secs = start.elapsed().as_secs_f64();

        match output {
            // [test:error] 标记文件：编译错误如预期 → PASS；
            // 没报错（退出码 0）→ FAIL（该报错没报）
            Ok(out) if expect_error && !out.status.success() => {
                passed += 1;
                println!("{:<50} PASS ({:.3}s)", display, secs);
            }
            Ok(out) if expect_error => {
                failed += 1;
                println!("{:<50} FAIL ({:.3}s)", display, secs);
                println!("      [test:error] 标记文件但编译通过了（应报编译错误）");
                let stderr = strip_ansi(&String::from_utf8_lossy(&out.stderr));
                for line in stderr.trim().lines() {
                    println!("      {}", line);
                }
            }
            Ok(out) if out.status.success() => {
                passed += 1;
                println!("{:<50} PASS ({:.3}s)", display, secs);
            }
            Ok(out) => {
                failed += 1;
                println!("{:<50} FAIL ({:.3}s)", display, secs);
                // 子进程 stderr 是渲染后的诊断（含 ANSI），剥离后缩进展示
                let stderr = strip_ansi(&String::from_utf8_lossy(&out.stderr));
                for line in stderr.trim().lines() {
                    println!("      {}", line);
                }
            }
            Err(e) => {
                failed += 1;
                println!("{:<50} FAIL ({:.3}s)", display, secs);
                println!("      failed to spawn test process: {}", e);
            }
        }
    }

    println!(
        "\nResults: {} passed, {} failed, 0 skipped ({:.3}s)",
        passed,
        failed,
        total_start.elapsed().as_secs_f64()
    );
    Ok(failed)
}

/// 检测文件头注释是否含 `[test:error]` 标记（前 16 行内）。
/// 06-compile-errors 目录的文件声明“本文件应编译失败”，
/// runner 据此反向判定（run 退出码非 0 = PASS）。
fn file_marks_expected_error(path: &Path) -> bool {
    let Ok(content) = std::fs::read_to_string(path) else {
        return false;
    };
    content.lines().take(16).any(|l| l.contains("[test:error]"))
}

/// 发现测试文件：显式路径优先，否则读配置，最终按 pattern 展开并排序去重。
fn discover(paths: &[PathBuf]) -> Vec<PathBuf> {
    let patterns: Vec<String> = if paths.is_empty() {
        load_config_patterns()
    } else {
        paths.iter().map(|p| p.display().to_string()).collect()
    };

    let mut files = Vec::new();
    for pattern in &patterns {
        collect_pattern(pattern, &mut files);
    }
    files.sort();
    files.dedup();
    files
}

/// 读取 `./yaoxiang.toml` 的 `[tool.test].patterns`，缺失或解析失败回退默认值。
fn load_config_patterns() -> Vec<String> {
    let config_path = PathBuf::from("yaoxiang.toml");
    if config_path.exists() {
        let content = std::fs::read_to_string(&config_path).unwrap_or_default();
        if let Ok(config) = toml::from_str::<ProjectConfig>(&content) {
            return config.tool.test.patterns;
        }
    }
    crate::util::config::TestConfig::default().patterns
}

/// ponytail: pattern 只支持两种形式——字面路径（文件或目录）与 `root/**/*.yx`。
/// 完整 glob 语法（`*`/`?`/字符类）等需要时再引 glob crate。
fn collect_pattern(
    pattern: &str,
    out: &mut Vec<PathBuf>,
) {
    if let Some((root, _)) = pattern.split_once("/**/") {
        collect_yx_recursive(Path::new(root), out);
        return;
    }
    let path = Path::new(pattern);
    if path.is_dir() {
        collect_yx_recursive(path, out);
    } else if path.extension().is_some_and(|e| e == "yx") && path.exists() {
        out.push(path.to_path_buf());
    }
}

/// 递归收集目录下全部 .yx 文件（目录不可读时静默跳过——发现不是错误边界）。
fn collect_yx_recursive(
    dir: &Path,
    out: &mut Vec<PathBuf>,
) {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            collect_yx_recursive(&path, out);
        } else if path.extension().is_some_and(|e| e == "yx") {
            out.push(path);
        }
    }
}
