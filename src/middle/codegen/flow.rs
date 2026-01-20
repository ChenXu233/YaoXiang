//! 流状态管理
//!
//! 整合寄存器分配、标签生成、跳转表管理和符号表/作用域管理。

use crate::frontend::typecheck::MonoType;
use std::collections::{HashMap, HashSet};

// ===== 跳转表和流程控制 =====

/// 标签生成器
#[derive(Debug, Default)]
pub struct LabelGenerator {
    next_label: usize,
}

impl LabelGenerator {
    pub fn new() -> Self {
        LabelGenerator { next_label: 0 }
    }
    pub fn next(&mut self) -> usize {
        let label = self.next_label;
        self.next_label += 1;
        label
    }
    pub fn reset(&mut self) {
        self.next_label = 0;
    }
    pub fn peek_next(&self) -> usize {
        self.next_label
    }
}

/// 寄存器分配器
#[derive(Debug, Default)]
pub struct RegisterAllocator {
    next_local: usize,
    next_temp: usize,
    allocated: HashSet<usize>,
}

impl RegisterAllocator {
    pub fn new() -> Self {
        RegisterAllocator {
            next_local: 0,
            next_temp: 0,
            allocated: HashSet::new(),
        }
    }
    pub fn alloc_local(&mut self) -> usize {
        let id = self.next_local;
        self.next_local += 1;
        self.allocated.insert(id);
        id
    }
    pub fn alloc_temp(&mut self) -> usize {
        let id = self.next_temp;
        self.next_temp += 1;
        self.allocated.insert(id);
        id
    }
    pub fn next_local_id(&self) -> usize {
        self.next_local
    }
    pub fn next_temp_id(&self) -> usize {
        self.next_temp
    }
    pub fn reset(&mut self) {
        self.next_local = 0;
        self.next_temp = 0;
        self.allocated.clear();
    }
    pub fn allocated_count(&self) -> usize {
        self.allocated.len()
    }
}

/// 跳转表
#[derive(Debug, Clone)]
pub struct JumpTable {
    pub index: u16,
    pub entries: HashMap<usize, usize>,
}

impl JumpTable {
    pub fn new(index: u16) -> Self {
        JumpTable {
            index,
            entries: HashMap::new(),
        }
    }
    pub fn add_entry(
        &mut self,
        key: usize,
        target: usize,
    ) {
        self.entries.insert(key, target);
    }
    pub fn get(
        &self,
        key: &usize,
    ) -> Option<&usize> {
        self.entries.get(key)
    }
    pub fn len(&self) -> usize {
        self.entries.len()
    }
}

/// 控制流管理器
#[derive(Debug, Default)]
pub struct FlowManager {
    register_allocator: RegisterAllocator,
    label_generator: LabelGenerator,
    code_offsets: HashMap<usize, usize>,
    jump_tables: HashMap<u16, JumpTable>,
    function_indices: HashMap<String, usize>,
    current_loop_label: Option<(usize, usize)>,
}

impl FlowManager {
    pub fn new() -> Self {
        FlowManager {
            register_allocator: RegisterAllocator::new(),
            label_generator: LabelGenerator::new(),
            code_offsets: HashMap::new(),
            jump_tables: HashMap::new(),
            function_indices: HashMap::new(),
            current_loop_label: None,
        }
    }

    // 寄存器分配
    pub fn alloc_local(&mut self) -> usize {
        self.register_allocator.alloc_local()
    }
    pub fn alloc_temp(&mut self) -> usize {
        self.register_allocator.alloc_temp()
    }
    pub fn next_local_id(&self) -> usize {
        self.register_allocator.next_local_id()
    }
    pub fn next_temp_id(&self) -> usize {
        self.register_allocator.next_temp_id()
    }
    pub fn reset_registers(&mut self) {
        self.register_allocator.reset();
    }

    // 标签生成
    pub fn next_label(&mut self) -> usize {
        self.label_generator.next()
    }

    // 跳转表
    pub fn add_code_offset(
        &mut self,
        instr_idx: usize,
        offset: usize,
    ) {
        self.code_offsets.insert(instr_idx, offset);
    }
    pub fn get_code_offset(
        &self,
        instr_idx: &usize,
    ) -> Option<&usize> {
        self.code_offsets.get(instr_idx)
    }
    pub fn add_jump_table(
        &mut self,
        table: JumpTable,
    ) {
        self.jump_tables.insert(table.index, table);
    }
    pub fn get_jump_table(
        &self,
        index: &u16,
    ) -> Option<&JumpTable> {
        self.jump_tables.get(index)
    }
    pub fn get_jump_table_index(&self) -> Option<u16> {
        if self.jump_tables.is_empty() {
            Some(0)
        } else {
            self.jump_tables.keys().max().map(|k| k + 1)
        }
    }

    // 函数索引
    pub fn add_function_index(
        &mut self,
        name: String,
        index: usize,
    ) {
        self.function_indices.insert(name, index);
    }
    pub fn get_function_index(
        &self,
        name: &str,
    ) -> Option<&usize> {
        self.function_indices.get(name)
    }
    pub fn function_indices(&self) -> &HashMap<String, usize> {
        &self.function_indices
    }

    // 循环标签
    pub fn set_loop_label(
        &mut self,
        loop_label: usize,
        end_label: usize,
    ) {
        self.current_loop_label = Some((loop_label, end_label));
    }
    pub fn loop_label(&self) -> Option<(usize, usize)> {
        self.current_loop_label
    }
    pub fn clear_loop_label(&mut self) {
        self.current_loop_label = None;
    }
}

// ===== 符号表和作用域 =====

