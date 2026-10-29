//! 死代码分析器
//!
//! 识别从未被引用的定义，生成警告信息。
//! 码义（RFC-013 + RFC-029f + RFC-029g）：顶层绑定不因任何修饰符豁免——
//! 可达性由文件内引用与**包内引用池**（[`DeadCodeAnalyzer::set_cross_file_refs`]
//! / [`DeadCodeAnalyzer::set_exempt_pub`]）共同判定。池的来源是包目标的
//! **编译面**（driver 侧 `collect_package_refs` / `collect_compilation_refs`，
//! RFC-029f 改进 1）；身份是 `(模块, 名字)`——名字只记到 import 解析出的那个
//! 模块，模块自有的名字（顶层定义与局部绑定）不算它对别的模块的消费
//! （改进 2/4，见 [`DeadCodeAnalyzer::collect_module_names`] 与 driver 侧
//! `RefPool`：只有「被 import 后**真实使用**」才算消费）。
//! `FileRole::Script` 不接池（单文件路径没有包视角，未引用的顶层绑定一律报
//! W1001/W1002/W1004/W1005，改进 3）；`FileRole::Test` 的引用单独成池、
//! 不豁免生产代码（改进 5）。`pub` 关键字已由 RFC-029g 删除，不再是豁免来源。
//! 未使用导入（W1003）由 typecheck 的 use elaboration 检测，不在此处。

use std::collections::{HashMap, HashSet, VecDeque};

use crate::util::span::Span;
use crate::util::diagnostic::{Diagnostic, ErrorCodeDefinition};

use crate::frontend::core::parser::ast::{Module, Stmt, StmtKind, Expr, Block};

/// 死代码分析器
pub struct DeadCodeAnalyzer {
    /// 入口点集合（可达性根：`main`）
    entry_points: HashSet<String>,
    /// 所有符号定义——**逐定义一条**（同名重载不互相覆盖：WBS 4.9.6 / #450）
    all_defs: Vec<SymbolDef>,
    /// Bin/Internal 的包内引用池（driver 侧按**编译集**装配、名字已按模块
    /// 身份剔除自有名：RFC-029f 改进 1+2）
    exempt_pub: HashSet<String>,
    /// Lib 的跨文件引用池（同上；`FileRole::Script` 不接池——改进 3）
    cross_file_refs: HashSet<String>,
    /// 被重载决议**命中**的方法定义标记 `(限定名去掉 #k 的基名, 声明序)`：
    /// 由 driver 从 D20 决议产物（`overload_resolutions`，RFC-039 D20 / WBS 4.5.2）
    /// 解析后传入——分析器**不认识 IR 名**，只收这个二元组（IR 名↔定义的配对
    /// 留在 L1；分层：L2 不新增对 ir_gen 命名实现的依赖）
    resolved_methods: HashSet<(String, usize)>,
    /// 出现过 D20 决议条目的**名字组**（基名，全局；WBS 4.9.7 / issue #464）：
    /// 这些组的方法级消费由决议逐候选给出 ⇒ 池的**名字级**豁免
    /// （`RefPool::field_names` 全局平铺）对它们不成立，判定不因同名兄弟的
    /// 跨文件同名引用而放宽（宁漏报方向仍保留给无决议记录的组）
    resolved_groups: HashSet<String>,
    /// 被**算子派发**命中的方法名组（`Type.method` 基名，全局；WBS 3.20.3 /
    /// issue #505）：`u[0]` / `u == t` 在 typecheck 侧的具名决议是
    /// `OperatorDispatch { type_name, method }`（独立于 D20 重载决议的义务
    /// 通道，driver 从 `TypeCheckResult::operator_dispatches` 解析后传入）。
    /// 派发名不分重载候选 ⇒ 组内**全部声明**按活判定；组未命中时其余判定
    /// 原样——接口声明了但算子从未作用于该类型 ⇒ 照报 W1005（守卫不放宽）
    operator_dispatched: HashSet<String>,
}

/// 符号定义
#[derive(Debug, Clone)]
pub struct SymbolDef {
    /// 符号名称
    pub name: String,
    /// 符号种类
    pub kind: SymbolKind,
    /// 定义位置
    pub location: Span,
}

/// 符号种类
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SymbolKind {
    /// 函数
    Function,
    /// 类型
    Type,
    /// 变量
    Variable,
    /// 方法
    Method,
}

/// 死代码警告
#[derive(Debug, Clone)]
pub struct DeadCodeWarning {
    /// 警告码
    pub code: String,
    /// 警告消息
    pub message: String,
    /// 警告位置
    pub span: Span,
}

