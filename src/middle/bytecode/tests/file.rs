//! 字节码序列化单元测试
//!
//! 测试 DebugSection 的序列化和反序列化（round-trip）功能。
//!
//! 规范来源：
//! - RFC-034 §「IR 调试元数据」（调试段格式）：v1 仅 ip→span；v2 增每函数
//!   局部变量名表；v3 增全局槽位名表。读取端接受 v1/v2/v3，旧产物补空表不 panic。
//!
//! 本文件另覆盖 .42 主格式的升版判据（RFC-039 D17）：
//! - **条款门**：RFC-039 D17「新增字段时把 .42 版本号从 4 升到 5」——函数记录的
//!   `upvalue_count` 必须随 `.42` 往返保真；旧版本文件按既有行为拒绝。
//!
//! 逐条判据的准入依据写在各自测试的文档注释里（文件头只给条款来源，
//! 不让整份文件的测试共享一条病史）。

use crate::frontend::core::sema::MonoType;
use crate::middle::bytecode::file::{
    mono_type_id, BytecodeFile, BytecodeInstruction, CodeSection, DebugSection, FileHeader,
    FunctionCode,
};
use crate::middle::bytecode::opcode;
use crate::middle::bytecode::GlobalInfo;
use crate::util::span::{DebugSpan, Position, SourceMap, Span};
use std::collections::HashMap;
use std::io;

// ── 辅助函数（规则 5.3：集中在文件顶部）──────────────

/// Helper: 构造含单个 main 函数的字节码文件，其 ip→span 调试映射为 `debug_span`。
fn bytecode_file_with_debug_map(
    sources: SourceMap,
    debug_span: DebugSpan,
) -> BytecodeFile {
    let function = FunctionCode {
        name: "main".to_string(),
        params: Vec::new(),
        return_type: MonoType::Void,
        instructions: vec![BytecodeInstruction::new(opcode::NOP, vec![])],
        local_count: 0,
        upvalue_count: 0,
        local_names: HashMap::new(),
        debug_map: HashMap::from([(0usize, debug_span)]),
    };

    let code_section = CodeSection {
        functions: vec![function],
    };

    let debug_section = DebugSection::from_sources_and_functions(sources, &code_section.functions);

    BytecodeFile {
        header: FileHeader::default(),
        type_table: Vec::new(),
        const_pool: Vec::new(),
        code_section,
        vtables: Vec::new(),
        globals: Vec::new(),
        debug_section: Some(debug_section),
    }
}

/// Helper: 序列化字节码文件后从尾部读回调试段。
fn roundtrip_debug_section(file: &BytecodeFile) -> DebugSection {
    let mut bytes = Vec::new();
    file.write_to(&mut bytes).expect("write bytecode");

    let mut cursor = io::Cursor::new(bytes);
    DebugSection::read_from_end(&mut cursor)
        .expect("read debug section")
        .expect("debug section should exist")
}
/// 病史门 + 条款门（RFC-039 D17）：`.42` 载入侧曾把 `upvalue_count` 硬编 0，
/// 闭包的上值槽数经一次落盘往返就丢了，而产物与运行结果都看不出异常
/// （WBS 7e.1 收编的数据丢失）。
#[test]
fn test_function_upvalue_count_round_trips_through_dot42() {
    // Arrange：带 2 个上值槽的函数（闭包体读 upvalue#0 / #1）
    let function = FunctionCode {
        name: "closure_0".to_string(),
        params: Vec::new(),
        return_type: MonoType::Void,
        instructions: vec![
            BytecodeInstruction::new(opcode::LOAD_UPVALUE, vec![0, 0]),
            BytecodeInstruction::new(opcode::LOAD_UPVALUE, vec![1, 1]),
            BytecodeInstruction::new(opcode::RETURN, vec![]),
        ],
        local_count: 2,
        upvalue_count: 2,
        local_names: HashMap::new(),
        debug_map: HashMap::new(),
    };
    let file = BytecodeFile {
        header: FileHeader::default(),
        type_table: Vec::new(),
        const_pool: Vec::new(),
        code_section: CodeSection {
            functions: vec![function],
        },
        vtables: Vec::new(),
        globals: Vec::new(),
        debug_section: None,
    };

    // Act：落盘 → 读回
    let mut bytes = Vec::new();
    file.write_to(&mut bytes).expect("write bytecode");
    let mut cursor = io::Cursor::new(bytes);
    let decoded = BytecodeFile::read_from(&mut cursor).expect("read bytecode");

    // Assert：上值槽数与源码侧一致（升版前这里恒为 0）
    assert_eq!(
        decoded.code_section.functions[0].upvalue_count, 2,
        "upvalue_count 必须随 .42 往返保真；读到 0 说明字段未落盘（D17 的数据丢失形态）"
    );
    assert_eq!(
        decoded.code_section.functions[0].local_count, 2,
        "同一记录的 local_count 必须与写入侧一致（字段顺序对称性的对照锚）"
    );
}

