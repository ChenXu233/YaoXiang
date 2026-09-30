//! 构建系统测试入口 — 基于 RFC-014b
//!
//! 按源文件对应（规则 1.3）：
//! - strategy.rs     ← mod.rs（BuildStrategy / 平台三元组 / 安装决策树）
//! - requirements.rs ← requirements.rs（[build.requirements] 预检）
//! - cargo.rs        ← cargo.rs（cargo 构建策略）
//! - binaries.rs     ← binaries.rs（[binaries] 预编译分发）
//! - custom.rs       ← custom.rs（build.yx + 信任门）

mod binaries;
mod cargo;
mod custom;
mod requirements;
mod strategy;

/// 在路径写入文本文件（含父目录创建）
pub(crate) fn write(
    path: &std::path::Path,
    content: &str,
) {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).expect("create parent dirs for fixture");
    }
    std::fs::write(path, content)
        .unwrap_or_else(|e| panic!("write fixture {}: {e}", path.display()));
}
