use super::*;

impl TypeChecker {
    /// 常量折叠：表达式在字面量值环境下求成具体值（精化值环境的数据源）
    ///
    /// 复用 convert_expr_to_const_expr + Evaluator——与 check_predicate
    /// 第 1 级同一台求值器，折叠语义与约束判定严格一致。转换/求值失败
    /// （变量值未知、调用等非内核形态）→ None：值回到「未知」，走
    /// 「缺值不送检」路径，绝不用陈旧值凑数。
    fn fold_value(
        ctx: &crate::frontend::core::typecheck::proof::context::ProofContext<'_>,
        expr: &Expr,
        values: &HashMap<String, ConstValue>,
    ) -> Option<ConstValue> {
        let const_expr = convert_expr_to_const_expr(expr)?;
        let mut evaluator = crate::frontend::core::types::eval::evaluator::Evaluator::new(
            ctx.env,
            &ctx.budget,
            &ctx.dep_env,
        );
        evaluator.eval_expr(&const_expr, values).ok()
    }

    /// 条件在值环境下可判定为 Bool（if 链守卫裁剪的判据）
    fn fold_bool(
        ctx: &crate::frontend::core::typecheck::proof::context::ProofContext<'_>,
        expr: &Expr,
        values: &HashMap<String, ConstValue>,
    ) -> Option<bool> {
        match Self::fold_value(ctx, expr, values)? {
            ConstValue::Bool(b) => Some(b),
            _ => None,
        }
    }

    /// Phase 2.5：精化类型绑定与依赖重验证（RFC-027 §6.1，#379 微重构）
    ///
    /// 单遍顺序走查，按 [`RefinedScopeUnit`] 切分作用域（模块层一个单元，
    /// 每个函数体一个新单元）。声明序即验证序：赋值点只重验证**当时已声明**
    /// 的依赖者——旧实现两遍全模块共享一张名字键图，跨函数同名误报与
    /// 「赋值先于声明」误触发都源于此。
    ///
    /// 初始校验与重验证统一经 check_predicate 送证明管道：
    /// - Proved → 通过
    /// - Disproved → E2030，反例进诊断
    /// - Unproven{calls} → 上抛 proof_calls，由 pipeline 编译期执行；
    ///   空调用集 → E2031（RFC-027 §9：Unproven 不得静默放行）
    /// - 约束自由变量缺已知值时**不送检**：缺值送 SMT 会退化成「对所有值
    ///   成立」的全称检查，产生伪反例。值环境由常量折叠供给（`n = n + 1`
    ///   以旧值折叠出新值）；折叠不出的（函数参数、I/O 等运行期值）是
    ///   当前机制的已知边界——精确处刑需假设注入 + 路径条件（RFC-027 §6
    ///   Floyd-Hoare 全貌，随数据流推理落地，#377 同盘）
    pub(super) fn collect_refined_binding_checks(
        &mut self,
        module: &Module,
        proof_calls: &mut Vec<crate::frontend::core::typecheck::proof::verdict::ProofFunctionCall>,
    ) {
        let mut shared_ctx =
            crate::frontend::core::typecheck::proof::context::ProofContext::new(&self.env);
        // #263：shared_ctx 持有 &self.env，精化解析诊断先汇入 sink，
        // 待 shared_ctx 释放后再写入 env.errors
        let mut refined_diags = Vec::new();
        {
            let mut ctx = RefinedWalkCtx {
                shared_ctx: &mut shared_ctx,
                proof_calls,
                diags: &mut refined_diags,
                fn_return_refined: None,
                fn_return_binder: None,
                fn_name: None,
            };
            let mut module_unit = RefinedScopeUnit::new();
            self.refined_walk_stmts(&module.items, &mut module_unit, &mut ctx);
        }

        // #263：精化解析诊断汇入（shared_ctx 已不再被使用，借用结束）
        self.env.errors.extend_errors(refined_diags);
    }

    /// 精化走查：一个语句列表 = 一个词法块。
    ///
    /// 块内注册的精化 dependant 在块退出时从依赖图移除（块局部变量出块即亡）；
    /// 函数体/模块层作为最外层块，单元随走查结束整体丢弃，移除只是兜底。
    fn refined_walk_stmts(
        &self,
        stmts: &[crate::frontend::core::parser::ast::Stmt],
        unit: &mut RefinedScopeUnit,
        ctx: &mut RefinedWalkCtx<'_, '_>,
    ) {
        let mut registered: Vec<String> = Vec::new();
        for stmt in stmts {
            // #324：嵌套语句逐条挂当前 span，诊断落到真实语句行
            let _span_guard = crate::util::diagnostic::push_current_span(stmt.span);
            registered.extend(self.refined_walk_stmt(stmt, unit, ctx));
        }
        for name in &registered {
            unit.deps.remove_dependant(name);
        }
    }

