//! B.4 契约 T01–T05：TypeDefPayload 的证据载体与指纹（05a §P6 6.9.5-B.4 §十）。

use crate::frontend::core::lexer::tokenize;
use crate::frontend::core::parser::parse_with_probe;
use crate::frontend::core::sema::checker::TypeChecker;
use crate::frontend::core::sema::predicate_resolver::collect_predicate_defs;
use crate::frontend::core::sema::probe::{SemanticProbe, TypeLevelNamespace};
use crate::frontend::module::TypeDefPayload;

fn setup(source: &str) -> (TypeChecker, crate::ast::Module) {
    let tokens = tokenize(source).expect("夹具必须能词法分析");
    let parsed = parse_with_probe(&tokens, &SemanticProbe);
    assert!(!parsed.has_errors, "夹具必须能解析");
    let predicate_defs = collect_predicate_defs(&parsed.module);
    let namespace = TypeLevelNamespace::scan_one(&parsed.module);
    let mut checker = TypeChecker::new("payload-test");
    checker.collect_signatures(&parsed.module, &namespace, &predicate_defs);
    (checker, parsed.module)
}

fn definition<'a>(
    module: &'a crate::ast::Module,
    name: &str,
) -> &'a crate::ast::Type {
    module
        .items
        .iter()
        .find_map(|stmt| match &stmt.kind {
            crate::ast::StmtKind::TypeDefinition {
                name: def_name,
                definition,
                ..
            } if def_name == name => Some(definition),
            _ => None,
        })
        .expect("夹具必须含目标类型定义")
}

fn payload_of(
    source: &str,
    name: &str,
) -> Option<TypeDefPayload> {
    let (checker, module) = setup(source);
    checker.type_def_export_payload(name, definition(&module, name))
}

const BOX_SOURCE: &str = "Pos: (v: Int) -> Type = { v > 0 }\n\nBox: Type = {\n    v: Int,\n    w: Pos(v),\n    z: Int,\n}\n";

#[test]
fn test_evidence_record_payload_carries_record_model() {
    // Arrange + Act
    let payload = payload_of(BOX_SOURCE, "Box").expect("证据记录必须有 payload");
    let model = payload
        .record_model
        .as_ref()
        .expect("Box 必须携带 record_model");

    // Assert
    assert_eq!(model.name, "Box", "模型名必须是定义名");
    assert_eq!(model.evidence.len(), 1, "Box 恰一条证据槽");
    assert_eq!(model.obligations.len(), 1, "Box 恰一条构造义务");
    let proposition = format!("{}", model.obligations[0].proposition.constraint);
    assert!(
        !proposition.contains("Pos("),
        "H1 后命题必须是内联形态而非 Call{{Pos}}；实际: {proposition}"
    );
    assert!(
        proposition.contains("> 0"),
        "命题应保留谓词体语义；实际: {proposition}"
    );
}

#[test]
fn test_generic_evidence_record_payload_has_model() {
    // Arrange
    let source = "Pos: (v: Int) -> Type = { v > 0 }\n\nG: (T: Type) -> Type = {\n    v: T,\n    w: Pos(v),\n}\n";

    // Act
    let payload = payload_of(source, "G").expect("泛型记录必须有 payload");

    // Assert
    assert!(
        payload.type_def.is_some(),
        "泛型模板必须沿既有 type_def 携带"
    );
    assert!(
        payload.record_model.is_some(),
        "泛型证据记录必须携带 record_model"
    );
}

#[test]
fn test_pure_data_record_payload_stays_none() {
    // Arrange
    let source = "Point: Type = {\n    x: Int,\n    y: Int,\n}\n";

    // Act + Assert
    assert!(
        payload_of(source, "Point").is_none(),
        "纯数据记录必须保持 None（零行为）"
    );
}

#[test]
fn test_signature_hash_is_recomputable_and_stable() {
    // Arrange
    let payload = payload_of(BOX_SOURCE, "Box").expect("证据记录必须有 payload");
    let model = payload.record_model.as_ref().expect("必须携带模型");

    // Act
    let recomputed = model.signature_hash();
    let hex = payload.signature_hash.to_hex();

    // Assert
    assert_eq!(
        payload.signature_hash, recomputed,
        "payload 指纹必须等于模型重算值"
    );
    assert_eq!(hex.len(), 64, "指纹必须是 64 位十六进制；实际: {hex}");
    assert_ne!(
        payload.signature_hash,
        crate::util::content_hash::ContentHash::of_text(""),
        "真实模型的指纹不得等于空文本指纹"
    );
}

#[test]
fn test_libraries_carry_proof_function_sources_only() {
    // Arrange
    let source = "Eq: (a: Int, b: Int) -> Type = { a > b }\n\nBox2: Type = {\n    v: Int,\n    w: Eq(v, 3),\n}\n";

    // Act
    let payload = payload_of(source, "Box2").expect("Box2 必须有 payload");
    let libraries = &payload.record_model.as_ref().expect("必须有模型").libraries;

    // Assert
    assert_eq!(
        libraries.len(),
        1,
        "Box2 恰一个证明函数来源；实际: {libraries:?}"
    );
    assert_eq!(libraries[0].id, "Eq", "来源必须是 Eq");
    assert_eq!(
        libraries[0].kind,
        crate::frontend::core::sema::record_model::LibraryKind::Proof,
        "V1 只产 Proof 类来源"
    );
    let box_payload = payload_of(BOX_SOURCE, "Box").expect("Box 必须有 payload");
    assert!(
        box_payload
            .record_model
            .as_ref()
            .expect("模型")
            .libraries
            .is_empty(),
        "谓词已内联，不应登记为库"
    );
}
