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
use std::collections::{HashMap, HashSet};
use super::error::OwnershipError;

/// 检测深度限制：只检测直接边界，不递归进入嵌套 spawn
/// 用于文档说明，实际通过 `find_spawn_result_direct` 实现深度限制
#[allow(dead_code)]
const MAX_DETECTION_DEPTH: usize = 1;

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
    /// unsafe 块范围：(block_idx, start_instr, end_instr)
    pub(crate) unsafe_ranges: Vec<(usize, usize, usize)>,
    /// unsafe 绕过记录（信息级别）
    unsafe_bypasses: Vec<OwnershipError>,
}

impl CycleChecker {
    /// 创建新的循环检测器
    pub fn new() -> Self {
        Self::default()
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
    pub fn unsafe_bypasses(&self) -> &[OwnershipError] {
        &self.unsafe_bypasses
    }
}

/// spawn 返回值持有外部 ref 的边
#[derive(Debug, Clone)]
struct SpawnRefEdge {
    /// spawn 返回值（持有 ref 的值）
    spawn_result: Operand,
    /// 返回值持有的 ref 目标
    ref_target: Operand,
}

/// spawn 参数来自另一个 spawn 返回值
#[derive(Debug, Clone)]
struct SpawnParamEdge {
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
    ) -> &[OwnershipError] {
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
                    Instruction::UnsafeBlockEnd => {
                        // 结束 unsafe 块
                        if in_unsafe {
                            if let Some(start) = unsafe_start {
                                self.unsafe_ranges.push((block_idx, start, instr_idx));
                            }
                            in_unsafe = false;
                            unsafe_start = None;
                        }
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
                        func: _,
                        args: _,
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
                        self.unsafe_bypasses
                            .push(OwnershipError::UnsafeBypassCycle {
                                details: format!(
                                    "spawn {} in unsafe block, cycle detection bypassed",
                                    self.operand_to_string(result)
                                ),
                                span: (block_idx, instr_idx),
                            });
                    }
                    continue;
                }

