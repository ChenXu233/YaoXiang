//! AST 到 IR 的代码生成器
//!
//! 将抽象语法树（AST）转换为中间表示（IR）。
//! 这是编译流程的第二步：解析 → 类型检查 → IR 生成 → 代码生成
//!
//! ## 设计原则
//!
//! 1. 单一职责：只负责 AST → IR 转换，不关心类型检查或代码生成
//! 2. 简洁直接：IR 结构简单，生成逻辑清晰
//! 3. 可测试性：独立的模块便于单元测试

use crate::frontend::lexer::tokens::Literal;
use crate::frontend::parser::ast::{self, Expr};
use crate::frontend::typecheck::{MonoType, PolyType, TypeCheckResult};
use crate::middle::core::ir::{BasicBlock, ConstValue, FunctionIR, Instruction, ModuleIR, Operand};
use crate::util::span::Span;
use crate::tlog;
use crate::util::i18n::MSG;
use std::collections::HashMap;

/// 符号表条目
#[derive(Debug, Clone)]
struct SymbolEntry {
    name: String,
    local_idx: usize,
}

/// IR 生成器配置
#[derive(Debug, Default, Clone)]
pub struct IrGeneratorConfig {
    /// 是否生成调试信息
    pub generate_debug_info: bool,
}

impl IrGeneratorConfig {
    /// 创建默认配置
    pub fn new() -> Self {
        Self::default()
    }
}

/// AST 到 IR 的生成器
///
/// 将 AST 节点转换为 IR 指令序列。
#[derive(Debug)]
pub struct AstToIrGenerator {
    /// 配置
    config: IrGeneratorConfig,
    /// 符号表（用于变量解析）
    symbols: Vec<HashMap<String, SymbolEntry>>,
    /// 类型检查结果（包含变量绑定信息）
    type_result: Option<Box<TypeCheckResult>>,
    /// 下一个临时寄存器编号
    next_temp: usize,
}

impl Default for AstToIrGenerator {
    fn default() -> Self {
        Self::new()
    }
}

impl AstToIrGenerator {
    /// 创建新的 IR 生成器
    pub fn new() -> Self {
        Self {
            config: IrGeneratorConfig::default(),
            symbols: vec![HashMap::new()], // 全局作用域
            type_result: None,
            next_temp: 0,
        }
    }

    /// 创建新的 IR 生成器（带类型信息）
    pub fn new_with_type_result(type_result: &TypeCheckResult) -> Self {
        Self {
            config: IrGeneratorConfig::default(),
            symbols: vec![HashMap::new()], // 全局作用域
            type_result: Some(Box::new(type_result.clone())),
            next_temp: 0,
        }
    }

    /// 进入新的作用域
    fn enter_scope(&mut self) {
        tlog!(debug, MSG::IrGenEnterScope, &self.symbols.len().to_string());
        self.symbols.push(HashMap::new());
        tlog!(debug, MSG::IrGenEnterScope, &self.symbols.len().to_string());
    }

    /// 退出当前作用域
    fn exit_scope(&mut self) {
        tlog!(debug, MSG::IrGenExitScope, &self.symbols.len().to_string());
        self.symbols.pop();
        tlog!(debug, MSG::IrGenExitScope, &self.symbols.len().to_string());
    }

    /// 注册局部变量
    fn register_local(
        &mut self,
        name: &str,
        local_idx: usize,
    ) {
        tlog!(
            debug,
            MSG::IrGenRegisterLocal,
            &name.to_string(),
            &local_idx.to_string()
        );
        if let Some(scope) = self.symbols.last_mut() {
            scope.insert(
                name.to_string(),
                SymbolEntry {
                    name: name.to_string(),
                    local_idx,
                },
            );
        }
    }

    /// 查找局部变量
    fn lookup_local(
        &self,
        name: &str,
    ) -> Option<usize> {
        for scope in self.symbols.iter().rev() {
            if let Some(entry) = scope.get(name) {
                tlog!(
                    debug,
                    MSG::IrGenLookupLocal,
                    &name.to_string(),
                    &entry.local_idx.to_string()
                );
                return Some(entry.local_idx);
            }
        }
        tlog!(debug, MSG::IrGenLookupLocalNotFound, &name.to_string());
        None
    }

