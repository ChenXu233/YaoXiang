//! 编译世界（World）
//!
//! 管理编译器状态和符号索引，为 LSP 提供语义分析支持。

use crate::frontend::core::lexer::symbols::{IndexedSymbol, SymbolIndex, SymbolKind, SymbolLocation};
use crate::frontend::core::parser::ast::{Block, Expr, Module, Stmt, StmtKind};
use crate::frontend::typecheck::semantic_db::SemanticDB;
use crate::lsp::handlers::semantic_tokens::SemanticTokensCache;

/// 编译世界
///
/// 聚合编译器运行所需的全局状态：
/// - 符号索引（用于 go-to-definition、find-references 等）
/// - 语义数据库（用于 semantic tokens、代码高亮等）
/// - 语义 tokens 缓存（用于 delta 模式增量更新）
/// - 编译器实例管理
///
/// 阶段 3 扩展：
/// - 从解析结果构建符号索引
/// - 支持增量更新（文件级别）
#[derive(Debug, Default)]
pub struct World {
    /// 全局符号索引
    symbol_index: SymbolIndex,
    /// 语义数据库（语义 token、作用域等）
    semantic_db: SemanticDB,
    /// 语义 tokens 版本缓存（支持 delta 更新）
    semantic_tokens_cache: SemanticTokensCache,
}

impl World {
    /// 创建新的编译世界
    pub fn new() -> Self {
        Self {
            symbol_index: SymbolIndex::new(),
            semantic_db: SemanticDB::default(),
            semantic_tokens_cache: SemanticTokensCache::new(),
        }
    }

    /// 获取符号索引（不可变）
    pub fn symbol_index(&self) -> &SymbolIndex {
        &self.symbol_index
    }

    /// 获取符号索引（可变）
    pub fn symbol_index_mut(&mut self) -> &mut SymbolIndex {
        &mut self.symbol_index
    }

    /// 获取语义数据库（不可变）
    pub fn semantic_db(&self) -> &SemanticDB {
        &self.semantic_db
    }

    /// 获取语义数据库（可变）
    pub fn semantic_db_mut(&mut self) -> &mut SemanticDB {
        &mut self.semantic_db
    }

    /// 获取语义 tokens 缓存（可变）
    pub fn semantic_tokens_cache_mut(&mut self) -> &mut SemanticTokensCache {
        &mut self.semantic_tokens_cache
    }

    /// 同时获取语义数据库（不可变）和 tokens 缓存（可变）
    ///
    /// 解决 Rust 借用规则限制：无法同时通过方法获取不可变和可变引用。
    pub fn semantic_db_and_cache(&mut self) -> (&SemanticDB, &mut SemanticTokensCache) {
        (&self.semantic_db, &mut self.semantic_tokens_cache)
    }

    /// 从 TypeCheckResult 的 SemanticDB 合并语义信息
    pub fn update_semantic_db(
        &mut self,
        other: SemanticDB,
    ) {
        self.semantic_db = other;
    }

    /// 移除某个文件的所有符号和语义信息（文件关闭或重新解析时调用）
    pub fn remove_file_symbols(
        &mut self,
        file_path: &str,
    ) {
        self.symbol_index.remove_file(file_path);
        self.semantic_db.remove_file(file_path);
    }

    /// 获取索引中的符号总数
    pub fn symbol_count(&self) -> usize {
        self.symbol_index.symbol_count()
    }

    /// 加载标准库符号到索引
    ///
    /// 从 ModuleRegistry 获取所有标准库模块的导出，
    /// 将其作为虚拟符号添加到符号索引中。
    /// 如果存在 `.yx` 接口文件，解析它获取精确的 Span 信息；
    /// 否则使用 Span::dummy() 作为占位。
    pub fn load_std_library_symbols(
        &mut self,
        project_dir: Option<&std::path::Path>,
    ) {
        use crate::util::span::Span;

        for module_info in crate::std::all_module_infos() {
            let module_name = module_info
                .path
                .strip_prefix("std.")
                .unwrap_or(&module_info.path);

            // 尝试查找接口文件
            let interface_file =
                crate::std::gen_interfaces::find_std_interface_file(project_dir, module_name);

            let virtual_file_path = if let Some(ref path) = interface_file {
                // 将文件路径转为 URI 格式
                format!("file:///{}", path.to_string_lossy().replace('\\', "/"))
            } else {
                // 使用虚拟路径标识标准库符号
                format!("std://{}", module_info.path)
            };

            // 尝试解析接口文件获取精确 Span
            let parsed_spans = if let Some(ref path) = interface_file {
                parse_interface_file_spans(path)
            } else {
                std::collections::HashMap::new()
            };

            for export in module_info.exports.values() {
                if export.kind == crate::frontend::module::ExportKind::SubModule {
                    continue;
                }

                let kind = match export.kind {
                    crate::frontend::module::ExportKind::Function => SymbolKind::Function,
                    crate::frontend::module::ExportKind::Constant => SymbolKind::Variable,
                    crate::frontend::module::ExportKind::Type => SymbolKind::Type,
                    crate::frontend::module::ExportKind::SubModule => continue,
                };

                let span = parsed_spans
                    .get(&export.name)
                    .copied()
                    .unwrap_or_else(Span::dummy);

                self.symbol_index.add(IndexedSymbol {
                    name: export.name.clone(),
                    kind,
                    arity: None,
                    location: SymbolLocation::new(virtual_file_path.clone(), span),
                });
            }
        }
    }

