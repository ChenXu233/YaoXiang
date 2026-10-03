//! RFC-011a §6 存在类型变体指令——运行时守卫与序列化往返测试
//!
//! 覆盖:
//! - Vec(Animal) 变体分发经字节码序列化往返后仍正确执行（CREATE_VARIANT /
//!   VARIANT_TAG / VARIANT_PAYLOAD 编解码链路）
//! - VariantTag 收到未包装值时显式报错（四层防御第③层：包装点遗漏在运行时
//!   响亮暴露，绝不静默产出错误数据）
//!
//! 值正确性（分发结果 == Woof/Meow）由 E2E interface_dynamic_dispatch.yx 覆盖。

use crate::middle::bytecode::{BytecodeFunction, BytecodeInstr, BytecodeModule, ConstValue, Reg};
use crate::middle::core::ir::Type;
use std::collections::HashMap;

/// Vec(Animal) 异构容器 + 变体分发，经字节码落盘/加载往返后执行正确。
/// Helper: test_variant_tag_guard_rejects_unwrapped_value 的完整夹具与断言（逐条断言见函数体）。
/// Fixture: `main` 对未包装的 Int(42) 执行 VariantTag 的模块（守卫应拒绝）。
fn unwrapped_variant_tag_module() -> BytecodeModule {
    BytecodeModule {
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
            local_names: HashMap::new(),
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
        init_function: None,
        debug_sources: None,
        // #368：本用例不涉及索引越界的变量名回溯，空表即可
        global_names: std::collections::HashMap::new(),
    }
}

/// Fixture + Act：Animal/Dog/Cat 动态分派项目 → 编译 → 落盘 → 反序列化加载。
fn compile_dispatch_project(dir: &std::path::Path) -> crate::middle::passes::codegen::BytecodeFile {
    let source_path = dir.join("dispatch.yx");
    let bytecode_path = dir.join("dispatch.42");
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

main = () => {
    animals: Vec(Animal) = [Dog("Rex"), Cat(9)]
    print(animals[0].speak())
    print(animals[1].speak())
}
"#,
    )
    .expect("write source file");
    crate::build_bytecode_with_options(&source_path, &bytecode_path, false)
        .expect("build bytecode");
    crate::middle::passes::codegen::BytecodeFile::load(&bytecode_path).expect("load bytecode file")
}

/// Assert：编码侧仍含 CREATE_VARIANT（编解码链路完整）。
fn assert_variant_opcode_survives_encode(
    bytecode_file: &crate::middle::passes::codegen::BytecodeFile
) {
    let has_variant_instr = bytecode_file.code_section.functions.iter().any(|f| {
        f.instructions
            .iter()
            .any(|i| i.opcode == crate::backends::common::opcode::CREATE_VARIANT)
    });
    assert!(has_variant_instr, "CREATE_VARIANT should survive encode");
}

/// Assert：解码侧还原 CreateVariant 与 VariantTag 指令。
fn assert_variant_instrs_decode_back(bytecode_module: &BytecodeModule) {
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
}

#[test]
fn test_variant_dispatch_roundtrip() {
    // Arrange & Act：Animal/Dog/Cat 动态分派项目 → 编译 → 落盘 → 反序列化
    let dir = tempfile::TempDir::new().expect("create temp dir");
    let bytecode_file = compile_dispatch_project(dir.path());

    // Assert：往返后变体指令仍在（编码侧）
    assert_variant_opcode_survives_encode(&bytecode_file);

    // Act：解码回模块，检查解码侧指令
    let bytecode_module = BytecodeModule::from(bytecode_file);

    // Assert：CreateVariant / VariantTag 均可解码回来
    assert_variant_instrs_decode_back(&bytecode_module);

    // Act & Assert：vtable 分派经解释器执行成功
    let interp = crate::backends::interpreter::Interpreter::new();
    let mut executor: Box<dyn crate::backends::Executor> = Box::new(interp);
    executor
        .execute_module(&bytecode_module)
        .expect("variant dispatch should execute after roundtrip");
}

/// VariantTag 收到未包装值（Int）→ 运行时错误（不静默），错误信息带期望组名。
#[test]
fn test_variant_tag_guard_rejects_unwrapped_value() {
    // Arrange：常量池 0=组名、1=未包装 Int(42)；main 加载后对 local0 打 VariantTag
    let module = unwrapped_variant_tag_module();

    // Act：执行应失败（守卫必须响亮拒绝）
    let interp = crate::backends::interpreter::Interpreter::new();
    let mut executor: Box<dyn crate::backends::Executor> = Box::new(interp);
    let err = executor
        .execute_module(&module)
        .expect_err("VariantTag on unwrapped value must fail loudly");

    // Assert：错误信息点名期望的组名
    assert!(
        err.to_string().contains("Animal$Group"),
        "error should name the expected group, got: {}",
        err
    );
}
