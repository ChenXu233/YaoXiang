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

use crate::middle::core::ir::{FunctionIR, Instruction, Operand};
use std::collections::{HashMap, HashSet};
use std::fmt;

// 子模块
pub mod chain_calls;
pub mod clone;
pub mod consume_analysis;
pub mod cycle_check;
pub mod drop_semantics;
pub mod error;
pub mod intra_task_cycle;
pub mod lifecycle;
pub mod move_semantics;
pub mod mut_check;
pub mod ownership_flow;
pub mod ref_semantics;
pub mod send_sync;
pub mod unsafe_check;

pub use chain_calls::*;
pub use clone::*;
pub use consume_analysis::*;
pub use cycle_check::*;
pub use drop_semantics::*;
pub use error::*;
pub use intra_task_cycle::*;
pub use lifecycle::*;
pub use move_semantics::*;
pub use mut_check::*;
pub use ownership_flow::*;
pub use ref_semantics::*;
pub use send_sync::*;
pub use unsafe_check::*;

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
/// 同时运行 Move 检查、Drop 检查、Mut 检查、Ref 检查、Clone 检查、Send/Sync 检查、
/// 跨 spawn 循环检查和任务内循环追踪，返回所有错误。
#[derive(Debug)]
pub struct OwnershipChecker {
    move_checker: MoveChecker,
    drop_checker: DropChecker,
    mut_checker: MutChecker,
    ref_checker: RefChecker,
    clone_checker: CloneChecker,
    send_sync_checker: SendSyncChecker,
    cycle_checker: CycleChecker,
    intra_task_tracker: IntraTaskCycleTracker,
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
            send_sync_checker: SendSyncChecker::new(),
            cycle_checker: CycleChecker::new(),
            intra_task_tracker: IntraTaskCycleTracker::new(),
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
        let send_sync_errors = self.send_sync_checker.check_function(func);
        let cycle_errors = self.cycle_checker.check_function(func);

        // 任务内循环追踪（警告模式，不计入错误）
        let _intra_task_warnings = self.intra_task_tracker.track_function(func);

        // 合并错误（不包含任务内循环警告）
        move_errors
            .iter()
            .chain(drop_errors)
            .chain(mut_errors)
            .chain(ref_errors)
            .chain(clone_errors)
            .chain(send_sync_errors)
            .chain(cycle_errors)
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

    /// 获取 Send/Sync 检查器的错误
    pub fn send_sync_errors(&self) -> &[OwnershipError] {
        self.send_sync_checker.errors()
    }

    /// 获取跨 spawn 循环检查的错误
    pub fn cycle_errors(&self) -> &[OwnershipError] {
        self.cycle_checker.errors()
    }

    /// 获取任务内循环警告（不阻断编译）
    pub fn intra_task_warnings(&self) -> &[OwnershipError] {
        self.intra_task_tracker.warnings()
    }