    /// 查找变量的类型
    fn lookup_var_type(
        &self,
        name: &str,
    ) -> Option<&PolyType> {
        if let Some(ref type_result) = self.type_result {
            // 调试：打印所有绑定
            tracing::debug!("Looking for variable '{}' in bindings", name);
            tracing::debug!("All bindings: {:?}", type_result.bindings);

            if let Some(poly_type) = type_result.bindings.get(name) {
                // 使用 debug 日志记录类型信息
                tracing::debug!("Found type for variable {}: {:?}", name, poly_type);
                return Some(poly_type);
            }
        } else {
            tracing::debug!("type_result is None!");
        }
        tracing::debug!("Type not found for variable: {}", name);
        None
    }

    // 删除的函数：extract_type_name_from_poly
    // 原因：根据设计文档，不再需要复杂的类型名提取逻辑
    // 方法调用现在直接生成简单函数名（方法名）

    /// 解析字段索引（简化版本）
    /// 在真正的实现中，需要从类型信息中查找字段在结构体中的位置
    fn resolve_field_index(
        &self,
        _expr: &ast::Expr,
        field_name: &str,
    ) -> Option<usize> {
        

        // 简化处理：假设常见字段名
        // x -> 0, y -> 1, value -> 2 等
        match field_name {
            "x" | "first" | "key" => Some(0),
            "y" | "second" | "value" => Some(1),
            "z" | "third" => Some(2),
            _ => {
                // 对于未知字段名，返回 None
                // 在真正的实现中，这里应该查询类型定义中的字段列表
                None
            }
        }
    }

    /// 获取下一个临时寄存器编号
    fn next_temp_reg(&mut self) -> usize {
        let reg = self.next_temp;
        self.next_temp += 1;
        reg
    }

    /// 从 AST 模块生成 IR 模块
    pub fn generate_module_ir(
        &mut self,
        module: &ast::Module,
    ) -> Result<ModuleIR, Vec<IrGenError>> {
        tlog!(info, MSG::Stage1Start);

        let mut functions = Vec::new();
        let mut errors = Vec::new();
        let mut constants = Vec::new();

        for stmt in &module.items {
            match self.generate_stmt_ir(stmt, &mut constants) {
                Ok(Some(func_ir)) => functions.push(func_ir),
                Ok(None) => {}
                Err(e) => errors.push(e),
            }
        }

        if !errors.is_empty() {
            return Err(errors);
        }

        Ok(ModuleIR {
            types: Vec::new(),
            globals: Vec::new(),
            functions,
        })
    }

    /// 生成语句的 IR
    fn generate_stmt_ir(
        &mut self,
        stmt: &ast::Stmt,
        constants: &mut Vec<ConstValue>,
    ) -> Result<Option<FunctionIR>, IrGenError> {
        match &stmt.kind {
            ast::StmtKind::Fn {
                name,
                type_annotation,
                params,
                body: (stmts, expr),
            } => self.generate_function_ir(
                name,
                type_annotation.as_ref(),
                params,
                stmts,
                expr,
                constants,
            ),
            ast::StmtKind::Var {
                name,
                type_annotation,
                initializer,
                is_mut: _,
            } => self.generate_global_var_ir(
                name,
                type_annotation.as_ref(),
                initializer.as_ref().map(|v| &**v),
            ),
            ast::StmtKind::MethodBind {
                type_name,
                method_name,
                method_type,
                params,
                body: (stmts, expr),
            } => self.generate_method_ir(
                type_name,
                method_name,
                method_type,
                params,
                stmts,
                expr,
                constants,
            ),
            ast::StmtKind::TypeDef { name, definition } => {
                self.generate_constructor_ir(name, definition)
            }
            _ => Ok(None),
        }
    }

