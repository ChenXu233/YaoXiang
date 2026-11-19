//! `lower/expr/access.rs` 测试 — 「模块导出作值」三路发射臂 × 名字形态的面板 pin
//!
//! 规范来源：
//! - docs/src/reference/language-spec/modules.md §2.2（`use path.{x}` 绑定同一顶层
//!   绑定：函数仍是函数、常量仍是常量）与 §3.1（顶层绑定一律可导入——整体导入经
//!   模块名访问）
//! - docs/src/rfc/accepted/036-module-system.md §4（嵌入纯 YaoXiang std 模块与
//!   native 模块同级注册）
//! - issue #494（WBS 3.6.6）：三臂判据收敛为 registry 单一查询接口
//!   `qualified_name_is`；本面板钉**每个消费臂对每个形态的回答**——判据只改错
//!   一处时，registry 侧面板（`src/frontend/module/tests/registry.rs`）与本面板
//!   同红，证明各臂同源接线；有人把某臂改回本地硬编时，该臂 pin 仍绿，由
//!   结构复审抓回退。
//!
//! 臂 × 形态覆盖（emit = `emit_module_export_value` 三路 + 命名空间臂，按值
//! 位置的可达路由取形）：
//! - emit × NativeName × native 常量（经 `use std.math` + `math.PI` 的命名空间
//!   数据访问路由）→ 零参 `Call`；此路由的兜底臂不是零参调用，故 NativeName
//!   改错即本 pin 红
//! - emit × FunctionCodeExport × native 常量 = 否、× native 函数 = 否
//!   （经 use 别名路由）：形态不得物化为 MakeClosure
//! - emit × native_fn_export × native 函数 → `Load { Const(NativeFn) }`
//!   （#492 形态；`native_fn_value.rs` 只钉 verify_loose，指令形状由本面板断言）
//! - emit × FunctionCodeExport × 纯 yx std 函数 → MakeClosure：由
//!   `src/middle/ir/tests/alias_fn_value.rs`（std.list.map）与
//!   tests/integration/fn_value_call.rs（用户模块）作锚，本面板不重复同一格
//! - 命名空间臂 × FunctionOrMethodDef：符号表 def 面的回答矩阵钉在 registry
//!   面板；本面板以「def 面对常量导出答否」的负格（pin A 的 FallThrough 断言）
//!   钉住发射序

use crate::frontend::core::lexer::tokenize;
use crate::frontend::core::parser::parse;
use crate::frontend::core::sema::checker::TypeChecker;
use crate::frontend::core::sema::probe::TypeLevelNamespace;
use crate::frontend::module::registry::ModuleRegistry;
use crate::middle::ir::ir::{ConstValue, FunctionBody, Instruction, ModuleIR, Operand};
use crate::middle::lower::AstToIrGenerator;
use crate::util::diagnostic::Severity;

/// 源码 → 真实生成序列（lex → parse → typecheck → ir_gen）的直驱探针。
/// 与 `src/middle/ir/tests/alias_fn_value.rs` / `native_fn_value.rs` 同款。
fn compile_ir(source: &str) -> ModuleIR {
    let tokens = tokenize(source).expect("夹具必须可 tokenize");
    let parsed = parse(&tokens);
    assert!(
        !parsed.has_errors,
        "夹具必须无语法错误: {:?}",
        parsed.errors
    );
    let mut checker = TypeChecker::new("<qualified-name-targets>");
    let type_result = checker.check_module(
        &parsed.module,
        &TypeLevelNamespace::scan_one(&parsed.module),
    );
    let errors: Vec<_> = type_result
        .diagnostics
        .iter()
        .filter(|d| matches!(d.severity, Severity::Error))
        .collect();
    assert!(errors.is_empty(), "夹具必须无类型错误: {errors:?}");
    let mut generator = AstToIrGenerator::new_with_type_result(
        &type_result,
        ModuleRegistry::with_std(),
        None,
        TypeLevelNamespace::scan_one(&parsed.module),
    );
    generator
        .generate_module_ir(&parsed.module)
        .expect("夹具必须生成 IR")
}

