//! 死代码分析器
//!
//! 识别从未被引用的私有定义，生成警告信息。
//! 码义（RFC-013，#321 定案 B）：`pub` 定义是对外接口，永不报告；
//! 仅私有（非 pub）定义参与死代码判定（W1001/W1002/W1004/W1005）。
//! 未使用导入（W1003）由 typecheck 的 use elaboration 检测，不在此处。

use std::collections::{HashMap, HashSet, VecDeque};

use crate::util::span::Span;
use crate::util::diagnostic::{Diagnostic, ErrorCodeDefinition};

use crate::frontend::core::parser::ast::{Module, Stmt, StmtKind, Expr, Block};

/// 死代码分析器
pub struct DeadCodeAnalyzer {
    /// 入口点集合（可达性根：main 与 pub 函数/类型）
    entry_points: HashSet<String>,
    /// 所有符号定义
    all_defs: HashMap<String, SymbolDef>,
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
    /// 是否导出（pub = 对外接口，永不报告）
    pub is_exported: bool,
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
        }
    }

    /// 收集入口点和符号定义（合并处理以减少代码重复）
    ///
    /// 入口点（可达性根）：
    /// 1. `main` 函数
    /// 2. `pub` 导出的函数与 `pub` 类型定义（对外接口，#321 定案 B）
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
                    value,
                    is_pub,
                    ..
                } => {
                    let Some((name, type_name)) = target.receiver_parts() else {
                        continue;
                    };
                    let (params, body): (Vec<_>, Vec<_>) = match value {
                        Some(v) => v.callable_parts(),
                        None => (Vec::new(), Vec::new()),
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
                            is_exported: *is_pub,
                        },
                    );
                    if !is_method && name == "main" {
                        self.entry_points.insert(name.clone());
                    }
                    if *is_pub {
                        self.entry_points.insert(def_name);
                    }
                }
                StmtKind::TypeDefinition { name, is_pub, .. } => {
                    self.all_defs.insert(
                        name.clone(),
                        SymbolDef {
                            name: name.clone(),
                            kind: SymbolKind::Type,
                            location: stmt.span,
                            is_exported: *is_pub,
                        },
                    );
                    // pub 类型是对外接口（可达根）；私有类型仅被引用时可达
                    if *is_pub {
                        self.entry_points.insert(name.clone());
                    }
                }
                _ => {}
            }
        }
    }

    /// 从 AST 中收集所有符号引用（定义处不算引用）
    ///
    /// 顶层 Assign 的目标名是定义不是使用——计入会使所有定义"自我可达"，
    /// 死代码判定永远落空（#321 M2 复盘）；函数体内的赋值目标是写入即使用。
    fn collect_references_from_ast(
        &self,
        ast: &Module,
    ) -> HashSet<String> {
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
                    var,
                    iterable,
                    condition,
                    ..
                } => {
                    referenced.insert(var.clone());
                    collect_from_expr(element, referenced);
                    collect_from_expr(iterable, referenced);
                    if let Some(cond) = condition {
                        collect_from_expr(cond, referenced);
                    }
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
                    // 嵌套是写入即使用；复合目标的被赋值对象同样是使用
                    if !in_toplevel {
                        if let Expr::Var(name, _) = target.as_ref() {
                            referenced.insert(name.clone());
                        }
                        if let Expr::FieldAccess { expr, field, .. } = target.as_ref() {
                            collect_from_expr(expr, referenced);
                            referenced.insert(field.clone());
                        }
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

        let ast_references = self.collect_references_from_ast(ast);

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

    /// 找出未被引用的私有定义
    ///
    /// pub 定义是对外接口，永不报告（#321 定案 B）。
    pub fn find_unused_private_defs(
        &self,
        reachable: &HashSet<String>,
    ) -> Vec<DeadCodeWarning> {
        let mut warnings = Vec::new();

        for (name, def) in &self.all_defs {
            if def.is_exported || Self::is_reachable(name, def, reachable) {
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

        // 3. 找出未被引用的私有定义
        self.find_unused_private_defs(&reachable)
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