    /// 生成方法 IR
    #[allow(clippy::too_many_arguments)]
    fn generate_method_ir(
        &mut self,
        _type_name: &str,
        method_name: &str,
        method_type: &ast::Type,
        params: &[ast::Param],
        stmts: &[ast::Stmt],
        expr: &Option<Box<ast::Expr>>,
        constants: &mut Vec<ConstValue>,
    ) -> Result<Option<FunctionIR>, IrGenError> {
        // 命名空间机制：方法函数名就是方法名，无复杂前缀
        // 例如：Point.get_x 生成函数名 "get_x"
        // 调用时：p.get_x() -> get_x(p)
        let func_name = method_name.to_string();

        // 解析返回类型
        let return_type = if let ast::Type::Fn { return_type, .. } = method_type {
            (**return_type).clone().into()
        } else {
            // 非函数类型，报错
            return Err(IrGenError::InternalError {
                message: format!("Method {} is not a function type", method_name),
                span: Span::default(),
            });
        };

        // 进入新作用域
        self.enter_scope();

        // 注册参数
        let mut param_types = Vec::new();
        for (i, param) in params.iter().enumerate() {
            if let Some(param_type_ast) = &param.ty {
                let param_type = param_type_ast.clone().into();
                param_types.push(param_type);
            } else {
                // 参数没有类型，默认为 Int64
                param_types.push(MonoType::Int(64));
            }

            // 注册参数到符号表
            self.register_local(&param.name, i);
        }

        // 生成指令序列
        let mut instructions = Vec::new();

        // 生成语句 IR
        for stmt in stmts {
            self.generate_local_stmt_ir(stmt, &mut instructions, constants)?;
        }

        // 生成表达式 IR
        if let Some(expr) = expr {
            let result_reg = 0;
            self.generate_expr_ir(expr, result_reg, &mut instructions, constants)?;
            instructions.push(Instruction::Ret(Some(Operand::Local(result_reg))));
        } else {
            instructions.push(Instruction::Ret(None));
        }

        // 退出作用域
        self.exit_scope();

        // 分配局部变量类型（简化：与参数相同）
        let locals_types = param_types.clone();

        // 构建函数 IR
        let func_ir = FunctionIR {
            name: func_name,
            params: param_types.clone(),
            return_type,
            is_async: false,
            locals: locals_types,
            blocks: vec![BasicBlock {
                label: 0,
                instructions,
                successors: Vec::new(),
            }],
            entry: 0,
        };

        Ok(Some(func_ir))
    }

    /// 生成函数 IR
    #[allow(clippy::only_used_in_recursion)]
    fn generate_function_ir(
        &mut self,
        name: &str,
        type_annotation: Option<&ast::Type>,
        params: &[ast::Param],
        stmts: &[ast::Stmt],
        expr: &Option<Box<ast::Expr>>,
        constants: &mut Vec<ConstValue>,
    ) -> Result<Option<FunctionIR>, IrGenError> {
        // 解析返回类型
        let return_type = match type_annotation {
            Some(ast::Type::Fn { return_type, .. }) => (**return_type).clone().into(),
            Some(ty) => ty.clone().into(),
            None => MonoType::Void,
        };

        // 生成函数体指令
        let mut instructions = Vec::new();

        // 进入函数体作用域
        self.enter_scope();

        // 为每个参数生成 LoadArg 指令并注册
        for (i, param) in params.iter().enumerate() {
            instructions.push(Instruction::Load {
                dst: Operand::Local(i),
                src: Operand::Arg(i),
            });
            // 存储到局部变量并注册
            instructions.push(Instruction::Store {
                dst: Operand::Local(i),
                src: Operand::Local(i),
            });
            self.register_local(&param.name, i);
        }

        // 记录局部变量起始位置（在参数之后）
        let local_var_start = params.len();
        self.next_temp = local_var_start;

        // 处理语句
        for stmt in stmts {
            tlog!(
                debug,
                MSG::IrGenBeforeProcessStmt,
                &self.symbols.len().to_string()
            );
            self.generate_local_stmt_ir(stmt, &mut instructions, constants)?;
            tlog!(
                debug,
                MSG::IrGenAfterProcessStmt,
                &self.symbols.len().to_string()
            );
        }

        // 处理返回值表达式
        // 注意：即使 expr 是 Some(Return(...))，也会被处理，
        // 因为 Return 本身就是表达式，会生成 Ret 指令
        if let Some(e) = expr {
            let result_reg = self.next_temp_reg();
            self.generate_expr_ir(e, result_reg, &mut instructions, constants)?;
            // 注意：generate_expr_ir 会为 Return 表达式添加 Ret 指令，
            // 所以这里不需要额外添加 Ret 指令
        } else {
            instructions.push(Instruction::Ret(None));
        }

        // 退出函数体作用域
        tlog!(
            debug,
            MSG::IrGenAboutToExitScope,
            &self.symbols.len().to_string()
        );
        self.exit_scope();
        tlog!(
            debug,
            MSG::IrGenAfterExitScope,
            &self.symbols.len().to_string()
        );

        // 计算局部变量总数（用于 VM 分配帧空间）
        // 局部变量包括参数和函数体中声明的变量
        // 参数数量 + 临时寄存器使用数量
        let total_locals = self.next_temp;
        const MAX_LOCALS: usize = 65_535;
        if total_locals > MAX_LOCALS {
            return Err(IrGenError::InternalError {
                message: format!(
                    "too many locals allocated in function '{}': {}",
                    name, total_locals
                ),
                span: Span::dummy(),
            });
        }
        let locals_types: Vec<MonoType> = (0..total_locals)
            .map(|_| MonoType::Int(64)) // 简化：所有局部变量默认为 Int64
            .collect();

        // 构建函数 IR
        let func_ir = FunctionIR {
            name: name.to_string(),
            params: params
                .iter()
                .filter_map(|p| p.ty.clone())
                .map(|t| t.into())
                .collect(),
            return_type,
            is_async: false,
            locals: locals_types,
            blocks: vec![BasicBlock {
                label: 0,
                instructions,
                successors: Vec::new(),
            }],
            entry: 0,
        };

        tlog!(info, MSG::Stage1Complete);

        Ok(Some(func_ir))
    }

