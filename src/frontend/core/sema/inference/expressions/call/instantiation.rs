//! 调用裁决族 · 调用点泛型实例化与类型实参归约。
//!
//! 自 `inference/expressions.rs` 纯搬移（WBS 7.0.4）：方法体与文档注释零改动，
//! 仅补跨模块可见性（`pub(super)`）与相对路径（`super::` 改指 crate 路径）。

use crate::frontend::core::types::MonoType;
use crate::frontend::module::symbol::SymbolTable;
use crate::middle::passes::mono::instance::{GenericFunctionId, InstantiationRequest};
use crate::util::diagnostic::ErrorCodeDefinition;
use std::collections::{HashMap, HashSet};

use super::super::{
    concrete_type_from_expr_arg, extract_const_value_from_expr, ConstArgOutcome, ExpressionInferrer,
};
use crate::frontend::core::sema::inference::arg_binding;

impl<'a> ExpressionInferrer<'a> {
    /// 递归收集类型中的所有 TypeVar 索引
    /// 按出现顺序收集 `TypeVar`（与集合语义的 `collect_type_var_indices` 相对）。
    /// 用于把「声明序类型参数名」与签名里的变量位逐一对齐。
    pub(super) fn collect_type_vars_positional(
        ty: &MonoType,
        out: &mut Vec<(usize, MonoType)>,
    ) {
        match ty {
            MonoType::TypeVar(tv) => out.push((tv.index(), ty.clone())),
            MonoType::Ref { inner, .. } => Self::collect_type_vars_positional(inner, out),
            MonoType::Generic { args, .. } => {
                for a in args {
                    Self::collect_type_vars_positional(a, out);
                }
            }
            MonoType::Fn {
                params,
                return_type,
                ..
            } => {
                for p in params {
                    Self::collect_type_vars_positional(p, out);
                }
                Self::collect_type_vars_positional(return_type, out);
            }
            MonoType::Union(items) | MonoType::Intersection(items) => {
                for t in items {
                    Self::collect_type_vars_positional(t, out);
                }
            }
            _ => {}
        }
    }

    pub(super) fn collect_type_var_indices(
        ty: &MonoType,
        out: &mut HashSet<usize>,
    ) {
        match ty {
            MonoType::TypeVar(tv) => {
                out.insert(tv.index());
            }
            MonoType::Fn {
                params,
                return_type,
                ..
            } => {
                for p in params {
                    Self::collect_type_var_indices(p, out);
                }
                Self::collect_type_var_indices(return_type, out);
            }
            MonoType::Generic { name, args } if name == "Range" && args.len() == 1 => {
                Self::collect_type_var_indices(&args[0], out)
            }
            MonoType::Union(types) | MonoType::Intersection(types) => {
                for t in types {
                    Self::collect_type_var_indices(t, out);
                }
            }
            MonoType::Ref { inner, .. } => Self::collect_type_var_indices(inner, out),
            MonoType::Struct(s) => {
                for field_info in &s.fields {
                    Self::collect_type_var_indices(&field_info.ty, out);
                }
            }
            MonoType::Generic { args, .. } => {
                for a in args {
                    Self::collect_type_var_indices(a, out);
                }
            }
            MonoType::AssocType {
                host_type,
                assoc_args,
                ..
            } => {
                Self::collect_type_var_indices(host_type, out);
                for a in assoc_args {
                    Self::collect_type_var_indices(a, out);
                }
            }
            _ => {}
        }
    }

    /// 将类型中的 TypeVar 根据替换映射替换为具体类型
    ///
    /// 递归遍历类型，将遇到的 TypeVar 在 `subst` 映射中查找，
    /// 若找到替换项则替换，否则保留原 TypeVar。
    /// 把「类型实参表达式」解成具体类型：`Int` → `Int(64)`、`Vec(Int)` → `Generic{"Vec",[Int]}`。
    ///
    /// 类型名在实参位置推断得到的只是笼统的 `MetaType`（类型宇宙的值），
    /// 而显式类型实参要的是它**指的类型**——`mk(Int)` 必须把 `A` 绑到 `Int`，
    /// 而不是绑到「某个类型值」。这里从 AST 形态直接取类型。
    pub(super) fn type_arg_of_expr(expr: &crate::ast::Expr) -> MonoType {
        use crate::ast::Expr;
        match expr {
            Expr::Var(name, _) => {
                MonoType::from_builtin_name(name).unwrap_or_else(|| MonoType::TypeRef(name.clone()))
            }
            // `Vec(Int)` / `Array(Int, 3)` 形态：类型构造器应用
            Expr::Call { func, args, .. } => {
                if let Expr::Var(cname, _) = func.as_ref() {
                    MonoType::Generic {
                        // #403：与注解同一落点判据——`List(Int)` 作类型实参
                        // 同样落 `Vec` 面（单源 `MonoType::source_container_face`，
                        // 源名进类型域的门共用）。
                        name: MonoType::source_container_face(cname),
                        args: args.iter().map(Self::type_arg_of_expr).collect(),
                    }
                } else {
                    MonoType::TypeRef("_".to_string())
                }
            }
            _ => MonoType::TypeRef("_".to_string()),
        }
    }

    /// 内置容器构造器 `Vec(T)` / `Array(T, N)` 在**值位置**的类型实参解包。
    ///
    /// `Vec(Int)` 的被调名推断成 `MetaType`，实参 `Int` 的推断结果同样只是
    /// `MetaType` 空壳（不携带具体类型名）——直接把这些实参塞进 `Generic{…}`
    /// 会让元素位变成 `Type`，`v[0] + 1` 报 E1002（#575）。这里按 AST 取回
    /// 具体类型：类型位复用 `type_arg_of_expr`（与显式类型实参 `mk(Int)` 同一
    /// 单点）；`Array` 的长度位按注解同形落 `Literal`（`container_length_slot`
    /// 读的就是它），非字面量保持原推断类型。实参不是类型表达式时（如
    /// `Vec(3)` 的长度字面量）保持既有推断结果，不把值误当类型。
    pub(super) fn builtin_container_ctor_args(
        &self,
        fn_name: Option<&str>,
        args: &[crate::ast::Expr],
        arg_types: &[MonoType],
    ) -> Option<Vec<MonoType>> {
        let is_type_arg = |i: usize| -> bool {
            matches!(arg_types.get(i), Some(MonoType::MetaType { .. }))
                || concrete_type_from_expr_arg(&args[i], self.type_defs, self.generic_type_defs)
                    .is_some()
        };
        let type_slot = |i: usize| -> MonoType {
            if is_type_arg(i) {
                Self::type_arg_of_expr(&args[i])
            } else {
                arg_types[i].clone()
            }
        };
        match fn_name? {
            "Vec" if args.len() == 1 => Some(vec![type_slot(0)]),
            "Array" if args.len() == 2 => {
                let length = match extract_const_value_from_expr(&args[1]) {
                    Some(value) => MonoType::Literal {
                        name: format!("{value}"),
                        base_type: Box::new(arg_types[1].clone()),
                        value,
                    },
                    None => arg_types[1].clone(),
                };
                Some(vec![type_slot(0), length])
            }
            _ => None,
        }
    }