    /// 加载内置类型到符号索引
    ///
    /// 将 Int, Float, Bool, String, Void, Char 等内置类型添加到符号索引，
    /// 使 LSP 功能（如 hover、definition）能够识别这些内置类型。
    pub fn load_builtin_types(&mut self) {
        use crate::util::span::Span;

        // YaoXiang 语言的核心内置类型
        let builtin_types = ["Int", "Float", "Bool", "String", "Void", "Char"];

        for type_name in builtin_types {
            self.symbol_index.add(IndexedSymbol {
                name: type_name.to_string(),
                kind: SymbolKind::Type,
                arity: None,
                location: SymbolLocation::new("builtin://types".to_string(), Span::dummy()),
            });
        }
    }

    /// 从解析后的 AST 更新指定文件的符号索引
    ///
    /// 先移除该文件的旧符号，再从 AST 提取新符号。
    /// 支持提取：
    /// - 模块顶层符号（Var, Fn, TypeDef, MethodBind）
    /// - Use 语句导入的符号
    /// - 函数参数和局部变量
    pub fn update_index_from_ast(
        &mut self,
        file_path: &str,
        module: &Module,
    ) {
        // 先移除旧索引
        self.symbol_index.remove_file(file_path);

        // 从模块顶层语句提取符号
        for stmt in &module.items {
            match &stmt.kind {
                StmtKind::Var { name, .. } => {
                    self.symbol_index.add(IndexedSymbol {
                        name: name.clone(),
                        kind: SymbolKind::Variable,
                        arity: None,
                        location: SymbolLocation::new(file_path.to_string(), stmt.span),
                    });
                }
                StmtKind::Fn {
                    name,
                    params,
                    generic_params,
                    body,
                    ..
                } => {
                    let kind = if generic_params.is_empty() {
                        SymbolKind::Function
                    } else {
                        SymbolKind::GenericFunction
                    };
                    self.symbol_index.add(IndexedSymbol {
                        name: name.clone(),
                        kind,
                        arity: Some(params.len()),
                        location: SymbolLocation::new(file_path.to_string(), stmt.span),
                    });

                    // 提取函数参数
                    for param in params {
                        self.symbol_index.add(IndexedSymbol {
                            name: param.name.clone(),
                            kind: SymbolKind::Variable,
                            arity: None,
                            location: SymbolLocation::new(file_path.to_string(), param.span),
                        });
                    }

                    // 递归提取函数体中的局部变量
                    for body_stmt in &body.0 {
                        self.index_stmt_symbols(file_path, body_stmt);
                    }
                    if let Some(ret_expr) = &body.1 {
                        self.index_expr_symbols(file_path, ret_expr);
                    }
                }
                StmtKind::TypeDef {
                    name,
                    generic_params,
                    ..
                } => {
                    let kind = if generic_params.is_empty() {
                        SymbolKind::Type
                    } else {
                        SymbolKind::GenericType
                    };
                    self.symbol_index.add(IndexedSymbol {
                        name: name.clone(),
                        kind,
                        arity: None,
                        location: SymbolLocation::new(file_path.to_string(), stmt.span),
                    });
                }
                StmtKind::MethodBind {
                    type_name,
                    method_name,
                    params,
                    body,
                    ..
                } => {
                    let full_name = format!("{}.{}", type_name, method_name);
                    self.symbol_index.add(IndexedSymbol {
                        name: full_name,
                        kind: SymbolKind::Function,
                        arity: Some(params.len()),
                        location: SymbolLocation::new(file_path.to_string(), stmt.span),
                    });

                    // 提取方法参数
                    for param in params {
                        self.symbol_index.add(IndexedSymbol {
                            name: param.name.clone(),
                            kind: SymbolKind::Variable,
                            arity: None,
                            location: SymbolLocation::new(file_path.to_string(), param.span),
                        });
                    }

                    // 递归提取方法体中的局部变量
                    for body_stmt in &body.0 {
                        self.index_stmt_symbols(file_path, body_stmt);
                    }
                    if let Some(ret_expr) = &body.1 {
                        self.index_expr_symbols(file_path, ret_expr);
                    }
                }
                StmtKind::Use { path, items, .. } => {
                    // 将 Use 导入的符号加入索引
                    self.index_use_symbols(file_path, path, items.as_deref(), stmt.span);
                }
                _ => {}
            }
        }
    }

