//! 运行期形参槽位口径 — 基于 RFC-039 6.5（表示重塑零行为差分）
//! 与 WBS 8.9.1（参数位形态归一，D53）
//!
//! 判据：参数位的**裸标识符**（`(Int64)` → `Param.name` 是标识符、`ty = None`）
//! **不占运行期形参位**——这是 3e4be8c27 的基线口径；6.5 把 `Type::Fn.params`
//! 从 `Vec<Type>` 重塑为 `Vec<Param>` 时不得改变它。
//!
//! **此口径由 8.9.1 重新裁定；改它必须改这条测试。**

use crate::frontend::core::sema::probe::TypeLevelNamespace;
use crate::frontend::core::lexer::tokenize;
use crate::frontend::core::parser::parse;
use crate::frontend::core::sema::checker::TypeChecker;
use crate::frontend::module::registry::ModuleRegistry;
use crate::middle::ir::ir::ModuleIR;
use crate::middle::ir::ir_gen::AstToIrGenerator;

/// 源码 → 规范化前的 module IR：与语料快照判据（`tests/snapshot.rs`）同一条
/// lex → parse → typecheck → ir_gen 生成序列。
fn generate_ir(source: &str) -> ModuleIR {
    let tokens = tokenize(source).expect("测试源码词法不应失败");
    let parsed = parse(&tokens);
    assert!(
        !parsed.has_errors,
        "测试源码语法必须正确（否则无法区分 bug 与测试写错）: {:?}",
        parsed.errors
    );
    let mut checker = TypeChecker::new("bare_param_slots");
    let type_result = checker.check_module(
        &parsed.module,
        &TypeLevelNamespace::scan_one(&parsed.module),
    );
    assert!(
        type_result.diagnostics.is_empty(),
        "测试源码类型检查应通过: {:?}",
        type_result.diagnostics
    );
    let mut generator =
        AstToIrGenerator::new_with_type_result(&type_result, ModuleRegistry::with_std(), None);
    generator
        .generate_module_ir(&parsed.module)
        .expect("IR 生成应成功")
}

#[test]
fn test_bare_identifier_param_takes_no_runtime_slot() {
    // Arrange & Act：外层参数位是裸标识符 `(Int64)`，内层是带类型形参 `(a: Int64)`
    let module = generate_ir("f: (Int64) -> (a: Int64) -> Int64 = 1");

    // Assert：外层函数不因裸标识符获得运行期形参位（基线 3e4be8c27 口径）
    let outer = module
        .functions
        .iter()
        .find(|f| f.name == "f")
        .expect("应生成外层函数 f");
    assert!(
        outer.params.is_empty(),
        "参数位裸标识符 (Int64) 不占运行期形参位（基线口径；8.9.1 重新裁定前不得改此测试），got: {:?}",
        outer.params
    );

    // Assert：内层带类型形参仍占位（过滤只作用于「不带类型的形参」，不是整条链）
    let inner = module
        .functions
        .iter()
        .find(|f| f.name.starts_with("__f_l"))
        .expect("应生成内层 curry 函数 __f_l0");
    assert_eq!(
        inner.params.len(),
        1,
        "内层带类型形参应保留 1 个运行期形参位，got: {:?}",
        inner.params
    );
}

// ─────────────────────────────────────────────────────────────────────────────
// WBS 6.8.3：实现头只承载作者写的形参 ⇒ IR 的形参类型从**声明**渲染
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn test_impl_head_param_type_rendered_from_declaration() {
    // Arrange & Act：实现头不带标注，声明的形参类型是非 Int 的 Bool
    //（parser 停止填声明类型后，IR 不能再退回 Int(64) 占位——占位会让帧布局里的
    //  形参类型与声明不符，而声明是 IR 唯一的类型来源）
    let module = generate_ir("f: (a: Bool) -> Int = (x) => 1");

    // Assert：形参类型取声明第 0 位（Bool）
    let f = module
        .functions
        .iter()
        .find(|f| f.name == "f")
        .expect("应生成函数 f");
    assert_eq!(
        f.params.len(),
        1,
        "实现头 1 位形参对应声明第 0 位，got: {:?}",
        f.params
    );
    assert!(
        matches!(
            f.params[0],
            crate::frontend::core::types::mono::MonoType::Bool
        ),
        "形参类型必须渲染声明的 Bool（Int(64) 占位说明声明来源没接上），got: {:?}",
        f.params[0]
    );
}

#[test]
fn test_impl_head_param_types_align_positionally_with_declaration() {
    // Arrange & Act：两位形参、实现头改名且不带标注，声明类型各不相同
    let module = generate_ir("f: (a: Int, b: Bool) -> Int = (x, y) => x");

    // Assert：按**位置**渲染（第 1 位取声明的 Bool，不是第 0 位的 Int、
    // 也不是 Int(64) 占位）——两位同型时这个错位是看不出
    let f = module
        .functions
        .iter()
        .find(|f| f.name == "f")
        .expect("应生成函数 f");
    assert_eq!(
        f.params.len(),
        2,
        "两位实现形参应对应声明前两位，got: {:?}",
        f.params
    );
    assert!(
        matches!(
            f.params[1],
            crate::frontend::core::types::mono::MonoType::Bool
        ),
        "第 1 位形参必须渲染声明第 1 位的 Bool（按位对齐），got: {:?}",
        f.params[1]
    );
}