    pub(super) fn substitute_type_vars(
        ty: &MonoType,
        subst: &HashMap<usize, MonoType>,
    ) -> MonoType {
        use crate::frontend::core::types::substitute::{Substituter, Substitution};
        let mut sub = Substitution::new();
        for (idx, replacement) in subst {
            sub.insert(*idx, replacement.clone());
        }
        Substituter::new().substitute(ty, &sub)
    }

    /// 类型参数推断时穿透借用包装：
    /// 递归到内层即可解出 T，但实参未带 Ref 时（或可变性不匹配）到不了内层，
    /// 故显式剥一层再补一次 unify。剥法本身单点在 MonoType::strip_ref
    /// （与函数体出口对照共用同一份实现）。
    /// 值位置的**类型值**判定：`Int` 这样的具体类型 infer 成 `MetaType`；
    /// `Box` / `Vec` 这样的类型构造器 infer 成类型级函数 `(T: Type) -> Type`
    /// （RFC-010 元类型宇宙的 Type1 值）。两者都是类型实参的候选形态。
    pub(super) fn is_type_value(ty: &MonoType) -> bool {
        match ty {
            MonoType::MetaType { .. } => true,
            MonoType::Fn {
                params,
                return_type,
            } => {
                !params.is_empty()
                    && matches!(return_type.as_ref(), MonoType::MetaType { .. })
                    && params
                        .iter()
                        .all(|p| matches!(p, MonoType::MetaType { .. }))
            }
            _ => false,
        }
    }

    /// 声明体里的**构造器变量头部**匹配（WBS 3.5.1）：`F(A)` 的 F 是构造器
    /// 参数名，绑定值是实参类型的头部构造器名（`[1,2]: Vec(Int)` 的 `Vec`、
    /// `Box(1): Box` 的 `Box`），写成 `TypeRef` 形态供后续替换使用。
    ///
    /// 实参位（`A`）不在本函数绑定：`F(A)` 的 A 由普通类型变量槽位解出，
    /// 头部绑定只是把 F 换成真实构造器，使 `Vec(A)` / `Box(A)` 能与实参类型对齐。
    pub(super) fn bind_ctor_param_heads(
        shape: &MonoType,
        actual: &MonoType,
        type_params: &[String],
        bindings: &mut HashMap<String, MonoType>,
    ) {
        match (shape, actual) {
            (
                MonoType::Generic { name, args: s_args },
                MonoType::Generic {
                    name: a_head,
                    args: a_args,
                },
            ) => {
                if type_params.iter().any(|p| p == name) {
                    bindings.insert(name.clone(), MonoType::TypeRef(a_head.clone()));
                }
                if s_args.len() == a_args.len() {
                    for (s, a) in s_args.iter().zip(a_args.iter()) {
                        Self::bind_ctor_param_heads(s, a, type_params, bindings);
                    }
                }
            }
            (MonoType::Generic { name, .. }, MonoType::Struct(st)) => {
                if type_params.iter().any(|p| p == name) {
                    bindings.insert(name.clone(), MonoType::TypeRef(st.name.clone()));
                }
            }
            (MonoType::Ref { inner: s, .. }, MonoType::Ref { inner: a, .. }) => {
                Self::bind_ctor_param_heads(s, a, type_params, bindings);
            }
            (
                MonoType::Fn {
                    params: sp,
                    return_type: sr,
                },
                MonoType::Fn {
                    params: ap,
                    return_type: ar,
                },
            ) if sp.len() == ap.len() => {
                for (s, a) in sp.iter().zip(ap.iter()) {
                    Self::bind_ctor_param_heads(s, a, type_params, bindings);
                }
                Self::bind_ctor_param_heads(sr, ar, type_params, bindings);
            }
            (MonoType::Union(s), MonoType::Union(a)) if s.len() == a.len() => {
                for (x, y) in s.iter().zip(a.iter()) {
                    Self::bind_ctor_param_heads(x, y, type_params, bindings);
                }
            }
            (MonoType::Intersection(s), MonoType::Intersection(a)) if s.len() == a.len() => {
                for (x, y) in s.iter().zip(a.iter()) {
                    Self::bind_ctor_param_heads(x, y, type_params, bindings);
                }
            }
            _ => {}
        }
    }

    /// 按名替换类型中的 `TypeRef`（声明的类型参数名 → fresh TypeVar）。
    /// 只替换 `subst` 里登记的名字，其余 TypeRef（std 的 `Error` 等）原样保留。
    fn substitute_type_refs(
        ty: &MonoType,
        subst: &HashMap<String, MonoType>,
    ) -> MonoType {
        match ty {
            MonoType::TypeRef(name) => subst.get(name).cloned().unwrap_or_else(|| ty.clone()),
            MonoType::Ref { mutable, inner } => MonoType::Ref {
                mutable: *mutable,
                inner: Box::new(Self::substitute_type_refs(inner, subst)),
            },
            MonoType::Generic { name, args } => MonoType::Generic {
                name: name.clone(),
                args: args
                    .iter()
                    .map(|a| Self::substitute_type_refs(a, subst))
                    .collect(),
            },
            MonoType::Fn {
                params,
                return_type,
            } => MonoType::Fn {
                params: params
                    .iter()
                    .map(|p| Self::substitute_type_refs(p, subst))
                    .collect(),
                return_type: Box::new(Self::substitute_type_refs(return_type, subst)),
            },
            MonoType::Struct(st) => {
                let mut st = st.clone();
                st.fields = st
                    .fields
                    .into_iter()
                    .map(
                        |field_info| crate::frontend::core::types::mono::StructFieldInfo {
                            ty: Self::substitute_type_refs(&field_info.ty, subst),
                            ..field_info.clone()
                        },
                    )
                    .collect();
                MonoType::Struct(st)
            }
            _ => ty.clone(),
        }
    }

