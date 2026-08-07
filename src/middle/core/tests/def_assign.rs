//! Stage 3 DefId 承重验证：assign_defs 为每个函数分配 DefId，并把静态调用解析到同一 DefId。
//!
//! 回归保护：若 assign_defs 失效（def 全为 None 或调用侧解析不到），codegen 会静默回退
//! 按名分发——行为不变但 DefId 不再承重，本测试即是防静默回退的哨兵。

use crate::frontend::compiler::Compiler;
use crate::middle::core::ir::{ConstValue, FunctionBody, Instruction, Operand};

#[test]
fn assign_defs_gives_every_function_def_and_resolves_static_calls() {
    let src = r#"
helper: (x: Int) -> Int = (x) => { return x }

main = {
    y = helper(1)
}
"#;
    let mut compiler = Compiler::new();
    let ir = compiler
        .compile("test", src)
        .expect("compile should succeed");

    // 每个函数（含 main）都分配到 DefId
    assert!(
        ir.functions.iter().all(|f| f.def.is_some()),
        "every function should carry a DefId, got: {:?}",
        ir.functions
            .iter()
            .map(|f| (f.name.clone(), f.def))
            .collect::<Vec<_>>()
    );

    // main 中对 helper 的静态调用解析到与 helper 定义相同的 DefId
    let helper_def = ir
        .functions
        .iter()
        .find(|f| f.name == "helper")
        .and_then(|f| f.def)
        .expect("helper should have a DefId");
    let main_func = ir
        .functions
        .iter()
        .find(|f| f.name == "main")
        .expect("main should exist");
    let FunctionBody::Code { blocks, .. } = &main_func.body else {
        panic!("main should have a code body")
    };
    let call_def = blocks
        .iter()
        .flat_map(|b| &b.instructions)
        .find_map(|i| match i {
            Instruction::Call {
                func: Operand::Const(ConstValue::String(n)),
                def,
                ..
            } if n == "helper" => *def,
            _ => None,
        })
        .expect("the static call to helper should carry a DefId");
    assert_eq!(
        call_def, helper_def,
        "call site and definition site should agree on the same DefId"
    );
}
