//! 平台信息获取
//!
//! 提供编译目标平台检测和平台类型信息
//!
//! # 平台特化设计 (RFC-011)
//!
//! - `P` 是预定义泛型参数名，被解析器占用
//! - `[P: X86_64]` 表示当前平台是 X86_64 时的特化
//! - 编译器自动选择匹配的特化

use std::fmt;
use std::sync::Arc;

/// 支持的目标平台
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum TargetPlatform {
    /// x86_64 架构 (64位 Intel/AMD)
    X86_64,
    /// AArch64 架构 (64位 ARM)
    AArch64,
    /// RISC-V 架构
    RiscV64,
    /// 32位 ARM
    Arm,
    /// 32位 x86
    X86,
    /// WebAssembly
    Wasm32,
    /// 未知/通用平台
    Unknown(String),
}

impl fmt::Display for TargetPlatform {
    fn fmt(
        &self,
        f: &mut fmt::Formatter<'_>,
    ) -> fmt::Result {
        match self {
            TargetPlatform::X86_64 => write!(f, "X86_64"),
            TargetPlatform::AArch64 => write!(f, "AArch64"),
            TargetPlatform::RiscV64 => write!(f, "RISC_V64"),
            TargetPlatform::Arm => write!(f, "ARM"),
            TargetPlatform::X86 => write!(f, "X86"),
            TargetPlatform::Wasm32 => write!(f, "WASM32"),
            TargetPlatform::Unknown(name) => write!(f, "{}", name),
        }
    }
}

impl TargetPlatform {
    /// 获取平台名称字符串
    pub fn as_str(&self) -> &str {
        match self {
            TargetPlatform::X86_64 => "X86_64",
            TargetPlatform::AArch64 => "AArch64",
            TargetPlatform::RiscV64 => "RISC_V64",
            TargetPlatform::Arm => "ARM",
            TargetPlatform::X86 => "X86",
            TargetPlatform::Wasm32 => "WASM32",
            TargetPlatform::Unknown(name) => name.as_str(),
        }
    }

    /// 检查是否是 64位平台
    pub fn is_64bit(&self) -> bool {
        matches!(
            self,
            TargetPlatform::X86_64 | TargetPlatform::AArch64 | TargetPlatform::RiscV64
        )
    }

    /// 检查是否是 32位平台
    pub fn is_32bit(&self) -> bool {
        matches!(
            self,
            TargetPlatform::Arm | TargetPlatform::X86 | TargetPlatform::Wasm32
        )
    }

    /// 检查是否是 ARM 架构
    pub fn is_arm(&self) -> bool {
        matches!(self, TargetPlatform::Arm | TargetPlatform::AArch64)
    }

    /// 检查是否是 x86 架构
    pub fn is_x86(&self) -> bool {
        matches!(self, TargetPlatform::X86 | TargetPlatform::X86_64)
    }
}

/// 平台信息
///
/// 存储当前编译目标平台的所有相关信息
#[derive(Debug, Clone)]
pub struct PlatformInfo {
    /// 目标平台
    target: TargetPlatform,

    /// 目标平台的三元组描述 (如 "x86_64-unknown-linux-gnu")
    target_triple: String,

    /// 供应商字符串
    vendor: String,

    /// 操作系统
    os: String,

    /// 运行环境
    environment: String,

    /// CPU 特性标志
    cpu_features: Vec<String>,
}

impl PlatformInfo {
    /// 创建指定平台的 PlatformInfo
    pub fn new(
        target: TargetPlatform,
        target_triple: String,
    ) -> Self {
        let (vendor, os, environment) = Self::parse_triple(&target_triple);

        PlatformInfo {
            target,
            target_triple,
            vendor,
            os,
            environment,
            cpu_features: Vec::new(),
        }
    }

    /// 创建未知平台的 PlatformInfo
    pub fn unknown(name: String) -> Self {
        PlatformInfo {
            target: TargetPlatform::Unknown(name.clone()),
            target_triple: name.clone(),
            vendor: "unknown".to_string(),
            os: "unknown".to_string(),
            environment: "unknown".to_string(),
            cpu_features: Vec::new(),
        }
    }

    /// 解析目标三元组
    fn parse_triple(triple: &str) -> (String, String, String) {
        let parts: Vec<&str> = triple.split('-').collect();
        let vendor = parts.get(1).unwrap_or(&"unknown").to_string();
        let os = parts.get(2).unwrap_or(&"unknown").to_string();
        let environment = parts.get(3).unwrap_or(&"").to_string();
        (vendor, os, environment)
    }

    /// 获取目标平台
    pub fn target(&self) -> &TargetPlatform {
        &self.target
    }

    /// 获取目标三元组
    pub fn target_triple(&self) -> &str {
        &self.target_triple
    }

    /// 获取供应商
    pub fn vendor(&self) -> &str {
        &self.vendor
    }

    /// 获取操作系统
    pub fn os(&self) -> &str {
        &self.os
    }

    /// 获取运行环境
    pub fn environment(&self) -> &str {
        &self.environment
    }

    /// 检查是否支持特定 CPU 特性
    pub fn has_cpu_feature(
        &self,
        feature: &str,
    ) -> bool {
        self.cpu_features.iter().any(|f| f == feature)
    }

    /// 添加 CPU 特性标志
    pub fn add_cpu_feature(
        &mut self,
        feature: String,
    ) {
        if !self.cpu_features.contains(&feature) {
            self.cpu_features.push(feature);
        }
    }

    /// 获取平台类型名称 (用于类型系统)
    ///
    /// 返回平台类型的字符串表示，如 "X86_64"
    pub fn platform_type_name(&self) -> String {
        self.target.as_str().to_string()
    }
}

