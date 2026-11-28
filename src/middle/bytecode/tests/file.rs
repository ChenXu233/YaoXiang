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
use crate::frontend::core::types::identity::Nominal;
use crate::middle::bytecode::file::{
    mono_type_id, type_id_to_monotype, BytecodeFile, BytecodeInstruction, CodeSection,
    DebugSection, FileHeader, FunctionCode,
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
    // Arrange：两个全局（Int / String），载体是 MonoType（与函数签名同口径）
    let int_ty = MonoType::Int(64);
    let str_ty = MonoType::make_string();
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
                ty: int_ty.clone(),
                initializer: None,
                is_mutable: false,
            },
            GlobalInfo {
                name: "label".to_string(),
                ty: str_ty.clone(),
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

    // Assert：表长、名字、类型（MonoType 恒等）、表序（= 槽位号）逐项一致
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
        decoded.globals[0].ty, int_ty,
        "counter 的类型（MonoType）必须原样往返；不等即类型号编码不可逆"
    );
    assert_eq!(
        decoded.globals[1].name, "label",
        "表序 1 的全局名必须与写入侧一致"
    );
    assert_eq!(
        decoded.globals[1].ty, str_ty,
        "label 的类型（MonoType）必须原样往返"
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
            ty: MonoType::Int(64),
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
            ty: MonoType::Int(64),
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

/// 条款门（.42 类型号）＋病史门（7e.3 收口时暴露的静默错配）：
/// 可逆类型号必须**无碰撞**且**往返恒等**——同一号两个类型等于一次落盘就把默认整数变成浮点。
///
/// 号段约定：`0..=12` 基本标量（Char/String/Bytes 保持历史号 10/11/12）；
/// `20..=50` 折叠类（Struct/Enum/Fn/… 仍单号，读回既定 lossy 形态 `TypeRef("_")`）；
/// `60` 以后是无运行时表示的元类型/底类型。
#[test]
fn test_reversible_type_codes_round_trip_and_are_distinct() {
    // Arrange：可逆类型全集（Int 五档 + Float 两档 + 基本标量），附期望号与名字
    let cases: &[(&str, MonoType, u32)] = &[
        ("Void", MonoType::Void, 0),
        ("Bool", MonoType::Bool, 1),
        ("Int(8)", MonoType::Int(8), 2),
        ("Int(16)", MonoType::Int(16), 3),
        ("Int(32)", MonoType::Int(32), 4),
        ("Int(64)", MonoType::Int(64), 5),
        ("Int(128)", MonoType::Int(128), 6),
        ("Float(32)", MonoType::Float(32), 7),
        ("Float(64)", MonoType::Float(64), 8),
        ("Char", MonoType::Char, 10),
        ("String", MonoType::make_string(), 11),
        (
            "Bytes",
            MonoType::Generic {
                name: Nominal::flat("Bytes").expect("测试类型名必须是裸名且非空"),
                args: Vec::new(),
            },
            12,
        ),
    ];

    // Act / Assert 1：逐项给出 id 与类型；编码 → 解码必须恒等
    for (name, ty, expected_id) in cases {
        let id = mono_type_id(ty).unwrap_or_else(|e| panic!("{name} 必须可编：{e}"));
        assert_eq!(
            id, *expected_id,
            "{name} 的类型号应为 {expected_id}，实际 {id}"
        );
        let back = type_id_to_monotype(id);
        assert_eq!(
            &back, ty,
            "{name} 经类型号 {id} 解码必须恒等，实际 {back:?}"
        );
    }

    // Assert 2：两两不同号（同号 = 一个号解不出两个类型）
    for (i, (name_a, _, id_a)) in cases.iter().enumerate() {
        for (name_b, _, id_b) in cases.iter().skip(i + 1) {
            assert_ne!(
                id_a, id_b,
                "{name_a} 与 {name_b} 不得共用类型号 {id_a}（碰撞会让解码二选一）"
            );
        }
    }
}

/// 条款门：未知位宽与编译期 FFI 类型在**写侧**拒绝，不就近折算成别的类型号。
#[test]
fn test_unsupported_type_widths_are_rejected_on_write() {
    // Arrange：语言不产生的位宽（Int 24 / Float 16 / Float 8）
    let unsupported = [
        ("Int(24)", MonoType::Int(24)),
        ("Float(16)", MonoType::Float(16)),
        ("Float(8)", MonoType::Float(8)),
    ];

    for (name, ty) in unsupported {
        // Act
        let err = mono_type_id(&ty).expect_err("未登记位宽必须拒绝，不得就近折算");

        // Assert：可分类 + 点出位宽
        assert_eq!(
            err.kind(),
            io::ErrorKind::InvalidInput,
            "{name} 的拒绝应是 InvalidInput，实际 {err:?}"
        );
        let msg = format!("{err}");
        assert!(
            msg.contains("位宽") && msg.contains(name),
            "诊断必须点出位宽与类型名（缺 {name}）：{msg}"
        );
    }
}

/// 条款门：折叠类**行为不变**（仍单号、读回既定 lossy 形态 `TypeRef("_")`），
/// 但彼此之间不得共用号：`TypeId::STRUCT=20` / `TypeId::ENUM=21` 与运行时常量对齐，
/// `Option/Result` 归入 21（运行时的 sum type 身份就是 21）。
#[test]
fn test_folded_type_codes_are_distinct_and_lossy() {
    // Arrange：每个折叠类一个代表 + 期望号
    let folds: &[(&str, MonoType, u32)] = &[
        (
            "Struct",
            MonoType::Struct(crate::frontend::core::sema::StructType {
                name: Nominal::flat("P").expect("测试类型名必须是裸名且非空"),
                fields: Vec::new(),
                methods: std::collections::HashMap::new(),
                interfaces: Vec::new(),
            }),
            20,
        ),
        (
            "Enum",
            MonoType::Enum(crate::frontend::core::sema::EnumType {
                name: Nominal::flat("E").expect("测试类型名必须是裸名且非空"),
                variants: Vec::new(),
            }),
            21,
        ),
        (
            "Option",
            MonoType::Generic {
                name: Nominal::flat("Option").expect("测试类型名必须是裸名且非空"),
                args: vec![MonoType::Int(64)],
            },
            21,
        ),
        (
            "Fn",
            MonoType::Fn {
                params: Vec::new(),
                return_type: Box::new(MonoType::Void),
            },
            30,
        ),
        (
            "DepFn",
            MonoType::DepFn {
                params: Vec::new(),
                return_type: Box::new(MonoType::Void),
            },
            31,
        ),
        (
            "TypeRef",
            MonoType::TypeRef(Nominal::flat("P").expect("测试占位名必须是裸名且非空")),
            40,
        ),
        ("Union", MonoType::Union(vec![MonoType::Int(64)]), 41),
        (
            "Intersection",
            MonoType::Intersection(vec![MonoType::Int(64)]),
            42,
        ),
        ("Never", MonoType::Never, 61),
    ];

    // Act / Assert 1：号与期望一致，且读回一律是既定 lossy 形态
    for (name, ty, expected_id) in folds {
        let id = mono_type_id(ty).unwrap_or_else(|e| panic!("{name} 必须可编：{e}"));
        assert_eq!(
            id, *expected_id,
            "{name} 的类型号应为 {expected_id}，实际 {id}"
        );
        assert_eq!(
            type_id_to_monotype(id),
            MonoType::TypeRef(Nominal::flat("_").expect("测试占位名必须是裸名且非空")),
            "{name}（号 {id}）读回必须是既定 lossy 形态 TypeRef(\"_\")"
        );
    }

    // Assert 2：折叠类之间除「Option/Result 归入 Enum 的 sum 身份」外两两不同号
    for (i, (name_a, _, id_a)) in folds.iter().enumerate() {
        for (name_b, _, id_b) in folds.iter().skip(i + 1) {
            if (*name_a == "Enum" && *name_b == "Option")
                || (*name_a == "Option" && *name_b == "Enum")
            {
                continue;
            }
            assert_ne!(id_a, id_b, "{name_a} 与 {name_b} 不得共用类型号 {id_a}");
        }
    }
}

/// 条款门（.42 类型号可逆段只认内建标量身份）：`String`/`Bytes` 只接受**扁平内建身份 +
/// 无类型参数**；名义作用域内的同名类型（`Nominal::item("m", "String")`）与带参数的实例化
/// （`Generic { name: String, args: [Int(64)] }`）都不是内建 `String`，不得拿到 11/12。
///
/// 病史门：`Generic { name, .. }` 的通配曾让「模块作用域同名类型」和「带参数实例化」一起命中
/// 内建号；读回变成 `String`/`Bytes`，是「一次落盘就把别的类型变成内建类型」的静默错配。
#[test]
fn test_builtin_string_and_bytes_codes_require_flat_identity_without_args() {
    // Arrange：两个正例（内建扁平身份）与四个负例（不该拿 11/12 的形态）
    let builtin_string = MonoType::make_string();
    let builtin_bytes = MonoType::Generic {
        name: Nominal::flat("Bytes").expect("内建名必须可解析"),
        args: Vec::new(),
    };
    let module_scoped_string = MonoType::Generic {
        name: Nominal::item("m", "String").expect("名义作用域类型名必须可解析"),
        args: Vec::new(),
    };
    let parameterised_string = MonoType::Generic {
        name: Nominal::flat("String").expect("内建名必须可解析"),
        args: vec![MonoType::Int(64)],
    };
    let module_scoped_bytes = MonoType::Generic {
        name: Nominal::item("m", "Bytes").expect("名义作用域类型名必须可解析"),
        args: Vec::new(),
    };
    let parameterised_bytes = MonoType::Generic {
        name: Nominal::flat("Bytes").expect("内建名必须可解析"),
        args: vec![MonoType::Int(64)],
    };

    // Act：取类型号
    let builtin_string_id = mono_type_id(&builtin_string).expect("内建 String 必须可编");
    let builtin_bytes_id = mono_type_id(&builtin_bytes).expect("内建 Bytes 必须可编");

    // Assert 1：内建扁平身份拿 11/12，且读回恒等
    assert_eq!(builtin_string_id, 11, "内建 String 的类型号必须是 11");
    assert_eq!(builtin_bytes_id, 12, "内建 Bytes 的类型号必须是 12");
    assert_eq!(
        type_id_to_monotype(builtin_string_id),
        builtin_string,
        "内建 String 经 11 读回必须恒等"
    );
    assert_eq!(
        type_id_to_monotype(builtin_bytes_id),
        builtin_bytes,
        "内建 Bytes 经 12 读回必须恒等"
    );

    // Assert 2：非内建形态一律落折叠号 47，并读回既定 lossy 形态
    for (name, ty) in [
        ("模块作用域的 String", module_scoped_string),
        ("带参数的 String", parameterised_string),
        ("模块作用域的 Bytes", module_scoped_bytes),
        ("带参数的 Bytes", parameterised_bytes),
    ] {
        let id = mono_type_id(&ty).unwrap_or_else(|e| panic!("{name} 应折叠而不是拒绝：{e}"));
        assert_eq!(
            id, 47,
            "{name} 只能拿折叠号 47（拿 11/12 会把别的类型读回成内建 String/Bytes）"
        );
        assert_eq!(
            type_id_to_monotype(id),
            MonoType::TypeRef(Nominal::flat("_").expect("lossy 占位名必须可解析")),
            "{name}（号 {id}）读回必须是既定 lossy 形态 TypeRef(\"_\")"
        );
    }
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

/// 条款门：`.42` 可逆段只覆盖**内建** `String` / `Bytes`（平路径、零实参）。
///
/// 病史门：D64 之后 `Nominal` 携带模块身份；若用裸名比较，`m.String` 或带实参的
/// `String<T>` 会被静默编成 11/12，读回成内建无参 `String`——模块身份与实参双丢。
#[test]
fn test_string_and_bytes_codes_reject_non_builtin_identity() {
    // Arrange：内建、带模块、带实参三种同名 Generic
    let builtin = MonoType::Generic {
        name: Nominal::flat("String").expect("内建类型名必须是裸名且非空"),
        args: Vec::new(),
    };
    let namespaced = MonoType::Generic {
        name: Nominal::item("m", "String").expect("模块内名义名必须合法"),
        args: Vec::new(),
    };
    let parameterized = MonoType::Generic {
        name: Nominal::flat("String").expect("内建类型名必须是裸名且非空"),
        args: vec![MonoType::Int(64)],
    };

    // Act
    let builtin_code = mono_type_id(&builtin).expect("内建 String 必须可编");
    let namespaced_code = mono_type_id(&namespaced).expect("带模块的 String 按折叠类编码");
    let parameterized_code = mono_type_id(&parameterized).expect("带实参的 String 按折叠类编码");

    // Assert：只有内建拿 11；其余落折叠号 47 且读回 lossy 形态，不静默变成内建 String
    assert_eq!(builtin_code, 11, "内建 String 必须是历史号 11");
    assert_eq!(
        namespaced_code, 47,
        "带模块的 String 不得占用内建号 11，必须按折叠类落 47"
    );
    assert_eq!(
        parameterized_code, 47,
        "带实参的 String 不得占用内建号 11，必须按折叠类落 47"
    );
    assert!(
        matches!(type_id_to_monotype(namespaced_code), MonoType::TypeRef(_)),
        "折叠号读回必须是 lossy 形态 TypeRef，而不是被重建为内建 String"
    );

    // Bytes 行与 String 对称：内建拿 12，带模块/带实参一律落折叠号
    let builtin_bytes = MonoType::Generic {
        name: Nominal::flat("Bytes").expect("内建类型名必须是裸名且非空"),
        args: Vec::new(),
    };
    let namespaced_bytes = MonoType::Generic {
        name: Nominal::item("m", "Bytes").expect("模块内名义名必须合法"),
        args: Vec::new(),
    };
    let parameterized_bytes = MonoType::Generic {
        name: Nominal::flat("Bytes").expect("内建类型名必须是裸名且非空"),
        args: vec![MonoType::Int(64)],
    };
    assert_eq!(
        mono_type_id(&builtin_bytes).expect("内建 Bytes 必须可编"),
        12,
        "内建 Bytes 必须是历史号 12"
    );
    assert_eq!(
        mono_type_id(&namespaced_bytes).expect("带模块的 Bytes 按折叠类编码"),
        47,
        "带模块的 Bytes 不得占用内建号 12，必须按折叠类落 47"
    );
    assert_eq!(
        mono_type_id(&parameterized_bytes).expect("带实参的 Bytes 按折叠类编码"),
        47,
        "带实参的 Bytes 不得占用内建号 12，必须按折叠类落 47"
    );
}
