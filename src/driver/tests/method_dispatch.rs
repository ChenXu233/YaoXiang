//! 跨单元同名类型的方法派发归因（WBS 4.9.8 / issue #465，D59 C 类）。
//!
//! 判据：两个编译单元各自定义**同名类型** `Temp` 与同名方法 `Temp.tag`，入口用
//! 内联别名（`use b.{Temp as TempB}`）或模块限定构造（`b.Temp(..)`）引入另一个。
//! **构造**一向正确（字段读到的值是对的），但派发名此前取「裸名归属单元」——
//! typecheck 的 `MonoType` 对导入类型只留定义侧裸名（`Temp`），两单元的同名类型
//! 分不出来，派发名于是被本单元 `use` 面上的**裸名提供者**（a）接管。修复后派发名
//! 按**接收者写出的类型名**归因：写出名 → 本单元导入面（`use_aliases`）与
//! [`crate::frontend::module::resolver::Resolver`] 的命名空间解析（单点）→ 定义
//! 单元的限定名（`b.Temp.tag`）。
//!
//! 红态（修复前实测，同一夹具）：`x=2.0 y=3.0`（y 走了 a 的 `v+1.0`，应 20.0）——静默错值。
//! 警告面与清单可读性有关（清单缺必填字段时会误报 `W1002`，见 029f 残余边界 4），不在本文件判据内。
//! 判据落两层：合并 IR 的**调用名**必须
//! 是定义单元的限定名（结构层），以及两个实例的**值**各走各自单元的实现（行为层）。

use super::driver::{make_script_project, run_multi, run_merged_slot_values};
use crate::middle::core::ir::{ConstValue, Instruction, Operand};

/// a 单元：`Temp.tag` = v + 1.0（与 b 的实现可区分——两单元实现相同时测试对缺陷是盲的）。
const UNIT_A: &str = "Temp: Type = { v: Float }\n\nTemp.tag: (self: &Temp) -> Float = (self) => {\n    self.v + 1.0\n}\n";

/// b 单元：同名类型、同名方法，实现 = v * 10.0。
const UNIT_B: &str = "Temp: Type = { v: Float }\n\nTemp.tag: (self: &Temp) -> Float = (self) => {\n    self.v * 10.0\n}\n";

/// 内联别名形态的入口（无 manifest ⇒ Script 语义：顶层绑定即程序体，值进全局槽位）。
const ENTRY_ALIAS: &str = "use a.{Temp}\nuse b.{Temp as TempB}\n\nt = Temp(1.0)\nu = TempB(2.0)\nx = t.tag()\ny = u.tag()\n";

/// 模块限定构造形态的入口。
const ENTRY_QUALIFIED: &str =
    "use a.{Temp}\nuse b\n\nt = Temp(1.0)\nu = b.Temp(2.0)\nx = t.tag()\ny = u.tag()\n";

/// 合并 IR 里全部 Call 的被调名（结构层判据的读取面）。
///
/// Script 形态下顶层绑定（`x = t.tag()`）生成在 `init` 段而不是函数体，故两处都读。
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

