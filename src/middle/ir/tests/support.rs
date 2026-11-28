//! IR 测试共用的直驱探针：源码 → 真实生成序列（lex → parse → typecheck → ir_gen）。
//!
//! 与 `src/middle/ir/tests/snapshots/` 的入库基线同源（不经 mono / 槽位分配），
//! 于是断言能读到 `Operand::Value` 与值表本体。多个判据文件共用这一处序列，
//! 避免各自复制一遍「先 tokenize 再 parse 再 check」的装配代码。

use crate::frontend::core::lexer::tokenize;
use crate::frontend::core::parser::parse;
use crate::frontend::core::sema::checker::TypeChecker;
use crate::frontend::core::sema::probe::TypeLevelNamespace;
use crate::frontend::module::registry::ModuleRegistry;
use crate::middle::ir::ir::ModuleIR;
use crate::middle::lower::AstToIrGenerator;
use crate::util::diagnostic::Severity;

/// 源码 → `ModuleIR`（失败即 panic：夹具必须可编译）。
pub(crate) fn compile_ir(source: &str) -> ModuleIR {
    let tokens = tokenize(source).expect("夹具必须可 tokenize");
    let parsed = parse(&tokens);
    assert!(
        !parsed.has_errors,
        "夹具必须无语法错误: {:?}",
        parsed.errors
    );
    let mut checker = TypeChecker::new("<ir-test-probe>");
    let type_result = checker.check_module(
        &parsed.module,
        &TypeLevelNamespace::scan_one(&parsed.module),
    );
    let errors: Vec<_> = type_result
        .diagnostics
        .iter()
        .filter(|d| matches!(d.severity, Severity::Error))
        .collect();
    assert!(errors.is_empty(), "夹具必须无类型错误: {errors:?}");
    let mut generator = AstToIrGenerator::new_with_type_result(
        &type_result,
        ModuleRegistry::with_std(),
        None,
        TypeLevelNamespace::scan_one(&parsed.module),
    );
    generator
        .generate_module_ir(&parsed.module)
        .expect("夹具必须生成 IR")
}
