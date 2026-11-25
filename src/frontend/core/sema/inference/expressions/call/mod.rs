//! 调用裁决族：调用表达式的推断与裁决入口。
//!
//! 自 `inference/expressions.rs` 纯搬移（WBS 7.0.4）：方法体与文档注释零改动，
//! 仅补跨模块可见性（`pub(super)`）与相对路径（`super::` 改指 crate 路径）。
//! 子模块按方法簇划分：
//! - `args` —— 命名/记录实参绑定；
//! - `instantiation` —— 调用点泛型实例化与类型实参归约；
//! - `method` —— 方法调用绑定键与接收者静态类型。

mod args;
mod instantiation;
mod method;

use crate::frontend::core::sema::judgement::decide;
use crate::frontend::core::sema::passes::overload;
use crate::frontend::core::types::MonoType;
use crate::util::diagnostic::{ErrorCodeDefinition, Result};
use std::collections::{HashMap, HashSet};

use super::{
    body_uses_ctor_head, concrete_type_from_expr_arg, expand_ctor_applications,
    extract_const_value_from_expr, extract_namespace_path, extract_string_literal_from_expr,
    instantiate_with_ctor_heads, is_ctor_reference, substitute_type_params_with_vars,
};
use super::{ConstArgOutcome, ExpressionInferrer};
use crate::frontend::core::sema::inference::arg_binding;
use crate::frontend::core::sema::inference::call_ownership::{self, CallOwnership, ParamOwnership};
use crate::frontend::core::sema::inference::statements;

impl<'a> ExpressionInferrer<'a> {
    /// RFC-010: 变体构造调用检查——`Result(Int, String).ok(5)` / `Color.red()`。
    ///
    /// 前置：func 形如 `FieldAccess { expr: Call(Var(和类型名), 类型实参), field: 变体名 }`
    /// 且和类型名已判定（sum_types）。流程：内层实例化（Call 臂和类型分支产出
    /// `Generic{名, 实参}`）→ 变体查找（声明序）→ 变体参数按类型实参替换 →
    /// 载荷逐参 unify → 返回该和类型。（IR 侧的 CreateVariant 由 ir_gen 就地
    /// 形态检测产生：见 lower/query.rs 的 detect_variant_ctor_call——span 键跨系统
    /// 语义不同，跨层传调用点表会引入失配缺陷，2026-10-13 WBS 3.4.4 裁决删。）
    #[allow(clippy::too_many_arguments)]
    fn infer_variant_ctor_call(
        &mut self,
        type_name: &str,
        variant_name: &str,
        concrete: Vec<MonoType>,
        value_args: &[crate::ast::Expr],
        named_args: &[(String, crate::ast::Expr)],
        span: crate::util::span::Span,
    ) -> Result<MonoType> {
        let Some(variants) = self.sum_types.get(type_name) else {
            return Err(
                ErrorCodeDefinition::field_not_found(variant_name, type_name)
                    .at(span)
                    .build(),
            );
        };
        let Some((_, vdef)) = variants
            .iter()
            .enumerate()
            .find(|(_, v)| v.name == variant_name)
        else {
            return Err(
                ErrorCodeDefinition::field_not_found(variant_name, type_name)
                    .at(span)
                    .build(),
            );
        };
        // 载荷签名 = 变体参数经类型实参替换
        // 非泛型和类型（Color）不在 generic_type_defs，形参表为空
        let param_names = self
            .generic_type_defs
            .get(type_name)
            .map(|d| d.type_param_names.clone())
            .unwrap_or_default();
        let subst_params: Vec<MonoType> = vdef
            .params
            .iter()
            .map(|pty| {
                crate::frontend::core::sema::TypeEnvironment::replace_type_params(
                    pty,
                    &param_names,
                    &concrete,
                )
            })
            .collect();
        if !named_args.is_empty() {
            return Err(ErrorCodeDefinition::type_mismatch(
                "位置实参（变体构造不支持命名参数）",
                &format!("命名参数 {} 个", named_args.len()),
            )
            .at(span)
            .build());
        }
        let value_arg_types: Vec<MonoType> = value_args
            .iter()
            .map(|a| self.infer_expr(a))
            .collect::<Result<_>>()?;
        if value_arg_types.len() != subst_params.len() {
            return Err(ErrorCodeDefinition::argument_count_mismatch(
                variant_name,
                subst_params.len(),
                value_arg_types.len(),
            )
            .at(span)
            .build());
        }
        for (pty, aty) in subst_params.iter().zip(value_arg_types.iter()) {
            if let Err(failure) = decide(
                self.solver,
                pty,
                aty,
                Some(span),
                self.type_defs,
                self.dep_env,
            ) {
                return Err(failure.or_site(
                    ErrorCodeDefinition::type_mismatch(&format!("{}", pty), &format!("{}", aty))
                        .at(span)
                        .build(),
                ));
            }
        }
        Ok(MonoType::Generic {
            name: type_name.to_string(),
            args: concrete,
        })
    }

    /// 函数调用表达式的类型推断（自 `infer_expr` 的 Call 臂抽出）。
    ///
    /// 覆盖面：变体构造拦截、内置容器构造、显式类型实参、闭包形参期望传播、
    /// 重载解析、方法调用糖、`Any` 多态槽位等。
    /// #394：空块表达式判定——`{}`（B 方案定案：空块值 Void，非空字典）
    fn is_empty_block_expr(expr: &crate::ast::Expr) -> bool {
        matches!(
            expr,
            crate::ast::Expr::Block(b) if b.stmts.is_empty()
        )
    }

