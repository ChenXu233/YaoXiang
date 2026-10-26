use super::*;

impl TypeChecker {
    /// 名是否是「返回 `Type` 的函数」（精化谓词 / 证明函数）或是内置谓词 `Terminates`。
    ///
    /// 这类名字在类型位置合法：`val: IsPositive(5)`、`acc: Terminates(n)`——
    /// 其实参是**值表达式/度量**，不是类型引用（RFC-028 §6.9、RFC-027a）。
    /// 判定与 `body_checker` 的 `proof_fn_bases` 同源（`Fn` 且返回 `MetaType`）。
    fn is_type_returning_fn(
        &self,
        name: &str,
    ) -> bool {
        if name == "Terminates" {
            return true;
        }
        self.env.vars.get(name).is_some_and(|poly| {
            matches!(
                &poly.body,
                MonoType::Fn { return_type, .. }
                    if matches!(return_type.as_ref(), MonoType::MetaType { .. })
            )
        })
    }

    /// 从一条语句里抽出所有类型注解并校验（#371/#372）。
    ///
    /// 覆盖位置：绑定/函数的类型注解（含返回位）、各层形参、类型定义体。
    /// 泛型参数名（`(A: Type)` 形态）先剔出来置顶跳过。
    pub(super) fn check_annotation_type_names(
        &mut self,
        stmt: &crate::frontend::core::parser::ast::Stmt,
    ) {
        self.check_annotation_type_names_with(stmt, &[]);
    }

    /// `check_annotation_type_names` 的带继承参数版（B6）。
    ///
    /// `inherited`：外层作用域已声明的类型参数名（如外层函数的 `T`/`N`）。
    /// 函数体内的绑定/lambda 形参注解可以引用它们，不能当未知名报错。
    fn check_annotation_type_names_with(
        &mut self,
        stmt: &crate::frontend::core::parser::ast::Stmt,
        inherited: &[String],
    ) {
        use crate::frontend::core::parser::ast::{StmtKind, Type as T};

        let (type_annotation, signature_params, definition) = match &stmt.kind {
            StmtKind::Assign {
                type_annotation,
                signature_params,
                ..
            } => (type_annotation.as_ref(), signature_params.as_slice(), None),
            StmtKind::TypeDefinition {
                signature_params,
                definition,
                ..
            } => (None, signature_params.as_slice(), Some(definition)),
            _ => return,
        };

        // 签名参数名：**类型参数**（`A: Type`）与**编译期值参数**（`N: Int`，靠
        // `(n: N)` 这种「形参被当类型用」的形态识别）都是声明处绑定的局部名字，
        // 在类型位置出现时不算未知名，先行剔除。
        //
        // ⚠ 不能只看「有注解」：那会把普通值形参（`x: Int` 的 `x`）也算作类型名，
        // 于是 `f: (bogus: Int) -> bogus = ...` 的返回位 `bogus` 被当合法类型
        // 放行，该函数的返回类型检查形同虚设（B5）。
        let generic_names: Vec<String> = {
            use crate::frontend::core::parser::ast::name_used_as_type_in;
            let mut names: Vec<String> = inherited.to_vec();
            for p in signature_params {
                let Some(ty) = p.ty.as_ref() else { continue };
                // 类型参数（`A: Type` 形态）直接计入
                if matches!(ty, T::MetaType { .. }) {
                    names.push(p.name.clone());
                    continue;
                }
                // 值参数：其名在**签名注解或类型定义体的类型位**被引用时才算类型名
                // （`(N: Int) -> (n: N) -> Int` 的内层 `N`；
                //   `SafeArray: (T: Type, N: Int) -> Type = { data: Array(T, N) }` 的 `N`）
                let referenced = signature_params.iter().any(|q| {
                    q.ty.as_ref()
                        .is_some_and(|t| name_used_as_type_in(&p.name, t))
                }) || type_annotation
                    .as_ref()
                    .is_some_and(|t| name_used_as_type_in(&p.name, t))
                    || definition
                        .as_ref()
                        .is_some_and(|t| name_used_as_type_in(&p.name, t));
                if referenced {
                    names.push(p.name.clone());
                }
            }
            names
        };

        // 各层形参注解（`A: Type` 形态的类型参数位自身不引用类型，跳过）
        for p in signature_params {
            if let Some(ty) = p.ty.as_ref() {
                if !matches!(ty, T::MetaType { .. }) {
                    self.check_type_names_in(ty, &generic_names);
                }
            }
        }
        // 顶层注解（含函数返回位）
        if let Some(ty) = type_annotation {
            self.check_type_names_in(ty, &generic_names);
        }
        // 类型定义体（字段类型就是 #372 的另一半）。
        //
        // 体内声明的**成员名**（RFC-011 §3.1 关联类型，如 `IteratorType: Iterator(Item)`）
        // 在本体内是合法的类型名：`iter: (Self) -> IteratorType` 要能引用它。
        // 这是体级作用域——不得溢出到其他类型体或形参（见 `collect_body_member_names`）。
        if let Some(def) = definition {
            let mut in_body = generic_names.clone();
            Self::collect_member_names(def, &mut in_body);
            self.check_type_names_in(def, &in_body);
        }

        // B6：体递归——本条语句的函数体/块/分支/循环里的绑定与 lambda
        // 形参注解同样受校验，且可引用外层已声明的类型参数（`inherited` +
        // 本条签名自身的 generic_names 向下传递）。
        //
        // 此前只扫 module.items，函数体内的错拼完全逃逸
        // （`main = { g: (a: BogusType) -> Int = ... }` 编译通过）。
        let mut nested_inherited = inherited.to_vec();
        nested_inherited.extend(generic_names.iter().cloned());
        self.check_nested_annotations(stmt, &nested_inherited);
    }

