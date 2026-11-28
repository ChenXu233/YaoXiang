//! LLVM AOT-0/AOT-1 构建与运行判据。
//!
//! 规范来源：RFC-018 §编译流程、附录 B；llvm-aot-execution §1/§2/双后端等价。
//! 测试经公共构建 API 与 CLI 驱动真实 Driver，不手工构造 LLVM 或 ModuleIR。

use std::path::PathBuf;

fn test_yx_bin() -> PathBuf {
    std::env::var_os("YAOXIANG_TEST_BIN").map_or_else(
        || PathBuf::from(env!("CARGO_BIN_EXE_yaoxiang-rs")),
        PathBuf::from,
    )
}

#[test]
fn test_build_default_remains_bytecode_during_migration() {
    // Arrange
    let dir = tempfile::TempDir::new().expect("build default tempdir");
    let source = dir.path().join("demo.yx");
    let output = dir.path().join("demo.42");
    std::fs::write(&source, "main = () => { print(42) }").expect("write default fixture");

    // Act
    let status = std::process::Command::new(test_yx_bin())
        .current_dir(dir.path())
        .arg("build")
        .arg(&source)
        .status()
        .expect("run default build");

    // Assert
    assert!(status.success(), "default bytecode build must succeed");
    let bytes = std::fs::read(&output).expect("default output must be .42");
    assert_eq!(
        &bytes[..4],
        b"YXBC",
        "default output must keep YXBC identity"
    );
}

#[cfg(all(feature = "llvm", target_os = "linux", target_arch = "x86_64"))]
mod enabled {
    use super::test_yx_bin;
    use std::path::{Path, PathBuf};
    use std::process::{Command, Output};

    use tempfile::TempDir;
    use yaoxiang::backends::llvm::compile_ir_to_executable;
    use yaoxiang::build_native_project_with_options;
    use yaoxiang::build_native_with_options;
    use yaoxiang::frontend::config::CompileConfig;
    use yaoxiang::middle::ir::ir::{
        BasicBlock, ConstValue, FunctionBody, FunctionIR, GlobalSlot, Instruction, LocalSlot,
        ModuleIR, Operand,
    };
    use yaoxiang::util::span::{Position, Span};

    fn build(
        source: &str,
        stem: &str,
        optimize: bool,
    ) -> (TempDir, PathBuf, PathBuf) {
        let dir = TempDir::new().expect("native test tempdir");
        let source_path = dir.path().join(format!("{stem}.yx"));
        let output_path = dir.path().join(stem);
        std::fs::write(&source_path, source)
            .unwrap_or_else(|e| panic!("write native fixture {stem}: {e}"));
        build_native_with_options(&source_path, &output_path, false, optimize)
            .unwrap_or_else(|e| panic!("build native fixture {stem}: {e:#}"));
        (dir, source_path, output_path)
    }

    fn run_native(path: &Path) -> Output {
        Command::new(path)
            .output()
            .unwrap_or_else(|e| panic!("run native executable {}: {e}", path.display()))
    }

    fn repo_project(name: &str) -> (PathBuf, PathBuf) {
        let project = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("tests")
            .join("yaoxiang-multifile")
            .join(name);
        let entry = project.join("main.yx");
        (project, entry)
    }

    fn assert_native_project_matches_vm(
        project: &Path,
        entry: &Path,
    ) {
        let vm = Command::new(test_yx_bin())
            .current_dir(project)
            .arg("run")
            .arg(entry)
            .output()
            .expect("run VM multi-file fixture");
        assert!(
            vm.status.success(),
            "VM multi-file fixture failed: {}",
            String::from_utf8_lossy(&vm.stderr)
        );

        let dir = TempDir::new().expect("multi-file API tempdir");
        for optimize in [false, true] {
            let output = dir.path().join(format!("api_o{}", u8::from(optimize)));
            build_native_project_with_options(
                entry,
                &output,
                false,
                optimize,
                CompileConfig::default(),
            )
            .unwrap_or_else(|e| panic!("API multi-file build {optimize}: {e:#}"));
            let native = run_native(&output);
            assert!(
                native.status.success(),
                "API multi-file native failed (optimize={optimize}): {}",
                String::from_utf8_lossy(&native.stderr)
            );
            assert_eq!(
                native.status.code(),
                vm.status.code(),
                "API native/VM status mismatch (optimize={optimize})"
            );
            assert_eq!(
                native.stdout, vm.stdout,
                "API native/VM stdout mismatch (optimize={optimize})"
            );
        }

        let cli_output = dir.path().join("cli");
        let cli = Command::new(test_yx_bin())
            .current_dir(project)
            .arg("build")
            .arg(entry)
            .arg("--emit")
            .arg("exe")
            .arg("--output")
            .arg(&cli_output)
            .output()
            .expect("run CLI multi-file build");
        assert!(
            cli.status.success(),
            "CLI multi-file build failed: {}",
            String::from_utf8_lossy(&cli.stderr)
        );
        let native = run_native(&cli_output);
        assert!(
            native.status.success(),
            "CLI multi-file native failed: {}",
            String::from_utf8_lossy(&native.stderr)
        );
        assert_eq!(
            native.status.code(),
            vm.status.code(),
            "CLI native/VM status mismatch"
        );
        assert_eq!(native.stdout, vm.stdout, "CLI native/VM stdout mismatch");
    }

    fn assert_diagnostic(
        stderr: &[u8],
        code: &str,
        file: &Path,
        line: usize,
    ) {
        let text = String::from_utf8_lossy(stderr);
        assert!(
            text.contains(code),
            "diagnostic should contain {code}: {text:?}"
        );
        let location = format!("{}:{line}:", file.display());
        assert!(
            text.contains(&location),
            "diagnostic should contain {location}: {text:?}"
        );
    }

    /// 从诊断文本里取该文件的第一个 `file:line:` 位置行号；没有时返回 None。
    /// 用于把原生诊断的位置与 VM 诊断逐项比对，而不是在测试里另抄一份行号。
    fn diagnostic_line(
        stderr: &[u8],
        file: &Path,
    ) -> Option<usize> {
        let text = String::from_utf8_lossy(stderr);
        let marker = format!("{}:", file.display());
        let start = text.find(&marker)? + marker.len();
        let digits: String = text[start..]
            .chars()
            .take_while(char::is_ascii_digit)
            .collect();
        digits.parse().ok()
    }

    #[test]
    fn test_native_project_bin_entry_function_is_called() {
        // Arrange：Bin 项目只定义 main，顶层不调用；入口必须由 entry_function 驱动。
        let root = TempDir::new().expect("bin entry project");
        write_project(
            root.path(),
            &[
                (
                    "yaoxiang.toml",
                    "[package]\nname = \"bin_entry\"\nversion = \"0.1.0\"\n",
                ),
                ("main.yx", "main = () => {\n    10 / 0\n}\n"),
            ],
        );
        let entry = root.path().join("main.yx");
        let output = root.path().join("bin_entry");

        // Act
        build_native_project_with_options(&entry, &output, false, false, CompileConfig::default())
            .unwrap_or_else(|e| panic!("build Bin entry fixture: {e:#}"));
        let native = run_native(&output);
        let vm = Command::new(test_yx_bin())
            .current_dir(root.path())
            .arg("run")
            .arg(&entry)
            .output()
            .expect("run VM Bin entry fixture");

        // Assert
        assert_eq!(native.status.code(), Some(1), "Bin entry native exit");
        assert_eq!(vm.status.code(), Some(1), "Bin entry VM exit");
        assert_diagnostic(&native.stderr, "E6001", &entry, 2);
        assert_diagnostic(&vm.stderr, "E6001", &entry, 2);
    }

    #[test]
    fn test_native_script_unused_main_is_rejected() {
        // Arrange：单文件 Script 只定义 main、顶层不调用；native 必须在构建期拒绝。
        let dir = TempDir::new().expect("script no-entry tempdir");
        let source = dir.path().join("script.yx");
        let executable = dir.path().join("script");
        std::fs::write(&source, "main = () => {\n    10 / 0\n}\n").expect("write Script fixture");

        // Act
        let error = build_native_with_options(&source, &executable, false, false)
            .expect_err("Script with uncalled main must fail at AOT build time");
        let detail = format!("{error:#}");
        let vm = Command::new(test_yx_bin())
            .current_dir(dir.path())
            .arg("run")
            .arg(&source)
            .output()
            .expect("run VM Script no-entry fixture");

        // Assert
        assert!(
            detail.contains("Script defines main but never calls it")
                && detail.contains("AOT does not call main implicitly"),
            "diagnostic must explain the Script/main boundary: {detail}"
        );
        assert!(
            detail.contains(&format!("{}:2:", source.display())),
            "diagnostic must carry the main definition position: {detail}"
        );
        assert!(
            !executable.exists(),
            "AOT must not write an artifact for rejected Script/main input"
        );
        assert_eq!(vm.status.code(), Some(1), "VM #476 hint still exits 1");
        let vm_diagnostic = String::from_utf8_lossy(&vm.stderr);
        assert!(
            vm_diagnostic.contains("不会自动执行 main") && vm_diagnostic.contains("main 未被调用"),
            "VM script hint must remain unchanged: {vm_diagnostic}"
        );
    }

    #[test]
    fn test_native_script_without_main_still_builds() {
        // Arrange：无 main 的 Script 不属于 F1 拒绝边界。
        let dir = TempDir::new().expect("script no-main tempdir");
        let source = dir.path().join("plain.yx");
        let executable = dir.path().join("plain");
        std::fs::write(&source, "1 + 1\n").expect("write Script no-main fixture");

        // Act
        build_native_with_options(&source, &executable, false, false)
            .unwrap_or_else(|e| panic!("Script without main must build: {e:#}"));
        let native = run_native(&executable);

        // Assert
        assert!(
            native.status.success(),
            "Script without main should build and run: {}",
            String::from_utf8_lossy(&native.stderr)
        );
    }

    #[test]
    fn test_native_multifile_dependency_init_diagnostic_uses_real_file() {
        // Arrange
        let root = TempDir::new().expect("dependency init project");
        write_project(
            root.path(),
            &[
                (
                    "yaoxiang.toml",
                    "[package]\nname = \"diag_init\"\nversion = \"0.1.0\"\n",
                ),
                ("main.yx", "use dep.{bad}\n\nmain = () => {\n}\n"),
                ("dep.yx", "bad: Int = 10 / 0\n"),
            ],
        );
        let entry = root.path().join("main.yx");
        let dep = root.path().join("dep.yx");

        // Act
        assert_project_diagnostic(root.path(), &entry, &dep);

        // Assert
        // helper 已断言 VM 与 native O0/O2 的 E6001 均指向 dep.yx:1。
    }

    #[test]
    fn test_native_multifile_dependency_function_diagnostic_uses_real_file() {
        // Arrange
        let root = TempDir::new().expect("dependency function project");
        write_project(
            root.path(),
            &[
                (
                    "yaoxiang.toml",
                    "[package]\nname = \"diag_fn\"\nversion = \"0.1.0\"\n",
                ),
                (
                    "main.yx",
                    "use dep.{boom}\n\nmain = () => {\n    boom()\n}\n",
                ),
                ("dep.yx", "boom: () -> Int = () => 10 / 0\n"),
            ],
        );
        let entry = root.path().join("main.yx");
        let dep = root.path().join("dep.yx");

        // Act
        assert_project_diagnostic(root.path(), &entry, &dep);

        // Assert
        // helper 已断言 VM 与 native O0/O2 的 E6001 均指向 dep.yx:1。
    }

    fn write_project(
        root: &Path,
        files: &[(&str, &str)],
    ) {
        std::fs::create_dir_all(root).expect("create temp project");
        for (name, source) in files {
            std::fs::write(root.join(name), source)
                .unwrap_or_else(|e| panic!("write project file {name}: {e}"));
        }
    }

    fn assert_project_diagnostic(
        root: &Path,
        entry: &Path,
        dependency: &Path,
    ) {
        let vm = Command::new(test_yx_bin())
            .current_dir(root)
            .arg("run")
            .arg(entry)
            .output()
            .expect("run VM dependency-error fixture");
        assert_eq!(
            vm.status.code(),
            Some(1),
            "VM dependency error exit: {}",
            String::from_utf8_lossy(&vm.stderr)
        );
        assert_diagnostic(&vm.stderr, "E6001", dependency, 1);

        let dir = TempDir::new().expect("dependency diagnostic tempdir");
        for optimize in [false, true] {
            let output = dir.path().join(format!("native_o{}", u8::from(optimize)));
            build_native_project_with_options(
                entry,
                &output,
                false,
                optimize,
                CompileConfig::default(),
            )
            .unwrap_or_else(|e| panic!("project diagnostic build {optimize}: {e:#}"));
            let native = run_native(&output);
            assert_eq!(
                native.status.code(),
                Some(1),
                "native dependency error exit (optimize={optimize}): {}",
                String::from_utf8_lossy(&native.stderr)
            );
            assert_diagnostic(&native.stderr, "E6001", dependency, 1);
        }
    }

    #[test]
    fn test_native_multifile_use_constant_cli_and_api() {
        // Arrange
        let (project, entry) = repo_project("use-constant");

        // Act
        assert_native_project_matches_vm(&project, &entry);

        // Assert
        // helper 已断言 CLI 与 API 的 native/VM 状态码和 stdout 一致。
    }