/// 引用/绑定收集器（RFC-029f 改进 2/4）：一趟遍历同时产出**标识符引用**
/// （死代码可达性与 W1003 复用）、**本模块自有的名字**（顶层定义 + 局部绑定）
/// 与**字段位引用**（方法/字段短名）。
///
/// 各集必须同源：池的口径（哪些名字算「别的模块在消费」）与可达性的口径
/// （哪些名字算「本文件在用」）若各走一趟遍历，分歧处就会让池自我豁免——
/// 正是 RFC-029f 缺陷 2 的成因。
#[derive(Default)]
struct IdentSink {
    /// 标识符引用（顶层定义处不算引用）
    refs: HashSet<String>,
    /// 本模块绑定的名字：顶层定义、函数体赋值目标、解构名、参数、
    /// for/spawn/gen 迭代变量、嵌套函数名
    bound: HashSet<String>,
    /// 字段/方法位引用（`obj.render()` 只出现短名 render），**基名不是 Var**
    /// 的那些（`f().render()`）：连命名空间归因都无从谈起，只能按名字维度计入
    field_refs_free: HashSet<String>,
    /// 命名空间位引用：`l.foo` 记成 基名 l 与 字段名 foo 一对——基名命中
    /// 命名空间导入时归因到该模块（RFC-029f 改进 4 的命名空间形态）
    namespaced_fields: Vec<(String, String)>,
}

impl IdentSink {
    /// 记一次标识符引用
    fn insert(
        &mut self,
        name: String,
    ) {
        self.refs.insert(name);
    }

    /// 记一次名字绑定（本模块自有）
    fn bind(
        &mut self,
        name: String,
    ) {
        self.bound.insert(name);
    }

    /// 记一次「基名不是 Var」的字段/方法位引用
    fn field_free(
        &mut self,
        name: String,
    ) {
        self.field_refs_free.insert(name);
    }
}

/// `use` 面（RFC-029f 改进 2/4 的池输入，读同一份 AST 的顶层 use 项）
#[derive(Default)]
struct ImportFace {
    /// 作用域内的名字（池的自有名扣减面：显式声明的对外消费面不算自有）
    names: HashSet<String>,
    /// 花括号项：作用域内名字 → 目标定义名 → 模块路径
    /// （`use lib.{f as g}` 记成 g、f、lib）
    items: Vec<(String, String, String)>,
    /// 整体导入：基名 → 模块路径（`use std.assert` 记成 assert、std.assert；
    /// `use lib as l` 记成 l、lib）
    namespaces: Vec<(String, String)>,
}

/// 收集 `use` 面：花括号项的别名优先；整体导入取路径末段（无别名时）
fn collect_import_face(ast: &Module) -> ImportFace {
    let mut face = ImportFace::default();
    for stmt in &ast.items {
        let StmtKind::Use {
            path,
            items,
            alias,
            item_aliases,
            ..
        } = &stmt.kind
        else {
            continue;
        };
        match items {
            Some(items) => {
                for (i, item) in items.iter().enumerate() {
                    let aliased = item_aliases
                        .as_ref()
                        .and_then(|aliases| aliases.get(i))
                        .and_then(|alias| alias.as_ref());
                    let local =
                        aliased.map_or_else(|| item.name.clone(), |alias| alias.name.clone());
                    face.names.insert(local.clone());
                    face.items.push((local, item.name.clone(), path.clone()));
                }
            }
            None => {
                let (local, module_path) = match alias.as_ref().and_then(|alias| alias.last()) {
                    Some(alias) => (alias.name.clone(), path.clone()),
                    None => match path.rsplit('.').next() {
                        Some(last) => (last.to_string(), path.clone()),
                        None => (path.clone(), path.clone()),
                    },
                };
                face.names.insert(local.clone());
                face.namespaces.push((local, module_path));
            }
        }
    }
    face
}

/// 模块的名字面——池（RFC-029f 改进 2/4/5）与可达性共用**一趟遍历**的产物
pub struct ModuleNames {
    /// 全部标识符引用（可达性 / W1003 用）
    pub refs: HashSet<String>,
    /// 本模块自有名：顶层定义 + 局部绑定，扣掉 `use` 带入的名字
    pub own: HashSet<String>,
    /// 基名不是 Var 的字段/方法位引用（`f().render()`）
    pub field_refs_free: HashSet<String>,
    /// 命名空间位引用：基名与字段名一对（`l.foo` → l、foo）
    pub namespaced_fields: Vec<(String, String)>,
    /// 花括号导入项：作用域内名字 → 目标定义名 → 模块路径
    pub imported: Vec<(String, String, String)>,
    /// 整体导入：基名 → 模块路径
    pub namespaces: Vec<(String, String)>,
}

