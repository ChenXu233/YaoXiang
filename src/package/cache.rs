//! 全局包缓存（RFC-014 Phase 3）
//!
//! 所有下载的依赖先落到 `~/.yaoxiang/cache/`，项目 vendor 目录从缓存复制。
//! 缓存条目按不可变键组织（tag / rev / commit），同一键永不重下载；
//! 分支引用经 `git ls-remote` 解析为 commit 后入缓存，指针文件支持离线回退。
//!
//! ```text
//! ~/.yaoxiang/cache/
//! ├── git/
//! │   ├── github.com-user-bar-v1.2.3/      # tag/commit 键条目
//! │   └── github.com-user-bar@HEAD.rev     # 分支→commit 指针（离线回退）
//! ├── registry/                            # RFC-014a 落地后启用
//! └── binaries/                            # RFC-014b 落地后启用
//! ```
//!
//! 缓存根目录可经用户配置 `[cache] dir` 覆盖（RFC 中的 `~/.yaoxiang/config.toml`
//! 并入既有 `~/.config/yaoxiang/config.toml` 配置体系，缓存数据位置不变）。

use std::path::{Path, PathBuf};

use crate::package::error::{PackageError, PackageResult};

/// 默认缓存根目录相对用户主目录的路径
const DEFAULT_CACHE_SUBDIR: &str = ".yaoxiang/cache";

/// 全局包缓存
#[derive(Debug, Clone)]
pub struct GlobalCache {
    root: PathBuf,
}

impl GlobalCache {
    /// 从用户配置构建缓存（`[cache] dir` 覆盖默认位置）
    pub fn from_config() -> PackageResult<Self> {
        let configured = crate::util::config::load_user_config()
            .ok()
            .and_then(|c| c.cache.dir)
            .map(|d| expand_home(&d));
        let root = match configured {
            Some(p) => p,
            None => Self::default_root()?,
        };
        Ok(GlobalCache { root })
    }

    /// 以指定根目录构建缓存（测试与程序化使用）
    pub fn with_root(root: PathBuf) -> Self {
        GlobalCache { root }
    }

    /// 缓存根目录
    pub fn root(&self) -> &Path {
        &self.root
    }

    /// git 条目目录（按需创建）
    pub fn git_dir(&self) -> PathBuf {
        let dir = self.root.join("git");
        let _ = std::fs::create_dir_all(&dir);
        dir
    }

    /// git 依赖的缓存条目路径
    ///
    /// 格式: `<cache>/git/<sanitized-url>-<sanitized-key>`
    pub fn git_entry(
        &self,
        url: &str,
        key: &str,
    ) -> PathBuf {
        self.git_dir()
            .join(format!("{}-{}", sanitize_key(url), sanitize_key(key)))
    }

    /// 分支 → commit 指针文件路径（离线回退用）
    ///
    /// 格式: `<cache>/git/<sanitized-url>@<sanitized-ref>.rev`
    pub fn pointer_file(
        &self,
        url: &str,
        ref_name: &str,
    ) -> PathBuf {
        self.git_dir().join(format!(
            "{}@{}.rev",
            sanitize_key(url),
            sanitize_key(ref_name)
        ))
    }

    /// 写入分支指针
    pub fn write_pointer(
        &self,
        url: &str,
        ref_name: &str,
        commit: &str,
    ) -> PackageResult<()> {
        let path = self.pointer_file(url, ref_name);
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        std::fs::write(path, commit)?;
        Ok(())
    }

    /// 读取分支指针；不存在返回 None
    pub fn read_pointer(
        &self,
        url: &str,
        ref_name: &str,
    ) -> PackageResult<Option<String>> {
        let path = self.pointer_file(url, ref_name);
        if !path.exists() {
            return Ok(None);
        }
        Ok(Some(std::fs::read_to_string(path)?.trim().to_string()))
    }

