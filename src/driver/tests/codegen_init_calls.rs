//! WBS 3.7.4 收口：**init 段与函数体的静态调用目标同口径**（2026-10-11 裁甲）。
//!
//! 规范来源：RFC-039 §3.7.4「init 段 DefId 与函数体同口径」。
//!
//! 背景：ir_gen::assign_defs 原只覆盖 ir.functions，ir.init（模块顶层语句序列）
//! 的调用目标 def 恒为 None；translator::translate_call 的解析优先级是
//! DefId 命中 → FFI 元数据 → 名字命中 → std native 回退，故 init 段恒定走
//! 「名字命中」。两套口径（函数体有 def、init 段无 def）本身不是活体缺陷——名字
//! 与 def 在 codegen 里指向同一函数表下标；本组探针钉的是**分叉已消除**：init 段
//! 与函数体由同一套规则（resolve_static_targets + 同一 resolve 闭包）解析，且这
//! 一次解析的 DefId 指向的正是目标函数自身。
//!
//! 每类调用钉三件事，且**断言顺序固定为「翻译 → 执行 → 身份」**：
//! ① codegen 后 CALL_STATIC 索引**精确**落在目标函数上（native 类落 CALL_NATIVE，
//!    不进函数表）；
//! ② 执行值与分级前的名字路径一致（同一函数、同一索引，不得错位）；
//! ③ init 段调用的 def 有值且**等于目标函数自身的 def**（同一绑定身份，不是
//!    「随便有个值」）。
//! 把身份断言放最后：回退 assign_defs 的 init 覆盖时，失败点落在最后一条本身就
//! 证明前两条（索引、执行值）在旧口径下已经成立——「翻译结果语义不变」的对照证据。

use super::driver::{make_script_project, run_multi};
use crate::frontend::module::symbol::DefId;
use crate::middle::core::ir::{ConstValue, Instruction, ModuleIR, Operand};