// ─────────────────────────────────────────────────────────────────────────────
// WBS 6.8.4-1f：实现头**绑定位 ≠ 运行期下标**（SPEC §7.1 两条序列）
// ─────────────────────────────────────────────────────────────────────────────
//
// 绑定序列 = 形参 − 类型参数 − 编译期值参数；调用点实参序列 = 形参 − 类型参数
//（编译期值参数**仍占**实参位）。故实现头第 i 位不一定落在运行期第 i 槽：
// `f: (N: Int, M: N) -> Int = (m) => m` 的 `m` 是绑定第 0 位、运行期第 **1** 位
//（第 0 位是编译期值参数 N）。判据只能落在 IR——typecheck 层看不见槽位。

#[test]
fn test_impl_head_binding_slot_differs_from_runtime_index() {
    // Arrange & Act：N 是编译期值参数（M 的标注用到它），实现头只写一位 `m`
    let module = generate_ir("f: (N: Int, M: N) -> Int = (m) => m");

    // Assert：`m` 必须占到运行期第 1 槽（arg 1），而不是第 0 槽（arg 0 = N 的值）
    let f = module
        .functions
        .iter()
        .find(|f| f.name == "f")
        .expect("应生成函数 f");
    let debug = format!("{:?}", f.body);
    assert!(
        debug.contains("Arg(1)"),
        "实现头唯一的形参 m 必须绑定运行期第 1 槽（编译期值参数 N 占第 0 槽），got: {debug}"
    );
}

// ─────────────────────────────────────────────────────────────────────────────
// WBS 6.8.2b（D54 退役面）：声明侧**编译期形参位**不得被灌进运行期值形参
// ─────────────────────────────────────────────────────────────────────────────
//
// `#295` 补参段（parser 对表达式体合成的 lambda + sema 的按名补参）会遍历整条
// 拍平签名；按位对齐改造后必须确认：类型位（`(T: Type)`）、类型级函数位
//（`(F: (T: Type) -> Type)`）、const 位（`(N: Int)` 里被类型体引用的 N）**不占**
// 本层值形参——否则体内会多出一个拿不到值的形参，IR 帧布局随之错位。
//
// 后两个夹具还同时覆盖「实现头改名」：改名后类型只能来自声明（按位），
// 一旦把编译期位按位灌进来，形参类型会立刻变成 Type / 元数错位。

#[test]
fn test_decl_type_position_takes_no_runtime_param_slot() {
    // Arrange & Act：类型位 T 在首、值位 x 只有声明给类型（实现头写 y 且不带类型）
    let module = generate_ir("f: (T: Type, x: T) -> T = (y) => y");

    // Assert：运行期形参只有值位那一个（类型位不占槽）
    let f = module
        .functions
        .iter()
        .find(|f| f.name == "f")
        .expect("应生成函数 f");
    assert_eq!(
        f.params.len(),
        1,
        "类型位 (T: Type) 不得占运行期形参槽（只剩值位 x），got: {:?}",
        f.params
    );
    assert!(
        !matches!(
            f.params[0],
            crate::frontend::core::types::mono::MonoType::MetaType { .. }
        ),
        "值位 y 的类型必须来自声明的值类型 T，不得被灌成 Type 元类型，got: {:?}",
        f.params[0]
    );
}

#[test]
fn test_decl_const_position_stays_in_its_own_curry_layer() {
    // Arrange & Act：const 柯里 (N: Int) -> (n: N) -> Int，实现头只写内层
    // 注：此处实现头与声明**同名**——柯里形态下改名会命中另一处已知缺口
    //（sema 放行、ir_gen 的 curry 路径按声明名注册局部名 ⇒ E3006「内部错误」，
    // 父提交 5c0ceeaf 实测同样复现，不属 WBS 6.8.2b 的退役面），故本夹具
    // 只钉「编译期位不占本层运行期位」这一口径。
    let module = generate_ir("f: (N: Int) -> (n: N) -> Int = (n) => n");

    // Assert：外层拿到 const 位（参数固化是本层运行期位），内层只有值位——
    // const 位不得被重复灌进内层值形参表
    let outer = module
        .functions
        .iter()
        .find(|f| f.name == "f")
        .expect("应生成外层函数 f");
    assert_eq!(
        outer.params.len(),
        1,
        "const 位 N 在自己那一层占 1 个运行期位，got: {:?}",
        outer.params
    );
    let inner = module
        .functions
        .iter()
        .find(|f| f.name.starts_with("__f_l"))
        .expect("应生成内层 curry 函数 __f_l0");
    assert_eq!(
        inner.params.len(),
        1,
        "内层值层只有 1 位（const 位不得被灌进内层），got: {:?}",
        inner.params
    );
}

