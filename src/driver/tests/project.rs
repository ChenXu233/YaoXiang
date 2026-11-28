//! 多文件源码 → 项目编译 helper（L3-1b S2 基础设施）。
//!
//! driver 侧现有的测试都手工构造 `Unit`/IR/typecheck 产物，没有「写几个 .yx
//! 文件 → 走项目编译」的路径，导致 E1106、同名用户类型这类**多文件语义**判据
//! 只能落在 `tests/integration`。本 helper 只做临时目录落盘，然后调用仓库既有
//! 公共入口 [`crate::run_project`]（与 `tests/integration/multifile.rs` 同一个
//! 入口；内部是 driver::Program::run → MultiFile → orchestrator），不新增生产
//! API、不复制第二套编译路径。

use std::path::PathBuf;

/// 一个写在临时目录里的多文件项目；drop 时自动清理。
pub(crate) struct ProjectFixture {
    dir: tempfile::TempDir,
}

impl ProjectFixture {
    /// 写入 `yaoxiang.toml`（Bin 语义，入口自动调用 main）与全部源码文件。
    pub(crate) fn new(files: &[(&str, &str)]) -> Self {
        let dir = tempfile::tempdir().expect("create tempdir");
        std::fs::write(
            dir.path().join("yaoxiang.toml"),
            "[package]\nname = \"t\"\nversion = \"0.1.0\"\n",
        )
        .expect("write manifest");
        for (name, content) in files {
            let path = dir.path().join(name);
            if let Some(parent) = path.parent() {
                std::fs::create_dir_all(parent).expect("create parent dirs");
            }
            std::fs::write(&path, content).expect("write source file");
        }
        Self { dir }
    }

    /// 项目根下的入口路径。
    pub(crate) fn entry(
        &self,
        name: &str,
    ) -> PathBuf {
        self.dir.path().join(name)
    }

    /// 走既有项目编译/运行入口；失败时返回可断言的调试文本。
    pub(crate) fn run(
        &self,
        entry: &str,
    ) -> Result<(), String> {
        crate::run_project(&self.entry(entry)).map_err(|e| format!("{e:?}"))
    }
}

/// 一步式：写入源码并运行入口。
pub(crate) fn run_project(
    files: &[(&str, &str)],
    entry: &str,
) -> Result<(), String> {
    ProjectFixture::new(files).run(entry)
}

#[test]
fn test_project_helper_runs_multifile_project() {
    // Arrange：lib 提供常量，main 经 use 导入并用 std.assert 验证值
    let lib = "value: Int = 7\n";
    let main = r#"
use std.assert
use lib.{value}

main = () => {
    assert.assert(value == 7, "imported value should be 7")
}
"#;
    // Act
    let result = run_project(&[("lib.yx", lib), ("main.yx", main)], "main.yx");
    // Assert：项目编译并运行通过（run_project 内部已执行断言）
    assert!(
        result.is_ok(),
        "multifile project should compile and run: {result:?}"
    );
}
