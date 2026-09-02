//! `yaoxiang test` 运行器 — RFC-036 Phase 1 + Phase 2
//!
//! 测试文件是普通 `.yx` 文件：发现 → 逐文件子进程 `yaoxiang run --debug-info` →
//! exit code 判定 → 汇总报告。进程级隔离，零编译器改动。
//!
//! 标记约定（见 tests/yaoxiang/TEST_STANDARDS.md）：
//! - `// [test:error]: <原因>` — 本文件应编译失败：run 退出码非 0 = PASS
//!   （06-compile-errors 目录用，验证编译器正确报错）；头部再带
//!   `// 预期: 编译错误 EXXXX` 时进一步实际比对输出中的 `[EXXXX]`，码不符 = FAIL（§8.2）
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
        let (expect_error, expected_codes) = expected_error_spec(file);
        let start = Instant::now();
        let output = Command::new(&exe)
            .arg("run")
            .arg(file)
            .arg("--debug-info")
            .output();
        let secs = start.elapsed().as_secs_f64();

        let result = match output {
            // [test:error] 标记文件：编译/运行错误如预期 → PASS；
            // 没报错（退出码 0）→ FAIL（该报错没报）
            Ok(out) => {
                let passed = if expect_error {
                    !out.status.success()
                } else {
                    out.status.success()
                };
                let stderr = String::from_utf8_lossy(&out.stderr).to_string();
                // RFC-036 §8.2 结构化比对：带预期码的文件，码须实际出现在输出中，
                // 不符 = FAIL（此前纯 exit≠0 判定——报错码不对也 PASS）
                let (passed, stderr) = if expect_error && passed && !expected_codes.is_empty() {
                    match check_expected_codes(&expected_codes, &stderr) {
                        Ok(()) => (passed, stderr),
                        Err(note) => (false, format!("{note}\n{stderr}")),
                    }
                } else {
                    (passed, stderr)
                };
                FileResult {
                    display,
                    passed,
                    secs,
                    exit_code: out.status.code(),
                    stdout: String::from_utf8_lossy(&out.stdout).to_string(),
                    stderr,
                }
            }
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

/// 解析文件头部（前 16 行）的负向测试标记（RFC-036 §8.2）。
/// 返回 (是否 `[test:error]`，预期错误码列表)。预期行形态 `// 预期: 编译错误 EXXXX`
/// 或 `// 预期: 运行时错误 EXXXX`（可带尾注）；无码文件回退 exit≠0 反向判定。
fn expected_error_spec(path: &Path) -> (bool, Vec<String>) {
    let Ok(content) = std::fs::read_to_string(path) else {
        return (false, Vec::new());
    };
    let mut is_error_test = false;
    let mut codes = Vec::new();
    for line in content.lines().take(16) {
        if line.contains("[test:error]") {
            is_error_test = true;
        }
        if let Some(code) = extract_expected_code(line) {
            if !codes.contains(&code) {
                codes.push(code);
            }
        }
    }
    (is_error_test, codes)
}

/// 提取一行中 `预期:` 之后的首个 `EXXXX` 码（无码形态返回 None）。
fn extract_expected_code(line: &str) -> Option<String> {
    let rest = line.split_once("预期:")?.1;
    let bytes = rest.as_bytes();
    for i in 0..bytes.len().saturating_sub(4) {
        if bytes[i] == b'E' && bytes[i + 1..i + 5].iter().all(|b| b.is_ascii_digit()) {
            return Some(rest[i..i + 5].to_string());
        }
    }
    None
}

/// RFC-036 §8.2 结构化预期码比对：每个预期码都须以 `[EXXXX]` 形态出现在
/// 编译器/运行时输出中，任一缺失即判 FAIL，并附实际出现的码以便定位。
fn check_expected_codes(
    expected: &[String],
    output: &str,
) -> Result<(), String> {
    let emitted = emitted_error_codes(&strip_ansi(output));
    let missing: Vec<&String> = expected.iter().filter(|c| !emitted.contains(c)).collect();
    if missing.is_empty() {
        return Ok(());
    }
    let missing = missing
        .iter()
        .map(|s| s.as_str())
        .collect::<Vec<_>>()
        .join(", ");
    let emitted_desc = if emitted.is_empty() {
        "无".to_string()
    } else {
        emitted.join(", ")
    };
    Err(format!(
        "[test:error] 预期错误码 {missing} 未在输出中出现（实际输出含: {emitted_desc}）"
    ))
}

/// 扫描输出中全部 `[EXXXX]` 形态的错误码（保序去重）。
fn emitted_error_codes(output: &str) -> Vec<String> {
    let bytes = output.as_bytes();
    let mut codes = Vec::new();
    let mut i = 0;
    while i + 7 <= bytes.len() {
        if bytes[i] == b'['
            && bytes[i + 1] == b'E'
            && bytes[i + 2..i + 6].iter().all(|b| b.is_ascii_digit())
            && bytes[i + 6] == b']'
        {
            let code = output[i + 1..i + 6].to_string();
            if !codes.contains(&code) {
                codes.push(code);
            }
            i += 7;
        } else {
            i += 1;
        }
    }
    codes
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