    /// 精化走查：单条语句。返回本语句注册的精化 dependant 名（供块退出清理）。
    fn refined_walk_stmt(
        &self,
        stmt: &crate::frontend::core::parser::ast::Stmt,
        unit: &mut RefinedScopeUnit,
        ctx: &mut RefinedWalkCtx<'_, '_>,
    ) -> Vec<String> {
        use crate::frontend::core::parser::ast::{Expr, StmtKind};

        match &stmt.kind {
            // 注解绑定 `x: T = v`：解析精化 → 注册依赖 + 初始校验 → 记录字面量值
            StmtKind::Assign {
                target,
                type_annotation: Some(type_ann),
                signature_params,
                value,
                ..
            } => {
                let mut registered = Vec::new();
                if let Expr::Var(name, _) = target.as_ref() {
                    let mono_ty = MonoType::from(type_ann.clone());
                    let resolved_ty = self.resolve_type_annotation(&mono_ty, ctx.diags);
                    // 初始值常量折叠（对前置值环境求值）——初始校验与值追踪共用
                    let init_value = value
                        .as_deref()
                        .and_then(|v| Self::fold_value(&*ctx.shared_ctx, v, &unit.values));
                    if matches!(resolved_ty, MonoType::Refined { .. })
                        && !constraint_is_terminates(&resolved_ty)
                    {
                        unit.deps.register_refined(name, &resolved_ty);
                        registered.push(name.clone());
                        // 初始校验：bindings 代入 name 自身的初始值（若可折叠）
                        self.revalidate_refined(
                            name,
                            name,
                            &resolved_ty,
                            (name, init_value.clone()),
                            &unit.values,
                            ctx,
                        );
                    }
                    unit.track(name, init_value.clone());
                }
                // 绑定值是函数体 → 嵌套函数体独立单元走查（判定与
                // `block_binding_is_function` 同源，#363：注解是推荐的函数写法）
                if crate::frontend::core::parser::ast::Expr::block_binding_is_function(
                    Some(type_ann),
                    value.as_deref(),
                ) {
                    if let Some(body) = extract_fn_body(value.as_deref()) {
                        let fn_name = match target.as_ref() {
                            Expr::Var(n, _) => Some(n.as_str()),
                            _ => None,
                        };
                        // RFC-027 §3：返回位精化的 binder 名与谓词实参只在 **AST** 里
                        // 无损（`MonoType` 已把符号实参降级成占位），故在此按声明解析，
                        // 走查只消费结果。
                        let ret_refinement =
                            self.resolve_return_refinement(type_ann, signature_params, ctx.diags);
                        self.refined_walk_fn_body(body, fn_name, ret_refinement, ctx);
                    }
                }
                // RFC-027 §3.4：绑定初始值若为调用，校验其实参
                if let Some(v) = value.as_deref() {
                    self.check_call_arg_refinements(v, unit, ctx);
                }
                registered
            }
            // 无注解赋值/绑定 `x = v`：更新值环境 + 重验证依赖者
            StmtKind::Assign {
                target,
                value: Some(rhs),
                ..
            } => {
                if let Expr::Var(name, _) = target.as_ref() {
                    // RHS 对**前置**值环境折叠：`n = n + 1` 以旧 n 值算出新值
                    let new_value = Self::fold_value(&*ctx.shared_ctx, rhs, &unit.values);
                    // RFC-027 §6.1：x 变更 → 对每个依赖 x 的变量生成 VC 重验证
                    for dependant in unit.deps.affected_by(name) {
                        let dependant = dependant.to_string();
                        if let Some(refined) = unit.deps.constraint_of(&dependant) {
                            self.revalidate_refined(
                                name,
                                &dependant,
                                refined,
                                (name, new_value.clone()),
                                &unit.values,
                                ctx,
                            );
                        }
                    }
                    unit.track(name, new_value);
                    // 变量重赋值 ⇒ 依赖它的假设作废（否则用旧值判新值）
                    ctx.shared_ctx.assumptions.kill(name);
                }
                // 复杂赋值目标（索引/解构）：v1 不建模对依赖者的影响（与旧实现一致）
                // RHS 内嵌函数体：`f = () => {...}` 同样独立单元
                match rhs.as_ref() {
                    Expr::Lambda { body, .. } => {
                        // 无注解绑定：无签名可依，后置条件与形参前置条件皆空
                        self.refined_walk_fn_body(body, None, None, ctx);
                    }
                    // 无注解的块值在当前作用域顺序执行：同单元走查
                    Expr::Block(block) => {
                        self.refined_walk_stmts(&block.stmts, unit, ctx);
                    }
                    // RFC-027 §3.4：调用点的形参精化义务
                    Expr::Call { .. } => {
                        self.check_call_arg_refinements(rhs, unit, ctx);
                    }
                    _ => {}
                }
                Vec::new()
            }
            StmtKind::If {
                condition,
                then_branch,
                else_if_branches,
                else_branch,
                ..
            } => {
                // 分支敏感 + 守卫裁剪：条件在当前值环境下可判定时只走会执行
                // 的分支（不可达分支不产生验证义务、不并进汇合）；不可判定时
                // 保守展开。各路径从同一前置值环境出发，汇合取等值交集——
                // 仅部分分支改写（或声明）的变量出块即值未知
                let paths = self.refined_walk_if_paths(
                    condition,
                    &then_branch.stmts,
                    else_if_branches,
                    else_branch.as_deref(),
                    unit,
                    ctx,
                );
                unit.values = meet_value_paths(paths);
                Vec::new()
            }
            StmtKind::For { var, body, .. } => {
                self.refined_walk_loop_body(Some(var), body, unit, ctx);
                Vec::new()
            }
            StmtKind::Expr(expr) => {
                match expr.as_ref() {
                    // 值块：当前作用域顺序执行
                    Expr::Block(block) => {
                        self.refined_walk_stmts(&block.stmts, unit, ctx);
                    }
                    Expr::While { body, .. } => {
                        self.refined_walk_loop_body(None, body, unit, ctx);
                    }
                    Expr::For { var, body, .. } | Expr::SpawnFor { var, body, .. } => {
                        self.refined_walk_loop_body(Some(var), body, unit, ctx);
                    }
                    // RFC-027 §3.4：调用点的形参精化义务
                    Expr::Call { .. } => {
                        self.check_call_arg_refinements(expr, unit, ctx);
                    }
                    // RFC-027：返回位精化在 `return` 点验证（后置条件）
                    Expr::Return(Some(v), return_span) => {
                        self.check_return_refinement(v, *return_span, unit, ctx);
                    }
                    _ => {}
                }
                Vec::new()
            }
            _ => Vec::new(),
        }
    }

    /// 在**守卫成立**的假设下走查一个分支（RFC-027 §3.4）。
    ///
    /// §3.4 的帮助文案自己举的例子就是这条路径：
    /// `if y > 0 { divide(x, y) }`——守卫 `y > 0` 正是精化实参所需的静态证据。
    /// 不注入守卫会把这类**合法**代码判伪（y 无约束时可取 0），是误报。
    ///
    /// 作用域配对保证守卫不泄漏到分支之外（`else` 不该看到 `then` 的条件）。
    fn refined_walk_branch_with_guard(
        &self,
        guard: Option<crate::frontend::core::types::const_data::ConstExpr>,
        stmts: &[crate::frontend::core::parser::ast::Stmt],
        unit: &mut RefinedScopeUnit,
        ctx: &mut RefinedWalkCtx<'_, '_>,
    ) {
        ctx.shared_ctx.assumptions.enter_scope();
        if let Some(g) = guard {
            ctx.shared_ctx.assumptions.inject(g);
        }
        self.refined_walk_stmts(stmts, unit, ctx);
        ctx.shared_ctx.assumptions.exit_scope();
    }