impl Default for DeadCodeAnalyzer {
    fn default() -> Self {
        Self::new()
    }
}

impl DeadCodeAnalyzer {
    /// 创建新的死代码分析器
    pub fn new() -> Self {
        Self {
            entry_points: HashSet::new(),
            all_defs: Vec::new(),
            exempt_pub: HashSet::new(),
            cross_file_refs: HashSet::new(),
            resolved_methods: HashSet::new(),
            resolved_groups: HashSet::new(),
            operator_dispatched: HashSet::new(),
        }
    }

    /// 设置 Bin/Internal 的包内引用池——池成员 = **别的模块**对某个名字的
    /// 引用（driver 侧 `collect_compilation_refs` 按编译集装配，已剔除
    /// 每个模块自有的名字）
    pub fn set_exempt_pub(
        &mut self,
        names: HashSet<String>,
    ) {
        self.exempt_pub = names;
    }

    /// 设置 Lib 的跨文件引用池（同 `set_exempt_pub`；Script 不接池——
    /// 单文件语义只按定义-使用图判定，RFC-029f 改进 3）
    pub fn set_cross_file_refs(
        &mut self,
        refs: HashSet<String>,
    ) {
        self.cross_file_refs = refs;
    }

    /// 设置「被重载决议命中的方法定义」标记 `(基名, 声明序)`（WBS 4.9.6 / #450）：
    /// driver 从 D20 决议产物解析后传入；同名重载因此**逐候选**判定——
    /// 未被决议命中的兄弟重载落回「未调用 ⇒ 报 W1005」（验收③）。
    /// 名字未出现在任何标记里时（调用点歧义 / 未经决议路径）回落短名共判（宁漏报）。
    pub fn set_resolved_methods(
        &mut self,
        marks: HashSet<(String, usize)>,
    ) {
        self.resolved_methods = marks;
    }

    /// 设置「出现过决议条目的名字组」（全局集合；WBS 4.9.7 / issue #464）
    pub fn set_resolved_groups(
        &mut self,
        groups: HashSet<String>,
    ) {
        self.resolved_groups = groups;
    }

    /// 设置「被算子派发命中的方法名组」（`Type.method` 基名，全局集合；
    /// WBS 3.20.3 / issue #505）：driver 从 `TypeCheckResult::operator_dispatches`
    /// （RFC-011b span 键控派发点）解析后传入。命中 ⇒ 组内全部声明按活判定；
    /// 未命中 ⇒ 其余判定原样（未被调用的算子方法照报 W1005）。
    pub fn set_operator_dispatched_groups(
        &mut self,
        groups: HashSet<String>,
    ) {
        self.operator_dispatched = groups;
    }

    /// 该名字组是否有决议记录（决定逐候选还是回落短名共判）——
    /// 用驱动侧汇总的**全局**名字组：跨单元同名方法的决议记在别的单元上，
    /// 本单元也要据「该组受决议控制」停用池的名字级豁免
    fn method_group_has_marks(
        &self,
        name: &str,
    ) -> bool {
        self.resolved_groups.contains(name)
    }

    /// 收集入口点和符号定义（合并处理以减少代码重复）
    ///
    /// 入口点（可达性根）：`main` 函数。RFC-029g 后没有第二个根——修饰符
    /// 不产生任何可见性语义，顶层绑定一律参与可达性判定。
    ///
    /// 类型定义走 `TypeDefinition` 语句（`Point: Type = {...}`），
    /// 不再从 Assign 形状猜测——带类型注解的变量赋值是变量，不是类型。
    pub fn collect_entry_points_and_definitions(
        &mut self,
        ast: &Module,
    ) {
        for stmt in &ast.items {
            match &stmt.kind {
                StmtKind::Assign {
                    target,
                    type_annotation,
                    value,
                    ..
                } => {
                    let Some((name, type_name)) = target.receiver_parts() else {
                        continue;
                    };
                    // RFC-010a 附录D：非 Fn 注解的块绑定是**块值**而非函数
                    let is_fn_binding =
                        crate::frontend::core::parser::ast::Expr::block_binding_is_function(
                            type_annotation.as_ref(),
                            value.as_deref(),
                        );
                    let (params, body): (Vec<_>, Vec<_>) = if is_fn_binding {
                        match value {
                            Some(v) => v.callable_parts(),
                            None => (Vec::new(), Vec::new()),
                        }
                    } else {
                        (Vec::new(), Vec::new())
                    };
                    let is_method = type_name.is_some();
                    let (def_name, kind) = if is_method {
                        let full_name = format!("{}.{}", type_name.as_ref().unwrap(), name);
                        (full_name, SymbolKind::Method)
                    } else if value.as_ref().is_some_and(|v| {
                        matches!(v.as_ref(), Expr::Lambda { .. } | Expr::Block(..))
                    }) {
                        (name.clone(), SymbolKind::Function)
                    } else if params.is_empty() && body.is_empty() {
                        (name.clone(), SymbolKind::Variable)
                    } else {
                        (name.clone(), SymbolKind::Function)
                    };
                    self.record_definition(SymbolDef {
                        name: def_name.clone(),
                        kind,
                        location: stmt.span,
                    });
                    if !is_method && name == "main" {
                        self.entry_points.insert(name.clone());
                    }
                }
                StmtKind::TypeDefinition { name, .. } => {
                    self.record_definition(SymbolDef {
                        name: name.clone(),
                        kind: SymbolKind::Type,
                        location: stmt.span,
                    });
                }
                _ => {}
            }
        }
    }

