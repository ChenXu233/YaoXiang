//! 代码生成器
//!
//! 将中间表示（IR）转换为类型化字节码。
//! 核心设计原则：
//! 1. 类型化指令：每条指令携带明确的类型信息
//! 2. 寄存器架构：所有操作在寄存器上进行
//! 3. 单态化输出：泛型已在编译期展开
//!
//! ## 关于逃逸分析
//!
//! ⚠️ 逃逸分析已废弃，代码中保留仅用于可能的细粒度优化参考。
//! YaoXiang 使用所有权模型，内存分配由程序员显式控制：
//! - 栈分配：默认行为
//! - 堆分配：`Box[T]`、`Arc[T]` 等显式智能指针

pub mod buffer;
pub mod bytecode;
pub mod flow;
pub mod ir_builder;

pub mod gen;

use crate::frontend::parser::ast::Type;
use crate::frontend::typecheck::MonoType;
use crate::middle::ir::{ConstValue, FunctionIR, Instruction, ModuleIR, Operand};
use crate::util::i18n::{t, t_simple, MSG};
use crate::util::logger::get_lang;
use crate::vm::opcode::TypedOpcode;
use std::fmt;
use tracing::debug;

use self::buffer::BytecodeBuffer;
use self::flow::{FlowManager, JumpTable, Storage, Symbol, SymbolScopeManager};

/// 代码生成器
///
/// 将中间表示（IR）转换为类型化字节码。
/// 使用三个管理器分离职责：
/// - BytecodeBuffer: 常量池和字节码缓冲区
/// - SymbolScopeManager: 符号表和作用域
/// - FlowManager: 寄存器分配、标签、跳转表
pub struct CodegenContext {
    /// 当前模块
    module: ModuleIR,

    /// 当前函数
    current_function: Option<FunctionIR>,

    /// 字节码缓冲区（常量池 + 字节码）
    bytecode_buffer: BytecodeBuffer,

    /// 符号表和作用域管理
    symbols: SymbolScopeManager,

    /// 控制流管理（寄存器、标签、跳转表）
    flow: FlowManager,

    /// 配置
    config: CodegenConfig,
}

#[derive(Debug, Clone)]
struct CodegenConfig {
    /// 是否启用优化
    enable_optimizations: bool,

    /// 是否生成调试信息
    generate_debug_info: bool,

    /// 是否启用内联缓存
    enable_inline_cache: bool,
}

impl Default for CodegenConfig {
    fn default() -> Self {
        CodegenConfig {
            enable_optimizations: true,
            generate_debug_info: false,
            enable_inline_cache: true,
        }
    }
}

/// 代码生成错误
#[derive(Debug, Clone)]
pub enum CodegenError {
    /// 未实现的表达式类型
    UnimplementedExpr { expr_type: String },

    /// 未实现的语句类型
    UnimplementedStmt { stmt_type: String },

    /// 未实现的调用
    UnimplementedCall,

    /// 无效的赋值目标
    InvalidAssignmentTarget,

    /// 符号未找到
    SymbolNotFound { name: String },

    /// 类型不匹配
    TypeMismatch { expected: String, found: String },

    /// 寄存器不足
    OutOfRegisters,

    /// 无效的操作数
    InvalidOperand,
}

impl fmt::Display for CodegenError {
    fn fmt(
        &self,
        f: &mut fmt::Formatter<'_>,
    ) -> fmt::Result {
        match self {
            CodegenError::UnimplementedExpr { expr_type } => {
                write!(f, "未实现的表达式类型: {}", expr_type)
            }
            CodegenError::UnimplementedStmt { stmt_type } => {
                write!(f, "未实现的语句类型: {}", stmt_type)
            }
            CodegenError::UnimplementedCall => write!(f, "未实现的函数调用"),
            CodegenError::InvalidAssignmentTarget => write!(f, "无效的赋值目标"),
            CodegenError::SymbolNotFound { name } => write!(f, "符号未找到: {}", name),
            CodegenError::TypeMismatch { expected, found } => {
                write!(f, "类型不匹配: 期望 {}, 实际 {}", expected, found)
            }
            CodegenError::OutOfRegisters => write!(f, "寄存器不足"),
            CodegenError::InvalidOperand => write!(f, "无效的操作数"),
        }
    }
}

