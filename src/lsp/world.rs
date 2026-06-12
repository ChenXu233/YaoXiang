//! 编译世界（World）
//!
//! 管理编译器状态和语义数据库，为 LSP 提供语义分析支持。

use crate::frontend::core::typecheck::semantic_db::{DefId, DefinitionInfo, DefinitionKind, SemanticDB};
use crate::lsp::handlers::semantic_tokens::SemanticTokensCache;
use crate::util::span::Span;

/// 编译世界
///
/// 聚合编译器运行所需的全局状态：
/// - 语义数据库（用于 go-to-definition、find-references、hover 等）
/// - 语义 tokens 缓存（用于 delta 模式增量更新）
#[derive(Debug, Default)]
pub struct World {
    /// 语义数据库
    semantic_db: SemanticDB,
    /// 语义 tokens 版本缓存（支持 delta 更新）
    semantic_tokens_cache: SemanticTokensCache,
}

impl World {
    /// 创建新的编译世界
    pub fn new() -> Self {
        Self {
            semantic_db: SemanticDB::default(),
            semantic_tokens_cache: SemanticTokensCache::new(),
        }
    }

    /// 为新的 LSP 会话重置运行时缓存
    ///
    /// 清空语义数据库和语义 tokens 缓存，
    /// 然后重新加载标准库符号和内置类型。
    pub fn reset_for_new_session(&mut self) {
        self.semantic_db = SemanticDB::default();
        self.semantic_tokens_cache = SemanticTokensCache::new();

        self.load_std_symbols_to_semantic_db();
        self.load_builtin_types_to_semantic_db();
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
        self.semantic_db.remove_file(file_path);
    }

    /// 加载标准库符号到语义数据库
    ///
    /// 从 ModuleRegistry 获取所有标准库模块的导出，
    /// 将其作为虚拟符号添加到语义数据库中。
    pub fn load_std_symbols_to_semantic_db(&mut self) {
        use crate::util::span::Position;

        let dummy_span = Span {
            start: Position {
                line: 1,
                column: 1,
                offset: 0,
            },
            end: Position {
                line: 1,
                column: 2,
                offset: 1,
            },
        };

        for module_info in crate::std::all_module_infos() {
            let module_name = module_info
                .path
                .strip_prefix("std.")
                .unwrap_or(&module_info.path);

            // 尝试查找接口文件
            let interface_file =
                crate::std::gen_interfaces::find_std_interface_file(None, module_name);

            let virtual_file_path = if let Some(ref path) = interface_file {
                format!("file:///{}", path.to_string_lossy().replace('\\', "/"))
            } else {
                format!("std://{}", module_info.path)
            };

            for export in module_info.exports.values() {
                if export.kind == crate::frontend::module::ExportKind::SubModule {
                    continue;
                }

                let kind = match export.kind {
                    crate::frontend::module::ExportKind::Function => DefinitionKind::Function,
                    crate::frontend::module::ExportKind::Constant => DefinitionKind::Variable,
                    crate::frontend::module::ExportKind::Type => DefinitionKind::Type,
                    crate::frontend::module::ExportKind::SubModule => continue,
                };

                let def = DefinitionInfo {
                    def_id: DefId {
                        file_path: virtual_file_path.clone(),
                        span: dummy_span,
                    },
                    name: export.name.clone(),
                    kind,
                    span: dummy_span,
                    file_path: virtual_file_path.clone(),
                    type_info: None,
                    signature: None,
                };
                self.semantic_db.add_definition(&virtual_file_path, def);
            }
        }
    }

    /// 加载内置类型到语义数据库
    ///
    /// 将 Int, Float, Bool, String, Void, Char 等内置类型添加到语义数据库，
    /// 使 LSP 功能（如 hover、definition）能够识别这些内置类型。
    pub fn load_builtin_types_to_semantic_db(&mut self) {
        use crate::util::span::Position;

        let dummy_span = Span {
            start: Position {
                line: 1,
                column: 1,
                offset: 0,
            },
            end: Position {
                line: 1,
                column: 2,
                offset: 1,
            },
        };

        // YaoXiang 语言的核心内置类型
        let builtin_types = [
            "Int", "Int8", "Int16", "Int32", "Int64", "Int128", "Uint", "Float", "Float32",
            "Float64", "Bool", "String", "Char", "Bytes", "Void",
        ];

        let file_path = "builtin://types".to_string();
        for type_name in builtin_types {
            let def = DefinitionInfo {
                def_id: DefId {
                    file_path: file_path.clone(),
                    span: dummy_span,
                },
                name: type_name.to_string(),
                kind: DefinitionKind::Type,
                span: dummy_span,
                file_path: file_path.clone(),
                type_info: Some("Type".to_string()),
                signature: None,
            };
            self.semantic_db.add_definition(&file_path, def);
        }
    }
}