    /// 单态化泛型函数类型：将泛型函数类型中的类型变量统一替换为具体类型。
    ///
    /// 当调用泛型函数（如 `fn identity[T](x: T) -> T`）时，根据实参类型
    /// 推断类型变量的具体类型，返回单态化后的函数类型。
    ///
    /// 仅处理 Fn 类型；非 Fn 类型或不含 MetaType 的 Fn 类型原样返回。
    pub(super) fn monomorphize(
        &mut self,
        func_ty: MonoType,
        arg_types: &[MonoType],
        fn_name: Option<&str>,
        builtin_ctor_args: Option<Vec<MonoType>>,
    ) -> MonoType {
        // 内置容器构器：`Vec(Int)` / `Array(Int, 3)`。
        //
        // 被调对象本身是 `MetaType`（类型宇宙的值，由值位置的类型名推断而来），
        // 而不是 `Fn`——下面的 `let MonoType::Fn` 会直接原样返回它，
        // `Vec(Int)()` 的 `func_ty` 就悬空成 `tN`，`.length` 报
        // E1053「非结构体不能取字段」（D6.2）。这里先把「MetaType 应用类型实参」
        // 收成 `Generic{name, args}`——这正是 `Vec(Int)` 应有的类型。
        //
        // 实参本身也只是 `MetaType` 空壳（`Int` 在值位置不携带具体名字，见
        // #575），故具体类型实参由调用点按 AST 形态解出后传入；直接复用
        // `arg_types` 会让元素位退化成 `Type`（`v[0] + 1` 报 E1002）。
        if matches!(func_ty, MonoType::MetaType { .. }) {
            if let (Some(name), Some(args)) = (fn_name, builtin_ctor_args) {
                if matches!(name, "Vec" | "Array") && !args.is_empty() {
                    self.last_type_args = args.clone();
                    return MonoType::Generic {
                        name: name.to_string(),
                        args,
                    };
                }
            }
        }
        // 入口清空：非泛型调用不得读到上次的残留。
        self.last_type_args.clear();
        let MonoType::Fn {
            params,
            return_type,
        } = &func_ty
        else {
            return func_ty;
        };

        // ── 单态化：每调用一次换一套 fresh 实例 ──────────────────────
        //
        // 泛型函数签名里的类型参数有两种表示：
        //   1. `TypeVar`  —— 值级泛型（`identity: (T: Type) -> (x: T) -> T`
        //                     在 scope 里已实例化为 TypeVar）
        //   2. `TypeRef`  —— 类型级泛型（`(T: Type) -> (l: &L(T)) -> Int`
        //                     声明侧把裸名转成 TypeRef；与 std 的 `Error` 等
        //                     普通类型名同形，故必须查**声明表**才能区分）
        //
        // 两趟都要跑，且都不能提前返回——同一签名里可能两种并存
        // （`&L(T)` 的 T 是 TypeRef，同签名的 lambda 形参是 TypeVar）。
        // 关键是**每次调用都要换新实例**：否则跨调用共享同一 TypeVar，
        // 两次不同类型调用（`identity(42)` 与 `identity("hi")`）会冲突。
        let mut new_params: Vec<MonoType> = params.clone();
        let mut new_return: MonoType = (**return_type).clone();
        let mut pending_slots: Vec<MonoType> = Vec::new();
        let mut changed = false;

        // 第 1 趟：TypeVar → fresh
        //
        // 类型参数的两种表示：`TypeRef("A")`（名字）与 `TypeVar(n)`（编号）。
        // 声明处（`checker.rs`）为每个声明类型参数创建 TypeVar 并替换进签名，
        // 所以**普通注解形态**（`(l: L(A))`）拿到的是 TypeVar；
        // 只有嵌在 `Ref` 内层等少数形态会保留 TypeRef 名字。
        //
        // 声明序已知：`declared[i]` 对应签名里「按出现顺序」的第 i 个 TypeVar。
        // 第 1 趟按索引升序换 fresh，因此可据此把名字与 fresh 槽位对应起来，
        // 让纯 TypeVar 形态也能收集到实例化请求（否则该泛型函数不会被特化，
        // 在 mono 的 build_output 里被当未特化泛型删掉 → 运行时报函数不存在）。
        let declared_names: Vec<String> = fn_name
            .and_then(|f| self.generic_fn_type_params.get(f).cloned())
            .unwrap_or_default();
        let mut var_indices = HashSet::new();
        Self::collect_type_var_indices(&func_ty, &mut var_indices);
        if !var_indices.is_empty() {
            let mut subst = HashMap::new();
            let mut sorted: Vec<usize> = var_indices.iter().copied().collect();
            sorted.sort_unstable();
            for idx in sorted {
                subst.insert(idx, self.solver.new_var());
            }
            new_params = new_params
                .iter()
                .map(|p| Self::substitute_type_vars(p, &subst))
                .collect();
            new_return = Self::substitute_type_vars(&new_return, &subst);
            changed = true;
        }

        // 第 2 趟：声明序的类型参数名（TypeRef）→ fresh，并记录解出的实参
        if let Some(fname) = fn_name {
            if let Some(declared) = self.generic_fn_type_params.get(fname).cloned() {
                let mut subst: HashMap<String, MonoType> = HashMap::new();
                for pname in &declared {
                    subst.insert(pname.clone(), self.solver.new_var());
                }
                // 只有签名里确实出现该名字时才替换（避免无谓复制）
                let mut any = false;
                let old_params = std::mem::take(&mut new_params);
                for p in old_params {
                    let r = Self::substitute_type_refs(&p, &subst);
                    if r != p {
                        any = true;
                    }
                    new_params.push(r);
                }
                let r = Self::substitute_type_refs(&new_return, &subst);
                if r != new_return {
                    any = true;
                }
                new_return = r;
                if any {
                    changed = true;
                }
                // 解出的实参按声明序留存（供实例化请求使用）
                // 记住待解的槽位（unify 之前它们还是 fresh TypeVar，解不出具体类型）
                pending_slots = declared
                    .iter()
                    .filter_map(|pname| subst.get(pname).cloned())
                    .collect();
            }
        }
        // TypeVar 形态（普通注解 `(l: L(A))`）：签名里的类型参数是 TypeVar 而非
        // TypeRef 名字，第 2 趟按名字收集不到。改为从**替换后的签名**里按出现顺序
        // 取回这些 TypeVar——必须与签名实际使用的是同一个变量；另建 fresh 会让
        // unify 绑到签名变量上、而收集的槽位永远解不出具体类型。
        // 第 2 趟按名字填的槽位可能仍是**未解变量**（签名里该名字根本没出现，
        // 只是照着 declared 列表建了 fresh）——这正是 TypeVar 形态的特征。
        let ref_slots_unresolved = !pending_slots.is_empty()
            && pending_slots
                .iter()
                .all(|t| matches!(self.solver.resolve_type(t), MonoType::TypeVar(_)));
        if (pending_slots.is_empty() || ref_slots_unresolved)
            && !declared_names.is_empty()
            && changed
        {
            let mut ordered: Vec<(usize, MonoType)> = Vec::new();
            for p in new_params.iter().chain(std::iter::once(&new_return)) {
                Self::collect_type_vars_positional(p, &mut ordered);
            }
            ordered.sort_by_key(|(i, _)| *i);
            ordered.dedup_by_key(|(i, _)| *i);
            if ordered.len() == declared_names.len() {
                pending_slots = ordered.into_iter().map(|(_, t)| t).collect();
            }
        }

        if !changed {
            // 无 TypeVar、无声明类型参数 → 原样返回，交给后续 MetaType 路径
            let has_meta = params
                .iter()
                .any(|p| matches!(p, MonoType::MetaType { .. }));
            if has_meta {
                // 落到下方 MetaType 处理（保持原逻辑）
            } else {
                return func_ty;
            }
        } else {
            // Unify 新参数与实参以推断具体类型。
            //
            // 形参带借用（`&List(T)`）而实参是 `&List(Int)` 时，`TypeRef`/`TypeVar`
            // 嵌在 `Ref` 内层——solver 的 Ref 分支会递归，但实参若未带 Ref
            // （或两侧可变性不同）就到不了内层，类型参数解不出来。
            // 因此逐条 unify 后，再用「剥离 Ref 的形态」补一次，保证内层类型参数能绑定。
            if arg_types.len() == new_params.len() {
                for (arg_ty, param_ty) in arg_types.iter().zip(new_params.iter()) {
                    let _ = self.solver.unify(arg_ty, param_ty);
                    let (a, p) = (arg_ty.strip_ref(), param_ty.strip_ref());
                    let _ = self.solver.unify(&a, &p);
                }
            }
            // 声明名 ↔ 变量的对应可能**断裂**：同一声明类型参数在签名里能以两种
            // 形态出现（`TypeRef("T")` 名字式、`TypeVar(n)` 编号式），声明处的替换
            // 只覆盖了其中一部分。于是按名字建的槽位可能指向一个**签名里根本没用到**
            // 的变量，永远解不出具体类型（`map(.., f: (item:T)->R)` 的 `R` 即如此：
            // 名字槽指向 t77，而签名实际用的是 t75）。
            //
            // 补救：按位置遍历签名取回**实际使用**的变量（unify 已把它们绑好），
            // 用它替换掉仍未解出的名字槽。已在 unify 中收敛的槽位保持不动
            // （它才是对应声明名的正主）。
            if !pending_slots.is_empty() && pending_slots.len() == declared_names.len() {
                let is_free = |s: &Self, t: &MonoType| {
                    matches!(s.solver.resolve_type(t), MonoType::TypeVar(_))
                };
                if pending_slots.iter().any(|t| is_free(self, t)) {
                    let mut positional: Vec<(usize, MonoType)> = Vec::new();
                    for p in new_params.iter().chain(std::iter::once(&new_return)) {
                        Self::collect_type_vars_positional(p, &mut positional);
                    }
                    positional.sort_by_key(|(i, _)| *i);
                    positional.dedup_by_key(|(i, _)| *i);
                    let mut used: Vec<MonoType> = pending_slots
                        .iter()
                        .filter(|t| !is_free(self, t))
                        .cloned()
                        .collect();
                    for slot in pending_slots.iter_mut() {
                        if !is_free(self, slot) {
                            continue;
                        }
                        // 取一个尚未被占用的、已收敛的位置变量
                        if let Some((_, cand)) = positional
                            .iter()
                            .find(|(_, t)| !is_free(self, t) && !used.iter().any(|u| u == t))
                        {
                            used.push(cand.clone());
                            *slot = cand.clone();
                        }
                    }
                }
                // 返回位与参数位可能是**两个不同的** TypeVar（声明处逐位置替换，
                // 同一类型参数在参数位、返回位各拿一个）。unify 只绑定了参数位那个，
                // 返回位仍是自由变量——调用方拿到的返回值类型解不出具体类型
                // （`r = rev(&a)` 后 `len(&r)` 的 A 解不出的根因）。
                // 按**出现位置**把返回位的自由变量与已解出的槽位统一。
                let free_ret: Vec<MonoType> = {
                    let mut v: Vec<(usize, MonoType)> = Vec::new();
                    Self::collect_type_vars_positional(&new_return, &mut v);
                    v.sort_by_key(|(i, _)| *i);
                    v.dedup_by_key(|(i, _)| *i);
                    v.into_iter()
                        .map(|(_, t)| t)
                        .filter(|t| matches!(self.solver.resolve_type(t), MonoType::TypeVar(_)))
                        .collect()
                };
                let resolved_slots: Vec<MonoType> = pending_slots
                    .iter()
                    .map(|t| self.solver.resolve_type(t))
                    .collect();
                if free_ret.len() == resolved_slots.len() {
                    for (slot, ty) in resolved_slots.iter().zip(free_ret.iter()) {
                        let _ = self.solver.unify(slot, ty);
                    }
                }
                new_return = self.solver.resolve_type(&new_return);
            }
            // 统一后解出具体类型（unify 之前它们还是 fresh TypeVar）
            self.last_type_args = pending_slots
                .iter()
                .map(|t| self.solver.resolve_type(t))
                .collect();
            let resolved_return = self.solver.resolve_type(&new_return);
            return MonoType::Fn {
                params: new_params,
                return_type: Box::new(resolved_return),
            };
        }

        // 检查参数中是否包含 MetaType（泛型类型构造器）
        let has_meta = params
            .iter()
            .any(|p| matches!(p, MonoType::MetaType { .. }));
        if !has_meta {
            return func_ty;
        }

        // 当没有 TypeVar 但有 MetaType 参数时，为 MetaType 创建新的 TypeVar
        // 用于处理 List(1, 2, 3) 这样的情况
        {
            // 为每个 MetaType 参数创建新的 TypeVar
            let mut subst = HashMap::new();
            for (i, param) in params.iter().enumerate() {
                if matches!(param, MonoType::MetaType { .. }) && i < arg_types.len() {
                    let fresh = self.solver.new_var();
                    let fresh_clone = fresh.clone();
                    subst.insert(i, fresh);
                    // 将新 TypeVar 与实参类型统一，以推断具体类型
                    let _ = self.solver.unify(&fresh_clone, &arg_types[i]);
                }
            }
            // 替换参数中的 MetaType 为推断出的具体类型
            let new_params: Vec<MonoType> = params
                .iter()
                .enumerate()
                .map(|(i, p)| {
                    if let Some(fresh) = subst.get(&i) {
                        self.solver.resolve_type(fresh)
                    } else {
                        p.clone()
                    }
                })
                .collect();
            let resolved_return = self.solver.resolve_type(return_type);

            // 如果返回类型是 MetaType，尝试从 generic_type_defs 中获取具体的类型
            if matches!(resolved_return, MonoType::MetaType { .. }) {
                // 把 MetaType 位置上的实参组装成 `Generic{name, args}`。
                //
                // 这条路径原本直接返回 fresh TypeVar（注释写“让调用者处理”），
                // 但调用者无从知晓函数名：`Vec(Int)` 于是永远悬空，
                // `Vec(Int)()` 的 `func_ty` 变成 `tN`，`.length` 报
                // E1053「非结构体不能取字段」（D6.2 的另一半）。
                //
                // 判据：被调形参里有 MetaType 且实参已解成具体类型——
                // 实参类型就是类型实参（类型宇宙的值），直接收进 args。
                if let Some(name) = fn_name {
                    // 只对内置容器构器展开（`Vec` / `Array`）：
                    // 其余泛型类型走各自的 instantiate_generic_type 路径。
                    if matches!(name, "Vec" | "Array") {
                        let args: Vec<MonoType> = arg_types.to_vec();
                        if !args.is_empty() {
                            return MonoType::Fn {
                                params: new_params,
                                return_type: Box::new(MonoType::Generic {
                                    name: name.to_string(),
                                    args,
                                }),
                            };
                        }
                    }
                }
                // 仍未知：返回 TypeVar，由调用点继续判定（原行为）
                return MonoType::Fn {
                    params: new_params,
                    return_type: Box::new(self.solver.new_var()),
                };
            }

            MonoType::Fn {
                params: new_params,
                return_type: Box::new(resolved_return),
            }
        }
    }

