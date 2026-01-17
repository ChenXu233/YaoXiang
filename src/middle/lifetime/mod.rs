//! 所有权分析与生命周期管理
//!
//! 实现 Move 语义检查、Drop 语义检查和 Clone 语义检查，确保内存正确释放而无需 GC。
//! 设计原则：
//! 1. 每个值有一个所有者
//! 2. 当所有者离开作用域时，值被释放
//! 3. 所有权可以转移（Move），但不能复制（除非使用 clone()）
//!
//! # 模块结构
//!
//! - `error.rs`: 所有权错误类型定义
//! - `move_semantics.rs`: Move 语义检查（UseAfterMove 检测）
//! - `drop_semantics.rs`: Drop 语义检查（UseAfterDrop、DropMovedValue、DoubleDrop 检测）
//! - `clone.rs`: Clone 语义检查（CloneMovedValue、CloneDroppedValue 检测）

use crate::middle::ir::{FunctionIR, Instruction, Operand};
use std::collections::{HashMap, HashSet};
use std::fmt;

// 子模块
mod clone;
mod drop_semantics;
mod error;
mod move_semantics;
mod mut_check;
mod ref_semantics;

pub use clone::*;
pub use error::*;
pub use move_semantics::*;
pub use drop_semantics::*;
pub use mut_check::*;
pub use ref_semantics::*;

/// 所有权分析结果
#[derive(Debug, Clone)]
pub struct OwnershipAnalysisResult {
    /// 所有权关系图
    pub ownership_graph: OwnershipGraph,
    /// 变量定义点
    pub definitions: HashMap<Operand, Definition>,
    /// 需要释放的变量（在作用域结束时）
    pub drop_points: HashMap<usize, Vec<Operand>>, // block_idx -> vars to drop
}

/// 变量定义信息
#[derive(Debug, Clone)]
pub struct Definition {
    /// 定义位置
    pub position: (usize, usize),
    /// 变量类型信息
    pub ty: Option<String>,
    /// 是否逃逸到作用域外
    pub escapes: bool,
    /// 是否被移动（所有权转移）
    pub is_moved: bool,
}

/// 所有权图
#[derive(Debug, Default, Clone)]
pub struct OwnershipGraph {
    /// 所有权边：source -> target 表示 source 拥有 target
    edges: HashMap<Operand, HashSet<Operand>>,
    /// 所有者的生命周期
    lifetimes: HashMap<Operand, Lifetime>,
}

/// 生命周期
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Lifetime {
    /// 开始位置
    pub start: (usize, usize),
    /// 结束位置
    pub end: (usize, usize),
    /// 是否逃逸到作用域外
    pub escapes: bool,
}

impl Lifetime {
    pub fn new(
        start: (usize, usize),
        end: (usize, usize),
    ) -> Self {
        Self {
            start,
            end,
            escapes: false,
        }
    }
}

/// 统一的所有权检查器
///
/// 同时运行 Move 检查、Drop 检查、Mut 检查、Ref 检查和 Clone 检查，返回所有错误。
#[derive(Debug)]
pub struct OwnershipChecker {
    move_checker: MoveChecker,
    drop_checker: DropChecker,
    mut_checker: MutChecker,
    ref_checker: RefChecker,
    clone_checker: CloneChecker,
}

impl OwnershipChecker {
    /// 创建新的所有权检查器
    pub fn new() -> Self {
        Self {
            move_checker: MoveChecker::new(),
            drop_checker: DropChecker::new(),
            mut_checker: MutChecker::new(),
            ref_checker: RefChecker::new(),
            clone_checker: CloneChecker::default(),
        }
    }

    /// 检查函数的所有权语义
    pub fn check_function(
        &mut self,
        func: &FunctionIR,
    ) -> Vec<OwnershipError> {
        let move_errors = self.move_checker.check_function(func);
        let drop_errors = self.drop_checker.check_function(func);
        let mut_errors = self.mut_checker.check_function(func);
        let ref_errors = self.ref_checker.check_function(func);
        let clone_errors = self.clone_checker.check_function(func);

        // 合并错误
        move_errors
            .iter()
            .chain(drop_errors)
            .chain(mut_errors)
            .chain(ref_errors)
            .chain(clone_errors)
            .cloned()
            .collect()
    }

    /// 获取 Move 检查器的错误
    pub fn move_errors(&self) -> &[OwnershipError] {
        &self.move_checker.errors
    }

    /// 获取 Drop 检查器的错误
    pub fn drop_errors(&self) -> &[OwnershipError] {
        &self.drop_checker.errors
    }

    /// 获取 Mut 检查器的错误
    pub fn mut_errors(&self) -> &[OwnershipError] {
        self.mut_checker.errors()
    }

    /// 获取 Ref 检查器的错误
    pub fn ref_errors(&self) -> &[OwnershipError] {
        self.ref_checker.errors()
    }

