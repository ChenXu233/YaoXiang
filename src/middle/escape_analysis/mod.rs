//! 逃逸分析器
//!
//! 决定对象应该在栈上分配还是堆上分配。
//! 采用基于规则的分析策略，辅以简单的数据流分析。

use crate::frontend::typecheck::MonoType;
use crate::middle::ir::{FunctionIR, Instruction, Operand};
use std::collections::{HashMap, HashSet};
use std::fmt;

/// 逃逸分析器
///
/// 决定对象应该在栈上分配还是堆上分配。
/// 采用基于规则的分析策略，辅以简单的数据流分析。
pub struct EscapeAnalyzer {
    /// 当前分析的函数
    current_function: Option<FunctionIR>,

    /// 局部变量信息
    local_vars: HashMap<LocalId, LocalInfo>,

    /// 调用图
    call_graph: CallGraph,

    /// 循环嵌套深度
    loop_depth: usize,

    /// 逃逸分析配置
    config: EscapeAnalysisConfig,
}

#[derive(Debug, Clone)]
struct LocalInfo {
    /// 变量类型
    ty: MonoType,

    /// 分配方式（待确定）
    allocation: Allocation,

    /// 是否逃逸
    escapes: bool,

    /// 被哪些闭包捕获
    captured_by: HashSet<ClosureId>,

    /// 是否被返回
    is_returned: bool,

    /// 是否被赋值给全局变量
    assigned_to_global: bool,

    /// 是否被传递给其他函数
    passed_to_function: bool,

    /// 是否是循环变量
    is_loop_variable: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Allocation {
    /// 栈分配（零开销）
    Stack,

    /// 堆分配（ARC）
    Heap,

    /// 待确定
    Undecided,
}

#[derive(Debug, Clone)]
pub struct EscapeAnalysisConfig {
    /// 是否启用优化
    enable_optimizations: bool,

    /// 循环变量的特殊处理策略
    loop_variable_strategy: LoopVariableStrategy,

    /// 是否启用逃逸分析的激进模式
    aggressive_escape: bool,
}

#[derive(Debug, Clone, Copy)]
enum LoopVariableStrategy {
    /// 总是栈分配循环变量
    AlwaysStack,

    /// 根据上下文决定
    ContextAware,

    /// 总是堆分配循环变量
    AlwaysHeap,
}

impl Default for EscapeAnalysisConfig {
    fn default() -> Self {
        EscapeAnalysisConfig {
            enable_optimizations: true,
            loop_variable_strategy: LoopVariableStrategy::ContextAware,
            aggressive_escape: false,
        }
    }
}

/// 调用图
#[derive(Debug, Default)]
struct CallGraph {
    /// 函数调用关系
    edges: HashMap<FunctionId, Vec<FunctionId>>,
}

/// 函数ID
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
struct FunctionId(String);

impl EscapeAnalyzer {
    /// 创建新的逃逸分析器
    pub fn new() -> Self {
        EscapeAnalyzer {
            current_function: None,
            local_vars: HashMap::new(),
            call_graph: CallGraph::new(),
            loop_depth: 0,
            config: EscapeAnalysisConfig::default(),
        }
    }

    /// 分析函数的逃逸情况
    pub fn analyze_function(
        &mut self,
        func: &FunctionIR,
    ) -> EscapeAnalysisResult {
        self.current_function = Some(func.clone());
        self.local_vars.clear();

        // 1. 收集所有局部变量
        self.collect_locals(func);

        // 2. 标记明显的逃逸情况
        self.mark_obvious_escapes(func);

        // 3. 传播逃逸状态
        self.propagate_escape(func);

        // 4. 生成分析结果
        self.generate_result(func)
    }

