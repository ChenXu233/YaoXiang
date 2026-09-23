//! # shootout — YaoXiang 多语言基准对比运行器
//!
//! 读取 `bench.yaml`，逐 benchmark 编译/运行各语言实现，输出对比结果。
//!
//! ## 用法
//! ```bash
//! cargo run -p shootout                          # 全部
//! cargo run -p shootout -- --bench fibonacci     # 单问题
//! cargo run -p shootout -- --lang yaoxiang       # 单语言
//! cargo run -p shootout -- --out-json r.json     # 另存机器可读结果
//! cargo run -p shootout -- --bench-root benches/shootout  # 指定根目录
//! ```
//!
//! CI 用 `--out-json` 产出结构化结果，由 `scripts/ci/bench-report.sh`
//! 渲染成 release notes 里的表格；人读的框线输出只作本地观看用。

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

use anyhow::{Context, Result};
use clap::Parser;
use serde::{Deserialize, Serialize};

// ============================================================================
// 配置类型
// ============================================================================

#[derive(Debug, Deserialize)]
struct Config {
    benchmarks: BTreeMap<String, BenchmarkDef>,
}

#[derive(Debug, Deserialize)]
struct BenchmarkDef {
    description: Option<String>,
    warmup: u32,
    runs: u32,
    inputs: Vec<String>,
    languages: BTreeMap<String, LangDef>,
}

#[derive(Debug, Deserialize)]
struct LangDef {
    src: String,
    compile: Option<CompileDef>,
    run: Option<RunDef>,
}

#[derive(Debug, Deserialize)]
struct CompileDef {
    cmd: String,
}

#[derive(Debug, Deserialize)]
struct RunDef {
    cmd: String,
}

// ============================================================================
// 结果类型（JSON 契约，被 bench-report.sh 消费）
// ============================================================================

/// 一条测量结果。
#[derive(Debug, Serialize)]
struct Row {
    bench: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    description: Option<String>,
    lang: String,
    input: String,
    /// 编译耗时（毫秒）。解释型语言（无 compile 步骤）为 null。
    compile_ms: Option<f64>,
    mean_ms: f64,
    stddev_ms: f64,
    /// 相对偏差（stddev/mean），百分比。
    relative_pct: f64,
    /// 被测程序本次的输出。跨语言跑同一输入，它应当一致，是正确性对照点。
    output: String,
}

/// 未能产出结果的语言。
///
/// 必须显式记录而不是静默省略：`matrix` 的 yaoxiang 实现曾因类型错误编译失败，
/// 报告里只是「少了一行」，读起来像「这个语言没配这项基准」——与「配了但挂了」
/// 是两回事。
#[derive(Debug, Serialize)]
struct Failure {
    bench: String,
    lang: String,
    /// 编译阶段失败时为 None（还没跑到具体的输入规模）
    input: Option<String>,
    error: String,
}

#[derive(Debug, Serialize, Default)]
struct Report {
    rows: Vec<Row>,
    failures: Vec<Failure>,
}

// ============================================================================
// CLI
// ============================================================================

#[derive(Parser)]
#[command(name = "shootout", about = "YaoXiang 多语言基准对比运行器")]
struct Cli {
    /// 只运行指定的 benchmark（问题名）
    #[arg(long, value_name = "NAME")]
    bench: Option<String>,

    /// 只运行指定的语言
    #[arg(long, value_name = "LANG")]
    lang: Option<String>,

    /// bench.yaml 路径
    #[arg(long, default_value = "benches/shootout/bench.yaml")]
    config: PathBuf,

    /// 基准根目录（源码、编译产物等基准路径）
    #[arg(long, default_value = "benches/shootout")]
    bench_root: PathBuf,

