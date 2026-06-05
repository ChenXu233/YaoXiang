//! 借用令牌冲突检测（Borrow Checker）
//!
//! 对借用令牌进行流敏感的活跃性分析，检测以下冲突：
//! - 同一来源的多个 `&T` 令牌：允许（Dup）
//! - `&mut T` 令牌活跃时，同一来源的 `&T` 令牌也活跃：错误
//! - `&mut T` 令牌被冻结后使用：错误
//! - 令牌来源已被移动后使用令牌：错误

use crate::middle::core::ir::{FunctionIR, Instruction, Operand};
use std::collections::HashMap;

/// 令牌状态
#[derive(Debug, Clone, PartialEq)]
pub enum TokenState {
    /// 令牌活跃可用
    Active,
    /// 令牌被冻结（来源的 &mut 被冻结以产生 &T）
    Frozen,
    /// 令牌已被移动/消耗
    Moved,
}

/// 借用令牌
#[derive(Debug, Clone)]
pub struct BorrowToken {
    /// 被借用的变量来源
    pub source: String,
    /// 是否为 &mut T 令牌
    pub mutable: bool,
    /// 令牌当前状态
    pub state: TokenState,
}

/// 借用错误类型
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BorrowError {
    /// 可变借用冲突：同一来源同时存在活跃的借用
    MutableBorrowConflict {
        /// 借用来源
        source: String,
        /// 已存在的借用令牌
        existing: String,
        /// 新的借用令牌
        new: String,
    },
    /// 移动后借用：来源已被移动后尝试使用借用令牌
    BorrowAfterMove {
        /// 借用来源
        source: String,
        /// 尝试使用的令牌
        token: String,
    },
    /// 冻结时使用：令牌被冻结后仍尝试使用
    UseWhileFrozen {
        /// 借用来源
        source: String,
        /// 被冻结的令牌
        token: String,
    },
}

impl std::fmt::Display for BorrowError {
    fn fmt(
        &self,
        f: &mut std::fmt::Formatter<'_>,
    ) -> std::fmt::Result {
        match self {
            BorrowError::MutableBorrowConflict {
                source,
                existing,
                new,
            } => {
                write!(
                    f,
                    "MutableBorrowConflict: cannot create mutable borrow '{}' for '{}' while '{}' is still active",
                    new, source, existing
                )
            }
            BorrowError::BorrowAfterMove { source, token } => {
                write!(
                    f,
                    "BorrowAfterMove: cannot use borrow token '{}' because source '{}' has been moved",
                    token, source
                )
            }
            BorrowError::UseWhileFrozen { source, token } => {
                write!(
                    f,
                    "UseWhileFrozen: cannot use borrow token '{}' because source '{}' is frozen",
                    token, source
                )
            }
        }
    }
}

/// 借用检查器
///
/// 追踪活跃的借用令牌并检测冲突：
/// - `&T` 令牌：不可变借用，允许多个同时存在
/// - `&mut T` 令牌：可变借用，同一来源只能有一个活跃
/// - 冻结机制：当 &mut 被冻结为 &T 时，&mut 进入 Frozen 状态
#[derive(Debug)]
pub struct BorrowChecker {
    /// 令牌表：令牌名 -> 令牌信息
    tokens: HashMap<String, BorrowToken>,
    /// 冻结来源映射：冻结的 &mut 令牌名 -> 源变量名
    /// 用于在 &T 令牌释放时解冻对应的 &mut 令牌
    frozen_sources: HashMap<String, String>,
    /// 收集的错误
    errors: Vec<BorrowError>,
    /// 当前检查位置 (block_idx, instr_idx)
    location: (usize, usize),
}

impl BorrowChecker {
    /// 创建新的借用检查器
    pub fn new() -> Self {
        Self {
            tokens: HashMap::new(),
            frozen_sources: HashMap::new(),
            errors: Vec::new(),
            location: (0, 0),
        }
    }

