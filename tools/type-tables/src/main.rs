//! type-tables 命令行：T1/T2 的报告。
//!
//! **T1 已是构建门禁**（6.6.2，2026-10-30）：`cargo build` 经根 `build.rs` 调 [`type_tables::build_gate`]，
//! 违规即 panic；本 bin 的 `--report` 输出同一份判定的完整报告（T2 仍只报告）。
//!
//! 用法（仓库根目录）：
//!   cargo run --manifest-path tools/type-tables/Cargo.toml -- --report   # T1/T2 文本报告
//!   cargo run --manifest-path tools/type-tables/Cargo.toml -- --json     # T1 结构化结果
//!   cargo run --manifest-path tools/type-tables/Cargo.toml -- --root <仓库根>

use std::path::PathBuf;
use std::process::ExitCode;

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    if args.iter().any(|a| a == "-h" || a == "--help") {
        eprintln!("用法: type-tables [--report|--json] [--root <仓库根>]");
        return ExitCode::SUCCESS;
    }

    let mut root = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
    let mut want_json = false;
    let mut idx = 0;
    while idx < args.len() {
        match args[idx].as_str() {
            "--report" => idx += 1,
            "--json" => {
                want_json = true;
                idx += 1;
            }
            "--root" => {
                let Some(path) = args.get(idx + 1) else {
                    eprintln!("error: --root 需要路径参数");
                    return ExitCode::FAILURE;
                };
                root = PathBuf::from(path);
                idx += 2;
            }
            other => {
                eprintln!("error: 未知参数 {other}（支持 --report / --json / --root <路径>）");
                return ExitCode::FAILURE;
            }
        }
    }

    let result = if want_json {
        type_tables::report_json(&root)
    } else {
        type_tables::report(&root)
    };
    match result {
        Ok(text) => {
            println!("{text}");
            ExitCode::SUCCESS
        }
        Err(msg) => {
            eprintln!("error: {msg}");
            ExitCode::FAILURE
        }
    }
}
