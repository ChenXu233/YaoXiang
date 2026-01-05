//! 语句和函数类型检查
//!
//! 实现语句的类型检查和函数定义的类型验证

use super::super::lexer::tokens::Literal;
use super::super::parser::ast;
use super::errors::{TypeError, TypeResult};
use super::infer::TypeInferrer;
use super::types::{MonoType, PolyType, TypeConstraintSolver};
use crate::middle;
use crate::util::span::Span;
use std::collections::HashMap;

/// 类型检查器
///
/// 负责检查模块、函数和语句的类型正确性
#[derive(Debug)]
pub struct TypeChecker<'a> {
    /// 类型推断器
    inferrer: TypeInferrer<'a>,
    /// 已检查的函数签名（用于递归检测）
    checked_functions: HashMap<String, bool>,
    /// 当前函数的返回类型
    current_return_type: Option<MonoType>,
    /// 泛型函数缓存
    generic_cache: HashMap<String, HashMap<String, PolyType>>,
    /// 收集的错误
    errors: Vec<TypeError>,
}

impl<'a> TypeChecker<'a> {
    /// 创建新的类型检查器
    pub fn new(solver: &'a mut TypeConstraintSolver) -> Self {
        TypeChecker {
            inferrer: TypeInferrer::new(solver),
            checked_functions: HashMap::new(),
            current_return_type: None,
            generic_cache: HashMap::new(),
            errors: Vec::new(),
        }
    }

    /// 获取错误列表
    pub fn errors(&self) -> &[TypeError] {
        &self.errors
    }

    /// 检查是否有错误
    pub fn has_errors(&self) -> bool {
        !self.errors.is_empty()
    }

    /// 添加变量绑定
    pub fn add_var(&mut self, name: String, poly: PolyType) {
        self.inferrer.add_var(name, poly);
    }

    /// 添加错误
    fn add_error(&mut self, error: TypeError) {
        self.errors.push(error);
    }

    // =========================================================================
    // 模块检查
    // =========================================================================

    /// 检查整个模块
    pub fn check_module(
        &mut self,
        module: &ast::Module,
    ) -> Result<middle::ModuleIR, Vec<TypeError>> {
        // 首先收集所有类型定义
        for stmt in &module.items {
            if let ast::StmtKind::TypeDef { name, definition } = &stmt.kind {
                self.add_type_definition(name, definition, stmt.span);
            }
        }

        // 然后检查所有语句
        for stmt in &module.items {
            if let Err(e) = self.check_stmt(stmt) {
                self.add_error(e);
            }
        }

        // 求解所有约束
        self.inferrer.solver().solve().map_err(|e| {
            e.into_iter()
                .map(|e| TypeError::TypeMismatch {
                    expected: e.error.left,
                    found: e.error.right,
                    span: e.span,
                })
                .collect::<Vec<_>>()
        })?;

        // 如果有错误，返回所有错误
        if self.has_errors() {
            return Err(self.errors.clone());
        }

        // 生成 IR（简化版本）
        self.generate_module_ir(module)
    }

    /// 添加类型定义
    fn add_type_definition(&mut self, name: &str, definition: &ast::Type, _span: Span) {
        let poly = PolyType::mono(MonoType::from(definition.clone()));
        self.inferrer.add_var(name.to_string(), poly);
    }

    // =========================================================================
    // 语句检查
    // =========================================================================

