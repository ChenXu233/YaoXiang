//! witness-bearing 记录的值级拒绝与整体不可变 — 6.9.5-B.1b / B.1c（5A）
//!
//! 规范锚：RFC-039 D62 / issue #536（B 路线，H0=B；4A+5A 裁决）；09a
//! §P6 6.9.5-B 设计 §一.5（投影 4A）与 §一.6（可变性 5A）、§P6 6.9.5-B.1
//! 契约 §五（T03/T04/T07–T12）。
//!
//! **为什么直驱构造 AST**：B.1a 保证 parser 与全部生产构造点都写
//! `quantity: None`，witness 的产出入 B.2——本批 E1114/E1115 的用户可达性
//! 留 B.2。故这里 parse 后把指定字段标成 witness（契约允许的直驱输入），再走
//! 真实的 `TypeChecker`：
//! - E1114：值级 `b.w`（witness 字段无运行期数据槽，读取无意义）；
//! - E1115：witness-bearing 记录整体不可变——`mut` 字段声明 / 字段赋值 /
//!   `&mut` 借用 / 需要写借用的方法接收者四类写入点。
//!
//! 每个正向判据都配同形负控（不标 witness 时零 E1114/E1115），防止「把普通
//! 记录也一并拒掉」被误当成通过。

use crate::ast::{Module, Quantity, StmtKind, Type, TypeBodyItem};
use crate::frontend::core::lexer::tokenize;
use crate::frontend::core::parser::parse;
use crate::frontend::core::sema::checker::TypeChecker;
use crate::frontend::core::sema::probe::TypeLevelNamespace;
use crate::frontend::core::sema::TypeCheckResult;

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

/// 解析源码 → （可选）标记 witness 字段 → 真实类型检查（收集模式）。
fn check(
    source: &str,
    witness_field: Option<(&str, &str)>,
) -> TypeCheckResult {
    let tokens = tokenize(source).expect("夹具必须能词法分析");
    let mut parsed = parse(&tokens);
    assert!(
        !parsed.has_errors,
        "夹具必须能解析；错误: {:?}",
        parsed.errors
    );
    if let Some((type_name, field_name)) = witness_field {
        assert!(
            mark_field_witness(&mut parsed.module, type_name, field_name),
            "夹具里必须有 {type_name}.{field_name} 字段可供标成 witness"
        );
    }
    let mut checker = TypeChecker::new("witness-fields");
    checker.check_module_collect_all(
        &parsed.module,
        &TypeLevelNamespace::scan_one(&parsed.module),
    )
}

/// 诊断集里是否有指定码。
fn has_code(
    result: &TypeCheckResult,
    code: &str,
) -> bool {
    result.diagnostics.iter().any(|diag| diag.code == code)
}

/// 诊断码列表（断言失败时附在消息里）。
fn codes_of(result: &TypeCheckResult) -> Vec<&str> {
    result
        .diagnostics
        .iter()
        .map(|diag| diag.code.as_str())
        .collect()
}

/// T03：witness-bearing 记录的**数据字段**读取照常放行（E1114 只针对 witness 字段）。
#[test]
fn test_witness_record_data_field_read_is_allowed() {
    // Arrange
    let source = concat!(
        "Box: Type = { v: Int, w: Int }\n",
        "\n",
        "main: () -> Void = {\n",
        "    b: Box = Box(v = 1, w = 2)\n",
        "    x: Int = b.v\n",
        "}\n",
    );

    // Act
    let result = check(source, Some(("Box", "w")));

    // Assert
    assert!(
        !has_code(&result, "E1114") && !has_code(&result, "E1115"),
        "数据字段读取不得被 witness 规则误伤；实际诊断: {:?}",
        codes_of(&result)
    );
}

/// T04：值级 `b.w` 命中 witness 字段 ⇒ E1114。
#[test]
fn test_witness_field_value_read_is_rejected() {
    // Arrange
    let source = concat!(
        "Box: Type = { v: Int, w: Int }\n",
        "\n",
        "main: () -> Void = {\n",
        "    b: Box = Box(v = 1, w = 2)\n",
        "    x: Int = b.w\n",
        "}\n",
    );

    // Act
    let result = check(source, Some(("Box", "w")));

    // Assert
    assert!(
        has_code(&result, "E1114"),
        "值级读取 witness 字段必须报 E1114；实际诊断: {:?}",
        codes_of(&result)
    );
}

/// T07：声明面 `mut` 字段 + witness ⇒ E1115（记录整体不可变）。
#[test]
fn test_witness_record_mut_field_declaration_is_rejected() {
    // Arrange — `v mut: Int` 是字段可变性语法位
    let source = concat!(
        "Box: Type = { v mut: Int, w: Int }\n",
        "\n",
        "main: () -> Void = {\n",
        "    b: Box = Box(v = 1, w = 2)\n",
        "}\n",
    );

    // Act
    let result = check(source, Some(("Box", "w")));

    // Assert
    assert!(
        has_code(&result, "E1115"),
        "witness-bearing 记录的 mut 字段声明必须报 E1115；实际诊断: {:?}",
        codes_of(&result)
    );
}

