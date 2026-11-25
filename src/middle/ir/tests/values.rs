//! 7b.2 值表判据：`next_temp_reg` 改产 `Operand::Value` 后的生产面。
//!
//! 覆盖：值表与槽映射同索引、真实类型（字面量 / 比较）、`defining_block`、
//! `debug_name` 恒 `None`、指令里的 ValueId 可索引；以及 normalize 的
//! `values_total` / `value` 段渲染（空表与非空表两种形态）。

use crate::frontend::compiler::Compiler;
use crate::frontend::core::sema::MonoType;
use crate::middle::ir::ir::{ConstValue, FunctionBody, Instruction, ModuleIR, Operand};
use crate::middle::ir::normalize::normalize_module;
use crate::util::span::Span;

use super::ir_build::{block, func, func_with_blocks, module_with, ssa_value, value_info};

fn compile(source: &str) -> ModuleIR {
    Compiler::new()
        .compile("values_probe.yx", source)
        .expect("夹具必须通过生产入口编译")
}

fn ret_void() -> Instruction {
    Instruction::Ret {
        value: None,
        span: Span::dummy(),
    }
}

/// 值表生产：ValueId 稠密可索引、槽映射同长、类型真实、定义块为真实 label、debug_name=None。
#[test]
fn test_values_production_type_block_and_index() {
    // Arrange：f 的比较结果落 Bool 值；main 的字面量落 Int(64) 值。
    let source = r#"
f: (x: Int) -> Bool = {
    x == 1
}

main: () -> Void = {
    b = 1 + 2
    c = f(b)
}

main()
"#;
    let ir = compile(source);

    // Act：逐函数收集值表事实
    let mut total_values = 0usize;
    let mut saw_bool = false;
    let mut saw_int = false;
    for function in &ir.functions {
        let FunctionBody::Code { blocks, values, .. } = &function.body else {
            continue;
        };
        if values.is_empty() {
            continue;
        }
        total_values += values.len();
        assert_eq!(
            function.value_slots().len(),
            values.len(),
            "{}: value_slots 必须与 values 同索引同长",
            function.name
        );
        let block_labels: std::collections::HashSet<usize> =
            blocks.iter().map(|block| block.label).collect();
        for info in values {
            assert!(
                block_labels.contains(&info.defining_block),
                "{}: defining_block({}) 必须是真实块 label 之一 {:?}",
                function.name,
                info.defining_block,
                block_labels
            );
            assert!(
                info.debug_name.is_none(),
                "{}: 纯临时值的 debug_name 必须是 None",
                function.name
            );
            if matches!(info.ty, MonoType::Bool) {
                saw_bool = true;
            }
            if matches!(info.ty, MonoType::Int(64)) {
                saw_int = true;
            }
        }
        for block in blocks {
            for instr in &block.instructions {
                let mut probe = instr.clone();
                probe.for_each_operand_mut(&mut |op| {
                    if let Operand::Value(id) = op {
                        assert!(
                            (id.0 as usize) < values.len(),
                            "{}: 指令里的 ValueId({}) 必须落在值表内（len={}）",
                            function.name,
                            id.0,
                            values.len()
                        );
                        if instr
                            .def_operands()
                            .iter()
                            .any(|def| matches!(def, Operand::Value(def_id) if def_id == id))
                        {
                            assert_eq!(
                                values[id.0 as usize].defining_block, block.label,
                                "{}: ValueId({}) 的 defining_block 必须等于定义块 {}",
                                function.name, id.0, block.label
                            );
                        }
                    }
                });
            }
        }
    }

    // Assert
    assert!(total_values > 0, "编译后必须真的有值被生产");
    assert!(saw_int, "字面量/算术必须产出一条 Int(64) 的真实类型值");
    assert!(saw_bool, "比较结果必须产出一条 Bool 的真实类型值");
}

/// normalize：空表与非空表的 `values_total` / `value` 段都可渲染。
#[test]
fn test_normalize_renders_values_table_both_shapes() {
    // Arrange：空表函数
    let empty = func("empty", vec![], MonoType::Void, vec![], vec![ret_void()]);
    // 非空表函数：一个 Int(64) 值被 Load 定义
    let load = Instruction::Load {
        dst: ssa_value(0),
        src: Operand::Const(ConstValue::Int(7)),
        span: Span::dummy(),
    };
    let with_value = func_with_blocks(
        "with_value",
        vec![],
        MonoType::Void,
        vec![],
        vec![value_info(MonoType::Int(64), 0)],
        vec![block(0, vec![load, ret_void()], vec![])],
    );
    let module = module_with(vec![empty, with_value]);

    // Act
    let rendered = normalize_module(&module);

    // Assert
    assert!(
        rendered.contains("values_total=0"),
        "空值表也要渲染 values_total=0：\n{rendered}"
    );
    assert!(
        rendered.contains("values_total=1"),
        "非空值表要渲染 values_total=1：\n{rendered}"
    );
    assert!(
        rendered.contains("value v0: Int(64) @block0"),
        "值项必须按 ValueId 顺序渲染类型与定义块：\n{rendered}"
    );
}
