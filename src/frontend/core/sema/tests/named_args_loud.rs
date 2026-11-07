//! #452 复审 F1：命名实参调用**编译期必响亮**（先红后绿判据）
//!
//! 规范来源:
//! - 用户 2026-10-14 裁决⑥（重载候选选择必须把命名实参计入匹配，禁止静默择一）
//! - #452 自订契约（冲突/缺参一律编译期报错）
//! - 批次 t9 / t3 复审 F1（多候选零命中时命名实参被静默接受）
//!
//! 红态（改动前，主仓真实序列实测）：含命名实参的方法调用在**决议零命中**时
//! 整段跳过元数与逐参类型检查——`check` exit=0、`run` 打印错值或 void；
//! 同调用点的位置实参形态却正确报 E1002/E1010（形态不对称）。
//!
//! 本文件把这些形态收成**仓内夹具**（外部探针包不入库），逐条断言编译期错误码。

use crate::frontend::core::sema::probe::TypeLevelNamespace;
use crate::frontend::core::lexer::tokenize;
use crate::frontend::core::parser::parse;
use crate::frontend::core::sema::checker::TypeChecker;

/// 编译期诊断码集合（真实序列：lex → parse → typecheck）。
fn compile_codes(source: &str) -> Vec<String> {
    let tokens = tokenize(source).expect("夹具必须可 tokenize");
    let parsed = parse(&tokens);
    assert!(
        !parsed.has_errors,
        "夹具必须无语法错误: {:?}",
        parsed.errors
    );
    let mut checker = TypeChecker::new("<named-args-loud>");
    let tr = checker.check_module(
        &parsed.module,
        &TypeLevelNamespace::scan_one(&parsed.module),
    );
    for d in tr.diagnostics.iter() {
        eprintln!("LOUD_DIAG {} :: {}", d.code, d.message);
    }
    tr.diagnostics.iter().map(|d| d.code.clone()).collect()
}

/// assert 侧同类：形参名与真实签名不符 ⇒ 编译期报错；相符 ⇒ 零诊断。
#[test]
fn test_native_assert_named_args_checked_by_signature() {
    // Arrange：native_assert 的签名是 (cond: Bool, ?msg: String) -> Never
    let ok = "use std.assert\nmain: () -> Void = {\n    assert.assert(cond = 1 == 1, msg = \"ok\")\n}\nmain()\n";
    let bad = "use std.assert\nmain: () -> Void = {\n    assert.assert(cond = 1 == 1, message = \"bad\")\n}\nmain()\n";

    // Act
    let ok_codes = compile_codes(ok);
    let bad_codes = compile_codes(bad);

    // Assert
    assert!(
        ok_codes.is_empty(),
        "assert(cond = …, msg = …) 形参名与签名相符，必须零诊断：{ok_codes:?}"
    );
    assert!(
        bad_codes.iter().any(|c| c == "E1014"),
        "assert(cond = …, message = …) 形参名不符，必须编译期 E1014（红态是运行期 E6007）：{bad_codes:?}"
    );
}