    /// 走查 if 链：返回所有**可达**路径的出口值环境（进入态不被污染）。
    ///
    /// 守卫裁剪：条件以当前值环境折叠可判定（`Some(true/false)`）时，只走
    /// 会执行的分支——不可达分支内的赋值不产生保守误报，其内的精化声明
    /// 不产生验证义务（块局部，出块即清理）。不可判定时保守展开为多路径。
    fn refined_walk_if_paths(
        &self,
        condition: &crate::frontend::core::parser::ast::Expr,
        then_stmts: &[crate::frontend::core::parser::ast::Stmt],
        else_ifs: &[(
            Box<crate::frontend::core::parser::ast::Expr>,
            Box<crate::frontend::core::parser::ast::Block>,
        )],
        else_branch: Option<&crate::frontend::core::parser::ast::Block>,
        unit: &mut RefinedScopeUnit,
        ctx: &mut RefinedWalkCtx<'_, '_>,
    ) -> Vec<HashMap<String, crate::frontend::core::types::ConstValue>> {
        match Self::fold_bool(&*ctx.shared_ctx, condition, &unit.values) {
            Some(true) => {
                self.refined_walk_branch_with_guard(None, then_stmts, unit, ctx);
                vec![unit.values.clone()]
            }
            Some(false) => match else_ifs.split_first() {
                Some(((cond, body), rest)) => {
                    self.refined_walk_if_paths(cond, &body.stmts, rest, else_branch, unit, ctx)
                }
                None => match else_branch {
                    Some(eb) => {
                        self.refined_walk_branch_with_guard(None, &eb.stmts, unit, ctx);
                        vec![unit.values.clone()]
                    }
                    None => vec![unit.values.clone()],
                },
            },
            None => {
                // then 臂：条件成立是一条假设（§3.4 的守卫证据）
                let then_guard =
                    crate::frontend::core::types::eval::const_eval::convert_expr_to_const_expr(
                        condition,
                    );
                let pre = unit.values.clone();
                self.refined_walk_branch_with_guard(then_guard, then_stmts, unit, ctx);
                let mut paths = vec![std::mem::replace(&mut unit.values, pre)];
                // 余下链递归（后续条件同样剪枝）；无 else-if 时
                // 「全部条件都不中」的路径 = else 臂（有 else）或进入态原样
                match else_ifs.split_first() {
                    Some(((cond, body), rest)) => paths.extend(self.refined_walk_if_paths(
                        cond,
                        &body.stmts,
                        rest,
                        else_branch,
                        unit,
                        ctx,
                    )),
                    None => match else_branch {
                        Some(eb) => {
                            // else 臂：条件不成立
                            self.refined_walk_branch_with_guard(
                                crate::frontend::core::typecheck::layers::termination::negate_guard(
                                    condition,
                                ),
                                &eb.stmts,
                                unit,
                                ctx,
                            );
                            paths.push(unit.values.clone());
                        }
                        None => paths.push(unit.values.clone()),
                    },
                }
                paths
            }
        }
    }

    /// 走查循环体：迭代间被改写的变量值未知——入环与出环都作废其字面量值
    /// （第 2+ 次迭代的取值与入口值不同，入口值对体内 VC 不安全）
    fn refined_walk_loop_body(
        &self,
        loop_var: Option<&str>,
        body: &crate::frontend::core::parser::ast::Block,
        unit: &mut RefinedScopeUnit,
        ctx: &mut RefinedWalkCtx<'_, '_>,
    ) {
        let mut assigned = collect_assigned_var_names(&body.stmts);
        if let Some(var) = loop_var {
            assigned.insert(var.to_string());
        }
        for name in &assigned {
            unit.values.remove(name);
        }
        self.refined_walk_stmts(&body.stmts, unit, ctx);
        for name in &assigned {
            unit.values.remove(name);
        }
    }

