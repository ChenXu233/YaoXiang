//! 可变性检查器
//!
//! 检查 `mut` 标记的使用是否符合规则：
//! - 所有变量默认不可变
//! - 只有标记 `mut` 的变量才能被修改
//! - 编译期检查，无需运行时开销

use super::error::{OwnershipCheck, OwnershipError, operand_to_string};
use crate::middle::ir::{FunctionIR, Instruction, Operand};
use std::collections::HashMap;

/// 可变性检查器
///
/// 检测以下错误：
/// - ImmutableAssign: 对不可变变量进行赋值
/// - ImmutableMutation: 调用不可变对象上的变异方法
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

    fn check_instruction(
        &mut self,
        instr: &Instruction,
    ) {
        match instr {
            // Store: 赋值操作，检查目标是否可变
            Instruction::Store { dst, .. } => self.check_store(dst),
            Instruction::StoreIndex { dst, .. } => self.check_store(dst),
            Instruction::StoreField { dst, .. } => self.check_store(dst),
            // Call: 方法调用，检查是否是变异方法
            Instruction::Call {
                func: Operand::Const(crate::middle::ir::ConstValue::String(method)),
                args,
                ..
            } if !args.is_empty() => {
                self.check_mutation_method(method, &args[0]);
            }
            _ => {}
        }
    }

    /// 检查赋值操作
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
        _operand: &Operand,
        _symbol_table: &HashMap<String, bool>,
    ) -> Option<bool> {
        match _operand {
            Operand::Local(_) => None,
            Operand::Temp(_) => None,
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
