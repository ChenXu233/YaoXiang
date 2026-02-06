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
use std::collections::{HashMap, HashSet};
use super::error::OwnershipError;

/// 任务内循环追踪器
#[derive(Debug, Default)]
pub struct IntraTaskCycleTracker {
    /// 任务内 ref 边
    ref_edges: Vec<RefEdge>,
    /// 检测到的循环（警告）
    warnings: Vec<OwnershipError>,
    /// ArcNew 指令位置追踪
    arc_new_locations: HashMap<Operand, (usize, usize)>,
    /// 值定义追踪（预留，用于更复杂的数据流分析）
    #[allow(dead_code)]
    value_defs: HashMap<Operand, Operand>,
}

/// ref 边
#[derive(Debug, Clone)]
struct RefEdge {
    /// 源操作数（创建 ref 的值）
    from: Operand,
    /// 目标操作数（被 ref 的值）
    to: Operand,
    /// 位置
    span: (usize, usize),
}

impl IntraTaskCycleTracker {
    /// 创建新的追踪器
    pub fn new() -> Self {
        Self::default()
    }

    /// 追踪函数中的任务内循环
    pub fn track_function(
        &mut self,
        func: &FunctionIR,
    ) -> &[OwnershipError] {
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
                            span: (block_idx, instr_idx),
                        });
                    }
                    // Move：追踪值流动
                    Instruction::Move { dst, src } => {
                        self.value_defs.insert(dst.clone(), src.clone());
                    }
                    // Store：追踪赋值
                    Instruction::Store { dst, src } => {
                        self.value_defs.insert(dst.clone(), src.clone());
                    }
                    // StoreField：追踪字段赋值（可能形成循环）
                    Instruction::StoreField { dst, src, .. } => {
                        // dst.field = src，如果 src 是 ref，可能形成循环
                        if self.arc_new_locations.contains_key(src) {
                            self.ref_edges.push(RefEdge {
                                from: dst.clone(),
                                to: src.clone(),
                                span: (block_idx, instr_idx),
                            });
                        }
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
                    let span = self.find_cycle_span(neighbor);
                    self.warnings.push(OwnershipError::IntraTaskCycle {
                        details: cycle_path,
                        span,
                    });
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
            .map(|p| self.operand_to_string(p))
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

    /// 获取警告列表
    pub fn warnings(&self) -> &[OwnershipError] {
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
    fn test_no_cycle() {
        let mut tracker = IntraTaskCycleTracker::new();
        let func = create_test_function(vec![
            Instruction::ArcNew {
                dst: Operand::Temp(0),
                src: Operand::Local(0),
            },
            Instruction::ArcNew {
                dst: Operand::Temp(1),
                src: Operand::Local(1),
            },
        ]);

        let warnings = tracker.track_function(&func);
        assert!(warnings.is_empty(), "不应有循环警告");
    }

    #[test]
    fn test_simple_cycle_warning() {
        let mut tracker = IntraTaskCycleTracker::new();

        // 模拟 a = ref b; b.field = a 形成循环
        let func = create_test_function(vec![
            Instruction::ArcNew {
                dst: Operand::Temp(0), // a = ref b
                src: Operand::Local(0),
            },
            Instruction::StoreField {
                dst: Operand::Local(0), // b.field = a
                src: Operand::Temp(0),
                field: 0,
                type_name: None,
                field_name: None,
            },
        ]);

        let warnings = tracker.track_function(&func);
        // 检测到任务内循环（警告，不报错）
        assert!(!warnings.is_empty(), "应检测到任务内循环");
        assert!(matches!(warnings[0], OwnershipError::IntraTaskCycle { .. }));
    }

    #[test]
    fn test_chain_no_cycle() {
        // a = ref b; c = ref d; 无循环
        let mut tracker = IntraTaskCycleTracker::new();
        let func = create_test_function(vec![
            Instruction::ArcNew {
                dst: Operand::Temp(0),
                src: Operand::Local(0),
            },
            Instruction::ArcNew {
                dst: Operand::Temp(1),
                src: Operand::Local(1),
            },
            Instruction::ArcNew {
                dst: Operand::Temp(2),
                src: Operand::Temp(0),
            },
        ]);

        let warnings = tracker.track_function(&func);
        assert!(warnings.is_empty(), "链式 ref 不应形成循环");
    }

    #[test]
    fn test_self_reference_cycle() {
        // a = ref a（自引用）
        let mut tracker = IntraTaskCycleTracker::new();
        let func = create_test_function(vec![Instruction::ArcNew {
            dst: Operand::Temp(0),
            src: Operand::Temp(0), // 自引用
        }]);

        let warnings = tracker.track_function(&func);
        // 自引用形成循环
        assert!(!warnings.is_empty(), "自引用应检测为循环");
    }

    #[test]
    fn test_multiple_cycles() {
        // 多个独立循环
        let mut tracker = IntraTaskCycleTracker::new();
        let func = create_test_function(vec![
            // 循环 1: temp_0 -> local_0 -> temp_0
            Instruction::ArcNew {
                dst: Operand::Temp(0),
                src: Operand::Local(0),
            },
            Instruction::StoreField {
                dst: Operand::Local(0),
                src: Operand::Temp(0),
                field: 0,
                type_name: None,
                field_name: None,
            },
            // 循环 2: temp_1 -> local_1 -> temp_1
            Instruction::ArcNew {
                dst: Operand::Temp(1),
                src: Operand::Local(1),
            },
            Instruction::StoreField {
                dst: Operand::Local(1),
                src: Operand::Temp(1),
                field: 0,
                type_name: None,
                field_name: None,
            },
        ]);

        let warnings = tracker.track_function(&func);
        // 应检测到至少一个循环
        assert!(!warnings.is_empty(), "应检测到循环");
    }

    #[test]
    fn test_clear_resets_state() {
        let mut tracker = IntraTaskCycleTracker::new();
        let func = create_test_function(vec![Instruction::ArcNew {
            dst: Operand::Temp(0),
            src: Operand::Local(0),
        }]);

        tracker.track_function(&func);
        tracker.clear();

        assert!(tracker.warnings.is_empty());
        assert!(tracker.ref_edges.is_empty());
        assert!(tracker.arc_new_locations.is_empty());
    }

    #[test]
    fn test_warning_contains_location() {
        let mut tracker = IntraTaskCycleTracker::new();
        let func = create_test_function(vec![Instruction::ArcNew {
            dst: Operand::Temp(0),
            src: Operand::Temp(0), // 自引用
        }]);

        let warnings = tracker.track_function(&func);
        if let Some(OwnershipError::IntraTaskCycle { span, .. }) = warnings.first() {
            assert_eq!(*span, (0, 0), "位置应为 (0, 0)");
        }
    }
}
