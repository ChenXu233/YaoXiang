//! 死代码分析器
//!
//! 识别从未被引用的定义，生成警告信息。
//! 码义（RFC-013 + RFC-029f + RFC-029g）：顶层绑定不因任何修饰符豁免——
//! 可达性由文件内引用与包内引用池（[`DeadCodeAnalyzer::set_project_refs`]）
//! 共同判定。单文件（Script）路径没有包视角，未引用的顶层绑定一律报
//! W1001/W1002/W1004/W1005。`pub` 关键字已由 RFC-029g 删除，不再是豁免来源。
//! 未使用导入（W1003）由 typecheck 的 use elaboration 检测，不在此处。

use std::collections::{HashMap, HashSet, VecDeque};

use crate::util::span::Span;
use crate::util::diagnostic::{Diagnostic, ErrorCodeDefinition};

use crate::frontend::core::parser::ast::{Module, Stmt, StmtKind, Expr, Block};

/// 死代码分析器
pub struct DeadCodeAnalyzer {
    /// 入口点集合（可达性根：`main`）
    entry_points: HashSet<String>,
    /// 所有符号定义
    all_defs: HashMap<String, SymbolDef>,
    /// Bin/Internal 的 pub 项豁免池（扫描目录所得，已知缺陷见 RFC-029f）
    exempt_pub: HashSet<String>,
    /// Lib/Script 的跨文件引用池（扫描目录所得，已知缺陷见 RFC-029f）
    cross_file_refs: HashSet<String>,
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
            all_defs: HashMap::new(),
            exempt_pub: HashSet::new(),
            cross_file_refs: HashSet::new(),
        }
    }

    /// 设置 Bin/Internal 的 pub 项豁免池（扫描目录所得，已知缺陷见 RFC-029f）
    pub fn set_exempt_pub(
        &mut self,
        names: HashSet<String>,
    ) {
        self.exempt_pub = names;
    }

    /// 设置 Lib/Script 的跨文件引用池（扫描目录所得，已知缺陷见 RFC-029f）
    pub fn set_cross_file_refs(
        &mut self,
        refs: HashSet<String>,
    ) {
        self.cross_file_refs = refs;
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
                    self.all_defs.insert(
                        def_name.clone(),
                        SymbolDef {
                            name: def_name.clone(),
                            kind,
                            location: stmt.span,
                        },
                    );
                    if !is_method && name == "main" {
                        self.entry_points.insert(name.clone());
                    }
                }
                StmtKind::TypeDefinition { name, .. } => {
                    self.all_defs.insert(
                        name.clone(),
                        SymbolDef {
                            name: name.clone(),
                            kind: SymbolKind::Type,
                            location: stmt.span,
                        },
                    );
                }
                _ => {}
            }
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
        let mut referenced = HashSet::new();

        fn collect_from_expr(
            expr: &Expr,
            referenced: &mut HashSet<String>,
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
                Expr::FnDef {
                    name, params, body, ..
                } => {
                    referenced.insert(name.clone());
                    collect_params_names_and_types(params, referenced);
                    collect_from_block(body, referenced);
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
            referenced: &mut HashSet<String>,
        ) {
            for param in params {
                referenced.insert(param.name.clone());
                if let Some(ty) = &param.ty {
                    ty.collect_name_refs(referenced);
                }
            }
        }

        fn collect_from_block(
            block: &Block,
            referenced: &mut HashSet<String>,
        ) {
            for stmt in &block.stmts {
                collect_from_stmt(stmt, referenced, false);
            }
        }

        /// match 模式中的名字引用：Identifier 兼作常量模式（无法与绑定区分，
        /// 一律收集），Struct/Union 头部是类型名引用，Guard 条件是真实表达式
        fn collect_pattern_refs(
            pattern: &crate::frontend::core::parser::ast::Pattern,
            referenced: &mut HashSet<String>,
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
            referenced: &mut HashSet<String>,
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
                    signature_params,
                    ..
                } => {
                    if let Some(expr) = value {
                        collect_from_expr(expr, referenced);
                    }
                    // 目标名：顶层是定义（不算引用，否则所有定义"自我可达"），
                    // 嵌套是写入即使用——通用走查覆盖 Var/FieldAccess/Index 等
                    // 复合目标（`counts["k"] = v` 是对 counts 的使用）
                    if !in_toplevel {
                        collect_from_expr(target, referenced);
                    }
                    if let Some(ty) = type_annotation {
                        ty.collect_name_refs(referenced);
                    }
                    for param in signature_params {
                        if let Some(ty) = &param.ty {
                            ty.collect_name_refs(referenced);
                        }
                    }
                }
                // 元组解构赋值：嵌套目标是写入即使用；顶层是定义（不计引用）
                StmtKind::DestructureAssign { names, rhs, .. } => {
                    collect_from_expr(rhs, referenced);
                    if !in_toplevel {
                        for name in names {
                            referenced.insert(name.name.clone());
                        }
                    }
                }
                StmtKind::TypeDefinition {
                    signature_params,
                    definition,
                    ..
                } => {
                    for param in signature_params {
                        if let Some(ty) = &param.ty {
                            ty.collect_name_refs(referenced);
                        }
                    }
                    definition.collect_name_refs(referenced);
                }
                StmtKind::For {
                    var,
                    iterable,
                    body,
                    ..
                } => {
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
                _ => {}
            }
        }

        for stmt in &ast.items {
            collect_from_stmt(stmt, &mut referenced, true);
        }

        referenced
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

            for def_name in self.all_defs.keys() {
                if ast_references.contains(def_name) && !reachable.contains(def_name) {
                    queue.push_back(def_name.clone());
                }
            }
        }

        reachable
    }

    /// 找出未被引用的定义（RFC-029f + RFC-029g）
    ///
    /// 判定只有两道闸：文件内可达性，以及包内引用池。
    /// - 提供引用池（项目模式）：池中命中即活——跨文件消费者在本文件不可见，
    ///   这是「宁漏报」方向（也曾是 Lib 非 pub 误报的根因）。
    /// - 无引用池（单文件 Script 路径）：只按文件内可达性判定，
    ///   未引用的顶层绑定一律报告。
    /// - 没有任何修饰符能豁免——`pub` 关键字已由 RFC-029g 删除。
    pub fn find_unused_defs(
        &self,
        reachable: &HashSet<String>,
    ) -> Vec<DeadCodeWarning> {
        let mut warnings = Vec::new();

        for (name, def) in &self.all_defs {
            if Self::is_reachable(name, def, reachable) {
                continue;
            }
            // Bin/Internal: exempt_pub 豁免；Lib/Script: cross_file_refs 豁免
            let short_name = name.rsplit('.').next().unwrap_or(name);
            if !self.exempt_pub.is_empty() && self.exempt_pub.contains(short_name) {
                continue;
            }
            if !self.cross_file_refs.is_empty() && self.cross_file_refs.contains(short_name) {
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
