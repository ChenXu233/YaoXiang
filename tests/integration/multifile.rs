//! RFC-029 多文件模块编排端到端测试
//!
//! 验证跨文件 `use` 导入：本地模块的类型与函数能被另一个文件导入、构造、调用并正确执行。
//! 正确性用语言内 `assert` 验证——断言失败会使 `run_project` 返回 Err。
//! 发现语义（#247）：沿 use 追踪 + 双根解析（导入者目录优先，项目根兑底）。
//! 入口语义：RFC-029f（Bin 入口按绑定存在性判定、值 main 初始化期求值，#388 定案）。

/// 在临时目录写入多文件项目并运行入口文件，返回运行结果（错误为调试文本）。
///
/// 夹具带 manifest（Bin 语义）：4.10.3 后 rootless = Script 语义（不自动调
/// main，#413），而本文件测的是 RFC-029 项目编排的入口语义（Bin 自动入口）。
fn run_project(
    files: &[(&str, &str)],
    entry: &str,
) -> Result<(), String> {
    let dir = tempfile::tempdir().expect("create tempdir");
    std::fs::write(
        dir.path().join("yaoxiang.toml"),
        "[package]\nname = \"t\"\nversion = \"0.1.0\"\n",
    )
    .expect("write manifest");
    for (name, content) in files {
        let path = dir.path().join(name);
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).expect("create parent dirs");
        }
        std::fs::write(&path, content).expect("write source file");
    }
    yaoxiang::run_project(&dir.path().join(entry)).map_err(|e| format!("{:?}", e))
}

/// 同 `run_project`，任何编译/运行/断言失败都会 panic（带项目内容上下文）。
fn run_project_ok(
    files: &[(&str, &str)],
    entry: &str,
) {
    run_project(files, entry)
        .unwrap_or_else(|e| panic!("run_project failed for entry {entry}:\n{e}"));
}

#[test]
fn test_multifile_use_type_and_function() {
    let lib = r#"
Point: Type = { x: Float, y: Float }

distance: (a: Point, b: Point) -> Float = (a, b) => {
    dx = a.x - b.x
    dy = a.y - b.y
    return dx * dx + dy * dy
}
"#;
    let main = r#"
use std.assert
use lib.{Point, distance}

main = () => {
    p = Point(1.0, 2.0)
    q = Point(4.0, 6.0)
    d = distance(p, q)
    assert.assert(d == 25.0, "distance should be 25")
}
"#;
    run_project_ok(&[("lib.yx", lib), ("main.yx", main)], "main.yx");
}

#[test]
fn test_multifile_use_type_only() {
    let lib = r#"
Point: Type = { x: Float, y: Float }
"#;
    let main = r#"
use std.assert
use lib.{Point}

main = () => {
    p = Point(3.0, 4.0)
    assert.assert(p.x == 3.0, "p.x should be 3")
    assert.assert(p.y == 4.0, "p.y should be 4")
}
"#;
    run_project_ok(&[("lib.yx", lib), ("main.yx", main)], "main.yx");
}

#[test]
fn test_multifile_use_constant() {
    let lib = r#"
value: Int = 7
"#;
    let main = r#"
use std.assert
use lib.{value}

main = () => {
    assert.assert(value == 7, "value should be 7")
}
"#;
    run_project_ok(&[("lib.yx", lib), ("main.yx", main)], "main.yx");
}

#[test]
fn test_multifile_use_type_with_method() {
    // 跨文件方法绑定：lib.yx 定义 Point 及其方法，main.yx 导入 Point 并调用方法。
    // 验证跨文件 type_bindings（方法调用脱糖需要的绑定信息）被预注册到 IR 生成上下文。
    let lib = r#"
Point: Type = { x: Float, y: Float }

Point.get_x: (self: &Point) -> Float = {
    return self.x
}

Point.norm_sq: (self: &Point) -> Float = {
    return self.x * self.x + self.y * self.y
}
"#;
    let main = r#"
use std.assert
use lib.{Point}

main = () => {
    p = Point(3.0, 4.0)
    x = p.get_x()
    n = p.norm_sq()
    assert.assert(x == 3.0, "cross-file get_x should return 3.0")
    assert.assert(n == 25.0, "cross-file norm_sq should return 25.0")
}
"#;
    run_project_ok(&[("lib.yx", lib), ("main.yx", main)], "main.yx");
}

