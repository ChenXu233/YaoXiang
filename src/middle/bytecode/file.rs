//! 字节码序列化
//!
//! 定义 .yx (.42) 字节码文件格式并实现序列化。

use crate::frontend::core::sema::MonoType;
use crate::frontend::core::types::identity::Nominal;
use crate::middle::bytecode::GlobalInfo;
use crate::middle::ir::ir::ConstValue;
use crate::util::span::{DebugSpan, FileId, Position, SourceMap, Span};
use std::collections::HashMap;
use std::io::{self, Read, Seek, SeekFrom, Write};

/// 字节码文件头魔数 (YaoXiang ByteCode: YXBC)
/// 0x59584243 = 'Y' 'X' 'B' 'C' = YaoXiang ByteCode
/// 文件格式采用混合端序：魔数大端序（方便调试），其他数据小端序（性能优化）
const MAGIC: u32 = 0x59584243;
/// 版本号（字节码格式的单一版本源；read_from 与 generate_header 均引用此常量）
///
/// 版本史（每条只记「格式观测量」的变化，不记内部重构）：
/// - v4 及以前：函数记录无上值槽数 ⇒ 载入侧只能硬编 0，闭包的上值槽数丢失。
/// - v5：函数记录追加 `upvalue_count`（u32）；前端值 `SWITCH`(0x06) 与
///   `TAIL_CALL`(0x09) 从词表回收（RFC-039 D17/D32/D34），值交回分配池。
///   同一未发布窗口追加**模块级全局表**（`globals`：名字 + 类型号，槽位序号即表序），
///   使 `.42` 载入后 `BytecodeModule.globals` 不再恒为空（RFC-039 D52 的 2341 项）。
///   同一未发布窗口再改**类型号表**：`Int` 五档与 `Float` 两档各占独立号
///   （此前 `Int(64)` 与 `Float(32)` 同为 `9`、`Float(64)` 读回 `TypeRef("_")`），
///   编号与解码共用一张表、可逆往返；写侧对未知位宽与编译期 FFI 类型
///   （`LibraryRef` / `ExternRef`）响亮拒绝而不是静默折算（依据见 WBS 09 的 7e.4 行
///   与本节 v5 条目；同源叙述在 [04-ssa.md](../../../docs/src/dev/architecture/04-ssa.md)
///   的「硬编码丢弃的跟踪」）。
///   同一未发布窗口回收 `TRY_BEGIN`(0xA0) / `TRY_END`(0xA1)（P10 S6）：两者**从无
///   生产者**（codegen 不产出、语言层无 try 语义）⇒ 旧 `.42` 不可能含这两个字节；
///   读取侧对未知 opcode 已是 panic（#271）而不是静默兜底 ⇒ 回收**不需要再升版**，
///   `VERSION` 保持 5。该回收不改变任何格式观测量（字节布局、段数、记录形状都没动）。
pub const VERSION: u32 = 5;

const FLAG_DEBUG_INFO: u32 = 0x02;

/// 无调试段时 `.42` 写出的段数：类型表 / 常量池 / 代码段 / vtable 表 / 全局表。
/// 段数只在 `write_to` 由本常量 + 可选调试段推导一次。
const BASE_SECTION_COUNT: u16 = 5;

const DEBUG_SECTION_MAGIC: u32 = 0x59584442; // 'Y' 'X' 'D' 'B'
/// 调试段格式版本。
/// v1: 源文件表 + 每函数 ip→span 映射
/// v2: 追加每函数局部变量名表（与 v1 布局向后兼容读取，见 decode）
const DEBUG_SECTION_VERSION: u32 = 3;

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

    /// 编译期类型方法表（type_name → [(裸方法名, 函数表索引)]）。
    /// codegen 从函数表按类型分组生成，解释器加载期直建 vtable 缓存，
    /// 分发全程按函数表索引，不按名解析。
    pub vtables: Vec<(String, Vec<(String, u32)>)>,

    /// 模块级全局表（v5）：表序即槽位号，`type_id` 是 .42 类型号。
    ///
    /// 语义见 [`GlobalInfo`]：`BytecodeModule.globals` 此前在载入侧被硬编空表
    /// （RFC-039 D52 的 `2341` 项），顶层绑定只剩调试段里的名字，没有「有哪些全局、
    /// 各是什么类型」这张表。
    pub globals: Vec<GlobalInfo>,

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
    /// 段数。口径：写出时按**实际写出的段数**填写——类型表 / 常量池 / 代码段 /
    /// vtable 表 / 全局表 = 5，带调试段 = 6，计算点只在 `write_to`；
    /// 读侧只搬运、不校验。构造侧填 0 表示未填写。
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
            // 未填写：写出时由 write_to 按实际段数计算（见 section_count 的注释）
            section_count: 0,
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
    /// 每函数的局部变量名表，与 `function_debug_maps` 平行（同一函数序号）
    pub function_local_names: Vec<HashMap<usize, String>>,
    /// 全局槽位名表（槽位号 → 顶层绑定名）。
    ///
    /// #368：顶层绑定走 `Operand::Global`，不在任何函数的局部名表里；
    /// 没有这张表，顶层 `a[i]` 的越界诊断只能报数值、报不出 `i`。
    /// v3 起才有；v1/v2 产物补空表。
    pub global_names: HashMap<usize, String>,
}

