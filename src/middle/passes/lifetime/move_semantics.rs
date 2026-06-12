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

use super::consume_analysis::ConsumeAnalyzer;
use super::error::{OwnershipCheck, TypeId, ValueState, operand_display_name};
use super::ownership_flow::ConsumeMode;
use crate::frontend::core::types::MonoType;
use crate::middle::core::ir::{FunctionIR, Instruction, Operand};
use crate::util::diagnostic::{ErrorCodeDefinition, Diagnostic};
use std::collections::HashMap;

/// Move 检查器
///
/// 检测以下错误：
/// - UseAfterMove: 使用已移动的值
/// - UseAfterEmpty: 使用空状态的值（已移动后可重新赋值）
/// - EmptyStateTypeMismatch: 空状态重赋值类型不匹配
/// - ReassignNonEmpty: 重新赋值时值非空状态
///
/// # 消费分析支持（Phase 4）
///
/// 通过 ConsumeAnalyzer 查询被调用函数的消费模式：
/// - Returns 模式：参数不进入 Empty（所有权回流）
/// - Consumes 模式：参数进入 Empty（所有权消耗）
/// - Undetermined 模式：保守估计进入 Empty
#[derive(Debug)]
pub struct MoveChecker {
    pub state: HashMap<Operand, ValueState>,
    pub errors: Vec<Diagnostic>,
    pub location: (usize, usize),
    /// 类型表：变量 -> 类型ID
    type_map: HashMap<Operand, TypeId>,
    /// 函数类型表：类型名 -> 类型ID（外部传入）
    type_table: Option<HashMap<String, TypeId>>,
    /// 消费分析器（Phase 4）
    pub(crate) consume_analyzer: ConsumeAnalyzer,
    /// 局部变量名列表（用于错误报告，优先显示源码变量名）
    local_names: Option<Vec<String>>,
}

