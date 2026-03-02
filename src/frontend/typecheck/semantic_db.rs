//! 语义信息数据库（SemanticDB）
//!
//! 统一管理语义信息，服务于：
//! - LSP 语义高亮
//! - 增量编译
//! - 死代码分析
//!
//! 设计原则：一次遍历（typecheck），多处使用

use std::collections::HashMap;
use crate::util::span::Span;

// ============ 核心数据结构 ============

/// 语义信息数据库
///
/// 由 typecheck 阶段产出，LSP 和增量编译直接查询复用。
#[derive(Debug, Clone, Default)]
pub struct SemanticDB {
    /// 文件路径 → 该文件中的语义信息
    by_file: HashMap<String, FileSemanticInfo>,
    /// 符号名 → 所有定义位置
    symbol_defs: HashMap<String, Vec<SymbolLocation>>,
    /// 符号名 → 所有引用位置
    symbol_refs: HashMap<String, Vec<SymbolLocation>>,
}

/// 单文件的语义信息
#[derive(Debug, Clone, Default)]
pub struct FileSemanticInfo {
    /// 文件路径
    pub file_path: String,
    /// 语义 tokens
    pub tokens: Vec<SemanticToken>,
    /// 作用域信息
    pub scopes: Vec<ScopeInfo>,
}

/// 语义 Token
///
/// 记录源码中每个有意义的标识符的类型和修饰符。
#[derive(Debug, Clone)]
pub struct SemanticToken {
    /// 标识符名称
    pub name: String,
    /// Token 类型
    pub token_type: SemanticTokenType,
    /// 修饰符列表
    pub modifiers: Vec<SemanticTokenModifier>,
    /// 源码位置
    pub span: Span,
}

/// 符号位置
#[derive(Debug, Clone)]
pub struct SymbolLocation {
    /// 文件路径
    pub file_path: String,
    /// 源码位置
    pub span: Span,
}

// ============ Token 类型枚举 ============

/// 语义 Token 类型（对齐 LSP SemanticTokenTypes）
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SemanticTokenType {
    /// 函数
    Function,
    /// 类型
    Type,
    /// 变量
    Variable,
    /// 属性/字段
    Property,
    /// 方法
    Method,
    /// 命名空间/模块
    Namespace,
    /// 函数参数
    Parameter,
    /// 局部变量
    LocalVariable,
    /// 类型参数
    TypeParameter,
    /// 关键字
    Keyword,
    /// 字符串字面量
    String,
    /// 数字字面量
    Number,
    EnumMember,
}

impl SemanticTokenType {
    /// 返回 LSP 标准 token 类型索引
    ///
    /// 索引顺序必须与 `server_capabilities` 中声明的 `token_types` 一致。
    pub fn index(&self) -> u32 {
        match self {
            SemanticTokenType::Function => 0,
            SemanticTokenType::Type => 1,
            SemanticTokenType::Variable => 2,
            SemanticTokenType::Property => 3,
            SemanticTokenType::Method => 4,
            SemanticTokenType::Namespace => 5,
            SemanticTokenType::Parameter => 6,
            SemanticTokenType::LocalVariable => 2, // 映射到 Variable
            SemanticTokenType::TypeParameter => 7,
            SemanticTokenType::Keyword => 8,
            SemanticTokenType::String => 9,
            SemanticTokenType::Number => 10,
            SemanticTokenType::EnumMember => 11,
        }
    }

    /// LSP token 类型名称列表（声明顺序）
    pub fn legend() -> Vec<&'static str> {
        vec![
            "function",      // 0
            "type",          // 1
            "variable",      // 2
            "property",      // 3
            "method",        // 4
            "namespace",     // 5
            "parameter",     // 6
            "typeParameter", // 7
            "keyword",       // 8
            "string",        // 9
            "number",        // 10
            "enumMember",    // 11
        ]
    }
}

/// 语义 Token 修饰符（对齐 LSP SemanticTokenModifiers）
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SemanticTokenModifier {
    /// 声明/定义
    Declaration,
    /// 不可变
    Readonly,
    /// 可变
    Mutable,
    /// 公开导出
    Public,
    /// 泛型
    Generic,
}

impl SemanticTokenModifier {
    /// 返回 LSP 标准修饰符的 bit flag
    pub fn bit_flag(&self) -> u32 {
        match self {
            SemanticTokenModifier::Declaration => 1 << 0,
            SemanticTokenModifier::Readonly => 1 << 1,
            SemanticTokenModifier::Mutable => 1 << 2,
            SemanticTokenModifier::Public => 1 << 3,
            SemanticTokenModifier::Generic => 1 << 4,
        }
    }