#[test]
fn test_single_file_via_project_path() {
    // 单文件项目（无本地 use）走编排路径也应正常
    let main = r#"
use std.assert

main = () => {
    assert.assert(42 == 42, "trivial")
}
"#;
    run_project_ok(&[("main.yx", main)], "main.yx");
}

#[test]
fn test_multifile_same_name_coexist() {
    // 两个文件各自定义同名顶层函数 helper——module=record 语义下它们是不同
    // record 的字段（a.helper / b.helper），限定名使其在扁平函数表中共存。
    // 各自内部调用 helper() 必须解析到自己的那份（限定前会静默互相覆盖，
    // 导致 use_a/use_b 返回同一个值）。
    let a = r#"
helper: () -> Int = () => {
    return 1
}

use_a: () -> Int = () => {
    return helper()
}
"#;
    let b = r#"
helper: () -> Int = () => {
    return 2
}

use_b: () -> Int = () => {
    return helper()
}
"#;
    let main = r#"
use std.assert
use a.{use_a}
use b.{use_b}

main = () => {
    assert.assert(use_a() == 1, "a.helper should return 1")
    assert.assert(use_b() == 2, "b.helper should return 2")
}
"#;
    run_project_ok(&[("a.yx", a), ("b.yx", b), ("main.yx", main)], "main.yx");
}

#[test]
fn test_multifile_whole_module_namespace_call() {
    // #243：`use lib`（整体导入）后 `lib.helper()` 限定调用。
    // typecheck 把 `lib` 注册为模块 record 变量，IR 生成经 user_namespaces
    // 识别为命名空间调用并解析为限定名 `lib.helper` / `lib.add`。
    let lib = r#"
helper: () -> Int = () => {
    return 42
}

add: (a: Int, b: Int) -> Int = (a, b) => {
    return a + b
}
"#;
    let main = r#"
use std.assert
use lib

main = () => {
    assert.assert(lib.helper() == 42, "lib.helper should return 42")
    assert.assert(lib.add(3, 4) == 7, "lib.add(3,4) should be 7")
}
"#;
    run_project_ok(&[("lib.yx", lib), ("main.yx", main)], "main.yx");
}

#[test]
fn test_multifile_same_type_name_coexist() {
    // #244：两个文件各自定义同名类型 Point——module=record 语义下 a.Point / b.Point
    // 是不同 record 的字段，限定名使构造器与 vtable 在扁平函数表中共存。
    // 各自内部构造 Point(...) 必须解析到自己的限定构造器（限定前构造器撞名）。
    // 注：把两个同名类型同时导入同一文件需要导入别名（#245，未实现），故这里
    // 验证“各自文件内部使用同名类型”这条核心路径。
    let a = r#"
Point: Type = { x: Float, y: Float }

make_a: () -> Float = () => {
    p = Point(1.0, 2.0)
    return p.x + p.y
}
"#;
    let b = r#"
Point: Type = { x: Float, z: Float }

make_b: () -> Float = () => {
    p = Point(3.0, 4.0)
    return p.x + p.z
}
"#;
    let main = r#"
use std.assert
use a.{make_a}
use b.{make_b}

main = () => {
    assert.assert(make_a() == 3.0, "a.Point(1,2) x+y should be 3.0")
    assert.assert(make_b() == 7.0, "b.Point(3,4) x+z should be 7.0")
}
"#;
    run_project_ok(&[("a.yx", a), ("b.yx", b), ("main.yx", main)], "main.yx");
}

