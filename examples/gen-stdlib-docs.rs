//! 重新生成标准库 API 文档的生成区（混合模式治愈工具）
//!
//! `docs/src/reference/stdlib/*.md` 里标记区间内的内容由
//! `StdModule::exports()` 派生；`exports()` 变更后运行本工具重写生成区，
//! 同步门禁随即恢复绿色（与 `gen-std-interfaces`、`tools/code-tables --fix`
//! 同构的治愈路径）。标记区间外的叙述与示例是手写的，本工具不触碰。
//!
//! 用法：`cargo run --example gen-stdlib-docs`

fn main() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));

    match yaoxiang::std::gen_docs::fix_stdlib_docs(root) {
        Ok(count) => {
            println!(
                "gen-stdlib-docs: rewrote {count} file(s) under {}",
                yaoxiang::std::gen_docs::STDLIB_DOCS_REL
            );
            let errors = yaoxiang::std::gen_docs::check_stdlib_docs(root);
            if !errors.is_empty() {
                for e in &errors {
                    eprintln!("error: {e}");
                }
                std::process::exit(1);
            }
            println!("gen-stdlib-docs: all generation regions match StdModule::exports()");
        }
        Err(msg) => {
            eprintln!("error: {msg}");
            std::process::exit(1);
        }
    }
}