    /// 注册一个新的借用令牌
    ///
    /// 检查与同一来源已有借用的冲突：
    /// - 如果是可变借用：同一来源不能有其他活跃借用
    /// - 如果是不可变借用：同一来源不能有活跃的可变借用
    pub fn create_borrow(
        &mut self,
        token_name: &str,
        source: &str,
        mutable: bool,
    ) {
        // 检查与已有借用的冲突
        for (name, existing) in &self.tokens {
            if existing.source != source {
                continue;
            }
            if existing.state != TokenState::Active {
                continue;
            }

            if mutable && existing.mutable {
                // 可变借用冲突：已有可变借用
                self.errors.push(BorrowError::MutableBorrowConflict {
                    source: source.to_string(),
                    existing: name.clone(),
                    new: token_name.to_string(),
                });
                return;
            }
            if mutable && !existing.mutable {
                // 可变借用冲突：已有不可变借用
                self.errors.push(BorrowError::MutableBorrowConflict {
                    source: source.to_string(),
                    existing: name.clone(),
                    new: token_name.to_string(),
                });
                return;
            }
            if !mutable && existing.mutable {
                // 不可变借用冲突：已有可变借用
                self.errors.push(BorrowError::MutableBorrowConflict {
                    source: source.to_string(),
                    existing: name.clone(),
                    new: token_name.to_string(),
                });
                return;
            }
            // 不可变借用 + 不可变借用：允许（Dup）
        }

        self.tokens.insert(
            token_name.to_string(),
            BorrowToken {
                source: source.to_string(),
                mutable,
                state: TokenState::Active,
            },
        );
    }

    /// 标记令牌被使用（验证仍然活跃）
    ///
    /// 检查令牌是否处于 Active 状态（非 Frozen 或 Moved）
    pub fn use_token(
        &mut self,
        token_name: &str,
    ) {
        let token = match self.tokens.get(token_name) {
            Some(t) => t.clone(),
            None => return,
        };

        match token.state {
            TokenState::Active => {
                // 令牌活跃，正常使用
            }
            TokenState::Frozen => {
                self.errors.push(BorrowError::UseWhileFrozen {
                    source: token.source.clone(),
                    token: token_name.to_string(),
                });
            }
            TokenState::Moved => {
                self.errors.push(BorrowError::BorrowAfterMove {
                    source: token.source.clone(),
                    token: token_name.to_string(),
                });
            }
        }
    }

    /// 冻结 &mut 令牌以产生 &T 视图
    ///
    /// 将 &mut 令牌标记为 Frozen，并创建一个同源的 &T 令牌
    pub fn freeze(
        &mut self,
        mut_token_name: &str,
        frozen_token_name: &str,
    ) {
        let source = match self.tokens.get_mut(mut_token_name) {
            Some(token) if token.state == TokenState::Active && token.mutable => {
                token.state = TokenState::Frozen;
                token.source.clone()
            }
            _ => return,
        };

        // 记录冻结关系，用于后续解冻
        self.frozen_sources
            .insert(frozen_token_name.to_string(), mut_token_name.to_string());

        // 创建新的 &T 令牌
        self.tokens.insert(
            frozen_token_name.to_string(),
            BorrowToken {
                source,
                mutable: false,
                state: TokenState::Active,
            },
        );
    }

    /// 结束令牌的生命周期（当令牌离开作用域时）
    ///
    /// 如果释放的是从冻结的 &mut 派生的 &T，则解冻源 &mut 令牌
    pub fn release_token(
        &mut self,
        token_name: &str,
    ) {
        if let Some(token) = self.tokens.remove(token_name) {
            // 如果这个 &T 令牌是从冻结的 &mut 派生的，检查是否可以解冻
            if !token.mutable && self.frozen_sources.remove(token_name).is_some() {
                // 检查是否还有其他活跃的 &T 令牌来自同一冻结来源
                let source = &token.source;
                let has_other_active = self
                    .tokens
                    .values()
                    .any(|t| t.source == *source && !t.mutable && t.state == TokenState::Active);

                if !has_other_active {
                    // 没有其他活跃的 &T 令牌，解冻 &mut 令牌
                    // 找到同一来源的 Frozen &mut 令牌
                    for t in self.tokens.values_mut() {
                        if t.source == *source && t.mutable && t.state == TokenState::Frozen {
                            t.state = TokenState::Active;
                            break;
                        }
                    }
                }
            }
        }
    }

    /// 获取所有错误
    pub fn errors(&self) -> &[BorrowError] {
        &self.errors
    }

