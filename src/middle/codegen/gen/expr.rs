//! 表达式代码生成
//!
//! 将表达式转换为字节码指令。

use crate::middle::codegen::{CodegenContext, CodegenError};
use crate::frontend::lexer::tokens::Literal;
use crate::frontend::parser::ast::{BinOp, Block, Expr, UnOp};
use crate::frontend::typecheck::check::infer_literal_type;
use crate::frontend::typecheck::MonoType;
use crate::middle::codegen::BytecodeInstruction;
use crate::middle::ir::{ConstValue, Operand};
use crate::backends::common::Opcode;

/// 表达式代码生成实现
impl CodegenContext {
    /// 生成表达式
    pub fn generate_expr(
        &mut self,
        expr: &Expr,
    ) -> Result<Operand, CodegenError> {
        match expr {
            Expr::Lit(literal, _) => self.generate_literal(literal),
            Expr::Var(name, span) => self.generate_variable(name, *span),
            Expr::BinOp {
                op, left, right, ..
            } => self.generate_binop(op, left, right),
            Expr::UnOp { op, expr, .. } => self.generate_unop(op, expr),
            Expr::Call { func, args, .. } => self.generate_call(func, args),
            Expr::FnDef { .. } => Err(CodegenError::UnimplementedExpr {
                expr_type: "FnDef".to_string(),
            }),
            // 控制流表达式（简化实现）
            Expr::If {
                condition,
                then_branch,
                elif_branches,
                else_branch,
                ..
            } => {
                let _cond = self.generate_expr(condition)?;
                let _ = self.generate_block(then_branch)?;
                for (_, elif_body) in elif_branches {
                    let _ = self.generate_block(elif_body)?;
                }
                if let Some(else_b) = else_branch {
                    let _ = self.generate_block(else_b)?;
                }
                Ok(Operand::Temp(self.next_temp()))
            }
            Expr::While {
                condition, body, ..
            } => {
                let _cond = self.generate_expr(condition)?;
                let _ = self.generate_block(body)?;
                Ok(Operand::Temp(self.next_temp()))
            }
            Expr::For {
                var: _var,
                iterable,
                body,
                ..
            } => {
                let _iter = self.generate_expr(iterable)?;
                let _ = self.generate_block(body)?;
                Ok(Operand::Temp(self.next_temp()))
            }
            Expr::Match { expr, arms, .. } => {
                let match_expr = Block {
                    stmts: Vec::new(),
                    expr: Some(Box::new(expr.as_ref().clone())),
                    span: Default::default(),
                };
                self.generate_match_stmt(&match_expr, arms)?;
                Ok(Operand::Temp(self.next_temp()))
            }
            Expr::Block(block) => self.generate_block(block),
            Expr::Return(value, _) => {
                if let Some(val) = value {
                    let operand = self.generate_expr(val)?;
                    self.emit(BytecodeInstruction::new(
                        Opcode::ReturnValue,
                        vec![self.operand_to_reg(&operand)?],
                    ));
                } else {
                    self.emit(BytecodeInstruction::new(Opcode::Return, vec![]));
                }
                Ok(Operand::Temp(self.next_temp()))
            }
            Expr::Break(_, _) => Err(CodegenError::UnimplementedExpr {
                expr_type: "Break".to_string(),
            }),
            Expr::Continue(_, _) => Err(CodegenError::UnimplementedExpr {
                expr_type: "Continue".to_string(),
            }),
            Expr::Tuple(exprs, _) => self.generate_tuple(exprs),
            Expr::List(exprs, _) => self.generate_list(exprs),
            Expr::Dict(pairs, _) => self.generate_dict(pairs),
            Expr::Cast {
                expr, target_type, ..
            } => self.generate_cast(expr, target_type),
            Expr::FieldAccess { expr, field, .. } => self.generate_field_access(expr, field),
            Expr::Index { expr, index, .. } => self.generate_index(expr, index),
            Expr::ListComp { .. } => unimplemented!("List comprehension codegen"),
            Expr::Try { expr, .. } => self.generate_try(expr),
            Expr::Ref { expr, .. } => self.generate_ref(expr),
        }
    }

