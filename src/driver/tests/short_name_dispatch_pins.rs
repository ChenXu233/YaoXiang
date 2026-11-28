//! 两条未收敛短名派发臂的「不可达性」钉测试（WBS 3.18.3 / issue #506）。
//!
//! 判据门：**病史**（issue #506 实测的两条不可达现状，本批 2026-11-20 在
//! revision 9d117bd7 独立复测同型复现）+ WBS 3.18.3 终态判据「补不可达性钉
//! 测试（可达性条件变化时变红）」；Try 语义条款见 RFC-011b 阶段 2（`?` 由
//! Try 接口驱动，四方法 is_failure/success/residual/from_error）。
//!
//! 两条臂此前都按**短类型名**拼调用名、不经单一判据（WBS 4.9.8 / #465 收敛的
//! 跨单元归因判据），本批已改为同经判据；WBS 4.9.10 起该判据的载体是 typecheck
//! 写进 `OperatorDispatch::dispatch_name` 的 IR 最终名（接收者身份 → 方法身份 →
//! D-8 名字），lower 侧不再按短名查表。两条
//! **可达性前提**照旧钉住——判据放开时这里先红，收敛动作不得绕过：
//! ① `print`/`println` 零开销 `to_string` 派发（`src/middle/lower/expr/call.rs`
//!    「零开销路径：直接调用 to_string 方法」臂）——对用户结构体**不可达**：
//!    `type_implements_stringable` 只认 `MonoType::Struct.methods` 里的
//!    `to_string` 条目，而该表在类型推断的各构造点恒为空（用户方法不经它
//!    登记），实测用户结构体带 `to_string` 仍走兜底 repr
//!    （`std.io.format_fallback`）。当 3.18 线放开 `type_implements_stringable`
//!    （或该方法派发被激活）时，零开销臂会开始产出 `{Type}.to_string` 短名
//!    调用 → 钉 ① 变红；
//! ② `Try` 四方法派发链（`src/middle/lower/expr/op.rs` `generate_try_expr_ir`）
//!    ——**别名导入的**用户 Try 类型此前不可达：实现登记按定义侧类型名键控，
//!    `use lib.{Maybe as MaybeB}` 后 `?` 的接收者叶子名是别名 `MaybeB`，
//!    typecheck `E1082` 硬错先挡。D64（类型表示层身份）起别名解析到定义侧身份
//!    `lib.Maybe`，可达性前提消失；钉 ② 改为钉**终点名**（`lib.Maybe.*`，
//!    回落别名叶子名即翻红）。
//!
//! 与 #465 同族的病理（放开任一前置即成静默错派）：短类型名是裸名义名，不
//! 携带定义模块；跨单元同名类型按裸名派发会落到**导入面**提供者（#465 红态
//! 实测静默错值）。本批收敛后两臂已同经**方法身份**判据（接收者身份 → 定义
//! 单元限定名，WBS 4.9.10 D-8 的 IR 最终名），但可达性前提仍被钉住——前提
//! 放开时判据面先红。
//!
//! 另附三条**未修分支对照**（可达路径现状，防收敛改动本身带漂移）：
//! ③ 基础类型 `print(x)` 走零开销臂（`type_implements_stringable` 对
//!    Int/Float/Bool/Char/String/Bytes 恒真；实参须携带类型信息——裸字面量
//!    `print(42)` 无类型信息走默认处理臂，实测）→ 调用名 `int.to_string`
//!    （std.convert 原生注册，`src/std/convert.rs`）；
//! ④ 同单元用户 Try 类型 `?` 的终点名带本单元键（合并 IR 里 `qualify_names`
//!    把本单元函数与调用统一限定为 `{module_key}.…`）；
//! ⑤ 非别名跨单元用户 Try 类型 `?` 今天可达（实现登记按定义侧名可查到），
//!    终点名是定义单元限定名（`lib.Maybe.is_failure`）——收敛后降出的
//!    已限定名经 `resolve_call_target` 原样通过，终点不变。

use super::driver::{make_script_project, run_merged_slot_values, run_multi};
use crate::middle::ir::ir::{ConstValue, Instruction, Operand};

