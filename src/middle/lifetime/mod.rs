//! 生命周期分析与引用计数插入
//!
//! 自动为堆分配对象插入 Retain/Release 指令，确保内存正确释放。
//! 设计原则：
//! 1. 作用域结束时插入 Release
//! 2. 跨函数调用时插入 Retain（在调用前）和 Release（调用后）
//! 3. 返回值自动 Retain，由调用者 Release

use crate::middle::ir::{BasicBlock, FunctionIR, Instruction, ModuleIR, Operand};
use std::collections::{HashMap, HashSet};
use std::fmt;

/// 生命周期分析结果
#[derive(Debug, Clone)]
pub struct LifetimeAnalysisResult {
    /// 需要插入 Retain 的位置
    pub retain_points: Vec<RetainPoint>,
    /// 需要插入 Release 的位置
    pub release_points: Vec<ReleasePoint>,
    /// 所有权关系图
    pub ownership_graph: OwnershipGraph,
}

/// Retain 插入点
#[derive(Debug, Clone)]
pub struct RetainPoint {
    /// 插入位置：基本块索引
    pub block_idx: usize,
    /// 插入位置：指令索引
    pub instr_idx: usize,
    /// 要 Retain 的操作数
    pub operand: Operand,
    /// 原因
    pub reason: RetainReason,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RetainReason {
    /// 函数参数传递
    ArgPass,
    /// 返回值传递
    ReturnValue,
    /// 赋值给全局变量
    AssignGlobal,
    /// 存储到容器
    StoreContainer,
}

/// Release 插入点
#[derive(Debug, Clone)]
pub struct ReleasePoint {
    /// 插入位置：基本块索引
    pub block_idx: usize,
    /// 插入位置：指令索引
    pub instr_idx: usize,
    /// 要 Release 的操作数
    pub operand: Operand,
    /// 释放原因
    pub reason: ReleaseReason,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReleaseReason {
    /// 作用域结束
    ScopeEnd,
    /// 函数返回后（调用者负责释放参数）
    AfterCall,
    /// 变量覆盖
    VariableOverride,
    /// 容器元素释放
    ContainerDrop,
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
    pub start: (usize, usize), // (block_idx, instr_idx)
    /// 结束位置
    pub end: (usize, usize),
    /// 是否逃逸
    pub escapes: bool,
}

impl Lifetime {
    pub fn new(start: (usize, usize), end: (usize, usize)) -> Self {
        Self {
            start,
            end,
            escapes: false,
        }
    }
}

/// 生命周期分析器
#[derive(Debug)]
pub struct LifetimeAnalyzer {
    /// 保留点集合
    retain_points: Vec<RetainPoint>,
    /// 释放点集合
    release_points: Vec<ReleasePoint>,
    /// 所有权图
    ownership_graph: OwnershipGraph,
    /// 当前作用域的变量
    current_scope_vars: HashSet<Operand>,
    /// 活跃变量分析
    live_vars: HashMap<usize, HashSet<Operand>>, // block_idx -> live vars
}

impl LifetimeAnalyzer {
    /// 创建新的生命周期分析器
    pub fn new() -> Self {
        Self {
            retain_points: Vec::new(),
            release_points: Vec::new(),
            ownership_graph: OwnershipGraph::default(),
            current_scope_vars: HashSet::new(),
            live_vars: HashMap::new(),
        }
    }

    /// 分析函数的生命周期
    pub fn analyze_function(&mut self, func: &FunctionIR) -> LifetimeAnalysisResult {
        // 重置状态
        self.retain_points.clear();
        self.release_points.clear();
        self.ownership_graph = OwnershipGraph::default();
        self.current_scope_vars.clear();
        self.live_vars.clear();

        // 1. 构建活跃变量分析
        self.liveness_analysis(func);

        // 2. 分析所有权关系
        self.analyze_ownership(func);

        // 3. 确定 Retain/Release 插入点
        self.insert_ref_counts(func);

        LifetimeAnalysisResult {
            retain_points: self.retain_points.clone(),
            release_points: self.release_points.clone(),
            ownership_graph: self.ownership_graph.clone(),
        }
    }

