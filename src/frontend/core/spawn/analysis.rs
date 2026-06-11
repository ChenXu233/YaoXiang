//! 编译期 DAG 分析：分析 spawn 块内的依赖关系，生成执行计划
//!
//! RFC-024 核心：spawn 块的直接子表达式创建并行任务。
//! 编译器在编译期分析这些子表达式的读写依赖，通过拓扑排序
//! 将无依赖的任务分组，生成 ExecutionPlan。
//!
//! RFC-024 扩展：Resource 类型感知。
//! 当两个任务操作同一个 Resource 类型的变量时，自动添加串行依赖。
//!
//! RFC-024 扩展：spawn for 数据并行。
//! `spawn for item in items { ... }` 展开为 N 个任务。

use std::collections::{HashMap, HashSet};

use crate::frontend::core::parser::ast::{BinOp, Block, Expr, FStringSegment, Stmt, StmtKind};
use crate::frontend::core::types::{MonoType, TraitTable};
use crate::middle::core::ir::{ExecutionPlan, TaskGroup};

// ============================================================================
// 公共 API
// ============================================================================

/// 单个任务的分析结果
pub struct TaskInfo {
    /// 任务在 spawn 块中的索引（0-based）
    pub index: usize,
    /// 赋值目标变量名（如 `t1 = fetch(...)` 中的 "t1"）
    pub target: Option<String>,
    /// 任务的 RHS 表达式（不含赋值左侧）
    pub expr: Expr,
    /// 此任务读取的变量集
    pub reads: HashSet<String>,
    /// 此任务写入的变量集
    pub writes: HashSet<String>,
    /// 此任务使用的 Resource 类型变量
    pub resource_vars: HashSet<String>,
}

/// spawn 块的完整分析结果
pub struct SpawnAnalysis {
    /// 所有任务的详细信息
    pub tasks: Vec<TaskInfo>,
    /// 执行计划（拓扑排序后的分组）
    pub plan: ExecutionPlan,
}

/// spawn for 块的分析结果
pub struct SpawnForAnalysis {
    /// 迭代变量名（如 "item"）
    pub iter_var: String,
    /// 可迭代表达式（如 "items"）
    pub iterable: Expr,
    /// 循环体（每个任务执行的代码）
    pub body: Block,
    /// 循环体中的读写集
    pub reads: HashSet<String>,
    pub writes: HashSet<String>,
    pub resource_vars: HashSet<String>,
}

/// 判断语句是否是 spawn 块的直接子表达式
///
/// spawn 块内的直接子表达式是顶层表达式语句（`StmtKind::Expr`），
/// 这些语句将被包装为闭包并作为并行任务执行。
///
/// 其他语句类型（如变量声明、绑定等）不是直接子表达式，
/// 它们在 spawn 块内按顺序执行。
pub fn is_direct_child(stmt: &Stmt) -> bool {
    matches!(stmt.kind, StmtKind::Expr(_))
}

/// 分析 spawn 块，生成执行计划
pub fn analyze_spawn_body(
    body: &Block,
    trait_table: &TraitTable,
    local_var_types: &HashMap<String, MonoType>,
) -> SpawnAnalysis {
    let mut tasks = Vec::new();

    for (i, stmt) in body.stmts.iter().enumerate() {
        if !is_direct_child(stmt) {
            continue;
        }
        if let StmtKind::Expr(expr) = &stmt.kind {
            // 检查是否是赋值表达式：t1 = fetch(...)
            if let Expr::BinOp {
                op: BinOp::Assign,
                left,
                right,
                ..
            } = expr.as_ref()
            {
                if let Expr::Var(name, _) = left.as_ref() {
                    let (reads, mut writes, resource_vars) =
                        analyze_reads_writes(right, trait_table, local_var_types);
                    // 赋值目标是写入变量
                    writes.insert(name.clone());
                    tasks.push(TaskInfo {
                        index: i,
                        target: Some(name.clone()),
                        expr: (**right).clone(),
                        reads,
                        writes,
                        resource_vars,
                    });
                } else {
                    let (reads, writes, resource_vars) =
                        analyze_reads_writes(expr, trait_table, local_var_types);
                    tasks.push(TaskInfo {
                        index: i,
                        target: None,
                        expr: (**expr).clone(),
                        reads,
                        writes,
                        resource_vars,
                    });
                }
            } else {
                let (reads, writes, resource_vars) =
                    analyze_reads_writes(expr, trait_table, local_var_types);
                tasks.push(TaskInfo {
                    index: i,
                    target: None,
                    expr: (**expr).clone(),
                    reads,
                    writes,
                    resource_vars,
                });
            }
        }
    }

    // 构建依赖 DAG 并拓扑排序
    let read_write_sets: Vec<(HashSet<String>, HashSet<String>)> = tasks
        .iter()
        .map(|t| (t.reads.clone(), t.writes.clone()))
        .collect();
    let resource_var_sets: Vec<HashSet<String>> =
        tasks.iter().map(|t| t.resource_vars.clone()).collect();
    let plan = build_execution_plan(&read_write_sets, &resource_var_sets);

    SpawnAnalysis { tasks, plan }
}

