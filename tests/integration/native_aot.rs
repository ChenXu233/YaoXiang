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
    use yaoxiang::build_native_with_options;

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
    fn test_native_unsupported_control_flow_preserves_output() {
        // Arrange
        let dir = TempDir::new().expect("native test tempdir");
        let source_path = dir.path().join("branch.yx");
        let output_path = dir.path().join("branch");
        std::fs::write(
            &source_path,
            concat!(
                "use std.assert\n",
                "choose: (a: Int) -> Int = (a) => if a == 0 { 1 } else { 2 }\n",
                "assert.assert(choose(0) == 1)\n",
            ),
        )
        .expect("write branch fixture");
        std::fs::write(&output_path, b"previous-good-output").expect("seed output");

        // Act
        let error = build_native_with_options(&source_path, &output_path, false, false)
            .expect_err("branches are outside AOT-1");

        // Assert
        assert!(
            error.to_string().contains("native AOT"),
            "error should identify native capability boundary: {error:#}"
        );
        assert_eq!(
            std::fs::read(&output_path).expect("read preserved output"),
            b"previous-good-output",
            "failed build must not overwrite previous output"
        );
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