    /// 收集所有局部变量
    fn collect_locals(
        &mut self,
        func: &FunctionIR,
    ) {
        for (idx, local_ty) in func.locals.iter().enumerate() {
            self.local_vars.insert(
                LocalId::new(idx),
                LocalInfo {
                    ty: local_ty.clone(),
                    allocation: Allocation::Undecided,
                    escapes: false,
                    captured_by: HashSet::new(),
                    is_returned: false,
                    assigned_to_global: false,
                    passed_to_function: false,
                    is_loop_variable: false,
                },
            );
        }
    }

    /// 标记明显的逃逸情况
    fn mark_obvious_escapes(
        &mut self,
        func: &FunctionIR,
    ) {
        for block in &func.blocks {
            for instr in &block.instructions {
                match instr {
                    // 返回值逃逸
                    Instruction::Ret(value) => {
                        if let Some(var) = value.as_ref() {
                            if let Some(local_id) = self.get_variable(var) {
                                self.local_vars.get_mut(&local_id).unwrap().escapes = true;
                                self.local_vars.get_mut(&local_id).unwrap().is_returned = true;
                            }
                        }
                    },

                    // 字段存储逃逸（赋值给对象的字段可能逃逸）
                    Instruction::StoreField { src, .. } => {
                        if let Some(local_id) = self.get_variable(src) {
                            self.local_vars.get_mut(&local_id).unwrap().escapes = true;
                        }
                    },

                    // 索引存储逃逸（赋值给数组元素可能逃逸）
                    Instruction::StoreIndex { src, .. } => {
                        if let Some(local_id) = self.get_variable(src) {
                            self.local_vars.get_mut(&local_id).unwrap().escapes = true;
                        }
                    },

                    // 函数调用可能导致参数逃逸
                    Instruction::Call { args, .. } => {
                        for arg in args {
                            if let Some(local_id) = self.get_variable(arg) {
                                self.local_vars
                                    .get_mut(&local_id)
                                    .unwrap()
                                    .passed_to_function = true;
                                // 保守假设：传递给函数的参数可能逃逸
                                self.local_vars.get_mut(&local_id).unwrap().escapes = true;
                            }
                        }
                    },

                    // 尾调用也可能导致参数逃逸
                    Instruction::TailCall { args, .. } => {
                        for arg in args {
                            if let Some(local_id) = self.get_variable(arg) {
                                self.local_vars
                                    .get_mut(&local_id)
                                    .unwrap()
                                    .passed_to_function = true;
                                self.local_vars.get_mut(&local_id).unwrap().escapes = true;
                            }
                        }
                    },

                    // 类型转换可能导致逃逸
                    Instruction::Cast { src, .. } => {
                        if let Some(local_id) = self.get_variable(src) {
                            self.local_vars.get_mut(&local_id).unwrap().escapes = true;
                        }
                    },

                    _ => {},
                }
            }
        }
    }

    /// 传播逃逸状态
    ///
    /// 通过数据流分析传播逃逸状态：
    /// 1. 赋值传播：如果 a 逃逸，a = b 则 b 也逃逸
    /// 2. phi 节点传播：合并的变量如果任一逃逸则结果逃逸
    /// 3. 循环变量传播：循环内的变量在循环外使用则逃逸
    fn propagate_escape(
        &mut self,
        func: &FunctionIR,
    ) {
        // 构建变量使用关系图
        let var_uses = self.build_var_use_graph(func);

        // 迭代传播，直到不动点
        let mut changed = true;
        let mut iterations = 0;
        const MAX_ITERATIONS: usize = 100; // 防止无限循环

        while changed && iterations < MAX_ITERATIONS {
            changed = false;
            iterations += 1;

            // 分析每条指令的逃逸传播
            for block in &func.blocks {
                for instr in &block.instructions {
                    if self.analyze_escape_propagation(instr, &var_uses) {
                        changed = true;
                    }
                }
            }

            // 如果达到最大迭代次数，发出警告
            if iterations >= MAX_ITERATIONS {
                // 在实际实现中可以记录警告日志
            }
        }
    }

