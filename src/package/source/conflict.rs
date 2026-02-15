//! 依赖冲突检测
//!
//! 检测项目依赖中的版本冲突。

use std::collections::BTreeMap;

use crate::package::dependency::DependencySpec;
use crate::package::error::{PackageError, PackageResult};
use crate::package::source::resolver::VersionReq;

/// 冲突信息
#[derive(Debug, Clone)]
pub struct ConflictInfo {
    /// 包名
    pub package_name: String,
    /// 冲突的版本要求列表（来自不同依赖者）
    pub requirements: Vec<ConflictRequirement>,
}

/// 单个冲突要求
#[derive(Debug, Clone)]
pub struct ConflictRequirement {
    /// 来源（哪个依赖者需要这个版本）
    pub from: String,
    /// 版本要求字符串
    pub version_req: String,
}

impl std::fmt::Display for ConflictInfo {
    fn fmt(
        &self,
        f: &mut std::fmt::Formatter<'_>,
    ) -> std::fmt::Result {
        writeln!(f, "包 '{}' 存在版本冲突:", self.package_name)?;
        for req in &self.requirements {
            writeln!(f, "  {} 要求: {}", req.from, req.version_req)?;
        }
        Ok(())
    }
}

/// 检测依赖列表中的版本冲突
///
/// 检查是否存在同一个包被要求不兼容版本的情况。
pub fn detect_conflicts(
    deps: &[DependencySpec],
    _dev_deps: &[DependencySpec],
) -> PackageResult<Vec<ConflictInfo>> {
    let mut version_reqs: BTreeMap<String, Vec<(String, VersionReq)>> = BTreeMap::new();

    // 收集所有版本要求
    for spec in deps.iter() {
        let req = VersionReq::parse(&spec.version)?;
        version_reqs
            .entry(spec.name.clone())
            .or_default()
            .push(("dependencies".to_string(), req));
    }

    // dev-dependencies 中的要求
    for spec in _dev_deps.iter() {
        let req = VersionReq::parse(&spec.version)?;
        version_reqs
            .entry(spec.name.clone())
            .or_default()
            .push(("dev-dependencies".to_string(), req));
    }

    let mut conflicts = Vec::new();

    // 检查每个包的版本要求是否兼容
    for (name, reqs) in &version_reqs {
        if reqs.len() <= 1 {
            continue;
        }

        // 两两检查兼容性
        let mut has_conflict = false;
        for i in 0..reqs.len() {
            for j in (i + 1)..reqs.len() {
                if !reqs[i].1.is_compatible(&reqs[j].1) {
                    has_conflict = true;
                    break;
                }
            }
            if has_conflict {
                break;
            }
        }

        if has_conflict {
            let requirements = reqs
                .iter()
                .map(|(from, req)| ConflictRequirement {
                    from: from.clone(),
                    version_req: req.to_string(),
                })
                .collect();

            conflicts.push(ConflictInfo {
                package_name: name.clone(),
                requirements,
            });
        }
    }

    Ok(conflicts)
}

/// 检测冲突并返回错误（如果有冲突）
pub fn check_conflicts(
    deps: &[DependencySpec],
    dev_deps: &[DependencySpec],
) -> PackageResult<()> {
    let conflicts = detect_conflicts(deps, dev_deps)?;

    if conflicts.is_empty() {
        return Ok(());
    }

    let messages: Vec<String> = conflicts.iter().map(|c| c.to_string()).collect();

    Err(PackageError::InvalidManifest(format!(
        "发现 {} 个版本冲突:\n{}",
        conflicts.len(),
        messages.join("\n")
    )))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_dep(
        name: &str,
        version: &str,
    ) -> DependencySpec {
        DependencySpec {
            name: name.to_string(),
            version: version.to_string(),
            git: None,
            path: None,
        }
    }

    #[test]
    fn test_no_conflicts() {
        let deps = vec![make_dep("foo", "^1.0.0"), make_dep("bar", "^2.0.0")];
        let conflicts = detect_conflicts(&deps, &[]).unwrap();
        assert!(conflicts.is_empty());
    }

    #[test]
    fn test_same_package_compatible() {
        let deps = vec![make_dep("foo", "^1.0.0")];
        let dev_deps = vec![make_dep("foo", "^1.5.0")];
        let conflicts = detect_conflicts(&deps, &dev_deps).unwrap();
        assert!(conflicts.is_empty());
    }

    #[test]
    fn test_same_package_incompatible() {
        let deps = vec![make_dep("foo", "^1.0.0")];
        let dev_deps = vec![make_dep("foo", "^2.0.0")];
        let conflicts = detect_conflicts(&deps, &dev_deps).unwrap();
        assert_eq!(conflicts.len(), 1);
        assert_eq!(conflicts[0].package_name, "foo");
    }

    #[test]
    fn test_check_conflicts_returns_error() {
        let deps = vec![make_dep("foo", "^1.0.0")];
        let dev_deps = vec![make_dep("foo", "^2.0.0")];
        let result = check_conflicts(&deps, &dev_deps);
        assert!(result.is_err());
    }

    #[test]
    fn test_check_conflicts_ok() {
        let deps = vec![make_dep("foo", "^1.0.0")];
        let dev_deps = vec![];
        let result = check_conflicts(&deps, &dev_deps);
        assert!(result.is_ok());
    }

    #[test]
    fn test_wildcard_no_conflict() {
        let deps = vec![make_dep("foo", "*")];
        let dev_deps = vec![make_dep("foo", "^1.0.0")];
        let conflicts = detect_conflicts(&deps, &dev_deps).unwrap();
        assert!(conflicts.is_empty());
    }

    #[test]
    fn test_conflict_info_display() {
        let info = ConflictInfo {
            package_name: "foo".to_string(),
            requirements: vec![
                ConflictRequirement {
                    from: "dependencies".to_string(),
                    version_req: ">=1.0.0, <2.0.0".to_string(),
                },
                ConflictRequirement {
                    from: "dev-dependencies".to_string(),
                    version_req: ">=2.0.0, <3.0.0".to_string(),
                },
            ],
        };
        let display = info.to_string();
        assert!(display.contains("foo"));
        assert!(display.contains("版本冲突"));
    }
}
