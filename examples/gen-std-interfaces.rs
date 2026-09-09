//! 重新生成仓库预生成标准库接口视图（RFC-037 治愈工具）
//!
//! `src/std/interfaces/*.yx` 是发行包 `lib/yaoxiang/std/` native 层的唯一来源，
//! 由 `StdModule::exports()` 派生。std 接口变更后运行本工具重写预生成文件，
//! 同步门禁 `test_committed_interface_files_match_generation` 随即恢复绿色
//! （与 `tools/code-tables --fix` 同构的治愈路径）。
//!
//! 用法：`cargo run --example gen-std-interfaces`

fn main() {
    let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src/std/interfaces");
    let count = yaoxiang::std::gen_interfaces::write_interfaces_to_dir(&dir)
        .unwrap_or_else(|e| panic!("写入接口视图到 {} 失败: {e}", dir.display()));
    println!(
        "gen-std-interfaces: wrote {count} files to {}",
        dir.display()
    );
}