/// WBS 4.9.8 / #465 主判据（内联别名形态）：派发名落**定义单元** b。
#[test]
fn test_cross_unit_same_named_type_alias_dispatch_uses_defining_unit() {
    // Arrange：a 与 b 各定义同名类型 Temp + 同名方法 tag，入口经内联别名引入 b 的类型
    let (_dir, entry) =
        make_script_project(&[("a.yx", UNIT_A), ("b.yx", UNIT_B), ("main.yx", ENTRY_ALIAS)]);

    // Act
    let outcome = run_multi(&entry).expect("driver run failed");

    // Assert（结构）：调用名是定义单元的限定名 b.Temp.tag（修复前是 a.Temp.tag）
    assert!(
        outcome.failure.is_none(),
        "同名类型经内联别名引入不得编译失败：{:?}",
        outcome.failure
    );
    let merged = outcome.module.expect("Linking+mono must produce IR");
    let callees = call_names(&merged);
    let count = |name: &str| callees.iter().filter(|c| c.as_str() == name).count();
    assert_eq!(
        count("a.Temp.tag"),
        1,
        "裸名引入的实例必须派发到 a（a.Temp.tag 恰好一次），实际调用名：{callees:?}"
    );
    assert_eq!(
        count("b.Temp.tag"),
        1,
        "别名引入的实例必须派发到定义单元 b（b.Temp.tag 恰好一次；修复前是第二次 a.Temp.tag），实际调用名：{callees:?}"
    );

    // Assert（行为）：t 走 a 的 v+1.0=2.0，u 走 b 的 v*10.0=20.0
    let values = run_merged_slot_values(merged);
    assert!(
        values.iter().any(|v| v == "Float(2.0)"),
        "t = Temp(1.0) 应得 a 的实现 2.0，实际全局值：{values:?}"
    );
    assert!(
        values.iter().any(|v| v == "Float(20.0)"),
        "u = TempB(2.0) 应得 b 的实现 20.0（修复前是 3.0 = a 的实现），实际全局值：{values:?}"
    );
}

/// WBS 4.9.8 / #465（模块限定构造形态）：`b.Temp(..)` 之后的方法调用同样归 b。
#[test]
fn test_cross_unit_same_named_type_qualified_dispatch_uses_defining_unit() {
    // Arrange：b 经 `use b` 整体导入，类型用限定名构造
    let (_dir, entry) = make_script_project(&[
        ("a.yx", UNIT_A),
        ("b.yx", UNIT_B),
        ("main.yx", ENTRY_QUALIFIED),
    ]);

    // Act
    let outcome = run_multi(&entry).expect("driver run failed");

    // Assert：调用名与值都落在定义单元 b 上
    assert!(
        outcome.failure.is_none(),
        "同名类型经模块限定构造引入不得编译失败：{:?}",
        outcome.failure
    );
    let merged = outcome.module.expect("Linking+mono must produce IR");
    let callees = call_names(&merged);
    assert!(
        callees.iter().any(|n| n == "b.Temp.tag"),
        "模块限定构造的实例必须派发到定义单元 b（b.Temp.tag），实际调用名：{callees:?}"
    );
    let values = run_merged_slot_values(merged);
    assert!(
        values.iter().any(|v| v == "Float(20.0)"),
        "u = b.Temp(2.0) 应得 b 的实现 20.0，实际全局值：{values:?}"
    );
}

/// 阴性对照：两单元类型名不同 ⇒ 派发名各自限定（不得因本次归因改动而互相串味）。
#[test]
fn test_cross_unit_distinct_type_names_keep_their_own_dispatch() {
    // Arrange：b 的类型叫 Other
    let other: &str = "Other: Type = { v: Float }\n\nOther.tag: (self: &Other) -> Float = (self) => {\n    self.v * 10.0\n}\n";
    let entry_src: &str =
        "use a.{Temp}\nuse b.{Other}\n\nt = Temp(1.0)\nu = Other(2.0)\nx = t.tag()\ny = u.tag()\n";
    let (_dir, entry) =
        make_script_project(&[("a.yx", UNIT_A), ("b.yx", other), ("main.yx", entry_src)]);

    // Act
    let outcome = run_multi(&entry).expect("driver run failed");

    // Assert
    assert!(
        outcome.failure.is_none(),
        "类型名不同不得编译失败：{:?}",
        outcome.failure
    );
    let values = run_merged_slot_values(outcome.module.expect("must produce IR"));
    assert!(
        values.iter().any(|v| v == "Float(2.0)") && values.iter().any(|v| v == "Float(20.0)"),
        "不同类型名应各走自己的实现（2.0 / 20.0），实际全局值：{values:?}"
    );
}

