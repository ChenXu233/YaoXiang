//! # bench-runner — YaoXiang 多语言基准测试运行器
//!
//! 读取 `bench.yaml`，逐 benchmark 编译/运行各语言实现，输出对比结果。
//!
//! ## 用法
//! ```bash
//! cargo run --package bench-runner                          # 全部
//! cargo run --package bench-runner -- --bench fibonacci     # 单问题
//! cargo run --package bench-runner -- --lang yaoxiang       # 单语言
//! cargo run --package bench-runner -- --bench-root bench    # 指定根目录
//! ```

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::{Duration, Instant};

use anyhow::{Context, Result};
use clap::Parser;
use serde::Deserialize;

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
// CLI
// ============================================================================

#[derive(Parser)]
#[command(name = "bench-runner", about = "YaoXiang 多语言基准测试运行器")]
struct Cli {
    /// 只运行指定的 benchmark（问题名）
    #[arg(long, value_name = "NAME")]
    bench: Option<String>,

    /// 只运行指定的语言
    #[arg(long, value_name = "LANG")]
    lang: Option<String>,

    /// bench.yaml 路径
    #[arg(long, default_value = "bench/bench.yaml")]
    config: PathBuf,

    /// 基准根目录（源码、编译产物等基准路径）
    #[arg(long, default_value = ".")]
    bench_root: PathBuf,
}

// ============================================================================
// 测量逻辑
// ============================================================================

/// 构造 Command，设置环境变量，执行并返回耗时
fn time_cmd(cmd_str: &str, input: &str) -> Result<Duration> {
    let start = Instant::now();

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

    let status = cmd.status().with_context(|| format!("执行命令失败: {}", cmd_str))?;

    if !status.success() {
        anyhow::bail!("命令退出码非零: {} (exit: {:?})", cmd_str, status.code());
    }
    Ok(start.elapsed())
}

/// 运行 N 轮测量，返回毫秒 Vec
fn run_rounds(cmd_str: &str, input: &str, warmup: u32, runs: u32) -> Result<Vec<f64>> {
    for _ in 0..warmup {
        time_cmd(cmd_str, input)?;
    }

    let mut samples = Vec::with_capacity(runs as usize);
    for _ in 0..runs {
        let d = time_cmd(cmd_str, input)?;
        samples.push(d.as_secs_f64() * 1000.0);
    }
    Ok(samples)
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

// ============================================================================
// 模板替换
// ============================================================================

fn render_cmd(template: &str, src: &Path, out: &Path) -> String {
    template
        .replace("%s", &src.to_string_lossy())
        .replace("%o", &out.to_string_lossy())
}

// ============================================================================
// Runner 核心
// ============================================================================

struct RunCtx {
    bench_root: PathBuf,
    filter_bench: Option<String>,
    filter_lang: Option<String>,
}

impl RunCtx {
    fn new(cli: &Cli) -> Self {
        Self {
            bench_root: cli.bench_root.clone(),
            filter_bench: cli.bench.clone(),
            filter_lang: cli.lang.clone(),
        }
    }

    fn run(&self, cfg: &Config) -> Result<()> {
        for (bench_name, bench_def) in &cfg.benchmarks {
            if let Some(filter) = &self.filter_bench {
                if bench_name != filter {
                    continue;
                }
            }

            println!();
            println!("╔══════════════════════════════════════════════════╗");
            println!("║  {:34} ║", bench_name);
            println!("╚══════════════════════════════════════════════════╝");
            if let Some(desc) = &bench_def.description {
                println!("  {}", desc);
            }
            println!();

            let out_dir = self.bench_root.join("bench/out");
            std::fs::create_dir_all(&out_dir)?;

            for (lang_name, lang_def) in &bench_def.languages {
                if let Some(filter) = &self.filter_lang {
                    if lang_name != filter {
                        continue;
                    }
                }

                let src_path = self.bench_root.join(&lang_def.src);
                let out_path = out_dir.join(format!("{}_{}", bench_name, lang_name));

                // 编译阶段
                let compile_time = if let Some(compile) = &lang_def.compile {
                    let cmd = render_cmd(&compile.cmd, &src_path, &out_path);
                    print!("  [{:>10}] 编译 ... ", lang_name);
                    std::io::Write::flush(&mut std::io::stdout())?;
                    match time_cmd(&cmd, "") {
                        Ok(d) => {
                            let ms = d.as_secs_f64() * 1000.0;
                            println!("{}", fmt_duration(ms));
                            Some(ms)
                        }
                        Err(e) => {
                            println!("失败: {}", e);
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
                        let run_cmd = render_cmd(&run_def.cmd, &src_path, &out_path);

                        print!("  [{:>10}] 输入={:>6} 运行 ... ", lang_name, input);
                        std::io::Write::flush(&mut std::io::stdout())?;

                        match run_rounds(&run_cmd, input, bench_def.warmup, bench_def.runs) {
                            Ok(samples) => {
                                let (mean, stddev) = stats(&samples);
                                let compile_info = match compile_time {
                                    Some(ct) => format!(" (编译: {})", fmt_duration(ct)),
                                    None => String::new(),
                                };
                                println!(
                                    "{} ± {} (相对偏差: {:.1}%){}",
                                    fmt_duration(mean),
                                    fmt_duration(stddev),
                                    if mean > 0.0 {
                                        (stddev / mean) * 100.0
                                    } else {
                                        0.0
                                    },
                                    compile_info,
                                );
                            }
                            Err(e) => {
                                println!("失败: {}", e);
                            }
                        }
                    }
                }
            }
            println!();
        }
        Ok(())
    }
}

// ============================================================================
// 入口
// ============================================================================

fn main() -> Result<()> {
    let cli = Cli::parse();

    let yaml_content =
        std::fs::read_to_string(&cli.config).context("读取 bench.yaml 失败")?;
    let cfg: Config = serde_yaml::from_str(&yaml_content).context("解析 bench.yaml 失败")?;

    let ctx = RunCtx::new(&cli);
    ctx.run(&cfg)
}