/// 符号信息
#[derive(Debug, Clone)]
pub struct Symbol {
    pub name: String,
    pub ty: MonoType,
    pub storage: Storage,
    pub is_mut: bool,
    pub scope_level: usize,
}

/// 存储位置
#[derive(Debug, Clone)]
pub enum Storage {
    Local(usize),
    Arg(usize),
    Temp(usize),
    Global(usize),
}

/// 符号表
#[derive(Debug, Default)]
pub struct SymbolTable {
    symbols: HashMap<String, Symbol>,
}

impl SymbolTable {
    pub fn new() -> Self {
        SymbolTable {
            symbols: HashMap::new(),
        }
    }
    pub fn insert(
        &mut self,
        name: String,
        symbol: Symbol,
    ) {
        self.symbols.insert(name, symbol);
    }
    pub fn get(
        &self,
        name: &str,
    ) -> Option<&Symbol> {
        self.symbols.get(name)
    }
    pub fn get_mut(
        &mut self,
        name: &str,
    ) -> Option<&mut Symbol> {
        self.symbols.get_mut(name)
    }
    pub fn contains(
        &self,
        name: &str,
    ) -> bool {
        self.symbols.contains_key(name)
    }
    pub fn len(&self) -> usize {
        self.symbols.len()
    }
    pub fn is_empty(&self) -> bool {
        self.symbols.is_empty()
    }
    pub fn iter(&self) -> impl Iterator<Item = (&String, &Symbol)> {
        self.symbols.iter()
    }
}

/// 作用域管理器
#[derive(Debug, Default)]
pub struct SymbolScopeManager {
    symbol_table: SymbolTable,
    scopes: Vec<HashMap<String, Symbol>>,
    scope_level: usize,
}

impl SymbolScopeManager {
    pub fn new() -> Self {
        SymbolScopeManager {
            symbol_table: SymbolTable::new(),
            scopes: vec![HashMap::new()],
            scope_level: 0,
        }
    }
    pub fn push_scope(&mut self) {
        self.scopes.push(HashMap::new());
        self.scope_level += 1;
    }
    pub fn pop_scope(&mut self) {
        if self.scopes.len() > 1 {
            self.scopes.pop();
            self.scope_level -= 1;
        }
    }
    pub fn current_scope(&mut self) -> &mut HashMap<String, Symbol> {
        self.scopes.last_mut().unwrap()
    }
    pub fn insert(
        &mut self,
        name: String,
        symbol: Symbol,
    ) {
        if let Some(scope) = self.scopes.last_mut() {
            scope.insert(name, symbol);
        }
    }
    pub fn lookup(
        &self,
        name: &str,
    ) -> Option<&Symbol> {
        for scope in self.scopes.iter().rev() {
            if let Some(symbol) = scope.get(name) {
                return Some(symbol);
            }
        }
        None
    }
    pub fn lookup_current(
        &self,
        name: &str,
    ) -> Option<&Symbol> {
        self.scopes.last().and_then(|s| s.get(name))
    }
    pub fn scope_level(&self) -> usize {
        self.scope_level
    }
    pub fn symbol_table(&self) -> &SymbolTable {
        &self.symbol_table
    }
    pub fn symbol_table_mut(&mut self) -> &mut SymbolTable {
        &mut self.symbol_table
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_label_generator() {
        let mut gen = LabelGenerator::new();
        assert_eq!(gen.next(), 0);
        assert_eq!(gen.next(), 1);
        gen.reset();
        assert_eq!(gen.next(), 0);
    }

    #[test]
    fn test_register_allocator() {
        let mut reg = RegisterAllocator::new();
        assert_eq!(reg.alloc_local(), 0);
        assert_eq!(reg.alloc_local(), 1);
        assert_eq!(reg.alloc_temp(), 0);
        assert_eq!(reg.alloc_temp(), 1);
    }

    #[test]
    fn test_flow_manager() {
        let mut cfm = FlowManager::new();
        assert_eq!(cfm.alloc_local(), 0);
        assert_eq!(cfm.alloc_temp(), 0);
        assert_eq!(cfm.next_label(), 0);
        cfm.add_function_index("main".to_string(), 0);
        assert_eq!(cfm.get_function_index("main"), Some(&0));
        cfm.set_loop_label(10, 20);
        assert_eq!(cfm.loop_label(), Some((10, 20)));
    }

    #[test]
    fn test_scope_manager_basic() {
        let mut manager = SymbolScopeManager::new();
        manager.insert(
            "x".to_string(),
            Symbol {
                name: "x".to_string(),
                ty: MonoType::Int(64),
                storage: Storage::Local(0),
                is_mut: false,
                scope_level: 0,
            },
        );
        assert!(manager.lookup("x").is_some());
        assert!(manager.lookup("y").is_none());
    }

    #[test]
    fn test_scope_nesting() {
        let mut manager = SymbolScopeManager::new();
        manager.insert(
            "a".to_string(),
            Symbol {
                name: "a".to_string(),
                ty: MonoType::Int(64),
                storage: Storage::Local(0),
                is_mut: false,
                scope_level: 0,
            },
        );
        manager.push_scope();
        assert_eq!(manager.scope_level(), 1);
        manager.insert(
            "b".to_string(),
            Symbol {
                name: "b".to_string(),
                ty: MonoType::String,
                storage: Storage::Local(1),
                is_mut: true,
                scope_level: 1,
            },
        );
        assert!(manager.lookup("a").is_some());
        assert!(manager.lookup("b").is_some());
        manager.pop_scope();
        assert_eq!(manager.scope_level(), 0);
        assert!(manager.lookup("b").is_none());
        assert!(manager.lookup("a").is_some());
    }
}
