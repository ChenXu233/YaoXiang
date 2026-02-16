//! 字节码序列化
//!
//! 定义 .yx (.42) 字节码文件格式并实现序列化。

use crate::frontend::typecheck::MonoType;
use crate::middle::core::ir::ConstValue;
use crate::backends::common::Opcode;
use std::io::{self, Write};

/// 字节码文件头魔数 (YaoXiang ByteCode: YXBC)
/// 0x59584243 = 'Y' 'X' 'B' 'C' = YaoXiang ByteCode
/// 文件格式采用混合端序：魔数大端序（方便调试），其他数据小端序（性能优化）
const MAGIC: u32 = 0x59584243;
/// 版本号
const VERSION: u32 = 2;

/// 字节码文件结构
#[derive(Debug, Clone)]
pub struct BytecodeFile {
    /// 文件头
    pub header: FileHeader,
    /// 类型表
    pub type_table: Vec<MonoType>,
    /// 常量池
    pub const_pool: Vec<ConstValue>,
    /// 代码段
    pub code_section: CodeSection,
}

#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct FileHeader {
    pub magic: u32,
    pub version: u32,
    pub flags: u32,
    pub entry_point: u32,
    pub section_count: u16,
    pub file_size: u32,
    pub checksum: u32,
}

impl Default for FileHeader {
    fn default() -> Self {
        Self {
            magic: MAGIC,
            version: VERSION,
            flags: 0,
            entry_point: 0,
            section_count: 4,
            file_size: 0,
            checksum: 0,
        }
    }
}

#[derive(Debug, Clone)]
pub struct CodeSection {
    pub functions: Vec<FunctionCode>,
}

#[derive(Debug, Clone)]
pub struct FunctionCode {
    pub name: String,
    pub params: Vec<MonoType>,
    pub return_type: MonoType,
    pub instructions: Vec<BytecodeInstruction>,
    pub local_count: usize,
}

#[derive(Debug, Clone)]
pub struct BytecodeInstruction {
    pub opcode: u8,
    pub operands: Vec<u8>,
}

impl BytecodeInstruction {
    pub fn new(
        opcode: Opcode,
        operands: Vec<u8>,
    ) -> Self {
        Self {
            opcode: opcode as u8,
            operands,
        }
    }

    /// 编码为字节序列
    pub fn encode(&self) -> Vec<u8> {
        let mut bytes = vec![self.opcode];
        bytes.extend(&self.operands);
        bytes
    }

    /// 获取编码后的大小
    pub fn encoded_size(&self) -> usize {
        1 + self.operands.len()
    }
}

/// 将 FunctionCode 编码为字节序列
impl FunctionCode {
    /// 编码所有指令为字节码
    pub fn encode_all(&self) -> Vec<u8> {
        self.instructions
            .iter()
            .flat_map(|instr| instr.encode())
            .collect()
    }
}

impl BytecodeFile {
    /// 序列化到 Writer
    /// 格式设计：魔数大端序（方便调试），其他数据小端序（x86 性能优化）
    pub fn write_to<W: Write>(
        &self,
        writer: &mut W,
    ) -> io::Result<()> {
        // 文件头：魔数大端序，其他小端序
        writer.write_all(&self.header.magic.to_be_bytes())?; // YXBC 方便调试
        writer.write_all(&self.header.version.to_le_bytes())?;
        writer.write_all(&self.header.flags.to_le_bytes())?;
        writer.write_all(&self.header.entry_point.to_le_bytes())?;
        writer.write_all(&self.header.section_count.to_le_bytes())?;
        writer.write_all(&self.header.file_size.to_le_bytes())?;
        writer.write_all(&self.header.checksum.to_le_bytes())?;

        // 类型表 (小端序，性能优化)
        writer.write_all(&(self.type_table.len() as u32).to_le_bytes())?;
        for ty in &self.type_table {
            writer.write_all(&ty.to_type_id().to_le_bytes())?;
        }

        // 常量池 (小端序，性能优化)
        writer.write_all(&(self.const_pool.len() as u32).to_le_bytes())?;
        for const_val in &self.const_pool {
            match const_val {
                ConstValue::Void => writer.write_all(&[0])?,
                ConstValue::Bool(b) => writer.write_all(&[1, if *b { 1 } else { 0 }])?,
                ConstValue::Int(n) => {
                    writer.write_all(&[2])?;
                    writer.write_all(&n.to_le_bytes())?;
                }
                ConstValue::Float(f) => {
                    writer.write_all(&[3])?;
                    writer.write_all(&f.to_le_bytes())?;
                }
                ConstValue::Char(c) => {
                    writer.write_all(&[4])?;
                    writer.write_all(&(*c as u32).to_le_bytes())?;
                }
                ConstValue::String(s) => {
                    writer.write_all(&[5])?;
                    writer.write_all(&(s.len() as u32).to_le_bytes())?;
                    writer.write_all(s.as_bytes())?;
                }
                ConstValue::Bytes(bytes) => {
                    writer.write_all(&[6])?;
                    writer.write_all(&(bytes.len() as u32).to_le_bytes())?;
                    writer.write_all(bytes)?;
                }
            }
        }

        // 代码段 (小端序，性能优化)
        writer.write_all(&(self.code_section.functions.len() as u32).to_le_bytes())?;
        for func in &self.code_section.functions {
            writer.write_all(&(func.name.len() as u32).to_le_bytes())?;
            writer.write_all(func.name.as_bytes())?;
            writer.write_all(&(func.params.len() as u32).to_le_bytes())?;
            writer.write_all(&func.return_type.to_type_id().to_le_bytes())?;
            writer.write_all(&(func.local_count as u32).to_le_bytes())?;
            writer.write_all(&(func.instructions.len() as u32).to_le_bytes())?;
            for instr in &func.instructions {
                writer.write_all(&[instr.opcode])?;
                writer.write_all(&instr.operands)?;
            }
        }

        writer.write_all(&[0u8; 4])?; // 跳转表
        Ok(())
    }
}

trait MonoTypeExt {
    fn to_type_id(&self) -> u32;
}

impl MonoTypeExt for MonoType {
    fn to_type_id(&self) -> u32 {
        match self {
            MonoType::Void => 0,
            MonoType::Bool => 1,
            MonoType::Int(n) => 2 + (*n as u32 / 8 - 1),
            MonoType::Float(n) => 6 + (*n as u32 / 8 - 1),
            MonoType::Char => 10,
            MonoType::String => 11,
            MonoType::Bytes => 12,
            MonoType::Struct(_) => 20,
            MonoType::Enum(_) => 21,
            MonoType::Tuple(_) => 22,
            MonoType::List(_) => 23,
            MonoType::Dict(_, _) => 24,
            MonoType::Set(_) => 25,
            MonoType::Fn { .. } => 30,
            MonoType::TypeRef(_) => 40,
            MonoType::TypeVar(_) => 50,
            MonoType::Range { .. } => 26,
            // 联合类型和交集类型暂时使用 TypeRef 的 ID
            MonoType::Union(_) => 40,
            MonoType::Intersection(_) => 40,
            MonoType::Arc(_) => 45,
            MonoType::Weak(_) => 46,
            MonoType::AssocType { .. } => 47, // 使用新的类型ID
            MonoType::Literal { .. } => 48,   // 字面量类型
            MonoType::MetaType { .. } => 0,   // 元类型无运行时表示
        }
    }
}
