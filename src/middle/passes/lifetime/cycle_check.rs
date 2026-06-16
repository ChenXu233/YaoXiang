//! 跨 spawn 循环引用检测
//!
//! 只追踪 spawn 的参数和返回值边界：
//! - spawn 参数（传入的 ref）
//! - spawn 返回值（传出的 ref）
//! - 检测参数和返回值之间是否形成环
//!
//! 检测限制：
//! - 只检测单层 spawn 边界（深度 = 1）
//! - 不递归检测嵌套 spawn 的间接引用
//! - unsafe 块内的 ref 操作跳过检测
//!
//! 单函数内循环和 spawn 内部循环由 IntraTaskCycleTracker 处理（警告模式）。

use crate::middle::core::ir::{FunctionIR, Instruction, Operand};
use crate::util::diagnostic::{ErrorCodeDefinition, Diagnostic};
use std::collections::{HashMap, HashSet};
use super::state_utils::{operand_display_name};

/// 检测深度限制：只检测直接边界，不递归进入嵌套 spawn
/// 用于文档说明，实际通过 `find_spawn_result_direct` 实现深度限制
#[allow(dead_code)]
const MAX_DETECTION_DEPTH: usize = 1;

/// 循环检测器
#[derive(Debug, Default)]
pub struct CycleChecker {
    /// 跨 spawn 引用边：spawn 返回值持有外部 ref
    pub(crate) spawn_ref_edges: Vec<SpawnRefEdge>,
    /// 跨 spawn 参数边：spawn 参数来自另一个 spawn 返回值
    pub(crate) spawn_param_edges: Vec<SpawnParamEdge>,
    /// 错误
    pub(crate) errors: Vec<Diagnostic>,
    /// spawn 返回值 → 所在基本块/指令位置
    pub(crate) spawn_results: HashMap<Operand, (usize, usize)>,
    /// 值定义追踪（用于追踪 ref 的来源）
    value_defs: HashMap<Operand, (Operand, (usize, usize))>,
    /// unsafe 块范围：(block_idx, start_instr, end_instr)
    pub(crate) unsafe_ranges: Vec<(usize, usize, usize)>,
    /// unsafe 绕过记录（信息级别）
    pub(crate) unsafe_bypasses: Vec<Diagnostic>,
    /// 局部变量名列表（用于错误报告中显示源码变量名）
    local_names: Option<Vec<String>>,
}

impl CycleChecker {
    /// 创建新的循环检测器
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

    /// 检查指令位置是否在 unsafe 块内
    fn is_in_unsafe(
        &self,
        block_idx: usize,
        instr_idx: usize,
    ) -> bool {
        self.unsafe_ranges
            .iter()
            .any(|(b, start, end)| *b == block_idx && instr_idx >= *start && instr_idx <= *end)
    }

    /// 获取 unsafe 绕过记录
    pub fn unsafe_bypasses(&self) -> &[Diagnostic] {
        &self.unsafe_bypasses
    }
}

/// spawn 返回值持有外部 ref 的边
#[derive(Debug, Clone)]
pub(crate) struct SpawnRefEdge {
    /// spawn 返回值（持有 ref 的值）
    spawn_result: Operand,
    /// 返回值持有的 ref 目标
    ref_target: Operand,
}

/// spawn 参数来自另一个 spawn 返回值
#[derive(Debug, Clone)]
pub(crate) struct SpawnParamEdge {
    /// 接收参数的 spawn 返回值
    consumer_spawn: Operand,
    /// 提供参数的 spawn 返回值
    producer_spawn: Operand,
}

impl CycleChecker {
    /// 检查函数的跨 spawn 循环引用
    pub fn check_function(
        &mut self,
        func: &FunctionIR,
    ) -> &[Diagnostic] {
        self.errors.clear();
        self.spawn_ref_edges.clear();
        self.spawn_param_edges.clear();
        self.spawn_results.clear();
        self.value_defs.clear();
        self.unsafe_ranges.clear();
        self.unsafe_bypasses.clear();

        // 0. 收集 unsafe 块范围
        self.collect_unsafe_ranges(func);

        // 1. 收集 spawn 信息
        self.collect_spawn_edges(func);

        // 2. 构建跨 spawn 引用图（限制深度 = 1）
        let graph = self.build_spawn_graph();

        // 3. 检测环（循环路径在 detect_cycle_dfs 中收集）
        self.has_cycle(&graph);

        &self.errors
    }

