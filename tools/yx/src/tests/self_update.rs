//! self update 写权限预检测试（RFC-037：安装器渠道（{autopf}）安装的 yx
//! 所在目录不可写，须提前给出可行动错误而非下载后 io error）

use tempfile::TempDir;

use crate::self_update::ensure_dir_writable;

#[test]
fn test_ensure_dir_writable_accepts_writable_dir() {
    // Arrange: 可写的临时目录
    let dir = TempDir::new().unwrap();

    // Act
    let result = ensure_dir_writable(dir.path());

    // Assert: 探针成功且已清理，不留残留文件
    assert!(
        result.is_ok(),
        "writable dir must pass the probe, got {result:?}"
    );
    assert!(
        !dir.path().join(".yx-write-probe").exists(),
        "probe file must be removed after a successful probe"
    );
}

#[test]
fn test_ensure_dir_writable_rejects_missing_dir() {
    // Arrange: 不存在的目录（展开即失败，等价于不可写）
    let dir = TempDir::new().unwrap();
    let missing = dir.path().join("no-such-bin");

    // Act
    let result = ensure_dir_writable(&missing);

    // Assert: 报错须指向目录并给出安装器渠道的可行动指引
    assert!(
        matches!(result, Err(crate::error::Error::Message(ref m))
            if m.contains("cannot write to") && m.contains("installer")),
        "missing dir must fail with actionable message, got {result:?}"
    );
}