    /// 清除状态
    pub fn clear(&mut self) {
        self.tokens.clear();
        self.frozen_sources.clear();
        self.errors.clear();
    }

    /// 检查函数的借用语义
    pub fn check_function(
        &mut self,
        func: &FunctionIR,
    ) -> &[BorrowError] {
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
            // ShareRef: 创建不可变借用（&T）
            Instruction::ShareRef { dst, src } => {
                let token_name = operand_to_string(dst);
                let source = operand_to_string(src);
                self.create_borrow(&token_name, &source, false);
            }
            // Move: 令牌被移动，来源进入 Moved 状态
            Instruction::Move { dst, src } => {
                let src_name = operand_to_string(src);
                if self.tokens.contains_key(&src_name) {
                    // 令牌被移动
                    if let Some(token) = self.tokens.get_mut(&src_name) {
                        token.state = TokenState::Moved;
                    }
                    // dst 继承来源信息
                    if let Some(token) = self.tokens.get(&src_name) {
                        let source = token.source.clone();
                        let mutable = token.mutable;
                        let dst_name = operand_to_string(dst);
                        self.tokens.insert(
                            dst_name,
                            BorrowToken {
                                source,
                                mutable,
                                state: TokenState::Active,
                            },
                        );
                    }
                }
            }
            // Borrow: 创建借用令牌
            Instruction::Borrow { dst, src, mutable } => {
                let token_name = operand_to_string(dst);
                let source = operand_to_string(src);
                self.create_borrow(&token_name, &source, *mutable);
            }
            // Release: 释放借用令牌
            Instruction::Release(operand) => {
                let name = operand_to_string(operand);
                self.release_token(&name);
            }
            // Drop: 令牌生命周期结束
            Instruction::Drop(operand) => {
                let name = operand_to_string(operand);
                self.release_token(&name);
            }
            // 使用令牌的指令
            Instruction::Load { src, .. }
            | Instruction::Neg { src, .. }
            | Instruction::Cast { src, .. } => {
                let name = operand_to_string(src);
                if self.tokens.contains_key(&name) {
                    self.use_token(&name);
                }
            }
            Instruction::LoadIndex { src, index, .. } => {
                let name = operand_to_string(src);
                if self.tokens.contains_key(&name) {
                    self.use_token(&name);
                }
                let idx_name = operand_to_string(index);
                if self.tokens.contains_key(&idx_name) {
                    self.use_token(&idx_name);
                }
            }
            Instruction::LoadField { src, .. } => {
                let name = operand_to_string(src);
                if self.tokens.contains_key(&name) {
                    self.use_token(&name);
                }
            }
            Instruction::Store { src, dst, .. } => {
                let src_name = operand_to_string(src);
                if self.tokens.contains_key(&src_name) {
                    self.use_token(&src_name);
                }
                let dst_name = operand_to_string(dst);
                if self.tokens.contains_key(&dst_name) {
                    self.use_token(&dst_name);
                }
            }
            Instruction::StoreField { src, dst, .. } => {
                let src_name = operand_to_string(src);
                if self.tokens.contains_key(&src_name) {
                    self.use_token(&src_name);
                }
                let dst_name = operand_to_string(dst);
                if self.tokens.contains_key(&dst_name) {
                    self.use_token(&dst_name);
                }
            }
            Instruction::StoreIndex {
                src, dst, index, ..
            } => {
                let src_name = operand_to_string(src);
                if self.tokens.contains_key(&src_name) {
                    self.use_token(&src_name);
                }
                let dst_name = operand_to_string(dst);
                if self.tokens.contains_key(&dst_name) {
                    self.use_token(&dst_name);
                }
                let idx_name = operand_to_string(index);
                if self.tokens.contains_key(&idx_name) {
                    self.use_token(&idx_name);
                }
            }
            Instruction::Ret(Some(value)) => {
                let name = operand_to_string(value);
                if self.tokens.contains_key(&name) {
                    self.use_token(&name);
                }
            }
            Instruction::Call { args, .. } => {
                for arg in args {
                    let name = operand_to_string(arg);
                    if self.tokens.contains_key(&name) {
                        self.use_token(&name);
                    }
                }
            }
            Instruction::CallVirt { obj, args, .. } => {
                let obj_name = operand_to_string(obj);
                if self.tokens.contains_key(&obj_name) {
                    self.use_token(&obj_name);
                }
                for arg in args {
                    let name = operand_to_string(arg);
                    if self.tokens.contains_key(&name) {
                        self.use_token(&name);
                    }
                }
            }
            _ => {}
        }
    }

    /// 将 BorrowError 转换为 OwnershipError
    pub fn to_ownership_errors(
        &self
    ) -> Vec<crate::middle::passes::lifetime::error::OwnershipError> {
        use crate::middle::passes::lifetime::error::OwnershipError;
        self.errors
            .iter()
            .map(|e| match e {
                BorrowError::MutableBorrowConflict {
                    source,
                    existing,
                    new,
                } => OwnershipError::MutableBorrowConflict {
                    source: source.clone(),
                    existing: existing.clone(),
                    new: new.clone(),
                    location: self.location,
                },
                BorrowError::BorrowAfterMove { source, token } => OwnershipError::BorrowAfterMove {
                    source: source.clone(),
                    token: token.clone(),
                    location: self.location,
                },
                BorrowError::UseWhileFrozen { source, token } => OwnershipError::UseWhileFrozen {
                    source: source.clone(),
                    token: token.clone(),
                    location: self.location,
                },
            })
            .collect()
    }
}

