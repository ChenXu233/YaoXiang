//! #454：stdlib 原生导出的命名实参（3.16 端到端值级 oracle）
//!
//! 规范来源:
//! - #452 方法级命名实参语义（位置按声明序占位、命名按形参名绑定）
//! - 用户 2026-10-14 裁决⑥（禁止静默择一/静默给错答案）
//! - D59 C 类（静默错值可随重构顺路修，条件：先红后绿 + 不改变原先成功程序）
//!
//! 红态（修复前实测，rev 携带 45418b717 的主仓二进制）：
//! `io.format_fallback(value = 42, type_name = "int")` 与
//! `io.format_fallback(value = 42, kind = "int")` 都 exit=0 输出 `()`
//! ——命名实参被整体丢弃（原生注册点只带签名串、不带形参名表）。
//! 本文件用 `yaoxiang::run` 跑**真实驱动链**，值级 oracle 用程序内 assert 表达。

use yaoxiang::run;

/// 位置实参对照（回归锚）：40 + 2 = 42。
#[test]
fn test_native_positional_args_still_bind_by_declaration_order() {
    // Arrange / Act / Assert
    run(r#"
use std.{assert, io}
main: () -> Void = {
    s = io.format_fallback(42, "int")
    assert.assert(s == "int(42)", "positional must still bind in order")
}
"#)
    .expect("位置实参调用必须成功");
}

/// 纯命名（形参名正确）→ 值真的送到形参位。
#[test]
fn test_native_named_args_bind_by_parameter_name() {
    // Arrange / Act / Assert
    run(r#"
use std.{assert, io}
main: () -> Void = {
    s = io.format_fallback(value = 42, type_name = "int")
    assert.assert(s == "int(42)", "named args must bind by name")
}
"#)
    .expect("纯命名实参（形参名正确）必须成功");
}

/// 混合（位置 + 命名）：位置占前导槽位、命名按名入槽，两者都真的送达。
#[test]
fn test_native_mixed_positional_and_named_args_both_arrive() {
    // Arrange / Act / Assert
    run(r#"
use std.{assert, io}
main: () -> Void = {
    s = io.format_fallback(42, type_name = "int")
    assert.assert(s == "int(42)", "positional first, then named")
}
"#)
    .expect("混合实参必须成功");
}

/// 断言侧同类：assert.assert 的形参名与签名一致（cond / msg）⇒ 通过。
/// 名字不符（message）⇒ 编译期 E1014，由
/// `typecheck::tests::named_args_loud::test_native_assert_named_args_checked_by_signature` 钉住。
#[test]
fn test_native_assert_named_args_match_signature() {
    // Arrange / Act / Assert
    run(r#"
use std.assert
main: () -> Void = {
    assert.assert(cond = 1 == 1, msg = "named on assert")
}
"#)
    .expect("assert 的形参名与签名一致（cond / msg）时必须通过");
}

/// 未知名 ⇒ 编译期 E1014（红态是 exit=0 静默输出 ()）。
#[test]
fn test_native_unknown_named_parameter_is_rejected_at_compile_time() {
    // Arrange
    let source = r#"
use std.io
main: () -> Void = {
    s = io.format_fallback(value = 42, kind = "int")
    io.println(s)
}
"#;

    // Act
    let err = run(source).expect_err("形参名不存在必须在编译期报错");

    // Assert
    let msg = format!("{err:#}");
    assert!(
        msg.contains("E1014"),
        "未知名必须报 E1014（红态是静默 ()）；实际错误：{msg}"
    );
}