/// T08 负控：同样的 `mut` 字段声明，不带 witness 时必须照常放行。
#[test]
fn test_plain_record_mut_field_declaration_is_allowed() {
    // Arrange
    let source = concat!(
        "Box: Type = { v mut: Int, w: Int }\n",
        "\n",
        "main: () -> Void = {\n",
        "    b: Box = Box(v = 1, w = 2)\n",
        "}\n",
    );

    // Act
    let result = check(source, None);

    // Assert
    assert!(
        !has_code(&result, "E1115"),
        "普通记录的 mut 字段是既有合法形态，不得被 E1115 误伤；实际诊断: {:?}",
        codes_of(&result)
    );
}

/// T09：对 witness-bearing 记录做字段赋值 ⇒ E1115。
#[test]
fn test_witness_record_field_assignment_is_rejected() {
    // Arrange
    let source = concat!(
        "Box: Type = { v: Int, w: Int }\n",
        "\n",
        "main: () -> Void = {\n",
        "    b: Box = Box(v = 1, w = 2)\n",
        "    b.v = 3\n",
        "}\n",
    );

    // Act
    let result = check(source, Some(("Box", "w")));

    // Assert
    assert!(
        has_code(&result, "E1115"),
        "witness-bearing 记录的字段赋值必须报 E1115；实际诊断: {:?}",
        codes_of(&result)
    );
}

/// T09 负控：普通记录的字段赋值照常放行。
#[test]
fn test_plain_record_field_assignment_is_allowed() {
    // Arrange
    let source = concat!(
        "Box: Type = { v: Int, w: Int }\n",
        "\n",
        "main: () -> Void = {\n",
        "    b: Box = Box(v = 1, w = 2)\n",
        "    b.v = 3\n",
        "}\n",
    );

    // Act
    let result = check(source, None);

    // Assert
    assert!(
        !has_code(&result, "E1115"),
        "普通记录的字段赋值是既有合法形态；实际诊断: {:?}",
        codes_of(&result)
    );
}

/// T10：对 witness-bearing 记录取 `&mut` ⇒ E1115。
#[test]
fn test_witness_record_mutable_borrow_is_rejected() {
    // Arrange
    let source = concat!(
        "Box: Type = { v: Int, w: Int }\n",
        "\n",
        "main: () -> Void = {\n",
        "    mut b: Box = Box(v = 1, w = 2)\n",
        "    r: &mut Box = &mut b\n",
        "}\n",
    );

    // Act
    let result = check(source, Some(("Box", "w")));

    // Assert
    assert!(
        has_code(&result, "E1115"),
        "witness-bearing 记录的 &mut 借用必须报 E1115；实际诊断: {:?}",
        codes_of(&result)
    );
}

/// T11 负控：普通记录的 `&mut` 借用照常放行。
#[test]
fn test_plain_record_mutable_borrow_is_allowed() {
    // Arrange
    let source = concat!(
        "Box: Type = { v: Int, w: Int }\n",
        "\n",
        "main: () -> Void = {\n",
        "    mut b: Box = Box(v = 1, w = 2)\n",
        "    r: &mut Box = &mut b\n",
        "}\n",
    );

    // Act
    let result = check(source, None);

    // Assert
    assert!(
        !has_code(&result, "E1115"),
        "普通记录的 &mut 借用是既有合法形态；实际诊断: {:?}",
        codes_of(&result)
    );
}

/// T11（方法接收者面）：`b.shift()` 的 `&mut self` 接收者命中 witness ⇒ E1115。
///
/// 这是 5A 的自动 `WriteBorrow` 路径：调用点 `co.receiver == WriteBorrow`，
/// 接收者静态类型 witness-bearing 即拒绝（用户不写 `&mut`）。
#[test]
fn test_witness_record_method_receiver_write_borrow_is_rejected() {
    // Arrange
    let source = concat!(
        "Box: Type = { v: Int, w: Int }\n",
        "Box.shift: (self: &mut Box) -> Void = {}\n",
        "\n",
        "main: () -> Void = {\n",
        "    b: Box = Box(v = 1, w = 2)\n",
        "    b.shift()\n",
        "}\n",
    );

    // Act
    let result = check(source, Some(("Box", "w")));

    // Assert
    assert!(
        has_code(&result, "E1115"),
        "witness-bearing 记录的 &mut self 方法接收者必须报 E1115；实际诊断: {:?}",
        codes_of(&result)
    );
}

/// T12：同形普通记录（无 witness）的四类写入点全部照常放行。
#[test]
fn test_plain_record_keeps_all_write_forms_allowed() {
    // Arrange
    let source = concat!(
        "Box: Type = { v mut: Int, w: Int }\n",
        "Box.shift: (self: &mut Box) -> Void = {}\n",
        "\n",
        "main: () -> Void = {\n",
        "    mut b: Box = Box(v = 1, w = 2)\n",
        "    b.v = 3\n",
        "    r: &mut Box = &mut b\n",
        "    b.shift()\n",
        "}\n",
    );

    // Act
    let result = check(source, None);

    // Assert
    assert!(
        !has_code(&result, "E1114") && !has_code(&result, "E1115"),
        "普通记录的四类写入点都是既有合法形态，E1114/E1115 不得误伤；实际诊断: {:?}",
        codes_of(&result)
    );
}