    /// 生成字面量
    fn generate_literal(
        &mut self,
        literal: &Literal,
    ) -> Result<Operand, CodegenError> {
        let dst = self.next_temp();
        let const_idx = self.add_constant(literal_to_const_value(literal));

        self.emit(BytecodeInstruction::new(
            Opcode::LoadConst,
            vec![dst as u8, const_idx as u8],
        ));

        Ok(Operand::Temp(dst))
    }

    /// 生成变量
    fn generate_variable(
        &mut self,
        name: &str,
        _span: crate::util::span::Span,
    ) -> Result<Operand, CodegenError> {
        let symbol = self.symbols.symbol_table().get(name).cloned();
        let dst = self.next_temp();

        if let Some(symbol) = symbol {
            match symbol.storage {
                super::super::Storage::Local(id) => {
                    self.emit(BytecodeInstruction::new(
                        Opcode::LoadLocal,
                        vec![dst as u8, id as u8],
                    ));
                    Ok(Operand::Temp(dst))
                }
                super::super::Storage::Arg(id) => {
                    self.emit(BytecodeInstruction::new(
                        Opcode::LoadArg,
                        vec![dst as u8, id as u8],
                    ));
                    Ok(Operand::Temp(dst))
                }
                super::super::Storage::Temp(id) => Ok(Operand::Temp(id)),
                super::super::Storage::Global(id) => {
                    self.emit(BytecodeInstruction::new(
                        Opcode::LoadLocal,
                        vec![dst as u8, id as u8],
                    ));
                    Ok(Operand::Temp(dst))
                }
            }
        } else {
            Err(CodegenError::SymbolNotFound {
                name: name.to_string(),
            })
        }
    }

    /// 获取表达式类型
    fn get_expr_type(
        &self,
        expr: &Expr,
    ) -> Result<MonoType, CodegenError> {
        match expr {
            Expr::Lit(lit, _) => Ok(infer_literal_type(lit)),
            Expr::Var(name, _) => {
                if let Some(symbol) = self.symbols.symbol_table().get(name) {
                    Ok(symbol.ty.clone())
                } else {
                    Err(CodegenError::SymbolNotFound {
                        name: name.to_string(),
                    })
                }
            }
            Expr::BinOp { left, .. } => self.get_expr_type(left),
            Expr::UnOp { expr, .. } => self.get_expr_type(expr),
            Expr::Call { func, .. } => {
                let func_ty = self.get_expr_type(func)?;
                match func_ty {
                    MonoType::Fn { return_type, .. } => Ok(*return_type),
                    _ => Err(CodegenError::TypeMismatch {
                        expected: "Function".to_string(),
                        found: format!("{:?}", func_ty),
                    }),
                }
            }
            _ => Ok(MonoType::Int(64)),
        }
    }

