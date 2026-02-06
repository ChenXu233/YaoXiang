//! 变量生命周期追踪器
//!
//! 追踪变量从创建到消费的完整生命周期：
//! - 创建点（Store、Alloc、HeapAlloc）
//! - 使用点（Load、运算）
//! - 消费点（Move、Call 参数、Ret）
//! - 释放点（Drop）
//!
//! # 设计原理
//!
//! 生命周期追踪器用于检测：
//! - 变量未消费就释放（潜在资源泄漏）
//! - 变量被多次消费（UseAfterMove）
//! - 变量在消费后继续使用（UseAfterMove）
//! - 变量从未使用（死代码）

use crate::middle::core::ir::{FunctionIR, Instruction, Operand};
use std::collections::HashMap;
use std::fmt;

/// 变量生命周期事件
///
/// 记录变量生命周期中的关键事件点
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LifecycleEvent {
    /// 变量创建（Store、Alloc、HeapAlloc）
    Created {
        /// 变量
        operand: Operand,
        /// 位置 (block_idx, instr_idx)
        location: (usize, usize),
    },
    /// 变量被消费（Move、Call 参数、Ret）
    Consumed {
        /// 变量
        operand: Operand,
        /// 消费类型
        consume_type: ConsumeType,
        /// 位置
        location: (usize, usize),
    },
    /// 变量被移动
    Moved {
        /// 变量
        operand: Operand,
        /// 目标变量
        dst: Operand,
        /// 位置
        location: (usize, usize),
    },
    /// 变量被释放
    Dropped {
        /// 变量
        operand: Operand,
        /// 位置
        location: (usize, usize),
    },
    /// 变量被返回
    Returned {
        /// 变量
        operand: Operand,
        /// 位置
        location: (usize, usize),
    },
    /// 变量被读取（用于检查是否在消费后继续使用）
    Read {
        /// 变量
        operand: Operand,
        /// 位置
        location: (usize, usize),
    },
}

/// 消费类型
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConsumeType {
    /// Move 指令消费
    Move,
    /// 函数调用参数消费
    CallArg,
    /// 返回值消费
    Return,
    /// 赋值消费
    Assign,
}

/// 变量的生命周期信息
#[derive(Debug, Clone)]
pub struct LifecycleInfo {
    /// 变量
    pub operand: Operand,
    /// 创建位置
    pub creation_location: Option<(usize, usize)>,
    /// 消费位置（如果有）
    pub consume_location: Option<(usize, usize)>,
    /// 释放位置（如果有）
    pub drop_location: Option<(usize, usize)>,
    /// 返回位置（如果有）
    pub return_location: Option<(usize, usize)>,
    /// 是否被多次消费
    pub multiple_consumes: bool,
    /// 是否在消费后继续使用
    pub used_after_consume: bool,
    /// 是否从未使用
    pub never_used: bool,
}

impl LifecycleInfo {
    /// 创建新的生命周期信息
    pub fn new(operand: Operand) -> Self {
        Self {
            operand,
            creation_location: None,
            consume_location: None,
            drop_location: None,
            return_location: None,
            multiple_consumes: false,
            used_after_consume: false,
            never_used: true,
        }
    }

    /// 检查变量是否已被消费
    pub fn is_consumed(&self) -> bool {
        self.consume_location.is_some()
    }
}

/// 生命周期问题
#[derive(Debug, Clone)]
pub struct LifecycleIssue {
    /// 问题类型
    pub kind: LifecycleIssueKind,
    /// 变量
    pub operand: Operand,
    /// 位置
    pub location: (usize, usize),
    /// 描述
    pub description: String,
}

/// 生命周期问题类型
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LifecycleIssueKind {
    /// 未消费就释放（潜在泄漏）
    DropWithoutConsume,
    /// 多次消费
    MultipleConsume,
    /// 消费后继续使用
    UseAfterConsume,
    /// 从未使用
    NeverUsed,
    /// 未定义状态
    UndefinedState,
}

