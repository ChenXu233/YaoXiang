//! 可变性检查器
//!
//! 检查 `mut` 标记的使用是否符合规则：
//! - 所有变量默认不可变
//! - 只有标记 `mut` 的变量才能被修改
//! - 编译期检查，无需运行时开销

use super::error::{OwnershipCheck, OwnershipError, operand_to_string};
use crate::middle::core::ir::{FunctionIR, Instruction, Operand};
use std::collections::HashMap;

/// 可变性检查器
///
/// 检测以下错误：
/// - ImmutableAssign: 对不可变变量进行赋值
/// - ImmutableMutation: 调用不可变对象上的变异方法
/// - ImmutableFieldAssign: 对不可变字段进行赋值
#[derive(Debug)]
pub struct MutChecker {
    /// 可变变量集合 (Operand -> is_mut)
    mutable_vars: HashMap<Operand, bool>,
    /// 可变变量修改错误
    errors: Vec<OwnershipError>,
    /// 当前检查位置
    location: (usize, usize),
    /// 符号表：变量名 -> 是否可变（从外部传入）
    symbol_table: Option<HashMap<String, bool>>,
    /// 类型表：类型名 -> StructType（包含字段可变性信息）
    type_table: Option<HashMap<String, crate::frontend::core::type_system::StructType>>,
    /// 兼容 OwnershipCheck trait 的状态字段（未使用）
    state: HashMap<Operand, super::error::ValueState>,
}

impl MutChecker {
    /// 创建新的可变性检查器
    pub fn new() -> Self {
        Self {
            mutable_vars: HashMap::new(),
            errors: Vec::new(),
            location: (0, 0),
            symbol_table: None,
            type_table: None,
            state: HashMap::new(),
        }
    }

    /// 设置符号表（用于查询变量是否可变）
    pub fn with_symbol_table(
        mut self,
        symbol_table: HashMap<String, bool>,
    ) -> Self {
        self.symbol_table = Some(symbol_table);
        self
    }

    /// 设置类型表（用于查询字段可变性）
    pub fn with_type_table(
        mut self,
        type_table: HashMap<String, crate::frontend::core::type_system::StructType>,
    ) -> Self {
        self.type_table = Some(type_table);
        self
    }

    fn check_instruction(
        &mut self,
        instr: &Instruction,
    ) {
        match instr {
            // Store: 赋值操作，检查目标是否可变
            Instruction::Store { dst, .. } => self.check_store(dst),
            // StoreIndex: 索引赋值，检查目标是否可变
            Instruction::StoreIndex { dst, .. } => self.check_store(dst),
            // StoreField: 字段赋值，需要检查字段可变性
            Instruction::StoreField {
                dst,
                field,
                type_name,
                field_name,
                ..
            } => self.check_store_field(dst, *field, type_name, field_name),
            // Call: 方法调用，检查是否是变异方法
            Instruction::Call {
                func: Operand::Const(crate::middle::core::ir::ConstValue::String(method)),
                args,
                ..
            } if !args.is_empty() => {
                self.check_mutation_method(method, &args[0]);
            }
            _ => {}
        }
    }

    /// 检查变量赋值操作
    fn check_store(
        &mut self,
        target: &Operand,
    ) {
        if self.is_mutable(target) {
            return;
        }
        self.errors.push(OwnershipError::ImmutableAssign {
            value: operand_to_string(target),
            location: self.location,
        });
    }

