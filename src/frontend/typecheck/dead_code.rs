//! 死代码分析器
//!
//! 分析未使用的导出符号和未使用的导入，生成警告信息。

use std::collections::{HashMap, HashSet, VecDeque};

use crate::util::span::Span;
use crate::util::diagnostic::{Diagnostic, ErrorCodeDefinition};

use super::semantic_db::{SemanticDB, SymbolLocation};
use super::super::core::parser::ast::{Module, Stmt, StmtKind, Expr, Block};

/// 死代码分析器
pub struct DeadCodeAnalyzer {
    /// 入口点集合
    entry_points: HashSet<String>,
    /// 所有符号定义
    all_defs: HashMap<String, SymbolDef>,
    /// 符号引用（从 SemanticDB 获取）
    references: HashMap<String, Vec<SymbolLocation>>,
    /// 导入列表
    imports: Vec<ImportInfo>,
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
    /// 是否导出
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

/// 导入信息
#[derive(Debug, Clone)]
pub struct ImportInfo {
    /// 导入路径
    pub path: String,
    /// 导入的符号列表（None 表示全部导入）
    pub items: Option<Vec<String>>,
    /// 导入位置
    pub location: Span,
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
            references: HashMap::new(),
            imports: Vec::new(),
        }
    }

    /// 收集入口点和符号定义（合并处理以减少代码重复）
    ///
    /// 入口点包括：
    /// 1. `main` 函数
    /// 2. 所有 `pub` 导出的函数
    /// 3. 所有类型定义（因为可被实例化）
    /// 4. 所有方法绑定
    pub fn collect_entry_points_and_definitions(
        &mut self,
        ast: &Module,
    ) {
        for stmt in &ast.items {
            match &stmt.kind {
                StmtKind::Fn { name, is_pub, .. } => {
                    let def = SymbolDef {
                        name: name.clone(),
                        kind: SymbolKind::Function,
                        location: stmt.span,
                        is_exported: *is_pub,
                    };

                    // main 函数是入口点
                    if name == "main" {
                        self.entry_points.insert(name.clone());
                    }
                    // pub 函数也是入口点
                    if *is_pub {
                        self.entry_points.insert(name.clone());
                    }

                    self.all_defs.insert(name.clone(), def);
                }
                StmtKind::TypeDef { name, .. } => {
                    // 类型是入口点（可被实例化）
                    self.entry_points.insert(name.clone());

                    self.all_defs.insert(
                        name.clone(),
                        SymbolDef {
                            name: name.clone(),
                            kind: SymbolKind::Type,
                            location: stmt.span,
                            is_exported: true,
                        },
                    );
                }
                StmtKind::Var { name, .. } => {
                    self.all_defs.insert(
                        name.clone(),
                        SymbolDef {
                            name: name.clone(),
                            kind: SymbolKind::Variable,
                            location: stmt.span,
                            is_exported: false,
                        },
                    );
                }
                StmtKind::MethodBind {
                    type_name,
                    method_name,
                    ..
                } => {
                    let full_name = format!("{}.{}", type_name, method_name);
                    self.entry_points.insert(full_name.clone());

                    self.all_defs.insert(
                        full_name.clone(),
                        SymbolDef {
                            name: full_name,
                            kind: SymbolKind::Method,
                            location: stmt.span,
                            is_exported: true,
                        },
                    );
                }
                StmtKind::Use { path, items, .. } => {
                    self.imports.push(ImportInfo {
                        path: path.clone(),
                        items: items.clone(),
                        location: stmt.span,
                    });
                }
                _ => {}
            }
        }
    }

    /// 从 AST 识别入口点（兼容旧接口）
    #[deprecated(
        since = "0.6.5",
        note = "Use collect_entry_points_and_definitions instead"
    )]
    pub fn collect_entry_points(
        &mut self,
        ast: &Module,
    ) {
        self.collect_entry_points_and_definitions(ast);
    }

    /// 收集所有符号定义（兼容旧接口）
    #[deprecated(
        since = "0.6.5",
        note = "Use collect_entry_points_and_definitions instead"
    )]
    pub fn collect_definitions(
        &mut self,
        _ast: &Module,
    ) {
        // 已合并到 collect_entry_points_and_definitions
    }

    /// 收集符号引用
    pub fn collect_references(
        &mut self,
        semantic_db: &SemanticDB,
    ) {
        // 从 SemanticDB 获取所有符号引用
        for name in semantic_db.defined_symbols() {
            if let Some(refs) = semantic_db.get_symbol_refs(name) {
                self.references.insert(name.clone(), refs.to_vec());
            }
        }
    }

    /// 从 AST 中收集所有符号引用（用于补充 SemanticDB 可能缺失的引用）
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
                Expr::FieldAccess { expr, .. } => {
                    collect_from_expr(expr, referenced);
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
                    elif_branches,
                    else_branch,
                    ..
                } => {
                    collect_from_expr(condition, referenced);
                    collect_from_block(then_branch, referenced);
                    for (_, branch) in elif_branches {
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
                    for param in params {
                        referenced.insert(param.name.clone());
                    }
                    collect_from_block(body, referenced);
                }
                Expr::Lambda { params, body, .. } => {
                    for param in params {
                        referenced.insert(param.name.clone());
                    }
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

        fn collect_from_block(
            block: &Block,
            referenced: &mut HashSet<String>,
        ) {
            for stmt in &block.stmts {
                collect_from_stmt(stmt, referenced);
            }
            if let Some(expr) = &block.expr {
                collect_from_expr(expr, referenced);
            }
        }

        fn collect_from_stmt(
            stmt: &Stmt,
            referenced: &mut HashSet<String>,
        ) {
            match &stmt.kind {
                StmtKind::Expr(expr) => {
                    collect_from_expr(expr, referenced);
                }
                StmtKind::Var {
                    name: _,
                    initializer: Some(expr),
                    ..
                } => {
                    collect_from_expr(expr, referenced);
                }
                StmtKind::Var {
                    name: _,
                    initializer: None,
                    ..
                } => {}
                StmtKind::Fn { name, body, .. } => {
                    referenced.insert(name.clone());
                    collect_from_block(
                        &Block {
                            stmts: body.0.clone(),
                            expr: body.1.clone(),
                            span: stmt.span,
                        },
                        referenced,
                    );
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
                    elif_branches,
                    else_branch,
                    ..
                } => {
                    collect_from_expr(condition, referenced);
                    collect_from_block(then_branch, referenced);
                    for (_, branch) in elif_branches {
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
            collect_from_stmt(stmt, &mut referenced);
        }

        referenced
    }

    /// 从入口点出发，计算可达符号集合
    pub fn compute_reachability(
        &self,
        ast: &Module,
    ) -> HashSet<String> {
        let mut reachable = HashSet::new();
        let mut queue = VecDeque::new();

        // 从入口点开始
        for entry in &self.entry_points {
            queue.push_back(entry.clone());
        }

        // 收集 AST 中的所有引用
        let ast_references = self.collect_references_from_ast(ast);

        // BFS 遍历
        while let Some(symbol) = queue.pop_front() {
            if reachable.contains(&symbol) {
                continue;
            }
            reachable.insert(symbol.clone());

            // 添加该符号引用的其他符号
            if let Some(refs) = self.references.get(&symbol) {
                for _ref in refs {
                    // 从引用中提取符号名（简化处理）
                    // 实际应该根据上下文推断
                }
            }

            // 也从 AST 引用中查找
            // 如果某个符号被引用，我们需要追踪它引用的其他符号
            for def_name in self.all_defs.keys() {
                // 如果 def_name 被引用过，添加到队列
                if ast_references.contains(def_name) && !reachable.contains(def_name) {
                    queue.push_back(def_name.clone());
                }
            }
        }

        reachable
    }

    /// 找出未使用的导出符号
    pub fn find_unused_exports(
        &self,
        reachable: &HashSet<String>,
    ) -> Vec<DeadCodeWarning> {
        let mut warnings = Vec::new();

        for (name, def) in &self.all_defs {
            if def.is_exported && !reachable.contains(name) {
                let (code, message) = match def.kind {
                    SymbolKind::Function => {
                        ("W1001", format!("Unused exported function: '{}'", name))
                    }
                    SymbolKind::Type => ("W1002", format!("Unused exported type: '{}'", name)),
                    SymbolKind::Variable => {
                        ("W1004", format!("Unused exported variable: '{}'", name))
                    }
                    SymbolKind::Method => ("W1005", format!("Unused exported method: '{}'", name)),
                };

                warnings.push(DeadCodeWarning {
                    code: code.to_string(),
                    message,
                    span: def.location,
                });
            }
        }

        warnings
    }

    /// 找出未使用的导入
    pub fn find_unused_imports(
        &self,
        reachable: &HashSet<String>,
    ) -> Vec<DeadCodeWarning> {
        let mut warnings = Vec::new();

        for import in &self.imports {
            let items = match &import.items {
                Some(items) => items.clone(),
                None => {
                    // 整个模块导入，需要检查模块名是否被使用
                    vec![import.path.clone()]
                }
            };

            for item in items {
                if !reachable.contains(&item) && !reachable.contains(&import.path) {
                    warnings.push(DeadCodeWarning {
                        code: "W1003".to_string(),
                        message: format!("Unused import: '{}'", item),
                        span: import.location,
                    });
                }
            }
        }

        warnings
    }

    /// 执行完整分析，返回警告列表
    pub fn analyze(
        &mut self,
        ast: &Module,
        semantic_db: &SemanticDB,
    ) -> Vec<DeadCodeWarning> {
        // 1. 收集入口点和符号定义（合并处理）
        self.collect_entry_points_and_definitions(ast);

        // 2. 收集符号引用
        self.collect_references(semantic_db);

        // 3. 计算可达性
        let reachable = self.compute_reachability(ast);

        // 4. 找出未使用的导出
        let mut warnings = self.find_unused_exports(&reachable);

        // 6. 找出未使用的导入
        warnings.extend(self.find_unused_imports(&reachable));

        warnings
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
                    .to_string();
                def.builder().param("name", name_param).at(w.span).build()
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_collect_entry_points() {
        let mut analyzer = DeadCodeAnalyzer::new();

        // 创建一个简单的 AST
        let ast = Module {
            items: vec![],
            span: Span::dummy(),
        };

        analyzer.collect_entry_points_and_definitions(&ast);
        assert!(analyzer.entry_points.is_empty());
    }

    #[test]
    fn test_unused_export_warning() {
        let warning = DeadCodeWarning {
            code: "W1001".to_string(),
            message: "Unused exported function: 'foo'".to_string(),
            span: Span::dummy(),
        };

        assert_eq!(warning.code, "W1001");
        assert!(warning.message.contains("foo"));
    }

    #[test]
    fn test_multi_level_dependency() {
        // 测试多层级依赖场景下的入口点识别
        let mut analyzer = DeadCodeAnalyzer::new();

        // 创建一个模拟依赖链的 AST
        // pub main -> 入口点
        // pub helper_a -> 入口点（pub）
        // pub helper_b -> 入口点（pub）
        // fn private_func -> 非入口点（不是 pub）

        let ast = Module {
            items: vec![
                // pub main - 入口点
                Stmt {
                    kind: StmtKind::Fn {
                        name: "main".to_string(),
                        is_pub: true,
                        params: vec![],
                        body: (vec![], None),
                        generic_params: vec![],
                        type_annotation: None,
                        eval: None,
                    },
                    span: Span::dummy(),
                },
                // pub helper_a - 入口点
                Stmt {
                    kind: StmtKind::Fn {
                        name: "helper_a".to_string(),
                        is_pub: true,
                        params: vec![],
                        body: (vec![], None),
                        generic_params: vec![],
                        type_annotation: None,
                        eval: None,
                    },
                    span: Span::dummy(),
                },
                // pub helper_b - 入口点
                Stmt {
                    kind: StmtKind::Fn {
                        name: "helper_b".to_string(),
                        is_pub: true,
                        params: vec![],
                        body: (vec![], None),
                        generic_params: vec![],
                        type_annotation: None,
                        eval: None,
                    },
                    span: Span::dummy(),
                },
                // fn private_func - 非入口点
                Stmt {
                    kind: StmtKind::Fn {
                        name: "private_func".to_string(),
                        is_pub: false,
                        params: vec![],
                        body: (vec![], None),
                        generic_params: vec![],
                        type_annotation: None,
                        eval: None,
                    },
                    span: Span::dummy(),
                },
            ],
            span: Span::dummy(),
        };

        analyzer.collect_entry_points_and_definitions(&ast);

        // main 和 pub 函数应该是入口点
        assert!(
            analyzer.entry_points.contains("main"),
            "main should be entry point"
        );
        assert!(
            analyzer.entry_points.contains("helper_a"),
            "helper_a should be entry point (pub)"
        );
        assert!(
            analyzer.entry_points.contains("helper_b"),
            "helper_b should be entry point (pub)"
        );

        // private_func 不是入口点
        assert!(
            !analyzer.entry_points.contains("private_func"),
            "private_func should NOT be entry point (not pub)"
        );

        // 所有函数都应该在 all_defs 中
        assert_eq!(analyzer.all_defs.len(), 4);
    }
}