    /// 生成全局变量 IR
    fn generate_global_var_ir(
        &self,
        name: &str,
        type_annotation: Option<&ast::Type>,
        _initializer: Option<&ast::Expr>,
    ) -> Result<Option<FunctionIR>, IrGenError> {
        let var_type = type_annotation
            .map(|t| (*t).clone().into())
            .unwrap_or(MonoType::Int(64));

        // 简化处理：将全局变量转换为返回常量的函数
        // x: Int = 42 => fn x() -> Int { return 0; }
        // 这样做是为了避免 CodegenError::InvalidOperand (不支持 Global 操作数)

        let result_reg = 0;
        let instructions = vec![
            Instruction::Load {
                dst: Operand::Local(result_reg),
                src: Operand::Const(ConstValue::Int(0)),
            },
            Instruction::Ret(Some(Operand::Local(result_reg))),
        ];

        // 为全局变量创建函数
        let func_ir = FunctionIR {
            name: name.to_string(),
            params: Vec::new(),
            return_type: var_type,
            is_async: false,
            locals: vec![MonoType::Int(64)], // 分配一个局部变量用于存储结果
            blocks: vec![BasicBlock {
                label: 0,
                instructions,
                successors: Vec::new(),
            }],
            entry: 0,
        };

        Ok(Some(func_ir))
    }

    /// 生成构造函数 IR
    fn generate_constructor_ir(
        &self,
        _name: &str,
        definition: &ast::Type,
    ) -> Result<Option<FunctionIR>, IrGenError> {
        // 只为结构体类型生成构造函数
        match definition {
            ast::Type::NamedStruct {
                name: struct_name,
                fields,
            } => self.generate_struct_constructor_ir(struct_name, fields),
            ast::Type::Struct(fields) => self.generate_struct_constructor_ir(_name, fields),
            _ => {
                // 非结构体类型，不生成构造函数
                Ok(None)
            }
        }
    }