    pub(super) fn infer_call_expr(
        &mut self,
        func: &crate::ast::Expr,
        args: &[crate::ast::Expr],
        named_args: &[(String, crate::ast::Expr)],
        span: crate::util::span::Span,
        call_plan_id: crate::util::plan_id::PlanId,
    ) -> Result<MonoType> {
        // RFC-010: 变体构造——Result(Int, String).ok(5) / Color.red()。
        // 构造器只以调用形态存在（无一等构造器值），在 infer(func) 之前
        // 拦截，避免 FieldAccess 臂的变体字段拒绝。
        if let crate::ast::Expr::FieldAccess {
            expr: base_expr,
            field: variant_name,
            ..
        } = func
        {
            // 两种 base 形态：泛型 `Call(Var(名), 类型实参)` / 非泛型 `Var(名)`
            let inner = if let crate::ast::Expr::Call {
                func: inner_func, ..
            } = base_expr.as_ref()
            {
                inner_func
            } else {
                base_expr.as_ref()
            };
            if let crate::ast::Expr::Var(type_name, _) = inner {
                if self.sum_types.contains_key(type_name) {
                    let concrete = match base_expr.as_ref() {
                        crate::ast::Expr::Var(_, _) => Vec::new(),
                        _ => {
                            let base_ty = self.infer_expr(base_expr)?;
                            match self.solver.resolve_type(&base_ty) {
                                MonoType::Generic { args, .. } => args,
                                other => {
                                    return Err(ErrorCodeDefinition::type_mismatch(
                                        "和类型实例化结果",
                                        &format!("{}", other),
                                    )
                                    .at(span)
                                    .build())
                                }
                            }
                        }
                    };
                    return self.infer_variant_ctor_call(
                        type_name,
                        variant_name,
                        concrete,
                        args,
                        named_args,
                        span,
                    );
                }
            }
        }

        let func_ty = self.infer_expr(func)?;
        // 方法绑定键的「解析-消费」窗口收缩到本次调用的被调方（E1113 方法面，
        // issue #538 状态位的判据收紧）：被调方是 FieldAccess 时，`infer_expr(func)`
        // 刚写入的键是本次判定唯一合法的键——立即快照（take）；此后实参推断对
        // 状态位的任何覆盖（实参自身的 FieldAccess 臂会清空/改写）都不影响本次
        // 判定。被调方非 FieldAccess 时不存在合法生产者，快照丢弃——调用边界
        // 顺带清除残留，陈旧键不再被任何判定读走。
        let method_binding_key = self
            .last_method_binding_key
            .take()
            .filter(|_| matches!(func, crate::ast::Expr::FieldAccess { .. }));
        // 可调用性校验：被调对象必须是函数（或 LibraryRef）。
        //
        // 此前完全不检——`x = 5; x()` 编译期静默通过，到运行时才报
        // E6006「函数找不到」，与真实原因（值不可调用）风马牛不相及。
        // （#364）
        {
            let ft = self.solver.resolve_type(&func_ty);
            // 可调用 = 函数 / lib 引用 / 未定形（类型变量、泛型、结构体构造器）。
            //
            // 只拦**确定不可调用**者：数值、布尔、字符串、容器等纯数据。
            // 结构体名既可作构造器（`Point(1,2)`）也可作值，不在此处判。
            let definitely_not_callable = matches!(
                ft,
                MonoType::Int(_)
                    | MonoType::Float(_)
                    | MonoType::Bool
                    | MonoType::Char
                    | MonoType::Void
            ) || ft.is_string();
            if definitely_not_callable {
                return Err(ErrorCodeDefinition::not_callable(&format!("{ft}"))
                    .at(span)
                    .build());
            }
        }

        // LibraryRef callable rule: when calling a LibraryRef with a string literal
        // e.g. sqlite3("sqlite3_open") where sqlite3: LibraryRef
        // Returns ExternRef at compile time
        let func_ty_resolved = self.solver.resolve_type(&func_ty);
        if let MonoType::LibraryRef { mechanism, .. } = &func_ty_resolved {
            if args.len() == 1 {
                if let Some(sym) = extract_string_literal_from_expr(&args[0]) {
                    return Ok(MonoType::ExternRef {
                        mechanism: mechanism.clone(),
                        lib: String::new(), // filled at IR gen
                        symbol: sym,
                    });
                }
                return Err(ErrorCodeDefinition::type_mismatch(
                    "String",
                    &format!(
                        "{}",
                        self.infer_expr(&args[0])
                            .unwrap_or_else(|_| self.solver.new_var())
                    ),
                )
                .at(span)
                .build());
            }
            return Err(ErrorCodeDefinition::argument_count_mismatch(
                "LibraryRef callable",
                1,
                args.len(),
            )
            .at(span)
            .build());
        }

        // 闭包作实参的期望类型传播（议题 1 机制的 Call 臂延伸）：
        // 被调签名已知时，形参位就是匿名 lambda 的权威对齐来源
        // （`apply(&xs, (x) => x * 2)` 的 x 从形参位取得类型）。
        // 个数对不上（方法调用的 params 含接收者位、部分应用等）
        // 不传播，行为与历史一致。
        let expected_params: Option<Vec<MonoType>> = match self.solver.resolve_type(&func_ty) {
            MonoType::Fn { params, .. } => Some(params.clone()),
            _ => None,
        };
        let arg_types: Vec<MonoType> = args
            .iter()
            .enumerate()
            .map(|(i, arg)| {
                let expected = expected_params
                    .as_ref()
                    .filter(|ps| ps.len() == args.len())
                    .and_then(|ps| ps.get(i));
                self.infer_expr_with_expected(arg, expected)
            })
            .collect::<Result<Vec<_>, _>>()?;

        // D6.3：显式类型实参（`mk(Int)(5)` 的内层 `mk(Int)`）。
        //
        // 语法上 `Int` 在实参位置是个 `MetaType`（类型宇宙的值）。
        // 若函数签名已声明类型参数（作用域里的 `Fn` 参数/返回含类型变量）
        // 且前导实参全是 `MetaType`，则它们是**类型实参**而非值实参——
        // 不能当值传给 `Fn{params:[t]}`，否则 `A` 绑到 `MetaType`，
        // `mk(Int)(5)` 静默出 void（D6.3）。
        //
        // 判据用「形参里有未绑定类型变量」而非 `lookup_type_params`：
        // 后者依赖 `generic_fn_type_params` 登记（仅在 check_fn_stmt 时填），
        // 单文件里 lambda 形态的绑定未必命中。
        // #452：命名实参的推断类型按**书写序**（与 named_args 同序）——候选逐
        // 形参位匹配时用它与形参类型 unify。此前命名实参只在「实参数与形参数
        // 相等」时被检查，混合形态整体跳过类型检查。
        let named_types: Vec<MonoType> = named_args
            .iter()
            .map(|(_, arg)| self.infer_expr(arg))
            .collect::<Result<Vec<_>, _>>()?;
        let named_types: Vec<&MonoType> = named_types.iter().collect();

        let mut explicit_type_args: Vec<MonoType> = Vec::new();
        let mut value_arg_types: Vec<MonoType> = arg_types.clone();
        {
            let leading = arg_types
                .iter()
                .take_while(|t| Self::is_type_value(t))
                .count();
            // 类型位判定用**递归**扫描：构造器变量的签名里类型变量嵌在应用实参位
            // （`x: F(A)` → `Generic{F,[TypeVar]}`），只看顶层会把这类签名漏掉。
            let fn_has_type_slot = match &func_ty {
                MonoType::Fn {
                    params,
                    return_type,
                } => {
                    let mut indices = HashSet::new();
                    for p in params.iter().chain(std::iter::once(&**return_type)) {
                        Self::collect_type_var_indices(p, &mut indices);
                    }
                    !indices.is_empty()
                }
                _ => false,
            };

            if leading > 0 && fn_has_type_slot && leading <= args.len() {
                // 把类型名实参换成**具体类型**（`Int` → `Int(64)`），
                // 而不是笼统的 `MetaType`：上层 `mk(Int)(5)` 需要的
                // 是已专用化的 `Fn{[Int], Int}`，用 MetaType 绑不出它。
                explicit_type_args = args[..leading].iter().map(Self::type_arg_of_expr).collect();
                value_arg_types = arg_types[leading..].to_vec();
            }
        }

        // 重载解析
        if let crate::ast::Expr::Var(ref name, _) = *func {
            if overload::has_overloads(self.overload_candidates, name) {
                match overload::resolve_overload_from_env(
                    self.overload_candidates,
                    name,
                    &arg_types,
                ) {
                    Ok(candidate) => {
                        return Ok(candidate.return_type.clone());
                    }
                    Err(_e) => {
                        if let Some(generic_candidate) = overload::resolve_generic_fallback(
                            self.overload_candidates,
                            name,
                            &arg_types,
                        ) {
                            let return_type =
                                overload::instantiate_return_type(generic_candidate, &arg_types);
                            return Ok(return_type);
                        }
                        return Ok(self.solver.new_var());
                    }
                }
            }
        }

        // RFC-010: 和类型的类型实参应用——`Result(Int, String)`。
        // func 是 Var、名已判定为和类型、func_ty 是 MetaType（Var 臂开口）
        // 时收成 `Generic{名, [实参…]}`（声明序变体集保留在 sum_types，
        // 变体构造在 FieldAccess+Call 形态处理）。类型表示与 `Result(T,E)`
        // 在类型位置的既有形态一致。
        if let crate::ast::Expr::Var(type_name, _) = func {
            // func_ty 在此处可能是 MetaType（Var 臂开口）或 Struct（类型声明的
            // scope 镜像先命中 Var 臂）——两者都收成 Generic 形态
            if self.sum_types.contains_key(type_name) {
                let type_param_count = self
                    .generic_type_defs
                    .get(type_name)
                    .map(|d| d.type_param_names.len())
                    .unwrap_or(0);
                if args.len() != type_param_count {
                    return Err(ErrorCodeDefinition::argument_count_mismatch(
                        type_name,
                        type_param_count,
                        args.len(),
                    )
                    .at(span)
                    .build());
                }
                let concrete: Vec<MonoType> = args
                    .iter()
                    .zip(value_arg_types.iter())
                    .map(|(a, ty)| {
                        // 类型参数位（泛型方法体内 `Result(T, E)` 的 T/E）：
                        // 解析为同一批新鲜类型变量，构造出的 Generic 与
                        // 值级签名可直接 unify
                        if let MonoType::TypeVar(_) = ty {
                            return Some(ty.clone());
                        }
                        if !matches!(ty, MonoType::MetaType { .. }) {
                            // 用户自定义类型名作实参（`Result(J, Error)` 的 J）：
                            // 类型声明的 scope 镜像先命中 Var 臂，ty 是 Struct
                            // 而非 MetaType。零类型参数的和类型收成与变体构造
                            // 输出一致的 `Generic{名, []}`（TypeRef 与之不 unify）；
                            // 其余按名收名义引用
                            if let crate::ast::Expr::Var(n, _) = a {
                                if self.sum_types.contains_key(n) {
                                    let has_params = self
                                        .generic_type_defs
                                        .get(n)
                                        .map(|d| !d.type_param_names.is_empty())
                                        .unwrap_or(false);
                                    if has_params {
                                        return Some(MonoType::TypeRef(n.clone()));
                                    }
                                    return Some(MonoType::Generic {
                                        name: n.clone(),
                                        args: vec![],
                                    });
                                }
                                if self.generic_type_defs.contains_key(n)
                                    || self.type_defs.contains_key(n)
                                {
                                    return Some(MonoType::TypeRef(n.clone()));
                                }
                            }
                            // 元组类型实参（`Result((Json, Int), Error)`）：语法
                            // 位置的元组即类型元组，元素按类型名收集
                            if let crate::ast::Expr::Tuple(items, _) = a {
                                let parts: Option<Vec<MonoType>> = items
                                    .iter()
                                    .map(|e| match e {
                                        crate::ast::Expr::Var(n, _) => concrete_type_from_expr_arg(
                                            e,
                                            self.type_defs,
                                            self.generic_type_defs,
                                        )
                                        .or_else(|| {
                                            if self.sum_types.contains_key(n) {
                                                let has_params = self
                                                    .generic_type_defs
                                                    .get(n)
                                                    .map(|d| !d.type_param_names.is_empty())
                                                    .unwrap_or(false);
                                                if has_params {
                                                    return Some(MonoType::TypeRef(n.clone()));
                                                }
                                                return Some(MonoType::Generic {
                                                    name: n.clone(),
                                                    args: vec![],
                                                });
                                            }
                                            None
                                        }),
                                        _ => None,
                                    })
                                    .collect();
                                if let Some(ps) = parts {
                                    return Some(MonoType::Generic {
                                        name: "Tuple".to_string(),
                                        args: ps,
                                    });
                                }
                            }
                            return None;
                        }
                        // 具名类型解包失败（Any 等非注册名）时保留
                        // 名义引用 TypeRef(名)——错拼名由类型名校验拦截
                        concrete_type_from_expr_arg(a, self.type_defs, self.generic_type_defs)
                            .or_else(|| match a {
                                crate::ast::Expr::Var(n, _) => Some(MonoType::TypeRef(n.clone())),
                                _ => None,
                            })
                    })
                    .collect::<Option<Vec<_>>>()
                    .ok_or_else(|| {
                        ErrorCodeDefinition::type_mismatch(
                            "类型实参",
                            "值（和类型实参必须是类型名）",
                        )
                        .at(span)
                        .build()
                    })?;
                return Ok(MonoType::Generic {
                    name: type_name.clone(),
                    args: concrete,
                });
            }
        }

        // 单态化：处理编译期泛型参数
        let fn_name_for_mono = match func {
            crate::ast::Expr::Var(n, _) => Some(n.as_str()),
            // 限定名调用（`list.len(v)`）：取函数名才能查声明期类型参数名表
            // 并按声明序绑定签名里的 `TypeRef("A")`；否则实参无法与
            // `&Vec(A)` unify 报 E1002。
            crate::ast::Expr::FieldAccess { field, .. } => Some(field.as_str()),
            _ => None,
        };
        // 内置容器构造器（`Vec(Int)` / `Array(Int, 3)`）的类型实参在值位置
        // 只是 `MetaType` 空壳，具体类型必须按 AST 形态解出（#575）。
        let builtin_ctor_args =
            self.builtin_container_ctor_args(fn_name_for_mono, args, &arg_types);
        let mono_func_ty = self.monomorphize(
            func_ty.clone(),
            &value_arg_types,
            fn_name_for_mono,
            builtin_ctor_args,
        );

        // #394：`{}` 空块落入容器参数位——调用实参位没有 current_expected
        // 可用，但签名形参在 mono 后已知，按位预扫：空块实参 × 容器形参
        // 直接报引导诊断（其余形态不受影响；期望为 Fn 的块实参是合法的
        // 块作值形态，容器判定天然排除）。
        if let MonoType::Fn {
            params: sig_params, ..
        } = self.solver.resolve_type(&mono_func_ty)
        {
            for (idx, arg) in args.iter().enumerate() {
                let Some(param_ty) = sig_params.get(idx) else {
                    break;
                };
                if Self::is_empty_block_expr(arg)
                    && Self::is_container_family(&self.solver.resolve_type(param_ty))
                {
                    return Err(ErrorCodeDefinition::empty_block_as_container()
                        .at(arg.span())
                        .build());
                }
            }
        }

        // D6.3：显式类型实参应用（`mk(Int)` 的内层）。
        //
        // 语义：`mk: (A: Type) -> (x: A) -> A` 里 `A` 是**类型参数**；
        // `mk(Int)` 把 `A` 绑为 `Int`，**返回专用化后的函数**
        // `(x: Int) -> Int`（而非某个值）。因此：
        //   1. 把类型实参绑到形参/返回里的类型变量（顺序对应声明序）；
        //   2. 从实参列表里**移除**它（它不是值实参），否则后面的
        //      逐参 unify 会拿 `MetaType` 去对 `Int` 报 E1002；
        //   3. 返回收敛后的 `Fn`。
        //
        // 单文件里 `A` 在作用域中是 fresh TypeVar（签名收集阶段已剥掉类型层）。
        if !explicit_type_args.is_empty() {
            if let MonoType::Fn {
                params,
                return_type,
            } = &mono_func_ty
            {
                let mut slots: Vec<(usize, MonoType)> = Vec::new();
                for p in params.iter().chain(std::iter::once(&**return_type)) {
                    Self::collect_type_vars_positional(p, &mut slots);
                }
                slots.sort_by_key(|(i, _)| *i);
                slots.dedup_by_key(|(i, _)| *i);
                // 声明序的类型参数名（跨模块导出的登记口；单文件函数在签名收集期填表）。
                let declared_names: Vec<String> = fn_name_for_mono
                    .and_then(|f| self.generic_fn_type_params.get(f).cloned())
                    .unwrap_or_default();
                // 构造器参数（高阶类型参数）在值级签名里**没有槽位**：它的类型实参绑定
                // 到类型应用头部（`F(A)` → `Box(Int)`），不占槽。判据 = 声明的类型参数
                // 个数多于签名中实际出现的类型变量槽位数，且该实参是构造器引用。
                let has_ctor_params = declared_names.len() > slots.len();
                let mut ctor_heads: HashMap<String, String> = HashMap::new();
                let mut slot_cursor = 0usize;
                for (i, ta) in explicit_type_args.iter().enumerate() {
                    if has_ctor_params {
                        if let (Some(dn), Some(head)) = (
                            declared_names.get(i),
                            crate::frontend::core::types::substitute::ctor_head_name(ta),
                        ) {
                            if is_ctor_reference(ta, self.generic_type_defs) {
                                ctor_heads.insert(dn.clone(), head);
                                continue;
                            }
                        }
                    }
                    if let Some((_, slot)) = slots.get(slot_cursor) {
                        let _ = self.solver.unify(slot, ta);
                        slot_cursor += 1;
                    }
                }
                // 构造器变量的头部替换：签名里的 `F(A)` 落成真实构造器应用
                // （`Box(Int)`），再展开成记录形态与值侧同形。
                let expand_heads = !ctor_heads.is_empty();
                let specialize = |ty: MonoType| -> MonoType {
                    let ty = crate::frontend::core::types::substitute::substitute_ctor_heads(
                        &ty,
                        &ctor_heads,
                    );
                    if expand_heads {
                        expand_ctor_applications(&ty, self.generic_type_defs)
                    } else {
                        ty
                    }
                };
                // 类型实参不是值实参：从后续逐参校验中移除
                let bound_params: Vec<MonoType> = params
                    .iter()
                    .map(|p| specialize(self.solver.resolve_type(p)))
                    .collect();
                let bound_ret = specialize(self.solver.resolve_type(return_type));
                // 返回专用化后的函数：curry 尾层已是具体形参
                if bound_params
                    .iter()
                    .all(|p| !matches!(p, MonoType::TypeVar(_)))
                    && !matches!(bound_ret, MonoType::TypeVar(_))
                {
                    self.last_type_args = explicit_type_args.clone();
                    let specialized = MonoType::Fn {
                        params: bound_params,
                        return_type: Box::new(bound_ret),
                    };
                    // 登记实例化请求：单态化阶段靠它特化并保留 `mk` 的
                    // 具体版本。不登记的话 mono 会把未特化的 `mk` 删掉，
                    // 运行时找不到 `mk`（E6006）。
                    self.collect_instantiation_request(
                        &func_ty,
                        func,
                        &explicit_type_args,
                        &specialized,
                        span,
                    );
                    return Ok(specialized);
                }
            }
        }
        explicit_type_args.clear();
        // 清除探针
        let _ = &explicit_type_args;

        // 6.8.5-1（A 路径）+ 6.8.7：const 实参的编译期特化——`f(42)` / `f(7)` 各一份 IR；
        // 同时返回把 const binder 换成字面量后的**返回类型**，使 `f(42)` 的
        // `n: N` 立即定成 `int64::42`（单值类型强制）。命中即返回：
        // 纯 const 泛型不会命中下面那条通用路径（它按 TypeVar / 声明类型名收实参），
        // 与显式类型实参路径（D6.3）同形。
        // 三态（6.9.2-B）：非目标 / 证据齐备（可能带特化）/ 目标但缺证据。
        // 缺证据必须**报错**（E1113），不再静默按运行期传参。
        match self.collect_const_arg_instantiation_request(
            func,
            args,
            named_args,
            span,
            method_binding_key,
        ) {
            ConstArgOutcome::NotTarget | ConstArgOutcome::Evidenced(None) => {}
            ConstArgOutcome::Evidenced(Some(specialized)) => return Ok(specialized),
            ConstArgOutcome::MissingEvidence(diag) => return Err(diag),
        }

        // 收集实例化请求：检测泛型函数调用并记录
        self.collect_instantiation_request(&func_ty, func, &arg_types, &mono_func_ty, span);

        // #335 G3 类型信息流接口：按单态化结果记录调用点所有权
        //（Ref{mutable}→Write/Read 借用，其余 Move；impl 方法 params[0]
        //  为接收者位；std 可变方法首实参为容器写借用）
        if let MonoType::Fn {
            params: sig_params, ..
        } = &mono_func_ty
        {
            let mut co = CallOwnership::default();
            let method_key_early = self.method_binding_call_key(func);
            if method_key_early.is_some() {
                // impl 绑定方法：params[0] = 接收者，实参对应 params[1..]
                co.receiver = sig_params.first().map(ParamOwnership::from_param_type);
                co.args = sig_params
                    .iter()
                    .skip(1)
                    .take(arg_types.len())
                    .map(ParamOwnership::from_param_type)
                    .collect();
            } else {
                co.args = sig_params
                    .iter()
                    .take(arg_types.len())
                    .map(ParamOwnership::from_param_type)
                    .collect();
                // std 可变容器方法：首实参（容器）按写借用处理——
                // 此前缺省 Move 被复制豁免，借用期间的原地修改不检
                if let crate::ast::Expr::FieldAccess {
                    expr: obj, field, ..
                } = func
                {
                    if matches!(**obj, crate::ast::Expr::Var(..))
                        && !co.args.is_empty()
                        && matches!(
                            call_ownership::std_native_receiver_ownership(field),
                            ParamOwnership::WriteBorrow
                        )
                    {
                        co.args[0] = ParamOwnership::WriteBorrow;
                    }
                }
            }
            // 6.9.5-B.1c / 5A：写借用的接收者若是 witness-bearing 记录，整体不可变。
            // 两条子路径收在同一点：impl 方法的接收者位（co.receiver）与 std
            // 容器方法的首实参（co.args[0] 由本函数改写为 WriteBorrow）。
            let writes_receiver = co.receiver == Some(ParamOwnership::WriteBorrow)
                || co.args.first() == Some(&ParamOwnership::WriteBorrow);
            if writes_receiver {
                if let Some(receiver_ty) = self.call_receiver_static_type(func) {
                    if self.is_witness_bearing_type(&receiver_ty) {
                        return Err(ErrorCodeDefinition::witness_record_immutable(&format!(
                            "{}",
                            receiver_ty
                        ))
                        .at(span)
                        .build());
                    }
                }
            }

            self.call_ownership.insert(span, co);
        }

        // #317：FieldAccess 目标若解析自 method_bindings（impl 方法绑定），
        // 接收者占签名 params[0]——arity 按「实参数+1==形参数」校验，
        // 逐参 unify 对齐 params[1..]（下方分发臂）
        let method_key = self.method_binding_call_key(func);

        // RFC-011a §3：方法级重载决议——同名多候选按 实参兼容 +
        // 期望返回 选择。候选的形参变量是 pass2 注册期共享的，
        // 逐调用 fresh 化后再 unify。选中后：
        //   - 以候选签名作为本调用的函数类型（分派/所有权同源）
        //   - 记录决议（**调用点 PlanId** → 混编 IR 名），ir_gen 据此命名调用
        let selected_overload: Option<(MonoType, Vec<String>)> = match &method_key {
            Some(key) => {
                let cands = self.method_overloads.as_ref().and_then(|m| m.get(key));
                let irs = self
                    .method_overload_ir_names
                    .as_ref()
                    .and_then(|m| m.get(key));
                // #452：候选的声明期形参名（与候选序一一对应；缺表/空表 = 该候选
                // 名字不可用，按「无约束」放行——宁可不按名匹配也不拿错位名字匹配）
                let cand_names = self
                    .method_overload_param_names
                    .as_ref()
                    .and_then(|m| m.get(key));
                match (cands, irs) {
                    // #452 / F1：候选**非空**即走决议——单候选也要跑，否则单签名的
                    // 命名实参调用整段跳过逐参类型检查（实测运行时才 E6007）。
                    (Some(cands), Some(irs)) if !cands.is_empty() && irs.len() == cands.len() => {
                        let mut matches: Vec<(usize, MonoType)> = Vec::new();
                        for (i, cand) in cands.iter().enumerate() {
                            let Some(MonoType::Fn { params, .. }) = Some(cand) else {
                                continue;
                            };
                            // 元数：候选值形参数 = 位置实参数 + 命名实参数
                            if params.len() != arg_types.len() + named_args.len() + 1 {
                                continue;
                            }
                            // #452 规则⑥：命名实参按**形参名**约束候选——形参表里
                            // 没有这个名字的候选直接淘汰（形参表为空 = 名字不可用，
                            // 不淘汰）。命名实参必须计入匹配，否则纯命名调用会静默
                            // 落到「默认绑定」（第一个候选），与用户写的名字无关。
                            // #466：判据收敛到共享的 `names_support_named_args`。
                            let names: &[String] = cand_names
                                .and_then(|l| l.get(i))
                                .map(Vec::as_slice)
                                .unwrap_or(&[]);
                            if !Self::names_support_named_args(names, named_args) {
                                continue;
                            }
                            // 逐调用 fresh 化（共享 pass2 变量会跨调用串型）
                            let freshened = {
                                let mut idxs = std::collections::HashSet::new();
                                Self::collect_type_var_indices(cand, &mut idxs);
                                let mut subst = HashMap::new();
                                for idx in idxs {
                                    subst.insert(idx, self.solver.new_var());
                                }
                                // 克隆后在副本上替换：替换实现若含就地改写，
                                // 直接作用在共享候选上会把后一个候选改脏
                                Self::substitute_type_vars(&cand.clone(), &subst)
                            };
                            let Some(MonoType::Fn {
                                params: fp,
                                return_type: fr,
                            }) = Some(&freshened)
                            else {
                                continue;
                            };
                            // 逐形参位类型匹配（#452 规则⑥）：位置实参占前导值形参位、
                            // 命名实参按名入槽，两侧都算进匹配。此前只按位置实参计数，
                            // 纯命名调用恒被元数闸拦下、决议永不产出。
                            //
                            // 两个坑（均已实测）：
                            // ① 候选签名跨调用共享，而 unify 会**就地绑定其中的类型变量**
                            //    ——「先绑后否」会把后一个候选改成上一次的形状
                            //    （实测：Int / String 两候选全被判成匹配 → 假歧义 E1107），
                            //    故匹配在**求解器克隆沙盒**里做，胜出后再提交；
                            // ② 逐位配对必须按**下标**取（iterator zip 在两侧都从 1 号
                            //    元素起时会把形参整体错位一格）。
                            let _ = fr;
                            let mut sandbox = self.solver.clone();
                            let mut bound: Vec<Option<MonoType>> = vec![None; fp.len()];
                            for (k, a) in arg_types.iter().enumerate() {
                                if let Some(slot) = bound.get_mut(1 + k) {
                                    *slot = Some(a.clone());
                                }
                            }
                            let mut ok = true;
                            for (ni, (name, _)) in named_args.iter().enumerate() {
                                let named_ty = named_types[ni];
                                let mut slot = None;
                                for (pi, pn) in names.iter().enumerate().skip(1) {
                                    if pn == name {
                                        slot = bound.get_mut(pi);
                                        break;
                                    }
                                }
                                let Some(slot) = slot else {
                                    ok = false; // 形参表里没有这个名字：候选不相容
                                    break;
                                };
                                *slot = Some(named_ty.clone());
                            }
                            if !ok {
                                continue;
                            }
                            for k in 0..fp.len().saturating_sub(1) {
                                let p = &fp[1 + k];
                                let Some(a) = bound.get(k + 1).and_then(|s| s.as_ref()) else {
                                    continue;
                                };
                                // 参数带借用而实参是值：自动借用形态兼容
                                let pr = sandbox.resolve_type(p);
                                let ar = sandbox.resolve_type(a);
                                let borrow_ok = matches!(&pr, MonoType::Ref { .. })
                                    && !matches!(&ar, MonoType::Ref { .. });
                                if !borrow_ok && sandbox.unify(&ar, &pr).is_err() {
                                    ok = false;
                                    break;
                                }
                            }
                            if !ok {
                                continue;
                            }
                            // 期望返回过滤（带注解绑定的调用点传入）。
                            // F4：先在自己那份沙盒里试 unify——unify 非失败原子
                            // （bind 一落即生效、无回滚），直接打在 self.solver 上时
                            // 失败路径会留下半截绑定，污染后续候选与同名调用。
                            if let Some(exp) = &self.current_expected {
                                let er = self.solver.resolve_type(exp);
                                let frr = sandbox.resolve_type(fr);
                                if sandbox.unify(&frr, &er).is_err() {
                                    continue;
                                }
                            }
                            matches.push((i, freshened));
                        }
                        // 把「选中候选的形参名 + 实参→形参位绑定」带出决议段：
                        // 冲突/缺参校验与逐参类型对齐都以它为准（#452）。
                        let chosen: Option<(usize, MonoType)> = match matches.len() {
                            1 => match (
                                matches.first(),
                                matches.first().and_then(|(i, _)| irs.get(*i)),
                            ) {
                                (Some((i, f)), Some(ir)) => {
                                    // 沙盒里判出的匹配要在求解器里**补做一次**
                                    // unify：候选胜出后其类型变量必须真的绑定，
                                    // 否则外层拿到的签名仍是未绑定变量
                                    // （形参位是变量的候选会因此丢约束）。
                                    let mut real = self.solver.clone();
                                    if let MonoType::Fn { params: rp, .. } = f {
                                        let mut bound_vals: Vec<Option<MonoType>> =
                                            vec![None; rp.len()];
                                        for (k, a) in arg_types.iter().enumerate() {
                                            if let Some(slot) = bound_vals.get_mut(1 + k) {
                                                *slot = Some(a.clone());
                                            }
                                        }
                                        for (ni, (name, _)) in named_args.iter().enumerate() {
                                            let nty = named_types[ni].clone();
                                            let mut slot = None;
                                            for (pi, pn) in cand_names
                                                .and_then(|l| l.get(*i))
                                                .map(Vec::as_slice)
                                                .unwrap_or(&[])
                                                .iter()
                                                .enumerate()
                                                .skip(1)
                                            {
                                                if pn == name {
                                                    slot = bound_vals.get_mut(pi);
                                                    break;
                                                }
                                            }
                                            if let Some(slot) = slot {
                                                *slot = Some(nty);
                                            }
                                        }
                                        for (p, a) in rp[1..].iter().zip(bound_vals.iter()) {
                                            if let Some(a) = a {
                                                let _ = real.unify(a, p);
                                            }
                                        }
                                    }
                                    *self.solver = real;
                                    // RFC-039 D20：键 = 调用表达式节点的身份（产出侧唯一写入点）
                                    self.overload_resolutions.push((call_plan_id, ir.clone()));
                                    Some((*i, f.clone()))
                                }
                                _ => None,
                            },
                            // 多候选无法区分 = 隐式假设调用者想要哪一个——
                            // 不该隐式的地方不隐式：歧义即报错（E1107）
                            n if n > 1 => {
                                return Err(ErrorCodeDefinition::ambiguous_method_overload(key, n)
                                    .at(span)
                                    .build());
                            }
                            _ => None, // 0 = 无实参匹配的候选（落旧路径报错）
                        };
                        chosen.map(|(i, f)| {
                            let names = cand_names
                                .and_then(|n| n.get(i))
                                .cloned()
                                .unwrap_or_default();
                            (f, names)
                        })
                    }
                    _ => None,
                }
            }
            None => None,
        };
        // 命中决议时，候选形参名随签名一并带出（未命中 = 无可信名字来源）
        let (selected_overload, selected_param_names) = match selected_overload {
            Some((ty, names)) => (Some(ty), Some(names)),
            None => (None, None),
        };
        // 选中候选的形参数（空形参名表要靠它结算元数：把空表读成「元数 0」
        // 会把合法的 t.pick(1) 误报成 expected 0 arguments）
        let selected_params_len: Option<usize> = match &selected_overload {
            Some(MonoType::Fn { params, .. }) if !params.is_empty() => Some(params.len()),
            _ => None,
        };

        // #452：命名实参 → 方法形参槽位的绑定。返回**逐实参的形参位**：位置实参占
        // 前导槽位、命名实参按名入槽。冲突 / 未知名 / 重复命名 / 命名后接位置实参 /
        // 缺实参在此一律编译期报错——**不得静默择一**。
        let mut param_positions: Option<Vec<usize>> = None;
        // 判定出的形参表元数（值形参数）——元数结算的权威值；不用 positions 的最大
        // 下标推（超出形参表的实参也会推出同样大的下标，见 p10 超参形态）
        let mut bound_arity: Option<usize> = None;
        // #466：决议零命中但命名实参唯一相容某候选时，该候选的签名——
        // 逐参类型检查的归因上下文随它（E1002 报「expected <该候选形参类型>」），
        // 不随默认绑定的末候选：名字所在的候选才是用户写的那个重载。
        let mut fallback_sig: Option<MonoType> = None;
        if let Some(key) = &method_key {
            // #466：未知名预检（名称优先于元数与槽位绑定）——名字不在**任何**
            // 候选的形参表里时，E1010/槽位错配都是误导，先报 E1014；available
            // 列出全部候选形参名的并集——只列首候选（旧预检读元数组首候选名表）
            // 会把用户往错误的重载上引（#466 复现即被 `(available: n)` 误导）。
            // 名字可用的候选一个都没有（全空表）⇒ 无可信名字来源，不检。
            if let Some(lists) = self
                .method_overload_param_names
                .as_ref()
                .and_then(|m| m.get(key))
            {
                if lists.iter().any(|l| !l.is_empty()) {
                    let mut known: Vec<&str> = Vec::new();
                    for name in lists.iter().flat_map(|l| l.iter().skip(1)) {
                        if !name.is_empty() && !known.contains(&name.as_str()) {
                            known.push(name.as_str());
                        }
                    }
                    if let Some((bad, _)) = named_args
                        .iter()
                        .find(|(n, _)| !known.contains(&n.as_str()))
                    {
                        return Err(ErrorCodeDefinition::unknown_argument(
                            key,
                            bad,
                            &known.join(", "),
                        )
                        .at(span)
                        .build());
                    }
                }
            }
            // 名字来源（按可信度）：① 决议选中的候选（多候选时唯一可信）；
            // ② 该键下只有一条候选签名（缺参/超参正是要报的形态，不能拿实参数
            //    去筛形参表，否则「缺参」这一格永远筛不到名字）⇒ 取它的名字；
            // ③ 元数与实参数相符的候选中按 [`Self::names_support_named_args`]
            //    筛（与决议循环同一判据）：唯一相容 ⇒ 名字与签名都随它；
            //    全不相容（名字存在于其它元数的候选）⇒ 取该元数组首候选的
            //    名字，E1014 报在元数相符的重载上；多个相容且名字全同 ⇒ 取之；
            //    多个相容而名字互异 ⇒ 名字不可判定，留空不猜；
            // ④ 其余（无元数相符组——缺参/超参）⇒ 取首个非空名表，让元数结算
            //    （E1010 的期望元数）有上下文。
            //
            // #452 / F1：②③④ 是「多候选但无一命中」路径的落点——决议零命中时
            // 名字不随签名带出，若就此留空，逐参类型检查与元数结算会被整段跳过，
            // 命名实参调用在编译期静默通过（实测 run 打印错值 / void）。
            let names: Option<Vec<String>> = match selected_param_names {
                Some(names) => Some(names),
                None => {
                    let lists = self
                        .method_overload_param_names
                        .as_ref()
                        .and_then(|m| m.get(key));
                    match lists {
                        None => None,
                        Some(lists) => {
                            let total_args = arg_types.len() + named_args.len();
                            let cands = self.method_overloads.as_ref().and_then(|m| m.get(key));
                            let (names, sig) = Self::fallback_named_arg_context(
                                lists, cands, total_args, named_args,
                            );
                            fallback_sig = sig;
                            names
                        }
                    }
                }
            };
            if let Some(names) = names {
                // 空表 = 名字不可用（登记期对不齐留空）：此时**只能**从选中候选的
                // 签名结算元数，不能把空表读成「元数 0」
                bound_arity = if names.is_empty() {
                    selected_params_len.map(|len| len - 1)
                } else {
                    // names 含接收者位 self（下标 0）——绑定与 IR 重排同用全形参表坐标
                    Some(names.len() - 1)
                };
                if !names.is_empty() {
                    param_positions =
                        Some(Self::bind_named_args(key, &names, args, named_args, span)?);
                }
            }
        }
        let mono_func_ty = selected_overload.or(fallback_sig).unwrap_or(mono_func_ty);

        // 两层调用：Container(Int)(42, 43) —— func 是泛型类型构造调用，
        // 内层已完成实例化（func_ty 是具体 Struct），外层实参是构造参数。
        if let crate::ast::Expr::Call {
            func: inner_func, ..
        } = func
        {
            if let crate::ast::Expr::Var(inner_name, _) = &**inner_func {
                if self.generic_type_defs.contains_key(inner_name) {
                    if let MonoType::Struct(st) = &func_ty {
                        // 空构造 X(参数)()：字段取默认值/零值（RFC §9.3 模式），合法
                        if arg_types.is_empty() && named_args.is_empty() {
                            return Ok(func_ty.clone());
                        }
                        // 构造参数 = **数据字段**（witness 不占槽位）；位置/命名/缺参/
                        // 超参/争槽全部交共享判据（与普通构造同一入口），逐字段类型
                        // 检查同源（`check_ctor_field_type`，Vec 字面量豁免只留一处）。
                        let data_fields: Vec<crate::frontend::core::types::mono::StructFieldInfo> =
                            st.fields
                                .iter()
                                .filter(|field| {
                                    !crate::frontend::core::types::mono::quantity_is_witness(
                                        field.quantity,
                                    )
                                })
                                .cloned()
                                .collect();
                        let param_names: Vec<String> =
                            data_fields.iter().map(|field| field.name.clone()).collect();
                        let optional: Vec<bool> =
                            data_fields.iter().map(|field| field.has_default).collect();
                        let positions = Self::bind_record_ctor_args(
                            inner_name,
                            &param_names,
                            &optional,
                            args,
                            named_args,
                            span,
                        )?;
                        for (arg_index, slot) in positions.iter().enumerate() {
                            let named_offset = arg_index.saturating_sub(arg_types.len());
                            let arg_ty = if arg_index < arg_types.len() {
                                arg_types[arg_index].clone()
                            } else {
                                named_types
                                    .get(named_offset)
                                    .map(|ty| (**ty).clone())
                                    .unwrap_or(MonoType::Void)
                            };
                            let arg_expr = if arg_index < args.len() {
                                &args[arg_index]
                            } else {
                                match named_args.get(named_offset) {
                                    Some((_, expr)) => expr,
                                    None => continue,
                                }
                            };
                            let Some(field) = data_fields.get(*slot) else {
                                continue;
                            };
                            self.check_ctor_field_type(&field.ty, &arg_ty, arg_expr, span)?;
                        }
                        return Ok(func_ty.clone());
                    }
                }
            }
        }

        // 泛型类型构造调用分派（SPEC type-system.md §4.3）：
        // 实参自左向右逐位匹配类型声明参数（Type 位收类型实参，const 位收字面量）。
        //   - 全匹配 → 类型构造实例化
        //   - 部分匹配（至少一位匹配上）→ 按类型构造报错：逐位检查，先报第一个错误位
        //   - 完全匹配不上 → 构造参数（自动生成的构造函数）：位置式按字段顺序填，
        //     类型参数从元素类型自动解包；const 位无法自动解包时报错。
        if let crate::ast::Expr::Var(ref fn_name, _) = *func {
            if let Some(generic_def) = self.generic_type_defs.get(fn_name).cloned() {
                // 构造/实例化用的记录体取自**声明模板**而不是 func_ty：泛型类型
                // 构造器名在值位置的类型已是类型级函数 (T: Type) -> Type（见 Var 臂
                // 的 type_ctor_value_type），记录体只有声明模板一处。
                if let crate::frontend::core::types::MonoType::Struct(struct_body) =
                    &generic_def.poly.body
                {
                    let type_param_count = generic_def.type_param_names.len();
                    let const_param_count = generic_def.poly.const_binders.len();
                    let total_params = type_param_count + const_param_count;

                    // 位匹配判定：Type 位收 MetaType 实参，const 位收字面量实参。
                    // 部分匹配 = 存在能填某个声明参数位的实参（MetaType 或
                    // const 位可收的字面量）；全匹配 = 位置对齐逐位吻合。
                    // 位匹配判定：Type 位收的实参是**类型值**——既可以是 `Int` 这样
                    // 的具体类型（infer 成 MetaType），也可以是 `Box` / `Vec` 这样的
                    // 类型构造器（infer 成类型级函数 (T: Type) -> Type，RFC-010 元类型
                    // 宇宙的 Type1 值）。判定与解包必须同源：都用 concrete_type_from_expr_arg。
                    let is_type_arg_expr = |i: usize| {
                        matches!(arg_types[i], MonoType::MetaType { .. })
                            || concrete_type_from_expr_arg(
                                &args[i],
                                self.type_defs,
                                self.generic_type_defs,
                            )
                            .is_some()
                    };
                    let meta_count = (0..arg_types.len())
                        .filter(|i| is_type_arg_expr(*i))
                        .count();
                    let lit_count = args
                        .iter()
                        .filter(|a| extract_const_value_from_expr(a).is_some())
                        .count();
                    let all_matched = args.len() == total_params
                        && (0..type_param_count).all(is_type_arg_expr)
                        && (type_param_count..total_params)
                            .all(|i| extract_const_value_from_expr(&args[i]).is_some());
                    let any_matched = meta_count > 0 || (const_param_count > 0 && lit_count > 0);

                    if all_matched {
                        // === 类型构造（全匹配）：SafeArray(Int, 3) / Container(Int) ===
                        let mut full_args = arg_types.clone();
                        // 类型实参解包：表达式位置的类型名 infer 成 MetaType 空壳
                        // （不存具体类型名），从 AST 实参名提取具体类型。
                        for i in 0..type_param_count {
                            // 类型实参解包：表达式位置的类型名 infer 成 MetaType 空壳
                            // （不存具体类型名）；类型构造器名 infer 成类型级函数。两者
                            // 都要从 AST 实参名解包成类型位要的具体类型 / 构造器名。
                            if let Some(concrete) = concrete_type_from_expr_arg(
                                &args[i],
                                self.type_defs,
                                self.generic_type_defs,
                            ) {
                                full_args[i] = concrete;
                            }
                        }
                        // const 参数需要 MonoType::Literal，而非 MonoType::Int
                        for (i, binder) in generic_def.poly.const_binders.iter().enumerate() {
                            let arg_idx = type_param_count + i;
                            if let Some(arg) = full_args.get_mut(arg_idx) {
                                if !matches!(arg, MonoType::Literal { .. }) {
                                    // 尝试从表达式提取字面量值
                                    if let Some(lit) = args.get(arg_idx) {
                                        if let Some(value) = extract_const_value_from_expr(lit) {
                                            *arg = MonoType::Literal {
                                                name: format!("{}", value),
                                                base_type: Box::new(arg.clone()),
                                                value,
                                            };
                                        }
                                    }
                                }
                            }
                            let _ = binder; // 避免未使用警告
                        }
                        return match crate::frontend::core::sema::TypeEnvironment::instantiate_generic_type(
                            &generic_def,
                            &full_args,
                        ) {
                            Ok(inst) => {
                                // 3.4.1：不可求值 const 约束事实上达（W1063 汇聚链）
                                self.unevaluable_const_constraints.extend(
                                    inst.unevaluable_constraints
                                        .iter()
                                        .map(|c| (span, c.clone())),
                                );
                                Ok(instantiate_with_ctor_heads(
                                    inst.ty,
                                    &generic_def,
                                    self.generic_type_defs,
                                ))
                            }
                            Err(e) => Err(e),
                        };
                    }

                    if any_matched {
                        // === 部分匹配 → 一层处理：逐位检查，先报第一个错误位 ===
                        // （Matrix(42)：位0 T←42 不匹配 → 报位0，即使位1 Rows 可匹配）
                        let check_len = total_params.min(args.len());
                        for i in 0..check_len {
                            let ok = if i < type_param_count {
                                is_type_arg_expr(i)
                            } else {
                                extract_const_value_from_expr(&args[i]).is_some()
                            };
                            if !ok {
                                let pname = if i < type_param_count {
                                    generic_def.type_param_names[i].clone()
                                } else {
                                    generic_def.poly.const_binders[i - type_param_count]
                                        .name
                                        .clone()
                                };
                                let expected = if i < type_param_count {
                                    "类型实参".to_string()
                                } else {
                                    "编译期常量".to_string()
                                };
                                return Err(ErrorCodeDefinition::type_mismatch(
                                    &format!("{}（{}）", pname, expected),
                                    &format!("{}", arg_types[i]),
                                )
                                .at(span)
                                .build());
                            }
                        }
                        // 已匹配的位全部正确：缺参或超参
                        return Err(ErrorCodeDefinition::argument_count_mismatch(
                            fn_name,
                            total_params,
                            args.len(),
                        )
                        .at(span)
                        .build());
                    }

                    // === 完全匹配不上 → 构造参数（自动生成的构造函数）===
                    // 位置式按字段顺序填；类型参数从元素自动解包。
                    // const 位无法从元素解包 → 必须显式两层 Matrix(Int, 3, 4)(...)
                    if const_param_count > 0 {
                        return Err(ErrorCodeDefinition::type_mismatch(
                            &format!(
                                "{}（显式类型构造参数，如 {}(类型, ...)(构造参数)",
                                fn_name, fn_name
                            ),
                            "构造参数值（编译期值参数无法从元素自动解包）",
                        )
                        .at(span)
                        .build());
                    }

                    // === 值构造（二层，无 const）：Container(42, 43) ===
                    // #287：arity = 构造参数数（有默认值字段可省略），
                    // 类型参数从字段值类型推断。
                    {
                        let total = struct_body.fields.len();
                        let required = struct_body.fields.iter().filter(|f| !f.has_default).count();
                        let provided = arg_types.len();
                        if provided < required || provided > total {
                            return Err(ErrorCodeDefinition::argument_count_mismatch(
                                fn_name, total, provided,
                            )
                            .at(span)
                            .build());
                        }

                        // 类型参数推断：TypeRef(param) → 独立 fresh TypeVar（同一参数共享）
                        // → unify 字段类型与实参类型 → resolve 出类型参数具体值。
                        let mut param_vars: HashMap<String, MonoType> = HashMap::new();
                        for pname in &generic_def.type_param_names {
                            param_vars.insert(pname.clone(), self.solver.new_var());
                        }
                        // 头部位置匹配（WBS 3.5.1）：声明体里的 `F(A)`，F 是构造器
                        // 参数名——F 只能绑定实参**类型的头部构造器**（`[1,2]` 的
                        // `Vec`、`Box(1)` 的 `Box`），不参与普通变量位匹配。
                        for (i, field_info) in struct_body.fields.iter().enumerate() {
                            let field_ty = &field_info.ty;
                            if i >= provided {
                                break;
                            }
                            let Some(arg_ty) = arg_types.get(i) else {
                                break;
                            };
                            Self::bind_ctor_param_heads(
                                field_ty,
                                arg_ty,
                                &generic_def.type_param_names,
                                &mut param_vars,
                            );
                        }
                        for (i, field_info) in struct_body.fields.iter().enumerate() {
                            let field_ty = &field_info.ty;
                            if i >= provided {
                                break;
                            }
                            let Some(arg_ty) = arg_types.get(i) else {
                                break;
                            };
                            let subst = substitute_type_params_with_vars(field_ty, &param_vars);
                            // 构造器变量头部替换后产出的是**应用形态**（`Box(Int)`），
                            // 实际值却是记录（`Struct{Box, ...}`）——先展开成记录形态再比。
                            let subst = if body_uses_ctor_head(
                                &generic_def.poly.body,
                                &generic_def.type_param_names,
                            ) {
                                expand_ctor_applications(&subst, self.generic_type_defs)
                            } else {
                                subst
                            };
                            if let Err(failure) = decide(
                                self.solver,
                                &subst,
                                arg_ty,
                                Some(span),
                                self.type_defs,
                                self.dep_env,
                            ) {
                                return Err(failure.or_site(
                                    ErrorCodeDefinition::type_mismatch(
                                        &format!("{}", subst),
                                        &format!("{}", arg_ty),
                                    )
                                    .at(span)
                                    .build(),
                                ));
                            }
                        }
                        let type_args: Vec<MonoType> = generic_def
                            .type_param_names
                            .iter()
                            .map(|p| {
                                param_vars
                                    .get(p)
                                    .map(|v| self.solver.resolve_type(v))
                                    .unwrap_or_else(|| MonoType::TypeRef(p.clone()))
                            })
                            .collect();
                        // 类型实参含未绑定类型参数（`L(T)`，T 是所在泛型的参数）时
                        // 不展开为 Struct：注解侧对同一形态也保持 `Generic`（见
                        // try_instantiate_generic_type），两侧表示须一致才能 unify。
                        // 全部实参具体时仍展开——与顶层 `L(Int)(...)` 路径一致。
                        // 未绑定类型参数（含所在泛型的参数）时保持 Generic 形态；但
                        // **构造器引用**（`Vec` / `Box`）是类型实参本身，不算未绑定，
                        // 否则推断路径与注解路径的表示分叉（一侧 Generic、一侧 Struct）。
                        if type_args.iter().any(|t| {
                            statements::contains_unresolved_param(t)
                                && !is_ctor_reference(t, self.generic_type_defs)
                        }) {
                            return Ok(MonoType::Generic {
                                name: fn_name.clone(),
                                args: type_args,
                            });
                        }
                        return match crate::frontend::core::sema::TypeEnvironment::instantiate_generic_type(
                            &generic_def,
                            &type_args,
                        ) {
                            Ok(inst) => {
                                // 3.4.1：不可求值 const 约束事实上达（W1063 汇聚链）
                                self.unevaluable_const_constraints.extend(
                                    inst.unevaluable_constraints
                                        .iter()
                                        .map(|c| (span, c.clone())),
                                );
                                Ok(instantiate_with_ctor_heads(
                                    inst.ty,
                                    &generic_def,
                                    self.generic_type_defs,
                                ))
                            }
                            Err(e) => Err(e),
                        };
                    }
                }
            }
        }

        // 效应消费：成功调用后向流敏感 Γ 注入谓词
        // （如 std.assert(x > 0) 成功后把 x > 0 加入 Γ）
        if let crate::ast::Expr::Var(ref fn_name, _) = *func {
            if let Some(dep_env) = self.dep_env {
                if let Some(spec) = dep_env.get_effect_spec(fn_name) {
                    // `Effect` 目前只有一个变体 GammaAssume——直接解构，
                    // 不用 `if let`（那需要 `#[allow(irrefutable_let_patterns)]`，
                    // 而那个 allow 掩盖的是「本可写成 let 的多余判断」）。
                    for crate::frontend::core::types::eval::dependent_types::Effect::GammaAssume { predicate_arg } in &spec.effects {
                            if let Some(arg_expr) = args.get(*predicate_arg) {
                                if let Some(pred) = crate::frontend::core::types::eval::const_eval::convert_expr_to_const_expr(arg_expr) {
                                    // F-1：证据面事实带**绑定身份快照**（自由名逐个按值
                                    // 环境同一解析链取最内层绑定），读取侧据此校验。
                                    let snapshot = crate::frontend::core::sema::proof::assumptions::binding_snapshot(self.scope, &pred);
                                    if let Some(gamma) = self.gamma.as_deref_mut() {
                                        gamma.inject_boundings(pred, snapshot);
                                    }
                                }
                            }
                        }
                }
            }
        }
        // #271#1：统一参数个数检查（构造器缺参/超参、函数缺参/超参）。
        // 泛型类型构造器（List(1,2,3) 值构造）豁免——其参数语义是类型参数+值参数，
        // 不适用字段计数（RFC-011），在 match 前拦下。
        if let crate::ast::Expr::Var(ref fn_name, _) = *func {
            // 豁免：泛型类型构造器（List(1,2,3) 值构造是 RFC-011 语义，非字段计数）。
            if !self.generic_type_defs.contains_key(fn_name) {
                let provided = arg_types.len();
                // #387：native arity 从整族豁免收紧为按签名 `?`/`...` 解析的
                // [min, max] 区间。表区间与签名参数数矛盾视为用户同名遮蔽，
                // 回退严格计数；无表项的 native（手注 FFI 签名路径）维持旧宽容。
                let native_range = self
                    .native_arity
                    .get(fn_name)
                    .copied()
                    .filter(|(min, max)| {
                        matches!(&mono_func_ty, MonoType::Fn { params, .. } if {
                            let n = params.len();
                            *min <= n && max.is_none_or(|m| n <= m)
                        })
                    });
                match &mono_func_ty {
                    MonoType::Struct(st) => {
                        // 只有**记录类型**的构造才按 Σ 语义结算（数据字段绑定 +
                        // 逐字段类型检查）。模块命名空间/导出面也以 Struct 形态
                        // 挂在 vars 上（use std.assert 后裸名 assert(...) 即此
                        // 形态），它们没有记录模型——保留旧的 arity 宽容，不把
                        // 「命名空间值被调用」误判成记录构造的字段类型错。
                        let is_declared_record = matches!(
                            self.type_defs.get(&st.name).map(|poly| &poly.body),
                            Some(MonoType::Struct(_))
                        );
                        if !is_declared_record {
                            let total = st.fields.len();
                            let required =
                                st.fields.iter().filter(|field| !field.has_default).count();
                            if named_args.is_empty() && (provided < required || provided > total) {
                                return Err(ErrorCodeDefinition::argument_count_mismatch(
                                    &st.name, total, provided,
                                )
                                .at(span)
                                .build());
                            }
                            return Ok(func_ty.clone());
                        }
                        // 普通 struct 构造器：Point(1.0, 2.0)。构造参数 = **数据字段**
                        // （witness 字段不占槽位，B.2）；位置/命名/争槽/未知名/重复名/
                        // 缺参/超参统一交共享判据，命名未知不再静默丢弃（#271#1 的
                        // bespoke 计数退场）。逐字段类型检查是 D59 C 类修复：此前
                        // Point("a", 2) 编译期通过、运行期才静默给错答案。
                        let data_fields: Vec<crate::frontend::core::types::mono::StructFieldInfo> =
                            st.fields
                                .iter()
                                .filter(|field| {
                                    !crate::frontend::core::types::mono::quantity_is_witness(
                                        field.quantity,
                                    )
                                })
                                .cloned()
                                .collect();
                        let param_names: Vec<String> =
                            data_fields.iter().map(|field| field.name.clone()).collect();
                        let optional: Vec<bool> =
                            data_fields.iter().map(|field| field.has_default).collect();
                        let positions = Self::bind_record_ctor_args(
                            &st.name,
                            &param_names,
                            &optional,
                            args,
                            named_args,
                            span,
                        )?;
                        for (arg_index, slot) in positions.iter().enumerate() {
                            let named_offset = arg_index.saturating_sub(arg_types.len());
                            let arg_ty = if arg_index < arg_types.len() {
                                arg_types[arg_index].clone()
                            } else {
                                named_types
                                    .get(named_offset)
                                    .map(|ty| (**ty).clone())
                                    .unwrap_or(MonoType::Void)
                            };
                            let arg_expr = if arg_index < args.len() {
                                &args[arg_index]
                            } else {
                                match named_args.get(named_offset) {
                                    Some((_, expr)) => expr,
                                    None => continue,
                                }
                            };
                            let Some(field) = data_fields.get(*slot) else {
                                continue;
                            };
                            self.check_ctor_field_type(&field.ty, &arg_ty, arg_expr, span)?;
                        }
                    }
                    MonoType::Fn { params, .. } => {
                        // 普通函数调用：add(5) 缺参 → E6007 运行时错（晚且误导）；
                        // add(1,2,3) 超参静默丢弃。拦为编译期 E1010。
                        // 命名参数也计入总数：`add(a = 1, b = 2)` 传了 2 个。
                        let range = native_range.or_else(|| {
                            if self.native_signatures.contains_key(fn_name) {
                                None // 无表项 native：维持旧宽容
                            } else if params.is_empty() {
                                // lambda/块函数绑定（mk: (Int,Int)->Int = (x,y)=>x+y）
                                // 在 scope 里参数类型丢失（params 为空），计数不可靠，
                                // 跳过避免误伤（#271 记 lambda 绑定参数丢失）
                                None
                            } else {
                                Some((params.len(), Some(params.len())))
                            }
                        });
                        if let Some((min_arity, max_arity)) = range {
                            let total = provided + named_args.len();
                            if total < min_arity || max_arity.is_some_and(|m| total > m) {
                                let expected = if total < min_arity {
                                    min_arity
                                } else {
                                    max_arity.unwrap_or(min_arity)
                                };
                                return Err(ErrorCodeDefinition::argument_count_mismatch(
                                    fn_name, expected, total,
                                )
                                .at(span)
                                .build());
                            }
                        }
                    }
                    _ => {}
                }
            }
        }
        // #317：方法调用 arity 检查——#271#1 的 Var 门槛覆盖不到 FieldAccess
        // 形态。接收者占 params[0]，实参数须等于 params.len()-1；不符拦为
        // 编译期 E1010（原先缺参拖到运行时 E6007、超参/类型错整体静默跳过，
        // 超参仅因 3==params.len() 错位对齐意外报错）
        // #452 / F1：命名实参同样计入——此前的 `named_args.is_empty()` 门让
        // 「带命名实参 + 决议零命中」整段跳过元数结算（实参被静默丢弃或补零）。
        // 期望元数取自**判定出的形参表**（bound_arity / 候选元数集合），拿不到
        // 才退回当前签名；只对元数明显不符响亮，元数相符的形态交给上面的
        // bind_named_args（缺参 E1010 / 未知名 E1014 / 撞槽 E1111）与逐参类型
        // 检查（E1002）报更精确的码。
        if let Some(method_key) = &method_key {
            // 候选形参表按元数分组（每组首候选作代表；空表不构成证据）
            let arity_groups: Vec<(usize, Vec<String>)> = self
                .method_overload_param_names
                .as_ref()
                .and_then(|m| m.get(method_key))
                .map(|lists| Self::candidate_arity_groups(lists))
                .unwrap_or_default();
            let bind_names: Option<&Vec<String>> = arity_groups.first().map(|(_, n)| n);
            let expected_arity: Option<usize> = bound_arity.or_else(|| match bind_names {
                // 有候选表：以候选的元数集合为权威。全为空表（名字不可用，
                // 如类型体绑定形态）⇒ 元数无可信来源，不报。
                Some(names) => Some(names.len() - 1),
                // 无候选表（调用点为单签名）：退回当前签名
                None => match &mono_func_ty {
                    MonoType::Fn { params, .. } if !params.is_empty() => Some(params.len() - 1),
                    _ => None,
                },
            });
            let total_args = arg_types.len() + named_args.len();
            // #466：未知名的预检已前移到槽位绑定之前（全候选名并集版）——
            // 这里只结算元数；名字层面的判定不得再以元数组首候选名表为据。
            if let Some(expected) = expected_arity {
                if total_args != expected {
                    return Err(ErrorCodeDefinition::argument_count_mismatch(
                        method_key, expected, total_args,
                    )
                    .at(span)
                    .build());
                }
            }
        }
        // #387：命名空间限定调用 arity 同规。RFC-029 §1/§6——模块 = record of
        // bindings，对 typecheck 而言 `use std.io.{println}` 与用户模块操作完全
        // 一样，native 特殊处理只属于 IR gen / codegen；此前 FieldAccess 形态
        // 绕过 #271#1（Var 臂）与 #317（method_key 臂），超签实参静默接受、
        // 落 ownership 层按 Move 兜底、运行时丢弃。判别两态：
        // - 命中 native arity 表（直接或 `std.` 前缀回退）→ 命名空间 native，
        //   区间严格，无接收者豁免（表区间与签名参数数矛盾视为同名遮蔽，回退
        //   严格计数）；
        // - 未命中表 → 先判命名空间还是值接收者：限定名根段命中模块别名
        //   （判定与 E1043 同款，#396）即模块限定调用——RFC-029 §6 模块同质，
        //   无接收者，严格计数；`+1` 启发式（显式实参数+1==形参数，接收者占
        //   params[0]，接口派发/impl 方法/Try 方法同族，#317 同款约定）只属于
        //   值接收者形态。params 为空（计数不可靠）维持跳过。
        if method_key.is_none() {
            if let crate::ast::Expr::FieldAccess {
                expr: obj, field, ..
            } = func
            {
                if let MonoType::Fn { params, .. } = &mono_func_ty {
                    let ns_key = extract_namespace_path(obj).map(|p| format!("{p}.{field}"));
                    // #387：native arity 区间（直接命中或 std. 前缀回退）
                    let table_range = ns_key
                        .as_deref()
                        .and_then(|k| self.native_arity.get(k).copied())
                        .or_else(|| {
                            ns_key
                                .as_deref()
                                .map(|k| format!("std.{k}"))
                                .and_then(|k| self.native_arity.get(k.as_str()).copied())
                        });
                    // #454 / t12-F1 / t14：原生导出与方法级调用**同一套判据**——
                    // 收敛到共享实现 `arg_binding::bind_args_to_params`。形参名与
                    // **必需下限**同出一源：`signature::export_signature_param_info`
                    // （同一签名串的解析出口），调用点不再各算一份——两侧各算曾
                    // 实体分歧（ir_gen 硬编全必需位 ⇒ 省略可选位的合法调用被误报 E1010）。
                    if !named_args.is_empty() {
                        if let Some(info) = ns_key
                            .as_deref()
                            .and_then(|k| self.native_param_names.get(k))
                            .or_else(|| {
                                ns_key
                                    .as_deref()
                                    .map(|k| format!("std.{k}"))
                                    .and_then(|k| self.native_param_names.get(k.as_str()))
                            })
                        {
                            let diag_name = ns_key.as_deref().unwrap_or(field);
                            // 可选位：第 `required_min` 位及其后（`?name` / `...args` 尾位）
                            let optional: Vec<bool> = (0..info.names.len())
                                .map(|i| i >= info.required_min)
                                .collect();
                            arg_binding::bind_args_to_params(
                                diag_name,
                                &info.names,
                                args,
                                named_args,
                                0, // 命名空间调用无接收者位
                                &optional,
                                span,
                            )?;
                        }
                    }
                    let total = arg_types.len() + named_args.len();
                    let range = match table_range {
                        Some((min, max)) => {
                            if min <= params.len() && max.is_none_or(|m| params.len() <= m) {
                                Some((min, max))
                            } else if params.is_empty() {
                                None
                            } else {
                                // 用户同名遮蔽：按遮蔽者签名严格计数
                                Some((params.len(), Some(params.len())))
                            }
                        }
                        None => {
                            // #387：缺参形态此前被 `+1` 启发式误吞——命名空间
                            // 调用没有接收者，`total+1==params.len()` 纯属巧合
                            // 命中，少传的实参落运行期以 Void 兜底填充炸 E6007，
                            // 报错点位还在被调函数内部。根段是模块别名的限定
                            // 调用一律严格计数。
                            let is_namespace_call = ns_key
                                .as_deref()
                                .and_then(|k| k.split('.').next())
                                .is_some_and(|root| {
                                    self.module_aliases
                                        .is_some_and(|aliases| aliases.contains(root))
                                });
                            if params.is_empty() {
                                // 计数不可靠：维持跳过
                                None
                            } else if is_namespace_call || total + 1 != params.len() {
                                Some((params.len(), Some(params.len())))
                            } else {
                                // 值接收者形态：放行
                                None
                            }
                        }
                    };
                    if let Some((min_arity, max_arity)) = range {
                        if total < min_arity || max_arity.is_some_and(|m| total > m) {
                            let name = ns_key.unwrap_or_else(|| field.clone());
                            let expected = if total < min_arity {
                                min_arity
                            } else {
                                max_arity.unwrap_or(min_arity)
                            };
                            return Err(ErrorCodeDefinition::argument_count_mismatch(
                                &name, expected, total,
                            )
                            .at(span)
                            .build());
                        }
                    }
                }
            }
        }
        // 分发
        match mono_func_ty {
            MonoType::Fn {
                params,
                return_type,
                ..
            } => {
                // 值级函数调用；方法调用（#317）实参对齐 params[1..]——
                // 接收者占 params[0]，不在实参列表中
                let recv_offset = usize::from(method_key.is_some() && !params.is_empty());
                let param_slice = &params[recv_offset..];
                // #452：逐实参 → 形参位对齐。有绑定表时**按位**取形参类型
                // （命名实参不再因「个数对不上」整体跳过类型检查）；无绑定表
                // （纯位置实参 / 名字不可用）维持旧的对位规则。
                //
                // #452 / F1：实参序 = 位置实参（前导，与 args 同序）+ 命名实参
                // （书写序）。元数一律按**两者之和**结算：此前用 arg_types.len()
                // （只含位置实参）与 positions 对齐，带命名实参的调用恒失配 ⇒
                // 整段逐参类型检查被静默跳过（多候选零命中/单候选都中招）。
                let total_args = arg_types.len() + named_args.len();
                let mut arg_types_all: Vec<&MonoType> = arg_types.iter().collect();
                arg_types_all.extend(named_types.iter().copied());
                let aligned: Option<Vec<(&MonoType, &MonoType, &crate::ast::Expr)>> =
                    param_positions.as_ref().and_then(|positions| {
                        if positions.len() != total_args {
                            return None;
                        }
                        let mut out = Vec::with_capacity(positions.len());
                        for (i, position) in positions.iter().enumerate() {
                            // 槽位下标含接收者位（全形参表坐标）；args 侧余下的是值形参
                            let value_slot = position.checked_sub(recv_offset)?;
                            let param_ty = param_slice.get(value_slot)?;
                            let arg_ty = *arg_types_all.get(i)?;
                            // 实参序 = 位置实参（前导，与 args 同序）+ 命名实参（书写序）
                            let arg_expr = if i < args.len() {
                                args.get(i)?
                            } else {
                                &named_args.get(i - args.len())?.1
                            };
                            out.push((arg_ty, param_ty, arg_expr));
                        }
                        Some(out)
                    });
                let checked_args: Vec<(&MonoType, &MonoType, &crate::ast::Expr)> = match aligned {
                    Some(list) => list,
                    None if total_args == param_slice.len() => args
                        .iter()
                        .zip(arg_types.iter().zip(param_slice.iter()))
                        .map(|(arg_expr, (arg_ty, param_ty))| (arg_ty, param_ty, arg_expr))
                        .collect(),
                    None => Vec::new(),
                };
                {
                    for (arg_ty, param_ty, arg_expr) in checked_args.iter() {
                        // 自动借用：当参数签名要求 &T 且实参是值类型时，
                        // 编译器自动创建令牌（RFC-009 §2.8）
                        let actual_arg = match (param_ty, arg_ty) {
                            (MonoType::Ref { mutable, .. }, a)
                                if !matches!(a, MonoType::Ref { .. }) =>
                            {
                                MonoType::Ref {
                                    mutable: *mutable,
                                    inner: Box::new((*a).clone()),
                                }
                            }
                            _ => (*arg_ty).clone(),
                        };
                        // resolve 参数类型：TypeRef("Int") → Int(64) 等
                        let mut resolved_param = self.solver.resolve_type(param_ty);
                        // #523（RFC-027 §6）：精化应用对统一**透明**——剥到基类型。
                        //
                        // 注册期只剥了形参位（`check_fn_stmt` 的外层参数组）；
                        // curry 内层参数组的精化标注（`(N: Int) -> (x: IsPositive(N))`
                        // 的内层 `IsPositive(N)`）原样进入调用点统一，基类型实参
                        // 对不上原始形态被假拒绝（E1002 Expected type
                        // 'IsPositive(N)', found type 'int64'）。此处与注册期共用
                        // 同一剥除实现（判据单源，见
                        // `statements::strip_refined_with_proof_fns`）：已剥形态
                        //（外层组参数）在此是恒等变换；内置谓词位换新鲜变量，
                        // 约束仍由证明管道验证，此处只管「按 base 统一」。
                        resolved_param = statements::strip_refined_with_proof_fns(
                            resolved_param,
                            self.proof_fn_bases,
                            self.solver,
                        );
                        // Int -> Float 扩展转换是允许的
                        if matches!(
                            (&actual_arg, &resolved_param),
                            (MonoType::Int(_), MonoType::Float(_))
                        ) {
                            continue;
                        }
                        // RFC-011a §6.3: 接口构造器形参（存在类型位）不做 type_defs
                        // 替换——交给 solver 的结构化子型臂按 interfaces 面判定，
                        // 替换成构造器 Struct 反而会与具体实参 Struct 撞名拒绝。
                        let is_iface_param = matches!(
                            &resolved_param,
                            MonoType::TypeRef(n)
                                if self.generic_type_defs.contains_key(n)
                        );
                        // TypeRef: 先 solver.resolve 解析内置类型（Int/Float 等），
                        // 再 type_defs 解析用户自定义类型；都不匹配则跳过
                        if let MonoType::TypeRef(name) = &resolved_param {
                            if !is_iface_param {
                                resolved_param = match self.type_defs.get(name).map(|p| &p.body) {
                                    Some(def_ty) => self.solver.resolve_type(def_ty),
                                    None => continue,
                                };
                            }
                        }
                        if let Err(failure) = decide(
                            self.solver,
                            &actual_arg,
                            &resolved_param,
                            Some(span),
                            self.type_defs,
                            self.dep_env,
                        ) {
                            // RFC-011a §6.3: 接口形参收到未实现接口的具体类型
                            // → 精确报 E1101（成员检查），非笼统类型不匹配
                            if is_iface_param {
                                if let MonoType::Struct(s) = self.solver.resolve_type(&actual_arg) {
                                    if let MonoType::TypeRef(iface) = &resolved_param {
                                        return Err(
                                            ErrorCodeDefinition::type_does_not_implement_interface(
                                                &s.name, iface,
                                            )
                                            .at(span)
                                            .build(),
                                        );
                                    }
                                }
                            }
                            return Err(failure.or_site(
                                ErrorCodeDefinition::type_mismatch(
                                    &format!("{}", resolved_param),
                                    &format!("{}", arg_ty),
                                )
                                .at(span)
                                .build(),
                            ));
                        }
                        if is_iface_param {
                            // 具体值进入存在类型形参位 → 检查实现并记录包装点
                            self.collect_existential_coercions(arg_expr, &resolved_param)?;
                        }
                    }
                }
                // 用 `resolve_type` 而非 `expand_type_shallow`：
                // 后者只做结构展开，**不跟随 TypeVar 绑定**。
                // 显式类型实参（`mk(Int)`）的绑定就在 solver 里，
                // 不 resolve 的话返回类型仍是 `t49`，外层 `mk(Int)(5)`
                // 拿不到 Int，静默当 void（D6.3）。
                let resolved_ret = self.solver.resolve_type(&return_type);
                return Ok(resolved_ret);
            }
            MonoType::Struct(_) | MonoType::TypeRef(_) | MonoType::Generic { .. } => {
                // 类型构造器：Point(1.0, 2.0) 或 `Vec(Int)` / `Array(Int, 3)`
                // 等内置容器类构造器（内层类型实参应用的结果就是
                // `Generic{name, args}`，必须原样交给调用点作为 `func_ty`，
                // 否则退化成 fresh TypeVar，`Vec(Int)()` 的 `.length` 无法解析）
                return Ok(mono_func_ty);
            }
            _ => {}
        }
        Ok(self.solver.new_var())
    }
}
