//! Clone 语义检查
//!
//! 检查 clone() 调用的所有权语义：
//! - clone() 只能用于有效状态的值（Owned，不能是 Moved 或 Dropped）
//! - clone() 后原值仍保持 Owned 状态

use super::error::{OwnershipCheck, ValueState, codes, operand_display_name};
use crate::middle::core::ir::{FunctionIR, Instruction, Operand};
use crate::util::diagnostic::Diagnostic;
use std::collections::HashMap;

/// Clone 检查器
///
/// 检测以下错误：
/// - CloneMovedValue: clone 已移动的值
/// - CloneDroppedValue: clone 已释放的值
#[derive(Debug, Default)]
pub struct CloneChecker {
    state: HashMap<Operand, ValueState>,
    errors: Vec<Diagnostic>,
    location: (usize, usize),
    /// 局部变量名列表（用于错误报告中显示源码变量名）
    local_names: Option<Vec<String>>,
}

impl CloneChecker {
    /// 设置局部变量名列表
    pub fn set_local_names(
        &mut self,
        local_names: Option<Vec<String>>,
    ) {
        self.local_names = local_names;
    }

    /// 检查 clone() 调用（核心逻辑）
    fn check_clone(
        &mut self,
        receiver: &Operand,
        dst: Option<&Operand>,
    ) {
        if let Some(state) = self.state.get(receiver) {
            match state {
                ValueState::Moved => self.error_clone_moved(receiver),
                ValueState::Dropped => self.error_clone_dropped(receiver),
                ValueState::Owned(_) => {}
                ValueState::Dup => {
                    // Dup 类型（如 &T）clone 后保持 Dup 状态
                    // dst 也变为 Dup 状态
                    if let Some(d) = dst {
                        self.state.insert(d.clone(), ValueState::Dup);
                    }
                    return;
                }
                ValueState::Empty => self.error_clone_moved(receiver), // 空状态不能 clone
            }
            self.state.insert(receiver.clone(), ValueState::Owned(None));
        }
        if let Some(d) = dst {
            self.state.insert(d.clone(), ValueState::Owned(None));
        }
    }

    fn error_clone_moved(
        &mut self,
        operand: &Operand,
    ) {
        let name = operand_display_name(operand, self.local_names.as_ref());
        self.errors.push(codes::clone_moved_value(&name));
    }

    fn error_clone_dropped(
        &mut self,
        operand: &Operand,
    ) {
        let name = operand_display_name(operand, self.local_names.as_ref());
        self.errors.push(codes::use_after_drop(&name));
    }

    fn set_owned(
        &mut self,
        operand: &Operand,
    ) {
        self.state.insert(operand.clone(), ValueState::Owned(None));
    }

    fn set_moved(
        &mut self,
        operand: &Operand,
    ) {
        self.state.insert(operand.clone(), ValueState::Moved);
    }

    fn set_empty(
        &mut self,
        operand: &Operand,
    ) {
        self.state.insert(operand.clone(), ValueState::Empty);
    }

    fn set_dropped(
        &mut self,
        operand: &Operand,
    ) {
        self.state.insert(operand.clone(), ValueState::Dropped);
    }

    fn check_instruction(
        &mut self,
        instr: &Instruction,
    ) {
        match instr {
            // clone() 方法调用：检查 receiver 状态
            Instruction::Call {
                dst,
                func: Operand::Local(_) | Operand::Temp(_),
                args,
                ..
            } => {
                if let Some(receiver) = args.first() {
                    self.check_clone(receiver, dst.as_ref());
                }
            }
            // Move：src 被移动（进入 Empty），dst 成为新所有者
            Instruction::Move { dst, src } => {
                self.set_empty(src);
                self.set_owned(dst);
            }
            // 函数调用：参数被移动（进入 Empty）
            Instruction::Call { args, dst, .. } => {
                for arg in args {
                    self.set_empty(arg);
                }
                if let Some(d) = dst {
                    self.set_owned(d);
                }
            }
            // 返回：返回值被移动（进入 Empty）
            Instruction::Ret(Some(value)) => self.set_empty(value),
            // Drop：值被释放
            Instruction::Drop(operand) => self.set_dropped(operand),
            // 堆分配：新值是有效的所有者
            Instruction::HeapAlloc { dst, .. } => self.set_owned(dst),
            // 闭包：环境变量被移动
            Instruction::MakeClosure { dst, env, .. } => {
                for var in env {
                    self.set_moved(var);
                }
                self.set_owned(dst);
            }
            // Arc 操作：不影响原值状态
            Instruction::ArcNew { dst, .. } | Instruction::ArcClone { dst, .. } => {
                self.set_owned(dst);
            }
            Instruction::ArcDrop(_) => {}
            _ => {}
        }
    }
}

impl OwnershipCheck for CloneChecker {
    fn check_function(
        &mut self,
        func: &FunctionIR,
    ) -> &[Diagnostic] {
        self.clear();
        for (block_idx, block) in func.blocks.iter().enumerate() {
            for (instr_idx, instr) in block.instructions.iter().enumerate() {
                self.location = (block_idx, instr_idx);
                self.check_instruction(instr);
            }
        }
        &self.errors
    }

    fn errors(&self) -> &[Diagnostic] {
        &self.errors
    }

    fn state(&self) -> &HashMap<Operand, ValueState> {
        &self.state
    }

    fn clear(&mut self) {
        self.state.clear();
        self.errors.clear();
    }
}