/// WBS 4.9.8 修复轮 R1 / issue #465：**算子接口**（Index / Equal）派发同样必须归定义单元。
///
/// 红态（修复前实测）：两单元同名 `Temp` 各实现 `Index` / `Equal`，入口经内联别名引入 b
/// ⇒ `u[0]` 得 3.0（应 20.0）、`u == t` 得 true（应 false）——两臂拼调用名时用的是
/// typecheck 过来的**短类型名**，不经 `method_dispatch_name`。
#[test]
fn test_cross_unit_same_named_type_operator_arms_dispatch_use_defining_unit() {
    // Arrange：两单元同名类型各带 Index / Equal 实现，实现可区分（a = v+1/true、b = v*10/false）
    let index_a: &str = "Temp: Type = { v: Float, Index(Temp, Int, Float), Equal(Temp, Temp) }\n\nTemp.index: (self: &Temp, key: &Int) -> Float = (self, key) => {\n    self.v + 1.0\n}\n\nTemp.equal: (self: &Temp, other: &Temp) -> Bool = (self, other) => {\n    true\n}\n";
    let index_b: &str = "Temp: Type = { v: Float, Index(Temp, Int, Float), Equal(Temp, Temp) }\n\nTemp.index: (self: &Temp, key: &Int) -> Float = (self, key) => {\n    self.v * 10.0\n}\n\nTemp.equal: (self: &Temp, other: &Temp) -> Bool = (self, other) => {\n    false\n}\n";
    let entry_src: &str = "use a.{Temp}\nuse b.{Temp as TempB}\n\nt = Temp(1.0)\nu = TempB(2.0)\nxi = u[0]\nne = u == t\nre = t == u\n";
    let (_dir, entry) =
        make_script_project(&[("a.yx", index_a), ("b.yx", index_b), ("main.yx", entry_src)]);

    // Act
    let outcome = run_multi(&entry).expect("driver run failed");

    // Assert：两个算子臂的调用名都落在定义单元 b 上
    assert!(
        outcome.failure.is_none(),
        "算子接口形态不得编译失败：{:?}",
        outcome.failure
    );
    let merged = outcome.module.expect("Linking+mono must produce IR");
    let callees = call_names(&merged);
    let count = |name: &str| callees.iter().filter(|c| c.as_str() == name).count();
    assert_eq!(
        count("b.Temp.index"),
        1,
        "别名引入实例的下标读取必须派发到 b.Temp.index，实际调用名：{callees:?}"
    );
    assert_eq!(
        count("b.Temp.equal"),
        1,
        "左操作数为 b 侧实例的 == 必须派发到 b.Temp.equal，实际调用名：{callees:?}"
    );

    // Assert（行为）：u[0] = 2.0*10.0 = 20.0；u == t 走 b 的 false
    let values = run_merged_slot_values(merged);
    assert!(
        values.iter().any(|v| v == "Float(20.0)"),
        "u[0] 应为 b 的实现 20.0（修复前是 a 的 3.0），实际全局值：{values:?}"
    );
    assert!(
        values.iter().any(|v| v == "Bool(false)"),
        "u == t（左操作数 = b 侧）应为 b 的 false（修复前是 a 的 true），实际全局值：{values:?}"
    );
    assert!(
        values.iter().any(|v| v == "Bool(true)"),
        "t == u（左操作数 = a 侧）应走 a 的 true（验收表述的方向；修复前亦成立），实际全局值：{values:?}"
    );
}

