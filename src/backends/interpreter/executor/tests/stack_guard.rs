//! 解释器深度守卫可达性测试 — WBS 3.9.1（09-execution-wbs.md）
//!
//! 规范锚点：
//! - `docs/src/dev/architecture/09-execution-wbs.md` §3.9.1：解释器调用深度守卫必须可达
//!   ——深度超限必然产生 E6004（含真实 limit），而非耗尽进程原生栈崩溃；
//!   良构程序的对外行为不变。
//! - E6004 定义：`src/util/diagnostic/codes/e6xxx.rs`（`stack_overflow(limit)`），
//!   报文模板见 `locales/zh.json`：「递归深度超出限制 {limit}」。
//! - 守卫上限：`ExecutorConfig::max_stack_depth`（`src/backends/mod.rs`，默认 1024）。
//!
//! 深度计价：深度守卫在 `push_frame` 拒绝 `call_stack.len() >= max_stack_depth`——
//! 即**同时驻留的帧数上限 = max_stack_depth**。宿主入口帧计入（`execute_function`
//! 压入被调函数帧），故「深度 d」= 被调函数帧 + d-1 层递归帧。

use crate::backends::common::RuntimeValue;
use crate::backends::interpreter::executor::Interpreter;
use crate::backends::{Executor, ExecutorConfig, ExecutorError};
use crate::middle::bytecode::{
    BinaryOp, BytecodeFunction, BytecodeInstr, CompareOp, ConstValue, Label, Reg,
};
use std::collections::HashMap;

/// `count_down(n) = if n == 0 { 0 } else { 1 + count_down(n - 1) }`
/// ——**非尾**递归：返回值由调用者做 `+ 1`，调用者帧必须存活到被调返回，
/// 深度逐层 +1（尾调用复用帧的形态不适用本测试）。
fn countdown_function() -> BytecodeFunction {
    BytecodeFunction {
        name: "count_down".to_string(),
        params: vec![crate::frontend::core::typecheck::MonoType::Void],
        return_type: crate::frontend::core::typecheck::MonoType::Void,
        local_count: 8,
        local_names: HashMap::new(),
        upvalue_count: 0,
        instructions: vec![
            // 0: r0 = n（参数由 Frame::with_args 铺进 slots[0]）
            BytecodeInstr::LoadArg {
                dst: Reg(0),
                arg_idx: 0,
            },
            // 1: r1 = 0
            BytecodeInstr::LoadConst {
                dst: Reg(1),
                const_idx: 0,
            },
            // 2: r2 = (n == 0)
            BytecodeInstr::Compare {
                dst: Reg(2),
                lhs: Reg(0),
                rhs: Reg(1),
                cmp: CompareOp::Eq,
            },
            // 3: if !r2 goto 6（相对偏移 3：apply_jump 以当前 ip 为基准）
            BytecodeInstr::JmpIfNot {
                cond: Reg(2),
                target: Label(3),
            },
            // 4-5: return 0
            BytecodeInstr::LoadConst {
                dst: Reg(3),
                const_idx: 0,
            },
            BytecodeInstr::ReturnValue { value: Reg(3) },
            // 6: r4 = 1
            BytecodeInstr::LoadConst {
                dst: Reg(4),
                const_idx: 1,
            },
            // 7: r5 = n - 1
            BytecodeInstr::BinaryOp {
                dst: Reg(5),
                lhs: Reg(0),
                rhs: Reg(4),
                op: BinaryOp::Sub,
            },
            // 8: r6 = count_down(r5)
            BytecodeInstr::CallStatic {
                dst: Some(Reg(6)),
                func: 0,
                args: vec![Reg(5)],
            },
            // 9: r7 = 1 + r6（非尾：调用点之后仍有运算）
            BytecodeInstr::BinaryOp {
                dst: Reg(7),
                lhs: Reg(4),
                rhs: Reg(6),
                op: BinaryOp::Add,
            },
            // 10: return r7
            BytecodeInstr::ReturnValue { value: Reg(7) },
        ],
        labels: HashMap::new(),
        exception_handlers: vec![],
        debug_map: HashMap::new(),
    }
}

