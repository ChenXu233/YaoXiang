//! `yaoxiang test` 运行器 — RFC-036 Phase 1 + Phase 2
//!
//! 测试文件是普通 `.yx` 文件：发现 → 逐文件子进程 `yaoxiang run --debug-info` →
//! exit code 判定 → 汇总报告。进程级隔离，零编译器改动。
//!
//! 标记约定（见 tests/yaoxiang/TEST_STANDARDS.md）：
//! - `// [test:error]: <原因>` — 本文件应编译失败：run 退出码非 0 = PASS
//!   （06-compile-errors 目录用，验证编译器正确报错）
//! - 无标记 — run 退出码 0 = PASS
//!
//! 报告契约（RFC-036 §1）：
//! - 默认输出 = 进度（表头 + per-file 行）+ 报告（FAIL 明细 + 汇总）
//! - `--no-progress` 只抑制进度；FAIL 明细与汇总始终输出——失败不可静默
//! - `--json`：stdout 仅一份 JSON 报告；失败文件附 `exit_code` 与 `stderr`
//!   （ANSI 剥离，CI 取证用），`--verbose` 时全部文件附 `stdout`/`stderr`
//! - `--fail-fast` 首个失败即停；`--list` 每行一个路径，不执行
//!
//! 规范来源：docs/src/design/rfc/accepted/036-test-framework.md §1 CLI 设计 / §5 发现与执行

use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::Instant;

use serde::Serialize;

use crate::util::config::ProjectConfig;
use crate::util::diagnostic::emitter::ansi::strip_ansi;

/// `yaoxiang test` 选项 — RFC-036 §1 CLI 设计
pub struct TestOptions {
    /// 显式路径（优先于配置发现，RFC-036 §5）
    pub paths: Vec<PathBuf>,
    /// `--filter <NAME>`：文件名包含 <NAME> 才执行
    pub filter: Option<String>,
    /// `--fail-fast`：首个失败文件后停止
    pub fail_fast: bool,
    /// `--verbose`：报告每个文件的详细 stdout/stderr
    pub verbose: bool,
    /// `--list`：只列出发现的文件，不执行
    pub list: bool,
    /// `--no-progress`：抑制进度行（表头与 PASS 行），失败明细与汇总保留
    pub no_progress: bool,
    /// `--json`：stdout 输出 JSON 报告（CI 集成）
    pub json: bool,
}

/// 单个测试文件的执行结果（人类报告与 JSON 报告共用）
struct FileResult {
    display: String,
    passed: bool,
    secs: f64,
    exit_code: Option<i32>,
    stdout: String,
    stderr: String,
}

// JSON 报告 — RFC-036 §1 输出格式

#[derive(Serialize)]
struct JsonReport {
    summary: JsonSummary,
    files: Vec<JsonFile>,
}

#[derive(Serialize)]
struct JsonSummary {
    total: usize,
    passed: usize,
    failed: usize,
    skipped: usize,
    time_secs: f64,
}

#[derive(Serialize)]
struct JsonFile {
    file: String,
    passed: bool,
    time_secs: f64,
    #[serde(skip_serializing_if = "Option::is_none")]
    exit_code: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    stdout: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    stderr: Option<String>,
}

/// 运行测试命令，返回失败数（0 = 全部通过或没有发现测试）。
///
/// 发现顺序：显式 paths → `./yaoxiang.toml` 的 `[tool.test].patterns` → 默认
/// `tests/**/*.yx`；之后应用 `--filter`（文件名包含，RFC-036 §5）。
pub fn run_test_command(options: &TestOptions) -> anyhow::Result<usize> {
    let cwd = std::env::current_dir().unwrap_or_default();
    let mut files = discover(&options.paths);
    if let Some(filter) = &options.filter {
        files.retain(|f| {
            f.file_name()
                .is_some_and(|n| n.to_string_lossy().contains(filter.as_str()))
        });
    }

    if files.is_empty() {
        if options.json {
            println!("{}", empty_json_report());
        } else {
            println!("No tests found.");
        }
        return Ok(0);
    }

    if options.list {
        for file in &files {
            println!("{}", display_path(file, &cwd));
        }
        return Ok(0);
    }

    let exe = std::env::current_exe()
        .map_err(|e| anyhow::anyhow!("Failed to locate current executable: {}", e))?;

    if !options.json && !options.no_progress {
        println!("Running {} test files...\n", files.len());
    }

    let total_start = Instant::now();
    let mut results: Vec<FileResult> = Vec::new();
    let mut stopped_early = false;

    for file in &files {
        let display = display_path(file, &cwd);
        let expect_error = file_marks_expected_error(file);
        let start = Instant::now();
        let output = Command::new(&exe)
            .arg("run")
            .arg(file)
            .arg("--debug-info")
            .output();
        let secs = start.elapsed().as_secs_f64();

        let result = match output {
            // [test:error] 标记文件：编译错误如预期 → PASS；
            // 没报错（退出码 0）→ FAIL（该报错没报）
            Ok(out) => FileResult {
                display,
                passed: if expect_error {
                    !out.status.success()
                } else {
                    out.status.success()
                },
                secs,
                exit_code: out.status.code(),
                stdout: String::from_utf8_lossy(&out.stdout).to_string(),
                stderr: String::from_utf8_lossy(&out.stderr).to_string(),
            },
            Err(e) => FileResult {
                display,
                passed: false,
                secs,
                exit_code: None,
                stdout: String::new(),
                stderr: format!("failed to spawn test process: {e}"),
            },
        };

        if !options.json {
            print_file_result(&result, options);
        }
        let failed_now = !result.passed;
        results.push(result);
        if failed_now && options.fail_fast {
            stopped_early = true;
            break;
        }
    }

    let passed = results.iter().filter(|r| r.passed).count();
    let failed = results.len() - passed;
    let total_secs = total_start.elapsed().as_secs_f64();

    if options.json {
        let report = JsonReport {
            summary: JsonSummary {
                total: results.len(),
                passed,
                failed,
                skipped: 0,
                time_secs: round_secs(total_secs),
            },
            files: results
                .iter()
                .map(|r| json_file(r, options.verbose))
                .collect(),
        };
        println!("{}", serde_json::to_string_pretty(&report)?);
    } else {
        if stopped_early {
            println!("\nStopped by --fail-fast after first failure.");
        }
        println!(
            "\nResults: {} {} passed, {} {} failed, 0 skipped ({:.3}s)",
            passed,
            plural_files(passed),
            failed,
            plural_files(failed),
            total_secs
        );
    }
    Ok(failed)
}

