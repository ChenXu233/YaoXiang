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
                    let (reads, mut writes, mut resource_vars) =
                        analyze_reads_writes(right, trait_table, local_var_types);
                    // 赋值目标是写入变量
                    writes.insert(name.clone());
                    if is_resource_type(name, trait_table, local_var_types) {
                        resource_vars.insert(name.clone());
                    }
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
            for s in body {
                collect_from_stmt(
                    s,
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
        StmtKind::Return(expr_opt) => {
            if let Some(expr) = expr_opt {
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

        // spawn for 数据并行循环
        Expr::SpawnFor { iterable, body, .. } => {
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
        return ExecutionPlan {
            groups: vec![],
            task_deps: vec![],
            task_resources: vec![],
        };
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

    // 构建资源变量列表
    let task_resources: Vec<Vec<String>> = resource_var_sets
        .iter()
        .map(|set| set.iter().cloned().collect())
        .collect();

    ExecutionPlan {
        groups,
        task_deps: deps,
        task_resources,
    }
}