    /// 检查语句
    #[allow(clippy::result_large_err)]
    pub fn check_stmt(&mut self, stmt: &ast::Stmt) -> TypeResult<()> {
        match &stmt.kind {
            ast::StmtKind::Expr(expr) => {
                if let ast::Expr::FnDef {
                    name,
                    params,
                    return_type,
                    body,
                    is_async,
                    span: _,
                } = expr.as_ref()
                {
                    // 有返回类型标注表示有完整类型签名
                    let is_annotated = return_type.is_some();
                    self.check_fn_def(
                        name,
                        params,
                        return_type.as_ref(),
                        body,
                        *is_async,
                        None,
                        is_annotated,
                    )?;
                    Ok(())
                } else {
                    self.inferrer.infer_expr(expr)?;
                    Ok(())
                }
            }
            ast::StmtKind::Fn {
                name,
                type_annotation,
                params,
                body: (stmts, expr),
            } => {
                eprintln!("[CHECK] check_stmt: processing Fn for '{}'", name);
                eprintln!(
                    "[DEBUG] type_annotation is_some: {}",
                    type_annotation.is_some()
                );
                if let Some(ann) = type_annotation {
                    eprintln!("[DEBUG] type_annotation: {:?}", ann);
                }

                // 1. Extract params and return type from annotation if available
                let (annotated_params, annotated_return) = if let Some(ast::Type::Fn {
                    params,
                    return_type,
                    ..
                }) = type_annotation
                {
                    (Some(params), Some(return_type.as_ref()))
                } else {
                    (None, None)
                };

                // 2. Prepare parameter types for the function signature
                let param_types: Vec<MonoType> = if let Some(a_params) = annotated_params {
                    if a_params.len() == params.len() {
                        a_params.iter().map(|t| MonoType::from(t.clone())).collect()
                    } else {
                        // Mismatch in count, fallback to params or error?
                        // For now, fallback to inferring from params if counts don't match,
                        // but ideally this should be an error caught by parser/validation.
                        // Given parser checks count, we can assume they match or use params.
                        params
                            .iter()
                            .map(|p| {
                                if let Some(ty) = &p.ty {
                                    MonoType::from(ty.clone())
                                } else {
                                    self.inferrer.solver().new_var()
                                }
                            })
                            .collect()
                    }
                } else {
                    params
                        .iter()
                        .map(|p| {
                            if let Some(ty) = &p.ty {
                                MonoType::from(ty.clone())
                            } else {
                                self.inferrer.solver().new_var()
                            }
                        })
                        .collect()
                };

                // 3. Prepare return type for the function signature
                // Handle "_" as "inferred" (None)
                let expected_return_type = if let Some(ty) = annotated_return {
                    if let ast::Type::Name(n) = ty {
                        if n == "_" {
                            None
                        } else {
                            Some(ty)
                        }
                    } else {
                        Some(ty)
                    }
                } else {
                    None
                };

                let return_ty = if let Some(ty) = expected_return_type {
                    MonoType::from(ty.clone())
                } else {
                    self.inferrer.solver().new_var()
                };

                // 4. Construct the inferred function type
                let fn_type = MonoType::Fn {
                    params: param_types.clone(),
                    return_type: Box::new(return_ty),
                    is_async: false,
                };

                // 5. If there is an annotation, constrain the inferred type to it
                if let Some(ann) = type_annotation {
                    let ann_ty = MonoType::from(ann.clone());
                    self.inferrer
                        .solver()
                        .add_constraint(fn_type.clone(), ann_ty, stmt.span);
                }

                // 6. Register the function in the scope
                self.inferrer.add_var(name.clone(), PolyType::mono(fn_type));

                let body = ast::Block {
                    stmts: stmts.clone(),
                    expr: expr.clone(),
                    span: stmt.span,
                };

                // Pass the constrained param_types to check_fn_def so inner scope matches outer signature
                eprintln!(
                    "[DEBUG] Calling check_fn_def from check_stmt. annotated_params.is_some(): {}",
                    annotated_params.is_some()
                );
                self.check_fn_def(
                    name,
                    params,
                    expected_return_type,
                    &body,
                    false,
                    Some(param_types),
                    annotated_params.is_some(),
                )?;
                Ok(())
            }
            ast::StmtKind::Var {
                name,
                type_annotation,
                initializer,
                is_mut: _,
            } => self.check_var(
                name,
                type_annotation.as_ref(),
                initializer.as_deref(),
                stmt.span,
            ),
            ast::StmtKind::For {
                var,
                iterable,
                body,
                ..
            } => {
                // 类型检查 for 循环
                // 1. 推断 iterable 的类型（应该是可迭代的）
                let _iter_ty = self.inferrer.infer_expr(iterable)?;

                // 2. 在循环体中，var 的类型取决于 iterable 的元素类型
                // 暂时添加一个类型变量
                let var_ty = self.inferrer.solver().new_var();
                let poly = PolyType::mono(var_ty);
                self.inferrer.add_var(var.clone(), poly);

                // 3. 类型检查循环体
                let _body_ty = self.inferrer.infer_block(body, false, None)?;
                Ok(())
            }
            ast::StmtKind::TypeDef { name, definition } => {
                self.check_type_def(name, definition, stmt.span)
            }
            ast::StmtKind::Module { name, items } => {
                self.check_module_alias(name, items, stmt.span)
            }
            ast::StmtKind::Use { path, items, alias } => {
                self.check_use(path, items.as_deref(), alias.as_deref(), stmt.span)
            }
        }
    }