/// 合并 IR 里全部静态 Call 的被调名（函数体 + init 段；与 method_dispatch.rs 同一读取面）。
fn call_names(merged: &crate::middle::ModuleIR) -> Vec<String> {
    let of = |instr: &Instruction| match instr {
        Instruction::Call {
            func: Operand::Const(ConstValue::String(name)),
            ..
        } => Some(name.clone()),
        _ => None,
    };
    let from_fns = merged.functions.iter().flat_map(|f| f.all_instructions());
    let from_init = merged.init.iter();
    from_fns.chain(from_init).filter_map(of).collect()
}

/// 用户结构体定义 + `to_string` 方法 + print 调用（Script 形态：顶层即程序体）。
const USER_STRUCT_TO_STRING: &str = "\
Point: Type = { x: Int }

Point.to_string: (self: &Point) -> String = (self) => {
    \"P\"
}

p = Point(1)
print(p)
";

/// 钉 ①：用户结构体带 `to_string` 方法，print 仍走兜底 repr——不得出现短名
/// `Point.to_string` 调用。3.18 线放开 `type_implements_stringable` 时本测试变红
/// （先红已实测：判据临时放开 → 零开销臂产出 `Point.to_string`，判据恢复 → 绿）。
#[test]
fn test_print_user_struct_with_to_string_keeps_fallback_repr() {
    // Arrange：单文件 Script 项目，结构体带 to_string 方法
    let (_dir, entry) = make_script_project(&[("main.yx", USER_STRUCT_TO_STRING)]);

    // Act
    let outcome = run_multi(&entry).expect("driver run failed");
    assert!(
        outcome.failure.is_none(),
        "夹具本身必须编译通过（判据是 IR 形态不是编译拒绝）：{:?}",
        outcome.failure
    );
    let callees = call_names(&outcome.module.expect("must produce IR"));

    // Assert：兜底 repr 在（print 对用户结构体的现行路径）；短名零开销派发不在
    assert!(
        callees.iter().any(|n| n == "std.io.format_fallback"),
        "print(用户结构体) 现行走 std.io.format_fallback 兜底 repr（type_implements_stringable 对用户结构体为假），实际调用名：{callees:?}"
    );
    assert!(
        !callees.iter().any(|n| n.contains("to_string")),
        "用户结构体的 to_string 不得被零开销臂按短名派发（Point.to_string 是 #465 同族短名形态；type_implements_stringable 放开时本判据变红，届时调用名须同经方法身份判据 D-8），实际调用名：{callees:?}"
    );
}

/// 对照 ③：基础类型实参走零开销臂（可达路径现状）——调用名 `int.to_string`。
/// 这是钉 ① 的未修分支对照：Stringable 判据对基础类型恒真，本测试钉住该路径
/// 在两臂收敛后**调用名逐字不变**（短名经身份判据后对基础类型必须原样
/// 返回——内建类型无模块归属 ⇒ 投影即裸名，D-1）。
#[test]
fn test_print_int_variable_uses_zero_cost_to_string_dispatch() {
    // Arrange：最小 Script 项目，print 一个带类型信息的整型变量
    let (_dir, entry) = make_script_project(&[("main.yx", "x: Int = 42\nprint(x)\n")]);

    // Act
    let outcome = run_multi(&entry).expect("driver run failed");
    assert!(
        outcome.failure.is_none(),
        "print(整型变量) 不得编译失败：{:?}",
        outcome.failure
    );
    let callees = call_names(&outcome.module.expect("must produce IR"));

    // Assert：零开销臂产出 int.to_string（std.convert 注册的短名），再转 std.io.print
    assert!(
        callees.iter().any(|n| n == "int.to_string"),
        "print(整型变量) 应走零开销臂直接调用 int.to_string（std.convert 注册名），实际调用名：{callees:?}"
    );
    assert!(
        callees.iter().any(|n| n == "std.io.print"),
        "零开销臂之后应继续调用 std.io.print 输出，实际调用名：{callees:?}"
    );
}

/// lib 单元：用户和类型 Maybe 实现 Try 四方法（与语料 user_try_type.yx 同构）。
const TRY_LIB: &str = "\
use std.assert

Maybe: Type = {
    just: (Int) -> Maybe,
    nothing: () -> Maybe,
    Try(Maybe, Int, String),
}

