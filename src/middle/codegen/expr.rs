//! 表达式代码生成
//!
//! 将表达式转换为字节码指令。

use super::{CodegenContext, CodegenError};
use crate::frontend::lexer::tokens::Literal;
use crate::frontend::parser::ast::{BinOp, Expr, UnOp};
use crate::frontend::typecheck::check::infer_literal_type;
use crate::frontend::typecheck::MonoType;
use crate::middle::codegen::BytecodeInstruction;
use crate::middle::ir::{ConstValue, Operand};
use crate::vm::opcode::TypedOpcode;

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
                // 生成条件
                let _cond = self.generate_expr(condition)?;
                // 生成 then 分支
                let _ = self.generate_block(then_branch)?;
                // 生成 elif 分支
                for (_, elif_body) in elif_branches {
                    let _ = self.generate_block(elif_body)?;
                }
                // 生成 else 分支
                if let Some(else_b) = else_branch {
                    let _ = self.generate_block(else_b)?;
                }
                Ok(Operand::Temp(self.next_temp()))
            }
            Expr::While {
                condition, body, ..
            } => {
                // 生成条件
                let _cond = self.generate_expr(condition)?;
                // 生成循环体
                let _ = self.generate_block(body)?;
                Ok(Operand::Temp(self.next_temp()))
            }
            Expr::For {
                var: _var,
                iterable,
                body,
                ..
            } => {
                // 生成可迭代对象
                let _iter = self.generate_expr(iterable)?;
                // 生成循环体
                let _ = self.generate_block(body)?;
                Ok(Operand::Temp(self.next_temp()))
            }
            Expr::Match { expr, arms, .. } => {
                // 生成匹配表达式
                let _match = self.generate_expr(expr)?;
                // 生成每个臂
                for arm in arms {
                    let _ = self.generate_expr(&arm.body)?;
                }
                Ok(Operand::Temp(self.next_temp()))
            }
            Expr::Block(block) => self.generate_block(block),
            Expr::Return(_, _) => Err(CodegenError::UnimplementedExpr {
                expr_type: "Return".to_string(),
            }),
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
        }
    }

    /// 生成字面量
    fn generate_literal(
        &mut self,
        literal: &Literal,
    ) -> Result<Operand, CodegenError> {
        let dst = self.next_temp();
        let const_idx = self.add_constant(literal_to_const_value(literal));

        // 发射加载常量指令
        self.emit(BytecodeInstruction::new(
            TypedOpcode::LoadConst,
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
        // 查找符号
        if let Some(symbol) = self.symbol_table.get(name) {
            match symbol.storage {
                super::Storage::Local(id) => {
                    let dst = self.next_temp();
                    // 发射加载局部变量指令
                    self.emit(BytecodeInstruction::new(
                        TypedOpcode::LoadLocal,
                        vec![dst as u8, id as u8],
                    ));
                    Ok(Operand::Temp(dst))
                }
                super::Storage::Arg(id) => {
                    let dst = self.next_temp();
                    // 发射加载参数指令
                    self.emit(BytecodeInstruction::new(
                        TypedOpcode::LoadArg,
                        vec![dst as u8, id as u8],
                    ));
                    Ok(Operand::Temp(dst))
                }
                super::Storage::Temp(id) => Ok(Operand::Temp(id)),
                super::Storage::Global(id) => {
                    let dst = self.next_temp();
                    self.emit(BytecodeInstruction::new(
                        TypedOpcode::LoadLocal, // 使用 LoadLocal 加载全局（简化）
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
                if let Some(symbol) = self.symbol_table.get(name) {
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
            _ => Ok(MonoType::Int(64)), // Default fallback
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

        // 根据操作符和类型选择类型化指令
        let opcode = match (op, &left_type) {
            (BinOp::Add, MonoType::Int(64)) => TypedOpcode::I64Add,
            (BinOp::Add, MonoType::Float(64)) => TypedOpcode::F64Add,
            (BinOp::Sub, MonoType::Int(64)) => TypedOpcode::I64Sub,
            (BinOp::Sub, MonoType::Float(64)) => TypedOpcode::F64Sub,
            (BinOp::Mul, MonoType::Int(64)) => TypedOpcode::I64Mul,
            (BinOp::Mul, MonoType::Float(64)) => TypedOpcode::F64Mul,
            (BinOp::Div, MonoType::Int(64)) => TypedOpcode::I64Div,
            (BinOp::Div, MonoType::Float(64)) => TypedOpcode::F64Div,
            (BinOp::Mod, MonoType::Int(64)) => TypedOpcode::I64Rem,

            // Comparisons
            (BinOp::Eq, MonoType::Int(64)) => TypedOpcode::I64Eq,
            (BinOp::Eq, MonoType::Float(64)) => TypedOpcode::F64Eq,
            (BinOp::Neq, MonoType::Int(64)) => TypedOpcode::I64Ne,
            (BinOp::Neq, MonoType::Float(64)) => TypedOpcode::F64Ne,
            (BinOp::Lt, MonoType::Int(64)) => TypedOpcode::I64Lt,
            (BinOp::Lt, MonoType::Float(64)) => TypedOpcode::F64Lt,
            (BinOp::Le, MonoType::Int(64)) => TypedOpcode::I64Le,
            (BinOp::Le, MonoType::Float(64)) => TypedOpcode::F64Le,
            (BinOp::Gt, MonoType::Int(64)) => TypedOpcode::I64Gt,
            (BinOp::Gt, MonoType::Float(64)) => TypedOpcode::F64Gt,
            (BinOp::Ge, MonoType::Int(64)) => TypedOpcode::I64Ge,
            (BinOp::Ge, MonoType::Float(64)) => TypedOpcode::F64Ge,

            // And/Or 在表达式中使用 I64Mul/I64Add 近似
            // 注意：真正的短路求值在 if 条件中通过控制流实现
            (BinOp::And, _) => TypedOpcode::I64Mul,
            (BinOp::Or, _) => TypedOpcode::I64Add,
            (BinOp::Range, _) => TypedOpcode::NewListWithCap,
            (BinOp::Assign, _) => return self.generate_assignment(left, right),

            // Fallback for other types (e.g. I32, F32) or mismatches
            _ => TypedOpcode::I64Add,
        };

        // 发射二元运算指令
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
        // 生成值表达式
        let src = self.generate_expr(value)?;

        match target {
            Expr::Var(name, _) => {
                if let Some(symbol) = self.symbol_table.get(name) {
                    match symbol.storage {
                        super::Storage::Local(id) => {
                            self.emit(BytecodeInstruction::new(
                                TypedOpcode::StoreLocal,
                                vec![self.operand_to_reg(&src)?, id as u8],
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
                    TypedOpcode::StoreElement,
                    vec![
                        self.operand_to_reg(&array)?,
                        self.operand_to_reg(&idx)?,
                        self.operand_to_reg(&src)?,
                    ],
                ));
            }
            Expr::FieldAccess { expr, field, .. } => {
                let obj = self.generate_expr(expr)?;
                // 假设字段偏移是字段名的哈希值（简化）
                let field_offset = self.get_field_offset(field);
                self.emit(BytecodeInstruction::new(
                    TypedOpcode::SetField,
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
    fn get_field_offset(
        &self,
        field: &str,
    ) -> u16 {
        // 简化：使用字段名的哈希值作为偏移
        use std::collections::hash_map::DefaultHasher;
        let mut hasher = DefaultHasher::new();
        std::hash::Hash::hash(field, &mut hasher);
        let hash = std::hash::Hasher::finish(&hasher);
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
                    TypedOpcode::I64Neg,
                    vec![dst as u8, self.operand_to_reg(&src)?],
                ));
            }
            UnOp::Pos => {
                self.emit(BytecodeInstruction::new(
                    TypedOpcode::Mov,
                    vec![dst as u8, self.operand_to_reg(&src)?],
                ));
            }
            UnOp::Not => {
                // !a 等价于 (a == 0)
                // 加载 0 常量
                let zero_reg = self.next_temp();
                self.emit(BytecodeInstruction::new(
                    TypedOpcode::I64Const,
                    vec![zero_reg as u8, 0, 0, 0, 0, 0, 0, 0, 0],
                ));
                // 比较 a == 0
                self.emit(BytecodeInstruction::new(
                    TypedOpcode::I64Eq,
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

        // 生成参数
        let mut arg_regs = Vec::new();
        for arg in args {
            let arg_reg = self.generate_expr(arg)?;
            arg_regs.push(self.operand_to_reg(&arg_reg)?);
        }

        // 参数寄存器从下一个临时寄存器开始
        let base_arg_reg = self.next_temp() as u8;
        // 保留更多寄存器给参数
        for _ in 1..arg_regs.len() {
            self.next_temp();
        }
        // 将参数移动到连续寄存器
        for (i, &arg_reg) in arg_regs.iter().enumerate() {
            let target_reg = base_arg_reg + i as u8;
            if arg_reg != target_reg {
                self.emit(BytecodeInstruction::new(
                    TypedOpcode::Mov,
                    vec![target_reg, arg_reg],
                ));
            }
        }

        match func {
            Expr::Var(name, _) => {
                // 静态函数调用
                // CallStatic: dst(1), func_id(u32, 4字节), base_arg_reg(1), arg_count(1)
                let func_idx = self.function_indices.get(name).copied().unwrap_or(0);
                let mut operands = vec![dst as u8];
                operands.extend_from_slice(&func_idx.to_le_bytes());
                operands.push(base_arg_reg);
                operands.push(arg_regs.len() as u8);
                self.emit(BytecodeInstruction::new(TypedOpcode::CallStatic, operands));
            }
            Expr::FieldAccess { expr, field, .. } => {
                // 方法调用（虚表分发）
                // CallVirt: dst(1), obj_reg(1), vtable_idx(u16, 2字节), base_arg_reg(1), arg_count(1)
                let obj = self.generate_expr(expr)?;
                let field_offset = self.get_field_offset(field);
                let mut operands = vec![dst as u8];
                operands.push(self.operand_to_reg(&obj)?);
                operands.extend_from_slice(&field_offset.to_le_bytes());
                operands.push(base_arg_reg);
                operands.push(arg_regs.len() as u8);
                self.emit(BytecodeInstruction::new(TypedOpcode::CallVirt, operands));
            }
            _ => {
                // 动态调用
                // CallDyn: dst(1), obj_reg(1), name_idx(u16, 2字节), base_arg_reg(1), arg_count(1)
                let name_idx = self.add_constant(ConstValue::String(format!("{:?}", func)));
                let mut operands = vec![dst as u8];
                operands.push(self.operand_to_reg(&Operand::Temp(0))?);
                operands.extend_from_slice(&(name_idx as u16).to_le_bytes());
                operands.push(base_arg_reg);
                operands.push(arg_regs.len() as u8);
                self.emit(BytecodeInstruction::new(TypedOpcode::CallDyn, operands));
            }
        }

        Ok(Operand::Temp(dst))
    }

    /// 生成元组
    fn generate_tuple(
        &mut self,
        _exprs: &[Expr],
    ) -> Result<Operand, CodegenError> {
        let dst = self.next_temp();
        // TODO: 实现元组代码生成
        Ok(Operand::Temp(dst))
    }

    /// 生成列表
    fn generate_list(
        &mut self,
        exprs: &[Expr],
    ) -> Result<Operand, CodegenError> {
        let dst = self.next_temp();

        // 预分配列表
        self.emit(BytecodeInstruction::new(
            TypedOpcode::NewListWithCap,
            vec![dst as u8, exprs.len() as u8],
        ));

        // 存储元素
        for (i, elem) in exprs.iter().enumerate() {
            let elem_reg = self.generate_expr(elem)?;
            self.emit(BytecodeInstruction::new(
                TypedOpcode::StoreElement,
                vec![dst as u8, i as u8, self.operand_to_reg(&elem_reg)?],
            ));
        }

        Ok(Operand::Temp(dst))
    }

    /// 生成字典
    fn generate_dict(
        &mut self,
        _pairs: &[(Expr, Expr)],
    ) -> Result<Operand, CodegenError> {
        let dst = self.next_temp();
        // TODO: 实现字典代码生成
        Ok(Operand::Temp(dst))
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
            TypedOpcode::Cast,
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
            TypedOpcode::GetField,
            vec![dst as u8, self.operand_to_reg(&obj)?, field_offset as u8],
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
            TypedOpcode::LoadElement,
            vec![
                dst as u8,
                self.operand_to_reg(&array)?,
                self.operand_to_reg(&idx)?,
            ],
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