                if let Instruction::Spawn {
                    func: _,
                    args,
                    result,
                } = instr
                {
                    // 参数边：arg 来自另一个 spawn 的结果（深度 = 1，只检测直接来源）
                    for arg in args {
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
            .map(|p| self.operand_to_string(p))
            .collect();

        format!(
            "跨任务循环引用: {} → {} (形成环). 建议: 使用 Weak 打破循环，或在 unsafe 块中绕过检测",
            cycle_strs.join(" → "),
            cycle_strs.first().unwrap_or(&"?".to_string())
        )
    }

    /// Operand 转字符串
    fn operand_to_string(
        &self,
        op: &Operand,
    ) -> String {
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
    fn find_cycle_span(
        &self,
        _graph: &HashMap<Operand, HashSet<Operand>>,
        cycle_start: &Operand,
    ) -> (usize, usize) {
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
        self.unsafe_ranges.clear();
        self.unsafe_bypasses.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::middle::core::ir::BasicBlock;
    use crate::frontend::typecheck::MonoType;

    /// 创建测试用的 FunctionIR
    fn create_test_function(instructions: Vec<Instruction>) -> FunctionIR {
        FunctionIR {
            name: "test".to_string(),
            params: vec![],
            return_type: MonoType::Void,
            is_async: false,
            locals: vec![],
            blocks: vec![BasicBlock {
                label: 0,
                instructions,
                successors: vec![],
            }],
            entry: 0,
        }
    }

    #[test]
    fn test_no_spawn_no_error() {
        let mut checker = CycleChecker::new();
        let func = create_test_function(vec![Instruction::Move {
            dst: Operand::Local(0),
            src: Operand::Arg(0),
        }]);

        let errors = checker.check_function(&func);
        assert!(errors.is_empty(), "无 spawn 不应有错误");
    }

    #[test]
    fn test_single_spawn_no_cycle() {
        let mut checker = CycleChecker::new();
        let func = create_test_function(vec![Instruction::Spawn {
            func: Operand::Global(0),
            args: vec![Operand::Local(0)],
            result: Operand::Temp(0),
        }]);

        let errors = checker.check_function(&func);
        assert!(errors.is_empty(), "单 spawn 不应有循环");
    }

    #[test]
    fn test_independent_spawns_no_cycle() {
        let mut checker = CycleChecker::new();
        let func = create_test_function(vec![
            Instruction::Spawn {
                func: Operand::Global(0),
                args: vec![Operand::Local(0)],
                result: Operand::Temp(0),
            },
            Instruction::Spawn {
                func: Operand::Global(1),
                args: vec![Operand::Local(1)],
                result: Operand::Temp(1),
            },
        ]);

        let errors = checker.check_function(&func);
        assert!(errors.is_empty(), "独立 spawn 不应有循环");
    }

    #[test]
    fn test_spawn_chain_no_cycle() {
        // spawn A -> spawn B (单向依赖，无循环)
        let mut checker = CycleChecker::new();
        let func = create_test_function(vec![
            Instruction::Spawn {
                func: Operand::Global(0),
                args: vec![Operand::Local(0)],
                result: Operand::Temp(0),
            },
            Instruction::Spawn {
                func: Operand::Global(1),
                args: vec![Operand::Temp(0)], // 使用前一个 spawn 的结果
                result: Operand::Temp(1),
            },
        ]);

        let errors = checker.check_function(&func);
        assert!(errors.is_empty(), "单向依赖不应有循环");
    }

    #[test]
    fn test_depth_limit_one_level() {
        // 测试深度限制：只检测直接依赖
        let mut checker = CycleChecker::new();
        let func = create_test_function(vec![
            Instruction::Spawn {
                func: Operand::Global(0),
                args: vec![],
                result: Operand::Temp(0),
            },
            Instruction::Move {
                dst: Operand::Temp(1),
                src: Operand::Temp(0),
            },
            Instruction::Move {
                dst: Operand::Temp(2),
                src: Operand::Temp(1), // 间接引用（深度 > 1）
            },
            Instruction::Spawn {
                func: Operand::Global(1),
                args: vec![Operand::Temp(2)], // 使用间接引用
                result: Operand::Temp(3),
            },
        ]);

        let errors = checker.check_function(&func);
        // 深度限制为 1，间接引用不应被追踪
        assert!(errors.is_empty(), "深度 > 1 的间接引用不应被检测");
    }

    #[test]
    fn test_clear_resets_all_state() {
        use super::super::error::OwnershipCheck;

        let mut checker = CycleChecker::new();
        let func = create_test_function(vec![Instruction::Spawn {
            func: Operand::Global(0),
            args: vec![],
            result: Operand::Temp(0),
        }]);

        checker.check_function(&func);
        OwnershipCheck::clear(&mut checker);

        assert!(checker.errors.is_empty());
        assert!(checker.spawn_results.is_empty());
        assert!(checker.spawn_ref_edges.is_empty());
        assert!(checker.spawn_param_edges.is_empty());
        assert!(checker.unsafe_ranges.is_empty());
        assert!(checker.unsafe_bypasses.is_empty());
    }

    #[test]
    fn test_unsafe_bypass_empty_by_default() {
        // 当前 Phase 6，unsafe 检测尚未实现，应返回空
        let mut checker = CycleChecker::new();
        let func = create_test_function(vec![Instruction::Spawn {
            func: Operand::Global(0),
            args: vec![],
            result: Operand::Temp(0),
        }]);

        checker.check_function(&func);
        assert!(
            checker.unsafe_bypasses().is_empty(),
            "Phase 6 默认无 unsafe 绕过"
        );
    }

    #[test]
    fn test_error_message_contains_suggestion() {
        // 确保错误消息包含建议
        let mut checker = CycleChecker::new();

        // 构造循环：spawn A 使用 spawn B 结果，spawn B 使用 spawn A 结果
        let func = create_test_function(vec![
            Instruction::Spawn {
                func: Operand::Global(0),
                args: vec![],
                result: Operand::Temp(0),
            },
            Instruction::Spawn {
                func: Operand::Global(1),
                args: vec![Operand::Temp(0)],
                result: Operand::Temp(1),
            },
            Instruction::Move {
                dst: Operand::Temp(0),
                src: Operand::Temp(1),
            },
        ]);

        let errors = checker.check_function(&func);
        // 如果检测到循环，消息应包含建议
        for error in errors {
            if let OwnershipError::CrossSpawnCycle { details, .. } = error {
                assert!(
                    details.contains("Weak") || details.contains("unsafe"),
                    "错误消息应包含解决建议"
                );
            }
        }
    }
}
