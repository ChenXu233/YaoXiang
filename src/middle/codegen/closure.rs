//! 闭包代码生成
//!
//! 处理闭包捕获和 Upvalue 生成。

use super::{BytecodeInstruction, CodegenContext, CodegenError};
use crate::frontend::parser::ast::{Block, Expr, Param};
use crate::middle::ir::{FunctionIR, Operand};
use crate::vm::opcode::TypedOpcode;
use std::collections::HashMap;

impl CodegenContext {
    /// 生成闭包表达式
    pub fn generate_closure_expr(
        &mut self,
        func_expr: &Expr,
    ) -> Result<Operand, CodegenError> {
        // 1. 提取 FnDef 表达式
        let (name, params, return_type, body, is_async) = match func_expr {
            Expr::FnDef {
                name,
                params,
                return_type,
                body,
                is_async,
                ..
            } => (
                name.clone(),
                params.clone(),
                return_type.clone(),
                body.clone(),
                *is_async,
            ),
            _ => {
                return Err(CodegenError::UnimplementedExpr {
                    expr_type: "Non-function closure".to_string(),
                });
            }
        };

        // 2. 分析捕获变量 (Upvalues)
        let upvalues = self.analyze_captures(&body)?;

        // 3. 编译闭包函数体为独立的 FunctionIR
        let func_id = self.module.functions.len() as u32;
        self.compile_function_body(&name, &params, return_type, is_async, func_id)?;

        // 4. 生成 MakeClosure 指令
        // 操作数：dst(1), func_id(u32, 4字节), upvalue_count(1)
        let closure_reg = self.next_temp();
        let mut operands = vec![closure_reg as u8];
        operands.extend_from_slice(&func_id.to_le_bytes());
        operands.push(upvalues.len() as u8);
        self.emit(BytecodeInstruction::new(TypedOpcode::MakeClosure, operands));

        // 5. 填充 Upvalues
        // StoreUpvalue: src, upvalue_idx (2 个操作数)
        for (i, upvalue) in upvalues.iter().enumerate() {
            let src_reg = self.operand_to_reg(&upvalue.source)?;
            self.emit(BytecodeInstruction::new(
                TypedOpcode::StoreUpvalue,
                vec![src_reg, i as u8],
            ));
        }

        Ok(Operand::Temp(closure_reg))
    }

    /// 生成 CloseUpvalue 指令
    ///
    /// 将栈上的局部变量搬迁到堆上，使闭包可以在函数返回后访问
    pub fn generate_close_upvalue(
        &mut self,
        reg: u8,
    ) {
        // CloseUpvalue: reg (1 个操作数)
        self.emit(BytecodeInstruction::new(
            TypedOpcode::CloseUpvalue,
            vec![reg],
        ));
    }

    /// 生成尾调用优化
    pub fn generate_tail_call(
        &mut self,
        func_id: u32,
        args: &[Operand],
    ) -> Result<(), CodegenError> {
        // TailCall: func_id(u32, 4字节), base_arg_reg(1), arg_count(1)
        let mut operands = vec![];
        operands.extend_from_slice(&func_id.to_le_bytes());
        operands.push(0); // base_arg_reg - 简化处理
        operands.push(args.len() as u8);
        self.emit(BytecodeInstruction::new(TypedOpcode::TailCall, operands));
        Ok(())
    }

    /// 分析闭包捕获的变量
    ///
    /// 遍历函数体，找出所有引用的非局部变量
    fn analyze_captures(
        &self,
        body: &Block,
    ) -> Result<Vec<UpvalueInfo>, CodegenError> {
        let mut captures = Vec::new();
        let mut var_positions: HashMap<String, Operand> = HashMap::new();

        // 收集当前作用域的变量
        for (name, symbol) in &self.symbol_table.symbols {
            var_positions.insert(
                name.clone(),
                match symbol.storage {
                    super::Storage::Local(id) => Operand::Local(id),
                    super::Storage::Arg(id) => Operand::Arg(id),
                    super::Storage::Temp(id) => Operand::Temp(id),
                    super::Storage::Global(id) => Operand::Global(id),
                },
            );
        }

        // 递归收集函数体中引用的变量
        self.collect_captures_block(body, &var_positions, &mut captures);

        Ok(captures)
    }