/// 函数体里三类目标指令：MakeClosure 目标名 / 零参 Call 目标名 /
/// Load(NativeFn) 目标名。
fn value_shapes(
    ir: &ModuleIR,
    func_name: &str,
) -> (Vec<String>, Vec<String>, Vec<String>) {
    let func = ir
        .functions
        .iter()
        .find(|f| f.name == func_name)
        .unwrap_or_else(|| panic!("函数 {func_name} 必须在 IR 函数表里"));
    let FunctionBody::Code { blocks, .. } = &func.body else {
        panic!("函数 {func_name} 必须是代码体");
    };
    let mut closures = Vec::new();
    let mut zero_arg_calls = Vec::new();
    let mut native_fn_loads = Vec::new();
    for block in blocks {
        for instr in &block.instructions {
            match instr {
                Instruction::MakeClosure { func, .. } => closures.push(func.clone()),
                Instruction::Call {
                    func: Operand::Const(ConstValue::String(name)),
                    args,
                    ..
                } if args.is_empty() => zero_arg_calls.push(name.clone()),
                Instruction::Load {
                    src: Operand::Const(ConstValue::NativeFn(name)),
                    ..
                } => native_fn_loads.push(name.clone()),
                _ => {}
            }
        }
    }
    (closures, zero_arg_calls, native_fn_loads)
}

/// 命名空间数据访问路由 × native 常量 → 零参 `Call`（既有成功形态，逐字节不变）。
///
/// 路由：`use std.math` + `math.PI` 作值 → 命名空间降级块
/// （access.rs `resolve_namespace_data_access` 命中）→ emit 三路：函数代码=否、
/// 原生函数=否、NativeName=是 → 零参 `Call("std.math.PI")`。该路由的兜底
/// （emit 未命中）落到普通字段访问并报内部错误——**不是**零参调用，故
/// NativeName 对此形态改错时本 pin 必红（同源接线证据）。
#[test]
fn test_namespace_constant_value_keeps_zero_arg_call() {
    // Arrange
    let ir = compile_ir(
        "use std.math
main: () -> Void = {
    f = math.PI
}
main()
",
    );

    // Act
    let (closures, zero_arg_calls, native_fn_loads) = value_shapes(&ir, "main");

    // Assert：零参调用是唯一形态；不得物化为闭包或原生函数值
    assert!(
        zero_arg_calls.iter().any(|name| name == "std.math.PI"),
        "native 常量作值必须保持零参 Call(\"std.math.PI\")（既有成功形态），\
         实际零参调用={zero_arg_calls:?}"
    );
    assert!(
        !closures.iter().any(|name| name == "std.math.PI"),
        "native 常量不是函数代码，不得物化为 MakeClosure，实际闭包={closures:?}"
    );
    assert!(
        !native_fn_loads.iter().any(|name| name == "std.math.PI"),
        "native 常量不是原生函数（#492 面只给 Function 导出），不得 Load NativeFn，\
         实际={native_fn_loads:?}"
    );
}

/// use 别名路由 × native 常量 → 零参 `Call`（别名兜底臂与发射臂同形态）。
///
/// `use std.math.{PI}` + `f = PI`：别名未命中跨文件值槽位（常量不是值绑定），
/// 落 emit 三路——函数代码=否、原生函数=否、NativeName=是 → 零参 `Call`。
/// FunctionCodeExport 对此形态若改答「是」会让常量物化为闭包，本 pin 必红。
#[test]
fn test_native_constant_alias_value_keeps_zero_arg_call() {
    // Arrange
    let ir = compile_ir(
        "use std.math.{PI}
main: () -> Void = {
    f = PI
}
main()
",
    );

    // Act
    let (closures, zero_arg_calls, native_fn_loads) = value_shapes(&ir, "main");

    // Assert
    assert!(
        zero_arg_calls.iter().any(|name| name == "std.math.PI"),
        "别名引入的 native 常量作值必须保持零参 Call（与修复前同形态），\
         实际零参调用={zero_arg_calls:?}"
    );
    assert!(
        !closures.iter().any(|name| name == "std.math.PI"),
        "native 常量不是函数代码（#479/#493 只给 yx 函数导出），不得 MakeClosure，\
         实际闭包={closures:?}"
    );
    assert!(
        !native_fn_loads.iter().any(|name| name == "std.math.PI"),
        "native 常量不得走 #492 的原生函数值形态，实际={native_fn_loads:?}"
    );
}

