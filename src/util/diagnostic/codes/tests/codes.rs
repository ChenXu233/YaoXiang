//! ErrorCodeDefinition 测试 — 基于 check-improvement 设计规范
//!
//! §3.4: i18n 修复（I3/I4: E1090/E1091 翻译）
//! §4.6: error! 宏简化
//! RFC-013 §错误码注册表：`DiagnosticBuilder::with_help` 的按站点 help 覆盖
//! （同码在不同位置需不同出路提示时的机制）
//! issue #469/#509（WBS 3.21.2/3.21.5）：per-site 平键机制——诊断 message/help
//! 文案在 locales、随 error_lang 渲染，发射点不硬编成品串；判据门 = 病史
//! （#469 六处硬编 help 迁移、#509 五项残留：E8001 降级文案 / E4018 constraint
//! 参数位 / render_key 护栏 / 四语言占位）。

use crate::util::diagnostic::codes::{ErrorCodeDefinition, I18nRegistry};
use crate::util::span::Span;

#[test]
fn test_error_code_find_existing_code() {
    let code = ErrorCodeDefinition::find("E0001");
    assert!(code.is_some(), "E0001 should exist in error code registry");
    assert_eq!(code.unwrap().code, "E0001");
}

#[test]
fn test_error_code_find_unknown_returns_none() {
    let code = ErrorCodeDefinition::find("E9999");
    assert!(
        code.is_none(),
        "E9999 should not exist in error code registry"
    );
}

#[test]
fn test_error_code_registry_has_minimum_count() {
    let all = ErrorCodeDefinition::all();
    assert!(
        all.len() > 30,
        "Expected more than 30 error codes, got {}",
        all.len()
    );
}

#[test]
fn test_i18n_registry_english_titles() {
    let en = I18nRegistry::new("en");
    // en 是 bot 翻译产物（zh 为源），措辞/大小写会随重翻变化——
    // 只锚定结构：title 取到了且不是 code 回退串
    let title = en.get_title("E0001");
    assert!(
        !title.is_empty() && title != "E0001",
        "E0001 en title should come from locale, got: {title}"
    );
    assert!(
        !en.get_help("E0001").is_empty(),
        "E0001 help text should not be empty"
    );
}

#[test]
fn test_i18n_registry_chinese_titles() {
    let zh = I18nRegistry::new("zh");
    assert_eq!(zh.get_title("E0001"), "无效字符");
    assert!(
        !zh.get_help("E0001").is_empty(),
        "E0001 help text should not be empty"
    );
}

#[test]
fn test_template_render_substitutes_params() {
    let en = I18nRegistry::new("en");
    let template = "Unknown variable: '{name}'";
    let params = vec![("name", "x".to_string())];
    let rendered = en.render(template, &params);
    assert_eq!(rendered, "Unknown variable: 'x'");
}

#[test]
fn test_diagnostic_builder_with_params_and_span() {
    let code = ErrorCodeDefinition::find("E0001").unwrap();

    let diagnostic = code
        .builder()
        .param("char", "@")
        .at(Span::default())
        .build();

    assert_eq!(diagnostic.code, "E0001");
    assert!(
        diagnostic.message.contains("@"),
        "message should contain param value"
    );
}

#[test]
fn test_with_help_overrides_code_default_help() {
    // Arrange
    // E1002 的 i18n 默认 help 是通用的「使用正确的类型或添加类型转换」，
    // 但同一码在不同站点可能需要不同的出路提示。
    let span = Span::default();
    let custom_help = "左侧是 Vec、右侧是 Int；请先取元素或解引用";

    // Act
    let with_override = ErrorCodeDefinition::type_mismatch("Int", "String")
        .at(span)
        .with_help(custom_help)
        .build();
    let without_override = ErrorCodeDefinition::type_mismatch("Int", "String")
        .at(span)
        .build();

    // Assert
    assert_eq!(
        with_override.help, custom_help,
        "with_help 应覆盖该码的默认 help"
    );
    assert_ne!(
        without_override.help, custom_help,
        "未调用 with_help 时不应出现调用方自定义的提示"
    );
}

// ============ issue #509 残留锚（WBS 3.21.5）============

