//! `yaoxiang test` 运行器 — RFC-036 Phase 1 + Phase 2 + §8.2 分流判定 + Phase 3
//!
//! 测试文件是普通 `.yx` 文件：发现 → 按头部指令分流执行 → 汇总报告。
//! 进程级隔离，零编译器改动。
//!
//! 指令约定（解析统一在 src/util/test_markers.rs，与 yx_runner 共用）：
//! - `// expect: compile-error EXXXX` — 编译期拒绝测试：单步 `check`，退出码
//!   非 0 且全部预期码出现 = PASS（run 不执行）
//! - `// expect: runtime-error EXXXX` — 运行期失败测试：`check` 必须通过 +
//!   `run` 必须失败且全部预期码出现 = PASS
//! - 无 expect 指令 = 行为测试：单步 `run`，退出码 0 = PASS
//! - 指令解析失败（unknown kind/缺码/重复声明/非法 mode）= 不执行直接 FAIL
//!   （构造期拒绝）
//! - `// skip: <原因>` — 跳过执行，计入 skipped
//! - `// mode: embedded|standard|full` — 子进程 `--runtime` 模式（仅 run 步消费）
//!
//! 执行模型（Phase 3）：默认串行；`--parallel` 或 `[tool.test].parallel = true`
//! 按可用核数起 worker 池（每文件仍是一个独立子进程）。skip/invalid 按发现序在
//! 主线程先行处理；执行结果按完成序流式输出（人类模式），JSON `files` 按路径
//! 排序保证稳定。`--fail-fast` 停止调度新文件，在途文件跑完并计入。
//!
//! 发现与排除：显式 paths 不读配置（RFC-036 §2）；配置模式下 `[tool.test].exclude`
//! （与 patterns 同形态，前缀命中）把文件从发现集剔除——excluded 意味着不是测试，
//! `--list` 同样剔除。
//!
//! 报告契约（RFC-036 §1）：
//! - 默认输出 = 进度（表头 + per-file 行）+ 报告（FAIL 明细 + 汇总 + 类别分布）
//! - `--no-progress` 只抑制进度；FAIL 明细与汇总始终输出——失败不可静默
//! - `--json`：stdout 仅一份 JSON 报告；失败文件附 `exit_code` 与 `stderr`
//!   （ANSI 剥离，CI 取证用），`--verbose` 时全部文件附 `stdout`/`stderr`；
//!   每文件附 `kind`，summary 附 `by_kind` 计数
//! - `--fail-fast` 首个失败即停；`--list` 每行一个路径，不执行
//!
//! 规范来源：docs/src/design/rfc/accepted/036-test-framework.md §1 CLI 设计 /
//! §5 发现与执行 / §8.2 负向测试判定

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{mpsc, Mutex};
use std::time::Instant;

use serde::Serialize;

use crate::util::config::TestConfig;
use crate::util::diagnostic::emitter::ansi::strip_ansi;
use crate::util::test_markers::{Expectation, TestFileSpec};

/// `yaoxiang test` 选项 — RFC-036 §1 CLI 设计
pub struct TestOptions {
    /// 显式路径（优先于配置发现，RFC-036 §5；此模式不读配置）
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
    /// `--parallel`：并行执行（与 `[tool.test].parallel` 取或，RFC-036 Phase 3）
    pub parallel: bool,
}

/// 单个测试文件的执行结果（人类报告与 JSON 报告共用）
struct FileResult {
    display: String,
    /// 类别标签：behavior / compile-error / runtime-error / invalid
    kind: &'static str,
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
    /// 按类别的执行计数（不含 skipped；含 invalid）
    by_kind: BTreeMap<String, usize>,
    time_secs: f64,
}