    /// 检查变量声明: `name[: type] [= expr]`
    #[allow(clippy::result_large_err)]
    fn check_var(
        &mut self,
        name: &str,
        type_annotation: Option<&ast::Type>,
        initializer: Option<&ast::Expr>,
        span: Span,
    ) -> TypeResult<()> {
        if let Some(init) = initializer {
            let init_ty = self.inferrer.infer_expr(init)?;

            if let Some(ann) = type_annotation {
                let ann_ty = MonoType::from(ann.clone());
                self.inferrer
                    .solver()
                    .add_constraint(init_ty.clone(), ann_ty, span);
            }

            // 泛化 initializer 的类型
            let poly = self.inferrer.solver().generalize(&init_ty);
            self.inferrer.add_var(name.to_string(), poly);
        } else if let Some(ann) = type_annotation {
            // 没有初始化时，使用类型注解
            let ty = MonoType::from(ann.clone());
            self.inferrer.add_var(name.to_string(), PolyType::mono(ty));
        } else {
            // 没有任何信息，创建新类型变量
            let ty = self.inferrer.solver().new_var();
            self.inferrer.add_var(name.to_string(), PolyType::mono(ty));
        }

        Ok(())
    }

    /// 检查类型定义
    #[allow(clippy::result_large_err)]
    fn check_type_def(
        &mut self,
        name: &str,
        definition: &ast::Type,
        _span: Span,
    ) -> TypeResult<()> {
        let ty = MonoType::from(definition.clone());
        self.inferrer.add_var(name.to_string(), PolyType::mono(ty));
        Ok(())
    }

    /// 检查模块别名
    #[allow(clippy::result_large_err)]
    fn check_module_alias(
        &mut self,
        _name: &str,
        _items: &[ast::Stmt],
        _span: Span,
    ) -> TypeResult<()> {
        // TODO: 实现模块别名检查
        Ok(())
    }

    /// 检查 use 语句
    #[allow(clippy::result_large_err)]
    fn check_use(
        &mut self,
        _path: &str,
        _items: Option<&[String]>,
        _alias: Option<&str>,
        _span: Span,
    ) -> TypeResult<()> {
        // TODO: 实现 use 语句检查
        Ok(())
    }

    // =========================================================================
    // 函数检查
    // =========================================================================