/// 判据（issue #509 残留①）：E8001 span-violation 降级诊断的 message 文案
/// 在 locales（E8001.span_missing.message），随 error_lang 渲染；zh 形态必须
/// 与迁移前 effective_span 的硬编文案**逐字节一致**（只换载体不换文面）。
#[test]
fn test_e8001_span_missing_message_renders_zh_byte_identical_to_pre_migration() {
    // Arrange：zh 注册表（语言显式传入，不依赖进程级 error_lang 状态）
    let zh = I18nRegistry::new("zh");

    // Act：以诊断码为唯一变量参数渲染降级 message
    let rendered = zh
        .render_key(
            "E8001.span_missing.message",
            &[("code", "E4023".to_string())],
        )
        .expect("E8001.span_missing.message 应在 zh 存在（六文件键集对账测试兜底）");

    // Assert：与迁移前 `format!("诊断 {} 缺少 span：…", self.code)` 逐字节一致
    assert_eq!(
        rendered,
        "诊断 E4023 缺少 span：调用点未 .at(span) 且不在类型检查 walk 上下文中（编译器 bug，见 #324）",
        "zh 渲染必须与迁移前硬编文案逐字节一致（issue #509 残留①）。Got: {rendered}"
    );
}

/// 判据（issue #509 残留①）：E8001 span-violation 降级诊断的 help 文案在
/// locales；en 形态为新增真文案（迁移前 en 用户看到的也是硬编中文）。
#[test]
fn test_e8001_span_missing_help_renders_en_from_locales() {
    // Arrange
    let en = I18nRegistry::new("en");

    // Act
    let help = en
        .render_key("E8001.span_missing.help", &[])
        .expect("E8001.span_missing.help 应在 en 存在");

    // Assert
    assert_eq!(
        help,
        "Add .at(span) at the emission point, or construct the diagnostic inside a typecheck walk context",
        "en help 必须来自 locales 真文案（非占位、非硬编中文）。Got: {help}"
    );
    // zh help 同源断言：与迁移前 build 降级臂的硬编文案逐字节一致
    let zh_help = I18nRegistry::new("zh")
        .render_key("E8001.span_missing.help", &[])
        .expect("E8001.span_missing.help 应在 zh 存在");
    assert_eq!(
        zh_help, "请在调用点补 .at(span)，或在类型检查 walk 上下文内构造",
        "zh help 必须与迁移前硬编文案逐字节一致（issue #509 残留①）。Got: {zh_help}"
    );
}

/// 判据（issue #509 残留②）：E4018 proof_false 形态的 constraint 参数位句式
/// 在 locales（E4018.proof_false.constraint），{func}/{args} 为模板参数；
/// zh 形态与迁移前发射点 format! 硬编文案逐字节一致。
#[test]
fn test_e4018_proof_false_constraint_renders_zh_byte_identical_to_pre_migration() {
    // Arrange：参数与生产发射点同形（func 名 + 实参 Debug 文本）
    let zh = I18nRegistry::new("zh");
    let params = [
        ("func", "Check".to_string()),
        ("args", "[Int(7), Int(0)]".to_string()),
    ];

    // Act
    let rendered = zh
        .render_key("E4018.proof_false.constraint", &params)
        .expect("E4018.proof_false.constraint 应在 zh 存在");

    // Assert
    assert_eq!(
        rendered,
        "证明函数 'Check' 返回 false，约束不满足（参数: [Int(7), Int(0)]）；检查约束条件或修改传入值",
        "zh constraint 渲染必须与迁移前 format! 硬编文案逐字节一致（issue #509 残留②）。Got: {rendered}"
    );
}

/// 判据（issue #509 残留②）：E4018 proof_error 形态的 constraint 参数位句式
/// 在 locales；en 形态为新增真文案。{error} 是内部 Err 文本的插值位——内部
/// 文本登记不迁移（issue 原意：message/参数位中文指诊断输出面），不在断言面。
#[test]
fn test_e4018_proof_error_constraint_renders_en_from_locales() {
    // Arrange
    let en = I18nRegistry::new("en");
    let params = [
        ("func", "Missing".to_string()),
        ("error", "internal detail".to_string()),
    ];

    // Act
    let rendered = en
        .render_key("E4018.proof_error.constraint", &params)
        .expect("E4018.proof_error.constraint 应在 en 存在");

    // Assert：wrapper 为 en 真文案；内部 Err 内容经 {error} 原样带入
    assert_eq!(
        rendered, "Proof function 'Missing' failed to execute: internal detail",
        "en constraint wrapper 必须来自 locales 真文案（issue #509 残留②）。Got: {rendered}"
    );
}

/// 判据（issue #509 残留②的机制不变量）：注册表快捷方法先传入占位空串、
/// `param_from_key` 后登记**同键**真值时，并入次序决定后者生效——constraint
/// 参数位必须是键渲染结果，而非空串。build 产物级断言（生产同形构造）。
#[test]
fn test_param_from_key_overrides_same_name_direct_param_by_merge_order() {
    // Arrange：与 proof_execution.rs 生产发射点同形的构造（占位空串 + 同键真值）
    crate::util::i18n::set_lang_from_string("en".to_string());
    let diagnostic = ErrorCodeDefinition::refinement_violated("")
        .param("func", "Check".to_string())
        .param("args", "[Int(7), Int(0)]".to_string())
        .param_from_key("constraint", "E4018.proof_false.constraint")
        .param_from_key("counterexample", "E4018.proof_false.note")
        .build();

    // Act：取 message（模板 = Refinement predicate violated: {constraint}{counterexample}）

    // Assert：constraint 位是键渲染结果；若并入次序失效（空串胜出），
    // message 将不含该句式
    assert!(
        diagnostic
            .message
            .contains("Proof function 'Check' returned false: constraint not satisfied (args: [Int(7), Int(0)]); check the constraint or change the arguments"),
        "param_from_key 的同键真值必须覆盖快捷方法的占位空串（并入次序不变量）。Got: {}",
        diagnostic.message
    );
}

