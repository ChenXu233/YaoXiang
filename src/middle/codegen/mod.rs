//! 代码生成器
//!
//! 将中间表示（IR）转换为类型化字节码。
//! 核心设计原则：
//! 1. 类型化指令：每条指令携带明确的类型信息
//! 2. 寄存器架构：所有操作在寄存器上进行
//! 3. 单态化输出：泛型已在编译期展开

pub mod bytecode;
pub mod closure;
pub mod control_flow;
pub mod expr;
pub mod generator;
pub mod loop_gen;
pub mod stmt;
pub mod switch;

use crate::frontend::parser::ast::Type;
use crate::frontend::typecheck::MonoType;
use crate::middle::escape_analysis::EscapeAnalysisResult;
use crate::middle::ir::{ConstValue, FunctionIR, Instruction, ModuleIR, Operand};
use crate::vm::opcode::TypedOpcode;
use std::collections::{HashMap, HashSet};
use std::fmt;

/// 代码生成器
///
/// 将中间表示（IR）转换为类型化字节码。
pub struct CodegenContext {
    /// 当前模块
    module: ModuleIR,

    /// 符号表
    symbol_table: SymbolTable,

    /// 常量池
    constant_pool: ConstantPool,

    /// 字节码缓冲区
    bytecode: Vec<u8>,

    /// 当前函数
    current_function: Option<FunctionIR>,

    /// 寄存器分配器
    register_allocator: RegisterAllocator,

    /// 标签生成器
    label_generator: LabelGenerator,

    /// 逃逸分析结果
    escape_analysis: Option<EscapeAnalysisResult>,

    /// 字节码偏移追踪
    code_offsets: HashMap<usize, usize>,

    /// 跳转表
    jump_tables: HashMap<u16, JumpTable>,

    /// 函数索引
    function_indices: HashMap<String, usize>,

    /// 配置
    config: CodegenConfig,

    /// 当前作用域级别
    scope_level: usize,

    /// 当前循环标签 (loop_label, end_label)，用于 break/continue
    current_loop_label: Option<(usize, usize)>,
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

/// 符号表
#[derive(Debug, Default)]
struct SymbolTable {
    /// 符号映射
    symbols: HashMap<String, Symbol>,

    /// 嵌套作用域
    scopes: Vec<HashMap<String, Symbol>>,
}

/// 符号信息
#[derive(Debug, Clone)]
struct Symbol {
    /// 符号名称
    name: String,

    /// 符号类型
    ty: MonoType,

    /// 存储位置
    storage: Storage,

    /// 是否可变
    is_mut: bool,

    /// 作用域级别
    scope_level: usize,
}

/// 存储位置
#[derive(Debug, Clone)]
enum Storage {
    /// 局部变量
    Local(usize),

    /// 参数
    Arg(usize),

    /// 临时寄存器
    Temp(usize),

    /// 全局变量
    Global(usize),
}

/// 常量池
#[derive(Debug, Default)]
struct ConstantPool {
    /// 常量列表
    constants: Vec<ConstValue>,
}

impl ConstantPool {
    fn new() -> Self {
        ConstantPool {
            constants: Vec::new(),
        }
    }

    /// 添加常量并返回索引
    fn add(
        &mut self,
        value: ConstValue,
    ) -> usize {
        self.constants.push(value);
        self.constants.len() - 1
    }

    /// 获取常量
    fn get(
        &self,
        index: usize,
    ) -> Option<&ConstValue> {
        self.constants.get(index)
    }

    /// 构建常量池
    fn build(self) -> Vec<ConstValue> {
        self.constants
    }
}

/// 寄存器分配器
#[derive(Debug, Default)]
struct RegisterAllocator {
    /// 下一个局部变量ID
    next_local: usize,

    /// 下一个临时寄存器ID
    next_temp: usize,

    /// 分配的寄存器
    allocated: HashSet<usize>,
}

impl RegisterAllocator {
    fn new() -> Self {
        RegisterAllocator {
            next_local: 0,
            next_temp: 0,
            allocated: HashSet::new(),
        }
    }

