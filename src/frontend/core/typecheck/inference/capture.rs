//! 闭包捕获分析模块
//!
//! 分析 Lambda 表达式对外部变量的捕获行为，确定每个被捕获变量的捕获模式。
//!
//! ## 设计概述
//!
//! 1. **逃逸分析** (`analyze_closure_usage`) — 判断闭包是否会逃逸当前作用域
//! 2. **捕获分析** (`analyze_captures`) — 遍历 Lambda 体 AST，收集对外部变量的引用
//! 3. **模式选择** (`determine_capture_mode`) — 根据使用方式和类型确定捕获模式
//! 4. **主入口** (`analyze_lambda_captures`) — 组合以上步骤，返回 `CaptureInfo`

use std::collections::{HashMap, HashSet};

use crate::frontend::core::parser::ast::{Block, Expr, Stmt, StmtKind};
use crate::frontend::core::typecheck::traits::solver::TraitSolver;
use crate::frontend::core::types::base::{MonoType, PolyType};

// ============================================================================
// 数据结构
// ============================================================================

/// 闭包的使用方式（决定逃逸状态）
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ClosureUsage {
    /// 立即调用或传递给同步函数
    Inline,
    /// spawn / return / 存储到变量或字段
    Escaping,
}

/// 被捕获变量在闭包体内的使用方式
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CaptureUsage {
    /// 仅读取
    Read,
    /// 被赋值或调用了 mut 方法
    Write,
    /// 所有权被转移（move 语义）
    Move,
}

/// 被闭包捕获的变量
#[derive(Debug, Clone)]
pub struct CapturedVar {
    /// 变量名
    pub name: String,
    /// 在闭包体内的使用方式
    pub usage: CaptureUsage,
}

/// 编译器选择的捕获模式
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CaptureMode {
    /// 原语值类型或 Dup 类型 — 隐式复制
    Copy,
    /// &T 令牌 — 不可变借用
    Borrow,
    /// &mut T 令牌 — 可变借用
    BorrowMut,
    /// 所有权转移
    Move,
}

/// 捕获分析的最终结果
#[derive(Debug, Clone)]
pub struct CaptureInfo {
    /// (变量名, 捕获模式) 列表
    pub captures: Vec<(String, CaptureMode)>,
}

// ============================================================================
// 逃逸分析
// ============================================================================

/// 分析闭包的使用方式，判断其是否会逃逸
///
/// 通过检查父表达式判断 Lambda 是否被传递到可能逃逸的位置：
/// - 作为 `spawn` 的主体 → Escaping
/// - 作为 `return` 的值 → Escaping
/// - 被赋值给变量或字段 → Escaping
/// - 作为函数参数传递 → 需进一步判断（保守处理为 Inline）
/// - 其他情况 → Inline
pub fn analyze_closure_usage(
    lambda_expr: &Expr,
    parent: Option<&Expr>,
) -> ClosureUsage {
    let Some(parent) = parent else {
        return ClosureUsage::Inline;
    };

    match parent {
        // spawn { ... lambda ... } — 闭包在并发块中，必定逃逸
        Expr::Spawn { .. } => ClosureUsage::Escaping,

        // return lambda — 闭包作为返回值，必定逃逸
        Expr::Return(Some(ret_expr), _) => {
            if std::ptr::eq(ret_expr.as_ref(), lambda_expr) {
                ClosureUsage::Escaping
            } else {
                ClosureUsage::Inline
            }
        }

        // 赋值语句：x = lambda 或 obj.field = lambda
        Expr::BinOp {
            op: crate::frontend::core::parser::ast::BinOp::Assign,
            left,
            ..
        } => match left.as_ref() {
            Expr::Var(_, _) => ClosureUsage::Escaping,
            Expr::FieldAccess { .. } => ClosureUsage::Escaping,
            Expr::Index { .. } => ClosureUsage::Escaping,
            _ => ClosureUsage::Inline,
        },

        // 函数调用参数中的 lambda — 保守处理为 Inline
        // （实际应检查被调函数是否为 spawn 等，留待后续增强）
        Expr::Call { .. } => ClosureUsage::Inline,

        // 其他上下文
        _ => ClosureUsage::Inline,
    }
}

// ============================================================================
// 捕获分析
// ============================================================================

