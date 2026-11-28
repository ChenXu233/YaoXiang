//! Bytecode opcode definitions for YaoXiang
//!
//! Opcode 是 .42 文件格式的 u8 编码标签。单一词表：
//! 编码（translator/emitter 写入 u8）与解码（mod.rs 的 `From<BytecodeFile>` 按 u8 匹配）
//! 共用同一组常量，
//! 不再有枚举 ↔ 字节的双词表漂移。
//!
//! 本文件同时持有 opcode 事实的**唯一权威表** [`OPCODE_INFO`]：码值 → 转储名 →
//! 指令固定长度。`opcode_name` / `opcode_size` 只读这张表，`BytecodeInstr::size`
//! 只从它取固定部分再加变长尾部。表与 `pub const` 声明的一致性由 build.rs 的
//! `code_tables::opcode_gate` 在**编译期**拒绝漂移（F8），编译后还有满覆盖对拍测试。

// Control Flow (0x00-0x0F)
pub const NOP: u8 = 0x00;
pub const RETURN: u8 = 0x01;
pub const RETURN_VALUE: u8 = 0x02;
pub const JMP: u8 = 0x03;
pub const JMP_IF: u8 = 0x04;
pub const JMP_IF_NOT: u8 = 0x05;
// 0x06 与 0x09 已回收（RFC-039 D17 随 .42 升版 v5 一并处理；D32/D34 指定对象）：
//   0x06 = `SWITCH`    —— 无生产者（translator 不产出、无 IR 构造点、无解码分支）
//   0x09 = `TAIL_CALL` —— IR `Instruction::TailCall` 全仓无构造点，编码分支同步删
// 「回收」的含义：这两个值不再属于词表，后续新指令可优先复用；旧 .42 里的这两个
// 字节不再被识别（v5 读取侧整体拒绝 v4 及更早的产物，见 `file::VERSION`）。
pub const YIELD: u8 = 0x0A;
pub const LABEL: u8 = 0x0B;
pub const SPAWN: u8 = 0x0E;
pub const SPAWN_FROM_LIST: u8 = 0x0F;

// Register Operations (0x10-0x1F)
pub const MOV: u8 = 0x10;
pub const LOAD_CONST: u8 = 0x11;
pub const LOAD_LOCAL: u8 = 0x12;
pub const STORE_LOCAL: u8 = 0x13;
pub const LOAD_ARG: u8 = 0x14;
pub const BORROW: u8 = 0x15;
pub const RELEASE: u8 = 0x16;
// 全局槽位（RFC-029f / 顶层绑定）：新 plan（top-level-binding）启用。
// 全局索引 u16（小端两字节），本地槽位为 u8，容量差异由 u16 承担。
pub const LOAD_GLOBAL: u8 = 0x17;
pub const STORE_GLOBAL: u8 = 0x18;

// Integer Operations (0x20-0x2F)
pub const I64_ADD: u8 = 0x20;
pub const I64_SUB: u8 = 0x21;
pub const I64_MUL: u8 = 0x22;
pub const I64_DIV: u8 = 0x23;
pub const I64_REM: u8 = 0x24;
pub const I64_AND: u8 = 0x25;
pub const I64_OR: u8 = 0x26;
pub const I64_XOR: u8 = 0x27;
pub const I64_SHL: u8 = 0x28;
pub const I64_SAR: u8 = 0x29;
pub const I64_SHR: u8 = 0x2A;
pub const I64_NEG: u8 = 0x2B;
/// 一元逻辑非（`!`，RFC-039 D33 / issue #562）：与 `I64_NEG`（取负 `-`）分码，
/// 编码/解码往返不再把 Not 改写为 Neg
pub const I64_NOT: u8 = 0x2C;

// Comparison Operations (0x60-0x65)
pub const I64_EQ: u8 = 0x60;
pub const I64_NE: u8 = 0x61;
pub const I64_LT: u8 = 0x62;
pub const I64_LE: u8 = 0x63;
pub const I64_GT: u8 = 0x64;
pub const I64_GE: u8 = 0x65;

