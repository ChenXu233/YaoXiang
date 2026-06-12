//! 任务内循环引用追踪
//!
//! 任务内循环是允许的（泄漏可控，任务结束后释放）。
//! 此模块检测并记录任务内的 ref 循环，以警告形式输出。
//!
//! 设计原则：
//! - 任务内循环不阻断编译
//! - 记录泄漏点位置供调试
//! - 与 CycleChecker（跨任务检测）协同工作

use crate::middle::core::ir::{FunctionIR, Instruction, Operand};
use crate::util::diagnostic::{ErrorCodeDefinition, Diagnostic};
use std::collections::{HashMap, HashSet};
use super::error::{operand_display_name};

/// 任务内循环追踪器
#[derive(Debug, Default)]
pub struct IntraTaskCycleTracker {
    /// 任务内 ref 边
    pub(crate) ref_edges: Vec<RefEdge>,
    /// 检测到的循环（警告）
    pub(crate) warnings: Vec<Diagnostic>,
    /// ArcNew 指令位置追踪
    pub(crate) arc_new_locations: HashMap<Operand, (usize, usize)>,
    /// 值定义追踪（预留，用于更复杂的数据流分析）
    #[allow(dead_code)]
    value_defs: HashMap<Operand, Operand>,
    /// 局部变量名列表（用于警告报告中显示源码变量名）
    local_names: Option<Vec<String>>,
}

/// ref 边
#[derive(Debug, Clone)]
pub(crate) struct RefEdge {
    /// 源操作数（创建 ref 的值）
    from: Operand,
    /// 目标操作数（被 ref 的值）
    to: Operand,
}

impl IntraTaskCycleTracker {
    /// 创建新的追踪器
    pub fn new() -> Self {
        Self::default()
    }

    /// 设置局部变量名列表
    pub fn set_local_names(
        &mut self,
        local_names: Option<Vec<String>>,
    ) {
        self.local_names = local_names;
    }

    /// 追踪函数中的任务内循环
    pub fn track_function(
        &mut self,
        func: &FunctionIR,
    ) -> &[Diagnostic] {
        self.clear();

        // 1. 收集 ArcNew 和值定义
        self.collect_ref_info(func);

        // 2. 构建 ref 图
        let graph = self.build_ref_graph();

        // 3. 检测任务内循环
        self.detect_intra_task_cycles(&graph);

        &self.warnings
    }

    /// 收集 ref 相关信息
    fn collect_ref_info(
        &mut self,
        func: &FunctionIR,
    ) {
        for (block_idx, block) in func.blocks.iter().enumerate() {
            for (instr_idx, instr) in block.instructions.iter().enumerate() {
                match instr {
                    // ArcNew：创建 ref
                    Instruction::ArcNew { dst, src } => {
                        self.arc_new_locations
                            .insert(dst.clone(), (block_idx, instr_idx));
                        self.ref_edges.push(RefEdge {
                            from: dst.clone(),
                            to: src.clone(),
                        });
                    }
                    // Move：追踪值流动
                    Instruction::Move { dst, src } => {
                        self.value_defs.insert(dst.clone(), src.clone());
                    }
                    // Store：追踪赋值
                    Instruction::Store { dst, src, .. } => {
                        self.value_defs.insert(dst.clone(), src.clone());
                    }
                    // StoreField：追踪字段赋值（可能形成循环）
                    Instruction::StoreField { dst, src, .. }
                        // dst.field = src，如果 src 是 ref，可能形成循环
                        if self.arc_new_locations.contains_key(src) => {
                            self.ref_edges.push(RefEdge {
                                from: dst.clone(),
                                to: src.clone(),
                            });
                        }
                    _ => {}
                }
            }
        }
    }

    /// 构建 ref 引用图
    fn build_ref_graph(&self) -> HashMap<Operand, HashSet<Operand>> {
        let mut graph: HashMap<Operand, HashSet<Operand>> = HashMap::new();

        for edge in &self.ref_edges {
            graph
                .entry(edge.from.clone())
                .or_default()
                .insert(edge.to.clone());
        }

        graph
    }

    /// 检测任务内循环（DFS）
    fn detect_intra_task_cycles(
        &mut self,
        graph: &HashMap<Operand, HashSet<Operand>>,
    ) {
        let mut visited = HashSet::new();
        let mut recursion_stack = HashSet::new();
        let mut path = Vec::new();

        for node in graph.keys() {
            if !visited.contains(node) {
                self.detect_cycle_dfs(node, graph, &mut visited, &mut recursion_stack, &mut path);
            }
        }
    }

    /// DFS 检测循环
    fn detect_cycle_dfs(
        &mut self,
        node: &Operand,
        graph: &HashMap<Operand, HashSet<Operand>>,
        visited: &mut HashSet<Operand>,
        recursion_stack: &mut HashSet<Operand>,
        path: &mut Vec<Operand>,
    ) -> bool {
        visited.insert(node.clone());
        recursion_stack.insert(node.clone());
        path.push(node.clone());

        if let Some(neighbors) = graph.get(node) {
            for neighbor in neighbors {
                if !visited.contains(neighbor) {
                    if self.detect_cycle_dfs(neighbor, graph, visited, recursion_stack, path) {
                        return true;
                    }
                } else if recursion_stack.contains(neighbor) {
                    // 找到任务内循环，记录警告
                    let cycle_path = self.format_cycle_path(path, neighbor);
                    let _span = self.find_cycle_span(neighbor);
                    self.warnings
                        .push(ErrorCodeDefinition::ownership_violation(&cycle_path).build());
                    // 继续检测其他循环，不立即返回
                }
            }
        }

        path.pop();
        recursion_stack.remove(node);
        false
    }

    /// 格式化循环路径
    fn format_cycle_path(
        &self,
        path: &[Operand],
        cycle_start: &Operand,
    ) -> String {
        let start_idx = path.iter().position(|p| p == cycle_start).unwrap_or(0);
        let cycle_nodes = &path[start_idx..];

        if cycle_nodes.is_empty() {
            return "任务内 ref 循环".to_string();
        }

        let cycle_strs: Vec<String> = cycle_nodes
            .iter()
            .map(|p| operand_display_name(p, self.local_names.as_ref()))
            .collect();

        format!(
            "任务内循环: {} → {} (泄漏在任务结束后释放)",
            cycle_strs.join(" → "),
            cycle_strs.first().unwrap_or(&"?".to_string())
        )
    }

    /// 获取循环位置
    fn find_cycle_span(
        &self,
        node: &Operand,
    ) -> (usize, usize) {
        self.arc_new_locations.get(node).copied().unwrap_or((0, 0))
    }

    /// 获取警告列表
    pub fn warnings(&self) -> &[Diagnostic] {
        &self.warnings
    }

    /// 清除状态
    pub fn clear(&mut self) {
        self.ref_edges.clear();
        self.warnings.clear();
        self.arc_new_locations.clear();
        self.value_defs.clear();
    }
}