    #[test]
    fn test_native_multifile_subdirectory_module_cli_and_api() {
        // Arrange
        let (project, entry) = repo_project("subdirectory-module");

        // Act
        assert_native_project_matches_vm(&project, &entry);

        // Assert
        // helper 已断言 CLI 与 API 的 native/VM 状态码和 stdout 一致。
    }

    #[test]
    fn test_native_multifile_init_order_cli_and_api() {
        // Arrange
        let (project, entry) = repo_project("init-order-entry-reads-dep-value");

        // Act
        assert_native_project_matches_vm(&project, &entry);

        // Assert
        // helper 已断言 CLI 与 API 的 native/VM 状态码和 stdout 一致。
    }

    #[test]
    fn test_native_arithmetic_matches_o0_and_o2() {
        // Arrange
        let source = concat!(
            "use std.assert\n",
            "add: (a: Int, b: Int) -> Int = (a, b) => a + b\n",
            "modulo: (a: Int, b: Int) -> Int = (a, b) => a % b\n",
            "assert.assert(add(20, 22) == 42, \"add result\")\n",
            "assert.assert(modulo(-7, 3) == 2, \"negative dividend\")\n",
            "assert.assert(modulo(7, -3) == -2, \"negative divisor\")\n",
        );

        for optimize in [false, true] {
            // Act
            let (_dir, _source, executable) = build(source, "arithmetic", optimize);
            let output = run_native(&executable);

            // Assert
            assert!(
                output.status.success(),
                "native run failed (optimize={optimize}): {}",
                String::from_utf8_lossy(&output.stderr)
            );
        }
    }

    #[test]
    fn test_native_assert_failure_matches_vm_diagnostic() {
        // Arrange
        let source = "use std.assert\nassert.assert(false, \"boom\")\n";

        for optimize in [false, true] {
            // Act
            let (_dir, source_path, executable) = build(source, "assert_failure", optimize);
            let native = run_native(&executable);
            let vm = Command::new(test_yx_bin())
                .arg("run")
                .arg(&source_path)
                .output()
                .expect("run VM assertion fixture");

            // Assert
            assert_eq!(native.status.code(), Some(1), "native assertion exit");
            assert_eq!(vm.status.code(), Some(1), "VM assertion exit");
            assert_diagnostic(&native.stderr, "E6005", &source_path, 2);
            assert_diagnostic(&vm.stderr, "E6005", &source_path, 2);
        }
    }

    #[test]
    fn test_native_loop_division_by_zero_matches_vm_diagnostic() {
        // Arrange：循环调用会除零的函数；无 Phi、无 std 调用、无合并 Local。
        let source = concat!(
            "broken: (n: Int) -> Int = (n) => 10 / n\n",
            "main: () -> Void = {\n",
            "    mut i = 1\n",
            "    mut sum = 0\n",
            "    while i < 3 {\n",
            "        sum = sum + broken(i - 1)\n",
            "        i = i + 1\n",
            "    }\n",
            "}\n",
            "\n",
            "main()\n",
        );
        let dir = TempDir::new().expect("loop division tempdir");
        let source_path = dir.path().join("loop_div_zero.yx");
        std::fs::write(&source_path, source).expect("write loop division fixture");
        let o0_path = dir.path().join("loop_div_zero_o0");
        let o2_path = dir.path().join("loop_div_zero_o2");

        // Act
        build_native_with_options(&source_path, &o0_path, false, false)
            .unwrap_or_else(|e| panic!("build loop division O0: {e:#}"));
        build_native_with_options(&source_path, &o2_path, false, true)
            .unwrap_or_else(|e| panic!("build loop division O2: {e:#}"));
        let vm = Command::new(test_yx_bin())
            .arg("run")
            .arg(&source_path)
            .output()
            .expect("run VM loop division fixture");
        let native_o0 = run_native(&o0_path);
        let native_o2 = run_native(&o2_path);

        // Assert
        for (optimize, native) in [("O0", &native_o0), ("O2", &native_o2)] {
            assert_eq!(
                native.status.code(),
                Some(1),
                "native {optimize} loop division exit: {}",
                String::from_utf8_lossy(&native.stderr)
            );
            assert_eq!(
                vm.status.code(),
                Some(1),
                "VM loop division exit: {}",
                String::from_utf8_lossy(&vm.stderr)
            );
            assert_diagnostic(&native.stderr, "E6001", &source_path, 1);
            assert_diagnostic(&vm.stderr, "E6001", &source_path, 1);
        }
    }

    #[test]
    fn test_native_division_by_zero_reports_source() {
        // Arrange
        let source = concat!(
            "use std.assert\n",
            "divide: (a: Int, b: Int) -> Int = (a, b) => a / b\n",
            "assert.assert(divide(1, 0) == 0)\n",
        );
        let (_dir, source_path, executable) = build(source, "div_zero", false);

        // Act
        let output = run_native(&executable);

        // Assert
        assert_eq!(output.status.code(), Some(1), "division by zero exit");
        assert_diagnostic(&output.stderr, "E6001", &source_path, 2);
    }

    #[test]
    fn test_native_add_overflow_reports_source() {
        // Arrange
        let source = concat!(
            "use std.assert\n",
            "add: (a: Int, b: Int) -> Int = (a, b) => a + b\n",
            "assert.assert(add(9223372036854775807, 1) == 0)\n",
        );
        let (_dir, source_path, executable) = build(source, "add_overflow", false);

        // Act
        let output = run_native(&executable);
        let vm = Command::new(test_yx_bin())
            .arg("run")
            .arg(&source_path)
            .output()
            .expect("run VM overflow fixture");

        // Assert
        assert_eq!(output.status.code(), Some(1), "overflow exit");
        assert_eq!(vm.status.code(), Some(1), "VM overflow exit");
        assert_diagnostic(&output.stderr, "E6007", &source_path, 3);
        assert_diagnostic(&vm.stderr, "E6007", &source_path, 3);
    }

    #[test]
    fn test_native_min_divided_by_minus_one_reports_overflow() {
        // Arrange
        let source = concat!(
            "use std.assert\n",
            "divide: (a: Int, b: Int) -> Int = (a, b) => a / b\n",
            "assert.assert(divide(-9223372036854775807 - 1, -1) == 0)\n",
        );
        let (_dir, source_path, executable) = build(source, "div_overflow", true);

        // Act
        let output = run_native(&executable);

        // Assert
        assert_eq!(output.status.code(), Some(1), "division overflow exit");
        assert_diagnostic(&output.stderr, "E6007", &source_path, 2);
    }

    #[test]
    fn test_native_min_literal_negation_reports_overflow() {
        // Arrange
        let source = "use std.assert\nassert.assert(-9223372036854775808 == 0)\n";
        let (_dir, _source, executable) = build(source, "neg_overflow", false);

        // Act
        let output = run_native(&executable);

        // Assert
        assert_eq!(
            output.status.code(),
            Some(1),
            "literal negation overflow exit"
        );
        assert!(
            String::from_utf8_lossy(&output.stderr).contains("E6007"),
            "literal negation should report E6007: {:?}",
            output.stderr
        );
    }

    #[test]
    fn test_native_if_tail_value_matches_vm() {
        // Arrange：if 表达式作尾值 → 汇合值；批次 3.5 起映射为 Phi/边拷贝，
        // 不再是原生能力边界（原拒绝用例转正向，见 test_native_missing_linker
        // 与 test_native_debug_info 仍守「失败构建不覆盖上次产物」）。
        let source = concat!(
            "use std.assert\n",
            "choose: (a: Int) -> Int = (a) => if a == 0 { 1 } else { 2 }\n",
            "assert.assert(choose(0) == 1, \"tail if true\")\n",
            "assert.assert(choose(5) == 2, \"tail if false\")\n",
        );

        // Act
        assert_native_matches_vm(source, "if_tail");

        // Assert
        // assert_native_matches_vm 已断言 native/VM 的退出码与 stdout 一致。
    }

    #[test]
    fn test_native_unreachable_unsupported_function_is_ignored() {
        // Arrange
        let source = concat!(
            "use std.assert\n",
            "choose: (a: Int) -> Int = (a) => if a == 0 { 1 } else { 2 }\n",
            "assert.assert(true)\n",
        );
        let (_dir, _source, executable) = build(source, "unreachable_branch", false);

        // Act
        let output = run_native(&executable);

        // Assert
        assert!(
            output.status.success(),
            "unreachable unsupported functions must not block AOT: {}",
            String::from_utf8_lossy(&output.stderr)
        );
    }

    #[test]
    fn test_native_debug_info_is_not_silently_ignored() {
        // Arrange
        let dir = TempDir::new().expect("native test tempdir");
        let source_path = dir.path().join("debug.yx");
        let output_path = dir.path().join("debug");
        std::fs::write(&source_path, "use std.assert\nassert.assert(true)\n")
            .expect("write debug fixture");

        // Act
        let error = build_native_with_options(&source_path, &output_path, true, false)
            .expect_err("AOT-1 has no native debug-info emitter");

        // Assert
        assert!(
            error.to_string().contains("--debug-info"),
            "error should name the unsupported option: {error:#}"
        );
        assert!(
            !output_path.exists(),
            "rejected build must not create output"
        );
    }

    #[test]
    fn test_native_missing_linker_preserves_output() {
        // Arrange
        let dir = TempDir::new().expect("native test tempdir");
        let source_path = dir.path().join("ok.yx");
        let output_path = dir.path().join("ok");
        std::fs::write(&source_path, "use std.assert\nassert.assert(true)\n")
            .expect("write linker fixture");
        std::fs::write(&output_path, b"previous-good-output").expect("seed output");
        let missing = dir.path().join("definitely-no-such-cc");

        // Act
        let output = Command::new(test_yx_bin())
            .arg("build")
            .arg(&source_path)
            .arg("--emit")
            .arg("exe")
            .arg("-o")
            .arg(&output_path)
            .env("YAOXIANG_CC", &missing)
            .output()
            .expect("run native build with missing linker");

        // Assert
        assert!(!output.status.success(), "missing linker must fail");
        assert!(
            String::from_utf8_lossy(&output.stderr).contains("toolchain"),
            "error should identify the toolchain: {:?}",
            output.stderr
        );
        assert_eq!(
            std::fs::read(&output_path).expect("read preserved output"),
            b"previous-good-output",
            "missing linker must not overwrite previous output"
        );
    }

    #[test]
    fn test_native_cli_emit_exe_uses_source_stem() {
        // Arrange
        let dir = TempDir::new().expect("native test tempdir");
        let source_path = dir.path().join("demo.yx");
        let executable = dir.path().join("demo");
        std::fs::write(&source_path, "use std.assert\nassert.assert(true)\n")
            .expect("write CLI fixture");

        // Act
        let output = Command::new(test_yx_bin())
            .current_dir(dir.path())
            .arg("build")
            .arg(&source_path)
            .arg("--emit")
            .arg("exe")
            .output()
            .expect("run native CLI build");

        // Assert
        assert!(
            output.status.success(),
            "CLI native build failed: {}",
            String::from_utf8_lossy(&output.stderr)
        );
        assert!(executable.is_file(), "default native output should exist");
        assert!(
            run_native(&executable).status.success(),
            "default native output should run"
        );
    }

    fn build_synthetic(
        module: &ModuleIR,
        stem: &str,
        optimize: bool,
    ) -> (TempDir, PathBuf) {
        let dir = TempDir::new().expect("synthetic native tempdir");
        let output = dir.path().join(format!("{stem}_{}", u8::from(optimize)));
        let source = dir.path().join(format!("{stem}.yx"));
        compile_ir_to_executable(module, &source.display().to_string(), &output, optimize)
            .unwrap_or_else(|e| {
                panic!(
                    "build synthetic native fixture {stem} (O{}): {e:#}",
                    u8::from(optimize)
                )
            });
        (dir, output)
    }