    /// 被调方在**泛型定义命名空间**里的名字（实例化请求的 `generic_id` 名）。
    ///
    /// - 裸 `Var`：其名本身（既有行为，`main`/单文件路径一字不改）；
    /// - 限定调用 `lib.f`：与 merged-IR 同源的限定名 `{module_key}.{f}`——
    ///   `module_key` 取该别名的模块限定键（use 路径），拼接经
    ///   [`SymbolTable::qualify`]（全仓限定名格式的唯一所有者，与 ir_gen
    ///   `qualify_names`、`Resolver::resolve_namespace` 同源）。
    ///
    /// 返回 None 的形态不发请求：非 Var 结尾的接收者（lambda 调用、值接收者
    /// 方法调用、嵌套限定路径 `a.b.f`）与未注入模块表的直构场景——
    /// 两者都保持注入前的行为。
    fn callee_generic_name(
        &self,
        func_expr: &crate::ast::Expr,
    ) -> Option<String> {
        use crate::ast::Expr;
        match func_expr {
            Expr::Var(name, _) => Some(name.clone()),
            Expr::FieldAccess {
                expr: obj, field, ..
            } => {
                let Expr::Var(alias, _) = obj.as_ref() else {
                    return None;
                };
                let module_key = self.module_namespaces?.get(alias)?;
                Some(SymbolTable::qualify(module_key, field))
            }
            _ => None,
        }
    }