    /// 记一条定义（WBS 4.9.6 / #450）：
    ///
    /// - **方法**逐定义**追加**——同名重载是各自独立的定义，不得互相覆盖
    ///   （旧实现按 full name 键控，两个重载只剩一条）；
    /// - 其余按**名字去重并保留最后一次**（顶层同名再绑定/重赋值是同一个绑定，
    ///   与旧 map 的「后写覆盖」行为一致——不额外制造同名重复告警）。
    fn record_definition(
        &mut self,
        def: SymbolDef,
    ) {
        if matches!(def.kind, SymbolKind::Method) {
            self.all_defs.push(def);
            return;
        }
        match self
            .all_defs
            .iter_mut()
            .find(|existing| existing.name == def.name)
        {
            Some(slot) => *slot = def,
            None => self.all_defs.push(def),
        }
    }

    /// 从 AST 中收集所有标识符引用（定义处不算引用）
    ///
    /// 顶层 Assign/DestructureAssign 的目标名是定义不是使用——计入会使所有
    /// 定义"自我可达"，死代码判定永远落空（#321 M2 复盘）；函数体内的赋值
    /// 目标是写入即使用。
    ///
    /// 供死代码可达性与 W1003 未使用导入判定共享（#321）：
    /// 语义级标记（Var 推断臂命中监视集）保证作用域正确性，
    /// 本遍历做语法级兜底——覆盖 match 模式、spawn 体、类型注解等
    /// Var 推断臂看不到的位置，宁多收（漏报方向）不漏收（误报方向）。
    pub fn collect_ident_refs(ast: &Module) -> HashSet<String> {
        Self::collect_module_names(ast).refs
    }

