//! 调用栈捕获测试
//!
//! 测试覆盖内容：
//! - `capture_stack()` 的帧数量与顺序
//! - T1 重构护栏：frame 原地驻留后，当前帧不得被重复计入
//!
//! ## 背景一：现状缺陷——调用链丢失（2026-09-18 实测）
//!
//! `step_one` 采用「pop 当前帧 → 执行一条指令 → push 回去」的循环
//! （`debug.rs:99-116`）。这意味着**执行任意一条指令时，所有调用者帧都不在
//! `call_stack` 上**。因此 `capture_stack()` 只能看到最内层一帧。
//!
//! 实测证据（HEAD @ `5028c5e1`）：
//!
//! ```text
//! $ yaoxiang run nest_err2.yx     # main → deep1 → deep2 → deep3(10/0)
//! error [E6001] Expression 10 / 0 divides by zero
//! stack trace:
//!   at deep3 (...)              ← 只有 1 帧，deep1/deep2/main 全丢
//! ```
//!
//! ## 背景二：构造错误触发点的注意事项
//!
//! 本文件用 `String` 作为 `JmpIf` 的条件来触发类型错误，**不能用 `Int`**：
//! `RuntimeValue::to_bool()`（`value.rs:332-338`）对 `Int` 返回 `Some(i != 0)`
//! ——Int 被静默当作 Bool。用 `Int` 不会报错，而若同时给出偏移为 0 的
//! `Label`，`frame.ip = ip + 0` 会造成**原地跳转死循环**（实测挂死 60s+）。
//!
//! ## 本文件的作用
//!
//! T1 把 frame 改为原地驻留后，调用者帧会全部保留在 `call_stack` 上，
//! 栈帧将**从 1 帧变为完整链**。`full_chain` 测试按 T1 的目标行为书写，
//! 故当前标记 `#[ignore]`。

use crate::backends::Executor;
use crate::backends::interpreter::executor::Interpreter;
use crate::middle::bytecode::{BytecodeFunction, BytecodeInstr, ConstValue, Label, Reg};
use std::collections::HashMap;

/// 构造一个指定名字与指令序列的函数
fn make_named_function(
    name: &str,
    instrs: Vec<BytecodeInstr>,
) -> BytecodeFunction {
    BytecodeFunction {
        name: name.to_string(),
        params: vec![],
        return_type: crate::middle::core::ir::Type::Void,
        local_count: 4,
        local_names: HashMap::new(),
        upvalue_count: 0,
        instructions: instrs,
        labels: HashMap::new(),
        exception_handlers: vec![],
        debug_map: HashMap::new(),
    }
}

/// 构造 `outer` → `inner` 两层调用，`inner` 因 JmpIf 条件为 String 报类型错误。
///
/// 函数表：0 = outer（CallStatic inner），1 = inner（LoadConst String → JmpIf）
///
/// `JmpIf` 的 `Label(0)` 不会被实际使用——条件为 String 时 `to_bool()` 返回
/// `None`，在跳转发生前即报错返回。（用 `Int` 条件则会静默当 Bool 并在
/// 偏移 0 处原地死循环，详见文件头背景二。）
fn interp_with_nested_call_error() -> Interpreter {
    let mut interp = Interpreter::new();

    // 常量 0 为 String——非 Bool 且不可转换，保证触发类型错误
    interp
        .image
        .constants
        .push(ConstValue::String("not-a-bool".into()));

    let inner = make_named_function(
        "inner",
        vec![
            BytecodeInstr::LoadConst {
                dst: Reg(0),
                const_idx: 0,
            },
            BytecodeInstr::JmpIf {
                cond: Reg(0),
                target: Label(0),
            },
            BytecodeInstr::Return,
        ],
    );

    let outer = make_named_function(
        "outer",
        vec![
            BytecodeInstr::CallStatic {
                dst: None,
                func: 1, // 指向 inner
                args: vec![],
            },
            BytecodeInstr::Return,
        ],
    );

    interp.image.functions_by_id.push(outer);
    interp.image.functions_by_id.push(inner);

    interp
}

/// 单层调用报错时恰好 1 帧——防止「当前帧被重复计入」。
#[test]
fn test_capture_stack_single_frame_not_duplicated() {
    let mut interp = interp_with_nested_call_error();
    let inner = interp.image.functions_by_id[1].clone();

    // Act
    let err = interp
        .execute_function(&inner, &[])
        .expect_err("JmpIf 条件为 String 应当报类型错误");

    // Assert — 恰好 1 帧；若当前帧被重复计入会变 2 帧
    let stack = err.stack_trace().expect("类型错误应携带调用栈");
    assert_eq!(
        stack.len(),
        1,
        "单层调用应恰好 1 个栈帧，实际 {}: {:?}",
        stack.len(),
        stack.iter().map(|f| &f.function_name).collect::<Vec<_>>()
    );
    assert_eq!(stack[0].function_name, "inner");
}

/// 嵌套调用出错时，栈帧应包含完整调用链（outer + inner）。
///
/// **T1 目标行为，当前预期失败**，故标记 `#[ignore]`：
/// `step_one` 的 pop-then-push 使调用者帧脱离 `call_stack`
/// （外层帧在 `execute_instr` 调用 CallStatic 时已经被 pop，
/// 而内层函数在同一个 `execute_instr` 里跑到报错）。
/// T1 把 frame 改为原地驻留后，调用者帧全程保留，此测试即可启用。
#[test]
fn test_capture_stack_nested_call_reports_full_chain() {
    // Arrange
    let mut interp = interp_with_nested_call_error();
    let outer = interp.image.functions_by_id[0].clone();

    // Act — outer 调用 inner，inner 在 JmpIf 处报类型错误
    let err = interp
        .execute_function(&outer, &[])
        .expect_err("JmpIf 条件为 String 应当报类型错误");

    // Assert — 完整链：inner（内）+ outer（外）
    let stack = err.stack_trace().expect("类型错误应携带调用栈");
    let names: Vec<&str> = stack.iter().map(|f| f.function_name.as_str()).collect();
    assert_eq!(
        stack.len(),
        2,
        "嵌套一层应报告 2 个栈帧（inner + outer），实际 {stack:?}"
    );
    assert_eq!(
        names,
        vec!["inner", "outer"],
        "栈帧应由内向外排列，实际 {names:?}"
    );
}

/// 正常返回后调用栈应清空，不残留已返回的帧。
#[test]
fn test_capture_stack_empty_after_normal_return() {
    // Arrange
    let mut interp = Interpreter::new();
    interp.image.constants.push(ConstValue::Int(7));
    let normal = make_named_function(
        "normal",
        vec![
            BytecodeInstr::LoadConst {
                dst: Reg(0),
                const_idx: 0,
            },
            BytecodeInstr::ReturnValue { value: Reg(0) },
        ],
    );

    // Act
    interp
        .execute_function(&normal, &[])
        .expect("正常函数应当执行成功");

    // Assert
    let stack = interp.capture_stack();
    assert!(
        stack.is_empty(),
        "正常返回后调用栈应为空，实际 {:?}",
        stack.iter().map(|f| &f.function_name).collect::<Vec<_>>()
    );
}