    /// 调用点视图的**一帧编译期环境**（6.9.2-C1 / 6.9.4）：只投影既有判据，不新增判据。
    ///
    /// - **有值**：值环境的**最内层绑定**（`ScopeManager::const_bindings`，t23 F1' 口径：
    ///   遮蔽按绑定解析，外层同名绑定的值不得顶上来）；
    /// - **无值（符号）**：本帧 q=0 binder 名 ∪ 被单值钉住的形参（按绑定解析，名字不继承）；
    /// - **等式事实（6.9.4 第四证据源）**：Γ 的常量等式
    ///   （`FlowSensitiveGamma::const_equalities`）并入**值**侧——分支守卫与裸 `assert`
    ///   给出的 `y == 42` 让 `y` 此刻有值。值环境已有的同名值优先（既有证据源在前），
    ///   符号集里的同名被剔除。
    ///
    /// 唯一生产构造点：`TermEnv::new` 的调用点不增。Σ 不在此列；导入面（#539）也不在此列。
    fn frame_term_env(&self) -> crate::frontend::core::types::eval::const_eval::TermEnv {
        // F-1：等式事实按**绑定身份**过滤——与值环境 `const_bindings` 同一条按名
        // 解析链（`VarInfo::definition_span`）；同名但不同绑定的事实不产值。
        let equalities = match self.gamma.as_deref() {
            Some(gamma) => gamma.const_equalities_resolved(&|name| {
                crate::frontend::core::sema::proof::assumptions::scope_binding_key(self.scope, name)
            }),
            None => Vec::new(),
        };
        crate::frontend::core::types::eval::const_eval::TermEnv::with_equalities(
            self.scope.symbolic_term_names(),
            self.scope.const_bindings(),
            equalities,
        )
    }

    /// 编译期项（6.9.2-C1 ②）：**结构准入**（唯一转换器）＋ **自由变量义务**。
    ///
    /// 结构准入由 `convert_expr_to_const_expr` 一条判据给定（运行期值在名字步被义务
    /// 拒掉；调用 / 字段访问 / 索引 / 块在转换器被拒），不再有第二份白名单。
    /// 自由变量义务：项里出现的每个名字都必须是本帧环境认识的名字——普通运行期形参
    /// （`y: Int`）不在环境里，故 `f(y)` 依旧缺证据（E1113）。
    fn compile_time_term(
        &self,
        expr: &crate::ast::Expr,
        env: &crate::frontend::core::types::eval::const_eval::TermEnv,
    ) -> Option<crate::frontend::core::types::const_data::ConstExpr> {
        let term =
            crate::frontend::core::types::eval::const_eval::convert_expr_to_const_expr(expr)?;
        // C3 P3（6.9.2-C3，F4 裁决 A 甲-闭）：**钉住消去**——`k: N` 的 `k` 在本帧
        // 编译期已知，把它消解回 binder 名 `N`（`k - 1` ⇒ `N - 1`）。消解后是**开项**，
        // 由 P4 进 deferred 桶、P5 在具体实例按 name_map 代入后再归一化。
        let term = self.eliminate_pinned_names(term);
        env.admits(&term).then_some(term)
    }

    /// 钉住消去（C3 P3）：把项里被单值钉住的名字换成本帧的 binder 名。
    ///
    /// 判据单源：`ScopeManager::var_pin_binder`（按绑定解析，遮蔽不继承）；
    /// 换名走 `const_eval::rename_term_names`（与值代入共用同一遍走树）。
    fn eliminate_pinned_names(
        &self,
        term: crate::frontend::core::types::const_data::ConstExpr,
    ) -> crate::frontend::core::types::const_data::ConstExpr {
        let mut renames: std::collections::HashMap<String, String> =
            std::collections::HashMap::new();
        for name in crate::frontend::core::types::eval::const_eval::const_expr_free_names(&term) {
            if let Some(binder) = self.scope.var_pin_binder(&name) {
                renames.insert(name, binder.to_string());
            }
        }
        if renames.is_empty() {
            return term;
        }
        crate::frontend::core::types::eval::const_eval::rename_term_names(&term, &renames)
    }

