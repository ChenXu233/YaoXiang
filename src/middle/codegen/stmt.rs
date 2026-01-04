//! 语句代码生成
//!
//! 将语句转换为字节码指令

use super::{CodegenContext, CodegenError, BytecodeInstruction};
use crate::frontend::parser::ast::{Block, Expr, Param, Stmt, StmtKind, Type};
use crate::frontend::typecheck::MonoType;
use crate::middle::ir::{BasicBlock, FunctionIR, Instruction};
use crate::vm::opcode::TypedOpcode;

/// 语句代码生成实现
impl CodegenContext {
    /// 生成语句
    pub fn generate_stmt(&mut self, stmt: &Stmt) -> Result<(), CodegenError> {
        match &stmt.kind {
            StmtKind::Expr(expr) => {
                // 生成表达式语句
                self.generate_expr(expr)?;
                Ok(())
            }

            StmtKind::Var { name, type_annotation, initializer, is_mut } => {
                self.generate_var_decl(name, type_annotation.as_ref(), initializer.as_deref(), *is_mut)
            }

            StmtKind::For { var, iterable, body, label } => {
                // For 循环由 control_flow.rs 中的 generate_for_stmt 处理
                // 这里仅生成占位符，实际生成在表达式层面处理
                let _ = (var, iterable, body, label);
                Ok(())
            }

            StmtKind::TypeDef { name, definition } => {
                // 类型定义在编译时处理，不生成运行时指令
                self.register_type_definition(name, definition);
                Ok(())
            }

            StmtKind::Module { name: _, items } => {
                // 模块处理：收集所有定义
                for item in items {
                    self.generate_stmt(item)?;
                }
                Ok(())
            }

            StmtKind::Use { .. } => {
                // use 语句在解析时处理，不生成字节码
                Ok(())
            }

            _ => Err(CodegenError::UnimplementedStmt {
                stmt_type: format!("{:?}", stmt.kind),
            }),
        }
    }

    /// 生成变量声明
    fn generate_var_decl(
        &mut self,
        name: &str,
        type_annotation: Option<&Type>,
        initializer: Option<&Expr>,
        is_mut: bool,
    ) -> Result<(), CodegenError> {
        // 解析类型
        let ty = match type_annotation {
            Some(ta) => self.type_from_ast(ta),
            None => {
                // 从初始化表达式推断类型
                match initializer {
                    Some(init) => self.infer_expr_type(init)?,
                    None => MonoType::Int(64), // 默认类型
                }
            }
        };

        // 分配局部变量
        let local_idx = self.next_local();

        // 如果有初始化表达式，生成它并存储
        if let Some(init) = initializer {
            let src = self.generate_expr(init)?;

            // 根据类型决定分配方式
            let should_heap_allocate = self.should_heap_allocate_for_type(&ty);

            if should_heap_allocate {
                // 堆分配
                self.emit(BytecodeInstruction::new(
                    TypedOpcode::HeapAlloc,
                    vec![local_idx as u8],
                ));
            } else {
                // 栈分配
                self.emit(BytecodeInstruction::new(
                    TypedOpcode::StackAlloc,
                    vec![local_idx as u8],
                ));
            }

            // 存储初始值
            self.emit(BytecodeInstruction::new(
                TypedOpcode::StoreLocal,
                vec![self.operand_to_reg(&src)?, local_idx as u8],
            ));
        }

        // 注册符号
        self.symbol_table.insert(name.to_string(), super::Symbol {
            name: name.to_string(),
            ty: ty.clone(),
            storage: super::Storage::Local(local_idx),
            is_mut,
            scope_level: self.scope_level,
        });

        Ok(())
    }

    /// 检查类型是否需要堆分配
    fn should_heap_allocate_for_type(&self, ty: &MonoType) -> bool {
        // 大型对象或可能逃逸的对象需要堆分配
        match ty {
            MonoType::List(_) => true,
            MonoType::Dict(_, _) => true,
            MonoType::Set(_) => true,
            MonoType::Tuple(types) if types.len() > 2 => true,
            MonoType::Struct(_) => true,
            MonoType::Fn { .. } => true,
            _ => false,
        }
    }

