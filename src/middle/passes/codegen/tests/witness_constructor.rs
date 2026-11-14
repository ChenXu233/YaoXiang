//! witness 字段的运行期布局 — 6.9.5-B.1b / H2（数据-only 构造）
//!
//! 规范锚：RFC-039 D62 / issue #536（B 路线，H0=B）；09a §P6 6.9.5-B 设计
//! §一.3（构造 arity 3B）与 §P6 6.9.5-B.1 契约 §五（T01/T02）。
//!
//! **为什么直驱构造 IR**：B.1a 保证 parser 与全部生产构造点都写
//! `quantity: None`，witness 的产出入 B.2——本批的生产入口不可达。故这里直接
//! 构造 `TypeDeclBody`（契约允许的直驱输入），验证
//! `translator::synthesize_constructor` 的两条性质：
//! 1. witness 字段不占构造参数位（`Box(v0)`，不是 `Box(v0, w0)`）；
//! 2. 声明序 → 运行期数据索引走唯一映射（`w` 之后的字段索引前移一位）。
//!
//! 判据锚：`Box(v, w, z)`（`w` 为 witness）⇒ 2 个 `Load`、2 个
//! `CreateStruct.fields`、`z` 的 operand 位置 = 1。

use crate::ast::Quantity;
use crate::frontend::core::types::MonoType;
use crate::middle::bytecode::file::FunctionCode;
use crate::middle::bytecode::opcode;
use crate::middle::ir::ir::{FunctionBody, FunctionIR, ModuleIR, TypeDeclBody, TypeDeclField};
use crate::middle::passes::codegen::translator::Translator;

/// 一个记录字段：名字 + 量词读数（`Some(Zero)` = witness）。
fn field(
    name: &str,
    quantity: Option<Quantity>,
) -> TypeDeclField {
    TypeDeclField {
        name: name.to_string(),
        ty: MonoType::Int(64),
        is_mut: false,
        has_default: false,
        quantity,
        is_method: false,
    }
}

/// 只含一个类型声明的模块 IR（类型名 `Box`）。
fn module_with_type_decl(fields: Vec<TypeDeclField>) -> ModuleIR {
    ModuleIR {
        functions: vec![FunctionIR {
            name: "Box".to_string(),
            def: None,
            params: Vec::new(),
            return_type: MonoType::TypeRef("Box".to_string()),
            body: FunctionBody::TypeDecl {
                body: TypeDeclBody {
                    fields,
                    self_type: MonoType::TypeRef("Box".to_string()),
                },
            },
            generic_params: None,
        }],
        ..ModuleIR::default()
    }
}

/// 合成构造函数并返回它的字节码。
fn constructor_code(fields: Vec<TypeDeclField>) -> FunctionCode {
    let module = module_with_type_decl(fields);
    let output = Translator::new()
        .translate_module(&module)
        .expect("类型声明的构造合成不应失败");
    output
        .code_section
        .functions
        .into_iter()
        .find(|function| function.name == "Box")
        .expect("函数表里应有合成构造函数 Box")
}

/// 取指定 opcode 的指令（按出现顺序）。
fn instructions_with_opcode(
    code: &FunctionCode,
    wanted: u8,
) -> Vec<&crate::middle::bytecode::file::BytecodeInstruction> {
    code.instructions
        .iter()
        .filter(|instruction| instruction.opcode == wanted)
        .collect()
}

/// 取 `CreateStruct` 的结果寄存器（dst 是结果槽；witness 不得占槽）。
fn create_struct_dst(code: &FunctionCode) -> u8 {
    code.instructions
        .iter()
        .find(|instruction| instruction.opcode == opcode::CREATE_STRUCT)
        .map(|instruction| instruction.operands[0])
        .expect("构造函数必须含 CreateStruct")
}

/// 解码 `CreateStruct`：`dst(1) + type_name_idx(4) + field_count(1) + fields(2×count)`。
fn create_struct_fields(code: &FunctionCode) -> (u8, Vec<u16>) {
    let instruction = code
        .instructions
        .iter()
        .find(|instruction| instruction.opcode == opcode::CREATE_STRUCT)
        .expect("构造函数必须含 CreateStruct");
    let declared_count = instruction.operands[5];
    let registers = instruction.operands[6..]
        .as_chunks::<2>()
        .0
        .iter()
        .map(|pair| u16::from_le_bytes(*pair))
        .collect();
    (declared_count, registers)
}