impl DebugSection {
    pub fn from_sources_and_functions(
        sources: SourceMap,
        functions: &[FunctionCode],
    ) -> Self {
        Self::with_global_names(sources, functions, HashMap::new())
    }

    /// 带全局槽位名表的构造（#368）。
    pub fn with_global_names(
        sources: SourceMap,
        functions: &[FunctionCode],
        global_names: HashMap<usize, String>,
    ) -> Self {
        let function_debug_maps = functions.iter().map(|f| f.debug_map.clone()).collect();
        let function_local_names = functions.iter().map(|f| f.local_names.clone()).collect();
        Self {
            sources,
            function_debug_maps,
            function_local_names,
            global_names,
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

        // v2：每函数局部变量名表（按槽位下标排序保证字节确定）
        out.write_all(&(self.function_local_names.len() as u32).to_le_bytes())?;
        for names in &self.function_local_names {
            let mut entries: Vec<(usize, &String)> = names.iter().map(|(k, v)| (*k, v)).collect();
            entries.sort_by_key(|(idx, _)| *idx);
            out.write_all(&(entries.len() as u32).to_le_bytes())?;
            for (idx, name) in entries {
                out.write_all(&(idx as u32).to_le_bytes())?;
                write_string(&mut out, name)?;
            }
        }

        // v3：全局槽位名表（排序保证字节确定）
        let mut globals: Vec<(usize, &String)> =
            self.global_names.iter().map(|(k, v)| (*k, v)).collect();
        globals.sort_by_key(|(idx, _)| *idx);
        out.write_all(&(globals.len() as u32).to_le_bytes())?;
        for (idx, name) in globals {
            out.write_all(&(idx as u32).to_le_bytes())?;
            write_string(&mut out, name)?;
        }

        Ok(out)
    }

    fn decode(bytes: &[u8]) -> io::Result<Self> {
        let mut cursor = io::Cursor::new(bytes);

        let version = read_u32(&mut cursor)?;
        // 接受 v1/v2/v3：v1 无局部名表、v2 无全局名表，读完补空表，读取路径不得 panic。
        if !(1..=DEBUG_SECTION_VERSION).contains(&version) {
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

        // v2 起才有局部名表；v1 产物在此补空表（长度与函数数一致）
        let function_local_names = if version >= 2 {
            let name_count = read_u32(&mut cursor)? as usize;
            let mut all = Vec::with_capacity(name_count);
            for _ in 0..name_count {
                let entry_count = read_u32(&mut cursor)? as usize;
                let mut map = HashMap::with_capacity(entry_count);
                for _ in 0..entry_count {
                    let idx = read_u32(&mut cursor)? as usize;
                    let name = read_string(&mut cursor)?;
                    map.insert(idx, name);
                }
                all.push(map);
            }
            all
        } else {
            vec![HashMap::new(); func_count]
        };

        // v3 起才有全局槽位名表；v1/v2 产物补空表（#368）
        let global_names = if version >= 3 {
            let count = read_u32(&mut cursor)? as usize;
            let mut map = HashMap::with_capacity(count);
            for _ in 0..count {
                let idx = read_u32(&mut cursor)? as usize;
                let name = read_string(&mut cursor)?;
                map.insert(idx, name);
            }
            map
        } else {
            HashMap::new()
        };

        Ok(Self {
            sources,
            function_debug_maps,
            function_local_names,
            global_names,
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
    /// 上值槽数（闭包环境槽数，RFC-039 D17）。
    ///
    /// 取值：本函数体内经 `LoadUpvalue` / `StoreUpvalue` 触达的上值槽数
    /// （最大槽下标 + 1），由 codegen 从指令流统计。
    /// 消费面：本字段只保产物事实（编解码与 `.42` 往返）；解释器闭包帧不读它——
    /// 上值经 `CallDyn` / `Spawn` 的 env 注入（`Frame::set_upvalue`），帧布局与此无关。
    /// 若日后要按它给闭包帧预分配上值槽，属行为变更：先立任务，先红后绿。
    /// v5 起随函数记录落盘；v4 及以前的产物没有该字段（且载入侧曾硬编 0，
    /// 属数据丢失缺陷），故读取侧按版本整体拒绝旧文件而不是补 0。
    pub upvalue_count: usize,
    /// 局部变量名（槽位下标 → 源码名）。
    /// 只含具名变量；编译器临时寄存器不出现在此表。
    pub local_names: HashMap<usize, String>,
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
        opcode: u8,
        operands: Vec<u8>,
    ) -> Self {
        Self { opcode, operands }
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
        }
        // 段数的唯一计算点：本函数实际写出的段 = 类型表 / 常量池 / 代码段 /
        // vtable 表 / 全局表（BASE_SECTION_COUNT），外加可选调试段。
        // 构造侧不手填——名称与值不符会误导 dump 读者。
        header.section_count = BASE_SECTION_COUNT + u16::from(has_debug_section);

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
            writer.write_all(&ty.to_type_id()?.to_le_bytes())?;
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
                // #492 原生函数作值：名字按字符串段写入（运行时按名分发）
                ConstValue::NativeFn(name) => {
                    writer.write_all(&[7])?;
                    writer.write_all(&(name.len() as u32).to_le_bytes())?;
                    writer.write_all(name.as_bytes())?;
                }
                // 可达性：`middle/lower/const_eval.rs` 会在编译期把
                // `Native.c("lib")` / `lib("sym")` 求值成这两个变体；只要其中一个
                // 以值的形式进 `Instruction::Load`，它就会经 `emitter.add_constant`
                // 进常量池并到这里。此前这里是 `todo!()`——写 .42 直接 panic，
                // 用户看到的是「thread panicked」而不是「这个东西不能进 .42」。
                // 改为响亮诊断：既指出是哪一类常量，也说明它只在编译期存在。
                ConstValue::LibraryRef { mechanism, lib } => {
                    return Err(io::Error::new(
                        io::ErrorKind::InvalidInput,
                        format!(
                            ".42 常量池无法表示 LibraryRef(mechanism={mechanism}, lib={lib})：\
                             它是编译期 FFI 描述值，运行期没有数据表示；\
                             前端若把它当值落进常量池，应在 codegen 前消解（FfiBinding）"
                        ),
                    ));
                }
                ConstValue::ExternRef {
                    mechanism,
                    lib,
                    symbol,
                } => {
                    return Err(io::Error::new(
                        io::ErrorKind::InvalidInput,
                        format!(
                            ".42 常量池无法表示 ExternRef(mechanism={mechanism}, lib={lib}, \
                             symbol={symbol})：它是编译期 FFI 绑定描述值，运行期没有数据表示；\
                             前端若把它当值落进常量池，应在 codegen 前消解（FfiBinding）"
                        ),
                    ));
                }
            }
        }

        // 代码段 (小端序，性能优化)
        writer.write_all(&(self.code_section.functions.len() as u32).to_le_bytes())?;
        for func in &self.code_section.functions {
            writer.write_all(&(func.name.len() as u32).to_le_bytes())?;
            writer.write_all(func.name.as_bytes())?;
            writer.write_all(&(func.params.len() as u32).to_le_bytes())?;
            for param in &func.params {
                writer.write_all(&param.to_type_id()?.to_le_bytes())?;
            }
            writer.write_all(&func.return_type.to_type_id()?.to_le_bytes())?;
            writer.write_all(&(func.local_count as u32).to_le_bytes())?;
            // v5：上值槽数随函数记录落盘（D17）。写在两个计数之间，
            // 与 read_from 的读取顺序对称。
            writer.write_all(&(func.upvalue_count as u32).to_le_bytes())?;
            writer.write_all(&(func.instructions.len() as u32).to_le_bytes())?;
            for instr in &func.instructions {
                writer.write_all(&[instr.opcode])?;
                writer.write_all(&(instr.operands.len() as u16).to_le_bytes())?;
                writer.write_all(&instr.operands)?;
            }
        }

        writer.write_all(&[0u8; 4])?; // 跳转表

        // vtables 段：编译期类型方法表（type_name → [(裸方法名, 函数表索引)]）
        writer.write_all(&(self.vtables.len() as u32).to_le_bytes())?;
        for (type_name, methods) in &self.vtables {
            writer.write_all(&(type_name.len() as u32).to_le_bytes())?;
            writer.write_all(type_name.as_bytes())?;
            writer.write_all(&(methods.len() as u32).to_le_bytes())?;
            for (bare_name, func_idx) in methods {
                writer.write_all(&(bare_name.len() as u32).to_le_bytes())?;
                writer.write_all(bare_name.as_bytes())?;
                writer.write_all(&func_idx.to_le_bytes())?;
            }
        }

        // 全局表（v5）：表序即槽位号。只落「名字 + 类型号」两项既有事实——
        // 见 `GlobalInfo` 的字段说明：initializer / is_mutable 在 IR 侧没有载体，
        // 写侧遇到非默认值即拒绝，绝不静默丢成 None/false。
        writer.write_all(&(self.globals.len() as u32).to_le_bytes())?;
        for global in &self.globals {
            if global.initializer.is_some() {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidInput,
                    format!(
                        ".42 全局表不携带 initializer（全局 '{}'）：初始化由模块初始化序列在运行时求值，编译期常量折叠只是优化；若确需落盘，先扩格式与载入侧",
                        global.name
                    ),
                ));
            }
            if global.is_mutable {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidInput,
                    format!(
                        ".42 全局表不携带可变性（全局 '{}'）：该事实在 IR `GlobalSlot` 里没有字段（lower 阶段即被丢弃），写侧拒绝而不是静默读成不可变",
                        global.name
                    ),
                ));
            }
            write_string(writer, &global.name)?;
            // 类型号由唯一编码口径产出（与形参/返回/类型表项同一套编号）
            writer.write_all(&mono_type_id(&global.ty)?.to_le_bytes())?;
        }

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
            return Err(unsupported_version_error(version));
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
                // #492 原生函数作值
                7 => {
                    let s = read_string(reader)?;
                    ConstValue::NativeFn(s)
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
            // v5：上值槽数（D17）——此前格式不落该字段，载入侧硬编 0
            let upvalue_count = read_u32(reader)? as usize;
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
                upvalue_count,
                // 名字来自调试段（v2）；由上层在读取调试段后回填
                local_names: HashMap::new(),
                debug_map: HashMap::new(),
            });
        }

        // 跳转表（4 字节填充）
        let mut jump_table = [0u8; 4];
        reader.read_exact(&mut jump_table)?;

        // vtables 段：编译期类型方法表（type_name → [(裸方法名, 函数表索引)]）
        let vtable_count = read_u32(reader)? as usize;
        let mut vtables = Vec::with_capacity(vtable_count);
        for _ in 0..vtable_count {
            let type_name = read_string(reader)?;
            let method_count = read_u32(reader)? as usize;
            let mut methods = Vec::with_capacity(method_count);
            for _ in 0..method_count {
                let bare_name = read_string(reader)?;
                let func_idx = read_u32(reader)?;
                methods.push((bare_name, func_idx));
            }
            vtables.push((type_name, methods));
        }

        // 全局表（v5）：表序即槽位号；类型号按 .42 的类型号原样搬运。
        let global_count = read_u32(reader)? as usize;
        let mut globals = Vec::with_capacity(global_count);
        for _ in 0..global_count {
            let name = read_string(reader)?;
            let type_code = read_u32(reader)?;
            globals.push(GlobalInfo {
                name,
                // 读侧解回 MonoType：与函数签名同载体，消费端不必认识「类型号」
                ty: type_id_to_monotype(type_code),
                // 格式不携带这两项：初始化在模块初始化序列里运行时求值；
                // 可变性事实在 IR GlobalSlot 没有字段（见 GlobalInfo 的文档注释）。
                initializer: None,
                is_mutable: false,
            });
        }

        // 可选的调试段（从文件尾向后读取）
        let debug_section = DebugSection::read_from_end(reader)?;

        // 代码段本身不序列化 local_names（名字只在调试段里）；
        // 读回后按函数序号平行回填，否则 dump/调试器看不到变量名。
        if let Some(d) = &debug_section {
            for (idx, func) in functions.iter_mut().enumerate() {
                if let Some(names) = d.function_local_names.get(idx) {
                    func.local_names = names.clone();
                }
            }
        }

        Ok(Self {
            header,
            type_table,
            const_pool,
            code_section: CodeSection { functions },
            vtables,
            globals,
            debug_section,
        })
    }

    /// 从文件路径加载字节码文件
    pub fn load<P: AsRef<std::path::Path>>(path: P) -> io::Result<Self> {
        let file = std::fs::File::open(path.as_ref())?;
        let mut reader = std::io::BufReader::new(file);
        Self::read_from(&mut reader)
    }

    /// 检测文件是否包含 YXBC 字节码魔数
    ///
    /// 只读取文件前 4 字节，不打开完整文件。
    /// 文件不存在、无权限、或 < 4 字节均返回 `Ok(false)`。
    pub fn probe(path: &std::path::Path) -> io::Result<bool> {
        let mut file = std::fs::File::open(path)?;
        let mut magic = [0u8; 4];
        let n = file.read(&mut magic)?;
        Ok(n >= 4 && u32::from_be_bytes(magic) == MAGIC)
    }
}

