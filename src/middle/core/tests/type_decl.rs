//! 类型声明的 IR 原生载体（`TypeDeclBody`）— 基于 RFC-039 WBS 6.6.12
//!
//! 判据（两条）：
//! 1. **载体收窄**：只带字段表（字段数 + 字段类型 `MonoType`）与字节码类型表项；
//!    默认值**表达式** / const 界表达式 / 匿名方法绑定 / span 都不进载体——结构上无处可放，
//!    本文件以「字段类型是 `MonoType`、默认值只剩 `has_default` 标记」取证。
//! 2. **字段分类**：函数类型字段（方法，SPEC `type-system.md:443`）`is_method == true`，
//!    数据字段 `== false`。这是 6.6.12 唯一新增的语义面，必须由断言守住。
//!
//! **本文件不断言构造参数个数**：`translator::synthesize_constructor` 目前仍按历史行为
//! 把两类字段都算进构造参数（与 SPEC 443 的已知不一致，登记见 09a §P6 6.6.12），
//! 改它属另一轮的可观察变更。

use crate::frontend::core::lexer::tokenize;
use crate::frontend::core::parser::parse;
use crate::frontend::core::typecheck::checker::TypeChecker;
use crate::frontend::core::typecheck::MonoType;
use crate::frontend::module::registry::ModuleRegistry;
use crate::middle::core::ir::{FunctionBody, ModuleIR, TypeDeclBody};
use crate::middle::core::ir_gen::AstToIrGenerator;

/// 源码 → module IR：与语料快照判据（`tests/snapshot.rs`）同一条
/// lex → parse → typecheck → ir_gen 生成序列。
fn generate_ir(source: &str) -> ModuleIR {
    let tokens = tokenize(source).expect("测试源码词法不应失败");
    let parsed = parse(&tokens);
    assert!(
        !parsed.has_errors,
        "测试源码语法必须正确（否则无法区分 bug 与测试写错）: {:?}",
        parsed.errors
    );
    let mut checker = TypeChecker::new("type_decl_carrier");
    let type_result = checker.check_module(
        &parsed.module,
        &crate::frontend::core::typecheck::probe::TypeLevelNamespace::scan_one(&parsed.module),
    );
    assert!(
        type_result.diagnostics.is_empty(),
        "测试源码类型检查应通过: {:?}",
        type_result.diagnostics
    );
    let mut generator =
        AstToIrGenerator::new_with_type_result(&type_result, ModuleRegistry::with_std(), None);
    generator
        .generate_module_ir(&parsed.module)
        .expect("IR 生成应成功")
}

/// 取名为 `name` 的类型声明载体；缺失或不是 TypeDecl 即带上下文 panic。
fn type_decl_body<'a>(
    module: &'a ModuleIR,
    name: &str,
) -> &'a TypeDeclBody {
    let func = module
        .functions
        .iter()
        .find(|f| f.name == name)
        .unwrap_or_else(|| {
            panic!(
                "应生成类型声明 {name}，实际函数表: {:?}",
                module.functions.iter().map(|f| &f.name).collect::<Vec<_>>()
            )
        });
    let FunctionBody::TypeDecl { body } = &func.body else {
        panic!("{name} 的 body 应是 TypeDecl");
    };
    body
}

#[test]
fn test_type_decl_classifies_function_typed_field_as_method() {
    // Arrange: 一个数据字段 + 一个函数类型字段（方法）
    let module = generate_ir("Box: Type = { value: Int, get: (a: Int) -> Int }");

    // Act
    let body = type_decl_body(&module, "Box");

    // Assert: 字段顺序 = 声明序，两类字段的分类正确
    let names: Vec<&str> = body.fields.iter().map(|f| f.name.as_str()).collect();
    assert_eq!(names, vec!["value", "get"], "字段顺序必须等于声明序");
    let value = &body.fields[0];
    assert_eq!(value.ty, MonoType::Int(64), "数据字段的类型应是 Int(64)");
    assert!(
        !value.is_method,
        "数据字段的 is_method 必须是 false（判定写反时本行先红）"
    );
    let get = &body.fields[1];
    assert!(
        get.is_method,
        "函数类型字段必须判为方法（SPEC type-system.md:443 的唯一依据；判定写反时本行红）"
    );
    assert!(
        matches!(get.ty, MonoType::Fn { .. }),
        "方法字段的字段类型仍是 Fn（分类标记不改变载荷形状），实际: {:?}",
        get.ty
    );
}

#[test]
fn test_type_decl_keeps_default_flag_but_not_expression() {
    // Arrange: 带默认值的字段（默认值表达式在 ir_gen 已降级成指令）
    let module = generate_ir("Config: Type = { retries: Int = 3 }");

    // Act
    let body = type_decl_body(&module, "Config");

    // Assert: 载体只留标记与类型，表达式无处可放
    assert_eq!(body.fields.len(), 1, "Config 应有一个字段");
    assert!(
        body.fields[0].has_default,
        "带默认值的字段必须置 has_default 标记"
    );
    assert_eq!(
        body.fields[0].ty,
        MonoType::Int(64),
        "字段类型是 MonoType（默认值表达式不进载体——它已降级成指令）"
    );
}

#[test]
fn test_type_decl_self_type_is_the_bytecode_type_table_entry() {
    // Arrange: 记录类型（构造函数型）与类型别名（Name 定义）各一
    let module = generate_ir(
        "Point: Type = { x: Int }\n\nAlias: Type = Int\n\nmain: () -> Void = { }\n\nmain()",
    );

    // Act
    let point = type_decl_body(&module, "Point");
    let alias = type_decl_body(&module, "Alias");

    // Assert: 与 6.6.12 前 `codegen::type_from_ast` 的逐臂结果一致——
    // `Struct` 落 `_ => Void`，`Name` 落 `TypeRef(name)`。
    assert_eq!(
        point.self_type,
        MonoType::Void,
        "记录类型的类型表项应是 Void（原 type_from_ast 对 Struct 的既有行为）"
    );
    assert_eq!(
        alias.self_type,
        MonoType::TypeRef("Int".to_string()),
        "别名定义的字段表为空、类型表项是 TypeRef(被别名指向的名字)"
    );
    assert!(alias.fields.is_empty(), "别名定义没有字段");
}