/// 跑合并 IR 的 codegen + 解释器，返回 (全部全局槽位值, 执行错误)。
fn run_codegen(module: ModuleIR) -> (Vec<String>, Option<String>) {
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

/// 合并 IR 的 init 段调用：(被调名, def) 列表。
fn init_calls(module: &ModuleIR) -> Vec<(String, Option<DefId>)> {
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

/// 合并 IR 里名为 name 的函数自身的 def——「指向正确函数」的判据基准。
///
/// 辅助函数内的失败必须带上下文（测试规范规则 5.2）：函数表全表随 panic 打印。
fn function_def(
    module: &ModuleIR,
    name: &str,
) -> DefId {
    let names: Vec<&String> = module.functions.iter().map(|f| &f.name).collect();
    let func = module
        .functions
        .iter()
        .find(|f| f.name == name)
        .unwrap_or_else(|| panic!("函数表应含 {name}，实际：{names:?}"));
    func.def
        .unwrap_or_else(|| panic!("函数 {name} 自身的 def 不应为 None（assign_defs 覆盖全部函数）"))
}

/// 函数 func_name 体内对 callee 的静态调用携带的 def——同口径对照用函数体侧样本。
fn function_body_call_def(
    module: &ModuleIR,
    func_name: &str,
    callee: &str,
) -> Option<DefId> {
    let names: Vec<&String> = module.functions.iter().map(|f| &f.name).collect();
    let func = module
        .functions
        .iter()
        .find(|f| f.name == func_name)
        .unwrap_or_else(|| panic!("函数表应含 {func_name}，实际：{names:?}"));
    func.blocks()
        .iter()
        .flat_map(|b| b.instructions.iter())
        .find_map(|i| match i {
            Instruction::Call {
                func: Operand::Const(ConstValue::String(n)),
                def,
                ..
            } if n == callee => Some(*def),
            _ => None,
        })
        .unwrap_or_else(|| panic!("{func_name} 体内应有对 {callee} 的静态调用"))
}

/// 编译后 __yx_module_init 的静态调用目标索引 + 函数表名字清单。
fn init_call_targets(module: ModuleIR) -> (Vec<u32>, Vec<String>) {
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

/// 编译后 __yx_module_init 里的 native 调用名（CALL_NATIVE 的名字操作数）。
fn init_native_calls(module: ModuleIR) -> Vec<String> {
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

/// 函数表里 name 的下标（CALL_STATIC 应精确等于它）。
fn function_index(
    names: &[String],
    name: &str,
) -> u32 {
    names
        .iter()
        .position(|n| n == name)
        .unwrap_or_else(|| panic!("函数表应含 {name}，实际：{names:?}")) as u32
}

/// 探针 A：**本模块函数**调用——init 段的 def 指向 main.helper 自身。
#[test]
fn test_init_call_local_function_resolves() {
    // Arrange：顶层初始化序列调用本模块顶层函数
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

    // Assert①：codegen 后的静态调用索引**精确**指向 main.helper
    let (targets, names) = init_call_targets(module.clone());
    let helper_idx = function_index(&names, "main.helper");
    assert!(
        targets.contains(&helper_idx),
        "init 段 CallStatic 应指向 main.helper(#{helper_idx})，实际 targets={targets:?}"
    );

    // Assert②：执行成功且拿到 7（值与名字路径一致）
    let (values, exec) = run_codegen(module.clone());
    assert!(exec.is_none(), "执行不得报错：{exec:?}");
    assert!(
        values.iter().any(|v| v == "Int(7)"),
        "v = helper() 应得 Int(7)，实际全局值：{values:?}"
    );

    // Assert③：init 段 main.helper 调用的 def 等于该函数自身的 def（同口径身份）
    let calls = init_calls(&module);
    let (_, init_def) = calls
        .iter()
        .find(|(n, _)| n == "main.helper")
        .unwrap_or_else(|| panic!("init 段应出现限定名 main.helper 调用，实际：{calls:?}"));
    assert_eq!(
        *init_def,
        Some(function_def(&module, "main.helper")),
        "init 段 main.helper 调用的 def 必须指向 main.helper 自身，实际：{calls:?}"
    );
}

/// 探针 B：**跨模块导入函数**调用——init 段的 def 指向 lib.add1 自身。
#[test]
fn test_init_call_cross_module_function_resolves() {
    // Arrange：lib 导出 add1，入口以具名导入裸名调用
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

    // Assert①：静态调用索引精确落在 lib.add1
    let (targets, names) = init_call_targets(module.clone());
    let add1_idx = function_index(&names, "lib.add1");
    assert!(
        targets.contains(&add1_idx),
        "init 段 CallStatic 应指向 lib.add1(#{add1_idx})，实际 targets={targets:?}"
    );

    // Assert②：执行成功且 add1(41) = 42
    let (values, exec) = run_codegen(module.clone());
    assert!(exec.is_none(), "执行不得报错：{exec:?}");
    assert!(
        values.iter().any(|v| v == "Int(42)"),
        "v = add1(41) 应得 Int(42)，实际全局值：{values:?}"
    );

    // Assert③：init 段 lib.add1 调用的 def 等于该函数自身的 def（同口径身份）
    let calls = init_calls(&module);
    let (_, init_def) = calls
        .iter()
        .find(|(n, _)| n == "lib.add1")
        .unwrap_or_else(|| panic!("init 段应出现限定名 lib.add1 调用，实际：{calls:?}"));
    assert_eq!(
        *init_def,
        Some(function_def(&module, "lib.add1")),
        "init 段 lib.add1 调用的 def 必须指向 lib.add1 自身，实际：{calls:?}"
    );
}

/// 探针 C：**构造器**调用——init 段的 def 指向类型声明 main.Point 自身
/// （codegen 对 TypeDecl 合成构造函数，索引即该声明下标）。
#[test]
fn test_init_constructor_call_resolves() {
    // Arrange：顶层构造 Point 并读取字段
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

    // Assert①：静态索引精确落在 main.Point（合成构造函数的位置）
    let (targets, names) = init_call_targets(module.clone());
    let point_idx = function_index(&names, "main.Point");
    assert!(
        targets.contains(&point_idx),
        "init 段 CallStatic 应指向 main.Point(#{point_idx})，实际 targets={targets:?}"
    );

    // Assert②：执行成功且 p.x = 5（构造器索引没错位到别处）
    let (values, exec) = run_codegen(module.clone());
    assert!(exec.is_none(), "执行不得报错：{exec:?}");
    assert!(
        values.iter().any(|v| v == "Int(5)"),
        "p.x 应得 Int(5)，实际全局值：{values:?}"
    );

    // Assert③：init 段构造调用的 def 等于类型声明 main.Point 自身的 def
    let calls = init_calls(&module);
    let (_, init_def) = calls
        .iter()
        .find(|(n, _)| n == "main.Point")
        .unwrap_or_else(|| panic!("init 段应出现 main.Point 构造调用，实际：{calls:?}"));
    assert_eq!(
        *init_def,
        Some(function_def(&module, "main.Point")),
        "init 段构造调用的 def 必须指向类型声明 main.Point 自身，实际：{calls:?}"
    );
}

/// 探针 D：**native 调用**——init 段的 def 是 std 符号（同函数体口径），但模块内
/// 无同名函数 ⇒ codegen 的 DefId 命中落空，仍翻译为 CALL_NATIVE(std.math.abs)。
#[test]
fn test_init_native_call_resolves() {
    // Arrange：abs 是 std native（无 IR 函数定义、不在函数表）
    let (_dir, entry) = make_script_project(&[("main.yx", "v = abs(-3)\n")]);

    // Act
    let outcome = run_multi(&entry).expect("driver run failed");
    assert!(
        outcome.failure.is_none(),
        "compile failure: {:?}",
        outcome.failure
    );
    let module = outcome.module.expect("multi-file must produce IR");

    // Assert①：翻译成 CALL_NATIVE，而不是被错塞进某个 CALL_STATIC 索引
    let native_calls = init_native_calls(module.clone());
    assert!(
        native_calls.iter().any(|n| n == "std.math.abs"),
        "init 段 native 调用应翻译为 CALL_NATIVE(std.math.abs)，实际：{native_calls:?}"
    );

    // Assert②：执行成功且 abs(-3) = 3
    let (values, exec) = run_codegen(module.clone());
    assert!(exec.is_none(), "执行不得报错：{exec:?}");
    assert!(
        values.iter().any(|v| v == "Int(3)"),
        "abs(-3) 应得 Int(3)，实际全局值：{values:?}"
    );

    // Assert③：init 段 native 调用名是 std.math.abs，且模块内无同名 IR 函数
    // ——DefId 有值也命不中字节码函数表（第 0 条落空），翻译保持 native 回退
    let calls = init_calls(&module);
    assert!(
        calls.iter().any(|(n, _)| n == "std.math.abs"),
        "init 段应出现 std.math.abs 调用，实际：{calls:?}"
    );
    assert!(
        module.functions.iter().all(|f| f.name != "std.math.abs"),
        "std native 不应出现在 IR 函数表（否则 DefId 命中路径会把它错当 IR 函数）：{:?}",
        module.functions.iter().map(|f| &f.name).collect::<Vec<_>>()
    );
}

/// 同口径判据①：同一目标函数在**函数体**与 **init 段**的调用获得同一个 DefId
/// ——解析规则只有一套（resolve_static_targets + 同一 resolve 闭包）。
#[test]
fn test_init_call_def_matches_function_body_call_def() {
    // Arrange：helper 同时被函数体与顶层初始化序列调用
    let (_dir, entry) = make_script_project(&[(
        "main.yx",
        "helper: () -> Int = () => { 7 }\ng: () -> Int = () => { return helper() }\nv = helper()\n",
    )]);

    // Act
    let outcome = run_multi(&entry).expect("driver run failed");
    assert!(
        outcome.failure.is_none(),
        "compile failure: {:?}",
        outcome.failure
    );
    let module = outcome.module.expect("multi-file must produce IR");

    // Assert：init 段的 def 有值，且与函数体内的 def 逐值相等（同一绑定身份）
    let calls = init_calls(&module);
    let (_, init_def) = calls
        .iter()
        .find(|(n, _)| n == "main.helper")
        .unwrap_or_else(|| panic!("init 段应出现 main.helper 调用，实际：{calls:?}"));
    let body_def = function_body_call_def(&module, "main.g", "main.helper");
    assert!(
        init_def.is_some(),
        "init 段调用必须携带 DefId（与函数体同口径），实际：{calls:?}"
    );
    assert_eq!(
        *init_def, body_def,
        "同一目标函数在 init 段与函数体必须解析到同一个 DefId（一套规则）"
    );
}

/// 同口径判据②：std native 调用在两侧同解——函数体与 init 段得到**相同**的 def，
/// 翻译结果都是 CALL_NATIVE。
#[test]
fn test_init_native_call_def_matches_function_body_call_def() {
    // Arrange：abs 同时被函数体与顶层初始化序列调用
    let (_dir, entry) = make_script_project(&[(
        "main.yx",
        "g: () -> Int = () => { return abs(-3) }\nv = abs(-3)\n",
    )]);

    // Act
    let outcome = run_multi(&entry).expect("driver run failed");
    assert!(
        outcome.failure.is_none(),
        "compile failure: {:?}",
        outcome.failure
    );
    let module = outcome.module.expect("multi-file must produce IR");

    // Assert：init 段仍落 CALL_NATIVE，且两侧 def 相等（同一套解析规则）
    let native_calls = init_native_calls(module.clone());
    assert!(
        native_calls.iter().any(|n| n == "std.math.abs"),
        "native 调用翻译必须保持 CALL_NATIVE(std.math.abs)，实际：{native_calls:?}"
    );
    let calls = init_calls(&module);
    let (_, init_def) = calls
        .iter()
        .find(|(n, _)| n == "std.math.abs")
        .unwrap_or_else(|| panic!("init 段应出现 std.math.abs 调用，实际：{calls:?}"));
    let body_def = function_body_call_def(&module, "main.g", "std.math.abs");
    assert_eq!(
        *init_def, body_def,
        "std native 在 init 段与函数体必须解析到同一个 def（一套规则）"
    );
}
