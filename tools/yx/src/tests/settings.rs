//! settings.toml 读写与镜像 URL 组装测试（RFC-037 §安装器支持）

use tempfile::TempDir;

use crate::settings::{download_url, Settings};

#[test]
fn test_settings_missing_file_yields_defaults() {
    // Act
    let settings = Settings::load_from("").unwrap();

    // Assert
    assert!(
        settings.default.is_none(),
        "empty settings must have no default version"
    );
    assert!(
        settings.mirror.is_none(),
        "empty settings must have no mirror"
    );
}

#[test]
fn test_settings_roundtrip_preserves_default_and_mirror() {
    // Arrange
    let dir = TempDir::new().unwrap();
    let settings = Settings {
        default: Some("0.7.14".to_string()),
        mirror: Some("https://ghproxy.example.com".to_string()),
    };

    // Act
    settings.save(dir.path()).unwrap();
    let loaded = Settings::load(dir.path()).unwrap();

    // Assert
    assert_eq!(
        loaded.default.as_deref(),
        Some("0.7.14"),
        "default version must survive save/load"
    );
    assert_eq!(
        loaded.mirror.as_deref(),
        Some("https://ghproxy.example.com"),
        "mirror must survive save/load"
    );
}

#[test]
fn test_download_url_prepends_mirror_ghproxy_style() {
    // Act & Assert: 镜像按 ghproxy 惯例前缀拼接完整 GitHub URL
    assert_eq!(
        download_url(
            Some("https://ghproxy.example.com"),
            "https://github.com/ChenXu233/YaoXiang/releases/download/v0.7.14/a.tar.gz"
        ),
        "https://ghproxy.example.com/https://github.com/ChenXu233/YaoXiang/releases/download/v0.7.14/a.tar.gz",
        "mirror must be prepended to the full github url"
    );
}

#[test]
fn test_download_url_passthrough_without_mirror() {
    // Act & Assert
    assert_eq!(
        download_url(None, "https://github.com/a/b"),
        "https://github.com/a/b",
        "no mirror must yield the original url"
    );
    assert_eq!(
        download_url(Some(""), "https://github.com/a/b"),
        "https://github.com/a/b",
        "empty mirror must yield the original url"
    );
}
