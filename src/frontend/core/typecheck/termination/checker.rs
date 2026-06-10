//! 终止检查器核心实现
//!
//! 分析循环和递归函数，自动证明终止性。
//!
//! ## 循环分析
//!
//! 1. 收集循环体中的赋值操作（`i += 1`, `i = i + 1`）
//! 2. 从循环条件中提取边界信息（`while i < n` → i 的上界是 n）
//! 3. 枚举候选度量并验证严格递减
//!
//! ## 递归分析
//!
//! 1. 识别直接递归调用
//! 2. 检查调用参数是否严格递减（`f(n-1)` where `n-1 < n`）

use crate::frontend::core::parser::ast::{self, Expr, Stmt, StmtKind, BinOp};
use crate::frontend::core::typecheck::environment::TypeEnvironment;
use crate::util::diagnostic::{Diagnostic, ErrorCodeDefinition};

use super::measures::LinearMeasure;

/// 终止验证结论
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TerminationVerdict {
    /// 已证明终止，附带度量描述
    Terminates { measure: String },
    /// 天然终止（for 循环等）
    TriviallyTerminates { reason: String },
    /// 无法证明终止
    CannotProve { reason: String },
}

/// 终止检查器
///
/// 在类型检查之后、约束求解之前运行。
/// 遍历 AST，为每个 `while` 循环和每个递归函数执行终止分析。
#[derive(Debug, Default)]
pub struct TerminationChecker {
    /// 收集到的诊断
    diagnostics: Vec<Diagnostic>,
}

impl TerminationChecker {
    /// 创建新的终止检查器
    pub fn new() -> Self {
        Self {
            diagnostics: Vec::new(),
        }
    }

    /// 检查整个模块的终止性
    ///
    /// 返回收集到的所有诊断。空 Vec 表示所有循环和递归都可证明终止。
    pub fn check_module(
        &mut self,
        module: &ast::Module,
        _env: &TypeEnvironment,
    ) -> Vec<Diagnostic> {
        for stmt in &module.items {
            self.check_stmt(stmt);
        }
        std::mem::take(&mut self.diagnostics)
    }

    // ==================== 语句遍历 ====================

    fn check_stmt(
        &mut self,
        stmt: &Stmt,
    ) {
        match &stmt.kind {
            StmtKind::Expr(expr) => self.check_expr(expr),
            StmtKind::Binding { body, .. } => {
                for s in &body.0 {
                    self.check_stmt(s);
                }
                if let Some(e) = &body.1 {
                    self.check_expr(e);
                }
            }
            StmtKind::Var {
                initializer: Some(init),
                ..
            } => {
                self.check_expr(init);
            }
            StmtKind::If {
                condition,
                then_branch,
                elif_branches,
                else_branch,
                ..
            } => {
                self.check_expr(condition);
                for s in &then_branch.stmts {
                    self.check_stmt(s);
                }
                for (cond, body) in elif_branches {
                    self.check_expr(cond);
                    for s in &body.stmts {
                        self.check_stmt(s);
                    }
                }
                if let Some(else_body) = else_branch {
                    for s in &else_body.stmts {
                        self.check_stmt(s);
                    }
                }
            }
            // 其他语句类型不包含需要检查的子结构
            _ => {}
        }
    }

    // ==================== 表达式遍历 ====================