    /// 检查函数定义
    ///
    /// 实现完整的类型推断规则：
    /// - 参数类型推断：有标注用标注，无标注尝试推断，Lambda 参数无法推断则拒绝
    /// - 返回类型推断：有标注用标注，无标注则从函数体推断
    #[allow(clippy::too_many_arguments)]
    #[allow(clippy::result_large_err)]
    pub fn check_fn_def(
        &mut self,
        name: &str,
        params: &[ast::Param],
        return_type: Option<&ast::Type>,
        body: &ast::Block,
        is_async: bool,
        external_param_types: Option<Vec<MonoType>>,
        is_annotated: bool,
    ) -> TypeResult<middle::FunctionIR> {
        // 保存当前返回类型
        let prev_return_type = self.current_return_type.take();
        let prev_inferrer_return_type = self.inferrer.current_return_type.take();

        let _has_external_types = external_param_types.is_some();

        // 创建参数类型列表
        let param_types: Vec<MonoType> = if let Some(types) = external_param_types {
            types
        } else {
            params
                .iter()
                .map(|p| {
                    if let Some(ty) = &p.ty {
                        MonoType::from(ty.clone())
                    } else {
                        self.inferrer.solver().new_var()
                    }
                })
                .collect()
        };

        // 跟踪未类型化的参数（使用 param_types 中对应位置的类型变量）
        let mut untyped_params: Vec<(String, usize, MonoType)> = Vec::new();
        for (i, param) in params.iter().enumerate() {
            if param.ty.is_none() {
                // 直接使用 param_types 中对应位置的类型变量
                untyped_params.push((param.name.clone(), i, param_types[i].clone()));
            }
        }

        // 处理返回类型
        let (return_ty, _inferred_return_ty) = if let Some(ty) = return_type {
            // 有标注类型，使用标注类型
            (MonoType::from(ty.clone()), None)
        } else {
            // 无标注类型，需要从函数体推断
            let inferred = self.inferrer.solver().new_var();
            (inferred.clone(), Some(inferred))
        };

        // 设置当前返回类型
        self.current_return_type = Some(return_ty.clone());
        self.inferrer.current_return_type = Some(return_ty.clone());

        // 进入函数体作用域
        self.inferrer.enter_scope();

        // 添加参数到作用域
        // 注意：不要使用 generalize/instantiate，因为参数类型需要在函数体内被约束
        for (param, param_ty) in params.iter().zip(param_types.iter()) {
            let poly = PolyType::mono(param_ty.clone());
            self.inferrer.add_var(param.name.clone(), poly);
        }

        // 推断函数体
        // infer_block 会调用 infer_expr，后者会调用 infer_return
        // infer_return 会使用 self.inferrer.current_return_type 进行检查
        let body_ty = self.inferrer.infer_block(body, true, None)?;

        // 检查隐式返回（最后表达式或 Void）
        if let Some(expr) = &body.expr {
            // 如果最后表达式是 `return`，则该 return 已在 infer_return 中
            // 对当前返回类型添加了约束，因此不需要（也不应）再将
            // 块的类型与返回类型约束在一起 — 否则会把 `Void` 约束到返回类型。
            if let ast::Expr::Return(_, _) = expr.as_ref() {
                // diverging via explicit return; nothing to do here
            } else {
                // 有最后表达式，约束其类型为返回类型
                self.inferrer.solver().add_constraint(
                    body_ty.clone(),
                    return_ty.clone(),
                    body.span,
                );
            }
        } else {
            // 无最后表达式，检查是否是发散的（以 return 结尾）
            // 如果不是发散的，则隐式返回 Void
            // Consider the function diverging if any top-level statement in the
            // block is an explicit `return`.
            let is_diverging = body.stmts.iter().any(|s| match &s.kind {
                ast::StmtKind::Expr(e) => matches!(e.as_ref(), ast::Expr::Return(_, _)),
                _ => false,
            });

            if !is_diverging {
                self.inferrer
                    .solver()
                    .add_constraint(MonoType::Void, return_ty.clone(), body.span);
            }
        }

        // 参数类型推断规则：检查未类型化参数
        // 如果没有外部类型标注，且参数本身没有类型标注，则报错（不支持从使用推断参数类型）
        eprintln!(
            "[DEBUG] is_annotated: {}, untyped_params len: {}",
            is_annotated,
            untyped_params.len()
        );
        if !is_annotated {
            for (param_name, param_idx, _param_ty) in &untyped_params {
                eprintln!("[DEBUG] Adding error for param: {}", param_name);
                self.add_error(TypeError::CannotInferParamType {
                    name: param_name.clone(),
                    span: params[*param_idx].span,
                });
            }
        }

        // 退出函数体作用域
        self.inferrer.exit_scope();

        // 恢复之前的返回类型
        self.current_return_type = prev_return_type;
        self.inferrer.current_return_type = prev_inferrer_return_type;

        // 生成函数 IR
        let fn_ir = middle::FunctionIR {
            name: name.to_string(),
            params: param_types,
            return_type: return_ty,
            is_async,
            locals: Vec::new(),
            blocks: Vec::new(),
            entry: 0,
        };

        Ok(fn_ir)
    }

    /// 检查类型变量是否未约束（未被使用）
    ///
    /// 如果类型变量仍然是 Unbound 状态，返回 true
    fn is_unconstrained_var(&self, ty: &MonoType) -> bool {
        match ty {
            MonoType::TypeVar(id) => self.inferrer.solver_ref().is_unconstrained(*id),
            _ => false,
        }
    }

    /// 检查函数调用
    #[allow(clippy::result_large_err)]
    fn check_fn_call(
        &mut self,
        func: &ast::Expr,
        args: &[ast::Expr],
        span: Span,
    ) -> TypeResult<MonoType> {
        self.inferrer.infer_call(func, args, span)
    }

    // =========================================================================
    // IR 生成
    // =========================================================================