    /// 构建变量使用关系图
    ///
    /// 返回：Map<变量ID, Set<被该变量赋值的变量ID>>
    fn build_var_use_graph(
        &mut self,
        func: &FunctionIR,
    ) -> HashMap<LocalId, HashSet<LocalId>> {
        let mut var_uses: HashMap<LocalId, HashSet<LocalId>> = HashMap::new();

        for block in &func.blocks {
            for instr in &block.instructions {
                self.analyze_var_uses(instr, &mut var_uses);
            }
        }

        var_uses
    }

    /// 分析单条指令中的变量使用关系
    fn analyze_var_uses(
        &mut self,
        instr: &Instruction,
        var_uses: &mut HashMap<LocalId, HashSet<LocalId>>,
    ) {
        match instr {
            // a = b 形式：b 被 a 使用
            Instruction::Move { dst, src } => {
                if let (Some(dst_id), Some(src_id)) =
                    (self.get_variable(dst), self.get_variable(src))
                {
                    var_uses.entry(src_id).or_default().insert(dst_id);
                }
            },

            // Load: a = *b 形式
            Instruction::Load { dst, src } => {
                if let (Some(dst_id), Some(src_id)) =
                    (self.get_variable(dst), self.get_variable(src))
                {
                    var_uses.entry(src_id).or_default().insert(dst_id);
                }
            },

            // Store: *a = b 形式
            Instruction::Store { dst, src } => {
                if let Some(src_id) = self.get_variable(src) {
                    // dst 可能是任何地址表达式，简化处理
                    if let Some(dst_local) = self.get_variable(dst) {
                        var_uses.entry(src_id).or_default().insert(dst_local);
                    }
                }
            },

            // 函数调用：返回值可能来自参数
            Instruction::Call { dst, args, .. } => {
                if let Some(dst_id) = dst.as_ref().and_then(|d| self.get_variable(d)) {
                    for arg in args {
                        if let Some(arg_id) = self.get_variable(arg) {
                            var_uses.entry(arg_id).or_default().insert(dst_id);
                        }
                    }
                }
            },

            // 尾调用
            Instruction::TailCall { func: _, args } => {
                // 尾调用的参数可能影响调用者状态
                for arg in args {
                    if let Some(arg_id) = self.get_variable(arg) {
                        // 标记参数可能逃逸
                        if let Some(info) = self.local_vars.get_mut(&arg_id) {
                            info.passed_to_function = true;
                            info.escapes = true;
                        }
                    }
                }
            },

            // 类型转换不改变逃逸状态
            Instruction::Cast { dst, src, .. } => {
                if let (Some(dst_id), Some(src_id)) =
                    (self.get_variable(dst), self.get_variable(src))
                {
                    var_uses.entry(src_id).or_default().insert(dst_id);
                }
            },

            _ => {},
        }
    }

