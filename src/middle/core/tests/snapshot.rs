//! WBS 2.2.2 / RFC-039 P2：规范化 IR 快照判据（07-equivalence-oracle 第二层）
//!
//! 判据形态：全语料（tests/yaoxiang/ + src/std/tests/，剔 skip / compile-error）
//! 逐文件走 lex→parse→typecheck→ir_gen 真实生成序列，normalize_module 规范化后
//! 与入库快照（snapshots/ 下按语料相对路径镜像存放）逐字节比对。
//!
//! 人工 review 流程（07 §128「输出必须入库，随 IR 定义变更显式更新，更新时
//! 人工 review diff」）：
//!   1. 更新：UPDATE_SNAPSHOTS=1 cargo test --lib snapshot -- --ignored 重写全部快照
//!   2. review：git diff 逐文件审阅（drift 即 IR 行为面变化，须能说清来源）
//!   3. 提交：快照与引发 drift 的代码改动同一 PR 入库
//!
//! 更新模式带 #[ignore] 常驻：更新是显式人工动作，不参与 CI；比对模式
//! test_ir_snapshots_stable 是常态判据（门禁 9.5 check-snapshot-drift.sh 驱动）。

use std::path::{Path, PathBuf};

use crate::frontend::core::lexer::tokenize;
use crate::frontend::core::parser::parse;
use crate::frontend::core::typecheck::checker::TypeChecker;
use crate::frontend::core::typecheck::MonoType;
use crate::frontend::module::registry::ModuleRegistry;
use crate::middle::core::ir::{FunctionBody, GlobalSlot, Instruction, Operand};
use crate::middle::core::ir_gen::AstToIrGenerator;
use crate::middle::core::normalize::normalize_module;
use crate::util::span::Span;

use super::corpus::{corpus_files, reaches_ir_generation};

/// 语料文件 → 快照路径：snapshots/ 下镜像语料相对路径，扩展名换 .snap。
fn snapshot_path_for(yx: &Path) -> PathBuf {
    let manifest = Path::new(env!("CARGO_MANIFEST_DIR"));
    let relative = yx.strip_prefix(manifest).unwrap_or(yx);
    let mut p = manifest
        .join("src")
        .join("middle")
        .join("core")
        .join("tests")
        .join("snapshots");
    for comp in relative.components() {
        if let std::path::Component::Normal(s) = comp {
            p.push(s);
        }
    }
    p.set_extension("snap");
    p
}

/// 逐文件生成规范化文本；生成前置阶段失败返回 None（不在判据合同面）。
fn normalize_corpus_file(path: &Path) -> Option<String> {
    let source = std::fs::read_to_string(path).ok()?;
    let tokens = tokenize(&source).ok()?;
    let parsed = parse(&tokens);
    if parsed.has_errors {
        return None;
    }
    let mut checker = TypeChecker::new(&path.display().to_string());
    let type_result = checker.check_module(&parsed.module);
    if !type_result.diagnostics.is_empty() {
        return None;
    }
    let mut generator =
        AstToIrGenerator::new_with_type_result(&type_result, ModuleRegistry::with_std(), None);
    let module_ir = generator.generate_module_ir(&parsed.module).ok()?;
    Some(normalize_module(&module_ir))
}

/// 全语料快照比对（常态判据）/ 更新（UPDATE_SNAPSHOTS=1 + --ignored）。
fn run_snapshot_mode(update: bool) -> (usize, Vec<String>) {
    let mut compared = 0usize;
    let mut drift: Vec<String> = Vec::new();
    for file in corpus_files() {
        if !reaches_ir_generation(&file) {
            continue;
        }
        let Some(text) = normalize_corpus_file(&file) else {
            continue;
        };
        let snap = snapshot_path_for(&file);
        let relative = file
            .strip_prefix(env!("CARGO_MANIFEST_DIR"))
            .unwrap_or(&file)
            .display()
            .to_string();
        compared += 1;
        if update {
            std::fs::create_dir_all(snap.parent().unwrap()).unwrap();
            std::fs::write(&snap, &text).unwrap();
        } else {
            match std::fs::read_to_string(&snap) {
                Ok(expected) if expected == text => {}
                Ok(_) => drift.push(format!("{relative}: 快照漂移")),
                Err(_) => drift.push(format!(
                    "{relative}: 快照缺失（先跑 UPDATE_SNAPSHOTS=1 更新流程）"
                )),
            }
        }
    }
    (compared, drift)
}

