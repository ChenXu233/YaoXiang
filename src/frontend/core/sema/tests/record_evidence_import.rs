//! B.4 导入面直驱判据（T06 / T16 / T18 / T23）
//!
//! 为什么走「生产导出管线 + 生产导入入口」：payload 由
//! `orchestrator::extract_module_info`（生产唯一导出构造点）产出，导入侧走
//! `TypeChecker::check_module` 的 `use` 解析 → `register_use_export` →
//! `register_type_payload`（生产唯一导入写入点）。手工构造登记数据而消费端无直驱
//! 证据，正是 coding-rules 第六部分点名的「静默通道」。
//!
//! 边界：本文件只钉判据本身；多文件语义另有 `tests/yaoxiang-multifile/` 夹具，
//! IR 形状见 `src/middle/ir/tests/record_evidence_layout.rs`（T15）。

use crate::frontend::core::lexer::tokenize;
use crate::frontend::core::parser::parse_with_probe;
use crate::frontend::core::sema::checker::TypeChecker;
use crate::frontend::core::sema::probe::{SemanticProbe, TypeLevelNamespace};
use crate::frontend::core::sema::TypeCheckResult;
use crate::frontend::module::orchestrator;
use crate::frontend::module::ModuleInfo;
use crate::util::content_hash::ContentHash;

/// 定义方模块源码：单参数谓词被内联（H1），`Box` 带一条 evidence 槽。
const LIB_BOX: &str = "Pos: (v: Int) -> Type = { v > 0 }\n\nBox: Type = {\n    v: Int,\n    w: Pos(v),\n    z: Int,\n}\n";

/// 定义方模块源码：多参数证明函数 `Eq` 不会被内联，故导出 `libraries` 有登记项。
const LIB_BOX2: &str =
    "Eq: (a: Int, b: Int) -> Type = { a > b }\n\nBox2: Type = {\n    v: Int,\n    w: Eq(v, 3),\n}\n";

/// 用**生产导出构造点**把一段源码变成模块导出面。
fn lib_module(source: &str) -> ModuleInfo {
    let tokens = tokenize(source).expect("lib 源码必须可词法分析");
    let parsed = parse_with_probe(&tokens, &SemanticProbe);
    assert!(
        !parsed.has_errors,
        "lib 源码必须可解析：{:?}",
        parsed.errors
    );
    orchestrator::extract_module_info(
        "lib",
        &parsed.module,
        &TypeLevelNamespace::scan_one(&parsed.module),
    )
}

/// 取某类型导出的 payload（夹具前提：必须存在）。
fn payload_mut<'a>(
    module: &'a mut ModuleInfo,
    type_name: &str,
) -> &'a mut crate::frontend::module::TypeDefPayload {
    module
        .exports
        .get_mut(type_name)
        .unwrap_or_else(|| panic!("lib 必须导出 {type_name}"))
        .type_payload
        .as_mut()
        .unwrap_or_else(|| panic!("{type_name} 必须携带 type_payload"))
}

/// 走真实入口检查流程（`use` 解析 → 导入写入点），模块集预先注册。
fn check_entry(
    source: &str,
    modules: Vec<ModuleInfo>,
) -> TypeCheckResult {
    let tokens = tokenize(source).expect("入口源码必须可词法分析");
    let parsed = parse_with_probe(&tokens, &SemanticProbe);
    assert!(
        !parsed.has_errors,
        "入口源码必须可解析：{:?}",
        parsed.errors
    );
    let mut checker = TypeChecker::new("entry");
    for module in modules {
        checker.env().module_registry.register(module);
    }
    checker.check_module_collect_all(
        &parsed.module,
        &TypeLevelNamespace::scan_one(&parsed.module),
    )
}