    /// yaoxiang 可执行文件路径（不依赖 PATH，避免用到旧版本）。
    ///
    /// 默认指向 `yaoxiang-rs`——RFC-037 引入 `yx`（前门）/`yaoxiang-rs`（引擎）
    /// 双二进制后，Cargo.toml 的 `[[bin]]` 名为 `yaoxiang-rs`，旧的
    /// `yaoxiang.exe` 已不再产出（此前默认值未同步，导致基准套件报
    /// 「不是内部或外部命令」）。
    #[arg(long, default_value = if cfg!(target_os = "windows") {
        "target/release/yaoxiang-rs.exe"
    } else {
        "target/release/yaoxiang-rs"
    })]
    yaoxiang_bin: PathBuf,

    /// 结构化结果输出路径（JSON）。CI 走这个，本地可不传。
    #[arg(long, value_name = "PATH")]
    out_json: Option<PathBuf>,
}

// ============================================================================
// 测量逻辑
// ============================================================================

/// 构造被测命令的 Command（Windows 经 cmd /C，其余经 sh -c）。
fn build_cmd(
    cmd_str: &str,
    input: &str,
) -> Command {
    let mut cmd = if cfg!(target_os = "windows") {
        let mut c = Command::new("cmd");
        c.args(["/C", cmd_str]);
        c
    } else {
        let mut c = Command::new("sh");
        c.args(["-c", cmd_str]);
        c
    };
    // 注入 BENCH_INPUT 环境变量，所有语言实现统一读取
    cmd.env("BENCH_INPUT", input);
    cmd
}

/// 执行一条命令并计时，返回 `(耗时, stdout)`。
///
/// 显式**捕获**子进程输出，不让它继承 stdout：之前用 `.status()` 继承，
/// 于是每次测量都把被测程序的输出（如 fibonacci 的 `6765`）直接混进 runner
/// 自己的结果流——日志里每个测量值后面都跟着 13 个 `6765`（warmup 3 + runs 10），
/// 这些噪声又被原样搬进 nightly 的 release notes。
fn time_cmd(
    cmd_str: &str,
    input: &str,
) -> Result<(Duration, String)> {
    let mut cmd = build_cmd(cmd_str, input);
    cmd.stdin(Stdio::null());
    cmd.stdout(Stdio::piped());
    cmd.stderr(Stdio::piped());

    let start = Instant::now();
    let out = cmd
        .output()
        .with_context(|| format!("执行命令失败: {}", cmd_str))?;
    let elapsed = start.elapsed();

    if !out.status.success() {
        // 带上子进程 stderr：否则只剩一个 exit code，无从判断失败原因
        let err = String::from_utf8_lossy(&out.stderr);
        anyhow::bail!(
            "命令退出码非零: {} (exit: {:?})\n{}",
            cmd_str,
            out.status.code(),
            err.trim()
        );
    }

    Ok((
        elapsed,
        String::from_utf8_lossy(&out.stdout).trim_end().to_string(),
    ))
}

/// 运行 N 轮测量，返回 `(毫秒样本, 首轮输出)`。
///
/// 顺带校验各轮输出一致：同一输入下重复执行必须逐字节相同。不一致说明程序
/// 有随机性（或读了未初始化内存），那样的读数不可比。空输出也在此暴露——
/// 此前 list_ops 缺 `main()` 调用而「静默空跑」，恒定读数看不出异常。
fn run_rounds(
    cmd_str: &str,
    input: &str,
    warmup: u32,
    runs: u32,
) -> Result<(Vec<f64>, String)> {
    let mut first: Option<String> = None;

    for _ in 0..warmup {
        let (_, output) = time_cmd(cmd_str, input)?;
        first.get_or_insert(output);
    }

    let mut samples = Vec::with_capacity(runs as usize);
    for _ in 0..runs {
        let (d, output) = time_cmd(cmd_str, input)?;
        samples.push(d.as_secs_f64() * 1000.0);
        match &first {
            Some(f) if *f != output => anyhow::bail!(
                "各轮输出不一致：首轮 {:?}，本轮 {:?}（同一输入应可复现）",
                f,
                output
            ),
            None => first = Some(output),
            _ => {}
        }
    }
    Ok((samples, first.unwrap_or_default()))
}