    /// 递归提取语句中的局部变量符号
    fn index_stmt_symbols(
        &mut self,
        file_path: &str,
        stmt: &Stmt,
    ) {
        match &stmt.kind {
            StmtKind::Var {
                name, initializer, ..
            } => {
                self.symbol_index.add(IndexedSymbol {
                    name: name.clone(),
                    kind: SymbolKind::Variable,
                    arity: None,
                    location: SymbolLocation::new(file_path.to_string(), stmt.span),
                });
                if let Some(init) = initializer {
                    self.index_expr_symbols(file_path, init);
                }
            }
            StmtKind::For {
                var,
                iterable,
                body,
                ..
            } => {
                self.symbol_index.add(IndexedSymbol {
                    name: var.clone(),
                    kind: SymbolKind::Variable,
                    arity: None,
                    location: SymbolLocation::new(file_path.to_string(), stmt.span),
                });
                self.index_expr_symbols(file_path, iterable);
                self.index_block_symbols(file_path, body);
            }
            StmtKind::Fn {
                name,
                params,
                body,
                generic_params,
                ..
            } => {
                let kind = if generic_params.is_empty() {
                    SymbolKind::Function
                } else {
                    SymbolKind::GenericFunction
                };
                self.symbol_index.add(IndexedSymbol {
                    name: name.clone(),
                    kind,
                    arity: Some(params.len()),
                    location: SymbolLocation::new(file_path.to_string(), stmt.span),
                });
                for param in params {
                    self.symbol_index.add(IndexedSymbol {
                        name: param.name.clone(),
                        kind: SymbolKind::Variable,
                        arity: None,
                        location: SymbolLocation::new(file_path.to_string(), param.span),
                    });
                }
                for body_stmt in &body.0 {
                    self.index_stmt_symbols(file_path, body_stmt);
                }
                if let Some(ret_expr) = &body.1 {
                    self.index_expr_symbols(file_path, ret_expr);
                }
            }
            StmtKind::If {
                condition,
                then_branch,
                elif_branches,
                else_branch,
                ..
            } => {
                self.index_expr_symbols(file_path, condition);
                self.index_block_symbols(file_path, then_branch);
                for (cond, body) in elif_branches {
                    self.index_expr_symbols(file_path, cond);
                    self.index_block_symbols(file_path, body);
                }
                if let Some(else_b) = else_branch {
                    self.index_block_symbols(file_path, else_b);
                }
            }
            StmtKind::Expr(expr) => {
                self.index_expr_symbols(file_path, expr);
            }
            _ => {}
        }
    }