    /// 生成模块 IR
    fn generate_module_ir(&self, module: &ast::Module) -> Result<middle::ModuleIR, Vec<TypeError>> {
        let mut functions = Vec::new();

        for stmt in &module.items {
            match &stmt.kind {
                ast::StmtKind::Fn {
                    name,
                    type_annotation,
                    params,
                    body: (stmts, expr),
                } => {
                    // 解析返回类型
                    let return_type = match type_annotation {
                        Some(ast::Type::Fn { return_type, .. }) => (**return_type).clone().into(),
                        Some(ty) => ty.clone().into(),
                        None => MonoType::Void,
                    };

                    // 生成函数体指令
                    let mut instructions = Vec::new();

                    // 为每个参数生成 LoadArg 指令
                    for (i, _param) in params.iter().enumerate() {
                        instructions.push(middle::Instruction::Load {
                            dst: middle::Operand::Local(i),
                            src: middle::Operand::Arg(i),
                        });
                    }

                    // 处理语句
                    for stmt in stmts {
                        self.generate_stmt_ir(stmt, &mut instructions);
                    }

                    // 处理返回值表达式
                    if let Some(e) = expr {
                        let result_reg = instructions.len(); // 使用新寄存器
                        self.generate_expr_ir(e, result_reg, &mut instructions);
                        instructions.push(middle::Instruction::Ret(Some(middle::Operand::Local(
                            result_reg,
                        ))));
                    } else {
                        instructions.push(middle::Instruction::Ret(None));
                    }

                    // 构建函数 IR
                    let func_ir = middle::FunctionIR {
                        name: name.clone(),
                        params: params
                            .iter()
                            .filter_map(|p| p.ty.clone())
                            .map(|t| t.into())
                            .collect(),
                        return_type,
                        is_async: false,
                        locals: Vec::new(),
                        blocks: vec![middle::BasicBlock {
                            label: 0,
                            instructions,
                            successors: Vec::new(),
                        }],
                        entry: 0,
                    };

                    functions.push(func_ir);
                }
                ast::StmtKind::Var {
                    name,
                    type_annotation,
                    initializer,
                    is_mut: _,
                } => {
                    // 全局变量处理（简化）
                    let var_type = type_annotation
                        .clone()
                        .map(|t| t.into())
                        .unwrap_or(MonoType::Int(64));

                    let init_instr = if let Some(_expr) = initializer {
                        // 简化：假设初始化为整数常量
                        middle::Instruction::Load {
                            dst: middle::Operand::Global(0),
                            src: middle::Operand::Const(middle::ConstValue::Int(0)),
                        }
                    } else {
                        middle::Instruction::Load {
                            dst: middle::Operand::Global(0),
                            src: middle::Operand::Const(middle::ConstValue::Int(0)),
                        }
                    };

                    // 为全局变量创建函数（简化处理）
                    let func_ir = middle::FunctionIR {
                        name: name.clone(),
                        params: Vec::new(),
                        return_type: var_type,
                        is_async: false,
                        locals: Vec::new(),
                        blocks: vec![middle::BasicBlock {
                            label: 0,
                            instructions: vec![init_instr, middle::Instruction::Ret(None)],
                            successors: Vec::new(),
                        }],
                        entry: 0,
                    };
                    functions.push(func_ir);
                }
                _ => {}
            }
        }

        Ok(middle::ModuleIR {
            types: Vec::new(),
            constants: Vec::new(),
            globals: Vec::new(),
            functions,
        })
    }

    /// 生成语句 IR
    fn generate_stmt_ir(&self, stmt: &ast::Stmt, instructions: &mut Vec<middle::Instruction>) {
        match &stmt.kind {
            ast::StmtKind::Expr(expr) => {
                let result_reg = instructions.len();
                self.generate_expr_ir(expr, result_reg, instructions);
            }
            ast::StmtKind::Var {
                name: _,
                type_annotation: _,
                initializer,
                is_mut: _,
            } => {
                // 生成变量声明指令
                let var_idx = instructions.len();
                if let Some(expr) = initializer {
                    self.generate_expr_ir(expr, var_idx, instructions);
                }
            }
            ast::StmtKind::Fn {
                name: _,
                type_annotation: _,
                params: _,
                body: _,
            } => {
                // 嵌套函数（简化处理）
            }
            _ => {}
        }
    }

