//! Symbol table management
//! Unified symbol management for RFC-004, RFC-010, and RFC-011 support

use crate::util::span::Span;
use std::collections::HashMap;

/// 符号位置信息
///
/// 记录符号在源码中的精确位置，用于 LSP 跳转定义等功能。
#[derive(Debug, Clone)]
pub struct SymbolLocation {
    /// 所在文件路径
    pub file_path: String,
    /// 符号所在的 Span
    pub span: Span,
}

impl SymbolLocation {
    /// 创建新的符号位置
    pub fn new(
        file_path: String,
        span: Span,
    ) -> Self {
        Self { file_path, span }
    }
}

/// Symbol table entry
#[derive(Debug, Clone)]
pub struct SymbolEntry {
    pub name: String,
    pub kind: SymbolKind,
    pub arity: Option<usize>,
    /// 符号定义的位置信息（用于 LSP 跳转定义）
    pub location: Option<SymbolLocation>,
}

/// Kind of symbol
#[derive(Debug, Clone, PartialEq)]
pub enum SymbolKind {
    // Basic symbols
    Variable,
    Function,
    Type,

    // RFC-010: Generic symbols
    GenericFunction,
    GenericType,
    TypeClass,
    Trait,

    // RFC-011: Advanced type system symbols
    ConstGeneric,
    HigherKindedType,
    TypeFamily,

    // RFC-004: Binding symbols
    Binding,
    PositionBinding,
}

/// Symbol table for managing identifiers
pub struct SymbolTable {
    symbols: Vec<SymbolEntry>,
}

impl SymbolTable {
    /// Create new empty symbol table
    pub fn new() -> Self {
        Self {
            symbols: Vec::new(),
        }
    }

    /// Insert a symbol
    pub fn insert(
        &mut self,
        name: String,
        kind: SymbolKind,
    ) {
        self.symbols.push(SymbolEntry {
            name,
            kind,
            arity: None,
            location: None,
        });
    }

    /// Insert a symbol with arity
    pub fn insert_with_arity(
        &mut self,
        name: String,
        kind: SymbolKind,
        arity: usize,
    ) {
        self.symbols.push(SymbolEntry {
            name,
            kind,
            arity: Some(arity),
            location: None,
        });
    }

    /// 插入带位置信息的符号
    pub fn insert_with_location(
        &mut self,
        name: String,
        kind: SymbolKind,
        location: SymbolLocation,
    ) {
        self.symbols.push(SymbolEntry {
            name,
            kind,
            arity: None,
            location: Some(location),
        });
    }

    /// 插入带位置和元数信息的符号
    pub fn insert_full(
        &mut self,
        name: String,
        kind: SymbolKind,
        arity: Option<usize>,
        location: Option<SymbolLocation>,
    ) {
        self.symbols.push(SymbolEntry {
            name,
            kind,
            arity,
            location,
        });
    }

    /// Lookup a symbol by name
    pub fn lookup(
        &self,
        name: &str,
    ) -> Option<&SymbolEntry> {
        self.symbols.iter().rev().find(|s| s.name == name)
    }

    /// Check if symbol exists
    pub fn contains(
        &self,
        name: &str,
    ) -> bool {
        self.lookup(name).is_some()
    }

    /// Get all symbols of a specific kind
    pub fn get_by_kind(
        &self,
        kind: &SymbolKind,
    ) -> Vec<&SymbolEntry> {
        self.symbols.iter().filter(|s| &s.kind == kind).collect()
    }

    /// Clear the symbol table
    pub fn clear(&mut self) {
        self.symbols.clear();
    }
}

impl Default for SymbolTable {
    fn default() -> Self {
        Self::new()
    }
}

/// 符号索引（反向索引）
///
/// 提供名称→位置列表和文件→符号列表的快速查询。
/// 主要用于 LSP 的跳转定义、查找引用和工作区符号搜索。
#[derive(Debug, Default)]
pub struct SymbolIndex {
    /// 名称 → 符号信息列表（支持重名/重载）
    by_name: HashMap<String, Vec<IndexedSymbol>>,
    /// 文件路径 → 该文件中的所有符号
    by_file: HashMap<String, Vec<IndexedSymbol>>,
}

/// 索引中的符号信息
#[derive(Debug, Clone)]
pub struct IndexedSymbol {
    /// 符号名称
    pub name: String,
    /// 符号种类
    pub kind: SymbolKind,
    /// 元数（函数参数数量）
    pub arity: Option<usize>,
    /// 位置信息
    pub location: SymbolLocation,
}

impl SymbolIndex {
    /// 创建空索引
    pub fn new() -> Self {
        Self::default()
    }