// Memory & Object Operations (0x72-0x7F)
pub const HEAP_ALLOC: u8 = 0x72;
pub const STACK_ALLOC: u8 = 0x73;
pub const DROP: u8 = 0x74;
pub const GET_FIELD: u8 = 0x75;
pub const SET_FIELD: u8 = 0x76;
pub const LOAD_ELEMENT: u8 = 0x77;
pub const STORE_ELEMENT: u8 = 0x78;
pub const CREATE_STRUCT: u8 = 0x79;
pub const NEW_LIST_WITH_CAP: u8 = 0x7A;
pub const ARC_NEW: u8 = 0x7B;
pub const ARC_CLONE: u8 = 0x7C;
pub const ARC_DROP: u8 = 0x7D;
pub const WEAK_NEW: u8 = 0x7E;
pub const WEAK_UPGRADE: u8 = 0x7F;

// Function Call (0x80-0x8F)
pub const CALL_STATIC: u8 = 0x80;
pub const CALL_VIRT: u8 = 0x81;
pub const CALL_DYN: u8 = 0x82;
pub const MAKE_CLOSURE: u8 = 0x83;
pub const LOAD_UPVALUE: u8 = 0x84;
pub const STORE_UPVALUE: u8 = 0x85;
pub const CLOSE_UPVALUE: u8 = 0x86;
pub const CALL_NATIVE: u8 = 0x87;
pub const NEW_DICT: u8 = 0x88;
pub const RC_NEW: u8 = 0x89;
pub const NEW_TUPLE: u8 = 0x8A;
/// 定长数组构造：分配 N 个元素、以默认值填充（#299 §2）
pub const NEW_ARRAY: u8 = 0x8B;
/// membership 谓词（#299 §3）：dst = elem in container
pub const CONTAINS: u8 = 0x8C;
/// Range 值构造（#302）：dst + start + end + step 三标量记录
pub const NEW_RANGE: u8 = 0x8D;

// String Operations (0x90-0x9F)
pub const STRING_LENGTH: u8 = 0x90;
pub const STRING_CONCAT: u8 = 0x91;
pub const STRING_EQUAL: u8 = 0x92;
pub const STRING_GET_CHAR: u8 = 0x93;
pub const STRING_FROM_INT: u8 = 0x94;
pub const STRING_FROM_FLOAT: u8 = 0x95;

// Exception Handling (0xA0-0xAF)
// 0xA0 与 0xA1 已回收（P10 S6；RFC-039 D52 复核）：`TRY_BEGIN` / `TRY_END` 两个
// 字节码指令**从无生产者**——codegen 不产出、IR 侧没有 try 语义（语言层无
// throw/try/catch，RFC-036 否决进程内 catch 边界）。因此：
//   ① 无生产者 ⇒ 任何旧 `.42` 产物都不可能含这两个字节（不是丢失数据，是死路径）；
//   ② 读取侧对未知 opcode 已是 panic（#271 的响亮失败，见 `mod.rs` 的 From 解码臂），
//      旧产物即便人工拼出这两个字节也会立即报错而不是被静默当成别的指令；
//   ⇒ 回收不需要升版：`file::VERSION` 仍为 5。
// 前向保留只有 `THROW`（RFC-030 记为「字节码层已有、语言层未暴露」的
// assert-as-raise 预留），单列待裁决，不在本次回收范围。
pub const THROW: u8 = 0xA2;

// Debug Operations (0xB0-0xBF)
pub const BOUNDS_CHECK: u8 = 0xB0;

// Type Operations (0xC0-0xCF)
pub const TYPE_CHECK: u8 = 0xC0;
pub const CAST: u8 = 0xC1;

// Reflection (0xD0-0xDF)
pub const TYPE_OF: u8 = 0xD0;

// Variant Operations (0xE0-0xEF) — RFC-011a §6 存在类型变体
pub const CREATE_VARIANT: u8 = 0xE0;
pub const VARIANT_TAG: u8 = 0xE1;
pub const VARIANT_PAYLOAD: u8 = 0xE2;

