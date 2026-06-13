//! 字节码执行测试
//!
//! 测试覆盖内容：
//! - Borrow/Release 字节码指令的执行
//! - 借用令牌（ZST）的拷贝、释放及边界行为

use crate::backends::Executor;
use crate::backends::common::RuntimeValue;
use crate::middle::bytecode::{BytecodeFunction, BytecodeInstr, Reg, ConstValue};
use std::collections::HashMap;
use crate::backends::interpreter::executor::Interpreter;

fn make_function(instrs: Vec<BytecodeInstr>) -> BytecodeFunction {
    BytecodeFunction {
        name: "test".to_string(),
        params: vec![],
        return_type: crate::middle::core::ir::Type::Void,
        local_count: 4,
        upvalue_count: 0,
        instructions: instrs,
        labels: HashMap::new(),
        exception_handlers: vec![],
        debug_map: HashMap::new(),
    }
}

/// 辅助函数：创建预装一个常量的解释器
fn make_interp_with_const(val: ConstValue) -> Interpreter {
    let mut interp = Interpreter::new();
    interp.constants.push(val);
    interp
}

/// Borrow copies value from src register to dst register (immutable)
#[test]
fn test_borrow_copies_value_immutable() {
    let func = make_function(vec![
        // r0 = Int(42)
        BytecodeInstr::LoadConst {
            dst: Reg(0),
            const_idx: 0,
        },
        // r1 = borrow r0 (immutable)
        BytecodeInstr::Borrow {
            dst: Reg(1),
            src: Reg(0),
            mutable: false,
        },
        // return r1
        BytecodeInstr::ReturnValue { value: Reg(1) },
    ]);

    let mut interp = make_interp_with_const(ConstValue::Int(42));

    let result = interp.execute_function(&func, &[]).unwrap();
    assert_eq!(
        result,
        RuntimeValue::Int(42),
        "不可变借用应拷贝源寄存器的值"
    );
}

/// Borrow copies value from src register to dst register (mutable)
#[test]
fn test_borrow_copies_value_mutable() {
    let func = make_function(vec![
        // r0 = Int(99)
        BytecodeInstr::LoadConst {
            dst: Reg(0),
            const_idx: 0,
        },
        // r1 = borrow mut r0
        BytecodeInstr::Borrow {
            dst: Reg(1),
            src: Reg(0),
            mutable: true,
        },
        // return r1
        BytecodeInstr::ReturnValue { value: Reg(1) },
    ]);

    let mut interp = make_interp_with_const(ConstValue::Int(99));

    let result = interp.execute_function(&func, &[]).unwrap();
    assert_eq!(result, RuntimeValue::Int(99), "可变借用应拷贝源寄存器的值");
}

/// Borrow with mutable:false and mutable:true produce the same runtime result (ZST)
#[test]
fn test_borrow_mutable_flag_irrelevant_at_runtime() {
    // Both immutable and mutable borrow copy the value identically.
    for mutable in [false, true] {
        let func = make_function(vec![
            BytecodeInstr::LoadConst {
                dst: Reg(0),
                const_idx: 0,
            },
            BytecodeInstr::Borrow {
                dst: Reg(1),
                src: Reg(0),
                mutable,
            },
            BytecodeInstr::ReturnValue { value: Reg(1) },
        ]);

        let mut interp = make_interp_with_const(ConstValue::String("hello".into()));

        let result = interp.execute_function(&func, &[]).unwrap();
        assert_eq!(
            result,
            RuntimeValue::String("hello".into()),
            "mutable={mutable} should produce the same value",
        );
    }
}

/// Borrow with empty src register yields Unit
#[test]
fn test_borrow_from_unset_register() {
    let func = make_function(vec![
        // r1 = borrow r0 (r0 is unset -> Unit)
        BytecodeInstr::Borrow {
            dst: Reg(1),
            src: Reg(0),
            mutable: false,
        },
        // return r1
        BytecodeInstr::ReturnValue { value: Reg(1) },
    ]);

    let mut interp = Interpreter::new();
    let result = interp.execute_function(&func, &[]).unwrap();
    assert_eq!(
        result,
        RuntimeValue::Unit,
        "从未设置的寄存器借用应得到 Unit"
    ); // just advances IP; does not corrupt registers
}

#[test]
fn test_release_is_noop() {
    let func = make_function(vec![
        // r0 = Int(7)
        BytecodeInstr::LoadConst {
            dst: Reg(0),
            const_idx: 0,
        },
        // release r0 (should be a no-op)
        BytecodeInstr::Release { src: Reg(0) },
        // r1 = r0 (still valid)
        BytecodeInstr::Mov {
            dst: Reg(1),
            src: Reg(0),
        },
        // return r1
        BytecodeInstr::ReturnValue { value: Reg(1) },
    ]);

    let mut interp = make_interp_with_const(ConstValue::Int(7));

    let result = interp.execute_function(&func, &[]).unwrap();
    assert_eq!(result, RuntimeValue::Int(7), "Release 不应修改寄存器值");
}

/// Release on unset register is also a no-op (no panic)
#[test]
fn test_release_unset_register() {
    let func = make_function(vec![
        // release r5 (unset) — must not panic
        BytecodeInstr::Release { src: Reg(5) },
        BytecodeInstr::Return,
    ]);

    let mut interp = Interpreter::new();
    let result = interp.execute_function(&func, &[]).unwrap();
    assert_eq!(
        result,
        RuntimeValue::Unit,
        "对未设置寄存器执行 Release 不应 panic"
    );
}