/// 分析 Lambda 体，收集对外部变量的捕获
///
/// 遍历 Lambda 体的完整 AST，找出所有引用了外部作用域变量的 `Expr::Var`，
/// 并根据变量的使用方式分类为 Read / Write / Move。
pub fn analyze_captures(
    lambda_body: &Block,
    outer_scope: &HashSet<String>,
) -> Vec<CapturedVar> {
    // 第一遍：收集所有读取的变量名
    let mut read_vars = HashSet::new();
    extract_read_vars_from_block(lambda_body, &mut read_vars);

    // 第二遍：收集所有被写入的变量名
    let mut written_vars = HashSet::new();
    extract_written_vars_from_block(lambda_body, &mut written_vars);

    // 合并结果，仅保留外部作用域变量
    let mut result = Vec::new();
    let mut seen = HashSet::new();

    // 先处理写入的变量（优先级更高）
    for name in &written_vars {
        if outer_scope.contains(name) && seen.insert(name.clone()) {
            result.push(CapturedVar {
                name: name.clone(),
                usage: CaptureUsage::Write,
            });
        }
    }

    // 再处理读取的变量
    for name in &read_vars {
        if outer_scope.contains(name) && seen.insert(name.clone()) {
            result.push(CapturedVar {
                name: name.clone(),
                usage: CaptureUsage::Read,
            });
        }
    }

    result
}

// ============================================================================
// 模式选择
// ============================================================================

/// 根据使用方式、类型和闭包逃逸状态确定捕获模式
///
/// 决策逻辑：
/// 1. 原语值类型（Int, Float, Bool, Char）→ Copy（编译器内置值复制）
/// 2. Dup 类型（&T, ref T, String, Bytes 等）→ Copy（浅拷贝，复制句柄共享数据）
/// 3. 闭包逃逸 + 非 Dup → Move（所有权转移）
/// 4. 闭包内联：
///    - Read → Borrow（不可变借用）
///    - Write → BorrowMut（可变借用）
///    - Move → Move（所有权转移）
pub fn determine_capture_mode(
    usage: &CaptureUsage,
    ty: &MonoType,
    closure_usage: &ClosureUsage,
    trait_solver: &TraitSolver,
) -> CaptureMode {
    // 原语值类型：编译器自动值复制
    if TraitSolver::is_primitive_value_type(ty) {
        return CaptureMode::Copy;
    }

    let is_dup = check_dup_trait(ty, trait_solver);

    match (is_dup, closure_usage) {
        // Dup 类型总是拷贝（浅拷贝共享数据）
        (true, _) => CaptureMode::Copy,
        // 逃逸的非 Dup 类型必须 move
        (false, ClosureUsage::Escaping) => CaptureMode::Move,
        // 内联闭包根据使用方式决定
        (false, ClosureUsage::Inline) => match usage {
            CaptureUsage::Read => CaptureMode::Borrow,
            CaptureUsage::Write => CaptureMode::BorrowMut,
            CaptureUsage::Move => CaptureMode::Move,
        },
    }
}

// ============================================================================
// 主入口
// ============================================================================

/// 分析 Lambda 的完整捕获信息
///
/// 组合逃逸分析、捕获收集和模式选择，返回最终的 `CaptureInfo`。
///
/// # 参数
/// - `lambda_expr`: Lambda 表达式 AST 节点
/// - `lambda_body`: Lambda 体（Block）
/// - `outer_scope`: 外部作用域中所有变量名的集合
/// - `var_types`: 外部作用域变量的类型映射（变量名 -> PolyType）
/// - `trait_solver`: 特质求解器，用于检查 Dup 特质
/// - `parent`: 父表达式（Lambda 所在的上下文），用于逃逸分析
pub fn analyze_lambda_captures(
    lambda_expr: &Expr,
    lambda_body: &Block,
    outer_scope: &HashSet<String>,
    var_types: &HashMap<String, PolyType>,
    trait_solver: &TraitSolver,
    parent: Option<&Expr>,
) -> CaptureInfo {
    let usage = analyze_closure_usage(lambda_expr, parent);
    let captures = analyze_captures(lambda_body, outer_scope);

    let modes = captures
        .into_iter()
        .map(|cap| {
            // 从 var_types 中获取变量类型
            let ty = var_types
                .get(&cap.name)
                .map(|poly| poly.body.clone())
                .unwrap_or(MonoType::Void);
            let mode = determine_capture_mode(&cap.usage, &ty, &usage, trait_solver);
            (cap.name, mode)
        })
        .collect();

    CaptureInfo { captures: modes }
}

// ============================================================================
// 内部辅助：AST 遍历 — 读取变量收集
// ============================================================================

/// 从 Block 中收集所有被读取的变量名
fn extract_read_vars_from_block(
    block: &Block,
    vars: &mut HashSet<String>,
) {
    for stmt in &block.stmts {
        extract_read_vars_from_stmt(stmt, vars);
    }
    if let Some(ref expr) = block.expr {
        extract_read_vars_from_expr(expr, vars);
    }
}

