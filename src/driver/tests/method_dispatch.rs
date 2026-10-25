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
//! 红态（修复前实测，同一夹具）：`x=2.0 y=3.0`（y 走了 a 的 `v+1.0`，应 20.0）、
//! `check` 0 error / 0 warning——静默错值。判据落两层：合并 IR 的**调用名**必须
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
