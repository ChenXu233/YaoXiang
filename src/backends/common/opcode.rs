//! Bytecode opcode definitions for YaoXiang
//!
//! Opcode 是 .42 文件格式的 u8 编码标签。单一词表：
//! 编码（translator/emitter 写入 u8）与解码（bytecode.rs 按 u8 匹配）共用同一组常量，
//! 不再有枚举 ↔ 字节的双词表漂移。

// Control Flow (0x00-0x1F)
pub const NOP: u8 = 0x00;
pub const RETURN: u8 = 0x01;
pub const RETURN_VALUE: u8 = 0x02;
pub const JMP: u8 = 0x03;
pub const JMP_IF: u8 = 0x04;
pub const JMP_IF_NOT: u8 = 0x05;
pub const SWITCH: u8 = 0x06;
pub const TAIL_CALL: u8 = 0x09;
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
pub const TRY_BEGIN: u8 = 0xA0;
pub const TRY_END: u8 = 0xA1;
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

/// 操作码名称（调试/转储用）
pub fn opcode_name(code: u8) -> &'static str {
    match code {
        NOP => "Nop",
        RETURN => "Return",
        RETURN_VALUE => "ReturnValue",
        JMP => "Jmp",
        JMP_IF => "JmpIf",
        JMP_IF_NOT => "JmpIfNot",
        SWITCH => "Switch",
        TAIL_CALL => "TailCall",
        YIELD => "Yield",
        LABEL => "Label",
        SPAWN => "Spawn",
        SPAWN_FROM_LIST => "SpawnFromList",
        MOV => "Mov",
        LOAD_CONST => "LoadConst",
        LOAD_LOCAL => "LoadLocal",
        STORE_LOCAL => "StoreLocal",
        LOAD_ARG => "LoadArg",
        BORROW => "Borrow",
        RELEASE => "Release",
        I64_ADD => "I64Add",
        I64_SUB => "I64Sub",
        I64_MUL => "I64Mul",
        I64_DIV => "I64Div",
        I64_REM => "I64Rem",
        I64_AND => "I64And",
        I64_OR => "I64Or",
        I64_XOR => "I64Xor",
        I64_SHL => "I64Shl",
        I64_SAR => "I64Sar",
        I64_SHR => "I64Shr",
        I64_NEG => "I64Neg",
        I64_EQ => "I64Eq",
        I64_NE => "I64Ne",
        I64_LT => "I64Lt",
        I64_LE => "I64Le",
        I64_GT => "I64Gt",
        I64_GE => "I64Ge",
        HEAP_ALLOC => "HeapAlloc",
        STACK_ALLOC => "StackAlloc",
        DROP => "Drop",
        GET_FIELD => "GetField",
        SET_FIELD => "SetField",
        LOAD_ELEMENT => "LoadElement",
        STORE_ELEMENT => "StoreElement",
        CREATE_STRUCT => "CreateStruct",
        NEW_LIST_WITH_CAP => "NewListWithCap",
        ARC_NEW => "ArcNew",
        ARC_CLONE => "ArcClone",
        ARC_DROP => "ArcDrop",
        WEAK_NEW => "WeakNew",
        WEAK_UPGRADE => "WeakUpgrade",
        CALL_STATIC => "CallStatic",
        CALL_VIRT => "CallVirt",
        CALL_DYN => "CallDyn",
        MAKE_CLOSURE => "MakeClosure",
        LOAD_UPVALUE => "LoadUpvalue",
        STORE_UPVALUE => "StoreUpvalue",
        CLOSE_UPVALUE => "CloseUpvalue",
        CALL_NATIVE => "CallNative",
        NEW_DICT => "NewDict",
        RC_NEW => "RcNew",
        NEW_TUPLE => "NewTuple",
        NEW_RANGE => "NewRange",
        NEW_ARRAY => "NewArray",
        CONTAINS => "Contains",
        STRING_LENGTH => "StringLength",
        STRING_CONCAT => "StringConcat",
        STRING_EQUAL => "StringEqual",
        STRING_GET_CHAR => "StringGetChar",
        STRING_FROM_INT => "StringFromInt",
        STRING_FROM_FLOAT => "StringFromFloat",
        TRY_BEGIN => "TryBegin",
        TRY_END => "TryEnd",
        THROW => "Throw",
        BOUNDS_CHECK => "BoundsCheck",
        TYPE_CHECK => "TypeCheck",
        CAST => "Cast",
        TYPE_OF => "TypeOf",
        CREATE_VARIANT => "CreateVariant",
        VARIANT_TAG => "VariantTag",
        VARIANT_PAYLOAD => "VariantPayload",
        _ => "Unknown",
    }
}