    /// 收集 unsafe 块范围
    ///
    /// Phase 7 实现 unsafe 语法后，解析 UnsafeBlockStart/End 指令。
    pub(crate) fn collect_unsafe_ranges(
        &mut self,
        func: &FunctionIR,
    ) {
        // 遍历所有基本块和指令
        for (block_idx, block) in func.blocks.iter().enumerate() {
            let mut in_unsafe = false;
            let mut unsafe_start = None;

            for (instr_idx, instr) in block.instructions.iter().enumerate() {
                match instr {
                    Instruction::UnsafeBlockStart => {
                        // 开始 unsafe 块
                        in_unsafe = true;
                        unsafe_start = Some(instr_idx);
                    }
                    Instruction::UnsafeBlockEnd
                        // 结束 unsafe 块
                        if in_unsafe => {
                            if let Some(start) = unsafe_start {
                                self.unsafe_ranges.push((block_idx, start, instr_idx));
                            }
                            in_unsafe = false;
                            unsafe_start = None;
                        }
                    _ => {}
                }
            }
        }
    }

    /// 收集 spawn 的参数和返回值信息
    ///
    /// 只收集单层 spawn 边界的直接引用，不递归进入嵌套 spawn。
    fn collect_spawn_edges(
        &mut self,
        func: &FunctionIR,
    ) {
        // 第一遍：收集所有 spawn 信息和 Move 定义
        for (block_idx, block) in func.blocks.iter().enumerate() {
            for (instr_idx, instr) in block.instructions.iter().enumerate() {
                match instr {
                    Instruction::Spawn {
                        closures: _,
                        plan: _,
                        result,
                    } => {
                        self.spawn_results
                            .insert(result.clone(), (block_idx, instr_idx));
                    }
                    Instruction::Move { dst, src } => {
                        self.value_defs
                            .insert(dst.clone(), (src.clone(), (block_idx, instr_idx)));
                    }
                    _ => {}
                }
            }
        }

        // 第二遍：建立跨 spawn 边（过滤 unsafe 块内的操作）
        for (block_idx, block) in func.blocks.iter().enumerate() {
            for (instr_idx, instr) in block.instructions.iter().enumerate() {
                // 跳过 unsafe 块内的操作
                if self.is_in_unsafe(block_idx, instr_idx) {
                    // 记录 unsafe 绕过信息
                    if let Instruction::Spawn { result, .. } = instr {
                        let details = format!(
                            "spawn {} in unsafe block, cycle detection bypassed",
                            operand_display_name(result, self.local_names.as_ref())
                        );
                        self.unsafe_bypasses
                            .push(ErrorCodeDefinition::ownership_violation(&details).build());
                    }
                    continue;
                }

                if let Instruction::Spawn {
                    closures,
                    plan: _,
                    result,
                } = instr
                {
                    // 参数边：arg 来自另一个 spawn 的结果（深度 = 1，只检测直接来源）
                    for arg in closures {
                        if let Some(producer) = self.find_spawn_result_direct(arg) {
                            self.spawn_param_edges.push(SpawnParamEdge {
                                consumer_spawn: result.clone(),
                                producer_spawn: producer,
                            });
                        }
                    }
                }
            }
        }

        // 第三遍：通过 Move 指令建立持有边（过滤 unsafe）
        // 如果 Move 的 src 来自另一个 spawn 的结果，则 dst 持有 src
        for (dst, (src, move_span)) in &self.value_defs.clone() {
            // 跳过 unsafe 块内的 Move
            if self.is_in_unsafe(move_span.0, move_span.1) {
                continue;
            }

            if let Some(producer) = self.find_spawn_result_direct(src) {
                // dst 持有 producer（通过 Move producer 到 dst）
                // 只有当 dst 是 spawn 结果时才建立持有边
                // 排除自引用：dst 和 producer 相同
                if self.spawn_results.contains_key(dst) && dst != &producer {
                    self.spawn_ref_edges.push(SpawnRefEdge {
                        spawn_result: dst.clone(),
                        ref_target: producer,
                    });
                }
            }
        }
    }

