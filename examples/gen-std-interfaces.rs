//! 重新生成仓库预生成标准库接口视图（RFC-037 治愈工具）
//!
//! `src/std/interfaces/*.yx` 是发行包 `lib/yaoxiang/std/` native 层的唯一来源，
//! 由 `StdModule::exports()` 派生。std 接口变更后运行本工具重写预生成文件，
//! 同步门禁 `test_committed_interface_files_match_generation` 随即恢复绿色
//! （与 `tools/code-tables --fix` 同构的治愈路径）。孤儿文件一并清理。
//!
//! 用法：`cargo run --example gen-std-interfaces`

use std::collections::HashSet;

fn main() {
    let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src/std/interfaces");
    let interfaces = yaoxiang::std::gen_interfaces::generate_all_interfaces();
    let count = yaoxiang::std::gen_interfaces::write_interfaces_to_dir(&dir)
        .unwrap_or_else(|e| panic!("写入接口视图到 {} 失败: {e}", dir.display()));

    // 清理生成器不再产出的孤儿文件（同步门禁的反向检查项）
    let valid: HashSet<String> = interfaces.iter().map(|(n, _)| format!("{n}.yx")).collect();
    let mut removed = 0;
    let entries =
        std::fs::read_dir(&dir).unwrap_or_else(|e| panic!("读取 {} 失败: {e}", dir.display()));
    for entry in entries {
        let path = entry
            .unwrap_or_else(|e| panic!("读取 {} 条目失败: {e}", dir.display()))
            .path();
        let Some(name) = path.file_name().and_then(|n| n.to_str()) else {
            continue;
        };
        if path.extension().is_some_and(|ext| ext == "yx") && !valid.contains(name) {
            std::fs::remove_file(&path)
                .unwrap_or_else(|e| panic!("删除孤儿文件 {} 失败: {e}", path.display()));
            removed += 1;
            println!("gen-std-interfaces: removed orphan {name}");
        }
    }

    println!(
        "gen-std-interfaces: wrote {count} files to {} (orphans removed: {removed})",
        dir.display()
    );
}
