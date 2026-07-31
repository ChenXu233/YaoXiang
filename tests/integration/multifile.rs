//! RFC-029 多文件模块编排端到端测试
//!
//! 验证跨文件 `use` 导入：本地模块的类型与函数能被另一个文件导入、构造、调用并正确执行。
//! 正确性用语言内 `assert` 验证——断言失败会使 `run_project` 返回 Err。

/// 在临时目录写入多文件项目并运行入口文件；任何编译/运行/断言失败都会 panic。
fn run_project_ok(
    files: &[(&str, &str)],
    entry: &str,
) {
    let dir = tempfile::tempdir().expect("create tempdir");
    for (name, content) in files {
        std::fs::write(dir.path().join(name), content).expect("write source file");
    }
    let entry_path = dir.path().join(entry);
    yaoxiang::run_project(&entry_path).unwrap_or_else(|e| panic!("run_project failed:\n{:?}", e));
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

main = {
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

main = {
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

main = {
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

main = {
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

main = {
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

main = {
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

main = {
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

main = {
    assert.assert(make_a() == 3.0, "a.Point(1,2) x+y should be 3.0")
    assert.assert(make_b() == 7.0, "b.Point(3,4) x+z should be 7.0")
}
"#;
    run_project_ok(&[("a.yx", a), ("b.yx", b), ("main.yx", main)], "main.yx");
}
