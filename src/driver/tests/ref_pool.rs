//! 引用池来源与身份的直驱钉测试（WBS 4.9 轮 1 = 4.9.1 + 4.9.2 + 4.9.3）
//!
//! 判据来源：RFC-029f §已知缺陷与后续改进 1+2+3 —— 编译集 / (module, name) /
//! Script 不用池（docs/src/rfc/accepted/029f-target-semantics.md）。
//!
//! - 4.9.1 池的来源 = **编译面**：根按判定三层取（manifest 声明面的包目标
//!   `[run].main`/`[[bin]]`/`[lib]`/`[exports]` 优先，无声明面退化为被检文件
//!   自身的发现集），引用取自各根 `discover_with_used` 的编译集——不再目录
//!   递归扫 `.yx`；
//! - 4.9.2 池的身份 = (模块, 名字)——模块**自有**的名字不算它对外的引用，
//!   同名标识符在不同模块各自计数；
//! - 4.9.3 `FileRole::Script` 不接池（单文件语义，只按定义-使用图判定）。
//!
//! 三条判据各自先红后绿：修复前 W1001 缺失（池把无关引用算成消费者）；
//! 对照组 `test_ref_pool_stays_exempt_...` 钉住「未修分支」——Bin/Lib 的
//! 跨文件消费仍必须豁免，防修复把池整体关掉（只证新行为对不足以放行）。

use std::path::{Path, PathBuf};

use crate::driver::helpers::collect_compilation_refs;
use crate::driver::{Driver, DriverError, DriverOutcome, Program, ProgramKind, Unit};
use crate::frontend::module::roles::FileRole;
use crate::frontend::config::CompileConfig;

/// 合法 manifest（[package] 可解析 ⇒ 角色上下文可用 ⇒ Bin/Lib/Internal 分类生效）
const MANIFEST: &str = "[package]\nname = \"ref-pool\"\nversion = \"0.1.0\"\n";

/// 只有 [tool.test] 的 manifest：`PackageManifest` 解析失败 ⇒ 角色上下文退化为
/// Script 态（helpers.rs 的 role_context → surfaces = None），但
/// `find_project_root` 仍命中 ⇒ 旧实现在这条路径上仍装配引用池（RFC-029f 缺陷 3）
const MANIFEST_UNPARSED: &str = "[tool.test]\npatterns = [\"tests/**/*.yx\"]\n";

/// 声明了包目标的 manifest（RFC-029f 判定三层第一层）：池的根 = `[run].main`，
/// 不是被检文件自身的发现集（WBS 4.9.1 / #334「包内 use 图」）
const MANIFEST_DECLARED: &str =
    "[package]\nname = \"ref-pool\"\nversion = \"0.1.0\"\n\n[run]\nmain = \"main.yx\"\n";

/// 落一个临时项目夹具；manifest 为 None = 无 manifest（standalone / Script 态）。
/// 入口是 main.yx（多文件夹具的入口约定）。
fn write_project(
    files: &[(&str, &str)],
    manifest: Option<&str>,
) -> (tempfile::TempDir, PathBuf) {
    let dir = tempfile::tempdir().expect("create tempdir");
    if let Some(src) = manifest {
        std::fs::write(dir.path().join("yaoxiang.toml"), src).expect("write manifest");
    }
    let mut entry = PathBuf::new();
    for (name, content) in files {
        let path = dir.path().join(name);
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).expect("create fixture dir");
        }
        std::fs::write(&path, content).expect("write project file");
        if *name == "main.yx" {
            entry = path;
        }
    }
    assert!(entry.is_file(), "夹具必须含入口 main.yx");
    (dir, entry)
}

/// 以 Check 形态跑 Driver（与 `yaoxiang check` 同一入口形态）
fn run_check(entry: &Path) -> Result<DriverOutcome, DriverError> {
    let program = Program::new(
        ProgramKind::Check,
        vec![Unit::new("main", entry, "")],
        CompileConfig::default(),
    );
    Driver::new().run(program)
}

