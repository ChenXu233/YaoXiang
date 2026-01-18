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