    /// RFC-027 §3：从函数注解 **AST** 解析返回位精化（后置条件）。
    ///
    /// 与 `resolve_type_annotation` 承担同一义务，区别只在**来源**：那里拿到的
    /// 是已降级的 `MonoType`，而 `MonoType::from` 会把不可折叠的谓词实参
    ///（`r + 100`）压成占位 `TypeRef("<const-expr>")`。实参一旦丢失，
    /// `-> (r: P(任意实参))` 就退化成 `-> (r: P(<const-expr>))`，再被旧的
    ///「不在作用域的自由变量即返回值形参」启发式代入返回值——实测 d1
    ///（`-> (r: IsPositive(r + 100))` 返 `-5`）报出的约束因此是 `-5 > 0`。
    ///
    /// 返回形式参数严格按**声明的名字**识别：有声明（`(r: ...)`）时该名字绑定
    /// `return` 的值；**其余不在作用域的自由变量不再被当成 binder**，而是报
    /// 未定义标识符（`(r: Eq2(m, r))` 的 `m` 此前被静默代入返回值，约束退化成
    /// `7 == 7` 恒真）。
    ///
    /// 失败路径（非谓词应用、实参不可转换、元数不符）一律静默返回 `None`：
    /// 那几条诊断归注册路径（`resolve_type_annotation`），此处不重复报。
    /// 唯独「有未声明的自由变量」是本函数新增的义务，必须报、不得静默放行。
    ///
    /// 作用域名单 = 本函数签名形参（`signature_params`，含 curry 各组与类型/
    /// const 参数）+ 模块级绑定（`env.vars`）。嵌套函数体引用**外层**函数的
    /// 形参时不在名单内——那类约束没有静态证据可判，报未定义标识符比静默
    /// 代入返回值安全。
    fn resolve_return_refinement(
        &self,
        type_ann: &crate::frontend::core::parser::ast::Type,
        params: &[Param],
        diags: &mut Vec<Diagnostic>,
    ) -> Option<ReturnRefinement> {
        use crate::frontend::core::parser::ast::Type as AstType;

        let AstType::Fn { return_type, .. } = type_ann else {
            return None;
        };
        let (binder, annotated) = match return_type.as_ref() {
            AstType::NamedParen { param, inner, .. } => (Some(param.clone()), inner.as_ref()),
            other => (None, other),
        };
        let AstType::Generic { name, args, .. } = strip_type_parens(annotated) else {
            return None;
        };
        // `Terminates(m)` 是终止测度而非值约束（RFC-027 §6.9）
        if name == "Terminates" {
            return None;
        }
        let refined = self.refine_predicate_from_ast_args(name, args)?;

        // RFC-027 §3：除声明名之外，不在作用域（形参/模块级绑定）的自由变量是
        // 未定义标识符。旧启发式把它们一并代入返回值，使约束退化成恒真。
        let mut undeclared: Vec<String> = Vec::new();
        for free in refined_free_vars(&refined) {
            if binder.as_deref() == Some(free.as_str()) {
                continue;
            }
            if params.iter().any(|p| p.name == free) || self.env.vars.contains_key(&free) {
                continue;
            }
            if !undeclared.contains(&free) {
                undeclared.push(free);
            }
        }
        if !undeclared.is_empty() {
            for name in &undeclared {
                diags.push(ErrorCodeDefinition::unknown_variable(name).build());
            }
            // 注解已被拒：不再生成返回点义务（避免连环诊断）
            return None;
        }
        Some(ReturnRefinement { binder, refined })
    }

    /// 把谓词应用正格化为 `Refined`，实参取自 **AST**。
    ///
    /// 与 `resolve_type_annotation` 的 `Generic` 分支同源：编译期谓词代入谓词体、
    /// 证明函数留作不透明 `Call`。唯一区别是实参来源——这里直接读 AST，
    /// `r + 100` 这类符号实参不再被降级成占位。
    ///
    /// 取不出实参（不是谓词、形态不可转换、元数不符）时返回 `None` 且**不报诊断**：
    /// 那几条路径的诊断归属注册路径，此处只负责「按 AST 无损代入」。
    fn refine_predicate_from_ast_args(
        &self,
        name: &str,
        args: &[crate::frontend::core::parser::ast::Type],
    ) -> Option<MonoType> {
        // 1. 编译期谓词定义（单参数）：实参代入谓词体模板
        if let Some(def) = self.env.predicate_defs.get(name) {
            if args.len() != 1 {
                return None;
            }
            let arg_expr = self.ast_type_to_const_expr(&args[0])?;
            let constraint =
                crate::frontend::core::typecheck::layers::termination::substitute_const_expr(
                    &def.constraint,
                    std::slice::from_ref(&def.param_name),
                    std::slice::from_ref(&arg_expr),
                );
            return Some(MonoType::Refined {
                base: Box::new(def.param_type.clone()),
                constraint,
            });
        }
        // 2. 证明函数（源码定义、返回 `Type`）：约束是不透明应用
        let mono_args: Vec<MonoType> = args.iter().map(|a| MonoType::from(a.clone())).collect();
        let base = match self.lookup_proof_fn_base_type(name, &mono_args) {
            Some(Ok(base)) => base,
            _ => return None,
        };
        let const_args: Option<Vec<ConstExpr>> = args
            .iter()
            .map(|a| self.ast_type_to_const_expr(a))
            .collect();
        Some(MonoType::Refined {
            base: Box::new(base),
            constraint: ConstExpr::Call {
                func: name.to_string(),
                args: const_args?,
            },
        })
    }

    /// 谓词实参（AST 形态）→ `ConstExpr`。
    ///
    /// **先取 AST**：`r + 100` 在 AST 里是原样的编译期表达式，降级成 `MonoType`
    /// 后才变成占位 `"<const-expr>"`。取不出时退回 `MonoType` 形态（与
    /// `mono_type_to_const_expr` 同源：裸名 → `NamedVar`、字面量 → `Lit`），
    /// 使实参覆盖面与既有路径一致。
    fn ast_type_to_const_expr(
        &self,
        ty: &crate::frontend::core::parser::ast::Type,
    ) -> Option<ConstExpr> {
        use crate::frontend::core::parser::ast::Type as AstType;
        match ty {
            AstType::ConstExpr(expr) => convert_expr_to_const_expr(expr),
            AstType::Paren(inner) | AstType::NamedParen { inner, .. } => {
                self.ast_type_to_const_expr(inner)
            }
            other => self.mono_type_to_const_expr(&MonoType::from(other.clone())),
        }
    }