/// 计算 mean ± stddev
fn stats(samples: &[f64]) -> (f64, f64) {
    let n = samples.len() as f64;
    let mean = samples.iter().sum::<f64>() / n;
    let variance = samples.iter().map(|v| (v - mean).powi(2)).sum::<f64>() / n;
    (mean, variance.sqrt())
}

// ============================================================================
// 格式化
// ============================================================================

fn fmt_duration(ms: f64) -> String {
    if ms >= 1000.0 {
        format!("{:.2}s", ms / 1000.0)
    } else if ms >= 1.0 {
        format!("{:.2}ms", ms)
    } else if ms >= 0.001 {
        format!("{:.2}µs", ms * 1000.0)
    } else {
        format!("{:.2}ns", ms * 1_000_000.0)
    }
}

/// 输出摘录：跨语言对照用。过长时截断——string_concat 单次输出可达数 KB，
/// 全量塞进报告会淹掉表格。
fn summarize_output(s: &str) -> String {
    const MAX: usize = 48;
    if s.chars().count() > MAX {
        let head: String = s.chars().take(MAX).collect();
        format!("{}…", head)
    } else {
        s.to_string()
    }
}

// ============================================================================
// 模板替换
// ============================================================================

fn render_cmd(
    template: &str,
    src: &Path,
    out: &Path,
    yaoxiang_bin: &Path,
) -> String {
    // Windows cmd /C 拒绝路径里混用 / 和 \，统一用 OS 原生分隔符
    let normalize = |p: &Path| {
        p.to_string_lossy()
            .replace('/', std::path::MAIN_SEPARATOR_STR)
    };
    template
        .replace("%y", &normalize(yaoxiang_bin))
        .replace("%s", &normalize(src))
        .replace("%o", &normalize(out))
}

// ============================================================================
// Runner 核心
// ============================================================================

/// 标题框内可用宽度（benchmark 名字段）。
const TITLE_W: usize = 34;

struct RunCtx {
    bench_root: PathBuf,
    yaoxiang_bin: PathBuf,
    filter_bench: Option<String>,
    filter_lang: Option<String>,
}

impl RunCtx {
    fn new(cli: &Cli) -> Self {
        Self {
            bench_root: cli.bench_root.clone(),
            yaoxiang_bin: cli.yaoxiang_bin.clone(),
            filter_bench: cli.bench.clone(),
            filter_lang: cli.lang.clone(),
        }
    }