/// 从 Stmt 中收集所有被读取的变量名
fn extract_read_vars_from_stmt(
    stmt: &Stmt,
    vars: &mut HashSet<String>,
) {
    match &stmt.kind {
        StmtKind::Expr(expr) => {
            extract_read_vars_from_expr(expr, vars);
        }
        StmtKind::Var {
            initializer, name, ..
        } => {
            // 变量声明中，初始化器中的变量是被读取的
            if let Some(init) = initializer {
                extract_read_vars_from_expr(init, vars);
            }
            // 声明的变量名本身不是"读取"，需要从后续使用中收集
            let _ = name;
        }
        StmtKind::If {
            condition,
            then_branch,
            elif_branches,
            else_branch,
            ..
        } => {
            extract_read_vars_from_expr(condition, vars);
            extract_read_vars_from_block(then_branch, vars);
            for (elif_cond, elif_body) in elif_branches {
                extract_read_vars_from_expr(elif_cond, vars);
                extract_read_vars_from_block(elif_body, vars);
            }
            if let Some(else_b) = else_branch {
                extract_read_vars_from_block(else_b, vars);
            }
        }
        StmtKind::For {
            var,
            iterable,
            body,
            ..
        } => {
            extract_read_vars_from_expr(iterable, vars);
            // For 循环变量在 body 内是局部的，但 iterable 中的变量是外部的
            let mut body_vars = HashSet::new();
            extract_read_vars_from_block(body, &mut body_vars);
            // 移除循环变量本身（它是局部的）
            body_vars.remove(var);
            vars.extend(body_vars);
        }
        StmtKind::Binding {
            params,
            body: (body_stmts, body_expr),
            ..
        } => {
            // Binding 的参数是局部的，但 body 中可能读取外部变量
            let param_names: HashSet<String> = params.iter().map(|p| p.name.clone()).collect();
            let mut body_vars = HashSet::new();
            for body_stmt in body_stmts {
                extract_read_vars_from_stmt(body_stmt, &mut body_vars);
            }
            if let Some(expr) = body_expr {
                extract_read_vars_from_expr(expr, &mut body_vars);
            }
            // 移除参数名（它们是局部的）
            for pn in &param_names {
                body_vars.remove(pn);
            }
            vars.extend(body_vars);
        }
        StmtKind::DestructureAssign { rhs, .. } => {
            extract_read_vars_from_expr(rhs, vars);
        }
        StmtKind::Use { .. } => {
            // use 语句不产生变量读取
        }
        StmtKind::ExternalBindingStmt { .. } => {
            // 外部绑定语句不直接读取变量
        }
        StmtKind::Error(_) => {
            // 错误恢复节点，跳过
        }
    }
}