    /// 走查函数体：独立单元——依赖与值环境以函数体为边界，
    /// 不与外层或兄弟函数互通（跨函数同名不互相触发重验证）
    fn refined_walk_fn_body(
        &self,
        body: &crate::frontend::core::parser::ast::Block,
        fn_name: Option<&str>,
        ret_refinement: Option<ReturnRefinement>,
        ctx: &mut RefinedWalkCtx<'_, '_>,
    ) {
        // RFC-027 §3.3：当前函数的**形参精化**进假设集 Γ——同函数的每个 `return`
        // 都在它的前置条件下成立，缺它则 `succ` 这类后置条件恒推不出。
        // 只注入本函数自己的： 跨函数同名不能互相作证。
        let assumptions: Vec<crate::frontend::core::types::const_data::ConstExpr> = fn_name
            .and_then(|n| self.collect_param_refinements().get(n).cloned())
            .unwrap_or_default();
        let saved_ret = ctx.fn_return_refined.take();
        let saved_binder = ctx.fn_return_binder.take();
        let saved_name = ctx.fn_name.take();
        // RFC-027 §3：后置条件在**声明处**由 AST 解析（见 `resolve_return_refinement`）。
        // 走查只消费结果——注册路径那份 `MonoType` 已把不可折叠的谓词实参
        //（`r + 100`）压成 `"<const-expr>"` 占位，据此判定会把实参丢掉。
        let (refined, binder) = match ret_refinement {
            Some(r) => (Some(r.refined), r.binder),
            None => (None, None),
        };
        ctx.fn_return_refined = refined;
        ctx.fn_return_binder = binder;
        ctx.fn_name = fn_name.map(str::to_string);

        ctx.shared_ctx.assumptions.enter_scope();
        for a in assumptions {
            ctx.shared_ctx.assumptions.inject(a);
        }

        let mut fn_unit = RefinedScopeUnit::new();
        self.refined_walk_stmts(&body.stmts, &mut fn_unit, ctx);

        // RFC-010a 规则①：块的值 = 尾表达式；而函数体的值即返回值。
        // 故**尾表达式是隐式 `return`**，后置条件同样要在它上面验证
        //（`bad: (b: IsPositive(b)) -> (r: IsPositive(r)) = { b - 1 }` 无 `return` 关键字）。
        // 显式 `return` 已在 `refined_walk_stmt` 里查过，不重复。
        if let Some(last) = body.stmts.last() {
            if let crate::frontend::core::parser::ast::StmtKind::Expr(e) = &last.kind {
                if !matches!(
                    e.as_ref(),
                    crate::frontend::core::parser::ast::Expr::Return(..)
                ) {
                    self.check_return_refinement(e, last.span, &fn_unit, ctx);
                }
            }
        }

        ctx.shared_ctx.assumptions.exit_scope();
        ctx.fn_return_refined = saved_ret;
        ctx.fn_return_binder = saved_binder;
        ctx.fn_name = saved_name;
    }

    /// 重验证 dependant 的精化约束（VC 生成 → 证明管道）
    ///
    /// `trigger` 进 E2030 文案（赋值场景 = 被赋值变量名，初始校验 = 声明名）。
    /// `changed`：本次变更 (变量, 新值)——已知则代入 bindings；未知（非字面量
    /// RHS）则作废旧值，用陈旧值判约束会产生伪 Proved。
    fn revalidate_refined(
        &self,
        trigger: &str,
        dependant: &str,
        refined: &MonoType,
        changed: (&str, Option<crate::frontend::core::types::ConstValue>),
        values: &HashMap<String, crate::frontend::core::types::ConstValue>,
        ctx: &mut RefinedWalkCtx<'_, '_>,
    ) {
        let mut bindings = values.clone();
        match changed.1 {
            Some(v) => {
                bindings.insert(changed.0.to_string(), v);
            }
            None => {
                bindings.remove(changed.0);
            }
        }
        // 缺值闸门：约束的自由变量不全有已知值则不送检（防全称检查伪反例）
        if !refined_free_vars(refined)
            .iter()
            .all(|v| bindings.contains_key(v))
        {
            return;
        }
        match crate::frontend::core::typecheck::layers::predicate::check_predicate(
            ctx.shared_ctx,
            refined,
            &bindings,
        ) {
            ProofResult::Proved => {}
            ProofResult::Disproved(model) => {
                if let MonoType::Refined { constraint, .. } = refined {
                    // 闭合约束（无自由变量）为假 ⇒ **注解自身不成立**，与传给它
                    // 的值无关（`IsPositive(-5)`：`-5 > 0` 恒假）。这一支报 E4018
                    // 且**不**展示反例：bindings 里那个变量与闭合约束无关，展示
                    // 它（实测 `y=Int(5)`）会把人引向 RHS。
                    if refined_free_vars(refined).is_empty() {
                        // 说明文字走 locales per-site 键（issue #469 处 1/2a）：
                        // 与 message 同语言渲染，不在发射点硬编成品串
                        ctx.diags.push(
                            ErrorCodeDefinition::refinement_violated(&constraint.to_string())
                                .param_from_key("counterexample", "E4018.no_free_var.note")
                                .build(),
                        );
                    } else {
                        // 开约束：依赖变量的当前取值使其为假 ⇒ 重验证失败
                        let counterexample = model
                            .assignments
                            .iter()
                            .map(|(k, v)| format!("{k}={v}"))
                            .collect::<Vec<_>>()
                            .join(", ");
                        ctx.diags.push(
                            ErrorCodeDefinition::refined_constraint_violated(
                                trigger,
                                dependant,
                                &constraint.to_string(),
                                &counterexample,
                            )
                            .build(),
                        );
                    }
                }
            }
            ProofResult::Unproven {
                proof_calls: calls, ..
            } => {
                if calls.is_empty() {
                    // RFC-027 §4/§9：Unproven → 编译错误，无降级、无 silent pass。
                    // 此分支 = 绑定完备仍证不出（约束形态超出证明内核：If/Range
                    // 形约束、SMT unknown、超预算）。谓词定义注册后（#377-3）
                    // 约束已可以是谓词体的任意形态（不再是清一色 Call），本分支
                    // 因此成为真防线而非前瞻兑底：不得让 silent pass 复活。
                    ctx.diags.push(
                        ErrorCodeDefinition::refined_unproven(
                            trigger,
                            dependant,
                            &refined.to_string(),
                        )
                        .build(),
                    );
                } else {
                    ctx.proof_calls.extend(calls.clone());
                }
            }
        }
    }

