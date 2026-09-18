//! 算术与比较指令族：BinaryOp / Compare / UnaryOp
//!
//! `execute_instr` 按指令族拆分的一部分。arm 体自原单函数逐字搬迁，
//! 仅去掉外层 `match` 包裹并统一以 `return` 形式返回，语义不变。

use crate::backends::common::RuntimeValue;
use crate::backends::interpreter::executor::debug::StepOutcome;
use crate::backends::interpreter::executor::Interpreter;
use crate::backends::ExecutorError;
use crate::middle::bytecode::BytecodeInstr;

impl Interpreter {
    /// 执行「算术与比较」族指令。
    ///
    /// 由 `execute_instr` 分派调用；`#[inline]` 保证不引入额外调用开销。
    #[inline]
    pub(in crate::backends::interpreter::executor) fn exec_arith(
        &mut self,
        fi: usize,
        instr: &BytecodeInstr,
    ) -> crate::backends::interpreter::executor::debug::ExecResult {
        let _ = fi;
        match instr {
            BytecodeInstr::BinaryOp { dst, lhs, rhs, op } => {
                self.exec_binary_op(*dst, *lhs, *rhs, *op, fi)?;
                self.call_stack[fi].advance();
                Ok(StepOutcome::Continue)
            }
            BytecodeInstr::Compare { dst, lhs, rhs, cmp } => {
                self.exec_compare(*dst, *lhs, *rhs, *cmp, fi)?;
                self.call_stack[fi].advance();
                Ok(StepOutcome::Continue)
            }
            BytecodeInstr::UnaryOp { dst, src, op } => {
                let val = self.force_slot(fi, *src)?;
                let result = match (op, val) {
                    (crate::middle::bytecode::UnaryOp::Neg, RuntimeValue::Int(n)) => {
                        RuntimeValue::Int(-n)
                    }
                    (crate::middle::bytecode::UnaryOp::Neg, RuntimeValue::Float(f)) => {
                        RuntimeValue::Float(-f)
                    }
                    (crate::middle::bytecode::UnaryOp::Neg, RuntimeValue::Bool(b)) => {
                        RuntimeValue::Bool(!b)
                    }
                    (crate::middle::bytecode::UnaryOp::Not, RuntimeValue::Int(n)) => {
                        RuntimeValue::Int(!n)
                    }
                    (crate::middle::bytecode::UnaryOp::Not, RuntimeValue::Bool(b)) => {
                        RuntimeValue::Bool(!b)
                    }
                    _ => {
                        let stack = self.capture_stack();
                        return Err(ExecutorError::type_error(
                            format!("type mismatch in unary operation {:?}", op),
                            stack,
                        ));
                    }
                };
                self.call_stack[fi].set_slot(dst.0 as usize, result);
                self.call_stack[fi].advance();
                Ok(StepOutcome::Continue)
            }
            // 分派层（`execute_instr`）保证只有本族指令到达此处。
            // 出现其他指令说明分派表与该族定义不一致，属编译器内部错误。
            other => Err(ExecutorError::runtime_only(format!(
                "exec_arith 收到非本族指令: {other:?}"
            ))),
        }
    }
}