    /// q=0 实参的**编译期项**判定与特化请求（6.8.5-1 / 6.9.2-C1）。
    ///
    /// 判定顺序（与 B 步同一骨架，只把「证据」换成「项」）：
    /// 1. 被调方有 q=0 形参（`PolyType::const_binders`，声明期登记口）；
    /// 2. 用调用点实参序列的形参名表 + 共享 `arg_binding::bind_args_to_params` 定位每个
    ///    q=0 槽（表缺项 = 事实不足 ⇒ 不判、不猜）；
    /// 3. 每个 q=0 实参取**项**（结构准入 + 自由变量义务），缺项 ⇒ `MissingEvidence`；
    ///    再归一化：`Value` 记值、`Unproven` 放行但不特化、`Disproved` ⇒ 拒绝（⑥）；
    /// 4. 全部为闭项且形态支持（纯 const 泛型 / 首层全 q=0 / 无命名实参）才登记请求——
    ///    实参以 `GenericArg::Const(项)` 入列（类型位由 `as_type` 单向派生）。
    pub(super) fn collect_const_arg_instantiation_request(
        &mut self,
        func: &crate::ast::Expr,
        args: &[crate::ast::Expr],
        named_args: &[(String, crate::ast::Expr)],
        span: crate::util::span::Span,
        method_binding_key: Option<String>,
    ) -> ConstArgOutcome {
        use crate::frontend::core::sema::proof::verdict::ProofFailure;
        use crate::frontend::core::types::const_data::{ConstExpr, ConstValue};
        use crate::frontend::core::types::mono::GenericArg;

        // 1. 被调方与 q=0 名表（声明期登记，与 `PolyType::new_with_const` 同一来源）。
        //
        // 两个被调面对齐到同一组事实（`fn_name` / `const_names` / 被调签名）后
        // 共用下方同一段判定（判据单源，issue #538）：
        // - **函数面**（`Var` / 模块限定 `FieldAccess`）：`callee_generic_name`；
        //   q=0 名取 `PolyType::const_binders`（声明期登记口）。
        // - **方法面**（值接收者 `FieldAccess`）：方法绑定键取**调用边界快照**
        //   （`method_binding_key` 参数——`infer_call_expr` 在 `infer(func)` 之后
        //   立即 `take` 传入；臂内已排除 native 命名空间与记录字段形态）；
        //   q=0 名与槽位名都来自方法注册写入的同一张槽位表
        //   （此前方法注册丢弃 q=0 事实 ⇒ 静默不判——#538 的缺口）。
        //
        // 方法面覆盖形态（issue #538 + #553）：无接收者位声明（#538）与显式
        // `self` 接收者（#553 形态①）两类。接收者形态的实参从 params[1..] 对位
        // （`recv_offset` 1——`method_binding_call_key` 是「接收者占 params[0]」的
        // 唯一判据，与调用侧方法绑定 `bind_named_args` 的 offset 1 同一口径，共用
        // 同一 `bind_args_to_params` 判据）；无接收者位形态实参直对声明槽位
        //（offset 0，与函数面同款）。显式接收者 + 槽位事实齐备的声明
        //（`Type.method: (self: .., N: Int) -> (n: N) -> ..`）的槽位条目由 #538 的
        // 方法注册口同款登记（`runtime_value_params` 全序含接收者位，q=0 子集
        // 不含接收者），本函数只补对位偏移、不造第二份判据。
        let mut recv_offset = 0usize;
        let fn_name;
        let const_names: Vec<String>;
        let body: MonoType;
        match self.callee_generic_name(func) {
            Some(name) => {
                let Some(poly) = self.scope.get_var(&name) else {
                    return ConstArgOutcome::NotTarget;
                };
                fn_name = name;
                const_names = poly.const_binders.iter().map(|b| b.name.clone()).collect();
                body = poly.body.clone();
            }
            None => {
                // 显式接收者形态：`method_binding_call_key` 是「接收者占
                // params[0]」的唯一判据（native 命名空间 / 记录字段 / 接收者校验
                // 都在它内）。命中 ⇒ 下方槽位对位从 params[1..] 起（#553 形态①）。
                if self.method_binding_call_key(func).is_some() {
                    recv_offset = 1;
                }
                // 方法键来自**调用边界快照**（`method_binding_key` 参数——
                // `infer_call_expr` 在 `infer(func)` 之后立即 take，见其注释）：
                // 实参推断对状态位的覆盖不参与本次判定，非 FieldAccess 被调方
                // 的调用边界不读走陈旧键。
                let Some(method_key) = method_binding_key else {
                    return ConstArgOutcome::NotTarget;
                };
                let Some(slots) = self.declared_call_site_params.get(&method_key) else {
                    // 事实不足（方法面的跨模块方法导入——#553 face ④ / #539 轴，
                    // 方法槽位尚无跨界载体；无 q=0 事实的类型体匿名绑定等非 const
                    // 形态——方法自带 q=0 槽被 E1003/E1094 拦在声明面）⇒ 不判，不猜
                    return ConstArgOutcome::NotTarget;
                };
                if slots.const_params.is_empty() {
                    return ConstArgOutcome::NotTarget;
                }
                let Some(method_ty) = self.method_bindings.get(&method_key) else {
                    return ConstArgOutcome::NotTarget;
                };
                fn_name = method_key;
                const_names = slots.const_params.clone();
                body = method_ty.clone();
            }
        }
        if const_names.is_empty() {
            return ConstArgOutcome::NotTarget;
        }
        let MonoType::Fn {
            params,
            return_type,
        } = &body
        else {
            return ConstArgOutcome::NotTarget;
        };
        // 2. 槽位：调用点形参名表 + 共享绑定判据（六条规则同在一处，不另写映射）
        let Some(slots) = self.declared_call_site_params.get(&fn_name) else {
            // 事实不足（未走签名收集的直构场景）⇒ 不判，不猜。导入函数面
            // 的事实已随 Export 跨界携带（#539：`Export::const_binders` +
            // `Export::call_site_params`，导入方 `import_binding` 按同构登记），
            // 不再属于缺项。
            return ConstArgOutcome::NotTarget;
        };
        let call_site_names = slots.names.clone();
        let optional = vec![false; call_site_names.len()];
        let positions = arg_binding::bind_args_to_params(
            &fn_name,
            &call_site_names,
            args,
            named_args,
            recv_offset,
            &optional,
            span,
        )
        // 绑定畸形（未知名 / 争槽 / 命名后接位置 / 缺参）已由别处发射对应码，
        // 这里退回位置对位，避免重复发射（与 refinement.rs 的调用点义务同一口径）。
        // 回退对位沿用 `bind_args_to_params` 自己的规则（slot = recv_offset + i，
        // issue #553 形态①）：柯里化签名（`f: (N: Int) -> (n: N) -> Int`）的槽位表
        // 是全链拍平名，首次调用只填外层位 ⇒ 缺参校验把内层位当缺参 ⇒ Err 落回退；
        // 回退必须跳过接收者位，否则显式接收者形态的实参会错对到接收者槽。
        .unwrap_or_else(|_| (recv_offset..recv_offset + args.len()).collect());
        let arg_pairs: Vec<(&crate::ast::Expr, usize)> = args
            .iter()
            .enumerate()
            .map(|(i, e)| (e, positions.get(i).copied().unwrap_or(recv_offset + i)))
            .chain(named_args.iter().enumerate().map(|(i, (_, e))| {
                let slot = positions
                    .get(args.len() + i)
                    .copied()
                    .unwrap_or(recv_offset + args.len() + i);
                (e, slot)
            }))
            .collect();
        // 3. 逐个 q=0 槽：项 + 归一化
        //
        // 拒绝出口只有一处：**缺项**（运行期值 / 不可转换形态）与**闭项求值失败**
        // 同码同出口（E1113，message 取同时覆盖两者的中性措辞），不按原因再造第二个码。
        let reject = |param: &str, at: crate::util::span::Span| {
            ConstArgOutcome::MissingEvidence(
                ErrorCodeDefinition::missing_compile_time_evidence(param)
                    .at(at)
                    .build(),
            )
        };
        let env = self.frame_term_env();
        // 闭项的归一化值：请求里放**归一化后**的项（见下方构造点说明）
        let mut closed: HashMap<String, ConstValue> = HashMap::new();
        // 开项（C3 甲-闭）：保留**代入前**的项（容器 binder 名原样），入请求后由 mono
        // 的 deferred 桶在具体实例按 name_map 代入 + 归一化（P5）。
        let mut open: HashMap<String, ConstExpr> = HashMap::new();
        for (arg_expr, slot) in arg_pairs {
            let Some(param_name) = call_site_names.get(slot) else {
                continue;
            };
            if !const_names.iter().any(|n| n == param_name) {
                continue;
            }
            let Some(term) = self.compile_time_term(arg_expr, &env) else {
                return reject(param_name, arg_expr.span());
            };
            // 一次实参判定 = 一次判定预算（`for_judgment` 的唯一工厂）
            let budget = crate::frontend::core::sema::proof::budget::BudgetTracker::for_judgment();
            match crate::frontend::core::types::eval::const_eval::normalize(&term, &env, &budget) {
                Ok(value) => {
                    closed.insert(param_name.clone(), value);
                }
                // 开项（Unproven，含符号名/预算耗尽）：不拒绝——C3 起**入请求**，
                // 由 mono 在具体实例代入后再归一化（甲-闭 + T-b）。`Unproven` 绝不
                // 降格成 `Disproved`。
                Err(ProofFailure::Unproven { .. }) => {
                    open.insert(param_name.clone(), term.clone());
                }
                // 闭项求值失败（如 `1 / 0`）：拒绝（D59 C 类，旧行为静默接受）
                Err(ProofFailure::Disproved(_)) => {
                    return reject(param_name, arg_expr.span());
                }
            }
        }
        // 4. 特化仍只对**闭项**（与 B 步同），且形态门槛不变
        let type_params = self.lookup_type_params(&fn_name);
        if type_params.is_empty()
            || type_params != const_names
            || params.len() != const_names.len()
            || !named_args.is_empty()
        {
            return ConstArgOutcome::Evidenced(None);
        }
        let mut type_args: Vec<GenericArg> = Vec::with_capacity(const_names.len());

        for name in &const_names {
            // 闭项 ⇒ 归一化值；开项 ⇒ 保留代入前的项（C3 甲-闭）。两者都**入请求**：
            // 开项由 mono 判 symbolic 进 deferred，到具体实例再代入 + 归一化。
            let term = match (closed.get(name), open.get(name)) {
                (Some(value), _) => ConstExpr::Lit(value.clone()),
                (None, Some(term)) => term.clone(),
                // 缺项（元数不足）：不特化；缺项交给既有缺参判定
                (None, None) => return ConstArgOutcome::Evidenced(None),
            };
            // 类型位投影：闭项必须存在（非标量值没有字面量类型，与 6.8.5-1 一致）；
            // 开项此刻**没有**类型位形态（由 mono 代入后归一化再投影），不在此拦。
            if closed.contains_key(name) && GenericArg::Const(term.clone()).as_type().is_none() {
                return ConstArgOutcome::Evidenced(None);
            }
            type_args.push(GenericArg::Const(term));
        }
        let generic_id = GenericFunctionId::new(fn_name, type_params);
        // 返回值：把 const binder 换成字面量后的**返回类型**——与请求共用同一次
        // 类型位投影（`as_type`），故 `f(42)` 之后 `n: N` 立刻定成 `int64::42`
        //（6.8.7 单值类型强制），外层 `f(42)(5)` 因此报 E1002。
        let subst: HashMap<String, MonoType> = const_names
            .iter()
            .cloned()
            .zip(type_args.iter().filter_map(|arg| arg.as_type()))
            .collect();
        let specialized_ret = return_type.as_ref().clone().substitute(&subst);
        let mut request = InstantiationRequest::new(generic_id, type_args, span);
        request.containing_fn = self.scope.fn_context().map(str::to_owned);
        self.instantiation_requests.push(request);
        ConstArgOutcome::Evidenced(Some(specialized_ret))
    }