    fn value_module() -> ModuleIR {
        let function = FunctionIR {
            name: "value_destination".to_string(),
            def: None,
            params: vec![],
            return_type: yaoxiang::frontend::core::sema::MonoType::Void,
            generic_params: None,
            body: FunctionBody::Code {
                blocks: vec![BasicBlock {
                    label: 0,
                    instructions: vec![
                        Instruction::Load {
                            dst: Operand::Value(yaoxiang::middle::ir::ir::ValueId(0)),
                            src: Operand::Const(ConstValue::Int(41)),
                            span: Span::dummy(),
                        },
                        Instruction::Add {
                            dst: Operand::Value(yaoxiang::middle::ir::ir::ValueId(1)),
                            lhs: Operand::Value(yaoxiang::middle::ir::ir::ValueId(0)),
                            rhs: Operand::Const(ConstValue::Int(1)),
                            span: Span::dummy(),
                        },
                        Instruction::Eq {
                            dst: Operand::Value(yaoxiang::middle::ir::ir::ValueId(2)),
                            lhs: Operand::Value(yaoxiang::middle::ir::ir::ValueId(1)),
                            rhs: Operand::Const(ConstValue::Int(42)),
                            span: Span::dummy(),
                        },
                        Instruction::Call {
                            dst: None,
                            func: Operand::Const(ConstValue::String(
                                "std.assert.assert".to_string(),
                            )),
                            args: vec![Operand::Value(yaoxiang::middle::ir::ir::ValueId(2))],
                            def: None,
                            span: Span::dummy(),
                        },
                        Instruction::Ret {
                            value: None,
                            span: Span::dummy(),
                        },
                    ],
                    successors: vec![],
                }],
                entry: 0,
                locals: vec![],
                values: vec![
                    yaoxiang::middle::ir::ir::ValueInfo {
                        ty: yaoxiang::frontend::core::sema::MonoType::Int(64),
                        debug_name: None,
                        defining_block: 0,
                    },
                    yaoxiang::middle::ir::ir::ValueInfo {
                        ty: yaoxiang::frontend::core::sema::MonoType::Int(64),
                        debug_name: None,
                        defining_block: 0,
                    },
                    yaoxiang::middle::ir::ir::ValueInfo {
                        ty: yaoxiang::frontend::core::sema::MonoType::Bool,
                        debug_name: None,
                        defining_block: 0,
                    },
                ],
                value_slots: vec![],
            },
        };
        let mut module = ModuleIR::default();
        module.functions.push(function);
        module.init.push(Instruction::Call {
            dst: None,
            func: Operand::Const(ConstValue::String("value_destination".to_string())),
            args: vec![],
            def: None,
            span: Span::dummy(),
        });
        module
    }

    fn branch_module(condition: bool) -> ModuleIR {
        let function = FunctionIR {
            name: "branch".to_string(),
            def: None,
            params: vec![yaoxiang::frontend::core::sema::MonoType::Bool],
            return_type: yaoxiang::frontend::core::sema::MonoType::Void,
            generic_params: None,
            body: FunctionBody::Code {
                blocks: vec![
                    BasicBlock {
                        label: 0,
                        instructions: vec![Instruction::JmpIfNot {
                            cond: Operand::Arg(0),
                            target: 3,
                            span: Span::dummy(),
                        }],
                        successors: vec![1, 2],
                    },
                    BasicBlock {
                        label: 1,
                        instructions: vec![
                            Instruction::Call {
                                dst: None,
                                func: Operand::Const(ConstValue::String(
                                    "std.assert.assert".to_string(),
                                )),
                                args: vec![Operand::Const(ConstValue::Bool(true))],
                                def: None,
                                span: Span::dummy(),
                            },
                            Instruction::Ret {
                                value: None,
                                span: Span::dummy(),
                            },
                        ],
                        successors: vec![],
                    },
                    BasicBlock {
                        label: 2,
                        instructions: vec![
                            Instruction::Call {
                                dst: None,
                                func: Operand::Const(ConstValue::String(
                                    "std.assert.assert".to_string(),
                                )),
                                args: vec![Operand::Const(ConstValue::Bool(false))],
                                def: None,
                                span: Span::dummy(),
                            },
                            Instruction::Ret {
                                value: None,
                                span: Span::dummy(),
                            },
                        ],
                        successors: vec![],
                    },
                ],
                entry: 0,
                locals: vec![],
                values: vec![],
                value_slots: vec![],
            },
        };
        let mut module = ModuleIR::default();
        module.functions.push(function);
        module.init.push(Instruction::Call {
            dst: None,
            func: Operand::Const(ConstValue::String("branch".to_string())),
            args: vec![Operand::Const(ConstValue::Bool(condition))],
            def: None,
            span: Span::dummy(),
        });
        module
    }

    #[test]
    fn test_native_local_multi_arm_merge_matches_vm() {
        // Arrange：同一 Local 在两臂各写、汇合后读；批次 3.5 起由入口块 alloca
        // 支持，原 AOT-3.5 拒绝用例转为正向等价用例（能力扩展，不是放宽判据）。
        let source = concat!(
            "use std.assert\n",
            "test: (c: Bool) -> Int = (c) => {\n",
            "    a = if c { 42 } else { 0 }\n",
            "    b = a + 1\n",
            "    b\n",
            "}\n",
            "assert.assert(test(true) == 43, \"true arm merges 42\")\n",
            "assert.assert(test(false) == 1, \"false arm merges 0\")\n",
        );

        // Act
        assert_native_matches_vm(source, "local_merge");

        // Assert
        // assert_native_matches_vm 已断言 native/VM 的退出码与 stdout 一致。
    }

    #[test]
    fn test_native_value_destination_readback() {
        // Arrange
        let module = value_module();

        // Act
        let (_dir, executable) = build_synthetic(&module, "value_destination", false);
        let output = run_native(&executable);

        // Assert
        assert!(
            output.status.success(),
            "Value 产出/读回链路应运行成功：{}",
            String::from_utf8_lossy(&output.stderr)
        );
    }

    #[test]
    fn test_native_no_phi_loop_matches_vm() {
        // Arrange：循环只有 Local 存储与 Int/Bool 值，不产生 Phi。
        let source = concat!(
            "main: () -> Void = {\n",
            "    mut i = 0\n",
            "    while i < 3 {\n",
            "        i = i + 1\n",
            "    }\n",
            "}\n",
            "\n",
            "main()\n",
        );
        let (_vm_dir, source_path, _) = build(source, "no_phi_loop_vm", false);

        // Act
        let vm = Command::new(test_yx_bin())
            .arg("run")
            .arg(&source_path)
            .output()
            .expect("run VM no-Phi loop fixture");

        // Assert
        for optimize in [false, true] {
            let (_dir, _source, executable) = build(source, "no_phi_loop", optimize);
            let native = run_native(&executable);
            assert!(
                native.status.success(),
                "native no-Phi loop failed (optimize={optimize}): {}",
                String::from_utf8_lossy(&native.stderr)
            );
            assert_eq!(
                native.status.success(),
                vm.status.success(),
                "native and VM must agree on no-Phi loop (optimize={optimize}): native={}, vm={}",
                String::from_utf8_lossy(&native.stderr),
                String::from_utf8_lossy(&vm.stderr)
            );
        }
    }

    #[test]
    fn test_native_no_phi_branch_executes_both_edges() {
        // Arrange：条件真走 fallthrough（assert true），条件假走 target（assert false）。
        let cases = [(true, true), (false, false)];
        let mut outcomes = Vec::new();

        // Act
        for (condition, expected_success) in cases {
            let module = branch_module(condition);
            let (_dir, executable) = build_synthetic(&module, "branch", false);
            let output = run_native(&executable);
            outcomes.push((condition, expected_success, output));
        }

        // Assert
        for (condition, expected_success, output) in outcomes {
            assert_eq!(
                output.status.success(),
                expected_success,
                "condition={condition} 的无 Phi 分支应走预期边：{}",
                String::from_utf8_lossy(&output.stderr)
            );
        }
    }

    fn assert_native_matches_vm(
        source: &str,
        stem: &str,
    ) {
        let (_vm_dir, source_path, _) = build(source, &format!("{stem}_vm"), false);
        let vm = Command::new(test_yx_bin())
            .arg("run")
            .arg(&source_path)
            .output()
            .expect("run VM control-flow fixture");

        for optimize in [false, true] {
            let (_dir, _source, executable) = build(source, stem, optimize);
            let native = run_native(&executable);
            assert!(
                native.status.success(),
                "native control-flow fixture {stem} failed (optimize={optimize}): {}",
                String::from_utf8_lossy(&native.stderr)
            );
            assert!(
                vm.status.success(),
                "VM control-flow fixture {stem} failed: {}",
                String::from_utf8_lossy(&vm.stderr)
            );
            assert_eq!(
                native.status.code(),
                vm.status.code(),
                "native/VM status mismatch for {stem} (optimize={optimize})"
            );
            assert_eq!(
                native.stdout, vm.stdout,
                "native/VM stdout mismatch for {stem} (optimize={optimize})"
            );
        }
    }

    #[test]
    fn test_native_break_continue_matches_vm() {
        // Arrange
        let source = concat!(
            "use std.assert\n",
            "main: () -> Int = {\n",
            "    mut i = 0\n",
            "    while i < 10 {\n",
            "        i = i + 1\n",
            "        break\n",
            "    }\n",
            "    while i < 10 {\n",
            "        i = i + 1\n",
            "        continue\n",
            "    }\n",
            "    i\n",
            "}\n",
            "assert.assert(main() == 10, \"break/continue final i is 10\")\n",
        );

        // Act
        assert_native_matches_vm(source, "break_continue");

        // Assert
        // assert_native_matches_vm 已断言 native/VM 的 O0 与 O2 结果一致。
    }

    #[test]
    fn test_native_nested_if_else_matches_vm() {
        // Arrange
        let source = concat!(
            "use std.assert\n",
            "nested: (c: Bool) -> Int = (c) => {\n",
            "    mut v = 0\n",
            "    if c {\n",
            "        if 1 < 2 {\n",
            "            v = 1\n",
            "        } else {\n",
            "            v = 2\n",
            "        }\n",
            "    } else {\n",
            "        v = 3\n",
            "    }\n",
            "    v\n",
            "}\n",
            "assert.assert(nested(true) == 1, \"nested if/else selects inner true\")\n",
        );

        // Act
        assert_native_matches_vm(source, "nested_if");

        // Assert
        // assert_native_matches_vm 已断言 native/VM 的 O0 与 O2 结果一致。
    }

    #[test]
    fn test_native_nested_while_matches_vm() {
        // Arrange
        let source = concat!(
            "use std.assert\n",
            "nested: () -> Int = {\n",
            "    mut i = 0\n",
            "    mut sum = 0\n",
            "    while i < 3 {\n",
            "        mut j = 0\n",
            "        while j < 2 {\n",
            "            sum = sum + 1\n",
            "            j = j + 1\n",
            "        }\n",
            "        i = i + 1\n",
            "    }\n",
            "    sum\n",
            "}\n",
            "assert.assert(nested() == 6, \"nested while executes 3 x 2\")\n",
        );

        // Act
        assert_native_matches_vm(source, "nested_while");

        // Assert
        // assert_native_matches_vm 已断言 native/VM 的 O0 与 O2 结果一致。
    }

    #[test]
    fn test_native_loop_local_update_matches_vm() {
        // Arrange
        let source = concat!(
            "use std.assert\n",
            "loop_update: () -> Int = {\n",
            "    mut i = 0\n",
            "    while i < 3 {\n",
            "        i = i + 1\n",
            "    }\n",
            "    i\n",
            "}\n",
            "assert.assert(loop_update() == 3, \"loop Local update\")\n",
        );

        // Act
        assert_native_matches_vm(source, "loop_update");

        // Assert
        // assert_native_matches_vm 已断言 native/VM 的 O0 与 O2 结果一致。
    }

    #[test]
    fn test_native_cross_block_value_matches_vm() {
        // Arrange
        let source = concat!(
            "use std.assert\n",
            "cross: () -> Int = {\n",
            "    v = 21\n",
            "    mut i = 0\n",
            "    while i < 2 {\n",
            "        i = i + 1\n",
            "    }\n",
            "    v\n",
            "}\n",
            "assert.assert(cross() == 21, \"cross-block Value direct use\")\n",
        );

        // Act
        assert_native_matches_vm(source, "cross_value");

        // Assert
        // assert_native_matches_vm 已断言 native/VM 的 O0 与 O2 结果一致。
    }

    #[test]
    fn test_native_if_value_merge_matches_vm() {
        // Arrange：if 表达式作值 → 两臂写同一 Local，汇合后读（批次 3.5 前被拒）。
        let source = concat!(
            "use std.assert\n",
            "pick: (c: Bool) -> Int = (c) => {\n",
            "    x = if c { 42 } else { 7 }\n",
            "    x + 1\n",
            "}\n",
            "assert.assert(pick(true) == 43, \"if true yields 43\")\n",
            "assert.assert(pick(false) == 8, \"if false yields 8\")\n",
        );

        // Act
        assert_native_matches_vm(source, "if_value");

        // Assert
        // assert_native_matches_vm 已断言 native/VM 的 O0 与 O2 结果一致。
    }

    #[test]
    fn test_native_while_merge_value_matches_vm() {
        // Arrange：while 条件里的 and 短路合流产生 Phi（循环头带显式未知 incoming），
        // 汇合值按前驱边登记并由布局落空边进入。
        let source = concat!(
            "use std.assert\n",
            "count: () -> Int = () => {\n",
            "    mut i = 0\n",
            "    while i < 5 and i != 3 {\n",
            "        i = i + 1\n",
            "    }\n",
            "    i\n",
            "}\n",
            "assert.assert(count() == 3, \"while and-merge exits at 3\")\n",
        );

        // Act
        assert_native_matches_vm(source, "while_merge");

        // Assert
        // assert_native_matches_vm 已断言 native/VM 的 O0 与 O2 结果一致。
    }

    #[test]
    fn test_native_nested_merge_value_matches_vm() {
        // Arrange：嵌套 if 表达式作值 → 两级 Phi 链 + 汇合后的 Local 读。
        let source = concat!(
            "use std.assert\n",
            "score: (s: Int) -> Int = (s) => {\n",
            "    grade = if s >= 90 { 1 } else { if s >= 80 { 2 } else { 3 } }\n",
            "    grade\n",
            "}\n",
            "assert.assert(score(95) == 1, \"grade A\")\n",
            "assert.assert(score(85) == 2, \"grade B\")\n",
            "assert.assert(score(70) == 3, \"grade C\")\n",
        );

        // Act
        assert_native_matches_vm(source, "nested_merge");

        // Assert
        // assert_native_matches_vm 已断言 native/VM 的 O0 与 O2 结果一致。
    }

