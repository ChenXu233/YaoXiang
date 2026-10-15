//! # shootout — YaoXiang 多语言基准对比运行器
//!
//! 读取 `bench.toml`，逐 benchmark 编译/运行各语言实现，输出对比结果。
//!
//! 配置格式为 TOML（2026-10-15 由 YAML 迁移）：`serde_yaml` 与 `serde_yaml_ng` 都
//! 依赖 `unsafe-libyaml`（RUSTSEC-2024-0449，unmaintained），而 `toml` 本来就在本
//! 仓库依赖树内（根 `Cargo.toml`、`tools/yx/Cargo.toml`）——本次迁移不引入新包，
//! 只净减该条链，`cargo audit` 由「1 allowed warning」转为零告警。取舍详见
//! `bench.toml` 头注。
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
#[serde(deny_unknown_fields)]
struct Config {
    benchmarks: BTreeMap<String, BenchmarkDef>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct BenchmarkDef {
    description: Option<String>,
    warmup: u32,
    runs: u32,
    inputs: Vec<String>,
    languages: BTreeMap<String, LangDef>,
    /// 输入规模 → 期望输出（金标）。逐语言比对，不符即判失败。
    ///
    /// 必须**全覆盖或全不写**：写一半会让剩下的输入静默失去校验
    /// （由 `test_config_manifest_parses_and_paths_exist` 钉住）。
    #[serde(default)]
    expected: BTreeMap<String, String>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct LangDef {
    src: String,
    compile: Option<CompileDef>,
    run: Option<RunDef>,
    /// 取版本号的命令（如 `rustc --version`）。用于报告与历史数据的归因：
    /// 只在配置里写一次，runner 每语言只跑一次（多 benchmark 共用会去重）。
    version: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct CompileDef {
    cmd: String,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
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
    /// 中位数。跨语言/跨夜的头条数字用它而非均值：墙钟样本右偏，一次调度
    /// 抖动就能把均值拉高，中位数抗离群。
    median_ms: f64,
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
    /// 本次运行的环境归因。读数变了却不知道工具链或机器也变了，就无从判断
    /// 是语言退步还是环境换版——故与 rows 同存、同入库。
    meta: Meta,
    rows: Vec<Row>,
    failures: Vec<Failure>,
}

/// 环境归因（跨夜可比性的前提条件）。
#[derive(Debug, Serialize, Default)]
struct Meta {
    /// 被测语言 → 版本号首行（由配置里该语言的 `version` 命令取得）。
    toolchains: BTreeMap<String, String>,
    os: String,
    arch: String,
    cpus: usize,
    /// CPU 型号：Linux 读 /proc/cpuinfo，Windows 读 PROCESSOR_IDENTIFIER。
    cpu: String,
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

    /// bench.toml 路径
    #[arg(long, default_value = "benches/shootout/bench.toml")]
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

/// 中位数（样本数为偶数时取中间两个的均值）。
fn median(samples: &[f64]) -> f64 {
    if samples.is_empty() {
        return 0.0;
    }
    let mut v = samples.to_vec();
    v.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    let n = v.len();
    if n % 2 == 1 {
        v[n / 2]
    } else {
        (v[n / 2 - 1] + v[n / 2]) / 2.0
    }
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
                                let med = median(&samples);
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
                                    "{} ± {}（中位 {}，相对偏差 {:.1}%）{} → {}",
                                    fmt_duration(mean),
                                    fmt_duration(stddev),
                                    fmt_duration(med),
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
                                    median_ms: med,
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

    /// 每门语言只取一次版本号（多 benchmark 共用同一条命令 → 去重）。
    ///
    /// 取不到版本不影响测量：记为 `<获取失败: …>` 继续跑——基准的价值在读数，
    /// 归因信息缺失应当显眼，但不该阻断整轮。
    fn collect_toolchains(
        &self,
        cfg: &Config,
    ) -> BTreeMap<String, String> {
        let mut out: BTreeMap<String, String> = BTreeMap::new();
        for bench_def in cfg.benchmarks.values() {
            for (lang_name, lang_def) in &bench_def.languages {
                if out.contains_key(lang_name) {
                    continue;
                }
                let Some(tpl) = &lang_def.version else {
                    continue;
                };
                let src_path = self.bench_root.join(&lang_def.src);
                let out_path = self
                    .bench_root
                    .join("out")
                    .join(format!("{lang_name}.version"));
                let cmd = render_cmd(tpl, &src_path, &out_path, &self.yaoxiang_bin);
                let text = match time_cmd(&cmd, "") {
                    Ok((_, t)) => t.lines().next().unwrap_or("").trim().to_string(),
                    Err(e) => format!("<获取失败: {e}>"),
                };
                out.insert(lang_name.clone(), text);
            }
        }
        out
    }
}

/// 采集环境归因（机器身份 + 语言工具链版本）。
fn env_meta(toolchains: BTreeMap<String, String>) -> Meta {
    let cpu = std::fs::read_to_string("/proc/cpuinfo")
        .ok()
        .and_then(|text| {
            text.lines()
                .find(|l| l.starts_with("model name"))
                .and_then(|l| l.split(':').nth(1))
                .map(|s| s.trim().to_string())
        })
        .or_else(|| std::env::var("PROCESSOR_IDENTIFIER").ok())
        .unwrap_or_else(|| "unknown".to_string());
    Meta {
        toolchains,
        os: std::env::consts::OS.to_string(),
        arch: std::env::consts::ARCH.to_string(),
        cpus: std::thread::available_parallelism()
            .map(|n| n.get())
            .unwrap_or(0),
        cpu,
    }
}

/// 正确性判据：金标 + 跨语言一致。
///
/// 把「哪门语言算错了」从人肉看表格变成显式失败。两级判据：
///   1. 配置 `[benchmarks.X.expected]` 声明的**金标**——逐语言逐输入比对；
///   2. 同一 (bench, input) 下各语言输出必须**互相一致**——无金标时的兜底。
///
/// 既无金标又只有一门语言时无法判定：此时**显式报告未校验的组数**，而不是
/// 默认通过——「没检查」与「检查通过」在报告里必须能分开。
fn verify_outputs(
    report: &mut Report,
    cfg: &Config,
) {
    // (bench, input) -> lang -> output
    let mut groups: BTreeMap<(String, String), BTreeMap<String, String>> = BTreeMap::new();
    for r in &report.rows {
        groups
            .entry((r.bench.clone(), r.input.clone()))
            .or_default()
            .insert(r.lang.clone(), r.output.clone());
    }

    let (mut golden_groups, mut consensus_groups, mut unchecked) = (0usize, 0usize, 0usize);
    for ((bench, input), by_lang) in &groups {
        let expected = cfg
            .benchmarks
            .get(bench)
            .and_then(|b| b.expected.get(input));
        match expected {
            Some(exp) => {
                golden_groups += 1;
                for (lang, out) in by_lang {
                    if out != exp {
                        report.failures.push(Failure {
                            bench: bench.clone(),
                            lang: lang.clone(),
                            input: Some(input.clone()),
                            error: format!("输出与金标不符：期望 {exp:?}，实际 {out:?}"),
                        });
                    }
                }
            }
            None if by_lang.len() > 1 => {
                consensus_groups += 1;
                let mut distinct: Vec<&String> = by_lang.values().collect();
                distinct.sort();
                distinct.dedup();
                if distinct.len() > 1 {
                    let detail = by_lang
                        .iter()
                        .map(|(l, o)| format!("{l}={o:?}"))
                        .collect::<Vec<_>>()
                        .join("，");
                    report.failures.push(Failure {
                        bench: bench.clone(),
                        lang: by_lang.keys().cloned().collect::<Vec<_>>().join("+"),
                        input: Some(input.clone()),
                        error: format!("跨语言输出不一致：{detail}"),
                    });
                }
            }
            None => unchecked += 1,
        }
    }

    println!(
        "  正确性：金标校验 {} 组、跨语言一致性 {} 组，⚠ 未校验 {} 组（无金标且仅一门语言）",
        golden_groups, consensus_groups, unchecked
    );
    for f in &report.failures {
        println!(
            "    ✗ {} / {} 输入 {}：{}",
            f.bench,
            f.lang,
            f.input.clone().unwrap_or_else(|| "-".into()),
            f.error
        );
    }
}

// ============================================================================
// 入口
// ============================================================================

fn main() -> Result<()> {
    let cli = Cli::parse();

    let config_text = std::fs::read_to_string(&cli.config)
        .with_context(|| format!("读取配置失败: {}", cli.config.display()))?;
    // 带路径的报错：--config 可指向任意文件，只说「bench.toml 失败」会指错地方
    let cfg: Config = toml::from_str(&config_text)
        .with_context(|| format!("解析配置失败: {}", cli.config.display()))?;

    let ctx = RunCtx::new(&cli);
    let mut report = ctx.run(&cfg)?;
    report.meta = env_meta(ctx.collect_toolchains(&cfg));
    verify_outputs(&mut report, &cfg);

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
            meta: Meta::default(),
            rows: vec![Row {
                bench: "fibonacci".into(),
                description: Some("迭代斐波那契".into()),
                lang: "rust".into(),
                input: "20".into(),
                compile_ms: Some(302.88),
                mean_ms: 24.3,
                median_ms: 24.1,
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
            "median_ms",
            "stddev_ms",
            "relative_pct",
            "output",
        ] {
            assert!(row.get(key).is_some(), "Row 缺少字段 {key}");
        }

        // 环境归因也是入库契约的一部分：历史数据少了工具链/机器字段就失去可比性
        let meta = &v["meta"];
        for key in ["toolchains", "os", "arch", "cpus", "cpu"] {
            assert!(meta.get(key).is_some(), "Meta 缺少字段 {key}");
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

    /// 清单必须能被真实解析，且每条语言配置都指向真实存在的源码。
    ///
    /// 配置只在 nightly CI 的基准 job 里被读取，而那个 job 不阻断发布：写错字段名
    /// 或路径，本地全绿、夜里 release notes 直接少一张表。故把整份清单当夹具解析
    /// 一遍。字段名拼错由 `deny_unknown_fields` 报错，路径写错由本测试报错。
    #[test]
    fn test_config_manifest_parses_and_paths_exist() {
        // Arrange
        let bench_root = Path::new(env!("CARGO_MANIFEST_DIR")).join("..");
        let config_path = bench_root.join("bench.toml");
        let text = std::fs::read_to_string(&config_path)
            .unwrap_or_else(|e| panic!("读取 {} 失败: {e}", config_path.display()));

        // Act
        let cfg: Config = toml::from_str(&text)
            .unwrap_or_else(|e| panic!("解析 {} 失败: {e}", config_path.display()));

        // Assert
        assert!(
            !cfg.benchmarks.is_empty(),
            "{} 里应至少有一个 benchmark",
            config_path.display()
        );
        for (bench, def) in &cfg.benchmarks {
            assert!(def.runs > 0, "{bench}: runs 必须大于 0，否则取样为空");
            assert!(!def.inputs.is_empty(), "{bench}: inputs 不能为空");
            assert!(!def.languages.is_empty(), "{bench}: 至少要配一种语言");
            for (lang, l) in &def.languages {
                let src = bench_root.join(&l.src);
                assert!(
                    src.is_file(),
                    "{bench}/{lang}: 源码不存在——{}（相对 {}）",
                    l.src,
                    bench_root.display()
                );
                assert!(
                    l.run.is_some(),
                    "{bench}/{lang}: 未配 run，该项不会产生任何读数"
                );
            }
            // 金标必须**全覆盖或全不写**：写一半会让余下的输入静默失去校验，
            // 而「没检查」在报告里必须能跟「检查通过」分开。
            if !def.expected.is_empty() {
                for input in &def.inputs {
                    assert!(
                        def.expected.contains_key(input),
                        "{bench}: 输入 {input} 缺金标（金标要么全覆盖，要么不写）"
                    );
                }
            }
        }
    }

    /// 中位数抗离群：偶数样本取中间两个的均值。
    #[test]
    fn test_median_ignores_outlier() {
        // Arrange：一个被调度抖动拉高的样本
        let samples = [10.0, 11.0, 12.0, 13.0, 900.0];

        // Act
        let got = median(&samples);

        // Assert
        assert!((got - 12.0).abs() < 1e-9, "中位数应为 12.0，实际 {got}");
        assert!(
            (median(&[1.0, 2.0, 3.0, 4.0]) - 2.5).abs() < 1e-9,
            "偶数样本应取中间两个的均值"
        );
        assert_eq!(median(&[]), 0.0, "空样本应为 0.0 而非 panic");
    }

    fn report_with(
        lang: &str,
        output: &str,
    ) -> Report {
        Report {
            meta: Meta::default(),
            rows: vec![Row {
                bench: "fibonacci".into(),
                description: None,
                lang: lang.into(),
                input: "20".into(),
                compile_ms: None,
                mean_ms: 1.0,
                median_ms: 1.0,
                stddev_ms: 0.1,
                relative_pct: 10.0,
                output: output.into(),
            }],
            failures: Vec::new(),
        }
    }

    fn cfg_with_expected(expected: &[(&str, &str)]) -> Config {
        let mut expected_map = BTreeMap::new();
        for (k, v) in expected {
            expected_map.insert((*k).to_string(), (*v).to_string());
        }
        let mut languages = BTreeMap::new();
        languages.insert(
            "rust".to_string(),
            LangDef {
                src: "src/fibonacci/fibonacci.rs".into(),
                compile: None,
                run: None,
                version: None,
            },
        );
        let mut benchmarks = BTreeMap::new();
        benchmarks.insert(
            "fibonacci".to_string(),
            BenchmarkDef {
                description: None,
                warmup: 0,
                runs: 1,
                inputs: vec!["20".into()],
                languages,
                expected: expected_map,
            },
        );
        Config { benchmarks }
    }

    /// 算错的输出必须被金标拦下：「跑得快的错答案」不是基准读数。
    #[test]
    fn test_verify_outputs_flags_golden_mismatch() {
        // Arrange
        let mut report = report_with("rust", "9999");
        let cfg = cfg_with_expected(&[("20", "6765")]);

        // Act
        verify_outputs(&mut report, &cfg);

        // Assert
        assert_eq!(report.failures.len(), 1, "金标不符应产出一条失败");
        assert!(
            report.failures[0].error.contains("金标"),
            "失败信息应指明是金标不符，实际 {:?}",
            report.failures[0].error
        );
    }

    /// 无金标时退化为跨语言一致性：两门语言输出不同必须响亮失败。
    #[test]
    fn test_verify_outputs_flags_cross_language_disagreement() {
        // Arrange：两门语言跑同一输入，输出不同
        let mut report = report_with("rust", "6765");
        report.rows.push(Row {
            bench: "fibonacci".into(),
            description: None,
            lang: "python".into(),
            input: "20".into(),
            compile_ms: None,
            mean_ms: 2.0,
            median_ms: 2.0,
            stddev_ms: 0.2,
            relative_pct: 10.0,
            output: "832040".into(),
        });
        let cfg = cfg_with_expected(&[]);

        // Act
        verify_outputs(&mut report, &cfg);

        // Assert
        assert_eq!(report.failures.len(), 1, "跨语言不一致应产出一条失败");
        assert!(
            report.failures[0].error.contains("跨语言"),
            "失败信息应指明是跨语言不一致，实际 {:?}",
            report.failures[0].error
        );
    }

    /// 两门语言输出一致时不得报失败（否则判据会变成噪声源）。
    #[test]
    fn test_verify_outputs_accepts_agreeing_languages() {
        // Arrange
        let mut report = report_with("rust", "6765");
        report.rows.push(Row {
            bench: "fibonacci".into(),
            description: None,
            lang: "python".into(),
            input: "20".into(),
            compile_ms: None,
            mean_ms: 2.0,
            median_ms: 2.0,
            stddev_ms: 0.2,
            relative_pct: 10.0,
            output: "6765".into(),
        });
        let cfg = cfg_with_expected(&[]);

        // Act
        verify_outputs(&mut report, &cfg);

        // Assert
        assert!(report.failures.is_empty(), "输出一致时不应有失败");
    }

    #[test]
    fn stats_mean_and_stddev() {
        let (mean, stddev) = stats(&[10.0, 20.0, 30.0]);
        assert!((mean - 20.0).abs() < 1e-9);
        assert!((stddev - 8.16496580927726).abs() < 1e-9);
    }
}
