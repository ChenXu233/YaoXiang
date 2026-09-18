//! 控制流指令族：Nop / 跳转 / 分支 / Switch / Return
//!
//! `execute_instr` 按指令族拆分的一部分。arm 体自原单函数逐字搬迁，
//! 仅去掉外层 `match` 包裹并统一以 `return` 形式返回，语义不变。

use crate::backends::common::RuntimeValue;
use crate::backends::interpreter::executor::debug::StepOutcome;
use crate::backends::interpreter::executor::Interpreter;
use crate::backends::ExecutorError;
use crate::middle::bytecode::BytecodeInstr;

impl Interpreter {
    /// 执行「控制流」族指令。
    ///
    /// 由 `execute_instr` 分派调用；`#[inline]` 保证不引入额外调用开销。
    #[inline]
    pub(in crate::backends::interpreter::executor) fn exec_control(
        &mut self,
        fi: usize,
        instr: &BytecodeInstr,
    ) -> crate::backends::interpreter::executor::debug::ExecResult {
        let _ = fi;
        match instr {
            BytecodeInstr::Nop
            | BytecodeInstr::Yield
            | BytecodeInstr::Drop { .. }
            | BytecodeInstr::Release { .. }
            | BytecodeInstr::StackAlloc { .. }
            | BytecodeInstr::TryBegin { .. }
            | BytecodeInstr::TryEnd
            | BytecodeInstr::ArcDrop { .. }
            | BytecodeInstr::CloseUpvalue { .. } => {
                self.call_stack[fi].advance();
                Ok(StepOutcome::Continue)
            }

            // ── Return ──────────────────────────────────────────
            BytecodeInstr::Return => {
                for task_id in self.call_stack[fi].take_all_spawned_tasks() {
                    let mut v = self.make_async_pending(task_id);
                    self.force_value_in_place(&mut v)?;
                }
                self.last_return_value = RuntimeValue::Void;
                // 帧原地驻留后，返回由指令自身弹出（旧实现由 step_one 省略 push 完成）
                self.call_stack.pop();
                Ok(StepOutcome::Returned)
            }
            BytecodeInstr::ReturnValue { value } => {
                let result = self.call_stack[fi]
                    .get_slot(value.0 as usize)
                    .cloned()
                    .unwrap_or(RuntimeValue::Void);
                for task_id in self.call_stack[fi].take_all_spawned_tasks() {
                    let mut v = self.make_async_pending(task_id);
                    self.force_value_in_place(&mut v)?;
                }
                self.last_return_value = result;
                // 帧原地驻留后，返回由指令自身弹出
                self.call_stack.pop();
                Ok(StepOutcome::Returned)
            }

            // ── Jumps ───────────────────────────────────────────
            BytecodeInstr::Jmp { target } => {
                let offset = Self::decode_label_offset(*target);
                self.call_stack[fi].ip = ((self.call_stack[fi].ip as i32) + offset) as usize;
                Ok(StepOutcome::Continue)
            }
            BytecodeInstr::JmpIf { cond, target } => {
                let c = self.force_slot(fi, *cond)?.to_bool().ok_or_else(|| {
                    ExecutorError::type_error("JmpIf 条件值不是布尔类型", self.capture_stack())
                })?;
                if c {
                    let offset = Self::decode_label_offset(*target);
                    self.call_stack[fi].ip = ((self.call_stack[fi].ip as i32) + offset) as usize;
                } else {
                    self.call_stack[fi].advance();
                }
                Ok(StepOutcome::Continue)
            }
            BytecodeInstr::JmpIfNot { cond, target } => {
                let c = self.force_slot(fi, *cond)?.to_bool().ok_or_else(|| {
                    ExecutorError::type_error("JmpIfNot 条件值不是布尔类型", self.capture_stack())
                })?;
                if !c {
                    let offset = Self::decode_label_offset(*target);
                    self.call_stack[fi].ip = ((self.call_stack[fi].ip as i32) + offset) as usize;
                } else {
                    self.call_stack[fi].advance();
                }
                Ok(StepOutcome::Continue)
            }
            BytecodeInstr::Switch { value, targets } => {
                let val = self.force_slot(fi, *value)?;
                let mut jumped = false;
                for (case_val, target) in targets {
                    if let Some(case_label) = case_val {
                        let case_offset = Self::decode_label_offset(*case_label);
                        let matches = match &val {
                            RuntimeValue::Int(n) => *n == case_offset as i64,
                            RuntimeValue::Bool(b) => *b == (case_offset != 0),
                            RuntimeValue::Enum { variant_id, .. } => {
                                *variant_id == case_offset as u32
                            }
                            _ => false,
                        };
                        if matches {
                            let offset = Self::decode_label_offset(*target);
                            self.call_stack[fi].ip =
                                ((self.call_stack[fi].ip as i32) + offset) as usize;
                            jumped = true;
                            break;
                        }
                    }
                }
                if !jumped {
                    if let Some((None, default_target)) = targets.last() {
                        let offset = Self::decode_label_offset(*default_target);
                        self.call_stack[fi].ip =
                            ((self.call_stack[fi].ip as i32) + offset) as usize;
                    } else {
                        self.call_stack[fi].advance();
                    }
                }
                Ok(StepOutcome::Continue)
            }
            // 分派层（`execute_instr`）保证只有本族指令到达此处。
            // 出现其他指令说明分派表与该族定义不一致，属编译器内部错误。
            other => Err(ExecutorError::runtime_only(format!(
                "exec_control 收到非本族指令: {other:?}"
            ))),
        }
    }
}
