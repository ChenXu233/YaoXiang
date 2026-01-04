//! Switch 语句代码生成
//!
//! 实现 O(1) 的跳转表优化。

use super::{BytecodeInstruction, CodegenContext, CodegenError};
use crate::frontend::lexer::tokens::Literal;
use crate::frontend::parser::ast::Expr;
use crate::middle::ir::Operand;
use crate::vm::opcode::TypedOpcode;

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
    fn try_build_jump_table(&self, cases: &[(Expr, Expr)]) -> Option<Vec<(i32, usize)>> {
        let mut table = Vec::new();

        for (idx, (value_expr, _body_expr)) in cases.iter().enumerate() {
            if let Expr::Lit(literal, _) = value_expr {
                if let Literal::Int(n) = literal {
                    table.push((*n as i32, idx));
                } else {
                    return None;
                }
            } else {
                return None;
            }
        }

        // 检查值是否密集
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

        let table_idx = self.jump_tables.len() as u16;
        let mut jump_table = super::JumpTable::new(table_idx);

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
            TypedOpcode::Switch,
            vec![
                self.operand_to_reg(&cond_reg)?,
                default_offset as u8,
                table_idx as u8,
            ],
        ));

        self.jump_tables.insert(table_idx, jump_table);

        // 生成各个 case 的代码块（简化版本，不使用 enter_block/exit_block）
        for (case_idx, _arm_label) in case_labels.iter() {
            // 找到原始的 case body 并生成
            if *case_idx < cases.len() {
                let (_value_expr, body_expr) = &cases[*case_idx];
                self.generate_expr(body_expr)?;
            }
        }

        // 生成 default 代码块（如果有）
        if let Some(default_expr) = default {
            self.generate_expr(default_expr)?;
        }

        self.emit(BytecodeInstruction::new(
            TypedOpcode::Label,
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
                TypedOpcode::I64Eq,
                vec![
                    cmp_dst as u8,
                    self.operand_to_reg(&cond_reg)?,
                    self.operand_to_reg(&value_reg)?,
                ],
            ));

            self.emit(BytecodeInstruction::new(
                TypedOpcode::JmpIfNot,
                vec![cmp_dst as u8, body_label as i8 as u8],
            ));

            self.generate_expr(body_expr)?;

            self.emit(BytecodeInstruction::new(
                TypedOpcode::Jmp,
                vec![end_label as u8],
            ));
        }

        if let Some(default_expr) = default {
            self.generate_expr(default_expr)?;
        }

        self.emit(BytecodeInstruction::new(
            TypedOpcode::Label,
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
