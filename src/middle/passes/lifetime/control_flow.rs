//! 控制流分析器
//!
//! 分析 if/match 等控制流分支的状态合并，确保空状态正确追踪。
//!
//! # 设计原理
//!
//! 在控制流中，不同分支可能对同一变量进行不同操作：
//! - 分支 A：Move 变量
//! - 分支 B：使用变量
//!
//! 汇合后，变量状态取决于两个分支的综合结果。
//! 控制流分析器使用保守策略：
//! - 任一分支 Move 变量 → 汇合后为 Moved
//! - 任一分支赋值变量 → 汇合后为 Owned
//! - 任一分支进入 Empty → 汇合后为 Empty

use super::error::ValueState;
use crate::middle::core::ir::{FunctionIR, Instruction, Operand};
use std::collections::{HashMap, HashSet};

/// 控制流分析结果
#[derive(Debug, Clone)]
pub struct ControlFlowAnalysisResult {
    /// 合并后的状态
    pub merged_state: HashMap<Operand, ValueState>,
    /// 条件分支信息
    pub branches: Vec<BranchInfo>,
}

/// 分支信息
#[derive(Debug, Clone)]
pub struct BranchInfo {
    /// 分支起始位置 (block_idx, instr_idx)
    pub start: (usize, usize),
    /// 分支结束位置
    pub end: (usize, usize),
    /// 分支条件变量
    pub condition: Operand,
    /// 真假分支目标块
    pub true_target: usize,
    pub false_target: usize,
    /// 分支内的状态变化
    pub state_delta: HashMap<Operand, ValueState>,
}

/// 控制流分析器
///
/// 分析函数中的控制流结构，追踪分支状态变化。
///
/// # 分析流程
///
/// 1. 识别所有控制流指令（JmpIf, JmpIfNot）
/// 2. 收集每个分支的状态变化
/// 3. 合并分支状态（保守策略）
/// 4. 返回合并后的状态
#[derive(Debug, Clone)]
pub struct ControlFlowAnalyzer {
    /// 分析结果
    result: ControlFlowAnalysisResult,
}

impl ControlFlowAnalyzer {
    /// 创建新的控制流分析器
    pub fn new() -> Self {
        Self {
            result: ControlFlowAnalysisResult {
                merged_state: HashMap::new(),
                branches: Vec::new(),
            },
        }
    }

    /// 分析函数
    pub fn analyze_function(
        &mut self,
        func: &FunctionIR,
    ) -> &ControlFlowAnalysisResult {
        self.clear();
        self.analyze_blocks(func);
        &self.result
    }

    /// 清空状态
    fn clear(&mut self) {
        self.result = ControlFlowAnalysisResult {
            merged_state: HashMap::new(),
            branches: Vec::new(),
        };
    }

    /// 分析所有基本块
    fn analyze_blocks(
        &mut self,
        func: &FunctionIR,
    ) {
        // 初始化：所有变量初始状态为 Owned（假设已定义）
        self.initialize_states(func);

        // 分析每个块
        for (block_idx, block) in func.blocks.iter().enumerate() {
            self.analyze_block(block, block_idx);
        }
    }

    /// 初始化状态
    fn initialize_states(
        &mut self,
        func: &FunctionIR,
    ) {
        // 函数参数初始状态为 Owned
        for (idx, _) in func.params.iter().enumerate() {
            self.result
                .merged_state
                .insert(Operand::Arg(idx), ValueState::Owned(None));
        }

        // 局部变量初始状态为 Owned
        for (idx, _) in func.locals.iter().enumerate() {
            self.result
                .merged_state
                .insert(Operand::Local(idx), ValueState::Owned(None));
        }
    }

    /// 分析单个基本块
    fn analyze_block(
        &mut self,
        block: &crate::middle::core::ir::BasicBlock,
        block_idx: usize,
    ) {
        let mut local_state = self.result.merged_state.clone();

        for (instr_idx, instr) in block.instructions.iter().enumerate() {
            let pos = (block_idx, instr_idx);

            // 分析指令对状态的影响
            self.analyze_instruction(instr, &mut local_state, pos);
        }

        // 更新合并状态
        self.merge_block_state(&local_state, block_idx);
    }

    /// 分析单条指令
    fn analyze_instruction(
        &self,
        _instr: &Instruction,
        _state: &mut HashMap<Operand, ValueState>,
        _pos: (usize, usize),
    ) {
        // 目前为空实现，后续可根据需要扩展
        // 控制流分析在 MoveChecker 中已有基本实现
    }

    /// 合并块状态
    fn merge_block_state(
        &mut self,
        _block_state: &HashMap<Operand, ValueState>,
        _block_idx: usize,
    ) {
        // 这里需要实现更复杂的状态合并逻辑
        // 考虑到多个分支可能汇入同一个块
    }

    /// 获取分析结果
    pub fn result(&self) -> &ControlFlowAnalysisResult {
        &self.result
    }

    /// 获取合并后的状态
    pub fn merged_state(&self) -> &HashMap<Operand, ValueState> {
        &self.result.merged_state
    }