/// 某文件条目里的 (码, 起始行) 列表——无条目 = panic（不静默返回空表）
fn diagnostics_of(
    outcome: &DriverOutcome,
    suffix: &str,
) -> Vec<(String, usize)> {
    outcome
        .check_diagnostics
        .iter()
        .find(|(p, _)| p.ends_with(suffix))
        .map(|(_, ds)| {
            ds.iter()
                .map(|d| (d.code.clone(), d.span.map(|s| s.start.line).unwrap_or(0)))
                .collect()
        })
        .unwrap_or_else(|| {
            panic!(
                "check 收集通道必须含 {suffix} 的条目（发现集成员）: {:?}",
                outcome
                    .check_diagnostics
                    .iter()
                    .map(|(p, _)| p.display().to_string())
                    .collect::<Vec<_>>()
            )
        })
}

/// 断言某文件的 W 码集合逐条命中且**不含**禁止项——防误报面整体漂移。
fn assert_warnings(
    outcome: &DriverOutcome,
    suffix: &str,
    required: &[(&str, usize)],
    forbidden: &[(&str, usize)],
) {
    let actual = diagnostics_of(outcome, suffix);
    for want in required {
        assert!(
            actual.contains(&(want.0.to_string(), want.1)),
            "{suffix} 必须报 {}（第 {} 行定义是死代码）；实际: {actual:?}",
            want.0,
            want.1
        );
    }
    for ban in forbidden {
        assert!(
            !actual.contains(&(ban.0.to_string(), ban.1)),
            "{suffix} 不得报 {}（第 {} 行定义有消费者）；实际: {actual:?}",
            ban.0,
            ban.1
        );
    }
}

/// 4.9.1：编译集外文件的标识符不再豁免本单元定义。
///
/// stray.yx 引用 lib.yx 的 helper，但谁也没有 use stray.yx——它不在入口的
/// 编译集里，故其引用不构成消费（RFC-029f 缺陷 1：目录递归扫到孤立文件）。
#[test]
fn test_ref_pool_excludes_refs_from_files_outside_compilation_set() {
    // Arrange：main 只 use lib.other；孤立文件 stray.yx 引用 lib.helper
    let lib = "// lib.yx —— 被导入模块\nother: () -> Int = () => { return 1 }\n\nhelper: () -> Int = () => { return 42 }\n";
    let main = "use std.assert\nuse lib.{other}\n\nmain: () -> Void = {\n    assert.assert(other() == 1, \"other() should be 1\")\n}\n";
    let stray = "// stray.yx —— 未被任何文件 use 的孤立文件\nuse lib.{helper}\n\nprobe: () -> Int = helper()\n";
    let (_dir, entry) = write_project(
        &[("lib.yx", lib), ("main.yx", main), ("stray.yx", stray)],
        Some(MANIFEST),
    );

    // Act
    let outcome = run_check(&entry).expect("check 必须返回产物");

    // Assert：编译集 = main + lib（+ 嵌入 std），不含 stray.yx
    assert!(
        !outcome
            .check_diagnostics
            .iter()
            .any(|(p, _)| p.ends_with("stray.yx")),
        "stray.yx 不在入口编译集，check 收集通道不得有它的条目: {:?}",
        outcome
            .check_diagnostics
            .iter()
            .map(|(p, _)| p.display().to_string())
            .collect::<Vec<_>>()
    );
    assert_warnings(&outcome, "lib.yx", &[("W1001", 4)], &[("W1001", 2)]);
}

