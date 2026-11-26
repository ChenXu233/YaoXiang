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
    use yaoxiang::build_native_with_options;
    use yaoxiang::middle::ir::ir::{
        BasicBlock, ConstValue, FunctionBody, FunctionIR, Instruction, ModuleIR, Operand,
    };
    use yaoxiang::util::span::Span;

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
    fn test_native_unsupported_merge_value_preserves_output() {
        // Arrange：跨分支合并值仍受 Phi/未知类型边界约束，不是无 Phi 分支。
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
            .expect_err("merge values outside the no-Phi branch subset must be rejected");

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

    fn build_synthetic(
        module: &ModuleIR,
        stem: &str,
    ) -> (TempDir, PathBuf) {
        let dir = TempDir::new().expect("synthetic native tempdir");
        let output = dir.path().join(stem);
        let source = dir.path().join(format!("{stem}.yx"));
        compile_ir_to_executable(module, &source.display().to_string(), &output, false)
            .unwrap_or_else(|e| panic!("build synthetic native fixture {stem}: {e:#}"));
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
    fn test_native_local_merge_boundary_reports_aot_35() {
        // Arrange：两臂各写 a、汇合后读 a；分析层必须在 LLVM verifier 前拒绝。
        let dir = TempDir::new().expect("local merge tempdir");
        let source_path = dir.path().join("local_merge.yx");
        let output_path = dir.path().join("local_merge");
        std::fs::write(
            &source_path,
            concat!(
                "test: (c: Bool) -> Void = (c) => {\n",
                "    a = if c { 42 } else { 0 }\n",
                "    b = a + 1\n",
                "}\n",
                "\n",
                "test(true)\n",
            ),
        )
        .expect("write local merge fixture");

        // Act
        let error = build_native_with_options(&source_path, &output_path, false, false)
            .expect_err("Local merge must be rejected by AOT analysis");

        // Assert
        let detail = format!("{error:#}");
        assert!(
            detail.contains("AOT-3.5"),
            "diagnostic must point at the AOT-3.5 boundary: {detail}"
        );
        assert!(
            detail.contains(&format!("{}:3:5", source_path.display())),
            "diagnostic must carry the source position: {detail}"
        );
        assert!(
            !detail.contains("Instruction does not dominate all uses"),
            "LLVM verifier text must not leak: {detail}"
        );
    }

    #[test]
    fn test_native_value_destination_readback() {
        // Arrange
        let module = value_module();

        // Act
        let (_dir, executable) = build_synthetic(&module, "value_destination");
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
            let (_dir, executable) = build_synthetic(&module, "branch");
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
    fn test_native_explicit_unknown_value_is_rejected() {
        // Arrange
        let module = simple_value_module(
            yaoxiang::frontend::core::sema::MonoType::TypeRef("?".to_string()),
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
        // Arrange
        let module =
            simple_value_module(yaoxiang::frontend::core::sema::MonoType::Float(64), false);

        // Act
        let detail = compile_error(&module);

        // Assert
        assert!(
            detail.contains("ValueId(0)") && detail.contains("Float(64)"),
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
    fn test_native_phi_is_rejected_at_batch_35() {
        // Arrange
        let module = phi_module();

        // Act
        let detail = compile_error(&module);

        // Assert
        assert!(
            detail.contains("3.5"),
            "Phi must be rejected with the AOT-3.5 boundary: {detail}"
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
