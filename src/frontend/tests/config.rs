//! 配置模块测试
//!
//! 测试编译配置相关功能，包括：
//! - 死代码分析配置
//! - 编译配置构建器方法
//! - 配置默认值验证

use crate::frontend::config::{CompileConfig, DeadCodeConfig};

#[test]
fn test_dead_code_config_default() {
    let config = DeadCodeConfig::default();
    assert!(
        config.enabled,
        "Dead code analysis should be enabled by default"
    );
}

#[test]
fn test_dead_code_config_disabled() {
    let config = DeadCodeConfig { enabled: false };
    assert!(!config.enabled, "Dead code analysis can be disabled");
}

#[test]
fn test_compile_config_with_dead_code_disabled() {
    let config = CompileConfig::new().with_dead_code_enabled(false);
    assert!(!config.dead_code.enabled);
}

#[test]
fn test_compile_config_with_dead_code_enabled() {
    let config = CompileConfig::new().with_dead_code_enabled(true);
    assert!(config.dead_code.enabled);
}
