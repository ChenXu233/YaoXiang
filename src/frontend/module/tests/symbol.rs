//! 绑定身份（`DefPath` + `DefId` 双代表）测试 — 规范来源：RFC-029 #243/#244
//! （同名类型跨模块共存、限定名唯一所有者）、09-execution-wbs 的 WBS 4.9.10、
//! 07-equivalence-oracle（C1 零差分：路径是事实、序号是本次编译的句柄）。
//!
//! 准入（coding-rules 第七部分·**证明门**）：
//! 1. 性质：限定名是 `(module, owner, name)` 的**纯函数**，且该函数只有一个实现
//!    （`DefPath::project`）——凡在多处各自拼名字的实现，登记侧与查找侧必然漂移。
//! 2. 为什么必须成立：限定名同时是「定义侧登记键」与「调用侧查找键」。两者若不同源，
//!    查找落空后退回按名分发（E6006）——IR 的 DefId 承重失效。这与「谁来拼」这个
//!    实现选择无关，是身份模型自身的性质。
//! 3. 为什么这里是最便宜的观察点：投影是纯函数，单测直接钉四种形态与两条幂等律，
//!    不需要编译任何语料。

use crate::frontend::module::symbol::{DefId, DefKind, DefPath, SymbolTable};

/// 顶层绑定的路径投影成 `{module}.{bare}`（`lib.helper`）。
#[test]
fn test_def_path_item_project_joins_module_and_bare_name() {
    // Arrange
    let path = DefPath::item("lib", "helper");

    // Act
    let projection = path.project();

    // Assert
    assert_eq!(
        projection, "lib.helper",
        "顶层绑定的投影必须是 {{module}}.{{bare}}（lib.helper）"
    );
}

/// 模块键**原样**参与投影：`std.math.geometry.foo` 的模块段不拆开、不再分段。
#[test]
fn test_def_path_item_project_keeps_module_key_verbatim() {
    // Arrange：注册表模块键 std.math.geometry 整串作为模块段
    let path = DefPath::item("std.math.geometry", "foo");

    // Act
    let projection = path.project();

    // Assert
    assert_eq!(
        projection, "std.math.geometry.foo",
        "模块键必须原样进入投影（std.math.geometry + foo）"
    );
}

/// 方法路径投影成 `{module}.{owner}.{bare}`（`lib.Point.get_x`）。
#[test]
fn test_def_path_method_project_joins_owner_segment() {
    // Arrange
    let path = DefPath::method("lib", "Point", "get_x");

    // Act
    let projection = path.project();

    // Assert
    assert_eq!(
        projection, "lib.Point.get_x",
        "方法投影必须是 {{module}}.{{owner}}.{{method}}（lib.Point.get_x）"
    );
    assert_eq!(
        path.owner.as_deref(),
        Some("Point"),
        "方法路径的归属段必须是裸类型名"
    );
}

/// 方法的模块段与归属段各自原样：`std.result.Result.is_ok`。
#[test]
fn test_def_path_method_project_joins_nested_module_and_owner() {
    // Arrange
    let path = DefPath::method("std.result", "Result", "is_ok");

    // Act
    let projection = path.project();

    // Assert
    assert_eq!(
        projection, "std.result.Result.is_ok",
        "嵌套模块键 + 泛型类型名的方法投影必须逐字为 std.result.Result.is_ok"
    );
}

/// 无模块归属的生成期扁平名：投影即名字本身（不产生前导点）。
#[test]
fn test_def_path_without_module_projects_bare_name() {
    // Arrange：curry 内层的合成名——没有注册表模块键可归属
    let path = DefPath::item("", "__Point.get_x_l0");

    // Act
    let projection = path.project();

    // Assert
    assert_eq!(
        projection, "__Point.get_x_l0",
        "空模块段的投影必须等于名字本身（历史 intern 的整串登记口径）"
    );
}

/// 同路径幂等：重复登记复用同一 DefId，表长不增。
#[test]
fn test_intern_same_path_returns_same_def_id() {
    // Arrange
    let mut symbols = SymbolTable::new();
    let path = DefPath::item("lib", "helper");

    // Act
    let first = symbols.intern(path.clone(), DefKind::Function);
    let second = symbols.intern(path.clone(), DefKind::Function);

    // Assert
    assert_eq!(first, second, "同一路径必须复用同一 DefId（幂等登记）");
    assert_eq!(symbols.len(), 1, "幂等登记不得增加符号表条目");
    assert_eq!(symbols.path(first), &path, "权威存储必须原样保存登记的路径");
    assert_eq!(
        symbols.def_path(&path),
        Some(first),
        "路径反查（权威）必须命中同一 DefId"
    );
}

/// 同名不同模块 = 不同路径 = 不同 DefId（#244 共存的基础）。
#[test]
fn test_intern_same_bare_name_in_different_modules_gets_distinct_def_ids() {
    // Arrange
    let mut symbols = SymbolTable::new();

    // Act
    let a = symbols.intern(DefPath::item("a", "Point"), DefKind::Type);
    let b = symbols.intern(DefPath::item("b", "Point"), DefKind::Type);

    // Assert
    assert_ne!(
        a, b,
        "同名类型在不同模块必须拿到不同 DefId（a.Point / b.Point 共存）"
    );
    assert_eq!(symbols.name(a), "a.Point", "a.Point 的投影必须与路径同源");
    assert_eq!(symbols.name(b), "b.Point", "b.Point 的投影必须与路径同源");
}

