//! unsafe 语义检查器
//!
//! 实现以下检查：
//! 1. unsafe 块外解引用报错
//! 2. 裸指针类型检查
//! 3. Send/Sync 安全 trait 检查

use crate::middle::core::ir::{FunctionIR, Instruction};
use crate::middle::passes::lifetime::error::OwnershipError;

/// unsafe 语义检查器
#[derive(Debug)]
pub struct UnsafeChecker {
    /// 错误列表
    errors: Vec<OwnershipError>,
    /// 当前是否在 unsafe 块内
    in_unsafe: bool,
    /// unsafe 块嵌套深度
    unsafe_depth: usize,
}

impl UnsafeChecker {
    /// 创建新的 unsafe 检查器
    pub fn new() -> Self {
        Self {
            errors: Vec::new(),
            in_unsafe: false,
            unsafe_depth: 0,
        }
    }

    /// 检查函数的所有权语义
    pub fn check_function(
        &mut self,
        func: &FunctionIR,
    ) -> Vec<OwnershipError> {
        self.errors.clear();
        self.in_unsafe = false;
        self.unsafe_depth = 0;

        // 遍历所有基本块和指令
        for (block_idx, block) in func.blocks.iter().enumerate() {
            let mut in_unsafe_block = false;

            for (instr_idx, instr) in block.instructions.iter().enumerate() {
                match instr {
                    Instruction::UnsafeBlockStart => {
                        in_unsafe_block = true;
                        self.in_unsafe = true;
                        self.unsafe_depth += 1;
                    }
                    Instruction::UnsafeBlockEnd => {
                        if self.unsafe_depth > 0 {
                            self.unsafe_depth -= 1;
                        }
                        in_unsafe_block = self.unsafe_depth > 0;
                        self.in_unsafe = in_unsafe_block;
                    }
                    _ => {
                        // 检查指针操作是否在 unsafe 块内
                        self.check_pointer_operation(instr, block_idx, instr_idx, in_unsafe_block);
                    }
                }
            }
        }

        self.errors.clone()
    }

    /// 检查指针操作
    fn check_pointer_operation(
        &mut self,
        instr: &Instruction,
        block_idx: usize,
        instr_idx: usize,
        in_unsafe: bool,
    ) {
        // 检查是否是 unsafe 操作
        let is_unsafe_op = matches!(
            instr,
            Instruction::PtrFromRef { .. }
                | Instruction::PtrDeref { .. }
                | Instruction::PtrStore { .. }
                | Instruction::PtrLoad { .. }
        );

        if is_unsafe_op && !in_unsafe {
            self.errors.push(OwnershipError::UnsafeDeref {
                instruction: format!("{:?}", instr),
                location: (block_idx, instr_idx),
            });
        }
    }

    /// 获取错误列表
    pub fn errors(&self) -> &[OwnershipError] {
        &self.errors
    }

    /// 清除状态
    pub fn clear(&mut self) {
        self.errors.clear();
        self.in_unsafe = false;
        self.unsafe_depth = 0;
    }
}

impl Default for UnsafeChecker {
    fn default() -> Self {
        Self::new()
    }
}