/// 平台检测器
///
/// 用于从编译参数和环境检测目标平台
#[derive(Debug, Clone, Default)]
pub struct PlatformDetector;

impl PlatformDetector {
    /// 创建新的平台检测器
    pub fn new() -> Self {
        PlatformDetector
    }

    /// 从编译参数检测目标平台
    ///
    /// # Arguments
    ///
    /// * `target` - 目标三元组字符串（如 "x86_64-unknown-linux-gnu"）
    ///
    /// # Returns
    ///
    /// 检测到的平台信息
    pub fn detect_from_target(target: &str) -> PlatformInfo {
        let platform = Self::parse_target(target);
        PlatformInfo::new(platform, target.to_string())
    }

    /// 解析目标三元组为平台枚举
    fn parse_target(target: &str) -> TargetPlatform {
        let lower = target.to_lowercase();

        if lower.contains("x86_64") {
            TargetPlatform::X86_64
        } else if lower.contains("aarch64") || lower.contains("arm64") {
            TargetPlatform::AArch64
        } else if lower.contains("riscv64") || lower.contains("riscv") {
            TargetPlatform::RiscV64
        } else if lower.contains("arm") {
            if lower.contains("64") || lower.contains("aarch") {
                TargetPlatform::AArch64
            } else {
                TargetPlatform::Arm
            }
        } else if lower.contains("i386")
            || lower.contains("i486")
            || lower.contains("i586")
            || lower.contains("i686")
            || lower.contains("x86")
        {
            if lower.contains("64") {
                TargetPlatform::X86_64
            } else {
                TargetPlatform::X86
            }
        } else if lower.contains("wasm") || lower.contains("webassembly") {
            TargetPlatform::Wasm32
        } else {
            TargetPlatform::Unknown(target.to_string())
        }
    }

    /// 检测当前编译环境（从环境变量）
    ///
    /// 优先使用 CARGO_TARGET 环境变量，其次使用标准环境变量
    pub fn detect_from_env() -> PlatformInfo {
        // 尝试从 CARGO_TARGET 环境变量获取
        if let Ok(target) = std::env::var("CARGO_TARGET") {
            return Self::detect_from_target(&target);
        }

        // 尝试从标准工具链变量获取
        if let Ok(target) = std::env::var("RUST_TARGET") {
            return Self::detect_from_target(&target);
        }

        // 尝试从 LLVM_TARGET 获取（clang/llvm 使用）
        if let Ok(target) = std::env::var("LLVM_TARGET") {
            return Self::detect_from_target(&target);
        }

        // 回退到未知平台
        PlatformInfo::unknown("unknown".to_string())
    }

    /// 规范化目标三元组
    ///
    /// 将不同格式的目标三元组规范化为标准格式
    pub fn normalize_target(target: &str) -> String {
        let platform = Self::parse_target(target);
        platform.as_str().to_string()
    }
}

/// 平台配置
///
/// 编译器的平台相关配置
#[derive(Debug, Clone)]
pub struct PlatformConfig {
    /// 目标平台信息
    platform_info: Arc<PlatformInfo>,

    /// 是否启用平台特化
    enabled: bool,
}

impl PlatformConfig {
    /// 创建新的平台配置
    pub fn new(platform_info: PlatformInfo) -> Self {
        PlatformConfig {
            platform_info: Arc::new(platform_info),
            enabled: true,
        }
    }

    /// 获取平台信息（不可变引用）
    pub fn platform_info(&self) -> &PlatformInfo {
        &self.platform_info
    }

    /// 获取平台信息（克隆 Arc）
    pub fn platform_info_arc(&self) -> Arc<PlatformInfo> {
        self.platform_info.clone()
    }

    /// 设置是否启用平台特化
    pub fn set_enabled(
        &mut self,
        enabled: bool,
    ) {
        self.enabled = enabled;
    }

    /// 检查是否启用平台特化
    pub fn is_enabled(&self) -> bool {
        self.enabled
    }
}

impl Default for PlatformConfig {
    fn default() -> Self {
        Self::new(PlatformDetector::detect_from_env())
    }
}

// ==================== 预定义泛型参数 P ====================

/// 预定义泛型参数名
///
/// RFC-011 规定 `P` 是预定义泛型参数名，被解析器占用
/// 代表当前编译平台，用于平台特化
pub const PLATFORM_PARAM_NAME: &str = "P";

/// 检查泛型参数名是否是预定义的平台参数
pub fn is_platform_param(name: &str) -> bool {
    name == PLATFORM_PARAM_NAME
}

/// 预定义平台参数列表
pub fn platform_param_names() -> &'static [&'static str] {
    &[PLATFORM_PARAM_NAME]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_x86_64() {
        let target = "x86_64-unknown-linux-gnu";
        let info = PlatformDetector::detect_from_target(target);
        assert_eq!(info.target(), &TargetPlatform::X86_64);
        assert_eq!(info.vendor(), "unknown");
        assert_eq!(info.os(), "linux");
        assert_eq!(info.environment(), "gnu");
    }

    #[test]
    fn test_parse_aarch64() {
        let target = "aarch64-apple-darwin";
        let info = PlatformDetector::detect_from_target(target);
        assert_eq!(info.target(), &TargetPlatform::AArch64);
        assert_eq!(info.vendor(), "apple");
    }

    #[test]
    fn test_platform_display() {
        assert_eq!(TargetPlatform::X86_64.to_string(), "X86_64");
        assert_eq!(TargetPlatform::AArch64.to_string(), "AArch64");
    }

    #[test]
    fn test_is_platform_param() {
        assert!(is_platform_param("P"));
        assert!(!is_platform_param("T"));
        assert!(!is_platform_param("X86_64"));
    }
}
