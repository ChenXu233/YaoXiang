//! 语义化版本解析器
//!
//! 支持以下版本要求格式:
//! - `^1.0.0` — 兼容版本 (>=1.0.0, <2.0.0)
//! - `~1.0.0` — 补丁版本 (>=1.0.0, <1.1.0)
//! - `1.0.0`  — 精确版本
//! - `*`      — 任意版本
//! - `>=1.0.0` — 大于等于
//! - `>1.0.0`  — 大于
//! - `<=1.0.0` — 小于等于
//! - `<1.0.0`  — 小于

use std::cmp::Ordering;
use std::fmt;

use crate::package::error::{PackageError, PackageResult};

/// 语义化版本号
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SemVer {
    /// 主版本号
    pub major: u64,
    /// 次版本号
    pub minor: u64,
    /// 补丁版本号
    pub patch: u64,
    /// 预发布标签 (如 alpha, beta, rc.1)
    pub pre: Option<String>,
}

impl SemVer {
    /// 创建新的版本号
    pub fn new(
        major: u64,
        minor: u64,
        patch: u64,
    ) -> Self {
        SemVer {
            major,
            minor,
            patch,
            pre: None,
        }
    }

    /// 创建带预发布标签的版本号
    pub fn with_pre(
        major: u64,
        minor: u64,
        patch: u64,
        pre: &str,
    ) -> Self {
        SemVer {
            major,
            minor,
            patch,
            pre: Some(pre.to_string()),
        }
    }

    /// 解析版本字符串
    ///
    /// 支持格式: `1.2.3`, `1.2.3-alpha`, `1.2`, `1`
    pub fn parse(s: &str) -> PackageResult<Self> {
        let s = s.trim();

        // 分离预发布标签
        let (version_part, pre) = if let Some(idx) = s.find('-') {
            (&s[..idx], Some(s[idx + 1..].to_string()))
        } else {
            (s, None)
        };

        let parts: Vec<&str> = version_part.split('.').collect();
        if parts.is_empty() || parts.len() > 3 {
            return Err(PackageError::InvalidManifest(format!(
                "无效的版本号: '{}'",
                s
            )));
        }

        let major = parts[0].parse::<u64>().map_err(|_| {
            PackageError::InvalidManifest(format!("无效的主版本号: '{}'", parts[0]))
        })?;

        let minor = if parts.len() > 1 {
            parts[1].parse::<u64>().map_err(|_| {
                PackageError::InvalidManifest(format!("无效的次版本号: '{}'", parts[1]))
            })?
        } else {
            0
        };

        let patch = if parts.len() > 2 {
            parts[2].parse::<u64>().map_err(|_| {
                PackageError::InvalidManifest(format!("无效的补丁版本号: '{}'", parts[2]))
            })?
        } else {
            0
        };

        Ok(SemVer {
            major,
            minor,
            patch,
            pre,
        })
    }
}

impl Ord for SemVer {
    fn cmp(
        &self,
        other: &Self,
    ) -> Ordering {
        match self.major.cmp(&other.major) {
            Ordering::Equal => {}
            ord => return ord,
        }
        match self.minor.cmp(&other.minor) {
            Ordering::Equal => {}
            ord => return ord,
        }
        match self.patch.cmp(&other.patch) {
            Ordering::Equal => {}
            ord => return ord,
        }
        // 预发布版本比正式版本低
        match (&self.pre, &other.pre) {
            (None, None) => Ordering::Equal,
            (None, Some(_)) => Ordering::Greater,
            (Some(_), None) => Ordering::Less,
            (Some(a), Some(b)) => a.cmp(b),
        }
    }
}

impl PartialOrd for SemVer {
    fn partial_cmp(
        &self,
        other: &Self,
    ) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl fmt::Display for SemVer {
    fn fmt(
        &self,
        f: &mut fmt::Formatter<'_>,
    ) -> fmt::Result {
        write!(f, "{}.{}.{}", self.major, self.minor, self.patch)?;
        if let Some(ref pre) = self.pre {
            write!(f, "-{}", pre)?;
        }
        Ok(())
    }
}

/// 版本比较运算符
#[derive(Debug, Clone, PartialEq)]
enum VersionOp {
    /// 精确匹配
    Exact,
    /// 大于等于
    Gte,
    /// 大于
    Gt,
    /// 小于等于
    Lte,
    /// 小于
    Lt,
}

/// 单个版本约束
#[derive(Debug, Clone)]
struct VersionConstraint {
    op: VersionOp,
    version: SemVer,
}

impl VersionConstraint {
    fn matches(
        &self,
        version: &SemVer,
    ) -> bool {
        match self.op {
            VersionOp::Exact => version == &self.version,
            VersionOp::Gte => version >= &self.version,
            VersionOp::Gt => version > &self.version,
            VersionOp::Lte => version <= &self.version,
            VersionOp::Lt => version < &self.version,
        }
    }
}

/// 版本要求
///
/// 表示一组版本约束的组合，所有约束必须同时满足。
#[derive(Debug, Clone)]
pub struct VersionReq {
    constraints: Vec<VersionConstraint>,
    /// 是否匹配任意版本
    any: bool,
}

impl VersionReq {
    /// 解析版本要求字符串
    ///
    /// # 支持的格式
    /// - `*` → 任意版本
    /// - `1.0.0` → 精确版本
    /// - `^1.0.0` → 兼容版本 (>=1.0.0, <2.0.0)
    /// - `~1.0.0` → 补丁版本 (>=1.0.0, <1.1.0)
    /// - `>=1.0.0` → 大于等于
    /// - `>1.0.0` → 大于
    /// - `<=1.0.0` → 小于等于
    /// - `<1.0.0` → 小于
    pub fn parse(s: &str) -> PackageResult<Self> {
        let s = s.trim();

        if s == "*" || s.is_empty() {
            return Ok(VersionReq {
                constraints: Vec::new(),
                any: true,
            });
        }

        // 处理逗号分隔的多个约束
        if s.contains(',') {
            let mut constraints = Vec::new();
            for part in s.split(',') {
                let part_req = Self::parse_single(part.trim())?;
                constraints.extend(part_req.constraints);
            }
            return Ok(VersionReq {
                constraints,
                any: false,
            });
        }

        Self::parse_single(s)
    }

