//! 6.9.5-B.1b / T05：`TypeDeclField.quantity` 的直读判据（AST → IR）
//!
//! 规范锚：RFC-039 D62 / issue #536（B 路线，H0=B）；09a §P6 6.9.5-B.1
//! 契约 §五 T05（IR quantity 直读）。
//!
//! **为什么直驱构造 AST**：B.1a 保证 parser 产出 `quantity: None`，witness 的
//! 产出入 B.2——用户源码写不出 witness 字段。故这里 parse 后把指定字段的
//! `quantity` 改成 `Some(Zero)`（契约允许的直驱输入），再走真实的
//! typecheck → ir_gen，判据是 IR 载体原样携带该事实（不是测试脚本自己读回）。

use crate::ast::{Module, Quantity, StmtKind, Type, TypeBodyItem};
use crate::frontend::core::lexer::tokenize;
use crate::frontend::core::parser::parse_with_probe;
use crate::frontend::core::sema::checker::TypeChecker;
use crate::frontend::core::sema::probe::TypeLevelNamespace;
use crate::frontend::core::sema::SemanticProbe;
use crate::frontend::module::registry::ModuleRegistry;
use crate::middle::ir::ir::{FunctionBody, ModuleIR};
use crate::middle::ir::ir_gen::AstToIrGenerator;

/// 把 `module.items` 里指定类型定义的指定字段标成 witness（q=0）。
fn mark_field_witness(
    module: &mut Module,
    type_name: &str,
    field_name: &str,
) -> bool {
    for stmt in &mut module.items {
        let StmtKind::TypeDefinition {
            name, definition, ..
        } = &mut stmt.kind
        else {
            continue;
        };
        if name != type_name {
            continue;
        }
        let Type::Struct { body } = definition else {
            continue;
        };
        for item in body {
            if let TypeBodyItem::Field(field) = item {
                if field.name == field_name {
                    field.quantity = Some(Quantity::Zero);
                    return true;
                }
            }
        }
    }
    false
}

/// 走真实四段管线产出模块 IR（类型检查必须无诊断）。
fn module_ir(source: &str) -> ModuleIR {
    let tokens = tokenize(source).expect("夹具必须能词法分析");
    let mut parsed = parse_with_probe(&tokens, &SemanticProbe);
    assert!(!parsed.has_errors, "夹具必须能解析");
    assert!(
        mark_field_witness(&mut parsed.module, "Box", "w"),
        "夹具里必须有 Box.w 字段可供标成 witness"
    );
    let mut checker = TypeChecker::new("<witness-quantity>");
    let type_result = checker.check_module(
        &parsed.module,
        &TypeLevelNamespace::scan_one(&parsed.module),
    );
    assert!(
        type_result.diagnostics.is_empty(),
        "直驱夹具必须无诊断；实际: {:?}",
        type_result.diagnostics
    );
    let mut generator = AstToIrGenerator::new_with_type_result(
        &type_result,
        ModuleRegistry::with_std(),
        None,
        TypeLevelNamespace::scan_one(&parsed.module),
    );
    generator
        .generate_module_ir(&parsed.module)
        .expect("直驱夹具必须能生成 IR")
}

/// T02（ir_gen 面）：字段访问的 `LoadField` 必须用运行期数据索引，
/// 不是声明序下标——`w` 之后的 `z` 声明序 2 必须映射为槽 1。
#[test]
fn test_field_access_uses_runtime_data_index() {
    // Arrange
    let source = concat!(
        "Box: Type = { v: Int, w: Int, z: Int }\n",
        "\n",
        "main: () -> Void = {\n",
        "    b: Box = Box(v = 1, w = 2, z = 3)\n",
        "    x: Int = b.z\n",
        "    y: Int = b.v\n",
        "}\n",
    );

    // Act
    let module = module_ir(source);
    let loaded_fields: Vec<usize> = module
        .functions
        .iter()
        .flat_map(|function| match &function.body {
            FunctionBody::Code { blocks, .. } => blocks
                .iter()
                .flat_map(|block| block.instructions.iter())
                .filter_map(|instruction| match instruction {
                    crate::middle::ir::ir::Instruction::LoadField { field, .. } => Some(*field),
                    _ => None,
                })
                .collect::<Vec<_>>(),
            _ => Vec::new(),
        })
        .collect();

    // Assert
    assert!(
        loaded_fields.contains(&0),
        "数据字段 v 的运行期索引必须仍是 0；实际 LoadField: {loaded_fields:?}"
    );
    assert!(
        loaded_fields.contains(&1),
        "z 的声明序是 2，运行期索引必须是 1（跳过 witness）；实际 LoadField: {loaded_fields:?}"
    );
    assert!(
        !loaded_fields.contains(&2),
        "声明序下标不得泄漏进 IR（witness 之后的字段必须前移）；实际 LoadField: {loaded_fields:?}"
    );
}

/// witness 字段的量词读数必须从 AST 原样进 IR 载体，其余字段保持 `None`。
#[test]
fn test_type_decl_field_quantity_is_read_directly_from_ast() {
    // Arrange — Box 的 w 字段在直驱侧标成 witness
    let source = concat!(
        "Box: Type = { v: Int, w: Int, z: Int }\n",
        "\n",
        "main: () -> Void = {\n",
        "    b: Box = Box(v = 1, w = 2, z = 3)\n",
        "    _ = b.z\n",
        "}\n",
    );

    // Act
    let module = module_ir(source);
    let declared: Vec<_> = module
        .functions
        .iter()
        .filter_map(|function| match &function.body {
            FunctionBody::TypeDecl { body } => Some(body.fields.clone()),
            _ => None,
        })
        .flatten()
        .collect();

    // Assert
    assert_eq!(
        declared.len(),
        3,
        "Box 的 IR 载体必须带三个字段；实际: {:?}",
        declared.iter().map(|f| &f.name).collect::<Vec<_>>()
    );
    assert_eq!(
        declared[0].quantity, None,
        "data 字段 v 的 IR 量词读数必须是 None"
    );
    assert_eq!(
        declared[1].quantity,
        Some(Quantity::Zero),
        "witness 字段 w 的 IR 量词读数必须是 Some(Zero)（B.1b 的唯一输入事实）"
    );
    assert_eq!(
        declared[2].quantity, None,
        "witness 之后的 data 字段 z 仍必须是 None"
    );
}