/// 分析 spawn for 块
///
/// 返回 SpawnForAnalysis，包含迭代信息和循环体的读写集。
/// 实际的任务数量在运行时才知道（取决于 items 的长度），
/// 编译期只分析循环体的依赖特征。
pub fn analyze_spawn_for(
    iter_var: &str,
    iterable: &Expr,
    body: &Block,
    trait_table: &TraitTable,
    local_var_types: &HashMap<String, MonoType>,
) -> SpawnForAnalysis {
    let (reads, writes, resource_vars) =
        analyze_block_reads_writes(body, trait_table, local_var_types);
    SpawnForAnalysis {
        iter_var: iter_var.to_string(),
        iterable: iterable.clone(),
        body: body.clone(),
        reads,
        writes,
        resource_vars,
    }
}

// ============================================================================
// 读写集分析（核心）
// ============================================================================

/// 分析表达式的变量读写集和 Resource 变量集
///
/// 返回 (reads, writes, resource_vars)：
/// - reads: 表达式读取的变量名
/// - writes: 表达式写入的变量名
/// - resource_vars: 表达式使用的 Resource 类型变量名
pub fn analyze_reads_writes(
    expr: &Expr,
    trait_table: &TraitTable,
    local_var_types: &HashMap<String, MonoType>,
) -> (HashSet<String>, HashSet<String>, HashSet<String>) {
    let mut reads = HashSet::new();
    let mut writes = HashSet::new();
    let mut resource_vars = HashSet::new();
    collect_reads_writes(
        expr,
        &mut reads,
        &mut writes,
        &mut resource_vars,
        trait_table,
        local_var_types,
    );
    (reads, writes, resource_vars)
}

/// 分析 Block 的变量读写集和 Resource 变量集
fn analyze_block_reads_writes(
    block: &Block,
    trait_table: &TraitTable,
    local_var_types: &HashMap<String, MonoType>,
) -> (HashSet<String>, HashSet<String>, HashSet<String>) {
    let mut reads = HashSet::new();
    let mut writes = HashSet::new();
    let mut resource_vars = HashSet::new();
    collect_from_block(
        block,
        &mut reads,
        &mut writes,
        &mut resource_vars,
        trait_table,
        local_var_types,
    );
    (reads, writes, resource_vars)
}

/// 检查变量类型是否实现了 Resource trait
fn is_resource_type(
    var_name: &str,
    trait_table: &TraitTable,
    local_var_types: &HashMap<String, MonoType>,
) -> bool {
    if let Some(mono_type) = local_var_types.get(var_name) {
        let type_name = mono_type.type_name();
        trait_table.has_impl("Resource", &type_name)
    } else {
        false
    }
}

/// 从 Block 中收集读写集
fn collect_from_block(
    block: &Block,
    reads: &mut HashSet<String>,
    writes: &mut HashSet<String>,
    resource_vars: &mut HashSet<String>,
    trait_table: &TraitTable,
    local_var_types: &HashMap<String, MonoType>,
) {
    for stmt in &block.stmts {
        collect_from_stmt(
            stmt,
            reads,
            writes,
            resource_vars,
            trait_table,
            local_var_types,
        );
    }
    if let Some(expr) = &block.expr {
        collect_reads_writes(
            expr,
            reads,
            writes,
            resource_vars,
            trait_table,
            local_var_types,
        );
    }
}

/// 从 Stmt 中收集读写集
fn collect_from_stmt(
    stmt: &Stmt,
    reads: &mut HashSet<String>,
    writes: &mut HashSet<String>,
    resource_vars: &mut HashSet<String>,
    trait_table: &TraitTable,
    local_var_types: &HashMap<String, MonoType>,
) {
    match &stmt.kind {
        StmtKind::Expr(expr) => {
            collect_reads_writes(
                expr,
                reads,
                writes,
                resource_vars,
                trait_table,
                local_var_types,
            );
        }
        StmtKind::Var { initializer, .. } => {
            if let Some(init) = initializer {
                collect_reads_writes(
                    init,
                    reads,
                    writes,
                    resource_vars,
                    trait_table,
                    local_var_types,
                );
            }
        }
        StmtKind::For { iterable, body, .. } => {
            collect_reads_writes(
                iterable,
                reads,
                writes,
                resource_vars,
                trait_table,
                local_var_types,
            );
            collect_from_block(
                body,
                reads,
                writes,
                resource_vars,
                trait_table,
                local_var_types,
            );
        }
        StmtKind::If {
            condition,
            then_branch,
            elif_branches,
            else_branch,
            ..
        } => {
            collect_reads_writes(
                condition,
                reads,
                writes,
                resource_vars,
                trait_table,
                local_var_types,
            );
            collect_from_block(
                then_branch,
                reads,
                writes,
                resource_vars,
                trait_table,
                local_var_types,
            );
            for (elif_cond, elif_body) in elif_branches {
                collect_reads_writes(
                    elif_cond,
                    reads,
                    writes,
                    resource_vars,
                    trait_table,
                    local_var_types,
                );
                collect_from_block(
                    elif_body,
                    reads,
                    writes,
                    resource_vars,
                    trait_table,
                    local_var_types,
                );
            }
            if let Some(else_body) = else_branch {
                collect_from_block(
                    else_body,
                    reads,
                    writes,
                    resource_vars,
                    trait_table,
                    local_var_types,
                );
            }
        }
        StmtKind::Binding { body, .. } => {
            for s in &body.0 {
                collect_from_stmt(
                    s,
                    reads,
                    writes,
                    resource_vars,
                    trait_table,
                    local_var_types,
                );
            }
            if let Some(expr) = &body.1 {
                collect_reads_writes(
                    expr,
                    reads,
                    writes,
                    resource_vars,
                    trait_table,
                    local_var_types,
                );
            }
        }
        StmtKind::DestructureAssign { rhs, .. } => {
            collect_reads_writes(
                rhs,
                reads,
                writes,
                resource_vars,
                trait_table,
                local_var_types,
            );
        }
        StmtKind::Use { .. } | StmtKind::ExternalBindingStmt { .. } | StmtKind::Error(_) => {}
    }
}