    /// RFC-027 §3.3/§3.4：调用点实参对**形参精化**的验证。
    ///
    /// 被调函数签名里的精化形参（`b: IsPositive(b)`）是调用方义务：实参值
    /// 编译期可知且使约束为假时报 E4018。与绑定位共用同一检查器
    /// （`check_predicate`），故两处判定一致。
    ///
    /// **本版只做证伪**：实参不可折叠、约束自由变量不为单个、或结果非
    /// `Disproved` 时一律放行。§3.4 要求的「无静态证据即拒绝」是更强的半场
    /// ——它会把 `divide(x, y)`（实参无界、无标注）一并拒掉，需单独决策后
    /// 另行落地，故此处不默认启用。
    ///
    /// 局限：只在调用点两种形态触发（裸调用语句、赋值 RHS 为调用）；嵌套在
    /// 更复杂表达式里的调用不查（无通用表达式遍历器）。
    fn check_call_arg_refinements(
        &self,
        expr: &Expr,
        unit: &RefinedScopeUnit,
        ctx: &mut RefinedWalkCtx<'_, '_>,
    ) {
        let Expr::Call {
            func, args, span, ..
        } = expr
        else {
            return;
        };
        let Expr::Var(fn_name, _) = func.as_ref() else {
            return;
        };
        // 签名取自 env：形参位已由 `resolve_type_annotation` 规范化为 Refined
        let Some(sig) = self.env.vars.get(fn_name) else {
            return;
        };
        let MonoType::Fn { params, .. } = &sig.body else {
            return;
        };
        for (i, param_ty) in params.iter().enumerate() {
            if !matches!(param_ty, MonoType::Refined { .. }) || constraint_is_terminates(param_ty) {
                continue;
            }
            let Some(arg) = args.get(i) else { continue };
            let MonoType::Refined { base, constraint } = param_ty.clone() else {
                continue;
            };
            let free = refined_free_vars(param_ty);
            if free.len() != 1 {
                continue;
            }
            // 实参**符号代入**约束（RFC-027 §3.3）：不要求实参可折成常量。
            //
            // 折常量版只能查 `f(-5)` 这类字面量，而 §3.4 要的是「运行时值进入
            // 精化类型参数必须提供静态证据」——`f(y)`（y 无界）与 `f(n)`（n 带
            // 精化标注）正是它的两行范例。符号代入把 `y > 0` / `n > 0` 形式化后
            // 交判定管道：Γ 里有证据则成立，无则判伪/判不出。
            let Some(arg_expr) =
                crate::frontend::core::types::eval::const_eval::convert_expr_to_const_expr(arg)
            else {
                continue;
            };
            let substituted = MonoType::Refined {
                base: base.clone(),
                constraint:
                    crate::frontend::core::typecheck::layers::termination::substitute_const_expr(
                        &constraint,
                        std::slice::from_ref(&free[0]),
                        &[arg_expr],
                    ),
            };
            // RFC-027 §3.3：实参**自身的精化标注**是证据（#395 第三行）。
            //
            // 在**本检查局部**注入实参变量已注册的精化约束，查完即弹：不放进
            // 声明处的全局 Γ。原因是重验证路径会自命中——它验的正是同一条约束，
            // 若该约束已在 Γ，判定会在 level 2（精确匹配）平凡通过，**掩盖真违反**
            //（实测：4 个「依赖变量重赋值」负例语料会被静默放过）。
            let arg_refined: Option<crate::frontend::core::types::const_data::ConstExpr> = match arg
            {
                Expr::Var(name, _) => unit.deps.constraint_of(name).and_then(|ty| match ty {
                    MonoType::Refined { constraint, .. } if !constraint_is_terminates(ty) => {
                        Some(constraint.clone())
                    }
                    _ => None,
                }),
                _ => None,
            };
            let scoped = arg_refined.is_some();
            if scoped {
                ctx.shared_ctx.assumptions.enter_scope();
                if let Some(c) = arg_refined {
                    ctx.shared_ctx.assumptions.inject(c);
                }
            }
            let bindings = unit.values.clone();
            let outcome = crate::frontend::core::typecheck::layers::predicate::check_predicate(
                ctx.shared_ctx,
                &substituted,
                &bindings,
            );
            if scoped {
                ctx.shared_ctx.assumptions.exit_scope();
            }
            match outcome {
                ProofResult::Proved => {}
                ProofResult::Disproved(model) => {
                    // issue #469 处 2a：反例段由 locales per-site 键渲染；模型未绑定
                    // 值时参数为空串——段整体缺席，不再输出悬空的「反例：」标题
                    let mut builder = ErrorCodeDefinition::refinement_violated(&model.constraint);
                    if model.assignments.is_empty() {
                        builder = builder.param("counterexample", "");
                    } else {
                        let assignments = model
                            .assignments
                            .iter()
                            .map(|(k, v)| format!("  {k} = {v}"))
                            .collect::<Vec<_>>()
                            .join("\n");
                        builder = builder
                            .param("assignments", assignments)
                            .param_from_key("counterexample", "E4018.counterexample.section");
                    }
                    ctx.diags.push(builder.at(*span).build());
                }
                // 具名证明函数形态的约束（`IsPositive(b)`）当下判不出真假：
                // 交由 pipeline 编译并执行证明函数，返回 false 即 E4018。
                // 与绑定位（`revalidate_refined`）同一通道，故两处判定一致。
                ProofResult::Unproven {
                    proof_calls: calls, ..
                } => {
                    if calls.is_empty() {
                        // 无证法可执行且证不出：报错，不静默放行（§3.4）
                        ctx.diags.push(
                            ErrorCodeDefinition::refined_unproven(
                                fn_name,
                                &free[0],
                                &param_ty.to_string(),
                            )
                            .at(*span)
                            .build(),
                        );
                    } else {
                        ctx.proof_calls.extend(calls);
                    }
                }
            }
        }
    }
    /// RFC-027 §3：返回位精化在 `return` 点验证（后置条件）。
    ///
    /// 形参位与返回位是**同一个规则**的两侧（§3「统一性」）：`形参名:
    /// 谓词调用(形参名)`，区别仅在于值由调用方提供还是由 `return` 提供。
    /// 故两处判定走同一管道，仅**绑定来源**不同：
    ///
    /// - 形参位 → 调用点义务（`check_call_arg_refinements`）：自由变量绑到实参
    /// - 返回位 → 返回点义务（本函数）：自由变量按下列规则绑定
    ///
    /// **返回值形参的识别**：严格按**声明的名字**——`resolve_return_refinement`
    /// 从 `-> (r: P(...))` 取出 binder（§3：它「仅存在于类型签名中、仅被谓词
    /// 引用，**不进入函数体作用域**」），只有它绑到 `return` 表达式的值。
    /// 作用域内的自由变量（形参/模块级绑定）用自己的值或符号；既非声明名、
    /// 又不在作用域的自由变量在声明处已报未定义标识符，不再被代入返回值。
    /// 裸形态（`-> P(b - 1)`，约束只涉及形参）无声明名，约束与返回值无关地
    /// 成立/不成立——两个形态因此归一条规则。
    ///
    /// 保守方向：返回表达式不可折叠时**不判**（无静态证据即不报）——强半场
    /// （§3.4「无静态证据则编译错误」）是单独的开关，见 #395。
    fn check_return_refinement(
        &self,
        value: &Expr,
        span: crate::util::span::Span,
        unit: &RefinedScopeUnit,
        ctx: &mut RefinedWalkCtx<'_, '_>,
    ) {
        let Some(ret_ty) = ctx.fn_return_refined.clone() else {
            return;
        };
        let MonoType::Refined { base, constraint } = ret_ty.clone() else {
            return;
        };
        if constraint_is_terminates(&ret_ty) {
            return;
        }
        let free = refined_free_vars(&ret_ty);
        // 无自由变量的闭合约束与返回表达式无关（它只是一个退化的精化类型，
        // 成立性由注解自身决定）——不在本义务范围内。
        if free.is_empty() {
            return;
        }
        // 约束里**符号代入**声明的返回形式参数（§3「值由 `return` 提供」）。
        //
        // 用符号代入而非折常量：后置条件的价值正在于形参未知时也能判——
        // `r > 0` 代入 `b - 1` 得 `b - 1 > 0`，在 Γ={b>0} 下可判伪（b = 1）。
        // 折常量会把这类全漏掉（`b - 1` 含未知形参，折不出值）。
        let mut constraint = constraint;
        if let Some(binder) = ctx.fn_return_binder.clone() {
            // 只有声明名由 `return` 供给；约束未引用它时（如 `-> (r: P(5))`）
            // 无需代入，照常判定。
            if free.iter().any(|name| name == &binder) {
                let Some(returned) =
                    crate::frontend::core::types::eval::const_eval::convert_expr_to_const_expr(
                        value,
                    )
                else {
                    // 返回表达式转不出常量表达式（如含调用）：无从代入，保守不判
                    return;
                };
                constraint =
                    crate::frontend::core::typecheck::layers::termination::substitute_const_expr(
                        &constraint,
                        std::slice::from_ref(&binder),
                        &[returned],
                    );
            }
        }
        // 作用域内变量的当前值环境进 bindings；形参不在其中，留给 SMT 作符号
        let bindings = unit.values.clone();
        // 代入后的约束文本（含 `return` 值的符号形态），进诊断比未代入形态更可读。
        // 它是**约束表达式**（不是自然语言句子），故与 locale 无关。
        let constraint_text = constraint.to_string();
        let substituted = MonoType::Refined { base, constraint };
        match crate::frontend::core::typecheck::layers::predicate::check_predicate(
            ctx.shared_ctx,
            &substituted,
            &bindings,
        ) {
            ProofResult::Proved => {}
            ProofResult::Disproved(model) => {
                // issue #469 处 2a：反例段由 locales per-site 键渲染；模型未绑定
                // 值时参数为空串——段整体缺席（与形参位同一形态）
                let mut builder = ErrorCodeDefinition::refinement_violated(&model.constraint);
                if model.assignments.is_empty() {
                    builder = builder.param("counterexample", "");
                } else {
                    let assignments = model
                        .assignments
                        .iter()
                        .map(|(k, v)| format!("  {k} = {v}"))
                        .collect::<Vec<_>>()
                        .join("\n");
                    builder = builder
                        .param("assignments", assignments)
                        .param_from_key("counterexample", "E4018.counterexample.section");
                }
                ctx.diags.push(builder.at(span).build());
            }
            ProofResult::Unproven {
                proof_calls: calls, ..
            } => {
                if calls.is_empty() {
                    let fn_name = ctx
                        .fn_name
                        .clone()
                        .unwrap_or_else(|| "<anonymous>".to_string());
                    // RFC-027 §3：返回点义务的对象是**返回位**，不是签名形参——
                    // 此前这里用 `free[0]`（多参数谓词时那是形参名），文案把约束
                    // 挂到了错误对象上（实测：`SumUpTo(b, r)` 报成 "'b' 的精化
                    // 类型约束"）。
                    //
                    // 本槽只传 **标识符**：声明了 binder 就传它的名字（`r`——它是
                    // 返回形式参数，不是签名形参）；裸形态没有声明名，按 RFC-027 §2
                    // 取返回值形参的规范名 `result`（见 `DEFAULT_RETURN_BINDER`）。
                    // 任何自然语言措辞都由 locale 模板负责，code 侧不得写某语言的词。
                    //
                    // 这里**不覆盖 help**：E2031 的本地化 help 已完整给出可操作的
                    // 替代写法（实参可静态取值 / 提供返回 Type 的证明函数），
                    // 位置化文案要走 locales，不在 code 里拼。
                    let subject = match &ctx.fn_return_binder {
                        Some(binder) => binder.clone(),
                        None => DEFAULT_RETURN_BINDER.to_string(),
                    };
                    ctx.diags.push(
                        ErrorCodeDefinition::refined_unproven(&fn_name, &subject, &constraint_text)
                            .at(span)
                            .build(),
                    );
                } else {
                    ctx.proof_calls.extend(calls);
                }
            }
        }
    }
}