    /// 分配局部变量
    fn alloc_local(&mut self) -> usize {
        let id = self.next_local;
        self.next_local += 1;
        self.allocated.insert(id);
        id
    }

    /// 分配临时寄存器
    fn alloc_temp(&mut self) -> usize {
        let id = self.next_temp;
        self.next_temp += 1;
        self.allocated.insert(id);
        id
    }

    /// 获取下一个局部变量ID
    fn next_local_id(&self) -> usize {
        self.next_local
    }

    /// 获取下一个临时寄存器ID
    fn next_temp_id(&self) -> usize {
        self.next_temp
    }
}

/// 标签生成器
#[derive(Debug, Default)]
struct LabelGenerator {
    /// 下一个标签ID
    next_label: usize,
}

impl LabelGenerator {
    fn new() -> Self {
        LabelGenerator { next_label: 0 }
    }

    /// 生成下一个标签
    fn next(&mut self) -> usize {
        let label = self.next_label;
        self.next_label += 1;
        label
    }
}

/// 跳转表
#[derive(Debug, Clone)]
struct JumpTable {
    /// 表索引
    index: u16,

    /// 跳转目标
    entries: HashMap<usize, usize>,
}

impl JumpTable {
    fn new(index: u16) -> Self {
        JumpTable {
            index,
            entries: HashMap::new(),
        }
    }

    /// 添加条目
    fn add_entry(
        &mut self,
        key: usize,
        target: usize,
    ) {
        self.entries.insert(key, target);
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
        let mut ctx = CodegenContext {
            module,
            symbol_table: SymbolTable::new(),
            constant_pool: ConstantPool::new(),
            bytecode: Vec::new(),
            current_function: None,
            register_allocator: RegisterAllocator::new(),
            label_generator: LabelGenerator::new(),
            escape_analysis: None,
            code_offsets: HashMap::new(),
            jump_tables: HashMap::new(),
            function_indices: HashMap::new(),
            config: CodegenConfig::default(),
            scope_level: 0,
            current_loop_label: None,
        };

        // 为所有函数建立索引
        for (idx, func) in ctx.module.functions.iter().enumerate() {
            ctx.function_indices.insert(func.name.clone(), idx);
        }

        ctx
    }