    /// 从 AST 收集模块的**名字面**（RFC-029f 改进 2/4/5 的池输入；一趟遍历）：
    /// 引用集、自有名集、字段位引用集、命名空间位引用与 `use` 面。
    ///
    /// `own` = 本模块绑定的名字（顶层定义 + 局部绑定）**扣掉** `use` 带入的
    /// 名字：模块自己命名空间里的标识符（`fn foo()` 的定义、函数体里的
    /// `foo = 1`、参数、for/迭代变量）不是它对**别的模块**同名定义的消费；
    /// `use` 带入的名字是显式声明的对外消费面，同名局部遮蔽不得吞掉它。
    pub fn collect_module_names(ast: &Module) -> ModuleNames {
        let mut referenced = IdentSink::default();

        fn collect_from_expr(
            expr: &Expr,
            referenced: &mut IdentSink,
        ) {
            match expr {
                Expr::Var(name, _) => {
                    referenced.insert(name.clone());
                }
                Expr::Call { func, args, .. } => {
                    collect_from_expr(func, referenced);
                    for arg in args {
                        collect_from_expr(arg, referenced);
                    }
                }
                Expr::FieldAccess { expr, field, .. } => {
                    collect_from_expr(expr, referenced);
                    // 字段名一并收集：方法调用 obj.render() 只出现短名
                    referenced.insert(field.clone());
                    // 基名是 Var 时记成（基名, 字段名）：命中命名空间导入即可归因；
                    // 其余（f().render() 等）连基名都没有，只能按名字维度计入
                    match expr.as_ref() {
                        Expr::Var(base, _) => {
                            referenced
                                .namespaced_fields
                                .push((base.clone(), field.clone()));
                        }
                        _ => referenced.field_free(field.clone()),
                    }
                }
                Expr::BinOp { left, right, .. } => {
                    collect_from_expr(left, referenced);
                    collect_from_expr(right, referenced);
                }
                Expr::UnOp { expr, .. } => {
                    collect_from_expr(expr, referenced);
                }
                Expr::If {
                    condition,
                    then_branch,
                    else_if_branches,
                    else_branch,
                    ..
                } => {
                    collect_from_expr(condition, referenced);
                    collect_from_block(then_branch, referenced);
                    for (_, branch) in else_if_branches {
                        collect_from_block(branch, referenced);
                    }
                    if let Some(branch) = else_branch {
                        collect_from_block(branch, referenced);
                    }
                }
                Expr::Match { expr, arms, .. } => {
                    collect_from_expr(expr, referenced);
                    for arm in arms {
                        collect_pattern_refs(&arm.pattern, referenced);
                        collect_from_block(&arm.body, referenced);
                    }
                }
                Expr::For {
                    var,
                    iterable,
                    body,
                    ..
                } => {
                    referenced.bind(var.clone());
                    referenced.insert(var.clone());
                    collect_from_expr(iterable, referenced);
                    collect_from_block(body, referenced);
                }
                Expr::While {
                    condition, body, ..
                } => {
                    collect_from_expr(condition, referenced);
                    collect_from_block(body, referenced);
                }
                Expr::Block(block) => {
                    collect_from_block(block, referenced);
                }
                Expr::Tuple(exprs, _) => {
                    for expr in exprs {
                        collect_from_expr(expr, referenced);
                    }
                }
                Expr::List(exprs, _) => {
                    for expr in exprs {
                        collect_from_expr(expr, referenced);
                    }
                }
                Expr::Dict(fields, _) => {
                    for (key, value) in fields {
                        collect_from_expr(key, referenced);
                        collect_from_expr(value, referenced);
                    }
                }
                Expr::Index { expr, index, .. } => {
                    collect_from_expr(expr, referenced);
                    collect_from_expr(index, referenced);
                }
                Expr::Lambda { params, body, .. } => {
                    collect_params_names_and_types(params, referenced);
                    collect_from_block(body, referenced);
                }
                Expr::ListComp {
                    element,
                    generators,
                    ..
                } => {
                    for gen in generators {
                        referenced.bind(gen.var.clone());
                        referenced.insert(gen.var.clone());
                        collect_from_expr(&gen.iterable, referenced);
                        if let Some(cond) = &gen.condition {
                            collect_from_expr(cond, referenced);
                        }
                    }
                    collect_from_expr(element, referenced);
                }
                Expr::FString { segments, .. } => {
                    for seg in segments {
                        match seg {
                            crate::frontend::core::parser::ast::FStringSegment::Text(_) => {}
                            crate::frontend::core::parser::ast::FStringSegment::Interpolation {
                                expr,
                                ..
                            } => {
                                collect_from_expr(expr, referenced);
                            }
                        }
                    }
                }
                Expr::Cast { expr, .. } => {
                    collect_from_expr(expr, referenced);
                }
                Expr::Try { expr, .. } => {
                    collect_from_expr(expr, referenced);
                }
                Expr::Ref { expr, .. } => {
                    collect_from_expr(expr, referenced);
                }
                // spawn 体是真实执行上下文，其中的引用必须收集（漏收会误报死代码）
                Expr::Spawn { body, .. } => {
                    collect_from_block(body, referenced);
                }
                Expr::SpawnFor {
                    var,
                    iterable,
                    body,
                    ..
                } => {
                    referenced.bind(var.clone());
                    referenced.insert(var.clone());
                    collect_from_expr(iterable, referenced);
                    collect_from_block(body, referenced);
                }
                Expr::In {
                    elem, container, ..
                } => {
                    collect_from_expr(elem, referenced);
                    collect_from_expr(container, referenced);
                }
                Expr::Borrow { expr, .. } => {
                    collect_from_expr(expr, referenced);
                }
                Expr::Unsafe { body, .. } => {
                    collect_from_block(body, referenced);
                }
                Expr::Return(Some(expr), _) => {
                    collect_from_expr(expr, referenced);
                }
                _ => {}
            }
        }

        fn collect_params_names_and_types(
            params: &[crate::frontend::core::parser::ast::Param],
            referenced: &mut IdentSink,
        ) {
            for param in params {
                referenced.bind(param.name.clone());
                referenced.insert(param.name.clone());
                if let Some(ty) = &param.ty {
                    ty.collect_name_refs(&mut referenced.refs);
                }
            }
        }

        fn collect_from_block(
            block: &Block,
            referenced: &mut IdentSink,
        ) {
            for stmt in &block.stmts {
                collect_from_stmt(stmt, referenced, false);
            }
        }

        /// match 模式中的名字引用：Identifier 兼作常量模式（无法与绑定区分，
        /// 一律收集），Struct/Union 头部是类型名引用，Guard 条件是真实表达式
        fn collect_pattern_refs(
            pattern: &crate::frontend::core::parser::ast::Pattern,
            referenced: &mut IdentSink,
        ) {
            use crate::frontend::core::parser::ast::Pattern;
            match pattern {
                Pattern::Identifier(name) => {
                    referenced.insert(name.clone());
                }
                Pattern::Tuple(ps) | Pattern::Or(ps) => {
                    for p in ps {
                        collect_pattern_refs(p, referenced);
                    }
                }
                Pattern::Struct { name, fields, .. } => {
                    referenced.insert(name.clone());
                    for (_, _, p) in fields {
                        collect_pattern_refs(p, referenced);
                    }
                }
                Pattern::Union { name, pattern, .. } => {
                    referenced.insert(name.clone());
                    if let Some(p) = pattern {
                        collect_pattern_refs(p, referenced);
                    }
                }
                Pattern::Guard {
                    pattern, condition, ..
                } => {
                    collect_pattern_refs(pattern, referenced);
                    collect_from_expr(condition, referenced);
                }
                _ => {}
            }
        }

        fn collect_from_stmt(
            stmt: &Stmt,
            referenced: &mut IdentSink,
            in_toplevel: bool,
        ) {
            match &stmt.kind {
                StmtKind::Expr(expr) => {
                    collect_from_expr(expr, referenced);
                }
                StmtKind::Assign {
                    value,
                    target,
                    type_annotation,
                    ..
                } => {
                    if let Some(expr) = value {
                        collect_from_expr(expr, referenced);
                    }
                    // 目标名：顶层是定义（不算引用，否则所有定义"自我可达"），
                    // 嵌套是写入即使用——通用走查覆盖 Var/FieldAccess/Index 等
                    // 复合目标（`counts["k"] = v` 是对 counts 的使用）
                    //
                    // 绑定侧（池用）：顶层目标经 `receiver_parts` 取定义名
                    // （方法绑定的裸名 `Point.render` → `render`，与 `all_defs`
                    // 的短名口径一致，池按短名匹配方法）；嵌套目标只有简单
                    // `Var` 是局部绑定——复合目标（字段/下标）不是新名字
                    if in_toplevel {
                        if let Some((name, _)) = target.receiver_parts() {
                            referenced.bind(name);
                        }
                    } else {
                        if let Expr::Var(name, _) = target.as_ref() {
                            referenced.bind(name.clone());
                        }
                        collect_from_expr(target, referenced);
                    }
                    if let Some(ty) = type_annotation {
                        ty.collect_name_refs(&mut referenced.refs);
                    }
                    // 形参类型名同样算引用：从注解的 Type::Fn 链拍平（6.5.2）
                    let signature = crate::frontend::core::parser::ast::signature_fn_params(
                        type_annotation.as_ref(),
                    );
                    for param in &signature {
                        if let Some(ty) = &param.ty {
                            ty.collect_name_refs(&mut referenced.refs);
                        }
                    }
                }
                // 元组解构赋值：嵌套目标是写入即使用；顶层是定义（不计引用）
                // 绑定侧（池用）：解构名两个层级都是新名字（顶层不计引用，
                // 故只有这里能记上）
                StmtKind::DestructureAssign { names, rhs, .. } => {
                    for name in names {
                        referenced.bind(name.name.clone());
                    }
                    collect_from_expr(rhs, referenced);
                    if !in_toplevel {
                        for name in names {
                            referenced.insert(name.name.clone());
                        }
                    }
                }
                StmtKind::TypeDefinition {
                    name,
                    type_annotation,
                    definition,
                    ..
                } => {
                    referenced.bind(name.clone());
                    let signature = crate::frontend::core::parser::ast::signature_fn_params(
                        type_annotation.as_ref(),
                    );
                    for param in &signature {
                        if let Some(ty) = &param.ty {
                            ty.collect_name_refs(&mut referenced.refs);
                        }
                    }
                    definition.collect_name_refs(&mut referenced.refs);
                }
                StmtKind::For {
                    var,
                    iterable,
                    body,
                    ..
                } => {
                    referenced.bind(var.clone());
                    referenced.insert(var.clone());
                    collect_from_expr(iterable, referenced);
                    collect_from_block(body, referenced);
                }
                StmtKind::If {
                    condition,
                    then_branch,
                    else_if_branches,
                    else_branch,
                    ..
                } => {
                    collect_from_expr(condition, referenced);
                    collect_from_block(then_branch, referenced);
                    for (_, branch) in else_if_branches {
                        collect_from_block(branch, referenced);
                    }
                    if let Some(branch) = else_branch {
                        collect_from_block(branch, referenced);
                    }
                }
                // 单表达式体 lambda 的合成体语句：parser 对 `() => expr`
                // 就地合成 `Return(Some(expr))` 糖（pratt/led.rs「Single
                // expression body」臂），不经 StmtKind::Expr——漏收会让
                // 闭包体内的引用整体丢失，仅被闭包引用的定义误报 W1001
                //（WBS 3.20.1：docs/src/reference/stdlib/test.md 的 t_add）
                StmtKind::Return(Some(expr)) => {
                    collect_from_expr(expr, referenced);
                }
                _ => {}
            }
        }

        for stmt in &ast.items {
            collect_from_stmt(stmt, &mut referenced, true);
        }
        // 自有的名字 = 本模块绑定的名字 − `use` 带入的名字（后者是显式声明的
        // 对外消费面，同名局部遮蔽不得吞掉它）
        let face = collect_import_face(ast);
        let own = referenced.bound.difference(&face.names).cloned().collect();
        ModuleNames {
            refs: referenced.refs,
            own,
            field_refs_free: referenced.field_refs_free,
            namespaced_fields: referenced.namespaced_fields,
            imported: face.items,
            namespaces: face.namespaces,
        }
    }