/// R1 阴性对照：**本地类型**（非导入）的 Index / Equal 两臂行为不变。
#[test]
fn test_local_type_operator_arms_keep_own_dispatch() {
    // Arrange：同文件一种类型，带 Index / Equal 实现
    let src: &str = "Temp: Type = { v: Float, Index(Temp, Int, Float), Equal(Temp, Temp) }\n\nTemp.index: (self: &Temp, key: &Int) -> Float = (self, key) => {\n    self.v + 1.0\n}\n\nTemp.equal: (self: &Temp, other: &Temp) -> Bool = (self, other) => {\n    true\n}\n\nt = Temp(1.0)\nt2 = Temp(9.0)\nxi = t[0]\nne = t == t2\n";
    let (_dir, entry) = make_script_project(&[("main.yx", src)]);

    // Act
    let outcome = run_multi(&entry).expect("driver run failed");

    // Assert：本地类型仍然派发到本单元的实现（1.0+1.0 / true）
    assert!(
        outcome.failure.is_none(),
        "本地类型算子形态不得编译失败：{:?}",
        outcome.failure
    );
    let values = run_merged_slot_values(outcome.module.expect("must produce IR"));
    assert!(
        values.iter().any(|v| v == "Float(2.0)"),
        "本地类型 t[0] 应得 2.0，实际全局值：{values:?}"
    );
    assert!(
        values.iter().any(|v| v == "Bool(true)"),
        "本地类型 t == t2 应走本单元 equal（true），实际全局值：{values:?}"
    );
}

/// R4 补钉①：两条 `use` 换序不改变归因（赢家不是「先出现的 import」）。
#[test]
fn test_use_order_does_not_change_cross_unit_dispatch() {
    // Arrange：`use b.{Temp as TempB}` 写在前面，`use a.{Temp}` 在后
    let entry_src: &str = "use b.{Temp as TempB}\nuse a.{Temp}\n\nt = Temp(1.0)\nu = TempB(2.0)\nx = t.tag()\ny = u.tag()\n";
    let (_dir, entry) =
        make_script_project(&[("a.yx", UNIT_A), ("b.yx", UNIT_B), ("main.yx", entry_src)]);

    // Act
    let outcome = run_multi(&entry).expect("driver run failed");

    // Assert：仍是 t→a（2.0）、u→b（20.0）
    let values = run_merged_slot_values(outcome.module.expect("must produce IR"));
    assert!(
        values.iter().any(|v| v == "Float(2.0)") && values.iter().any(|v| v == "Float(20.0)"),
        "use 顺序不得改变归因（2.0 / 20.0），实际全局值：{values:?}"
    );
}

/// R4 补钉②：两单元方法体对调 ⇒ 值跟着**单元**走（不是跟着调用点位置走）。
#[test]
fn test_swapped_method_bodies_follow_the_defining_unit() {
    // Arrange：a = v*10、b = v+1（与常规夹具相反）
    let unit_a_swapped: &str = "Temp: Type = { v: Float }\n\nTemp.tag: (self: &Temp) -> Float = (self) => {\n    self.v * 10.0\n}\n";
    let unit_b_swapped: &str = "Temp: Type = { v: Float }\n\nTemp.tag: (self: &Temp) -> Float = (self) => {\n    self.v + 1.0\n}\n";
    let (_dir, entry) = make_script_project(&[
        ("a.yx", unit_a_swapped),
        ("b.yx", unit_b_swapped),
        ("main.yx", ENTRY_ALIAS),
    ]);

    // Act
    let outcome = run_multi(&entry).expect("driver run failed");

    // Assert：t = 1.0 走 a 的 *10 = 10.0；u = 2.0 走 b 的 +1 = 3.0
    let values = run_merged_slot_values(outcome.module.expect("must produce IR"));
    assert!(
        values.iter().any(|v| v == "Float(10.0)") && values.iter().any(|v| v == "Float(3.0)"),
        "方法体对调后值必须跟着单元走（10.0 / 3.0），实际全局值：{values:?}"
    );
}