    /// 递归提取表达式中定义的符号（如 Lambda 参数）
    fn index_expr_symbols(
        &mut self,
        file_path: &str,
        expr: &Expr,
    ) {
        match expr {
            Expr::Lambda { params, body, .. } => {
                for param in params {
                    self.symbol_index.add(IndexedSymbol {
                        name: param.name.clone(),
                        kind: SymbolKind::Variable,
                        arity: None,
                        location: SymbolLocation::new(file_path.to_string(), param.span),
                    });
                }
                self.index_block_symbols(file_path, body);
            }
            Expr::FnDef {
                name, params, body, ..
            } => {
                if !name.is_empty() {
                    self.symbol_index.add(IndexedSymbol {
                        name: name.clone(),
                        kind: SymbolKind::Function,
                        arity: Some(params.len()),
                        location: SymbolLocation::new(file_path.to_string(), body.span),
                    });
                }
                for param in params {
                    self.symbol_index.add(IndexedSymbol {
                        name: param.name.clone(),
                        kind: SymbolKind::Variable,
                        arity: None,
                        location: SymbolLocation::new(file_path.to_string(), param.span),
                    });
                }
                self.index_block_symbols(file_path, body);
            }
            Expr::Block(block) => {
                self.index_block_symbols(file_path, block);
            }
            Expr::If {
                condition,
                then_branch,
                elif_branches,
                else_branch,
                ..
            } => {
                self.index_expr_symbols(file_path, condition);
                self.index_block_symbols(file_path, then_branch);
                for (cond, body) in elif_branches {
                    self.index_expr_symbols(file_path, cond);
                    self.index_block_symbols(file_path, body);
                }
                if let Some(else_b) = else_branch {
                    self.index_block_symbols(file_path, else_b);
                }
            }
            Expr::For {
                var,
                iterable,
                body,
                span,
                ..
            } => {
                self.symbol_index.add(IndexedSymbol {
                    name: var.clone(),
                    kind: SymbolKind::Variable,
                    arity: None,
                    location: SymbolLocation::new(file_path.to_string(), *span),
                });
                self.index_expr_symbols(file_path, iterable);
                self.index_block_symbols(file_path, body);
            }
            Expr::While {
                condition, body, ..
            } => {
                self.index_expr_symbols(file_path, condition);
                self.index_block_symbols(file_path, body);
            }
            Expr::Match {
                expr: match_expr,
                arms,
                ..
            } => {
                self.index_expr_symbols(file_path, match_expr);
                for arm in arms {
                    self.index_block_symbols(file_path, &Box::new(arm.body.clone()));
                }
            }
            Expr::ListComp {
                element,
                iterable,
                condition,
                span,
                var,
                ..
            } => {
                self.symbol_index.add(IndexedSymbol {
                    name: var.clone(),
                    kind: SymbolKind::Variable,
                    arity: None,
                    location: SymbolLocation::new(file_path.to_string(), *span),
                });
                self.index_expr_symbols(file_path, element);
                self.index_expr_symbols(file_path, iterable);
                if let Some(cond) = condition {
                    self.index_expr_symbols(file_path, cond);
                }
            }
            _ => {}
        }
    }

    /// 递归提取块中的符号
    fn index_block_symbols(
        &mut self,
        file_path: &str,
        block: &Block,
    ) {
        for stmt in &block.stmts {
            self.index_stmt_symbols(file_path, stmt);
        }
        if let Some(expr) = &block.expr {
            self.index_expr_symbols(file_path, expr);
        }
    }

    /// 将 Use 语句导入的符号加入索引
    ///
    /// 利用 ModuleRegistry 查找模块导出，将导出的符号添加到符号索引。
    fn index_use_symbols(
        &mut self,
        file_path: &str,
        module_path: &str,
        items: Option<&[String]>,
        use_span: crate::util::span::Span,
    ) {
        // 从 ModuleRegistry 查找模块
        let registry = crate::frontend::module::registry::ModuleRegistry::with_std();
        if let Some(module) = registry.get(module_path) {
            let exports: Vec<_> = module.exports.values().collect();
            for export in exports {
                // 如果指定了 items，只导入指定的项
                if let Some(item_list) = items {
                    if !item_list.contains(&export.name) {
                        continue;
                    }
                }
                let kind = match export.kind {
                    crate::frontend::module::ExportKind::Function => SymbolKind::Function,
                    crate::frontend::module::ExportKind::Constant => SymbolKind::Variable,
                    crate::frontend::module::ExportKind::Type => SymbolKind::Type,
                    crate::frontend::module::ExportKind::SubModule => continue,
                };
                self.symbol_index.add(IndexedSymbol {
                    name: export.name.clone(),
                    kind,
                    arity: None,
                    location: SymbolLocation::new(file_path.to_string(), use_span),
                });
            }
        }
    }
}

