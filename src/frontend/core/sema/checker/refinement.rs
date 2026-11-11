use super::*;
use crate::frontend::core::sema::proof::verdict::ProofFailure;

impl TypeChecker {
    /// 常量折叠：表达式在字面量值环境下求成具体值（精化值环境的数据源）
    ///
    /// 复用 convert_expr_to_const_expr + Evaluator——与 check_predicate
    /// 第 1 级同一台求值器，折叠语义与约束判定严格一致。转换/求值失败
    /// （变量值未知、调用等非内核形态）→ None：值回到「未知」，走
    /// 「缺值不送检」路径，绝不用陈旧值凑数。
    fn fold_value(
        ctx: &crate::frontend::core::sema::proof::context::ProofContext<'_>,
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
        ctx: &crate::frontend::core::sema::proof::context::ProofContext<'_>,
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
        &self,
        module: &Module,
        proof_calls: &mut Vec<crate::frontend::core::sema::proof::verdict::ProofFunctionCall>,
    ) -> Vec<Diagnostic> {
        let mut shared_ctx =
            crate::frontend::core::sema::proof::context::ProofContext::new(&self.env);
        // 精化解析诊断汇入本地 sink，由调用方（check_module_impl）统一写入——
        // 入口不再借 &mut self 隐式写错误通道，写入点唯一且在调用处可见。
        let mut refined_diags = Vec::new();
        // 声明序形参名表：多形参关系谓词的调用点义务要把约束里的形参名换成
        // 实参，而 `MonoType` 侧已把名字压成自由变量——唯一来源是声明 AST。
        // 整棵声明树一次收齐（调用点不必等声明走过；同名只有一处登记）。
        let mut declared_value_params = std::collections::HashMap::new();
        collect_declared_value_params(&module.items, &mut declared_value_params);
        {
            let mut ctx = RefinedWalkCtx {
                shared_ctx: &mut shared_ctx,
                proof_calls,
                diags: &mut refined_diags,
                fn_return_refined: None,
                fn_return_binder: None,
                fn_name: None,
                declared_value_params,
            };
            let mut module_unit = RefinedScopeUnit::new();
            self.refined_walk_stmts(&module.items, &mut module_unit, &mut ctx);
        }

        refined_diags
    }

