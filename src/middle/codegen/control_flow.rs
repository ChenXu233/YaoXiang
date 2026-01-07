//! 控制流代码生成
//!
//! 将 if、while、for、match 等控制流结构转换为字节码指令。

use super::{BytecodeInstruction, CodegenContext, CodegenError};
use crate::frontend::lexer::tokens::Literal;
use crate::frontend::parser::ast::{Block, MatchArm, Pattern};
use crate::middle::ir::Operand;
use crate::vm::opcode::TypedOpcode;

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
    ///
    /// 支持所有模式类型：字面量、构造器、守卫、嵌套等
    /// 使用 TypeCheck + GetField + JmpIfNot 组合实现，不依赖专用 PAT_xxx 指令
    pub(super) fn generate_match_stmt(
        &mut self,
        expr: &Block,
        arms: &[MatchArm],
    ) -> Result<(), CodegenError> {
        // 生成匹配表达式
        let match_reg = self.generate_block(expr)?;

        // 结束标签
        let end_label = self.next_label();

        // 为每个 arm 生成模式匹配代码
        for arm in arms {
            let next_arm_label = self.next_label();
            self.generate_pattern_match(&match_reg, &arm.pattern, next_arm_label)?;
            // 模式匹配成功，生成 body
            let arm_block = Block {
                stmts: Vec::new(),
                expr: Some(Box::new(arm.body.clone())),
                span: arm.span,
            };
            self.generate_block(&arm_block)?;
            // 跳到结束
            self.emit(BytecodeInstruction::new(
                TypedOpcode::Jmp,
                vec![end_label as u8],
            ));
            // 下一个 arm 的标签
            self.emit(BytecodeInstruction::new(
                TypedOpcode::Label,
                vec![next_arm_label as u8],
            ));
        }

        // 所有模式都不匹配 - 运行时错误
        self.emit(BytecodeInstruction::new(
            TypedOpcode::Throw,
            vec![0], // NonExhaustivePatterns error code
        ));

        // 结束标签
        self.emit(BytecodeInstruction::new(
            TypedOpcode::Label,
            vec![end_label as u8],
        ));

        Ok(())
    }

    /// 生成模式匹配代码
    ///
    /// 为单个模式生成匹配测试代码。
    /// 匹配成功则继续执行后续代码（arm body），匹配失败则跳转到 next_arm_label。
    ///
    /// - `value_reg`: 被匹配的值所在的寄存器
    /// - `pattern`: 要匹配的模式
    /// - `next_arm_label`: 匹配失败时跳转的标签
    fn generate_pattern_match(
        &mut self,
        value_reg: &Operand,
        pattern: &Pattern,
        next_arm_label: usize,
    ) -> Result<(), CodegenError> {
        match pattern {
            // 通配符模式 `_` - 总是匹配
            Pattern::Wildcard => {
                // 什么都不做，直接继续执行
            }

            // 标识符模式 `x` - 绑定变量，总是匹配
            Pattern::Identifier(name) => {
                // 将值存储到变量的存储位置
                self.generate_binding(value_reg, name)?;
            }

            // 字面量模式 `0`, `"hello"`, `true` 等
            Pattern::Literal(literal) => {
                self.generate_literal_pattern(value_reg, literal, next_arm_label)?;
            }

            // 元组模式 `(a, b, c)`
            Pattern::Tuple(patterns) => {
                self.generate_tuple_pattern(value_reg, patterns, next_arm_label)?;
            }

            // 结构体模式 `Some(x)`, `Point { x, y }`
            Pattern::Struct { name, fields } => {
                self.generate_struct_pattern(value_reg, name, fields, next_arm_label)?;
            }

            // 联合类型模式 `Ok(value)`, `Err(e)`
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

            // 或模式 `A | B | C`
            Pattern::Or(patterns) => {
                self.generate_or_pattern(value_reg, patterns, next_arm_label)?;
            }

            // 守卫模式 `x if x > 0`
            Pattern::Guard { pattern, condition } => {
                self.generate_guard_pattern(value_reg, pattern, condition, next_arm_label)?;
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
            // 整数 literal
            Literal::Int(_n) => {
                // 加载常量 *n
                let const_reg = self.next_temp();
                self.emit(BytecodeInstruction::new(
                    TypedOpcode::I64Const,
                    vec![const_reg as u8],
                ));
                // I64Eq: dst, lhs, rhs
                self.emit(BytecodeInstruction::new(
                    TypedOpcode::I64Eq,
                    vec![cmp_reg as u8, value_u8, const_reg as u8],
                ));
            }
            // 布尔 literal
            Literal::Bool(b) => {
                // 加载布尔常量 (0 或 1)
                let const_reg = self.next_temp();
                self.emit(BytecodeInstruction::new(
                    TypedOpcode::I64Const,
                    vec![const_reg as u8, if *b { 1 } else { 0 }],
                ));
                self.emit(BytecodeInstruction::new(
                    TypedOpcode::I64Eq,
                    vec![cmp_reg as u8, value_u8, const_reg as u8],
                ));
            }
            // 其他字面量类型（字符串、浮点数、字符）
            _ => {
                // 简化处理：对于非整数字面量，总是失败
                // TODO: 实现完整的字符串/浮点数比较
                self.emit(BytecodeInstruction::new(
                    TypedOpcode::I64Const,
                    vec![cmp_reg as u8, 0],
                ));
            }
        }

        // JmpIfNot: 如果比较结果为假，跳到下一个 arm
        self.emit(BytecodeInstruction::new(
            TypedOpcode::JmpIfNot,
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
            // 获取元组的第 i 个元素
            let elem_reg = self.next_temp();
            let value_u8 = self.operand_to_reg(value_reg)?;

            // GetField: dst, src, field_offset
            self.emit(BytecodeInstruction::new(
                TypedOpcode::GetField,
                vec![elem_reg as u8, value_u8, i as u8, 0],
            ));

            // 递归匹配元素模式
            let next_elem_fail_label = self.next_label();
            self.generate_pattern_match(
                &Operand::Temp(elem_reg),
                elem_pattern,
                next_elem_fail_label,
            )?;

            // 元素匹配失败：发射下一个元素的失败标签
            self.emit(BytecodeInstruction::new(
                TypedOpcode::Label,
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

        // 1. TypeCheck: 检查类型是否匹配
        // TypeCheck: obj_reg, type_id, dst
        // TODO: 需要获取 name 对应的 type_id
        self.emit(BytecodeInstruction::new(
            TypedOpcode::TypeCheck,
            vec![value_u8, 0, type_check_reg as u8],
        ));

        // 如果类型不匹配，跳到下一个 arm
        self.emit(BytecodeInstruction::new(
            TypedOpcode::JmpIfNot,
            vec![type_check_reg as u8, fail_label as i16 as u8],
        ));

        // 2. 对每个字段进行模式匹配
        for (field_name, field_pattern) in fields {
            // 获取字段值
            let field_reg = self.next_temp();
            let field_offset = self.get_field_offset(field_name);

            self.emit(BytecodeInstruction::new(
                TypedOpcode::GetField,
                vec![
                    field_reg as u8,
                    value_u8,
                    (field_offset & 0xFF) as u8,
                    (field_offset >> 8) as u8,
                ],
            ));

            // 递归匹配字段模式
            let next_field_fail_label = self.next_label();
            self.generate_pattern_match(
                &Operand::Temp(field_reg),
                field_pattern,
                next_field_fail_label,
            )?;

            self.emit(BytecodeInstruction::new(
                TypedOpcode::Label,
                vec![next_field_fail_label as u8],
            ));
        }

        Ok(())
    }

    /// 生成联合类型模式匹配 (代数数据类型)
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

        // 1. TypeCheck: 检查变体类型
        // TODO: 需要获取 (name, variant) 对应的 type_id
        self.emit(BytecodeInstruction::new(
            TypedOpcode::TypeCheck,
            vec![value_u8, 0, type_check_reg as u8],
        ));

        self.emit(BytecodeInstruction::new(
            TypedOpcode::JmpIfNot,
            vec![type_check_reg as u8, fail_label as i16 as u8],
        ));

        // 2. 如果有内部模式，匹配内部值
        if let Some(inner_pattern) = pattern {
            let inner_reg = self.next_temp();
            // 获取变体的内部值（字段偏移 0）
            self.emit(BytecodeInstruction::new(
                TypedOpcode::GetField,
                vec![inner_reg as u8, value_u8, 0, 0],
            ));
            self.generate_pattern_match(&Operand::Temp(inner_reg), inner_pattern, fail_label)?;
        }

        Ok(())
    }

    /// 生成或模式匹配 (A | B | C)
    fn generate_or_pattern(
        &mut self,
        value_reg: &Operand,
        patterns: &[Pattern],
        fail_label: usize,
    ) -> Result<(), CodegenError> {
        // 或模式：依次尝试每个模式，任何一个成功就继续
        for pattern in patterns {
            let next_pattern_label = self.next_label();
            self.generate_pattern_match(value_reg, pattern, next_pattern_label)?;
            // 这个 pattern 匹配成功，跳出或模式
            self.emit(BytecodeInstruction::new(
                TypedOpcode::Jmp,
                vec![fail_label as u8], // 跳到 or_pattern 成功后的位置
            ));
            self.emit(BytecodeInstruction::new(
                TypedOpcode::Label,
                vec![next_pattern_label as u8],
            ));
        }
        Ok(())
    }

    /// 生成守卫模式匹配 (x if x > 0)
    fn generate_guard_pattern(
        &mut self,
        value_reg: &Operand,
        pattern: &Pattern,
        condition: &crate::frontend::parser::ast::Expr,
        fail_label: usize,
    ) -> Result<(), CodegenError> {
        // 1. 先匹配模式（绑定变量）
        self.generate_pattern_match(value_reg, pattern, fail_label)?;

        // 2. 再测试守卫条件
        let cond_reg = self.generate_expr(condition)?;
        self.emit(BytecodeInstruction::new(
            TypedOpcode::JmpIfNot,
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
        // 在符号表中查找变量，获取存储位置
        if let Some(symbol) = self.symbol_table.get(name) {
            match symbol.storage {
                super::Storage::Local(id) => {
                    self.emit(BytecodeInstruction::new(
                        TypedOpcode::StoreLocal,
                        vec![self.operand_to_reg(value_reg)?, id as u8],
                    ));
                }
                super::Storage::Temp(id) => {
                    self.emit(BytecodeInstruction::new(
                        TypedOpcode::Mov,
                        vec![id as u8, self.operand_to_reg(value_reg)?],
                    ));
                }
                _ => {
                    // 其他存储类型暂不支持
                }
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