/// 将 .42 类型号转换回 [`MonoType`]（与编码共用 [`SCALAR_TYPE_CODES`] 一张表）。
///
/// **可逆段**（0..=12：Void/Bool/Int 五档/Float 两档/Char/String/Bytes）精确重建；
/// **折叠类**（20..=61：Struct/Enum/Option/Result/Fn/DepFn/TypeRef/Union/…）仍是单号，
/// 读回既定 lossy 形态 [`lossy_type_ref`]——这条降级契约与本批前一致，没有收紧也没有放宽。
/// 未知号同样落 lossy 形态（防御：正常产物不会出现未登记号，版本校验在更外层）。
pub(crate) fn type_id_to_monotype(id: u32) -> MonoType {
    // 与编码共用同一张表：能编出来的标量一定能原样读回
    if let Some(row) = SCALAR_TYPE_CODES.iter().find(|row| row.code == id) {
        return (row.build)();
    }
    // 折叠类与未知号：单号无参数，读回既定 lossy 形态（既有的降级契约，不变）
    lossy_type_ref()
}

/// 版本不匹配的诊断（RFC-039 D17：升版后旧 `.42` 按既有行为拒绝）。
///
/// 两类要素都要给全，调用方才能既分类又定位：
/// - **码**：`io::ErrorKind::Unsupported`——「这个文件本读取器不支持」，
///   与同函数里魔数错误的 `InvalidData`（「文件本身坏了」）分得开；
/// - **消息**：文件版本、当前版本、差在哪个方向、以及可执行的动作
///   （`.42` 是构建产物，重新构建而不是复用旧文件）。
///
/// 不新增诊断码：RFC-013 码表归 `src/util/diagnostic/codes/`，扩容是独立裁决面；
/// 本函数只用 `io::Error` 的既有分类，不另立一套平行码。
fn unsupported_version_error(version: u32) -> io::Error {
    let direction = if version < VERSION {
        "older than"
    } else {
        "newer than"
    };
    io::Error::new(
        io::ErrorKind::Unsupported,
        format!(
            "unsupported bytecode version {version}: the file is {direction} the current .42 \
             format version {VERSION}. .42 files are build artifacts, not source — rebuild the \
             bytecode from source instead of reusing this file"
        ),
    )
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

/// 可逆类型号表的一行：**编码与解码共用同一份口诀**。
///
/// 一行为一个「能原样读回」的类型：`matches` 判编码，`build` 造解码结果。
/// 两向同表是刻意的——分成两张表就会漂移，而漂移只会表现为「版本没变、类型变了」。
pub(crate) struct ScalarTypeCode {
    pub(crate) code: u32,
    pub(crate) name: &'static str,
    matches: fn(&MonoType) -> bool,
    build: fn() -> MonoType,
}

/// .42 类型号表（可逆段）。
///
/// 号段约定：
/// - `0..=12`：基本标量，可逆往返（Char/String/Bytes 保持历史号 10/11/12，缩小改动面）；
/// - `20..=50`：折叠类（Struct/Enum/Fn/…）**单号、无参数**，读回既定 lossy 形态
///   [`lossy_type_ref`]；`TypeId::STRUCT = 20` / `TypeId::ENUM = 21` 与运行时身份常量对齐，
///   `Option`/`Result` 归入 21（运行时 sum type 身份即 21）；
/// - `60` 以后：无运行时表示的元类型/底类型。
///
/// 不在本表、也不是折叠类的类型（未知位宽、编译期 FFI 类型）在写侧**拒绝**。
pub(crate) const SCALAR_TYPE_CODES: &[ScalarTypeCode] = &[
    ScalarTypeCode {
        code: 0,
        name: "Void",
        matches: |t| matches!(t, MonoType::Void),
        build: || MonoType::Void,
    },
    ScalarTypeCode {
        code: 1,
        name: "Bool",
        matches: |t| matches!(t, MonoType::Bool),
        build: || MonoType::Bool,
    },
    ScalarTypeCode {
        code: 2,
        name: "Int(8)",
        matches: |t| matches!(t, MonoType::Int(8)),
        build: || MonoType::Int(8),
    },
    ScalarTypeCode {
        code: 3,
        name: "Int(16)",
        matches: |t| matches!(t, MonoType::Int(16)),
        build: || MonoType::Int(16),
    },
    ScalarTypeCode {
        code: 4,
        name: "Int(32)",
        matches: |t| matches!(t, MonoType::Int(32)),
        build: || MonoType::Int(32),
    },
    ScalarTypeCode {
        code: 5,
        name: "Int(64)",
        matches: |t| matches!(t, MonoType::Int(64)),
        build: || MonoType::Int(64),
    },
    ScalarTypeCode {
        code: 6,
        name: "Int(128)",
        matches: |t| matches!(t, MonoType::Int(128)),
        build: || MonoType::Int(128),
    },
    ScalarTypeCode {
        code: 7,
        name: "Float(32)",
        matches: |t| matches!(t, MonoType::Float(32)),
        build: || MonoType::Float(32),
    },
    ScalarTypeCode {
        code: 8,
        name: "Float(64)",
        matches: |t| matches!(t, MonoType::Float(64)),
        build: || MonoType::Float(64),
    },
    ScalarTypeCode {
        code: 10,
        name: "Char",
        matches: |t| matches!(t, MonoType::Char),
        build: || MonoType::Char,
    },
    ScalarTypeCode {
        code: 11,
        name: "String",
        // 只认内建（平路径、无实参）的 String：module/args 任一非空都属另一个类型，
        // 编成 11 会把它读回成内建 String——静默丢身份（D64 后 name 携带模块）。
        matches: |t| {
            matches!(t, MonoType::Generic { name, args }
            if args.is_empty() && name.module().is_empty() && name.bare() == "String")
        },
        build: MonoType::make_string,
    },
    ScalarTypeCode {
        code: 12,
        name: "Bytes",
        matches: |t| {
            matches!(t, MonoType::Generic { name, args }
            if args.is_empty() && name.module().is_empty() && name.bare() == "Bytes")
        },
        build: || MonoType::Generic {
            name: Nominal::flat("Bytes").expect("内建类型名必须是裸名且非空"),
            args: Vec::new(),
        },
    },
];

/// 折叠类的既定 lossy 读回形态（单号无参数，解不出原结构）。
fn lossy_type_ref() -> MonoType {
    MonoType::TypeRef(Nominal::flat("_").expect("lossy 占位名必须是裸名且非空"))
}

/// 表里查编码号；命中 `Some`，未登记 `None`（由调用方决定折叠还是拒绝）。
fn scalar_code_of(ty: &MonoType) -> Option<u32> {
    SCALAR_TYPE_CODES
        .iter()
        .find(|row| (row.matches)(ty))
        .map(|row| row.code)
}

/// 未登记位宽：拒绝而不就近折算（Int(24) 折算成 Int(32) 就是静默错配）。
fn unsupported_width(
    kind: &str,
    width: usize,
) -> io::Error {
    io::Error::new(
        io::ErrorKind::InvalidInput,
        format!(
            ".42 类型表不支持 {kind}({width}) 位宽：可逆号表只登记 Int(8/16/32/64/128) 与 Float(32/64)，\
             其余一律拒绝而不是就近折算"
        ),
    )
}

/// 把 [`MonoType`] 编成 .42 类型号（`.42` 内所有类型字段的唯一编码口径）。
///
/// 生产者与容器写侧共用本函数；表 [`SCALAR_TYPE_CODES`] 同时供解码使用，
/// 因此不存在「编码口诀」与「解码口诀」两份而漂移的可能。
pub(crate) fn mono_type_id(ty: &MonoType) -> io::Result<u32> {
    ty.to_type_id()
}

/// 把 `MonoType` 映射成 .42 的类型号。
///
/// 返回 `Result` 而不是裸 `u32`：`LibraryRef` / `ExternRef` 是编译期 FFI 描述类型，
/// 运行期没有表示，写进类型表只能得到「读回来变成别的东西」的静默错配——
/// 那是 `todo!()` 时代的 panic 点换了个形态。写侧因此拒绝并给出可定位的诊断。
trait MonoTypeExt {
    fn to_type_id(&self) -> io::Result<u32>;
}

impl MonoTypeExt for MonoType {
    fn to_type_id(&self) -> io::Result<u32> {
        // 先查可逆表：命中即原样读回（这一半是「不失真」的契约）
        if let Some(code) = scalar_code_of(self) {
            return Ok(code);
        }
        let code = match self {
            // 未登记位宽：拒绝而不就近折算
            MonoType::Int(width) => return Err(unsupported_width("Int", *width)),
            MonoType::Float(width) => return Err(unsupported_width("Float", *width)),
            // ── 折叠类：单号、无参数，读回 lossy_type_ref() ──────────────
            // 20/21 与运行时常量 `TypeId::STRUCT` / `TypeId::ENUM` 对齐
            //（`src/backends/common/value.rs`）；Option/Result 归入 21——
            // 运行时的 sum type 身份就是 21，不是按名折叠的巧合。
            MonoType::Struct(_) => 20,
            MonoType::Enum(_) => 21,
            MonoType::Generic { name, .. } => match name.as_str() {
                "Option" | "Result" => 21,
                "Tuple" => 22,
                "List" => 23,
                "Dict" => 24,
                "Set" => 25,
                "Range" => 26,
                "Arc" => 45,
                "Weak" => 46,
                _ => 47, // 泛型实例化
            },
            MonoType::Fn { .. } => 30,
            MonoType::DepFn { .. } => 31,
            MonoType::TypeRef(_) => 40,
            MonoType::Union(_) => 41,
            MonoType::Intersection(_) => 42,
            MonoType::Literal { .. } => 48,
            MonoType::Ref { .. } => 49,
            MonoType::TypeVar(_) => 50,
            MonoType::AssocType { .. } => 51,
            MonoType::MetaType { .. } => 60,
            MonoType::Never => 61,
            // 精化类型的运行时表示就是基类型：与 base 同号是**设计**（透明别名），
            // 因此它是「不同 MonoType 同号」的合法例外之一。
            MonoType::Refined { base, .. } => return base.to_type_id(),
            MonoType::LibraryRef { mechanism, lib } => {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidInput,
                    format!(
                        ".42 类型表无法表示 LibraryRef(mechanism={mechanism}, lib={lib})：\
                         它是编译期 FFI 描述类型，运行期没有表示；\
                         出现在签名/类型表里说明前端把编译期值当成了运行期类型"
                    ),
                ));
            }
            MonoType::ExternRef {
                mechanism,
                lib,
                symbol,
            } => {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidInput,
                    format!(
                        ".42 类型表无法表示 ExternRef(mechanism={mechanism}, lib={lib}, \
                         symbol={symbol})：它是编译期 FFI 绑定描述类型，运行期没有表示；\
                         出现在签名/类型表里说明前端把编译期值当成了运行期类型"
                    ),
                ));
            }
            // Void/Bool/Char/String/Bytes 已在可逆表里命中：走到这里说明表漏登记。
            // 响亮失败而不是给个近号——静默给号正是本批要修的那类缺陷。
            other => {
                let registered: Vec<&str> = SCALAR_TYPE_CODES.iter().map(|row| row.name).collect();
                return Err(io::Error::new(
                    io::ErrorKind::InvalidInput,
                    format!(
                        "未登记的可逆类型 {other:?}：必须先加进 SCALAR_TYPE_CODES 表（编码/解码共用同一份口诀）；当前可逆段：{registered:?}"
                    ),
                ));
            }
        };
        Ok(code)
    }
}