/// 4.9.2：a.yx 的 foo 与 b.yx 的 foo 独立计数——别的模块里的同名标识符
/// （顶层定义**或局部变量**）都不再豁免 a.yx 的顶级定义。
#[test]
fn test_ref_pool_counts_same_named_definitions_per_module() {
    // Arrange：lib.foo 无人消费；main 有自己的顶层 foo，helper 有同名局部 foo
    //（helper 里没有同名顶层定义，故 foo = n 是局部绑定而非对 lib.foo 的引用）
    let lib = "// lib.yx —— 被导入模块\nfoo: () -> Int = () => { return 7 }\n\nused: () -> Int = () => { return 1 }\n";
    let helper = "// helper.yx —— 局部变量 foo（无同名顶层定义）\ntouch: (n: Int) -> Void = (n) => {\n    foo = n\n}\n";
    let main = "use std.assert\nuse lib.{used}\nuse helper.{touch}\n\nfoo: () -> Int = () => { return 9 }\n\nmain: () -> Void = {\n    assert.assert(foo() == 9, \"own foo should be 9\")\n    assert.assert(used() == 1, \"used() should be 1\")\n    touch(1)\n}\n";
    let (_dir, entry) = write_project(
        &[("lib.yx", lib), ("helper.yx", helper), ("main.yx", main)],
        Some(MANIFEST),
    );

    // Act
    let outcome = run_check(&entry).expect("check 必须返回产物");

    // Assert：lib.foo（第 2 行）是死代码；lib.used（第 4 行）有 main 消费
    assert_warnings(&outcome, "lib.yx", &[("W1001", 2)], &[("W1001", 4)]);
}

/// 4.9.3：Script 路径不接池——项目内其他文件的引用不影响 script.yx 的 W1001。
///
/// 触发条件是「manifest 存在但 PackageManifest 解析失败」：角色上下文退化为
/// Script（RFC-029f 附录C：无 manifest 上下文 = Script 态），而 find_project_root
/// 仍命中 ⇒ 池在旧实现里非空（缺陷 3 的唯一可达路径）。
#[test]
fn test_ref_pool_is_not_assembled_for_script_role() {
    // Arrange：lib.yx 的局部变量 helper 与 main.yx 的顶级定义同名
    let main = "use std.assert\nuse lib.{touch}\n\nhelper: () -> Int = () => { return 42 }\n\nmain: () -> Void = {\n    touch()\n}\n";
    let lib = "// lib.yx —— 项目内另一文件\ntouch: () -> Void = () => {\n    helper = 1\n    n = helper\n    return\n}\n";
    let (_dir, entry) = write_project(
        &[("main.yx", main), ("lib.yx", lib)],
        Some(MANIFEST_UNPARSED),
    );

    // Act
    let outcome = run_check(&entry).expect("check 必须返回产物");

    // Assert：Script 态没有包视角，helper（第 4 行）一律报 W1001
    assert_warnings(&outcome, "main.yx", &[("W1001", 4)], &[]);
}

/// 对照组（未修分支）：合法 manifest 下 Bin/Lib 的跨文件消费仍豁免——
/// 池不得因 4.9.1-4.9.3 的收紧而整体失效。
#[test]
fn test_ref_pool_stays_exempt_for_cross_module_consumption() {
    // Arrange：lib.yx 的两个定义都被 main.yx use 并调用
    let lib = "// lib.yx —— 被导入模块\nused: () -> Int = () => { return 1 }\n\nPoint: Type = { x: Int }\n";
    let main = "use std.assert\nuse lib.{used, Point}\n\nmain: () -> Void = {\n    p = Point(1)\n    assert.assert(used() == 1, \"used() should be 1\")\n    assert.assert(p.x == 1, \"Point field should be 1\")\n}\n";
    let (_dir, entry) = write_project(&[("lib.yx", lib), ("main.yx", main)], Some(MANIFEST));

    // Act
    let outcome = run_check(&entry).expect("check 必须返回产物");

    // Assert：跨文件消费者在两个定义上都被承认（无 W1001/W1002）
    assert_warnings(&outcome, "lib.yx", &[], &[("W1001", 2), ("W1002", 4)]);
}

