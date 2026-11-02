//! WBS 6.8.2a / D54：**声明驱动的 Fn 标注 ↔ 函数体对照**（形参维度 + 出口统一）
//!
//! 背景：D54（2026-10-05）报告的「同一语义不同语法、check 绿运行期红」在
//! 出口维度已于 #501 修复轮（48e25ca6）堵上，**剩余面在形参维度**：`=>` 后的
//! 实现头与标注的运行时形参位之间，旧行为按**名字交集**合并，交集为空就整段
//! 跳过——标注的形参类型从未到达实现头。
//!
//! 本文件把 D54 的残留矩阵收成仓内夹具（外部探针目录不入库），逐条断言：
//! - ① 形参按位驱动：同一语义的三种语法（同名箭头 / 异名箭头 / block 体）
//!   必须给出**同一条诊断**；声明类型按位置到达实现头，改名不再改变判据。
//! - ② 出口两叉收一叉：箭头糖（StmtKind::Return）与显式 return
//!   （Expr::Return）都是 E1012、都指向注解。
//! - ③ 注册形态同源：命名实参的可用名取**声明**，实现头改名不影响调用侧。
//! - ④ 形参标注冲突：实现头自带标注与声明不符 → E1011（旧行为是静默覆盖）。
//!
//! 判据来源：WBS 6.8.2a 任务契约 + D54；E1112 为本轮新增的对外码
//! （实现形参个数超出声明）。

use crate::frontend::core::lexer::tokenize;
use crate::frontend::core::parser::parse;
use crate::frontend::core::sema::checker::TypeChecker;
use crate::frontend::core::sema::probe::TypeLevelNamespace;

/// 编译期诊断 (码, 行号) 序列（真实序列：lex → parse → typecheck）。
fn compile_diags(source: &str) -> Vec<(String, usize)> {
    let tokens = tokenize(source).expect("夹具必须可 tokenize");
    let parsed = parse(&tokens);
    assert!(
        !parsed.has_errors,
        "夹具必须无语法错误: {:?}",
        parsed.errors
    );
    let mut checker = TypeChecker::new("<fn-annotation-binding>");
    let tr = checker.check_module(
        &parsed.module,
        &TypeLevelNamespace::scan_one(&parsed.module),
    );
    for d in tr.diagnostics.iter() {
        eprintln!("BIND_DIAG {} @{:?} :: {}", d.code, d.span, d.message);
    }
    tr.diagnostics
        .iter()
        .map(|d| (d.code.clone(), d.span.map_or(0, |s| s.start.line)))
        .collect()
}

/// 编译期诊断码集合。
fn compile_codes(source: &str) -> Vec<String> {
    compile_diags(source).into_iter().map(|(c, _)| c).collect()
}

/// 夹具：一份「同语义、三种语法」的三个函数定义。
///
/// 三个函数都是「一个 Int 形参、体内把字符串加进去」：
/// - `same`：箭头 lambda，形参同名（旧行为：标注类型按位覆盖 → E1002）
/// - `renamed`：箭头 lambda，形参改名为 b（旧行为：交集为空 → 静默放行，
///   运行期 E6007）
/// - `blocky`：block 体（形参由 parser 按标注合成 → E1002）
const THREE_SYNTAX: &str = r#"
same: (a: Int) -> Int = (a) => a + "s"
renamed: (a: Int) -> Int = (b) => b + "s"
blocky: (a: Int) -> Int = { a + "s" }
"#;

#[test]
fn test_fn_annotation_param_type_drives_every_syntax_form() {
    // Arrange：同语义、三种语法（同名箭头 / 异名箭头 / block 体）
    // Act
    let codes = compile_codes(THREE_SYNTAX);
    // Assert：三格必须是同一条诊断（旧行为：改名那一格 check 绿、运行期 E6007）
    assert_eq!(
        codes.iter().filter(|c| *c == "E1002").count(),
        3,
        "三种语法都必须报 E1002（形参类型按位来自声明）：{codes:?}"
    );
}

#[test]
fn test_fn_annotation_declared_types_bind_positionally_when_renamed() {
    // Arrange：第一个形参的类型只有声明里有（实现头不带标注）
    const SRC: &str = r#"
f: (a: Int, b: String) -> String = (x, y) => x
"#;
    // Act
    let codes = compile_codes(SRC);
    // Assert：x 必须是 Int（按位取自声明），返回位是 String → E1012；
    // 旧行为：形参退回 fresh var，x 自由 unify 成 String，check 静默通过。
    assert!(
        codes.iter().any(|c| c == "E1012"),
        "改名形参必须按位拿到声明类型（x: Int ≠ String）：{codes:?}"
    );
}

#[test]
fn test_fn_annotation_named_argument_uses_declared_name() {
    // Arrange：声明名 a、实现头名 b，调用点用**声明名**传命名实参
    const SRC: &str = r#"
f: (a: Int) -> Int = (b) => b
main: () -> Void = {
    x: Int = f(a = 7)
}
"#;
    // Act
    let codes = compile_codes(SRC);
    // Assert：可用名取声明（③）——旧行为报 E1014（available: b）
    assert!(
        !codes.iter().any(|c| c == "E1014"),
        "命名实参必须按**声明**名绑定：{codes:?}"
    );
    assert!(
        codes.is_empty(),
        "声明名与实现头名不同不应产生诊断：{codes:?}"
    );
}