    #[test]
    fn test_native_phi_short_circuit_merge_matches_vm() {
        // Arrange：and/or 短路合流产生 Phi（合流块经块尾落空边进入，无 Jmp）。
        let source = concat!(
            "use std.assert\n",
            "both: (a: Bool, b: Bool) -> Bool = (a, b) => a and b\n",
            "either: (a: Bool, b: Bool) -> Bool = (a, b) => a or b\n",
            "assert.assert(both(true, true), \"and tt\")\n",
            "assert.assert(both(true, false) == false, \"and tf\")\n",
            "assert.assert(either(false, true), \"or ft\")\n",
            "assert.assert(either(false, false) == false, \"or ff\")\n",
        );

        // Act
        assert_native_matches_vm(source, "phi_short_circuit");

        // Assert
        // assert_native_matches_vm 已断言 native/VM 的 O0 与 O2 结果一致。
    }

    #[test]
    fn test_native_phi_failure_path_matches_vm_diagnostic() {
        // Arrange：含 Phi 汇合的函数里发生运行期失败，诊断码与源码行必须与 VM 一致。
        let source = concat!(
            "use std.assert\n",
            "broken: (n: Int) -> Int = (n) => 10 / n\n",
            "choose: (c: Bool) -> Int = (c) => if c { broken(0) } else { 1 }\n",
            "assert.assert(choose(true) == 0)\n",
        );
        let dir = TempDir::new().expect("phi failure tempdir");
        let source_path = dir.path().join("phi_failure.yx");
        std::fs::write(&source_path, source).expect("write phi failure fixture");

        // Act
        let vm = Command::new(test_yx_bin())
            .arg("run")
            .arg(&source_path)
            .output()
            .expect("run VM phi failure fixture");
        let mut outcomes = Vec::new();
        for optimize in [false, true] {
            let output = dir
                .path()
                .join(format!("phi_failure_o{}", u8::from(optimize)));
            build_native_with_options(&source_path, &output, false, optimize)
                .unwrap_or_else(|e| panic!("build phi failure native {optimize}: {e:#}"));
            outcomes.push((optimize, run_native(&output)));
        }

        // Assert
        assert_eq!(
            vm.status.code(),
            Some(1),
            "VM phi failure exit: {}",
            String::from_utf8_lossy(&vm.stderr)
        );
        assert_diagnostic(&vm.stderr, "E6001", &source_path, 2);
        for (optimize, native) in outcomes {
            assert_eq!(
                native.status.code(),
                Some(1),
                "native phi failure exit (optimize={optimize}): {}",
                String::from_utf8_lossy(&native.stderr)
            );
            assert_diagnostic(&native.stderr, "E6001", &source_path, 2);
        }
    }

    fn simple_value_module(
        ty: yaoxiang::frontend::core::sema::MonoType,
        duplicate: bool,
    ) -> ModuleIR {
        let mut instructions = vec![Instruction::Load {
            dst: Operand::Value(yaoxiang::middle::ir::ir::ValueId(0)),
            src: Operand::Const(ConstValue::Int(1)),
            span: Span::dummy(),
        }];
        if duplicate {
            instructions.push(Instruction::Load {
                dst: Operand::Value(yaoxiang::middle::ir::ir::ValueId(0)),
                src: Operand::Const(ConstValue::Int(2)),
                span: Span::dummy(),
            });
        }
        instructions.push(Instruction::Ret {
            value: Some(Operand::Value(yaoxiang::middle::ir::ir::ValueId(0))),
            span: Span::dummy(),
        });
        let function = FunctionIR {
            name: "value".to_string(),
            def: None,
            params: vec![],
            return_type: yaoxiang::frontend::core::sema::MonoType::Int(64),
            generic_params: None,
            body: FunctionBody::Code {
                blocks: vec![BasicBlock {
                    label: 0,
                    instructions,
                    successors: vec![],
                }],
                entry: 0,
                locals: vec![],
                values: vec![yaoxiang::middle::ir::ir::ValueInfo {
                    ty,
                    debug_name: None,
                    defining_block: 0,
                }],
                value_slots: vec![],
            },
        };
        let mut module = ModuleIR::default();
        module.functions.push(function);
        module.init.push(Instruction::Call {
            dst: None,
            func: Operand::Const(ConstValue::String("value".to_string())),
            args: vec![],
            def: None,
            span: Span::dummy(),
        });
        module
    }

    fn phi_module() -> ModuleIR {
        let function = FunctionIR {
            name: "phi".to_string(),
            def: None,
            params: vec![],
            return_type: yaoxiang::frontend::core::sema::MonoType::Int(64),
            generic_params: None,
            body: FunctionBody::Code {
                blocks: vec![
                    BasicBlock {
                        label: 0,
                        instructions: vec![Instruction::Jmp {
                            target: 1,
                            span: Span::dummy(),
                        }],
                        successors: vec![1],
                    },
                    BasicBlock {
                        label: 1,
                        instructions: vec![
                            Instruction::Phi {
                                dst: Operand::Value(yaoxiang::middle::ir::ir::ValueId(0)),
                                incoming: vec![(0, Operand::Const(ConstValue::Int(1)))],
                                span: Span::dummy(),
                            },
                            Instruction::Ret {
                                value: Some(Operand::Value(yaoxiang::middle::ir::ir::ValueId(0))),
                                span: Span::dummy(),
                            },
                        ],
                        successors: vec![],
                    },
                ],
                entry: 0,
                locals: vec![],
                values: vec![yaoxiang::middle::ir::ir::ValueInfo {
                    ty: yaoxiang::frontend::core::sema::MonoType::Int(64),
                    debug_name: None,
                    defining_block: 1,
                }],
                value_slots: vec![],
            },
        };
        let mut module = ModuleIR::default();
        module.functions.push(function);
        module.init.push(Instruction::Call {
            dst: None,
            func: Operand::Const(ConstValue::String("phi".to_string())),
            args: vec![],
            def: None,
            span: Span::dummy(),
        });
        module
    }

    fn compile_error(module: &ModuleIR) -> String {
        let dir = TempDir::new().expect("synthetic error tempdir");
        let output = dir.path().join("rejected");
        let source = dir.path().join("rejected.yx");
        compile_ir_to_executable(module, &source.display().to_string(), &output, false)
            .expect_err("synthetic IR must be rejected")
            .to_string()
    }

    #[test]
    fn test_native_rejected_synthetic_ir_preserves_previous_output() {
        // Arrange：先放一个上次成功的产物；不支持类型的合成 IR 仍须在分析层被拒，
        // 且失败构建不得覆盖旧产物——能力扩展不放宽失败语义。
        let dir = TempDir::new().expect("synthetic preserve tempdir");
        let output = dir.path().join("preserved");
        let source = dir.path().join("preserved.yx");
        std::fs::write(&output, b"previous-good-output").expect("seed output");
        let module =
            simple_value_module(yaoxiang::frontend::core::sema::MonoType::Float(64), false);

        // Act
        let error =
            compile_ir_to_executable(&module, &source.display().to_string(), &output, false)
                .expect_err("unsupported value type must be rejected");

        // Assert
        assert!(
            error.to_string().contains("ValueId(0)"),
            "rejection must name the unsupported value: {error:#}"
        );
        assert_eq!(
            std::fs::read(&output).expect("read preserved output"),
            b"previous-good-output",
            "rejected build must not overwrite previous output"
        );
    }

    /// 活的合并值 + 显式未知 incoming：`ssa::materialize_undef` 的 `Load Void`
    /// 作为一个前驱的 Phi incoming，Phi 的 dst 被返回（可观察）。
    fn live_unknown_incoming_phi_module() -> ModuleIR {
        let function = FunctionIR {
            name: "live_unknown_phi".to_string(),
            def: None,
            params: vec![],
            return_type: yaoxiang::frontend::core::sema::MonoType::Int(64),
            generic_params: None,
            body: FunctionBody::Code {
                blocks: vec![
                    BasicBlock {
                        label: 0,
                        instructions: vec![
                            Instruction::Load {
                                dst: Operand::Value(yaoxiang::middle::ir::ir::ValueId(0)),
                                src: Operand::Const(ConstValue::Void),
                                span: Span::dummy(),
                            },
                            Instruction::Jmp {
                                target: 2,
                                span: Span::dummy(),
                            },
                        ],
                        successors: vec![1],
                    },
                    BasicBlock {
                        label: 1,
                        instructions: vec![
                            Instruction::Phi {
                                dst: Operand::Value(yaoxiang::middle::ir::ir::ValueId(1)),
                                incoming: vec![(
                                    0,
                                    Operand::Value(yaoxiang::middle::ir::ir::ValueId(0)),
                                )],
                                span: Span::new(Position::new(5, 13), Position::new(5, 14)),
                            },
                            Instruction::Ret {
                                value: Some(Operand::Value(yaoxiang::middle::ir::ir::ValueId(1))),
                                span: Span::dummy(),
                            },
                        ],
                        successors: vec![],
                    },
                ],
                entry: 0,
                locals: vec![],
                values: vec![
                    yaoxiang::middle::ir::ir::ValueInfo {
                        ty: yaoxiang::frontend::core::sema::MonoType::TypeRef(
                            yaoxiang::frontend::core::types::identity::Nominal::flat("?")
                                .expect("'?' is a valid bare type name"),
                        ),
                        debug_name: None,
                        defining_block: 0,
                    },
                    yaoxiang::middle::ir::ir::ValueInfo {
                        ty: yaoxiang::frontend::core::sema::MonoType::Int(64),
                        debug_name: None,
                        defining_block: 1,
                    },
                ],
                value_slots: vec![],
            },
        };
        let mut module = ModuleIR::default();
        module.functions.push(function);
        module.init.push(Instruction::Call {
            dst: None,
            func: Operand::Const(ConstValue::String("live_unknown_phi".to_string())),
            args: vec![],
            def: None,
            span: Span::dummy(),
        });
        module
    }

    #[test]
    fn test_native_live_unknown_incoming_phi_is_rejected_with_location() {
        // Arrange：live Phi + 显式未知 incoming（materialize_undef 的 Load Void）——
        // undef 会成为可观察的未定义值，必须在分析层带位置拒绝，不产生产物。
        let dir = TempDir::new().expect("live unknown phi tempdir");
        let source = dir.path().join("live_unknown.yx");
        let output = dir.path().join("live_unknown");
        let module = live_unknown_incoming_phi_module();

        // Act
        let error =
            compile_ir_to_executable(&module, &source.display().to_string(), &output, false)
                .expect_err("a live Phi merging an explicit unknown incoming must be rejected");

        // Assert
        let detail = format!("{error:#}");
        assert!(
            detail.contains("native AOT unsupported input")
                && detail.contains(&format!("{}:5:13", source.display())),
            "diagnostic must carry the source position: {detail}"
        );
        assert!(
            detail.contains("explicitly unknown")
                && detail.contains("Void")
                && detail.contains("undef"),
            "diagnostic must explain the observability boundary: {detail}"
        );
        assert!(
            !output.exists(),
            "rejected build must not create a native artifact"
        );
    }

    #[test]
    fn test_native_explicit_unknown_value_is_rejected() {
        // Arrange
        let module = simple_value_module(
            yaoxiang::frontend::core::sema::MonoType::TypeRef(
                yaoxiang::frontend::core::types::identity::Nominal::flat("?")
                    .expect("'?' is a valid bare type name"),
            ),
            false,
        );

        // Act
        let detail = compile_error(&module);

        // Assert
        assert!(
            detail.contains("ValueId(0)") && detail.contains("TypeRef(\"?\")"),
            "unknown value diagnostic must name id/type: {detail}"
        );
    }

    #[test]
    fn test_native_unsupported_value_type_is_rejected() {
        // Arrange：容器类型仍在支持集外（Float 自批次 4.5 起已是标量支持集成员，
        // 不能再当「不支持类型」的样例）。
        let module = simple_value_module(
            yaoxiang::frontend::core::sema::MonoType::Generic {
                name: yaoxiang::frontend::core::types::identity::Nominal::flat("List")
                    .expect("List is a valid bare type name"),
                args: vec![yaoxiang::frontend::core::sema::MonoType::Int(64)],
            },
            false,
        );

        // Act
        let detail = compile_error(&module);

        // Assert
        assert!(
            detail.contains("ValueId(0)") && detail.contains("List"),
            "unsupported value diagnostic must name id/type: {detail}"
        );
    }

    #[test]
    fn test_native_multi_definition_value_is_rejected() {
        // Arrange
        let module = simple_value_module(yaoxiang::frontend::core::sema::MonoType::Int(64), true);

        // Act
        let detail = compile_error(&module);

        // Assert
        assert!(
            detail.contains("verification"),
            "multi-definition must fail SSA entry verification: {detail}"
        );
    }