    /// 生成二元运算（类型化指令）
    fn generate_binop(
        &mut self,
        op: &BinOp,
        left: &Expr,
        right: &Expr,
    ) -> Result<Operand, CodegenError> {
        let dst = self.next_temp();
        let lhs = self.generate_expr(left)?;
        let rhs = self.generate_expr(right)?;
        let left_type = self.get_expr_type(left)?;

        let opcode = match (op, &left_type) {
            (BinOp::Add, MonoType::Int(64)) => Opcode::I64Add,
            (BinOp::Add, MonoType::Float(64)) => Opcode::F64Add,
            (BinOp::Sub, MonoType::Int(64)) => Opcode::I64Sub,
            (BinOp::Sub, MonoType::Float(64)) => Opcode::F64Sub,
            (BinOp::Mul, MonoType::Int(64)) => Opcode::I64Mul,
            (BinOp::Mul, MonoType::Float(64)) => Opcode::F64Mul,
            (BinOp::Div, MonoType::Int(64)) => Opcode::I64Div,
            (BinOp::Div, MonoType::Float(64)) => Opcode::F64Div,
            (BinOp::Mod, MonoType::Int(64)) => Opcode::I64Rem,
            (BinOp::Eq, MonoType::Int(64)) => Opcode::I64Eq,
            (BinOp::Eq, MonoType::Float(64)) => Opcode::F64Eq,
            (BinOp::Neq, MonoType::Int(64)) => Opcode::I64Ne,
            (BinOp::Neq, MonoType::Float(64)) => Opcode::F64Ne,
            (BinOp::Lt, MonoType::Int(64)) => Opcode::I64Lt,
            (BinOp::Lt, MonoType::Float(64)) => Opcode::F64Lt,
            (BinOp::Le, MonoType::Int(64)) => Opcode::I64Le,
            (BinOp::Le, MonoType::Float(64)) => Opcode::F64Le,
            (BinOp::Gt, MonoType::Int(64)) => Opcode::I64Gt,
            (BinOp::Gt, MonoType::Float(64)) => Opcode::F64Gt,
            (BinOp::Ge, MonoType::Int(64)) => Opcode::I64Ge,
            (BinOp::Ge, MonoType::Float(64)) => Opcode::F64Ge,
            (BinOp::And, _) => Opcode::I64Mul,
            (BinOp::Or, _) => Opcode::I64Add,
            (BinOp::Range, _) => Opcode::NewListWithCap,
            (BinOp::Assign, _) => return self.generate_assignment(left, right),
            _ => Opcode::I64Add,
        };

        self.emit(BytecodeInstruction::new(
            opcode,
            vec![
                dst as u8,
                self.operand_to_reg(&lhs)?,
                self.operand_to_reg(&rhs)?,
            ],
        ));

        Ok(Operand::Temp(dst))
    }

    /// 生成赋值
    fn generate_assignment(
        &mut self,
        target: &Expr,
        value: &Expr,
    ) -> Result<Operand, CodegenError> {
        let src = self.generate_expr(value)?;

        match target {
            Expr::Var(name, _) => {
                if let Some(symbol) = self.symbols.symbol_table().get(name) {
                    match &symbol.storage {
                        super::super::Storage::Local(id) => {
                            self.emit(BytecodeInstruction::new(
                                Opcode::StoreLocal,
                                vec![self.operand_to_reg(&src)?, *id as u8],
                            ));
                        }
                        _ => return Err(CodegenError::InvalidAssignmentTarget),
                    }
                } else {
                    return Err(CodegenError::SymbolNotFound {
                        name: name.to_string(),
                    });
                }
            }
            Expr::Index { expr, index, .. } => {
                let array = self.generate_expr(expr)?;
                let idx = self.generate_expr(index)?;
                self.emit(BytecodeInstruction::new(
                    Opcode::StoreElement,
                    vec![
                        self.operand_to_reg(&array)?,
                        self.operand_to_reg(&idx)?,
                        self.operand_to_reg(&src)?,
                    ],
                ));
            }
            Expr::FieldAccess { expr, field, .. } => {
                let obj = self.generate_expr(expr)?;
                let field_offset = self.get_field_offset(field);
                self.emit(BytecodeInstruction::new(
                    Opcode::SetField,
                    vec![
                        self.operand_to_reg(&obj)?,
                        field_offset as u8,
                        self.operand_to_reg(&src)?,
                    ],
                ));
            }
            _ => return Err(CodegenError::InvalidAssignmentTarget),
        }

        Ok(src)
    }