#[test]
fn test_multifile_use_item_inline_alias() {
    // #245：`use lib.{helper as double}` 内联别名，别名参与限定调用。
    let lib = r#"
helper: (x: Int) -> Int = (x) => {
    return x * 2
}
"#;
    let main = r#"
use std.assert
use lib.{helper as double}

main = () => {
    assert.assert(double(21) == 42, "double(21) should be 42")
}
"#;
    run_project_ok(&[("lib.yx", lib), ("main.yx", main)], "main.yx");
}

#[test]
fn test_multifile_use_item_mixed_alias_and_plain() {
    // #245：混合导入——带别名项与原名项共存。
    let lib = r#"
Point: Type = { x: Float }

helper: (x: Int) -> Int = (x) => {
    return x * 2
}
"#;
    let main = r#"
use std.assert
use lib.{helper as double, Point}

main = () => {
    p = Point(3.0)
    assert.assert(p.x == 3.0, "p.x should be 3")
    assert.assert(double(21) == 42, "double(21) should be 42")
}
"#;
    run_project_ok(&[("lib.yx", lib), ("main.yx", main)], "main.yx");
}

#[test]
fn test_multifile_use_type_alias_constructor() {
    // #245：类型别名——`use lib.{Point as P}` 后构造调用经别名解析。
    let lib = r#"
Point: Type = { x: Float }
"#;
    let main = r#"
use std.assert
use lib.{Point as P}

main = () => {
    p = P(3.0)
    assert.assert(p.x == 3.0, "p.x should be 3")
}
"#;
    run_project_ok(&[("lib.yx", lib), ("main.yx", main)], "main.yx");
}

#[test]
fn test_multifile_cross_unit_method_overload_resolves_by_argument_type() {
    // WBS 4.9.7 / #464：跨单元（registry 解析）方法调用的同名重载按**实参类型**决议。
    // 判据（先红后绿）：修复前调用方 env 没有定义单元的候选表 ⇒ 决议段整段跳过、
    // 退回跨单元单值绑定表（同名定义 last-wins）⇒ p.scale(2) 落到 String 候选上
    // 报 E1002「期望 string，实际 int64」，两条调用根本无法通过编译；修复后按实参
    // 逐候选决议 ⇒ 两条调用各自命中对应候选并跑出各自的值。
    // Arrange - lib 定义两个同名同序（参数个数相同）重载，声明序 0=Int / 1=String
    let lib = r#"
Point: Type = { x: Float, y: Float }

Point.scale: (self: &Point, k: Int) -> Float = (self, k) => {
    return self.x * k
}

Point.scale: (self: &Point, s: String) -> Float = (self, s) => {
    return self.x + 100.0
}
"#;
    let main = r#"
use std.assert
use lib.{Point}

main = () => {
    p = Point(3.0, 4.0)
    a: Float = p.scale(2)
    b: Float = p.scale("x")
    assert.assert(a == 6.0, "Int 实参必须命中声明序 0 的候选（self.x * k）")
    assert.assert(b == 103.0, "String 实参必须命中声明序 1 的候选（self.x + 100.0）")
}
"#;

    // Act & Assert - 编译到执行全链：两条调用各自命中正确候选
    run_project_ok(&[("lib.yx", lib), ("main.yx", main)], "main.yx");
}

#[test]
fn test_multifile_subdir_entry_imports_project_root_module() {
    // Arrange - 入口在 tests/ 子目录，lib 在项目根（yaoxiang.toml 标记项目根）
    let toml = "[project]\nname = \"demo\"\n";
    let lib = r#"
helper: (x: Int) -> Int = (x) => {
    return x * 2
}
"#;
    let test = r#"
use std.assert
use lib.{helper}

main = () => {
    assert.assert(helper(21) == 42, "helper(21) should be 42")
}
"#;

    // Act & Assert - 双根解析：导入者目录未命中，项目根兜底命中（RFC-036 测试场景）
    run_project_ok(
        &[
            ("yaoxiang.toml", toml),
            ("lib.yx", lib),
            ("tests/math_test.yx", test),
        ],
        "tests/math_test.yx",
    );
}