Maybe.is_failure: (self: &Maybe) -> Bool = (self) => {
    match self {
        nothing() -> true,
        just(_) -> false,
    }
}

Maybe.success: (self: &Maybe) -> Int = (self) => {
    match self {
        just(v) -> v,
        nothing() -> assert.assert(false),
    }
}

Maybe.residual: (self: &Maybe) -> String = (self) => {
    match self {
        nothing() -> \"empty\",
        just(_) -> assert.assert(false),
    }
}

Maybe.from_error: (e: String) -> Maybe = (e) => {
    match e {
        _ -> Maybe.nothing(),
    }
}
";

/// 别名导入用户 Try 类型 + `?` 的入口单元：接收者叶子名是别名（`MaybeB`），
/// 实现登记按定义侧名（`Maybe`）键控——E1082 的复现形态（issue #506 实测同型）。
const ALIASED_TRY_ENTRY: &str = "\
use lib.{Maybe as MaybeB}

bump: (m: &MaybeB) -> MaybeB = (m) => {
    v = m?
    return MaybeB.just(v + 1)
}

r = MaybeB.just(4)
t = bump(&r)
";

/// 钉 ②（D64 语义更新）：**别名导入的**用户 Try 类型与定义侧是**同一身份**
/// （`lib.Maybe`），`?` 在导入单元因此可达——终点名必须同经方法身份判据
/// 落到定义单元的限定名（`lib.Maybe.*`），不得回落别名叶子名（`MaybeB.*`，
/// #465 同族短名病理）。
///
/// 历史：L3 之前别名导入时实现登记按定义侧名键控、typecheck 查不到 ⇒ E1082
/// 硬错先挡；本测试当时钉 `E1082`。D64 让类型身份随别名解析到 (lib, Maybe)，
/// 该拒绝面不再成立——本测试改为钉可达后的**派发名**（判据更强：短名回落会
/// 在此翻红）。D59 分流：**B 类收紧**（旧 E1082 是身份缺口造成的假拒绝；
/// 新判据比旧判据更强——除「可编译」外还钉终点名且禁止回落别名叶子名）。
#[test]
fn test_aliased_imported_user_try_type_dispatch_uses_definition_identity() {
    // Arrange：lib 定义 Maybe+Try，入口以别名 MaybeB 导入后在自己的函数里用 `?`
    let (_dir, entry) = make_script_project(&[("lib.yx", TRY_LIB), ("main.yx", ALIASED_TRY_ENTRY)]);

    // Act
    let outcome = run_multi(&entry).expect("driver run failed");

    // Assert：D64 身份下可编译（别名解析到定义侧身份 lib.Maybe）
    assert!(
        outcome.failure.is_none(),
        "别名导入的用户 Try 类型与定义侧同一身份（D64），? 必须可达：{:?}",
        outcome.failure
    );
    let callees = call_names(&outcome.module.expect("must produce IR"));
    // Assert：四个 Try 方法调用名 = 定义单元限定名，不得是别名叶子名
    for method in ["is_failure", "success", "residual", "from_error"] {
        let qualified = format!("lib.Maybe.{method}");
        assert!(
            callees.iter().any(|c| c == &qualified),
            "? 的 {method} 派发必须落到定义单元限定名 {qualified}（D-8），实际调用名：{callees:?}"
        );
        let aliased = format!("MaybeB.{method}");
        assert!(
            !callees.iter().any(|c| c == &aliased),
            "? 的 {method} 派发不得回落别名叶子名 {aliased}，实际调用名：{callees:?}"
        );
    }
}