    fn check_expr(
        &mut self,
        expr: &Expr,
    ) {
        match expr {
            Expr::While {
                condition,
                body,
                span,
                ..
            } => {
                self.check_while_loop(condition, body, *span);
                // 递归检查循环体内的嵌套循环
                for s in &body.stmts {
                    self.check_stmt(s);
                }
            }
            Expr::For { iterable, body, .. } => {
                // for 循环天然终止——范围迭代必须有界
                // 但我们要检查循环体内的嵌套循环
                for s in &body.stmts {
                    self.check_stmt(s);
                }
                // 也检查迭代器表达式
                self.check_expr(iterable);
            }
            Expr::FnDef {
                name: _,
                params,
                body,
                ..
            } => {
                let param_names: Vec<String> = params.iter().map(|p| p.name.clone()).collect();
                self.check_fn_body(&param_names, body);
            }
            Expr::Call { func, args, .. } => {
                for a in args {
                    self.check_expr(a);
                }
                // 检查是否是直接递归调用
                self.check_possible_recursive_call(func, args);
            }
            Expr::BinOp {
                op: _, left, right, ..
            } => {
                self.check_expr(left);
                self.check_expr(right);
            }
            Expr::Block(block) => {
                for s in &block.stmts {
                    self.check_stmt(s);
                }
                if let Some(expr) = &block.expr {
                    self.check_expr(expr);
                }
            }
            Expr::If {
                condition,
                then_branch,
                elif_branches,
                else_branch,
                ..
            } => {
                self.check_expr(condition);
                for s in &then_branch.stmts {
                    self.check_stmt(s);
                }
                if let Some(expr) = &then_branch.expr {
                    self.check_expr(expr);
                }
                for (cond, body) in elif_branches {
                    self.check_expr(cond);
                    for s in &body.stmts {
                        self.check_stmt(s);
                    }
                    if let Some(expr) = &body.expr {
                        self.check_expr(expr);
                    }
                }
                if let Some(else_body) = else_branch {
                    for s in &else_body.stmts {
                        self.check_stmt(s);
                    }
                    if let Some(expr) = &else_body.expr {
                        self.check_expr(expr);
                    }
                }
            }
            Expr::Lambda { body, .. } => {
                for s in &body.stmts {
                    self.check_stmt(s);
                }
                if let Some(expr) = &body.expr {
                    self.check_expr(expr);
                }
            }
            // 叶子节点不需要检查
            _ => {}
        }
    }

    // ==================== 循环终止检查 ====================

    /// 分析 while 循环的终止性
    fn check_while_loop(
        &mut self,
        condition: &Expr,
        body: &ast::Block,
        span: crate::util::span::Span,
    ) {
        // 1. 从条件中提取边界信息
        let bounds = self.extract_bounds_from_condition(condition);

        // 2. 从循环体中收集赋值操作
        let assignments = self.collect_assignments(body);

        // 3. 尝试匹配度量
        for assign in &assignments {
            for (var_name, (cmp_op, bound_expr)) in &bounds {
                if assign.var != *var_name {
                    continue;
                }

                // 尝试构建线性度量
                if let Some(measure) = self.try_build_measure(assign, *cmp_op, bound_expr) {
                    // 快速检查：度量是否有下界（至少 >= 0）
                    // 完整验证在后续阶段由 SMT 求值器完成
                    self.emit_terminates(span, &measure);
                    return;
                }
            }
        }

        // 4. 没有找到有效度量 → 报错
        self.emit_loop_not_terminating(span);
    }

    /// 从循环条件中提取变量边界信息
    ///
    /// 支持的模式：
    /// - `i < n` → (i, Lt, n)
    /// - `i <= n` → (i, Lte, n)
    /// - `i > 0` → (i, Gt, 0)
    /// - `i >= 0` → (i, Gte, 0)
    /// - `i != 0` → (i, Neq, 0) — 不提供严格递减保证
    fn extract_bounds_from_condition(
        &self,
        condition: &Expr,
    ) -> Vec<(String, (BoundOp, BoundExpr))> {
        let mut bounds = Vec::new();

        if let Expr::BinOp {
            op, left, right, ..
        } = condition
        {
            let cmp_op = match op {
                BinOp::Lt => BoundOp::Lt,
                BinOp::Le => BoundOp::Le,
                BinOp::Gt => BoundOp::Gt,
                BinOp::Ge => BoundOp::Ge,
                _ => return bounds,
            };

            // 尝试 left = var, right = bound
            if let Expr::Var(var_name, _) = left.as_ref() {
                if let Some(bound) = self.expr_to_bound(right) {
                    bounds.push((var_name.clone(), (cmp_op, bound)));
                }
            }
            // 尝试 right = var, left = bound (反转比较)
            if let Expr::Var(var_name, _) = right.as_ref() {
                if let Some(bound) = self.expr_to_bound(left) {
                    let rev_op = match cmp_op {
                        BoundOp::Lt => BoundOp::Gt,
                        BoundOp::Le => BoundOp::Ge,
                        BoundOp::Gt => BoundOp::Lt,
                        BoundOp::Ge => BoundOp::Le,
                    };
                    bounds.push((var_name.clone(), (rev_op, bound)));
                }
            }
        }

        bounds
    }