/// 条款门（RFC-039 D17「升版后旧 .42 被读取侧按既有行为拒绝」）：旧版本文件必须
/// 得到可分类、可定位的诊断，且不得 panic、不得按新布局继续读（回归风险：
/// 版本校验一旦失效，v4 产物的字段会按 v5 布局错位解释）。
#[test]
fn test_old_format_version_file_is_rejected_with_diagnostic() {
    // Arrange：合法魔数 + 旧版本号 4（当前格式版本的前一代）
    let mut bytes = Vec::new();
    bytes.extend_from_slice(&0x5958_4243u32.to_be_bytes());
    bytes.extend_from_slice(&4u32.to_le_bytes());

    // Act
    let mut cursor = io::Cursor::new(bytes);
    let err = BytecodeFile::read_from(&mut cursor)
        .expect_err("旧版本 .42 必须被拒绝，不得按 v5 布局继续读");

    // Assert：错误码（ErrorKind）与消息都能定位「版本不匹配」
    assert_eq!(
        err.kind(),
        io::ErrorKind::Unsupported,
        "版本不匹配必须是 Unsupported，与魔数错误的 InvalidData 分开；实际 {err:?}"
    );
    let msg = format!("{err}");
    for needle in ["unsupported", "4", "5"] {
        assert!(
            msg.contains(needle),
            "诊断消息必须给出文件版本与当前版本（缺 {needle:?}）：{msg}"
        );
    }
    assert!(
        msg.contains("rebuild"),
        "诊断消息必须给出可执行动作（重新构建 .42）：{msg}"
    );
}

/// 条款门（RFC-039 D17 的格式完备性）+ 病史门：写侧曾对
/// `ConstValue::LibraryRef` / `ExternRef` 直接 `todo!()`——可达路径上的 panic。
///
/// 可达性：`middle/lower/const_eval.rs` 在编译期把 `Native.c("lib")` / `lib("sym")`
/// 求值成这两个变体；只要其中一个以值进入 `Instruction::Load`，就会经
/// `emitter.add_constant` 进常量池并到达写侧。故本锚要求：拒绝，且是诊断不是 panic。
#[test]
fn test_ffi_compile_time_constants_are_rejected_without_panic() {
    // Arrange：常量池里放一个编译期 FFI 描述值
    let function = FunctionCode {
        name: "main".to_string(),
        params: Vec::new(),
        return_type: MonoType::Void,
        instructions: vec![BytecodeInstruction::new(opcode::RETURN, vec![])],
        local_count: 0,
        upvalue_count: 0,
        local_names: HashMap::new(),
        debug_map: HashMap::new(),
    };
    let file = BytecodeFile {
        header: FileHeader::default(),
        type_table: Vec::new(),
        const_pool: vec![crate::middle::ir::ir::ConstValue::LibraryRef {
            mechanism: "Native".to_string(),
            lib: "sqlite3".to_string(),
        }],
        code_section: CodeSection {
            functions: vec![function],
        },
        vtables: Vec::new(),
        globals: Vec::new(),
        debug_section: None,
    };

    // Act：写侧必须返回错误（panic 会让本用例直接失败）
    let mut bytes = Vec::new();
    let err = file
        .write_to(&mut bytes)
        .expect_err("编译期 FFI 常量必须被拒绝，不得写入 .42");

    // Assert：可分类 + 可定位（含机制与库名）
    assert_eq!(
        err.kind(),
        io::ErrorKind::InvalidInput,
        "拒绝编译期 FFI 常量应是 InvalidInput（调用方输入不可表示），实际 {err:?}"
    );
    let msg = format!("{err}");
    for needle in ["LibraryRef", "Native", "sqlite3"] {
        assert!(
            msg.contains(needle),
            "诊断必须点出常量种类与定位信息（缺 {needle:?}）：{msg}"
        );
    }
}