/// 从 Expr 中收集所有被读取的变量名
fn extract_read_vars_from_expr(
    expr: &Expr,
    vars: &mut HashSet<String>,
) {
    match expr {
        Expr::Var(name, _) => {
            vars.insert(name.clone());
        }
        Expr::Lit(_, _) => {}
        Expr::BinOp {
            op, left, right, ..
        } => {
            if *op == crate::frontend::core::parser::ast::BinOp::Assign {
                // 赋值：左侧如果是变量则不在此处收集（视为写入），
                // 但左侧的复杂表达式（如 Index、FieldAccess）中的变量仍需读取
                match left.as_ref() {
                    Expr::Var(_, _) => {
                        // 简单变量赋值：变量是写入目标，不作为读取收集
                    }
                    _ => {
                        extract_read_vars_from_expr(left, vars);
                    }
                }
                extract_read_vars_from_expr(right, vars);
            } else {
                extract_read_vars_from_expr(left, vars);
                extract_read_vars_from_expr(right, vars);
            }
        }
        Expr::UnOp { expr, .. } => {
            extract_read_vars_from_expr(expr, vars);
        }
        Expr::Call {
            func,
            args,
            named_args,
            ..
        } => {
            extract_read_vars_from_expr(func, vars);
            for arg in args {
                extract_read_vars_from_expr(arg, vars);
            }
            for (_, arg) in named_args {
                extract_read_vars_from_expr(arg, vars);
            }
        }
        Expr::FnDef { params, body, .. } => {
            // 函数定义：参数是局部的，但 body 中可能读取外部变量
            let param_names: HashSet<String> = params.iter().map(|p| p.name.clone()).collect();
            let mut body_vars = HashSet::new();
            extract_read_vars_from_block(body, &mut body_vars);
            for pn in &param_names {
                body_vars.remove(pn);
            }
            vars.extend(body_vars);
        }
        Expr::If {
            condition,
            then_branch,
            elif_branches,
            else_branch,
            ..
        } => {
            extract_read_vars_from_expr(condition, vars);
            extract_read_vars_from_block(then_branch, vars);
            for (elif_cond, elif_body) in elif_branches {
                extract_read_vars_from_expr(elif_cond, vars);
                extract_read_vars_from_block(elif_body, vars);
            }
            if let Some(else_b) = else_branch {
                extract_read_vars_from_block(else_b, vars);
            }
        }
        Expr::Match {
            expr: match_expr,
            arms,
            ..
        } => {
            extract_read_vars_from_expr(match_expr, vars);
            for arm in arms {
                extract_read_vars_from_block(&arm.body, vars);
            }
        }
        Expr::While {
            condition, body, ..
        } => {
            extract_read_vars_from_expr(condition, vars);
            extract_read_vars_from_block(body, vars);
        }
        Expr::For {
            var,
            iterable,
            body,
            ..
        } => {
            extract_read_vars_from_expr(iterable, vars);
            let mut body_vars = HashSet::new();
            extract_read_vars_from_block(body, &mut body_vars);
            body_vars.remove(var);
            vars.extend(body_vars);
        }
        Expr::Block(block) => {
            extract_read_vars_from_block(block, vars);
        }
        Expr::Return(Some(ret_expr), _) => {
            extract_read_vars_from_expr(ret_expr, vars);
        }
        Expr::Return(None, _) => {}
        Expr::Break(_, _) | Expr::Continue(_, _) => {}
        Expr::Cast { expr, .. } => {
            extract_read_vars_from_expr(expr, vars);
        }
        Expr::Tuple(elems, _) => {
            for elem in elems {
                extract_read_vars_from_expr(elem, vars);
            }
        }
        Expr::List(elems, _) => {
            for elem in elems {
                extract_read_vars_from_expr(elem, vars);
            }
        }
        Expr::ListComp {
            element,
            var,
            iterable,
            condition,
            ..
        } => {
            extract_read_vars_from_expr(iterable, vars);
            // 列表推导中的变量是局部的
            let mut inner_vars = HashSet::new();
            extract_read_vars_from_expr(element, &mut inner_vars);
            if let Some(cond) = condition {
                extract_read_vars_from_expr(cond, &mut inner_vars);
            }
            inner_vars.remove(var);
            vars.extend(inner_vars);
        }
        Expr::Dict(pairs, _) => {
            for (key, val) in pairs {
                extract_read_vars_from_expr(key, vars);
                extract_read_vars_from_expr(val, vars);
            }
        }
        Expr::Index { expr, index, .. } => {
            extract_read_vars_from_expr(expr, vars);
            extract_read_vars_from_expr(index, vars);
        }
        Expr::FieldAccess { expr, .. } => {
            extract_read_vars_from_expr(expr, vars);
        }
        Expr::Try { expr, .. } => {
            extract_read_vars_from_expr(expr, vars);
        }
        Expr::Ref { expr, .. } => {
            extract_read_vars_from_expr(expr, vars);
        }
        Expr::Unsafe { body, .. } => {
            extract_read_vars_from_block(body, vars);
        }
        Expr::Spawn { body, .. } => {
            extract_read_vars_from_block(body, vars);
        }
        Expr::Lambda { params, body, .. } => {
            // Lambda 是一个新的闭包边界：其参数是局部的
            let param_names: HashSet<String> = params.iter().map(|p| p.name.clone()).collect();
            let mut body_vars = HashSet::new();
            extract_read_vars_from_block(body, &mut body_vars);
            for pn in &param_names {
                body_vars.remove(pn);
            }
            vars.extend(body_vars);
        }
        Expr::FString { segments, .. } => {
            for seg in segments {
                if let crate::frontend::core::parser::ast::FStringSegment::Interpolation {
                    expr,
                    ..
                } = seg
                {
                    extract_read_vars_from_expr(expr, vars);
                }
            }
        }
        Expr::Borrow { expr, .. } => {
            extract_read_vars_from_expr(expr, vars);
        }
        Expr::Error(_) => {
            // 错误恢复节点，跳过
        }
    }
}

// ============================================================================
// 内部辅助：AST 遍历 — 写入变量收集
// ============================================================================

/// 从 Block 中收集所有被写入的变量名
fn extract_written_vars_from_block(
    block: &Block,
    vars: &mut HashSet<String>,
) {
    for stmt in &block.stmts {
        extract_written_vars_from_stmt(stmt, vars);
    }
    // 尾部表达式通常不是赋值，但为完整性仍检查
    if let Some(ref expr) = block.expr {
        extract_written_vars_from_expr(expr, vars);
    }
}

