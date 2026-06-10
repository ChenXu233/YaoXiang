//! 借用令牌冲突检测（Borrow Checker）
//!
//! 对借用令牌进行流敏感的活跃性分析，检测以下冲突：
//! - 同一来源的多个 `&T` 令牌：允许（Dup）
//! - `&mut T` 令牌活跃时，同一来源的 `&T` 令牌也活跃：错误
//! - 令牌来源已被移动后使用令牌：错误

use crate::middle::core::ir::{FunctionIR, Instruction};
use crate::util::diagnostic::{Diagnostic, ErrorCodeDefinition};
use super::error::operand_display_name;
use std::collections::HashMap;

/// 令牌状态
#[derive(Debug, Clone, PartialEq)]
pub enum TokenState {
    /// 令牌活跃可用
    Active,
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

/// 借用检查器
///
/// 追踪活跃的借用令牌并检测冲突：
/// - `&T` 令牌：不可变借用，允许多个同时存在
/// - `&mut T` 令牌：可变借用，同一来源只能有一个活跃
#[derive(Debug)]
pub struct BorrowChecker {
    /// 令牌表：令牌名 -> 令牌信息
    tokens: HashMap<String, BorrowToken>,
    /// 收集的错误
    errors: Vec<Diagnostic>,
    /// 当前检查位置 (block_idx, instr_idx)
    location: (usize, usize),
    /// 局部变量名列表（用于错误报告中显示源码变量名）
    local_names: Option<Vec<String>>,
}

impl BorrowChecker {
    /// 创建新的借用检查器
    pub fn new() -> Self {
        Self {
            tokens: HashMap::new(),
            errors: Vec::new(),
            location: (0, 0),
            local_names: None,
        }
    }

