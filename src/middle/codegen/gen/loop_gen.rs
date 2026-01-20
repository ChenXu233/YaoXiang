//! 循环优化代码生成
//!
//! 生成高效的 range 循环和迭代器循环。

use crate::middle::codegen::{BytecodeInstruction, CodegenContext, CodegenError};
use crate::frontend::parser::ast::{Expr, Stmt};
use crate::middle::ir::Operand;
use crate::vm::opcode::TypedOpcode;

impl CodegenContext {
    /// 生成 For 循环
    pub fn generate_for_loop(
        &mut self,
        var_name: &str,
        iterable: &Expr,
        body: &Stmt,
    ) -> Result<Operand, CodegenError> {
        if let Some(range_info) = self.try_match_range(iterable) {
            return self.generate_range_loop(
                var_name,
                range_info.0,
                range_info.1,
                range_info.2,
                body,
            );
        }
        self.generate_iterator_loop(var_name, iterable, body)
    }

    /// 尝试匹配 range 调用
    fn try_match_range<'a>(
        &self,
        expr: &'a Expr,
    ) -> Option<(&'a Expr, &'a Expr, Option<&'a Expr>)> {
        if let Expr::Call { func, args, .. } = expr {
            if let Expr::Var(name, _) = &**func {
                if name == "range" {
                    match args.len() {
                        2 => Some((&args[0], &args[1], None)),
                        3 => Some((&args[0], &args[1], Some(&args[2]))),
                        _ => None,
                    }
                } else {
                    None
                }
            } else {
                None
            }
        } else {
            None
        }
    }

    /// 生成优化后的 Range 循环
    fn generate_range_loop(
        &mut self,
        var_name: &str,
        start: &Expr,
        end: &Expr,
        step: Option<&Expr>,
        body: &Stmt,
    ) -> Result<Operand, CodegenError> {
        let loop_start_label = self.next_label();
        let loop_exit_label = self.next_label();
        let _prev_label = self.flow.loop_label();
        self.flow.set_loop_label(loop_start_label, loop_exit_label);

        let start_reg = self.generate_expr(start)?;
        let end_reg = self.generate_expr(end)?;

        let one_idx = self.add_constant(crate::middle::ir::ConstValue::Int(1));
        let step_reg = if let Some(s) = step {
            self.generate_expr(s)?
        } else {
            let dst = self.next_temp();
            self.emit(BytecodeInstruction::new(
                TypedOpcode::LoadConst,
                vec![dst as u8, one_idx as u8],
            ));
            Operand::Temp(dst)
        };

        let current_reg = self.next_temp();
        self.emit(BytecodeInstruction::new(
            TypedOpcode::Mov,
            vec![current_reg as u8, self.operand_to_reg(&start_reg)?],
        ));

        let local_idx = self.next_local();
        self.symbols.insert(
            var_name.to_string(),
            super::super::Symbol {
                name: var_name.to_string(),
                ty: crate::frontend::typecheck::MonoType::Int(64),
                storage: super::super::Storage::Local(local_idx),
                is_mut: true,
                scope_level: self.symbols.scope_level(),
            },
        );

        self.emit(BytecodeInstruction::new(
            TypedOpcode::LoopStart,
            vec![
                current_reg as u8,
                self.operand_to_reg(&end_reg)?,
                self.operand_to_reg(&step_reg)?,
                loop_exit_label as u8,
            ],
        ));

        self.emit(BytecodeInstruction::new(
            TypedOpcode::StoreLocal,
            vec![current_reg as u8, local_idx as u8],
        ));

        self.generate_stmt(body)?;

        self.emit(BytecodeInstruction::new(
            TypedOpcode::LoopInc,
            vec![
                current_reg as u8,
                self.operand_to_reg(&step_reg)?,
                loop_start_label as u8,
            ],
        ));

        self.emit(BytecodeInstruction::new(
            TypedOpcode::Label,
            vec![loop_exit_label as u8],
        ));

        if let Some((loop_lbl, end_lbl)) = _prev_label {
            self.flow.set_loop_label(loop_lbl, end_lbl);
        } else {
            self.flow.clear_loop_label();
        }

        Ok(Operand::Temp(current_reg))
    }

    /// 生成普通迭代器循环
    fn generate_iterator_loop(
        &mut self,
        var_name: &str,
        iterable: &Expr,
        body: &Stmt,
    ) -> Result<Operand, CodegenError> {
        let loop_start_label = self.next_label();
        let loop_exit_label = self.next_label();
        let loop_body_label = self.next_label();

        let _prev_label = self.flow.loop_label();
        self.flow.set_loop_label(loop_start_label, loop_exit_label);

        let _iterable_reg = self.generate_expr(iterable)?;
        let iter_reg = self.next_temp();

        self.emit(BytecodeInstruction::new(
            TypedOpcode::CallStatic,
            vec![iter_reg as u8, 0, 0, 1],
        ));

        self.emit(BytecodeInstruction::new(
            TypedOpcode::Label,
            vec![loop_start_label as u8],
        ));

        let val_reg = self.next_temp();
        self.emit(BytecodeInstruction::new(
            TypedOpcode::CallStatic,
            vec![val_reg as u8, 0, 0, 1],
        ));

        let cmp_dst = self.next_temp();
        self.emit(BytecodeInstruction::new(
            TypedOpcode::I64Eq,
            vec![cmp_dst as u8, val_reg as u8, 0],
        ));

        self.emit(BytecodeInstruction::new(
            TypedOpcode::JmpIf,
            vec![cmp_dst as u8, loop_exit_label as u8],
        ));

        let local_idx = self.next_local();
        self.symbols.insert(
            var_name.to_string(),
            super::super::Symbol {
                name: var_name.to_string(),
                ty: crate::frontend::typecheck::MonoType::Void,
                storage: super::super::Storage::Local(local_idx),
                is_mut: false,
                scope_level: self.symbols.scope_level(),
            },
        );

        self.emit(BytecodeInstruction::new(
            TypedOpcode::StoreLocal,
            vec![val_reg as u8, local_idx as u8],
        ));

        self.emit(BytecodeInstruction::new(
            TypedOpcode::Label,
            vec![loop_body_label as u8],
        ));
        self.generate_stmt(body)?;
        self.emit(BytecodeInstruction::new(
            TypedOpcode::Jmp,
            vec![loop_start_label as u8],
        ));
        self.emit(BytecodeInstruction::new(
            TypedOpcode::Label,
            vec![loop_exit_label as u8],
        ));

        if let Some((loop_lbl, end_lbl)) = _prev_label {
            self.flow.set_loop_label(loop_lbl, end_lbl);
        } else {
            self.flow.clear_loop_label();
        }

        Ok(Operand::Temp(iter_reg))
    }

    /// 生成 while 循环
    pub fn generate_while_loop(
        &mut self,
        condition: &Expr,
        body: &Stmt,
    ) -> Result<Operand, CodegenError> {
        let loop_start_label = self.next_label();
        let loop_exit_label = self.next_label();

        let _prev_label = self.flow.loop_label();
        self.flow.set_loop_label(loop_start_label, loop_exit_label);

        self.emit(BytecodeInstruction::new(
            TypedOpcode::Label,
            vec![loop_start_label as u8],
        ));
        let cond_reg = self.generate_expr(condition)?;

        self.emit(BytecodeInstruction::new(
            TypedOpcode::JmpIfNot,
            vec![self.operand_to_reg(&cond_reg)?, loop_exit_label as u8],
        ));

        self.generate_stmt(body)?;
        self.emit(BytecodeInstruction::new(
            TypedOpcode::Jmp,
            vec![loop_start_label as u8],
        ));
        self.emit(BytecodeInstruction::new(
            TypedOpcode::Label,
            vec![loop_exit_label as u8],
        ));

        if let Some((loop_lbl, end_lbl)) = _prev_label {
            self.flow.set_loop_label(loop_lbl, end_lbl);
        } else {
            self.flow.clear_loop_label();
        }

        Ok(Operand::Temp(self.next_temp()))
    }
}
