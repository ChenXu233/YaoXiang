//! settings.toml（默认版本、镜像源）

use std::path::Path;

use crate::error::Result;

/// `~/.yaoxiang/settings.toml`
#[derive(Debug, Default, Clone, serde::Serialize, serde::Deserialize)]
pub struct Settings {
    /// 默认工具链版本（`yx toolchain default <ver>` 写入）
    #[serde(skip_serializing_if = "Option::is_none")]
    pub default: Option<String>,

    /// 下载镜像（ghproxy 风格前缀拼接：`<mirror>https://github.com/...`），
    /// 服务 Z3 同款网络受限场景；空/缺省直连 GitHub
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mirror: Option<String>,
}

impl Settings {
    pub fn load(home: &Path) -> Result<Self> {
        Self::load_from(&std::fs::read_to_string(settings_path(home)).unwrap_or_default())
    }

    /// `load` 的纯函数形态（内容可注入，供测试）
    pub fn load_from(content: &str) -> Result<Self> {
        if content.trim().is_empty() {
            return Ok(Self::default());
        }
        Ok(toml::from_str(content)?)
    }

    pub fn save(
        &self,
        home: &Path,
    ) -> Result<()> {
        std::fs::create_dir_all(home)?;
        std::fs::write(settings_path(home), toml::to_string_pretty(self)?)?;
        Ok(())
    }
}

fn settings_path(home: &Path) -> std::path::PathBuf {
    home.join("settings.toml")
}

/// 下载 URL 组装：镜像存在时按 ghproxy 惯例前缀拼接完整 GitHub URL
pub fn download_url(
    mirror: Option<&str>,
    github_url: &str,
) -> String {
    match mirror.map(str::trim).filter(|m| !m.is_empty()) {
        Some(m) => format!("{}/{}", m.trim_end_matches('/'), github_url),
        None => github_url.to_string(),
    }
}

#[cfg(test)]
mod tests;
