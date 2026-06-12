//! 寄存器文件测试
//!
//! 测试覆盖内容：
//! - RegisterFile 的创建和配置
//! - 寄存器的读写操作
//! - 寄存器的复制操作

use crate::backends::common::RuntimeValue;
use crate::backends::interpreter::registers::{RegisterFile, GENERAL_PURPOSE_REGS};

#[test]
fn test_register_file_new() {
    let rf = RegisterFile::new();
    assert_eq!(rf.len(), GENERAL_PURPOSE_REGS);
}

#[test]
fn test_register_set_get() {
    let mut rf = RegisterFile::new();
    rf.set(0, RuntimeValue::Int(42));
    assert_eq!(rf.at(0).to_int(), Some(42));
}

#[test]
fn test_register_copy() {
    let mut rf = RegisterFile::new();
    rf.set(0, RuntimeValue::Int(42));
    rf.copy(1, 0);
    assert_eq!(rf.at(1).to_int(), Some(42));
}