    /// 获取 Clone 检查器的错误
    pub fn clone_errors(&self) -> &[OwnershipError] {
        self.clone_checker.errors()
    }
}

impl Default for OwnershipChecker {
    fn default() -> Self {
        Self::new()
    }
}

/// 所有权分析器（保留原有实现，用于引用计数插入）
#[derive(Debug)]
pub struct OwnershipAnalyzer {
    /// 所有权图
    ownership_graph: OwnershipGraph,
    /// 变量定义
    definitions: HashMap<Operand, Definition>,
    /// 活跃变量分析
    live_vars: HashMap<usize, HashSet<Operand>>,
    /// 当前作用域的变量
    scope_vars: HashSet<Operand>,
    /// 需要释放的变量
    drop_points: HashMap<usize, Vec<Operand>>,
}

impl OwnershipAnalyzer {
    /// 创建新的所有权分析器
    pub fn new() -> Self {
        Self {
            ownership_graph: OwnershipGraph::default(),
            definitions: HashMap::new(),
            live_vars: HashMap::new(),
            scope_vars: HashSet::new(),
            drop_points: HashMap::new(),
        }
    }

    /// 分析函数的所有权
    pub fn analyze_function(
        &mut self,
        func: &FunctionIR,
    ) -> OwnershipAnalysisResult {
        // 重置状态
        self.ownership_graph = OwnershipGraph::default();
        self.definitions = HashMap::new();
        self.live_vars = HashMap::new();
        self.scope_vars = HashSet::new();
        self.drop_points = HashMap::new();

        // 1. 构建活跃变量分析
        self.liveness_analysis(func);

        // 2. 分析所有权关系
        self.analyze_ownership(func);

        // 3. 确定释放点
        self.compute_drop_points(func);

        OwnershipAnalysisResult {
            ownership_graph: self.ownership_graph.clone(),
            definitions: self.definitions.clone(),
            drop_points: self.drop_points.clone(),
        }
    }

    /// 活跃变量分析（反向数据流）
    fn liveness_analysis(
        &mut self,
        func: &FunctionIR,
    ) {
        // 初始化：每个基本块的活跃变量集
        for (block_idx, _) in func.blocks.iter().enumerate() {
            self.live_vars.insert(block_idx, HashSet::new());
        }

        // 迭代直到不动点
        let mut changed = true;
        while changed {
            changed = false;

            for (block_idx, block) in func.blocks.iter().enumerate() {
                let mut live_out = HashSet::new();

                // 收集后继块的入口活跃变量
                for &succ in &block.successors {
                    if let Some(succ_live) = self.live_vars.get(&succ) {
                        live_out.extend(succ_live.iter().cloned());
                    }
                }

                let mut live_in = HashSet::new();
                live_in.extend(live_out.iter().cloned());

                // 计算活跃变量（反向遍历）
                let mut block_live = HashSet::new();
                for instr in block.instructions.iter().rev() {
                    self.update_live_vars(instr, &mut block_live, &mut live_in);
                }

                // 检查是否有变化
                let current_live = self.live_vars.get(&block_idx).cloned().unwrap_or_default();
                if current_live != block_live {
                    self.live_vars.insert(block_idx, block_live);
                    changed = true;
                }
            }
        }
    }

    /// 更新活跃变量
    fn update_live_vars(
        &self,
        instr: &Instruction,
        block_live: &mut HashSet<Operand>,
        live_in: &mut HashSet<Operand>,
    ) {
        match instr {
            // Move：定义新值，旧值不再活跃（所有权转移）
            Instruction::Move { dst, src } => {
                block_live.remove(dst);
                block_live.insert(src.clone());
                live_in.insert(src.clone());
            }

            // 加载：定义 dst，src 活跃
            Instruction::LoadIndex { dst, src, index } => {
                block_live.remove(dst);
                block_live.insert(src.clone());
                block_live.insert(index.clone());
            }
            Instruction::LoadField { dst, src, .. } => {
                block_live.remove(dst);
                block_live.insert(src.clone());
            }

            // 存储：src 和 dst 都活跃
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

            // 函数调用：参数活跃，返回值定义新变量
            Instruction::Call { dst, args, .. } => {
                if let Some(d) = dst {
                    block_live.remove(d);
                }
                for arg in args {
                    block_live.insert(arg.clone());
                }
            }

            // 返回：返回值活跃
            Instruction::Ret(Some(value)) => {
                block_live.insert(value.clone());
            }
            Instruction::Ret(None) => {}

            // 内存分配：定义新变量
            Instruction::HeapAlloc { dst, .. } => {
                block_live.remove(dst);
            }

            // 类型转换
            Instruction::Cast { dst, src, .. } => {
                block_live.remove(dst);
                block_live.insert(src.clone());
            }

            _ => {}
        }
    }

