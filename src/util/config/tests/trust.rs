//! 构建信任记录持久化测试 — 基于 RFC-014b 2026-09-15 决议 1
//!
//! `--trust` / 交互确认把信任写入用户配置 `[trust] build-scripts`
//! （`name@version` 键）；持久化是幂等的。

use crate::util::config::{self, UserConfig};

#[test]
fn test_trust_store_round_trip() {
    // Arrange：空配置落盘
    let dir = tempfile::tempdir().expect("create tempdir");
    let store = dir.path().join("config.toml");
    config::save_user_config_to(&store, &UserConfig::default()).expect("save empty config");

    // Act：写入信任记录后再读
    let mut cfg = config::load_user_config_from(&store).expect("load config");
    assert!(cfg.trust.build_scripts.is_empty(), "新配置的信任列表应为空");
    cfg.trust.build_scripts.push("a@1.0.0".to_string());
    config::save_user_config_to(&store, &cfg).expect("save config with trust");

    // Assert：记录持久化成功
    let cfg = config::load_user_config_from(&store).expect("reload config");
    assert_eq!(
        cfg.trust.build_scripts,
        vec!["a@1.0.0".to_string()],
        "信任记录应跨保存/加载保留"
    );
}