/// 检查 + 证明函数执行（与 driver 的 ProofExecution 臂同一个执行器）。
///
/// 构造义务的 Unproven（证明调用）在这一步才结算——只看 typecheck 会漏掉
/// E4018，那正是 B.4 之前「静默放行」的形态。
fn check_entry_with_proofs(
    source: &str,
    modules: Vec<ModuleInfo>,
) -> (TypeCheckResult, Vec<crate::util::diagnostic::Diagnostic>) {
    let tokens = tokenize(source).expect("入口源码必须可词法分析");
    let parsed = parse_with_probe(&tokens, &SemanticProbe);
    assert!(
        !parsed.has_errors,
        "入口源码必须可解析：{:?}",
        parsed.errors
    );
    let mut checker = TypeChecker::new("entry");
    for module in modules {
        checker.env().module_registry.register(module);
    }
    let result = checker.check_module_collect_all(
        &parsed.module,
        &TypeLevelNamespace::scan_one(&parsed.module),
    );
    let proof_errors = crate::frontend::proof_execution::execute_proof_calls(
        result.proof_calls(),
        &parsed.module,
        &result,
    );
    (result, proof_errors)
}

fn codes_of(result: &TypeCheckResult) -> Vec<&str> {
    result
        .diagnostics
        .iter()
        .map(|diagnostic| diagnostic.code.as_str())
        .collect()
}

// ── T06：载荷指纹完整性（直驱构造篡改）─────────────────────────────────

/// T06：注册**指纹不符**的 payload ⇒ 恰一条 E5002，且模型不注册。
///
/// 用户可见形态：错误码 E5002 有独立的码表项与文案（`import_error` 两参数
/// module/reason），本断言同时钉住 reason 段被填充。
#[test]
fn test_tampered_payload_fingerprint_reports_e5002_and_skips_model() {
    // Arrange — 篡改导出侧指纹（模拟内存/载体被改写）
    let mut module = lib_module(LIB_BOX);
    payload_mut(&mut module, "Box").signature_hash = ContentHash::of_text("tampered-payload");

    // Act
    let result = check_entry(
        "use lib.{Box}\n\nmain: () -> Void = {\n    b = Box(3, 5)\n}\n",
        vec![module],
    );

    // Assert
    let codes = codes_of(&result);
    assert_eq!(
        codes,
        vec!["E5002"],
        "指纹不符必须恰报一条 E5002（不得静默、不得连带其它码）；实际: {:?}",
        result.diagnostics
    );
    let diagnostic = &result.diagnostics[0];
    assert!(
        diagnostic.message.contains("证据载荷指纹不一致"),
        "E5002 必须带 reason 段（用户可见）；实际: {}",
        diagnostic.message
    );
    assert!(
        !result.record_models.contains_key("Box") && result.record_models.is_empty(),
        "指纹不符的模型不得注册；实际键: {:?}",
        result.record_models.keys().collect::<Vec<_>>()
    );
}

// ── T16：导入模型的双键同源 ───────────────────────────────────────────

/// T16：`register_type_payload` 后 `record_models` 同时有**本地名**与**定义名**
/// 两键，且两键同源（同一模型 clone）；`TypeCheckResult.record_models` 含导入模型。
#[test]
fn test_imported_record_model_has_local_and_definition_keys() {
    // Arrange — 别名导入：本地名 B、定义名 Box
    let module = lib_module(LIB_BOX);

    // Act
    let result = check_entry(
        "use lib.{Box as B}\n\nmain: () -> Void = {\n    b = B(3, 5)\n}\n",
        vec![module],
    );

    // Assert
    assert_eq!(
        codes_of(&result),
        Vec::<&str>::new(),
        "别名导入正控必须零诊断；实际: {:?}",
        result.diagnostics
    );
    let local = result
        .record_models
        .get("B")
        .expect("本地名 B 必须有导入模型");
    let defined = result
        .record_models
        .get("Box")
        .expect("定义名 Box 必须有同源模型（H4 双键）");
    assert!(
        local.from_payload && defined.from_payload,
        "双键都必须标记 from_payload"
    );
    assert_eq!(
        local.name, "Box",
        "导入模型名保持导出方定义名（M08 反向锚）"
    );
    assert_eq!(
        local.canonical_text(),
        defined.canonical_text(),
        "两键必须同源（同一模型 clone，不得各存一份）"
    );
    assert_eq!(local.fields.len(), 3, "恢复的模型字段数必须与导出方一致");
    assert!(
        local.is_witness(1),
        "w 必须是 witness 字段（量词随模型恢复）"
    );
}

// ── T23：未登记来源负控 ───────────────────────────────────────────────