#[derive(Serialize)]
struct JsonFile {
    file: String,
    kind: String,
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
/// 发现顺序：显式 paths（不读配置）→ `./yaoxiang.toml` 的 `[tool.test]`；
/// 之后应用 exclude（配置模式）与 `--filter`（文件名包含，RFC-036 §5）。
pub fn run_test_command(options: &TestOptions) -> anyhow::Result<usize> {
    let cwd = std::env::current_dir().unwrap_or_default();
    let (mut files, exclude_patterns, config_parallel) = if options.paths.is_empty() {
        let config = load_config();
        (
            discover_from(&config.patterns),
            config.exclude,
            config.parallel,
        )
    } else {
        let patterns: Vec<String> = options
            .paths
            .iter()
            .map(|p| p.display().to_string())
            .collect();
        (discover_from(&patterns), Vec::new(), false)
    };
    // exclude：命中即从发现集剔除（--list 同样剔除——excluded 意味着不是测试）
    files.retain(|f| !is_excluded(f, &exclude_patterns));
    if let Some(filter) = &options.filter {
        files.retain(|f| {
            f.file_name()
                .is_some_and(|n| n.to_string_lossy().contains(filter.as_str()))
        });
    }
    let parallel = options.parallel || config_parallel;

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
            // // skip: 文件不会执行，不在执行集清单中列出
            if TestFileSpec::parse(file).skip_reason.is_some() {
                continue;
            }
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
    let mut skipped = 0usize;
    let mut stopped_early = false;

    // 调度：skip/invalid 按发现序在主线程处理（SKIP 行顺序确定、零进程开销），
    // 可执行文件进入工作队列——串行 = 单 worker，`--parallel` = 核数 worker
    let mut queue: Vec<(PathBuf, String, TestFileSpec)> = Vec::new();
    for file in &files {
        let spec = TestFileSpec::parse(file);
        // // skip: 跳过执行，只计入 skipped（RFC-036 §1 汇总字段的真实来源）
        if let Some(reason) = &spec.skip_reason {
            skipped += 1;
            if !options.json && !options.no_progress {
                let display = display_path(file, &cwd);
                println!("{} {} SKIP ({reason})", display, progress_dots(&display));
            }
            continue;
        }
        if let Some(reason) = &spec.invalid {
            // 构造期拒绝：指令解析失败 = FAIL，不 spawn 子进程
            let result = FileResult {
                display: display_path(file, &cwd),
                kind: "invalid",
                passed: false,
                secs: 0.0,
                exit_code: None,
                stdout: String::new(),
                stderr: format!("无效的头部指令: {reason}"),
            };
            if !options.json {
                print!("{}", format_file_result(&result, options));
            }
            let failed_now = !result.passed;
            results.push(result);
            if failed_now && options.fail_fast {
                stopped_early = true;
                break;
            }
            continue;
        }
        queue.push((file.clone(), display_path(file, &cwd), spec));
    }

    let stop = AtomicBool::new(false);
    let workers = if parallel {
        std::thread::available_parallelism()
            .map(|n| n.get())
            .unwrap_or(1)
    } else {
        1
    };
    let workers = workers.min(queue.len()).max(1);
    let (tx, rx) = mpsc::channel::<FileResult>();
    let queue = Mutex::new(queue.into_iter());
    std::thread::scope(|scope| {
        for _ in 0..workers {
            let queue = &queue;
            let stop = &stop;
            let tx = tx.clone();
            let exe = &exe;
            let options = &options;
            scope.spawn(move || loop {
                if stop.load(Ordering::Relaxed) {
                    break;
                }
                let next = queue.lock().expect("test work queue").next();
                let Some((file, display, spec)) = next else {
                    break;
                };
                let result = execute_file(exe, &file, &spec, display, Instant::now());
                let failed = !result.passed;
                let _ = tx.send(result);
                if failed && options.fail_fast {
                    // 停止调度新文件；在途文件照常跑完并计入（RFC-036 §5）
                    stop.store(true, Ordering::Relaxed);
                }
            });
        }
        drop(tx);
        while let Ok(result) = rx.recv() {
            if !options.json {
                print!("{}", format_file_result(&result, options));
            }
            results.push(result);
        }
    });
    let stopped_early = stopped_early || stop.load(Ordering::Relaxed);

    let passed = results.iter().filter(|r| r.passed).count();
    let failed = results.len() - passed;
    let total_secs = total_start.elapsed().as_secs_f64();
    let by_kind = count_by_kind(&results);

    if options.json {
        let mut files: Vec<JsonFile> = results
            .iter()
            .map(|r| json_file(r, options.verbose))
            .collect();
        // 并行完成序不确定——按路径排序保证 JSON 输出稳定（CI diff 友好）
        files.sort_by(|a, b| a.file.cmp(&b.file));
        let report = JsonReport {
            summary: JsonSummary {
                total: results.len(),
                passed,
                failed,
                skipped,
                by_kind,
                time_secs: round_secs(total_secs),
            },
            files,
        };
        println!("{}", serde_json::to_string_pretty(&report)?);
    } else {
        if stopped_early {
            println!("\nStopped by --fail-fast after first failure.");
        }
        println!(
            "\nResults: {} {} passed, {} {} failed, {} skipped ({:.3}s)",
            passed,
            plural_files(passed),
            failed,
            plural_files(failed),
            skipped,
            total_secs
        );
        println!("{}", categories_line(&by_kind));
    }
    Ok(failed)
}

/// 按期望类别执行单个文件（RFC-036 §8.2 判定矩阵）：
///
/// - Behavior：单步 run，退出码 0 = PASS
/// - CompileError：单步 check，退出码非 0 且全部预期码出现 = PASS（run 不执行）
/// - RuntimeError：check 必须通过 + run 必须失败且全部预期码出现 = PASS；
///   check 就失败 = FAIL（语料分类错误：运行期失败不该在编译期被拒）
fn execute_file(
    exe: &Path,
    file: &Path,
    spec: &TestFileSpec,
    display: String,
    start: Instant,
) -> FileResult {
    let secs = || start.elapsed().as_secs_f64();
    match &spec.expectation {
        Expectation::Behavior => match spawn_run(exe, file, spec.mode.as_ref()) {
            Ok(out) => FileResult {
                kind: "behavior",
                passed: out.status.success(),
                exit_code: out.status.code(),
                stdout: String::from_utf8_lossy(&out.stdout).to_string(),
                stderr: String::from_utf8_lossy(&out.stderr).to_string(),
                display,
                secs: secs(),
            },
            Err(e) => spawn_failure(spec, display, secs(), e),
        },
        Expectation::CompileError(_) => match spawn_check(exe, file) {
            Ok(out) => {
                let stderr = String::from_utf8_lossy(&out.stderr).to_string();
                let (passed, stderr) = if out.status.success() {
                    (false, format!("预期编译失败，但 check 通过\n{stderr}"))
                } else if let Err(note) = spec.check_expected_codes(&stderr) {
                    (false, format!("{note}\n{stderr}"))
                } else {
                    (true, stderr)
                };
                FileResult {
                    kind: "compile-error",
                    passed,
                    exit_code: out.status.code(),
                    stdout: String::from_utf8_lossy(&out.stdout).to_string(),
                    stderr,
                    display,
                    secs: secs(),
                }
            }
            Err(e) => spawn_failure(spec, display, secs(), e),
        },
        Expectation::RuntimeError(_) => {
            let check_out = match spawn_check(exe, file) {
                Ok(out) => out,
                Err(e) => return spawn_failure(spec, display, secs(), e),
            };
            if !check_out.status.success() {
                return FileResult {
                    kind: "runtime-error",
                    passed: false,
                    exit_code: check_out.status.code(),
                    stdout: String::from_utf8_lossy(&check_out.stdout).to_string(),
                    stderr: format!(
                        "预期运行期失败，但编译被拒（check 失败）\n{}",
                        String::from_utf8_lossy(&check_out.stderr)
                    ),
                    display,
                    secs: secs(),
                };
            }
            match spawn_run(exe, file, spec.mode.as_ref()) {
                Ok(out) => {
                    let stderr = String::from_utf8_lossy(&out.stderr).to_string();
                    let (passed, stderr) = if out.status.success() {
                        (false, format!("预期运行期失败，但 run 成功退出\n{stderr}"))
                    } else if let Err(note) = spec.check_expected_codes(&stderr) {
                        (false, format!("{note}\n{stderr}"))
                    } else {
                        (true, stderr)
                    };
                    FileResult {
                        kind: "runtime-error",
                        passed,
                        exit_code: out.status.code(),
                        stdout: String::from_utf8_lossy(&out.stdout).to_string(),
                        stderr,
                        display,
                        secs: secs(),
                    }
                }
                Err(e) => spawn_failure(spec, display, secs(), e),
            }
        }
    }
}

/// 子进程 spawn 失败（罕见：exe 消失/权限）——按声明的类别记 FAIL。
fn spawn_failure(
    spec: &TestFileSpec,
    display: String,
    secs: f64,
    e: std::io::Error,
) -> FileResult {
    FileResult {
        display,
        kind: spec.kind_label(),
        passed: false,
        secs,
        exit_code: None,
        stdout: String::new(),
        stderr: format!("failed to spawn test process: {e}"),
    }
}

/// `check` 单文件（编译期判定步）。
fn spawn_check(
    exe: &Path,
    file: &Path,
) -> std::io::Result<std::process::Output> {
    Command::new(exe).arg("check").arg(file).output()
}

/// `run` 单文件（行为/运行期判定步；`// mode:` 透传 `--runtime`）。
/// #327 起运行时错误默认携带 debug_map，无需再传旗标。
fn spawn_run(
    exe: &Path,
    file: &Path,
    mode: Option<&String>,
) -> std::io::Result<std::process::Output> {
    let mut command = Command::new(exe);
    command.arg("run");
    if let Some(mode) = mode {
        command.arg("--runtime").arg(mode);
    }
    command.arg(file);
    command.output()
}

/// exclude pattern 匹配：与 patterns 同形态——字面路径（文件或目录）或
/// `root/**…`，一律按路径前缀命中（组件级比较，跨平台分隔符安全）。
/// `/**` 后无论有无后缀段（`root/**`、`root/**/*.yx`）都等价于排除 root 子树。
fn is_excluded(
    file: &Path,
    patterns: &[String],
) -> bool {
    patterns.iter().any(|p| {
        let p = p.trim_end_matches(['/', '\\', '*']);
        let root = match p.split_once("/**") {
            Some((root, _)) => root,
            None => p,
        };
        file.strip_prefix(root).is_ok()
    })
}

/// 执行集的类别计数（固定四键，schema 稳定供 CI 消费）。
fn count_by_kind(results: &[FileResult]) -> BTreeMap<String, usize> {
    let mut by_kind: BTreeMap<String, usize> = BTreeMap::new();
    for kind in ["behavior", "compile-error", "runtime-error", "invalid"] {
        by_kind.insert(kind.to_string(), 0);
    }
    for result in results {
        *by_kind.entry(result.kind.to_string()).or_default() += 1;
    }
    by_kind
}

/// 人类可读的类别分布行（invalid 仅在出现时展示）。
fn categories_line(by_kind: &BTreeMap<String, usize>) -> String {
    let get = |k: &str| by_kind.get(k).copied().unwrap_or(0);
    let mut parts = vec![
        format!("{} behavior", get("behavior")),
        format!("{} compile-error", get("compile-error")),
        format!("{} runtime-error", get("runtime-error")),
    ];
    let invalid = get("invalid");
    if invalid > 0 {
        parts.push(format!("{invalid} invalid"));
    }
    format!("Categories: {}", parts.join(", "))
}

/// RFC-036 §1 单文件结果块：进度行 + 失败明细（+ verbose 详情）。
///
/// 整块拼成单个 String 一次输出——并行模式下不同文件的行不会交错。
/// PASS 行受 `--no-progress` 抑制；FAIL 行与诊断明细始终输出——失败不可静默。
fn format_file_result(
    result: &FileResult,
    options: &TestOptions,
) -> String {
    use std::fmt::Write as _;
    let mut out = String::new();
    if result.passed && options.no_progress {
        return out;
    }
    let status = if result.passed { "PASS" } else { "FAIL" };
    let _ = writeln!(
        out,
        "{} {} {} ({:.3}s)",
        result.display,
        progress_dots(&result.display),
        status,
        result.secs
    );
    if !result.passed && !result.stderr.trim().is_empty() {
        for line in strip_ansi(&result.stderr).trim().lines() {
            let _ = writeln!(out, "      {line}");
        }
    }
    if options.verbose {
        if !result.stdout.trim().is_empty() {
            let _ = writeln!(out, "      [stdout]");
            for line in strip_ansi(&result.stdout).trim().lines() {
                let _ = writeln!(out, "      {line}");
            }
        }
        if result.passed && !result.stderr.trim().is_empty() {
            let _ = writeln!(out, "      [stderr]");
            for line in strip_ansi(&result.stderr).trim().lines() {
                let _ = writeln!(out, "      {line}");
            }
        }
    }
    out
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

/// JSON 报告条目：附 `kind`；失败文件附 `exit_code` 与 `stderr`（CI 取证）；
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
        kind: result.kind.to_string(),
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
            by_kind: count_by_kind(&[]),
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

/// 读取 `./yaoxiang.toml` 的 `[tool.test]`，缺失或解析失败回退默认值。
fn load_config() -> TestConfig {
    let config_path = PathBuf::from("yaoxiang.toml");
    if config_path.exists() {
        let content = std::fs::read_to_string(&config_path).unwrap_or_default();
        if let Ok(config) = toml::from_str::<crate::util::config::ProjectConfig>(&content) {
            return config.tool.test;
        }
    }
    TestConfig::default()
}

/// 按 pattern 集发现测试文件，排序去重。
fn discover_from(patterns: &[String]) -> Vec<PathBuf> {
    let mut files = Vec::new();
    for pattern in patterns {
        collect_pattern(pattern, &mut files);
    }
    files.sort();
    files.dedup();
    files
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
