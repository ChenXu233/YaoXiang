//! Move 语义检查
//!
//! 检查 UseAfterMove 错误：检测赋值和函数调用后对原值的使用。
//!
//! # 空状态重用支持
//!
//! 实现 Move 后变量进入空状态（Empty），允许重新赋值复用变量名：
//! - `p = Point(1.0); p2 = p; p = Point(2.0)` - 编译通过
//! - `p = Point(1.0); p2 = p; print(p)` - 编译失败（UseAfterMove）
//!
//! # 状态转换
//!
//! ```text
//!     Owned ──Move──► Moved ──(Store)──► Empty ──(Store, 类型一致)──► Owned
//!                                          ▲
//!                                          │
//!                                     报错：类型不匹配
//!
//!     Owned ──(Store)──► 报错：ReassignNonEmpty
//! ```

use super::error::{OwnershipCheck, OwnershipError, TypeId, ValueState, operand_to_string};
use crate::middle::core::ir::{FunctionIR, Instruction, Operand};
use std::collections::HashMap;

/// Move 检查器
///
/// 检测以下错误：
/// - UseAfterMove: 使用已移动的值
/// - UseAfterEmpty: 使用空状态的值（已移动后可重新赋值）
/// - EmptyStateTypeMismatch: 空状态重赋值类型不匹配
/// - ReassignNonEmpty: 重新赋值时值非空状态
#[derive(Debug)]
pub struct MoveChecker {
    pub state: HashMap<Operand, ValueState>,
    pub errors: Vec<OwnershipError>,
    pub location: (usize, usize),
    /// 类型表：变量 -> 类型ID
    type_map: HashMap<Operand, TypeId>,
    /// 函数类型表：类型名 -> 类型ID（外部传入）
    type_table: Option<HashMap<String, TypeId>>,
}

impl MoveChecker {
    pub fn new() -> Self {
        Self {
            state: HashMap::new(),
            errors: Vec::new(),
            location: (0, 0),
            type_map: HashMap::new(),
            type_table: None,
        }
    }

    /// 设置变量的类型
    pub fn set_type(
        &mut self,
        operand: Operand,
        type_id: TypeId,
    ) {
        self.type_map.insert(operand, type_id);
    }

    /// 设置类型表（用于类型检查）
    pub fn with_type_table(
        mut self,
        type_table: HashMap<String, TypeId>,
    ) -> Self {
        self.type_table = Some(type_table);
        self
    }

    fn check_instruction(
        &mut self,
        instr: &Instruction,
    ) {
        match instr {
            Instruction::Move { dst, src } => self.check_move(dst, src),
            Instruction::Call { dst, args, .. } => self.check_call(args, dst.as_ref()),
            Instruction::Ret(Some(value)) => self.check_ret(value),
            Instruction::Store { dst, src } => self.check_store(dst, src),
            Instruction::LoadIndex { src, .. }
            | Instruction::LoadField { src, .. }
            | Instruction::Neg { src, .. }
            | Instruction::Cast { src, .. } => self.check_used(src),
            Instruction::Add { lhs, rhs, .. }
            | Instruction::Sub { lhs, rhs, .. }
            | Instruction::Mul { lhs, rhs, .. }
            | Instruction::Div { lhs, rhs, .. }
            | Instruction::Mod { lhs, rhs, .. } => {
                self.check_used(lhs);
                self.check_used(rhs);
            }
            Instruction::Eq { lhs, rhs, .. }
            | Instruction::Ne { lhs, rhs, .. }
            | Instruction::Lt { lhs, rhs, .. }
            | Instruction::Le { lhs, rhs, .. }
            | Instruction::Gt { lhs, rhs, .. }
            | Instruction::Ge { lhs, rhs, .. } => {
                self.check_used(lhs);
                self.check_used(rhs);
            }
            _ => {}
        }
    }

    fn check_move(
        &mut self,
        dst: &Operand,
        src: &Operand,
    ) {
        // 检查 src 的状态
        if let Some(state) = self.state.get(src) {
            match state {
                ValueState::Empty => {
                    // 空状态变量的 Move 是合法的
                    // 保持 Empty 状态
                }
                ValueState::Owned(_) => {
                    // 正常 Move：src 变为 Empty（可以直接重新赋值）
                    self.state.insert(src.clone(), ValueState::Empty);
                    // 继承类型信息到 Empty 状态
                    if let Some(t) = self.type_map.get(src).cloned() {
                        self.type_map.insert(src.clone(), t);
                    }
                }
                ValueState::Moved => {
                    // 已移动的值再次被 Move，报错
                    self.report_use_after_move(src);
                }
                ValueState::Dropped => {
                    // 已释放的值不能 Move
                }
            }
        } else {
            // 首次使用，设为 Empty
            self.state.insert(src.clone(), ValueState::Empty);
        }

        // dst 变为 Owned，继承 src 的类型
        let src_type = self.type_map.get(src).cloned();
        self.state
            .insert(dst.clone(), ValueState::Owned(src_type.clone()));
        if let Some(t) = src_type {
            self.type_map.insert(dst.clone(), t);
        }
    }