impl fmt::Display for LifecycleIssueKind {
    fn fmt(
        &self,
        f: &mut fmt::Formatter<'_>,
    ) -> fmt::Result {
        match self {
            LifecycleIssueKind::DropWithoutConsume => write!(f, "drop without consume"),
            LifecycleIssueKind::MultipleConsume => write!(f, "multiple consume"),
            LifecycleIssueKind::UseAfterConsume => write!(f, "use after consume"),
            LifecycleIssueKind::NeverUsed => write!(f, "never used"),
            LifecycleIssueKind::UndefinedState => write!(f, "undefined state"),
        }
    }
}

/// 生命周期追踪器
///
/// 分析函数的变量生命周期，检测潜在问题
#[derive(Debug, Clone)]
pub struct LifecycleTracker {
    /// 事件列表
    events: Vec<LifecycleEvent>,
    /// 变量 -> 创建事件
    creation_points: HashMap<Operand, (usize, usize)>,
    /// 变量 -> 消费次数统计
    consume_count: HashMap<Operand, usize>,
    /// 变量 -> 读取次数统计
    read_count: HashMap<Operand, usize>,
    /// 变量 -> 首次消费位置
    first_consume: HashMap<Operand, (usize, usize)>,
    /// 变量 -> 首次读取位置
    first_read: HashMap<Operand, (usize, usize)>,
    /// 函数名
    function_name: String,
}

impl LifecycleTracker {
    /// 创建新的生命周期追踪器
    pub fn new(function_name: String) -> Self {
        Self {
            events: Vec::new(),
            creation_points: HashMap::new(),
            consume_count: HashMap::new(),
            read_count: HashMap::new(),
            first_consume: HashMap::new(),
            first_read: HashMap::new(),
            function_name,
        }
    }

    /// 分析函数的变量生命周期
    pub fn analyze_function(
        &mut self,
        func: &FunctionIR,
    ) {
        // 重置状态
        self.events.clear();
        self.creation_points.clear();
        self.consume_count.clear();
        self.read_count.clear();
        self.first_consume.clear();
        self.first_read.clear();

        self.function_name = func.name.clone();

        // 初始化函数参数的创建信息
        for (idx, _) in func.params.iter().enumerate() {
            self.creation_points.insert(Operand::Arg(idx), (0, 0));
        }

        // 遍历所有指令，收集生命周期事件
        for (block_idx, block) in func.blocks.iter().enumerate() {
            for (instr_idx, instr) in block.instructions.iter().enumerate() {
                self.process_instruction(instr, block_idx, instr_idx);
            }
        }
    }

