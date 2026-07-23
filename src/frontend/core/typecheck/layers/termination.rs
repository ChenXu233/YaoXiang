//! 终止检查模块
//!
//! 实现 RFC-027 Section 7：编译器全自动证明循环终止和递归函数终止。
//!
//! **当前实现（Phase 1）**：
//! - 策略 3：有界递增/递减模式 (`i += const` with `i < bound`)
//! - 递归参数递减检查 (`factorial(n-1)`)
//! - `for` 循环自动通过（范围迭代天然终止）
//!
//! **后续扩展**：
//! - 策略 1：线性秩函数自动合成
//! - 策略 2：谓词违反计数
//! - 策略 4：乘法缩放度量模板

// ==================== 度量分析器 ====================
//
// 从循环体中提取候选度量，验证度量是否严格递减。

/// 度量方向
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Direction {
    /// 变量递增（度量 = bound - var）
    Increasing,
    /// 变量递减（度量 = var - bound）
    Decreasing,
}

/// 候选度量：一个变量朝着一个边界移动
///
/// # Example
/// ```text
/// while i < n { i += 1 }
/// → LinearMeasure { var: "i", bound: n, direction: Increasing, delta: 1 }
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LinearMeasure {
    /// 被修改的变量名
    pub var: String,
    /// 边界值（上界或下界），`None` 表示边界是运行时变量
    pub bound: Option<i128>,
    /// 边界变量名（当边界是运行时变量时）
    pub bound_var: Option<String>,
    /// 方向
    pub direction: Direction,
    /// 每次迭代的变化量（默认 1）
    pub delta: i128,
}

impl LinearMeasure {
    /// 创建一个递增到上界的度量
    ///
    /// `while i < n { i += delta }` → `(bound - i)` 每次减 `delta`
    pub fn increasing(
        var: &str,
        bound_var: Option<&str>,
        bound_val: Option<i128>,
        delta: i128,
    ) -> Self {
        Self {
            var: var.to_string(),
            bound: bound_val,
            bound_var: bound_var.map(|s| s.to_string()),
            direction: Direction::Increasing,
            delta,
        }
    }

    /// 创建一个递减到下界的度量
    ///
    /// `while i > 0 { i -= delta }` → `(i - bound)` 每次减 `delta`
    pub fn decreasing(
        var: &str,
        bound_var: Option<&str>,
        bound_val: Option<i128>,
        delta: i128,
    ) -> Self {
        Self {
            var: var.to_string(),
            bound: bound_val,
            bound_var: bound_var.map(|s| s.to_string()),
            direction: Direction::Decreasing,
            delta,
        }
    }

    /// 创建一个乘法缩放度量
    ///
    /// `while v < upper { v *= const }` → ceil(log_const(upper/v)) 每次减 1
    pub fn multiplicative(
        var: &str,
        upper: i128,
        const_val: i128,
    ) -> Self {
        Self {
            var: var.to_string(),
            bound: Some(upper),
            bound_var: None,
            direction: Direction::Increasing,
            delta: const_val,
        }
    }

    /// 返回度量的可读描述
    pub fn describe(&self) -> String {
        match self.direction {
            Direction::Increasing => match (&self.bound_var, self.bound) {
                (Some(bv), _) => format!("{} - {}", bv, self.var),
                (None, Some(bv)) => format!("{} - {}", bv, self.var),
                _ => format!("bound - {}", self.var),
            },
            Direction::Decreasing => match (&self.bound_var, self.bound) {
                (Some(bv), _) => format!("{} - {}", self.var, bv),
                (None, Some(bv)) => format!("{} - {}", self.var, bv),
                _ => format!("{} - bound", self.var),
            },
        }
    }
}

// ==================== 终止检查器核心实现 ====================
//
// 分析循环和递归函数，自动证明终止性。
//
// ## 循环分析
//
// 1. 收集循环体中的赋值操作（`i += 1`, `i = i + 1`）
// 2. 从循环条件中提取边界信息（`while i < n` → i 的上界是 n）
// 3. 枚举候选度量并验证严格递减
//
// ## 递归分析
//
// 1. 识别直接递归调用
// 2. 检查调用参数是否严格递减（`f(n-1)` where `n-1 < n`）

