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

/// 将 SemanticTokenType 映射为 DefinitionKind
fn token_type_to_def_kind(token_type: SemanticTokenType) -> DefinitionKind {
    match token_type {
        SemanticTokenType::Function => DefinitionKind::Function,
        SemanticTokenType::Type => DefinitionKind::Type,
        SemanticTokenType::Variable => DefinitionKind::Variable,
        SemanticTokenType::Parameter => DefinitionKind::Parameter,
        SemanticTokenType::Method => DefinitionKind::Method,
        SemanticTokenType::TypeParameter => DefinitionKind::GenericParameter,
        _ => DefinitionKind::Variable,
    }
}

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
    /// 该文件中的定义条目
    pub definitions: Vec<DefinitionInfo>,
    /// 该文件中的引用条目
    pub references: Vec<ReferenceInfo>,
    /// 模块导入信息
    pub imports: Vec<ImportInfo>,
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

// ============ 定义与引用条目（LSP 重构）============
/// 后续任务（5-8）会把所有写入迁移到这些字段，Task 9 时统一清理旧字段。
/// 全局唯一定义标识符
///
/// 由 (文件路径, 定义 span) 共同确定。
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct DefId {
    pub file_path: String,
    pub span: Span,
}

/// 定义种类
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DefinitionKind {
    Variable,
    Function,
    Type,
    Parameter,
    GenericParameter,
    Interface,
    Method,
}

/// 定义条目 — 包含完整类型信息
#[derive(Debug, Clone)]
pub struct DefinitionInfo {
    pub def_id: DefId,
    pub name: String,
    pub kind: DefinitionKind,
    pub span: Span,
    pub file_path: String,
    /// 推断类型，如 `"Int"`, `"(Int, Int) -> Int"`
    pub type_info: Option<String>,
    /// 函数签名，如 `"(a: Int, b: Int) -> Int"`
    pub signature: Option<String>,
}

/// 引用条目 — 每个标识符出现，指向它的定义
#[derive(Debug, Clone)]
pub struct ReferenceInfo {
    pub name: String,
    pub span: Span,
    pub file_path: String,
    /// 指向的定义
    pub resolves_to: DefId,
}

/// 模块导入信息
#[derive(Debug, Clone)]
pub struct ImportInfo {
    /// 例如 `"std.io"`
    pub module_path: String,
    /// 例如 `["print", "println"]`
    pub imported_names: Vec<String>,
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