/// 解析接口文件，提取函数名到 Span 的映射
///
/// 简单的文本解析：查找 `name:` 模式获取行号信息
fn parse_interface_file_spans(
    path: &std::path::Path
) -> std::collections::HashMap<String, crate::util::span::Span> {
    use crate::util::span::{Position, Span};

    let mut result = std::collections::HashMap::new();

    let content = match std::fs::read_to_string(path) {
        Ok(c) => c,
        Err(_) => return result,
    };

    for (line_idx, line) in content.lines().enumerate() {
        let trimmed = line.trim();
        // 匹配 "name: signature = {" 模式
        if let Some(colon_pos) = trimmed.find(':') {
            let name = trimmed[..colon_pos].trim();
            // 验证是合法的标识符名称
            if !name.is_empty()
                && !name.starts_with("//")
                && !name.starts_with("/*")
                && name.chars().all(|c| c.is_alphanumeric() || c == '_')
            {
                let line_num = line_idx + 1; // 1-indexed
                let col_start = 1;
                let col_end = col_start + name.len();
                result.insert(
                    name.to_string(),
                    Span {
                        start: Position::with_offset(line_num, col_start, 0),
                        end: Position::with_offset(line_num, col_end, 0),
                    },
                );
            }
        }
    }

    result
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::frontend::core::parser::ast::Stmt;
    use crate::util::span::{Position, Span};

    fn dummy_span() -> Span {
        Span {
            start: Position {
                line: 1,
                column: 1,
                offset: 0,
            },
            end: Position {
                line: 1,
                column: 10,
                offset: 9,
            },
        }
    }

    #[test]
    fn test_world_new() {
        let world = World::new();
        assert_eq!(world.symbol_count(), 0);
    }

    #[test]
    fn test_update_index_from_ast_var() {
        let mut world = World::new();
        let module = Module {
            items: vec![Stmt {
                kind: StmtKind::Var {
                    name: "x".to_string(),
                    name_span: Span::dummy(),
                    type_annotation: None,
                    initializer: None,
                    is_mut: false,
                },
                span: dummy_span(),
            }],
            span: dummy_span(),
        };

        world.update_index_from_ast("file:///test.yx", &module);
        assert_eq!(world.symbol_count(), 1);
        let syms = world.symbol_index().find_by_name("x");
        assert_eq!(syms.len(), 1);
        assert_eq!(syms[0].kind, SymbolKind::Variable);
    }

    #[test]
    fn test_update_index_from_ast_fn() {
        let mut world = World::new();
        let module = Module {
            items: vec![Stmt {
                kind: StmtKind::Fn {
                    name: "add".to_string(),
                    generic_params: vec![],
                    type_annotation: None,
                    eval: None,
                    params: vec![],
                    body: (vec![], None),
                    is_pub: false,
                },
                span: dummy_span(),
            }],
            span: dummy_span(),
        };

        world.update_index_from_ast("file:///test.yx", &module);
        assert_eq!(world.symbol_count(), 1);
        let syms = world.symbol_index().find_by_name("add");
        assert_eq!(syms.len(), 1);
        assert_eq!(syms[0].kind, SymbolKind::Function);
    }

    #[test]
    fn test_update_index_from_ast_typedef() {
        let mut world = World::new();
        let module = Module {
            items: vec![Stmt {
                kind: StmtKind::TypeDef {
                    name: "Point".to_string(),
                    name_span: dummy_span(),
                    definition: crate::frontend::core::parser::ast::Type::Name {
                        name: "Int".to_string(),
                        span: dummy_span(),
                    },
                    generic_params: vec![],
                },
                span: dummy_span(),
            }],
            span: dummy_span(),
        };

        world.update_index_from_ast("file:///test.yx", &module);
        assert_eq!(world.symbol_count(), 1);
        let syms = world.symbol_index().find_by_name("Point");
        assert_eq!(syms.len(), 1);
        assert_eq!(syms[0].kind, SymbolKind::Type);
    }

    #[test]
    fn test_update_index_incremental() {
        let mut world = World::new();

        let module_v1 = Module {
            items: vec![Stmt {
                kind: StmtKind::Var {
                    name: "old".to_string(),
                    name_span: Span::dummy(),
                    type_annotation: None,
                    initializer: None,
                    is_mut: false,
                },
                span: dummy_span(),
            }],
            span: dummy_span(),
        };

        world.update_index_from_ast("file:///test.yx", &module_v1);
        assert_eq!(world.symbol_count(), 1);
        assert!(!world.symbol_index().find_by_name("old").is_empty());

        // 更新同一文件 → 旧符号被替换
        let module_v2 = Module {
            items: vec![Stmt {
                kind: StmtKind::Var {
                    name: "new_var".to_string(),
                    name_span: Span::dummy(),
                    type_annotation: None,
                    initializer: None,
                    is_mut: false,
                },
                span: dummy_span(),
            }],
            span: dummy_span(),
        };

        world.update_index_from_ast("file:///test.yx", &module_v2);
        assert_eq!(world.symbol_count(), 1);
        assert!(world.symbol_index().find_by_name("old").is_empty());
        assert!(!world.symbol_index().find_by_name("new_var").is_empty());
    }

    #[test]
    fn test_update_index_fn_params() {
        let mut world = World::new();
        let module = Module {
            items: vec![Stmt {
                kind: StmtKind::Fn {
                    name: "add".to_string(),
                    generic_params: vec![],
                    type_annotation: None,
                    eval: None,
                    params: vec![
                        crate::frontend::core::parser::ast::Param {
                            name: "a".to_string(),
                            ty: None,
                            is_mut: false,
                            span: Span {
                                start: Position {
                                    line: 1,
                                    column: 8,
                                    offset: 7,
                                },
                                end: Position {
                                    line: 1,
                                    column: 9,
                                    offset: 8,
                                },
                            },
                        },
                        crate::frontend::core::parser::ast::Param {
                            name: "b".to_string(),
                            ty: None,
                            is_mut: false,
                            span: Span {
                                start: Position {
                                    line: 1,
                                    column: 11,
                                    offset: 10,
                                },
                                end: Position {
                                    line: 1,
                                    column: 12,
                                    offset: 11,
                                },
                            },
                        },
                    ],
                    body: (vec![], None),
                    is_pub: false,
                },
                span: dummy_span(),
            }],
            span: dummy_span(),
        };

        world.update_index_from_ast("file:///test.yx", &module);
        // 函数 + 2 个参数 = 3 个符号
        assert_eq!(world.symbol_count(), 3);
        assert!(!world.symbol_index().find_by_name("add").is_empty());
        assert!(!world.symbol_index().find_by_name("a").is_empty());
        assert!(!world.symbol_index().find_by_name("b").is_empty());
    }

    #[test]
    fn test_update_index_use_stmt() {
        let mut world = World::new();
        let module = Module {
            items: vec![Stmt {
                kind: StmtKind::Use {
                    path: "std.io".to_string(),
                    path_span: dummy_span(),
                    path_parts: vec![],
                    items: None,
                    alias: None,
                },
                span: dummy_span(),
            }],
            span: dummy_span(),
        };

        world.update_index_from_ast("file:///test.yx", &module);
        // 应导入 std.io 的所有导出
        assert!(
            !world.symbol_index().find_by_name("print").is_empty(),
            "use std.io 应导入 print"
        );
        assert!(
            !world.symbol_index().find_by_name("println").is_empty(),
            "use std.io 应导入 println"
        );
    }

    #[test]
    fn test_update_index_use_stmt_with_items() {
        let mut world = World::new();
        let module = Module {
            items: vec![Stmt {
                kind: StmtKind::Use {
                    path: "std.io".to_string(),
                    path_span: dummy_span(),
                    path_parts: vec![],
                    items: Some(vec!["println".to_string()]),
                    alias: None,
                },
                span: dummy_span(),
            }],
            span: dummy_span(),
        };

        world.update_index_from_ast("file:///test.yx", &module);
        assert!(
            !world.symbol_index().find_by_name("println").is_empty(),
            "use std.io 指定项应导入 println"
        );
        // print 不应该被导入
        assert!(
            world.symbol_index().find_by_name("print").is_empty(),
            "use std.io 指定项不应导入 print"
        );
    }

    #[test]
    fn test_load_std_library_symbols() {
        let mut world = World::new();
        world.load_std_library_symbols(None);

        // 标准库符号应该被加载
        assert!(
            !world.symbol_index().find_by_name("print").is_empty(),
            "标准库应包含 print"
        );
        assert!(
            !world.symbol_index().find_by_name("println").is_empty(),
            "标准库应包含 println"
        );
        assert!(
            !world.symbol_index().find_by_name("push").is_empty(),
            "标准库应包含 push"
        );
        assert!(
            !world.symbol_index().find_by_name("sqrt").is_empty(),
            "标准库应包含 sqrt"
        );
    }

    #[test]
    fn test_parse_interface_file_spans() {
        let temp_dir = std::env::temp_dir().join("yaoxiang_test_spans");
        let _ = std::fs::create_dir_all(&temp_dir);
        let file_path = temp_dir.join("test.yx");

        std::fs::write(
            &file_path,
            "print: (...args) -> () = {\n    ...\n}\n\nprintln: (...args) -> () = {\n    ...\n}\n",
        )
        .unwrap();

        let spans = parse_interface_file_spans(&file_path);
        assert!(spans.contains_key("print"), "应解析到 print");
        assert!(spans.contains_key("println"), "应解析到 println");
        assert_eq!(spans["print"].start.line, 1);
        assert_eq!(spans["println"].start.line, 5);

        let _ = std::fs::remove_dir_all(&temp_dir);
    }
}