/// 登记记录类别：Function / Type / Constant 各自保留，普通绑定无 parent。
#[test]
fn test_intern_records_kind_and_parent() {
    // Arrange
    let mut symbols = SymbolTable::new();

    // Act
    let f = symbols.intern(DefPath::item("m", "helper"), DefKind::Function);
    let ty = symbols.intern(DefPath::item("m", "Point"), DefKind::Type);
    let c = symbols.intern(DefPath::item("m", "PI"), DefKind::Constant);

    // Assert
    assert_eq!(
        symbols.kind(f),
        DefKind::Function,
        "函数绑定类别必须是 Function"
    );
    assert_eq!(symbols.kind(ty), DefKind::Type, "类型绑定类别必须是 Type");
    assert_eq!(
        symbols.kind(c),
        DefKind::Constant,
        "常量绑定类别必须是 Constant"
    );
    assert_eq!(symbols.parent(f), None, "普通绑定不得有归属类型");
    assert_eq!(symbols.parent(ty), None, "类型绑定不得有归属类型");
}

/// 方法登记：链接到自动登记的类型、按登记序构建静态 vtable。
#[test]
fn test_intern_method_links_parent_and_builds_vtable() {
    // Arrange
    let mut symbols = SymbolTable::new();

    // Act
    let get_x = symbols.intern_method(DefPath::method("lib", "Point", "get_x"));
    let get_y = symbols.intern_method(DefPath::method("lib", "Point", "get_y"));

    // Assert
    assert_eq!(
        symbols.name(get_x),
        "lib.Point.get_x",
        "方法限定名沿用既有约定（Type.method 前加模块键）"
    );
    assert_eq!(
        symbols.path(get_x),
        &DefPath::method("lib", "Point", "get_x"),
        "权威存储必须原样保存方法路径（模块/归属/裸名三段）"
    );
    assert_eq!(
        symbols.kind(get_x),
        DefKind::Method,
        "方法登记类别必须是 Method"
    );
    let point = symbols
        .def_path(&DefPath::item("lib", "Point"))
        .expect("方法登记必须自动登记所属类型");
    assert_eq!(
        symbols.kind(point),
        DefKind::Type,
        "所属类型类别必须是 Type"
    );
    assert_eq!(
        symbols.parent(get_x),
        Some(point),
        "方法 parent 必须指向所属类型"
    );
    assert_eq!(
        symbols.vtable(point),
        &[("get_x".to_string(), get_x), ("get_y".to_string(), get_y)],
        "静态 vtable 必须按登记序排列"
    );
}

/// 方法登记幂等：重复登记不产生重复 vtable 条目。
#[test]
fn test_intern_method_is_idempotent() {
    // Arrange
    let mut symbols = SymbolTable::new();

    // Act
    let first = symbols.intern_method(DefPath::method("lib", "Point", "get_x"));
    let second = symbols.intern_method(DefPath::method("lib", "Point", "get_x"));

    // Assert
    assert_eq!(first, second, "同一方法路径必须复用同一 DefId");
    let point = symbols
        .def_path(&DefPath::item("lib", "Point"))
        .expect("所属类型已登记");
    assert_eq!(
        symbols.vtable(point).len(),
        1,
        "幂等登记不得重复追加 vtable 条目"
    );
}

/// 同名类型在两个模块各有独立 vtable（#244）。
#[test]
fn test_method_paths_of_same_type_name_stay_independent() {
    // Arrange
    let mut symbols = SymbolTable::new();

    // Act
    let a_get = symbols.intern_method(DefPath::method("a", "Point", "get"));
    let b_get = symbols.intern_method(DefPath::method("b", "Point", "get"));

    // Assert
    assert_ne!(a_get, b_get, "同名类型的方法在不同模块必须是不同身份");
    let a_point = symbols
        .def_path(&DefPath::item("a", "Point"))
        .expect("a.Point 已登记");
    let b_point = symbols
        .def_path(&DefPath::item("b", "Point"))
        .expect("b.Point 已登记");
    assert_ne!(a_point, b_point, "同名类型必须是不同身份");
    assert_eq!(
        symbols.vtable(a_point),
        &[("get".to_string(), a_get)],
        "a.Point 的 vtable 只含自己的方法"
    );
    assert_eq!(
        symbols.vtable(b_point),
        &[("get".to_string(), b_get)],
        "b.Point 的 vtable 只含自己的方法"
    );
}

/// 非类型绑定没有 vtable。
#[test]
fn test_vtable_empty_for_binding_without_methods() {
    // Arrange
    let mut symbols = SymbolTable::new();

    // Act
    let f = symbols.intern(DefPath::item("m", "helper"), DefKind::Function);

    // Assert
    assert!(symbols.vtable(f).is_empty(), "函数绑定不得有静态 vtable");
}

/// 投影索引只认**本项目投影过**的字符串：命中同一 DefId，未投影串不命中。
#[test]
fn test_def_by_projection_indexes_only_projected_names() {
    // Arrange
    let mut symbols = SymbolTable::new();
    let helper: DefId = symbols.intern(DefPath::item("lib", "helper"), DefKind::Function);
    let point = symbols.intern_method(DefPath::method("lib", "Point", "get_x"));

    // Act
    let by_item = symbols.def_by_projection("lib.helper");
    let by_method = symbols.def_by_projection("lib.Point.get_x");
    let unknown = symbols.def_by_projection("lib.helper.extra");

    // Assert
    assert_eq!(
        by_item,
        Some(helper),
        "投影 lib.helper 必须索引到同一 DefId"
    );
    assert_eq!(
        by_method,
        Some(point),
        "投影 lib.Point.get_x 必须索引到同一 DefId"
    );
    assert_eq!(
        unknown, None,
        "未投影过的字符串不得命中（索引只读、不反解）"
    );
}