    /// 分析所有权关系
    fn analyze_ownership(
        &mut self,
        func: &FunctionIR,
    ) {
        for (block_idx, block) in func.blocks.iter().enumerate() {
            for (instr_idx, instr) in block.instructions.iter().enumerate() {
                self.analyze_instruction_ownership(instr, block_idx, instr_idx);
            }
        }
    }

    fn analyze_instruction_ownership(
        &mut self,
        instr: &Instruction,
        block_idx: usize,
        instr_idx: usize,
    ) {
        let pos = (block_idx, instr_idx);

        match instr {
            // Move：所有权从 src 转移到 dst
            Instruction::Move { dst, src } => {
                // 记录 dst 的定义
                self.definitions.insert(
                    dst.clone(),
                    Definition {
                        position: pos,
                        ty: None,
                        escapes: false,
                        is_moved: false,
                    },
                );

                // dst 拥有 src 的所有权（所有权转移）
                self.ownership_graph
                    .edges
                    .entry(dst.clone())
                    .or_default()
                    .insert(src.clone());

                // src 被移动后，不再拥有自己的所有权
                self.ownership_graph
                    .lifetimes
                    .insert(src.clone(), Lifetime::new(pos, pos));
            }

            // 函数调用：返回值拥有参数的所有权
            Instruction::Call {
                dst: Some(d), args, ..
            } => {
                self.definitions.insert(
                    d.clone(),
                    Definition {
                        position: pos,
                        ty: None,
                        escapes: false,
                        is_moved: false,
                    },
                );

                // 返回值拥有参数的所有权
                for arg in args {
                    self.ownership_graph
                        .edges
                        .entry(d.clone())
                        .or_default()
                        .insert(arg.clone());
                }
            }
            Instruction::Call {
                dst: None, args, ..
            } => {
                // 无返回值时，参数仍可能被使用
                for arg in args {
                    self.ownership_graph
                        .edges
                        .entry(arg.clone())
                        .or_default()
                        .insert(arg.clone());
                }
            }

            // 堆分配：新变量拥有新内存的所有权
            Instruction::HeapAlloc { dst, .. } => {
                self.definitions.insert(
                    dst.clone(),
                    Definition {
                        position: pos,
                        ty: None,
                        escapes: false,
                        is_moved: false,
                    },
                );

                self.ownership_graph
                    .lifetimes
                    .insert(dst.clone(), Lifetime::new(pos, pos));
            }

            // 闭包：闭包拥有捕获变量的所有权
            Instruction::MakeClosure { dst, env, .. } => {
                self.definitions.insert(
                    dst.clone(),
                    Definition {
                        position: pos,
                        ty: None,
                        escapes: false,
                        is_moved: false,
                    },
                );

                for var in env {
                    self.ownership_graph
                        .edges
                        .entry(dst.clone())
                        .or_default()
                        .insert(var.clone());
                }
            }

            _ => {}
        }
    }

    /// 计算释放点
    fn compute_drop_points(
        &mut self,
        func: &FunctionIR,
    ) {
        for (block_idx, _block) in func.blocks.iter().enumerate() {
            let mut drops = Vec::new();

            // 获取该块末尾的活跃变量
            let live_at_end = self.live_vars.get(&block_idx).cloned().unwrap_or_default();

            // 检查每个活跃变量是否应该被释放
            for var in &live_at_end {
                // 只释放局部变量和临时变量
                if matches!(var, Operand::Local(_) | Operand::Temp(_)) {
                    // 检查变量是否在当前作用域定义
                    if self.definitions.contains_key(var) {
                        drops.push(var.clone());
                    }
                }
            }

            if !drops.is_empty() {
                self.drop_points.insert(block_idx, drops);
            }
        }
    }

    /// 将分析结果应用到 IR
    ///
    /// 在作用域结束时插入 Drop 指令
    pub fn apply_to_ir(
        &self,
        func: &FunctionIR,
    ) -> FunctionIR {
        let mut new_func = func.clone();

        // 按块索引倒序处理（从后往前插入不影响索引）
        let mut block_indices: Vec<usize> = self.drop_points.keys().cloned().collect();
        block_indices.sort_by(|a, b| b.cmp(a));

        for block_idx in block_indices {
            if block_idx >= new_func.blocks.len() {
                continue;
            }

            let drops = match self.drop_points.get(&block_idx) {
                Some(d) => d.clone(),
                None => continue,
            };

            let block = &mut new_func.blocks[block_idx];

            // 在块末尾插入 Drop 指令
            for var in drops {
                block.instructions.push(Instruction::Drop(var));
            }
        }

        new_func
    }
}

impl Default for OwnershipAnalyzer {
    fn default() -> Self {
        Self::new()
    }
}

impl fmt::Display for Definition {
    fn fmt(
        &self,
        f: &mut fmt::Formatter<'_>,
    ) -> fmt::Result {
        write!(
            f,
            "Definition at {:?}, escapes={}, moved={}",
            self.position, self.escapes, self.is_moved
        )
    }
}

#[cfg(test)]
mod tests;