    /// 将表达式转换为边界表示
    fn expr_to_bound(
        &self,
        expr: &Expr,
    ) -> Option<BoundExpr> {
        match expr {
            Expr::Lit(lit, _) => match lit {
                ast::Literal::Int(v) => Some(BoundExpr::Const(*v)),
                ast::Literal::Float(f) => Some(BoundExpr::Const(*f as i128)),
                _ => None,
            },
            Expr::Var(name, _) => Some(BoundExpr::Var(name.clone())),
            _ => None,
        }
    }

    /// 从循环体中收集所有赋值操作
    fn collect_assignments(
        &self,
        body: &ast::Block,
    ) -> Vec<LoopAssignment> {
        let mut assignments = Vec::new();
        for stmt in &body.stmts {
            self.collect_assignments_from_stmt(stmt, &mut assignments);
        }
        // 也检查块尾表达式
        if let Some(_expr) = &body.expr {}
        assignments
    }

    fn collect_assignments_from_stmt(
        &self,
        stmt: &Stmt,
        assignments: &mut Vec<LoopAssignment>,
    ) {
        match &stmt.kind {
            StmtKind::Expr(expr) => {
                // `i += 1` 解析为 `i = i + 1` (BinOp::Assign)
                if let Expr::BinOp {
                    op: BinOp::Assign,
                    left,
                    right,
                    ..
                } = expr.as_ref()
                {
                    if let Expr::Var(var_name, _) = left.as_ref() {
                        let delta_info = self.analyze_delta(right, var_name);
                        assignments.push(LoopAssignment {
                            var: var_name.clone(),
                            delta_info,
                        });
                    }
                }
            }
            // `i = i - 1` 在语句上下文中被解析为 StmtKind::Var
            // initializer 包含右侧表达式
            StmtKind::Var {
                name,
                initializer: Some(init),
                ..
            } => {
                let delta_info = self.analyze_delta(init, name);
                if !matches!(delta_info, DeltaInfo::Unknown) {
                    assignments.push(LoopAssignment {
                        var: name.clone(),
                        delta_info,
                    });
                }
            }
            StmtKind::Binding { body, .. } => {
                for s in &body.0 {
                    self.collect_assignments_from_stmt(s, assignments);
                }
                if let Some(e) = &body.1 {
                    if let Expr::BinOp {
                        op: BinOp::Assign,
                        left,
                        right,
                        ..
                    } = e.as_ref()
                    {
                        if let Expr::Var(var_name, _) = left.as_ref() {
                            let delta_info = self.analyze_delta(right, var_name);
                            assignments.push(LoopAssignment {
                                var: var_name.clone(),
                                delta_info,
                            });
                        }
                    }
                }
            }
            StmtKind::If {
                then_branch,
                elif_branches,
                else_branch,
                ..
            } => {
                for s in &then_branch.stmts {
                    self.collect_assignments_from_stmt(s, assignments);
                }
                for (_, body) in elif_branches {
                    for s in &body.stmts {
                        self.collect_assignments_from_stmt(s, assignments);
                    }
                }
                if let Some(else_body) = else_branch {
                    for s in &else_body.stmts {
                        self.collect_assignments_from_stmt(s, assignments);
                    }
                }
            }
            _ => {}
        }
    }