    /// 下钻语句的嵌套作用域校验注解（B6）。只下钻承载函数体/块的位置。
    fn check_nested_annotations(
        &mut self,
        stmt: &crate::frontend::core::parser::ast::Stmt,
        inherited: &[String],
    ) {
        use crate::frontend::core::parser::ast::StmtKind;
        match &stmt.kind {
            StmtKind::Assign { value, .. } => {
                if let Some(e) = value.as_deref() {
                    self.check_nested_annotations_in_expr(e, inherited);
                }
            }
            StmtKind::Expr(e) => self.check_nested_annotations_in_expr(e, inherited),
            StmtKind::Return(Some(e)) => self.check_nested_annotations_in_expr(e, inherited),
            StmtKind::If {
                then_branch,
                else_if_branches,
                else_branch,
                ..
            } => {
                for s in &then_branch.stmts {
                    self.check_annotation_type_names_with(s, inherited);
                }
                for (_, body) in else_if_branches {
                    for s in &body.stmts {
                        self.check_annotation_type_names_with(s, inherited);
                    }
                }
                if let Some(b) = else_branch {
                    for s in &b.stmts {
                        self.check_annotation_type_names_with(s, inherited);
                    }
                }
            }
            _ => {}
        }
    }

    /// 表达式内的嵌套作用域下钻（块 / lambda / 循环 / 分支）。
    fn check_nested_annotations_in_expr(
        &mut self,
        expr: &crate::frontend::core::parser::ast::Expr,
        inherited: &[String],
    ) {
        use crate::frontend::core::parser::ast::Expr;
        match expr {
            Expr::Block(b) => {
                for s in &b.stmts {
                    self.check_annotation_type_names_with(s, inherited);
                }
            }
            Expr::While {
                condition, body, ..
            } => {
                self.check_nested_annotations_in_expr(condition, inherited);
                for s in &body.stmts {
                    self.check_annotation_type_names_with(s, inherited);
                }
            }
            Expr::For { iterable, body, .. } => {
                self.check_nested_annotations_in_expr(iterable, inherited);
                for s in &body.stmts {
                    self.check_annotation_type_names_with(s, inherited);
                }
            }
            Expr::If {
                condition,
                then_branch,
                else_if_branches,
                else_branch,
                ..
            } => {
                self.check_nested_annotations_in_expr(condition, inherited);
                for s in &then_branch.stmts {
                    self.check_annotation_type_names_with(s, inherited);
                }
                for (c, body) in else_if_branches {
                    self.check_nested_annotations_in_expr(c, inherited);
                    for s in &body.stmts {
                        self.check_annotation_type_names_with(s, inherited);
                    }
                }
                if let Some(b) = else_branch {
                    for s in &b.stmts {
                        self.check_annotation_type_names_with(s, inherited);
                    }
                }
            }
            Expr::Lambda { body, .. } => {
                for s in &body.stmts {
                    self.check_annotation_type_names_with(s, inherited);
                }
            }
            _ => {}
        }
    }