    /// 递归收集捕获的变量
    fn collect_captures(
        &self,
        expr: &Expr,
        var_positions: &HashMap<String, Operand>,
        captures: &mut Vec<UpvalueInfo>,
    ) {
        match expr {
            Expr::Var(name, _) => {
                if let Some(source) = var_positions.get(name) {
                    if !captures.iter().any(|u| u.source == *source) {
                        let is_local = matches!(source, Operand::Local(_));
                        captures.push(UpvalueInfo {
                            is_local,
                            index: captures.len() as u8,
                            source: source.clone(),
                        });
                    }
                }
            }
            Expr::BinOp { left, right, .. } => {
                self.collect_captures(left, var_positions, captures);
                self.collect_captures(right, var_positions, captures);
            }
            Expr::UnOp { expr, .. } => {
                self.collect_captures(expr, var_positions, captures);
            }
            Expr::Call { func, args, .. } => {
                self.collect_captures(func, var_positions, captures);
                for arg in args {
                    self.collect_captures(arg, var_positions, captures);
                }
            }
            Expr::If {
                condition,
                then_branch,
                elif_branches,
                else_branch,
                ..
            } => {
                self.collect_captures(condition, var_positions, captures);
                self.collect_captures_block(then_branch, var_positions, captures);
                for (_, elif_body) in elif_branches {
                    self.collect_captures_block(elif_body, var_positions, captures);
                }
                if let Some(else_b) = else_branch {
                    self.collect_captures_block(else_b, var_positions, captures);
                }
            }
            Expr::While {
                condition, body, ..
            } => {
                self.collect_captures(condition, var_positions, captures);
                self.collect_captures_block(body, var_positions, captures);
            }
            Expr::For { iterable, body, .. } => {
                self.collect_captures(iterable, var_positions, captures);
                self.collect_captures_block(body, var_positions, captures);
            }
            Expr::Match { expr, arms, .. } => {
                self.collect_captures(expr, var_positions, captures);
                for arm in arms {
                    self.collect_captures(&arm.body, var_positions, captures);
                }
            }
            Expr::Block(block) => {
                self.collect_captures_block(block, var_positions, captures);
            }
            Expr::Return(Some(value), _) => {
                self.collect_captures(value, var_positions, captures);
            }
            Expr::Cast { expr, .. } => {
                self.collect_captures(expr, var_positions, captures);
            }
            Expr::Tuple(exprs, _) => {
                for e in exprs {
                    self.collect_captures(e, var_positions, captures);
                }
            }
            Expr::List(exprs, _) => {
                for e in exprs {
                    self.collect_captures(e, var_positions, captures);
                }
            }
            Expr::Dict(pairs, _) => {
                for (k, v) in pairs {
                    self.collect_captures(k, var_positions, captures);
                    self.collect_captures(v, var_positions, captures);
                }
            }
            Expr::Index { expr, index, .. } => {
                self.collect_captures(expr, var_positions, captures);
                self.collect_captures(index, var_positions, captures);
            }
            Expr::FieldAccess { expr, .. } => {
                self.collect_captures(expr, var_positions, captures);
            }
            _ => {}
        }
    }

    /// 收集 Block 中的捕获变量
    fn collect_captures_block(
        &self,
        block: &Block,
        var_positions: &HashMap<String, Operand>,
        captures: &mut Vec<UpvalueInfo>,
    ) {
        for stmt in &block.stmts {
            self.collect_captures_stmt(stmt, var_positions, captures);
        }
        if let Some(expr) = &block.expr {
            self.collect_captures(expr, var_positions, captures);
        }
    }

    /// 收集语句中的捕获变量
    fn collect_captures_stmt(
        &self,
        stmt: &crate::frontend::parser::ast::Stmt,
        var_positions: &HashMap<String, Operand>,
        captures: &mut Vec<UpvalueInfo>,
    ) {
        use crate::frontend::parser::ast::StmtKind;

        match &stmt.kind {
            StmtKind::Expr(expr) => {
                self.collect_captures(expr, var_positions, captures);
            }
            StmtKind::Var { initializer, .. } => {
                if let Some(init) = initializer {
                    self.collect_captures(init, var_positions, captures);
                }
            }
            StmtKind::For { iterable, body, .. } => {
                self.collect_captures(iterable, var_positions, captures);
                self.collect_captures_block(body, var_positions, captures);
            }
            StmtKind::TypeDef { .. } => {}
            StmtKind::Module { items, .. } => {
                for item in items {
                    self.collect_captures_stmt(item, var_positions, captures);
                }
            }
            StmtKind::Use { .. } => {}
            StmtKind::Fn { body, .. } => {
                let (stmts, expr) = body;
                for stmt in stmts {
                    self.collect_captures_stmt(stmt, var_positions, captures);
                }
                if let Some(e) = expr {
                    self.collect_captures(e, var_positions, captures);
                }
            }
        }
    }

    /// 编译函数体
    fn compile_function_body(
        &mut self,
        name: &str,
        params: &[Param],
        return_type: Option<crate::frontend::parser::ast::Type>,
        is_async: bool,
        func_id: u32,
    ) -> Result<(), CodegenError> {
        // 转换参数类型
        let params: Vec<_> = params
            .iter()
            .map(|p| {
                p.ty.as_ref()
                    .map_or(Ok(crate::frontend::typecheck::MonoType::Void), |t| {
                        Ok(crate::frontend::typecheck::MonoType::from(t.clone()))
                    })
            })
            .collect::<Result<Vec<_>, _>>()?;

        // 转换返回类型
        let return_type = return_type
            .map_or(Ok(crate::frontend::typecheck::MonoType::Void), |t| {
                Ok(crate::frontend::typecheck::MonoType::from(t))
            })?;

        // 创建函数 IR
        let func_ir = FunctionIR {
            name: if name.is_empty() {
                format!("closure_{}", func_id)
            } else {
                name.to_string()
            },
            params,
            return_type,
            is_async,
            locals: Vec::new(),
            blocks: Vec::new(),
            entry: 0,
        };

        // 添加到模块
        self.module.functions.push(func_ir);

        Ok(())
    }
}

/// Upvalue 信息
#[derive(Debug, Clone)]
struct UpvalueInfo {
    is_local: bool,
    index: u8,
    source: Operand,
}