/// 同一条拒绝面覆盖**类型表/签名**：`MonoType::LibraryRef` / `ExternRef` 写侧
/// 曾同样 `todo!()`。二者都是编译期 FFI 描述类型，运行期没有表示——
/// 写进类型表只能读回成别的东西（静默错配），故必须拒绝而不是降级。
#[test]
fn test_ffi_compile_time_types_are_rejected_without_panic() {
    // Arrange：返回类型是编译期 FFI 绑定描述类型
    let function = FunctionCode {
        name: "main".to_string(),
        params: Vec::new(),
        return_type: MonoType::ExternRef {
            mechanism: "Native".to_string(),
            lib: "sqlite3".to_string(),
            symbol: "sqlite3_open".to_string(),
        },
        instructions: vec![BytecodeInstruction::new(opcode::RETURN, vec![])],
        local_count: 0,
        upvalue_count: 0,
        local_names: HashMap::new(),
        debug_map: HashMap::new(),
    };
    let file = BytecodeFile {
        header: FileHeader::default(),
        type_table: Vec::new(),
        const_pool: Vec::new(),
        code_section: CodeSection {
            functions: vec![function],
        },
        vtables: Vec::new(),
        globals: Vec::new(),
        debug_section: None,
    };

    // Act
    let mut bytes = Vec::new();
    let err = file
        .write_to(&mut bytes)
        .expect_err("编译期 FFI 类型必须被拒绝，不得写入 .42");

    // Assert
    assert_eq!(
        err.kind(),
        io::ErrorKind::InvalidInput,
        "拒绝编译期 FFI 类型应是 InvalidInput，实际 {err:?}"
    );
    let msg = format!("{err}");
    for needle in ["ExternRef", "sqlite3_open"] {
        assert!(
            msg.contains(needle),
            "诊断必须点出类型种类与符号（缺 {needle:?}）：{msg}"
        );
    }
}

/// 病史门 + 条款门（RFC-039 D52 的 `2341` 项）：`.42` 的全局表往返保真。
///
/// 缺陷形态：`BytecodeFile` 没有全局表，`From<BytecodeFile>` 里 `globals: Vec::new()`——
/// 载入后「有哪些全局、各是什么类型」整张表消失，只剩调试段里的名字。
/// 表序即槽位号，`type_id` 是 .42 类型号（与形参/返回/类型表同一套编号）。
#[test]
fn test_globals_table_round_trips_through_dot42() {
    // Arrange：两个全局（Int / String），类型号经 `mono_type_id` 取——与生产者同一口径
    let int_id = mono_type_id(&MonoType::Int(64)).expect("Int 类型号必须可编");
    let str_id = mono_type_id(&MonoType::make_string()).expect("String 类型号必须可编");
    let file = BytecodeFile {
        header: FileHeader::default(),
        type_table: Vec::new(),
        const_pool: Vec::new(),
        code_section: CodeSection {
            functions: vec![FunctionCode {
                name: "__yx_module_init".to_string(),
                params: Vec::new(),
                return_type: MonoType::Void,
                instructions: vec![BytecodeInstruction::new(opcode::RETURN, vec![])],
                local_count: 0,
                upvalue_count: 0,
                local_names: HashMap::new(),
                debug_map: HashMap::new(),
            }],
        },
        vtables: Vec::new(),
        globals: vec![
            GlobalInfo {
                name: "counter".to_string(),
                type_id: int_id as u16,
                initializer: None,
                is_mutable: false,
            },
            GlobalInfo {
                name: "label".to_string(),
                type_id: str_id as u16,
                initializer: None,
                is_mutable: false,
            },
        ],
        debug_section: None,
    };

    // Act：落盘 → 读回
    let mut bytes = Vec::new();
    file.write_to(&mut bytes).expect("write bytecode");
    let mut cursor = io::Cursor::new(bytes);
    let decoded = BytecodeFile::read_from(&mut cursor).expect("read bytecode");

    // Assert：表长、名字、类型号、表序（= 槽位号）逐项一致
    assert_eq!(
        decoded.globals.len(),
        2,
        "全局表必须随 .42 往返；读到空表说明载入侧又硬编了 Vec::new()"
    );
    assert_eq!(
        decoded.globals[0].name, "counter",
        "表序 0 的全局名必须与写入侧一致（表序即槽位号）"
    );
    assert_eq!(
        decoded.globals[0].type_id, int_id as u16,
        "counter 的类型号必须原样往返"
    );
    assert_eq!(
        decoded.globals[1].name, "label",
        "表序 1 的全局名必须与写入侧一致"
    );
    assert_eq!(
        decoded.globals[1].type_id, str_id as u16,
        "label 的类型号必须原样往返"
    );
}

