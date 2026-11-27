//! 右移语义回归测试 — 有符号 `>>` 与逻辑右移的区分
//!
//! 语言只有有符号 `Int`（64 位）。前端把 `>>` 降到 `Instruction::Sar`，VM 的
//! `BinaryOp::Sar` 做算术右移（高位补符号位）；`BinaryOp::Shr` 是逻辑右移（高位
//! 补零），前端今天不产出，留给未来的无符号右移运算符。本测试直接驱动两条
//! 字节码指令，确认两者对同一负数给出不同结果，且越界移位量都经 `int_op` 报错。

use crate::backends::common::RuntimeValue;
use crate::backends::interpreter::executor::Interpreter;
use crate::backends::{Executor, ExecutorError};
use crate::frontend::core::sema::MonoType;
use crate::middle::bytecode::{BinaryOp, BytecodeFunction, BytecodeInstr, ConstValue, Reg};
use std::collections::HashMap;

/// 构造一个函数：r0 = const[lhs], r1 = const[rhs], r2 = r0 <op> r1, return r2
fn make_shift_function(op: BinaryOp) -> BytecodeFunction {
    BytecodeFunction {
        name: "test_shift_semantics".to_string(),
        params: vec![],
        return_type: MonoType::Int(64),
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
            BytecodeInstr::BinaryOp {
                dst: Reg(2),
                lhs: Reg(0),
                rhs: Reg(1),
                op,
            },
            BytecodeInstr::ReturnValue { value: Reg(2) },
        ],
        labels: HashMap::new(),
        exception_handlers: vec![],
        debug_map: HashMap::new(),
    }
}

/// 跑一次移位，返回执行结果
fn run_shift(
    op: BinaryOp,
    lhs: i64,
    rhs: i64,
) -> Result<RuntimeValue, ExecutorError> {
    let func = make_shift_function(op);
    let mut interp = Interpreter::new();
    {
        let image = std::sync::Arc::get_mut(&mut interp.image)
            .expect("fresh interpreter image must be uniquely owned");
        image.constants.push(ConstValue::Int(lhs as i128));
        image.constants.push(ConstValue::Int(rhs as i128));
    }
    interp.execute_function(&func, &[])
}

fn run_shift_int(
    op: BinaryOp,
    lhs: i64,
    rhs: i64,
) -> i64 {
    match run_shift(op, lhs, rhs) {
        Ok(RuntimeValue::Int(value)) => value,
        other => panic!("{op:?} should return Int, got {other:?}"),
    }
}

#[test]
fn test_sar_shifts_negative_lhs_arithmetically() {
    // Arrange：-8 的二进制补码是 0xFFFF_FFFF_FFFF_FFF8，算术右移补符号位。
    // Act
    let result = run_shift_int(BinaryOp::Sar, -8, 1);

    // Assert
    assert_eq!(result, -4, "Sar must shift arithmetically: -8 >> 1 == -4");
}

#[test]
fn test_shr_shifts_negative_lhs_logically() {
    // Arrange：同一输入走逻辑右移，高位补零，结果为 0x7FFF_FFFF_FFFF_FFFC。
    // Act
    let result = run_shift_int(BinaryOp::Shr, -8, 1);

    // Assert
    assert_eq!(
        result, 9_223_372_036_854_775_804,
        "Shr must shift logically: -8 >>> 1 == 0x7FFF_FFFF_FFFF_FFFC"
    );
    assert_ne!(
        result,
        run_shift_int(BinaryOp::Sar, -8, 1),
        "logical and arithmetic right shifts must differ on negative input"
    );
}

#[test]
fn test_shr_by_zero_keeps_negative_value() {
    // Arrange：位移量为 0 时逻辑右移与算术右移同值（-1 的位型不变）。
    // Act
    let result = run_shift_int(BinaryOp::Shr, -1, 0);

    // Assert
    assert_eq!(result, -1, "a zero shift amount must keep the value");
}

#[test]
fn test_shr_out_of_range_amount_reports_runtime_error() {
    // Arrange：位移量 64 越界；两条右移都经 int_op 报同一个运行期错误，
    // 但错误文案里的算子不同（>> 与 >>>），可区分走的是哪条臂。
    // Act
    let logical = run_shift(BinaryOp::Shr, 1, 64);
    let arithmetic = run_shift(BinaryOp::Sar, 1, 64);

    // Assert
    match logical {
        Err(ExecutorError::Runtime(message, _)) => assert!(
            message.contains("integer overflow") && message.contains(">>>"),
            "logical shift must report the >>> operator: {message}"
        ),
        other => panic!("Shr by 64 must be a runtime error, got {other:?}"),
    }
    match arithmetic {
        Err(ExecutorError::Runtime(message, _)) => assert!(
            message.contains("integer overflow") && message.contains(">>"),
            "arithmetic shift must report the >> operator: {message}"
        ),
        other => panic!("Sar by 64 must be a runtime error, got {other:?}"),
    }
}