    fn parse_single(s: &str) -> PackageResult<Self> {
        let s = s.trim();

        if let Some(stripped) = s.strip_prefix('^') {
            // 兼容版本: ^1.2.3 → >=1.2.3, <2.0.0
            let version = SemVer::parse(stripped)?;
            let upper = if version.major > 0 {
                SemVer::new(version.major + 1, 0, 0)
            } else if version.minor > 0 {
                SemVer::new(0, version.minor + 1, 0)
            } else {
                SemVer::new(0, 0, version.patch + 1)
            };

            Ok(VersionReq {
                constraints: vec![
                    VersionConstraint {
                        op: VersionOp::Gte,
                        version,
                    },
                    VersionConstraint {
                        op: VersionOp::Lt,
                        version: upper,
                    },
                ],
                any: false,
            })
        } else if let Some(stripped) = s.strip_prefix('~') {
            // 补丁版本: ~1.2.3 → >=1.2.3, <1.3.0
            let version = SemVer::parse(stripped)?;
            let upper = SemVer::new(version.major, version.minor + 1, 0);

            Ok(VersionReq {
                constraints: vec![
                    VersionConstraint {
                        op: VersionOp::Gte,
                        version,
                    },
                    VersionConstraint {
                        op: VersionOp::Lt,
                        version: upper,
                    },
                ],
                any: false,
            })
        } else if let Some(stripped) = s.strip_prefix(">=") {
            let version = SemVer::parse(stripped)?;
            Ok(VersionReq {
                constraints: vec![VersionConstraint {
                    op: VersionOp::Gte,
                    version,
                }],
                any: false,
            })
        } else if let Some(stripped) = s.strip_prefix('>') {
            let version = SemVer::parse(stripped)?;
            Ok(VersionReq {
                constraints: vec![VersionConstraint {
                    op: VersionOp::Gt,
                    version,
                }],
                any: false,
            })
        } else if let Some(stripped) = s.strip_prefix("<=") {
            let version = SemVer::parse(stripped)?;
            Ok(VersionReq {
                constraints: vec![VersionConstraint {
                    op: VersionOp::Lte,
                    version,
                }],
                any: false,
            })
        } else if let Some(stripped) = s.strip_prefix('<') {
            let version = SemVer::parse(stripped)?;
            Ok(VersionReq {
                constraints: vec![VersionConstraint {
                    op: VersionOp::Lt,
                    version,
                }],
                any: false,
            })
        } else {
            // 精确版本
            let version = SemVer::parse(s)?;
            Ok(VersionReq {
                constraints: vec![VersionConstraint {
                    op: VersionOp::Exact,
                    version,
                }],
                any: false,
            })
        }
    }

    /// 检查版本是否满足要求
    pub fn matches(
        &self,
        version: &SemVer,
    ) -> bool {
        if self.any {
            return true;
        }
        self.constraints.iter().all(|c| c.matches(version))
    }

    /// 从候选版本列表中选择最佳匹配
    ///
    /// 返回满足要求的最高版本。
    pub fn select_best<'a>(
        &self,
        versions: &'a [SemVer],
    ) -> Option<&'a SemVer> {
        let mut candidates: Vec<&SemVer> = versions.iter().filter(|v| self.matches(v)).collect();
        candidates.sort_by(|a, b| b.cmp(a));
        candidates.into_iter().next()
    }

    /// 检查两个版本要求是否兼容（是否存在共同满足的版本范围）
    pub fn is_compatible(
        &self,
        other: &VersionReq,
    ) -> bool {
        if self.any || other.any {
            return true;
        }

        // 简单的兼容性检查：尝试一些常见版本
        // 更完整的实现需要区间交集分析
        for major in 0..100 {
            for minor in 0..50 {
                for patch in 0..20 {
                    let v = SemVer::new(major, minor, patch);
                    if self.matches(&v) && other.matches(&v) {
                        return true;
                    }
                }
            }
        }
        false
    }
}

impl fmt::Display for VersionReq {
    fn fmt(
        &self,
        f: &mut fmt::Formatter<'_>,
    ) -> fmt::Result {
        if self.any {
            return write!(f, "*");
        }

        let parts: Vec<String> = self
            .constraints
            .iter()
            .map(|c| {
                let op = match c.op {
                    VersionOp::Exact => "=",
                    VersionOp::Gte => ">=",
                    VersionOp::Gt => ">",
                    VersionOp::Lte => "<=",
                    VersionOp::Lt => "<",
                };
                format!("{}{}", op, c.version)
            })
            .collect();

        write!(f, "{}", parts.join(", "))
    }
}
