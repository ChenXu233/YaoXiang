//! 跨 spawn 循环引用检测
//!
//! 只追踪 spawn 的参数和返回值边界：
//! - spawn 参数（传入的 ref）
//! - spawn 返回值（传出的 ref）
//! - 检测参数和返回值之间是否形成环
//!
//! 单函数内循环和 spawn 内部循环由 OwnershipChecker 处理（允许）。

use crate::middle::ir::{FunctionIR, Instruction, Operand};
use std::collections::{HashMap, HashSet};
use super::error::OwnershipError;

/// 循环检测器
#[derive(Debug, Default)]
pub struct CycleChecker {
    /// 跨 spawn 引用边：spawn 返回值持有外部 ref
    spawn_ref_edges: Vec<SpawnRefEdge>,
    /// 跨 spawn 参数边：spawn 参数来自另一个 spawn 返回值
    spawn_param_edges: Vec<SpawnParamEdge>,
    /// 错误
    errors: Vec<OwnershipError>,
    /// spawn 返回值 → 所在基本块/指令位置
    spawn_results: HashMap<Operand, (usize, usize)>,
    /// 值定义追踪（用于追踪 ref 的来源）
    value_defs: HashMap<Operand, (Operand, (usize, usize))>,
}

impl CycleChecker {
    /// 创建新的循环检测器
    pub fn new() -> Self {
        Self::default()
    }
}

/// spawn 返回值持有外部 ref 的边
#[derive(Debug, Clone)]
struct SpawnRefEdge {
    /// spawn 返回值（持有 ref 的值）
    spawn_result: Operand,
    /// 返回值持有的 ref 目标
    ref_target: Operand,
    /// 位置
    span: (usize, usize),
}

/// spawn 参数来自另一个 spawn 返回值
#[derive(Debug, Clone)]
struct SpawnParamEdge {
    /// 接收参数的 spawn 返回值
    consumer_spawn: Operand,
    /// 提供参数的 spawn 返回值
    producer_spawn: Operand,
    /// 位置
    span: (usize, usize),
}

impl CycleChecker {
    /// 检查函数的跨 spawn 循环引用
    pub fn check_function(&mut self, func: &FunctionIR) -> &[OwnershipError] {
        self.errors.clear();
        self.spawn_ref_edges.clear();
        self.spawn_param_edges.clear();
        self.spawn_results.clear();
        self.value_defs.clear();

        // 1. 收集 spawn 信息
        self.collect_spawn_edges(func);

        // 2. 构建跨 spawn 引用图
        let graph = self.build_spawn_graph();

        // 3. 检测环（循环路径在 detect_cycle_dfs 中收集）
        self.has_cycle(&graph);

        &self.errors
    }

    /// 收集 spawn 的参数和返回值信息
    fn collect_spawn_edges(&mut self, func: &FunctionIR) {
        // 第一遍：收集所有 spawn 信息和 Move 定义
        for (block_idx, block) in func.blocks.iter().enumerate() {
            for (instr_idx, instr) in block.instructions.iter().enumerate() {
                match instr {
                    Instruction::Spawn {
                        func: _,
                        args: _,
                        result,
                    } => {
                        self.spawn_results.insert(result.clone(), (block_idx, instr_idx));
                    }
                    Instruction::Move { dst, src } => {
                        self.value_defs.insert(dst.clone(), (src.clone(), (block_idx, instr_idx)));
                    }
                    _ => {}
                }
            }
        }

        // 第二遍：建立跨 spawn 边
        for (block_idx, block) in func.blocks.iter().enumerate() {
            for (instr_idx, instr) in block.instructions.iter().enumerate() {
                match instr {
                    Instruction::Spawn {
                        func: _,
                        args,
                        result,
                    } => {
                        // 参数边：arg 来自另一个 spawn 的结果
                        for arg in args {
                            if let Some(producer) = self.find_spawn_result(arg) {
                                self.spawn_param_edges.push(SpawnParamEdge {
                                    consumer_spawn: result.clone(),
                                    producer_spawn: producer,
                                    span: (block_idx, instr_idx),
                                });
                            }
                        }
                    }
                    _ => {}
                }
            }
        }

        // 第三遍：通过 Move 指令建立持有边
        // 如果 Move 的 src 来自另一个 spawn 的结果，则 dst 持有 src
        for (dst, (src, move_span)) in &self.value_defs {
            if let Some(producer) = self.find_spawn_result(src) {
                // dst 持有 producer（通过 Move producer 到 dst）
                // 只有当 dst 是 spawn 结果时才建立持有边
                // 排除自引用：dst 和 producer 相同
                if self.spawn_results.contains_key(dst) && dst != &producer {
                    self.spawn_ref_edges.push(SpawnRefEdge {
                        spawn_result: dst.clone(),
                        ref_target: producer,
                        span: *move_span, // 使用 Move 指令的真实位置
                    });
                }
            }
        }
    }

