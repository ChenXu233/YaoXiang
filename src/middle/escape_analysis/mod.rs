//! 逃逸分析器 [已简化 - 未来参考]
//!
//! ⚠️ **此模块已简化**，仅保留核心设计思路供未来实现参考。
//!
//! ## 核心思想
//!
//! 逃逸分析决定变量是否应该栈分配：
//! - **不逃逸** → 栈分配（快，无 GC 压力）
//! - **逃逸** → 堆分配（Box/Arc）
//!
//! ## 简化规则
//!
//! 只检查三种"逃逸"情况：
//! 1. **返回** - 返回值逃逸
//! 2. **闭包捕获** - 被闭包捕获的变量逃逸
//! 3. **跨函数传递** - 传给其他函数的参数可能逃逸
//!
//! ## 简单实现（伪代码）
//!
//! ```text
//! 对于每个变量，检查它是否"逃逸"到：
//!   - 返回值（被 return 语句使用）
//!   - 函数调用参数（传给其他函数）
//!   - 闭包捕获（被闭包引用）
//! 如果逃逸，标记为堆分配；否则栈分配
//! ```
//!
//! ## 何时实现
//!
//! - AOT 编译阶段需要极致性能时
//! - 栈分配收益 > 实现成本时
//! - 当前：使用所有权模型 + 全堆分配即可工作

use crate::middle::ir::{FunctionIR, Instruction, Operand};
use std::collections::HashSet;

/// 逃逸分析结果
#[derive(Debug, Clone)]
pub struct EscapeAnalysisResult {
    /// 逃逸的变量 ID
    pub escapes: HashSet<usize>,
}

impl EscapeAnalysisResult {
    /// 检查变量是否逃逸
    pub fn escapes(
        &self,
        var_id: usize,
    ) -> bool {
        self.escapes.contains(&var_id)
    }
}

/// 逃逸分析器（简化版）
#[derive(Debug, Default)]
pub struct EscapeAnalyzer;

impl EscapeAnalyzer {
    /// 创建新的分析器
    pub fn new() -> Self {
        EscapeAnalyzer
    }

    /// 分析函数的逃逸情况
    pub fn analyze(
        &mut self,
        func: &FunctionIR,
    ) -> EscapeAnalysisResult {
        let mut escapes = HashSet::new();

        for instr in func.all_instructions() {
            match instr {
                // 返回值逃逸
                Instruction::Ret(Some(var)) => {
                    if let Some(id) = Self::operand_as_local(var) {
                        escapes.insert(id);
                    }
                }
                // 函数调用参数可能逃逸
                Instruction::Call { args, .. } => {
                    for arg in args {
                        if let Some(id) = Self::operand_as_local(arg) {
                            escapes.insert(id);
                        }
                    }
                }
                _ => {}
            }
        }

        EscapeAnalysisResult { escapes }
    }

    /// 提取 Operand 中的局部变量 ID
    fn operand_as_local(operand: &Operand) -> Option<usize> {
        match operand {
            Operand::Local(id) => Some(*id),
            _ => None,
        }
    }
}