    /// 处理单条指令，收集生命周期事件
    fn process_instruction(
        &mut self,
        instr: &Instruction,
        block_idx: usize,
        instr_idx: usize,
    ) {
        let location = (block_idx, instr_idx);

        match instr {
            // Move: src 被消费，dst 被创建
            Instruction::Move { dst, src } => {
                self.record_consume(src, ConsumeType::Move, location);
                self.record_creation(dst, location);
                self.record_moved(src, dst.clone(), location);
            }

            // Store: dst 被创建，src 被消费
            Instruction::Store { dst, src } => {
                self.record_creation(dst, location);
                self.record_consume(src, ConsumeType::Assign, location);
            }

            // StoreField: dst 被创建，src 被消费
            Instruction::StoreField { dst, src, .. } => {
                self.record_creation(dst, location);
                self.record_consume(src, ConsumeType::Assign, location);
            }

            // StoreIndex: dst 被创建，src 被消费
            Instruction::StoreIndex { dst, src, .. } => {
                self.record_creation(dst, location);
                self.record_consume(src, ConsumeType::Assign, location);
            }

            // Call: 参数被消费，返回值 dst 被创建
            Instruction::Call { dst, args, .. } => {
                for arg in args {
                    self.record_consume(arg, ConsumeType::CallArg, location);
                }
                if let Some(d) = dst {
                    self.record_creation(d, location);
                }
            }

            // CallVirt: 参数被消费，obj 被消费
            Instruction::CallVirt { dst, obj, args, .. } => {
                self.record_consume(obj, ConsumeType::CallArg, location);
                for arg in args {
                    self.record_consume(arg, ConsumeType::CallArg, location);
                }
                if let Some(d) = dst {
                    self.record_creation(d, location);
                }
            }

            // Ret: 返回值被消费
            Instruction::Ret(Some(value)) => {
                self.record_consume(value, ConsumeType::Return, location);
                self.record_return(value, location);
            }

            // Drop: 变量被释放
            Instruction::Drop(operand) => {
                self.record_drop(operand, location);
            }

            // Load: src 被读取
            Instruction::Load { src, .. } => {
                self.record_read(src, location);
            }

            // LoadField: src 被读取
            Instruction::LoadField { src, .. } => {
                self.record_read(src, location);
            }

            // LoadIndex: src 被读取
            Instruction::LoadIndex { src, .. } => {
                self.record_read(src, location);
            }

            // 算术运算：操作数被读取
            Instruction::Add { lhs, rhs, .. }
            | Instruction::Sub { lhs, rhs, .. }
            | Instruction::Mul { lhs, rhs, .. }
            | Instruction::Div { lhs, rhs, .. }
            | Instruction::Mod { lhs, rhs, .. } => {
                self.record_read(lhs, location);
                self.record_read(rhs, location);
            }

            // 比较运算：操作数被读取
            Instruction::Eq { lhs, rhs, .. }
            | Instruction::Ne { lhs, rhs, .. }
            | Instruction::Lt { lhs, rhs, .. }
            | Instruction::Le { lhs, rhs, .. }
            | Instruction::Gt { lhs, rhs, .. }
            | Instruction::Ge { lhs, rhs, .. } => {
                self.record_read(lhs, location);
                self.record_read(rhs, location);
            }

            // Alloc: 新变量被创建
            Instruction::Alloc { dst, .. } => {
                self.record_creation(dst, location);
            }

            // HeapAlloc: 新变量被创建
            Instruction::HeapAlloc { dst, .. } => {
                self.record_creation(dst, location);
            }

            // Cast: src 被读取，dst 被创建
            Instruction::Cast { dst, src, .. } => {
                self.record_read(src, location);
                self.record_creation(dst, location);
            }

            // Neg: src 被读取，dst 被创建
            Instruction::Neg { dst, src, .. } => {
                self.record_read(src, location);
                self.record_creation(dst, location);
            }

            // 虚调用等（类似 Call）
            Instruction::CallDyn { dst, args, .. } => {
                for arg in args {
                    self.record_consume(arg, ConsumeType::CallArg, location);
                }
                if let Some(d) = dst {
                    self.record_creation(d, location);
                }
            }

            // TailCall（无返回值）
            Instruction::TailCall { args, .. } => {
                for arg in args {
                    self.record_consume(arg, ConsumeType::CallArg, location);
                }
            }

            // Spawn: 参数被消费
            Instruction::Spawn { args, .. } => {
                for arg in args {
                    self.record_consume(arg, ConsumeType::CallArg, location);
                }
            }

            // MakeClosure: env 变量被消费，dst 被创建
            Instruction::MakeClosure { dst, env, .. } => {
                for var in env {
                    self.record_consume(var, ConsumeType::CallArg, location);
                }
                self.record_creation(dst, location);
            }

            // StringConcat: lhs/rhs 被读取，dst 被创建
            Instruction::StringConcat { dst, lhs, rhs } => {
                self.record_read(lhs, location);
                self.record_read(rhs, location);
                self.record_creation(dst, location);
            }

            // StringLength/StringFromInt 等：src 被读取，dst 被创建
            Instruction::StringLength { dst, src, .. }
            | Instruction::StringFromInt { dst, src, .. }
            | Instruction::StringFromFloat { dst, src, .. }
            | Instruction::StringGetChar { dst, src, .. } => {
                self.record_read(src, location);
                self.record_creation(dst, location);
            }

            // ArcClone: src 被读取，dst 被创建
            Instruction::ArcClone { dst, src } => {
                self.record_read(src, location);
                self.record_creation(dst, location);
            }

            // ArcNew: src 被读取，dst 被创建
            Instruction::ArcNew { dst, src } => {
                self.record_read(src, location);
                self.record_creation(dst, location);
            }

            // Dup/Swap/Yield: 无生命周期影响
            Instruction::Dup | Instruction::Swap | Instruction::Yield => {}

            // Jump: 无生命周期影响
            Instruction::Jmp(_) => {}

            // Jump conditional: 条件被读取
            Instruction::JmpIf(cond, _) | Instruction::JmpIfNot(cond, _) => {
                self.record_read(cond, location);
            }

            // Push/Pop: 操作数被读取
            Instruction::Push(v) | Instruction::Pop(v) => {
                self.record_read(v, location);
            }

            // Free: 变量被释放
            Instruction::Free(v) => {
                self.record_drop(v, location);
            }

            // TypeTest: 变量被读取
            Instruction::TypeTest(v, _) => {
                self.record_read(v, location);
            }

            // Upvalue 操作
            Instruction::LoadUpvalue { dst, .. } => {
                self.record_read(dst, location);
            }
            Instruction::StoreUpvalue { src, .. } => {
                self.record_consume(src, ConsumeType::Assign, location);
            }

            // CloseUpvalue
            Instruction::CloseUpvalue(v) => {
                self.record_drop(v, location);
            }

            // 其他未列出的指令：无生命周期影响
            _ => {}
        }
    }