    /// 查找某个值是否来自某个 spawn 的返回值
    fn find_spawn_result(&self, val: &Operand) -> Option<Operand> {
        if self.spawn_results.contains_key(val) {
            return Some(val.clone());
        }

        let mut current = val.clone();
        let mut visited = HashSet::new();

        loop {
            if visited.contains(&current) {
                return None;
            }
            visited.insert(current.clone());

            if let Some((source, _)) = self.value_defs.get(&current) {
                if self.spawn_results.contains_key(source) {
                    return Some(source.clone());
                }
                current = source.clone();
            } else {
                return None;
            }
        }
    }

    /// 构建跨 spawn 引用图
    fn build_spawn_graph(&self) -> HashMap<Operand, HashSet<Operand>> {
        let mut graph: HashMap<Operand, HashSet<Operand>> = HashMap::new();

        // 参数边：producer → consumer
        for edge in &self.spawn_param_edges {
            graph
                .entry(edge.producer_spawn.clone())
                .or_default()
                .insert(edge.consumer_spawn.clone());
        }

        // 持有边：holder → held（通过 Move）
        for edge in &self.spawn_ref_edges {
            graph
                .entry(edge.ref_target.clone())
                .or_default()
                .insert(edge.spawn_result.clone());
        }

        graph
    }

    /// 检测是否有环（简化版：只检测 spawn 参数/返回值之间）
    fn has_cycle(&mut self, graph: &HashMap<Operand, HashSet<Operand>>) -> bool {
        let mut visited = HashSet::new();
        let mut recursion_stack = HashSet::new();
        let mut path = Vec::new();

        for node in graph.keys() {
            if !visited.contains(node) {
                if self.detect_cycle_dfs(node, graph, &mut visited, &mut recursion_stack, &mut path) {
                    return true;
                }
            }
        }

        false
    }

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

        if let Some(edges) = graph.get(node) {
            for neighbor in edges {
                if !visited.contains(neighbor) {
                    if self.detect_cycle_dfs(neighbor, graph, visited, recursion_stack, path) {
                        return true;
                    }
                } else if recursion_stack.contains(neighbor) {
                    // 找到环！path 中从 neighbor 到末尾就是环
                    self.errors.push(OwnershipError::CrossSpawnCycle {
                        details: self.format_cycle_path(path, neighbor),
                        span: self.find_cycle_span(graph, neighbor),
                    });
                    return true;
                }
            }
        }

        path.pop();
        recursion_stack.remove(node);
        false
    }

    /// 格式化循环路径
    fn format_cycle_path(&self, path: &Vec<Operand>, cycle_start: &Operand) -> String {
        // 找到环的起始位置
        let start_idx = path.iter().position(|p| p == cycle_start).unwrap_or(0);
        let cycle_nodes = &path[start_idx..];

        if cycle_nodes.is_empty() {
            return "检测到跨 spawn 循环引用".to_string();
        }

        let cycle_strs: Vec<String> = cycle_nodes
            .iter()
            .map(|p| self.operand_to_string(p))
            .collect();

        format!("循环引用: {} → ... → {} → (回到起点)",
            cycle_strs.join(" → "),
            cycle_strs.first().unwrap_or(&"?".to_string()))
    }

    /// Operand 转字符串
    fn operand_to_string(&self, op: &Operand) -> String {
        match op {
            Operand::Local(idx) => format!("local_{}", idx),
            Operand::Arg(idx) => format!("arg_{}", idx),
            Operand::Temp(idx) => format!("temp_{}", idx),
            Operand::Global(idx) => format!("global_{}", idx),
            Operand::Const(c) => format!("{:?}", c),
            Operand::Label(idx) => format!("label_{}", idx),
            Operand::Register(idx) => format!("reg_{}", idx),
        }
    }

    /// 找到环的位置（使用 cycle_start 作为参考点）
    fn find_cycle_span(&self, _graph: &HashMap<Operand, HashSet<Operand>>, cycle_start: &Operand) -> (usize, usize) {
        // 返回环起始点的位置
        if let Some(span) = self.spawn_results.get(cycle_start) {
            return *span;
        }
        (0, 0)
    }
}

/// 为 CycleChecker 实现 OwnershipCheck trait
impl super::error::OwnershipCheck for CycleChecker {
    fn check_function(
        &mut self,
        func: &FunctionIR,
    ) -> &[OwnershipError] {
        self.check_function(func)
    }

    fn errors(&self) -> &[OwnershipError] {
        &self.errors
    }

    fn state(&self) -> &HashMap<Operand, super::error::ValueState> {
        unimplemented!()
    }

    fn clear(&mut self) {
        self.errors.clear();
        self.spawn_ref_edges.clear();
        self.spawn_param_edges.clear();
        self.spawn_results.clear();
        self.value_defs.clear();
    }
}