/// 别名导入入口源码：本地声明**同源**的 `Eq`（可本地执行），构造 `Box2(5)`。
const ENTRY_BOX2_WITH_LOCAL_EQ: &str = "use lib.{Box2}\n\nEq: (a: Int, b: Int) -> Type = { a > b }\n\nmain: () -> Void = {\n    b = Box2(5)\n}\n";

/// T23：手工构造「`Call` 头无对应 `LibraryRef`」的导入模型 ⇒ 拒绝（E4020）。
///
/// 防「列表缺项时静默放行」：本地**确实**有可执行的同源 `Eq`，唯一缺的是
/// payload 的登记项——前置检查必须凭「未登记」拒绝，而不是凭「不可执行」。
#[test]
fn test_imported_model_without_library_ref_reports_e4020() {
    // Arrange — 清空 libraries（其余投影不动），并重算指纹使完整性检查通过
    let mut module = lib_module(LIB_BOX2);
    {
        let payload = payload_mut(&mut module, "Box2");
        let model = payload
            .record_model
            .as_mut()
            .expect("Box2 必须携带 record_model");
        assert!(
            !model.libraries.is_empty(),
            "夹具前提：lib 的 Box2 必须登记 Eq 来源；实际: {:?}",
            model.libraries
        );
        assert!(
            model.obligations.iter().any(|obligation| obligation
                .proposition
                .constraint
                .call_heads()
                .contains(&"Eq")),
            "夹具前提：义务必须是 Call{{Eq}} 形态（否则走不到来源前置检查）"
        );
        model.libraries.clear();
        payload.signature_hash = model.signature_hash();
    }

    // Act
    let result = check_entry(ENTRY_BOX2_WITH_LOCAL_EQ, vec![module]);

    // Assert
    let codes = codes_of(&result);
    assert_eq!(
        codes,
        vec!["E4020"],
        "缺登记来源必须恰一条 E4020（不连锁 E4018）；实际: {:?}",
        result.diagnostics
    );
}

/// T23 正控 / T18 行为面：来源已登记且本地同源 ⇒ **通过**前置检查，义务交唯一
/// 判定单元重证（此处 `Eq(5,3)` = `5 > 3` 成立 ⇒ 零诊断）。
#[test]
fn test_imported_model_with_registered_library_ref_is_accepted() {
    // Arrange — 完整 payload（libraries 有 Eq 登记项）
    let module = lib_module(LIB_BOX2);

    // Act
    let (result, proof_errors) = check_entry_with_proofs(ENTRY_BOX2_WITH_LOCAL_EQ, vec![module]);

    // Assert
    assert_eq!(
        codes_of(&result),
        Vec::<&str>::new(),
        "来源齐备 + 本地同源 ⇒ 不该有类型期诊断；实际: {:?}",
        result.diagnostics
    );
    assert!(
        proof_errors.is_empty(),
        "证明执行也必须通过（5 > 3）；实际: {:?}",
        proof_errors
    );
}

/// T18 行为面：导入构造点的反例由**唯一判定单元**给出（E4018 携模型约束文本）。
#[test]
fn test_imported_construction_false_obligation_reports_e4018() {
    // Arrange — `Box2(3)` 的义务是 `Eq(3,3)` = `3 > 3`（假）
    let module = lib_module(LIB_BOX2);
    let source = ENTRY_BOX2_WITH_LOCAL_EQ.replace("Box2(5)", "Box2(3)");

    // Act
    let (result, proof_errors) = check_entry_with_proofs(&source, vec![module]);

    // Assert
    let all: Vec<&crate::util::diagnostic::Diagnostic> = result
        .diagnostics
        .iter()
        .chain(proof_errors.iter())
        .collect();
    let violated = all
        .iter()
        .find(|diagnostic| diagnostic.code.as_str() == "E4018")
        .unwrap_or_else(|| panic!("构造义务为假必须报 E4018；实际: {all:?}"));
    assert!(
        violated.message.contains("'Eq'") && violated.message.contains("[Int(3), Int(3)]"),
        "E4018 必须由唯一判定单元结算：消息须点名模型里的证明函数 Eq 及其实参 [Int(3), Int(3)]；实际: {}",
        violated.message
    );
}

// ── T18：单源判定（grep 级）────────────────────────────────────────────