/// 从 Stmt 中收集所有被写入的变量名
fn extract_written_vars_from_stmt(
    stmt: &Stmt,
    vars: &mut HashSet<String>,
) {
    match &stmt.kind {
        StmtKind::Expr(expr) => {
            extract_written_vars_from_expr(expr, vars);
        }
        StmtKind::Var { .. } => {
            // 变量声明本身是对新变量的写入，但新变量是局部的，不在此收集
        }
        StmtKind::If {
            then_branch,
            elif_branches,
            else_branch,
            ..
        } => {
            extract_written_vars_from_block(then_branch, vars);
            for (_, elif_body) in elif_branches {
                extract_written_vars_from_block(elif_body, vars);
            }
            if let Some(else_b) = else_branch {
                extract_written_vars_from_block(else_b, vars);
            }
        }
        StmtKind::For { var, body, .. } => {
            let mut body_vars = HashSet::new();
            extract_written_vars_from_block(body, &mut body_vars);
            body_vars.remove(var);
            vars.extend(body_vars);
        }
        StmtKind::Binding {
            params,
            body: (body_stmts, body_expr),
            ..
        } => {
            let param_names: HashSet<String> = params.iter().map(|p| p.name.clone()).collect();
            let mut body_vars = HashSet::new();
            for body_stmt in body_stmts {
                extract_written_vars_from_stmt(body_stmt, &mut body_vars);
            }
            if let Some(expr) = body_expr {
                extract_written_vars_from_expr(expr, &mut body_vars);
            }
            for pn in &param_names {
                body_vars.remove(pn);
            }
            vars.extend(body_vars);
        }
        StmtKind::DestructureAssign { .. } => {
            // 解构赋值中声明的变量是局部的，不在此收集
        }
        StmtKind::Use { .. } | StmtKind::ExternalBindingStmt { .. } | StmtKind::Error(_) => {}
    }
}

/// 从 Expr 中收集所有被写入的变量名
fn extract_written_vars_from_expr(
    expr: &Expr,
    vars: &mut HashSet<String>,
) {
    match expr {
        Expr::BinOp {
            op, left, right, ..
        } if *op == crate::frontend::core::parser::ast::BinOp::Assign => {
            // 赋值：左侧的 Var 是写入目标
            if let Expr::Var(name, _) = left.as_ref() {
                vars.insert(name.clone());
            }
            // 右侧可能包含嵌套赋值
            extract_written_vars_from_expr(right, vars);
        }
        Expr::If {
            then_branch,
            elif_branches,
            else_branch,
            ..
        } => {
            extract_written_vars_from_block(then_branch, vars);
            for (_, elif_body) in elif_branches {
                extract_written_vars_from_block(elif_body, vars);
            }
            if let Some(else_b) = else_branch {
                extract_written_vars_from_block(else_b, vars);
            }
        }
        Expr::While { body, .. } => {
            extract_written_vars_from_block(body, vars);
        }
        Expr::For { var, body, .. } => {
            let mut body_vars = HashSet::new();
            extract_written_vars_from_block(body, &mut body_vars);
            body_vars.remove(var);
            vars.extend(body_vars);
        }
        Expr::Block(block) => {
            extract_written_vars_from_block(block, vars);
        }
        Expr::Lambda { params, body, .. } => {
            let param_names: HashSet<String> = params.iter().map(|p| p.name.clone()).collect();
            let mut body_vars = HashSet::new();
            extract_written_vars_from_block(body, &mut body_vars);
            for pn in &param_names {
                body_vars.remove(pn);
            }
            vars.extend(body_vars);
        }
        Expr::Spawn { body, .. } => {
            extract_written_vars_from_block(body, vars);
        }
        Expr::Unsafe { body, .. } => {
            extract_written_vars_from_block(body, vars);
        }
        // 其他表达式类型不直接写入变量
        _ => {}
    }
}

// ============================================================================
// 内部辅助：Dup 特质检查
// ============================================================================

/// 检查类型是否满足 Dup 特质（浅拷贝：复制句柄，共享底层数据）
///
/// Dup 适用于引用/令牌类型和内部引用计数的类型。
/// 原语值类型（Int, Float, Bool, Char）不是 Dup——使用 is_primitive_value_type 检查。
fn check_dup_trait(
    ty: &MonoType,
    trait_solver: &TraitSolver,
) -> bool {
    // 优先使用 TraitSolver 的公开 check_trait 接口
    let mut solver = TraitSolver::new();
    if let Some(table) = trait_solver.trait_table() {
        solver.set_trait_table(table.clone());
    }
    solver.check_trait(ty, "Dup")
}

// ============================================================================
// 测试
// ============================================================================

#[cfg(test)]
mod tests {
    //! 闭包捕获模式推断测试
    //!
    //! 对应规范:
    //! - RFC-009 v9 (所有权模型): 捕获模式决策 (Copy / Borrow / BorrowMut / Move)
    //! - RFC-023 (闭包捕获模型): 捕获变量的用途分析 (Read / Write) 与逃逸判定

    use super::*;
    use crate::frontend::core::parser::ast::BinOp;
    use crate::util::span::Span;

    fn dummy_span() -> Span {
        Span::dummy()
    }

