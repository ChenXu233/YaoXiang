//! 编译世界（World）
//!
//! 管理编译器状态和符号索引，为 LSP 提供语义分析支持。

use crate::frontend::core::lexer::symbols::{IndexedSymbol, SymbolIndex, SymbolKind, SymbolLocation};
use crate::frontend::core::parser::ast::{Module, StmtKind};

/// 编译世界
///
/// 聚合编译器运行所需的全局状态：
/// - 符号索引（用于 go-to-definition、find-references 等）
/// - 编译器实例管理
///
/// 阶段 3 扩展：
/// - 从解析结果构建符号索引
/// - 支持增量更新（文件级别）
#[derive(Debug, Default)]
pub struct World {
    /// 全局符号索引
    symbol_index: SymbolIndex,
}

impl World {
    /// 创建新的编译世界
    pub fn new() -> Self {
        Self {
            symbol_index: SymbolIndex::new(),
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

    /// 移除某个文件的所有符号（文件关闭或重新解析时调用）
    pub fn remove_file_symbols(
        &mut self,
        file_path: &str,
    ) {
        self.symbol_index.remove_file(file_path);
    }

    /// 获取索引中的符号总数
    pub fn symbol_count(&self) -> usize {
        self.symbol_index.symbol_count()
    }

    /// 从解析后的 AST 更新指定文件的符号索引
    ///
    /// 先移除该文件的旧符号，再从 AST 提取新符号。
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
                    ..
                } => {
                    let full_name = format!("{}.{}", type_name, method_name);
                    self.symbol_index.add(IndexedSymbol {
                        name: full_name,
                        kind: SymbolKind::Function,
                        arity: Some(params.len()),
                        location: SymbolLocation::new(file_path.to_string(), stmt.span),
                    });
                }
                _ => {}
            }
        }
    }
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
                    definition: crate::frontend::core::parser::ast::Type::Name("Int".to_string()),
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
}