/// 条款门（D52 的「不得静默丢弃」）：全局表不携带 `initializer`，写侧必须拒绝而非丢成 None。
#[test]
fn test_global_initializer_is_rejected_without_silent_loss() {
    // Arrange：一个带编译期初始化值的全局（IR 已把它降级成初始化序列，此处刻意构造）
    let file = BytecodeFile {
        header: FileHeader::default(),
        type_table: Vec::new(),
        const_pool: Vec::new(),
        code_section: CodeSection {
            functions: Vec::new(),
        },
        vtables: Vec::new(),
        globals: vec![GlobalInfo {
            name: "seeded".to_string(),
            type_id: 5,
            initializer: Some(crate::middle::ir::ir::ConstValue::Int(7)),
            is_mutable: false,
        }],
        debug_section: None,
    };

    // Act
    let mut bytes = Vec::new();
    let err = file
        .write_to(&mut bytes)
        .expect_err("带 initializer 的全局不得静默落盘");

    // Assert：可分类 + 可定位到具体全局
    assert_eq!(
        err.kind(),
        io::ErrorKind::InvalidInput,
        "拒绝未携带字段应是 InvalidInput，实际 {err:?}"
    );
    let msg = format!("{err}");
    assert!(
        msg.contains("initializer") && msg.contains("seeded"),
        "诊断必须点出字段与全局名：{msg}"
    );
}

/// 条款门（同上；可变性的载体在 AST/lower，不在字节码 globals 表）：
/// 写侧遇到 `is_mutable = true` 必须拒绝，绝不静默读成不可变。
#[test]
fn test_global_mutability_is_rejected_without_silent_loss() {
    // Arrange
    let file = BytecodeFile {
        header: FileHeader::default(),
        type_table: Vec::new(),
        const_pool: Vec::new(),
        code_section: CodeSection {
            functions: Vec::new(),
        },
        vtables: Vec::new(),
        globals: vec![GlobalInfo {
            name: "counter".to_string(),
            type_id: 5,
            initializer: None,
            is_mutable: true,
        }],
        debug_section: None,
    };

    // Act
    let mut bytes = Vec::new();
    let err = file
        .write_to(&mut bytes)
        .expect_err("可变性没有载体，不得静默丢成 false");

    // Assert
    assert_eq!(
        err.kind(),
        io::ErrorKind::InvalidInput,
        "拒绝未携带字段应是 InvalidInput，实际 {err:?}"
    );
    let msg = format!("{err}");
    assert!(
        msg.contains("可变性") && msg.contains("counter"),
        "诊断必须点出字段语义与全局名：{msg}"
    );
}

#[test]
fn test_debug_section_round_trip() {
    // Arrange
    let mut sources = SourceMap::new();
    let file_id = sources.add_file("main.yx".to_string(), "main = () => { 1 / 0 }".to_string());

    let span = Span::new(
        Position::with_offset(1, 1, 0),
        Position::with_offset(1, 5, 4),
    );
    let debug_span = DebugSpan::new(file_id, span);
    let file = bytecode_file_with_debug_map(sources.clone(), debug_span);

    // Act
    let decoded = roundtrip_debug_section(&file);

    assert_eq!(decoded.sources.files().len(), 1);
    assert_eq!(decoded.sources.files()[0].name, "main.yx");
    assert_eq!(decoded.sources.files()[0].content, "main = () => { 1 / 0 }");
    assert_eq!(decoded.function_debug_maps.len(), 1);
    assert_eq!(
        decoded.function_debug_maps[0].get(&0).copied(),
        Some(debug_span)
    );
}

#[test]
fn test_debug_section_round_trip_global_names() {
    // #368：全局槽位名表（v3）——顶层绑定的名字不在任何函数局部名表里，
    // 顶层 `a[i]` 的越界诊断要靠它才能报出 `i`。往返必须保真。
    let mut sources = SourceMap::new();
    sources.add_file("script.yx".to_string(), "a = [1]\ni = 5".to_string());

    let code_section = CodeSection { functions: vec![] };
    let debug_section = DebugSection::with_global_names(
        sources,
        &code_section.functions,
        HashMap::from([(0usize, "a".to_string()), (1usize, "i".to_string())]),
    );
    let file = BytecodeFile {
        header: FileHeader::default(),
        type_table: Vec::new(),
        const_pool: Vec::new(),
        code_section,
        vtables: Vec::new(),
        globals: Vec::new(),
        debug_section: Some(debug_section),
    };

    let mut bytes = Vec::new();
    file.write_to(&mut bytes).expect("write bytecode");

    let mut cursor = io::Cursor::new(bytes);
    let decoded = DebugSection::read_from_end(&mut cursor)
        .expect("read debug section")
        .expect("debug section should exist");

    assert_eq!(decoded.global_names.get(&0).map(String::as_str), Some("a"));
    assert_eq!(decoded.global_names.get(&1).map(String::as_str), Some("i"));
}