    /// LSP 修饰符名称列表（声明顺序）
    pub fn legend() -> Vec<&'static str> {
        vec![
            "declaration",  // bit 0
            "readonly",     // bit 1
            "modification", // bit 2 (mutable → modification in LSP)
            "public",       // bit 3 (custom)
            "generic",      // bit 4 (custom)
        ]
    }
}

// ============ 作用域信息 ============

/// 作用域类型
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ScopeKind {
    /// 全局作用域
    Global,
    /// 函数作用域
    Function,
    /// 块级作用域
    Block,
    /// Lambda 作用域
    Lambda,
}

/// 作用域信息
#[derive(Debug, Clone)]
pub struct ScopeInfo {
    /// 作用域的源码范围
    pub span: Span,
    /// 父作用域索引（None 表示全局作用域）
    pub parent: Option<usize>,
    /// 作用域内定义的符号名
    pub symbols: Vec<String>,
    /// 作用域类型
    pub kind: ScopeKind,
}

// ============ SemanticDB 实现 ============

impl SemanticDB {
    /// 创建空的语义信息数据库
    pub fn new() -> Self {
        Self::default()
    }

    /// 获取指定文件的语义信息
    pub fn get_file_info(
        &self,
        file_path: &str,
    ) -> Option<&FileSemanticInfo> {
        self.by_file.get(file_path)
    }

    /// 获取指定文件的语义 tokens
    pub fn get_tokens(
        &self,
        file_path: &str,
    ) -> Option<&[SemanticToken]> {
        self.by_file
            .get(file_path)
            .map(|info| info.tokens.as_slice())
    }

    /// 获取指定文件的作用域信息
    pub fn get_scopes(
        &self,
        file_path: &str,
    ) -> Option<&[ScopeInfo]> {
        self.by_file
            .get(file_path)
            .map(|info| info.scopes.as_slice())
    }

    /// 获取符号的所有定义位置
    pub fn get_symbol_defs(
        &self,
        name: &str,
    ) -> Option<&[SymbolLocation]> {
        self.symbol_defs.get(name).map(|v| v.as_slice())
    }

    /// 获取符号的所有引用位置
    pub fn get_symbol_refs(
        &self,
        name: &str,
    ) -> Option<&[SymbolLocation]> {
        self.symbol_refs.get(name).map(|v| v.as_slice())
    }

    /// 获取所有已记录的文件路径
    pub fn file_paths(&self) -> Vec<&String> {
        self.by_file.keys().collect()
    }

    /// 获取所有已定义的符号名
    pub fn defined_symbols(&self) -> Vec<&String> {
        self.symbol_defs.keys().collect()
    }

    /// 获取所有符号引用（用于死代码分析）
    pub fn all_symbol_refs(&self) -> &HashMap<String, Vec<SymbolLocation>> {
        &self.symbol_refs
    }

    /// 获取所有符号定义（用于死代码分析）
    pub fn all_symbol_defs(&self) -> &HashMap<String, Vec<SymbolLocation>> {
        &self.symbol_defs
    }

    /// 获取数据库中的 token 总数
    pub fn token_count(&self) -> usize {
        self.by_file.values().map(|info| info.tokens.len()).sum()
    }

    /// 是否为空
    pub fn is_empty(&self) -> bool {
        self.by_file.is_empty()
    }

    /// 设置文件的语义信息（覆盖已有数据）
    pub fn set_file_info(
        &mut self,
        file_path: String,
        info: FileSemanticInfo,
    ) {
        // 先清理该文件的旧符号索引
        self.remove_file_symbols(&file_path);

        // 索引新的符号定义和引用
        for token in &info.tokens {
            let location = SymbolLocation {
                file_path: file_path.clone(),
                span: token.span,
            };

            if token
                .modifiers
                .contains(&SemanticTokenModifier::Declaration)
            {
                self.symbol_defs
                    .entry(token.name.clone())
                    .or_default()
                    .push(location);
            } else {
                self.symbol_refs
                    .entry(token.name.clone())
                    .or_default()
                    .push(location);
            }
        }

        self.by_file.insert(file_path, info);
    }

    /// 移除指定文件的所有语义信息
    pub fn remove_file(
        &mut self,
        file_path: &str,
    ) {
        self.remove_file_symbols(file_path);
        self.by_file.remove(file_path);
    }