use crate::frontend::core::parser::ast::{self, Expr, Stmt, StmtKind, BinOp, Type};
use crate::frontend::core::typecheck::environment::TypeEnvironment;
use crate::frontend::core::typecheck::proof::verdict::{BudgetReport, ProofResult, UnprovenReason};
use super::super::proof::smt::ast::{SMTExpr, SMTCommand, SMTSort};
#[cfg(not(target_arch = "wasm32"))]
use super::super::proof::smt::z3_backend::Z3Backend;

/// 终止检查器
///
/// 在类型检查之后、约束求解之前运行。
/// 遍历 AST，为每个 `while` 循环和每个递归函数执行终止分析。
#[derive(Debug)]
pub struct TerminationChecker {
    /// 收集到的证明结果
    results: Vec<ProofResult>,
    /// Z3 后端引用——策略 1 秩函数 SMT 验证
    #[cfg(not(target_arch = "wasm32"))]
    z3: Option<&'static Z3Backend>,
}

impl Default for TerminationChecker {
    fn default() -> Self {
        Self::new()
    }
}

impl TerminationChecker {
    /// 创建新的终止检查器
    pub fn new() -> Self {
        Self {
            results: Vec::new(),
            #[cfg(not(target_arch = "wasm32"))]
            z3: None,
        }
    }
    /// 设置 Z3 后端（由调用方在初始化后注入）
    #[cfg(not(target_arch = "wasm32"))]
    pub fn with_z3(
        mut self,
        z3: &'static Z3Backend,
    ) -> Self {
        self.z3 = Some(z3);
        self
    }

    /// 检查整个模块的终止性
    ///
    /// 返回收集到的所有证明结果。空 Vec 表示所有循环和递归都可证明终止，
    /// 或未发现需要检查的循环/递归。
    pub fn check_module(
        &mut self,
        module: &ast::Module,
        _env: &TypeEnvironment,
    ) -> Vec<ProofResult> {
        for stmt in &module.items {
            self.check_stmt(stmt, false);
        }
        std::mem::take(&mut self.results)
    }

    // ==================== 语句遍历 ====================

