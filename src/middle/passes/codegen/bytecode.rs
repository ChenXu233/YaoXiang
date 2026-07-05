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
const VERSION: u32 = 3;

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
        bytes.extend(&(self.operands.len() as u16).to_le_bytes());
        bytes.extend(&self.operands);
        bytes
    }

    /// 获取编码后的大小
    pub fn encoded_size(&self) -> usize {
        3 + self.operands.len()
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
                ConstValue::LibraryRef { .. } | ConstValue::ExternRef { .. } => todo!(),
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
                writer.write_all(&(instr.operands.len() as u16).to_le_bytes())?;
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

impl BytecodeFile {
    ///
    /// 格式与 `write_to` 对称。支持通过文件尾的 YXDB 魔数检测可选的调试段。
    pub fn read_from<R: Read + Seek>(reader: &mut R) -> io::Result<Self> {
        // 读取文件头
        let mut buf32 = [0u8; 4];
        reader.read_exact(&mut buf32)?;
        let magic = u32::from_be_bytes(buf32);
        if magic != MAGIC {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                format!("invalid magic: expected YXBC (0x{MAGIC:08X}), got 0x{magic:08X}"),
            ));
        }

        let version = read_u32(reader)?;
        if version != VERSION {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                format!("unsupported bytecode version {version}, expected {VERSION}"),
            ));
        }

        let flags = read_u32(reader)?;
        let entry_point = read_u32(reader)?;

        let mut buf16 = [0u8; 2];
        reader.read_exact(&mut buf16)?;
        let section_count = u16::from_le_bytes(buf16);

        let _file_size = read_u32(reader)?;
        let _checksum = read_u32(reader)?;

        let header = FileHeader {
            magic,
            version,
            flags,
            entry_point,
            section_count,
            file_size: _file_size,
            checksum: _checksum,
        };

        // 读取类型表
        let type_count = read_u32(reader)? as usize;
        let mut type_table = Vec::with_capacity(type_count);
        for _ in 0..type_count {
            let type_id = read_u32(reader)?;
            type_table.push(type_id_to_monotype(type_id));
        }

        // 读取常量池
        let const_count = read_u32(reader)? as usize;
        let mut const_pool = Vec::with_capacity(const_count);
        for _ in 0..const_count {
            let mut tag_buf = [0u8; 1];
            reader.read_exact(&mut tag_buf)?;
            let tag = tag_buf[0];

            let const_val = match tag {
                0 => ConstValue::Void,
                1 => {
                    let mut b = [0u8; 1];
                    reader.read_exact(&mut b)?;
                    ConstValue::Bool(b[0] != 0)
                }
                2 => {
                    let mut n = [0u8; 16];
                    reader.read_exact(&mut n)?;
                    ConstValue::Int(i128::from_le_bytes(n))
                }
                3 => {
                    let mut f = [0u8; 8];
                    reader.read_exact(&mut f)?;
                    ConstValue::Float(f64::from_le_bytes(f))
                }
                4 => {
                    let mut c = [0u8; 4];
                    reader.read_exact(&mut c)?;
                    let code = u32::from_le_bytes(c);
                    ConstValue::Char(char::from_u32(code).unwrap_or('\u{FFFD}'))
                }
                5 => {
                    let s = read_string(reader)?;
                    ConstValue::String(s)
                }
                6 => {
                    let len = read_u32(reader)? as usize;
                    let mut bytes = vec![0u8; len];
                    reader.read_exact(&mut bytes)?;
                    ConstValue::Bytes(bytes)
                }
                _ => {
                    return Err(io::Error::new(
                        io::ErrorKind::InvalidData,
                        format!("unknown const tag: {tag}"),
                    ));
                }
            };
            const_pool.push(const_val);
        }

        // 读取代码段
        let func_count = read_u32(reader)? as usize;
        let mut functions = Vec::with_capacity(func_count);
        for _ in 0..func_count {
            let name = read_string(reader)?;

            let param_count = read_u32(reader)? as usize;
            let mut params = Vec::with_capacity(param_count);
            for _ in 0..param_count {
                let type_id = read_u32(reader)?;
                params.push(type_id_to_monotype(type_id));
            }

            let return_type_id = read_u32(reader)?;
            let return_type = type_id_to_monotype(return_type_id);

            let local_count = read_u32(reader)? as usize;
            let instr_count = read_u32(reader)? as usize;

            let mut instructions = Vec::with_capacity(instr_count);
            for _ in 0..instr_count {
                let mut opcode_buf = [0u8; 1];
                reader.read_exact(&mut opcode_buf)?;
                let opcode = opcode_buf[0];

                let mut len_buf = [0u8; 2];
                reader.read_exact(&mut len_buf)?;
                let operand_len = u16::from_le_bytes(len_buf) as usize;

                let mut operands = vec![0u8; operand_len];
                if operand_len > 0 {
                    reader.read_exact(&mut operands)?;
                }

                instructions.push(BytecodeInstruction { opcode, operands });
            }

            functions.push(FunctionCode {
                name,
                params,
                return_type,
                instructions,
                local_count,
                debug_map: HashMap::new(),
            });
        }

        // 跳转表（4 字节填充）
        let mut jump_table = [0u8; 4];
        reader.read_exact(&mut jump_table)?;

        // 可选的调试段（从文件尾向后读取）
        let debug_section = DebugSection::read_from_end(reader)?;

        Ok(Self {
            header,
            type_table,
            const_pool,
            code_section: CodeSection { functions },
            debug_section,
        })
    }

    /// 从文件路径加载字节码文件
    pub fn load<P: AsRef<std::path::Path>>(path: P) -> io::Result<Self> {
        let file = std::fs::File::open(path.as_ref())?;
        let mut reader = std::io::BufReader::new(file);
        Self::read_from(&mut reader)
    }
}

/// 将 type_id (u32) 转换为相应的 MonoType。
///
/// 序列化是 lossy 的（复杂类型如 Struct/Enum 只存储一个 id），
/// 因此复杂类型会 fallback 到 `TypeRef("_")`。
/// 简单类型（Void/Bool/Int/Float/Char/String/Bytes）可以精确重建。
fn type_id_to_monotype(id: u32) -> MonoType {
    match id {
        0 => MonoType::Void,
        1 => MonoType::Bool,
        2..=5 => MonoType::Int(((id - 2) * 8 + 8) as usize),
        6..=9 => MonoType::Float(((id - 6) * 8 + 8) as usize),
        10 => MonoType::Char,
        11 => MonoType::String,
        12 => MonoType::Bytes,
        _ => MonoType::TypeRef("_".to_string()),
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
            MonoType::Refined { base, .. } => base.to_type_id(),
            MonoType::DepFn { .. } => 30, // 依赖函数类型，与普通函数同ID
            MonoType::LibraryRef { .. } | MonoType::ExternRef { .. } => todo!(),
        }
    }
}