#[test]
fn test_fn_annotation_named_argument_positional_equivalent() {
    // Arrange：同一函数两种调用形态（位置 / 命名）必须同样放行
    const SRC: &str = r#"
f: (a: Int, b: Int) -> Int = (x, y) => x + y
main: () -> Void = {
    p: Int = f(1, 2)
    q: Int = f(b = 2, a = 1)
}
"#;
    // Act
    let codes = compile_codes(SRC);
    // Assert
    assert!(
        codes.is_empty(),
        "命名与位置调用应同样放行（同一绑定判据）：{codes:?}"
    );
}

#[test]
fn test_fn_annotation_return_explicit_and_arrow_share_code() {
    // Arrange：同一「返回位不符」的两种 AST 形状（箭头糖 vs 显式 return）
    const SRC: &str = r#"
arrow_tail: () -> Int = () => "hello"
arrow_explicit: () -> Int = () => { return "hello" }
block_explicit: () -> Int = { return "hello" }
"#;
    // Act
    let diags = compile_diags(SRC);
    let e1012_lines: Vec<usize> = diags
        .iter()
        .filter(|(c, _)| c == "E1012")
        .map(|(_, line)| *line)
        .collect();
    let e1002_count = diags.iter().filter(|(c, _)| c == "E1002").count();
    // Assert：两叉收一叉——三条都是 E1012，位置都指向**注解**所在行
    assert_eq!(
        (e1012_lines.len(), e1002_count),
        (3, 0),
        "尾表达式与显式 return 必须同码（E1012），不得再出现 E1002：{diags:?}"
    );
    assert_eq!(
        e1012_lines,
        vec![2, 3, 4],
        "E1012 必须指向声明注解所在行（不是载荷所在的体内位置）：{diags:?}"
    );
}

#[test]
fn test_fn_annotation_param_annotation_conflict_reports_e1011() {
    // Arrange：实现头自带 String 标注，声明写 Int（旧行为：静默覆盖，打印 1）
    const SRC: &str = r#"
f: (a: Int) -> Int = (a: String) => a
"#;
    // Act
    let diags = compile_diags(SRC);
    // Assert：冲突必须响亮（④），且码是形参类型不符 E1011
    assert!(
        diags.iter().any(|(c, _)| c == "E1011"),
        "实现头的形参标注与声明冲突必须报 E1011：{diags:?}"
    );
    assert_eq!(
        diags[0].1, 2,
        "E1011 必须指向声明语句（含形参的那一行）：{diags:?}"
    );
}

#[test]
fn test_fn_annotation_impl_param_count_exceeds_declaration() {
    // Arrange：声明 0 个运行期形参，实现写了 1 个（旧行为：check 绿、运行期 void）
    const SRC: &str = r#"
f: () -> Int = (a: Int) => a
"#;
    // Act
    let codes = compile_codes(SRC);
    // Assert：多出的实现形参没有声明位可绑 → 新增对外码 E1112
    assert!(
        codes.iter().any(|c| c == "E1112"),
        "实现形参个数超出声明必须报 E1112：{codes:?}"
    );
}

#[test]
fn test_fn_annotation_impl_fewer_params_is_filled_from_declaration() {
    // Arrange：声明 1 个形参、实现不写形参（体内不引用它）
    const SRC: &str = r#"
f: (a: Int) -> Int = () => 1
"#;
    // Act
    let codes = compile_codes(SRC);
    // Assert：反向不报——缺席位由声明名补齐，函数照常按声明元数调用
    assert!(
        codes.is_empty(),
        "实现少写形参不是错误（缺席位由声明补名）：{codes:?}"
    );
}

#[test]
fn test_fn_annotation_curry_layer_params_stay_visible() {
    // Arrange：柯里外层值参数 t 只在体内引用（#295 的参数固化）
    const SRC: &str = r#"
gt: (t: Int) -> (x: Int) -> Bool = (x) => x > t
"#;
    // Act
    let codes = compile_codes(SRC);
    // Assert：按位驱动不得把柯里外层位算进「实现形参」——否则误报 E1112/E1001
    assert!(
        codes.is_empty(),
        "柯里外层值参数必须仍在体内可见，且不占实现形参位：{codes:?}"
    );
}

#[test]
fn test_fn_annotation_const_generic_curry_still_passes() {
    // Arrange：const 泛型柯里（(N: Int) -> (n: N) -> Int），实现头只写内层
    const SRC: &str = r#"
f: (N: Int) -> (n: N) -> Int = (n) => n
"#;
    // Act
    let codes = compile_codes(SRC);
    // Assert：const 位不占运行期形参位——元数对照必须按同一口径（不得误报 E1112）
    assert!(codes.is_empty(), "const 泛型柯里不应产生诊断：{codes:?}");
}

#[test]
fn test_fn_annotation_paren_return_closure_stays_valid() {
    // Arrange：括号返回（返回闭包）+ 闭包包装形态
    const SRC: &str = r#"
mk: () -> ((a: Int) -> Int) = (a) => a + 1
make_adder: (n: Int) -> ((x: Int) -> Int) = (x) => x + n
"#;
    // Act
    let codes = compile_codes(SRC);
    // Assert：括号语义（RFC-004）不受按位驱动影响
    assert!(
        codes.is_empty(),
        "括号返回/闭包包装形态不应产生诊断：{codes:?}"
    );
}