    /// 精化走查：一个语句列表 = 一个词法块。
    ///
    /// 块内注册的精化 dependant 在块退出时从依赖图移除（块局部变量出块即亡）；
    /// 函数体/模块层作为最外层块，单元随走查结束整体丢弃，移除只是兜底。
    fn refined_walk_stmts(
        &self,
        stmts: &[crate::ast::Stmt],
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
        stmt: &crate::ast::Stmt,
        unit: &mut RefinedScopeUnit,
        ctx: &mut RefinedWalkCtx<'_, '_>,
    ) -> Vec<String> {
        use crate::ast::{Expr, StmtKind};

        match &stmt.kind {
            // 注解绑定 `x: T = v`：解析精化 → 注册依赖 + 初始校验 → 记录字面量值
            StmtKind::Assign {
                target,
                type_annotation: Some(type_ann),
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
                if crate::ast::Expr::block_binding_is_function(Some(type_ann), value.as_deref()) {
                    if let Some(body) = extract_fn_body(value.as_deref()) {
                        let fn_name = match target.as_ref() {
                            Expr::Var(n, _) => Some(n.as_str()),
                            _ => None,
                        };
                        // RFC-027 §3：返回位精化的 binder 名与谓词实参只在 **AST** 里
                        // 无损（`MonoType` 已把符号实参降级成占位），故在此按声明解析，
                        // 走查只消费结果。
                        let ret_refinement = self.resolve_return_refinement(type_ann, ctx.diags);
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
        stmts: &[crate::ast::Stmt],
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
        condition: &crate::ast::Expr,
        then_stmts: &[crate::ast::Stmt],
        else_ifs: &[(Box<crate::ast::Expr>, Box<crate::ast::Block>)],
        else_branch: Option<&crate::ast::Block>,
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
                                crate::frontend::core::sema::layers::termination::negate_guard(
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
        body: &crate::ast::Block,
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
    /// 作用域名单 = 本函数签名形参（注解 `Type::Fn` 链拍平，含 curry 各组与
    /// 类型/const 参数）+ 模块级绑定（`env.vars`）。嵌套函数体引用**外层**函数的
    /// 形参时不在名单内——那类约束没有静态证据可判，报未定义标识符比静默
    /// 代入返回值安全。
    fn resolve_return_refinement(
        &self,
        type_ann: &crate::ast::Type,
        diags: &mut Vec<Diagnostic>,
    ) -> Option<ReturnRefinement> {
        use crate::ast::Type as AstType;

        // 签名形参从本注解的 Type::Fn 链拍平（6.5.2：不再有并行形参表）
        let params = type_ann.fn_params_flat();

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
        if crate::frontend::core::sema::is_builtin_predicate_name(name) {
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
        args: &[crate::ast::Type],
    ) -> Option<MonoType> {
        // 1. 编译期谓词定义（单参数）：实参代入谓词体模板
        if let Some(def) = self.env.predicate_defs.get(name) {
            if args.len() != 1 {
                return None;
            }
            let arg_expr = self.ast_type_to_const_expr(&args[0])?;
            let constraint =
                crate::frontend::core::sema::layers::termination::substitute_const_expr(
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
        ty: &crate::ast::Type,
    ) -> Option<ConstExpr> {
        use crate::ast::Type as AstType;
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
        body: &crate::ast::Block,
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
            if let crate::ast::StmtKind::Expr(e) = &last.kind {
                if !matches!(e.as_ref(), crate::ast::Expr::Return(..)) {
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
        match crate::frontend::core::sema::layers::predicate::check_predicate(
            ctx.shared_ctx,
            refined,
            &bindings,
        ) {
            Ok(()) => {}
            Err(ProofFailure::Disproved(model)) => {
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
            Err(ProofFailure::Unproven {
                proof_calls: calls, ..
            }) => {
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

    /// 精化义务的**唯一判定单元**：已代入的 `Refined` → `check_predicate` → verdict。
    ///
    /// 形参位（调用点义务）与返回位（后置条件）是**同一条规则的两侧**（RFC-027
    /// §3）：判定与发射因此共用本单元，三值结果的映射只在这里出现一次——
    /// Disproved → E4018（带反例段）；Unproven 有证法 → `proof_calls` 上抛给
    /// pipeline 编译期执行；Unproven 无证法 → E2031（RFC-027 §9：不得静默放行）。
    /// 禁令三 A：同一行为不得在 ≥2 处复制。
    ///
    /// `evidence` 是**只能在本检查局部**成立的假设（实参变量自身的精化标注，
    /// #395 第三行）：本单元负责进出作用域，调用方不得把它塞进全局 Γ。
    ///
    /// # 6.9.5（Σ 依赖记录）复用契约
    ///
    /// Σ 构造点的字段级谓词义务**只允许复用本单元**。调用方负责三步：
    /// ① 从**声明 AST** 取形参名与字段位；② 把实参/字段值转成 `ConstExpr` 并按位
    ///（或按名）对齐；③ 用 `substitute_const_expr` 把约束整体代入成闭合式。
    /// 本单元负责「判定 + 诊断 + 证明调用上抛」。不得为 Σ 另开证明入口，也不得
    /// 复制这段 verdict 映射（禁令三 A）。
    ///
    /// # 可见性 / 位置前提（当前**只在**本模块内可复用）
    ///
    /// 本单元是 `checker/refinement.rs` 内的私有方法，而承载它的子模块
    /// `mod refinement;` 本身是私有的——复用面因此**仅限本文件**。若 6.9.5 的
    /// Σ 构造点检查落在 `checker` 的**其它子模块**，届时再评估把本单元放宽到
    /// `pub(super)`（同层，不构成跨层泄漏，`pub(crate)` 计数不增）；本轮**不**
    /// 增加任何 pub 面。
    ///
    /// 参数口径：
    /// - `assigned` / `subject`：E2031 的 `{assigned}` / `{var}` 两个标识符槽
    ///   （调用点 = 被调函数名 / 被约束形参的声明名；返回点 = 当前函数名 / 返回位名）
    /// - `display`：E2031 的 `{constraint}` 槽，按各站点既有口径传入
    /// - `span`：诊断挂点（调用点 = 调用表达式；返回点 = `return` 语句）
    // 8 个参数都是**义务身份**的不同侧面（判定上下文 / 已代入约束 / 值环境 /
    // 诊断的两个标识符槽 / 约束显示文本 / 挂点 / 局部证据），把它压成结构体只会
    // 新增一个只服务本函数的载体（禁令一 D1：能用既有概念表达就不新增）。两个
    // 调用点的差异面因此显式留在签名里，不靠隐式约定。
    #[allow(clippy::too_many_arguments)]
    fn judge_refined_obligation(
        ctx: &mut RefinedWalkCtx<'_, '_>,
        substituted: &MonoType,
        bindings: &HashMap<String, ConstValue>,
        assigned: &str,
        subject: &str,
        display: &str,
        span: crate::util::span::Span,
        evidence: Option<crate::frontend::core::types::const_data::ConstExpr>,
    ) {
        let scoped = evidence.is_some();
        if scoped {
            ctx.shared_ctx.assumptions.enter_scope();
            if let Some(evidence) = evidence {
                ctx.shared_ctx.assumptions.inject(evidence);
            }
        }
        let outcome = crate::frontend::core::sema::layers::predicate::check_predicate(
            ctx.shared_ctx,
            substituted,
            bindings,
        );
        if scoped {
            ctx.shared_ctx.assumptions.exit_scope();
        }
        match outcome {
            Ok(()) => {}
            Err(ProofFailure::Disproved(model)) => {
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
                ctx.diags.push(builder.at(span).build());
            }
            Err(ProofFailure::Unproven {
                proof_calls: calls, ..
            }) => {
                if calls.is_empty() {
                    // 无证法可执行且证不出：报错，不静默放行（§3.4/§9）
                    ctx.diags.push(
                        ErrorCodeDefinition::refined_unproven(assigned, subject, display)
                            .at(span)
                            .build(),
                    );
                } else {
                    ctx.proof_calls.extend(calls);
                }
            }
        }
    }

    /// RFC-027 §3.1/§3.3/§3.4：调用点实参对**形参精化**的验证。
    ///
    /// 被调函数签名里的精化形参（`b: IsPositive(b)`）是调用方义务：实参值
    /// 编译期可知且使约束为假时报 E4018。判定与发射走**唯一单元**
    /// [`Self::judge_refined_obligation`]（与返回位共用），故两处判定一致。
    ///
    /// **槽位绑定不在此重判**：实参 ↔ 声明形参的对应关系消费共享权威层
    /// `inference::arg_binding::bind_args_to_params`（位置实参占前导槽、命名实参
    /// 按名入槽、六条规则同在一处）。本函数只做「按槽取实参 → 代入 → 判定」。
    ///
    /// **本版边界**（不得读成「关系谓词已被全面接管」，逐条登记在
    /// `docs/src/dev/architecture/09a-wbs-history.md` 的 6.9.1 节）：
    /// - 只在调用点两种形态触发（裸调用语句、赋值 RHS 为调用）。嵌套在更复杂
    ///   表达式里的调用不查（无通用表达式遍历器）——`print(f(42, 7))` 当前不判；
    /// - 多形参关系谓词要求**实参全部有已知值**（缺值闸门，与 `revalidate_refined`
    ///   同一条）：符号实参、绑定位开约束、约束引用非形参名（模块级常量）、
    ///   柯里声明的内层组均不判——这些形态的证据面归 6.9.4 的 Γ 等式桥；
    /// - 一元开约束仍交判定管道找反例，与多元缺值闸门的分叉是**已知且登记**的
    ///   边界，不是等价语义；
    /// - 两条路径的**槽位判据都是「自由变量名 → 声明形参名」**。无本模块声明名表
    ///   时（导入函数；类型体方法的调用形态是字段访问，本函数不进入），一元路径
    ///   退回**位置近似**（`slot = i`）——「被约束位的 binder 就是本位形参名」在
    ///   该形态下是**未校验的假设**，故列为边界而非承诺；多元路径无名字表即不判。
    fn check_call_arg_refinements(
        &self,
        expr: &Expr,
        unit: &RefinedScopeUnit,
        ctx: &mut RefinedWalkCtx<'_, '_>,
    ) {
        let Expr::Call {
            func,
            args,
            named_args,
            span,
            ..
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
        // 声明序运行期形参名（调用点序列）——槽位绑定与多形参代入的共同依据。
        // 只对本模块声明的函数有值；缺项（导入函数 / 类型体方法）⇒ 退回既有
        // 位置对位，不按猜测的位置造义务。
        let declared = ctx.declared_value_params.get(fn_name).cloned();
        // 实参 ↔ 声明形参槽位：**共享权威层**的唯一入口。
        // - `None`：缺声明名，或声明形参表与注册签名不同长（导入函数 / 类型体
        //   方法 / 柯里声明拍平后与注册签名不同长）⇒ 退回既有位置对位；
        // - `Err`：绑定畸形（缺参 / 未知名 / 争槽 / 命名后接位置 / 重复命名）。
        //   那几张诊断的发射点在推断层与 IR 层，本走查**只消费映射、不重复发射**；
        //   退回位置对位——位置实参永远落在前导槽位，前缀义务按旧口径仍生效。
        let arg_slots: Option<Vec<usize>> = match declared
            .as_deref()
            .filter(|names| names.len() == params.len())
        {
            Some(names) => {
                // 普通函数的形参全是必需位（声明不支持默认值/可选参数）。
                let optional = vec![false; names.len()];
                // 绑定畸形 = 该调用已被拒绝（E1010 缺参 / E1014 未知名 / E1111 争槽
                // 或命名后接位置 / E1015 重复命名），诊断的发射点在推断层与 IR 层；
                // 本走查**不重复发射**，退回位置对位——位置实参永远落在前导槽位，
                // 故前缀义务仍按旧口径生效，命名实参在畸形调用里不再造义务。
                crate::frontend::core::sema::inference::arg_binding::bind_args_to_params(
                    fn_name, names, args, named_args, 0, // 普通函数调用无接收者位
                    &optional, *span,
                )
                .ok()
            }
            None => None,
        };
        // 第 `slot` 个声明形参位收到的实参（位置实参在前、命名实参按书写序）——
        // 与 IR 侧的实参重排同一坐标（两侧同源 `arg_binding`）。
        let arg_of_slot = |slot: usize| -> Option<&Expr> {
            match &arg_slots {
                Some(slots) => {
                    slots
                        .iter()
                        .position(|bound| *bound == slot)
                        .and_then(|arg_index| {
                            if arg_index < args.len() {
                                args.get(arg_index)
                            } else {
                                named_args.get(arg_index - args.len()).map(|(_, arg)| arg)
                            }
                        })
                }
                None => args.get(slot),
            }
        };
        // **自由变量 → 声明形参槽位**：一元与多元两条路径共用的唯一判据。
        //
        // 判据是**名字**不是位次：`b: IsPositive(a)` 的约束属于 `a`，与 `b` 的位次
        // 无关。名字不在本模块声明表（约束引用模块级常量 `IsPositive(LIMIT)` 等）
        // ⇒ `None`：保留既有边界，不判、也不拿别的实参顶替。
        let free_slot = |free_var: &str| -> Option<usize> {
            declared
                .as_deref()
                .and_then(|names| names.iter().position(|n| n == free_var))
        };
        for (i, param_ty) in params.iter().enumerate() {
            if !matches!(param_ty, MonoType::Refined { .. }) || constraint_is_terminates(param_ty) {
                continue;
            }
            let MonoType::Refined { base, constraint } = param_ty.clone() else {
                continue;
            };
            let free = refined_free_vars(param_ty);
            // 代入形态按**自由变量个数**分流；判定与诊断只有一份（下面共用）。
            // - 1 个：binder 由**约束自己写下的名字**决定（`IsPositive(b)` 的谓词体
            //   展开成 `b > 0`；`b: IsPositive(a)` 展开成 `a > 0`）——按名字解析
            //   槽位，不得用「被约束位 i 的实参」顶替；
            // - ≥2 个：多形参**关系**谓词（`Eq(n, N)`、`SumUpTo(n, s)`），自由
            //   变量是多个声明形参名，逐个按名定位后整体代入；
            // - 0 个：闭合约束与实参无关（既有边界，不在此判）。
            let (substituted, subject, evidence_arg) = if free.len() == 1 {
                // 槽位按 free[0] 的**名字**解析（与多元分支同一判据 `free_slot`）：
                // 约束属于哪个形参由约束自己写下的名字决定，不由被约束位的位次决定。
                // 无本模块声明名表（导入函数 / 类型体方法）时退回**位置近似**
                //（`slot = i`）——「被约束位的 binder 就是本位的形参名」在该形态下
                // 是未校验的假设，已作为边界登记（见 09a），本轮不修。
                let slot = if declared.is_some() {
                    let Some(slot) = free_slot(&free[0]) else {
                        continue;
                    };
                    slot
                } else {
                    i
                };
                let Some(arg) = arg_of_slot(slot) else {
                    continue;
                };
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
                (
                    MonoType::Refined {
                        base: base.clone(),
                        constraint:
                            crate::frontend::core::sema::layers::termination::substitute_const_expr(
                                &constraint,
                                std::slice::from_ref(&free[0]),
                                &[arg_expr],
                            ),
                    },
                    free[0].clone(),
                    Some(arg),
                )
            } else if free.len() >= 2 {
                // 多形参关系谓词的调用点义务（#532）：`n: Eq(n, N)` 的自由变量是
                // **多个声明形参名**，先把它们各自绑到对应位的实参，约束才闭合
                //（`Eq(n, N)` → `Eq(42, 7)`）；判定仍走同一条管道。
                //
                // 三条对齐前置，任一不成立 ⇒ 保留既有边界（不按猜测的位置造义务）：
                // ① 被调函数在本模块有声明（名字表命中）；② 是全应用（实参数 =
                // 声明形参数 = 注册签名的形参数）；③ 每个自由变量都是声明的形参名、
                // 且对应位实参可转成编译期表达式。
                let Some(names) = declared.as_deref() else {
                    continue;
                };
                // 全应用（实参数 = 声明形参数 = 注册签名形参数）才谈按位对齐；
                // 部分应用 / 超签的元数诊断归推断层与 IR 层，不在此造义务。
                if names.len() != params.len() || args.len() + named_args.len() != names.len() {
                    continue;
                }
                let mut formals = Vec::with_capacity(free.len());
                let mut actuals = Vec::with_capacity(free.len());
                let mut aligned = true;
                for name in &free {
                    let Some(pos) = free_slot(name) else {
                        aligned = false;
                        break;
                    };
                    let Some(actual) = arg_of_slot(pos).and_then(|a| {
                        crate::frontend::core::types::eval::const_eval::convert_expr_to_const_expr(
                            a,
                        )
                    }) else {
                        aligned = false;
                        break;
                    };
                    formals.push(name.clone());
                    actuals.push(actual);
                }
                if !aligned {
                    continue;
                }
                let substituted = MonoType::Refined {
                    base: base.clone(),
                    constraint:
                        crate::frontend::core::sema::layers::termination::substitute_const_expr(
                            &constraint,
                            &formals,
                            &actuals,
                        ),
                };
                // 缺值闸门（与 `revalidate_refined` 同一条）：代入后仍有自由变量
                // 没有已知值 ⇒ 不送检。符号实参（`f(N, n)`）代入后是开约束，
                // 「证不出」不等于「已证伪」——其证据面归 6.9.4 的 Γ 等式桥。
                if !refined_free_vars(&substituted)
                    .iter()
                    .all(|v| unit.values.contains_key(v))
                {
                    continue;
                }
                // 诊断对象 = 本位**声明写的**形参名（`n: Eq(n, N)` 的 `n`）
                let subject = names.get(i).cloned().unwrap_or_else(|| free[0].clone());
                // 证据取本位实参（既有口径，可缺席）：多元约束的多个自由变量各自
                // 有实参，不把某一侧的标注当整条约束的证据。
                (substituted, subject, arg_of_slot(i))
            } else {
                continue;
            };
            // RFC-027 §3.3：实参**自身的精化标注**是证据（#395 第三行）。
            //
            // 在**本检查局部**注入实参变量已注册的精化约束，查完即弹：不放进
            // 声明处的全局 Γ。原因是重验证路径会自命中——它验的正是同一条约束，
            // 若该约束已在 Γ，判定会在 level 2（精确匹配）平凡通过，**掩盖真违反**
            //（实测：4 个「依赖变量重赋值」负例语料会被静默放过）。
            let arg_refined: Option<crate::frontend::core::types::const_data::ConstExpr> =
                match evidence_arg {
                    Some(Expr::Var(name, _)) => {
                        unit.deps.constraint_of(name).and_then(|ty| match ty {
                            MonoType::Refined { constraint, .. }
                                if !constraint_is_terminates(ty) =>
                            {
                                Some(constraint.clone())
                            }
                            _ => None,
                        })
                    }
                    _ => None,
                };
            // E2031 的 `{constraint}` 槽沿用本位既有口径（注册签名的形参类型）：
            // 它是「哪个形参的精化」的归因文本，不随代入后的闭合式改变。
            let display = param_ty.to_string();
            Self::judge_refined_obligation(
                ctx,
                &substituted,
                &unit.values,
                fn_name,
                &subject,
                &display,
                *span,
                arg_refined,
            );
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
                    crate::frontend::core::sema::layers::termination::substitute_const_expr(
                        &constraint,
                        std::slice::from_ref(&binder),
                        &[returned],
                    );
            }
        }
        // 代入后的约束文本（含 `return` 值的符号形态），进诊断比未代入形态更可读。
        // 它是**约束表达式**（不是自然语言句子），故与 locale 无关。
        let constraint_text = constraint.to_string();
        let substituted = MonoType::Refined { base, constraint };
        // 返回点义务的对象是**返回位**（RFC-027 §3）：此前这里用 `free[0]`（多参数
        // 谓词时那是形参名），文案把约束挂到了错误对象上（实测：`SumUpTo(b, r)`
        // 报成 "'b' 的精化类型约束"）。声明了 binder 就传它的名字（`r` 是返回值
        // 形参，不是签名形参）；裸形态没有声明名，按 RFC-027 §2 取规范名 `result`。
        // 本槽只传**标识符**，任何自然语言措辞都由 locale 模板负责——code 侧不得
        // 写某语言的词，也**不覆盖 help**（E2031 的本地化 help 已给出可操作的替代
        // 写法：实参可静态取值 / 提供返回 Type 的证明函数）。
        let subject = match &ctx.fn_return_binder {
            Some(binder) => binder.clone(),
            None => DEFAULT_RETURN_BINDER.to_string(),
        };
        let fn_name = ctx
            .fn_name
            .clone()
            .unwrap_or_else(|| "<anonymous>".to_string());
        // 作用域内变量的当前值环境进 bindings；形参不在其中，留给 SMT 作符号。
        Self::judge_refined_obligation(
            ctx,
            &substituted,
            &unit.values,
            &fn_name,
            &subject,
            &constraint_text,
            span,
            None, // 返回点没有「实参自身标注」这类局部证据
        );
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
fn strip_type_parens(ty: &crate::ast::Type) -> &crate::ast::Type {
    use crate::ast::Type as AstType;
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
    shared_ctx: &'a mut crate::frontend::core::sema::proof::context::ProofContext<'env>,
    proof_calls: &'a mut Vec<crate::frontend::core::sema::proof::verdict::ProofFunctionCall>,
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
    /// 本模块函数的**声明序运行期形参名**（函数名 → 调用点序列的名字）。
    ///
    /// 多形参关系谓词的调用点义务要把约束里的自由变量换成实参；`MonoType`
    /// 侧只留下自由变量的名字，位次只能回声明 AST 反解（见
    /// [`collect_declared_value_params`]）。缺项 ⇒ 该调用形态保持既有边界。
    declared_value_params: HashMap<String, Vec<String>>,
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
fn collect_assigned_var_names(stmts: &[crate::ast::Stmt]) -> std::collections::HashSet<String> {
    use crate::ast::{Expr, StmtKind};
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
    expr: &crate::ast::Expr,
    assigned: &mut std::collections::HashSet<String>,
) {
    use crate::ast::Expr;
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

/// 收集本模块（含嵌套声明）函数的**声明序运行期形参名**：函数名 → 调用点序列。
///
/// 为什么回到声明 AST：精化约束的自由变量写的是**声明的形参名**
///（`n: Eq(n, N)` 的 `n` / `N`），调用点只有实参位置；`MonoType` 侧已把名字
/// 压成自由变量，位次无法反推。名次口径不另立——`Type::runtime_value_params`
/// 是「调用时按序给几个值」的唯一实现（与 E1112 的元数对照、IR 的调用侧可用名
/// 同源），故与调用点实参按位对齐。
///
/// 只收**函数绑定**（判据与走查同一条：`Expr::block_binding_is_function`）；
/// 方法（类型体绑定）与导入函数不在此表——缺项时调用点保留既有边界。
fn collect_declared_value_params(
    stmts: &[crate::ast::Stmt],
    out: &mut HashMap<String, Vec<String>>,
) {
    use crate::ast::{Expr, StmtKind};
    for stmt in stmts {
        match &stmt.kind {
            StmtKind::Assign {
                target,
                type_annotation,
                value,
                ..
            } => {
                let is_fn = value.as_ref().is_some_and(|v| {
                    matches!(v.as_ref(), Expr::Lambda { .. })
                        || Expr::block_binding_is_function(
                            type_annotation.as_ref(),
                            Some(v.as_ref()),
                        )
                });
                if is_fn {
                    if let (Expr::Var(name, _), Some(ann)) =
                        (target.as_ref(), type_annotation.as_ref())
                    {
                        out.insert(name.clone(), declared_value_param_names(ann));
                    }
                }
                if let Some(v) = value.as_deref() {
                    collect_declared_value_params_in_expr(v, out);
                }
            }
            StmtKind::If {
                then_branch,
                else_if_branches,
                else_branch,
                ..
            } => {
                collect_declared_value_params(&then_branch.stmts, out);
                for (_, body) in else_if_branches {
                    collect_declared_value_params(&body.stmts, out);
                }
                if let Some(eb) = else_branch {
                    collect_declared_value_params(&eb.stmts, out);
                }
            }
            StmtKind::For { body, .. } => collect_declared_value_params(&body.stmts, out),
            StmtKind::DestructureAssign { rhs, .. } => {
                collect_declared_value_params_in_expr(rhs, out)
            }
            StmtKind::Return(Some(expr)) => collect_declared_value_params_in_expr(expr, out),
            StmtKind::Expr(expr) => collect_declared_value_params_in_expr(expr, out),
            _ => {}
        }
    }
}

/// 表达式形态的控制流体内声明收集（值块 / While / For / 闭包体）
fn collect_declared_value_params_in_expr(
    expr: &crate::ast::Expr,
    out: &mut HashMap<String, Vec<String>>,
) {
    use crate::ast::Expr;
    match expr {
        Expr::Block(block) => collect_declared_value_params(&block.stmts, out),
        Expr::While { body, .. } => collect_declared_value_params(&body.stmts, out),
        Expr::For { body, .. } | Expr::SpawnFor { body, .. } => {
            collect_declared_value_params(&body.stmts, out)
        }
        Expr::Lambda { body, .. } => collect_declared_value_params(&body.stmts, out),
        _ => {}
    }
}

/// 声明注解里**调用点序列**的形参名（声明序）。
fn declared_value_param_names(type_ann: &crate::ast::Type) -> Vec<String> {
    type_ann
        .runtime_value_params()
        .into_iter()
        .map(|p| p.name)
        .collect()
}