#[test]
fn test_multifile_importer_dir_module_wins_over_project_root() {
    // Arrange - tests/ 与项目根各有一个 lib.yx，helper 行为不同（*3 vs *2）以区分命中者
    let toml = "[project]\nname = \"demo\"\n";
    let root_lib = r#"
helper: (x: Int) -> Int = (x) => {
    return x * 2
}
"#;
    let local_lib = r#"
helper: (x: Int) -> Int = (x) => {
    return x * 3
}
"#;
    let test = r#"
use std.assert
use lib.{helper}

main = () => {
    assert.assert(helper(21) == 63, "importer-dir lib should win: helper(21) == 63")
}
"#;

    // Act & Assert - 双根第一规则：导入者目录优先；根上的同名模块不可达即忽略
    run_project_ok(
        &[
            ("yaoxiang.toml", toml),
            ("lib.yx", root_lib),
            ("tests/lib.yx", local_lib),
            ("tests/math_test.yx", test),
        ],
        "tests/math_test.yx",
    );
}

#[test]
fn test_multifile_unrelated_broken_file_does_not_block_run() {
    // Arrange - 项目中存在与入口无关的坏文件（目录递归时代会阻塞整个编译）
    let main = r#"
use std.assert

main = () => {
    assert.assert(1 + 1 == 2, "math still works")
}
"#;
    let broken = "this is not valid yaoxiang (((";

    // Act & Assert - use 追踪下不可达文件不参与编译（#247）
    run_project_ok(&[("main.yx", main), ("broken.yx", broken)], "main.yx");
}

#[test]
fn test_multifile_transitive_imports_discovered() {
    // Arrange - main → a → b 传递依赖链
    let b = r#"
fb: () -> Int = () => {
    return 42
}
"#;
    let a = r#"
use b.{fb}

fa: () -> Int = () => {
    return fb()
}
"#;
    let main = r#"
use std.assert
use a.{fa}

main = () => {
    assert.assert(fa() == 42, "transitive fa() should be 42")
}
"#;

    // Act & Assert - BFS 沿 use 逐层发现
    run_project_ok(&[("main.yx", main), ("a.yx", a), ("b.yx", b)], "main.yx");
}

#[test]
fn test_multifile_same_module_key_from_two_roots_is_ambiguity_error() {
    // Arrange - main 命中项目根的 lib；sub/consumer 按导入者目录命中 sub/lib——
    // 两个不同文件得到同一模块键 "lib"
    let toml = "[project]\nname = \"demo\"\n";
    let main = r#"
use lib.{helper}
use sub.consumer

main = () => {
    println(helper(1))
}
"#;
    let root_lib = r#"
helper: (x: Int) -> Int = (x) => {
    return x * 2
}
"#;
    let consumer = r#"
use lib.{helper}

consume: (x: Int) -> Int = (x) => {
    return helper(x)
}
"#;
    let sub_lib = r#"
helper: (x: Int) -> Int = (x) => {
    return x * 3
}
"#;

    // Act
    let result = run_project(
        &[
            ("yaoxiang.toml", toml),
            ("main.yx", main),
            ("lib.yx", root_lib),
            ("sub/consumer.yx", consumer),
            ("sub/lib.yx", sub_lib),
        ],
        "main.yx",
    );

    // Assert - 歧义必须显式报错，而非静默遮蔽其一
    let err = result.expect_err("同名模块键来自两个根，应报歧义错误");
    assert!(
        err.contains("歧义") || err.contains("Ambigu"),
        "错误应说明模块键歧义，实际: {err}"
    );
}

// T5：跨文件顶层绑定（全局槽位）