impl Default for BorrowChecker {
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
    //! 借用检查器单元与端到端测试
    //!
    //! 参考规范：RFC-009 v9 §4.1 借用令牌冲突检测。
    //! 覆盖不可变/可变借用冲突、冻结/解冻、移动后使用等场景。

    use super::*;
    use crate::middle::core::ir::BasicBlock;
    use crate::frontend::core::typecheck::MonoType;

    /// 辅助函数：构建包含给定指令的 FunctionIR
    fn make_func_ir(instructions: Vec<Instruction>) -> FunctionIR {
        FunctionIR {
            name: "test_fn".to_string(),
            params: vec![],
            return_type: MonoType::Void,
            locals: vec![],
            blocks: vec![BasicBlock {
                label: 0,
                instructions,
                successors: vec![],
            }],
            entry: 0,
        }
    }

    /// 辅助函数：创建新的借用检查器
    fn make_checker() -> BorrowChecker {
        BorrowChecker::new()
    }

    /// 辅助函数：对给定指令列表运行端到端借用检查
    fn run_borrow_check(instructions: Vec<Instruction>) -> Vec<BorrowError> {
        let func = make_func_ir(instructions);
        let mut checker = BorrowChecker::new();
        checker.check_function(&func).to_vec()
    }

    #[test]
    fn test_multiple_immutable_borrows() {
        // Arrange
        let mut checker = make_checker();
        // Act
        checker.create_borrow("ref_a", "x", false);
        checker.create_borrow("ref_b", "x", false);
        // Assert
        assert!(
            checker.errors().is_empty(),
            "同一来源的多个不可变借用应允许，但得到错误: {:?}",
            checker.errors()
        );
    }

    #[test]
    fn test_mutable_borrow_conflict_with_immutable() {
        // Arrange
        let mut checker = make_checker();
        // Act
        checker.create_borrow("ref_a", "x", false);
        checker.create_borrow("ref_mut_b", "x", true);
        // Assert
        assert_eq!(
            checker.errors().len(),
            1,
            "不可变借用后创建可变借用应产生 1 个错误，但得到: {:?}",
            checker.errors()
        );
        assert!(
            matches!(
                checker.errors()[0],
                BorrowError::MutableBorrowConflict { .. }
            ),
            "错误类型应为 MutableBorrowConflict，但得到: {:?}",
            checker.errors()[0]
        );
    }

    #[test]
    fn test_mutable_borrow_conflict_with_mutable() {
        // Arrange
        let mut checker = make_checker();
        // Act
        checker.create_borrow("ref_mut_a", "x", true);
        checker.create_borrow("ref_mut_b", "x", true);
        // Assert
        assert_eq!(
            checker.errors().len(),
            1,
            "同一来源的两个可变借用应产生 1 个错误，但得到: {:?}",
            checker.errors()
        );
        assert!(
            matches!(
                checker.errors()[0],
                BorrowError::MutableBorrowConflict { .. }
            ),
            "错误类型应为 MutableBorrowConflict，但得到: {:?}",
            checker.errors()[0]
        );
    }