    /// 生成表达式 IR
    #[allow(clippy::only_used_in_recursion)]
    fn generate_expr_ir(
        &self,
        expr: &ast::Expr,
        result_reg: usize,
        instructions: &mut Vec<middle::Instruction>,
    ) {
        match expr {
            ast::Expr::Lit(literal, _) => {
                // 常量加载
                let const_val = match literal {
                    Literal::Int(n) => middle::ConstValue::Int(*n),
                    Literal::Float(f) => middle::ConstValue::Float(*f),
                    Literal::Bool(b) => middle::ConstValue::Bool(*b),
                    Literal::String(s) => middle::ConstValue::String(s.clone()),
                    Literal::Char(c) => middle::ConstValue::Char(*c),
                };
                instructions.push(middle::Instruction::Load {
                    dst: middle::Operand::Local(result_reg),
                    src: middle::Operand::Const(const_val),
                });
            }
            ast::Expr::Var(_, _) => {
                // 变量加载
                instructions.push(middle::Instruction::Load {
                    dst: middle::Operand::Local(result_reg),
                    src: middle::Operand::Local(result_reg), // 简化处理
                });
            }
            ast::Expr::BinOp {
                op,
                left,
                right,
                span: _,
            } => {
                // 二元运算
                let left_reg = result_reg;
                let right_reg = result_reg + 1;

                self.generate_expr_ir(left, left_reg, instructions);
                self.generate_expr_ir(right, right_reg, instructions);

                let instr = match op {
                    ast::BinOp::Add => middle::Instruction::Add {
                        dst: middle::Operand::Local(result_reg),
                        lhs: middle::Operand::Local(left_reg),
                        rhs: middle::Operand::Local(right_reg),
                    },
                    ast::BinOp::Sub => middle::Instruction::Sub {
                        dst: middle::Operand::Local(result_reg),
                        lhs: middle::Operand::Local(left_reg),
                        rhs: middle::Operand::Local(right_reg),
                    },
                    ast::BinOp::Mul => middle::Instruction::Mul {
                        dst: middle::Operand::Local(result_reg),
                        lhs: middle::Operand::Local(left_reg),
                        rhs: middle::Operand::Local(right_reg),
                    },
                    ast::BinOp::Div => middle::Instruction::Div {
                        dst: middle::Operand::Local(result_reg),
                        lhs: middle::Operand::Local(left_reg),
                        rhs: middle::Operand::Local(right_reg),
                    },
                    ast::BinOp::Mod => middle::Instruction::Mod {
                        dst: middle::Operand::Local(result_reg),
                        lhs: middle::Operand::Local(left_reg),
                        rhs: middle::Operand::Local(right_reg),
                    },
                    ast::BinOp::Eq => middle::Instruction::Eq {
                        dst: middle::Operand::Local(result_reg),
                        lhs: middle::Operand::Local(left_reg),
                        rhs: middle::Operand::Local(right_reg),
                    },
                    ast::BinOp::Neq => middle::Instruction::Ne {
                        dst: middle::Operand::Local(result_reg),
                        lhs: middle::Operand::Local(left_reg),
                        rhs: middle::Operand::Local(right_reg),
                    },
                    ast::BinOp::Lt => middle::Instruction::Lt {
                        dst: middle::Operand::Local(result_reg),
                        lhs: middle::Operand::Local(left_reg),
                        rhs: middle::Operand::Local(right_reg),
                    },
                    ast::BinOp::Le => middle::Instruction::Le {
                        dst: middle::Operand::Local(result_reg),
                        lhs: middle::Operand::Local(left_reg),
                        rhs: middle::Operand::Local(right_reg),
                    },
                    ast::BinOp::Gt => middle::Instruction::Gt {
                        dst: middle::Operand::Local(result_reg),
                        lhs: middle::Operand::Local(left_reg),
                        rhs: middle::Operand::Local(right_reg),
                    },
                    ast::BinOp::Ge => middle::Instruction::Ge {
                        dst: middle::Operand::Local(result_reg),
                        lhs: middle::Operand::Local(left_reg),
                        rhs: middle::Operand::Local(right_reg),
                    },
                    _ => middle::Instruction::Add {
                        dst: middle::Operand::Local(result_reg),
                        lhs: middle::Operand::Local(left_reg),
                        rhs: middle::Operand::Local(right_reg),
                    },
                };
                instructions.push(instr);
            }
            ast::Expr::Call {
                func: _,
                args,
                span: _,
            } => {
                // 函数调用
                let mut arg_regs = Vec::new();
                for (i, arg) in args.iter().enumerate() {
                    self.generate_expr_ir(arg, result_reg + i + 1, instructions);
                    arg_regs.push(middle::Operand::Local(result_reg + i + 1));
                }

                instructions.push(middle::Instruction::Call {
                    dst: Some(middle::Operand::Local(result_reg)),
                    func: middle::Operand::Local(result_reg), // 简化
                    args: arg_regs,
                });
            }
            _ => {
                // 默认返回 0
                instructions.push(middle::Instruction::Load {
                    dst: middle::Operand::Local(result_reg),
                    src: middle::Operand::Const(middle::ConstValue::Int(0)),
                });
            }
        }
    }
}