/// 入口 main 是**无注解块值绑定**的多文件项目（#388 定案：入口按绑定存在性）。
///
/// 该形态在 B 方案（33f2ebbe）后落全局槽位而非函数表，T4 原始入口检查曾
/// 报 E3021「入口不是函数」。定案后值 main 在初始化期求值即执行——这是
/// 「Script 顶层语句包进 main 迁入 Bin」的自然迁移形态。
#[test]
fn test_multifile_entry_value_main_block_binding() {
    // Arrange - lib 导出函数，入口文件以无注解块 main 调用它
    let lib = r#"
answer: () -> Int = () => {
    return 42
}
"#;
    let main = r#"
use std.assert
use lib.{answer}

main = {
    assert.assert(answer() == 42, "answer should be 42")
}
"#;

    // Act & Assert - 值 main 合法入口，断言在初始化期真实执行
    run_project_ok(&[("lib.yx", lib), ("main.yx", main)], "main.yx");
}

/// 跨文件块值绑定：`x: Int = { 5 }` 在 lib 中定义，入口文件读取。
///
/// 覆盖 T2（块值运行时初始化）+ T5（跨文件槽位布局）。内容决定类型下
/// 非 Fn 注解的块值是**值绑定**，故它占全局槽位而非函数表。
#[test]
fn test_multifile_use_block_value_binding() {
    let lib = "x: Int = { 5 }\n";
    let main = r#"
use std.assert
use lib.{x}

main = () => {
    assert.assert(x == 5, "跨文件块值绑定应为 5")
}
"#;
    run_project_ok(&[("lib.yx", lib), ("main.yx", main)], "main.yx");
}

/// 跨文件前向引用：lib 的绑定依赖另一个后声明的绑定。
///
/// 覆盖 T3 拓扑排序在跨文件场景的行为——每个文件的初始化序列各自有序，
/// 文件间按发现顺序（被依赖者先）。
#[test]
fn test_multifile_forward_reference_in_lib() {
    let lib = r#"
derived: Int = base * 3
base: Int = 7
"#;
    let main = r#"
use std.assert
use lib.{derived}

main = () => {
    assert.assert(derived == 21, "跨文件前向引用应为 21")
}
"#;
    run_project_ok(&[("lib.yx", lib), ("main.yx", main)], "main.yx");
}

/// 议题 2 基石：跨模块和类型传播。
/// lib 定义和类型（变体构造器 + match 解构），main 导入后使用——
/// 要求导入方 typecheck 环境拥有 generic_type_defs / sum_types /
/// 接口实现登记（Export 通道此前完全不携带，导入方全部落空）。
#[test]
fn test_multifile_sum_type_cross_module() {
    let lib = r#"
Shape: Type = {
    circle: (Float) -> Shape,
    square: (String) -> Shape,
}
"#;
    let main = r#"
use std.assert
use lib.{Shape}

main = () => {
    c = Shape.circle(2.0)
    v = match c {
        circle(r) => r,
        square(_) => 0.0,
    }
    assert.assert(v == 2.0, "cross-module sum type ctor + match")
}
"#;
    run_project_ok(&[("lib.yx", lib), ("main.yx", main)], "main.yx");
}

/// 同上，但走**整体导入**（`use lib`）——类型导出按自身名补镜像，
/// 裸名 `Shape` 直接可用（预置退役后 std.result 的实际用法）。
#[test]
fn test_multifile_sum_type_whole_import() {
    let lib = r#"
Shape: Type = {
    circle: (Float) -> Shape,
    square: (String) -> Shape,
}
"#;
    let main = r#"
use std.assert
use lib

main = () => {
    s = Shape.square("hi")
    v = match s {
        circle(r) => r,
        square(_) => 1.0,
    }
    assert.assert(v == 1.0, "whole-import sum type ctor + match")
}
"#;
    run_project_ok(&[("lib.yx", lib), ("main.yx", main)], "main.yx");
}

// === #396/#397：namespace 数据访问（`use lib;` 后 `lib.member` 表达式位置取值）===
//
// 规范来源：语言参考 modules.md §3.1（顶层绑定一律可导入，整体导入经模块名
// 访问）+ RFC-029 模块语义（导出面 / 名字解析归 Resolver）。
//
// 此前只有调用位置有 namespace 降级，`lib.v` 落到「普通字段访问」→ 对 `lib`
// 的变量读取 → E3006 内部错误。修复后与 `use lib.{v}` 花括号导入同一数据面
// （跨文件全局布局按限定名登记 → Load Global；函数值物化为 MakeClosure）。