    /// 查找某个值是否直接来自某个 spawn 的返回值（深度限制 = 1）
    ///
    /// 只追踪一层，不递归查找间接来源。
    fn find_spawn_result_direct(
        &self,
        val: &Operand,
    ) -> Option<Operand> {
        // 直接是 spawn 结果
        if self.spawn_results.contains_key(val) {
            return Some(val.clone());
        }

        // 只追踪一层 Move（深度 = 1）
        if let Some((source, _)) = self.value_defs.get(val) {
            if self.spawn_results.contains_key(source) {
                return Some(source.clone());
            }
        }

        None
    }

    /// 构建跨 spawn 引用图
    ///
    /// 图结构：spawn_result -> spawn_result（单层边界）
    /// 深度限制：只包含直接参数/返回值边，不递归。
    fn build_spawn_graph(&self) -> HashMap<Operand, HashSet<Operand>> {
        let mut graph: HashMap<Operand, HashSet<Operand>> = HashMap::new();

        // 参数边：producer → consumer
        // 表示 consumer spawn 使用了 producer spawn 的返回值作为参数
        for edge in &self.spawn_param_edges {
            graph
                .entry(edge.producer_spawn.clone())
                .or_default()
                .insert(edge.consumer_spawn.clone());
        }

        // 持有边：holder → held（通过 Move）
        // 表示 holder spawn 的返回值持有 held spawn 的返回值
        for edge in &self.spawn_ref_edges {
            graph
                .entry(edge.ref_target.clone())
                .or_default()
                .insert(edge.spawn_result.clone());
        }

        graph
    }

    /// 检测是否有环（简化版：只检测 spawn 参数/返回值之间）
    fn has_cycle(
        &mut self,
        graph: &HashMap<Operand, HashSet<Operand>>,
    ) -> bool {
        let mut visited = HashSet::new();
        let mut recursion_stack = HashSet::new();
        let mut path = Vec::new();

        for node in graph.keys() {
            if !visited.contains(node)
                && self.detect_cycle_dfs(node, graph, &mut visited, &mut recursion_stack, &mut path)
            {
                return true;
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
                    self.errors.push(
                        ErrorCodeDefinition::ownership_violation(
                            &self.format_cycle_path(path, neighbor),
                        )
                        .build(),
                    );
                    return true;
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
        // 找到环的起始位置
        let start_idx = path.iter().position(|p| p == cycle_start).unwrap_or(0);
        let cycle_nodes = &path[start_idx..];

        if cycle_nodes.is_empty() {
            return "检测到跨 spawn 循环引用".to_string();
        }

        let cycle_strs: Vec<String> = cycle_nodes
            .iter()
            .map(|p| operand_display_name(p, self.local_names.as_ref()))
            .collect();

        format!(
            "跨任务循环引用: {} → {} (形成环). 建议: 使用 Weak 打破循环，或在 unsafe 块中绕过检测",
            cycle_strs.join(" → "),
            cycle_strs.first().unwrap_or(&"?".to_string())
        )
    }
}

/// 为 CycleChecker 实现 Checker trait
impl super::state_utils::Checker for CycleChecker {
    fn check_function(
        &mut self,
        func: &FunctionIR,
    ) -> &[Diagnostic] {
        self.check_function(func)
    }

    fn errors(&self) -> &[Diagnostic] {
        &self.errors
    }

    fn clear(&mut self) {
        self.errors.clear();
        self.spawn_ref_edges.clear();
        self.spawn_param_edges.clear();
        self.spawn_results.clear();
        self.value_defs.clear();
        self.unsafe_ranges.clear();
        self.unsafe_bypasses.clear();
    }
}