    #[test]
    fn test_immutable_borrow_conflict_with_mutable() {
        // Arrange
        let mut checker = make_checker();
        // Act
        checker.create_borrow("ref_mut_a", "x", true);
        checker.create_borrow("ref_b", "x", false);
        // Assert
        assert_eq!(
            checker.errors().len(),
            1,
            "可变借用后创建不可变借用应产生 1 个错误，但得到: {:?}",
            checker.errors()
        );
        assert!(
            matches!(
                checker.errors()[0],
                BorrowError::MutableBorrowConflict { .. }
            ),
            "错误类型应为 MutableBorrowConflict，但得到: {:?}",
            checker.errors()[0]
        );
    }

    #[test]
    fn test_use_active_token() {
        // Arrange
        let mut checker = make_checker();
        checker.create_borrow("ref_a", "x", false);
        // Act
        checker.use_token("ref_a");
        // Assert
        assert!(
            checker.errors().is_empty(),
            "使用活跃令牌不应产生错误，但得到: {:?}",
            checker.errors()
        );
    }

    #[test]
    fn test_use_frozen_token() {
        // Arrange
        let mut checker = make_checker();
        checker.create_borrow("ref_mut_a", "x", true);
        checker.freeze("ref_mut_a", "ref_b");
        // Act
        checker.use_token("ref_mut_a");
        // Assert
        assert_eq!(
            checker.errors().len(),
            1,
            "使用冻结令牌应产生 1 个错误，但得到: {:?}",
            checker.errors()
        );
        assert!(
            matches!(checker.errors()[0], BorrowError::UseWhileFrozen { .. }),
            "错误类型应为 UseWhileFrozen，但得到: {:?}",
            checker.errors()[0]
        );
    }

    #[test]
    fn test_use_moved_token() {
        // Arrange
        let mut checker = make_checker();
        checker.create_borrow("ref_a", "x", false);
        if let Some(token) = checker.tokens.get_mut("ref_a") {
            token.state = TokenState::Moved;
        }
        // Act
        checker.use_token("ref_a");
        // Assert
        assert_eq!(
            checker.errors().len(),
            1,
            "使用已移动令牌应产生 1 个错误，但得到: {:?}",
            checker.errors()
        );
        assert!(
            matches!(checker.errors()[0], BorrowError::BorrowAfterMove { .. }),
            "错误类型应为 BorrowAfterMove，但得到: {:?}",
            checker.errors()[0]
        );
    }

    #[test]
    fn test_freeze_and_unfreeze() {
        // Arrange
        let mut checker = make_checker();
        checker.create_borrow("ref_mut_a", "x", true);
        // Act: 冻结为不可变借用
        checker.freeze("ref_mut_a", "ref_b");
        // Assert: ref_mut_a 应该是 Frozen
        assert_eq!(
            checker.tokens.get("ref_mut_a").unwrap().state,
            TokenState::Frozen,
            "冻结后 ref_mut_a 应处于 Frozen 状态"
        );
        // Assert: ref_b 应该是 Active
        assert_eq!(
            checker.tokens.get("ref_b").unwrap().state,
            TokenState::Active,
            "新建的 ref_b 应处于 Active 状态"
        );
        // Act: 释放 ref_b，应解冻 ref_mut_a
        checker.release_token("ref_b");
        // Assert
        assert_eq!(
            checker.tokens.get("ref_mut_a").unwrap().state,
            TokenState::Active,
            "释放 ref_b 后 ref_mut_a 应恢复为 Active 状态"
        );
    }

    #[test]
    fn test_freeze_multiple_immutable() {
        // Arrange
        let mut checker = make_checker();
        checker.create_borrow("ref_mut_a", "x", true);
        checker.freeze("ref_mut_a", "ref_b");
        // Act: 第二次冻结应失败（ref_mut_a 已经是 Frozen）
        checker.freeze("ref_mut_a", "ref_c");
        // Assert: ref_mut_a 仍应为 Frozen
        assert_eq!(
            checker.tokens.get("ref_mut_a").unwrap().state,
            TokenState::Frozen,
            "第二次冻结后 ref_mut_a 应仍为 Frozen"
        );
        // Assert: ref_c 不应被创建
        assert!(
            !checker.tokens.contains_key("ref_c"),
            "对已 Frozen 的令牌再次冻结不应创建新令牌 ref_c"
        );
    }