/// use 别名路由 × native 函数 → `Load { Const(NativeFn) }`（#492 形态）。
///
/// `use std.io.{println}` + `f = println`：原生函数没有 IR 函数体，MakeClosure
/// 按名查函数表必然落空——必须物化为原生函数值。本 pin 把
/// `native_fn_value.rs` 头注记录的指令形状变成断言；零参 `Call` 是修复前的
/// 静默不动作形态（stdout 空、exit 0），出现即红。
#[test]
fn test_native_fn_alias_value_loads_native_fn() {
    // Arrange
    let ir = compile_ir(
        "use std.io.{println}
main: () -> Void = {
    f = println
    f(\"hello\")
}
main()
",
    );

    // Act
    let (closures, zero_arg_calls, native_fn_loads) = value_shapes(&ir, "main");

    // Assert
    assert!(
        native_fn_loads.iter().any(|name| name == "std.io.println"),
        "原生函数作值必须物化为 Load(NativeFn(\"std.io.println\"))（#492），\
         实际={native_fn_loads:?}"
    );
    assert!(
        !zero_arg_calls.iter().any(|name| name == "std.io.println"),
        "原生函数作值不得落零参 Call（修复前静默不动作形态），\
         实际零参调用={zero_arg_calls:?}"
    );
    assert!(
        !closures.iter().any(|name| name == "std.io.println"),
        "原生函数没有 IR 函数体，MakeClosure 查表必落空，不得物化为闭包，\
         实际闭包={closures:?}"
    );
}

/// 命名空间臂 × 可调用 def（Method）→ `MakeClosure`（#396 降级块的 ③ 臂）。
///
/// `use std.option` + `option.Option.is_failure`：std yx 模块（src/std/option.yx，
/// 无 native 对应 ⇒ 走全新 register 路径）的方法绑定在注册期 intern 为
/// Method def（`std.option.Option.is_failure`）——它不是模块导出（emit 三路
/// 全否：函数代码/原生函数/NativeName 都答「否」），只有符号表 def 面能答
/// 「是」（FunctionOrMethodDef）。判据已收敛进 registry 单一查询接口
/// （issue #494），此形态改判或臂被改回本地硬编时本 pin 必红。
/// 混合面反例（`std.result` 的 Try 方法）：嵌入 yx 合并面（register 的
/// surface-merge 分支）不 intern 方法 ⇒ def=None ⇒ 落普通字段访问报 E3005
/// ——那是既有行为，本 pin 不钉它（属 #494 收敛面之外）。
#[test]
fn test_method_def_via_namespace_materializes_closure() {
    // Arrange
    let ir = compile_ir(
        "use std.option
main: () -> Void = {
    f = option.Option.is_failure
}
main()
",
    );

    // Act
    let (closures, zero_arg_calls, native_fn_loads) = value_shapes(&ir, "main");

    // Assert
    assert!(
        closures
            .iter()
            .any(|name| name == "std.option.Option.is_failure"),
        "方法 def 作值必须由命名空间臂物化为 MakeClosure（符号表 FunctionOrMethodDef 面），\
         实际闭包={closures:?}"
    );
    assert!(
        !zero_arg_calls
            .iter()
            .any(|name| name == "std.option.Option.is_failure"),
        "方法 def 不是导出，emit 三路必须全否（不得零参调用），\
         实际零参调用={zero_arg_calls:?}"
    );
    assert!(
        !native_fn_loads
            .iter()
            .any(|name| name == "std.option.Option.is_failure"),
        "方法 def 不是原生函数（#492 只给 native Function 导出），不得 Load NativeFn，\
         实际={native_fn_loads:?}"
    );
}
