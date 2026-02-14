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
use super::error::{OwnershipCheck, OwnershipError, TypeId, ValueState, operand_to_string};
use super::ownership_flow::ConsumeMode;
use crate::middle::core::ir::{FunctionIR, Instruction, Operand};
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
    pub errors: Vec<OwnershipError>,
    pub location: (usize, usize),
    /// 类型表：变量 -> 类型ID
    type_map: HashMap<Operand, TypeId>,
    /// 函数类型表：类型名 -> 类型ID（外部传入）
    type_table: Option<HashMap<String, TypeId>>,
    /// 消费分析器（Phase 4）
    consume_analyzer: ConsumeAnalyzer,
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
            Instruction::Call {
                dst, args, func, ..
            } => {
                let func_name = extract_function_name(func);
                self.check_call(args, dst.as_ref(), func_name.as_deref());
            }
            Instruction::Ret(Some(value)) => self.check_ret(value),
            Instruction::Store { dst, src, .. } => self.check_store(dst, src),
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
        // 处理返回值目标
        if let Some(dst_operand) = dst {
            // 返回值赋值给 dst，dst 进入 Owned 状态
            // 注意：这里不检查返回值是否被"移动"，因为 Call 返回的是一个新值
            self.state.insert(dst_operand.clone(), ValueState::Empty);
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
                Some(ConsumeMode::Returns) => {
                    // Returns 模式：参数所有权回流，保持 Owned 状态
                    // 不修改参数状态
                }
                Some(ConsumeMode::Consumes) | Some(ConsumeMode::Undetermined) | None => {
                    // Consumes/Undetermined/未知：保守处理，参数进入 Empty
                    if let Some(state) = self.state.get(arg) {
                        match state {
                            ValueState::Empty => {
                                self.report_use_after_move(arg);
                            }
                            ValueState::Owned(_) => {
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::middle::core::ir::{BasicBlock, ConstValue};
    use crate::frontend::typecheck::MonoType;

    fn make_test_function() -> FunctionIR {
        FunctionIR {
            name: "returns_param".to_string(),
            params: vec![MonoType::Int(0)],
            return_type: MonoType::Int(0),
            is_async: false,
            locals: vec![],
            blocks: vec![BasicBlock {
                label: 0,
                instructions: vec![Instruction::Ret(Some(Operand::Arg(0)))],
                successors: vec![],
            }],
            entry: 0,
        }
    }

    #[test]
    fn test_returns_mode_preserves_param() {
        let mut checker = MoveChecker::new();
        let func = make_test_function();

        // 分析函数消费模式
        checker.consume_analyzer.analyze_and_cache(&func);

        // 参数是 Returns 模式，调用后应该保持 Owned
        let modes = checker.consume_analyzer.get_function_consume_mode(&func);
        assert_eq!(modes.len(), 1);
        assert_eq!(modes[0], ConsumeMode::Returns);
    }

    #[test]
    fn test_consumes_mode_empties_param() {
        // 测试 Consumes 模式的函数调用
        let mut checker = MoveChecker::new();

        let func = FunctionIR {
            name: "consumes_param".to_string(),
            params: vec![MonoType::Int(0)],
            return_type: MonoType::Void,
            is_async: false,
            locals: vec![],
            blocks: vec![BasicBlock {
                label: 0,
                instructions: vec![Instruction::Ret(None)],
                successors: vec![],
            }],
            entry: 0,
        };

        checker.consume_analyzer.analyze_and_cache(&func);

        let modes = checker.consume_analyzer.get_function_consume_mode(&func);
        assert_eq!(modes.len(), 1);
        assert_eq!(modes[0], ConsumeMode::Consumes);
    }

    #[test]
    fn test_move_checker_state_tracking() {
        // 测试 MoveChecker 状态追踪
        let mut checker = MoveChecker::new();

        let func = FunctionIR {
            name: "test".to_string(),
            params: vec![MonoType::Int(0)],
            return_type: MonoType::Int(0),
            is_async: false,
            locals: vec![],
            blocks: vec![BasicBlock {
                label: 0,
                instructions: vec![
                    Instruction::Move {
                        dst: Operand::Temp(0),
                        src: Operand::Arg(0),
                    },
                    Instruction::Ret(Some(Operand::Temp(0))),
                ],
                successors: vec![],
            }],
            entry: 0,
        };

        let errors = checker.check_function(&func);

        // 不应该有错误
        assert!(errors.is_empty());
    }

    #[test]
    fn test_use_after_move_detection() {
        // 测试 UseAfterMove 检测
        let mut checker = MoveChecker::new();

        let func = FunctionIR {
            name: "test".to_string(),
            params: vec![MonoType::Int(0)],
            return_type: MonoType::Int(0),
            is_async: false,
            locals: vec![],
            blocks: vec![BasicBlock {
                label: 0,
                instructions: vec![
                    Instruction::Move {
                        dst: Operand::Temp(0),
                        src: Operand::Arg(0),
                    },
                    // 再次使用 Arg(0) 应该报错
                    Instruction::Add {
                        dst: Operand::Temp(1),
                        lhs: Operand::Arg(0),
                        rhs: Operand::Const(ConstValue::Int(1)),
                    },
                ],
                successors: vec![],
            }],
            entry: 0,
        };

        let errors = checker.check_function(&func);

        // 应该有 UseAfterMove 错误
        let has_use_after_move = errors
            .iter()
            .any(|e| matches!(e, OwnershipError::UseAfterMove { .. }));
        assert!(has_use_after_move);
    }

    #[test]
    fn test_checker_clear() {
        // 测试 MoveChecker 清除状态
        let mut checker = MoveChecker::new();

        let func = FunctionIR {
            name: "test".to_string(),
            params: vec![MonoType::Int(0)],
            return_type: MonoType::Int(0),
            is_async: false,
            locals: vec![],
            blocks: vec![BasicBlock {
                label: 0,
                instructions: vec![Instruction::Ret(Some(Operand::Arg(0)))],
                successors: vec![],
            }],
            entry: 0,
        };

        // 第一次检查
        checker.check_function(&func);

        // 清除状态
        checker.clear();

        // 再次检查应该正常
        let errors = checker.check_function(&func);
        assert!(errors.is_empty());
    }

    #[test]
    fn test_multiple_params_consume_modes() {
        // 测试多参数的消费模式
        let mut checker = MoveChecker::new();

        let func = FunctionIR {
            name: "multi_param".to_string(),
            params: vec![MonoType::Int(0), MonoType::Int(0)],
            return_type: MonoType::Int(0),
            is_async: false,
            locals: vec![],
            blocks: vec![BasicBlock {
                label: 0,
                instructions: vec![Instruction::Ret(Some(Operand::Arg(0)))],
                successors: vec![],
            }],
            entry: 0,
        };

        checker.consume_analyzer.analyze_and_cache(&func);

        let modes = checker.consume_analyzer.get_function_consume_mode(&func);
        assert_eq!(modes.len(), 2);
        assert_eq!(modes[0], ConsumeMode::Returns);
        assert_eq!(modes[1], ConsumeMode::Consumes);
    }
}