    /// 收集泛型函数实例化请求
    ///
    /// 检测函数调用是否是泛型函数调用，如果是则构造 InstantiationRequest
    /// 并添加到实例化请求列表中。
    pub(super) fn collect_instantiation_request(
        &mut self,
        func_ty: &MonoType,
        func_expr: &crate::ast::Expr,
        _arg_types: &[MonoType],
        mono_func_ty: &MonoType,
        call_span: crate::util::span::Span,
    ) {
        // 只处理 Fn 类型
        let MonoType::Fn {
            params: original_params,
            ..
        } = func_ty
        else {
            return;
        };

        // 被调方在泛型定义命名空间里的名字：裸 Var 用其名（既有行为），
        // 限定调用 `lib.f` 用与 merged-IR 同源的限定名。不是这两种形态
        //（lambda 调用、值接收者方法调用）→ 不收集，与既有行为一致。
        let Some(fn_name) = self.callee_generic_name(func_expr) else {
            return;
        };
        // 限定调用（被调方是 FieldAccess）标记：下方第一条路径的 arity
        // 判据随之收紧（见该处注释）。
        let is_qualified = matches!(func_expr, crate::ast::Expr::FieldAccess { .. });

        // ── 层级：声明名为权威，签名形态反推只兜底 ────────────────────
        //
        // 第一条路径（下方）：`last_type_args` 由 `monomorphize_func_ty` 按**声明序
        // 类型参数名**解出（名字来自 `lookup_type_params` → `generic_fn_type_params`，
        // 被调模块的名字随 `Export::type_params` 的声明期快照跨界）。名字是权威：
        // 只有名字能把签名里的 `TypeRef("A")` 与声明表对上，嵌套/借用位置才解得开。
        //
        // 第二条路径（再下方）：从签名**实际形态**反推（按位置取 `TypeVar`）。
        // 它是显式标注的**兜底**，只在第一条拿不到声明名时生效——类型参数
        // 只出现在嵌套/借用位置（`x: &T`、`Vec(T)`）时反推取不到任何东西，
        // 故它不能替代声明名，只能服务声明名表未覆盖的老路径。
        //
        // 优先：单态化阶段已按声明序解出的类型实参。
        // 它覆盖按名声明的类型参数（`(T: Type, Acc: Type)`）——这类参数在
        // MonoType 侧是 TypeRef 或嵌在 `Ref(Generic)` 里，下面的 TypeVar 位置
        // 启发式猜不全（#361）。
        let monomorphized_args = std::mem::take(&mut self.last_type_args);
        // 只接受**已解成具体类型**的实参；仍是 TypeVar 的说明单态化未收敛，
        // 落回下方「参数位是 TypeVar」的既有路径（那里的 resolved_params 已统一过）。
        let monomorphized_args: Vec<MonoType> = monomorphized_args
            .into_iter()
            .filter(|t| !matches!(t, MonoType::TypeVar(_)))
            .collect();
        if !monomorphized_args.is_empty() {
            let type_params: Vec<String> = self.lookup_type_params(&fn_name);
            // arity 判据：裸名沿用既有行为（空表 = 无 arity 约束）。
            // 限定名没有「无约束」余地——`last_type_args` 是 monomorphize 按
            // **裸字段名**的表解出来的，那个键可能属于本模块的同名函数而非被调
            // 模块的声明（跨模块导出只在随导出携带 `type_params` 时有该键）。
            // 限定名表解不出时宁可落第二条路径（它按实际签名取值，不会错配），
            // 否则会发出 arity 错配的请求，mono 侧硬报 E3018（比不特化更糟）。
            let arity_ok = if type_params.is_empty() {
                !is_qualified
            } else {
                type_params.len() == monomorphized_args.len()
            };
            if arity_ok {
                let generic_id = GenericFunctionId::new(fn_name.clone(), type_params);
                // 6.9.2-C1：这些实参都是**类型位**（单态化推出来的具体类型），
                // 与 q=0 的编译期项位不同列。
                let type_args = monomorphized_args
                    .into_iter()
                    .map(crate::frontend::core::types::mono::GenericArg::Type)
                    .collect();
                let mut request = InstantiationRequest::new(generic_id, type_args, call_span);
                request.containing_fn = self.scope.fn_context().map(str::to_owned);
                self.instantiation_requests.push(request);
                return;
            }
        }

        // ── 兜底路径：从签名形态反推（按位置取 TypeVar）────────────────
        //
        // 仅当上面的**声明名**路径没给出实参（`last_type_args` 空或不收敛）时走到
        // 这里：按位置扫形参，参数位**直接是** TypeVar 的才收（如 identity 的
        // x: T、twice 的 x: T）。这是能力受限的兜底——类型参数只出现在嵌套/
        // 借用位置（`x: &T`、`Vec(T)`）时什么也收不到，那类形态必须靠声明名。
        //
        // 不能收集嵌套 TypeVar 的参数位：twice(f: (T) -> T, x: T) 的 f 位是 fn(T)->T，
        // 解析后是 fn(int64)->int64，用它当 type_arg 会让 T 绑到错误类型（#255）。
        let mut type_var_indices = HashSet::new();
        for (i, param) in original_params.iter().enumerate() {
            if matches!(self.solver.resolve_type(param), MonoType::TypeVar(_)) {
                type_var_indices.insert(i);
            }
        }

        // 没有 TypeVar → 不是泛型函数调用（除非是 MetaType 构造器，交给类型特化路径）
        if type_var_indices.is_empty() {
            let has_meta = original_params
                .iter()
                .any(|p| matches!(p, MonoType::MetaType { .. }));
            if !has_meta {
                return;
            }
        }

        // 获取泛型参数名称列表（裸名或限定名，见 callee_generic_name）
        let type_params: Vec<String> = self.lookup_type_params(&fn_name);

        // 从单态化后的函数类型中提取具体的类型参数
        if let MonoType::Fn {
            params: resolved_params,
            ..
        } = mono_func_ty
        {
            // 只收集 TypeVar 参数位对应的具体类型（去重）。
            // 不能收集所有具体参数类型：twice(x => x+1, 5) 的参位含 fn(int64)->int64 与
            // int64，但 T 只有一个（int64）——全收集会让 type_args 长度与 type_params 不匹配、
            // 特化失败（#255）。
            let mut type_args = Vec::new();
            let mut seen = HashSet::new();
            for &idx in &type_var_indices {
                if let Some(resolved) = resolved_params.get(idx) {
                    let resolved = self.solver.resolve_type(resolved);
                    if !matches!(resolved, MonoType::TypeVar(_)) {
                        let key = format!("{}", resolved);
                        if seen.insert(key) {
                            type_args.push(resolved);
                        }
                    }
                }
            }

            if !type_args.is_empty() {
                let generic_id = if type_params.is_empty() {
                    GenericFunctionId::new(fn_name, vec![])
                } else {
                    GenericFunctionId::new(fn_name, type_params)
                };
                let type_args = type_args
                    .into_iter()
                    .map(crate::frontend::core::types::mono::GenericArg::Type)
                    .collect();
                let mut request = InstantiationRequest::new(generic_id, type_args, call_span);
                // #335 路径 A：记录所在函数——嵌套调用请求的符号化实参
                //（TypeRef(参数名)）由 mono 按所在泛型函数的 name_map 求值
                request.containing_fn = self.scope.fn_context().map(str::to_owned);
                self.instantiation_requests.push(request);
            }
        }
    }