    /// 为结构体生成构造函数 IR 的辅助方法
    fn generate_struct_constructor_ir(
        &self,
        struct_name: &str,
        fields: &[(String, ast::Type)],
    ) -> Result<Option<FunctionIR>, IrGenError> {
        // 创建构造函数函数的参数列表
        let mut param_types = Vec::new();
        for (_, field_type) in fields {
            param_types.push(field_type.clone().into());
        }

        // 创建构造函数函数的指令序列
        let mut instructions = Vec::new();

        // 为每个字段参数生成返回指令
        // 这里简化处理：返回第一个参数作为结构体的表示
        // 在真正的实现中，应该创建结构体并设置字段
        let result_reg = 0;
        instructions.push(Instruction::Load {
            dst: Operand::Local(result_reg),
            src: Operand::Arg(0),
        });
        instructions.push(Instruction::Ret(Some(Operand::Local(result_reg))));

        // 分配局部变量类型
        let locals_types = vec![MonoType::Int(64)];

        // 构建构造函数函数 IR
        // 直接使用结构体名称，完全透明化，避免与用户代码冲突
        let func_ir = FunctionIR {
            name: struct_name.to_string(),
            params: param_types,
            return_type: MonoType::TypeRef(struct_name.to_string()),
            is_async: false,
            locals: locals_types,
            blocks: vec![BasicBlock {
                label: 0,
                instructions,
                successors: Vec::new(),
            }],
            entry: 0,
        };

        Ok(Some(func_ir))
    }

    /// 生成局部语句 IR
    #[allow(clippy::only_used_in_recursion)]
    fn generate_local_stmt_ir(
        &mut self,
        stmt: &ast::Stmt,
        instructions: &mut Vec<Instruction>,
        constants: &mut Vec<ConstValue>,
    ) -> Result<(), IrGenError> {
        match &stmt.kind {
            ast::StmtKind::Expr(expr) => {
                let result_reg = self.next_temp_reg();
                self.generate_expr_ir(expr, result_reg, instructions, constants)?;
            }
            ast::StmtKind::Var {
                name,
                type_annotation: _,
                initializer,
                is_mut: _,
            } => {
                // 生成变量声明指令
                let var_idx = self.next_temp_reg();
                self.register_local(name, var_idx);
                if let Some(expr) = initializer {
                    self.generate_expr_ir(expr, var_idx, instructions, constants)?;
                } else {
                    // 默认初始化为 0
                    instructions.push(Instruction::Load {
                        dst: Operand::Local(var_idx),
                        src: Operand::Const(ConstValue::Int(0)),
                    });
                }
                // 生成 Store 指令将值存储到局部变量
                instructions.push(Instruction::Store {
                    dst: Operand::Local(var_idx),
                    src: Operand::Local(var_idx),
                });
            }
            ast::StmtKind::Fn {
                name: _,
                type_annotation: _,
                params: _,
                body: _,
            } => {
                // 嵌套函数（简化处理）
            }
            // 处理其他语句类型
            _ => {}
        }
        Ok(())
    }

