//! 代码生成器
//!
//! 将中间表示（IR）转换为类型化字节码。
//! 核心设计原则：
//! 1. 类型化指令：每条指令携带明确的类型信息
//! 2. 寄存器架构：所有操作在寄存器上进行
//! 3. 单态化输出：泛型已在编译期展开

pub mod expr;
pub mod stmt;
pub mod control_flow;
pub mod switch;
pub mod loop_gen;
pub mod closure;
pub mod bytecode;
pub mod generator;

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
    fn add(&mut self, value: ConstValue) -> usize {
        self.constants.push(value);
        self.constants.len() - 1
    }

    /// 获取常量
    fn get(&self, index: usize) -> Option<&ConstValue> {
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
    fn add_entry(&mut self, key: usize, target: usize) {
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
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
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
        let type_table: Vec<MonoType> = self.module.types.iter().map(|t| self.type_from_ast(t)).collect();

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
    fn generate_instructions(&self, func: &FunctionIR) -> Result<Vec<BytecodeInstruction>, CodegenError> {
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
    fn translate_instruction(&self, _instr: &Instruction) -> Result<BytecodeInstruction, CodegenError> {
        // 简化实现：返回空指令
        Ok(BytecodeInstruction::new(TypedOpcode::Nop, vec![]))
    }

    /// 将操作数转换为寄存器编号
    fn operand_to_reg(&self, operand: &Operand) -> Result<u8, CodegenError> {
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
            entry_point: self.find_entry_point(),
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
    fn type_from_ast(&self, ast_type: &Type) -> MonoType {
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
            Type::Tuple(types) => MonoType::Tuple(
                types.iter().map(|t| self.type_from_ast(t)).collect(),
            ),
            Type::Fn { params, return_type } => MonoType::Fn {
                params: params.iter().map(|t| self.type_from_ast(t)).collect(),
                return_type: Box::new(self.type_from_ast(return_type)),
                is_async: false,
            },
            _ => MonoType::Void,
        }
    }

    /// 添加常量
    fn add_constant(&mut self, value: ConstValue) -> usize {
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
    fn emit(&mut self, instr: BytecodeInstruction) {
        self.bytecode.extend(instr.encode());
    }
}

/// 字节码文件
#[derive(Debug, Clone)]
pub struct BytecodeFile {
    /// 文件头
    pub header: BytecodeHeader,

    /// 类型表
    pub type_table: Vec<MonoType>,

    /// 常量池
    pub const_pool: Vec<ConstValue>,

    /// 代码段
    pub code_section: CodeSection,
}

/// 文件头
#[derive(Debug, Clone)]
pub struct BytecodeHeader {
    /// 魔术数
    pub magic: u32,

    /// 版本
    pub version: u32,

    /// 标志
    pub flags: u32,

    /// 入口点
    pub entry_point: usize,

    /// 节数量
    pub section_count: u16,

    /// 文件大小
    pub file_size: u32,

    /// 校验和
    pub checksum: u32,
}

/// 代码段
#[derive(Debug, Clone)]
pub struct CodeSection {
    /// 函数列表
    pub functions: Vec<FunctionCode>,
}

/// 函数代码
#[derive(Debug, Clone)]
pub struct FunctionCode {
    /// 函数名
    pub name: String,

    /// 参数类型
    pub params: Vec<MonoType>,

    /// 返回类型
    pub return_type: MonoType,

    /// 指令列表
    pub instructions: Vec<BytecodeInstruction>,

    /// 局部变量数量
    pub local_count: usize,
}

/// 字节码指令
#[derive(Debug, Clone)]
pub struct BytecodeInstruction {
    /// 操作码
    pub opcode: TypedOpcode,

    /// 操作数
    pub operands: Vec<u8>,
}

impl BytecodeInstruction {
    /// 创建新指令
    pub fn new(opcode: TypedOpcode, operands: Vec<u8>) -> Self {
        BytecodeInstruction { opcode, operands }
    }

    /// 编码为字节序列
    pub fn encode(&self) -> Vec<u8> {
        let mut bytes = vec![self.opcode as u8];
        bytes.extend(&self.operands);
        bytes
    }
}

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

    fn insert(&mut self, name: String, symbol: Symbol) {
        self.symbols.insert(name, symbol);
    }

    fn get(&self, name: &str) -> Option<&Symbol> {
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