/// 输出单个文件的结果行与失败明细。
///
/// PASS 行受 `--no-progress` 抑制；FAIL 行与诊断明细始终输出——失败不可静默。
/// `--verbose` 额外显示每个文件捕获的 stdout（PASS 文件的 stderr 一并显示）。
fn print_file_result(
    result: &FileResult,
    options: &TestOptions,
) {
    if result.passed && options.no_progress {
        return;
    }
    let status = if result.passed { "PASS" } else { "FAIL" };
    println!(
        "{} {} {} ({:.3}s)",
        result.display,
        progress_dots(&result.display),
        status,
        result.secs
    );
    if !result.passed && !result.stderr.trim().is_empty() {
        for line in strip_ansi(&result.stderr).trim().lines() {
            println!("      {line}");
        }
    }
    if options.verbose {
        if !result.stdout.trim().is_empty() {
            println!("      [stdout]");
            for line in strip_ansi(&result.stdout).trim().lines() {
                println!("      {line}");
            }
        }
        if result.passed && !result.stderr.trim().is_empty() {
            println!("      [stderr]");
            for line in strip_ansi(&result.stderr).trim().lines() {
                println!("      {line}");
            }
        }
    }
}

/// RFC-036 §1 默认输出的点线填充：文件名 + 点线 + 结果对齐。
fn progress_dots(display: &str) -> String {
    let dots = 44usize.saturating_sub(display.chars().count()).max(4);
    ".".repeat(dots)
}

/// RFC-036 §1 汇总行的单复数（示例："2 files passed, 1 file failed"）。
fn plural_files(count: usize) -> &'static str {
    if count == 1 {
        "file"
    } else {
        "files"
    }
}

/// JSON 报告条目：失败文件附 `exit_code` 与 `stderr`（CI 取证）；
/// `--verbose` 时全部文件附 `stdout`/`stderr`。
fn json_file(
    result: &FileResult,
    verbose: bool,
) -> JsonFile {
    let (stdout, stderr) = if verbose {
        (
            Some(strip_ansi(&result.stdout)),
            Some(strip_ansi(&result.stderr)),
        )
    } else if result.passed {
        (None, None)
    } else {
        (None, Some(strip_ansi(&result.stderr)))
    };
    JsonFile {
        file: result.display.clone(),
        passed: result.passed,
        time_secs: round_secs(result.secs),
        exit_code: if result.passed {
            None
        } else {
            result.exit_code
        },
        stdout,
        stderr,
    }
}

/// 零发现时的 JSON 报告（CI 消费者无需特判空目录）。
fn empty_json_report() -> String {
    serde_json::to_string_pretty(&JsonReport {
        summary: JsonSummary {
            total: 0,
            passed: 0,
            failed: 0,
            skipped: 0,
            time_secs: 0.0,
        },
        files: Vec::new(),
    })
    .expect("JSON report serialization cannot fail")
}

/// 时间戳保留 3 位小数（RFC-036 §1 示例：0.006）。
fn round_secs(secs: f64) -> f64 {
    (secs * 1000.0).round() / 1000.0
}

/// 相对 cwd 的显示路径（cwd 外的路径原样显示）。
fn display_path(
    path: &Path,
    cwd: &Path,
) -> String {
    path.strip_prefix(cwd).unwrap_or(path).display().to_string()
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