/// 生产源码（排除 `tests/` 目录）里某模式出现的文件与次数。
fn production_hits(pattern: &str) -> Vec<(String, usize)> {
    fn walk(
        dir: &std::path::Path,
        pattern: &str,
        out: &mut Vec<(String, usize)>,
    ) {
        let Ok(entries) = std::fs::read_dir(dir) else {
            return;
        };
        let mut paths: Vec<std::path::PathBuf> = entries
            .filter_map(|entry| entry.ok().map(|entry| entry.path()))
            .collect();
        paths.sort();
        for path in paths {
            if path.is_dir() {
                if path.file_name().is_some_and(|name| name == "tests") {
                    continue;
                }
                walk(&path, pattern, out);
            } else if path.extension().is_some_and(|ext| ext == "rs") {
                let Ok(text) = std::fs::read_to_string(&path) else {
                    continue;
                };
                let count = text.matches(pattern).count();
                if count > 0 {
                    let relative = path
                        .strip_prefix(env!("CARGO_MANIFEST_DIR"))
                        .unwrap_or(&path)
                        .to_string_lossy()
                        .replace('\\', "/");
                    out.push((relative, count));
                }
            }
        }
    }

    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
    let mut out = Vec::new();
    walk(&root, pattern, &mut out);
    out
}

/// T18：构造义务的判定**只有一个入口**（`check_record_ctor_obligations`：
/// 定义 + 唯一调用点），且证明判定单元只在 `checker/refinement.rs` 被调用。
#[test]
fn test_constructor_obligation_judgment_has_single_entry() {
    // Arrange
    let ctor_entry = production_hits("check_record_ctor_obligations");
    let judge = production_hits("judge_refined_obligation_with_mode");

    // Assert — 构造义务入口：一个文件、定义 + 调用共 2 处
    assert_eq!(
        ctor_entry,
        vec![(
            "src/frontend/core/sema/checker/refinement.rs".to_string(),
            2
        )],
        "构造义务入口必须唯一（定义 + 唯一调用点）；实际: {ctor_entry:?}"
    );
    // Assert — 判定单元（含其定义/包装）只在该文件出现，没有第二个模块自建判定
    assert_eq!(
        judge.len(),
        1,
        "判定单元只允许出现在 checker/refinement.rs；实际: {judge:?}"
    );
    assert_eq!(
        judge[0].0, "src/frontend/core/sema/checker/refinement.rs",
        "判定单元不得外溢到其它模块；实际: {judge:?}"
    );
    // Assert — 构造义务的证明来源前置检查（E4020）也只在该文件发射一次
    let e4020 = production_hits("proof_function_required()");
    assert!(
        e4020.iter().any(
            |(path, count)| path == "src/frontend/core/sema/checker/refinement.rs" && *count == 1
        ),
        "构造点来源前置检查必须恰一条发射点；实际: {e4020:?}"
    );
}
// ── T23 增补（契约增补）：同名不同绑定的来源不得被当作同一来源 ──────────

/// T23 增补：payload 的 `libraries` 里同名两条**不同指纹**（= 同名两次绑定）⇒ 拒绝。
///
/// 旧行为按名字取第一条（`find(|l| l.id == head)`）——正好命中匹配的那条就放行，
/// 等于把「同名不同绑定」当成同一来源（F-1 同病史：按名不按绑定）。
#[test]
fn test_imported_model_with_ambiguous_library_ref_is_rejected() {
    // Arrange — 复制一份 Eq 来源并改指纹（模拟同名两次绑定），重算 payload 指纹
    let mut module = lib_module(LIB_BOX2);
    {
        let payload = payload_mut(&mut module, "Box2");
        let model = payload
            .record_model
            .as_mut()
            .expect("Box2 必须携带 record_model");
        let mut second = model.libraries[0].clone();
        second.hash = ContentHash::of_text("another-eq-binding");
        model.libraries.push(second);
        payload.signature_hash = model.signature_hash();
    }

    // Act
    let result = check_entry(ENTRY_BOX2_WITH_LOCAL_EQ, vec![module]);

    // Assert
    assert_eq!(
        codes_of(&result),
        vec!["E4020"],
        "同名不同绑定的来源必须失败闭合（不得按名字取第一条放行）；实际: {:?}",
        result.diagnostics
    );
}