    /// 生成表达式 IR
    #[allow(clippy::only_used_in_recursion)]
    fn generate_expr_ir(
        &mut self,
        expr: &ast::Expr,
        result_reg: usize,
        instructions: &mut Vec<Instruction>,
        constants: &mut Vec<ConstValue>,
    ) -> Result<(), IrGenError> {
        match expr {
            Expr::Lit(literal, _) => {
                // 常量加载
                let const_val = match literal {
                    Literal::Int(n) => ConstValue::Int(*n),
                    Literal::Float(f) => ConstValue::Float(*f),
                    Literal::Bool(b) => ConstValue::Bool(*b),
                    Literal::String(s) => ConstValue::String(s.clone()),
                    Literal::Char(c) => ConstValue::Char(*c),
                };
                // 添加到常量池
                constants.push(const_val.clone());
                instructions.push(Instruction::Load {
                    dst: Operand::Local(result_reg),
                    src: Operand::Const(const_val),
                });
            }
            Expr::Var(_, _) => {
                // 变量加载 - 查找符号表获取正确的局部变量索引
                let local_idx = if let Expr::Var(name, _) = expr {
                    self.lookup_local(name).unwrap_or(result_reg)
                } else {
                    result_reg
                };
                instructions.push(Instruction::Load {
                    dst: Operand::Local(result_reg),
                    src: Operand::Local(local_idx),
                });
            }
            Expr::BinOp {
                op,
                left,
                right,
                span: _,
            } => {
                tlog!(debug, MSG::DebugGeneratingIRBinOp, &format!("{:?}", op));
                // 二元运算
                let instr = match op {
                    ast::BinOp::Assign => {
                        if let Expr::Var(var_name, _) = left.as_ref() {
                            let local_idx = if let Some(idx) = self.lookup_local(var_name) {
                                idx
                            } else {
                                let idx = self.next_temp_reg();
                                self.register_local(var_name, idx);
                                idx
                            };
                            let val_reg = self.next_temp_reg();
                            self.generate_expr_ir(right, val_reg, instructions, constants)?;
                            instructions.push(Instruction::Store {
                                dst: Operand::Local(local_idx),
                                src: Operand::Local(val_reg),
                            });
                            instructions.push(Instruction::Load {
                                dst: Operand::Local(result_reg),
                                src: Operand::Local(local_idx),
                            });
                        }
                        return Ok(());
                    }
                    _ => {
                        let left_reg = self.next_temp_reg();
                        let right_reg = self.next_temp_reg();
                        self.generate_expr_ir(left, left_reg, instructions, constants)?;
                        self.generate_expr_ir(right, right_reg, instructions, constants)?;

                        match op {
                            ast::BinOp::Add => Instruction::Add {
                                dst: Operand::Local(result_reg),
                                lhs: Operand::Local(left_reg),
                                rhs: Operand::Local(right_reg),
                            },
                            ast::BinOp::Sub => Instruction::Sub {
                                dst: Operand::Local(result_reg),
                                lhs: Operand::Local(left_reg),
                                rhs: Operand::Local(right_reg),
                            },
                            ast::BinOp::Mul => Instruction::Mul {
                                dst: Operand::Local(result_reg),
                                lhs: Operand::Local(left_reg),
                                rhs: Operand::Local(right_reg),
                            },
                            ast::BinOp::Div => Instruction::Div {
                                dst: Operand::Local(result_reg),
                                lhs: Operand::Local(left_reg),
                                rhs: Operand::Local(right_reg),
                            },
                            ast::BinOp::Mod => Instruction::Mod {
                                dst: Operand::Local(result_reg),
                                lhs: Operand::Local(left_reg),
                                rhs: Operand::Local(right_reg),
                            },
                            ast::BinOp::Eq => Instruction::Eq {
                                dst: Operand::Local(result_reg),
                                lhs: Operand::Local(left_reg),
                                rhs: Operand::Local(right_reg),
                            },
                            ast::BinOp::Neq => Instruction::Ne {
                                dst: Operand::Local(result_reg),
                                lhs: Operand::Local(left_reg),
                                rhs: Operand::Local(right_reg),
                            },
                            ast::BinOp::Lt => Instruction::Lt {
                                dst: Operand::Local(result_reg),
                                lhs: Operand::Local(left_reg),
                                rhs: Operand::Local(right_reg),
                            },
                            ast::BinOp::Le => Instruction::Le {
                                dst: Operand::Local(result_reg),
                                lhs: Operand::Local(left_reg),
                                rhs: Operand::Local(right_reg),
                            },
                            ast::BinOp::Gt => Instruction::Gt {
                                dst: Operand::Local(result_reg),
                                lhs: Operand::Local(left_reg),
                                rhs: Operand::Local(right_reg),
                            },
                            ast::BinOp::Ge => Instruction::Ge {
                                dst: Operand::Local(result_reg),
                                lhs: Operand::Local(left_reg),
                                rhs: Operand::Local(right_reg),
                            },
                            // ast::BinOp::Assign case is handled above checking left/right generation.
                            // This placeholder is just to remove the old duplicated block.
                            _ => Instruction::Move {
                                dst: Operand::Local(result_reg),
                                src: Operand::Const(ConstValue::Int(0)),
                            },
                        }
                    }
                };
                instructions.push(instr);
            }
            Expr::Call {
                func,
                args,
                span: _,
            } => {
                // 检查是否是方法调用：func 是 FieldAccess
                if let Expr::FieldAccess { expr, field, .. } = func.as_ref() {
                    // 方法调用 - 转换为普通函数调用
                    // 命名空间机制：p.method() -> method(p)
                    let mut arg_regs = Vec::new();

                    // 首先生成对象表达式 IR（用于 self）
                    let obj_reg = self.next_temp_reg();
                    self.generate_expr_ir(expr, obj_reg, instructions, constants)?;
                    arg_regs.push(Operand::Local(obj_reg));

                    // 生成参数 IR
                    for arg in args.iter() {
                        let arg_reg = self.next_temp_reg();
                        self.generate_expr_ir(arg, arg_reg, instructions, constants)?;
                        arg_regs.push(Operand::Local(arg_reg));
                    }

                    // 命名空间机制：将方法调用转换为简单函数调用
                    // 例如：p.get_x() -> get_x(p)
                    // 函数名就是方法名，无复杂前缀
                    let method_function_name = field;

                    instructions.push(Instruction::Call {
                        dst: Some(Operand::Local(result_reg)),
                        func: Operand::Const(ConstValue::String(method_function_name.to_string())),
                        args: arg_regs,
                    });
                } else {
                    // 普通函数调用
                    let mut arg_regs = Vec::new();
                    for arg in args.iter() {
                        let arg_reg = self.next_temp_reg();
                        self.generate_expr_ir(arg, arg_reg, instructions, constants)?;
                        arg_regs.push(Operand::Local(arg_reg));
                    }

                    // 直接将函数名作为 String 存储在 Operand 中
                    let func_operand = if let Expr::Var(name, _) = func.as_ref() {
                        Operand::Const(ConstValue::String(name.clone()))
                    } else {
                        Operand::Const(ConstValue::Int(0))
                    };

                    instructions.push(Instruction::Call {
                        dst: Some(Operand::Local(result_reg)),
                        func: func_operand,
                        args: arg_regs,
                    });
                }
            }
            Expr::FieldAccess { expr, field, .. } => {
                // 字段访问：加载对象的字段
                // 生成对象表达式 IR
                let obj_reg = self.next_temp_reg();
                self.generate_expr_ir(expr, obj_reg, instructions, constants)?;

                // 尝试从类型信息中获取字段索引
                // 简化处理：使用字段名的哈希值作为索引（临时方案）
                // 在真正的实现中，需要完整的类型信息来查找字段位置
                let field_index = self.resolve_field_index(expr, field).unwrap_or(0);

                // 使用 LoadField 指令加载字段
                instructions.push(Instruction::LoadField {
                    dst: Operand::Local(result_reg),
                    src: Operand::Local(obj_reg),
                    field: field_index,
                });
            }
            Expr::Return(expr, _) => {
                // 生成返回指令
                if let Some(e) = expr {
                    self.generate_expr_ir(e, result_reg, instructions, constants)?;
                    instructions.push(Instruction::Ret(Some(Operand::Local(result_reg))));
                } else {
                    instructions.push(Instruction::Ret(None));
                }
            }
            _ => {
                // 默认返回 0
                instructions.push(Instruction::Load {
                    dst: Operand::Local(result_reg),
                    src: Operand::Const(ConstValue::Int(0)),
                });
            }
        }
        Ok(())
    }
}

/// IR 生成错误
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum IrGenError {
    /// 未实现的表达式类型
    UnimplementedExpr { expr_type: String, span: Span },

    /// 未实现的语句类型
    UnimplementedStmt { stmt_type: String, span: Span },

    /// 无效的操作数
    InvalidOperand { span: Span },

    /// 内部错误
    InternalError { message: String, span: Span },
}

impl std::fmt::Display for IrGenError {
    fn fmt(
        &self,
        f: &mut std::fmt::Formatter<'_>,
    ) -> std::fmt::Result {
        match self {
            IrGenError::UnimplementedExpr { expr_type, span: _ } => {
                write!(f, "未实现的表达式类型: {}", expr_type)
            }
            IrGenError::UnimplementedStmt { stmt_type, span: _ } => {
                write!(f, "未实现的语句类型: {}", stmt_type)
            }
            IrGenError::InvalidOperand { span: _ } => write!(f, "无效的操作数"),
            IrGenError::InternalError { message, span: _ } => write!(f, "内部错误: {}", message),
        }
    }
}

impl std::error::Error for IrGenError {}