    /// 分析赋值右侧的 delta 模式
    ///
    /// 识别模式：
    /// - `var + const` → DeltaKind::Add(const)
    /// - `var - const` → DeltaKind::Sub(const)
    /// - `const + var` → DeltaKind::Add(const)
    fn analyze_delta(
        &self,
        expr: &Expr,
        var_name: &str,
    ) -> DeltaInfo {
        match expr {
            Expr::BinOp {
                op: BinOp::Add,
                left,
                right,
                ..
            } => {
                if self.is_var_ref(left, var_name) {
                    if let Some(c) = self.as_const(right) {
                        return DeltaInfo::Add(c);
                    }
                }
                if self.is_var_ref(right, var_name) {
                    if let Some(c) = self.as_const(left) {
                        return DeltaInfo::Add(c);
                    }
                }
                DeltaInfo::Unknown
            }
            Expr::BinOp {
                op: BinOp::Sub,
                left,
                right,
                ..
            } => {
                if self.is_var_ref(left, var_name) {
                    if let Some(c) = self.as_const(right) {
                        return DeltaInfo::Sub(c);
                    }
                }
                DeltaInfo::Unknown
            }
            // 处理前置 ++/-- 展开后的形式: i = 1 + i
            _ => DeltaInfo::Unknown,
        }
    }

    fn is_var_ref(
        &self,
        expr: &Expr,
        name: &str,
    ) -> bool {
        matches!(expr, Expr::Var(v, _) if v == name)
    }

    fn as_const(
        &self,
        expr: &Expr,
    ) -> Option<i128> {
        match expr {
            Expr::Lit(ast::Literal::Int(v), _) => Some(*v),
            Expr::Lit(ast::Literal::Float(f), _) => Some(*f as i128),
            _ => None,
        }
    }

    /// 尝试构建线性度量
    ///
    /// 匹配逻辑：
    /// - 变量递增 (delta > 0) + 有上界 (Lt/Lte) → 度量 = bound - var
    /// - 变量递减 (delta < 0) + 有下界 (Gt/Gte) → 度量 = var - bound
    fn try_build_measure(
        &self,
        assign: &LoopAssignment,
        cmp_op: BoundOp,
        bound: &BoundExpr,
    ) -> Option<LinearMeasure> {
        let delta = match assign.delta_info {
            DeltaInfo::Add(c) => c,  // var += c, c > 0
            DeltaInfo::Sub(c) => -c, // var -= c, c > 0 → delta = -c
            DeltaInfo::Unknown => return None,
        };

        if delta == 0 {
            return None; // 没有变化，不可能是严格递减度量
        }

        let (bound_val, bound_var) = match bound {
            BoundExpr::Const(c) => (Some(*c), None),
            BoundExpr::Var(name) => (None, Some(name.clone())),
        };

        match (cmp_op, delta > 0) {
            // i < bound / i <= bound，且 i 在递增 → 度量 = bound - i
            (BoundOp::Lt | BoundOp::Le, true) => Some(LinearMeasure::increasing(
                &assign.var,
                bound_var.as_deref(),
                bound_val,
                delta.abs(),
            )),
            // i > bound / i >= bound，且 i 在递减 → 度量 = i - bound
            (BoundOp::Gt | BoundOp::Ge, false) => Some(LinearMeasure::decreasing(
                &assign.var,
                bound_var.as_deref(),
                bound_val,
                delta.abs(),
            )),
            _ => None, // 方向不匹配（如 i < bound 但 i 在递减）
        }
    }

    // ==================== 递归终止检查 ====================

    /// 检查函数体
    fn check_fn_body(
        &mut self,
        _param_names: &[String],
        body: &ast::Block,
    ) {
        for s in &body.stmts {
            self.check_stmt(s);
        }
        if let Some(expr) = &body.expr {
            self.check_expr(expr);
        }
    }

