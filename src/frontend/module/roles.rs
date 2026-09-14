//! 编译目标角色分类（RFC-029f）
//!
//! 为源文件定义文件级五角色：Script / Bin / Lib / Test / Internal。
//! 判定三层：manifest 显式声明（`[run].main`/`[[bin]]`/`[lib]`/`[exports]`，
//! RFC-015 字段）> 入口可达性推断（被 use = Lib，含 main = Bin）> RFC-036
//! 测试约定（覆盖 Lib/Internal，不覆盖显式 Bin）。
//! 无 manifest 项目整体为 Script 态——一切行为与引入角色模型前一致。
//!
//! 本模块不依赖 `crate::package`（wasm32 下不编译）——manifest 信息经
//! [`TargetViews`] 纯数据视图传入，由调用方（非 wasm 编排器）负责解析。

use std::collections::HashSet;
use std::path::{Path, PathBuf};

/// 源文件的编译目标角色
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FileRole {
    /// 单文件直跑（无 manifest 上下文）：旁路整个模型，行为与现状一致
    Script,
    /// 程序入口：pub 可报死代码（无包外消费者）
    Bin,
    /// 分发边界：pub 豁免死代码（包外消费者不可见，宁漏报）
    Lib,
    /// 测试代码：不参与死代码判定
    Test,
    /// 包内实现：pub 豁免至二阶段（Phase 2 按包内 use 图收紧，RFC-029f）
    Internal,
}

/// manifest 显式声明面（路径已与文件发现侧同口径 canonicalize）
#[derive(Debug, Clone, Default)]
pub struct ExplicitSurfaces {
    /// `[[bin]].path` 与 `[run].main` 指向的文件
    pub bins: HashSet<PathBuf>,
    /// `[lib].path` 与 `[exports]` 值指向的文件
    pub libs: HashSet<PathBuf>,
}

/// manifest 目标字段的纯数据视图（RFC-015）——与 `crate::package` 解耦的
/// 解耦层：bin/lib/exports 的路径字符串列表，由调用方从各自 manifest 解析填充。
#[derive(Debug, Clone, Default)]
pub struct TargetViews {
    /// `[[bin]].path` 与 `[run].main`
    pub bins: Vec<String>,
    /// `[lib].path` 与 `[exports]` 值
    pub libs: Vec<String>,
}

/// canonicalize，失败（不存在等）回退原路径——与文件发现侧同口径
fn canonical(path: &Path) -> PathBuf {
    path.canonicalize().unwrap_or_else(|_| path.to_path_buf())
}

/// 从 manifest 目标视图提取显式声明面（路径相对 manifest 所在目录，即 project_root）
pub fn explicit_surfaces(
    views: &TargetViews,
    project_root: &Path,
) -> ExplicitSurfaces {
    let mut surfaces = ExplicitSurfaces::default();
    for bin in &views.bins {
        surfaces.bins.insert(canonical(&project_root.join(bin)));
    }
    for path in &views.libs {
        surfaces.libs.insert(canonical(&project_root.join(path)));
    }
    surfaces
}

/// RFC-036 测试发现规则（`[tool.test]` patterns/exclude，RFC-029f 消费）。
/// pattern 形态与 test_runner 同构：字面路径或 `root/**/*.yx`——`/**` 后缀
/// 无关紧要，一律按 root 子树前缀命中。默认 `tests/**/*.yx`。
#[derive(Debug, Clone, Default)]
pub struct TestRules {
    include: Vec<String>,
    exclude: Vec<String>,
}

impl TestRules {
    /// RFC-036 默认约定（无 `[tool.test]` 配置时）
    pub fn default_rules() -> Self {
        Self {
            include: vec!["tests/**/*.yx".to_string()],
            exclude: Vec::new(),
        }
    }

    /// 从 `util::config::TestConfig` 转换
    pub fn from_config(config: &crate::util::config::TestConfig) -> Self {
        Self {
            include: config.patterns.clone(),
            exclude: config.exclude.clone(),
        }
    }

    /// pattern 根子树前缀命中（组件级比较，跨平台分隔符安全）
    fn matches(
        file: &Path,
        project_root: &Path,
        pattern: &str,
    ) -> bool {
        let trimmed = pattern.trim_end_matches(['/', '\\', '*']);
        let rel_root = match trimmed.split_once("/**") {
            Some((root, _)) => root,
            None => trimmed,
        };
        file.starts_with(project_root.join(rel_root))
    }

    /// 文件是否命中测试规则
    pub fn is_test(
        &self,
        file: &Path,
        project_root: &Path,
    ) -> bool {
        if self
            .exclude
            .iter()
            .any(|p| Self::matches(file, project_root, p))
        {
            return false;
        }
        self.include
            .iter()
            .any(|p| Self::matches(file, project_root, p))
            // 命名约定兜底：*_test.yx 无论目录
            || file
                .file_stem()
                .map(|s| s.to_string_lossy().ends_with("_test"))
                .unwrap_or(false)
    }
}

