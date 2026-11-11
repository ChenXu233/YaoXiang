//! 字节码域测试模块
//!
//! 测试与被测实现同域（D1 6.7.1b'）：`bytecode.rs` 覆盖字节码 IR（`mod.rs`），
//! `file.rs` 覆盖 `.42` 容器与序列化（`file.rs`）；opcode 词表的编码/解码对拍
//! 也在 `bytecode.rs` 内（其余三支是对 `opcode.rs` / `mod.rs` / `translator.rs` 的源码级对拍）。

pub mod bytecode;
pub mod file;