/// T01：witness 不占构造参数位——`Box(v, w)`（`w` 为 witness）只装 1 个数据字段。
#[test]
fn test_synthesize_constructor_skips_witness_field() {
    // Arrange
    let fields = vec![field("v", None), field("w", Some(Quantity::Zero))];

    // Act
    let code = constructor_code(fields);

    // Assert
    let loads = instructions_with_opcode(&code, opcode::LOAD_ARG);
    assert_eq!(
        loads.len(),
        1,
        "witness 字段不得占构造参数位：只应有 1 个 LOAD_ARG，实际 {} 个",
        loads.len()
    );
    assert_eq!(
        loads[0].operands,
        vec![0, 0],
        "唯一数据字段 v 的 Load 应是 dst=0 / arg=0"
    );
    let (declared_count, registers) = create_struct_fields(&code);
    assert_eq!(declared_count, 1, "CreateStruct 的字段计数只含数据字段");
    assert_eq!(registers, vec![0], "CreateStruct 只装数据字段的槽位");
    assert_eq!(
        code.local_count, 2,
        "局部槽 = 1 个数据字段 + 1 个结果槽；witness 不得占槽"
    );
    assert_eq!(
        create_struct_dst(&code),
        1,
        "结果槽 = 数据字段数；若把 witness 计入布局，dst 会变成 2"
    );
}

/// T01 + T02：三字段 `Box(v, w, z)`（`w` 为 witness）⇒ 2 个 Load / 2 个 operand，
/// 且 `z` 的声明序 2 映射到运行期槽 1。
#[test]
fn test_synthesize_constructor_maps_third_field_to_runtime_index() {
    // Arrange
    let fields = vec![
        field("v", None),
        field("w", Some(Quantity::Zero)),
        field("z", None),
    ];

    // Act
    let code = constructor_code(fields);

    // Assert
    let loads = instructions_with_opcode(&code, opcode::LOAD_ARG);
    assert_eq!(
        loads.len(),
        2,
        "两个数据字段应各有一个 LOAD_ARG（witness 不产生 Load），实际 {} 个",
        loads.len()
    );
    assert_eq!(
        loads[1].operands,
        vec![1, 1],
        "z 的声明序是 2，但运行期 operand 位置必须是 1（前移一位）"
    );
    let (declared_count, registers) = create_struct_fields(&code);
    assert_eq!(declared_count, 2, "CreateStruct 的字段计数只含数据字段");
    assert_eq!(
        registers,
        vec![0, 1],
        "CreateStruct 的 operand 应是运行期槽 0、1"
    );
    assert_eq!(code.local_count, 3, "局部槽 = 2 个数据字段 + 1 个结果槽");
    assert_eq!(
        create_struct_dst(&code),
        2,
        "结果槽 = 数据字段数（2）；witness 计入布局时这里会变成 3"
    );
}

/// T12（IR 面）：无 witness 的记录构造保持恒等——三个 Load、三个 operand、槽位不变。
#[test]
fn test_synthesize_constructor_keeps_all_data_fields() {
    // Arrange
    let fields = vec![field("v", None), field("w", None), field("z", None)];

    // Act
    let code = constructor_code(fields);

    // Assert
    let loads = instructions_with_opcode(&code, opcode::LOAD_ARG);
    assert_eq!(loads.len(), 3, "无 witness 时每个字段都应占一个构造参数位");
    assert_eq!(loads[0].operands, vec![0, 0], "字段 v 的槽位保持 0");
    assert_eq!(loads[1].operands, vec![1, 1], "字段 w 的槽位保持 1");
    assert_eq!(loads[2].operands, vec![2, 2], "字段 z 的槽位保持 2");
    let (declared_count, registers) = create_struct_fields(&code);
    assert_eq!(declared_count, 3, "普通记录的 CreateStruct 字段数不变");
    assert_eq!(
        registers,
        vec![0, 1, 2],
        "普通记录的 CreateStruct operand 不变"
    );
    assert_eq!(code.local_count, 4, "局部槽 = 3 个字段 + 1 个结果槽");
    assert_eq!(
        create_struct_dst(&code),
        3,
        "无 witness 时结果槽 = 字段数 3"
    );
}