    /// 添加单个语义 token 到指定文件
    pub fn add_token(
        &mut self,
        file_path: &str,
        token: SemanticToken,
    ) {
        let location = SymbolLocation {
            file_path: file_path.to_string(),
            span: token.span,
        };

        if token
            .modifiers
            .contains(&SemanticTokenModifier::Declaration)
        {
            self.symbol_defs
                .entry(token.name.clone())
                .or_default()
                .push(location);
        } else {
            self.symbol_refs
                .entry(token.name.clone())
                .or_default()
                .push(location);
        }

        self.by_file
            .entry(file_path.to_string())
            .or_insert_with(|| FileSemanticInfo {
                file_path: file_path.to_string(),
                ..Default::default()
            })
            .tokens
            .push(token);
    }

    /// 添加作用域信息到指定文件
    pub fn add_scope(
        &mut self,
        file_path: &str,
        scope: ScopeInfo,
    ) {
        self.by_file
            .entry(file_path.to_string())
            .or_insert_with(|| FileSemanticInfo {
                file_path: file_path.to_string(),
                ..Default::default()
            })
            .scopes
            .push(scope);
    }

    /// 查找包含给定位置的最内层作用域
    pub fn find_innermost_scope(
        &self,
        file_path: &str,
        line: usize,
        column: usize,
    ) -> Option<&ScopeInfo> {
        let scopes = self.get_scopes(file_path)?;
        let mut best: Option<&ScopeInfo> = None;
        for scope in scopes {
            let in_scope = (scope.span.start.line < line
                || (scope.span.start.line == line && scope.span.start.column <= column))
                && (scope.span.end.line > line
                    || (scope.span.end.line == line && scope.span.end.column >= column));
            if in_scope {
                // 选择更小的（更内层的）作用域
                if let Some(current_best) = best {
                    if scope.span.len() < current_best.span.len() {
                        best = Some(scope);
                    }
                } else {
                    best = Some(scope);
                }
            }
        }
        best
    }

    // ---- 内部方法 ----

    /// 从符号索引中移除指定文件的条目
    fn remove_file_symbols(
        &mut self,
        file_path: &str,
    ) {
        for locs in self.symbol_defs.values_mut() {
            locs.retain(|loc| loc.file_path != file_path);
        }
        self.symbol_defs.retain(|_, locs| !locs.is_empty());

        for locs in self.symbol_refs.values_mut() {
            locs.retain(|loc| loc.file_path != file_path);
        }
        self.symbol_refs.retain(|_, locs| !locs.is_empty());
    }
}

// ============ 测试 ============

#[cfg(test)]
mod tests {
    use super::*;
    use crate::util::span::Position;

    fn make_span(
        line: usize,
        col_start: usize,
        col_end: usize,
    ) -> Span {
        Span {
            start: Position::with_offset(line, col_start, 0),
            end: Position::with_offset(line, col_end, 0),
        }
    }

    fn make_token(
        name: &str,
        ty: SemanticTokenType,
        is_decl: bool,
        span: Span,
    ) -> SemanticToken {
        let mut modifiers = Vec::new();
        if is_decl {
            modifiers.push(SemanticTokenModifier::Declaration);
        }
        SemanticToken {
            name: name.to_string(),
            token_type: ty,
            modifiers,
            span,
        }
    }

    // ---- 构造与空数据库测试 ----

    #[test]
    fn test_new_semantic_db_is_empty() {
        let db = SemanticDB::new();
        assert!(db.is_empty());
        assert_eq!(db.token_count(), 0);
        assert!(db.file_paths().is_empty());
        assert!(db.defined_symbols().is_empty());
    }

    // ---- 按文件查询测试 ----

    #[test]
    fn test_get_file_info() {
        let mut db = SemanticDB::new();

        let info = FileSemanticInfo {
            file_path: "test.yx".to_string(),
            tokens: vec![make_token(
                "foo",
                SemanticTokenType::Function,
                true,
                make_span(1, 1, 4),
            )],
            scopes: vec![],
        };

        db.set_file_info("test.yx".to_string(), info);

        assert!(!db.is_empty());
        assert_eq!(db.token_count(), 1);

        let file_info = db.get_file_info("test.yx").unwrap();
        assert_eq!(file_info.tokens.len(), 1);
        assert_eq!(file_info.tokens[0].name, "foo");

        // 不存在的文件返回 None
        assert!(db.get_file_info("nonexistent.yx").is_none());
    }

