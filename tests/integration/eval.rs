//! `yaoxiang eval` 命令集成测试
//!
//! 设计文档: docs/superpowers/specs/2026-07-05-yaoxiang-eval-design.md
//! 测试 eval_code() 函数在脚本模式下的行为。

use yaoxiang::eval_code;

#[test]
fn test_eval_single_line() {
    assert!(
        eval_code("print(\"hello\")").is_ok(),
        "eval 应能自动包装无 main 的代码"
    );
}

#[test]
fn test_eval_main_defined() {
    assert!(
        eval_code("main = { print(\"with main\") }").is_ok(),
        "eval 有 main 时不应包装，直接执行"
    );
}

#[test]
fn test_eval_use_no_main() {
    assert!(
        eval_code("use std::io; println(\"hello from eval\")").is_ok(),
        "eval 应支持 use 导入 + 无 main 场景"
    );
}

#[test]
fn test_eval_use_and_main_defined() {
    assert!(
        eval_code("use std::fmt; main = { print(\"hi\") }").is_ok(),
        "eval 应支持 use 导入 + main 同时存在"
    );
}

#[test]
fn test_eval_syntax_error() {
    let result = eval_code("print('unclosed string");
    assert!(result.is_err(), "语法错误时应返回错误而非 panic");
}