    /// 分析指令的逃逸传播
    ///
    /// 返回 true 表示状态发生了变化
    fn analyze_escape_propagation(
        &mut self,
        instr: &Instruction,
        var_uses: &HashMap<LocalId, HashSet<LocalId>>,
    ) -> bool {
        let mut changed = false;

        match instr {
            // 赋值：目标继承源的逃逸状态
            Instruction::Move { dst, src } => {
                if let (Some(dst_id), Some(src_id)) =
                    (self.get_variable(dst), self.get_variable(src))
                {
                    let src_escapes = self
                        .local_vars
                        .get(&src_id)
                        .map(|i| i.escapes)
                        .unwrap_or(false);

                    if let Some(dst_info) = self.local_vars.get_mut(&dst_id) {
                        if src_escapes && !dst_info.escapes {
                            dst_info.escapes = true;
                            changed = true;
                        }
                    }

                    // 递归传播：如果源逃逸，传播到所有使用目标的地方
                    if src_escapes {
                        changed |= self.propagate_to_uses(dst_id, src_escapes, var_uses);
                    }
                }
            },

            // 函数调用：返回值可能继承参数的逃逸状态
            Instruction::Call { dst, args, .. } => {
                // 检查返回值是否应该逃逸（基于参数）
                let mut return_escapes = false;
                for arg in args {
                    if let Some(arg_id) = self.get_variable(arg) {
                        if self
                            .local_vars
                            .get(&arg_id)
                            .map(|i| i.escapes)
                            .unwrap_or(false)
                        {
                            return_escapes = true;
                            break;
                        }
                    }
                }

                if return_escapes {
                    if let Some(dst_id) = dst.as_ref().and_then(|d| self.get_variable(d)) {
                        if let Some(dst_info) = self.local_vars.get_mut(&dst_id) {
                            if !dst_info.escapes {
                                dst_info.escapes = true;
                                changed = true;
                            }
                        }
                    }
                }
            },

            // 移动传播
            Instruction::Load { dst, src } => {
                if let (Some(dst_id), Some(src_id)) =
                    (self.get_variable(dst), self.get_variable(src))
                {
                    let src_escapes = self
                        .local_vars
                        .get(&src_id)
                        .map(|i| i.escapes)
                        .unwrap_or(false);

                    if let Some(dst_info) = self.local_vars.get_mut(&dst_id) {
                        if src_escapes && !dst_info.escapes {
                            dst_info.escapes = true;
                            changed = true;
                        }
                    }
                }
            },

            _ => {},
        }

        changed
    }

    /// 递归传播逃逸状态到使用该变量的所有变量
    fn propagate_to_uses(
        &mut self,
        var_id: LocalId,
        escapes: bool,
        var_uses: &HashMap<LocalId, HashSet<LocalId>>,
    ) -> bool {
        if !escapes {
            return false;
        }

        let mut changed = false;
        let mut to_process = vec![var_id];
        let mut processed = HashSet::new();

        while let Some(current) = to_process.pop() {
            if processed.contains(&current) {
                continue;
            }
            processed.insert(current);

            // 获取所有使用当前变量的变量
            if let Some(used_by) = var_uses.get(&current) {
                for &dependent in used_by {
                    if let Some(info) = self.local_vars.get_mut(&dependent) {
                        if !info.escapes {
                            info.escapes = true;
                            changed = true;
                            to_process.push(dependent);
                        }
                    }
                }
            }
        }

        changed
    }

    /// 标记被调用函数返回值导致的逃逸
    fn mark_callee_return_escapes(
        &mut self,
        var: LocalId,
        _callee: &Operand,
    ) -> bool {
        // 保守假设：如果变量被传递给可能返回它的函数，则逃逸
        // 在实际实现中需要更精确的分析
        let mut escaped = false;

        if let Some(info) = self.local_vars.get_mut(&var) {
            info.passed_to_function = true;
            info.escapes = true;
            escaped = true;
        }

        escaped
    }

    /// 生成分析结果
    fn generate_result(
        &self,
        _func: &FunctionIR,
    ) -> EscapeAnalysisResult {
        let mut stack_allocated = HashSet::new();
        let mut heap_allocated = HashSet::new();

        for (var_id, info) in &self.local_vars {
            if info.escapes || info.is_returned || info.assigned_to_global {
                // 逃逸的对象必须堆分配
                heap_allocated.insert(*var_id);
            } else if info.captured_by.is_empty() {
                // 不逃逸且不被捕获，栈分配
                stack_allocated.insert(*var_id);
            } else {
                // 被闭包捕获但不逃逸，可以栈分配但需要闭包包装
                heap_allocated.insert(*var_id);
            }
        }

        EscapeAnalysisResult {
            stack_allocated,
            heap_allocated,
        }
    }

    /// 从操作数获取变量ID
    fn get_variable(
        &self,
        operand: &Operand,
    ) -> Option<LocalId> {
        match operand {
            Operand::Local(id) => Some(LocalId::new(*id)),
            _ => None,
        }
    }