    /// 检查字段赋值操作
    ///
    /// 规则：
    /// - 绑定可变：可以写任意字段
    /// - 绑定不可变：只能写可变字段
    ///
    /// 通过 StoreField 指令携带的类型信息进行检查
    fn check_store_field(
        &mut self,
        target: &Operand,
        field_index: usize,
        type_name: &Option<String>,
        field_name: &Option<String>,
    ) {
        // 1. 首先检查绑定本身是否可变
        let binding_is_mutable = self.is_mutable(target);

        if binding_is_mutable {
            // 绑定可变，可以写任意字段
            return;
        }

        // 2. 绑定不可变，检查目标字段是否可变
        if let Some(type_name) = type_name {
            if let Some(type_table) = &self.type_table {
                if let Some(struct_type) = type_table.get(type_name) {
                    // 获取字段可变性
                    let field_is_mutable = struct_type
                        .field_mutability
                        .get(field_index)
                        .copied()
                        .unwrap_or(false);

                    if !field_is_mutable {
                        // 字段不可变，报错
                        let struct_name = type_name.clone();
                        let field = field_name
                            .clone()
                            .unwrap_or_else(|| format!("field_{}", field_index));
                        self.errors.push(OwnershipError::ImmutableFieldAssign {
                            struct_name,
                            field,
                            location: self.location,
                        });
                    }

                    // 字段可变，允许
                }
            }
        }

        // 无法确定类型信息，先允许（保守策略）
        // 或者可以选择严格模式，在缺少类型信息时报错
    }

    /// 检查变异方法调用
    fn check_mutation_method(
        &mut self,
        method: &str,
        target: &Operand,
    ) {
        if !is_mutation_method(method) {
            return;
        }
        if self.is_mutable(target) {
            return;
        }
        self.errors.push(OwnershipError::ImmutableMutation {
            value: operand_to_string(target),
            method: method.to_string(),
            location: self.location,
        });
    }

    /// 检查变量是否可变（通用逻辑）
    fn is_mutable(
        &self,
        target: &Operand,
    ) -> bool {
        // 检查可变变量集合
        if let Some(&is_mut) = self.mutable_vars.get(target) {
            return is_mut;
        }
        // 检查符号表
        if let Some(symbol_table) = &self.symbol_table {
            if let Some(is_mut) = self.get_symbol_mutability(target, symbol_table) {
                return is_mut;
            }
        }
        false
    }

    /// 从 Operand 获取符号的可变性
    fn get_symbol_mutability(
        &self,
        operand: &Operand,
        symbol_table: &HashMap<String, bool>,
    ) -> Option<bool> {
        match operand {
            Operand::Local(idx) => {
                // 尝试从寄存器索引构建符号名
                // 注意：这需要 IR 生成阶段将寄存器索引与符号名关联
                let symbol_name = format!("local_{}", idx);
                symbol_table.get(&symbol_name).copied()
            }
            Operand::Temp(_) => {
                // 临时变量默认不可变
                None
            }
            _ => None,
        }
    }

    /// 记录 mut 声明
    pub fn record_mut_declaration(
        &mut self,
        value_id: Operand,
    ) {
        self.mutable_vars.insert(value_id, true);
    }
}

impl OwnershipCheck for MutChecker {
    fn check_function(
        &mut self,
        func: &FunctionIR,
    ) -> &[OwnershipError] {
        self.clear();

        for (block_idx, block) in func.blocks.iter().enumerate() {
            for (instr_idx, instr) in block.instructions.iter().enumerate() {
                self.location = (block_idx, instr_idx);
                self.check_instruction(instr);
            }
        }

        &self.errors
    }

    fn errors(&self) -> &[OwnershipError] {
        &self.errors
    }

    fn state(&self) -> &HashMap<Operand, super::error::ValueState> {
        &self.state
    }

    fn clear(&mut self) {
        self.mutable_vars.clear();
        self.errors.clear();
        self.state.clear();
    }
}

impl Default for MutChecker {
    fn default() -> Self {
        Self::new()
    }
}

/// 判断是否是变异方法
///
/// 变异方法会修改调用者本身，而不是返回新值。
/// 函数式风格的方法通常返回新值（如 `concat`），而不修改原值。
pub fn is_mutation_method(method: &str) -> bool {
    MUTATION_METHODS.contains(&method)
}

/// 变异方法集合（使用 HashSet 实现 O(1) 查询）
static MUTATION_METHODS: once_cell::sync::Lazy<std::collections::HashSet<&'static str>> =
    once_cell::sync::Lazy::new(|| {
        [
            "push", "pop", "insert", "remove", "clear", "append", "extend", "set", "update", "add",
            "delete", "discard", "swap", "fill",
        ]
        .into_iter()
        .collect()
    });
