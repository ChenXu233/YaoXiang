//! 编译单元 —— 02-stage-contract §编译器改动清单（driver/unit.rs）。
//!
//! `PerModule` 作用域阶段的执行粒度：单文件程序 1 个单元，
//! 多文件程序 N 个单元，嵌入 std 模块以虚拟路径（`<std.test>`）为单元。

use std::path::PathBuf;

/// 一个编译单元（一个模块的一份源码）。
#[derive(Clone, Debug)]
pub struct Unit {
    /// 模块键（限定名前缀，如 `app` / `std.test`）
    pub key: String,
    /// 源路径（嵌入 std 模块为 `<std/test>` 形虚拟路径）
    pub path: PathBuf,
    /// 源码全文
    pub source: String,
}

impl Unit {
    /// 构造一个编译单元。
    pub fn new(
        key: impl Into<String>,
        path: impl Into<PathBuf>,
        source: impl Into<String>,
    ) -> Self {
        Self {
            key: key.into(),
            path: path.into(),
            source: source.into(),
        }
    }
}
