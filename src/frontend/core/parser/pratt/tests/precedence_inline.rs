//! Precedence 模块测试
//!
//! 规范来源：pratt/precedence.rs 的绑定力常量。
//! 历史：本文件由 P1 复活；原先还覆盖 P8 删除的死阶梯 `Precedence` /
//! `PrecedenceContext`，8.0.1 随死代码一并删除（见 09a §P8 8.0），
//! 现只保留活常量的相对顺序断言。

use crate::frontend::core::parser::pratt::precedence::{
    BP_ASSIGN, BP_CALL, BP_COMPARISON, BP_EQUALITY, BP_FACTOR, BP_HIGHEST, BP_LOGICAL_AND,
    BP_LOGICAL_OR, BP_LOWEST, BP_TERM,
};

#[test]
#[allow(clippy::assertions_on_constants)]
fn test_bp_ordering_ascends_from_lowest_to_highest() {
    assert!(
        BP_LOWEST < BP_ASSIGN,
        "BP_LOWEST 必须低于赋值层: {BP_LOWEST} vs {BP_ASSIGN}"
    );
    assert!(
        BP_ASSIGN < BP_LOGICAL_OR,
        "赋值层必须低于逻辑或: {BP_ASSIGN} vs {BP_LOGICAL_OR}"
    );
    assert!(
        BP_LOGICAL_AND < BP_EQUALITY,
        "逻辑与必须低于相等比较: {BP_LOGICAL_AND} vs {BP_EQUALITY}"
    );
    assert!(
        BP_COMPARISON < BP_TERM,
        "比较层必须低于加减层: {BP_COMPARISON} vs {BP_TERM}"
    );
    assert!(
        BP_TERM < BP_FACTOR,
        "加减层必须低于乘除层: {BP_TERM} vs {BP_FACTOR}"
    );
    assert!(
        BP_FACTOR <= BP_CALL,
        "乘除层不得高于调用层: {BP_FACTOR} vs {BP_CALL}"
    );
    assert!(
        BP_CALL < BP_HIGHEST,
        "调用层必须低于最高层: {BP_CALL} vs {BP_HIGHEST}"
    );
}