/// 判据（issue #509 残留⑤护栏）：`render_key` 的键在整条回退链
/// （请求语言 → en → zh）miss 时，help 会静默置空省略——与 message 缺参的
/// debug panic 策略不对称。debug 下必须断言失败，让键拼错/键集漂移在测试期
/// 暴露（release 无断言、返回 None 由调用方兜底，故本测试仅 debug 常驻）。
#[test]
#[cfg(debug_assertions)]
#[should_panic(expected = "在整条回退链")]
fn test_render_key_debug_asserts_when_key_missing_on_whole_chain() {
    // Arrange / Act：请求语言、en、zh 三级都没有的键（拼错形态）
    let en = I18nRegistry::new("en");
    let _ = en.render_key("E9999.chain_missing.key", &[]);

    // Assert：debug_assert! 触发 panic（should_panic 承接）
}

/// 判据（issue #509 残留⑥机械对账）：六个 locale 文件的平键（per-site 机制键）
/// 键集**全等**（#469 复审轮六文件机械比对的常驻化），且四语言
/// （ja/ru/zh-classical/zh-x-miao）的平键值不得与 en 逐字相同——逐字相同即
/// #469 落库时的英文占位（机制通、回退链生效，但文案未译）。文件经
/// include_str! 编译期嵌入（与 src/util/i18n 的 LOCALE_FILES 同一文件源），
/// 不在测试期读文件系统。
#[test]
fn test_per_site_flat_keys_have_no_english_placeholder_left_in_four_langs() {
    // Arrange：与 i18n LOCALE_FILES 同源的六个文件（编译期嵌入）+ 平键提取
    const ZH: &str = include_str!("../../../../../locales/zh.json");
    const EN: &str = include_str!("../../../../../locales/en.json");
    const JA: &str = include_str!("../../../../../locales/ja.json");
    const RU: &str = include_str!("../../../../../locales/ru.json");
    const ZH_CLASSICAL: &str = include_str!("../../../../../locales/zh-classical.json");
    const ZH_X_MIAO: &str = include_str!("../../../../../locales/zh-x-miao.json");

    let flat = |text: &str| -> std::collections::BTreeMap<String, String> {
        let value: serde_json::Value = serde_json::from_str(text).expect("locale JSON 必须可解析");
        let mut out = std::collections::BTreeMap::new();
        if let serde_json::Value::Object(map) = value {
            for (key, val) in map {
                if let serde_json::Value::String(s) = val {
                    if key.contains('.') {
                        out.insert(key, s);
                    }
                }
            }
        }
        out
    };

    let zh = flat(ZH);
    let en = flat(EN);
    let ja = flat(JA);
    let ru = flat(RU);
    let zh_classical = flat(ZH_CLASSICAL);
    let zh_x_miao = flat(ZH_X_MIAO);

    // Assert 1：键集非空且六文件全等（键集漂移 = 机制回退链出现空洞）
    assert!(
        !zh.is_empty(),
        "平键集不得为空——per-site 机制键至少含 #469/#509 的 13 键"
    );
    for (lang, keys) in [
        ("en", en.keys().collect::<Vec<_>>()),
        ("ja", ja.keys().collect::<Vec<_>>()),
        ("ru", ru.keys().collect::<Vec<_>>()),
        ("zh-classical", zh_classical.keys().collect::<Vec<_>>()),
        ("zh-x-miao", zh_x_miao.keys().collect::<Vec<_>>()),
    ] {
        assert_eq!(
            keys,
            zh.keys().collect::<Vec<_>>(),
            "{lang} 的平键集必须与 zh 全等（#509 残留⑥键集对账）"
        );
    }

    // Assert 2：四语言平键值不得与 en 逐字相同（占位残留即此形态）
    for (lang, table) in [
        ("ja", &ja),
        ("ru", &ru),
        ("zh-classical", &zh_classical),
        ("zh-x-miao", &zh_x_miao),
    ] {
        for (key, value) in table {
            assert_ne!(
                value, &en[key],
                "{lang} 的平键 {key} 与 en 逐字相同 = 英文占位残留（issue #509 残留⑥）"
            );
        }
    }
}