// =========================================================================
// 类型环境扩展
// =========================================================================

/// 扩展类型环境，支持更多操作
#[derive(Debug, Default)]
pub struct ExtendedTypeEnvironment {
    /// 变量绑定
    vars: HashMap<String, PolyType>,
    /// 类型定义
    types: HashMap<String, PolyType>,
    /// 求解器
    solver: TypeConstraintSolver,
    /// 错误列表
    errors: Vec<TypeError>,
}

impl ExtendedTypeEnvironment {
    /// 创建新的扩展类型环境
    pub fn new() -> Self {
        ExtendedTypeEnvironment {
            vars: HashMap::new(),
            types: HashMap::new(),
            solver: TypeConstraintSolver::new(),
            errors: Vec::new(),
        }
    }

    /// 添加变量
    pub fn add_var(&mut self, name: String, poly: PolyType) {
        self.vars.insert(name, poly);
    }

    /// 添加类型
    pub fn add_type(&mut self, name: String, poly: PolyType) {
        self.types.insert(name, poly);
    }

    /// 获取变量
    pub fn get_var(&self, name: &str) -> Option<&PolyType> {
        self.vars.get(name)
    }

    /// 获取类型
    pub fn get_type(&self, name: &str) -> Option<&PolyType> {
        self.types.get(name)
    }

    /// 添加错误
    pub fn add_error(&mut self, error: TypeError) {
        self.errors.push(error);
    }

    /// 获取错误
    pub fn errors(&self) -> &[TypeError] {
        &self.errors
    }

    /// 是否有错误
    pub fn has_errors(&self) -> bool {
        !self.errors.is_empty()
    }
}

/// 检查字面量类型
pub fn infer_literal_type(lit: &Literal) -> MonoType {
    match lit {
        Literal::Int(_) => MonoType::Int(64),
        Literal::Float(_) => MonoType::Float(64),
        Literal::Bool(_) => MonoType::Bool,
        Literal::Char(_) => MonoType::Char,
        Literal::String(_) => MonoType::String,
    }
}

/// 获取二元运算的结果类型
pub fn binop_result_type(op: &ast::BinOp, left: &MonoType, right: &MonoType) -> Option<MonoType> {
    match op {
        ast::BinOp::Add | ast::BinOp::Sub | ast::BinOp::Mul | ast::BinOp::Div | ast::BinOp::Mod => {
            if left == right && left.is_numeric() {
                Some(left.clone())
            } else {
                None
            }
        }
        ast::BinOp::Eq
        | ast::BinOp::Neq
        | ast::BinOp::Lt
        | ast::BinOp::Le
        | ast::BinOp::Gt
        | ast::BinOp::Ge => {
            if left == right {
                Some(MonoType::Bool)
            } else {
                None
            }
        }
        ast::BinOp::And | ast::BinOp::Or => {
            if *left == MonoType::Bool && *right == MonoType::Bool {
                Some(MonoType::Bool)
            } else {
                None
            }
        }
        ast::BinOp::Assign => Some(MonoType::Void),
        ast::BinOp::Range => None, // 范围运算暂时不支持类型检查
    }
}

/// 获取一元运算的结果类型
pub fn unop_result_type(op: &ast::UnOp, expr: &MonoType) -> Option<MonoType> {
    match op {
        ast::UnOp::Neg | ast::UnOp::Pos => {
            if expr.is_numeric() {
                Some(expr.clone())
            } else {
                None
            }
        }
        ast::UnOp::Not => {
            if *expr == MonoType::Bool {
                Some(MonoType::Bool)
            } else {
                None
            }
        }
    }
}