    #[test]
    fn test_native_phi_module_builds_and_runs() {
        // Arrange：合成 IR 的单前驱 Phi（批次 3.5 前被 AOT-3.5 边界拒绝）。
        // O0 与 O2 都跑：优化器会重排汇合边，判据不能只钉在 O0。
        let module = phi_module();

        // Act & Assert
        for optimize in [false, true] {
            let (_dir, executable) = build_synthetic(&module, "phi", optimize);
            let output = run_native(&executable);
            assert!(
                output.status.success(),
                "synthetic Phi module should build and run (O{}): {}",
                u8::from(optimize),
                String::from_utf8_lossy(&output.stderr)
            );
        }
    }

    /// 合成 IR：菱形 CFG 的 String Phi。两个前驱各载入内容相同的字面量，汇合块用
    /// `Phi` 取回并与第三个同内容字面量比较。三处字面量在 LLVM 里是三个不同的
    /// 全局，因此这条判据钉住「内容相等，不是指针相等」（批次 4.1）。
    fn string_phi_module() -> ModuleIR {
        let function = FunctionIR {
            name: "string_phi".to_string(),
            def: None,
            params: vec![yaoxiang::frontend::core::sema::MonoType::Bool],
            return_type: yaoxiang::frontend::core::sema::MonoType::Void,
            generic_params: None,
            body: FunctionBody::Code {
                blocks: vec![
                    BasicBlock {
                        label: 0,
                        instructions: vec![Instruction::JmpIfNot {
                            cond: Operand::Arg(0),
                            target: 3,
                            span: Span::dummy(),
                        }],
                        successors: vec![1, 2],
                    },
                    BasicBlock {
                        label: 1,
                        instructions: vec![
                            Instruction::Load {
                                dst: Operand::Value(yaoxiang::middle::ir::ir::ValueId(0)),
                                src: Operand::Const(ConstValue::String("same".to_string())),
                                span: Span::dummy(),
                            },
                            Instruction::Jmp {
                                target: 5,
                                span: Span::dummy(),
                            },
                        ],
                        successors: vec![3],
                    },
                    BasicBlock {
                        label: 2,
                        instructions: vec![
                            Instruction::Load {
                                dst: Operand::Value(yaoxiang::middle::ir::ir::ValueId(1)),
                                src: Operand::Const(ConstValue::String("same".to_string())),
                                span: Span::dummy(),
                            },
                            Instruction::Jmp {
                                target: 5,
                                span: Span::dummy(),
                            },
                        ],
                        successors: vec![3],
                    },
                    BasicBlock {
                        label: 3,
                        instructions: vec![
                            Instruction::Phi {
                                dst: Operand::Value(yaoxiang::middle::ir::ir::ValueId(2)),
                                incoming: vec![
                                    (1, Operand::Value(yaoxiang::middle::ir::ir::ValueId(0))),
                                    (2, Operand::Value(yaoxiang::middle::ir::ir::ValueId(1))),
                                ],
                                span: Span::dummy(),
                            },
                            Instruction::Load {
                                dst: Operand::Value(yaoxiang::middle::ir::ir::ValueId(3)),
                                src: Operand::Const(ConstValue::String("same".to_string())),
                                span: Span::dummy(),
                            },
                            Instruction::Eq {
                                dst: Operand::Value(yaoxiang::middle::ir::ir::ValueId(4)),
                                lhs: Operand::Value(yaoxiang::middle::ir::ir::ValueId(2)),
                                rhs: Operand::Value(yaoxiang::middle::ir::ir::ValueId(3)),
                                span: Span::dummy(),
                            },
                            Instruction::Call {
                                dst: None,
                                func: Operand::Const(ConstValue::String(
                                    "std.assert.assert".to_string(),
                                )),
                                args: vec![Operand::Value(yaoxiang::middle::ir::ir::ValueId(4))],
                                def: None,
                                span: Span::dummy(),
                            },
                            Instruction::Ret {
                                value: None,
                                span: Span::dummy(),
                            },
                        ],
                        successors: vec![],
                    },
                ],
                entry: 0,
                locals: vec![],
                values: vec![
                    string_value_info(1),
                    string_value_info(2),
                    string_value_info(3),
                    string_value_info(3),
                    bool_value_info(3),
                ],
                value_slots: vec![],
            },
        };
        let mut module = ModuleIR::default();
        module.functions.push(function);
        for arm in [true, false] {
            module.init.push(Instruction::Call {
                dst: None,
                func: Operand::Const(ConstValue::String("string_phi".to_string())),
                args: vec![Operand::Const(ConstValue::Bool(arm))],
                def: None,
                span: Span::dummy(),
            });
        }
        module
    }

    fn string_value_info(block: usize) -> yaoxiang::middle::ir::ir::ValueInfo {
        yaoxiang::middle::ir::ir::ValueInfo {
            ty: yaoxiang::frontend::core::sema::MonoType::Generic {
                name: yaoxiang::frontend::core::types::identity::Nominal::flat("String")
                    .expect("String is a valid bare type name"),
                args: Vec::new(),
            },
            debug_name: None,
            defining_block: block,
        }
    }

    fn bool_value_info(block: usize) -> yaoxiang::middle::ir::ir::ValueInfo {
        yaoxiang::middle::ir::ir::ValueInfo {
            ty: yaoxiang::frontend::core::sema::MonoType::Bool,
            debug_name: None,
            defining_block: block,
        }
    }

    #[test]
    fn test_native_string_equality_matches_vm() {
        // Arrange：判等按内容比较——独立字面量、经形参、空串，以及同长度同前缀
        // 但后位不同的用例。静态字面量在目标常量池里会合并成同一个地址，因此
        // 「内容相等而指针不等」在本片还不可观察（后续 owned 字符串切片才出现
        // 两个地址）；可观察的内容判据是同长度不同字节的用例。
        let source = concat!(
            "use std.assert\n",
            "same: (s: String) -> Bool = (s) => s == \"same\"\n",
            "differ: (s: String) -> Bool = (s) => s != \"same\"\n",
            "differ_shared: (s: String) -> Bool = (s) => s != \"sAme\"\n",
            "empty: (s: String) -> Bool = (s) => s == \"\"\n",
            "main: () -> Void = {\n",
            "    a = \"same\"\n",
            "    b = \"same\"\n",
            "    assert.assert(a == b, \"two independent literals with identical content compare equal\")\n",
            "    assert.assert(same(a), \"parameter carries the same content\")\n",
            "    assert.assert(differ(\"other\"), \"different content is not equal\")\n",
            "    assert.assert(differ_shared(\"same\"), \"same length and shared first byte still differ\")\n",
            "    assert.assert(empty(\"\"), \"empty strings are equal\")\n",
            "    assert.assert(differ(\"\"), \"empty differs from the non-empty literal\")\n",
            "}\n",
            "main()\n",
        );

        // Act
        assert_native_matches_vm(source, "string_equality");

        // Assert
        // assert_native_matches_vm 已断言 native/VM 的 O0 与 O2 结果一致。
    }

    #[test]
    fn test_native_string_ordering_matches_vm() {
        // Arrange：六个算子的顺序语义——前缀、多字节、空串、大小写都是字节序。
        let source = concat!(
            "use std.assert\n",
            "lt: (a: String, b: String) -> Bool = (a, b) => a < b\n",
            "le: (a: String, b: String) -> Bool = (a, b) => a <= b\n",
            "gt: (a: String, b: String) -> Bool = (a, b) => a > b\n",
            "ge: (a: String, b: String) -> Bool = (a, b) => a >= b\n",
            "main: () -> Void = {\n",
            "    assert.assert(lt(\"abc\", \"abd\"), \"same prefix compares the next byte\")\n",
            "    assert.assert(lt(\"a\", \"ab\"), \"a prefix is smaller than its extension\")\n",
            "    assert.assert(gt(\"ab\", \"a\"), \"an extension is greater than its prefix\")\n",
            "    assert.assert(lt(\"\", \"a\"), \"the empty string is the smallest\")\n",
            "    assert.assert(le(\"abc\", \"abc\"), \"equal content is less-or-equal\")\n",
            "    assert.assert(ge(\"abc\", \"abc\"), \"equal content is greater-or-equal\")\n",
            "    assert.assert(gt(\"é\", \"z\"), \"multibyte compares by UTF-8 bytes\")\n",
            "    assert.assert(lt(\"Z\", \"a\"), \"uppercase bytes precede lowercase\")\n",
            "}\n",
            "main()\n",
        );

        // Act
        assert_native_matches_vm(source, "string_ordering");

        // Assert
        // assert_native_matches_vm 已断言 native/VM 的 O0 与 O2 结果一致。
    }

    #[test]
    fn test_native_string_compare_through_return_matches_vm() {
        // Arrange：String 经返回值与 if 值汇合后再比较，来源不是字面量。
        let source = concat!(
            "use std.assert\n",
            "pick: (c: Bool) -> String = (c) => {\n",
            "    x = if c { \"yes\" } else { \"no\" }\n",
            "    x\n",
            "}\n",
            "main: () -> Void = {\n",
            "    assert.assert(pick(true) == \"yes\", \"if true merges yes\")\n",
            "    assert.assert(pick(false) != \"yes\", \"if false merges no\")\n",
            "    assert.assert(pick(false) == \"no\", \"merged value compares by content\")\n",
            "}\n",
            "main()\n",
        );

        // Act
        assert_native_matches_vm(source, "string_return");

        // Assert
        // assert_native_matches_vm 已断言 native/VM 的 O0 与 O2 结果一致。
    }

    #[test]
    fn test_native_string_phi_merge_compares_by_content() {
        // Arrange：菱形 CFG 的 String Phi；合并值与第三个同内容字面量比较。
        // O0 与 O2 都跑：该判据不能只在 O0 单点成立。
        let module = string_phi_module();

        // Act & Assert
        for optimize in [false, true] {
            let (_dir, executable) = build_synthetic(&module, "string_phi", optimize);
            let output = run_native(&executable);
            assert!(
                output.status.success(),
                "String Phi merged value must compare equal by content (O{}): {}",
                u8::from(optimize),
                String::from_utf8_lossy(&output.stderr)
            );
        }
    }

    #[test]
    fn test_native_string_operation_outside_slice_is_rejected() {
        // Arrange：`s[i]`（StringGetChar）不在批次 4.1 的支持集内，必须带位置拒绝。
        let dir = TempDir::new().expect("string slice tempdir");
        let source = dir.path().join("string_index.yx");
        let executable = dir.path().join("string_index");
        std::fs::write(
            &source,
            concat!(
                "use std.assert\n",
                "main: () -> Void = {\n",
                "    s = \"abc\"\n",
                "    c = s[0]\n",
                "    assert.assert(c == \"a\")\n",
                "}\n",
                "main()\n",
            ),
        )
        .expect("write unsupported string op fixture");

        // Act
        let error = build_native_with_options(&source, &executable, false, false)
            .expect_err("StringGetChar stays outside the batch 4.1 support set");

        // Assert
        let detail = format!("{error:#}");
        assert!(
            detail.contains(&format!("{}:4:", source.display())),
            "rejection must carry the string index position: {detail}"
        );
        assert!(
            detail.contains("StringGetChar"),
            "rejection must identify the unsupported instruction: {detail}"
        );
        assert!(
            !executable.exists(),
            "rejected build must not write a native artifact"
        );
    }

    #[test]
    fn test_native_int_bitwise_matches_vm() {
        // Arrange：And/Or/Xor 覆盖负数、大数、0 与 -1（二进制补码按位语义）。
        let source = concat!(
            "use std.assert\n",
            "band: (a: Int, b: Int) -> Int = (a, b) => a & b\n",
            "bor: (a: Int, b: Int) -> Int = (a, b) => a | b\n",
            "bxor: (a: Int, b: Int) -> Int = (a, b) => a ^ b\n",
            "main: () -> Void = {\n",
            "    assert.assert(band(6, 2) == 2, \"and keeps the shared bits\")\n",
            "    assert.assert(bor(6, 2) == 6, \"or unions the bits\")\n",
            "    assert.assert(bxor(6, 2) == 4, \"xor keeps the differing bits\")\n",
            "    assert.assert(band(-1, 255) == 255, \"and with -1 keeps the low byte\")\n",
            "    assert.assert(bor(0, -1) == -1, \"or with -1 fills every bit\")\n",
            "    assert.assert(bxor(-1, 255) == -256, \"xor with -1 complements\")\n",
            "    assert.assert(band(-8, -4) == -8, \"and of two negatives\")\n",
            "    assert.assert(bor(-8, 7) == -1, \"or of two negatives fills every bit\")\n",
            "    assert.assert(bxor(0, 0) == 0, \"xor of zeros is zero\")\n",
            "    assert.assert(band(4611686018427387904, 4611686018427387904) == 4611686018427387904, \"and of a large power of two\")\n",
            "    assert.assert(bxor(4611686018427387904, 1) == 4611686018427387905, \"xor keeps the large bit\")\n",
            "}\n",
            "main()\n",
        );

        // Act
        assert_native_matches_vm(source, "int_bitwise");

        // Assert
        // assert_native_matches_vm 已断言 native/VM 的 O0 与 O2 结果一致。
    }

