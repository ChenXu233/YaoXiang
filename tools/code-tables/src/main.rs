//! code-tables 命令行：RFC-013 码表区间的检查与治愈（#326）
//!
//! 用法（仓库根目录）：
//!   cargo run --manifest-path tools/code-tables/Cargo.toml              # = --check
//!   cargo run --manifest-path tools/code-tables/Cargo.toml -- --check   # 干跑，区间漂移时 exit 1
//!   cargo run --manifest-path tools/code-tables/Cargo.toml -- --fix     # 就地重写区间

use code_tables::{check_rfc_tables, fix_rfc_tables, parse_registry, validate, RFC013_REL};
use std::path::Path;

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let mode = args.iter().find(|a| *a == "--check" || *a == "--fix");
    let fix = matches!(mode.map(String::as_str), Some("--fix"));
    if args.iter().any(|a| a == "-h" || a == "--help") {
        eprintln!("用法: code-tables [--check|--fix]");
        std::process::exit(2);
    }

    let root = Path::new(".");
    let entries = match parse_registry(root) {
        Ok(e) => e,
        Err(msg) => {
            eprintln!("error: {msg}");
            std::process::exit(1);
        }
    };

    // 注册表/locales 校验（与 build.rs 门槛同一函数）
    let report = validate(root, &entries).expect("校验失败");
    for w in &report.warnings {
        eprintln!("warning: {w}");
    }
    for e in &report.errors {
        eprintln!("error: {e}");
    }

    // RFC-013 码表区间
    let doc_path = root.join(RFC013_REL);
    let doc = std::fs::read_to_string(&doc_path).unwrap_or_else(|e| {
        eprintln!("error: 读取 {} 失败: {}", doc_path.display(), e);
        std::process::exit(1);
    });
    let zh: serde_json::Value = serde_json::from_str(
        &std::fs::read_to_string(root.join("locales/zh.json")).expect("读 zh.json"),
    )
    .expect("解析 zh.json");

    if fix {
        match fix_rfc_tables(&doc, &entries, &zh) {
            Ok((new_doc, n)) => {
                if new_doc == doc {
                    println!(
                        "RFC-013 码表区间已一致（{} 段），未改动",
                        code_tables::RFC_SEGMENTS.len()
                    );
                } else {
                    std::fs::write(&doc_path, new_doc)
                        .unwrap_or_else(|e| panic!("写入 {} 失败: {}", doc_path.display(), e));
                    println!("已重写 {n} 个码表区间: {}", doc_path.display());
                }
            }
            Err(msg) => {
                eprintln!("error: {msg}");
                std::process::exit(1);
            }
        }
    } else {
        let mismatches = check_rfc_tables(&doc, &entries, &zh);
        for m in &mismatches {
            eprintln!("error: {m}");
        }
        if !mismatches.is_empty() {
            eprintln!("运行 code-tables --fix 治愈");
            std::process::exit(1);
        }
        println!(
            "RFC-013 码表区间与注册表一致（{} 段）",
            code_tables::RFC_SEGMENTS.len()
        );
    }

    if !report.is_ok() {
        std::process::exit(1);
    }
}