/// 单个 opcode 的权威事实：码值、转储名、指令的固定字节数。
///
/// 「长度」在本表的定义（**唯一**表达点）：该 opcode 编码的**固定部分**——含
/// opcode 字节本身，不含随实例伸缩的尾部。变长尾部（实参表 / 字段表 / 任务描述表 /
/// 常量名字节）只由 `BytecodeInstr::size` 在表值之上追加，那里只剩「每元素几字节」，
/// 不再出现任何按码分列的长度常数。
///
/// 注意区分两个「长度」：本表记的是 `BytecodeInstr::size()` 这条查询的固定部分；
/// `.42` 落盘时每条指令的真实字节数由 `BytecodeInstruction::encoded_size`
/// （opcode 1 + 操作数长度前缀 2 + 操作数）决定，两者不是同一事实。
///
/// 表与 `pub const` 声明的一致性由两道判据守着：构建期 `code_tables::opcode_gate`
///（build.rs 硬门禁 F8）与 `middle/bytecode/tests` 的满覆盖对拍测试。
pub struct OpcodeInfo {
    /// `.42` 文件里的 u8 编码标签（写成同名 `pub const`，值不在表里重复一遍）
    pub code: u8,
    /// 调试/转储名，[`opcode_name`] 的唯一返回值来源
    pub name: &'static str,
    /// 指令的固定字节数（含 opcode 字节；变长尾部另计，见类型文档）
    pub size: usize,
}