    /// 记录变量创建
    fn record_creation(
        &mut self,
        operand: &Operand,
        location: (usize, usize),
    ) {
        self.events.push(LifecycleEvent::Created {
            operand: operand.clone(),
            location,
        });

        // 只追踪局部变量、临时变量、参数
        if matches!(
            operand,
            Operand::Local(_) | Operand::Temp(_) | Operand::Arg(_)
        ) && !self.creation_points.contains_key(operand)
        {
            self.creation_points.insert(operand.clone(), location);
        }
    }

    /// 记录变量消费
    fn record_consume(
        &mut self,
        operand: &Operand,
        consume_type: ConsumeType,
        location: (usize, usize),
    ) {
        self.events.push(LifecycleEvent::Consumed {
            operand: operand.clone(),
            consume_type,
            location,
        });

        let count = self.consume_count.entry(operand.clone()).or_insert(0);
        *count += 1;

        if *count == 1 {
            self.first_consume.insert(operand.clone(), location);
        }
    }

    /// 记录变量移动
    fn record_moved(
        &mut self,
        src: &Operand,
        dst: Operand,
        location: (usize, usize),
    ) {
        self.events.push(LifecycleEvent::Moved {
            operand: src.clone(),
            dst,
            location,
        });
    }

    /// 记录变量释放
    fn record_drop(
        &mut self,
        operand: &Operand,
        location: (usize, usize),
    ) {
        self.events.push(LifecycleEvent::Dropped {
            operand: operand.clone(),
            location,
        });
    }

    /// 记录变量返回
    fn record_return(
        &mut self,
        operand: &Operand,
        location: (usize, usize),
    ) {
        self.events.push(LifecycleEvent::Returned {
            operand: operand.clone(),
            location,
        });
    }

    /// 记录变量读取
    fn record_read(
        &mut self,
        operand: &Operand,
        location: (usize, usize),
    ) {
        self.events.push(LifecycleEvent::Read {
            operand: operand.clone(),
            location,
        });

        let count = self.read_count.entry(operand.clone()).or_insert(0);
        *count += 1;

        if *count == 1 {
            self.first_read.insert(operand.clone(), location);
        }
    }

    /// 获取变量的生命周期信息
    pub fn get_lifecycle(
        &self,
        operand: &Operand,
    ) -> Option<LifecycleInfo> {
        if !self.creation_points.contains_key(operand) {
            return None;
        }

        let creation_location = self.creation_points.get(operand).cloned();
        let consume_count = self.consume_count.get(operand).cloned().unwrap_or(0);
        let first_consume = self.first_consume.get(operand).cloned();
        let first_read = self.first_read.get(operand).cloned();
        let read_count = self.read_count.get(operand).cloned().unwrap_or(0);

        // 检查是否有返回
        let return_location = self
            .events
            .iter()
            .find(|e| matches!(e, LifecycleEvent::Returned { operand: op, .. } if op == operand))
            .and_then(|e| {
                if let LifecycleEvent::Returned { location, .. } = e {
                    Some(*location)
                } else {
                    None
                }
            });

        // 检查是否有释放
        let drop_location = self
            .events
            .iter()
            .find(|e| matches!(e, LifecycleEvent::Dropped { operand: op, .. } if op == operand))
            .and_then(|e| {
                if let LifecycleEvent::Dropped { location, .. } = e {
                    Some(*location)
                } else {
                    None
                }
            });

        // 判断是否多次消费
        let multiple_consumes = consume_count > 1;

        // 判断是否消费后继续使用
        let used_after_consume = if let Some(consume_loc) = first_consume {
            if let Some(read_loc) = first_read {
                read_loc > consume_loc
            } else {
                false
            }
        } else {
            false
        };

        // 判断是否从未使用
        let never_used = read_count == 0 && consume_count == 0;

        Some(LifecycleInfo {
            operand: operand.clone(),
            creation_location,
            consume_location: first_consume,
            drop_location,
            return_location,
            multiple_consumes,
            used_after_consume,
            never_used,
        })
    }

