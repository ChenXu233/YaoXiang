//! 空状态追踪器
//!
//! 追踪变量的空状态（Empty State），实现 Move 后变量可重新赋值的功能。
//!
//! # 设计原理
//!
//! 在 Move 语义中，变量被移动后进入空状态（Empty），此时可以重新赋值复用变量名。
//! 空状态追踪器负责：
//! 1. 记录每个变量的当前状态
//! 2. 追踪状态变化（Move、Store、Drop）
//! 3. 检查重新赋值时的类型一致性
//!
//! # 状态语义
//!
//! | 状态 | 含义 | 可执行操作 |
//! |------|------|------------|
//! | Owned | 值有效，所有者可用 | 使用、Move、Store（报错） |
//! | Moved | 值已被移动 | Store（进入 Empty）、无法使用 |
//! | Empty | 空状态，可重新赋值 | Store（检查类型后覆盖） |
//! | Dropped | 值已被释放 | 同 Moved |

use super::error::{OwnershipError, TypeId, ValueState};
use crate::middle::core::ir::{FunctionIR, Instruction, Operand};
use std::collections::HashMap;

/// 空状态追踪器
///
/// 追踪变量的所有权状态，支持：
/// - 状态转换追踪（Owned → Moved → Empty → Owned）
/// - 类型一致性检查
/// - Store 时的状态验证
///
/// # 示例
///
/// ```ignore
/// let p = Point(1.0, 2.0);           // p: Owned(Point)
/// let p2 = p;                         // p: Moved, p2: Owned(Point)
/// p = Point(3.0, 4.0);               // p: Empty → Owned(Point)，类型一致
/// ```
#[derive(Debug, Clone)]
pub struct EmptyStateTracker {
    /// 变量状态追踪：Operand -> ValueState
    state: HashMap<Operand, ValueState>,
    /// 类型表：变量 -> 类型ID（用于重赋值检查）
    type_map: HashMap<Operand, TypeId>,
    /// 错误列表
    errors: Vec<OwnershipError>,
    /// 当前检查位置
    location: (usize, usize),
    /// 函数类型表：类型名 -> 类型ID（外部传入）
    type_table: Option<HashMap<String, TypeId>>,
}

impl EmptyStateTracker {
    /// 创建新的空状态追踪器
    pub fn new() -> Self {
        Self {
            state: HashMap::new(),
            type_map: HashMap::new(),
            errors: Vec::new(),
            location: (0, 0),
            type_table: None,
        }
    }

    /// 设置类型表（用于查询类型信息）
    pub fn with_type_table(
        mut self,
        type_table: HashMap<String, TypeId>,
    ) -> Self {
        self.type_table = Some(type_table);
        self
    }

    /// 获取当前状态
    pub fn state(&self) -> &HashMap<Operand, ValueState> {
        &self.state
    }

    /// 获取类型映射
    pub fn type_map(&self) -> &HashMap<Operand, TypeId> {
        &self.type_map
    }

    /// 获取错误列表
    pub fn errors(&self) -> &[OwnershipError] {
        &self.errors
    }

    /// 清除状态
    pub fn clear(&mut self) {
        self.state.clear();
        self.type_map.clear();
        self.errors.clear();
        self.location = (0, 0);
    }

