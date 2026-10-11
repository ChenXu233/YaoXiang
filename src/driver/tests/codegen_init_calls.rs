//! 实证：**init 段（`def=None`）的调用在 codegen→字节码路径能否解析**。
//!
//! 背景：`ir_gen::assign_defs` 只覆盖 `ir.functions`，`ir.init` 的调用目标
//! `def` 恒为 None；`translator::translate_call` 的解析优先级是
//! DefId 命中 → FFI 元数据 → **名字命中** → std native 回退。
//! 本组探针只做实证（不改解析优先级）：直接跑「编译 → codegen → 执行」。
//!
//! 每类调用都钉两件事：① 合并 IR 的 init 段确实 `def=None`；② 该调用在 codegen
//! 后被解析成 CALL_STATIC 且索引落在真实函数上（执行成功 + 全局槽位取到正确值
//! 即反向证明索引没错位）。

use super::driver::{make_script_project, run_multi};

/// 跑合并 IR 的 codegen + 解释器，返回 (全部全局槽位值, 执行错误)。
fn run_codegen(module: crate::middle::ModuleIR) -> (Vec<String>, Option<String>) {
    let mut ctx = crate::middle::passes::codegen::CodegenContext::new(module);
    let file = match ctx.generate() {
        Ok(f) => f,
        Err(d) => return (Vec::new(), Some(format!("codegen error: {d}"))),
    };
    let bytecode_module = crate::middle::bytecode::BytecodeModule::from(file);
    let mut interp = crate::backends::interpreter::Interpreter::new();
    use crate::backends::Executor as _;
    let exec = interp
        .execute_module(&bytecode_module)
        .err()
        .map(|e| format!("{e:?}"));
    let values = (0..16)
        .map_while(|i| interp.global_slot(i).map(|v| format!("{v:?}")))
        .collect();
    (values, exec)
}

/// 合并 IR 的 init 段调用：`(被调名, def)` 列表——用于证明 `def=None`。
fn init_calls(
    module: &crate::middle::ModuleIR
) -> Vec<(String, Option<crate::frontend::module::symbol::DefId>)> {
    use crate::middle::core::ir::{ConstValue, Instruction, Operand};
    module
        .init
        .iter()
        .filter_map(|i| match i {
            Instruction::Call { func, def, .. } | Instruction::TailCall { func, def, .. } => {
                match func {
                    Operand::Const(ConstValue::String(n)) => Some((n.clone(), *def)),
                    _ => None,
                }
            }
            _ => None,
        })
        .collect()
}

/// 编译后 `__yx_module_init` 的静态调用目标索引 + 函数表名字清单。
fn init_call_targets(module: crate::middle::ModuleIR) -> (Vec<u32>, Vec<String>) {
    let mut ctx = crate::middle::passes::codegen::CodegenContext::new(module);
    let file = ctx.generate().expect("codegen must succeed");
    let bytecode = crate::middle::bytecode::BytecodeModule::from(file);
    let names: Vec<String> = bytecode.functions.iter().map(|f| f.name.clone()).collect();
    let targets: Vec<u32> = bytecode
        .functions
        .iter()
        .filter(|f| f.name == "__yx_module_init")
        .flat_map(|f| f.instructions.iter())
        .filter_map(|i| match i {
            crate::middle::core::bytecode::BytecodeInstr::CallStatic { func, .. } => Some(*func),
            _ => None,
        })
        .collect();
    (targets, names)
}

/// 编译后 `__yx_module_init` 里的 native 调用名（CALL_NATIVE 的名字操作数）。
fn init_native_calls(module: crate::middle::ModuleIR) -> Vec<String> {
    let mut ctx = crate::middle::passes::codegen::CodegenContext::new(module);
    let file = ctx.generate().expect("codegen must succeed");
    let bytecode = crate::middle::bytecode::BytecodeModule::from(file);
    bytecode
        .functions
        .iter()
        .filter(|f| f.name == "__yx_module_init")
        .flat_map(|f| f.instructions.iter())
        .filter_map(|i| match i {
            crate::middle::core::bytecode::BytecodeInstr::CallNative { func_name, .. } => {
                Some(func_name.clone())
            }
            _ => None,
        })
        .collect()
}
/// 探针 A：**本模块函数**调用（init 段，def=None）。
#[test]
fn test_init_call_local_function_resolves() {
    // Arrange
    let (_dir, entry) =
        make_script_project(&[("main.yx", "helper: () -> Int = () => { 7 }\nv = helper()\n")]);

    // Act
    let outcome = run_multi(&entry).expect("driver run failed");
    assert!(
        outcome.failure.is_none(),
        "compile failure: {:?}",
        outcome.failure
    );
    let module = outcome.module.expect("multi-file must produce IR");

    // Assert①：init 段调用确实 def=None（本实证的前提）
    let calls = init_calls(&module);
    assert!(
        calls.iter().any(|(n, _)| n == "main.helper"),
        "init 段应出现限定名 main.helper 调用，实际：{calls:?}"
    );
    assert!(
        calls.iter().all(|(_, def)| def.is_none()),
        "本实证前提：init 段 def 应恒为 None，实际：{calls:?}"
    );

    // Assert②：codegen 后的静态调用索引落在真实函数上
    let (targets, names) = init_call_targets(module.clone());
    assert!(
        !targets.is_empty(),
        "init 段应有静态调用，实际字节码函数表：{names:?}"
    );
    assert!(
        targets.iter().all(|t| (*t as usize) < names.len()),
        "CALL_STATIC 索引必须落在函数表内：targets={targets:?} functions={names:?}"
    );
    // 索引必须**精确**指向 main.helper（不是同表内碰巧可执行的别的函数）
    let helper_idx = names
        .iter()
        .position(|n| n == "main.helper")
        .unwrap_or_else(|| panic!("函数表应含 main.helper，实际：{names:?}"));
    assert!(
        targets.contains(&(helper_idx as u32)),
        "init 段 CallStatic 应指向 main.helper(#{helper_idx})，实际 targets={targets:?}"
    );

    // Assert③：执行成功且拿到 7（名字回退兜住了）
    let (values, exec) = run_codegen(module);
    assert!(exec.is_none(), "执行不得报错：{exec:?}");
    assert!(
        values.iter().any(|v| v == "Int(7)"),
        "v = helper() 应得 Int(7)，实际全局值：{values:?}"
    );
}