    fn run(
        &self,
        cfg: &Config,
    ) -> Result<Report> {
        let mut report = Report::default();

        for (bench_name, bench_def) in &cfg.benchmarks {
            if let Some(filter) = &self.filter_bench {
                if bench_name != filter {
                    continue;
                }
            }

            // 边框宽度与标题实际占位对齐。此前顶/底边写死 50 个 `═`，而标题行
            // 只占 37 字符，右边明显缺一角（release notes 里肉眼可见）。
            println!();
            println!("╔{}╗", "═".repeat(TITLE_W + 4));
            println!("║  {:TITLE_W$}  ║", bench_name);
            println!("╚{}╝", "═".repeat(TITLE_W + 4));
            if let Some(desc) = &bench_def.description {
                println!("  {}", desc);
            }
            println!();

            let out_dir = self.bench_root.join("out");
            std::fs::create_dir_all(&out_dir)?;

            for (lang_name, lang_def) in &bench_def.languages {
                if let Some(filter) = &self.filter_lang {
                    if lang_name != filter {
                        continue;
                    }
                }

                let src_path = self.bench_root.join(&lang_def.src);
                // Windows 上**原生编译产物**带 `.exe` 后缀（rustc/g++/go 均如此），
                // 而 runner 此前拼的是无后缀路径，导致运行阶段报
                // 「不是内部或外部命令」。
                //
                // 不能只判 `compile.is_some()`：YaoXiang 的编译步骤产出的是
                // `YXBC` 魔数的 .42 字节码（非可执行文件），加 .exe 会得到
                // 误导性的 `*_yaoxiang.exe`（内容仍是字节码）。
                let mut out_name = format!("{}_{}", bench_name, lang_name);
                let is_native_compile = lang_def
                    .compile
                    .as_ref()
                    .is_some_and(|c| !c.cmd.contains("%y"));
                if cfg!(target_os = "windows") && is_native_compile {
                    out_name.push_str(".exe");
                }
                let out_path = out_dir.join(out_name);

                // 编译阶段
                let compile_time = if let Some(compile) = &lang_def.compile {
                    let cmd = render_cmd(&compile.cmd, &src_path, &out_path, &self.yaoxiang_bin);
                    print!("  [{:>10}] 编译 ... ", lang_name);
                    std::io::Write::flush(&mut std::io::stdout())?;
                    match time_cmd(&cmd, "") {
                        Ok((d, _)) => {
                            let ms = d.as_secs_f64() * 1000.0;
                            println!("{}", fmt_duration(ms));
                            Some(ms)
                        }
                        Err(e) => {
                            println!("失败: {}", e);
                            report.failures.push(Failure {
                                bench: bench_name.clone(),
                                lang: lang_name.clone(),
                                input: None,
                                error: e.to_string(),
                            });
                            // 编译失败直接跳过该语言，不跑旧产物
                            continue;
                        }
                    }
                } else {
                    println!("  [{:>10}] 解释执行 (无需编译)", lang_name);
                    None
                };

                // 运行阶段
                if let Some(run_def) = &lang_def.run {
                    for input in &bench_def.inputs {
                        let run_cmd =
                            render_cmd(&run_def.cmd, &src_path, &out_path, &self.yaoxiang_bin);

                        print!("  [{:>10}] 输入={:>6} 运行 ... ", lang_name, input);
                        std::io::Write::flush(&mut std::io::stdout())?;

                        match run_rounds(run_cmd.as_str(), input, bench_def.warmup, bench_def.runs)
                        {
                            Ok((samples, output)) => {
                                let (mean, stddev) = stats(&samples);
                                let relative = if mean > 0.0 {
                                    (stddev / mean) * 100.0
                                } else {
                                    0.0
                                };
                                let compile_info = match compile_time {
                                    Some(ct) => format!(" (编译: {})", fmt_duration(ct)),
                                    None => String::new(),
                                };
                                println!(
                                    "{} ± {} (相对偏差: {:.1}%){} → {}",
                                    fmt_duration(mean),
                                    fmt_duration(stddev),
                                    relative,
                                    compile_info,
                                    summarize_output(&output),
                                );

                                report.rows.push(Row {
                                    bench: bench_name.clone(),
                                    description: bench_def.description.clone(),
                                    lang: lang_name.clone(),
                                    input: input.clone(),
                                    compile_ms: compile_time,
                                    mean_ms: mean,
                                    stddev_ms: stddev,
                                    relative_pct: relative,
                                    output,
                                });
                            }
                            Err(e) => {
                                println!("失败: {}", e);
                                report.failures.push(Failure {
                                    bench: bench_name.clone(),
                                    lang: lang_name.clone(),
                                    input: Some(input.clone()),
                                    error: e.to_string(),
                                });
                            }
                        }
                    }
                }
            }
            println!();
        }
        Ok(report)
    }
}

// ============================================================================
// 入口
// ============================================================================