    /// 从入口点出发，计算可达符号集合
    ///
    /// 扁平近似：入口点 + AST 中被引用过的所有定义。
    /// 仅被死代码引用的定义同样记为可达（只报死代码根，宁漏报不误报）。
    pub fn compute_reachability(
        &self,
        ast: &Module,
    ) -> HashSet<String> {
        let mut reachable = HashSet::new();
        let mut queue = VecDeque::new();

        for entry in &self.entry_points {
            queue.push_back(entry.clone());
        }

        let ast_references = Self::collect_ident_refs(ast);

        while let Some(symbol) = queue.pop_front() {
            if reachable.contains(&symbol) {
                continue;
            }
            reachable.insert(symbol.clone());

            // 同名方法组的声明序（逐候选标记的配对键：第 k 个同名声明 ↔ 决议名 #k）
            let mut ordinal: HashMap<String, usize> = HashMap::new();
            for def in &self.all_defs {
                // 方法只以**短名**出现在引用里（`obj.render()` 收集到的是 render，
                // 不是 Point.render），故方法用短名探针——被调用的方法必须可达。
                // WBS 4.9.6 / #450 逐候选：该名字组**有决议记录**时只认被命中的
                // 那一个声明（未被命中的兄弟重载仍不可达 ⇒ 如实报 W1005）；
                // 无任何决议记录（调用点歧义 / 未经决议路径）时回落短名共判（宁漏报）
                let hit = match def.kind {
                    SymbolKind::Method => {
                        let k = ordinal.entry(def.name.clone()).or_insert(0);
                        let this = *k;
                        *k += 1;
                        // WBS 3.20.3 / issue #505：算子派发命中**整组**——算子
                        // 调用点不产生 D20 重载决议条目（`operator_dispatches`
                        // 是独立义务通道），派发名 `Type.method` 不分重载候选，
                        // 故组内全部声明按活判定
                        if self.operator_dispatched.contains(&def.name) {
                            true
                        } else if self.method_group_has_marks(&def.name) {
                            self.resolved_methods.contains(&(def.name.clone(), this))
                        } else {
                            let short = def.name.rsplit('.').next().unwrap_or(&def.name);
                            ast_references.contains(short)
                        }
                    }
                    _ => ast_references.contains(def.name.as_str()),
                };
                if hit && !reachable.contains(&def.name) {
                    queue.push_back(def.name.clone());
                }
            }
        }

        reachable
    }