    #[test]
    fn test_different_sources_no_conflict() {
        // Arrange
        let mut checker = make_checker();
        // Act
        checker.create_borrow("ref_a", "x", true);
        checker.create_borrow("ref_b", "y", true);
        // Assert
        assert!(
            checker.errors().is_empty(),
            "不同来源的借用不应冲突，但得到错误: {:?}",
            checker.errors()
        );
    }

    #[test]
    fn test_release_nonexistent_token() {
        // Arrange
        let mut checker = make_checker();
        // Act
        checker.release_token("nonexistent");
        // Assert
        assert!(
            checker.errors().is_empty(),
            "释放不存在的令牌不应产生错误，但得到: {:?}",
            checker.errors()
        );
    }

    // ===== 端到端测试：通过 check_function + Instruction::Borrow/Release =====

    #[test]
    fn test_e2e_single_immutable_borrow() {
        // Arrange + Act
        let errors = run_borrow_check(vec![Instruction::Borrow {
            dst: Operand::Temp(0),
            src: Operand::Local(0),
            mutable: false,
        }]);
        // Assert
        assert!(
            errors.is_empty(),
            "单个不可变借用不应产生错误，但得到: {:?}",
            errors
        );
    }

    #[test]
    fn test_e2e_multiple_immutable_borrows() {
        // Arrange + Act
        let errors = run_borrow_check(vec![
            Instruction::Borrow {
                dst: Operand::Temp(0),
                src: Operand::Local(0),
                mutable: false,
            },
            Instruction::Borrow {
                dst: Operand::Temp(1),
                src: Operand::Local(0),
                mutable: false,
            },
        ]);
        // Assert
        assert!(
            errors.is_empty(),
            "多个不可变借用不应产生错误，但得到: {:?}",
            errors
        );
    }

    #[test]
    fn test_e2e_mutable_then_immutable_conflict() {
        // Arrange + Act
        let errors = run_borrow_check(vec![
            Instruction::Borrow {
                dst: Operand::Temp(0),
                src: Operand::Local(0),
                mutable: true,
            },
            Instruction::Borrow {
                dst: Operand::Temp(1),
                src: Operand::Local(0),
                mutable: false,
            },
        ]);
        // Assert
        assert_eq!(
            errors.len(),
            1,
            "可变借用后不可变借用应产生 1 个错误，但得到: {:?}",
            errors
        );
        assert!(
            matches!(errors[0], BorrowError::MutableBorrowConflict { .. }),
            "错误类型应为 MutableBorrowConflict，但得到: {:?}",
            errors[0]
        );
    }

    #[test]
    fn test_e2e_mutable_then_mutable_conflict() {
        // Arrange + Act
        let errors = run_borrow_check(vec![
            Instruction::Borrow {
                dst: Operand::Temp(0),
                src: Operand::Local(0),
                mutable: true,
            },
            Instruction::Borrow {
                dst: Operand::Temp(1),
                src: Operand::Local(0),
                mutable: true,
            },
        ]);
        // Assert
        assert_eq!(
            errors.len(),
            1,
            "两个可变借用应产生 1 个错误，但得到: {:?}",
            errors
        );
        assert!(
            matches!(errors[0], BorrowError::MutableBorrowConflict { .. }),
            "错误类型应为 MutableBorrowConflict，但得到: {:?}",
            errors[0]
        );
    }

    #[test]
    fn test_e2e_mutable_release_reborrow() {
        // Arrange + Act
        let errors = run_borrow_check(vec![
            Instruction::Borrow {
                dst: Operand::Temp(0),
                src: Operand::Local(0),
                mutable: true,
            },
            Instruction::Release(Operand::Temp(0)),
            Instruction::Borrow {
                dst: Operand::Temp(1),
                src: Operand::Local(0),
                mutable: true,
            },
        ]);
        // Assert
        assert!(
            errors.is_empty(),
            "释放后重新借用不应产生错误，但得到: {:?}",
            errors
        );
    }
}