/// 递归收集表达式的读写集
fn collect_reads_writes(
    expr: &Expr,
    reads: &mut HashSet<String>,
    writes: &mut HashSet<String>,
    resource_vars: &mut HashSet<String>,
    trait_table: &TraitTable,
    local_var_types: &HashMap<String, MonoType>,
) {
    match expr {
        // 叶子节点：变量引用
        Expr::Var(name, _) => {
            reads.insert(name.clone());
            if is_resource_type(name, trait_table, local_var_types) {
                resource_vars.insert(name.clone());
            }
        }

        // 叶子节点：字面量、控制流
        Expr::Lit(..) | Expr::Break(..) | Expr::Continue(..) => {}

        // 编译错误节点
        Expr::Error(_) => {}

        // 赋值：左侧写入，右侧读取
        Expr::BinOp {
            op: BinOp::Assign,
            left,
            right,
            ..
        } => {
            if let Expr::Var(name, _) = left.as_ref() {
                writes.insert(name.clone());
                if is_resource_type(name, trait_table, local_var_types) {
                    resource_vars.insert(name.clone());
                }
            }
            collect_reads_writes(
                right,
                reads,
                writes,
                resource_vars,
                trait_table,
                local_var_types,
            );
        }

        // 其他二元运算
        Expr::BinOp { left, right, .. } => {
            collect_reads_writes(
                left,
                reads,
                writes,
                resource_vars,
                trait_table,
                local_var_types,
            );
            collect_reads_writes(
                right,
                reads,
                writes,
                resource_vars,
                trait_table,
                local_var_types,
            );
        }

        // 一元运算
        Expr::UnOp { expr, .. } => {
            collect_reads_writes(
                expr,
                reads,
                writes,
                resource_vars,
                trait_table,
                local_var_types,
            );
        }

        // 函数调用
        Expr::Call {
            func,
            args,
            named_args,
            ..
        } => {
            collect_reads_writes(
                func,
                reads,
                writes,
                resource_vars,
                trait_table,
                local_var_types,
            );
            for arg in args {
                collect_reads_writes(
                    arg,
                    reads,
                    writes,
                    resource_vars,
                    trait_table,
                    local_var_types,
                );
            }
            for (_name, val) in named_args {
                collect_reads_writes(
                    val,
                    reads,
                    writes,
                    resource_vars,
                    trait_table,
                    local_var_types,
                );
            }
        }

        // 字段访问
        Expr::FieldAccess { expr, .. } => {
            collect_reads_writes(
                expr,
                reads,
                writes,
                resource_vars,
                trait_table,
                local_var_types,
            );
        }

        // 索引访问
        Expr::Index { expr, index, .. } => {
            collect_reads_writes(
                expr,
                reads,
                writes,
                resource_vars,
                trait_table,
                local_var_types,
            );
            collect_reads_writes(
                index,
                reads,
                writes,
                resource_vars,
                trait_table,
                local_var_types,
            );
        }

        // 类型转换
        Expr::Cast { expr, .. } => {
            collect_reads_writes(
                expr,
                reads,
                writes,
                resource_vars,
                trait_table,
                local_var_types,
            );
        }

        // 列表
        Expr::List(elems, _) => {
            for e in elems {
                collect_reads_writes(
                    e,
                    reads,
                    writes,
                    resource_vars,
                    trait_table,
                    local_var_types,
                );
            }
        }

        // 元组
        Expr::Tuple(elems, _) => {
            for e in elems {
                collect_reads_writes(
                    e,
                    reads,
                    writes,
                    resource_vars,
                    trait_table,
                    local_var_types,
                );
            }
        }

        // 字典
        Expr::Dict(pairs, _) => {
            for (k, v) in pairs {
                collect_reads_writes(
                    k,
                    reads,
                    writes,
                    resource_vars,
                    trait_table,
                    local_var_types,
                );
                collect_reads_writes(
                    v,
                    reads,
                    writes,
                    resource_vars,
                    trait_table,
                    local_var_types,
                );
            }
        }

        // 列表推导
        Expr::ListComp {
            element,
            iterable,
            condition,
            ..
        } => {
            collect_reads_writes(
                element,
                reads,
                writes,
                resource_vars,
                trait_table,
                local_var_types,
            );
            collect_reads_writes(
                iterable,
                reads,
                writes,
                resource_vars,
                trait_table,
                local_var_types,
            );
            if let Some(cond) = condition {
                collect_reads_writes(
                    cond,
                    reads,
                    writes,
                    resource_vars,
                    trait_table,
                    local_var_types,
                );
            }
        }

        // if 表达式
        Expr::If {
            condition,
            then_branch,
            elif_branches,
            else_branch,
            ..
        } => {
            collect_reads_writes(
                condition,
                reads,
                writes,
                resource_vars,
                trait_table,
                local_var_types,
            );
            collect_from_block(
                then_branch,
                reads,
                writes,
                resource_vars,
                trait_table,
                local_var_types,
            );
            for (elif_cond, elif_body) in elif_branches {
                collect_reads_writes(
                    elif_cond,
                    reads,
                    writes,
                    resource_vars,
                    trait_table,
                    local_var_types,
                );
                collect_from_block(
                    elif_body,
                    reads,
                    writes,
                    resource_vars,
                    trait_table,
                    local_var_types,
                );
            }
            if let Some(else_body) = else_branch {
                collect_from_block(
                    else_body,
                    reads,
                    writes,
                    resource_vars,
                    trait_table,
                    local_var_types,
                );
            }
        }

        // match 表达式
        Expr::Match { expr, arms, .. } => {
            collect_reads_writes(
                expr,
                reads,
                writes,
                resource_vars,
                trait_table,
                local_var_types,
            );
            for arm in arms {
                collect_from_block(
                    &arm.body,
                    reads,
                    writes,
                    resource_vars,
                    trait_table,
                    local_var_types,
                );
            }
        }

        // while 循环
        Expr::While {
            condition, body, ..
        } => {
            collect_reads_writes(
                condition,
                reads,
                writes,
                resource_vars,
                trait_table,
                local_var_types,
            );
            collect_from_block(
                body,
                reads,
                writes,
                resource_vars,
                trait_table,
                local_var_types,
            );
        }

        // for 循环
        Expr::For { iterable, body, .. } => {
            collect_reads_writes(
                iterable,
                reads,
                writes,
                resource_vars,
                trait_table,
                local_var_types,
            );
            collect_from_block(
                body,
                reads,
                writes,
                resource_vars,
                trait_table,
                local_var_types,
            );
        }

        // 块表达式
        Expr::Block(block) => {
            collect_from_block(
                block,
                reads,
                writes,
                resource_vars,
                trait_table,
                local_var_types,
            );
        }

        // return
        Expr::Return(expr_opt, _) => {
            if let Some(e) = expr_opt {
                collect_reads_writes(
                    e,
                    reads,
                    writes,
                    resource_vars,
                    trait_table,
                    local_var_types,
                );
            }
        }

        // ref
        Expr::Ref { expr, .. } => {
            collect_reads_writes(
                expr,
                reads,
                writes,
                resource_vars,
                trait_table,
                local_var_types,
            );
        }

        // borrow
        Expr::Borrow { expr, .. } => {
            collect_reads_writes(
                expr,
                reads,
                writes,
                resource_vars,
                trait_table,
                local_var_types,
            );
        }

        // try
        Expr::Try { expr, .. } => {
            collect_reads_writes(
                expr,
                reads,
                writes,
                resource_vars,
                trait_table,
                local_var_types,
            );
        }

        // unsafe 块
        Expr::Unsafe { body, .. } => {
            collect_from_block(
                body,
                reads,
                writes,
                resource_vars,
                trait_table,
                local_var_types,
            );
        }

        // spawn 块（嵌套 spawn）
        Expr::Spawn { body, .. } => {
            collect_from_block(
                body,
                reads,
                writes,
                resource_vars,
                trait_table,
                local_var_types,
            );
        }

        // 函数定义（闭包捕获的外层变量）
        Expr::FnDef { body, .. } => {
            collect_from_block(
                body,
                reads,
                writes,
                resource_vars,
                trait_table,
                local_var_types,
            );
        }

        // lambda（闭包捕获的外层变量）
        Expr::Lambda { body, .. } => {
            collect_from_block(
                body,
                reads,
                writes,
                resource_vars,
                trait_table,
                local_var_types,
            );
        }

        // f-string 插值
        Expr::FString { segments, .. } => {
            for seg in segments {
                if let FStringSegment::Interpolation { expr, .. } = seg {
                    collect_reads_writes(
                        expr,
                        reads,
                        writes,
                        resource_vars,
                        trait_table,
                        local_var_types,
                    );
                }
            }
        }
    }
}

