//! 6.9.5-B.2：声明期分类 → IR 量词传播与数据布局的直读判据。
//!
//! 规范锚：RFC-027 §14.1（space→position→category）、§14.6（擦除与数据索引）、
//! SPEC §8.3.2 / §8.3.6；09a §P6 6.9.5-B.2 契约 T27 / T28。
//!
//! 为什么走真实管线：手搓 TypeDeclBody 只能证明字段存在，证明不了
//! 「RecordModel → TypeCheckResult.record_models → TypeDeclBody」这条映射；
//! 而这条映射正是「登记-消费」型机制的消费端。

use crate::ast::Quantity;
use crate::frontend::core::lexer::tokenize;
use crate::frontend::core::parser::parse_with_probe;
use crate::frontend::core::sema::checker::TypeChecker;
use crate::frontend::core::sema::probe::TypeLevelNamespace;
use crate::frontend::core::sema::SemanticProbe;
use crate::frontend::module::registry::ModuleRegistry;
use crate::middle::ir::ir::{FunctionBody, Instruction, ModuleIR};
use crate::middle::ir::ir_gen::AstToIrGenerator;

/// 走真实四段管线产出模块 IR（类型检查必须无诊断）。
fn module_ir(source: &str) -> ModuleIR {
    let tokens = tokenize(source).expect("夹具必须能词法分析");
    let parsed = parse_with_probe(&tokens, &SemanticProbe);
    assert!(!parsed.has_errors, "夹具必须能解析");
    let mut checker = TypeChecker::new("<record-evidence-layout>");
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

/// Box 的 evidence 字段在中间，后面还有数据字段——用来钉「声明序 → 运行期索引」。
fn box_source() -> &'static str {
    concat!(
        "Pos: (v: Int) -> Type = { v > 0 }\n",
        "\n",
        "Box: Type = {\n",
        "    v: Int,\n",
        "    w: Pos(v),\n",
        "    z: Int,\n",
        "}\n",
        "\n",
        "main: () -> Void = {\n",
        "    b = Box(3, 5)\n",
        "    z = b.z\n",
        "}\n",
    )
}

/// T27：TypeDeclBody.fields[].quantity 来自记录模型（witness 在声明序 1）。
#[test]
fn test_type_decl_quantity_from_record_model() {
    // Arrange
    let source = box_source();

    // Act
    let module = module_ir(source);
    let fields = module
        .functions
        .iter()
        .find(|function| function.name == "Box")
        .and_then(|function| match &function.body {
            FunctionBody::TypeDecl { body } => Some(body.fields.clone()),
            _ => None,
        })
        .expect("Box 类型声明必须进 IR");

    // Assert
    assert_eq!(
        fields.len(),
        3,
        "Box 的三个字段必须全部进 IR（量词不删字段）"
    );
    assert_eq!(
        fields[0].quantity, None,
        "v 是数据字段；实际 {:?}",
        fields[0].quantity
    );
    assert_eq!(
        fields[1].quantity,
        Some(Quantity::Zero),
        "w: Pos(v) 是 evidence 字段，量词必须是 Some(Zero)；实际 {:?}",
        fields[1].quantity
    );
    assert_eq!(
        fields[2].quantity, None,
        "z 是数据字段；实际 {:?}",
        fields[2].quantity
    );
}

/// T28：运行期数据索引走唯一映射——b.z 的 LoadField.field 是 1（前移一位），
/// 且构造只装 2 个数据字段（witness 不占运行期槽）。
#[test]
fn test_runtime_field_index_skips_witness() {
    // Arrange
    let source = box_source();

    // Act
    let module = module_ir(source);
    let main = module
        .functions
        .iter()
        .find(|function| function.name == "main")
        .expect("main 必须进 IR");
    let blocks = match &main.body {
        FunctionBody::Code { blocks, .. } => blocks,
        _ => panic!("main 必须是代码块函数"),
    };
    let mut ctor_args: Option<usize> = None;
    let mut loaded_field: Option<usize> = None;
    for instruction in blocks.iter().flat_map(|block| block.instructions.iter()) {
        match instruction {
            Instruction::Call {
                func:
                    crate::middle::ir::ir::Operand::Const(crate::middle::ir::ir::ConstValue::String(
                        name,
                    )),
                args,
                ..
            } => {
                if name == "Box" {
                    ctor_args = Some(args.len());
                }
            }
            Instruction::LoadField { field, .. } => loaded_field = Some(*field),
            _ => {}
        }
    }

    // Assert
    assert_eq!(
        ctor_args,
        Some(2),
        "Box(3, 5) 的构造调用只装 2 个数据字段（witness 不占运行期槽）；实际 {:?}",
        ctor_args
    );
    assert_eq!(
        loaded_field,
        Some(1),
        "b.z 的声明序是 2，运行期字段索引必须是 1（witness 擦除后前移一位）"
    );
}

/// T28（端到端）：真实管线产出的 TypeDeclBody 交给 `synthesize_constructor`，
/// 构造函数的形状必须是「2 个数据字段 + 1 个结果槽」。
#[test]
fn test_synthesized_constructor_shape_from_real_pipeline() {
    // Arrange
    let source = box_source();
    let module = module_ir(source);

    // Act
    let output = crate::middle::passes::codegen::translator::Translator::new()
        .translate_module(&module)
        .expect("真实模块 IR 必须能翻译");
    let constructor = output
        .code_section
        .functions
        .into_iter()
        .find(|function| function.name == "Box")
        .expect("函数表里应有合成构造函数 Box");
    let load_args: Vec<_> = constructor
        .instructions
        .iter()
        .filter(|instruction| instruction.opcode == crate::middle::bytecode::opcode::LOAD_ARG)
        .collect();
    let create = constructor
        .instructions
        .iter()
        .find(|instruction| instruction.opcode == crate::middle::bytecode::opcode::CREATE_STRUCT)
        .expect("构造函数必须含 CreateStruct");

    // Assert
    assert_eq!(
        load_args.len(),
        2,
        "合成构造函数只装 2 个数据字段（witness 不占构造参数位），实际 {}",
        load_args.len()
    );
    assert_eq!(
        create.operands[5], 2,
        "CreateStruct 的字段计数只含数据字段，实际 {}",
        create.operands[5]
    );
    assert_eq!(
        constructor.local_count, 3,
        "局部槽 = 2 个数据字段 + 1 个结果槽（witness 不占槽），实际 {}",
        constructor.local_count
    );
}