    /// 查找函数的泛型类型参数名称
    ///
    /// `fn_name` 接受两种键：本模块的裸函数名，以及**跨模块限定名**
    /// `{module_key}.{bare}`（限定调用 `lib.f(x)` 的请求用它）。跨模块导出
    /// 随 `Export::type_params` 携带声明名时，`export_type` 会同时登记
    /// `export.name` 与 `export.full_path` 两个键，故限定名在此可解析。
    ///
    /// 三条来源的层级（权威 → 兜底）：
    ///
    /// 1. 重载候选（`OverloadCandidate::type_params`）——声明侧的显式登记；
    /// 2. 作用域 `PolyType::type_binders`——只剩编号、无名字，故退回占位名
    ///    `T0`/`T1`（下游按位置匹配）；
    /// 3. `generic_fn_type_params` 声明名表——**跨模块导出的登记口**：
    ///    本模块函数由 `check_fn_stmt` 登记，被调模块的由
    ///    `export_type` 按 `Export::type_params`（声明期快照）登记。
    ///
    /// 三条都是「按声明名查」的不同通道，层级只关乎查得到多少；三条都拿不到
    /// （如导出面来自旧路径、未携带声明名）时返回空表——调用方按「无 arity
    /// 约束」处理，由签名形态反推兜底（见 `collect_instantiation_request`
    /// 第二条路径；反推收不全嵌套/借用位的类型参数）。
    fn lookup_type_params(
        &self,
        fn_name: &str,
    ) -> Vec<String> {
        // 1. 优先从重载候选中查找（OverloadCandidate 包含 type_params）
        if let Some(candidates) = self.overload_candidates.get(fn_name) {
            for candidate in candidates {
                if candidate.is_generic {
                    return candidate.type_params.clone();
                }
            }
        }

        // 2. 从作用域中查找 PolyType
        if let Some(poly) = self.scope.get_var(fn_name) {
            // type_binders 是 TypeVar 列表，按索引顺序对应类型参数
            // 由于当前系统不存储类型参数名称，返回空列表
            // Monomorphizer 可以通过函数名匹配（name 唯一时）
            if !poly.type_binders.is_empty() {
                // 如果有 type_binders，说明是泛型函数
                // 生成占位名称（如 "T0", "T1"）以便单态化器识别
                return poly
                    .type_binders
                    .iter()
                    .enumerate()
                    .map(|(i, _)| format!("T{}", i))
                    .collect();
            }
        }

        // 3. 声明的类型参数名表（`check_fn_stmt` 登记）。
        //
        // 这是**最可靠**的来源：`poly.type_binders` 在签名收集阶段已被剥掉，
        // 所以上面第 2 步对 `mk: (A: Type) -> (x: A) -> A` 拿不到名字，
        // 返回空表——而空表会被下游当成「无 arity 约束」放行，
        // 实例化请求的 `GenericFunctionId` 就带不上声明名，单态化器匹配不到
        // `mk` 的特化，运行时报 E6006。
        if let Some(names) = self.generic_fn_type_params.get(fn_name) {
            if !names.is_empty() {
                return names.clone();
            }
        }

        vec![]
    }
}
