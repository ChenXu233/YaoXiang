//! L3-1 收尾：lower 的类型表按**身份**取（同名跨模块）
//!
//! 判据（D64；设计 §2.5 执行口径「身份不得从名字串反解」）：
//! - 字段索引按类型身份取该定义自己的字段序（a.T{b} 与 b.T{x,y} 各用各的）；
//! - 和类型变体序按类型身份取（a.Color{red,blue} 与 b.Color{green,cyan} 各用各的）。
//!
//! 红态：旧实现（裸名 last-wins）下两个夹具分别报 E3005「无法解析字段索引」
//! 与 unsupported 变体——两个同名定义共用一条表项。

use super::driver::{make_script_project, run_merged_slot_values, run_multi};

/// 字段序不同的两个同名结构体：a.T 只有 v，b.T 是 x 在前、y 在后。
const STRUCT_A: &str = "T: Type = { v: Float }\n";
const STRUCT_B: &str = "T: Type = { x: Float, y: Float }\n";

#[test]
fn test_multifile_same_named_struct_field_index_uses_definition() {
    // Arrange - 两个同名类型经别名导入，各自字段位置不同（v=0 / y=1）
    let main = "use a.{T}\nuse b.{T as TB}\n\nt = T(9.0)\nu = TB(1.0, 2.0)\ntv = t.v\nuy = u.y\n";
    let (_dir, entry) =
        make_script_project(&[("a.yx", STRUCT_A), ("b.yx", STRUCT_B), ("main.yx", main)]);

    // Act
    let outcome = run_multi(&entry).expect("driver run failed");

    // Assert - 两种字段访问都按各自定义解析（旧实现缺 v 或 y 即 E3005）
    assert!(
        outcome.failure.is_none(),
        "同名跨模块类型的字段访问必须按身份取各自字段序：{:?}",
        outcome.failure
    );
    let values = run_merged_slot_values(outcome.module.expect("must produce IR"));
    assert!(
        values.iter().any(|v| v == "Float(9.0)"),
        "t.v 必须是 a.T 的 v（9.0），实际全局值：{values:?}"
    );
    assert!(
        values.iter().any(|v| v == "Float(2.0)"),
        "u.y 必须是 b.T 的第二个字段 y（2.0），实际全局值：{values:?}"
    );
}

/// 变体名不重叠的两个同名和类型：a.Color{red,blue}、b.Color{green,cyan}。
const SUM_A: &str = "Color: Type = {\n    red: () -> Color,\n    blue: () -> Color,\n}\n";
const SUM_B: &str = "Color: Type = {\n    green: () -> Color,\n    cyan: () -> Color,\n}\n";

#[test]
fn test_multifile_same_named_sum_variants_use_definition() {
    // Arrange - 两个同名和类型经别名导入，变体名互不相同（裸名表只有一条）
    let main = "use a.{Color as ColorA}\nuse b.{Color as ColorB}\n\nca = ColorA.red()\ncb = ColorB.green()\nra = match ca {\n    red() -> 1,\n    blue() -> 2,\n}\nrb = match cb {\n    green() -> 3,\n    cyan() -> 4,\n}\n";
    let (_dir, entry) = make_script_project(&[("a.yx", SUM_A), ("b.yx", SUM_B), ("main.yx", main)]);

    // Act
    let outcome = run_multi(&entry).expect("driver run failed");

    // Assert - 两次 match 各按自己的变体序编译（旧实现共用一条表项即 unsupported）
    assert!(
        outcome.failure.is_none(),
        "同名跨模块和类型的变体匹配必须按身份取各自变体序：{:?}",
        outcome.failure
    );
    let values = run_merged_slot_values(outcome.module.expect("must produce IR"));
    assert!(
        values.iter().any(|v| v == "Int(1)"),
        "a.Color.red 必须命中 a 的变体序（ra=1），实际全局值：{values:?}"
    );
    assert!(
        values.iter().any(|v| v == "Int(3)"),
        "b.Color.green 必须命中 b 的变体序（rb=3），实际全局值：{values:?}"
    );
}
