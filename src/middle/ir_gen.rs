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
use crate::frontend::typecheck::MonoType;
use crate::middle::ir::{BasicBlock, ConstValue, FunctionIR, Instruction, ModuleIR, Operand};
use crate::util::span::Span;
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
            next_temp: 0,
        }
    }

    /// 进入新的作用域
    fn enter_scope(&mut self) {
        self.symbols.push(HashMap::new());
    }

    /// 退出当前作用域
    fn exit_scope(&mut self) {
        self.symbols.pop();
    }

    /// 注册局部变量
    fn register_local(
        &mut self,
        name: &str,
        local_idx: usize,
    ) {
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
                return Some(entry.local_idx);
            }
        }
        None
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
            _ => Ok(None),
        }
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
            self.generate_local_stmt_ir(stmt, &mut instructions, constants)?;
        }

        // 退出函数体作用域
        self.exit_scope();

        // 处理返回值表达式
        if let Some(e) = expr {
            let result_reg = self.next_temp_reg();
            self.generate_expr_ir(e, result_reg, &mut instructions, constants)?;
            instructions.push(Instruction::Ret(Some(Operand::Local(result_reg))));
        } else {
            instructions.push(Instruction::Ret(None));
        }

        // 计算局部变量总数（用于 VM 分配帧空间）
        // 局部变量包括参数和函数体中声明的变量
        // 参数数量 + 临时寄存器使用数量
        let total_locals = self.next_temp;
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

        Ok(Some(func_ir))
    }

    /// 生成全局变量 IR
    fn generate_global_var_ir(
        &self,
        name: &str,
        type_annotation: Option<&ast::Type>,
        initializer: Option<&ast::Expr>,
    ) -> Result<Option<FunctionIR>, IrGenError> {
        let var_type = type_annotation
            .map(|t| (*t).clone().into())
            .unwrap_or(MonoType::Int(64));

        let init_instr = if let Some(_expr) = initializer {
            // 简化：假设初始化为整数常量
            Instruction::Load {
                dst: Operand::Global(0),
                src: Operand::Const(ConstValue::Int(0)),
            }
        } else {
            Instruction::Load {
                dst: Operand::Global(0),
                src: Operand::Const(ConstValue::Int(0)),
            }
        };

        // 为全局变量创建函数（简化处理）
        let func_ir = FunctionIR {
            name: name.to_string(),
            params: Vec::new(),
            return_type: var_type,
            is_async: false,
            locals: Vec::new(),
            blocks: vec![BasicBlock {
                label: 0,
                instructions: vec![init_instr, Instruction::Ret(None)],
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
                // 二元运算
                let left_reg = result_reg;
                let right_reg = result_reg + 1;

                self.generate_expr_ir(left, left_reg, instructions, constants)?;
                self.generate_expr_ir(right, right_reg, instructions, constants)?;

                let instr = match op {
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
                    ast::BinOp::Assign => {
                        // 赋值操作: left = right
                        // 注意：left 应该是一个变量（Expr::Var），right 是要赋值表达式
                        // 查找左边的变量索引
                        if let Expr::Var(var_name, _) = left.as_ref() {
                            if let Some(local_idx) = self.lookup_local(var_name) {
                                // 生成右侧表达式的 IR 到 right_reg
                                self.generate_expr_ir(right, right_reg, instructions, constants)?;
                                // 生成 Store 指令将右侧的值存储到局部变量
                                instructions.push(Instruction::Store {
                                    dst: Operand::Local(local_idx),
                                    src: Operand::Local(right_reg),
                                });
                                // 将右侧的值复制到结果寄存器（赋值表达式的值）
                                instructions.push(Instruction::Load {
                                    dst: Operand::Local(result_reg),
                                    src: Operand::Local(local_idx),
                                });
                                return Ok(());
                            }
                        }
                        // 如果找不到变量，使用默认行为
                        Instruction::Add {
                            dst: Operand::Local(result_reg),
                            lhs: Operand::Local(left_reg),
                            rhs: Operand::Local(right_reg),
                        }
                    }
                    _ => Instruction::Add {
                        dst: Operand::Local(result_reg),
                        lhs: Operand::Local(left_reg),
                        rhs: Operand::Local(right_reg),
                    },
                };
                instructions.push(instr);
            }
            Expr::Call {
                func,
                args,
                span: _,
            } => {
                // 函数调用
                let mut arg_regs = Vec::new();
                for (i, arg) in args.iter().enumerate() {
                    self.generate_expr_ir(arg, result_reg + i + 1, instructions, constants)?;
                    arg_regs.push(Operand::Local(result_reg + i + 1));
                }

                // 函数名添加到常量池，使用索引
                let func_idx = if let Expr::Var(name, _) = func.as_ref() {
                    let const_idx = constants.len();
                    constants.push(ConstValue::String(name.clone()));
                    const_idx as i128
                } else {
                    0
                };

                instructions.push(Instruction::Call {
                    dst: Some(Operand::Local(result_reg)),
                    func: Operand::Const(ConstValue::Int(func_idx)),
                    args: arg_regs,
                });
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
