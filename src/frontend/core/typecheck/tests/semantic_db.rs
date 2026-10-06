//! SemanticDB 模块测试
//!
//! 测试语义数据库功能与生产端语义事件管线（#433/RFC-039 D55）：
//! - checker 语义事件排空：definitions/references/imports 由真实 typecheck 管线产出
//! - resolve_reference 往返：引用 resolves_to 与定义 def_id 同源衔接（跳转定义数据链）
//! - dummy 定义过滤：导入名/占位绑定不产生定义事件，precise-only 降级
//! - upsert_from：按文件替换，会话级数据（std/内置类型）保留
//!
//! 静默通道判定（coding-rules 第六部分）：本文件即「生产端直驱」测试——
//! 语义数据全部来自真实 tokenize → parse → check 管线，不手工构造登记数据。

use crate::frontend::core::lexer::tokenize;
use crate::frontend::core::parser::parse;
use crate::frontend::core::typecheck::TypeChecker;
use crate::frontend::core::typecheck::semantic_db::SemanticDB;
use crate::util::span::Span;

/// 以生产管线检查源码，返回文件键为 `uri` 的 SemanticDB
fn check_to_semantic_db(
    uri: &str,
    source: &str,
) -> SemanticDB {
    // Act：tokenize → parse → check_module_collect_all（与 server.rs 同管线）
    let tokens = tokenize(source).expect("测试源码必须可词法分析");
    let parsed = parse(&tokens);
    let mut checker = TypeChecker::new(uri);
    let result = checker.check_module_collect_all(&parsed.module);
    result.semantic_db
}

#[test]
fn test_semantic_events_link_references_to_binding_definitions() {
    // Arrange：第二行 `y = x + x` 对 x 有两个引用点，定义在第一行
    let uri = "file:///t.yx";

    // Act：真实管线产出 SemanticDB
    let db = check_to_semantic_db(uri, "x = 1\ny = x + x\n");

    // Assert：x 有定义事件与两个引用事件，resolves_to 指回定义
    let defs = db.get_definitions(uri);
    let x_def = defs
        .iter()
        .find(|d| d.name == "x")
        .expect("顶层绑定 x 必须产出定义事件");
    assert_eq!(x_def.span.start.line, 1, "x 定义在第一行");

    let refs = db.get_references(uri);
    let x_refs: Vec<_> = refs.iter().filter(|r| r.name == "x").collect();
    assert_eq!(x_refs.len(), 2, "y = x + x 应产出两个 x 引用事件");
    for r in &x_refs {
        assert_eq!(r.span.start.line, 2, "x 引用在第二行");
        assert_eq!(
            r.resolves_to.span, x_def.def_id.span,
            "引用 resolves_to 必须指向 x 定义（同源 VarInfo.definition_span）"
        );
    }
}

#[test]
fn test_resolve_reference_round_trip_for_goto_definition() {
    // Arrange：同一 x 样例；引用点在第二行第 5 列（1-indexed）
    let uri = "file:///t.yx";
    let db = check_to_semantic_db(uri, "x = 1\ny = x + x\n");

    // Act：以引用起点精确解析（跳转定义的核心查询路径）
    let def = db.resolve_reference(uri, 2, 5);

    // Assert：解析回 x 的定义
    let def = def.expect("x 引用点必须解析到定义（跳转定义数据链闭合）");
    assert_eq!(def.name, "x", "解析目标应为 x");
    assert_eq!(def.span.start.line, 1, "定义在第一行");
}

#[test]
fn test_lambda_params_produce_definitions_and_body_references() {
    // Arrange：lambda 参数在体内被引用
    let uri = "file:///t.yx";
    let source = "add = (a, b) => a + b\n";

    // Act
    let db = check_to_semantic_db(uri, source);

    // Assert：参数 a 有定义事件，且体内 a 的引用 resolves_to 指回该定义
    let defs = db.get_definitions(uri);
    let a_def = defs
        .iter()
        .find(|d| d.name == "a")
        .expect("lambda 参数 a 必须产出定义事件");
    let refs = db.get_references(uri);
    let a_ref = refs
        .iter()
        .find(|r| r.name == "a")
        .expect("体内 a 必须产出引用事件");
    assert_eq!(
        a_ref.resolves_to.span, a_def.def_id.span,
        "参数引用必须指回参数定义"
    );
}

#[test]
fn test_use_statement_registers_import_and_skips_dummy_definitions() {
    // Arrange：模块整体导入（导入名 definition_span 为 dummy，不得入定义表）
    let uri = "file:///t.yx";
    let source = "use std.list\nmain = () => ()\n";

    // Act
    let db = check_to_semantic_db(uri, source);

    // Assert：use 语句产出导入事件；导入名不产生本文件定义事件
    let imports = db.get_imports(uri);
    assert!(
        imports.iter().any(|i| i.module_path == "std.list"),
        "use 语句必须产出导入事件（module_path=std.list）"
    );
    let defs = db.get_definitions(uri);
    assert!(
        defs.iter().all(|d| d.name != "list"),
        "导入名 definition_span 为 dummy，不得产生定义事件（precise-only）"
    );
}

#[test]
fn test_upsert_from_replaces_only_target_file_and_keeps_session_data() {
    // Arrange：会话库带一个 std 虚拟定义（World 启动加载的形态），随后
    // 依次 upsert 文件 A 与文件 B
    use crate::frontend::core::typecheck::semantic_db::{DefId, DefinitionInfo, DefinitionKind};
    let mut db = SemanticDB::new();
    let std_def = DefinitionInfo {
        def_id: DefId {
            file_path: "std://std.list".to_string(),
            span: Span::default(),
        },
        name: "list".to_string(),
        kind: DefinitionKind::Type,
        span: Span::default(),
        file_path: "std://std.list".to_string(),
        type_info: None,
        signature: None,
    };
    db.add_definition("std://std.list", std_def);

    // Act：先 upsert 文件 A，再 upsert 文件 B（模拟用户先后编辑两个文件）
    db.upsert_from(check_to_semantic_db("file:///a.yx", "x = 1\n"));
    db.upsert_from(check_to_semantic_db("file:///b.yx", "y = 2\n"));

    // Assert：A 的数据在 upsert B 后仍在（多文件不互踩），std 会话级定义保留
    assert!(
        !db.get_tokens("file:///a.yx").unwrap_or(&[]).is_empty(),
        "upsert 文件 B 不得清掉文件 A 的 tokens（#433 多文件互踩）"
    );
    assert!(
        !db.get_tokens("file:///b.yx").unwrap_or(&[]).is_empty(),
        "文件 B 的 tokens 应已入库"
    );
    assert!(
        !db.get_definitions("std://std.list").is_empty(),
        "std 会话级定义必须在 upsert 后保留"
    );
    assert!(
        db.get_definitions("file:///b.yx")
            .iter()
            .any(|d| d.name == "y"),
        "文件 B 的绑定应产出定义"
    );
}
