//! 寄存器与槽位指令族：Mov / Load* / Store* —— 槽位、全局、上值访问
//!
//! `execute_instr` 按指令族拆分的一部分。arm 体自原单函数逐字搬迁，
//! 仅去掉外层 `match` 包裹并统一以 `return` 形式返回，语义不变。

use crate::backends::common::RuntimeValue;
use crate::backends::interpreter::executor::debug::StepOutcome;
use crate::backends::interpreter::executor::Interpreter;
use crate::backends::ExecutorError;
use crate::middle::bytecode::BytecodeInstr;

impl Interpreter {
    /// 执行「寄存器与槽位」族指令。
    ///
    /// 由 `execute_instr` 分派调用；`#[inline]` 保证不引入额外调用开销。
    #[inline]
    pub(in crate::backends::interpreter::executor) fn exec_reg(
        &mut self,
        fi: usize,
        instr: &BytecodeInstr,
    ) -> crate::backends::interpreter::executor::debug::ExecResult {
        let _ = fi;
        match instr {
            BytecodeInstr::Mov { dst, src } => {
                let val = self.call_stack[fi]
                    .get_slot(src.0 as usize)
                    .cloned()
                    .unwrap_or(RuntimeValue::Void);
                self.call_stack[fi].set_slot(dst.0 as usize, val);
                self.call_stack[fi].advance();
                Ok(StepOutcome::Continue)
            }
            BytecodeInstr::LoadConst { dst, const_idx } => {
                let val = self.load_constant(*const_idx);
                self.call_stack[fi].set_slot(dst.0 as usize, val);
                self.call_stack[fi].advance();
                Ok(StepOutcome::Continue)
            }
            BytecodeInstr::LoadLocal { dst, local_idx } => {
                let val = self.call_stack[fi]
                    .get_slot(*local_idx as usize)
                    .cloned()
                    .unwrap_or(RuntimeValue::Void);
                self.call_stack[fi].set_slot(dst.0 as usize, val);
                self.call_stack[fi].advance();
                Ok(StepOutcome::Continue)
            }
            BytecodeInstr::StoreLocal { local_idx, src } => {
                let val = self.call_stack[fi]
                    .get_slot(src.0 as usize)
                    .cloned()
                    .unwrap_or(RuntimeValue::Void);
                self.call_stack[fi].set_slot(*local_idx as usize, val);
                self.call_stack[fi].advance();
                Ok(StepOutcome::Continue)
            }
            BytecodeInstr::LoadArg { dst, arg_idx } => {
                // Args are stored in locals by Frame::with_args
                let val = self.call_stack[fi]
                    .get_slot(*arg_idx as usize)
                    .cloned()
                    .unwrap_or(RuntimeValue::Void);
                self.call_stack[fi].set_slot(dst.0 as usize, val);
                self.call_stack[fi].advance();
                Ok(StepOutcome::Continue)
            }
            BytecodeInstr::LoadGlobal { dst, global_idx } => {
                let val = self
                    .global_slots
                    .get(*global_idx as usize)
                    .cloned()
                    .unwrap_or(RuntimeValue::Void);
                self.call_stack[fi].set_slot(dst.0 as usize, val);
                self.call_stack[fi].advance();
                Ok(StepOutcome::Continue)
            }
            BytecodeInstr::StoreGlobal { global_idx, src } => {
                let val = self.call_stack[fi]
                    .get_slot(src.0 as usize)
                    .cloned()
                    .unwrap_or(RuntimeValue::Void);
                let idx = *global_idx as usize;
                if idx >= self.global_slots.len() {
                    self.global_slots.resize(idx + 1, RuntimeValue::Void);
                }
                self.global_slots[idx] = val;
                self.call_stack[fi].advance();
                Ok(StepOutcome::Continue)
            }
            BytecodeInstr::LoadUpvalue { dst, upvalue_idx } => {
                let val = self.call_stack[fi]
                    .get_upvalue(*upvalue_idx as usize)
                    .cloned()
                    .unwrap_or(RuntimeValue::Void);
                self.call_stack[fi].set_slot(dst.0 as usize, val);
                self.call_stack[fi].advance();
                Ok(StepOutcome::Continue)
            }
            BytecodeInstr::StoreUpvalue { src, upvalue_idx } => {
                let val = self.call_stack[fi]
                    .get_slot(src.0 as usize)
                    .cloned()
                    .expect("register index out of bounds");
                self.call_stack[fi].set_upvalue(*upvalue_idx as usize, val);
                self.call_stack[fi].advance();
                Ok(StepOutcome::Continue)
            }
            // 分派层（`execute_instr`）保证只有本族指令到达此处。
            // 出现其他指令说明分派表与该族定义不一致，属编译器内部错误。
            other => Err(ExecutorError::runtime_only(format!(
                "exec_reg 收到非本族指令: {other:?}"
            ))),
        }
    }
}