    /// 生成字节码
    pub fn generate(&mut self) -> Result<BytecodeFile, CodegenError> {
        // 1. 生成常量池
        let const_pool = std::mem::take(&mut self.constant_pool.constants);

        // 2. 生成代码段
        let mut code_section = CodeSection {
            functions: Vec::new(),
        };

        // 克隆函数以避免借用问题
        let functions = self.module.functions.clone();
        for func in functions {
            self.generate_function(&func, &mut code_section)?;
        }

        // 3. 生成类型表
        let type_table: Vec<MonoType> = self
            .module
            .types
            .iter()
            .map(|t| self.type_from_ast(t))
            .collect();

        // 4. 生成文件头
        let header = self.generate_header();

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
        self.current_function = Some(func.clone());
        self.register_allocator = RegisterAllocator::new();

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
        let mut instructions = Vec::new();

        for block in &func.blocks {
            for instr in &block.instructions {
                let bytecode_instr = self.translate_instruction(instr)?;
                instructions.push(bytecode_instr);
            }
        }

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
                        let const_idx = self.add_constant(const_val.clone());
                        Ok(BytecodeInstruction::new(
                            TypedOpcode::LoadConst,
                            vec![dst_reg, (const_idx as u16) as u8, (const_idx >> 8) as u8],
                        ))
                    }
                    _ => {
                        let src_reg = self.operand_to_reg(src)?;
                        Ok(BytecodeInstruction::new(
                            TypedOpcode::Mov,
                            vec![dst_reg, src_reg],
                        ))
                    }
                }
            }

            Store { dst: _, src } => {
                // Store 指令 dst 是存储目标，src 是源
                let src_reg = self.operand_to_reg(src)?;
                Ok(BytecodeInstruction::new(TypedOpcode::Mov, vec![src_reg])) // 简化
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
                    Operand::Const(ConstValue::Int(i)) => *i as u32,
                    _ => 0,
                };
                let mut operands = vec![dst_reg];
                operands.extend_from_slice(&func_id.to_le_bytes());
                operands.push(0); // base_arg_reg
                operands.push(args.len() as u8);
                Ok(BytecodeInstruction::new(TypedOpcode::CallStatic, operands))
            }

            CallAsync {
                dst: _,
                func: _,
                args: _,
            } => Ok(BytecodeInstruction::new(TypedOpcode::Yield, vec![])),

            TailCall { func, args } => {
                // TailCall: func_id(4), base_arg_reg(1), arg_count(1)
                let func_id = match func {
                    Operand::Const(ConstValue::Int(i)) => *i as u32,
                    _ => 0,
                };
                let mut operands = vec![];
                operands.extend_from_slice(&func_id.to_le_bytes());
                operands.push(0); // base_arg_reg
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
            LoadField {
                dst,
                src: _,
                field: _,
            } => {
                let dst_reg = self.operand_to_reg(dst)?;
                Ok(BytecodeInstruction::new(
                    TypedOpcode::GetField,
                    vec![dst_reg, 0, 0, 0],
                ))
            }

            StoreField {
                dst: _,
                field: _,
                src: _,
            } => Ok(BytecodeInstruction::new(
                TypedOpcode::SetField,
                vec![0, 0, 0],
            )),

            LoadIndex {
                dst,
                src: _,
                index: _,
            } => {
                let dst_reg = self.operand_to_reg(dst)?;
                Ok(BytecodeInstruction::new(
                    TypedOpcode::LoadElement,
                    vec![dst_reg, 0, 0],
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
            // 异步操作
            // =====================
            Spawn { func: _ } => Ok(BytecodeInstruction::new(TypedOpcode::Nop, vec![])),

            Await(_) => Ok(BytecodeInstruction::new(TypedOpcode::Yield, vec![])),

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
        self.constant_pool.add(value)
    }

    /// 获取下一个临时寄存器
    fn next_temp(&mut self) -> usize {
        self.register_allocator.alloc_temp()
    }

    /// 获取下一个局部变量
    fn next_local(&mut self) -> usize {
        self.register_allocator.alloc_local()
    }

    /// 生成标签
    fn next_label(&mut self) -> usize {
        self.label_generator.next()
    }

    /// 发射指令
    fn emit(
        &mut self,
        instr: BytecodeInstruction,
    ) {
        self.bytecode.extend(instr.encode());
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
            module: ModuleIR {
                types: Vec::new(),
                constants: Vec::new(),
                globals: Vec::new(),
                functions: Vec::new(),
            },
            symbol_table: SymbolTable::new(),
            constant_pool: ConstantPool::new(),
            bytecode: Vec::new(),
            current_function: None,
            register_allocator: RegisterAllocator::new(),
            label_generator: LabelGenerator::new(),
            escape_analysis: None,
            code_offsets: HashMap::new(),
            jump_tables: HashMap::new(),
            function_indices: HashMap::new(),
            config: CodegenConfig::default(),
            scope_level: 0,
            current_loop_label: None,
        }
    }
}

impl SymbolTable {
    fn new() -> Self {
        SymbolTable {
            symbols: HashMap::new(),
            scopes: vec![HashMap::new()],
        }
    }

    fn insert(
        &mut self,
        name: String,
        symbol: Symbol,
    ) {
        self.symbols.insert(name, symbol);
    }

    fn get(
        &self,
        name: &str,
    ) -> Option<&Symbol> {
        self.symbols.get(name)
    }

    fn push_scope(&mut self) {
        self.scopes.push(HashMap::new());
    }

    fn pop_scope(&mut self) {
        if self.scopes.len() > 1 {
            self.scopes.pop();
        }
    }

    fn current_scope(&mut self) -> &mut HashMap<String, Symbol> {
        self.scopes.last_mut().unwrap()
    }
}

#[cfg(test)]
mod tests;
