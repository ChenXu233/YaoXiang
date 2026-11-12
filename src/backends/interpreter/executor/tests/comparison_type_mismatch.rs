//! 比较指令类型不匹配兜底臂的消息形态 — 基于 RFC-013 与 issue #500
//!
//! RFC-013（accepted/013-error-code-specification.md）§运行时错误值与码贯通：
//! E6007 是 `ExecutorError` 未映射变体的统一落点（编译器/CLI 诊断通道，
//! 经 `runtime_error(message)` 单轨渲染）。
//!
//! issue #500：环内读未初始化顶层值时比较操作数为 Void，消息原样说
//! type mismatch 会把用户引向错误方向——修后带 Void 操作数的比较错误在消息尾
//! 追加初始化顺序 hint（观测陈述 + 条件归因，不断言成因）；**无 Void 操作数时
//! 消息逐字不变**。本文件直接驱动 `BytecodeInstr::Compare` 钉两级消息形态。
//!
//! 集成面锚在 `tests/integration/multifile.rs`（三模块导入环端到端，
//! hint 经 CLI stderr 渲染链验证）；编译期排序比较（Lt 等）拒绝跨类型的
//! 守卫在 `tests/integration/check.rs`。

use std::collections::HashMap;

use crate::backends::interpreter::executor::Interpreter;
use crate::backends::{Executor, ExecutorError};
use crate::middle::bytecode::{BytecodeFunction, BytecodeInstr, CompareOp, ConstValue, Reg};
use crate::frontend::core::sema::MonoType;

/// hint 观测陈述段——与 exec_compare 兜底臂的追加文案逐字同源（断言值）。
const INIT_ORDER_HINT_OBSERVATION: &str =
    "a Void operand usually means a top-level value was read before initialization";
/// hint 的 W1007 指引段——指回编译期模块导入环告警。
const INIT_ORDER_HINT_W1007: &str = "W1007";

/// 构造一个函数：r0 = const[0], r1 = const[1], r2 = r0 <op> r1, return r2
///
/// 操作数经常量池注入（`ConstValue::Void` 直接落 `RuntimeValue::Void`），
/// 与「未初始化全局槽读到 Void」在 exec_compare 看到的操作数形态一致。
fn make_compare_function(op: CompareOp) -> BytecodeFunction {
    BytecodeFunction {
        name: "test_comparison_type_mismatch".to_string(),
        params: vec![],
        return_type: MonoType::Bool,
        local_count: 3,
        local_names: HashMap::new(),
        upvalue_count: 0,
        instructions: vec![
            BytecodeInstr::LoadConst {
                dst: Reg(0),
                const_idx: 0,
            },
            BytecodeInstr::LoadConst {
                dst: Reg(1),
                const_idx: 1,
            },
            BytecodeInstr::Compare {
                cmp: op,
                dst: Reg(2),
                lhs: Reg(0),
                rhs: Reg(1),
            },
            BytecodeInstr::ReturnValue { value: Reg(2) },
        ],
        labels: HashMap::new(),
        exception_handlers: vec![],
        debug_map: HashMap::new(),
    }
}

/// 跑一次比较，失败时取 type_error 的消息文本（非 Type 变体即 panic）
fn compare_error_message(
    lhs: ConstValue,
    rhs: ConstValue,
) -> String {
    // Arrange
    let func = make_compare_function(CompareOp::Eq);
    let mut interp = Interpreter::new();
    std::sync::Arc::get_mut(&mut interp.image)
        .expect("image Arc must be uniquely owned in test")
        .constants
        .push(lhs);
    std::sync::Arc::get_mut(&mut interp.image)
        .expect("image Arc must be uniquely owned in test")
        .constants
        .push(rhs);

    // Act
    let err = interp
        .execute_function(&func, &[])
        .expect_err("类型不匹配的比较必须落兜底臂报错");

    // Assert（辅助函数内 panic 带上下文）
    match err {
        ExecutorError::Type(message, _) => message,
        other => panic!("比较类型不匹配必须落 ExecutorError::Type（E6007 通道），实际: {other:?}"),
    }
}

/// 带 Void 操作数的比较错误：原消息主体逐字保留 + 尾部追加初始化顺序 hint
///（观测陈述段 + W1007 指引段两段齐全）。
#[test]
fn test_compare_void_operand_message_keeps_prefix_and_carries_init_order_hint() {
    // Arrange：Void（未初始化全局槽读出的形态）vs Int
    // Act
    let message = compare_error_message(ConstValue::Void, ConstValue::Int(1));

    // Assert：前缀与修前逐字一致（#500 判据：无 Void 场景之外，既有文本不动），
    // hint 两段各断一次
    assert_eq!(
        message,
        format!(
            "type mismatch in comparison Eq: Void vs Int(1); {INIT_ORDER_HINT_OBSERVATION} — \
             if this happened during module initialization, check the initialization order \
             inside the import cycle (compile-time warning {INIT_ORDER_HINT_W1007} reports \
             that cycle)"
        ),
        "Void 操作数的比较错误消息 = 原文本逐字前缀 + 初始化顺序 hint"
    );
}

/// 无 Void 操作数的比较错误：消息逐字等于裸 type mismatch 文本，**不含** hint
/// 任一段——hint 的触发面只有 Void 操作数（#500 判据：其他场景文案逐字不变）。
#[test]
fn test_compare_without_void_operand_message_is_bare_and_has_no_hint() {
    // Arrange：Int vs String（跨类型相等比较运行期落兜底臂的形态）
    // Act
    let message = compare_error_message(ConstValue::Int(1), ConstValue::String("a".into()));

    // Assert
    assert_eq!(
        message, r#"type mismatch in comparison Eq: Int(1) vs String("a")"#,
        "无 Void 操作数时消息必须与修前逐字一致"
    );
    assert!(
        !message.contains(INIT_ORDER_HINT_OBSERVATION),
        "无 Void 操作数的比较错误不得携带初始化顺序 hint，实际: {message}"
    );
    assert!(
        !message.contains(INIT_ORDER_HINT_W1007),
        "无 Void 操作数的比较错误不得引用 W1007，实际: {message}"
    );
}

/// Void 在**右侧**同样触发 hint——触发判据是「任一操作数为 Void」。
#[test]
fn test_compare_void_on_right_operand_message_carries_hint() {
    // Arrange：Int vs Void（操作数顺序翻转）
    // Act
    let message = compare_error_message(ConstValue::Int(1), ConstValue::Void);

    // Assert：消息主体前缀不变 + hint 在位（只断两段关键片段，主体形态由
    // test_compare_void_operand_message_keeps_prefix_and_carries_init_order_hint 钉）
    assert!(
        message.starts_with(r#"type mismatch in comparison Eq: Int(1) vs Void"#),
        "原消息主体必须保留，实际: {message}"
    );
    assert!(
        message.contains(INIT_ORDER_HINT_OBSERVATION),
        "右侧 Void 也必须触发观测陈述段，实际: {message}"
    );
    assert!(
        message.contains(INIT_ORDER_HINT_W1007),
        "右侧 Void 也必须触发 W1007 指引段，实际: {message}"
    );
}
