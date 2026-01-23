//! 控制流代码生成
//!
//! 将 if、while、for、match、switch 等控制流结构转换为字节码指令。

use crate::middle::codegen::{BytecodeInstruction, CodegenContext, CodegenError};
use crate::frontend::lexer::tokens::Literal;
use crate::frontend::parser::ast::{Block, MatchArm, Pattern};
use crate::middle::ir::Operand;
use crate::backends::common::Opcode;

/// 控制流代码生成实现
impl CodegenContext {
    /// 生成 if 语句
    fn generate_if_stmt(
        &mut self,
        condition: &Block,
        then_branch: &Block,
        elif_branches: &[(Block, Block)],
        else_branch: Option<&Block>,
    ) -> Result<(), CodegenError> {
        let cond = self.generate_block(condition)?;
        let end_label = self.next_label();
        let then_label = self.next_label();

        self.emit(BytecodeInstruction::new(
            Opcode::JmpIfNot,
            vec![self.operand_to_reg(&cond)?, then_label as u8],
        ));

        self.generate_block(then_branch)?;
        self.emit(BytecodeInstruction::new(Opcode::Jmp, vec![end_label as u8]));

        for (elif_cond, elif_body) in elif_branches {
            let elif_label = self.next_label();
            let elif_cond_result = self.generate_block(elif_cond)?;

            self.emit(BytecodeInstruction::new(
                Opcode::JmpIfNot,
                vec![self.operand_to_reg(&elif_cond_result)?, then_label as u8],
            ));

            self.emit(BytecodeInstruction::new(
                Opcode::Label,
                vec![elif_label as u8],
            ));
            self.generate_block(elif_body)?;
            self.emit(BytecodeInstruction::new(Opcode::Jmp, vec![end_label as u8]));
        }

        if let Some(else_block) = else_branch {
            self.generate_block(else_block)?;
        }

        self.emit(BytecodeInstruction::new(
            Opcode::Label,
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
        let loop_label = self.next_label();
        let end_label = self.next_label();
        let prev_loop_label = self.flow.loop_label();
        self.flow.set_loop_label(loop_label, end_label);

        self.emit(BytecodeInstruction::new(
            Opcode::Label,
            vec![loop_label as u8],
        ));
        let cond = self.generate_block(condition)?;

        self.emit(BytecodeInstruction::new(
            Opcode::JmpIfNot,
            vec![self.operand_to_reg(&cond)?, end_label as u8],
        ));

        self.generate_block(body)?;
        self.emit(BytecodeInstruction::new(
            Opcode::Jmp,
            vec![loop_label as u8],
        ));
        self.emit(BytecodeInstruction::new(
            Opcode::Label,
            vec![end_label as u8],
        ));

        if let Some((loop_lbl, end_lbl)) = prev_loop_label {
            self.flow.set_loop_label(loop_lbl, end_lbl);
        } else {
            self.flow.clear_loop_label();
        }

        Ok(())
    }

    /// 生成 for 循环
    fn generate_for_stmt(
        &mut self,
        var: &str,
        iterable: &Block,
        body: &Block,
    ) -> Result<(), CodegenError> {
        let loop_label = self.next_label();
        let end_label = self.next_label();
        let prev_loop_label = self.flow.loop_label();
        self.flow.set_loop_label(loop_label, end_label);

        self.generate_block(iterable)?;
        self.emit(BytecodeInstruction::new(
            Opcode::Label,
            vec![loop_label as u8],
        ));
        self.generate_block(body)?;
        self.emit(BytecodeInstruction::new(
            Opcode::Jmp,
            vec![loop_label as u8],
        ));
        self.emit(BytecodeInstruction::new(
            Opcode::Label,
            vec![end_label as u8],
        ));

        if let Some((loop_lbl, end_lbl)) = prev_loop_label {
            self.flow.set_loop_label(loop_lbl, end_lbl);
        } else {
            self.flow.clear_loop_label();
        }

        let _ = var;
        Ok(())
    }

    /// 生成 match 表达式
    pub(super) fn generate_match_stmt(
        &mut self,
        expr: &Block,
        arms: &[MatchArm],
    ) -> Result<(), CodegenError> {
        let match_reg = self.generate_block(expr)?;
        let end_label = self.next_label();

        for arm in arms {
            let next_arm_label = self.next_label();
            self.generate_pattern_match(&match_reg, &arm.pattern, next_arm_label)?;
            let arm_block = Block {
                stmts: Vec::new(),
                expr: Some(Box::new(arm.body.clone())),
                span: arm.span,
            };
            self.generate_block(&arm_block)?;
            self.emit(BytecodeInstruction::new(Opcode::Jmp, vec![end_label as u8]));
            self.emit(BytecodeInstruction::new(
                Opcode::Label,
                vec![next_arm_label as u8],
            ));
        }

        self.emit(BytecodeInstruction::new(Opcode::Throw, vec![0]));
        self.emit(BytecodeInstruction::new(
            Opcode::Label,
            vec![end_label as u8],
        ));

        Ok(())
    }

    /// 生成模式匹配代码
    fn generate_pattern_match(
        &mut self,
        value_reg: &Operand,
        pattern: &Pattern,
        next_arm_label: usize,
    ) -> Result<(), CodegenError> {
        match pattern {
            Pattern::Wildcard => {}
            Pattern::Identifier(name) => self.generate_binding(value_reg, name)?,
            Pattern::Literal(literal) => {
                self.generate_literal_pattern(value_reg, literal, next_arm_label)?
            }
            Pattern::Tuple(patterns) => {
                self.generate_tuple_pattern(value_reg, patterns, next_arm_label)?
            }
            Pattern::Struct { name, fields } => {
                self.generate_struct_pattern(value_reg, name, fields, next_arm_label)?
            }
            Pattern::Union {
                name,
                variant,
                pattern,
            } => {
                self.generate_union_pattern(
                    value_reg,
                    name,
                    variant,
                    pattern.as_deref(),
                    next_arm_label,
                )?;
            }
            Pattern::Or(patterns) => {
                self.generate_or_pattern(value_reg, patterns, next_arm_label)?
            }
            Pattern::Guard { pattern, condition } => {
                self.generate_guard_pattern(value_reg, pattern, condition, next_arm_label)?
            }
        }
        Ok(())
    }

    /// 生成字面量模式匹配
    fn generate_literal_pattern(
        &mut self,
        value_reg: &Operand,
        literal: &Literal,
        fail_label: usize,
    ) -> Result<(), CodegenError> {
        let cmp_reg = self.next_temp();
        let value_u8 = self.operand_to_reg(value_reg)?;

        match literal {
            Literal::Int(_n) => {
                let const_reg = self.next_temp();
                self.emit(BytecodeInstruction::new(
                    Opcode::I64Const,
                    vec![const_reg as u8],
                ));
                self.emit(BytecodeInstruction::new(
                    Opcode::I64Eq,
                    vec![cmp_reg as u8, value_u8, const_reg as u8],
                ));
            }
            Literal::Bool(b) => {
                let const_reg = self.next_temp();
                self.emit(BytecodeInstruction::new(
                    Opcode::I64Const,
                    vec![const_reg as u8, if *b { 1 } else { 0 }],
                ));
                self.emit(BytecodeInstruction::new(
                    Opcode::I64Eq,
                    vec![cmp_reg as u8, value_u8, const_reg as u8],
                ));
            }
            _ => self.emit(BytecodeInstruction::new(
                Opcode::I64Const,
                vec![cmp_reg as u8, 0],
            )),
        }

        self.emit(BytecodeInstruction::new(
            Opcode::JmpIfNot,
            vec![cmp_reg as u8, fail_label as i16 as u8],
        ));

        Ok(())
    }

    /// 生成元组模式匹配
    fn generate_tuple_pattern(
        &mut self,
        value_reg: &Operand,
        patterns: &[Pattern],
        _fail_label: usize,
    ) -> Result<(), CodegenError> {
        for (i, elem_pattern) in patterns.iter().enumerate() {
            let elem_reg = self.next_temp();
            let value_u8 = self.operand_to_reg(value_reg)?;

            self.emit(BytecodeInstruction::new(
                Opcode::GetField,
                vec![elem_reg as u8, value_u8, i as u8, 0],
            ));

            let next_elem_fail_label = self.next_label();
            self.generate_pattern_match(
                &Operand::Temp(elem_reg),
                elem_pattern,
                next_elem_fail_label,
            )?;
            self.emit(BytecodeInstruction::new(
                Opcode::Label,
                vec![next_elem_fail_label as u8],
            ));
        }
        Ok(())
    }

    /// 生成结构体模式匹配
    fn generate_struct_pattern(
        &mut self,
        value_reg: &Operand,
        _name: &str,
        fields: &[(String, Pattern)],
        fail_label: usize,
    ) -> Result<(), CodegenError> {
        let value_u8 = self.operand_to_reg(value_reg)?;
        let type_check_reg = self.next_temp();

        self.emit(BytecodeInstruction::new(
            Opcode::TypeCheck,
            vec![value_u8, 0, type_check_reg as u8],
        ));

        self.emit(BytecodeInstruction::new(
            Opcode::JmpIfNot,
            vec![type_check_reg as u8, fail_label as i16 as u8],
        ));

        for (field_name, field_pattern) in fields {
            let field_reg = self.next_temp();
            let field_offset = self.get_field_offset(field_name);

            self.emit(BytecodeInstruction::new(
                Opcode::GetField,
                vec![
                    field_reg as u8,
                    value_u8,
                    (field_offset & 0xFF) as u8,
                    (field_offset >> 8) as u8,
                ],
            ));

            let next_field_fail_label = self.next_label();
            self.generate_pattern_match(
                &Operand::Temp(field_reg),
                field_pattern,
                next_field_fail_label,
            )?;
            self.emit(BytecodeInstruction::new(
                Opcode::Label,
                vec![next_field_fail_label as u8],
            ));
        }

        Ok(())
    }

    /// 生成联合类型模式匹配
    fn generate_union_pattern(
        &mut self,
        value_reg: &Operand,
        _name: &str,
        _variant: &str,
        pattern: Option<&Pattern>,
        fail_label: usize,
    ) -> Result<(), CodegenError> {
        let value_u8 = self.operand_to_reg(value_reg)?;
        let type_check_reg = self.next_temp();

        self.emit(BytecodeInstruction::new(
            Opcode::TypeCheck,
            vec![value_u8, 0, type_check_reg as u8],
        ));

        self.emit(BytecodeInstruction::new(
            Opcode::JmpIfNot,
            vec![type_check_reg as u8, fail_label as i16 as u8],
        ));

        if let Some(inner_pattern) = pattern {
            let inner_reg = self.next_temp();
            self.emit(BytecodeInstruction::new(
                Opcode::GetField,
                vec![inner_reg as u8, value_u8, 0, 0],
            ));
            self.generate_pattern_match(&Operand::Temp(inner_reg), inner_pattern, fail_label)?;
        }

        Ok(())
    }

    /// 生成或模式匹配
    fn generate_or_pattern(
        &mut self,
        value_reg: &Operand,
        patterns: &[Pattern],
        fail_label: usize,
    ) -> Result<(), CodegenError> {
        for pattern in patterns {
            let next_pattern_label = self.next_label();
            self.generate_pattern_match(value_reg, pattern, next_pattern_label)?;
            self.emit(BytecodeInstruction::new(
                Opcode::Jmp,
                vec![fail_label as u8],
            ));
            self.emit(BytecodeInstruction::new(
                Opcode::Label,
                vec![next_pattern_label as u8],
            ));
        }
        Ok(())
    }

    /// 生成守卫模式匹配
    fn generate_guard_pattern(
        &mut self,
        value_reg: &Operand,
        pattern: &Pattern,
        condition: &crate::frontend::parser::ast::Expr,
        fail_label: usize,
    ) -> Result<(), CodegenError> {
        self.generate_pattern_match(value_reg, pattern, fail_label)?;
        let cond_reg = self.generate_expr(condition)?;
        self.emit(BytecodeInstruction::new(
            Opcode::JmpIfNot,
            vec![self.operand_to_reg(&cond_reg)?, fail_label as i16 as u8],
        ));
        Ok(())
    }

    /// 生成变量绑定
    fn generate_binding(
        &mut self,
        value_reg: &Operand,
        name: &str,
    ) -> Result<(), CodegenError> {
        if let Some(symbol) = self.symbols.symbol_table().get(name) {
            match &symbol.storage {
                super::super::Storage::Local(id) => {
                    self.emit(BytecodeInstruction::new(
                        Opcode::StoreLocal,
                        vec![self.operand_to_reg(value_reg)?, *id as u8],
                    ));
                }
                super::super::Storage::Temp(id) => {
                    self.emit(BytecodeInstruction::new(
                        Opcode::Mov,
                        vec![*id as u8, self.operand_to_reg(value_reg)?],
                    ));
                }
                _ => {}
            }
        }
        Ok(())
    }

    /// 生成返回语句
    pub(super) fn generate_return(
        &mut self,
        value: Option<&Block>,
    ) -> Result<(), CodegenError> {
        if let Some(block) = value {
            let result = self.generate_block(block)?;
            self.emit(BytecodeInstruction::new(
                Opcode::ReturnValue,
                vec![self.operand_to_reg(&result)?],
            ));
        } else {
            self.emit(BytecodeInstruction::new(Opcode::Return, vec![]));
        }
        Ok(())
    }

    /// 生成 break 语句
    pub(super) fn generate_break(
        &mut self,
        _label: Option<&str>,
    ) -> Result<(), CodegenError> {
        if let Some((_, end_label)) = self.flow.loop_label() {
            self.emit(BytecodeInstruction::new(Opcode::Jmp, vec![end_label as u8]));
        }
        Ok(())
    }

    /// 生成 continue 语句
    pub(super) fn generate_continue(
        &mut self,
        _label: Option<&str>,
    ) -> Result<(), CodegenError> {
        if let Some((loop_label, _)) = self.flow.loop_label() {
            self.emit(BytecodeInstruction::new(
                Opcode::Jmp,
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

        for stmt in &block.stmts {
            self.generate_stmt(stmt)?;
        }

        if let Some(expr) = &block.expr {
            result = self.generate_expr(expr)?;
        }

        Ok(result)
    }
}

// ===== Switch 语句（从 switch.rs 合并）=====

use crate::frontend::parser::ast::Expr;

impl CodegenContext {
    /// 生成 Switch 语句
    pub fn generate_switch(
        &mut self,
        cond: &Expr,
        cases: &[(Expr, Expr)],
        default: Option<&Expr>,
    ) -> Result<Operand, CodegenError> {
        let cond_reg = self.generate_expr(cond)?;

        if let Some(table) = self.try_build_jump_table(cases) {
            self.generate_jump_table_switch(cond_reg.clone(), cases, table, default)
        } else {
            self.generate_if_else_chain_switch(cond_reg, cases, default)
        }
    }

    /// 尝试构建跳转表
    fn try_build_jump_table(
        &self,
        cases: &[(Expr, Expr)],
    ) -> Option<Vec<(i32, usize)>> {
        let mut table = Vec::new();

        for (idx, (value_expr, _body_expr)) in cases.iter().enumerate() {
            if let Expr::Lit(Literal::Int(n), _) = value_expr {
                table.push((*n as i32, idx));
            } else {
                return None;
            }
        }

        if let Some((min, max)) = get_min_max(table.iter().map(|(v, _)| v)) {
            let range = (*max as usize) as i32 - (*min as usize) as i32;
            let count = table.len();

            if range > count as i32 * 2 {
                return None;
            }
        }

        Some(table)
    }

    /// 生成跳转表 Switch 指令
    fn generate_jump_table_switch(
        &mut self,
        cond_reg: Operand,
        cases: &[(Expr, Expr)],
        table: Vec<(i32, usize)>,
        default: Option<&Expr>,
    ) -> Result<Operand, CodegenError> {
        let end_label = self.next_label();
        let dst = self.next_temp();

        let table_idx = self.flow.get_jump_table_index().unwrap_or(0);
        let mut jump_table = super::super::JumpTable::new(table_idx);

        let mut case_labels = Vec::new();
        for (_, case_idx) in &table {
            let arm_label = self.next_label();
            case_labels.push((*case_idx, arm_label));
            jump_table.add_entry(*case_idx, arm_label);
        }

        let default_label = if default.is_some() {
            Some(self.next_label())
        } else {
            None
        };
        let default_offset = default_label.map_or(end_label as i32, |l| l as i32);

        self.emit(BytecodeInstruction::new(
            Opcode::Switch,
            vec![
                self.operand_to_reg(&cond_reg)?,
                default_offset as u8,
                table_idx as u8,
            ],
        ));

        self.flow.add_jump_table(jump_table);

        for (case_idx, _arm_label) in case_labels.iter() {
            if *case_idx < cases.len() {
                let (_value_expr, body_expr) = &cases[*case_idx];
                self.generate_expr(body_expr)?;
            }
        }

        if let Some(default_expr) = default {
            self.generate_expr(default_expr)?;
        }

        self.emit(BytecodeInstruction::new(
            Opcode::Label,
            vec![end_label as u8],
        ));

        Ok(Operand::Temp(dst))
    }

    /// 生成 If-Else 链（回退模式）
    fn generate_if_else_chain_switch(
        &mut self,
        cond_reg: Operand,
        cases: &[(Expr, Expr)],
        default: Option<&Expr>,
    ) -> Result<Operand, CodegenError> {
        let end_label = self.next_label();

        for (value_expr, body_expr) in cases {
            let body_label = self.next_label();
            let value_reg = self.generate_expr(value_expr)?;
            let cmp_dst = self.next_temp();

            self.emit(BytecodeInstruction::new(
                Opcode::I64Eq,
                vec![
                    cmp_dst as u8,
                    self.operand_to_reg(&cond_reg)?,
                    self.operand_to_reg(&value_reg)?,
                ],
            ));

            self.emit(BytecodeInstruction::new(
                Opcode::JmpIfNot,
                vec![cmp_dst as u8, body_label as i8 as u8],
            ));

            self.generate_expr(body_expr)?;
            self.emit(BytecodeInstruction::new(Opcode::Jmp, vec![end_label as u8]));
        }

        if let Some(default_expr) = default {
            self.generate_expr(default_expr)?;
        }

        self.emit(BytecodeInstruction::new(
            Opcode::Label,
            vec![end_label as u8],
        ));

        Ok(Operand::Temp(self.next_temp()))
    }
}

/// 获取迭代器的最小值和最大值
fn get_min_max<T: Copy + PartialOrd, I: Iterator<Item = T>>(mut iter: I) -> Option<(T, T)> {
    match iter.next() {
        None => None,
        Some(first) => {
            let mut min = first;
            let mut max = first;
            for item in iter {
                if item < min {
                    min = item;
                }
                if item > max {
                    max = item;
                }
            }
            Some((min, max))
        }
    }
}
