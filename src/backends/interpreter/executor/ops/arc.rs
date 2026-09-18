//! 引用计数指令族：Arc / Rc / Weak
//!
//! `execute_instr` 按指令族拆分的一部分。arm 体自原单函数逐字搬迁，
//! 仅去掉外层 `match` 包裹并统一以 `return` 形式返回，语义不变。

use crate::backends::common::RuntimeValue;
use crate::backends::interpreter::executor::debug::StepOutcome;
use crate::backends::interpreter::executor::Interpreter;
use crate::backends::ExecutorError;
use crate::middle::bytecode::BytecodeInstr;

impl Interpreter {
    /// 执行「引用计数」族指令。
    ///
    /// 由 `execute_instr` 分派调用；`#[inline]` 保证不引入额外调用开销。
    #[inline]
    pub(in crate::backends::interpreter::executor) fn exec_arc(
        &mut self,
        fi: usize,
        instr: &BytecodeInstr,
    ) -> crate::backends::interpreter::executor::debug::ExecResult {
        let _ = fi;
        match instr {
            BytecodeInstr::ArcNew { dst, src } => {
                let val = self.call_stack[fi]
                    .get_slot(src.0 as usize)
                    .cloned()
                    .unwrap_or(RuntimeValue::Void);
                self.call_stack[fi].set_slot(dst.0 as usize, val.into_arc());
                self.call_stack[fi].advance();
                Ok(StepOutcome::Continue)
            }
            BytecodeInstr::RcNew { dst, src } => {
                let val = self.call_stack[fi]
                    .get_slot(src.0 as usize)
                    .cloned()
                    .unwrap_or(RuntimeValue::Void);
                self.call_stack[fi].set_slot(dst.0 as usize, val.into_arc());
                self.call_stack[fi].advance();
                Ok(StepOutcome::Continue)
            }
            BytecodeInstr::ArcClone { dst, src } => {
                let val = self.call_stack[fi]
                    .get_slot(src.0 as usize)
                    .cloned()
                    .unwrap_or(RuntimeValue::Void);
                if let RuntimeValue::Arc(inner) = val {
                    self.call_stack[fi].set_slot(dst.0 as usize, RuntimeValue::Arc(inner));
                }
                self.call_stack[fi].advance();
                Ok(StepOutcome::Continue)
            }
            BytecodeInstr::WeakNew { dst, src } => {
                let val = self.call_stack[fi]
                    .get_slot(src.0 as usize)
                    .cloned()
                    .unwrap_or(RuntimeValue::Void);
                if let RuntimeValue::Arc(arc) = val {
                    self.call_stack[fi].set_slot(
                        dst.0 as usize,
                        RuntimeValue::Weak(std::sync::Arc::downgrade(&arc)),
                    );
                } else {
                    self.call_stack[fi].set_slot(dst.0 as usize, RuntimeValue::Void);
                }
                self.call_stack[fi].advance();
                Ok(StepOutcome::Continue)
            }
            BytecodeInstr::WeakUpgrade { dst, src } => {
                let val = self.call_stack[fi]
                    .get_slot(src.0 as usize)
                    .cloned()
                    .unwrap_or(RuntimeValue::Void);
                if let RuntimeValue::Weak(weak) = val {
                    if let Some(arc) = weak.upgrade() {
                        self.call_stack[fi].set_slot(dst.0 as usize, RuntimeValue::Arc(arc));
                    } else {
                        self.call_stack[fi].set_slot(dst.0 as usize, RuntimeValue::Void);
                    }
                } else {
                    self.call_stack[fi].set_slot(dst.0 as usize, RuntimeValue::Void);
                }
                self.call_stack[fi].advance();
                Ok(StepOutcome::Continue)
            }

            // ── Borrow (ZST, runtime equivalent to Mov) ─────────
            BytecodeInstr::Borrow { dst, src, .. } => {
                let val = self.call_stack[fi]
                    .get_slot(src.0 as usize)
                    .cloned()
                    .unwrap_or(RuntimeValue::Void);
                self.call_stack[fi].set_slot(dst.0 as usize, val);
                self.call_stack[fi].advance();
                Ok(StepOutcome::Continue)
            }
            // 分派层（`execute_instr`）保证只有本族指令到达此处。
            // 出现其他指令说明分派表与该族定义不一致，属编译器内部错误。
            other => Err(ExecutorError::runtime_only(format!(
                "exec_arc 收到非本族指令: {other:?}"
            ))),
        }
    }
}