    #[test]
    fn test_native_shift_left_matches_vm() {
        // Arrange：左移覆盖 0/1/63 位移量与负数 lhs；值溢出不是错误（位被移出）。
        let source = concat!(
            "use std.assert\n",
            "shl: (a: Int, n: Int) -> Int = (a, n) => a << n\n",
            "shr: (a: Int, n: Int) -> Int = (a, n) => a >> n\n",
            "main: () -> Void = {\n",
            "    assert.assert(shl(1, 0) == 1, \"shift by zero keeps the value\")\n",
            "    assert.assert(shl(1, 1) == 2, \"shift by one doubles\")\n",
            "    assert.assert(shl(1, 63) < 0, \"the top bit becomes the sign bit\")\n",
            "    assert.assert(shr(shl(1, 63), 63) == -1, \"arithmetic shift sees the sign bit\")\n",
            "    assert.assert(shl(-1, 1) == -2, \"negative lhs shifts its bits\")\n",
            "    assert.assert(shl(-5, 2) == -20, \"negative lhs by two\")\n",
            "    assert.assert(shl(12344, 63) == 0, \"an even lhs loses every bit at 63\")\n",
            "    assert.assert(shl(0, 63) == 0, \"zero stays zero\")\n",
            "}\n",
            "main()\n",
        );

        // Act
        assert_native_matches_vm(source, "shift_left");

        // Assert
        // assert_native_matches_vm 已断言 native/VM 的 O0 与 O2 结果一致。
    }

    #[test]
    fn test_native_shift_right_matches_vm() {
        // Arrange：右移覆盖 0/1/63 位移量与负数 lhs。`>>` 是有符号算术右移：
        // 前端降到 `Sar`，AOT 发 `ashr`，负数的符号位保持。
        let source = concat!(
            "use std.assert\n",
            "shr: (a: Int, n: Int) -> Int = (a, n) => a >> n\n",
            "min_value: () -> Int = () => 0 - 9223372036854775807 - 1\n",
            "main: () -> Void = {\n",
            "    assert.assert(shr(1024, 0) == 1024, \"shift by zero keeps the value\")\n",
            "    assert.assert(shr(1024, 1) == 512, \"positive lhs halves\")\n",
            "    assert.assert(shr(1024, 63) == 0, \"every set bit leaves at 63\")\n",
            "    assert.assert(shr(-8, 1) == -4, \"negative lhs keeps the sign bit\")\n",
            "    assert.assert(shr(-1, 63) == -1, \"negative one stays negative one\")\n",
            "    assert.assert(shr(min_value(), 63) == -1, \"the sign bit reaches every position\")\n",
            "}\n",
            "main()\n",
        );

        // Act
        assert_native_matches_vm(source, "shift_right");

        // Assert
        // assert_native_matches_vm 已断言 native/VM 的 O0 与 O2 结果一致。
    }

    #[test]
    fn test_native_shift_out_of_range_matches_vm_diagnostic() {
        // Arrange：位移量 64 与 -1 越界；VM 报 E6007 并带移位语句的源码位置，
        // AOT 必须同码同位置失败，而不是静默 wrap。
        for (amount, stem) in [("64", "shift_64"), ("-1", "shift_neg1")] {
            let dir = TempDir::new().expect("shift out of range tempdir");
            let source_path = dir.path().join(format!("{stem}.yx"));
            let o0_path = dir.path().join(format!("{stem}_o0"));
            let o2_path = dir.path().join(format!("{stem}_o2"));
            let source = concat!(
                "use std.assert\n",
                "shl: (a: Int, n: Int) -> Int = (a, n) => a << n\n",
                "main: () -> Void = {\n",
                "    assert.assert(shl(1, AMOUNT) == 0)\n",
                "}\n",
                "main()\n",
            )
            .replace("AMOUNT", amount);
            std::fs::write(&source_path, source).expect("write shift out of range fixture");

            // Act
            build_native_with_options(&source_path, &o0_path, false, false)
                .unwrap_or_else(|e| panic!("build {stem} O0: {e:#}"));
            build_native_with_options(&source_path, &o2_path, false, true)
                .unwrap_or_else(|e| panic!("build {stem} O2: {e:#}"));
            let vm = Command::new(test_yx_bin())
                .arg("run")
                .arg(&source_path)
                .output()
                .expect("run VM shift out of range fixture");

            // Assert
            assert_eq!(
                vm.status.code(),
                Some(1),
                "VM {stem} exit: {}",
                String::from_utf8_lossy(&vm.stderr)
            );
            assert_diagnostic(&vm.stderr, "E6007", &source_path, 3);
            let vm_line = diagnostic_line(&vm.stderr, &source_path)
                .unwrap_or_else(|| panic!("VM {stem} diagnostic must carry a line"));
            for (optimize, path) in [("O0", &o0_path), ("O2", &o2_path)] {
                let native = run_native(path);
                assert_eq!(
                    native.status.code(),
                    Some(1),
                    "native {optimize} {stem} exit: {}",
                    String::from_utf8_lossy(&native.stderr)
                );
                // 位置必须与 VM 逐项一致：诊断位置来自前端 span，不由后端另定。
                assert_diagnostic(&native.stderr, "E6007", &source_path, vm_line);
            }
        }
    }

    #[test]
    fn test_native_shift_amount_truncates_like_vm() {
        // Arrange：VM 以位移量的 u32 截断值判定范围，因此 2^32 等价于 0、
        // 2^32+1 等价于 1。AOT 必须按同一口径发射，不能直接拿 i64 与 64 比较。
        let source = concat!(
            "use std.assert\n",
            "shr: (a: Int, n: Int) -> Int = (a, n) => a >> n\n",
            "main: () -> Void = {\n",
            "    assert.assert(shr(1024, 4294967296) == 1024, \"amount 2^32 behaves like 0\")\n",
            "    assert.assert(shr(1024, 4294967297) == 512, \"amount 2^32+1 behaves like 1\")\n",
            "}\n",
            "main()\n",
        );

        // Act
        assert_native_matches_vm(source, "shift_amount_truncate");

        // Assert
        // assert_native_matches_vm 已断言 native/VM 的 O0 与 O2 结果一致。
    }

    /// 合成 IR：单条右移指令 + 与 `expected` 的相等断言，不通过就以 E6005 退出。
    /// `arithmetic` 选变体：`true` = `Sar`（`>>` 的 lowering 目标，算术右移），
    /// `false` = `Shr`（逻辑右移，前端今天不产出）。两条变体对同一负数必须给出
    /// 不同结果，这是本判据的区分点。
    fn right_shift_probe_module(
        arithmetic: bool,
        expected: i128,
    ) -> ModuleIR {
        let target = if arithmetic { "sar_probe" } else { "shr_probe" };
        let shift = if arithmetic {
            Instruction::Sar {
                dst: Operand::Value(yaoxiang::middle::ir::ir::ValueId(0)),
                lhs: Operand::Const(ConstValue::Int(-8)),
                rhs: Operand::Const(ConstValue::Int(1)),
                span: Span::dummy(),
            }
        } else {
            Instruction::Shr {
                dst: Operand::Value(yaoxiang::middle::ir::ir::ValueId(0)),
                lhs: Operand::Const(ConstValue::Int(-8)),
                rhs: Operand::Const(ConstValue::Int(1)),
                span: Span::dummy(),
            }
        };
        let function = FunctionIR {
            name: target.to_string(),
            def: None,
            params: vec![],
            return_type: yaoxiang::frontend::core::sema::MonoType::Void,
            generic_params: None,
            body: FunctionBody::Code {
                blocks: vec![BasicBlock {
                    label: 0,
                    instructions: vec![
                        shift,
                        Instruction::Eq {
                            dst: Operand::Value(yaoxiang::middle::ir::ir::ValueId(1)),
                            lhs: Operand::Value(yaoxiang::middle::ir::ir::ValueId(0)),
                            rhs: Operand::Const(ConstValue::Int(expected)),
                            span: Span::dummy(),
                        },
                        Instruction::Call {
                            dst: None,
                            func: Operand::Const(ConstValue::String(
                                "std.assert.assert".to_string(),
                            )),
                            args: vec![Operand::Value(yaoxiang::middle::ir::ir::ValueId(1))],
                            def: None,
                            span: Span::dummy(),
                        },
                        Instruction::Ret {
                            value: None,
                            span: Span::dummy(),
                        },
                    ],
                    successors: vec![],
                }],
                entry: 0,
                locals: vec![],
                values: vec![
                    yaoxiang::middle::ir::ir::ValueInfo {
                        ty: yaoxiang::frontend::core::sema::MonoType::Int(64),
                        debug_name: None,
                        defining_block: 0,
                    },
                    yaoxiang::middle::ir::ir::ValueInfo {
                        ty: yaoxiang::frontend::core::sema::MonoType::Bool,
                        debug_name: None,
                        defining_block: 0,
                    },
                ],
                value_slots: vec![],
            },
        };
        let mut module = ModuleIR::default();
        module.functions.push(function);
        module.init.push(Instruction::Call {
            dst: None,
            func: Operand::Const(ConstValue::String(target.to_string())),
            args: vec![],
            def: None,
            span: Span::dummy(),
        });
        module
    }

    fn sar_module() -> ModuleIR {
        right_shift_probe_module(true, -4)
    }

    fn shr_module() -> ModuleIR {
        right_shift_probe_module(false, 9_223_372_036_854_775_804)
    }

    #[test]
    fn test_native_sar_instruction_uses_arithmetic_shift() {
        // Arrange：合成 IR 固定 Sar 变体；O0 与 O2 都跑，受检移位的守卫在优化下
        // 也不许退化成静默 wrap。
        let module = sar_module();

        // Act & Assert
        for optimize in [false, true] {
            let (_dir, executable) = build_synthetic(&module, "sar_probe", optimize);
            let output = run_native(&executable);
            assert!(
                output.status.success(),
                "Sar must emit the arithmetic right shift (-8 >> 1 == -4) (O{}): {}",
                u8::from(optimize),
                String::from_utf8_lossy(&output.stderr)
            );
        }
    }

    #[test]
    fn test_native_shr_instruction_uses_logical_shift() {
        // Arrange：合成 IR 固定 Shr 变体（前端不产出），期望逻辑右移的高位补零
        // 结果 0x7FFF_FFFF_FFFF_FFFC；若发射成算术右移会得到 -4 而断言失败。
        let module = shr_module();

        // Act & Assert
        for optimize in [false, true] {
            let (_dir, executable) = build_synthetic(&module, "shr_probe", optimize);
            let output = run_native(&executable);
            assert!(
                output.status.success(),
                "Shr must emit the logical right shift (-8 >>> 1 == 0x7FFF_FFFF_FFFF_FFFC) (O{}): {}",
                u8::from(optimize),
                String::from_utf8_lossy(&output.stderr)
            );
        }
    }

    /// 合成 IR：Void 全局槽位（模块声明）与 Void 局部槽位的读取。Void 没有 LLVM 形态，
    /// 读取按类型物化为 Void——判据是「不依赖缺指针兜底」的那条显式规则。
    fn void_slots_module() -> ModuleIR {
        let mut module = ModuleIR::default();
        module.globals.push(GlobalSlot {
            name: "g".to_string(),
            ty: yaoxiang::frontend::core::sema::MonoType::Void,
            index: 0,
        });
        module.functions.push(FunctionIR {
            name: "void_probe".to_string(),
            def: None,
            params: vec![],
            return_type: yaoxiang::frontend::core::sema::MonoType::Void,
            generic_params: None,
            body: FunctionBody::Code {
                blocks: vec![BasicBlock {
                    label: 0,
                    instructions: vec![
                        Instruction::Store {
                            dst: Operand::Global(0),
                            src: Operand::Const(ConstValue::Void),
                            span: Span::dummy(),
                        },
                        Instruction::Load {
                            dst: Operand::Local(0),
                            src: Operand::Const(ConstValue::Void),
                            span: Span::dummy(),
                        },
                        // 读 Void 全局槽位（旧实现在这里静默返回 Void，现按类型规则物化）。
                        Instruction::Move {
                            dst: Operand::Local(1),
                            src: Operand::Global(0),
                            span: Span::dummy(),
                        },
                        // 读 Void 局部槽位（旧实现的 load_slot 兜底）。
                        Instruction::Move {
                            dst: Operand::Local(2),
                            src: Operand::Local(0),
                            span: Span::dummy(),
                        },
                        Instruction::Ret {
                            value: None,
                            span: Span::dummy(),
                        },
                    ],
                    successors: vec![],
                }],
                entry: 0,
                locals: vec![
                    LocalSlot::temp(yaoxiang::frontend::core::sema::MonoType::Void),
                    LocalSlot::temp(yaoxiang::frontend::core::sema::MonoType::Void),
                    LocalSlot::temp(yaoxiang::frontend::core::sema::MonoType::Void),
                ],
                values: vec![],
                value_slots: vec![],
            },
        });
        module.init.push(Instruction::Call {
            dst: None,
            func: Operand::Const(ConstValue::String("void_probe".to_string())),
            args: vec![],
            def: None,
            span: Span::dummy(),
        });
        module
    }

