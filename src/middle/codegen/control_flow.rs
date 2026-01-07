//! 控制流代码生成
//!
//! 将 if、while、for、match 等控制流结构转换为字节码指令。

use super::{CodegenContext, CodegenError};
use crate::frontend::parser::ast::{Block, MatchArm};
use crate::middle::ir::Operand;

/// 控制流代码生成实现
impl CodegenContext {
    fn generate_if_stmt(
        &mut self,
        condition: &Block,
        then_branch: &Block,
        elif_branches: &[(Block, Block)],
        else_branch: Option<&Block>,
    ) -> Result<(), CodegenError> {
        // 生成条件表达式
        let cond = self.generate_block(condition)?;

        // 创建标签
        let end_label = self.next_label();
        let then_label = self.next_label();

        // 条件为假则跳转到 then 分支
        self.emit(super::BytecodeInstruction::new(
            crate::vm::opcode::TypedOpcode::JmpIfNot,
            vec![self.operand_to_reg(&cond)?, then_label as u8],
        ));

        // 生成 then 分支
        self.generate_block(then_branch)?;
        self.emit(super::BytecodeInstruction::new(
            crate::vm::opcode::TypedOpcode::Jmp,
            vec![end_label as u8],
        ));

        // 处理 elif 分支
        for (elif_cond, elif_body) in elif_branches {
            let elif_label = self.next_label();

            // 生成 elif 条件
            let elif_cond_result = self.generate_block(elif_cond)?;

            // 条件为假则跳转到下一个 elif 或 else
            self.emit(super::BytecodeInstruction::new(
                crate::vm::opcode::TypedOpcode::JmpIfNot,
                vec![self.operand_to_reg(&elif_cond_result)?, then_label as u8],
            ));

            // 生成 elif 标签
            self.emit(super::BytecodeInstruction::new(
                crate::vm::opcode::TypedOpcode::Label,
                vec![elif_label as u8],
            ));

            self.generate_block(elif_body)?;
            self.emit(super::BytecodeInstruction::new(
                crate::vm::opcode::TypedOpcode::Jmp,
                vec![end_label as u8],
            ));
        }

        // 生成 else 分支（如果有）
        if let Some(else_block) = else_branch {
            self.generate_block(else_block)?;
        }

        // 结束标签
        self.emit(super::BytecodeInstruction::new(
            crate::vm::opcode::TypedOpcode::Label,
            vec![end_label as u8],
        ));

        Ok(())
    }

    /// 生成 while 循环
    fn generate_while_stmt(
        &mut self,
        condition: &Block,
        body: &Block,
        _label: Option<&str>,
    ) -> Result<(), CodegenError> {
        // 创建标签
        let loop_label = self.next_label();
        let end_label = self.next_label();

        // 保存循环标签（用于 break/continue）
        let prev_loop_label = self.current_loop_label.replace((loop_label, end_label));

        // 循环开始标签
        self.emit(super::BytecodeInstruction::new(
            crate::vm::opcode::TypedOpcode::Label,
            vec![loop_label as u8],
        ));

        // 生成条件
        let cond = self.generate_block(condition)?;

        // 条件为假则跳出循环
        self.emit(super::BytecodeInstruction::new(
            crate::vm::opcode::TypedOpcode::JmpIfNot,
            vec![self.operand_to_reg(&cond)?, end_label as u8],
        ));

        // 生成循环体
        self.generate_block(body)?;

        // 跳回循环开始
        self.emit(super::BytecodeInstruction::new(
            crate::vm::opcode::TypedOpcode::Jmp,
            vec![loop_label as u8],
        ));

        // 循环结束标签
        self.emit(super::BytecodeInstruction::new(
            crate::vm::opcode::TypedOpcode::Label,
            vec![end_label as u8],
        ));

        // 恢复循环标签
        self.current_loop_label = prev_loop_label;

        Ok(())
    }