    #[test]
    fn test_get_tokens_and_scopes() {
        let mut db = SemanticDB::new();

        let info = FileSemanticInfo {
            file_path: "test.yx".to_string(),
            tokens: vec![
                make_token("x", SemanticTokenType::Variable, true, make_span(1, 1, 2)),
                make_token("y", SemanticTokenType::Variable, true, make_span(2, 1, 2)),
            ],
            scopes: vec![ScopeInfo {
                span: make_span(1, 1, 20),
                parent: None,
                symbols: vec!["x".to_string(), "y".to_string()],
                kind: ScopeKind::Global,
            }],
        };

        db.set_file_info("test.yx".to_string(), info);

        let tokens = db.get_tokens("test.yx").unwrap();
        assert_eq!(tokens.len(), 2);

        let scopes = db.get_scopes("test.yx").unwrap();
        assert_eq!(scopes.len(), 1);
        assert_eq!(scopes[0].kind, ScopeKind::Global);
    }

    // ---- 按符号名查询测试 ----

    #[test]
    fn test_symbol_defs_and_refs() {
        let mut db = SemanticDB::new();

        let info = FileSemanticInfo {
            file_path: "main.yx".to_string(),
            tokens: vec![
                // 函数定义
                make_token("add", SemanticTokenType::Function, true, make_span(1, 1, 4)),
                // 函数引用
                make_token(
                    "add",
                    SemanticTokenType::Function,
                    false,
                    make_span(5, 1, 4),
                ),
            ],
            scopes: vec![],
        };

        db.set_file_info("main.yx".to_string(), info);

        // 定义
        let defs = db.get_symbol_defs("add").unwrap();
        assert_eq!(defs.len(), 1);
        assert_eq!(defs[0].file_path, "main.yx");

        // 引用
        let refs = db.get_symbol_refs("add").unwrap();
        assert_eq!(refs.len(), 1);
        assert_eq!(refs[0].span.start.line, 5);

        // 不存在的符号
        assert!(db.get_symbol_defs("nonexistent").is_none());
        assert!(db.get_symbol_refs("nonexistent").is_none());
    }

    // ---- 多文件管理测试 ----

    #[test]
    fn test_multi_file() {
        let mut db = SemanticDB::new();

        db.set_file_info(
            "a.yx".to_string(),
            FileSemanticInfo {
                file_path: "a.yx".to_string(),
                tokens: vec![make_token(
                    "foo",
                    SemanticTokenType::Function,
                    true,
                    make_span(1, 1, 4),
                )],
                scopes: vec![],
            },
        );
        db.set_file_info(
            "b.yx".to_string(),
            FileSemanticInfo {
                file_path: "b.yx".to_string(),
                tokens: vec![make_token(
                    "bar",
                    SemanticTokenType::Function,
                    true,
                    make_span(1, 1, 4),
                )],
                scopes: vec![],
            },
        );

        assert_eq!(db.file_paths().len(), 2);
        assert_eq!(db.token_count(), 2);

        assert!(db.get_file_info("a.yx").is_some());
        assert!(db.get_file_info("b.yx").is_some());

        // 跨文件符号引用
        assert!(db.get_symbol_defs("foo").is_some());
        assert!(db.get_symbol_defs("bar").is_some());
    }

    // ---- 文件覆盖更新测试 ----

    #[test]
    fn test_file_override_update() {
        let mut db = SemanticDB::new();

        // 第一版
        db.set_file_info(
            "test.yx".to_string(),
            FileSemanticInfo {
                file_path: "test.yx".to_string(),
                tokens: vec![make_token(
                    "old_fn",
                    SemanticTokenType::Function,
                    true,
                    make_span(1, 1, 7),
                )],
                scopes: vec![],
            },
        );
        assert!(db.get_symbol_defs("old_fn").is_some());

        // 覆盖更新
        db.set_file_info(
            "test.yx".to_string(),
            FileSemanticInfo {
                file_path: "test.yx".to_string(),
                tokens: vec![make_token(
                    "new_fn",
                    SemanticTokenType::Function,
                    true,
                    make_span(1, 1, 7),
                )],
                scopes: vec![],
            },
        );

        // 旧符号已移除
        assert!(db.get_symbol_defs("old_fn").is_none());
        // 新符号已存在
        assert!(db.get_symbol_defs("new_fn").is_some());
        assert_eq!(db.token_count(), 1);
    }

    // ---- 文件移除测试 ----

    #[test]
    fn test_remove_file() {
        let mut db = SemanticDB::new();

        db.set_file_info(
            "test.yx".to_string(),
            FileSemanticInfo {
                file_path: "test.yx".to_string(),
                tokens: vec![make_token(
                    "x",
                    SemanticTokenType::Variable,
                    true,
                    make_span(1, 1, 2),
                )],
                scopes: vec![],
            },
        );

        assert!(!db.is_empty());
        db.remove_file("test.yx");
        assert!(db.is_empty());
        assert!(db.get_symbol_defs("x").is_none());
    }

