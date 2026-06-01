//! 字节码序列化
//!
//! 定义 .yx (.42) 字节码文件格式并实现序列化。

use crate::frontend::core::typecheck::MonoType;
use crate::middle::core::ir::ConstValue;
use crate::util::span::{DebugSpan, FileId, Position, SourceMap, Span};
use crate::backends::common::Opcode;
use std::collections::HashMap;
use std::io::{self, Read, Seek, SeekFrom, Write};

/// 字节码文件头魔数 (YaoXiang ByteCode: YXBC)
/// 0x59584243 = 'Y' 'X' 'B' 'C' = YaoXiang ByteCode
/// 文件格式采用混合端序：魔数大端序（方便调试），其他数据小端序（性能优化）
const MAGIC: u32 = 0x59584243;
/// 版本号
const VERSION: u32 = 2;

const FLAG_DEBUG_INFO: u32 = 0x02;

const DEBUG_SECTION_MAGIC: u32 = 0x59584442; // 'Y' 'X' 'D' 'B'
const DEBUG_SECTION_VERSION: u32 = 1;

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

    /// 可选调试信息段（用于离线 .42 调试/定位）
    pub debug_section: Option<DebugSection>,
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

/// Debug section (sources + per-function ip mapping)
#[derive(Debug, Clone)]
pub struct DebugSection {
    pub sources: SourceMap,
    pub function_debug_maps: Vec<HashMap<usize, DebugSpan>>,
}

impl DebugSection {
    pub fn from_sources_and_functions(
        sources: SourceMap,
        functions: &[FunctionCode],
    ) -> Self {
        let function_debug_maps = functions.iter().map(|f| f.debug_map.clone()).collect();
        Self {
            sources,
            function_debug_maps,
        }
    }

    fn encode(&self) -> io::Result<Vec<u8>> {
        let mut out = Vec::new();

        out.write_all(&DEBUG_SECTION_VERSION.to_le_bytes())?;

        out.write_all(&(self.sources.files().len() as u32).to_le_bytes())?;
        for file in self.sources.files() {
            write_string(&mut out, &file.name)?;
            write_string(&mut out, &file.content)?;
        }

        out.write_all(&(self.function_debug_maps.len() as u32).to_le_bytes())?;
        for map in &self.function_debug_maps {
            let mut entries: Vec<(usize, DebugSpan)> = map.iter().map(|(k, v)| (*k, *v)).collect();
            entries.sort_by_key(|(ip, _)| *ip);

            out.write_all(&(entries.len() as u32).to_le_bytes())?;
            for (ip, ds) in entries {
                out.write_all(&(ip as u32).to_le_bytes())?;
                out.write_all(&ds.file_id.to_le_bytes())?;
                write_position(&mut out, ds.span.start)?;
                write_position(&mut out, ds.span.end)?;
            }
        }

        Ok(out)
    }

    fn decode(bytes: &[u8]) -> io::Result<Self> {
        let mut cursor = io::Cursor::new(bytes);

        let version = read_u32(&mut cursor)?;
        if version != DEBUG_SECTION_VERSION {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                format!("Unsupported debug section version: {version}"),
            ));
        }

        let file_count = read_u32(&mut cursor)? as usize;
        let mut sources = SourceMap::new();
        for _ in 0..file_count {
            let name = read_string(&mut cursor)?;
            let content = read_string(&mut cursor)?;
            sources.add_file(name, content);
        }

        let func_count = read_u32(&mut cursor)? as usize;
        let mut function_debug_maps = Vec::with_capacity(func_count);
        for _ in 0..func_count {
            let entry_count = read_u32(&mut cursor)? as usize;
            let mut map = HashMap::with_capacity(entry_count);
            for _ in 0..entry_count {
                let ip = read_u32(&mut cursor)? as usize;
                let file_id = read_u32(&mut cursor)? as FileId;
                let start = read_position(&mut cursor)?;
                let end = read_position(&mut cursor)?;
                map.insert(ip, DebugSpan::new(file_id, Span::new(start, end)));
            }
            function_debug_maps.push(map);
        }

        Ok(Self {
            sources,
            function_debug_maps,
        })
    }

    pub fn read_from_end<R: Read + Seek>(reader: &mut R) -> io::Result<Option<Self>> {
        let file_end = reader.seek(SeekFrom::End(0))?;
        if file_end < 8 {
            return Ok(None);
        }

        reader.seek(SeekFrom::End(-8))?;
        let mut footer = [0u8; 8];
        reader.read_exact(&mut footer)?;

        let magic = u32::from_be_bytes([footer[0], footer[1], footer[2], footer[3]]);
        if magic != DEBUG_SECTION_MAGIC {
            return Ok(None);
        }

        let payload_len = u32::from_le_bytes([footer[4], footer[5], footer[6], footer[7]]) as u64;
        if file_end < 8 + payload_len {
            return Err(io::Error::new(
                io::ErrorKind::UnexpectedEof,
                "Invalid debug section length",
            ));
        }

        let payload_start = file_end - 8 - payload_len;
        reader.seek(SeekFrom::Start(payload_start))?;
        let mut payload = vec![0u8; payload_len as usize];
        reader.read_exact(&mut payload)?;
        Ok(Some(Self::decode(&payload)?))
    }
}