/// `lib.typed_v`：有标注常量的整体导入数据访问，真跑取值。
#[test]
fn test_namespace_data_typed_constant() {
    // Arrange：lib 导出有标注常量，main 整体导入后表达式位置访问
    let lib = r#"
typed_v: Int = 42
"#;
    let main = r#"
use std.assert
use lib

main = () => {
    assert.assert(lib.typed_v == 42, "lib.typed_v should be 42")
}
"#;

    // Act：run_project 编译到 IR 并执行入口
    run_project_ok(&[("lib.yx", lib), ("main.yx", main)], "main.yx");

    // Assert：yx 侧 assert.assert(lib.typed_v == 42) 已在执行中验证
}

/// `lib.untyped_v`：无标注常量（修复 #397 后进入导出面）。
#[test]
fn test_namespace_data_unannotated_constant() {
    // Arrange：lib 导出无标注常量
    let lib = r#"
untyped_v = 7
"#;
    let main = r#"
use std.assert
use lib

main = () => {
    assert.assert(lib.untyped_v == 7, "lib.untyped_v should be 7")
}
"#;

    // Act：run_project 编译到 IR 并执行入口
    run_project_ok(&[("lib.yx", lib), ("main.yx", main)], "main.yx");

    // Assert：yx 侧 assert.assert(lib.untyped_v == 7) 已在执行中验证
}

/// `lib.greet` 作一等值：函数成员物化为闭包后仍可调用。
#[test]
fn test_namespace_data_function_as_value() {
    // Arrange：lib 导出函数，main 把它绑给局部名再调用
    let lib = r#"
greet: () -> Int = () => { 5 }
"#;
    let main = r#"
use std.assert
use lib

main = () => {
    f = lib.greet
    assert.assert(f() == 5, "lib.greet as value should be callable")
}
"#;

    // Act：run_project 编译到 IR 并执行入口
    run_project_ok(&[("lib.yx", lib), ("main.yx", main)], "main.yx");

    // Assert：yx 侧 assert.assert(f() == 5) 已在执行中验证
}

/// `lib.Point(1, 2)`：类型构造经整体导入仍然可用（回归防线）。
#[test]
fn test_namespace_data_constructor_still_works() {
    // Arrange：lib 导出类型，main 整体导入后限定构造
    let lib = r#"
Point: Type = { x: Int, y: Int }
"#;
    let main = r#"
use std.assert
use lib

main = () => {
    p = lib.Point(1, 2)
    assert.assert(p.x == 1, "lib.Point ctor should work")
}
"#;

    // Act：run_project 编译到 IR 并执行入口
    run_project_ok(&[("lib.yx", lib), ("main.yx", main)], "main.yx");

    // Assert：yx 侧 assert.assert(p.x == 1) 已在执行中验证
}

/// 无标注绑定初始化式引用**其他模块**（`use other;` 后 `v = other.greet()`）：
/// 收割路径加工 use 后照常进导出面（#397 闭环）。
#[test]
fn test_namespace_data_cross_module_initializer() {
    // Arrange：other 导出函数，lib 无标注绑定其返回值，main 整体导入 lib
    let other = r#"
greet: () -> Int = () => { 5 }
"#;
    let lib = r#"
use other

v = other.greet()
"#;
    let main = r#"
use std.assert
use lib

main = () => {
    assert.assert(lib.v == 5, "lib.v should be 5 (cross-module initializer)")
}
"#;

    // Act：run_project 编译到 IR 并执行入口
    run_project_ok(
        &[("other.yx", other), ("lib.yx", lib), ("main.yx", main)],
        "main.yx",
    );

    // Assert：yx 侧 assert.assert(lib.v == 5) 已在执行中验证
}

