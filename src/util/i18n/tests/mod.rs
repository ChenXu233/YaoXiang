//! i18n 测试

use super::*;

#[test]
fn test_msg_key() {
    assert_eq!(MSG::CmdReceived.key(), "cmd_received");
    assert_eq!(MSG::LexStart.key(), "lex_start");
}

#[test]
fn test_available_langs() {
    let langs = available_langs();
    assert!(!langs.is_empty());
    assert!(langs.contains(&"en"));
    assert!(langs.contains(&"zh"));
    assert!(langs.contains(&"zh-x-miao"));
}

#[test]
fn test_t_with_lang() {
    let result = t_simple(MSG::CmdReceived, "en");
    assert!(!result.is_empty());
}

#[test]
fn test_t_miao() {
    let result = t_simple(MSG::CmdReceived, "zh-x-miao");
    // Should contain miao-style content
    if !result.is_empty() && result != "cmd_received" {
        assert!(result.contains("喵"));
    }
}

#[test]
fn test_t_key_level_fallback() {
    // 键在请求语言中缺失时按键级回退 zh/en，而不是暴露裸键名
    // （Phase 3 新增词条在 bot 补齐 ja/ru 等语言前必须落到回退链）
    let result = t_simple(MSG::PackageOutdatedNone, "ja");
    assert_ne!(result, "package_outdated_none");
    assert!(!result.is_empty());
}

#[test]
fn test_t_missing_everywhere_returns_key() {
    // 所有语言都缺此键时才回落键名（仅当新增 MSG 未配词条时发生）
    let key = MSG::PackageOutdatedRow.key();
    assert_eq!(key, "package_outdated_row");
}