    /// 生成 for 循环
    fn generate_for_stmt(
        &mut self,
        var: &str,
        iterable: &Block,
        body: &Block,
    ) -> Result<(), CodegenError> {
        // 创建标签
        let loop_label = self.next_label();
        let end_label = self.next_label();

        // 保存循环标签
        let prev_loop_label = self.current_loop_label.replace((loop_label, end_label));

        // 生成可迭代对象
        self.generate_block(iterable)?;

        // 循环开始
        self.emit(super::BytecodeInstruction::new(
            crate::vm::opcode::TypedOpcode::Label,
            vec![loop_label as u8],
        ));

        // 生成循环体
        self.generate_block(body)?;

        // 跳回循环开始
        self.emit(super::BytecodeInstruction::new(
            crate::vm::opcode::TypedOpcode::Jmp,
            vec![loop_label as u8],
        ));

        // 循环结束
        self.emit(super::BytecodeInstruction::new(
            crate::vm::opcode::TypedOpcode::Label,
            vec![end_label as u8],
        ));

        // 恢复循环标签
        self.current_loop_label = prev_loop_label;

        // 使用 var 避免未使用警告
        let _ = var;
        Ok(())
    }

    /// 生成 match 表达式
    fn generate_match_stmt(
        &mut self,
        expr: &Block,
        arms: &[MatchArm],
    ) -> Result<(), CodegenError> {
        // 生成匹配表达式
        let match_reg = self.generate_block(expr)?;

        // 为每个臂创建标签
        let end_label = self.next_label();
        let arm_labels: Vec<usize> = (0..arms.len()).map(|_| self.next_label()).collect();

        // 发射 Switch 指令
        self.emit(super::BytecodeInstruction::new(
            crate::vm::opcode::TypedOpcode::Switch,
            vec![self.operand_to_reg(&match_reg)?, end_label as i16 as u8, 0],
        ));

        // 生成每个臂的代码
        for (i, arm) in arms.iter().enumerate() {
            let arm_label = arm_labels[i];

            self.emit(super::BytecodeInstruction::new(
                crate::vm::opcode::TypedOpcode::Label,
                vec![arm_label as u8],
            ));

            // 生成臂体（从 Block 生成）
            let arm_block = Block {
                stmts: Vec::new(),
                expr: Some(Box::new(arm.body.clone())),
                span: arm.span,
            };
            self.generate_block(&arm_block)?;

            self.emit(super::BytecodeInstruction::new(
                crate::vm::opcode::TypedOpcode::Jmp,
                vec![end_label as u8],
            ));
        }

        // 结束标签
        self.emit(super::BytecodeInstruction::new(
            crate::vm::opcode::TypedOpcode::Label,
            vec![end_label as u8],
        ));

        Ok(())
    }

    /// 生成返回语句
    pub(super) fn generate_return(
        &mut self,
        value: Option<&Block>,
    ) -> Result<(), CodegenError> {
        if let Some(block) = value {
            let result = self.generate_block(block)?;
            self.emit(super::BytecodeInstruction::new(
                crate::vm::opcode::TypedOpcode::ReturnValue,
                vec![self.operand_to_reg(&result)?],
            ));
        } else {
            self.emit(super::BytecodeInstruction::new(
                crate::vm::opcode::TypedOpcode::Return,
                vec![],
            ));
        }
        Ok(())
    }

    /// 生成 break 语句
    pub(super) fn generate_break(
        &mut self,
        _label: Option<&str>,
    ) -> Result<(), CodegenError> {
        if let Some((_, end_label)) = self.current_loop_label {
            self.emit(super::BytecodeInstruction::new(
                crate::vm::opcode::TypedOpcode::Jmp,
                vec![end_label as u8],
            ));
        }
        Ok(())
    }

    /// 生成 continue 语句
    pub(super) fn generate_continue(
        &mut self,
        _label: Option<&str>,
    ) -> Result<(), CodegenError> {
        if let Some((loop_label, _)) = self.current_loop_label {
            self.emit(super::BytecodeInstruction::new(
                crate::vm::opcode::TypedOpcode::Jmp,
                vec![loop_label as u8],
            ));
        }
        Ok(())
    }

    /// 生成块
    pub(super) fn generate_block(
        &mut self,
        block: &Block,
    ) -> Result<Operand, CodegenError> {
        let mut result = Operand::Temp(self.next_temp());

        // 生成所有语句
        for stmt in &block.stmts {
            self.generate_stmt(stmt)?;
        }

        // 生成尾表达式（如果有）
        if let Some(expr) = &block.expr {
            result = self.generate_expr(expr)?;
        }

        Ok(result)
    }
}
