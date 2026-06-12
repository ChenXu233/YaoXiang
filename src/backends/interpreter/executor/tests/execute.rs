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