impl std::error::Error for CodegenError {}

impl CodegenContext {
    /// 创建新的代码生成上下文
    pub fn new(module: ModuleIR) -> Self {
        let lang = get_lang();
        let func_count = module.functions.len();
        debug!("{}", t(MSG::CodegenStart, lang, Some(&[&func_count])));

        let mut ctx = CodegenContext {
            module,
            current_function: None,
            bytecode_buffer: BytecodeBuffer::new(),
            symbols: SymbolScopeManager::new(),
            flow: FlowManager::new(),
            config: CodegenConfig::default(),
        };

        // 为所有函数建立索引
        for (idx, func) in ctx.module.functions.iter().enumerate() {
            ctx.flow.add_function_index(func.name.clone(), idx);
        }

        ctx
    }

    /// 生成字节码
    pub fn generate(&mut self) -> Result<BytecodeFile, CodegenError> {
        let lang = get_lang();
        let func_count = self.module.functions.len();
        debug!("{}", t(MSG::CodegenFunctions, lang, Some(&[&func_count])));

        // 1. 生成代码段（这会填充常量池）
        let mut code_section = CodeSection {
            functions: Vec::new(),
        };

        debug!("{}", t(MSG::CodegenCodeSection, lang, Some(&[&func_count])));
        // 克隆函数以避免借用问题
        let functions = self.module.functions.clone();
        for func in functions {
            self.generate_function(&func, &mut code_section)?;
        }

        // 2. 生成常量池（在代码段生成之后）
        let const_pool = self.bytecode_buffer.take_constant_pool();
        let const_count = const_pool.len();
        debug!("{}", t(MSG::CodegenConstPool, lang, Some(&[&const_count])));

        // 3. 生成类型表
        let type_count = self.module.types.len();
        let type_table: Vec<MonoType> = self
            .module
            .types
            .iter()
            .map(|t| self.type_from_ast(t))
            .collect();
        debug!("{}", t(MSG::CodegenTypeTable, lang, Some(&[&type_count])));

        // 4. 生成文件头
        let header = self.generate_header();

        debug!("{}", t_simple(MSG::CodegenComplete, lang));
        Ok(BytecodeFile {
            header,
            type_table,
            const_pool,
            code_section,
        })
    }

    /// 生成函数
    fn generate_function(
        &mut self,
        func: &FunctionIR,
        code_section: &mut CodeSection,
    ) -> Result<(), CodegenError> {
        let lang = get_lang();
        debug!("{}", t(MSG::CodegenGenFn, lang, Some(&[&func.name])));

        self.current_function = Some(func.clone());
        self.flow.reset_registers();

        // 生成函数体
        let instructions = self.generate_instructions(func)?;

        code_section.functions.push(FunctionCode {
            name: func.name.clone(),
            params: func.params.clone(),
            return_type: func.return_type.clone(),
            instructions,
            local_count: func.locals.len(),
        });

        Ok(())
    }

    /// 生成函数指令
    fn generate_instructions(
        &mut self,
        func: &FunctionIR,
    ) -> Result<Vec<BytecodeInstruction>, CodegenError> {
        let lang = get_lang();
        let mut instructions = Vec::new();

        for (block_idx, block) in func.blocks.iter().enumerate() {
            debug!("{}", t(MSG::CodegenGenBlock, lang, Some(&[&block_idx])));

            for instr in &block.instructions {
                let bytecode_instr = self.translate_instruction(instr)?;
                let instr_name = format!("{:?}", instr);
                debug!("{}", t(MSG::CodegenGenInstr, lang, Some(&[&instr_name])));
                instructions.push(bytecode_instr);
            }
        }

        // Log register allocation
        let next_local = self.flow.next_local_id();
        let next_temp = self.flow.next_temp_id();
        debug!(
            "{}",
            t(
                MSG::CodegenRegAlloc,
                lang,
                Some(&[&format!("locals={}, temps={}", next_local, next_temp)])
            )
        );

        Ok(instructions)
    }