    /// 合成 IR：已声明 Int 全局槽位的读取（非 Void 槽位必须由 declare_globals 物化）。
    fn int_global_module() -> ModuleIR {
        let mut module = ModuleIR::default();
        module.globals.push(GlobalSlot {
            name: "g".to_string(),
            ty: yaoxiang::frontend::core::sema::MonoType::Int(64),
            index: 0,
        });
        module.functions.push(FunctionIR {
            name: "int_global_probe".to_string(),
            def: None,
            params: vec![],
            return_type: yaoxiang::frontend::core::sema::MonoType::Void,
            generic_params: None,
            body: FunctionBody::Code {
                blocks: vec![BasicBlock {
                    label: 0,
                    instructions: vec![
                        Instruction::Load {
                            dst: Operand::Value(yaoxiang::middle::ir::ir::ValueId(0)),
                            src: Operand::Global(0),
                            span: Span::dummy(),
                        },
                        Instruction::Eq {
                            dst: Operand::Value(yaoxiang::middle::ir::ir::ValueId(1)),
                            lhs: Operand::Value(yaoxiang::middle::ir::ir::ValueId(0)),
                            rhs: Operand::Const(ConstValue::Int(7)),
                            span: Span::dummy(),
                        },
                        Instruction::Call {
                            dst: None,
                            func: Operand::Const(ConstValue::String(
                                "std.assert.assert".to_string(),
                            )),
                            args: vec![Operand::Value(yaoxiang::middle::ir::ir::ValueId(1))],
                            def: None,
                            span: Span::dummy(),
                        },
                        Instruction::Ret {
                            value: None,
                            span: Span::dummy(),
                        },
                    ],
                    successors: vec![],
                }],
                entry: 0,
                locals: vec![],
                values: vec![
                    yaoxiang::middle::ir::ir::ValueInfo {
                        ty: yaoxiang::frontend::core::sema::MonoType::Int(64),
                        debug_name: None,
                        defining_block: 0,
                    },
                    yaoxiang::middle::ir::ir::ValueInfo {
                        ty: yaoxiang::frontend::core::sema::MonoType::Bool,
                        debug_name: None,
                        defining_block: 0,
                    },
                ],
                value_slots: vec![],
            },
        });
        module.init.push(Instruction::Store {
            dst: Operand::Global(0),
            src: Operand::Const(ConstValue::Int(7)),
            span: Span::dummy(),
        });
        module.init.push(Instruction::Call {
            dst: None,
            func: Operand::Const(ConstValue::String("int_global_probe".to_string())),
            args: vec![],
            def: None,
            span: Span::dummy(),
        });
        module
    }

    #[test]
    fn test_native_void_slot_reads_are_explicit() {
        // Arrange：Void 全局与 Void 局部各读一次；两处都不再有「缺指针 → Void」兜底。
        let module = void_slots_module();

        // Act & Assert
        for optimize in [false, true] {
            let (_dir, executable) = build_synthetic(&module, "void_slots", optimize);
            let output = run_native(&executable);
            assert!(
                output.status.success(),
                "Void slot reads must build and run (O{}): {}",
                u8::from(optimize),
                String::from_utf8_lossy(&output.stderr)
            );
        }
    }

    #[test]
    fn test_native_declared_int_global_read_is_materialized() {
        // Arrange：已声明 Int 全局由 declare_globals 物化指针，读取必须拿到值而非 Void。
        let module = int_global_module();

        // Act & Assert
        for optimize in [false, true] {
            let (_dir, executable) = build_synthetic(&module, "int_global", optimize);
            let output = run_native(&executable);
            assert!(
                output.status.success(),
                "a declared Int global must read back its stored value (O{}): {}",
                u8::from(optimize),
                String::from_utf8_lossy(&output.stderr)
            );
        }
    }

    #[test]
    fn test_native_undeclared_global_read_is_rejected() {
        // Arrange：读一个模块从未声明的全局槽位——内部的 verify_ssa 闸门先按槽位
        // 越界点名拒绝，发射层收不到这条读取，也就不可能静默物化 Void。
        let mut module = ModuleIR::default();
        module.functions.push(FunctionIR {
            name: "bad_read".to_string(),
            def: None,
            params: vec![],
            return_type: yaoxiang::frontend::core::sema::MonoType::Void,
            generic_params: None,
            body: FunctionBody::Code {
                blocks: vec![BasicBlock {
                    label: 0,
                    instructions: vec![
                        Instruction::Load {
                            dst: Operand::Value(yaoxiang::middle::ir::ir::ValueId(0)),
                            src: Operand::Global(3),
                            span: Span::dummy(),
                        },
                        Instruction::Ret {
                            value: None,
                            span: Span::dummy(),
                        },
                    ],
                    successors: vec![],
                }],
                entry: 0,
                locals: vec![],
                values: vec![yaoxiang::middle::ir::ir::ValueInfo {
                    ty: yaoxiang::frontend::core::sema::MonoType::Int(64),
                    debug_name: None,
                    defining_block: 0,
                }],
                value_slots: vec![],
            },
        });
        module.init.push(Instruction::Call {
            dst: None,
            func: Operand::Const(ConstValue::String("bad_read".to_string())),
            args: vec![],
            def: None,
            span: Span::dummy(),
        });
        let dir = TempDir::new().expect("undeclared global tempdir");
        let source = dir.path().join("undeclared_global.yx");
        let executable = dir.path().join("undeclared_global");

        // Act
        let error =
            compile_ir_to_executable(&module, &source.display().to_string(), &executable, false)
                .expect_err("a read of an undeclared global slot must be rejected");

        // Assert
        let detail = format!("{error:#}");
        assert!(
            detail.contains("global 槽位 3"),
            "rejection must name the undeclared slot: {detail}"
        );
        assert!(
            !executable.exists(),
            "rejected build must not write a native artifact"
        );
    }

    #[test]
    fn test_native_embedded_nul_stdout_matches_vm() {
        // Arrange：字符串常量含**内嵌 NUL 字节**——`String` 是 `{ptr,len}` 借用视图，长度是
        // 真实字节数，输出按显式长度落字节（不追加 NUL、不以 NUL 为界）。
        // 注：含 NUL 的形态三种写法都可达——源码里的裸 NUL 字节，以及 `\\0` / `\\u{0}`
        // 两种转义拼写（前端 `decode_escape` 的 `0` 臂与 `u{...}` 解码都会解成真实 NUL
        // 字节并进入 IR），三种写法的字节由同一条 `{ptr,len}` 物化路径承载；这里用裸字节
        // 钉端到端契约（VM 与 AOT 输出同为 `61 00 62 0a`）。
        let source = concat!(
            "use std.io\n",
            "main: () -> Void = {\n",
            "    io.println(\"a\u{0}b\")\n",
            "    io.print(\"x\u{0}y\")\n",
            "    io.println(\"\")\n",
            "    io.println(\"\u{0}\")\n",
            "    io.println(\"h\\u{00e9}\u{0}z\\u{4e16}\")\n",
            "}\n",
            "main()\n",
        );

        // Act
        assert_native_matches_vm(source, "embedded_nul_stdout");

        // Assert
        // assert_native_matches_vm 已逐字节比对 VM 与 O0/O2 的 stdout：VM 侧 a<NUL>b 是
        // `61 00 62 0a`；常量若在 NUL 处截断会只剩 `61`，输出按 NUL 为界则整段消失。
    }

    #[test]
    fn test_native_embedded_nul_comparison_matches_vm() {
        // Arrange：含内嵌 NUL 的串参与判等与长度——比较按显式长度的内容，不看 NUL。
        let source = concat!(
            "use std.string\n",
            "use std.assert\n",
            "main: () -> Void = {\n",
            "    assert.assert(\"a\u{0}b\" != \"a\u{0}c\", \"differ after NUL\")\n",
            "    assert.assert(\"a\u{0}b\" == \"a\u{0}b\", \"equal with NUL\")\n",
            "    assert.assert(\"a\u{0}b\" != \"a\u{0}b\u{0}\", \"length matters\")\n",
            "    assert.assert(string.len(\"a\u{0}b\") == 3, \"NUL is one code point\")\n",
            "    assert.assert(\"\u{0}\" != \"\", \"NUL string is not empty\")\n",
            "}\n",
            "main()\n",
        );

        // Act
        assert_native_matches_vm(source, "embedded_nul_compare");

        // Assert
        // assert_native_matches_vm 已比对 VM 与 O0/O2 的退出码与 stdout。
    }

    #[test]
    fn test_native_println_stdout_matches_vm() {
        // Arrange：println 的 Int/Bool/String（含 UTF-8）显示、print 无换行、多次拼接。
        // 判据是 stdout 逐字节相等（assert_native_matches_vm 比对 VM 与 O0/O2）。
        let source = concat!(
            "use std.io\n",
            "main: () -> Void = {\n",
            "    io.println(42)\n",
            "    io.println(true)\n",
            "    io.println(\"h\u{00e9}llo \u{4e16}\u{754c}\")\n",
            "    io.print(\"a\")\n",
            "    io.print(\"b\")\n",
            "    io.println(\"\")\n",
            "    io.print(-7)\n",
            "    io.println(\"\")\n",
            "    io.println(false)\n",
            "}\n",
            "main()\n",
        );

        // Act
        assert_native_matches_vm(source, "stdout_println");

        // Assert
        // assert_native_matches_vm 已逐字节比对 VM 与 O0/O2 的 stdout 与退出码。
    }

    #[test]
    fn test_native_string_len_matches_vm() {
        // Arrange：`std.string.len` 是码点数口径（#385 D1/D2），不是 UTF-8 字节数；
        // 结果要能参与比较与断言的整型运算。
        let source = concat!(
            "use std.string\n",
            "use std.assert\n",
            "main: () -> Void = {\n",
            "    assert.assert(string.len(\"\") == 0, \"empty string\")\n",
            "    assert.assert(string.len(\"abc\") == 3, \"ascii\")\n",
            "    assert.assert(string.len(\"h\u{00e9}llo\") == 5, \"five code points, six bytes\")\n",
            "    assert.assert(string.len(\"\u{4e16}\u{754c}\") == 2, \"two code points\")\n",
            "    assert.assert(string.len(\"abcd\") - string.len(\"ab\") == 2, \"arithmetic\")\n",
            "    assert.assert(string.len(\"ab\") < string.len(\"abc\"), \"ordering\")\n",
            "}\n",
            "main()\n",
        );

        // Act
        assert_native_matches_vm(source, "string_len");

        // Assert
        // assert_native_matches_vm 已比对 VM 与 O0/O2 的退出码与 stdout。
    }

    #[test]
    fn test_native_std_call_outside_allowlist_is_rejected() {
        // Arrange：白名单外的 std 调用仍带位置拒绝，不静默 Void。
        let dir = TempDir::new().expect("std allowlist tempdir");
        let source = dir.path().join("outside.yx");
        let executable = dir.path().join("outside");
        std::fs::write(
            &source,
            concat!(
                "use std.string\n",
                "main: () -> Void = {\n",
                "    n = string.len(\"abc\")\n",
                "    chars = string.chars(\"abc\")\n",
                "}\n",
                "main()\n",
            ),
        )
        .expect("write std allowlist fixture");

        // Act
        let error = build_native_with_options(&source, &executable, false, false)
            .expect_err("std call outside the allowlist must be rejected at build time");

        // Assert
        let detail = format!("{error:#}");
        assert!(
            detail.contains("std.string.chars"),
            "rejection must name the unsupported std call: {detail}"
        );
        assert!(
            detail.contains(&format!("{}:4:", source.display())),
            "rejection must carry the call position: {detail}"
        );
        assert!(
            !executable.exists(),
            "rejected build must not write a native artifact"
        );
    }

    #[test]
    fn test_native_std_print_unsupported_arguments_are_rejected() {
        // Arrange：Float 实参与多参数（VM 用空格拼接）都在单参数 Int/Bool/String 之外。
        let fixtures: [(&str, &str, &str); 2] = [
            (
                "float_arg",
                concat!(
                    "use std.io\n",
                    "main: () -> Void = {\n",
                    "    io.println(1.5)\n",
                    "}\n",
                    "main()\n",
                ),
                "accepts",
            ),
            (
                "two_args",
                concat!(
                    "use std.io\n",
                    "main: () -> Void = {\n",
                    "    io.println(1, 2)\n",
                    "}\n",
                    "main()\n",
                ),
                "expects 1 argument(s), got 2",
            ),
        ];

        for (stem, program, expected) in fixtures {
            let dir = TempDir::new().expect("std print argument tempdir");
            let source = dir.path().join(format!("{stem}.yx"));
            let executable = dir.path().join(stem);
            std::fs::write(&source, program).expect("write std print argument fixture");

            // Act
            let error = build_native_with_options(&source, &executable, false, false)
                .expect_err("unsupported console argument must be rejected");

            // Assert
            let detail = format!("{error:#}");
            assert!(
                detail.contains("std.io.println") && detail.contains(expected),
                "rejection must name the console boundary ({expected}): {detail}"
            );
            assert!(
                detail.contains(&format!("{}:3:", source.display())),
                "rejection must carry the call position: {detail}"
            );
            assert!(
                !executable.exists(),
                "rejected build must not write a native artifact"
            );
        }
    }

