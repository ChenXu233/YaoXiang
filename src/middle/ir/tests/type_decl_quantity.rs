//! 6.9.5-B.1a / R5：IR 侧字段量词读数的直读判据。
//!
//! 判据锚：`TypeDeclField.quantity` 必须存在，且 B.1a 的所有构造点写 `None`
//!（witness 的产出入 B.2）。这里走真实管线（lex → parse → typecheck → ir_gen），
//! 而不是手搓 IR——手搓只能证明字段存在，证明不了映射本身。

use crate::frontend::core::lexer::tokenize;
use crate::frontend::core::parser::parse_with_probe;
use crate::frontend::core::sema::checker::TypeChecker;
use crate::frontend::core::sema::probe::TypeLevelNamespace;
use crate::frontend::core::sema::SemanticProbe;
use crate::frontend::module::registry::ModuleRegistry;
use crate::middle::ir::ir::{FunctionBody, ModuleIR};
use crate::middle::ir::ir_gen::AstToIrGenerator;

/// 走真实四段管线产出模块 IR（类型检查必须无诊断）。
fn module_ir(source: &str) -> ModuleIR {
    let tokens = tokenize(source).expect("夹具必须能词法分析");
    let parsed = parse_with_probe(&tokens, &SemanticProbe);
    assert!(!parsed.has_errors, "夹具必须能解析");
    let mut checker = TypeChecker::new("<type-decl-quantity>");
    let type_result = checker.check_module(
        &parsed.module,
        &TypeLevelNamespace::scan_one(&parsed.module),
    );
    assert!(
        type_result.diagnostics.is_empty(),
        "夹具必须无诊断；实际: {:?}",
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
        .expect("夹具必须能生成 IR")
}

/// 记录声明降级成 IR 后，字段的量词读数必须是 `None`（data 字段）。
#[test]
fn test_type_decl_field_quantity_is_none_from_definition() {
    // Arrange
    let source = concat!(
        "Point: Type = { x: Int, y: Int }\n",
        "\n",
        "main: () -> Void = {\n",
        "    p: Point = Point(x = 1, y = 2)\n",
        "}",
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
    assert!(
        !declared.is_empty(),
        "夹具必须产生至少一个类型声明的字段表（否则判据没有覆盖面）"
    );
    assert!(
        declared.iter().all(|field| field.quantity.is_none()),
        "B.1a 的所有 IR 构造点都写 None；实际: {:?}",
        declared
            .iter()
            .map(|f| (&f.name, f.quantity))
            .collect::<Vec<_>>()
    );
}