    /// 构造一个包含 `Fn` 字段的非 Dup 结构体类型 (用于捕获模式测试)。
    /// 对应 RFC-009: 不满足 `Dup` trait 的类型在不同场景下的捕获行为。
    fn make_non_dup_struct_ty() -> MonoType {
        use std::collections::HashMap;
        let fn_ty = MonoType::Fn {
            params: vec![MonoType::Int(64)],
            return_type: Box::new(MonoType::Void),
        };
        MonoType::Struct(crate::frontend::core::types::base::mono::StructType {
            name: "MyStruct".to_string(),
            fields: vec![("callback".to_string(), fn_ty)],
            methods: HashMap::new(),
            field_mutability: vec![],
            field_has_default: vec![],
            interfaces: vec![],
        })
    }

    #[test]
    fn test_analyze_captures_read_only() {
        // lambda: () => x + y
        // outer scope: {x, y, z}
        let block = Block {
            stmts: vec![],
            expr: Some(Box::new(Expr::BinOp {
                op: BinOp::Add,
                left: Box::new(Expr::Var("x".to_string(), dummy_span())),
                right: Box::new(Expr::Var("y".to_string(), dummy_span())),
                span: dummy_span(),
            })),
            span: dummy_span(),
        };

        let outer: HashSet<String> = ["x", "y", "z"].iter().map(|s| s.to_string()).collect();
        let captures = analyze_captures(&block, &outer);

        assert_eq!(
            captures.len(),
            2,
            "x and y should be captured, z should not"
        );
        let names: HashSet<&str> = captures.iter().map(|c| c.name.as_str()).collect();
        assert!(names.contains("x"), "x should be in captures");
        assert!(names.contains("y"), "y should be in captures");
        assert!(
            !names.contains("z"),
            "z should not be captured (not in outer scope)"
        );

        for cap in &captures {
            assert_eq!(
                cap.usage,
                CaptureUsage::Read,
                "all captures should be Read-only"
            );
        }
    }

    #[test]
    fn test_analyze_captures_write() {
        // lambda: () => { x = 42 }
        // outer scope: {x}
        let block = Block {
            stmts: vec![Stmt {
                kind: StmtKind::Expr(Box::new(Expr::BinOp {
                    op: BinOp::Assign,
                    left: Box::new(Expr::Var("x".to_string(), dummy_span())),
                    right: Box::new(Expr::Lit(
                        crate::frontend::core::lexer::tokens::Literal::Int(42),
                        dummy_span(),
                    )),
                    span: dummy_span(),
                })),
                span: dummy_span(),
            }],
            expr: None,
            span: dummy_span(),
        };

        let outer: HashSet<String> = ["x"].iter().map(|s| s.to_string()).collect();
        let captures = analyze_captures(&block, &outer);

        assert_eq!(captures.len(), 1, "only x should be captured");
        assert_eq!(captures[0].name, "x", "captured variable should be x");
        assert_eq!(
            captures[0].usage,
            CaptureUsage::Write,
            "x is assigned, should be Write"
        );
    }

    #[test]
    fn test_analyze_captures_local_var_not_captured() {
        // lambda: () => { let z = 10; z }
        // outer scope: {x}
        let block = Block {
            stmts: vec![Stmt {
                kind: StmtKind::Var {
                    name: "z".to_string(),
                    name_span: dummy_span(),
                    type_annotation: None,
                    initializer: Some(Box::new(Expr::Lit(
                        crate::frontend::core::lexer::tokens::Literal::Int(10),
                        dummy_span(),
                    ))),
                    is_mut: false,
                },
                span: dummy_span(),
            }],
            expr: Some(Box::new(Expr::Var("z".to_string(), dummy_span()))),
            span: dummy_span(),
        };

        let outer: HashSet<String> = ["x"].iter().map(|s| s.to_string()).collect();
        let captures = analyze_captures(&block, &outer);

        // z 是局部变量，不应被捕获
        assert_eq!(captures.len(), 0, "local variable z should not be captured");
    }

    #[test]
    fn test_analyze_captures_nested_lambda() {
        // outer lambda: () => { (a) => x + a }
        // outer scope: {x}
        let inner_lambda = Expr::Lambda {
            params: vec![crate::frontend::core::parser::ast::Param {
                name: "a".to_string(),
                ty: None,
                is_mut: false,
                span: dummy_span(),
            }],
            body: Box::new(Block {
                stmts: vec![],
                expr: Some(Box::new(Expr::BinOp {
                    op: BinOp::Add,
                    left: Box::new(Expr::Var("x".to_string(), dummy_span())),
                    right: Box::new(Expr::Var("a".to_string(), dummy_span())),
                    span: dummy_span(),
                })),
                span: dummy_span(),
            }),
            span: dummy_span(),
        };

        let block = Block {
            stmts: vec![],
            expr: Some(Box::new(inner_lambda)),
            span: dummy_span(),
        };

        let outer: HashSet<String> = ["x"].iter().map(|s| s.to_string()).collect();
        let captures = analyze_captures(&block, &outer);

        assert_eq!(
            captures.len(),
            1,
            "only x from outer scope should be captured"
        );
        assert_eq!(captures[0].name, "x", "captured variable should be x");
        assert_eq!(
            captures[0].usage,
            CaptureUsage::Read,
            "x is only read in nested lambda"
        );
    }