/// RFC-036 测试文件约定兜底（无 `[tool.test]` 配置对象时）：
/// `tests/` 目录下，或文件名以 `_test` 结尾
pub fn is_test_file(
    file: &Path,
    project_root: &Path,
) -> bool {
    if file.starts_with(project_root.join("tests")) {
        return true;
    }
    file.file_stem()
        .map(|s| s.to_string_lossy().ends_with("_test"))
        .unwrap_or(false)
}

/// 分类单个文件（RFC-029f 判定优先级：显式 Bin > Test > 显式 Lib >
/// 推断被 use = Lib > 含 main = Bin > Internal）
///
/// * `project_root`：None 表示无 manifest 上下文 → Script
/// * `used_by_other`：该文件被 ≥1 个其他文件的 `use` 引用（发现期边信息）
/// * `has_main`：文件顶层存在 `main` 绑定
pub fn classify(
    file: &Path,
    project_root: Option<&Path>,
    surfaces: Option<&ExplicitSurfaces>,
    test_rules: Option<&TestRules>,
    used_by_other: bool,
    has_main: bool,
) -> FileRole {
    let (Some(root), Some(surfaces)) = (project_root, surfaces) else {
        return FileRole::Script;
    };
    if surfaces.bins.contains(file) {
        return FileRole::Bin;
    }
    let is_test = match test_rules {
        Some(rules) => rules.is_test(file, root),
        None => is_test_file(file, root),
    };
    if is_test {
        return FileRole::Test;
    }
    if surfaces.libs.contains(file) {
        return FileRole::Lib;
    }
    if used_by_other {
        return FileRole::Lib;
    }
    if has_main {
        return FileRole::Bin;
    }
    FileRole::Internal
}