    /// 收集类型体内声明的成员名（RFC-011 §3.1 关联类型）。
    ///
    /// 背景：类型体里的 `Name: Type` 在 AST 里**只有一种形态**
    /// （`TypeBodyItem::Field`），关联类型成员与运行时数据字段**同形**，无从区分。
    /// 因此这里把全部字段名都当作潜在的体内类型名——这是 AST 表达力所限，
    /// 不是偷懒（见下方「残留宽松」）。
    ///
    /// **作用域**：仅用于定义体自身的校验。调用方只把结果传给
    /// `check_type_names_in(def, ..)`，不会写入 `env`，所以不会溢出到
    /// 别的类型体、形参或返回值。
    ///
    /// 残留宽松：`S: Type = { a: Int, b: a }` 里的 `a` 会被当作合法类型名。
    /// 这拦不了——`a` 确实被引用，而「引用」正是 RFC-011 判定成员的依据；
    /// 要根治需给关联类型一个**区别于数据字段的语法形态**（如 `type Name: T`），
    /// 属语言设计变更，不在本修复范围。
    fn collect_member_names(
        ty: &crate::frontend::core::parser::ast::Type,
        out: &mut Vec<String>,
    ) {
        use crate::frontend::core::parser::ast::{Type as T, TypeBodyItem};
        match ty {
            T::Struct { body } => {
                for item in body {
                    if let TypeBodyItem::Field(f) = item {
                        out.push(f.name.clone());
                    }
                }
            }
            T::NamedStruct { fields, .. } => {
                for f in fields {
                    out.push(f.name.clone());
                }
            }
            _ => {}
        }
    }

    /// 若表达式是索引形态（`Name[...]`），返回被索引的名字与源码位置（#371）。
    ///
    /// 用于把 `List[Int]` 这类**错写法**变成一条指向注解本身的诊断：
    /// 它在类型位置无合法解释（编译期值参数不用下标），而默认路径会把它
    /// 静默变成一个名叫 `<const-expr>` 的占位类型，直到使用处才报出
    /// 完全无关的错误。
    fn index_expr_head(
        expr: &crate::frontend::core::parser::ast::Expr
    ) -> Option<(String, crate::util::span::Span)> {
        use crate::frontend::core::parser::ast::Expr as E;
        match expr {
            E::Index { expr, .. } => match expr.as_ref() {
                E::Var(name, span) => Some((name.clone(), *span)),
                // 链式索引 `A[0][1]`：递归到最内层
                other => Self::index_expr_head(other),
            },
            _ => None,
        }
    }

