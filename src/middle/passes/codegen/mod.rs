//! 代码生成器
//!
//! 将中间表示（IR）转换为类型化字节码。
//!
//! ## 模块职责
//!
//! - `mod.rs`: 编排层，组装各子模块
//! - `emitter.rs`: 字节码发射 + 跳转回填
//! - `translator.rs`: IR → 字节码翻译
//! - `operand.rs`: 操作数解析
//! - `buffer.rs`: 常量池 + 字节码缓冲区
//! - `bytecode.rs`: 字节码格式定义 + 序列化
//! - `flow.rs`: 寄存器分配 + 标签生成 + 符号表

pub mod buffer;
pub mod bytecode;
pub mod emitter;
pub mod flow;
pub mod operand;
pub mod translator;

use crate::frontend::core::parser::ast::Type;
use crate::frontend::typecheck::MonoType;
use crate::middle::core::ir::{ConstValue, ModuleIR, Operand};
use crate::middle::passes::codegen::translator::Translator;
use crate::middle::passes::codegen::flow::{FlowManager, SymbolScopeManager};
use crate::middle::passes::codegen::operand::OperandResolver;
use crate::util::i18n::{t, t_simple, MSG};
use crate::util::logger::get_lang;
use tracing::debug;

/// 代码生成上下文
///
/// 仅负责流程编排，具体职责委托给子模块：
/// - 翻译：Translator
/// - 发射：Emitter
/// - 流管理：FlowManager
pub struct CodegenContext {
    /// 当前模块
    module: ModuleIR,

    /// 字节码翻译器
    translator: Translator,

    /// 控制流管理
    flow: FlowManager,

    /// 符号表管理（用于测试）
    symbols: SymbolScopeManager,

    /// 配置
    config: CodegenConfig,
}

#[derive(Debug, Clone)]
struct CodegenConfig {
    enable_optimizations: bool,
    generate_debug_info: bool,
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
    UnimplementedExpr { expr_type: String },
    UnimplementedStmt { stmt_type: String },
    UnimplementedCall,
    InvalidAssignmentTarget,
    SymbolNotFound { name: String },
    TypeMismatch { expected: String, found: String },
    OutOfRegisters,
    InvalidOperand,
    RegisterOverflow { id: usize, limit: u8 },
    TranslationError(String),
}

impl std::fmt::Display for CodegenError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CodegenError::UnimplementedExpr { expr_type } => write!(f, "未实现的表达式类型: {}", expr_type),
            CodegenError::UnimplementedStmt { stmt_type } => write!(f, "未实现的语句类型: {}", stmt_type),
            CodegenError::UnimplementedCall => write!(f, "未实现的函数调用"),
            CodegenError::InvalidAssignmentTarget => write!(f, "无效的赋值目标"),
            CodegenError::SymbolNotFound { name } => write!(f, "符号未找到: {}", name),
            CodegenError::TypeMismatch { expected, found } => write!(f, "类型不匹配: 期望 {}, 实际 {}", expected, found),
            CodegenError::OutOfRegisters => write!(f, "寄存器不足"),
            CodegenError::InvalidOperand => write!(f, "无效的操作数"),
            CodegenError::RegisterOverflow { id, limit } => write!(f, "寄存器编号 {} 超过最大限制 ({})", id, limit),
            CodegenError::TranslationError(msg) => write!(f, "翻译错误: {}", msg),
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
            translator: Translator::new(),
            flow: FlowManager::new(),
            symbols: SymbolScopeManager::new(),
            config: CodegenConfig::default(),
        };

        // 为所有函数建立索引
        for (idx, func) in ctx.module.functions.iter().enumerate() {
            ctx.flow.add_function_index(func.name.clone(), idx);
        }

        ctx
    }

    /// 生成下一个标签（委托给 FlowManager）
    pub fn next_label(&mut self) -> usize {
        self.flow.next_label()
    }

    /// 获取 FlowManager 引用（用于测试）
    pub fn test_flow(&mut self) -> &mut FlowManager {
        &mut self.flow
    }

    /// 获取下一个临时寄存器 ID（用于测试）
    pub fn test_next_temp(&mut self) -> usize {
        self.flow.alloc_temp()
    }

    /// 获取下一个局部变量 ID（用于测试）
    pub fn test_next_local(&mut self) -> usize {
        self.flow.alloc_local()
    }

    /// 添加常量（用于测试）
    pub fn test_add_constant(&mut self, value: ConstValue) -> usize {
        self.translator.test_add_constant(value)
    }

    /// 将操作数转换为寄存器（用于测试）
    pub fn test_operand_to_reg(&self, operand: &Operand) -> Result<u8, CodegenError> {
        let resolver = OperandResolver::new();
        resolver.to_reg(operand)
    }

    /// 获取符号表管理器（用于测试）
    pub fn test_symbols(&mut self) -> &mut SymbolScopeManager {
        &mut self.symbols
    }

    /// 生成字节码
    pub fn generate(&mut self) -> Result<BytecodeFile, CodegenError> {
        let lang = get_lang();
        let func_count = self.module.functions.len();
        debug!("{}", t(MSG::CodegenFunctions, lang, Some(&[&func_count])));

        // 1. 翻译所有函数
        debug!("{}", t(MSG::CodegenCodeSection, lang, Some(&[&func_count])));
        let output = self.translator.translate_module(&self.module)?;

        // 2. 生成常量池
        let const_pool = output.const_pool;
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
            code_section: output.code_section,
        })
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
            Type::Tuple(types) => {
                MonoType::Tuple(types.iter().map(|t| self.type_from_ast(t)).collect())
            }
            Type::Fn { params, return_type, .. } => MonoType::Fn {
                params: params.iter().map(|t| self.type_from_ast(t)).collect(),
                return_type: Box::new(self.type_from_ast(return_type)),
                is_async: false,
            },
            _ => MonoType::Void,
        }
    }
}

impl Default for CodegenContext {
    fn default() -> Self {
        CodegenContext {
            module: ModuleIR::default(),
            translator: Translator::new(),
            flow: FlowManager::new(),
            symbols: SymbolScopeManager::new(),
            config: CodegenConfig::default(),
        }
    }
}

// 重新导出公共类型
pub use bytecode::BytecodeFile;
pub use bytecode::BytecodeInstruction;
pub use bytecode::CodeSection;
pub use bytecode::FileHeader as BytecodeHeader;
pub use bytecode::FunctionCode;

/// 常量定义
pub const YAOXIANG_MAGIC: u32 = 0x59584243;
pub const BYTECODE_VERSION: u32 = 2;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_basic_codegen_context() {
        let module = ModuleIR::default();
        let ctx = CodegenContext::new(module);
        assert_eq!(ctx.module.functions.len(), 0);
    }
}