/// 探针 B：**跨模块导入函数**调用（init 段，def=None，名字是 lib.* 限定名）。
#[test]
fn test_init_call_cross_module_function_resolves() {
    // Arrange
    let (_dir, entry) = make_script_project(&[
        ("lib.yx", "add1: (x: Int) -> Int = (x) => { x + 1 }\n"),
        ("main.yx", "use lib.{add1}\nv = add1(41)\n"),
    ]);

    // Act
    let outcome = run_multi(&entry).expect("driver run failed");
    assert!(
        outcome.failure.is_none(),
        "compile failure: {:?}",
        outcome.failure
    );
    let module = outcome.module.expect("multi-file must produce IR");

    // Assert①：init 调用名已限定为 lib.add1 且 def=None
    let calls = init_calls(&module);
    assert!(
        calls
            .iter()
            .any(|(n, def)| n == "lib.add1" && def.is_none()),
        "init 段应出现 def=None 的限定名 lib.add1 调用，实际：{calls:?}"
    );

    // Assert②：执行成功且 add1(41) = 42
    let (values, exec) = run_codegen(module);
    assert!(exec.is_none(), "执行不得报错：{exec:?}");
    assert!(
        values.iter().any(|v| v == "Int(42)"),
        "v = add1(41) 应得 Int(42)，实际全局值：{values:?}"
    );
}

/// 探针 C：**构造器**调用（init 段；`Point(...)` 的身份是类型 DefId，def=None
/// 时只能靠名字命中合成构造函数）。
#[test]
fn test_init_constructor_call_resolves() {
    // Arrange
    let (_dir, entry) = make_script_project(&[(
        "main.yx",
        "Point: Type = { x: Int }\np = Point(5)\nv = p.x\n",
    )]);

    // Act
    let outcome = run_multi(&entry).expect("driver run failed");
    assert!(
        outcome.failure.is_none(),
        "compile failure: {:?}",
        outcome.failure
    );
    let module = outcome.module.expect("multi-file must produce IR");

    // Assert①：init 段有 def=None 的构造调用（限定名 main.Point）
    let calls = init_calls(&module);
    assert!(
        calls
            .iter()
            .any(|(n, def)| n.ends_with("Point") && def.is_none()),
        "init 段应出现 def=None 的构造器调用，实际：{calls:?}"
    );

    // Assert②：执行成功且 p.x = 5（构造器索引没错位到别处）
    let (values, exec) = run_codegen(module);
    assert!(exec.is_none(), "执行不得报错：{exec:?}");
    assert!(
        values.iter().any(|v| v == "Int(5)"),
        "p.x 应得 Int(5)，实际全局值：{values:?}"
    );
}

/// 探针 D：**native 调用**（init 段，def=None，函数表无此名 → 落 std native 回退）。
#[test]
fn test_init_native_call_resolves() {
    // Arrange：abs 是 std native（无 IR 定义、不在函数表）
    let (_dir, entry) = make_script_project(&[("main.yx", "v = abs(-3)\n")]);

    // Act
    let outcome = run_multi(&entry).expect("driver run failed");
    assert!(
        outcome.failure.is_none(),
        "compile failure: {:?}",
        outcome.failure
    );
    let module = outcome.module.expect("multi-file must produce IR");

    // Assert①：init 段调用 def=None（native 名不在函数表）
    let calls = init_calls(&module);
    assert!(
        calls.iter().any(|(_, def)| def.is_none()),
        "init 段 native 调用应为 def=None，实际：{calls:?}"
    );

    // Assert②：翻译成 CALL_NATIVE（名字不在函数表 → 第 3 条 native 回退），
    // 而不是被错塞进某个 CALL_STATIC 索引
    let native_calls = init_native_calls(module.clone());
    assert!(
        native_calls.iter().any(|n| n == "std.math.abs"),
        "init 段 native 调用应翻译为 CALL_NATIVE(std.math.abs)（名字不在函数表 → 第 3 条回退），实际：{native_calls:?}"
    );

    // Assert③：执行成功且 abs(-3) = 3
    let (values, exec) = run_codegen(module);
    assert!(exec.is_none(), "执行不得报错：{exec:?}");
    assert!(
        values.iter().any(|v| v == "Int(3)"),
        "abs(-3) 应得 Int(3)，实际全局值：{values:?}"
    );
}