    /// 翻译 IR 指令为字节码指令
    fn translate_instruction(
        &mut self,
        instr: &Instruction,
    ) -> Result<BytecodeInstruction, CodegenError> {
        use Instruction::*;

        match instr {
            // =====================
            // 移动和加载
            // =====================
            Move { dst, src } => {
                let dst_reg = self.operand_to_reg(dst)?;
                let src_reg = self.operand_to_reg(src)?;
                Ok(BytecodeInstruction::new(
                    TypedOpcode::Mov,
                    vec![dst_reg, src_reg],
                ))
            }

            Load { dst, src } => {
                let dst_reg = self.operand_to_reg(dst)?;
                // src 可以是常量或寄存器
                match src {
                    Operand::Const(const_val) => {
                        // 常量统一放入常量池
                        let const_idx = self.add_constant(const_val.clone());
                        Ok(BytecodeInstruction::new(
                            TypedOpcode::LoadConst,
                            vec![dst_reg, (const_idx as u16) as u8, (const_idx >> 8) as u8],
                        ))
                    }
                    Operand::Local(local_idx) => Ok(BytecodeInstruction::new(
                        TypedOpcode::LoadLocal,
                        vec![dst_reg, *local_idx as u8],
                    )),
                    Operand::Arg(arg_idx) => Ok(BytecodeInstruction::new(
                        TypedOpcode::LoadArg,
                        vec![dst_reg, *arg_idx as u8],
                    )),
                    _ => {
                        let src_reg = self.operand_to_reg(src)?;
                        Ok(BytecodeInstruction::new(
                            TypedOpcode::Mov,
                            vec![dst_reg, src_reg],
                        ))
                    }
                }
            }

            Store { dst, src } => {
                // Store 指令：写入局部变量
                if let Operand::Local(local_idx) = dst {
                    let src_reg = self.operand_to_reg(src)?;
                    Ok(BytecodeInstruction::new(
                        TypedOpcode::StoreLocal,
                        vec![*local_idx as u8, src_reg],
                    ))
                } else {
                    Err(CodegenError::InvalidOperand)
                }
            }

            // =====================
            // 整数运算 (I64)
            // =====================
            Add { dst, lhs, rhs } => {
                let dst_reg = self.operand_to_reg(dst)?;
                let lhs_reg = self.operand_to_reg(lhs)?;
                let rhs_reg = self.operand_to_reg(rhs)?;
                Ok(BytecodeInstruction::new(
                    TypedOpcode::I64Add,
                    vec![dst_reg, lhs_reg, rhs_reg],
                ))
            }

            Sub { dst, lhs, rhs } => {
                let dst_reg = self.operand_to_reg(dst)?;
                let lhs_reg = self.operand_to_reg(lhs)?;
                let rhs_reg = self.operand_to_reg(rhs)?;
                Ok(BytecodeInstruction::new(
                    TypedOpcode::I64Sub,
                    vec![dst_reg, lhs_reg, rhs_reg],
                ))
            }

            Mul { dst, lhs, rhs } => {
                let dst_reg = self.operand_to_reg(dst)?;
                let lhs_reg = self.operand_to_reg(lhs)?;
                let rhs_reg = self.operand_to_reg(rhs)?;
                Ok(BytecodeInstruction::new(
                    TypedOpcode::I64Mul,
                    vec![dst_reg, lhs_reg, rhs_reg],
                ))
            }

            Div { dst, lhs, rhs } => {
                let dst_reg = self.operand_to_reg(dst)?;
                let lhs_reg = self.operand_to_reg(lhs)?;
                let rhs_reg = self.operand_to_reg(rhs)?;
                Ok(BytecodeInstruction::new(
                    TypedOpcode::I64Div,
                    vec![dst_reg, lhs_reg, rhs_reg],
                ))
            }

            Mod { dst, lhs, rhs } => {
                let dst_reg = self.operand_to_reg(dst)?;
                let lhs_reg = self.operand_to_reg(lhs)?;
                let rhs_reg = self.operand_to_reg(rhs)?;
                Ok(BytecodeInstruction::new(
                    TypedOpcode::I64Rem,
                    vec![dst_reg, lhs_reg, rhs_reg],
                ))
            }

            // =====================
            // 位运算指令
            // =====================
            And { dst, lhs, rhs } => {
                let dst_reg = self.operand_to_reg(dst)?;
                let lhs_reg = self.operand_to_reg(lhs)?;
                let rhs_reg = self.operand_to_reg(rhs)?;
                Ok(BytecodeInstruction::new(
                    TypedOpcode::I64And,
                    vec![dst_reg, lhs_reg, rhs_reg],
                ))
            }

            Or { dst, lhs, rhs } => {
                let dst_reg = self.operand_to_reg(dst)?;
                let lhs_reg = self.operand_to_reg(lhs)?;
                let rhs_reg = self.operand_to_reg(rhs)?;
                Ok(BytecodeInstruction::new(
                    TypedOpcode::I64Or,
                    vec![dst_reg, lhs_reg, rhs_reg],
                ))
            }

            Xor { dst, lhs, rhs } => {
                let dst_reg = self.operand_to_reg(dst)?;
                let lhs_reg = self.operand_to_reg(lhs)?;
                let rhs_reg = self.operand_to_reg(rhs)?;
                Ok(BytecodeInstruction::new(
                    TypedOpcode::I64Xor,
                    vec![dst_reg, lhs_reg, rhs_reg],
                ))
            }

            Shl { dst, lhs, rhs } => {
                let dst_reg = self.operand_to_reg(dst)?;
                let lhs_reg = self.operand_to_reg(lhs)?;
                let rhs_reg = self.operand_to_reg(rhs)?;
                Ok(BytecodeInstruction::new(
                    TypedOpcode::I64Shl,
                    vec![dst_reg, lhs_reg, rhs_reg],
                ))
            }

            Shr { dst, lhs, rhs } => {
                let dst_reg = self.operand_to_reg(dst)?;
                let lhs_reg = self.operand_to_reg(lhs)?;
                let rhs_reg = self.operand_to_reg(rhs)?;
                Ok(BytecodeInstruction::new(
                    TypedOpcode::I64Shr,
                    vec![dst_reg, lhs_reg, rhs_reg],
                ))
            }

            Sar { dst, lhs, rhs } => {
                let dst_reg = self.operand_to_reg(dst)?;
                let lhs_reg = self.operand_to_reg(lhs)?;
                let rhs_reg = self.operand_to_reg(rhs)?;
                Ok(BytecodeInstruction::new(
                    TypedOpcode::I64Sar,
                    vec![dst_reg, lhs_reg, rhs_reg],
                ))
            }

            Neg { dst, src } => {
                let dst_reg = self.operand_to_reg(dst)?;
                let src_reg = self.operand_to_reg(src)?;
                Ok(BytecodeInstruction::new(
                    TypedOpcode::I64Neg,
                    vec![dst_reg, src_reg],
                ))
            }

            // =====================
            // 比较指令
            // =====================
            Eq { dst, lhs, rhs } => {
                let dst_reg = self.operand_to_reg(dst)?;
                let lhs_reg = self.operand_to_reg(lhs)?;
                let rhs_reg = self.operand_to_reg(rhs)?;
                Ok(BytecodeInstruction::new(
                    TypedOpcode::I64Eq,
                    vec![dst_reg, lhs_reg, rhs_reg],
                ))
            }

            Ne { dst, lhs, rhs } => {
                let dst_reg = self.operand_to_reg(dst)?;
                let lhs_reg = self.operand_to_reg(lhs)?;
                let rhs_reg = self.operand_to_reg(rhs)?;
                Ok(BytecodeInstruction::new(
                    TypedOpcode::I64Ne,
                    vec![dst_reg, lhs_reg, rhs_reg],
                ))
            }

            Lt { dst, lhs, rhs } => {
                let dst_reg = self.operand_to_reg(dst)?;
                let lhs_reg = self.operand_to_reg(lhs)?;
                let rhs_reg = self.operand_to_reg(rhs)?;
                Ok(BytecodeInstruction::new(
                    TypedOpcode::I64Lt,
                    vec![dst_reg, lhs_reg, rhs_reg],
                ))
            }

            Le { dst, lhs, rhs } => {
                let dst_reg = self.operand_to_reg(dst)?;
                let lhs_reg = self.operand_to_reg(lhs)?;
                let rhs_reg = self.operand_to_reg(rhs)?;
                Ok(BytecodeInstruction::new(
                    TypedOpcode::I64Le,
                    vec![dst_reg, lhs_reg, rhs_reg],
                ))
            }

            Gt { dst, lhs, rhs } => {
                let dst_reg = self.operand_to_reg(dst)?;
                let lhs_reg = self.operand_to_reg(lhs)?;
                let rhs_reg = self.operand_to_reg(rhs)?;
                Ok(BytecodeInstruction::new(
                    TypedOpcode::I64Gt,
                    vec![dst_reg, lhs_reg, rhs_reg],
                ))
            }

            Ge { dst, lhs, rhs } => {
                let dst_reg = self.operand_to_reg(dst)?;
                let lhs_reg = self.operand_to_reg(lhs)?;
                let rhs_reg = self.operand_to_reg(rhs)?;
                Ok(BytecodeInstruction::new(
                    TypedOpcode::I64Ge,
                    vec![dst_reg, lhs_reg, rhs_reg],
                ))
            }

            // =====================
            // 控制流
            // =====================
            Jmp(target) => {
                let offset = *target as i32;
                let bytes = offset.to_le_bytes();
                Ok(BytecodeInstruction::new(TypedOpcode::Jmp, bytes.to_vec()))
            }

            JmpIf(cond, target) => {
                let cond_reg = self.operand_to_reg(cond)?;
                let offset = *target as i32;
                let offset_bytes = (offset as i16).to_le_bytes();
                Ok(BytecodeInstruction::new(
                    TypedOpcode::JmpIf,
                    vec![cond_reg, offset_bytes[0], offset_bytes[1]],
                ))
            }

            JmpIfNot(cond, target) => {
                let cond_reg = self.operand_to_reg(cond)?;
                let offset = *target as i32;
                let offset_bytes = (offset as i16).to_le_bytes();
                Ok(BytecodeInstruction::new(
                    TypedOpcode::JmpIfNot,
                    vec![cond_reg, offset_bytes[0], offset_bytes[1]],
                ))
            }

            Ret(value) => {
                if let Some(v) = value {
                    let reg = self.operand_to_reg(v)?;
                    Ok(BytecodeInstruction::new(
                        TypedOpcode::ReturnValue,
                        vec![reg],
                    ))
                } else {
                    Ok(BytecodeInstruction::new(TypedOpcode::Return, vec![]))
                }
            }

            // =====================
            // 函数调用
            // =====================
            Call { dst, func, args } => {
                // Call: dst(1), func_id(4), base_arg_reg(1), arg_count(1)
                let dst_reg = if let Some(d) = dst {
                    self.operand_to_reg(d)?
                } else {
                    0
                };
                let func_id = match func {
                    // 用户定义函数或外部函数：func_id 是常量池索引
                    Operand::Const(ConstValue::Int(i)) => *i as u32,
                    // 如果是字符串常量（外部函数名），添加到常量池并使用索引
                    Operand::Const(ConstValue::String(name)) => {
                        let const_idx = self.add_constant(ConstValue::String(name.clone()));
                        const_idx as u32
                    }
                    _ => 0,
                };
                let base_arg_reg = if let Some(first_arg) = args.first() {
                    self.operand_to_reg(first_arg)?
                } else {
                    0
                };
                let mut operands = vec![dst_reg];
                operands.extend_from_slice(&func_id.to_le_bytes());
                operands.push(base_arg_reg); // base_arg_reg
                operands.push(args.len() as u8);
                Ok(BytecodeInstruction::new(TypedOpcode::CallStatic, operands))
            }

            // 注意：根据 RFC-008，CallAsync 已移除
            // await 不是关键字，运行时自动处理
            TailCall { func, args } => {
                // TailCall: func_id(4), base_arg_reg(1), arg_count(1)
                let func_id = match func {
                    Operand::Const(ConstValue::Int(i)) => *i as u32,
                    _ => 0,
                };
                let base_arg_reg = if let Some(first_arg) = args.first() {
                    self.operand_to_reg(first_arg)?
                } else {
                    0
                };
                let mut operands = vec![];
                operands.extend_from_slice(&func_id.to_le_bytes());
                operands.push(base_arg_reg); // base_arg_reg
                operands.push(args.len() as u8);
                Ok(BytecodeInstruction::new(TypedOpcode::TailCall, operands))
            }

            // =====================
            // 内存操作
            // =====================
            Alloc { dst, size: _ } => {
                let dst_reg = self.operand_to_reg(dst)?;
                Ok(BytecodeInstruction::new(
                    TypedOpcode::StackAlloc,
                    vec![dst_reg],
                ))
            }

            Free(_) => Ok(BytecodeInstruction::new(TypedOpcode::Nop, vec![])),

            AllocArray {
                dst,
                size: _,
                elem_size: _,
            } => {
                let dst_reg = self.operand_to_reg(dst)?;
                Ok(BytecodeInstruction::new(
                    TypedOpcode::NewListWithCap,
                    vec![dst_reg, 0, 0],
                ))
            }

            // =====================
            // 字段操作
            // =====================
            LoadField { dst, src, field } => {
                // GetField: dst(1), src(1), field_offset(u16, 2字节)
                let dst_reg = self.operand_to_reg(dst)?;
                let src_reg = self.operand_to_reg(src)?;
                let field_offset = *field as u16;
                Ok(BytecodeInstruction::new(
                    TypedOpcode::GetField,
                    vec![
                        dst_reg,
                        src_reg,
                        (field_offset & 0xFF) as u8,
                        (field_offset >> 8) as u8,
                    ],
                ))
            }

            StoreField { dst, field, src } => {
                // SetField: dst(1), field_offset(u16, 2字节), src(1)
                let dst_reg = self.operand_to_reg(dst)?;
                let src_reg = self.operand_to_reg(src)?;
                let field_offset = *field as u16;
                Ok(BytecodeInstruction::new(
                    TypedOpcode::SetField,
                    vec![
                        dst_reg,
                        (field_offset & 0xFF) as u8,
                        (field_offset >> 8) as u8,
                        src_reg,
                    ],
                ))
            }

            LoadIndex { dst, src, index } => {
                // LoadElement: dst, src, index
                let dst_reg = self.operand_to_reg(dst)?;
                let src_reg = self.operand_to_reg(src)?;
                let index_reg = self.operand_to_reg(index)?;
                Ok(BytecodeInstruction::new(
                    TypedOpcode::LoadElement,
                    vec![dst_reg, src_reg, index_reg],
                ))
            }

            StoreIndex {
                dst: _,
                index: _,
                src: _,
            } => Ok(BytecodeInstruction::new(
                TypedOpcode::StoreElement,
                vec![0, 0, 0],
            )),

            // =====================
            // 类型操作
            // =====================
            Cast {
                dst,
                src,
                target_type: _,
            } => {
                let dst_reg = self.operand_to_reg(dst)?;
                let src_reg = self.operand_to_reg(src)?;
                Ok(BytecodeInstruction::new(
                    TypedOpcode::Cast,
                    vec![dst_reg, src_reg, 0, 0],
                ))
            }

            TypeTest(_, _) => Ok(BytecodeInstruction::new(
                TypedOpcode::TypeCheck,
                vec![0, 0, 0],
            )),

            // =====================
            // 并发操作（基于 RFC-008）
            // spawn 是注解标记，await 不是关键字（运行时自动处理）
            // =====================
            Spawn { func: _, .. } => {
                // 根据 RFC-008，spawn 标记由运行时处理
                // 编译产物是普通函数调用，调度器负责创建 Async[T]
                Ok(BytecodeInstruction::new(TypedOpcode::Nop, vec![]))
            }

            Yield => Ok(BytecodeInstruction::new(TypedOpcode::Yield, vec![])),

            // =====================
            // 内存管理
            // =====================
            HeapAlloc { dst, type_id: _ } => {
                let dst_reg = self.operand_to_reg(dst)?;
                Ok(BytecodeInstruction::new(
                    TypedOpcode::HeapAlloc,
                    vec![dst_reg, 0, 0],
                ))
            }

            MakeClosure { dst, func, env: _ } => {
                // MakeClosure: dst(1), func_id(u32, 4字节), upvalue_count(1)
                let dst_reg = self.operand_to_reg(dst)?;
                // func 是函数索引 (usize)，不是 Operand
                let func_id = *func as u32;
                let mut operands = vec![dst_reg];
                operands.extend_from_slice(&func_id.to_le_bytes());
                operands.push(0); // upvalue_count
                Ok(BytecodeInstruction::new(TypedOpcode::MakeClosure, operands))
            }

            Drop(operand) => {
                let reg = self.operand_to_reg(operand)?;
                Ok(BytecodeInstruction::new(TypedOpcode::Drop, vec![reg]))
            }

            // =====================
            // 栈操作
            // =====================
            Push(operand) => {
                let reg = self.operand_to_reg(operand)?;
                Ok(BytecodeInstruction::new(TypedOpcode::Mov, vec![reg]))
            }

            Pop(operand) => {
                let reg = self.operand_to_reg(operand)?;
                Ok(BytecodeInstruction::new(TypedOpcode::Mov, vec![reg]))
            }

            Dup => Ok(BytecodeInstruction::new(TypedOpcode::Nop, vec![])),

            Swap => Ok(BytecodeInstruction::new(TypedOpcode::Nop, vec![])),

            // =====================
            // Arc 指令
            // =====================
            ArcNew { dst, src } => {
                let dst_reg = self.operand_to_reg(dst)?;
                let src_reg = self.operand_to_reg(src)?;
                Ok(BytecodeInstruction::new(
                    TypedOpcode::ArcNew,
                    vec![dst_reg, src_reg],
                ))
            }

            ArcClone { dst, src } => {
                let dst_reg = self.operand_to_reg(dst)?;
                let src_reg = self.operand_to_reg(src)?;
                Ok(BytecodeInstruction::new(
                    TypedOpcode::ArcClone,
                    vec![dst_reg, src_reg],
                ))
            }

            ArcDrop(operand) => {
                let reg = self.operand_to_reg(operand)?;
                Ok(BytecodeInstruction::new(TypedOpcode::ArcDrop, vec![reg]))
            }

            // =====================
            // 字符串指令
            // =====================
            StringLength { dst, src } => {
                let dst_reg = self.operand_to_reg(dst)?;
                let src_reg = self.operand_to_reg(src)?;
                Ok(BytecodeInstruction::new(
                    TypedOpcode::StringLength,
                    vec![dst_reg, src_reg],
                ))
            }

            StringConcat { dst, lhs, rhs } => {
                let dst_reg = self.operand_to_reg(dst)?;
                let lhs_reg = self.operand_to_reg(lhs)?;
                let rhs_reg = self.operand_to_reg(rhs)?;
                Ok(BytecodeInstruction::new(
                    TypedOpcode::StringConcat,
                    vec![dst_reg, lhs_reg, rhs_reg],
                ))
            }

            StringGetChar { dst, src, index } => {
                let dst_reg = self.operand_to_reg(dst)?;
                let src_reg = self.operand_to_reg(src)?;
                let index_reg = self.operand_to_reg(index)?;
                Ok(BytecodeInstruction::new(
                    TypedOpcode::StringGetChar,
                    vec![dst_reg, src_reg, index_reg],
                ))
            }

            StringFromInt { dst, src } => {
                let dst_reg = self.operand_to_reg(dst)?;
                let src_reg = self.operand_to_reg(src)?;
                Ok(BytecodeInstruction::new(
                    TypedOpcode::StringFromInt,
                    vec![dst_reg, src_reg],
                ))
            }

            StringFromFloat { dst, src } => {
                let dst_reg = self.operand_to_reg(dst)?;
                let src_reg = self.operand_to_reg(src)?;
                Ok(BytecodeInstruction::new(
                    TypedOpcode::StringFromFloat,
                    vec![dst_reg, src_reg],
                ))
            }

            // =====================
            // 闭包 Upvalue 指令
            // =====================
            LoadUpvalue { dst, upvalue_idx } => {
                let dst_reg = self.operand_to_reg(dst)?;
                Ok(BytecodeInstruction::new(
                    TypedOpcode::LoadUpvalue,
                    vec![dst_reg, *upvalue_idx as u8],
                ))
            }

            StoreUpvalue { src, upvalue_idx } => {
                let src_reg = self.operand_to_reg(src)?;
                Ok(BytecodeInstruction::new(
                    TypedOpcode::StoreUpvalue,
                    vec![src_reg, *upvalue_idx as u8],
                ))
            }

            CloseUpvalue(operand) => {
                let reg = self.operand_to_reg(operand)?;
                Ok(BytecodeInstruction::new(TypedOpcode::CloseUpvalue, vec![reg]))
            }
        }
    }