    /// 检查函数并返回错误
    pub fn check_function(
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

    /// 检查单条指令
    fn check_instruction(
        &mut self,
        instr: &Instruction,
    ) {
        match instr {
            // Move: dst = src，src 进入 Moved，dst 进入 Owned
            Instruction::Move { dst, src } => self.check_move(dst, src),

            // Store: dst = src（重新赋值）
            Instruction::Store { dst, src } => self.check_store(dst, src),

            // StoreField: dst.field = src（字段赋值，不影响变量状态）
            // 字段的可变性检查在 MutChecker 中进行
            Instruction::StoreField { .. } => {}

            // StoreIndex: dst[index] = src（索引赋值，不影响变量状态）
            Instruction::StoreIndex { .. } => {}

            // Call: 函数调用，参数进入 Moved
            Instruction::Call { args, .. } => self.check_call(args),

            // Ret: 返回值处理
            Instruction::Ret(Some(v)) => self.check_ret(v),
            Instruction::Ret(None) => {}

            // Load/LoadIndex/LoadField: 读取值，检查是否已被 Move
            Instruction::Load { src, .. }
            | Instruction::LoadIndex { src, .. }
            | Instruction::LoadField { src, .. } => self.check_used(src),

            // 一元/二元运算：操作数被使用
            Instruction::Neg { src, .. } | Instruction::Cast { src, .. } => self.check_used(src),
            Instruction::Add { lhs, rhs, .. }
            | Instruction::Sub { lhs, rhs, .. }
            | Instruction::Mul { lhs, rhs, .. }
            | Instruction::Div { lhs, rhs, .. }
            | Instruction::Mod { lhs, rhs, .. }
            | Instruction::And { lhs, rhs, .. }
            | Instruction::Or { lhs, rhs, .. }
            | Instruction::Xor { lhs, rhs, .. }
            | Instruction::Shl { lhs, rhs, .. }
            | Instruction::Shr { lhs, rhs, .. }
            | Instruction::Sar { lhs, rhs, .. }
            | Instruction::Eq { lhs, rhs, .. }
            | Instruction::Ne { lhs, rhs, .. }
            | Instruction::Lt { lhs, rhs, .. }
            | Instruction::Le { lhs, rhs, .. }
            | Instruction::Gt { lhs, rhs, .. }
            | Instruction::Ge { lhs, rhs, .. } => {
                self.check_used(lhs);
                self.check_used(rhs);
            }

            // 比较运算
            Instruction::StringConcat { lhs, rhs, .. } => {
                self.check_used(lhs);
                self.check_used(rhs);
            }

            _ => {}
        }
    }

    /// 检查 Move 操作
    ///
    /// 规则：
    /// - src 状态变为 Empty（可以直接重新赋值）
    /// - dst 状态变为 Owned(类型)
    fn check_move(
        &mut self,
        dst: &Operand,
        src: &Operand,
    ) {
        // 检查 src 是否已被使用
        if let Some(state) = self.state.get(src) {
            match state {
                ValueState::Empty => {
                    // 空状态变量的 Move 操作是合法的
                    // 保持 Empty 状态，等待重新赋值
                }
                ValueState::Owned(_) => {
                    // 正常情况：src 变为 Empty
                    self.state.insert(src.clone(), ValueState::Empty);
                    // 继承类型信息
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

        // dst 变为 Owned，尝试从 src 获取类型
        let dst_type = self.type_map.get(src).cloned();
        self.state
            .insert(dst.clone(), ValueState::Owned(dst_type.clone()));
        if let Some(t) = dst_type {
            self.type_map.insert(dst.clone(), t);
        }
    }

    /// 检查 Store 操作（重新赋值）
    ///
    /// 规则：
    /// - dst 必须是 Empty 状态才能重新赋值
    /// - 重新赋值的类型必须与之前的类型一致
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
                    // Moved 状态需要先进入 Empty 才能赋值
                    // 这是一个语义问题：如何从 Moved 变为 Empty？
                    // 答案：使用被移动的值本身就是错误的，但我们允许重新赋值
                    // 重新赋值会使变量进入新状态
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
                    // 这应该是 DropChecker 的职责
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
    /// 函数调用会消费参数，使参数进入 Empty 状态
    fn check_call(
        &mut self,
        args: &[Operand],
    ) {
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
                    ValueState::Dropped => {}
                }
            } else {
                self.state.insert(arg.clone(), ValueState::Empty);
            }
        }
    }

    /// 检查返回值
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
                    // 已移动的值被返回是合法的
                }
                ValueState::Dropped => {}
            }
        }
    }

    /// 检查值是否已被使用
    ///
    /// 如果值处于 Moved 或 Empty 状态，报错
    fn check_used(
        &mut self,
        operand: &Operand,
    ) {
        if let Some(state) = self.state.get(operand) {
            match state {
                ValueState::Moved | ValueState::Empty => {
                    self.report_use_after_move(operand);
                }
                ValueState::Owned(_) | ValueState::Dropped => {
                    // 正常使用
                }
            }
        }
    }

    /// 报告 UseAfterMove 错误
    fn report_use_after_move(
        &mut self,
        operand: &Operand,
    ) {
        self.errors.push(OwnershipError::UseAfterMove {
            value: operand_to_string(operand),
            location: self.location,
        });
    }

    /// 手动设置变量为空状态
    ///
    /// 用于控制流分析中，某个分支将变量设为空状态
    pub fn set_empty(
        &mut self,
        operand: Operand,
    ) {
        self.state.insert(operand, ValueState::Empty);
    }

    /// 手动设置变量为已拥有状态
    ///
    /// 用于控制流分析中，某个分支将变量赋值
    pub fn set_owned(
        &mut self,
        operand: Operand,
        type_id: Option<TypeId>,
    ) {
        self.state
            .insert(operand.clone(), ValueState::Owned(type_id.clone()));
        if let Some(t) = type_id {
            self.type_map.insert(operand, t);
        }
    }

    /// 获取变量的当前状态
    pub fn get_state(
        &self,
        operand: &Operand,
    ) -> Option<&ValueState> {
        self.state.get(operand)
    }

    /// 获取变量的类型
    pub fn get_type(
        &self,
        operand: &Operand,
    ) -> Option<&TypeId> {
        self.type_map.get(operand)
    }

    /// 设置变量的类型
    pub fn set_type(
        &mut self,
        operand: Operand,
        type_id: TypeId,
    ) {
        self.type_map.insert(operand, type_id);
    }

    /// 合并两个状态（用于分支汇合）
    ///
    /// 规则（保守分析）：
    /// - 任一为 Empty，则汇合后为 Empty
    /// - 任一为 Moved，则汇合后为 Moved
    /// - 都是 Owned，保留第一个（需要 SSA 才能精确追踪）
    pub fn merge_states(
        &self,
        state1: &ValueState,
        state2: &ValueState,
    ) -> ValueState {
        match (state1, state2) {
            // 任一为 Empty，则汇合后为 Empty
            (ValueState::Empty, _) | (_, ValueState::Empty) => ValueState::Empty,

            // 任一为 Moved，则汇合后为 Moved
            (ValueState::Moved, _) | (_, ValueState::Moved) => ValueState::Moved,

            // 都是 Owned，保留第一个（需要 SSA 才能精确追踪）
            (ValueState::Owned(t1), ValueState::Owned(t2)) => {
                // 类型一致则保留，不一致则无法确定
                if t1 == t2 {
                    state1.clone()
                } else {
                    // 类型不一致，保守起见返回 Owned
                    ValueState::Owned(t1.clone())
                }
            }

            // Dropped 与任何状态合并
            (ValueState::Dropped, s) | (s, ValueState::Dropped) => s.clone(),
        }
    }
}