/// opcode 权威表：按 `code` 严格升序，每个 `pub const` opcode 恰好一项。
///
/// 这张表是「码 → 转储名」「码 → 固定长度」的唯一表达：[`opcode_name`] 与
/// [`opcode_size`] 都只读它。新增 opcode 的义务是**三处同提交**：常量声明、
/// 本表一项、解码臂（build.rs 门禁会在前两处缺一时拒绝编译）。
pub static OPCODE_INFO: &[OpcodeInfo] = &[
    // ── 控制流 0x00-0x0F ──
    OpcodeInfo {
        code: NOP,
        name: "Nop",
        size: 1,
    },
    OpcodeInfo {
        code: RETURN,
        name: "Return",
        size: 1,
    },
    OpcodeInfo {
        code: RETURN_VALUE,
        name: "ReturnValue",
        size: 3,
    },
    OpcodeInfo {
        code: JMP,
        name: "Jmp",
        size: 5,
    },
    OpcodeInfo {
        code: JMP_IF,
        name: "JmpIf",
        size: 5,
    },
    OpcodeInfo {
        code: JMP_IF_NOT,
        name: "JmpIfNot",
        size: 5,
    },
    OpcodeInfo {
        code: YIELD,
        name: "Yield",
        size: 1,
    },
    OpcodeInfo {
        code: LABEL,
        name: "Label",
        size: 5,
    },
    OpcodeInfo {
        code: SPAWN,
        name: "Spawn",
        size: 5,
    },
    OpcodeInfo {
        code: SPAWN_FROM_LIST,
        name: "SpawnFromList",
        size: 5,
    },
    // ── 寄存器与槽位 0x10-0x1F ──
    OpcodeInfo {
        code: MOV,
        name: "Mov",
        size: 5,
    },
    OpcodeInfo {
        code: LOAD_CONST,
        name: "LoadConst",
        size: 5,
    },
    OpcodeInfo {
        code: LOAD_LOCAL,
        name: "LoadLocal",
        size: 4,
    },
    OpcodeInfo {
        code: STORE_LOCAL,
        name: "StoreLocal",
        size: 4,
    },
    OpcodeInfo {
        code: LOAD_ARG,
        name: "LoadArg",
        size: 4,
    },
    OpcodeInfo {
        code: BORROW,
        name: "Borrow",
        size: 6,
    },
    OpcodeInfo {
        code: RELEASE,
        name: "Release",
        size: 3,
    },
    OpcodeInfo {
        code: LOAD_GLOBAL,
        name: "LoadGlobal",
        size: 5,
    },
    OpcodeInfo {
        code: STORE_GLOBAL,
        name: "StoreGlobal",
        size: 5,
    },
    // ── 整数运算 0x20-0x2F ──
    OpcodeInfo {
        code: I64_ADD,
        name: "I64Add",
        size: 7,
    },
    OpcodeInfo {
        code: I64_SUB,
        name: "I64Sub",
        size: 7,
    },
    OpcodeInfo {
        code: I64_MUL,
        name: "I64Mul",
        size: 7,
    },
    OpcodeInfo {
        code: I64_DIV,
        name: "I64Div",
        size: 7,
    },
    OpcodeInfo {
        code: I64_REM,
        name: "I64Rem",
        size: 7,
    },
    OpcodeInfo {
        code: I64_AND,
        name: "I64And",
        size: 7,
    },
    OpcodeInfo {
        code: I64_OR,
        name: "I64Or",
        size: 7,
    },
    OpcodeInfo {
        code: I64_XOR,
        name: "I64Xor",
        size: 7,
    },
    OpcodeInfo {
        code: I64_SHL,
        name: "I64Shl",
        size: 7,
    },
    OpcodeInfo {
        code: I64_SAR,
        name: "I64Sar",
        size: 7,
    },
    OpcodeInfo {
        code: I64_SHR,
        name: "I64Shr",
        size: 7,
    },
    OpcodeInfo {
        code: I64_NEG,
        name: "I64Neg",
        size: 5,
    },
    OpcodeInfo {
        code: I64_NOT,
        name: "I64Not",
        size: 5,
    },
    // ── 比较 0x60-0x65 ──
    OpcodeInfo {
        code: I64_EQ,
        name: "I64Eq",
        size: 7,
    },
    OpcodeInfo {
        code: I64_NE,
        name: "I64Ne",
        size: 7,
    },
    OpcodeInfo {
        code: I64_LT,
        name: "I64Lt",
        size: 7,
    },
    OpcodeInfo {
        code: I64_LE,
        name: "I64Le",
        size: 7,
    },
    OpcodeInfo {
        code: I64_GT,
        name: "I64Gt",
        size: 7,
    },
    OpcodeInfo {
        code: I64_GE,
        name: "I64Ge",
        size: 7,
    },
    // ── 内存与对象 0x72-0x7F ──
    OpcodeInfo {
        code: HEAP_ALLOC,
        name: "HeapAlloc",
        size: 4,
    },
    OpcodeInfo {
        code: STACK_ALLOC,
        name: "StackAlloc",
        size: 2,
    },
    OpcodeInfo {
        code: DROP,
        name: "Drop",
        size: 2,
    },
    OpcodeInfo {
        code: GET_FIELD,
        name: "GetField",
        size: 5,
    },
    OpcodeInfo {
        code: SET_FIELD,
        name: "SetField",
        size: 5,
    },
    OpcodeInfo {
        code: LOAD_ELEMENT,
        name: "LoadElement",
        size: 5,
    },
    OpcodeInfo {
        code: STORE_ELEMENT,
        name: "StoreElement",
        size: 5,
    },
    OpcodeInfo {
        code: CREATE_STRUCT,
        name: "CreateStruct",
        size: 7,
    },
    OpcodeInfo {
        code: NEW_LIST_WITH_CAP,
        name: "NewListWithCap",
        size: 5,
    },
    OpcodeInfo {
        code: ARC_NEW,
        name: "ArcNew",
        size: 3,
    },
    OpcodeInfo {
        code: ARC_CLONE,
        name: "ArcClone",
        size: 3,
    },
    OpcodeInfo {
        code: ARC_DROP,
        name: "ArcDrop",
        size: 2,
    },
    OpcodeInfo {
        code: WEAK_NEW,
        name: "WeakNew",
        size: 5,
    },
    OpcodeInfo {
        code: WEAK_UPGRADE,
        name: "WeakUpgrade",
        size: 5,
    },
    // ── 函数调用 0x80-0x8F ──
    OpcodeInfo {
        code: CALL_STATIC,
        name: "CallStatic",
        size: 5,
    },
    OpcodeInfo {
        code: CALL_VIRT,
        name: "CallVirt",
        size: 5,
    },
    OpcodeInfo {
        code: CALL_DYN,
        name: "CallDyn",
        size: 5,
    },
    OpcodeInfo {
        code: MAKE_CLOSURE,
        name: "MakeClosure",
        size: 5,
    },
    OpcodeInfo {
        code: LOAD_UPVALUE,
        name: "LoadUpvalue",
        size: 4,
    },
    OpcodeInfo {
        code: STORE_UPVALUE,
        name: "StoreUpvalue",
        size: 4,
    },
    OpcodeInfo {
        code: CLOSE_UPVALUE,
        name: "CloseUpvalue",
        size: 3,
    },
    OpcodeInfo {
        code: CALL_NATIVE,
        name: "CallNative",
        size: 5,
    },
    OpcodeInfo {
        code: NEW_DICT,
        name: "NewDict",
        size: 7,
    },
    OpcodeInfo {
        code: RC_NEW,
        name: "RcNew",
        size: 3,
    },
    OpcodeInfo {
        code: NEW_TUPLE,
        name: "NewTuple",
        size: 7,
    },
    OpcodeInfo {
        code: NEW_ARRAY,
        name: "NewArray",
        size: 6,
    },
    OpcodeInfo {
        code: CONTAINS,
        name: "Contains",
        size: 4,
    },
    OpcodeInfo {
        code: NEW_RANGE,
        name: "NewRange",
        size: 9,
    },
    // ── 字符串 0x90-0x9F ──
    OpcodeInfo {
        code: STRING_LENGTH,
        name: "StringLength",
        size: 5,
    },
    OpcodeInfo {
        code: STRING_CONCAT,
        name: "StringConcat",
        size: 5,
    },
    OpcodeInfo {
        code: STRING_EQUAL,
        name: "StringEqual",
        size: 5,
    },
    OpcodeInfo {
        code: STRING_GET_CHAR,
        name: "StringGetChar",
        size: 5,
    },
    OpcodeInfo {
        code: STRING_FROM_INT,
        name: "StringFromInt",
        size: 5,
    },
    OpcodeInfo {
        code: STRING_FROM_FLOAT,
        name: "StringFromFloat",
        size: 5,
    },
    // ── 异常 0xA0-0xAF（0xA0/0xA1 已回收，见上） ──
    OpcodeInfo {
        code: THROW,
        name: "Throw",
        size: 3,
    },
    // ── 调试 / 类型 / 反射 0xB0-0xDF ──
    OpcodeInfo {
        code: BOUNDS_CHECK,
        name: "BoundsCheck",
        size: 5,
    },
    // ── 类型 0xC0-0xCF ──
    OpcodeInfo {
        code: TYPE_CHECK,
        name: "TypeCheck",
        size: 5,
    },
    OpcodeInfo {
        code: CAST,
        name: "Cast",
        size: 5,
    },
    // ── 反射 0xD0-0xDF ──
    OpcodeInfo {
        code: TYPE_OF,
        name: "TypeOf",
        size: 5,
    },
    // ── 变体 0xE0-0xEF ──
    OpcodeInfo {
        code: CREATE_VARIANT,
        name: "CreateVariant",
        size: 11,
    },
    OpcodeInfo {
        code: VARIANT_TAG,
        name: "VariantTag",
        size: 7,
    },
    OpcodeInfo {
        code: VARIANT_PAYLOAD,
        name: "VariantPayload",
        size: 7,
    },
];

/// 按码值查权威表项；`None` 表示该码不在词表里（已回收或从未分配）。
pub fn opcode_info(code: u8) -> Option<&'static OpcodeInfo> {
    OPCODE_INFO.iter().find(|info| info.code == code)
}

/// 操作码名称（调试/转储用）。表外码返回 `"Unknown"`。
pub fn opcode_name(code: u8) -> &'static str {
    match opcode_info(code) {
        Some(info) => info.name,
        None => "Unknown",
    }
}

/// 指令的固定字节数（含 opcode 字节，不含变长尾部）。
pub fn opcode_size(code: u8) -> Option<usize> {
    opcode_info(code).map(|info| info.size)
}