    /// 将操作数转换为寄存器编号
    fn operand_to_reg(
        &self,
        operand: &Operand,
    ) -> Result<u8, CodegenError> {
        match operand {
            Operand::Local(id) => Ok(*id as u8),
            Operand::Temp(id) => Ok(*id as u8),
            Operand::Arg(id) => Ok(*id as u8),
            _ => Err(CodegenError::InvalidOperand),
        }
    }

    /// 生成文件头
    fn generate_header(&self) -> BytecodeHeader {
        BytecodeHeader {
            magic: YAOXIANG_MAGIC,
            version: BYTECODE_VERSION,
            flags: self.compute_flags(),
            entry_point: self.find_entry_point() as u32,
            section_count: 4,
            file_size: 0,
            checksum: 0,
        }
    }

    /// 计算文件标志
    fn compute_flags(&self) -> u32 {
        let mut flags = 0u32;
        if self.config.generate_debug_info {
            flags |= 0x02;
        }
        flags
    }

    /// 查找入口点
    fn find_entry_point(&self) -> usize {
        for (idx, func) in self.module.functions.iter().enumerate() {
            if func.name == "main" {
                return idx;
            }
        }
        0
    }

    /// 从 AST 类型转换
    #[allow(clippy::only_used_in_recursion)]
    fn type_from_ast(
        &self,
        ast_type: &Type,
    ) -> MonoType {
        match ast_type {
            Type::Name(name) => MonoType::TypeRef(name.clone()),
            Type::Int(n) => MonoType::Int(*n),
            Type::Float(n) => MonoType::Float(*n),
            Type::Char => MonoType::Char,
            Type::String => MonoType::String,
            Type::Bool => MonoType::Bool,
            Type::Void => MonoType::Void,
            Type::List(elem) => MonoType::List(Box::new(self.type_from_ast(elem))),
            Type::Dict(key, value) => MonoType::Dict(
                Box::new(self.type_from_ast(key)),
                Box::new(self.type_from_ast(value)),
            ),
            Type::Tuple(types) => {
                MonoType::Tuple(types.iter().map(|t| self.type_from_ast(t)).collect())
            }
            Type::Fn {
                params,
                return_type,
            } => MonoType::Fn {
                params: params.iter().map(|t| self.type_from_ast(t)).collect(),
                return_type: Box::new(self.type_from_ast(return_type)),
                is_async: false,
            },
            _ => MonoType::Void,
        }
    }