    /// 添加符号到索引
    pub fn add(
        &mut self,
        symbol: IndexedSymbol,
    ) {
        self.by_file
            .entry(symbol.location.file_path.clone())
            .or_default()
            .push(symbol.clone());
        self.by_name
            .entry(symbol.name.clone())
            .or_default()
            .push(symbol);
    }

    /// 从 SymbolTable 构建索引
    pub fn from_table(table: &SymbolTable) -> Self {
        let mut index = Self::new();
        for entry in &table.symbols {
            if let Some(ref loc) = entry.location {
                index.add(IndexedSymbol {
                    name: entry.name.clone(),
                    kind: entry.kind.clone(),
                    arity: entry.arity,
                    location: loc.clone(),
                });
            }
        }
        index
    }

    /// 根据名称查找所有定义位置
    pub fn find_by_name(
        &self,
        name: &str,
    ) -> &[IndexedSymbol] {
        self.by_name.get(name).map(|v| v.as_slice()).unwrap_or(&[])
    }

    /// 根据文件查找该文件中的所有符号
    pub fn find_by_file(
        &self,
        file_path: &str,
    ) -> &[IndexedSymbol] {
        self.by_file
            .get(file_path)
            .map(|v| v.as_slice())
            .unwrap_or(&[])
    }

    /// 检查名称是否存在于索引中
    pub fn contains(
        &self,
        name: &str,
    ) -> bool {
        self.by_name.contains_key(name)
    }

    /// 获取索引中的所有符号名称
    pub fn all_names(&self) -> Vec<&str> {
        self.by_name.keys().map(|s| s.as_str()).collect()
    }

    /// 获取索引中的所有文件路径
    pub fn all_files(&self) -> Vec<&str> {
        self.by_file.keys().map(|s| s.as_str()).collect()
    }

    /// 清空某个文件的符号（用于增量更新）
    pub fn remove_file(
        &mut self,
        file_path: &str,
    ) {
        if let Some(symbols) = self.by_file.remove(file_path) {
            for sym in &symbols {
                if let Some(entries) = self.by_name.get_mut(&sym.name) {
                    entries.retain(|s| s.location.file_path != file_path);
                    if entries.is_empty() {
                        self.by_name.remove(&sym.name);
                    }
                }
            }
        }
    }

    /// 获取索引中的符号总数
    pub fn symbol_count(&self) -> usize {
        self.by_file.values().map(|v| v.len()).sum()
    }
}

/// RFC-004: Binding position validator
pub struct BindingValidator {
    max_positions: usize,
}

impl BindingValidator {
    /// Create new validator
    pub fn new(max_positions: usize) -> Self {
        Self { max_positions }
    }

    /// Validate binding positions
    /// Returns Ok if positions are valid, Err with error message otherwise
    pub fn validate_positions(
        &self,
        positions: &[i32],
    ) -> Result<(), String> {
        for &pos in positions {
            if pos < 0 {
                return Err(format!("Negative position index: {}", pos));
            }
            if pos as usize >= self.max_positions {
                return Err(format!(
                    "Position index {} exceeds maximum allowed positions {}",
                    pos, self.max_positions
                ));
            }
        }
        Ok(())
    }

    /// Validate binding syntax
    /// Supports RFC-004 binding syntax: function[0, 1, 2]
    pub fn validate_binding_syntax(
        &self,
        binding: &str,
    ) -> Result<(), String> {
        // Check for valid binding syntax pattern
        if !binding.contains('[') || !binding.contains(']') {
            return Err("Invalid binding syntax: missing brackets".to_string());
        }

        // Extract position list
        let positions_str = binding
            .split('[')
            .nth(1)
            .ok_or("Invalid binding syntax")?
            .trim_end_matches(']');

        // Parse positions
        let positions: Result<Vec<i32>, _> =
            positions_str.split(',').map(|s| s.trim().parse()).collect();

        let positions = positions.map_err(|_| "Invalid position value")?;

        // Validate positions
        self.validate_positions(&positions)?;

        Ok(())
    }
}

/// RFC-010: Generic parameter validator
pub struct GenericValidator {
    max_type_params: usize,
    max_const_params: usize,
}

impl GenericValidator {
    /// Create new validator
    pub fn new(
        max_type_params: usize,
        max_const_params: usize,
    ) -> Self {
        Self {
            max_type_params,
            max_const_params,
        }
    }