#[test]
fn test_named_argument_uses_declared_name_at_ir_level() {
    // Arrange & Act：实现头改名，调用侧用**声明名**传命名实参——
    // 命名实参的重排在 IR 侧做（fn_param_names 表），IR 生成不得报 E1014
    let module = generate_ir(
        "f: (a: Int) -> Int = (b) => b\n\
         main: () -> Void = {\n\
             x: Int = f(a = 7)\n\
         }",
    );

    // Assert：函数照常生成（旧口径此处报 E1014，available 列的是实现头名 b）
    assert!(
        module.functions.iter().any(|f| f.name == "f"),
        "命名实参按声明名绑定后 IR 仍应生成函数 f"
    );
}

/// 源码 → 前端（typecheck）阶段的诊断消息。
fn typecheck_errors(source: &str) -> Vec<String> {
    let tokens = tokenize(source).expect("测试源码词法不应失败");
    let parsed = parse(&tokens);
    assert!(
        !parsed.has_errors,
        "测试源码语法必须正确: {:?}",
        parsed.errors
    );
    let mut checker = TypeChecker::new("named_arg_names");
    checker
        .check_module(
            &parsed.module,
            &TypeLevelNamespace::scan_one(&parsed.module),
        )
        .diagnostics
        .into_iter()
        .map(|d| d.message)
        .collect()
}

/// 源码 → IR 级诊断（typecheck 必须干净，只回 IR 阶段报告的错误）。
fn generate_ir_errors(source: &str) -> Vec<String> {
    let tokens = tokenize(source).expect("测试源码词法不应失败");
    let parsed = parse(&tokens);
    assert!(
        !parsed.has_errors,
        "测试源码语法必须正确: {:?}",
        parsed.errors
    );
    let mut checker = TypeChecker::new("named_arg_names");
    let type_result = checker.check_module(
        &parsed.module,
        &TypeLevelNamespace::scan_one(&parsed.module),
    );
    assert!(
        type_result.diagnostics.is_empty(),
        "测试源码类型检查应通过（E1014 的 available 列表在 IR 阶段结算）: {:?}",
        type_result.diagnostics
    );
    let mut generator =
        AstToIrGenerator::new_with_type_result(&type_result, ModuleRegistry::with_std(), None);
    match generator.generate_module_ir(&parsed.module) {
        Ok(_) => Vec::new(),
        Err(diags) => diags.into_iter().map(|d| d.message).collect(),
    }
}

#[test]
fn test_named_argument_available_names_are_declaration_names() {
    // Arrange — 实现头改名（x, y）而声明名是 a, b；调用侧写两个不存在的名字。
    // 元数对得上 ⇒ 走到 IR 的命名实参检查（typecheck 只结算元数）。
    const SRC: &str = "f: (a: Int, b: String) -> Int = (x, y) => x\n\
         r: Int = f(c = 1, d = \"s\")";

    // Act
    let messages = generate_ir_errors(SRC);

    // Assert — available 列表 = 声明名集合（a, b）：实现头名 x/y 不得冒充可用名
    //（旧口径列 `x, y`，正是 6.8.2 记的「available: b 而第 0 槽是 a 的槽」那类误报）。
    assert!(
        messages.iter().any(|m| m.contains("(available: a, b)")),
        "available 必须列出声明名集合 (a, b)，实际消息：{messages:?}"
    );
    assert!(
        !messages.iter().any(|m| m.contains("available: x")),
        "实现头名不得进入可用名集合，实际消息：{messages:?}"
    );
}

#[test]
fn test_method_named_argument_available_names_are_declaration_names() {
    // Arrange — 方法绑定实现头改名（m, q）而声明名是 n, a；调用侧写不存在的名字。
    const SRC: &str = "Temp: Type = { v: Int }\n\
         Temp.pick: (self: &Temp, n: Int, a: Int) -> Int = (self, m, q) => m\n\
         main: () -> Int = {\n\
             t = Temp(1)\n\
             return t.pick(z = 1, w = 2)\n\
         }\n\
         r: Int = main()";

    // Act — 方法形态的命名实参在 **typecheck** 阶段结算（普通函数在 IR 阶段）
    let messages = typecheck_errors(SRC);

    // Assert — 方法形态同样按声明名列出（接收者位不参与可用名集合）。
    assert!(
        messages.iter().any(|m| m.contains("(available: n, a)")),
        "方法形态的 available 必须列出声明名集合 (n, a)，实际消息：{messages:?}"
    );
}

#[test]
fn test_method_binding_named_argument_uses_declared_name() {
    // Arrange & Act：方法绑定实现头改名（self/v），调用侧用声明名传命名实参
    let module = generate_ir(
        "Point: Type = { x: Int }\n\
         Point.with_x: (self: &Point, x: Int) -> Int = (self, v) => v\n\
         main: () -> Void = {\n\
             p: Point = Point(1)\n\
             y: Int = p.with_x(x = 2)\n\
         }",
    );

    // Assert：方法函数照常生成（旧口径按实现头名 v 判，报 E1014）
    assert!(
        module.functions.iter().any(|f| f.name == "Point.with_x"),
        "方法绑定的命名实参同样按声明名绑定，IR 应生成 Point.with_x"
    );
}