/// 裸形态返回值形参的**规范名**：`-> P(...)` 在语义上就是 `-> (result: P(...))`
/// 的简写（RFC-027 §2：spec 的 `max` 例子即 `-> (result: IsMax(T, arr, result))`，
/// 「`result` 是返回值形参，值由 `return` 提供」）。
///
/// 用途：裸形态没有声明名，E2031 的 `{var}` 槽取它作标识——与「按声明名识别」
/// 同一口径、是合法标识符（无任何语言的散文），跨 6 语言模板都自然：
/// en 渲染 `the refinement type constraint of 'result'`，zh 渲染 `'result' 的精化类型约束`。
const DEFAULT_RETURN_BINDER: &str = "result";

/// RFC-027 §3：返回位精化（后置条件）的解析结果。
///
/// `binder` 是**声明的**返回形式参数名（`-> (r: P(r))` 的 `r`）：只有它由
/// `return` 的值供给；`refined` 的约束已按 AST 实参代入——`r + 100` 保持符号
/// 形态，而不是被降级成占位 `<const-expr>`。
struct ReturnRefinement {
    /// `-> (r: P(...))` 声明的返回形式参数名；裸精化形态（`-> P(b - 1)`）为 `None`
    binder: Option<String>,
    /// 精化类型（`Refined { base, constraint }`）
    refined: MonoType,
}