/// 依赖包的导入面（RFC-029f 解析序）：
/// `[exports]` 非空 → 面 = 映射值文件集合；否则 `[lib].path` → 单文件面；
/// 两者皆无 → `None` = 无声明面，维持现状全放行。
///
/// `views` 由调用方从依赖包 manifest 提取（exports 值列表；lib.path 单独传）。
pub fn import_surface_views(
    dep_root: &Path,
    exports: &[String],
    lib_path: Option<&str>,
) -> Option<HashSet<PathBuf>> {
    if !exports.is_empty() {
        return Some(
            exports
                .iter()
                .map(|p| canonical(&dep_root.join(p)))
                .collect(),
        );
    }
    lib_path.map(|lib| HashSet::from([canonical(&dep_root.join(lib))]))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn no_project_root_yields_script() {
        let role = classify(Path::new("/x/main.yx"), None, None, None, false, true);
        assert_eq!(role, FileRole::Script, "无 manifest 上下文应旁路为 Script");
    }

    #[test]
    fn explicit_bin_wins_over_test_path() {
        let root = Path::new("/proj");
        let bin_file = canonical(&root.join("tests/tool.yx"));
        let surfaces = ExplicitSurfaces {
            bins: HashSet::from([bin_file.clone()]),
            libs: HashSet::new(),
        };
        let role = classify(&bin_file, Some(root), Some(&surfaces), None, false, false);
        assert_eq!(role, FileRole::Bin, "显式 Bin 不被 Test 覆盖");
    }

    #[test]
    fn test_dir_overrides_inferred_lib() {
        let root = Path::new("/proj");
        let test_file = canonical(&root.join("tests/util_test.yx"));
        let role = classify(
            &test_file,
            Some(root),
            Some(&ExplicitSurfaces::default()),
            None,
            true,
            false,
        );
        assert_eq!(role, FileRole::Test, "tests/ 下即使被 use 也是 Test");
    }

    #[test]
    fn test_suffix_convention() {
        let root = Path::new("/proj");
        let test_file = canonical(&root.join("src/ops_test.yx"));
        let role = classify(
            &test_file,
            Some(root),
            Some(&ExplicitSurfaces::default()),
            None,
            false,
            true,
        );
        assert_eq!(
            role,
            FileRole::Test,
            "*_test.yx 命名约定命中 Test（含 main 也不例外）"
        );
    }

    #[test]
    fn explicit_lib_beats_inference() {
        let root = Path::new("/proj");
        let lib_file = canonical(&root.join("src/lib.yx"));
        let surfaces = ExplicitSurfaces {
            bins: HashSet::new(),
            libs: HashSet::from([lib_file.clone()]),
        };
        let role = classify(&lib_file, Some(root), Some(&surfaces), None, false, true);
        assert_eq!(role, FileRole::Lib, "显式 [lib] 文件即使无人 use 也是 Lib");
    }

    #[test]
    fn used_by_other_infers_lib() {
        let root = Path::new("/proj");
        let file = canonical(&root.join("src/util.yx"));
        let role = classify(
            &file,
            Some(root),
            Some(&ExplicitSurfaces::default()),
            None,
            true,
            false,
        );
        assert_eq!(role, FileRole::Lib, "被其他文件 use 推断为 Lib");
    }

    #[test]
    fn main_without_use_infers_bin() {
        let root = Path::new("/proj");
        let file = canonical(&root.join("src/main.yx"));
        let role = classify(
            &file,
            Some(root),
            Some(&ExplicitSurfaces::default()),
            None,
            false,
            true,
        );
        assert_eq!(role, FileRole::Bin, "含 main 且无人 use 推断为 Bin");
    }

    #[test]
    fn plain_internal_file() {
        let root = Path::new("/proj");
        let file = canonical(&root.join("src/helper.yx"));
        let role = classify(
            &file,
            Some(root),
            Some(&ExplicitSurfaces::default()),
            None,
            false,
            false,
        );
        assert_eq!(role, FileRole::Internal);
    }

    #[test]
    fn used_main_file_is_lib_not_bin() {
        let root = Path::new("/proj");
        let file = canonical(&root.join("src/main.yx"));
        let role = classify(
            &file,
            Some(root),
            Some(&ExplicitSurfaces::default()),
            None,
            true,
            true,
        );
        assert_eq!(role, FileRole::Lib, "被 use 优先于含 main（RFC 推断顺序）");
    }

    #[test]
    fn surfaces_collect_run_main_and_exports() {
        let root = Path::new("/proj");
        let views = TargetViews {
            bins: vec!["src/cli.yx".to_string(), "src/main.yx".to_string()],
            libs: vec!["src/foo.yx".to_string()],
        };
        let surfaces = explicit_surfaces(&views, root);
        assert!(surfaces.bins.contains(&canonical(&root.join("src/cli.yx"))));
        assert!(surfaces
            .bins
            .contains(&canonical(&root.join("src/main.yx"))));
        assert!(surfaces.libs.contains(&canonical(&root.join("src/foo.yx"))));
    }

    #[test]
    fn import_surface_exports_over_lib_over_none() {
        let dep = Path::new("/vendor/dep-0.1.0");
        // exports 非空 → 面 = exports 值（lib.path 不并入）
        let surface = import_surface_views(dep, &["src/dep.yx".to_string()], Some("src/lib.yx"))
            .expect("exports 存在应有面");
        assert!(surface.contains(&canonical(&dep.join("src/dep.yx"))));
        assert!(!surface.contains(&canonical(&dep.join("src/lib.yx"))));

        // exports 空 → lib 单文件面
        let surface = import_surface_views(dep, &[], Some("src/lib.yx")).expect("lib 存在应有面");
        assert!(surface.contains(&canonical(&dep.join("src/lib.yx"))));

        // 都没有 → None（全放行）
        assert!(
            import_surface_views(dep, &[], None).is_none(),
            "无声明面应全放行"
        );
    }
}

#[cfg(test)]
mod test_rules_tests {
    use super::*;

    fn root() -> std::path::PathBuf {
        // 用真实存在的根避免 starts_with 语义歧义；路径仅做前缀比较
        std::env::temp_dir()
    }

    fn file_under(
        root: &Path,
        rel: &str,
    ) -> std::path::PathBuf {
        canonical(&root.join(rel))
    }

    #[test]
    fn default_rules_hit_tests_dir() {
        let r = root();
        let rules = TestRules::default_rules();
        assert!(rules.is_test(&file_under(&r, "tests/util_test.yx"), &r));
        assert!(!rules.is_test(&file_under(&r, "src/util.yx"), &r));
    }

    #[test]
    fn custom_patterns_hit_nested_dir() {
        let r = root();
        let rules = TestRules {
            include: vec!["feature_tests/**/*.yx".to_string()],
            exclude: vec![],
        };
        assert!(
            rules.is_test(&file_under(&r, "feature_tests/sub/case.yx"), &r),
            "自定义 pattern 目录子树应命中"
        );
        assert!(!rules.is_test(&file_under(&r, "src/case.yx"), &r));
    }

    #[test]
    fn exclude_removes_from_include_subtree() {
        let r = root();
        let rules = TestRules {
            include: vec!["tests/**/*.yx".to_string()],
            exclude: vec!["tests/fixtures/**".to_string()],
        };
        assert!(rules.is_test(&file_under(&r, "tests/a.yx"), &r));
        assert!(
            !rules.is_test(&file_under(&r, "tests/fixtures/dataset.yx"), &r),
            "exclude 子树应从测试判定剔除（RFC-036 §2）"
        );
    }
}
