//! RFC-011a §6 存在类型变体指令——运行时守卫与序列化往返测试
//!
//! 覆盖:
//! - List(Animal) 变体分发经字节码序列化往返后仍正确执行（CREATE_VARIANT /
//!   VARIANT_TAG / VARIANT_PAYLOAD 编解码链路）
//! - VariantTag 收到未包装值时显式报错（四层防御第③层：包装点遗漏在运行时
//!   响亮暴露，绝不静默产出错误数据）
//!
//! 值正确性（分发结果 == Woof/Meow）由 E2E interface_dynamic_dispatch.yx 覆盖。

use crate::middle::bytecode::{BytecodeFunction, BytecodeInstr, BytecodeModule, ConstValue, Reg};
use crate::middle::core::ir::Type;
use std::collections::HashMap;

/// List(Animal) 异构容器 + 变体分发，经字节码落盘/加载往返后执行正确。
#[test]
fn test_variant_dispatch_roundtrip() {
    let dir = tempfile::TempDir::new().expect("create temp dir");
    let source_path = dir.path().join("dispatch.yx");
    let bytecode_path = dir.path().join("dispatch.42");
    std::fs::write(
        &source_path,
        r#"
Animal: (Self: Type) -> Type = {
    speak: (self: Self) -> String,
}
Dog: Type = {
    name: String,
    Animal(Dog),
}
Dog.speak: (self: Dog) -> String = { return "Woof" }
Cat: Type = {
    lives: Int,
    Animal(Cat),
}
Cat.speak: (self: Cat) -> String = { return "Meow" }

main = {
    animals: List(Animal) = [Dog("Rex"), Cat(9)]
    print(animals[0].speak())
    print(animals[1].speak())
}
"#,
    )
    .expect("write source file");

    crate::build_bytecode_with_options(&source_path, &bytecode_path, false)
        .expect("build bytecode");
    let bytecode_file = crate::middle::passes::codegen::BytecodeFile::load(&bytecode_path)
        .expect("load bytecode file");

    // Assert：往返后变体指令仍在（编解码链路完整）
    let has_variant_instr = bytecode_file.code_section.functions.iter().any(|f| {
        f.instructions
            .iter()
            .any(|i| i.opcode == crate::backends::common::opcode::CREATE_VARIANT)
    });
    assert!(has_variant_instr, "CREATE_VARIANT should survive encode");

    let bytecode_module = BytecodeModule::from(bytecode_file);
    assert!(
        bytecode_module.functions.iter().any(|f| f
            .instructions
            .iter()
            .any(|i| matches!(i, BytecodeInstr::CreateVariant { .. }))),
        "CREATE_VARIANT should decode back"
    );
    assert!(
        bytecode_module.functions.iter().any(|f| f
            .instructions
            .iter()
            .any(|i| matches!(i, BytecodeInstr::VariantTag { .. }))),
        "VARIANT_TAG should decode back"
    );

    let interp = crate::backends::interpreter::Interpreter::new();
    let mut executor: Box<dyn crate::backends::Executor> = Box::new(interp);
    executor
        .execute_module(&bytecode_module)
        .expect("variant dispatch should execute after roundtrip");
}

/// VariantTag 收到未包装值（Int）→ 运行时错误（不静默），错误信息带期望组名。
#[test]
fn test_variant_tag_guard_rejects_unwrapped_value() {
    let module = BytecodeModule {
        name: "guard_test".to_string(),
        // 常量池: 0 = 组名（守卫错误信息用），1 = 未包装的 Int(42)
        constants: vec![
            ConstValue::String("Animal$Group".to_string()),
            ConstValue::Int(42),
        ],
        functions: vec![BytecodeFunction {
            name: "main".to_string(),
            params: vec![],
            return_type: Type::Void,
            local_count: 4,
            upvalue_count: 0,
            instructions: vec![
                // local0 = Int(42)（未包装的具体值）
                BytecodeInstr::LoadConst {
                    dst: Reg(0),
                    const_idx: 1,
                },
                // local1 = VariantTag(local0)——应报错
                BytecodeInstr::VariantTag {
                    dst: Reg(1),
                    obj: Reg(0),
                    group_idx: 0,
                },
            ],
            labels: HashMap::new(),
            exception_handlers: Vec::new(),
            debug_map: HashMap::new(),
        }],
        type_table: vec![],
        vtables: vec![],
        globals: vec![],
        entry_point: Some(0),
    };

    let interp = crate::backends::interpreter::Interpreter::new();
    let mut executor: Box<dyn crate::backends::Executor> = Box::new(interp);
    let err = executor
        .execute_module(&module)
        .expect_err("VariantTag on unwrapped value must fail loudly");
    assert!(
        err.to_string().contains("Animal$Group"),
        "error should name the expected group, got: {}",
        err
    );
}