#[test]
fn test_ir_snapshots_stable() {
    let (compared, drift) = run_snapshot_mode(false);
    // 非空断言（门禁 9.5 防假门禁）：快照面必须真实覆盖语料
    assert!(compared > 200, "快照面异常收窄（{compared}）——判据失效");
    assert!(
        drift.is_empty(),
        "IR 快照漂移（{}/{} 文件）——C1 zero-diff 判据红：\n{}\n",
        drift.len(),
        compared,
        drift.join("\n"),
    );
}

/// 快照更新模式：显式人工动作（07 §128 流程第 1 步），不参与常态套件。
#[test]
#[ignore = "快照更新是显式人工动作：UPDATE_SNAPSHOTS=1 cargo test --lib snapshot -- --ignored"]
fn update_ir_snapshots() {
    assert!(
        std::env::var_os("UPDATE_SNAPSHOTS").is_some(),
        "更新快照必须显式设 UPDATE_SNAPSHOTS=1（防误触）"
    );
    let (updated, _) = run_snapshot_mode(true);
    assert!(updated > 200, "更新面异常收窄（{updated}）");
    eprintln!("已更新 {updated} 个快照——下一步：git diff 人工 review");
}

// =====================
// 规范化规则单元测试（07 §第二层表格逐项）
// =====================

use super::ir_build::{func, int_const, module_with, move_instr, ret_void, temp_slot};

/// 剥 Span：仅 span 不同的两个模块，规范化文本必须全等。
#[test]
fn span_stripped_makes_identical_output() {
    let mk = |line: u32| {
        let mut f = func(
            "f",
            vec![],
            MonoType::Void,
            vec![temp_slot(MonoType::Int(64))],
            vec![ret_void()],
        );
        if let FunctionBody::Code { blocks, .. } = &mut f.body {
            blocks[0].instructions[0] = Instruction::Ret {
                value: None,
                span: Span::new(
                    crate::util::span::Position::with_offset(line as usize, 1, 0),
                    crate::util::span::Position::with_offset(line as usize, 2, 1),
                ),
            };
        }
        module_with(vec![f])
    };
    assert_eq!(normalize_module(&mk(1)), normalize_module(&mk(99)));
}

/// 临时值重命名：按首次出现顺序 %0 %1（与原始槽号无关）。
#[test]
fn locals_renamed_by_first_use_order() {
    let f = func(
        "f",
        vec![],
        MonoType::Void,
        vec![temp_slot(MonoType::Int(64)); 10],
        vec![
            move_instr(7, int_const(1)),
            move_instr(3, Operand::Local(7)),
            ret_void(),
        ],
    );
    let text = normalize_module(&module_with(vec![f]));
    assert!(text.contains("Move %0 <- int(1)"), "{text}");
    assert!(text.contains("Move %1 <- %0"), "{text}");
}

/// 全局槽位相对化：Global(7) 首次出现即 @0。
#[test]
fn globals_relativized_by_first_use() {
    let mut m = module_with(vec![func(
        "f",
        vec![],
        MonoType::Void,
        vec![temp_slot(MonoType::Int(64))],
        vec![
            Instruction::Load {
                dst: Operand::Local(0),
                src: Operand::Global(7),
                span: Span::dummy(),
            },
            ret_void(),
        ],
    )]);
    m.globals.push(GlobalSlot {
        name: "g".to_string(),
        ty: MonoType::Int(64),
        index: 7,
    });
    let text = normalize_module(&m);
    assert!(text.contains("Load %0 <- @0"), "{text}");
}

/// 确定性：HashMap 元数据（function_files）排序后两次输出全等。
#[test]
fn deterministic_across_hashmap_iteration() {
    let mut m = module_with(vec![func(
        "f",
        vec![],
        MonoType::Void,
        vec![],
        vec![ret_void()],
    )]);
    m.function_files.insert("b_fn".to_string(), 1);
    m.function_files.insert("a_fn".to_string(), 0);
    let first = normalize_module(&m);
    let second = normalize_module(&m);
    assert_eq!(first, second);
    let a_pos = first.find("a_fn").unwrap();
    let b_pos = first.find("b_fn").unwrap();
    assert!(a_pos < b_pos, "function_files 必须按名排序：{first}");
}

/// 结构体字段不排序：构造顺序是语义（07 原文），字段序保留即 drift 可见。
#[test]
fn struct_field_order_preserved() {
    let f = func(
        "f",
        vec![],
        MonoType::Void,
        vec![temp_slot(MonoType::Int(64)); 3],
        vec![
            Instruction::CreateStruct {
                dst: Operand::Local(0),
                type_name: "P".to_string(),
                fields: vec![Operand::Local(1), Operand::Local(2)],
                span: Span::dummy(),
            },
            ret_void(),
        ],
    );
    let text = normalize_module(&module_with(vec![f]));
    assert!(text.contains("CreateStruct %0 <- P[%1, %2]"), "{text}");
}