    /// 校验注解引用的类型名是否都可解析（#371/#372）。
    ///
    /// 为何需要：`MonoType::from(ast::Type)` 对 `Type::Name` 不查表（直接包成
    /// `TypeRef(name)`）。于是形参/字段里错拼的类型名会静默成为一个「合法类型」，
    /// 而 solver 对两个不同的 `TypeRef` 判不等的分支根本走不到——**该参数上的
    /// 类型检查随之全部失效**（任意实参都能传）。
    ///
    /// 必须放在**所有注册完成之后**跑（见调用处）：`use` 导入的类型、接口、
    /// 泛型构造器、std 导出都陆续进 env，提前跑会把它们误判为未知。
    ///
    /// `generic_names` 是本签名的类型参数名（如 `(A: Type)` 里的 `A`）——
    /// 它们是声明处绑定的局部名字，不是可解析类型。
    fn check_type_names_in(
        &mut self,
        ty: &crate::frontend::core::parser::ast::Type,
        generic_names: &[String],
    ) {
        let mut unknown: Vec<(String, crate::util::span::Span, bool)> = Vec::new();
        Self::collect_unknown_type_names(ty, generic_names, &self.env, &mut unknown);
        for (name, span, was_bracket) in unknown {
            // 值空间兼容（合法形态三类）：
            //
            // 1. **编译期值函数**：`Array(Int, factorial(5))` 的 `factorial`。
            // 2. **精化谓词 / 证明函数**：`IsPositive(5)`、`Terminates(n)`——
            //    其类型位写的是「返回 Type 的函数名（类型宇宙）或内置谓词」，
            //    实参是值表达式或度量。 RFC-027 §6.9 / RFC-027a。
            // 3. **顶层值绑定**：pass2 只进 `early_value_bindings`（pass3 才入 env），
            //    模块级依赖精化 `mut s: SumUpTo(n, 6)` 的 `n` 由此容错（#379）。
            //
            // ⚠ 不放行「任何已声明值」（`env.vars` 通查）：那会让
            // `f: (x: println) -> Int` 通过（`println` 是值位置的函数名），
            // 该形参的类型检查形同虚设（B5）。#379 需要的是**用户顶层绑定**，
            // 由 `early_value_bindings` 精确覆盖，无需放宽到全量值的并集。
            if self.env.is_const_function(&name)
                || self.is_type_returning_fn(&name)
                || self.early_value_bindings.contains_key(&name)
            {
                continue;
            }
            // 方括号写法（`List[Int]`）→ 专用码，直接给出 `List(...)` 的写法；
            // 其余未知名 → E1003。两者都不再让错误拖到使用处（#371）。
            let code = if was_bracket {
                ErrorCodeDefinition::bracket_in_type_position(&name)
            } else {
                ErrorCodeDefinition::unknown_type(&name)
            };
            self.add_error(code.at(span).build());
        }
    }