    /// 获取 unsafe 绕过记录
    pub fn unsafe_bypasses(&self) -> &[OwnershipError] {
        self.cycle_checker.unsafe_bypasses()
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

/// 从指令中提取所有操作数（包括dst、src、索引等）
fn extract_operands(instr: &Instruction) -> Vec<Operand> {
    match instr {
        // 二元运算：dst = lhs op rhs
        Instruction::Add { dst, lhs, rhs }
        | Instruction::Sub { dst, lhs, rhs }
        | Instruction::Mul { dst, lhs, rhs }
        | Instruction::Div { dst, lhs, rhs }
        | Instruction::Mod { dst, lhs, rhs }
        | Instruction::And { dst, lhs, rhs }
        | Instruction::Or { dst, lhs, rhs }
        | Instruction::Xor { dst, lhs, rhs }
        | Instruction::Shl { dst, lhs, rhs }
        | Instruction::Shr { dst, lhs, rhs }
        | Instruction::Sar { dst, lhs, rhs }
        | Instruction::Eq { dst, lhs, rhs }
        | Instruction::Ne { dst, lhs, rhs }
        | Instruction::Lt { dst, lhs, rhs }
        | Instruction::Le { dst, lhs, rhs }
        | Instruction::Gt { dst, lhs, rhs }
        | Instruction::Ge { dst, lhs, rhs }
        | Instruction::StringConcat { dst, lhs, rhs } => {
            vec![dst.clone(), lhs.clone(), rhs.clone()]
        }

        // 一元运算：dst = op src
        Instruction::Move { dst, src }
        | Instruction::Load { dst, src }
        | Instruction::Neg { dst, src }
        | Instruction::Cast { dst, src, .. }
        | Instruction::StringLength { dst, src }
        | Instruction::StringFromInt { dst, src }
        | Instruction::StringFromFloat { dst, src }
        | Instruction::ArcClone { dst, src }
        | Instruction::ShareRef { dst, src } => vec![dst.clone(), src.clone()],

        // 堆分配：dst = HeapAlloc(type_id)
        Instruction::HeapAlloc { dst, .. } => vec![dst.clone()],

        // 存储指令：dst = src
        Instruction::Store { dst, src, .. } => vec![dst.clone(), src.clone()],
        Instruction::StoreField { dst, src, .. } => vec![dst.clone(), src.clone()],
        Instruction::Drop(src) | Instruction::ArcDrop(src) | Instruction::CloseUpvalue(src) => {
            vec![src.clone()]
        }

        // 索引访问：dst[index] = src 或 dst = src[index]
        Instruction::LoadIndex {
            dst, src, index, ..
        }
        | Instruction::StoreIndex {
            dst, index, src, ..
        } => {
            vec![dst.clone(), src.clone(), index.clone()]
        }

        // 数组分配：dst = new Array(size)
        Instruction::AllocArray { dst, size, .. } => vec![dst.clone(), size.clone()],

        // 函数调用：dst = func(args...)
        Instruction::Call { dst, args, .. } => {
            let mut ops = Vec::new();
            if let Some(d) = dst {
                ops.push(d.clone());
            }
            ops.extend(args.iter().cloned());
            ops
        }

        // 虚函数调用：dst = obj.method(args...)
        Instruction::CallVirt { dst, obj, args, .. } => {
            let mut ops = vec![obj.clone()];
            ops.extend(args.iter().cloned());
            if let Some(d) = dst {
                ops.push(d.clone());
            }
            ops
        }

        // 动态调用：dst = func_ptr(args...)
        Instruction::CallDyn { dst, func, args } => {
            let mut ops = vec![func.clone()];
            ops.extend(args.iter().cloned());
            if let Some(d) = dst {
                ops.push(d.clone());
            }
            ops
        }

        // 尾调用：func(args...)
        Instruction::TailCall { func, args } => {
            let mut ops = vec![func.clone()];
            ops.extend(args.iter().cloned());
            ops
        }

        //：result = spawn Spawn func(args...)
        Instruction::Spawn { func, args, result } => {
            let mut ops = vec![func.clone(), result.clone()];
            ops.extend(args.iter().cloned());
            ops
        }

        // MakeClosure：dst = closure(func, env...)
        Instruction::MakeClosure { dst, env, .. } => {
            let mut ops = vec![dst.clone()];
            ops.extend(env.iter().cloned());
            ops
        }

        // 返回值
        Instruction::Ret(value) => {
            if let Some(v) = value {
                vec![v.clone()]
            } else {
                Vec::new()
            }
        }

        // Upvalue访问
        Instruction::LoadUpvalue { dst, .. } => vec![dst.clone()],
        Instruction::StoreUpvalue { src, .. } => vec![src.clone()],

        // 条件跳转
        Instruction::JmpIf(v, _) | Instruction::JmpIfNot(v, _) => vec![v.clone()],

        // 单个操作数
        Instruction::Push(v) | Instruction::Pop(v) => vec![v.clone()],

        // 无操作数的指令
        Instruction::Dup | Instruction::Swap | Instruction::Yield => Vec::new(),

        // 简单的跳转
        Instruction::Jmp(_) => Vec::new(),

        // 堆分配
        Instruction::Alloc { dst, size } => vec![dst.clone(), size.clone()],
        Instruction::Free(v) => vec![v.clone()],

        // 类型测试
        Instruction::TypeTest(v, _) => vec![v.clone()],

        // 字符串操作
        Instruction::StringGetChar { dst, src, index } => {
            vec![dst.clone(), src.clone(), index.clone()]
        }

        // 字段访问
        Instruction::LoadField { dst, src, .. } => vec![dst.clone(), src.clone()],

        // Arc操作
        Instruction::ArcNew { dst, src } => vec![dst.clone(), src.clone()],

        // unsafe 和指针操作
        Instruction::UnsafeBlockStart | Instruction::UnsafeBlockEnd => Vec::new(),
        Instruction::PtrFromRef { dst, src } => vec![dst.clone(), src.clone()],
        Instruction::PtrDeref { dst, src } => vec![dst.clone(), src.clone()],
        Instruction::PtrStore { dst, src } => vec![dst.clone(), src.clone()],
        Instruction::PtrLoad { dst, src } => vec![dst.clone(), src.clone()],

        // 结构体创建
        Instruction::CreateStruct { dst, fields, .. } => {
            let mut ops = vec![dst.clone()];
            ops.extend(fields.iter().cloned());
            ops
        }
    }
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

    /// 辅助函数：为操作数创建定义
    #[inline]
    fn create_definition(
        &mut self,
        operand: Operand,
        position: (usize, usize),
    ) {
        self.definitions.insert(
            operand,
            Definition {
                position,
                ty: None,
                escapes: false,
                is_moved: false,
            },
        );
    }

    /// 辅助函数：扫描函数找出最大临时变量索引
    fn find_max_temp_index(
        &self,
        func: &FunctionIR,
    ) -> usize {
        let mut max_index = 0;
        for block in &func.blocks {
            for instr in &block.instructions {
                let operands = extract_operands(instr);
                for operand in operands {
                    if let Operand::Temp(idx) = operand {
                        if idx > max_index {
                            max_index = idx;
                        }
                    }
                }
            }
        }
        max_index
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

        // 0. 为所有变量创建定义点
        // 函数参数在入口处定义
        for (idx, _) in func.params.iter().enumerate() {
            self.create_definition(Operand::Arg(idx), (0, 0));
        }

        // 局部变量在声明处定义
        for (idx, _) in func.locals.iter().enumerate() {
            self.create_definition(Operand::Local(idx), (1, idx));
        }

        // 临时变量通过扫描指令确定其索引范围
        let max_temp_index = self.find_max_temp_index(func);

        // 为所有临时变量创建定义
        for idx in 0..=max_temp_index {
            self.create_definition(Operand::Temp(idx), (1, idx));
        }

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
            Instruction::Store { src, dst, .. } => {
                block_live.insert(src.clone());
                block_live.insert(dst.clone());
            }
            Instruction::StoreIndex {
                src, dst, index, ..
            } => {
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

            // Store：初始化局部变量，记录定义
            Instruction::Store { dst, .. } => {
                // 记录 dst 的定义（初始化）
                self.definitions.insert(
                    dst.clone(),
                    Definition {
                        position: pos,
                        ty: None,
                        escapes: false,
                        is_moved: false,
                    },
                );
            }

            // StoreIndex：存储到索引位置
            Instruction::StoreIndex { dst, .. } => {
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
            }

            // StoreField：存储到字段
            Instruction::StoreField { dst, .. } => {
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

            // 返回指令：记录返回值定义
            Instruction::Ret(Some(value)) => {
                // 记录返回值的定义（无论是常量、参数还是局部变量）
                self.definitions.insert(
                    value.clone(),
                    Definition {
                        position: pos,
                        ty: None,
                        escapes: false,
                        is_moved: false,
                    },
                );
            }
            Instruction::Ret(None) => {}

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