    /// 添加常量
    fn add_constant(
        &mut self,
        value: ConstValue,
    ) -> usize {
        self.bytecode_buffer.add_constant(value)
    }

    /// 获取下一个临时寄存器
    fn next_temp(&mut self) -> usize {
        self.flow.alloc_temp()
    }

    /// 获取下一个局部变量
    fn next_local(&mut self) -> usize {
        self.flow.alloc_local()
    }

    /// 生成标签
    fn next_label(&mut self) -> usize {
        self.flow.next_label()
    }

    /// 发射指令
    fn emit(
        &mut self,
        instr: BytecodeInstruction,
    ) {
        self.bytecode_buffer.emit(&instr.encode());
    }
}

pub use bytecode::BytecodeFile;
pub use bytecode::BytecodeInstruction;
pub use bytecode::CodeSection;
pub use bytecode::FileHeader as BytecodeHeader;
pub use bytecode::FunctionCode;

/// 常量定义
pub const YAOXIANG_MAGIC: u32 = 0x59584243;
pub const BYTECODE_VERSION: u32 = 2;

impl Default for CodegenContext {
    fn default() -> Self {
        CodegenContext {
            module: ModuleIR::default(),
            current_function: None,
            bytecode_buffer: BytecodeBuffer::new(),
            symbols: SymbolScopeManager::new(),
            flow: FlowManager::new(),
            config: CodegenConfig::default(),
        }
    }
}

#[cfg(test)]
mod tests;