    fn check_store(
        &mut self,
        dst: &Operand,
        src: &Operand,
    ) {
        // 获取 src 的类型
        let src_type = self.type_map.get(src).cloned();

        // 检查 dst 的当前状态
        if let Some(state) = self.state.get(dst) {
            match state {
                ValueState::Owned(_) => {
                    // 非空状态变量的重新赋值，报错
                    self.errors.push(OwnershipError::ReassignNonEmpty {
                        value: operand_to_string(dst),
                        location: self.location,
                    });
                    return;
                }
                ValueState::Moved => {
                    // Moved 状态可以直接赋值
                    // 赋值后变为 Owned(新类型)
                }
                ValueState::Empty => {
                    // 空状态，检查类型一致性
                    if let Some(expected_type) = self.type_map.get(dst) {
                        if let Some(actual_type) = &src_type {
                            if expected_type != actual_type {
                                self.errors.push(OwnershipError::EmptyStateTypeMismatch {
                                    value: operand_to_string(dst),
                                    expected_type: expected_type.0.clone(),
                                    actual_type: actual_type.0.clone(),
                                    location: self.location,
                                });
                                return;
                            }
                        }
                    }
                }
                ValueState::Dropped => {
                    // 已释放的变量不能重新赋值
                }
            }
        }

        // dst 变为 Owned(新类型)
        self.state
            .insert(dst.clone(), ValueState::Owned(src_type.clone()));
        if let Some(t) = src_type {
            self.type_map.insert(dst.clone(), t);
        }

        // src 被使用，进入 Moved
        self.check_used(src);
    }

    fn check_call(
        &mut self,
        args: &[Operand],
        dst: Option<&Operand>,
    ) {
        // 处理返回值目标
        if let Some(dst_operand) = dst {
            // 返回值赋值给 dst，dst 进入 Owned 状态
            // 注意：这里不检查返回值是否被"移动"，因为 Call 返回的是一个新值
            self.state.insert(dst_operand.clone(), ValueState::Empty);
        }

        // 处理参数
        for arg in args {
            if let Some(state) = self.state.get(arg) {
                match state {
                    ValueState::Empty => {
                        self.report_use_after_move(arg);
                    }
                    ValueState::Owned(_) => {
                        // 函数调用消费参数，进入 Empty 状态
                        self.state.insert(arg.clone(), ValueState::Empty);
                    }
                    ValueState::Moved => {
                        self.report_use_after_move(arg);
                    }
                    ValueState::Dropped => {
                        // 已释放的值不能使用
                    }
                }
            } else {
                // 首次使用，设为 Empty
                self.state.insert(arg.clone(), ValueState::Empty);
            }
        }
    }

    fn check_ret(
        &mut self,
        value: &Operand,
    ) {
        if let Some(state) = self.state.get(value) {
            match state {
                ValueState::Owned(_) => {
                    // 返回值被移动，进入 Empty 状态
                    self.state.insert(value.clone(), ValueState::Empty);
                }
                ValueState::Empty => {
                    // 空状态返回值是合法的
                }
                ValueState::Moved => {
                    // 已移动的值被返回是合法的（所有权转移）
                }
                ValueState::Dropped => {
                    // 已释放的值不能返回
                }
            }
        }
    }

    fn check_used(
        &mut self,
        operand: &Operand,
    ) {
        if let Some(state) = self.state.get(operand) {
            match state {
                ValueState::Moved => {
                    // 已移动的值不能使用
                    self.report_use_after_move(operand);
                }
                ValueState::Empty => {
                    // 空状态的值不能再被"移动"使用，但可以被重新赋值
                    // 检查是否是作为 dst 使用（即将被赋值）
                    // 这里我们假设 check_used 主要用于读取场景
                    self.report_use_after_move(operand);
                }
                ValueState::Owned(_) | ValueState::Dropped => {
                    // 正常使用
                }
            }
        }
    }

    fn report_use_after_move(
        &mut self,
        operand: &Operand,
    ) {
        self.errors.push(OwnershipError::UseAfterMove {
            value: operand_to_string(operand),
            location: self.location,
        });
    }
}

impl OwnershipCheck for MoveChecker {
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

    fn state(&self) -> &HashMap<Operand, ValueState> {
        &self.state
    }

    fn clear(&mut self) {
        self.state.clear();
        self.errors.clear();
        self.type_map.clear();
    }
}

impl Default for MoveChecker {
    fn default() -> Self {
        Self::new()
    }
}