/// 对照 ④：**同单元**用户 Try 类型 `?` 的终点名带本单元键（可达路径现状）。
/// 合并 IR 里 `qualify_names` 把本单元函数与调用统一限定为 `{module_key}.…`
/// （入口单元键是 main）；两臂收敛后本判据钉住终点不变——方法身份自带本单元
/// 键，名字因此与 `qualify_names` 的终点逐字相同。
#[test]
fn test_local_user_try_type_dispatch_keeps_module_qualified_names() {
    // Arrange：单文件 Script 项目，Maybe+Try 本地定义，顶层经 bump 触发 `?`
    let local_try = format!(
        "{TRY_LIB}\nbump: (m: &Maybe) -> Maybe = (m) => {{\n    v = m?\n    return Maybe.just(v + 1)\n}}\n\ns = Maybe.just(4)\nt = bump(&s)\n"
    );
    let (_dir, entry) = make_script_project(&[("main.yx", local_try.as_str())]);

    // Act
    let outcome = run_multi(&entry).expect("driver run failed");
    assert!(
        outcome.failure.is_none(),
        "同单元用户 Try 类型的 ? 必须编译通过（语料 user_try_type.yx 同形态）：{:?}",
        outcome.failure
    );
    let callees = call_names(&outcome.module.expect("must produce IR"));

    // Assert：Try 四方法链的终点名带本单元键（qualify_names 单点限定）
    assert!(
        callees.iter().any(|n| n == "main.Maybe.is_failure"),
        "同单元 Try 类型的 ? 终点名应是 main.Maybe.is_failure（qualify_names 把本单元方法统一限定），实际调用名：{callees:?}"
    );
    assert!(
        callees.iter().any(|n| n == "main.Maybe.success"),
        "成功路径终点应是 main.Maybe.success，实际调用名：{callees:?}"
    );
}

/// 对照 ⑤：**非别名**跨单元用户 Try 类型 `?` 今天可达（实现登记按定义侧名
/// 可查到），终点名是定义单元限定名 `lib.Maybe.is_failure`。收敛后降出的
/// 已限定名经 `resolve_call_target` 原样通过——本测试钉住终点不变。
#[test]
fn test_imported_user_try_type_dispatch_keeps_defining_unit_names() {
    // Arrange：lib 定义 Maybe+Try，入口按原名导入（入口不再本地重定义），函数里用 `?`
    let imported_try = "use lib.{Maybe}\n\nbump: (m: &Maybe) -> Maybe = (m) => {\n    v = m?\n    return Maybe.just(v + 1)\n}\n\nr = Maybe.just(4)\nt = bump(&r)\n";
    let (_dir, entry) = make_script_project(&[("lib.yx", TRY_LIB), ("main.yx", imported_try)]);

    // Act
    let outcome = run_multi(&entry).expect("driver run failed");
    assert!(
        outcome.failure.is_none(),
        "非别名导入的用户 Try 类型用 ? 必须编译通过（实现登记按定义侧名可查到）：{:?}",
        outcome.failure
    );
    let callees = call_names(&outcome.module.expect("must produce IR"));

    // Assert：终点名落定义单元 lib（与 #465 收敛判据同向——按接收者归属限定）
    assert!(
        callees.iter().any(|n| n == "lib.Maybe.is_failure"),
        "跨单元 Try 类型的 ? 终点名应是 lib.Maybe.is_failure（定义单元限定），实际调用名：{callees:?}"
    );
    assert!(
        callees.iter().any(|n| n == "lib.Maybe.success"),
        "成功路径终点应是 lib.Maybe.success，实际调用名：{callees:?}"
    );
}

/// 行为对照（随对照 ⑤ 的夹具）：`?` 成功路径真把载荷取出来（+1）。
/// 结构判据防名字漂移，这条防「名字对、语义断」。
#[test]
fn test_imported_user_try_type_question_propagates_payload() {
    // Arrange：与对照 ⑤ 同一夹具形状
    let imported_try = "use lib.{Maybe}\n\nbump: (m: &Maybe) -> Maybe = (m) => {\n    v = m?\n    return Maybe.just(v + 1)\n}\n\nr = Maybe.just(4)\nt = bump(&r)\n";
    let (_dir, entry) = make_script_project(&[("lib.yx", TRY_LIB), ("main.yx", imported_try)]);

    // Act
    let outcome = run_multi(&entry).expect("driver run failed");
    assert!(
        outcome.failure.is_none(),
        "编译必须通过：{:?}",
        outcome.failure
    );

    // Assert：t = just(4+1)——全局槽位里出现载荷 Int(5)（枚举值调试串含 payload 字段）
    let values = run_merged_slot_values(outcome.module.expect("must produce IR"));
    assert!(
        values.iter().any(|v| v.contains("Int(5)")),
        "`?` 成功路径应取出载荷 4 并 +1 得 5，实际全局值：{values:?}"
    );
}