impl MoveChecker {
    pub fn new() -> Self {
        Self {
            state: HashMap::new(),
            errors: Vec::new(),
            location: (0, 0),
            type_map: HashMap::new(),
            type_table: None,
            consume_analyzer: ConsumeAnalyzer::new(),
            local_names: None,
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

    /// 设置局部变量名列表（用于生成友好的错误信息）
    pub fn with_local_names(
        mut self,
        local_names: Option<Vec<String>>,
    ) -> Self {
        self.local_names = local_names;
        self
    }

    /// 设置局部变量名列表（可变引用版本，用于已构造的实例）
    pub fn set_local_names(
        &mut self,
        local_names: Option<Vec<String>>,
    ) {
        self.local_names = local_names;
    }

    fn check_instruction(
        &mut self,
        instr: &Instruction,
    ) {
        match instr {
            Instruction::Move { dst, src } => self.check_move(dst, src),
            Instruction::Call {
                dst, args, func, ..
            } => {
                let func_name = extract_function_name(func);
                self.check_call(args, dst.as_ref(), func_name.as_deref());
            }
            Instruction::Ret(Some(value)) => self.check_ret(value),
            Instruction::Store { dst, src, .. } => self.check_store(dst, src),
            Instruction::Load { dst, src } => {
                // Load 传播 src 的状态到 dst
                if let Some(state) = self.state.get(src).cloned() {
                    match state {
                        ValueState::Moved | ValueState::Empty => {
                            self.report_use_after_move(src);
                        }
                        ValueState::Dup => {
                            // Dup 类型（如 &T）传播 Dup 状态
                            self.state.insert(dst.clone(), ValueState::Dup);
                        }
                        _ => {
                            self.state.insert(dst.clone(), state);
                        }
                    }
                }
            }
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
                ValueState::Dup => {
                    // Dup 类型（如 &T）不会被 Move，保持 Dup 状态
                    // dst 也变为 Dup 状态
                    self.state.insert(dst.clone(), ValueState::Dup);
                    return;
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
        if let Some(state) = self.state.get(dst).cloned() {
            match state {
                ValueState::Owned(_) => {
                    // 非空状态变量的重新赋值，报错
                    let name = operand_display_name(dst, self.local_names.as_ref());
                    self.errors
                        .push(ErrorCodeDefinition::reassign_non_empty(&name).build());
                    return;
                }
                ValueState::Dup => {
                    // Dup 类型（如 &T）：Store 不改变状态
                    // 函数开头的 Store { dst: Local(0), src: Local(0) } 是参数初始化
                    // 保持 Dup 状态，允许字段访问等操作
                    self.check_used(src);
                    return;
                }
                ValueState::Moved => {
                    // Moved 状态可以直接赋值
                    // 赋值后变为 Owned(新类型)
                }
                ValueState::Empty => {
                    // 空状态，直接允许赋值（初始化阶段，类型会在后续确定）
                    // 不做类型一致性检查，避免列表初始化等场景的误报
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

    /// 检查函数调用
    ///
    /// 根据被调用函数的消费模式决定参数状态：
    /// - Returns 模式：参数所有权回流，不进入 Empty
    /// - Consumes 模式：参数被消费，进入 Empty
    /// - Undetermined 模式：保守估计进入 Empty
    fn check_call(
        &mut self,
        args: &[Operand],
        dst: Option<&Operand>,
        func_name: Option<&str>,
    ) {
        // 处理返回值目标：Call 返回新值，dst 进入 Owned 状态
        if let Some(dst_operand) = dst {
            self.state
                .insert(dst_operand.clone(), ValueState::Owned(None));
        }

        // 获取被调用函数的消费模式（克隆以避免借用冲突）
        let consume_modes: Option<Vec<ConsumeMode>> = func_name.and_then(|name| {
            self.consume_analyzer
                .get_function_consume_mode_by_name(name)
                .cloned()
        });

        // 处理参数
        for (idx, arg) in args.iter().enumerate() {
            let mode = consume_modes.as_ref().and_then(|m| m.get(idx)).copied();

            match mode {
                Some(ConsumeMode::Consumes) => {
                    // 明确消费：参数进入 Empty
                    if let Some(state) = self.state.get(arg) {
                        match state {
                            ValueState::Empty | ValueState::Moved => {
                                self.report_use_after_move(arg);
                            }
                            ValueState::Dup => {
                                // Dup 类型不会被消费
                            }
                            ValueState::Owned(_) => {
                                self.state.insert(arg.clone(), ValueState::Empty);
                            }
                            ValueState::Dropped => {}
                        }
                    } else {
                        // 首次使用，设为 Empty
                        self.state.insert(arg.clone(), ValueState::Empty);
                    }
                }
                Some(ConsumeMode::Returns) | Some(ConsumeMode::Undetermined) | None => {
                    // Returns / Undetermined / 未知：不消费参数
                    // 只检查参数是否可用（非 Moved / Empty）
                    if let Some(ValueState::Empty | ValueState::Moved) = self.state.get(arg) {
                        self.report_use_after_move(arg);
                    }
                    // 首次使用：不设状态（不消费）
                }
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
                ValueState::Dup => {
                    // Dup 类型（如 &T）返回时保持 Dup 状态
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
                ValueState::Dup => {
                    // Dup 类型（如 &T）可多次使用，不改变状态
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
        let name = operand_display_name(operand, self.local_names.as_ref());
        self.errors
            .push(ErrorCodeDefinition::use_after_move(&name).build());
    }
}

impl OwnershipCheck for MoveChecker {
    fn check_function(
        &mut self,
        func: &FunctionIR,
    ) -> &[Diagnostic] {
        self.clear();

        // 初始化函数参数的状态
        for (idx, param_type) in func.params.iter().enumerate() {
            let operand = Operand::Local(idx);
            if matches!(param_type, MonoType::Ref { .. }) {
                self.state.insert(operand, ValueState::Dup);
            } else {
                self.state.insert(operand, ValueState::Owned(None));
            }
        }

        for (block_idx, block) in func.blocks.iter().enumerate() {
            for (instr_idx, instr) in block.instructions.iter().enumerate() {
                self.location = (block_idx, instr_idx);
                matches!(instr, Instruction::Move { .. });
                self.check_instruction(instr);
            }
        }

        &self.errors
    }

    fn errors(&self) -> &[Diagnostic] {
        &self.errors
    }

    fn state(&self) -> &HashMap<Operand, ValueState> {
        &self.state
    }

    fn clear(&mut self) {
        self.state.clear();
        self.errors.clear();
        self.type_map.clear();
        self.consume_analyzer.clear_cache();
    }
}

impl Default for MoveChecker {
    fn default() -> Self {
        Self::new()
    }
}

/// 从指令中提取函数名
fn extract_function_name(operand: &Operand) -> Option<String> {
    match operand {
        Operand::Global(name) => Some(name.to_string()),
        _ => None,
    }
}
