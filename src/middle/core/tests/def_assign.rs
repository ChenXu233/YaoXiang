//! Stage 3 DefId 承重验证：assign_defs 为每个函数分配 DefId，并把静态调用解析到同一 DefId。
//!
//! 回归保护：若 assign_defs 失效（def 全为 None 或调用侧解析不到），codegen 会静默回退
//! 按名分发——行为不变但 DefId 不再承重，本测试即是防静默回退的哨兵。

use crate::frontend::compiler::Compiler;
use crate::frontend::module::symbol::DefId;
use crate::middle::core::ir::{
    BasicBlock, ConstValue, FunctionBody, FunctionIR, Instruction, ModuleIR, Operand,
};

/// 夹具源码：main 里有一次对 helper 的静态调用。
const SRC: &str = r#"
helper: (x: Int) -> Int = (x) => { return x }

main = () => {
    y = helper(1)
}
"#;

/// 编译夹具源码并取回模块 IR；失败时带上错误原因。
fn compile(src: &str) -> ModuleIR {
    Compiler::new()
        .compile("test", src)
        .unwrap_or_else(|e| panic!("compile should succeed, but failed with: {e:?}"))
}

/// 取名为 name 的函数定义——找不到即哨兵失守。
fn function_named<'a>(
    ir: &'a ModuleIR,
    name: &str,
) -> &'a FunctionIR {
    ir.functions
        .iter()
        .find(|f| f.name == name)
        .unwrap_or_else(|| panic!("{name} should be emitted"))
}

/// 取名为 name 的函数所携带的 DefId。
fn def_of(
    ir: &ModuleIR,
    name: &str,
) -> DefId {
    function_named(ir, name)
        .def
        .unwrap_or_else(|| panic!("{name} should have a DefId"))
}

/// 在 blocks 的指令流里找出对 callee 的静态调用所携带的 DefId。
fn call_def_of(
    blocks: &[BasicBlock],
    callee: &str,
) -> DefId {
    blocks
        .iter()
        .flat_map(|b| &b.instructions)
        .find_map(|i| match i {
            Instruction::Call {
                func: Operand::Const(ConstValue::String(n)),
                def,
                ..
            } if n == callee => *def,
            _ => None,
        })
        .unwrap_or_else(|| panic!("the static call to {callee} should carry a DefId"))
}

#[test]
fn assign_defs_gives_every_function_def_and_resolves_static_calls() {
    let ir = compile(SRC);

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
    let helper_def = def_of(&ir, "helper");
    let main_func = function_named(&ir, "main");
    let FunctionBody::Code { blocks, .. } = &main_func.body else {
        panic!("main should have a code body")
    };
    let call_def = call_def_of(blocks, "helper");
    assert_eq!(
        call_def, helper_def,
        "call site and definition site should agree on the same DefId"
    );
}