fn main() -> Result<()> {
    let cli = Cli::parse();

    let yaml_content = std::fs::read_to_string(&cli.config).context("读取 bench.yaml 失败")?;
    let cfg: Config = serde_yaml::from_str(&yaml_content).context("解析 bench.yaml 失败")?;

    let ctx = RunCtx::new(&cli);
    let report = ctx.run(&cfg)?;

    // 先写 JSON 再决定是否 bail：benchmark 挂了但结果仍要留下，
    // 否则 CI 里只剩一个退出码，报告端拿不到失败详情。
    if let Some(path) = &cli.out_json {
        let json = serde_json::to_string_pretty(&report).context("序列化结果失败")?;
        std::fs::write(path, format!("{}\n", json))
            .with_context(|| format!("写入 {} 失败", path.display()))?;
        println!("结果已写入 {}", path.display());
    }

    // 有语言失败时以非零退出：CI 里让该步显式报红，而不是悄悄少几行结果。
    // （nightly 的 shootout job 用 set -o pipefail + tee，故整体仍会失败。）
    if !report.failures.is_empty() {
        anyhow::bail!("有 {} 项基准失败，详见上方输出", report.failures.len());
    }

    Ok(())
}

// ============================================================================
// 测试
// ============================================================================

/// JSON 字段名是**跨语言契约**：`scripts/ci/bench-report.sh` 按这些名字取数。
/// 改名会让报告静默变空（表格没了、也不报错），所以在此钉住。
///
/// 同步维护点：`scripts/ci/bench-report.sh` 里的 `--shootout-json` 分支。
#[cfg(test)]
mod tests {
    use super::*;

    fn sample_report() -> Report {
        Report {
            rows: vec![Row {
                bench: "fibonacci".into(),
                description: Some("迭代斐波那契".into()),
                lang: "rust".into(),
                input: "20".into(),
                compile_ms: Some(302.88),
                mean_ms: 24.3,
                stddev_ms: 2.87,
                relative_pct: 11.8,
                output: "6765".into(),
            }],
            failures: vec![Failure {
                bench: "matrix".into(),
                lang: "yaoxiang".into(),
                input: None,
                error: "编译失败".into(),
            }],
        }
    }

    #[test]
    fn json_contract_keys_are_stable() {
        let v: serde_json::Value =
            serde_json::from_str(&serde_json::to_string(&sample_report()).unwrap()).unwrap();

        let row = &v["rows"][0];
        for key in [
            "bench",
            "description",
            "lang",
            "input",
            "compile_ms",
            "mean_ms",
            "stddev_ms",
            "relative_pct",
            "output",
        ] {
            assert!(row.get(key).is_some(), "Row 缺少字段 {key}");
        }

        let f = &v["failures"][0];
        for key in ["bench", "lang", "input", "error"] {
            assert!(f.get(key).is_some(), "Failure 缺少字段 {key}");
        }
        // 编译阶段失败没有具体输入规模，此时 input 允许为 null
        assert!(f["input"].is_null());
    }

    /// 解释型语言没有编译步骤，`compile_ms` 必须是 null（报告端据此显示「—」，
    /// 而不是把它当成 0 毫秒的编译耗时）。
    #[test]
    fn missing_compile_is_null_not_zero() {
        let mut r = sample_report();
        r.rows[0].compile_ms = None;
        let v: serde_json::Value =
            serde_json::from_str(&serde_json::to_string(&r).unwrap()).unwrap();
        assert!(v["rows"][0]["compile_ms"].is_null());
    }

    /// 长输出要截断：string_concat 单次可达数 KB，全量进报告会淹掉表格。
    #[test]
    fn long_output_is_truncated() {
        let short = "6765";
        assert_eq!(summarize_output(short), short);

        let long = "Hello World!".repeat(20);
        let got = summarize_output(&long);
        assert!(got.ends_with('…'), "应截断并带省略号，实际 {got:?}");
        assert_eq!(got.chars().count(), 49, "48 字符 + 省略号");
    }

    #[test]
    fn stats_mean_and_stddev() {
        let (mean, stddev) = stats(&[10.0, 20.0, 30.0]);
        assert!((mean - 20.0).abs() < 1e-9);
        assert!((stddev - 8.16496580927726).abs() < 1e-9);
    }
}
