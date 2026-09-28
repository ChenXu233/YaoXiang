//! `util::arith` 模块的单元测试
//!
//! 覆盖 RFC-011b「`%` 语义修正」：`%` 为 floor 取模（结果符号跟随除数，
//! Python 语义），非截断余数、非欧几里得取模。
//!
//! RFC-011b 动机 §5：语言参考优先级表早已写「乘除取模」，此前实现给的是
//! 截断余数（`-7 % 3 == -1`），属实现违反已发布文档的缺陷，不留兼容期。
//!
//! 本测试的期望值全部从 RFC-011b 列出的例子推导，不从当前实现输出反推。

use crate::util::arith::{floor_mod_f64, floor_mod_i64};

/// RFC-011b §「`%` 语义修正」：符号跟随除数。
///
/// 期望值取自 RFC 明文（`-7 % 3` 从 `-1` 变为 `2`），非当前实现输出。
#[test]
fn test_floor_mod_int_sign_follows_divisor() {
    // Arrange + Act + Assert：逐条对照 RFC 的符号约定
    assert_eq!(
        floor_mod_i64(-7, 3),
        Some(2),
        "-7 % 3 应为 2（非截断余数 -1）"
    );
    assert_eq!(
        floor_mod_i64(7, -3),
        Some(-2),
        "7 % -3 应为 -2（符号随除数）"
    );
    assert_eq!(floor_mod_i64(-7, -3), Some(-1), "-7 % -3 应为 -1");
    assert_eq!(floor_mod_i64(7, 3), Some(1), "7 % 3 应为 1");
    assert_eq!(floor_mod_i64(-6, 3), Some(0), "整除时结果恒 0（不产生 -0）");
}

/// 溢出边界：`MIN % -1` 是整型溢出的真实情形，必须报错而非 panic。
///
/// 返回 `None` 让调用方（解释器 `int_op` / 编译期求值）走各自的溢出诊断；
/// 若此处 panic，用户看到的是编译器崩溃而非语言错误。
#[test]
fn test_floor_mod_int_overflow_returns_none() {
    // Arrange
    let (min, minus_one) = (i64::MIN, -1i64);

    // Act
    let result = floor_mod_i64(min, minus_one);

    // Assert
    assert_eq!(result, None, "MIN % -1 必须报溢出（None）而非 panic");
}

/// 浮点实现与整型同语义（符号跟随除数，含负数与小数）。
#[test]
fn test_floor_mod_float_sign_follows_divisor() {
    // Arrange + Act + Assert
    assert_eq!(floor_mod_f64(-7.5, 3.0), 1.5, "-7.5 % 3.0 应为 1.5");
    assert_eq!(floor_mod_f64(2.5, -1.5), -0.5, "2.5 % -1.5 应为 -0.5");
    assert_eq!(floor_mod_f64(7.0, 3.0), 1.0, "7.0 % 3.0 应为 1.0");
}

/// 两条实现（整型 / 浮点）在所有小范围内必须给出同一语义。
///
/// 存在意义：`%` 在解释器里有整型臂和浮点臂两条路径，历史缺陷正是
/// 「符号约定在一边改了、另一边漏改」；本测试把它们钉在一起。
#[test]
fn test_int_and_float_implementations_agree() {
    // Arrange
    let pairs: Vec<(i64, i64)> = (-7i64..=7)
        .flat_map(|a| [-3i64, -2, 2, 3].map(move |b| (a, b)))
        .collect();

    // Act + Assert
    for (a, b) in pairs {
        let int_result = floor_mod_i64(a, b).expect("测试范围内不应溢出");
        let float_result = floor_mod_f64(a as f64, b as f64);

        assert_eq!(
            int_result as f64, float_result,
            "整型与浮点 floor 取模必须同语义：{a} % {b}"
        );
    }
}