// ============================================================================
// 依赖 DAG 构建
// ============================================================================

/// 构建依赖 DAG，拓扑排序生成执行计划
///
/// 规则：
/// 1. 写后读：任务 i 读取任务 j 写入的变量 → i 依赖 j
/// 2. 写后写：任务 i 写入任务 j 写入的变量 → i 依赖 j
/// 3. 资源冲突：任务 i 和 j 使用同一 Resource 变量 → i 依赖 j（串行）
pub fn build_execution_plan(
    read_write_sets: &[(HashSet<String>, HashSet<String>)],
    resource_var_sets: &[HashSet<String>],
) -> ExecutionPlan {
    let n = read_write_sets.len();
    if n == 0 {
        return ExecutionPlan { groups: vec![] };
    }

    // 构建依赖关系：deps[i] = 任务 i 必须等待的任务列表
    let mut deps: Vec<Vec<usize>> = vec![vec![]; n];
    for i in 0..n {
        for j in 0..i {
            // 规则 1：任务 i 读取了任务 j 写入的变量 → i 依赖 j（写后读）
            let reads_i = &read_write_sets[i].0;
            let writes_j = &read_write_sets[j].1;
            if !reads_i.is_disjoint(writes_j) {
                deps[i].push(j);
            }
            // 规则 2：任务 i 写入了任务 j 写入的变量 → i 依赖 j（写后写）
            let writes_i = &read_write_sets[i].1;
            if !writes_i.is_disjoint(writes_j) && !deps[i].contains(&j) {
                deps[i].push(j);
            }
            // 规则 3：两个任务使用同一 Resource 变量 → 串行
            if !resource_var_sets[i].is_disjoint(&resource_var_sets[j]) && !deps[i].contains(&j) {
                deps[i].push(j);
            }
        }
    }

    // BFS 分层拓扑排序
    let mut groups = Vec::new();
    let mut remaining: HashSet<usize> = (0..n).collect();

    loop {
        // 找出所有依赖已满足的任务（可立即执行）
        let ready: Vec<usize> = remaining
            .iter()
            .filter(|&&i| deps[i].iter().all(|d| !remaining.contains(d)))
            .copied()
            .collect();

        if ready.is_empty() {
            break;
        }

        for &task in &ready {
            remaining.remove(&task);
        }
        groups.push(TaskGroup {
            task_indices: ready,
        });
    }

    // 处理剩余任务（环依赖）
    if !remaining.is_empty() {
        groups.push(TaskGroup {
            task_indices: remaining.into_iter().collect(),
        });
    }

    ExecutionPlan { groups }
}

    //! spawn 分析测试 — 基于 RFC-024 和 RFC-018
    //!
    //! RFC-024 §2.1: spawn 块的直接子表达式创建并行任务
    //! RFC-024 §2.3: 编译期 DAG 分析，依赖驱动执行
    //! RFC-024 §2.5: Resource 类型感知，同资源自动串行
    //! RFC-024 §2.4: spawn for 数据并行
    //! RFC-018 §5: spawn 块代码生成

    use crate::frontend::core::parser::ast::{
        BinOp, Block, Expr, FStringSegment, MatchArm, Pattern, Stmt, StmtKind,
    };
    use crate::frontend::core::spawn::analysis::{
        analyze_reads_writes, analyze_spawn_body, analyze_spawn_for, build_execution_plan,
        is_direct_child,
    };
    use crate::frontend::core::types::{MonoType, TraitTable};
    use crate::util::span::Span;
    use std::collections::{HashMap, HashSet};

    // ========================================================================
    // 辅助函数
    // ========================================================================

    fn dummy_span() -> Span {
        Span::dummy()
    }

    fn empty_trait_table() -> TraitTable {
        TraitTable::default()
    }

    fn empty_var_types() -> HashMap<String, MonoType> {
        HashMap::new()
    }

    fn var_expr(name: &str) -> Expr {
        Expr::Var(name.to_string(), dummy_span())
    }

    fn assign_stmt(
        target: &str,
        rhs: Expr,
    ) -> Stmt {
        Stmt {
            kind: StmtKind::Expr(Box::new(Expr::BinOp {
                op: BinOp::Assign,
                left: Box::new(var_expr(target)),
                right: Box::new(rhs),
                span: dummy_span(),
            })),
            span: dummy_span(),
        }
    }

    fn call_expr(
        func: &str,
        args: Vec<Expr>,
    ) -> Expr {
        Expr::Call {
            func: Box::new(var_expr(func)),
            args,
            named_args: vec![],
            span: dummy_span(),
        }
    }

    fn resource_trait_table() -> (TraitTable, HashMap<String, MonoType>) {
        use crate::frontend::core::types::{TraitDefinition, TraitImplementation};
        let mut trait_table = TraitTable::default();
        trait_table.add_trait(TraitDefinition {
            name: "Resource".to_string(),
            methods: HashMap::new(),
            parent_traits: Vec::new(),
            generic_params: vec![],
            span: None,
            is_marker: true,
        });
        trait_table.add_impl(TraitImplementation {
            trait_name: "Resource".to_string(),
            for_type_name: "FilePath".to_string(),
            methods: HashMap::new(),
        });
        let mut local_var_types = HashMap::new();
        local_var_types.insert(
            "file".to_string(),
            MonoType::TypeRef("FilePath".to_string()),
        );
        (trait_table, local_var_types)
    }

    fn spawn_body(stmts: Vec<Stmt>) -> Block {
        Block {
            stmts,
            expr: None,
            span: dummy_span(),
        }
    }

    // ========================================================================
    // is_direct_child
    // ========================================================================

    #[test]
    fn test_is_direct_child_with_expr_stmt() {
        // RFC-024 §2.1: 直接子表达式是顶层 StmtKind::Expr
        // Arrange
        let stmt = Stmt {
            kind: StmtKind::Expr(Box::new(var_expr("x"))),
            span: dummy_span(),
        };

        // Act
        let result = is_direct_child(&stmt);

        // Assert
        assert!(result, "StmtKind::Expr 应被视为直接子表达式");
    }

    #[test]
    fn test_is_direct_child_with_var_decl_is_false() {
        // RFC-024 §2.1: 变量声明不是直接子表达式
        // Arrange
        let stmt = Stmt {
            kind: StmtKind::Var {
                name: "x".to_string(),
                name_span: dummy_span(),
                type_annotation: None,
                initializer: Some(Box::new(Expr::Lit(
                    crate::frontend::core::lexer::tokens::Literal::Int(42),
                    dummy_span(),
                ))),
                is_mut: false,
            },
            span: dummy_span(),
        };

        // Act
        let result = is_direct_child(&stmt);

        // Assert
        assert!(!result, "StmtKind::Var 不应被视为直接子表达式");
    }

    // ========================================================================
    // analyze_reads_writes — 基础
    // ========================================================================

    #[test]
    fn test_reads_writes_var_reference() {
        // Arrange
        let expr = var_expr("x");

        // Act
        let (reads, writes, _) =
            analyze_reads_writes(&expr, &empty_trait_table(), &empty_var_types());

        // Assert
        assert!(reads.contains("x"), "变量引用应被记入 reads");
        assert!(writes.is_empty(), "变量引用不应产生 writes");
    }

    #[test]
    fn test_reads_writes_assignment_target_and_rhs() {
        // Arrange
        let expr = Expr::BinOp {
            op: BinOp::Assign,
            left: Box::new(var_expr("t1")),
            right: Box::new(call_expr("fetch", vec![var_expr("a")])),
            span: dummy_span(),
        };

        // Act
        let (reads, writes, _) =
            analyze_reads_writes(&expr, &empty_trait_table(), &empty_var_types());

        // Assert
        assert!(writes.contains("t1"), "赋值目标应被记入 writes");
        assert!(reads.contains("fetch"), "函数名应被记入 reads");
        assert!(reads.contains("a"), "函数参数应被记入 reads");
    }

    #[test]
    fn test_reads_writes_call_args() {
        // Arrange
        let expr = call_expr("process", vec![var_expr("x"), var_expr("y")]);

        // Act
        let (reads, _, _) = analyze_reads_writes(&expr, &empty_trait_table(), &empty_var_types());

        // Assert
        assert!(reads.contains("process"), "函数名应被记入 reads");
        assert!(reads.contains("x"), "第一个参数应被记入 reads");
        assert!(reads.contains("y"), "第二个参数应被记入 reads");
    }

    // ========================================================================
    // analyze_reads_writes — 表达式覆盖
    // ========================================================================

    #[test]
    fn test_reads_writes_list_elements() {
        // Arrange
        let expr = Expr::List(
            vec![var_expr("a"), var_expr("b"), var_expr("c")],
            dummy_span(),
        );

        // Act
        let (reads, _, _) = analyze_reads_writes(&expr, &empty_trait_table(), &empty_var_types());

        // Assert
        assert!(reads.contains("a"), "列表元素 a 应被记入 reads");
        assert!(reads.contains("b"), "列表元素 b 应被记入 reads");
        assert!(reads.contains("c"), "列表元素 c 应被记入 reads");
    }

    #[test]
    fn test_reads_writes_tuple_elements() {
        // Arrange
        let expr = Expr::Tuple(vec![var_expr("x"), var_expr("y")], dummy_span());

        // Act
        let (reads, _, _) = analyze_reads_writes(&expr, &empty_trait_table(), &empty_var_types());

        // Assert
        assert!(reads.contains("x"), "元组元素 x 应被记入 reads");
        assert!(reads.contains("y"), "元组元素 y 应被记入 reads");
    }

    #[test]
    fn test_reads_writes_dict_pairs() {
        // Arrange
        let expr = Expr::Dict(vec![(var_expr("k"), var_expr("v"))], dummy_span());

        // Act
        let (reads, _, _) = analyze_reads_writes(&expr, &empty_trait_table(), &empty_var_types());

        // Assert
        assert!(reads.contains("k"), "字典键 k 应被记入 reads");
        assert!(reads.contains("v"), "字典值 v 应被记入 reads");
    }

    #[test]
    fn test_reads_writes_index_access() {
        // Arrange
        let expr = Expr::Index {
            expr: Box::new(var_expr("arr")),
            index: Box::new(var_expr("i")),
            span: dummy_span(),
        };

        // Act
        let (reads, _, _) = analyze_reads_writes(&expr, &empty_trait_table(), &empty_var_types());

        // Assert
        assert!(reads.contains("arr"), "索引目标 arr 应被记入 reads");
        assert!(reads.contains("i"), "索引值 i 应被记入 reads");
    }

    #[test]
    fn test_reads_writes_cast_expr() {
        // Arrange
        let expr = Expr::Cast {
            expr: Box::new(var_expr("x")),
            target_type: crate::frontend::core::parser::ast::Type::Name {
                name: "Int".to_string(),
                span: dummy_span(),
            },
            span: dummy_span(),
        };

        // Act
        let (reads, _, _) = analyze_reads_writes(&expr, &empty_trait_table(), &empty_var_types());

        // Assert
        assert!(reads.contains("x"), "类型转换表达式中的变量应被记入 reads");
    }

    #[test]
    fn test_reads_writes_match_expr_and_arms() {
        // Arrange
        let expr = Expr::Match {
            expr: Box::new(var_expr("val")),
            arms: vec![MatchArm {
                pattern: Pattern::Wildcard,
                body: Block {
                    stmts: vec![],
                    expr: Some(Box::new(var_expr("result"))),
                    span: dummy_span(),
                },
                span: dummy_span(),
            }],
            span: dummy_span(),
        };

        // Act
        let (reads, _, _) = analyze_reads_writes(&expr, &empty_trait_table(), &empty_var_types());

        // Assert
        assert!(
            reads.contains("val"),
            "match 表达式 scrutinee 应被记入 reads"
        );
        assert!(
            reads.contains("result"),
            "match arm body 中的变量应被记入 reads"
        );
    }

    #[test]
    fn test_reads_writes_if_expr_branches() {
        // Arrange
        let expr = Expr::If {
            condition: Box::new(var_expr("cond")),
            then_branch: Box::new(Block {
                stmts: vec![],
                expr: Some(Box::new(var_expr("a"))),
                span: dummy_span(),
            }),
            elif_branches: vec![],
            else_branch: Some(Box::new(Block {
                stmts: vec![],
                expr: Some(Box::new(var_expr("b"))),
                span: dummy_span(),
            })),
            span: dummy_span(),
        };

        // Act
        let (reads, _, _) = analyze_reads_writes(&expr, &empty_trait_table(), &empty_var_types());

        // Assert
        assert!(reads.contains("cond"), "if 条件应被记入 reads");
        assert!(reads.contains("a"), "then 分支变量应被记入 reads");
        assert!(reads.contains("b"), "else 分支变量应被记入 reads");
    }

    #[test]
    fn test_reads_writes_fstring_interpolation() {
        // Arrange
        let expr = Expr::FString {
            segments: vec![FStringSegment::Interpolation {
                expr: Box::new(var_expr("name")),
                format_spec: None,
            }],
            span: dummy_span(),
        };

        // Act
        let (reads, _, _) = analyze_reads_writes(&expr, &empty_trait_table(), &empty_var_types());

        // Assert
        assert!(reads.contains("name"), "f-string 插值变量应被记入 reads");
    }

    // ========================================================================
    // build_execution_plan
    // ========================================================================

    #[test]
    fn test_plan_independent_tasks_single_group() {
        // RFC-024 §2.1: 无依赖的直接子表达式可并行执行
        // Arrange
        let sets: Vec<(HashSet<String>, HashSet<String>)> = vec![
            (HashSet::from(["a".into()]), HashSet::from(["t1".into()])),
            (HashSet::from(["b".into()]), HashSet::from(["t2".into()])),
        ];
        let resource_sets: Vec<HashSet<String>> = vec![HashSet::new(), HashSet::new()];

        // Act
        let plan = build_execution_plan(&sets, &resource_sets);

        // Assert
        assert_eq!(plan.groups.len(), 1, "无依赖任务应在同一并行组");
        assert_eq!(
            plan.groups[0].task_indices.len(),
            2,
            "两个独立任务应都在组内"
        );
    }

    #[test]
    fn test_plan_read_after_write_creates_dependency() {
        // RFC-024 §2.3: 任务 i 读取任务 j 写入的变量 → i 依赖 j
        // Arrange
        let sets: Vec<(HashSet<String>, HashSet<String>)> = vec![
            (HashSet::new(), HashSet::from(["t1".into()])),
            (HashSet::from(["t1".into()]), HashSet::from(["t2".into()])),
        ];
        let resource_sets: Vec<HashSet<String>> = vec![HashSet::new(), HashSet::new()];

        // Act
        let plan = build_execution_plan(&sets, &resource_sets);

        // Assert
        assert_eq!(plan.groups.len(), 2, "读写依赖应产生两个串行组");
        assert_eq!(plan.groups[0].task_indices, vec![0], "第一个任务应在第一组");
        assert_eq!(plan.groups[1].task_indices, vec![1], "依赖任务应在第二组");
    }

    #[test]
    fn test_plan_diamond_dependency_two_groups() {
        // RFC-024 §2.3: 菱形依赖 — x,y 独立并行，z 依赖两者
        // Arrange
        let sets: Vec<(HashSet<String>, HashSet<String>)> = vec![
            (HashSet::new(), HashSet::from(["x".into()])),
            (HashSet::new(), HashSet::from(["y".into()])),
            (
                HashSet::from(["x".into(), "y".into()]),
                HashSet::from(["z".into()]),
            ),
        ];
        let resource_sets: Vec<HashSet<String>> =
            vec![HashSet::new(), HashSet::new(), HashSet::new()];

        // Act
        let plan = build_execution_plan(&sets, &resource_sets);

        // Assert
        assert_eq!(plan.groups.len(), 2, "菱形依赖应产生两个组");
        assert_eq!(
            plan.groups[0].task_indices.len(),
            2,
            "独立任务应并行（第一组 2 个）"
        );
        assert_eq!(plan.groups[1].task_indices, vec![2], "依赖任务应在第二组");
    }

    #[test]
    fn test_plan_empty_input_no_groups() {
        // Arrange
        let sets: Vec<(HashSet<String>, HashSet<String>)> = vec![];
        let resource_sets: Vec<HashSet<String>> = vec![];

        // Act
        let plan = build_execution_plan(&sets, &resource_sets);

        // Assert
        assert_eq!(plan.groups.len(), 0, "空输入应产生空执行计划");
    }

    #[test]
    fn test_plan_write_write_conflict_serializes() {
        // RFC-024 §2.3: 写后写冲突 → 串行
        // Arrange
        let sets: Vec<(HashSet<String>, HashSet<String>)> = vec![
            (HashSet::new(), HashSet::from(["x".into()])),
            (HashSet::new(), HashSet::from(["x".into()])),
        ];
        let resource_sets: Vec<HashSet<String>> = vec![HashSet::new(), HashSet::new()];

        // Act
        let plan = build_execution_plan(&sets, &resource_sets);

        // Assert
        assert_eq!(plan.groups.len(), 2, "写后写冲突应产生两个串行组");
    }

    #[test]
    fn test_plan_resource_conflict_serializes() {
        // RFC-024 §2.5: 同一 Resource 变量被多个任务使用 → 串行
        // Arrange
        let sets: Vec<(HashSet<String>, HashSet<String>)> = vec![
            (HashSet::from(["file".into()]), HashSet::from(["a".into()])),
            (HashSet::from(["file".into()]), HashSet::from(["b".into()])),
        ];
        let resource_sets: Vec<HashSet<String>> = vec![
            HashSet::from(["file".into()]),
            HashSet::from(["file".into()]),
        ];

        // Act
        let plan = build_execution_plan(&sets, &resource_sets);

        // Assert
        assert_eq!(plan.groups.len(), 2, "Resource 冲突应产生两个串行组");
    }

    #[test]
    fn test_plan_non_resource_read_read_allows_parallel() {
        // Arrange
        let sets: Vec<(HashSet<String>, HashSet<String>)> = vec![
            (HashSet::from(["x".into()]), HashSet::from(["a".into()])),
            (HashSet::from(["x".into()]), HashSet::from(["b".into()])),
        ];
        let resource_sets: Vec<HashSet<String>> = vec![HashSet::new(), HashSet::new()];

        // Act
        let plan = build_execution_plan(&sets, &resource_sets);

        // Assert
        assert_eq!(
            plan.groups.len(),
            1,
            "非 Resource 变量的读-读不冲突，应允许并行"
        );
    }

    // ========================================================================
    // analyze_spawn_body 端到端
    // ========================================================================

    #[test]
    fn test_spawn_body_independent_tasks() {
        // RFC-024 §2.1: t1 = f(), t2 = g() → 无依赖，并行执行
        // Arrange
        let body = spawn_body(vec![
            assign_stmt("t1", call_expr("f", vec![])),
            assign_stmt("t2", call_expr("g", vec![])),
        ]);

        // Act
        let analysis = analyze_spawn_body(&body, &empty_trait_table(), &empty_var_types());

        // Assert
        assert_eq!(analysis.tasks.len(), 2, "应识别出 2 个任务");
        assert_eq!(
            analysis.tasks[0].target.as_deref(),
            Some("t1"),
            "第一个任务目标应为 t1"
        );
        assert_eq!(
            analysis.tasks[1].target.as_deref(),
            Some("t2"),
            "第二个任务目标应为 t2"
        );
        assert_eq!(analysis.plan.groups.len(), 1, "无依赖任务应在同一并行组");
    }

    #[test]
    fn test_spawn_body_with_read_after_write_dependency() {
        // RFC-024 §2.3: x = f(), y = g(x) → y 依赖 x
        // Arrange
        let body = spawn_body(vec![
            assign_stmt("x", call_expr("f", vec![])),
            assign_stmt("y", call_expr("g", vec![var_expr("x")])),
        ]);

        // Act
        let analysis = analyze_spawn_body(&body, &empty_trait_table(), &empty_var_types());

        // Assert
        assert_eq!(analysis.plan.groups.len(), 2, "读写依赖应产生两个串行组");
        assert_eq!(
            analysis.plan.groups[0].task_indices,
            vec![0],
            "第一个任务应在第一组"
        );
        assert_eq!(
            analysis.plan.groups[1].task_indices,
            vec![1],
            "依赖任务应在第二组"
        );
    }

    #[test]
    fn test_spawn_body_resource_conflict_serializes() {
        // RFC-024 §2.5: 同一 Resource 类型变量 → 自动串行
        // Arrange
        let (trait_table, local_var_types) = resource_trait_table();
        let body = spawn_body(vec![
            assign_stmt("a", call_expr("read_file", vec![var_expr("file")])),
            assign_stmt("b", call_expr("stat_file", vec![var_expr("file")])),
        ]);

        // Act
        let analysis = analyze_spawn_body(&body, &trait_table, &local_var_types);

        // Assert
        assert_eq!(
            analysis.plan.groups.len(),
            2,
            "Resource 冲突应产生两个串行组"
        );
    }

    #[test]
    fn test_spawn_body_task_info_exposes_reads_writes() {
        // Arrange
        let body = spawn_body(vec![assign_stmt("t1", call_expr("f", vec![var_expr("a")]))]);

        // Act
        let analysis = analyze_spawn_body(&body, &empty_trait_table(), &empty_var_types());

        // Assert
        let task = &analysis.tasks[0];
        assert!(task.reads.contains("f"), "任务 reads 应包含函数名 f");
        assert!(task.reads.contains("a"), "任务 reads 应包含参数 a");
        assert!(task.writes.contains("t1"), "任务 writes 应包含赋值目标 t1");
    }

    // ========================================================================
    // analyze_spawn_for
    // ========================================================================

    #[test]
    fn test_spawn_for_reads_writes() {
        // RFC-024 §2.4: spawn for 展开为 N 个任务
        // Arrange
        let body = Block {
            stmts: vec![],
            expr: Some(Box::new(call_expr("process", vec![var_expr("item")]))),
            span: dummy_span(),
        };
        let iterable = var_expr("items");

        // Act
        let analysis = analyze_spawn_for(
            "item",
            &iterable,
            &body,
            &empty_trait_table(),
            &empty_var_types(),
        );

        // Assert
        assert_eq!(analysis.iter_var, "item", "迭代变量名应为 item");
        assert!(
            analysis.reads.contains("process"),
            "循环体 reads 应包含函数名 process"
        );
        assert!(
            analysis.reads.contains("item"),
            "循环体 reads 应包含迭代变量 item"
        );
    }
}