    /// 获取所有变量的生命周期信息
    pub fn get_all_lifecycles(&self) -> Vec<LifecycleInfo> {
        self.creation_points
            .keys()
            .filter_map(|op| self.get_lifecycle(op))
            .collect()
    }

    /// 检测生命周期问题
    pub fn detect_issues(&self) -> Vec<LifecycleIssue> {
        let mut issues = Vec::new();

        for lifecycle in self.get_all_lifecycles() {
            // 检查未消费就释放
            if lifecycle.drop_location.is_some() && !lifecycle.is_consumed() {
                issues.push(LifecycleIssue {
                    kind: LifecycleIssueKind::DropWithoutConsume,
                    operand: lifecycle.operand.clone(),
                    location: lifecycle.drop_location.unwrap(),
                    description: format!(
                        "variable '{}' dropped without being consumed",
                        operand_to_string(&lifecycle.operand)
                    ),
                });
            }

            // 检查多次消费
            if lifecycle.multiple_consumes {
                issues.push(LifecycleIssue {
                    kind: LifecycleIssueKind::MultipleConsume,
                    operand: lifecycle.operand.clone(),
                    location: lifecycle.consume_location.unwrap(),
                    description: format!(
                        "variable '{}' consumed multiple times",
                        operand_to_string(&lifecycle.operand)
                    ),
                });
            }

            // 检查消费后继续使用
            if lifecycle.used_after_consume {
                issues.push(LifecycleIssue {
                    kind: LifecycleIssueKind::UseAfterConsume,
                    operand: lifecycle.operand.clone(),
                    location: lifecycle.creation_location.unwrap_or((0, 0)),
                    description: format!(
                        "variable '{}' used after being consumed",
                        operand_to_string(&lifecycle.operand)
                    ),
                });
            }

            // 检查从未使用
            if lifecycle.never_used && lifecycle.drop_location.is_none() {
                issues.push(LifecycleIssue {
                    kind: LifecycleIssueKind::NeverUsed,
                    operand: lifecycle.operand.clone(),
                    location: lifecycle.creation_location.unwrap_or((0, 0)),
                    description: format!(
                        "variable '{}' never used",
                        operand_to_string(&lifecycle.operand)
                    ),
                });
            }
        }

        issues
    }

    /// 获取事件列表（用于调试）
    pub fn events(&self) -> &[LifecycleEvent] {
        &self.events
    }
}

impl Default for LifecycleTracker {
    fn default() -> Self {
        Self::new(String::new())
    }
}