/// 剥离类型外侧的括号：精化解析里 `Paren`（RFC-004）与 `NamedParen`
///（RFC-027 §3 具名形态）同为透明，只递归内层。
fn strip_type_parens(
    ty: &crate::frontend::core::parser::ast::Type
) -> &crate::frontend::core::parser::ast::Type {
    use crate::frontend::core::parser::ast::Type as AstType;
    match ty {
        AstType::Paren(inner) => strip_type_parens(inner),
        AstType::NamedParen { inner, .. } => strip_type_parens(inner),
        other => other,
    }
}

/// 精化走查的共享管道三件套：证明上下文 / 证明调用收集 / 诊断汇。
/// 走查各环节（初始校验、重验证、分支与循环下钻）都经它流转，
/// 不再逐层透传散参数。
struct RefinedWalkCtx<'a, 'env> {
    shared_ctx: &'a mut crate::frontend::core::typecheck::proof::context::ProofContext<'env>,
    proof_calls: &'a mut Vec<crate::frontend::core::typecheck::proof::verdict::ProofFunctionCall>,
    diags: &'a mut Vec<Diagnostic>,
    /// 当前函数的**返回位精化类型**（RFC-027 后置条件）。
    ///
    /// 函数体边界保存/恢复（lambda 可嵌套）——外层函数的后置条件不得
    /// 泄漏到内层函数体的 `return` 上。`None` = 无后置条件或无函数上下文。
    fn_return_refined: Option<MonoType>,
    /// 当前函数**声明的返回形式参数名**（`-> (r: P(r))` 的 `r`）。
    ///
    /// RFC-027 §3：只有这个名字由 `return` 的值供给；其余自由变量必须在
    /// 作用域内（形参/模块级绑定），否则在解析处报未定义标识符。
    /// 裸形态（`-> P(b - 1)`，约束只涉及形参）无声明名 ⇒ `None`。
    fn_return_binder: Option<String>,
    /// 当前函数名（进 E4018 的未证明文案）。
    fn_name: Option<String>,
}

/// 分支汇合：仅保留所有路径出口**等值**的变量。
/// 被任一路径改写（或仅部分路径声明）的变量出块即值未知——
/// 保守作废是 sound 的：宁可少判，不可用错值判。
fn meet_value_paths(
    paths: Vec<HashMap<String, crate::frontend::core::types::ConstValue>>
) -> HashMap<String, crate::frontend::core::types::ConstValue> {
    let mut meet = HashMap::new();
    let Some(first) = paths.first() else {
        return meet;
    };
    'outer: for (name, value) in first {
        for path in &paths[1..] {
            if path.get(name) != Some(value) {
                continue 'outer;
            }
        }
        meet.insert(name.clone(), value.clone());
    }
    meet
}

/// 收集语句列表中被**改写**的变量名（无注解赋值 `x = v` 的目标），
/// 供循环体作废字面量值。注解绑定（`mut x: T = ...`）每次迭代重新
/// 初始化且字面量值恒定，不在收集之列。递归覆盖嵌套控制流与值块。
fn collect_assigned_var_names(
    stmts: &[crate::frontend::core::parser::ast::Stmt]
) -> std::collections::HashSet<String> {
    use crate::frontend::core::parser::ast::{Expr, StmtKind};
    let mut assigned = std::collections::HashSet::new();
    for stmt in stmts {
        match &stmt.kind {
            StmtKind::Assign {
                target,
                type_annotation: None,
                value,
                ..
            } => {
                if let Expr::Var(name, _) = target.as_ref() {
                    assigned.insert(name.clone());
                }
                if let Some(v) = value.as_deref() {
                    collect_assigned_in_expr(v, &mut assigned);
                }
            }
            StmtKind::Assign { value, .. } => {
                if let Some(v) = value.as_deref() {
                    collect_assigned_in_expr(v, &mut assigned);
                }
            }
            StmtKind::If {
                then_branch,
                else_if_branches,
                else_branch,
                ..
            } => {
                assigned.extend(collect_assigned_var_names(&then_branch.stmts));
                for (_, body) in else_if_branches {
                    assigned.extend(collect_assigned_var_names(&body.stmts));
                }
                if let Some(eb) = else_branch {
                    assigned.extend(collect_assigned_var_names(&eb.stmts));
                }
            }
            StmtKind::For { var, body, .. } => {
                assigned.insert(var.clone());
                assigned.extend(collect_assigned_var_names(&body.stmts));
            }
            StmtKind::Expr(expr) => collect_assigned_in_expr(expr, &mut assigned),
            _ => {}
        }
    }
    assigned
}

/// 表达式形态的控制流体内改写收集（值块 / While / For 表达式）
fn collect_assigned_in_expr(
    expr: &crate::frontend::core::parser::ast::Expr,
    assigned: &mut std::collections::HashSet<String>,
) {
    use crate::frontend::core::parser::ast::Expr;
    match expr {
        Expr::Block(block) => {
            assigned.extend(collect_assigned_var_names(&block.stmts));
        }
        Expr::While { body, .. } => {
            assigned.extend(collect_assigned_var_names(&body.stmts));
        }
        Expr::For { var, body, .. } => {
            assigned.insert(var.clone());
            assigned.extend(collect_assigned_var_names(&body.stmts));
        }
        _ => {}
    }
}