    /// 活跃变量分析（反向数据流）
    fn liveness_analysis(&mut self, func: &FunctionIR) {
        // 初始化：每个基本块的活跃变量集
        for (block_idx, block) in func.blocks.iter().enumerate() {
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
            // 定义新值：使旧值不再活跃，新值活跃
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
            Instruction::Ret(value) => {
                if let Some(v) = value {
                    block_live.insert(v.clone());
                }
            }

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
    fn analyze_ownership(&mut self, func: &FunctionIR) {
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
            // 赋值：新变量获得所有权
            Instruction::Move { dst, src } => {
                self.ownership_graph.lifetimes.insert(
                    dst.clone(),
                    Lifetime::new(pos, pos),
                );
                // dst 拥有 src 的所有权
                self.ownership_graph
                    .edges
                    .entry(dst.clone())
                    .or_insert_with(HashSet::new)
                    .insert(src.clone());
            }

            // 函数调用：返回值拥有参数的所有权（简化）
            Instruction::Call { dst, args, .. } => {
                if let Some(d) = dst {
                    self.ownership_graph.lifetimes.insert(
                        d.clone(),
                        Lifetime::new(pos, pos),
                    );
                    for arg in args {
                        self.ownership_graph
                            .edges
                            .entry(d.clone())
                            .or_insert_with(HashSet::new)
                            .insert(arg.clone());
                    }
                }
            }

            // 堆分配：新变量拥有新内存的所有权
            Instruction::HeapAlloc { dst, .. } => {
                self.ownership_graph.lifetimes.insert(
                    dst.clone(),
                    Lifetime::new(pos, pos),
                );
            }

            // 闭包：闭包拥有捕获变量的所有权
            Instruction::MakeClosure { dst, env, .. } => {
                self.ownership_graph.lifetimes.insert(
                    dst.clone(),
                    Lifetime::new(pos, pos),
                );
                for var in env {
                    self.ownership_graph
                        .edges
                        .entry(dst.clone())
                        .or_insert_with(HashSet::new)
                        .insert(var.clone());
                }
            }

            _ => {}
        }
    }

    /// 插入引用计数指令
    fn insert_ref_counts(&mut self, func: &FunctionIR) {
        for (block_idx, block) in func.blocks.iter().enumerate() {
            for (instr_idx, instr) in block.instructions.iter().enumerate() {
                self.insert_ref_count_for_instr(instr, block_idx, instr_idx);
            }
        }
    }

    fn insert_ref_count_for_instr(
        &mut self,
        instr: &Instruction,
        block_idx: usize,
        instr_idx: usize,
    ) {
        match instr {
            // 赋值时：如果目标已存在，需要 Release
            Instruction::Move { dst, src } => {
                // src 需要 Retain（获得所有权）
                self.retain_points.push(RetainPoint {
                    block_idx,
                    instr_idx,
                    operand: src.clone(),
                    reason: RetainReason::ReturnValue,
                });

                // dst 原来指向的对象需要 Release
                self.release_points.push(ReleasePoint {
                    block_idx,
                    instr_idx,
                    operand: dst.clone(),
                    reason: ReleaseReason::VariableOverride,
                });
            }

            // 函数调用时：参数需要 Retain，返回值需要 Release
            Instruction::Call { dst, args, .. } => {
                for arg in args {
                    self.retain_points.push(RetainPoint {
                        block_idx,
                        instr_idx,
                        operand: arg.clone(),
                        reason: RetainReason::ArgPass,
                    });
                }

                // 调用后：参数需要 Release（如果调用者不保留）
                for arg in args {
                    self.release_points.push(ReleasePoint {
                        block_idx,
                        instr_idx: instr_idx + 1,
                        operand: arg.clone(),
                        reason: ReleaseReason::AfterCall,
                    });
                }
            }

            // 返回时：返回值需要 Retain
            Instruction::Ret(value) => {
                if let Some(v) = value {
                    self.retain_points.push(RetainPoint {
                        block_idx,
                        instr_idx,
                        operand: v.clone(),
                        reason: RetainReason::ReturnValue,
                    });
                }
            }

            // 存储到容器：需要 Retain
            Instruction::StoreIndex { src, .. } => {
                self.retain_points.push(RetainPoint {
                    block_idx,
                    instr_idx,
                    operand: src.clone(),
                    reason: RetainReason::StoreContainer,
                });
            }

            // 字段存储：需要 Retain
            Instruction::StoreField { src, .. } => {
                self.retain_points.push(RetainPoint {
                    block_idx,
                    instr_idx,
                    operand: src.clone(),
                    reason: RetainReason::StoreContainer,
                });
            }

            _ => {}
        }
    }

    /// 将分析结果应用到 IR
    ///
    /// 将分析得到的 Retain/Release 点转换为实际的 IR 指令并插入
    pub fn apply_to_ir(&self, func: &FunctionIR) -> FunctionIR {
        let mut new_func = func.clone();

        // 收集所有需要插入的指令
        // (block_idx, instr_idx, instruction, is_retain)
        let mut insertions: Vec<(usize, usize, Instruction, bool)> = Vec::new();

        // 收集 Retain 插入点
        for point in &self.retain_points {
            if point.block_idx < new_func.blocks.len() {
                let retain_instr = Instruction::Retain(point.operand.clone());
                insertions.push((point.block_idx, point.instr_idx, retain_instr, true));
            }
        }

        // 收集 Release 插入点
        for point in &self.release_points {
            if point.block_idx < new_func.blocks.len() {
                let release_instr = Instruction::Release(point.operand.clone());
                insertions.push((point.block_idx, point.instr_idx, release_instr, false));
            }
        }

        // 按块索引和指令索引排序（倒序，以便从后往前插入不影响索引）
        insertions.sort_by(|a, b| {
            b.1.cmp(&a.1) // instr_idx 倒序
                .then_with(|| b.0.cmp(&a.0)) // block_idx 倒序
        });

        // 执行插入
        for (block_idx, instr_idx, instr, _is_retain) in insertions {
            if block_idx < new_func.blocks.len() {
                let block = &mut new_func.blocks[block_idx];
                // 在 instr_idx 位置插入（在其之后）
                let insert_idx = std::cmp::min(instr_idx + 1, block.instructions.len());
                block.instructions.insert(insert_idx, instr);
            }
        }

        // 添加作用域结束时的 Release
        self.insert_scope_end_releases(&mut new_func);

        new_func
    }

    /// 在作用域结束时插入 Release 指令
    fn insert_scope_end_releases(&self, func: &mut FunctionIR) {
        for (block_idx, block) in func.blocks.iter_mut().enumerate() {
            // 获取该块的活跃变量（在块末尾）
            let live_at_end = self.live_vars.get(&block_idx).cloned().unwrap_or_default();

            // 为每个在块末尾仍然活跃的堆分配变量添加 Release
            for operand in live_at_end {
                // 只为局部变量和临时变量添加 Release
                if matches!(operand, Operand::Local(_) | Operand::Temp(_)) {
                    // 在块的最后一个指令后添加 Release
                    let release_instr = Instruction::Release(operand);
                    block.instructions.push(release_instr);
                }
            }
        }
    }
}

impl Default for LifetimeAnalyzer {
    fn default() -> Self {
        Self::new()
    }
}

impl fmt::Display for RetainReason {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            RetainReason::ArgPass => write!(f, "参数传递"),
            RetainReason::ReturnValue => write!(f, "返回值传递"),
            RetainReason::AssignGlobal => write!(f, "赋值给全局变量"),
            RetainReason::StoreContainer => write!(f, "存储到容器"),
        }
    }
}

impl fmt::Display for ReleaseReason {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ReleaseReason::ScopeEnd => write!(f, "作用域结束"),
            ReleaseReason::AfterCall => write!(f, "函数调用后"),
            ReleaseReason::VariableOverride => write!(f, "变量覆盖"),
            ReleaseReason::ContainerDrop => write!(f, "容器释放"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::middle::ir::{BasicBlock, FunctionIR, Instruction, Operand};
    use crate::frontend::typecheck::MonoType;

    fn create_test_function() -> FunctionIR {
        FunctionIR {
            name: "test".to_string(),
            params: vec![MonoType::Int(64)],
            return_type: MonoType::Int(64),
            is_async: false,
            locals: vec![MonoType::Int(64), MonoType::Int(64)],
            blocks: vec![
                BasicBlock {
                    label: 0,
                    instructions: vec![
                        Instruction::Move {
                            dst: Operand::Temp(0),
                            src: Operand::Local(0),
                        },
                        Instruction::Move {
                            dst: Operand::Temp(1),
                            src: Operand::Local(1),
                        },
                        Instruction::Call {
                            dst: Some(Operand::Temp(2)),
                            func: Operand::Global(0),
                            args: vec![Operand::Temp(0), Operand::Temp(1)],
                        },
                        Instruction::Ret(Some(Operand::Temp(2))),
                    ],
                    successors: vec![],
                },
            ],
            entry: 0,
        }
    }

    #[test]
    fn test_lifetime_analysis() {
        let func = create_test_function();
        let mut analyzer = LifetimeAnalyzer::new();
        let result = analyzer.analyze_function(&func);

        // 函数调用应该产生 Retain 点
        assert!(!result.retain_points.is_empty() || !result.release_points.is_empty());

        println!("Retain points: {:?}", result.retain_points.len());
        println!("Release points: {:?}", result.release_points.len());
    }
}