    /// 递归收集注解中无法解析的类型名（#372）。
    fn collect_unknown_type_names(
        ty: &crate::frontend::core::parser::ast::Type,
        generic_names: &[String],
        env: &crate::frontend::core::typecheck::environment::TypeEnvironment,
        out: &mut Vec<(String, crate::util::span::Span, bool)>,
    ) {
        use crate::frontend::core::parser::ast::Type as T;
        match ty {
            T::Name { name, span } => {
                if !generic_names.iter().any(|g| g == name) && !env.resolves_type_name(name) {
                    out.push((name.clone(), *span, false));
                }
            }
            T::Generic {
                name,
                name_span,
                args,
            } => {
                if !generic_names.iter().any(|g| g == name) && !env.resolves_type_name(name) {
                    out.push((name.clone(), *name_span, false));
                }
                // 精化谓词 / 证明函数的**实参位是值**（`Terminates(n)` 的测度 n、
                // `IsPositive(5)` 的 5），不是类型引用——不下钻，否则测度里的
                // 局部变量名会被误报为未知名类型（RFC-027a）。
                if is_predicate_head(name, env) {
                    return;
                }
                for a in args {
                    Self::collect_unknown_type_names(a, generic_names, env, out);
                }
            }
            T::Fn {
                params,
                return_type,
            } => {
                for p in params {
                    Self::collect_unknown_type_names(p, generic_names, env, out);
                }
                Self::collect_unknown_type_names(return_type, generic_names, env, out);
            }
            T::Tuple(items) | T::Sum(items) => {
                for i in items {
                    Self::collect_unknown_type_names(i, generic_names, env, out);
                }
            }
            T::Option(inner) | T::Ptr(inner) => {
                Self::collect_unknown_type_names(inner, generic_names, env, out);
            }
            T::Result(ok, err) => {
                Self::collect_unknown_type_names(ok, generic_names, env, out);
                Self::collect_unknown_type_names(err, generic_names, env, out);
            }
            T::Ref { inner, .. } => {
                Self::collect_unknown_type_names(inner, generic_names, env, out);
            }
            T::MetaType { args, .. } => {
                for a in args {
                    Self::collect_unknown_type_names(a, generic_names, env, out);
                }
            }
            T::Literal { base_type, .. } => {
                Self::collect_unknown_type_names(base_type, generic_names, env, out);
            }
            T::Struct { body } => {
                for item in body {
                    if let crate::frontend::core::parser::ast::TypeBodyItem::Field(f) = item {
                        Self::collect_unknown_type_names(&f.ty, generic_names, env, out);
                    }
                }
            }
            T::NamedStruct { fields, .. } => {
                for f in fields {
                    Self::collect_unknown_type_names(&f.ty, generic_names, env, out);
                }
            }
            T::AssocType {
                host_type,
                assoc_args,
                ..
            } => {
                Self::collect_unknown_type_names(host_type, generic_names, env, out);
                for a in assoc_args {
                    Self::collect_unknown_type_names(a, generic_names, env, out);
                }
            }
            T::Union(members) => {
                for (_, m) in members {
                    if let Some(m) = m {
                        Self::collect_unknown_type_names(m, generic_names, env, out);
                    }
                }
            }
            // ConstExpr（RFC-027 编译期值参数）：合法形态是**值表达式**
            // （`Assert(N > 0)`、`Array(Int, factorial(5))`）。
            //
            // 但 `List[Int]` 这类**误解**也会落到这里：类型位置的标识符后跟 `[`
            // 会被当索引表达式，于是注解变成一个「名字叫 `<const-expr>` 的类型」。
            // 那是个凭空造出的占位类型，谁也认不出（#371）。
            //
            // 索引形态的表达式在**类型位置**没有任何合法解释（编译期值参数
            // 不用下标），所以一律报未知名——指向被索引的名字，并给出正确写法。
            T::ConstExpr(expr) => {
                if let Some((name, span)) = Self::index_expr_head(expr) {
                    // 下标形态在**类型位置**无任何合法解释（编译期值参数不用下标），
                    // 一律报专用码，指向注解并给出正确写法。
                    out.push((name, span, true));
                }
            }
            // 括号类型、Range、字面量类型、通配等：不引用具名类型。
            _ => {}
        }
    }
}

/// 精化谓词 / 证明函数的头名判定：`Terminates` 或「返回 `Type` 的函数」。
///
/// 这类应用（`Terminates(n)`、`IsPositive(5)`）的**实参位是值**，解析器与
/// 谓词解析器都按值处理；注解类型名校验因此不得下钻其实参，否则测度里的
/// 局部变量名会被误报为未知名类型（RFC-027 §6.9 / RFC-027a）。
fn is_predicate_head(
    name: &str,
    env: &crate::frontend::core::typecheck::environment::TypeEnvironment,
) -> bool {
    if name == "Terminates" {
        return true;
    }
    env.get_var(name).is_some_and(|poly| {
        matches!(
            &poly.body,
            crate::frontend::core::types::MonoType::Fn { return_type, .. }
                if matches!(return_type.as_ref(), crate::frontend::core::types::MonoType::MetaType { .. })
        )
    })
}
