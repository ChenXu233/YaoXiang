//! 表达式代码生成
//!
//! 将表达式转换为字节码指令。

use super::{CodegenContext, CodegenError};
use crate::frontend::lexer::tokens::Literal;
use crate::frontend::parser::ast::{BinOp, Block, Expr, UnOp};
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
                let match_expr = Block {
                    stmts: Vec::new(),
                    expr: Some(Box::new(expr.as_ref().clone())),
                    span: Default::default(),
                };
                // 生成 match 表达式代码
                self.generate_match_stmt(&match_expr, arms)?;
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
    pub(super) fn get_field_offset(
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
    /// 元组是匿名结构体，字段用数字索引 (0, 1, 2, ...)
    /// (1, "hello") 生成：HeapAlloc -> SetField(0, 1) -> SetField(1, "hello")
    fn generate_tuple(
        &mut self,
        exprs: &[Expr],
    ) -> Result<Operand, CodegenError> {
        if exprs.is_empty() {
            // 空元组 ()
            let dst = self.next_temp();
            return Ok(Operand::Temp(dst));
        }

        let dst = self.next_temp();

        // 预分配元组结构体（使用 HeapAlloc，type_id=0 表示匿名元组）
        // HeapAlloc: dst(1), type_id(u16, 2字节)
        self.emit(BytecodeInstruction::new(
            TypedOpcode::HeapAlloc,
            vec![dst as u8, 0, 0], // type_id=0 表示元组
        ));

        // 设置每个字段（字段偏移 = 索引）
        for (i, elem) in exprs.iter().enumerate() {
            let elem_reg = self.generate_expr(elem)?;
            let field_offset = i as u16;
            self.emit(BytecodeInstruction::new(
                TypedOpcode::SetField,
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

        // 预分配列表
        // NewListWithCap: dst(1), capacity(u16, 2字节)
        let cap = exprs.len();
        self.emit(BytecodeInstruction::new(
            TypedOpcode::NewListWithCap,
            vec![dst as u8, (cap & 0xFF) as u8, (cap >> 8) as u8],
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
    /// 字典字面量 {"key1": val1, "key2": val2} 生成：
    /// 1. CallStatic Dict.new() 创建空字典
    /// 2. 对每个键值对调用 Dict.insert(dict, key, value)
    fn generate_dict(
        &mut self,
        pairs: &[(Expr, Expr)],
    ) -> Result<Operand, CodegenError> {
        if pairs.is_empty() {
            // 空字典 {}
            let dst = self.next_temp();
            return Ok(Operand::Temp(dst));
        }

        // 首先创建空字典
        let dict_reg = self.next_temp();

        // 尝试调用 Dict.new（使用 function_indices，如果不存在则用动态调用）
        if let Some(&dict_new_idx) = self.function_indices.get("Dict.new") {
            // CallStatic: dst(1), func_id(4), base_arg_reg(1), arg_count(1)
            self.emit(BytecodeInstruction::new(
                TypedOpcode::CallStatic,
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
            // 动态调用 Dict.new（使用常量池中的函数名）
            let name_idx = self.add_constant(ConstValue::String("Dict.new".to_string()));
            self.emit(BytecodeInstruction::new(
                TypedOpcode::CallDyn,
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

        // 对每个键值对调用 Dict.insert(dict, key, value)
        for (key, value) in pairs {
            let key_reg = self.generate_expr(key)?;
            let value_reg = self.generate_expr(value)?;

            // 准备参数：dict, key, value
            let base_arg = self.next_temp() as u8;
            self.next_temp(); // key
            self.next_temp(); // value

            // 移动参数到连续寄存器
            self.emit(BytecodeInstruction::new(
                TypedOpcode::Mov,
                vec![base_arg, dict_reg as u8],
            ));
            self.emit(BytecodeInstruction::new(
                TypedOpcode::Mov,
                vec![base_arg + 1, self.operand_to_reg(&key_reg)?],
            ));
            self.emit(BytecodeInstruction::new(
                TypedOpcode::Mov,
                vec![base_arg + 2, self.operand_to_reg(&value_reg)?],
            ));

            // 调用 Dict.insert
            if let Some(&dict_insert_idx) = self.function_indices.get("Dict.insert") {
                self.emit(BytecodeInstruction::new(
                    TypedOpcode::CallStatic,
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
                // 动态调用
                let name_idx = self.add_constant(ConstValue::String("Dict.insert".to_string()));
                self.emit(BytecodeInstruction::new(
                    TypedOpcode::CallDyn,
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
        // GetField: dst(1), obj_reg(1), field_offset(u16, 2字节)
        let field_offset = self.get_field_offset(field);

        self.emit(BytecodeInstruction::new(
            TypedOpcode::GetField,
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

        // BoundsCheck: array_reg, index_reg
        self.emit(BytecodeInstruction::new(
            TypedOpcode::BoundsCheck,
            vec![self.operand_to_reg(&array)?, self.operand_to_reg(&idx)?],
        ));

        // LoadElement: dst, array_reg, index_reg
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

    /// 生成 try 运算符（错误传播）`expr?`
    ///
    /// 生成与 `match expr { Ok(v) => v, Err(e) => return Err(e) }` 相同的字节码：
    /// 1. 生成内部表达式
    /// 2. TypeCheck 检查是否为 Err/None
    /// 3. JmpIfNot 跳过错误处理（成功路径）
    /// 4. GetField 提取错误值
    /// 5. ReturnValue 返回错误
    fn generate_try(
        &mut self,
        expr: &Expr,
    ) -> Result<Operand, CodegenError> {
        // 生成内部表达式
        let result_reg = self.generate_expr(expr)?;

        // 创建标签
        let continue_label = self.next_label();
        let _error_label = self.next_label(); // 用于错误路径（当前不需要特殊处理）

        // TypeCheck: 检查是否为 Err 变体
        // TypeCheck: obj_reg, type_id, dst
        let type_check_reg = self.next_temp();
        self.emit(BytecodeInstruction::new(
            TypedOpcode::TypeCheck,
            vec![
                self.operand_to_reg(&result_reg)?,
                0, // type_id=0 表示 Err (TODO: 需要正确的 type_id)
                type_check_reg as u8,
            ],
        ));

        // JmpIfNot: 如果不是 Err，继续执行（成功路径）
        self.emit(BytecodeInstruction::new(
            TypedOpcode::JmpIfNot,
            vec![type_check_reg as u8, continue_label as i16 as u8],
        ));

        // 是 Err，提取错误值
        let error_reg = self.next_temp();
        self.emit(BytecodeInstruction::new(
            TypedOpcode::GetField,
            vec![
                error_reg as u8,
                self.operand_to_reg(&result_reg)?,
                0, // field_offset=0 表示 error 字段 (TODO: 需要正确的 offset)
                0,
            ],
        ));

        // ReturnValue: 返回错误值（提前返回）
        self.emit(BytecodeInstruction::new(
            TypedOpcode::ReturnValue,
            vec![error_reg as u8],
        ));

        // 成功路径标签
        self.emit(BytecodeInstruction::new(
            TypedOpcode::Label,
            vec![continue_label as u8],
        ));

        // 返回 Ok 中的值（成功路径）
        // GetField: dst, obj_reg, field_offset
        let ok_value_reg = self.next_temp();
        self.emit(BytecodeInstruction::new(
            TypedOpcode::GetField,
            vec![
                ok_value_reg as u8,
                self.operand_to_reg(&result_reg)?,
                0, // field_offset=0 表示 value 字段 (TODO: 需要正确的 offset)
                0,
            ],
        ));

        Ok(Operand::Temp(ok_value_reg))
    }

    /// 生成 ref 表达式：`ref expr` 创建 Arc
    ///
    /// ArcNew: dst, src
    /// - 分配新的 Arc 结构（包含指针和原子计数）
    /// - 将 src 的值复制到 Arc 内部
    /// - 引用计数初始化为 1
    fn generate_ref(
        &mut self,
        expr: &Expr,
    ) -> Result<Operand, CodegenError> {
        let dst = self.next_temp();
        let src = self.generate_expr(expr)?;

        // ArcNew: dst(1), src(1)
        self.emit(BytecodeInstruction::new(
            TypedOpcode::ArcNew,
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