/// 池的机制直证（4.9.4 改进 4：消费只记到 import 解析出的定义模块，且必须
/// 「被 import 后真实使用」）：装配只读传入的 units/asts（合成夹具不经文件系统）。
#[test]
fn test_collect_ref_pool_credits_imported_and_used_names_only() {
    // Arrange：a 定义 foo/other；b 的 use 项里 foo 真实使用、guard 只 import 未用，
    // 另有未 import 的同名引用 other；t 是 Test 模块（引用 other）
    let a_src = "foo: () -> Int = () => { return 1 }\n\nother: () -> Int = () => { return 2 }\n";
    let b_src = "use a.{foo, guard}\n\nbar: () -> Int = foo()\n\nbaz: () -> Int = other()\n";
    let t_src = "use a.{other}\n\nprobe: () -> Int = other()\n";
    let parse = |src: &str| {
        let tokens = crate::frontend::core::lexer::tokenize(src).expect("夹具可词法分析");
        let parsed = crate::frontend::core::parser::parse(&tokens);
        assert!(!parsed.has_errors, "夹具必须可解析：{:?}", parsed.errors);
        parsed.module
    };
    let units = vec![
        Unit::new("a", "a.yx", a_src),
        Unit::new("b", "b.yx", b_src),
        Unit::new("t", "tests/t.yx", t_src),
    ];
    let asts = vec![Some(parse(a_src)), Some(parse(b_src)), Some(parse(t_src))];
    let roles = vec![
        Some(FileRole::Bin),
        Some(FileRole::Bin),
        Some(FileRole::Test),
    ];

    // Act
    let pool = collect_compilation_refs(&units, &asts, &roles);
    let credited = |name: &str| -> bool {
        pool.consumed_from
            .values()
            .any(|names| names.contains(name))
    };

    // Assert：foo 被 import 后真实使用 ⇒ 记到 a；guard 只在 use 项里 ⇒ 不算；
    // other 未 import（b）或来自 Test 模块（t）⇒ 不算；b 自有的引用不给自己记账
    assert!(
        pool.consumed_from
            .get("a")
            .is_some_and(|names| names.contains("foo")),
        "被 import 后真实使用的名字必须记到目标模块: {:?}",
        pool.consumed_from
    );
    assert!(
        !credited("guard"),
        "只出现在 use 项里、没有使用点的名字不算消费: {:?}",
        pool.consumed_from
    );
    assert!(
        !credited("other"),
        "未 import 的同名引用与 Test 模块的引用都不算消费: {:?}",
        pool.consumed_from
    );
    assert!(
        !pool.consumed_from.contains_key("b") && !pool.consumed_from.contains_key("t"),
        "模块自己的引用不得给自己记账: {:?}",
        pool.consumed_from
    );
}

/// 池的**根**按 RFC-029f 判定三层解析（WBS 4.9.1 / #334「包内 use 图」）：
/// manifest 声明面的包目标（[run].main / [[bin]] / [lib] / [exports]）优先——
/// 被检文件自身的发现集里没有的消费者（包入口）也救活定义；而**编译面之外**
/// 的文件（不在任何包目标 discover_with_used 编译集里的孤立文件）不算消费者。
#[test]
fn test_ref_pool_root_follows_declared_package_target() {
    // Arrange：manifest 声明 [run].main = main.yx；main 消费 internal.util；
    // 孤立文件 stray.yx 消费 internal.orphan（两者都不在被检文件自己的发现集里）
    let internal = "// internal.yx —— 被检文件\nutil: (x: Int) -> Int = (x) => { return x }\n\norphan: (x: Int) -> Int = (x) => { return x + 1 }\n";
    let main = "use internal.{util}\n\nmain: () -> Void = {\n    n = util(1)\n}\n";
    let stray = "use internal.{orphan}\n\nprobe: (x: Int) -> Int = (x) => { return orphan(x) }\n";
    let (_dir, entry) = write_project(
        &[
            ("internal.yx", internal),
            ("main.yx", main),
            ("stray.yx", stray),
        ],
        Some(MANIFEST_DECLARED),
    );

    // Act：检查非入口文件 internal.yx——它自己的发现集里没有 main.yx / stray.yx
    let outcome = run_check(&entry).expect("check 必须返回产物");

    // Assert：util（第 2 行）被包入口消费 ⇒ 活；orphan（第 4 行）只被编译面外的
    // 孤立文件引用 ⇒ 死（旧口径的目录扫描会把它误救，这正是 RFC-029f 缺陷 1）
    assert_warnings(&outcome, "internal.yx", &[("W1001", 4)], &[("W1001", 2)]);
}