    #[test]
    fn test_native_float_arithmetic_matches_vm() {
        // Arrange：四则、Neg、floor 取模（四个符号组合）、Float 局部/形参/返回与 Phi。
        let source = concat!(
            "use std.assert\n",
            "add: (a: Float, b: Float) -> Float = (a, b) => a + b\n",
            "sub: (a: Float, b: Float) -> Float = (a, b) => a - b\n",
            "mul: (a: Float, b: Float) -> Float = (a, b) => a * b\n",
            "div: (a: Float, b: Float) -> Float = (a, b) => a / b\n",
            "rem: (a: Float, b: Float) -> Float = (a, b) => a % b\n",
            "neg: (a: Float) -> Float = (a) => -a\n",
            "pick: (flag: Bool, a: Float, b: Float) -> Float = (flag, a, b) => { if flag { a } else { b } }\n",
            "scaled: (x: Float) -> Float = (x) => { y = x * 2.0; y + 0.5 }\n",
            "main: () -> Void = {\n",
            "    assert.assert(add(1.5, 2.25) == 3.75, \"fadd\")\n",
            "    assert.assert(sub(1.5, 2.25) == -0.75, \"fsub\")\n",
            "    assert.assert(mul(1.5, 2.0) == 3.0, \"fmul\")\n",
            "    assert.assert(div(7.0, 2.0) == 3.5, \"fdiv\")\n",
            "    assert.assert(neg(2.5) == -2.5, \"fneg\")\n",
            "    assert.assert(rem(7.5, 2.0) == 1.5, \"floor mod positive\")\n",
            "    assert.assert(rem(-7.5, 2.0) == 0.5, \"floor mod negative dividend\")\n",
            "    assert.assert(rem(7.5, -2.0) == -0.5, \"floor mod negative divisor\")\n",
            "    assert.assert(rem(-7.5, -2.0) == -1.5, \"floor mod both negative\")\n",
            "    assert.assert(scaled(1.25) == 3.0, \"float local\")\n",
            "    assert.assert(pick(true, 1.0, 2.0) == 1.0 and pick(false, 1.0, 2.0) == 2.0, \"float phi\")\n",
            "}\n",
            "main()\n",
        );

        // Act
        assert_native_matches_vm(source, "float_arith");

        // Assert
        // assert_native_matches_vm 已断言 VM 与 O0/O2 的退出码与 stdout 一致。
    }

    #[test]
    fn test_native_float_nan_and_infinity_match_vm() {
        // Arrange：NaN 与 ±Inf 的六种比较。VM 用 Rust f64 比较，NaN 与自身「不等于」
        // 且四种序比较全假——AOT 的谓词必须逐项对齐（Ne 用无序 UNE）。
        let source = concat!(
            "use std.assert\n",
            "eq: (a: Float, b: Float) -> Bool = (a, b) => a == b\n",
            "ne: (a: Float, b: Float) -> Bool = (a, b) => a != b\n",
            "lt: (a: Float, b: Float) -> Bool = (a, b) => a < b\n",
            "le: (a: Float, b: Float) -> Bool = (a, b) => a <= b\n",
            "gt: (a: Float, b: Float) -> Bool = (a, b) => a > b\n",
            "ge: (a: Float, b: Float) -> Bool = (a, b) => a >= b\n",
            "nan: () -> Float = () => 0.0 / 0.0\n",
            "inf: () -> Float = () => 1.0 / 0.0\n",
            "main: () -> Void = {\n",
            "    n = nan()\n",
            "    i = inf()\n",
            "    assert.assert(ne(n, n), \"NaN != NaN\")\n",
            "    assert.assert(!eq(n, n), \"not (NaN == NaN)\")\n",
            "    assert.assert(!lt(n, 1.0) and !le(n, 1.0) and !gt(n, 1.0) and !ge(n, 1.0), \"NaN ordered comparisons are false\")\n",
            "    assert.assert(eq(i, i) and ge(i, i) and le(i, i), \"inf == inf\")\n",
            "    assert.assert(gt(i, 1000000.0) and lt(-i, 0.0), \"infinity ordering\")\n",
            "    assert.assert(eq(1.0, 1.0) and le(1.0, 1.0) and ge(1.0, 1.0) and !lt(1.0, 1.0), \"finite comparisons\")\n",
            "}\n",
            "main()\n",
        );

        // Act
        assert_native_matches_vm(source, "float_nan_inf");

        // Assert
        // assert_native_matches_vm 已断言 VM 与 O0/O2 的退出码与 stdout 一致。
    }

    #[test]
    fn test_native_float_mixed_int_operands_are_rejected() {
        // Arrange：Int×Float 混合算术——VM 在 typecheck 层把 Int 提升成 f64，但 IR 不落
        // 显式 widening 指令，AOT 侧无法判出提升点，因此带位置拒绝而不是猜一种语义。
        let dir = TempDir::new().expect("mixed float tempdir");
        let source = dir.path().join("mixed_float.yx");
        let executable = dir.path().join("mixed_float");
        std::fs::write(
            &source,
            concat!(
                "use std.assert\n",
                "main: () -> Void = {\n",
                "    a = 1\n",
                "    b = 2.5\n",
                "    assert.assert(a + b == 3.5, \"mixed arithmetic\")\n",
                "}\n",
                "main()\n",
            ),
        )
        .expect("write mixed float fixture");

        // Act
        let error = build_native_with_options(&source, &executable, false, false)
            .expect_err("mixed Int/Float arithmetic must be rejected at AOT build time");

        // Assert
        let detail = format!("{error:#}");
        assert!(
            detail.contains("mixed Int/Float"),
            "rejection must name the mixed operand boundary: {detail}"
        );
        assert!(
            detail.contains(&format!("{}:5:", source.display())),
            "rejection must carry the arithmetic position: {detail}"
        );
        assert!(
            !executable.exists(),
            "rejected build must not write a native artifact"
        );
        let vm = Command::new(test_yx_bin())
            .arg("run")
            .arg(&source)
            .output()
            .expect("run VM mixed float fixture");
        assert_eq!(
            vm.status.code(),
            Some(0),
            "VM widens Int/Float, so the fixture must run there: {}",
            String::from_utf8_lossy(&vm.stderr)
        );
    }

    #[test]
    fn test_native_global_scalars_match_vm() {
        // Arrange：模块级 Int/Bool/String 全局；b 的初始化读 a，钉住初始化顺序。
        let source = concat!(
            "use std.assert\n",
            "a: Int = 1\n",
            "b: Int = a + 1\n",
            "c: Bool = b == 2\n",
            "s: String = \"one\"\n",
            "main: () -> Void = {\n",
            "    assert.assert(a == 1, \"int global initial value\")\n",
            "    assert.assert(b == 2, \"initializer sees earlier globals in order\")\n",
            "    assert.assert(c, \"bool global initializer\")\n",
            "    assert.assert(s == \"one\", \"string global initial value\")\n",
            "}\n",
            "main()\n",
        );

        // Act
        assert_native_matches_vm(source, "global_scalars");

        // Assert
        // assert_native_matches_vm 已断言 VM 与 O0/O2 的退出码与 stdout 一致。
    }

    #[test]
    fn test_native_global_writes_from_function_match_vm() {
        // Arrange：三个全局各由另一个函数写入，回到 main 后必须看见新值。
        let source = concat!(
            "use std.assert\n",
            "mut n: Int = 3\n",
            "mut f: Bool = false\n",
            "mut s: String = \"start\"\n",
            "set: () -> Void = {\n",
            "    n = 42\n",
            "    f = true\n",
            "    s = \"changed\"\n",
            "}\n",
            "main: () -> Void = {\n",
            "    set()\n",
            "    assert.assert(n == 42, \"int global write\")\n",
            "    assert.assert(f, \"bool global write\")\n",
            "    assert.assert(s == \"changed\", \"string global write\")\n",
            "}\n",
            "main()\n",
        );

        // Act
        assert_native_matches_vm(source, "global_writes");

        // Assert
        // assert_native_matches_vm 已断言 VM 与 O0/O2 的退出码与 stdout 一致。
    }

    #[test]
    fn test_native_project_global_mutation_matches_vm() {
        // Arrange：多文件项目——依赖单元的 Int/Bool/String 全局由其自身函数改写，
        // 入口单元经 import 读到新值。
        let root = TempDir::new().expect("global project tempdir");
        write_project(
            root.path(),
            &[
                (
                    "yaoxiang.toml",
                    "[package]\nname = \"global_project\"\nversion = \"0.1.0\"\n",
                ),
                (
                    "lib.yx",
                    concat!(
                        "mut counter: Int = 0\n",
                        "label: String = \"lib\"\n",
                        "flag: Bool = true\n",
                        "bump: () -> Int = {\n",
                        "    counter = counter + 1\n",
                        "    return counter\n",
                        "}\n",
                    ),
                ),
                (
                    "main.yx",
                    concat!(
                        "use std.assert\n",
                        "use lib.{bump, counter, label, flag}\n",
                        "main: () -> Void = {\n",
                        "    assert.assert(counter == 0, \"dependency global initial\")\n",
                        "    assert.assert(label == \"lib\", \"dependency string global\")\n",
                        "    assert.assert(flag, \"dependency bool global\")\n",
                        "    assert.assert(bump() == 1, \"dependency global write via its own fn\")\n",
                        "    assert.assert(counter == 1, \"dependency global write visible\")\n",
                        "}\n",
                    ),
                ),
            ],
        );
        let entry = root.path().join("main.yx");

        // Act
        let vm = Command::new(test_yx_bin())
            .current_dir(root.path())
            .arg("run")
            .arg(&entry)
            .output()
            .expect("run VM global project fixture");

        // Assert
        assert_eq!(
            vm.status.code(),
            Some(0),
            "VM global project exit: {}",
            String::from_utf8_lossy(&vm.stderr)
        );
        for optimize in [false, true] {
            let output = root
                .path()
                .join(format!("global_project_o{}", u8::from(optimize)));
            build_native_project_with_options(
                &entry,
                &output,
                false,
                optimize,
                CompileConfig::default(),
            )
            .unwrap_or_else(|e| panic!("global project build (O{}): {e:#}", u8::from(optimize)));
            let native = run_native(&output);
            assert_eq!(
                native.status.code(),
                vm.status.code(),
                "global project status mismatch (O{}): {}",
                u8::from(optimize),
                String::from_utf8_lossy(&native.stderr)
            );
            assert_eq!(
                native.stdout,
                vm.stdout,
                "global project stdout mismatch (O{})",
                u8::from(optimize)
            );
        }
    }

    #[test]
    fn test_native_unsupported_global_type_is_rejected() {
        // Arrange：Float 全局不在支持集——必须带源码位置拒绝且不产生产物。
        let dir = TempDir::new().expect("float global tempdir");
        let source = dir.path().join("float_global.yx");
        let executable = dir.path().join("float_global");
        std::fs::write(&source, "pi: Float = 3.5\nmain: () -> Void = { }\nmain()\n")
            .expect("write float global fixture");

        // Act
        let error = build_native_with_options(&source, &executable, false, false)
            .expect_err("Float global must stay outside the support set");

        // Assert
        let detail = format!("{error:#}");
        assert!(
            detail.contains(&format!("{}:1:", source.display())),
            "rejection must carry the initializer position: {detail}"
        );
        assert!(
            !executable.exists(),
            "rejected build must not write a native artifact"
        );
    }

    #[test]
    fn test_native_container_global_is_rejected() {
        // Arrange：容器全局（List）不在支持集——构建期拒绝、不静默 Void。
        // 注：拒绝落在容器构造指令上，该指令的 span 由 lower 留空（(0,0)），
        // 因此这里只断言「点名不支持指令 + 不产生产物」。
        let dir = TempDir::new().expect("list global tempdir");
        let source = dir.path().join("list_global.yx");
        let executable = dir.path().join("list_global");
        std::fs::write(
            &source,
            "xs: List(Int) = [1, 2, 3]\nmain: () -> Void = { }\nmain()\n",
        )
        .expect("write list global fixture");

        // Act
        let error = build_native_with_options(&source, &executable, false, false)
            .expect_err("container global must stay outside the support set");

        // Assert
        let detail = format!("{error:#}");
        assert!(
            detail.contains("AllocArray"),
            "rejection must name the unsupported instruction: {detail}"
        );
        assert!(
            !executable.exists(),
            "rejected build must not write a native artifact"
        );
    }
}

#[cfg(not(all(feature = "llvm", target_os = "linux", target_arch = "x86_64")))]
mod unavailable {
    use tempfile::TempDir;
    use yaoxiang::build_native_with_options;

    #[test]
    fn test_native_build_without_llvm_never_falls_back() {
        // Arrange
        let dir = TempDir::new().expect("native test tempdir");
        let source_path = dir.path().join("demo.yx");
        let output_path = dir.path().join("demo");
        std::fs::write(&source_path, "use std.assert\nassert.assert(true)\n")
            .expect("write no-LLVM fixture");

        // Act
        let error = build_native_with_options(&source_path, &output_path, false, false)
            .expect_err("engine without LLVM must reject native output");

        // Assert
        assert!(
            error.to_string().contains("--features llvm"),
            "error should explain the required build feature: {error:#}"
        );
        assert!(
            !output_path.exists(),
            "no native or bytecode fallback artifact may be written"
        );
    }
}