    /// 把缓存条目复制到目标目录（跳过 `.git`，先清空既有目标）
    pub fn copy_into(
        &self,
        entry: &Path,
        dest: &Path,
    ) -> PackageResult<()> {
        if dest.exists() {
            std::fs::remove_dir_all(dest)?;
        }
        if let Some(parent) = dest.parent() {
            std::fs::create_dir_all(parent)?;
        }
        std::fs::create_dir_all(dest)?;
        copy_dir_recursive(entry, dest)
    }

    /// 清空缓存，返回释放的字节数；缓存不存在时返回 None
    pub fn clean(&self) -> PackageResult<Option<u64>> {
        if !self.root.exists() {
            return Ok(None);
        }
        let freed = Self::dir_size(&self.root);
        std::fs::remove_dir_all(&self.root)?;
        Ok(Some(freed))
    }

    /// 递归统计目录字节数
    pub fn dir_size(path: &Path) -> u64 {
        let mut total = 0u64;
        if let Ok(entries) = std::fs::read_dir(path) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.is_dir() {
                    total += Self::dir_size(&path);
                } else if let Ok(meta) = entry.metadata() {
                    total += meta.len();
                }
            }
        }
        total
    }

    /// 字节数的人类可读形式
    pub fn format_size(bytes: u64) -> String {
        const KB: f64 = 1024.0;
        const MB: f64 = KB * 1024.0;
        const GB: f64 = MB * 1024.0;
        let b = bytes as f64;
        if b >= GB {
            format!("{:.2} GB", b / GB)
        } else if b >= MB {
            format!("{:.2} MB", b / MB)
        } else if b >= KB {
            format!("{:.2} KB", b / KB)
        } else {
            format!("{} B", bytes)
        }
    }

    fn default_root() -> PackageResult<PathBuf> {
        if let Ok(home) = std::env::var("HOME") {
            return Ok(PathBuf::from(home).join(DEFAULT_CACHE_SUBDIR));
        }
        if let Ok(appdata) = std::env::var("APPDATA") {
            return Ok(PathBuf::from(appdata)
                .join("yaoxiang")
                .join(DEFAULT_CACHE_SUBDIR));
        }
        Err(PackageError::Cache(
            "无法定位用户主目录（HOME / APPDATA 均未设置）".to_string(),
        ))
    }
}

/// URL / 引用 → 缓存目录名安全片段
///
/// 去掉 scheme 与 `git@` 前缀、结尾 `.git`；非 `[A-Za-z0-9._-]` 字符折叠为 `-`。
pub(crate) fn sanitize_key(s: &str) -> String {
    let mut s = s.trim().to_string();
    for prefix in ["https://", "http://", "git://", "ssh://", "git@"] {
        if let Some(rest) = s.strip_prefix(prefix) {
            s = rest.to_string();
            break;
        }
    }
    if let Some(rest) = s.strip_suffix(".git") {
        s = rest.to_string();
    }

    let mut out = String::with_capacity(s.len());
    let mut last_dash = false;
    for c in s.chars() {
        if c.is_ascii_alphanumeric() || matches!(c, '.' | '_' | '-') {
            out.push(c);
            last_dash = false;
        } else if !last_dash {
            out.push('-');
            last_dash = true;
        }
    }
    out.trim_matches('-').to_string()
}

/// `~` 前缀展开到用户主目录
fn expand_home(s: &str) -> PathBuf {
    if let Ok(home) = std::env::var("HOME") {
        if s == "~" {
            return PathBuf::from(home);
        }
        if let Some(rest) = s.strip_prefix("~/").or_else(|| s.strip_prefix("~\\")) {
            return PathBuf::from(home).join(rest);
        }
    }
    PathBuf::from(s)
}

/// 递归复制目录，跳过 `.git`
fn copy_dir_recursive(
    src: &Path,
    dst: &Path,
) -> PackageResult<()> {
    for entry in std::fs::read_dir(src)? {
        let entry = entry?;
        let file_name = entry.file_name();
        if file_name == ".git" {
            continue;
        }
        let from = entry.path();
        let to = dst.join(&file_name);
        if from.is_dir() {
            std::fs::create_dir_all(&to)?;
            copy_dir_recursive(&from, &to)?;
        } else {
            std::fs::copy(&from, &to)?;
        }
    }
    Ok(())
}