    /// Validate generic parameter list
    /// Supports RFC-010 syntax: (T: Type), (T: Clone), (T: Type, N: Int)
    ///
    /// Note: This method validates type parameters only. For const parameters,
    /// use `validate_const_params` method separately.
    pub fn validate_generic_params(
        &self,
        params: &[String],
    ) -> Result<(), String> {
        // Count type params (excluding potential const params)
        // Heuristic: const params are typically ALL_CAPS with numbers like N32, MAX_SIZE
        let type_params: Vec<&String> = params
            .iter()
            .filter(|p| !Self::is_const_param_style(p))
            .collect();

        if type_params.len() > self.max_type_params {
            return Err(format!(
                "Too many type parameters: {}, maximum is {}",
                type_params.len(),
                self.max_type_params
            ));
        }

        // Validate const params count using heuristic
        let const_params: Vec<&String> = params
            .iter()
            .filter(|p| Self::is_const_param_style(p))
            .collect();

        if const_params.len() > self.max_const_params {
            return Err(format!(
                "Too many const parameters: {}, maximum is {}",
                const_params.len(),
                self.max_const_params
            ));
        }

        for param in params {
            if param.trim().is_empty() {
                return Err("Empty generic parameter".to_string());
            }
        }

        Ok(())
    }

    /// Validate const parameter count directly
    /// Use this when you know the exact number of const parameters
    pub fn validate_const_params(
        &self,
        count: usize,
    ) -> Result<(), String> {
        if count > self.max_const_params {
            return Err(format!(
                "Too many const parameters: {}, maximum is {}",
                count, self.max_const_params
            ));
        }
        Ok(())
    }

    /// Check if a parameter name looks like a const parameter
    /// Heuristic: const params are typically ALL_CAPS with optional trailing digits
    fn is_const_param_style(param: &str) -> bool {
        let trimmed = param.trim();
        if trimmed.is_empty() {
            return false;
        }

        // Must be all uppercase or followed by digits
        let has_upper = trimmed.chars().any(|c| c.is_uppercase());
        let has_digits = trimmed.chars().any(|c| c.is_ascii_digit());

        // Const style: ALL_CAPS or UPPER_CASE_123 or just NUMBERS like 32
        has_upper && has_digits || trimmed.chars().all(|c| c.is_ascii_digit())
    }

    /// Validate generic constraint
    /// Supports RFC-010 syntax: T: Clone + Add
    pub fn validate_constraint(
        &self,
        constraint: &str,
    ) -> Result<(), String> {
        // Basic constraint validation
        if !constraint.contains(':') {
            return Err("Invalid constraint syntax: missing ':'".to_string());
        }

        // Check for valid trait names (basic validation)
        let parts: Vec<&str> = constraint.split(':').collect();
        if parts.len() != 2 {
            return Err("Invalid constraint syntax".to_string());
        }

        let param_name = parts[0].trim();
        let traits = parts[1].trim();

        if param_name.is_empty() {
            return Err("Empty parameter name in constraint".to_string());
        }

        // Validate trait names
        for trait_name in traits.split('+') {
            let trait_name = trait_name.trim();
            if trait_name.is_empty() {
                return Err("Empty trait name in constraint".to_string());
            }
            // Basic identifier validation
            if !trait_name
                .chars()
                .all(|c| c.is_ascii_alphanumeric() || c == '_')
            {
                return Err(format!("Invalid trait name: {}", trait_name));
            }
        }

        Ok(())
    }
}

/// RFC-011: Type system validator
pub struct TypeSystemValidator {
    max_type_depth: usize,
    max_constraint_count: usize,
}

impl TypeSystemValidator {
    /// Create new validator
    pub fn new(
        max_type_depth: usize,
        max_constraint_count: usize,
    ) -> Self {
        Self {
            max_type_depth,
            max_constraint_count,
        }
    }

    /// Validate type expression complexity
    pub fn validate_type_complexity(
        &self,
        type_str: &str,
    ) -> Result<(), String> {
        let depth = self.calculate_nesting_depth(type_str);
        if depth > self.max_type_depth {
            return Err(format!(
                "Type nesting too deep: {}, maximum is {}",
                depth, self.max_type_depth
            ));
        }
        Ok(())
    }

    /// Calculate nesting depth of a type expression
    fn calculate_nesting_depth(
        &self,
        type_str: &str,
    ) -> usize {
        let mut max_depth = 0;
        let mut current_depth: usize = 0;

        for ch in type_str.chars() {
            match ch {
                '<' | '{' | '(' => {
                    current_depth += 1;
                    max_depth = max_depth.max(current_depth);
                }
                '>' | '}' | ')' => {
                    current_depth = current_depth.saturating_sub(1);
                }
                _ => {}
            }
        }

        max_depth
    }

    /// Validate constraint count
    pub fn validate_constraint_count(
        &self,
        constraints: &[String],
    ) -> Result<(), String> {
        if constraints.len() > self.max_constraint_count {
            return Err(format!(
                "Too many constraints: {}, maximum is {}",
                constraints.len(),
                self.max_constraint_count
            ));
        }
        Ok(())
    }
}