/// R4 补钉③（关键）：**裸名属主是字典序靠后的 `z.yx`** ⇒ 赢家仍是裸名属主 z，
/// 不是按文件名/发现序的第一位（证明归因来自 `use` 面而不是排序）。
#[test]
fn test_bare_name_owner_is_z_unit_not_lexicographic_first() {
    // Arrange：a = v*10（经内联别名引入）、z = v+1（经裸名引入，字典序在 a 之后）
    let (_dir, entry) = make_script_project(&[
        ("a.yx", UNIT_B),
        ("z.yx", UNIT_A),
        ("main.yx", "use z.{Temp}\nuse a.{Temp as TempA}\n\nt = Temp(1.0)\nu = TempA(2.0)\nx = t.tag()\ny = u.tag()\n"),
    ]);

    // Act
    let outcome = run_multi(&entry).expect("driver run failed");

    // Assert：t（裸名 → z，v+1）= 2.0；u（别名 → a，v*10）= 20.0
    assert!(
        outcome.failure.is_none(),
        "裸名属主为 z 的形态不得编译失败：{:?}",
        outcome.failure
    );
    let merged = outcome.module.expect("Linking+mono must produce IR");
    let callees = call_names(&merged);
    assert_eq!(
        callees
            .iter()
            .filter(|c| c.as_str() == "z.Temp.tag")
            .count(),
        1,
        "裸名引入的实例必须派发到 z（z.Temp.tag），实际调用名：{callees:?}"
    );
    assert_eq!(
        callees
            .iter()
            .filter(|c| c.as_str() == "a.Temp.tag")
            .count(),
        1,
        "别名引入的实例必须派发到 a（a.Temp.tag），实际调用名：{callees:?}"
    );
    let values = run_merged_slot_values(merged);
    assert!(
        values.iter().any(|v| v == "Float(2.0)") && values.iter().any(|v| v == "Float(20.0)"),
        "z 侧 +1 与 a 侧 *10 应各得 2.0 / 20.0，实际全局值：{values:?}"
    );
}

/// R4 补钉④：**只调别名侧**（入口没有 a 侧调用点）⇒ 仍派发到 b。
#[test]
fn test_alias_only_call_site_still_dispatches_to_defining_unit() {
    // Arrange：没有 `t = Temp(1.0)`，只有别名引入的实例
    let (_dir, entry) = make_script_project(&[
        ("a.yx", UNIT_A),
        ("b.yx", UNIT_B),
        (
            "main.yx",
            "use a.{Temp}\nuse b.{Temp as TempB}\n\nu = TempB(2.0)\ny = u.tag()\n",
        ),
    ]);

    // Act
    let outcome = run_multi(&entry).expect("driver run failed");

    // Assert：b 的实现 = 20.0（修复前是 a 的 3.0）
    let values = run_merged_slot_values(outcome.module.expect("must produce IR"));
    assert!(
        values.iter().any(|v| v == "Float(20.0)"),
        "只调别名侧时应得 b 的实现 20.0，实际全局值：{values:?}"
    );
}

/// R4 补钉⑤（阴性对照）：**同文件两种类型**各带同名方法 ⇒ 各走各的（不误伤）。
#[test]
fn test_same_file_two_types_keep_their_own_methods() {
    // Arrange：同一文件内 Temp（v+1）与 Other（v*10）
    let src: &str = "Temp: Type = { v: Float }\n\nTemp.tag: (self: &Temp) -> Float = (self) => {\n    self.v + 1.0\n}\n\nOther: Type = { v: Float }\n\nOther.tag: (self: &Other) -> Float = (self) => {\n    self.v * 10.0\n}\n\nt = Temp(1.0)\nu = Other(2.0)\nx = t.tag()\ny = u.tag()\n";
    let (_dir, entry) = make_script_project(&[("main.yx", src)]);

    // Act
    let outcome = run_multi(&entry).expect("driver run failed");

    // Assert：2.0 / 20.0（同文件两类型本来就正确，改动不得串味）
    let values = run_merged_slot_values(outcome.module.expect("must produce IR"));
    assert!(
        values.iter().any(|v| v == "Float(2.0)") && values.iter().any(|v| v == "Float(20.0)"),
        "同文件两类型应各走自己的实现（2.0 / 20.0），实际全局值：{values:?}"
    );
}
