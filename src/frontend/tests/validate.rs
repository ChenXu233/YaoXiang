//! 前端验证模块测试
//!
//! 测试 `validate_source` 函数的行为，覆盖：
//! - 空输入处理
//! - 语义错误检测（未声明变量）
//! - 合法代码验证
//! - 语法错误检测
//! - 缓存命中行为

use crate::frontend::validate::validate_source;

#[test]
fn test_empty_input_produces_no_diagnostics() {
    let result = validate_source("");
    assert!(
        result.diagnostics.is_empty(),
        "空输入不应产生诊断，实际得到 {} 个",
        result.diagnostics.len()
    );
    assert!(result.module.is_some(), "空输入应产生模块 AST");
}

#[test]
fn test_undeclared_let_produces_diagnostics() {
    // `let` 作为未声明的标识符使用 —— 解析成功但类型检查报错
    let result = validate_source("let x = 1");
    assert!(!result.diagnostics.is_empty(), "未声明的 `let` 应产生诊断");
    assert!(result.module.is_some(), "解析成功时模块 AST 应存在");
}

#[test]
fn test_valid_assignment_no_diagnostics() {
    let result = validate_source("x = 1");
    assert!(
        result.diagnostics.is_empty(),
        "合法赋值不应产生诊断，实际得到 {} 个",
        result.diagnostics.len()
    );
    assert!(result.module.is_some(), "合法赋值应产生模块 AST");
}

#[test]
fn test_cache_hit_returns_equivalent_result() {
    // 连续两次调用相同源码应返回相同结果（内容等价 + 缓存命中）
    let a = validate_source("x = 1");
    let b = validate_source("x = 1");
    assert_eq!(
        a.diagnostics.len(),
        b.diagnostics.len(),
        "缓存命中时两次调用的诊断数量应相同"
    );
    assert_eq!(
        a.module.is_some(),
        b.module.is_some(),
        "缓存命中时两次调用的模块存在性应相同"
    );
}

#[test]
fn test_syntax_error_returns_no_module() {
    // `@` 不能作为语句开始 —— 显式产生语法错误
    let result = validate_source("@");
    assert!(!result.diagnostics.is_empty(), "语法错误应产生诊断");
    assert!(result.module.is_none(), "语法错误时模块 AST 应为 None");
}
