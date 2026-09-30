//! RFC-027 谓词定义注册（#377-3）测试
//!
//! 编译期谓词 `IsPositive: (x: Int) -> Type = { x > 0 }` 的定义此前**从未被
//! 登记**：`env.predicate_defs` 只读不写，于是 `IsPositive(b)` 解析成
//! `Refined { constraint: Call { "IsPositive", [b] } }`——**不透明应用**，
//! 求解器无法把它与 `b > 0` 联系起来，混合了谓词的命题（例如终止检查的良基性
//! `b >= 0`）一律推不出。
//!
//! 注册后约束是**代入实参的谓词体**（`PredicateResolver::try_resolve` 早已实现
//! 代入，只是被空表挡住），于是：
//! 1. 常量实参可直接折叠判伪——诊断在 `check_module` 内产生；
//! 2. 符号实参（形参名）留下自由变量，可进 SMT 与假设合证。
//!
//! 本文件钉住第 1 条（可观测），第 2 条是终止检查良基性的前置。

use crate::frontend::core::typecheck::checker::TypeChecker;
use crate::frontend::core::typecheck::types::TypeCheckResult;
use crate::frontend::core::lexer::tokenize;
use crate::frontend::core::parser::parse;
use crate::util::diagnostic::Diagnostic;

/// 解析并类型检查
fn check_source(source: &str) -> TypeCheckResult {
    let tokens = match tokenize(source) {
        Ok(t) => t,
        Err(e) => {
            return TypeCheckResult {
                diagnostics: vec![Diagnostic::error(
                    "E0001".to_string(),
                    format!("词法错误: {}", e),
                    String::new(),
                    None,
                )],
                ..Default::default()
            };
        }
    };
    let module = match parse(&tokens) {
        result if result.has_errors => {
            return TypeCheckResult {
                diagnostics: vec![Diagnostic::error(
                    "E0001".to_string(),
                    format!("解析错误: {:?}", result.errors),
                    String::new(),
                    None,
                )],
                ..Default::default()
            };
        }
        result => result.module,
    };
    let mut checker = TypeChecker::new("test");
    checker.check_module(&module)
}

/// 注册后约束是谓词体（可折叠），常量违反在 `check_module` 内即判出。
///
/// 注册前约束是不透明应用 `IsPositive(-5)`，须由 pipeline 执行证明函数才发现，
/// `check_module` 内零诊断。
#[test]
fn test_predicate_def_registration_makes_constraint_foldable() {
    // Arrange
    let source = "IsPositive: (x: Int) -> Type = { x > 0 }\n\
                  main: () -> Void = {\n    y: IsPositive(-5) = 5\n}";

    // Act
    let result = check_source(source);

    // Assert
    assert!(
        !result.diagnostics.is_empty(),
        "谓词定义注册后约束应为可折叠的谓词体 (-5 > 0)，违反须在 check_module \
         内即判出；实际零诊断（说明约束仍是不透明应用）"
    );
}

/// 合法值不得误报（反面守卫）。
#[test]
fn test_predicate_def_registration_keeps_valid_value_clean() {
    // Arrange
    let source = "IsPositive: (x: Int) -> Type = { x > 0 }\n\
                  main: () -> Void = {\n    y: IsPositive(5) = 5\n}";

    // Act
    let result = check_source(source);

    // Assert
    assert!(
        result.diagnostics.is_empty(),
        "满足约束的值不得报错；实际: {:#?}",
        result.diagnostics
    );
}

/// 前置条件采集：形参精化（**谓词体**）进终止检查器的假设集。
///
/// 这是生产接线点 `collect_param_refinements` 的唯一测试。判定侧（良基性
/// `m >= 0` 的 SMT 判定）另有单测；此处只钉住**采集**——它从 env 的已解析
/// 函数类型里取约束，形态必须是谓词体 `b > 0` 而非不透明应用
/// `IsPositive(b)`，否则 SMT 见到未解释函数、良基性永远判不出。
#[test]
fn test_collect_param_refinements_yields_predicate_body() {
    use crate::frontend::core::types::const_data::{BinOp, ConstExpr, ConstValue};

    // Arrange
    let source = "IsPositive: (x: Int) -> Type = { x > 0 }\n\
                  f: (b: IsPositive(b)) -> Int = { 0 }";
    let tokens = tokenize(source).expect("词法分析应成功");
    let parsed = parse(&tokens);
    assert!(!parsed.has_errors, "解析应无错误: {:?}", parsed.errors);
    let mut checker = TypeChecker::new("test");
    checker.check_module(&parsed.module);

    // Act
    let collected = checker.collect_param_refinements();

    // Assert
    let expected = vec![ConstExpr::BinOp {
        op: BinOp::Gt,
        left: Box::new(ConstExpr::NamedVar("b".to_string())),
        right: Box::new(ConstExpr::Lit(ConstValue::Int(0))),
    }];
    assert_eq!(
        collected.get("f"),
        Some(&expected),
        "形参 b: IsPositive(b) 的前置条件应为谓词体 b > 0；实际: {collected:?}"
    );
}