    /// 找出未被引用的定义（RFC-029f + RFC-029g）
    ///
    /// 判定只有两道闸：文件内可达性，以及包内引用池。
    /// - 提供引用池（Bin/Lib，项目模式）：池中命中即活——跨文件消费者在
    ///   本文件不可见，这是「宁漏报」方向（也曾是 Lib 非 pub 误报的根因）。
    ///   池的身份是 `(模块, 名字)`：池里出现的是**别的模块**对该名字的引用
    ///   （每个模块自有的名字在装配期已剔除），故同名的局部绑定不构成消费。
    /// - 无引用池（`FileRole::Script`）：只按文件内可达性判定，
    ///   未引用的顶层绑定一律报告——单文件语义不受项目内其它文件影响。
    /// - 没有任何修饰符能豁免——`pub` 关键字已由 RFC-029g 删除。
    pub fn find_unused_defs(
        &self,
        reachable: &HashSet<String>,
    ) -> Vec<DeadCodeWarning> {
        let mut warnings = Vec::new();
        // 同名方法组的声明序：有决议记录时按「命中该声明」逐候选判定（WBS 4.9.6 / #450）
        let mut ordinal: HashMap<String, usize> = HashMap::new();

        for def in &self.all_defs {
            let name = &def.name;
            // 声明序推进先于任何判定分支：同组后续声明的序号不得因本条提前
            // 判活而错位
            let this_decl = match def.kind {
                SymbolKind::Method => {
                    let k = ordinal.entry(def.name.clone()).or_insert(0);
                    let this = *k;
                    *k += 1;
                    this
                }
                _ => 0,
            };
            // 算子派发命中的组整组按活（WBS 3.20.3 / issue #505）；
            // 未命中时照走原判定（未被调用的算子方法不得被豁免）
            if matches!(def.kind, SymbolKind::Method) && self.operator_dispatched.contains(name) {
                continue;
            }
            let marked_group =
                matches!(def.kind, SymbolKind::Method) && self.method_group_has_marks(name);
            if marked_group {
                // 逐候选：决议命中**本单元这条定义** ⇒ 活；未命中 ⇒ 判死
                // （WBS 4.9.7 / #464：跨单元同名同序不被兄弟的调用豁免）
                if self.resolved_methods.contains(&(name.clone(), this_decl)) {
                    continue;
                }
            } else if Self::is_reachable(name, def, reachable) {
                continue;
            }
            // Bin/Internal: exempt_pub 豁免；Lib: cross_file_refs 豁免
            // （Script 不接池；两池的装配口径见 driver 侧 collect_compilation_refs）。
            // **有决议记录的名字组不走名字级豁免**：field_names 是全局平铺的短名集合，
            // 会让同单元/跨单元同名兄弟互相通吃（WBS 4.9.7 / issue #464）
            let short_name = name.rsplit('.').next().unwrap_or(name);
            if !marked_group && !self.exempt_pub.is_empty() && self.exempt_pub.contains(short_name)
            {
                continue;
            }
            if !marked_group
                && !self.cross_file_refs.is_empty()
                && self.cross_file_refs.contains(short_name)
            {
                continue;
            }
            let (code, message) = match def.kind {
                SymbolKind::Function => ("W1001", format!("Unused function: '{}'", name)),
                SymbolKind::Type => ("W1002", format!("Unused type: '{}'", name)),
                SymbolKind::Variable => ("W1004", format!("Unused variable: '{}'", name)),
                SymbolKind::Method => ("W1005", format!("Unused method: '{}'", name)),
            };

            warnings.push(DeadCodeWarning {
                code: code.to_string(),
                message,
                span: def.location,
            });
        }

        warnings
    }