    /// 获取字段偏移
    pub(super) fn get_field_offset(
        &self,
        field: &str,
    ) -> u16 {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};
        let mut hasher = DefaultHasher::new();
        Hash::hash(field, &mut hasher);
        let hash = Hasher::finish(&hasher);
        (hash % 1000) as u16
    }

    /// 生成一元运算
    fn generate_unop(
        &mut self,
        op: &UnOp,
        expr: &Expr,
    ) -> Result<Operand, CodegenError> {
        let dst = self.next_temp();
        let src = self.generate_expr(expr)?;

        match op {
            UnOp::Neg => {
                self.emit(BytecodeInstruction::new(
                    Opcode::I64Neg,
                    vec![dst as u8, self.operand_to_reg(&src)?],
                ));
            }
            UnOp::Pos => {
                self.emit(BytecodeInstruction::new(
                    Opcode::Mov,
                    vec![dst as u8, self.operand_to_reg(&src)?],
                ));
            }
            UnOp::Not => {
                let zero_reg = self.next_temp();
                self.emit(BytecodeInstruction::new(
                    Opcode::I64Const,
                    vec![zero_reg as u8, 0, 0, 0, 0, 0, 0, 0, 0],
                ));
                self.emit(BytecodeInstruction::new(
                    Opcode::I64Eq,
                    vec![dst as u8, self.operand_to_reg(&src)?, zero_reg as u8],
                ));
            }
        }

        Ok(Operand::Temp(dst))
    }

    /// 生成函数调用
    fn generate_call(
        &mut self,
        func: &Expr,
        args: &[Expr],
    ) -> Result<Operand, CodegenError> {
        let dst = self.next_temp();
        let mut arg_regs = Vec::new();
        for arg in args {
            let arg_reg = self.generate_expr(arg)?;
            arg_regs.push(self.operand_to_reg(&arg_reg)?);
        }

        let base_arg_reg = self.next_temp() as u8;
        for _ in 1..arg_regs.len() {
            self.next_temp();
        }
        for (i, &arg_reg) in arg_regs.iter().enumerate() {
            let target_reg = base_arg_reg + i as u8;
            if arg_reg != target_reg {
                self.emit(BytecodeInstruction::new(
                    Opcode::Mov,
                    vec![target_reg, arg_reg],
                ));
            }
        }

        match func {
            Expr::Var(name, _) => {
                let func_idx = self.flow.function_indices().get(name).copied();
                let func_id = if let Some(idx) = func_idx {
                    idx as u32
                } else {
                    let const_idx = self.add_constant(ConstValue::String(name.clone()));
                    const_idx as u32
                };
                let mut operands = vec![dst as u8];
                operands.extend_from_slice(&func_id.to_le_bytes());
                operands.push(base_arg_reg);
                operands.push(arg_regs.len() as u8);
                self.emit(BytecodeInstruction::new(Opcode::CallStatic, operands));
            }
            Expr::FieldAccess { expr, field, .. } => {
                let obj = self.generate_expr(expr)?;
                let field_offset = self.get_field_offset(field);
                let mut operands = vec![dst as u8];
                operands.push(self.operand_to_reg(&obj)?);
                operands.extend_from_slice(&field_offset.to_le_bytes());
                operands.push(base_arg_reg);
                operands.push(arg_regs.len() as u8);
                self.emit(BytecodeInstruction::new(Opcode::CallVirt, operands));
            }
            _ => {
                let name_idx = self.add_constant(ConstValue::String(format!("{:?}", func)));
                let mut operands = vec![dst as u8];
                operands.push(self.operand_to_reg(&Operand::Temp(0))?);
                operands.extend_from_slice(&(name_idx as u16).to_le_bytes());
                operands.push(base_arg_reg);
                operands.push(arg_regs.len() as u8);
                self.emit(BytecodeInstruction::new(Opcode::CallDyn, operands));
            }
        }

        Ok(Operand::Temp(dst))
    }

    /// 生成元组
    fn generate_tuple(
        &mut self,
        exprs: &[Expr],
    ) -> Result<Operand, CodegenError> {
        if exprs.is_empty() {
            return Ok(Operand::Temp(self.next_temp()));
        }

        let dst = self.next_temp();
        self.emit(BytecodeInstruction::new(
            Opcode::HeapAlloc,
            vec![dst as u8, 0, 0],
        ));

        for (i, elem) in exprs.iter().enumerate() {
            let elem_reg = self.generate_expr(elem)?;
            let field_offset = i as u16;
            self.emit(BytecodeInstruction::new(
                Opcode::SetField,
                vec![
                    dst as u8,
                    (field_offset & 0xFF) as u8,
                    (field_offset >> 8) as u8,
                    self.operand_to_reg(&elem_reg)?,
                ],
            ));
        }

        Ok(Operand::Temp(dst))
    }

    /// 生成列表
    fn generate_list(
        &mut self,
        exprs: &[Expr],
    ) -> Result<Operand, CodegenError> {
        let dst = self.next_temp();
        let cap = exprs.len();
        self.emit(BytecodeInstruction::new(
            Opcode::NewListWithCap,
            vec![dst as u8, (cap & 0xFF) as u8, (cap >> 8) as u8],
        ));

        for (i, elem) in exprs.iter().enumerate() {
            let elem_reg = self.generate_expr(elem)?;
            self.emit(BytecodeInstruction::new(
                Opcode::StoreElement,
                vec![dst as u8, i as u8, self.operand_to_reg(&elem_reg)?],
            ));
        }

        Ok(Operand::Temp(dst))
    }

    /// 生成字典
    fn generate_dict(
        &mut self,
        pairs: &[(Expr, Expr)],
    ) -> Result<Operand, CodegenError> {
        if pairs.is_empty() {
            return Ok(Operand::Temp(self.next_temp()));
        }

        let dict_reg = self.next_temp();

        if let Some(&dict_new_idx) = self.flow.function_indices().get("Dict.new") {
            self.emit(BytecodeInstruction::new(
                Opcode::CallStatic,
                vec![
                    dict_reg as u8,
                    (dict_new_idx & 0xFF) as u8,
                    ((dict_new_idx >> 8) & 0xFF) as u8,
                    ((dict_new_idx >> 16) & 0xFF) as u8,
                    ((dict_new_idx >> 24) & 0xFF) as u8,
                    0,
                    0,
                ],
            ));
        } else {
            let name_idx = self.add_constant(ConstValue::String("Dict.new".to_string()));
            self.emit(BytecodeInstruction::new(
                Opcode::CallDyn,
                vec![
                    dict_reg as u8,
                    0,
                    (name_idx & 0xFF) as u8,
                    (name_idx >> 8) as u8,
                    0,
                    0,
                ],
            ));
        }

        for (key, value) in pairs {
            let key_reg = self.generate_expr(key)?;
            let value_reg = self.generate_expr(value)?;
            let base_arg = self.next_temp() as u8;
            self.next_temp();
            self.next_temp();

            self.emit(BytecodeInstruction::new(
                Opcode::Mov,
                vec![base_arg, dict_reg as u8],
            ));
            self.emit(BytecodeInstruction::new(
                Opcode::Mov,
                vec![base_arg + 1, self.operand_to_reg(&key_reg)?],
            ));
            self.emit(BytecodeInstruction::new(
                Opcode::Mov,
                vec![base_arg + 2, self.operand_to_reg(&value_reg)?],
            ));

            if let Some(&dict_insert_idx) = self.flow.function_indices().get("Dict.insert") {
                self.emit(BytecodeInstruction::new(
                    Opcode::CallStatic,
                    vec![
                        dict_reg as u8,
                        (dict_insert_idx & 0xFF) as u8,
                        ((dict_insert_idx >> 8) & 0xFF) as u8,
                        ((dict_insert_idx >> 16) & 0xFF) as u8,
                        ((dict_insert_idx >> 24) & 0xFF) as u8,
                        base_arg,
                        3,
                    ],
                ));
            } else {
                let name_idx = self.add_constant(ConstValue::String("Dict.insert".to_string()));
                self.emit(BytecodeInstruction::new(
                    Opcode::CallDyn,
                    vec![
                        dict_reg as u8,
                        0,
                        (name_idx & 0xFF) as u8,
                        (name_idx >> 8) as u8,
                        base_arg,
                        3,
                    ],
                ));
            }
        }

        Ok(Operand::Temp(dict_reg))
    }

    /// 生成类型转换
    fn generate_cast(
        &mut self,
        expr: &Expr,
        _target_type: &crate::frontend::parser::ast::Type,
    ) -> Result<Operand, CodegenError> {
        let dst = self.next_temp();
        let src = self.generate_expr(expr)?;

        self.emit(BytecodeInstruction::new(
            Opcode::Cast,
            vec![dst as u8, self.operand_to_reg(&src)?, 0],
        ));

        Ok(Operand::Temp(dst))
    }

    /// 生成字段访问
    fn generate_field_access(
        &mut self,
        expr: &Expr,
        field: &str,
    ) -> Result<Operand, CodegenError> {
        let dst = self.next_temp();
        let obj = self.generate_expr(expr)?;
        let field_offset = self.get_field_offset(field);

        self.emit(BytecodeInstruction::new(
            Opcode::GetField,
            vec![
                dst as u8,
                self.operand_to_reg(&obj)?,
                (field_offset & 0xFF) as u8,
                (field_offset >> 8) as u8,
            ],
        ));

        Ok(Operand::Temp(dst))
    }

    /// 生成索引访问
    fn generate_index(
        &mut self,
        expr: &Expr,
        index: &Expr,
    ) -> Result<Operand, CodegenError> {
        let dst = self.next_temp();
        let array = self.generate_expr(expr)?;
        let idx = self.generate_expr(index)?;

        self.emit(BytecodeInstruction::new(
            Opcode::BoundsCheck,
            vec![self.operand_to_reg(&array)?, self.operand_to_reg(&idx)?],
        ));

        self.emit(BytecodeInstruction::new(
            Opcode::LoadElement,
            vec![
                dst as u8,
                self.operand_to_reg(&array)?,
                self.operand_to_reg(&idx)?,
            ],
        ));

        Ok(Operand::Temp(dst))
    }

    /// 生成 try 运算符
    fn generate_try(
        &mut self,
        expr: &Expr,
    ) -> Result<Operand, CodegenError> {
        let result_reg = self.generate_expr(expr)?;
        let continue_label = self.next_label();
        let _error_label = self.next_label();
        let type_check_reg = self.next_temp();

        self.emit(BytecodeInstruction::new(
            Opcode::TypeCheck,
            vec![self.operand_to_reg(&result_reg)?, 0, type_check_reg as u8],
        ));

        self.emit(BytecodeInstruction::new(
            Opcode::JmpIfNot,
            vec![type_check_reg as u8, continue_label as i16 as u8],
        ));

        let error_reg = self.next_temp();
        self.emit(BytecodeInstruction::new(
            Opcode::GetField,
            vec![error_reg as u8, self.operand_to_reg(&result_reg)?, 0, 0],
        ));

        self.emit(BytecodeInstruction::new(
            Opcode::ReturnValue,
            vec![error_reg as u8],
        ));

        self.emit(BytecodeInstruction::new(
            Opcode::Label,
            vec![continue_label as u8],
        ));

        let ok_value_reg = self.next_temp();
        self.emit(BytecodeInstruction::new(
            Opcode::GetField,
            vec![ok_value_reg as u8, self.operand_to_reg(&result_reg)?, 0, 0],
        ));

        Ok(Operand::Temp(ok_value_reg))
    }

    /// 生成 ref 表达式
    fn generate_ref(
        &mut self,
        expr: &Expr,
    ) -> Result<Operand, CodegenError> {
        let dst = self.next_temp();
        let src = self.generate_expr(expr)?;

        self.emit(BytecodeInstruction::new(
            Opcode::ArcNew,
            vec![dst as u8, self.operand_to_reg(&src)?],
        ));

        Ok(Operand::Temp(dst))
    }
}

/// 将字面量转换为常量值
fn literal_to_const_value(literal: &Literal) -> ConstValue {
    match literal {
        Literal::Int(n) => ConstValue::Int(*n),
        Literal::Float(f) => ConstValue::Float(*f),
        Literal::Bool(b) => ConstValue::Bool(*b),
        Literal::String(s) => ConstValue::String(s.clone()),
        Literal::Char(c) => ConstValue::Char(*c),
    }
}