/// 前向引用：无标注绑定引用**后置**绑定（`b = a + 1` 中 a 在后），
/// 收割按 pass3 同款 T3 占位机制解析，不因声明序退化。
#[test]
fn test_namespace_data_forward_reference() {
    // Arrange：b 引用后置的 a
    let lib = r#"
b = a + 1
a: Int = 2
"#;
    let main = r#"
use std.assert
use lib

main = () => {
    assert.assert(lib.b == 3, "lib.b should be 3 (forward reference)")
}
"#;

    // Act：run_project 编译到 IR 并执行入口
    run_project_ok(&[("lib.yx", lib), ("main.yx", main)], "main.yx");

    // Assert：yx 侧 assert.assert(lib.b == 3) 已在执行中验证
}

/// WBS 3.6.2：跨文件导入的**函数类型值**按名调用走间接调用。
///
/// `f = double` 是值绑定（占 lib 的全局槽位、不进函数表）。调用点此前被当成
/// 静态函数名，运行期按 `lib.f` 查扁平函数表落空（`E6006 Native function not
/// found: lib.f`）；修复后调用形态判据经 `use_alias_value_slot` 认出它是值，
/// 发射 `LoadGlobal` + `CallDyn`。
#[test]
fn test_multifile_imported_fn_value_call_by_name_uses_indirect_call() {
    // Arrange：lib 导出函数类型值 f；main 由 use 导入后按名调用
    let lib = r#"
double: (x: Int) -> Int = (x) => x * 2

f = double
"#;
    let main = r#"
use std.assert
use lib.{f}

main = () => {
    assert.assert(f(3) == 6, "imported function value called by name must run the bound function")
}
"#;

    // Act
    let result = run_project(&[("lib.yx", lib), ("main.yx", main)], "main.yx");

    // Assert
    assert!(
        result.is_ok(),
        "跨文件导入的函数类型值按名调用应走间接调用（此前 E6006 Native function not found: lib.f）：{result:?}"
    );
}

/// WBS 3.6.2：`use lib.{f as g}` 内联别名形态——别名解析到同一个跨文件值，
/// 调用同样走间接调用。
#[test]
fn test_multifile_imported_fn_value_call_by_name_with_as_alias_uses_indirect_call() {
    // Arrange：main 用 `as g` 给导入的函数类型值起别名，再按别名调用
    let lib = r#"
double: (x: Int) -> Int = (x) => x * 2

f = double
"#;
    let main = r#"
use std.assert
use lib.{f as g}

main = () => {
    assert.assert(g(3) == 6, "aliased import of a function value must run the bound function")
}
"#;

    // Act
    let result = run_project(&[("lib.yx", lib), ("main.yx", main)], "main.yx");

    // Assert
    assert!(
        result.is_ok(),
        "`use lib.{{f as g}}` 的函数类型值按名调用应走间接调用（此前 E6006 lib.f）：{result:?}"
    );
}

/// WBS 3.6.2 回归防线：导入的**函数定义**（进函数表）不因值臂扩展被当成值，
/// 仍按名静态调用。
#[test]
fn test_multifile_imported_function_definition_still_calls_statically() {
    // Arrange：lib 导出函数 double 与值绑定 f；main 只导入函数定义
    let lib = r#"
double: (x: Int) -> Int = (x) => x * 2

f = double
"#;
    let main = r#"
use std.assert
use lib.{double}

main = () => {
    assert.assert(double(3) == 6, "imported function definition must still call statically")
}
"#;

    // Act
    let result = run_project(&[("lib.yx", lib), ("main.yx", main)], "main.yx");

    // Assert
    assert!(
        result.is_ok(),
        "导入的函数定义仍须走静态调用（值臂不得误伤函数表条目）：{result:?}"
    );
}