    #[test]
    fn test_determine_capture_mode_dup_inline() {
        let solver = TraitSolver::new();
        let mode = determine_capture_mode(
            &CaptureUsage::Read,
            &MonoType::Int(64),
            &ClosureUsage::Inline,
            &solver,
        );
        assert_eq!(
            mode,
            CaptureMode::Copy,
            "Int is Dup, inline read should Copy"
        );
    }

    #[test]
    fn test_determine_capture_mode_dup_escaping() {
        let solver = TraitSolver::new();
        let mode = determine_capture_mode(
            &CaptureUsage::Read,
            &MonoType::Int(64),
            &ClosureUsage::Escaping,
            &solver,
        );
        // Dup 类型即使逃逸也是 Copy
        assert_eq!(
            mode,
            CaptureMode::Copy,
            "Int is Dup, escaping should still Copy"
        );
    }

    #[test]
    fn test_determine_capture_mode_non_dup_read_inline() {
        let solver = TraitSolver::new();
        let struct_ty = make_non_dup_struct_ty();
        let mode = determine_capture_mode(
            &CaptureUsage::Read,
            &struct_ty,
            &ClosureUsage::Inline,
            &solver,
        );
        assert_eq!(
            mode,
            CaptureMode::Borrow,
            "non-Dup inline read should Borrow"
        );
    }

    #[test]
    fn test_determine_capture_mode_non_dup_write_inline() {
        let solver = TraitSolver::new();
        let struct_ty = make_non_dup_struct_ty();
        let mode = determine_capture_mode(
            &CaptureUsage::Write,
            &struct_ty,
            &ClosureUsage::Inline,
            &solver,
        );
        assert_eq!(
            mode,
            CaptureMode::BorrowMut,
            "non-Dup inline write should BorrowMut"
        );
    }

    #[test]
    fn test_determine_capture_mode_non_dup_escaping() {
        let solver = TraitSolver::new();
        let struct_ty = make_non_dup_struct_ty();
        let mode = determine_capture_mode(
            &CaptureUsage::Read,
            &struct_ty,
            &ClosureUsage::Escaping,
            &solver,
        );
        assert_eq!(mode, CaptureMode::Move, "non-Dup escaping should Move");
    }

    #[test]
    fn test_closure_usage_inline() {
        let lambda = Expr::Lambda {
            params: vec![],
            body: Box::new(Block {
                stmts: vec![],
                expr: None,
                span: dummy_span(),
            }),
            span: dummy_span(),
        };
        let usage = analyze_closure_usage(&lambda, None);
        assert_eq!(
            usage,
            ClosureUsage::Inline,
            "lambda with no parent should be Inline"
        );
    }

    #[test]
    fn test_closure_usage_escaping_spawn() {
        let lambda = Expr::Lambda {
            params: vec![],
            body: Box::new(Block {
                stmts: vec![],
                expr: None,
                span: dummy_span(),
            }),
            span: dummy_span(),
        };
        let parent = Expr::Spawn {
            body: Box::new(Block {
                stmts: vec![],
                expr: None,
                span: dummy_span(),
            }),
            span: dummy_span(),
        };
        let usage = analyze_closure_usage(&lambda, Some(&parent));
        assert_eq!(
            usage,
            ClosureUsage::Escaping,
            "lambda inside spawn should be Escaping"
        );
    }

    #[test]
    fn test_analyze_captures_no_captures() {
        // lambda: () => 42
        // outer scope: {x, y} — body references nothing
        let block = Block {
            stmts: vec![],
            expr: Some(Box::new(Expr::Lit(
                crate::frontend::core::lexer::tokens::Literal::Int(42),
                dummy_span(),
            ))),
            span: dummy_span(),
        };

        let outer: HashSet<String> = ["x", "y"].iter().map(|s| s.to_string()).collect();
        let captures = analyze_captures(&block, &outer);

        assert_eq!(
            captures.len(),
            0,
            "body referencing no outer variables should have no captures"
        );
    }

