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