impl Default for EmptyStateTracker {
    fn default() -> Self {
        Self::new()
    }
}

/// 将 Operand 转换为字符串标识
fn operand_to_string(operand: &Operand) -> String {
    match operand {
        Operand::Local(idx) => format!("local_{}", idx),
        Operand::Arg(idx) => format!("arg_{}", idx),
        Operand::Temp(idx) => format!("temp_{}", idx),
        Operand::Global(idx) => format!("global_{}", idx),
        Operand::Const(c) => format!("const_{:?}", c),
        Operand::Label(idx) => format!("label_{}", idx),
        Operand::Register(idx) => format!("reg_{}", idx),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_state_merge() {
        let tracker = EmptyStateTracker::new();

        // Empty + Owned = Empty（任一为 Empty 则为 Empty）
        assert_eq!(
            tracker.merge_states(&ValueState::Empty, &ValueState::Owned(None)),
            ValueState::Empty
        );

        // Moved + Owned = Moved（任一为 Moved 则为 Moved）
        assert_eq!(
            tracker.merge_states(&ValueState::Moved, &ValueState::Owned(None)),
            ValueState::Moved
        );

        // Empty + Moved = Empty（Empty 优先级更高）
        assert_eq!(
            tracker.merge_states(&ValueState::Empty, &ValueState::Moved),
            ValueState::Empty
        );

        // Moved + Moved = Moved
        assert_eq!(
            tracker.merge_states(&ValueState::Moved, &ValueState::Moved),
            ValueState::Moved
        );

        // Empty + Empty = Empty
        assert_eq!(
            tracker.merge_states(&ValueState::Empty, &ValueState::Empty),
            ValueState::Empty
        );
    }
}