/// 将 Operand 转换为字符串标识
fn operand_to_string(operand: &Operand) -> String {
    match operand {
        Operand::Local(idx) => format!("local_{}", idx),
        Operand::Arg(idx) => format!("arg_{}", idx),
        Operand::Temp(idx) => format!("temp_{}", idx),
        Operand::Global(idx) => format!("global_{}", idx),
        Operand::Const(c) => format!("const_{:?}", c),
        Operand::Label(idx) => format!("label_{}", idx),
        Operand::Register(idx) => format!("reg_{}", idx),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::middle::core::ir::{BasicBlock, ConstValue, FunctionIR};
    use crate::frontend::typecheck::MonoType;

    fn make_simple_function() -> FunctionIR {
        FunctionIR {
            name: "test".to_string(),
            params: vec![MonoType::Int(0)],
            return_type: MonoType::Int(0),
            is_async: false,
            locals: vec![],
            blocks: vec![BasicBlock {
                label: 0,
                instructions: vec![Instruction::Ret(Some(Operand::Arg(0)))],
                successors: vec![],
            }],
            entry: 0,
        }
    }

    #[test]
    fn test_track_created() {
        let mut tracker = LifecycleTracker::new("test".to_string());
        let func = make_simple_function();

        tracker.analyze_function(&func);

        // 参数应该被追踪
        assert!(tracker.get_lifecycle(&Operand::Arg(0)).is_some());
    }

    #[test]
    fn test_detect_never_used() {
        let mut tracker = LifecycleTracker::new("test".to_string());

        let func = FunctionIR {
            name: "test".to_string(),
            params: vec![MonoType::Int(0)],
            return_type: MonoType::Void,
            is_async: false,
            locals: vec![],
            blocks: vec![BasicBlock {
                label: 0,
                instructions: vec![
                    Instruction::Alloc {
                        dst: Operand::Local(0),
                        size: Operand::Const(ConstValue::Int(4)),
                    },
                    Instruction::Drop(Operand::Local(0)),
                ],
                successors: vec![],
            }],
            entry: 0,
        };

        tracker.analyze_function(&func);

        let issues = tracker.detect_issues();
        let never_used = issues
            .iter()
            .any(|i| i.kind == LifecycleIssueKind::NeverUsed);
        assert!(never_used);
    }

    #[test]
    fn test_multiple_consume() {
        let mut tracker = LifecycleTracker::new("test".to_string());

        let func = FunctionIR {
            name: "test".to_string(),
            params: vec![MonoType::Int(0)],
            return_type: MonoType::Int(0),
            is_async: false,
            locals: vec![],
            blocks: vec![BasicBlock {
                label: 0,
                instructions: vec![
                    Instruction::Move {
                        dst: Operand::Temp(0),
                        src: Operand::Arg(0),
                    },
                    Instruction::Move {
                        dst: Operand::Temp(1),
                        src: Operand::Arg(0),
                    },
                    Instruction::Ret(Some(Operand::Temp(0))),
                ],
                successors: vec![],
            }],
            entry: 0,
        };

        tracker.analyze_function(&func);

        let issues = tracker.detect_issues();
        let multiple_consume = issues
            .iter()
            .any(|i| i.kind == LifecycleIssueKind::MultipleConsume);
        assert!(multiple_consume);
    }

    #[test]
    fn test_track_store_instruction() {
        // 测试 Store 指令的创建和消费追踪
        let mut tracker = LifecycleTracker::new("test".to_string());

        let func = FunctionIR {
            name: "test".to_string(),
            params: vec![MonoType::Int(0)],
            return_type: MonoType::Void,
            is_async: false,
            locals: vec![],
            blocks: vec![BasicBlock {
                label: 0,
                instructions: vec![
                    Instruction::Alloc {
                        dst: Operand::Local(0),
                        size: Operand::Const(ConstValue::Int(4)),
                    },
                    Instruction::Store {
                        dst: Operand::Local(0),
                        src: Operand::Arg(0),
                    },
                ],
                successors: vec![],
            }],
            entry: 0,
        };

        tracker.analyze_function(&func);

        // Local 0 应该被创建
        let local_lifecycle = tracker.get_lifecycle(&Operand::Local(0));
        assert!(local_lifecycle.is_some());
        assert!(local_lifecycle.unwrap().creation_location.is_some());
    }

    #[test]
    fn test_track_call_instruction() {
        // 测试 Call 指令的参数消费和返回值创建
        let mut tracker = LifecycleTracker::new("test".to_string());

        let func = FunctionIR {
            name: "test".to_string(),
            params: vec![MonoType::Int(0)],
            return_type: MonoType::Int(0),
            is_async: false,
            locals: vec![],
            blocks: vec![BasicBlock {
                label: 0,
                instructions: vec![
                    Instruction::Call {
                        dst: Some(Operand::Temp(0)),
                        func: Operand::Global(0), // 使用索引表示函数
                        args: vec![Operand::Arg(0)],
                    },
                    Instruction::Ret(Some(Operand::Temp(0))),
                ],
                successors: vec![],
            }],
            entry: 0,
        };

        tracker.analyze_function(&func);

        // 参数应该被消费
        let arg_lifecycle = tracker.get_lifecycle(&Operand::Arg(0));
        assert!(arg_lifecycle.is_some());
        assert!(arg_lifecycle.unwrap().is_consumed());

        // 返回值应该被创建
        let ret_lifecycle = tracker.get_lifecycle(&Operand::Temp(0));
        assert!(ret_lifecycle.is_some());
        assert!(ret_lifecycle.unwrap().creation_location.is_some());
    }

    #[test]
    fn test_track_drop_without_consume() {
        // 测试检测未消费就释放
        let mut tracker = LifecycleTracker::new("test".to_string());

        let func = FunctionIR {
            name: "test".to_string(),
            params: vec![],
            return_type: MonoType::Void,
            is_async: false,
            locals: vec![],
            blocks: vec![BasicBlock {
                label: 0,
                instructions: vec![
                    Instruction::Alloc {
                        dst: Operand::Local(0),
                        size: Operand::Const(ConstValue::Int(4)),
                    },
                    // 不使用直接 Drop
                    Instruction::Drop(Operand::Local(0)),
                ],
                successors: vec![],
            }],
            entry: 0,
        };

        tracker.analyze_function(&func);

        let issues = tracker.detect_issues();
        let drop_without_consume = issues
            .iter()
            .any(|i| i.kind == LifecycleIssueKind::DropWithoutConsume);
        assert!(drop_without_consume);
    }

    #[test]
    fn test_get_all_lifecycles() {
        // 测试获取所有生命周期信息
        let mut tracker = LifecycleTracker::new("test".to_string());

        let func = FunctionIR {
            name: "test".to_string(),
            params: vec![MonoType::Int(0)],
            return_type: MonoType::Void,
            is_async: false,
            locals: vec![],
            blocks: vec![BasicBlock {
                label: 0,
                instructions: vec![Instruction::Ret(Some(Operand::Arg(0)))],
                successors: vec![],
            }],
            entry: 0,
        };

        tracker.analyze_function(&func);

        let lifecycles = tracker.get_all_lifecycles();
        // 应该包含参数
        assert!(lifecycles.len() >= 1);
    }

    #[test]
    fn test_events_collection() {
        // 测试事件收集
        let mut tracker = LifecycleTracker::new("test".to_string());

        let func = FunctionIR {
            name: "test".to_string(),
            params: vec![MonoType::Int(0)],
            return_type: MonoType::Int(0),
            is_async: false,
            locals: vec![],
            blocks: vec![BasicBlock {
                label: 0,
                instructions: vec![Instruction::Ret(Some(Operand::Arg(0)))],
                successors: vec![],
            }],
            entry: 0,
        };

        tracker.analyze_function(&func);

        let events = tracker.events();
        // 应该有事件记录
        assert!(!events.is_empty());
    }

    #[test]
    fn test_lifecycle_info_is_consumed() {
        // 测试 is_consumed 方法
        let mut tracker = LifecycleTracker::new("test".to_string());

        let func = FunctionIR {
            name: "test".to_string(),
            params: vec![MonoType::Int(0)],
            return_type: MonoType::Int(0),
            is_async: false,
            locals: vec![],
            blocks: vec![BasicBlock {
                label: 0,
                instructions: vec![
                    Instruction::Move {
                        dst: Operand::Temp(0),
                        src: Operand::Arg(0),
                    },
                    Instruction::Ret(Some(Operand::Temp(0))),
                ],
                successors: vec![],
            }],
            entry: 0,
        };

        tracker.analyze_function(&func);

        let arg_lifecycle = tracker.get_lifecycle(&Operand::Arg(0)).unwrap();
        assert!(arg_lifecycle.is_consumed());
    }

    #[test]
    fn test_unknown_operand_returns_none() {
        // 测试未知操作数返回 None
        let tracker = LifecycleTracker::new("test".to_string());

        let lifecycle = tracker.get_lifecycle(&Operand::Temp(999));
        assert!(lifecycle.is_none());
    }
}