/// WBS 3.6.4：`use lib`（整体导入）后的**限定名调用** `lib.f(3)`——`f` 是函数
/// 类型值（占全局槽位、不进函数表）。此前走 FieldAccess callee 的命名空间分支
/// 发射静态 `Call "lib.f"`，运行期按扁平函数表查名落空（`E6006 Native function
/// not found: lib.f`）；修复后限定名若落在值绑定槽位则 `LoadGlobal` + `CallDyn`。
///
/// 同项目内保留三条对照：命名空间**数据**访问 `lib.v`、命名空间**静态函数**
/// 调用 `lib.add`、方法调用 `p.get_x()`——值臂不得误伤任何一条。
#[test]
fn test_multifile_namespace_qualified_fn_value_call_uses_indirect_call() {
    // Arrange：lib 导出函数类型值 f、数据 v、函数定义 add、带方法的类型 Point
    let lib = r#"
double: (x: Int) -> Int = (x) => x * 2

f = double

v: Int = 5

add: (a: Int, b: Int) -> Int = (a, b) => a + b

Point: Type = { x: Int }

Point.get_x: (self: &Point) -> Int = (self) => self.x
"#;
    let main = r#"
use std.assert
use lib
use lib.{Point}

main = () => {
    assert.assert(lib.f(3) == 6, "namespace-qualified function value call must run the bound function")
    assert.assert(lib.v == 5, "namespace data access must still read the global slot")
    assert.assert(lib.add(3, 4) == 7, "namespace function definition must still call statically")
    p = Point(7)
    assert.assert(p.get_x() == 7, "method call must still dispatch through the method path")
}
"#;

    // Act
    let result = run_project(&[("lib.yx", lib), ("main.yx", main)], "main.yx");

    // Assert
    assert!(
        result.is_ok(),
        "use lib 整体导入后的限定名调用 lib.f(3) 应走间接调用（此前 E6006 Native function not found: lib.f）：{result:?}"
    );
}

/// WBS 3.6.4 产物判据：该调用点的 IR 形态是 `LoadGlobal` + `CallDyn`，
/// 且不存在以 `lib.f` 为名字的静态 `Call`（那正是运行期 E6006 的来源）。
#[test]
fn test_multifile_namespace_qualified_fn_value_call_emits_call_dyn() {
    use yaoxiang::frontend::module::orchestrator::compile_project;
    use yaoxiang::middle::core::ir::{ConstValue, FunctionBody, Instruction, Operand};

    // Arrange：与行为测试同一项目形态
    let dir = tempfile::tempdir().expect("create tempdir");
    std::fs::write(
        dir.path().join("yaoxiang.toml"),
        "[package]\nname = \"t\"\nversion = \"0.1.0\"\n",
    )
    .expect("write manifest");
    std::fs::write(
        dir.path().join("lib.yx"),
        "double: (x: Int) -> Int = (x) => x * 2\n\nf = double\n",
    )
    .expect("write lib");
    std::fs::write(
        dir.path().join("main.yx"),
        "use std.assert\nuse lib\n\nmain = () => {\n    assert.assert(lib.f(3) == 6, \"namespace call\")\n}\n",
    )
    .expect("write main");

    // Act
    let (ir, _diagnostics) = compile_project(&dir.path().join("main.yx")).expect("compile project");

    // Assert
    let mut has_call_dyn = false;
    let mut static_calls: Vec<String> = Vec::new();
    for func in &ir.functions {
        let FunctionBody::Code { blocks, .. } = &func.body else {
            continue;
        };
        for block in blocks {
            for instr in &block.instructions {
                match instr {
                    Instruction::CallDyn { .. } => has_call_dyn = true,
                    Instruction::Call {
                        func: Operand::Const(ConstValue::String(name)),
                        ..
                    } => static_calls.push(name.clone()),
                    _ => {}
                }
            }
        }
    }
    assert!(
        has_call_dyn,
        "限定名调用函数类型值应发射 CallDyn（间接调用），实际静态调用: {static_calls:?}"
    );
    assert!(
        !static_calls.iter().any(|name| name == "lib.f"),
        "不得再以 lib.f 为名字发静态 Call（运行期会 E6006 Native function not found）：{static_calls:?}"
    );
}