/// 预装 `count_down`（函数表索引 0）与常量 0/1 的解释器。
fn countdown_interpreter(max_stack_depth: usize) -> Interpreter {
    let mut interp = Interpreter::with_config(ExecutorConfig { max_stack_depth });
    let image = std::sync::Arc::get_mut(&mut interp.image).expect("image 应独占（无其它引用）");
    image.constants.push(ConstValue::Int(0));
    image.constants.push(ConstValue::Int(1));
    image.functions_by_id.push(countdown_function());
    interp
}

/// 从镜像取一份 `count_down`（`execute_function` 按名解析函数表索引）。
fn countdown_of(interp: &Interpreter) -> BytecodeFunction {
    interp.image.functions_by_id[0].clone()
}

/// 验收给出的上限样本：极小值、常规值、默认值、大值。
const LIMIT_SAMPLES: [usize; 4] = [5, 64, 1024, 4096];

#[test]
fn test_countdown_recursion_just_below_limit_returns_value() {
    for limit in LIMIT_SAMPLES {
        // Arrange — 上限 limit，入参 limit - 1 ⇒ 驻留帧数恰为 limit（被调帧 + 递归帧）
        let mut interp = countdown_interpreter(limit);
        let func = countdown_of(&interp);
        let depth = (limit - 1) as i64;

        // Act
        let result = interp.execute_function(&func, &[RuntimeValue::Int(depth)]);

        // Assert — 上限内的良构递归必须正常返回值（守卫不得误伤）
        assert_eq!(
            result.expect("上限内的良构递归应返回值"),
            RuntimeValue::Int(depth),
            "max_stack_depth={limit} 时深度上限内的递归应返回 {depth}"
        );
    }
}

#[test]
fn test_countdown_recursion_at_limit_reports_e6004_with_real_limit() {
    for limit in LIMIT_SAMPLES {
        // Arrange — 入参 limit ⇒ 需要 limit + 1 帧，超出上限一帧
        let mut interp = countdown_interpreter(limit);
        let func = countdown_of(&interp);

        // Act
        let err = interp
            .execute_function(&func, &[RuntimeValue::Int(limit as i64)])
            .expect_err("深度超限应报 E6004，而不是耗尽进程原生栈崩溃");

        // Assert — 码与 limit 同源：limit 必须是配置的真实上限，而不是占位 0
        assert!(
            matches!(&err, ExecutorError::StackOverflow { limit: l, .. } if *l == limit),
            "max_stack_depth={limit} 时应为 StackOverflow {{ limit: {limit} }}，实际 {err:?}"
        );
    }
}

#[test]
fn test_deep_recursion_consumes_heap_not_native_stack() {
    // Arrange — 深递归跑在 512 KiB 原生栈的线程上：若每层解释型调用仍消耗原生栈
    // （修复前约 1 KiB/层），2 万层必然爆栈；蹦床后原生栈用量与深度无关。
    const DEPTH: usize = 20_000;
    let handle = std::thread::Builder::new()
        .name("stack-guard-probe".to_string())
        .stack_size(512 * 1024)
        .spawn(|| {
            let mut interp = countdown_interpreter(DEPTH + 1);
            let func = countdown_of(&interp);
            interp.execute_function(&func, &[RuntimeValue::Int(DEPTH as i64)])
        })
        .expect("探测线程应能创建");

    // Act
    let result = handle
        .join()
        .expect("探测线程不应崩溃（崩即原生栈被解释型深度耗尽）");

    // Assert
    assert_eq!(
        result.expect("深递归应返回值"),
        RuntimeValue::Int(DEPTH as i64),
        "解释型深度 {DEPTH} 应只消耗堆，不消耗原生栈"
    );
}