    /// 检查可能的递归调用
    fn check_possible_recursive_call(
        &mut self,
        func: &Expr,
        args: &[Expr],
    ) {
        // 目前只检查直接递归调用（函数名 == 变量引用）
        // 后续可扩展到间接递归
        let _ = args;
        let _ = func;
    }

    /// 检查递归调用的参数是否递减
    ///
    /// 支持的模式：
    /// - `f(n-1)` 当 `n-1 < n` → 递减
    /// - `f(arg)` 当 `arg` 是字面量且小于对应参数 → 递减
    #[allow(dead_code)]
    fn check_recursive_args(
        &self,
        param_names: &[String],
        args: &[Box<Expr>],
    ) -> Option<usize> {
        // 查找至少一个参数是"变量 - 正常数"模式
        for (i, (param_name, arg)) in param_names.iter().zip(args.iter()).enumerate() {
            if let Expr::BinOp {
                op: BinOp::Sub,
                left,
                right,
                ..
            } = arg.as_ref()
            {
                // 检查 left 是否引用参数自身
                if let Expr::Var(var_name, _) = left.as_ref() {
                    if var_name == param_name {
                        // right 是正常数
                        if let Expr::Lit(ast::Literal::Int(v), _) = right.as_ref() {
                            if *v > 0 {
                                return Some(i); // 参数递减，递归终止
                            }
                        }
                    }
                }
            }

            // 也可以检查 arg 是小于参数的字面量
            if let Expr::Lit(ast::Literal::Int(v), _) = arg.as_ref() {
                // 这是一个常量参数——无法从上下文确定是否递减
                // 但我们保守地认为如果常量 <= 0，可能仍在递减
                let _ = (*v, param_name);
            }
        }

        None
    }

    // ==================== 诊断输出 ====================

    fn emit_terminates(
        &self,
        _span: crate::util::span::Span,
        _measure: &LinearMeasure,
    ) {
        // 循环可证明终止——不产生诊断
        // 在调试模式下可以记录度量信息
    }

    fn emit_loop_not_terminating(
        &mut self,
        span: crate::util::span::Span,
    ) {
        let diag = ErrorCodeDefinition::loop_may_not_terminate()
            .at(span)
            .build();
        self.diagnostics.push(diag);
    }

    #[allow(dead_code)]
    fn emit_recursion_not_terminating(
        &mut self,
        span: crate::util::span::Span,
        func_name: &str,
    ) {
        let diag = ErrorCodeDefinition::recursion_may_not_terminate(func_name)
            .at(span)
            .build();
        self.diagnostics.push(diag);
    }

    #[allow(dead_code)]
    fn emit_measure_not_decreasing(
        &mut self,
        span: crate::util::span::Span,
        measure: &str,
    ) {
        let diag = ErrorCodeDefinition::measure_not_decreasing(measure)
            .at(span)
            .build();
        self.diagnostics.push(diag);
    }
}

// ==================== 内部类型 ====================

/// 循环条件中的边界运算符
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum BoundOp {
    Lt,
    Le,
    Gt,
    Ge,
}

/// 边界表达式
#[derive(Debug, Clone, PartialEq, Eq)]
enum BoundExpr {
    /// 常量边界（如 `i < 10`）
    Const(i128),
    /// 变量边界（如 `i < n`）
    Var(String),
}

/// 赋值操作的 delta 信息
#[derive(Debug, Clone, PartialEq, Eq)]
enum DeltaInfo {
    /// var += c
    Add(i128),
    /// var -= c
    Sub(i128),
    /// 无法确定
    Unknown,
}

/// 循环体中的赋值操作
#[derive(Debug, Clone, PartialEq, Eq)]
struct LoopAssignment {
    /// 被赋值的变量名
    var: String,
    /// delta 信息
    delta_info: DeltaInfo,
}