    /// 手动合并两个状态
    ///
    /// 使用保守策略：
    /// - 任一状态为 Moved → Moved
    /// - 任一状态为 Empty → Empty
    /// - 都是 Owned → Owned
    pub fn merge_states(
        state1: &ValueState,
        state2: &ValueState,
    ) -> ValueState {
        match (state1, state2) {
            // Empty 优先级最高
            (ValueState::Empty, _) | (_, ValueState::Empty) => ValueState::Empty,
            // Moved 优先级次之
            (ValueState::Moved, _) | (_, ValueState::Moved) => ValueState::Moved,
            // Owned 处理
            (ValueState::Owned(t1), ValueState::Owned(t2)) => {
                if t1 == t2 {
                    state1.clone()
                } else {
                    ValueState::Owned(t1.clone())
                }
            }
            (ValueState::Dropped, s) | (s, ValueState::Dropped) => s.clone(),
        }
    }
}

impl Default for ControlFlowAnalyzer {
    fn default() -> Self {
        Self::new()
    }
}

/// 活跃变量分析（辅助函数）
///
/// 用于确定在控制流汇合点哪些变量是活跃的。
pub fn liveness_analysis(func: &FunctionIR) -> HashMap<usize, HashSet<Operand>> {
    let mut live_vars = HashMap::new();

    // 初始化：每个块的人口活跃变量为空
    for (block_idx, _) in func.blocks.iter().enumerate() {
        live_vars.insert(block_idx, HashSet::new());
    }

    // 迭代直到不动点
    let mut changed = true;
    while changed {
        changed = false;

        for (block_idx, block) in func.blocks.iter().enumerate() {
            let mut live_out = HashSet::new();

            // 收集后继块的入口活跃变量
            for &succ in &block.successors {
                if let Some(succ_live) = live_vars.get(&succ) {
                    live_out.extend(succ_live.iter().cloned());
                }
            }

            let mut live_in = live_out.clone();
            let mut block_live = live_in.clone();

            // 反向遍历指令
            for instr in block.instructions.iter().rev() {
                update_liveness(instr, &mut block_live, &mut live_in);
            }

            // 检查变化
            let current_live = live_vars.get(&block_idx).cloned().unwrap_or_default();
            if current_live != block_live {
                live_vars.insert(block_idx, block_live);
                changed = true;
            }
        }
    }

    live_vars
}

/// 更新活跃变量
fn update_liveness(
    instr: &Instruction,
    block_live: &mut HashSet<Operand>,
    live_in: &mut HashSet<Operand>,
) {
    match instr {
        Instruction::Move { dst, src } => {
            block_live.remove(dst);
            block_live.insert(src.clone());
            live_in.insert(src.clone());
        }
        Instruction::LoadIndex { dst, src, index } => {
            block_live.remove(dst);
            block_live.insert(src.clone());
            block_live.insert(index.clone());
        }
        Instruction::LoadField { dst, src, .. } => {
            block_live.remove(dst);
            block_live.insert(src.clone());
        }
        Instruction::Store { src, dst } => {
            block_live.insert(src.clone());
            block_live.insert(dst.clone());
        }
        Instruction::StoreIndex { src, dst, index } => {
            block_live.insert(src.clone());
            block_live.insert(dst.clone());
            block_live.insert(index.clone());
        }
        Instruction::StoreField { src, dst, .. } => {
            block_live.insert(src.clone());
            block_live.insert(dst.clone());
        }
        Instruction::Call { dst, args, .. } => {
            if let Some(d) = dst {
                block_live.remove(d);
            }
            for arg in args {
                block_live.insert(arg.clone());
            }
        }
        Instruction::Ret(Some(value)) => {
            block_live.insert(value.clone());
        }
        Instruction::Ret(None) => {}
        Instruction::HeapAlloc { dst, .. } => {
            block_live.remove(dst);
        }
        Instruction::Cast { dst, src, .. } => {
            block_live.remove(dst);
            block_live.insert(src.clone());
        }
        Instruction::Neg { dst, src } | Instruction::Load { dst, src } => {
            block_live.remove(dst);
            block_live.insert(src.clone());
        }
        _ => {}
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_merge_states() {
        // Empty + Owned = Empty（任一为 Empty 则为 Empty）
        assert_eq!(
            ControlFlowAnalyzer::merge_states(&ValueState::Empty, &ValueState::Owned(None)),
            ValueState::Empty
        );

        // Moved + Owned = Moved（任一为 Moved 则为 Moved）
        assert_eq!(
            ControlFlowAnalyzer::merge_states(&ValueState::Moved, &ValueState::Owned(None)),
            ValueState::Moved
        );

        // Empty + Moved = Empty（Empty 优先级更高）
        assert_eq!(
            ControlFlowAnalyzer::merge_states(&ValueState::Empty, &ValueState::Moved),
            ValueState::Empty
        );

        // Moved + Moved = Moved
        assert_eq!(
            ControlFlowAnalyzer::merge_states(&ValueState::Moved, &ValueState::Moved),
            ValueState::Moved
        );

        // Empty + Empty = Empty
        assert_eq!(
            ControlFlowAnalyzer::merge_states(&ValueState::Empty, &ValueState::Empty),
            ValueState::Empty
        );
    }
}