    // ---- add_token 增量添加测试 ----

    #[test]
    fn test_add_token_incrementally() {
        let mut db = SemanticDB::new();

        db.add_token(
            "test.yx",
            make_token("a", SemanticTokenType::Variable, true, make_span(1, 1, 2)),
        );
        db.add_token(
            "test.yx",
            make_token("b", SemanticTokenType::Variable, true, make_span(2, 1, 2)),
        );

        assert_eq!(db.token_count(), 2);
        assert!(db.get_symbol_defs("a").is_some());
        assert!(db.get_symbol_defs("b").is_some());
    }

    // ---- 作用域查找测试 ----

    #[test]
    fn test_scope_basic() {
        let mut db = SemanticDB::new();

        db.add_scope(
            "test.yx",
            ScopeInfo {
                span: Span::new(
                    Position::with_offset(1, 1, 0),
                    Position::with_offset(10, 1, 100),
                ),
                parent: None,
                symbols: vec!["x".to_string()],
                kind: ScopeKind::Global,
            },
        );

        let scopes = db.get_scopes("test.yx").unwrap();
        assert_eq!(scopes.len(), 1);
        assert_eq!(scopes[0].kind, ScopeKind::Global);
    }

    #[test]
    fn test_find_innermost_scope() {
        let mut db = SemanticDB::new();

        // 全局作用域 1:1 - 20:1
        db.add_scope(
            "test.yx",
            ScopeInfo {
                span: Span::new(
                    Position::with_offset(1, 1, 0),
                    Position::with_offset(20, 1, 200),
                ),
                parent: None,
                symbols: vec!["x".to_string()],
                kind: ScopeKind::Global,
            },
        );

        // 函数作用域 5:1 - 10:1
        db.add_scope(
            "test.yx",
            ScopeInfo {
                span: Span::new(
                    Position::with_offset(5, 1, 50),
                    Position::with_offset(10, 1, 100),
                ),
                parent: Some(0),
                symbols: vec!["y".to_string()],
                kind: ScopeKind::Function,
            },
        );

        // 第 7 行应该在函数作用域内
        let scope = db.find_innermost_scope("test.yx", 7, 1).unwrap();
        assert_eq!(scope.kind, ScopeKind::Function);

        // 第 15 行在全局作用域
        let scope = db.find_innermost_scope("test.yx", 15, 1).unwrap();
        assert_eq!(scope.kind, ScopeKind::Global);

        // 不存在的文件
        assert!(db.find_innermost_scope("nonexistent.yx", 1, 1).is_none());
    }

    // ---- Token 类型索引测试 ----

    #[test]
    fn test_token_type_index() {
        assert_eq!(SemanticTokenType::Function.index(), 0);
        assert_eq!(SemanticTokenType::Type.index(), 1);
        assert_eq!(SemanticTokenType::Variable.index(), 2);
        assert_eq!(SemanticTokenType::LocalVariable.index(), 2); // 映射到 Variable
        assert_eq!(SemanticTokenType::Property.index(), 3);
        assert_eq!(SemanticTokenType::Method.index(), 4);
        assert_eq!(SemanticTokenType::Namespace.index(), 5);
        assert_eq!(SemanticTokenType::Parameter.index(), 6);
        assert_eq!(SemanticTokenType::TypeParameter.index(), 7);
        assert_eq!(SemanticTokenType::EnumMember.index(), 11);
    }

    #[test]
    fn test_token_type_legend() {
        let legend = SemanticTokenType::legend();
        assert_eq!(legend.len(), 12);
        assert_eq!(legend[0], "function");
        assert_eq!(legend[1], "type");
        assert_eq!(legend[11], "enumMember");
    }

    #[test]
    fn test_modifier_bit_flags() {
        assert_eq!(SemanticTokenModifier::Declaration.bit_flag(), 1);
        assert_eq!(SemanticTokenModifier::Readonly.bit_flag(), 2);
        assert_eq!(SemanticTokenModifier::Mutable.bit_flag(), 4);
        assert_eq!(SemanticTokenModifier::Public.bit_flag(), 8);
        assert_eq!(SemanticTokenModifier::Generic.bit_flag(), 16);
    }

    #[test]
    fn test_modifier_legend() {
        let legend = SemanticTokenModifier::legend();
        assert_eq!(legend.len(), 5);
    }
}