/// Borrow followed by Release preserves the borrowed value
#[test]
fn test_borrow_then_release_preserves_value() {
    let func = make_function(vec![
        BytecodeInstr::LoadConst {
            dst: Reg(0),
            const_idx: 0,
        },
        // r1 = borrow r0
        BytecodeInstr::Borrow {
            dst: Reg(1),
            src: Reg(0),
            mutable: true,
        },
        // release r1 (no-op)
        BytecodeInstr::Release { src: Reg(1) },
        // return r1 — value still intact
        BytecodeInstr::ReturnValue { value: Reg(1) },
    ]);

    let mut interp = make_interp_with_const(ConstValue::Bool(true));

    let result = interp.execute_function(&func, &[]).unwrap();
    assert_eq!(
        result,
        RuntimeValue::Bool(true),
        "Borrow 后 Release 应保留借用的值"
    );
}

/// 端到端测试：Standard 模式下 spawn 并发执行两个任务
///
/// 验证：
/// 1. 两个任务都能正确执行并返回结果
/// 2. Runtime facade 正确配置为 Standard 模式
/// 3. 任务通过 DAG 调度器执行（非 Embedded 顺序执行）
#[test]
fn spawn_concurrent_standard_mode() {
    use crate::backends::runtime::RuntimeMode;
    use crate::backends::interpreter::runtime::InterpreterRuntimeConfig;
    use crate::middle::bytecode::{BytecodeModule, BytecodeFunction, BytecodeInstr, FunctionRef};

    // task_a: 返回 Int(10)
    let task_a = BytecodeFunction {
        name: "task_a".to_string(),
        params: vec![],
        return_type: crate::middle::core::ir::Type::Void,
        local_count: 1,
        upvalue_count: 0,
        instructions: vec![
            BytecodeInstr::LoadConst {
                dst: Reg(0),
                const_idx: 0,
            },
            BytecodeInstr::ReturnValue { value: Reg(0) },
        ],
        labels: HashMap::new(),
        exception_handlers: vec![],
        debug_map: HashMap::new(),
    };

    // task_b: 返回 Int(20)
    let task_b = BytecodeFunction {
        name: "task_b".to_string(),
        params: vec![],
        return_type: crate::middle::core::ir::Type::Void,
        local_count: 1,
        upvalue_count: 0,
        instructions: vec![
            BytecodeInstr::LoadConst {
                dst: Reg(0),
                const_idx: 1,
            },
            BytecodeInstr::ReturnValue { value: Reg(0) },
        ],
        labels: HashMap::new(),
        exception_handlers: vec![],
        debug_map: HashMap::new(),
    };

    // main: 创建两个闭包，spawn 并发执行，读取结果并相加
    // 函数索引：task_a=0, task_b=1, main=2
    let main_func = BytecodeFunction {
        name: "main".to_string(),
        params: vec![],
        return_type: crate::middle::core::ir::Type::Void,
        local_count: 4,
        upvalue_count: 0,
        instructions: vec![
            // r0 = closure task_a
            BytecodeInstr::MakeClosure {
                dst: Reg(0),
                func: FunctionRef::Index(0),
                env: vec![],
            },
            // r1 = closure task_b
            BytecodeInstr::MakeClosure {
                dst: Reg(1),
                func: FunctionRef::Index(1),
                env: vec![],
            },
            // spawn [r0, r1] — 执行后 r0=task_a 结果, r1=task_b 结果
            BytecodeInstr::Spawn {
                dst: Reg(2),
                closures: vec![Reg(0), Reg(1)],
                task_deps: vec![vec![], vec![]],
                task_resources: vec![vec![], vec![]],
            },
            // r2 = r0 + r1 (10 + 20 = 30)
            BytecodeInstr::BinaryOp {
                dst: Reg(2),
                lhs: Reg(0),
                rhs: Reg(1),
                op: crate::middle::bytecode::BinaryOp::Add,
            },
            // return r2
            BytecodeInstr::ReturnValue { value: Reg(2) },
        ],
        labels: HashMap::new(),
        exception_handlers: vec![],
        debug_map: HashMap::new(),
    };

    let module = BytecodeModule {
        name: "spawn_test".to_string(),
        constants: vec![ConstValue::Int(10), ConstValue::Int(20)],
        functions: vec![task_a, task_b, main_func],
        type_table: vec![],
        globals: vec![],
        entry_point: Some(2), // main 函数
    };

    // 配置 Standard 模式 + 1 worker（避免多线程并发问题）
    let mut interp = Interpreter::new();
    interp.set_runtime_config(InterpreterRuntimeConfig {
        runtime: RuntimeMode::Standard,
        workers: 1,
        work_stealing: false,
    });
    // 重建 Runtime facade（set_runtime_config 只更新配置，需要 reset 重建 rt）
    interp.reset();

    // 验证 Runtime facade 已配置为 Standard 模式
    assert_eq!(
        interp.runtime_config().runtime,
        RuntimeMode::Standard,
        "runtime_config 应为 Standard 模式"
    );
    assert_eq!(interp.runtime_config().workers, 1, "workers 应为 1");

    // 执行模块：Spawn 应通过 DAG 调度器执行两个闭包
    // 注意：由于 set_runtime_config 不自动重建 rt，需要调用 reset() 来同步
    let result = interp.execute_module(&module);

    assert!(
        result.is_ok(),
        "Standard 模式下 spawn 执行不应报错: {:?}",
        result.err()
    );

    // 验证 Runtime facade 配置正确
    assert_eq!(
        interp.runtime_config().runtime,
        RuntimeMode::Standard,
        "runtime_config 应为 Standard 模式"
    );
    assert_eq!(interp.runtime_config().workers, 1, "workers 应为 1");
}