    /// 设置局部变量名列表（用于生成友好的错误信息）
    pub fn set_local_names(
        &mut self,
        local_names: Option<Vec<String>>,
    ) {
        self.local_names = local_names;
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
        for existing in self.tokens.values() {
            if existing.source != source {
                continue;
            }
            if existing.state != TokenState::Active {
                continue;
            }

            if mutable && existing.mutable {
                self.errors
                    .push(ErrorCodeDefinition::mutable_borrow_conflict(source).build());
                return;
            }
            if mutable && !existing.mutable {
                self.errors
                    .push(ErrorCodeDefinition::mutable_borrow_conflict(source).build());
                return;
            }
            if !mutable && existing.mutable {
                self.errors
                    .push(ErrorCodeDefinition::mutable_borrow_conflict(source).build());
                return;
            }
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
    pub fn use_token(
        &mut self,
        token_name: &str,
    ) {
        let token = match self.tokens.get(token_name) {
            Some(t) => t.clone(),
            None => return,
        };

        match token.state {
            TokenState::Active => {}
            TokenState::Moved => {
                self.errors
                    .push(ErrorCodeDefinition::borrow_after_move(&token.source).build());
            }
        }
    }

    /// 结束令牌的生命周期
    pub fn release_token(
        &mut self,
        token_name: &str,
    ) {
        self.tokens.remove(token_name);
    }

    /// 获取所有错误
    pub fn errors(&self) -> &[Diagnostic] {
        &self.errors
    }

    /// 清除状态
    pub fn clear(&mut self) {
        self.tokens.clear();
        self.errors.clear();
    }

    /// 检查函数的借用语义
    pub fn check_function(
        &mut self,
        func: &FunctionIR,
    ) -> &[Diagnostic] {
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
            Instruction::ShareRef { dst, src } => {
                let token_name = operand_display_name(dst, self.local_names.as_ref());
                let source = operand_display_name(src, self.local_names.as_ref());
                self.create_borrow(&token_name, &source, false);
            }
            Instruction::Move { dst, src } => {
                let src_name = operand_display_name(src, self.local_names.as_ref());
                if self.tokens.contains_key(&src_name) {
                    if let Some(token) = self.tokens.get_mut(&src_name) {
                        token.state = TokenState::Moved;
                    }
                    if let Some(token) = self.tokens.get(&src_name) {
                        let source = token.source.clone();
                        let mutable = token.mutable;
                        let dst_name = operand_display_name(dst, self.local_names.as_ref());
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
            Instruction::Borrow { dst, src, mutable } => {
                let token_name = operand_display_name(dst, self.local_names.as_ref());
                let source = operand_display_name(src, self.local_names.as_ref());
                self.create_borrow(&token_name, &source, *mutable);
            }
            Instruction::Release(operand) => {
                let name = operand_display_name(operand, self.local_names.as_ref());
                self.release_token(&name);
            }
            Instruction::Drop(operand) => {
                let name = operand_display_name(operand, self.local_names.as_ref());
                self.release_token(&name);
            }
            Instruction::Load { src, .. }
            | Instruction::Neg { src, .. }
            | Instruction::Cast { src, .. } => {
                let name = operand_display_name(src, self.local_names.as_ref());
                if self.tokens.contains_key(&name) {
                    self.use_token(&name);
                }
            }
            Instruction::LoadIndex { src, index, .. } => {
                let name = operand_display_name(src, self.local_names.as_ref());
                if self.tokens.contains_key(&name) {
                    self.use_token(&name);
                }
                let idx_name = operand_display_name(index, self.local_names.as_ref());
                if self.tokens.contains_key(&idx_name) {
                    self.use_token(&idx_name);
                }
            }
            Instruction::LoadField { src, .. } => {
                let name = operand_display_name(src, self.local_names.as_ref());
                if self.tokens.contains_key(&name) {
                    self.use_token(&name);
                }
            }
            Instruction::Store { src, dst, .. } => {
                let src_name = operand_display_name(src, self.local_names.as_ref());
                if self.tokens.contains_key(&src_name) {
                    self.use_token(&src_name);
                }
                let dst_name = operand_display_name(dst, self.local_names.as_ref());
                if self.tokens.contains_key(&dst_name) {
                    self.use_token(&dst_name);
                }
            }
            Instruction::StoreField { src, dst, .. } => {
                let src_name = operand_display_name(src, self.local_names.as_ref());
                if self.tokens.contains_key(&src_name) {
                    self.use_token(&src_name);
                }
                let dst_name = operand_display_name(dst, self.local_names.as_ref());
                if self.tokens.contains_key(&dst_name) {
                    self.use_token(&dst_name);
                }
            }
            Instruction::StoreIndex {
                src, dst, index, ..
            } => {
                let src_name = operand_display_name(src, self.local_names.as_ref());
                if self.tokens.contains_key(&src_name) {
                    self.use_token(&src_name);
                }
                let dst_name = operand_display_name(dst, self.local_names.as_ref());
                if self.tokens.contains_key(&dst_name) {
                    self.use_token(&dst_name);
                }
                let idx_name = operand_display_name(index, self.local_names.as_ref());
                if self.tokens.contains_key(&idx_name) {
                    self.use_token(&idx_name);
                }
            }
            Instruction::Ret(Some(value)) => {
                let name = operand_display_name(value, self.local_names.as_ref());
                if self.tokens.contains_key(&name) {
                    self.use_token(&name);
                }
            }
            Instruction::Call { args, .. } => {
                for arg in args {
                    let name = operand_display_name(arg, self.local_names.as_ref());
                    if self.tokens.contains_key(&name) {
                        self.use_token(&name);
                    }
                }
            }
            Instruction::CallVirt { obj, args, .. } => {
                let obj_name = operand_display_name(obj, self.local_names.as_ref());
                if self.tokens.contains_key(&obj_name) {
                    self.use_token(&obj_name);
                }
                for arg in args {
                    let name = operand_display_name(arg, self.local_names.as_ref());
                    if self.tokens.contains_key(&name) {
                        self.use_token(&name);
                    }
                }
            }
            _ => {}
        }
    }
}

impl Default for BorrowChecker {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::middle::Operand;
    use crate::middle::core::ir::BasicBlock;
    use crate::frontend::core::typecheck::MonoType;

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

    fn make_checker() -> BorrowChecker {
        BorrowChecker::new()
    }

    fn run_borrow_check(instructions: Vec<Instruction>) -> Vec<Diagnostic> {
        let func = make_func_ir(instructions);
        let mut checker = BorrowChecker::new();
        checker.check_function(&func).to_vec()
    }

    #[test]
    fn test_multiple_immutable_borrows() {
        let mut checker = make_checker();
        checker.create_borrow("ref_a", "x", false);
        checker.create_borrow("ref_b", "x", false);
        assert!(
            checker.errors().is_empty(),
            "多不可变借用应允许，得: {:?}",
            checker.errors()
        );
    }

    #[test]
    fn test_mutable_borrow_conflict_with_immutable() {
        let mut checker = make_checker();
        checker.create_borrow("ref_a", "x", false);
        checker.create_borrow("ref_mut_b", "x", true);
        assert_eq!(checker.errors().len(), 1);
        assert!(
            checker.errors()[0].code == "E2017",
            "应得 E2017, 得: {}",
            checker.errors()[0].code
        );
    }

    #[test]
    fn test_mutable_borrow_conflict_with_mutable() {
        let mut checker = make_checker();
        checker.create_borrow("ref_mut_a", "x", true);
        checker.create_borrow("ref_mut_b", "x", true);
        assert_eq!(checker.errors().len(), 1);
        assert!(
            checker.errors()[0].code == "E2017",
            "应得 E2017, 得: {}",
            checker.errors()[0].code
        );
    }

    #[test]
    fn test_immutable_borrow_conflict_with_mutable() {
        let mut checker = make_checker();
        checker.create_borrow("ref_mut_a", "x", true);
        checker.create_borrow("ref_b", "x", false);
        assert_eq!(checker.errors().len(), 1);
        assert!(
            checker.errors()[0].code == "E2017",
            "应得 E2017, 得: {}",
            checker.errors()[0].code
        );
    }

    #[test]
    fn test_use_active_token() {
        let mut checker = make_checker();
        checker.create_borrow("ref_a", "x", false);
        checker.use_token("ref_a");
        assert!(checker.errors().is_empty());
    }

    #[test]
    fn test_use_moved_token() {
        let mut checker = make_checker();
        checker.create_borrow("ref_a", "x", false);
        if let Some(token) = checker.tokens.get_mut("ref_a") {
            token.state = TokenState::Moved;
        }
        checker.use_token("ref_a");
        assert_eq!(checker.errors().len(), 1);
        assert!(checker.errors()[0].code == "E2015");
    }

    #[test]
    fn test_different_sources_no_conflict() {
        let mut checker = make_checker();
        checker.create_borrow("ref_a", "x", true);
        checker.create_borrow("ref_b", "y", true);
        assert!(checker.errors().is_empty());
    }

    #[test]
    fn test_release_nonexistent_token() {
        let mut checker = make_checker();
        checker.release_token("nonexistent");
        assert!(checker.errors().is_empty());
    }

    #[test]
    fn test_e2e_single_immutable_borrow() {
        let errors = run_borrow_check(vec![Instruction::Borrow {
            dst: Operand::Temp(0),
            src: Operand::Local(0),
            mutable: false,
        }]);
        assert!(errors.is_empty(), "单不可变借用应允许: {:?}", errors);
    }

    #[test]
    fn test_e2e_multiple_immutable_borrows() {
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
        assert!(errors.is_empty());
    }

    #[test]
    fn test_e2e_mutable_then_immutable_conflict() {
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
        assert_eq!(errors.len(), 1);
        assert!(errors[0].code == "E2017");
    }

    #[test]
    fn test_e2e_mutable_then_mutable_conflict() {
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
        assert_eq!(errors.len(), 1);
        assert!(errors[0].code == "E2017");
    }

    #[test]
    fn test_e2e_mutable_release_reborrow() {
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
        assert!(errors.is_empty(), "释放后重新借用应允许: {:?}", errors);
    }
}