/// 4.9.4：消费只记到 **import 解析出的那个模块**——别的模块调用同名标识符
/// （use b.{foo} + foo()）不该救活 a.yx 的同名定义；而「只被 import、从未被
/// 调用」的名字（use 项里出现但无使用点）同样不算消费。
#[test]
fn test_ref_pool_credits_consumption_to_the_imported_module_only() {
    // Arrange：main 从 b 取 foo 并调用；a.foo 无人消费；a.guard 只出现在 use 项里
    let a = "// a.yx —— foo 与 b.foo 同名但无人消费；guard 只被 import 未使用\nfoo: () -> Int = () => { return 7 }\n\nguard: () -> Int = () => { return 8 }\n";
    let b = "// b.yx —— 被 main 真实使用的 foo\nfoo: () -> Int = () => { return 9 }\n";
    let main = "use std.assert\nuse a.{guard}\nuse b.{foo}\n\nmain: () -> Void = {\n    assert.assert(foo() == 9, \"b.foo() should be 9\")\n}\n";
    let (_dir, entry) = write_project(
        &[("a.yx", a), ("b.yx", b), ("main.yx", main)],
        Some(MANIFEST_DECLARED),
    );
    let a_path = entry.with_file_name("a.yx");
    let b_path = entry.with_file_name("b.yx");

    // Act：分别检查 a.yx / b.yx（池的根都是声明的包目标 main.yx）
    let a_outcome = run_check(&a_path).expect("check 必须返回产物");
    let b_outcome = run_check(&b_path).expect("check 必须返回产物");

    // Assert：a.foo（第 2 行）与 a.guard（第 4 行）都是死代码；b.foo（第 2 行）被
    // main 的 use b.{foo} + 调用消费，不得报
    assert_warnings(&a_outcome, "a.yx", &[("W1001", 2), ("W1001", 4)], &[]);
    assert_warnings(&b_outcome, "b.yx", &[], &[("W1001", 2)]);
}

/// 4.9.5：tests/ 的引用不豁免生产代码——测试文件引用 src/util.yx 的 debug_print，
/// 而 src/ 内无消费者时 debug_print 仍报 W1001（测试引用单独成池，只豁免测试自身）。
#[test]
fn test_ref_pool_excludes_test_role_references() {
    // Arrange：src/util.yx 的 debug_print 只被 tests/helper.yx 引用；helper 被 main 消费
    let util = "// src/util.yx —— debug_print 在 src/ 内无消费者（4.9.5 判据）\ndebug_print: (x: Int) -> Void = (x) => {\n    return\n}\n\nhelper: () -> Int = () => { return 1 }\n";
    let main = "// 项目入口：只消费 src/util.yx 的 helper\nuse src.util.{helper}\n\nmain: () -> Void = {\n    n = helper()\n}\n";
    let test_helper = "// tests/helper.yx —— 测试代码：引用 src/util.yx 的 debug_print\nuse src.util.{debug_print}\n\nprobe: () -> Void = () => {\n    debug_print(1)\n}\n";
    let (_dir, entry) = write_project(
        &[
            ("src/util.yx", util),
            ("main.yx", main),
            ("tests/helper.yx", test_helper),
        ],
        Some(MANIFEST),
    );
    let util_path = entry.with_file_name("src").join("util.yx");
    let test_path = entry.with_file_name("tests").join("helper.yx");

    // Act：① 以测试文件为被检入口（其编译集里含 src/util.yx，测试引用最接近消费者）
    // ② 以生产文件为被检入口（src/ 内无消费者）
    let from_test = run_check(&test_path).expect("check 必须返回产物");
    let from_src = run_check(&util_path).expect("check 必须返回产物");

    // Assert：debug_print（第 2 行）在两种入口下都必须报
    assert_warnings(&from_test, "src/util.yx", &[("W1001", 2)], &[]);
    assert_warnings(&from_src, "src/util.yx", &[("W1001", 2)], &[]);
}
