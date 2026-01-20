//! 闭包代码生成
//!
//! 处理闭包捕获和 Upvalue 生成。

use crate::middle::codegen::{BytecodeInstruction, CodegenContext, CodegenError};
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

        let upvalues = self.analyze_captures(&body)?;
        let func_id = self.module.functions.len() as u32;
        self.compile_function_body(&name, &params, return_type, is_async, func_id)?;

        let closure_reg = self.next_temp();
        let mut operands = vec![closure_reg as u8];
        operands.extend_from_slice(&func_id.to_le_bytes());
        operands.push(upvalues.len() as u8);
        self.emit(BytecodeInstruction::new(TypedOpcode::MakeClosure, operands));

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
    pub fn generate_close_upvalue(
        &mut self,
        reg: u8,
    ) {
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
        let mut operands = vec![];
        operands.extend_from_slice(&func_id.to_le_bytes());
        operands.push(0);
        operands.push(args.len() as u8);
        self.emit(BytecodeInstruction::new(TypedOpcode::TailCall, operands));
        Ok(())
    }

    /// 分析闭包捕获的变量
    fn analyze_captures(
        &self,
        body: &Block,
    ) -> Result<Vec<UpvalueInfo>, CodegenError> {
        let mut captures = Vec::new();
        let mut var_positions: HashMap<String, Operand> = HashMap::new();

        for (name, symbol) in self.symbols.symbol_table().iter() {
            var_positions.insert(
                name.clone(),
                match &symbol.storage {
                    super::super::Storage::Local(id) => Operand::Local(*id),
                    super::super::Storage::Arg(id) => Operand::Arg(*id),
                    super::super::Storage::Temp(id) => Operand::Temp(*id),
                    super::super::Storage::Global(id) => Operand::Global(*id),
                },
            );
        }

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
            Expr::UnOp { expr, .. } => self.collect_captures(expr, var_positions, captures),
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
            Expr::Block(block) => self.collect_captures_block(block, var_positions, captures),
            Expr::Return(Some(value), _) => self.collect_captures(value, var_positions, captures),
            Expr::Cast { expr, .. } => self.collect_captures(expr, var_positions, captures),
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
            Expr::FieldAccess { expr, .. } => self.collect_captures(expr, var_positions, captures),
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
            StmtKind::Expr(expr) => self.collect_captures(expr, var_positions, captures),
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
        let params: Vec<_> = params
            .iter()
            .map(|p| {
                p.ty.as_ref()
                    .map_or(Ok(crate::frontend::typecheck::MonoType::Void), |t| {
                        Ok(crate::frontend::typecheck::MonoType::from(t.clone()))
                    })
            })
            .collect::<Result<Vec<_>, _>>()?;

        let return_type = return_type
            .map_or(Ok(crate::frontend::typecheck::MonoType::Void), |t| {
                Ok(crate::frontend::typecheck::MonoType::from(t))
            })?;

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