#[derive(Debug, Clone)]
pub struct FunctionCode {
    pub name: String,
    pub params: Vec<MonoType>,
    pub return_type: MonoType,
    pub instructions: Vec<BytecodeInstruction>,
    pub local_count: usize,
    /// Debug info: mapping from IP to source Span
    pub debug_map: HashMap<usize, DebugSpan>,
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
        let mut header = self.header;
        let has_debug_section = self.debug_section.is_some();
        if has_debug_section {
            header.flags |= FLAG_DEBUG_INFO;
            header.section_count = 5;
        }

        // 文件头：魔数大端序，其他小端序
        writer.write_all(&header.magic.to_be_bytes())?; // YXBC 方便调试
        writer.write_all(&header.version.to_le_bytes())?;
        writer.write_all(&header.flags.to_le_bytes())?;
        writer.write_all(&header.entry_point.to_le_bytes())?;
        writer.write_all(&header.section_count.to_le_bytes())?;
        writer.write_all(&header.file_size.to_le_bytes())?;
        writer.write_all(&header.checksum.to_le_bytes())?;

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

        if (header.flags & FLAG_DEBUG_INFO) != 0 {
            let Some(debug) = &self.debug_section else {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidInput,
                    "DEBUG_INFO flag is set but debug_section is missing",
                ));
            };

            if debug.function_debug_maps.len() != self.code_section.functions.len() {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidInput,
                    "debug_section function count mismatch",
                ));
            }

            let payload = debug.encode()?;
            writer.write_all(&payload)?;
            writer.write_all(&DEBUG_SECTION_MAGIC.to_be_bytes())?;
            writer.write_all(&(payload.len() as u32).to_le_bytes())?;
        }

        Ok(())
    }
}

fn write_string<W: Write>(
    writer: &mut W,
    s: &str,
) -> io::Result<()> {
    let bytes = s.as_bytes();
    writer.write_all(&(bytes.len() as u32).to_le_bytes())?;
    writer.write_all(bytes)
}

fn read_string<R: Read>(reader: &mut R) -> io::Result<String> {
    let len = read_u32(reader)? as usize;
    let mut buf = vec![0u8; len];
    reader.read_exact(&mut buf)?;
    String::from_utf8(buf).map_err(|_| io::Error::new(io::ErrorKind::InvalidData, "Invalid utf-8"))
}

fn write_position<W: Write>(
    writer: &mut W,
    pos: Position,
) -> io::Result<()> {
    writer.write_all(&(pos.line as u32).to_le_bytes())?;
    writer.write_all(&(pos.column as u32).to_le_bytes())?;
    writer.write_all(&(pos.offset as u32).to_le_bytes())?;
    Ok(())
}

fn read_position<R: Read>(reader: &mut R) -> io::Result<Position> {
    let line = read_u32(reader)? as usize;
    let column = read_u32(reader)? as usize;
    let offset = read_u32(reader)? as usize;
    Ok(Position::with_offset(line, column, offset))
}

fn read_u32<R: Read>(reader: &mut R) -> io::Result<u32> {
    let mut buf = [0u8; 4];
    reader.read_exact(&mut buf)?;
    Ok(u32::from_le_bytes(buf))
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
            MonoType::Option(_) => 21,
            MonoType::Result(_, _) => 21,
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
            MonoType::Ref { .. } => 49,       // 借用引用类型
            MonoType::AssocType { .. } => 47, // 使用新的类型ID
            MonoType::Literal { .. } => 48,   // 字面量类型
            MonoType::MetaType { .. } => 0,   // 元类型无运行时表示
            MonoType::Generic { .. } => 47,   // 泛型实例化，使用结构体类型ID
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_debug_section_round_trip() {
        let mut sources = SourceMap::new();
        let file_id = sources.add_file("main.yx".to_string(), "main = () => { 1 / 0 }".to_string());

        let span = Span::new(
            Position::with_offset(1, 1, 0),
            Position::with_offset(1, 5, 4),
        );
        let debug_span = DebugSpan::new(file_id, span);

        let function = FunctionCode {
            name: "main".to_string(),
            params: Vec::new(),
            return_type: MonoType::Void,
            instructions: vec![BytecodeInstruction::new(Opcode::Nop, vec![])],
            local_count: 0,
            debug_map: HashMap::from([(0usize, debug_span)]),
        };

        let code_section = CodeSection {
            functions: vec![function],
        };

        let debug_section =
            DebugSection::from_sources_and_functions(sources.clone(), &code_section.functions);
        let file = BytecodeFile {
            header: FileHeader::default(),
            type_table: Vec::new(),
            const_pool: Vec::new(),
            code_section,
            debug_section: Some(debug_section),
        };

        let mut bytes = Vec::new();
        file.write_to(&mut bytes).expect("write bytecode");

        let mut cursor = io::Cursor::new(bytes);
        let decoded = DebugSection::read_from_end(&mut cursor)
            .expect("read debug section")
            .expect("debug section should exist");

        assert_eq!(decoded.sources.files().len(), 1);
        assert_eq!(decoded.sources.files()[0].name, "main.yx");
        assert_eq!(decoded.sources.files()[0].content, "main = () => { 1 / 0 }");
        assert_eq!(decoded.function_debug_maps.len(), 1);
        assert_eq!(
            decoded.function_debug_maps[0].get(&0).copied(),
            Some(debug_span)
        );
    }
}