    #[test]
    fn test_closure_usage_escaping_return() {
        // Build the full parent expression, then extract a reference to the
        // inner lambda so that std::ptr::eq inside analyze_closure_usage sees
        // the same allocation.
        let parent = Expr::Return(
            Some(Box::new(Expr::Lambda {
                params: vec![],
                body: Box::new(Block {
                    stmts: vec![],
                    expr: None,
                    span: dummy_span(),
                }),
                span: dummy_span(),
            })),
            dummy_span(),
        );
        if let Expr::Return(Some(ret_expr), _) = &parent {
            let lambda_ref = ret_expr.as_ref();
            let usage = analyze_closure_usage(lambda_ref, Some(&parent));
            assert_eq!(
                usage,
                ClosureUsage::Escaping,
                "lambda inside return should be Escaping"
            );
        } else {
            panic!("expected Return expression");
        }
    }

    #[test]
    fn test_analyze_captures_mixed_read_write() {
        // lambda: () => { x = x + 1 }
        // x is both read (right side) and written (left side)
        let block = Block {
            stmts: vec![Stmt {
                kind: StmtKind::Expr(Box::new(Expr::BinOp {
                    op: BinOp::Assign,
                    left: Box::new(Expr::Var("x".to_string(), dummy_span())),
                    right: Box::new(Expr::BinOp {
                        op: BinOp::Add,
                        left: Box::new(Expr::Var("x".to_string(), dummy_span())),
                        right: Box::new(Expr::Lit(
                            crate::frontend::core::lexer::tokens::Literal::Int(1),
                            dummy_span(),
                        )),
                        span: dummy_span(),
                    }),
                    span: dummy_span(),
                })),
                span: dummy_span(),
            }],
            expr: None,
            span: dummy_span(),
        };

        let outer: HashSet<String> = ["x"].iter().map(|s| s.to_string()).collect();
        let captures = analyze_captures(&block, &outer);

        assert_eq!(captures.len(), 1, "only x should be captured");
        assert_eq!(captures[0].name, "x", "captured variable should be x");
        // Write 取优先（因为 written_vars 先处理）
        assert_eq!(
            captures[0].usage,
            CaptureUsage::Write,
            "Write should take priority over Read"
        );
    }

    // ========================================================================
    // Ref 类型捕获模式测试
    // ========================================================================

    #[test]
    fn test_capture_mode_for_immutable_ref() {
        // &T 是 Dup → 应该 Copy
        let solver = TraitSolver::new();
        let ref_ty = MonoType::Ref {
            mutable: false,
            inner: Box::new(MonoType::Int(64)),
        };
        let mode =
            determine_capture_mode(&CaptureUsage::Read, &ref_ty, &ClosureUsage::Inline, &solver);
        assert_eq!(mode, CaptureMode::Copy, "&T is Dup, should Copy");
    }

    #[test]
    fn test_capture_mode_for_immutable_ref_escaping() {
        // &T 是 Dup → 即使逃逸也是 Copy
        let solver = TraitSolver::new();
        let ref_ty = MonoType::Ref {
            mutable: false,
            inner: Box::new(MonoType::Int(64)),
        };
        let mode = determine_capture_mode(
            &CaptureUsage::Read,
            &ref_ty,
            &ClosureUsage::Escaping,
            &solver,
        );
        assert_eq!(
            mode,
            CaptureMode::Copy,
            "&T is Dup, escaping should still Copy"
        );
    }

    #[test]
    fn test_capture_mode_for_mutable_ref_inline_read() {
        // &mut T 不是 Dup，内联读取 → Borrow
        let solver = TraitSolver::new();
        let ref_ty = MonoType::Ref {
            mutable: true,
            inner: Box::new(MonoType::Int(64)),
        };
        let mode =
            determine_capture_mode(&CaptureUsage::Read, &ref_ty, &ClosureUsage::Inline, &solver);
        assert_eq!(mode, CaptureMode::Borrow, "&mut T inline read → Borrow");
    }

    #[test]
    fn test_capture_mode_for_mutable_ref_inline_write() {
        // &mut T 不是 Dup，内联写入 → BorrowMut
        let solver = TraitSolver::new();
        let ref_ty = MonoType::Ref {
            mutable: true,
            inner: Box::new(MonoType::Int(64)),
        };
        let mode = determine_capture_mode(
            &CaptureUsage::Write,
            &ref_ty,
            &ClosureUsage::Inline,
            &solver,
        );
        assert_eq!(
            mode,
            CaptureMode::BorrowMut,
            "&mut T inline write → BorrowMut"
        );
    }

    #[test]
    fn test_capture_mode_for_mutable_ref_escaping() {
        // &mut T 不是 Dup，逃逸 → Move
        let solver = TraitSolver::new();
        let ref_ty = MonoType::Ref {
            mutable: true,
            inner: Box::new(MonoType::Int(64)),
        };
        let mode = determine_capture_mode(
            &CaptureUsage::Read,
            &ref_ty,
            &ClosureUsage::Escaping,
            &solver,
        );
        assert_eq!(mode, CaptureMode::Move, "&mut T escaping → Move");
    }
}