    /// 配置分析器
    pub fn configure<F>(
        &mut self,
        f: F,
    ) where
        F: FnOnce(&mut EscapeAnalysisConfig),
    {
        f(&mut self.config);
    }
}

/// 逃逸分析结果
#[derive(Debug, Clone)]
pub struct EscapeAnalysisResult {
    /// 栈分配的变量
    pub stack_allocated: HashSet<LocalId>,

    /// 堆分配的变量
    pub heap_allocated: HashSet<LocalId>,
}

impl EscapeAnalysisResult {
    /// 检查变量是否应该栈分配
    pub fn should_stack_allocate(
        &self,
        var_id: LocalId,
    ) -> bool {
        self.stack_allocated.contains(&var_id)
    }

    /// 检查变量是否应该堆分配
    pub fn should_heap_allocate(
        &self,
        var_id: LocalId,
    ) -> bool {
        self.heap_allocated.contains(&var_id)
    }

    /// 获取分配方式
    pub fn get_allocation(
        &self,
        var_id: LocalId,
    ) -> Allocation {
        if self.stack_allocated.contains(&var_id) {
            Allocation::Stack
        } else if self.heap_allocated.contains(&var_id) {
            Allocation::Heap
        } else {
            Allocation::Undecided
        }
    }
}

/// 局部变量ID
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct LocalId(usize);

impl LocalId {
    /// 创建新的局部变量ID
    pub fn new(id: usize) -> Self {
        LocalId(id)
    }

    /// 获取内部索引
    pub fn index(&self) -> usize {
        self.0
    }
}

impl fmt::Display for LocalId {
    fn fmt(
        &self,
        f: &mut fmt::Formatter<'_>,
    ) -> fmt::Result {
        write!(f, "local_{}", self.0)
    }
}

/// 闭包ID
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
struct ClosureId(usize);

impl ClosureId {
    fn new(id: usize) -> Self {
        ClosureId(id)
    }
}

impl Default for EscapeAnalyzer {
    fn default() -> Self {
        EscapeAnalyzer::new()
    }
}

impl CallGraph {
    fn new() -> Self {
        CallGraph {
            edges: HashMap::new(),
        }
    }

    fn add_edge(
        &mut self,
        from: FunctionId,
        to: FunctionId,
    ) {
        self.edges.entry(from).or_default().push(to);
    }
}

/// 逃逸分析结果构建器
///
/// 用于逐步构建逃逸分析结果
#[derive(Debug, Default)]
pub struct EscapeAnalysisResultBuilder {
    stack_allocated: HashSet<LocalId>,
    heap_allocated: HashSet<LocalId>,
}

impl EscapeAnalysisResultBuilder {
    /// 创建新的构建器
    pub fn new() -> Self {
        EscapeAnalysisResultBuilder {
            stack_allocated: HashSet::new(),
            heap_allocated: HashSet::new(),
        }
    }

    /// 标记变量为栈分配
    pub fn mark_stack_allocated(
        &mut self,
        var_id: LocalId,
    ) {
        self.stack_allocated.insert(var_id);
    }

    /// 标记变量为堆分配
    pub fn mark_heap_allocated(
        &mut self,
        var_id: LocalId,
    ) {
        self.heap_allocated.insert(var_id);
    }

    /// 批量标记为栈分配
    pub fn mark_stack_allocated_batch(
        &mut self,
        vars: impl IntoIterator<Item = LocalId>,
    ) {
        for var in vars {
            self.stack_allocated.insert(var);
        }
    }

    /// 批量标记为堆分配
    pub fn mark_heap_allocated_batch(
        &mut self,
        vars: impl IntoIterator<Item = LocalId>,
    ) {
        for var in vars {
            self.heap_allocated.insert(var);
        }
    }

    /// 构建结果
    pub fn build(self) -> EscapeAnalysisResult {
        EscapeAnalysisResult {
            stack_allocated: self.stack_allocated,
            heap_allocated: self.heap_allocated,
        }
    }
}