    /// 从表达式推断类型
    fn infer_expr_type(&self, expr: &Expr) -> Result<MonoType, CodegenError> {
        match expr {
            Expr::Lit(literal, _) => Ok(self.infer_literal_type(literal)),
            Expr::Var(name, _) => {
                if let Some(symbol) = self.symbol_table.get(name) {
                    Ok(symbol.ty.clone())
                } else {
                    Err(CodegenError::SymbolNotFound { name: name.clone() })
                }
            }
            Expr::BinOp { left, .. } => self.infer_expr_type(left),
            Expr::Call { func, .. } => {
                let func_ty = self.infer_expr_type(func)?;
                match func_ty {
                    MonoType::Fn { return_type, .. } => Ok(*return_type),
                    _ => Err(CodegenError::TypeMismatch {
                        expected: "Function".to_string(),
                        found: format!("{:?}", func_ty),
                    }),
                }
            }
            _ => Ok(MonoType::Int(64)), // 默认推断为 Int64
        }
    }

    /// 从字面量推断类型
    fn infer_literal_type(&self, literal: &crate::frontend::lexer::tokens::Literal) -> MonoType {
        match literal {
            crate::frontend::lexer::tokens::Literal::Int(_) => MonoType::Int(64),
            crate::frontend::lexer::tokens::Literal::Float(_) => MonoType::Float(64),
            crate::frontend::lexer::tokens::Literal::Bool(_) => MonoType::Bool,
            crate::frontend::lexer::tokens::Literal::String(_) => MonoType::String,
            crate::frontend::lexer::tokens::Literal::Char(_) => MonoType::Char,
        }
    }

    /// 注册类型定义
    fn register_type_definition(
        &mut self,
        _name: &str,
        _definition: &Type,
    ) {
        // TODO: 实现类型定义注册
        // 类型定义需要添加到模块的 types 列表中
    }

    /// 生成函数定义
    pub fn generate_fn_def(
        &mut self,
        name: &str,
        params: &[Param],
        return_type: &Option<Type>,
        body: &Block,
    ) -> Result<(), CodegenError> {
        // 创建函数IR
        let mut func_ir = FunctionIR {
            name: name.to_string(),
            params: Vec::new(),
            return_type: self.type_from_ast_option(return_type),
            is_async: false,
            locals: Vec::new(),
            blocks: Vec::new(),
            entry: 0,
        };

        // 设置参数类型
        for param in params {
            func_ir.params.push(self.type_from_ast_option(&param.ty));
        }

        // 生成函数体
        self.generate_block_to_ir(body, &mut func_ir)?;

        // 添加到模块
        self.module.functions.push(func_ir);

        Ok(())
    }

    /// 从 AST 类型转换（处理 Option）
    fn type_from_ast_option(&self, ast_type: &Option<Type>) -> crate::frontend::typecheck::MonoType {
        match ast_type {
            Some(ty) => self.type_from_ast(ty),
            None => crate::frontend::typecheck::MonoType::Int(64),
        }
    }

    /// 生成块并填充到函数IR
    fn generate_block_to_ir(&mut self, block: &Block, func_ir: &mut FunctionIR) -> Result<(), CodegenError> {
        let mut current_block = BasicBlock {
            label: 0,
            instructions: Vec::new(),
            successors: Vec::new(),
        };

        for stmt in &block.stmts {
            match &stmt.kind {
                StmtKind::Expr(expr) => {
                    // 生成表达式
                    let _operand = self.generate_expr(expr)?;
                }
                _ => {
                    // 其他语句暂不处理
                }
            }
        }

        // 如果块有表达式，添加返回指令
        if let Some(expr) = &block.expr {
            let operand = self.generate_expr(expr)?;
            current_block.instructions.push(Instruction::Ret(Some(operand)));
        } else {
            current_block.instructions.push(Instruction::Ret(None));
        }

        func_ir.blocks.push(current_block);
        Ok(())
    }
}