    /// 获取数据库中所有文件路径（调试用）
    pub fn all_files(&self) -> Vec<&str> {
        self.by_file.keys().map(|s| s.as_str()).collect()
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

                // 同时填充新的 definitions 结构
                let file_info =
                    self.by_file
                        .entry(file_path.clone())
                        .or_insert_with(|| FileSemanticInfo {
                            file_path: file_path.clone(),
                            ..Default::default()
                        });
                // 暂存：实际类型信息将在后续从 TypeChecker 同步
                file_info.definitions.push(DefinitionInfo {
                    def_id: DefId {
                        file_path: file_path.clone(),
                        span: token.span,
                    },
                    name: token.name.clone(),
                    kind: token_type_to_def_kind(token.token_type),
                    span: token.span,
                    file_path: file_path.clone(),
                    type_info: None,
                    signature: None,
                });
            } else {
                self.symbol_refs
                    .entry(token.name.clone())
                    .or_default()
                    .push(location);

                // 同时填充新的 references 结构
                let file_info =
                    self.by_file
                        .entry(file_path.clone())
                        .or_insert_with(|| FileSemanticInfo {
                            file_path: file_path.clone(),
                            ..Default::default()
                        });
                file_info.references.push(ReferenceInfo {
                    name: token.name.clone(),
                    span: token.span,
                    file_path: file_path.clone(),
                    resolves_to: DefId {
                        file_path: String::new(),
                        span: crate::util::span::Span::default(),
                    },
                });
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

    /// 添加一条定义条目，同时维护旧的 `symbol_defs` 索引。
    ///
    /// 在 Task 5-8 完成迁移后，所有调用方应改为直接写入 `definitions`，
    /// `symbol_defs` 将在 Task 9 统一删除。
    pub fn add_definition(
        &mut self,
        file_path: &str,
        def: DefinitionInfo,
    ) {
        // 维护旧索引：以声明位置为符号定义点
        self.symbol_defs
            .entry(def.name.clone())
            .or_default()
            .push(SymbolLocation {
                file_path: file_path.to_string(),
                span: def.span,
            });

        self.by_file
            .entry(file_path.to_string())
            .or_insert_with(|| FileSemanticInfo {
                file_path: file_path.to_string(),
                ..Default::default()
            })
            .definitions
            .push(def);
    }

    /// 添加一条引用条目，同时维护旧的 `symbol_refs` 索引。
    pub fn add_reference(
        &mut self,
        file_path: &str,
        r#ref: ReferenceInfo,
    ) {
        self.symbol_refs
            .entry(r#ref.name.clone())
            .or_default()
            .push(SymbolLocation {
                file_path: file_path.to_string(),
                span: r#ref.span,
            });

        self.by_file
            .entry(file_path.to_string())
            .or_insert_with(|| FileSemanticInfo {
                file_path: file_path.to_string(),
                ..Default::default()
            })
            .references
            .push(r#ref);
    }

    /// 添加一条模块导入信息
    pub fn add_import(
        &mut self,
        file_path: &str,
        import: ImportInfo,
    ) {
        self.by_file
            .entry(file_path.to_string())
            .or_insert_with(|| FileSemanticInfo {
                file_path: file_path.to_string(),
                ..Default::default()
            })
            .imports
            .push(import);
    }

    /// 获取该文件中所有定义条目
    pub fn get_definitions(
        &self,
        file_path: &str,
    ) -> &[DefinitionInfo] {
        self.by_file
            .get(file_path)
            .map(|info| info.definitions.as_slice())
            .unwrap_or(&[])
    }

    /// 获取该文件中所有引用条目
    pub fn get_references(
        &self,
        file_path: &str,
    ) -> &[ReferenceInfo] {
        self.by_file
            .get(file_path)
            .map(|info| info.references.as_slice())
            .unwrap_or(&[])
    }

    /// 获取该文件中的模块导入信息
    pub fn get_imports(
        &self,
        file_path: &str,
    ) -> &[ImportInfo] {
        self.by_file
            .get(file_path)
            .map(|info| info.imports.as_slice())
            .unwrap_or(&[])
    }

    /// 根据位置精确解析引用（用于跳转定义）。
    ///
    /// 1. 在 `by_file[file].references` 中查找 `(line, col)` 处的引用；
    /// 2. 通过 `resolves_to` 在所有文件中找到对应定义。
    pub fn resolve_reference(
        &self,
        file: &str,
        line: usize,
        col: usize,
    ) -> Option<&DefinitionInfo> {
        let file_info = self.by_file.get(file)?;
        let target_def_id = file_info
            .references
            .iter()
            .find(|r| {
                r.file_path == file && r.span.start.line == line && r.span.start.column == col
            })
            .map(|r| r.resolves_to.clone())?;

        for info in self.by_file.values() {
            if let Some(def) = info.definitions.iter().find(|d| d.def_id == target_def_id) {
                return Some(def);
            }
        }
        None
    }

    /// 查找指向某个定义的所有引用（用于 find references / rename）。
    ///
    /// 先通过 `(file, span)` 匹配到 `DefId`，再遍历所有文件收集
    /// `resolves_to` 等于该 `DefId` 的引用。
    pub fn find_all_references_to(
        &self,
        file: &str,
        span: &Span,
    ) -> Vec<&ReferenceInfo> {
        let target = DefId {
            file_path: file.to_string(),
            span: *span,
        };

        let mut out = Vec::new();
        for info in self.by_file.values() {
            for r in &info.references {
                if r.resolves_to == target {
                    out.push(r);
                }
            }
        }
        out
    }

    /// 获取文件（位置）处可见的所有符号（用于补全）。
    ///
    /// 完整实现需要沿作用域链向外查找并合并 `imports`。
    /// 当前为占位实现，返回空列表，后续任务会补充。
    pub fn visible_definitions(
        &self,
        _file: &str,
        _line: usize,
        _col: usize,
    ) -> Vec<&DefinitionInfo> {
        Vec::new()
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