    /// 判定定义是否可达；方法用短名匹配（调用点 `w.render()` 只出现短名）
    fn is_reachable(
        name: &str,
        def: &SymbolDef,
        reachable: &HashSet<String>,
    ) -> bool {
        if reachable.contains(name) {
            return true;
        }
        if matches!(def.kind, SymbolKind::Method) {
            let short_name = name.rsplit('.').next().unwrap_or(name);
            return reachable.contains(short_name);
        }
        false
    }

    /// 执行完整分析，返回警告列表
    pub fn analyze(
        &mut self,
        ast: &Module,
    ) -> Vec<DeadCodeWarning> {
        // 1. 收集入口点和符号定义
        self.collect_entry_points_and_definitions(ast);

        // 2. 计算可达性
        let reachable = self.compute_reachability(ast);

        // 3. 找出未被引用的定义
        self.find_unused_defs(&reachable)
    }

    /// 将警告转换为诊断信息
    pub fn to_diagnostics(
        &self,
        warnings: &[DeadCodeWarning],
    ) -> Vec<Diagnostic> {
        warnings
            .iter()
            .map(|w| {
                let def = ErrorCodeDefinition::find(&w.code).unwrap();
                let name_param = w
                    .message
                    .split(':')
                    .nth(1)
                    .unwrap_or(&w.message)
                    .trim()
                    .trim_matches('\'')
                    .to_string();
                def.builder().param("name", name_param).at(w.span).build()
            })
            .collect()
    }
}