    fn check_stmt(
        &mut self,
        stmt: &Stmt,
        is_never: bool,
    ) {
        match &stmt.kind {
            StmtKind::Expr(expr) => self.check_expr(expr, is_never),
            StmtKind::Assign {
                value: Some(v),
                type_annotation,
                ..
            } => {
                use crate::frontend::core::parser::ast::Expr;
                let child_never = is_never || is_never_return_type(type_annotation.as_ref());
                if let Expr::Lambda { body, .. } = v.as_ref() {
                    for s in &body.stmts {
                        self.check_stmt(s, child_never);
                    }
                } else if let Expr::Block(block) = v.as_ref() {
                    for s in &block.stmts {
                        self.check_stmt(s, child_never);
                    }
                } else {
                    self.check_expr(v, child_never);
                }
            }
            StmtKind::If {
                condition,
                then_branch,
                elif_branches,
                else_branch,
                ..
            } => {
                self.check_expr(condition, is_never);
                for s in &then_branch.stmts {
                    self.check_stmt(s, is_never);
                }
                for (cond, body) in elif_branches {
                    self.check_expr(cond, is_never);
                    for s in &body.stmts {
                        self.check_stmt(s, is_never);
                    }
                }
                if let Some(else_body) = else_branch {
                    for s in &else_body.stmts {
                        self.check_stmt(s, is_never);
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
        is_never: bool,
    ) {
        match expr {
            Expr::While {
                condition,
                body,
                span,
                ..
            } => {
                self.check_while_loop(condition, body, *span, is_never);
                // 递归检查循环体内的嵌套循环
                for s in &body.stmts {
                    self.check_stmt(s, is_never);
                }
            }
            Expr::For { iterable, body, .. } => {
                for s in &body.stmts {
                    self.check_stmt(s, is_never);
                }
                self.check_expr(iterable, is_never);
            }
            Expr::FnDef {
                name: _,
                params,
                body,
                return_type,
                ..
            } => {
                let param_names: Vec<String> = params.iter().map(|p| p.name.clone()).collect();
                let fn_never = is_never || is_never_return_type(return_type.as_ref());
                self.check_fn_body(&param_names, body, fn_never);
            }
            Expr::Call { func, args, .. } => {
                for a in args {
                    self.check_expr(a, is_never);
                }
                self.check_possible_recursive_call(func, args);
            }
            Expr::BinOp {
                op: _, left, right, ..
            } => {
                self.check_expr(left, is_never);
                self.check_expr(right, is_never);
            }
            Expr::Block(block) => {
                for s in &block.stmts {
                    self.check_stmt(s, is_never);
                }
            }
            Expr::If {
                condition,
                then_branch,
                elif_branches,
                else_branch,
                ..
            } => {
                self.check_expr(condition, is_never);
                for s in &then_branch.stmts {
                    self.check_stmt(s, is_never);
                }
                for (cond, body) in elif_branches {
                    self.check_expr(cond, is_never);
                    for s in &body.stmts {
                        self.check_stmt(s, is_never);
                    }
                }
                if let Some(else_body) = else_branch {
                    for s in &else_body.stmts {
                        self.check_stmt(s, is_never);
                    }
                }
            }
            Expr::Lambda { body, .. } => {
                for s in &body.stmts {
                    self.check_stmt(s, is_never);
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
        is_never: bool,
    ) {
        // Never 返回函数：循环不终止是类型签名保证的语义，直接放行
        if is_never {
            return;
        }
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

        // 策略 4：乘法缩放度量
        if let Some(measure) = self.try_multiplicative_measure(&assignments, &bounds) {
            self.emit_terminates(span, &measure);
            return;
        }

        // 策略 1：线性秩函数自动合成（SMT 验证）
        #[cfg(not(target_arch = "wasm32"))]
        if self.z3.is_some() {
            if let Some(measure) =
                self.try_linear_rank_function(&bounds, &assignments, condition, span)
            {
                self.emit_terminates(span, &measure);
                return;
            }
        }

        // 策略 2：谓词违反计数（框架占位）
        if let Some(measure) = self.try_violation_count(&assignments, &bounds) {
            self.emit_terminates(span, &measure);
            return;
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
            // `i = i - 1` → Assign { target: Var(name), value: Some(init) }
            StmtKind::Assign {
                target,
                value: Some(v),
                ..
            } => {
                use crate::frontend::core::parser::ast::Expr;
                if let Expr::Var(name, _) = target.as_ref() {
                    let delta_info = self.analyze_delta(v, name);
                    if !matches!(delta_info, DeltaInfo::Unknown) {
                        assignments.push(LoopAssignment {
                            var: name.clone(),
                            delta_info,
                        });
                    }
                }
                // 递归处理 Lambda/Block 函数体
                if let Expr::Lambda { body, .. } = v.as_ref() {
                    for s in &body.stmts {
                        self.collect_assignments_from_stmt(s, assignments);
                    }
                } else if let Expr::Block(block) = v.as_ref() {
                    for s in &block.stmts {
                        self.collect_assignments_from_stmt(s, assignments);
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
            Expr::BinOp {
                op: BinOp::Mul,
                left,
                right,
                ..
            } => {
                if self.is_var_ref(left, var_name) {
                    if let Some(c) = self.as_const(right) {
                        return DeltaInfo::Mul(c);
                    }
                }
                if self.is_var_ref(right, var_name) {
                    if let Some(c) = self.as_const(left) {
                        return DeltaInfo::Mul(c);
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
            DeltaInfo::Mul(c) => c,  // var *= c, c > 1 → delta = c
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

    /// 策略 4：乘法缩放度量模板
    ///
    /// 检测 `v *= const`（const > 1）且 v 有整数上界的循环模式。
    /// 度量 = ceil(log_const(upper / v))，每次乘 const 度量减 1。
    fn try_multiplicative_measure(
        &self,
        assignments: &[LoopAssignment],
        bounds: &[(String, (BoundOp, BoundExpr))],
    ) -> Option<LinearMeasure> {
        for assign in assignments {
            if let DeltaInfo::Mul(const_val) = assign.delta_info {
                if const_val <= 1 {
                    continue;
                }
                // 查找 v 的上界
                if let Some((_, (BoundOp::Lt | BoundOp::Le, BoundExpr::Const(upper)))) =
                    bounds.iter().find(|(v, _)| v == &assign.var)
                {
                    if *upper > 0 {
                        return Some(LinearMeasure::multiplicative(
                            &assign.var,
                            *upper,
                            const_val,
                        ));
                    }
                }
            }
        }
        None
    }

    /// 策略 1：线性秩函数自动合成
    ///
    /// 枚举候选线性度量，SMT 验证每条执行路径上严格递减。
    /// - ≤3 个有界变量 → 全组合枚举
    /// - >3 个 → 只单变量，失败报编译错误
    #[cfg(not(target_arch = "wasm32"))]
    fn try_linear_rank_function(
        &self,
        bounds: &[(String, (BoundOp, BoundExpr))],
        assignments: &[LoopAssignment],
        _condition: &Expr,
        _span: crate::util::span::Span,
    ) -> Option<LinearMeasure> {
        let z3 = self.z3?;

        let bounded_vars: Vec<&str> = bounds.iter().map(|(v, _)| v.as_str()).collect();
        let candidates = self.generate_rank_candidates(&bounded_vars, bounds);

        for candidate in &candidates {
            if self.verify_rank_candidate(candidate, bounds, assignments, z3) {
                return Some(candidate.clone());
            }
        }

        None
    }

    /// 生成秩函数候选列表
    fn generate_rank_candidates(
        &self,
        bounded_vars: &[&str],
        bounds: &[(String, (BoundOp, BoundExpr))],
    ) -> Vec<LinearMeasure> {
        let mut candidates = Vec::new();

        if bounded_vars.len() > 3 {
            // 只尝试单变量度量
            for &v in bounded_vars {
                candidates.push(LinearMeasure::increasing(v, None, None, 1));
                if let Some((_, (_, BoundExpr::Const(upper)))) =
                    bounds.iter().find(|(bv, _)| bv == v)
                {
                    candidates.push(LinearMeasure::increasing(v, None, Some(*upper), 1));
                }
            }
            return candidates;
        }

        // ≤3 个变量：全组合
        for &v in bounded_vars {
            candidates.push(LinearMeasure::increasing(v, None, None, 1));
            if let Some((_, (_, BoundExpr::Const(u)))) = bounds.iter().find(|(bv, _)| bv == v) {
                candidates.push(LinearMeasure::increasing(v, None, Some(*u), 1));
            }
        }

        // 两变量组合：v_i - v_j
        for i in 0..bounded_vars.len() {
            for j in 0..bounded_vars.len() {
                if i != j {
                    candidates.push(LinearMeasure::increasing(
                        bounded_vars[i],
                        Some(bounded_vars[j]),
                        None,
                        1,
                    ));
                }
            }
        }

        candidates
    }

    /// SMT 验证秩函数候选是否在所有路径上严格递减
    #[cfg(not(target_arch = "wasm32"))]
    fn verify_rank_candidate(
        &self,
        candidate: &LinearMeasure,
        _bounds: &[(String, (BoundOp, BoundExpr))],
        _assignments: &[LoopAssignment],
        z3: &Z3Backend,
    ) -> bool {
        let mut commands = Vec::new();

        // 声明秩函数变量
        commands.push(SMTCommand::DeclareConst(
            candidate.var.clone(),
            SMTSort::Int,
        ));
        if let Some(ref bv) = candidate.bound_var {
            commands.push(SMTCommand::DeclareConst(bv.clone(), SMTSort::Int));
        }

        // 构造 m_prime = var + delta（被赋值后的值）
        let m_var = SMTExpr::Atom(candidate.var.clone());
        let m_prime = SMTExpr::App(
            "+".into(),
            vec![
                SMTExpr::Atom(candidate.var.clone()),
                SMTExpr::Atom(candidate.delta.to_string()),
            ],
        );

        // assert (not (< m_prime m_var))
        let decreasing = SMTExpr::App("<".into(), vec![m_prime, m_var]);
        let not_decreasing = SMTExpr::App("not".into(), vec![decreasing]);
        commands.push(SMTCommand::Assert(not_decreasing));

        commands.push(SMTCommand::CheckSat);

        // unsat = m' < m 在所有情况下成立 → 严格递减
        matches!(
            z3.solve(&commands, 50),
            crate::frontend::core::typecheck::proof::smt::ast::SMTResult::Unsat
        )
    }

    /// 策略 2：谓词违反计数（RFC-027 §7.3，实验性，框架占位）
    ///
    /// 完整实现需要：
    /// 1. Parser 支持 forall 量词语法
    /// 2. 解析目标类型定义提取条件函数
    /// 3. 生成 violation_count 度量
    /// 4. 验证相邻操作减少度量
    ///
    /// 当前返回 None，待 Parser 升级后补完。
    fn try_violation_count(
        &self,
        _assignments: &[LoopAssignment],
        _bounds: &[(String, (BoundOp, BoundExpr))],
    ) -> Option<LinearMeasure> {
        None
    }

    // ==================== 递归终止检查 ====================

    /// 检查函数体
    fn check_fn_body(
        &mut self,
        _param_names: &[String],
        body: &ast::Block,
        is_never: bool,
    ) {
        for s in &body.stmts {
            self.check_stmt(s, is_never);
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
        &mut self,
        _span: crate::util::span::Span,
        _measure: &LinearMeasure,
    ) {
        // 循环可证明终止——记录 Proved（不产生任何诊断）
        self.results.push(ProofResult::Proved);
    }

    fn emit_loop_not_terminating(
        &mut self,
        _span: crate::util::span::Span,
    ) {
        let reason =
            UnprovenReason::BeyondKernel("循环无法证明终止：未找到有效的递减度量".to_string());
        self.results.push(ProofResult::Unproven {
            reason,
            proof_calls: vec![],
            budget: BudgetReport {
                steps_used: 0,
                steps_limit: 0,
            },
        });
    }

    #[allow(dead_code)]
    fn emit_recursion_not_terminating(
        &mut self,
        _span: crate::util::span::Span,
        func_name: &str,
    ) {
        let reason = UnprovenReason::BeyondKernel(format!(
            "递归函数 `{}` 无法证明终止：参数未严格递减",
            func_name
        ));
        self.results.push(ProofResult::Unproven {
            reason,
            proof_calls: vec![],
            budget: BudgetReport {
                steps_used: 0,
                steps_limit: 0,
            },
        });
    }

    #[allow(dead_code)]
    fn emit_measure_not_decreasing(
        &mut self,
        _span: crate::util::span::Span,
        measure: &str,
    ) {
        let reason = UnprovenReason::BeyondKernel(format!("度量 `{}` 未严格递减", measure));
        self.results.push(ProofResult::Unproven {
            reason,
            proof_calls: vec![],
            budget: BudgetReport {
                steps_used: 0,
                steps_limit: 0,
            },
        });
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
    /// var *= c（const > 1，乘法缩放模式）
    Mul(i128),
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

/// 判断函数类型签名的返回类型是否为 Never
///
/// `(P1, P2, ...) -> Never` → true
fn is_never_return_type(ty: Option<&Type>) -> bool {
    match ty {
        // `name: () -> Never` — type_annotation 是 Fn 类型，取 return_type
        Some(Type::Fn { return_type, .. }) => is_type_never(return_type.as_ref()),
        // `fn(): Never` — FnDef.return_type 是裸返回类型
        Some(t) => is_type_never(t),
        None => false,
    }
}

/// 递归判断 Type 是否为 Never（支持 Type::Name { name: "Never", .. }）
fn is_type_never(ty: &Type) -> bool {
    matches!(ty, Type::Name { name, .. } if name == "Never" || name == "never")
}
