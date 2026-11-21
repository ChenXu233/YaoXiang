//! DCE 直驱测试：N1–N6 负控矩阵 + 根面/自洽/回滚/幂等 + 真实 post-mono 直驱（pre-DCE 自洽 → DCE 删除，重锚后判据见 test_post_mono_dce_removes_mono_retained_originals_and_keeps_instances 头注释）。
//!
//! 规范：t1 设计包 §2（根集九行表，单一事实源在 dce 模块 doc）、§3（删除判据）。
//! 每个测试只钉一个可证伪的区分；四格基线 = 死函数删除 / 导出面保留 /
//! MakeClosure 目标保留 / 字符串常量名保留（裁决 A 的 N1–N6）。

use std::collections::{HashMap, HashSet};

use crate::frontend::core::sema::MonoType;
use crate::frontend::module::symbol::DefId;
use crate::middle::ir::ir::{
    BasicBlock, ConstValue, FunctionBody, FunctionIR, Instruction, LocalSlot, ModuleIR, Operand,
    TypeDeclBody, ValueId, ValueInfo,
};
use crate::middle::ir::normalize::normalize_module;
use crate::middle::ir::verify::verify_loose;
use crate::util::span::Span;

use super::{eliminate, eliminate_with, static_def_target, static_name_target, DceReport};

// ==================== 夹具 ====================

fn span() -> Span {
    Span::dummy()
}

fn ret_void() -> Instruction {
    Instruction::Ret {
        value: None,
        span: span(),
    }
}

fn call_name(
    name: &str,
    def: Option<DefId>,
) -> Instruction {
    Instruction::Call {
        dst: None,
        func: Operand::Const(ConstValue::String(name.to_string())),
        args: Vec::new(),
        def,
        span: span(),
    }
}

fn make_closure(
    target: &str,
    dst: usize,
) -> Instruction {
    Instruction::MakeClosure {
        dst: Operand::Local(dst),
        func: target.to_string(),
        def: None,
        env: Vec::new(),
        span: span(),
    }
}

fn code_fn(
    name: &str,
    instructions: Vec<Instruction>,
) -> FunctionIR {
    FunctionIR {
        name: name.to_string(),
        def: None,
        params: Vec::new(),
        return_type: MonoType::Void,
        generic_params: None,
        body: FunctionBody::Code {
            blocks: vec![BasicBlock {
                label: 0,
                instructions,
                successors: Vec::new(),
            }],
            entry: 0,
            locals: Vec::new(),
            values: Vec::new(),
            value_slots: Vec::new(),
        },
    }
}

fn code_fn_with_def(
    name: &str,
    def: DefId,
    instructions: Vec<Instruction>,
) -> FunctionIR {
    let mut func = code_fn(name, instructions);
    func.def = Some(def);
    func
}

fn type_decl_fn(name: &str) -> FunctionIR {
    FunctionIR {
        name: name.to_string(),
        def: None,
        params: Vec::new(),
        return_type: MonoType::Void,
        generic_params: None,
        body: FunctionBody::TypeDecl {
            body: TypeDeclBody {
                fields: Vec::new(),
                self_type: MonoType::Void,
            },
        },
    }
}

/// 具名局部槽（支配性判定只覆盖具名槽：temp 槽由帧语义预初始化）。
fn named_local(name: &str) -> LocalSlot {
    LocalSlot {
        name: Some(name.to_string()),
        ty: MonoType::Int(64),
        scope_depth: 0,
        quantity: None,
        // 直构夹具无生成来源：不参与「内层隔离」的帧归属判定（WBS 2.1.4）。
        origin: None,
    }
}

fn module_with(functions: Vec<FunctionIR>) -> ModuleIR {
    ModuleIR {
        functions,
        ..Default::default()
    }
}

fn names(module: &ModuleIR) -> Vec<String> {
    module.functions.iter().map(|f| f.name.clone()).collect()
}

fn has(
    module: &ModuleIR,
    name: &str,
) -> bool {
    module.functions.iter().any(|f| f.name == name)
}

/// 删除后引用自洽：存活体/init 里凡是「删除前落在函数表里」的名字/DefId 目标必须仍存活；
/// 且不得出现索引形态调用目标（它会让函数表下标失配）。
fn assert_references_closed(
    before: &ModuleIR,
    after: &ModuleIR,
) {
    let pre_names: HashSet<&str> = before.functions.iter().map(|f| f.name.as_str()).collect();
    let pre_defs: HashMap<DefId, &str> = before
        .functions
        .iter()
        .filter_map(|f| f.def.map(|d| (d, f.name.as_str())))
        .collect();
    let after_names: HashSet<&str> = after.functions.iter().map(|f| f.name.as_str()).collect();

    let check = |owner: &str, instr: &Instruction| {
        if let Some(target) = static_name_target(instr) {
            if pre_names.contains(target) {
                assert!(
                    after_names.contains(target),
                    "{owner} 仍引用被删函数 {target}（引用闭合失败）"
                );
            }
        }
        if let Some(def) = static_def_target(instr) {
            if let Some(target) = pre_defs.get(&def) {
                assert!(
                    after_names.contains(*target),
                    "{owner} 的 DefId({}) 目标 {target} 被删（DefId 悬空）",
                    def.0
                );
            }
        }
        assert!(
            !matches!(
                instr,
                Instruction::Call {
                    func: Operand::Const(ConstValue::Int(_)),
                    ..
                } | Instruction::TailCall {
                    func: Operand::Const(ConstValue::Int(_)),
                    ..
                }
            ),
            "{owner} 出现了索引形态调用目标（函数表下标会失配）"
        );
    };

    for func in &after.functions {
        for instr in func.all_instructions() {
            check(&func.name, instr);
        }
    }
    for instr in &after.init {
        check("<init>", instr);
    }

    let mut seen = HashSet::new();
    for func in &after.functions {
        assert!(
            seen.insert(func.name.as_str()),
            "删除后函数名重复：{}",
            func.name
        );
    }
}

// ==================== N 矩阵与四格 ====================

/// N6（防假绿）：完全无引用的函数必须被删。
#[test]
fn test_n6_unreferenced_function_is_deleted() {
    // Arrange
    let mut module = module_with(vec![
        code_fn("main", vec![ret_void()]),
        code_fn("dead", vec![ret_void()]),
    ]);
    let before = module.clone();

    // Act
    let report = eliminate(&mut module).expect("无索引目标/无重复名，DCE 应成功");

    // Assert
    assert_eq!(names(&module), vec!["main".to_string()], "死函数必须被删");
    assert_eq!(
        report.deleted,
        vec!["dead".to_string()],
        "归因报告必须逐名给删除"
    );
    assert_eq!(report.kept, 1, "kept 必须是删除后的函数表长度");
    assert_references_closed(&before, &module);
}

/// N6 迁移形态：只被另一个死函数引用的函数也属死代码（传递闭包）。
#[test]
fn test_n6_transitively_dead_chain_is_deleted() {
    // Arrange：main ← dead_a ← dead_b；dead_a 只被 dead_b 用（而 dead_b 自身不可达）
    let mut module = module_with(vec![
        code_fn("main", vec![ret_void()]),
        code_fn("dead_a", vec![ret_void()]),
        code_fn("dead_b", vec![call_name("dead_a", None), ret_void()]),
    ]);
    let before = module.clone();

    // Act
    let report = eliminate(&mut module).expect("DCE 应成功");

    // Assert
    assert_eq!(
        names(&module),
        vec!["main".to_string()],
        "传递死链必须整体删除"
    );
    assert_eq!(
        report.deleted,
        vec!["dead_a".to_string(), "dead_b".to_string()],
        "删除归因按模块原序"
    );
    assert_references_closed(&before, &module);
}

/// N1：只以字符串常量名被 Call 引用（def=None）的函数必须保留。
#[test]
fn test_n1_string_constant_call_name_is_kept() {
    // Arrange
    let mut module = module_with(vec![
        code_fn("main", vec![call_name("only.by.string", None), ret_void()]),
        code_fn("only.by.string", vec![ret_void()]),
        code_fn("dead", vec![ret_void()]),
    ]);
    let before = module.clone();

    // Act
    eliminate(&mut module).expect("DCE 应成功");

    // Assert
    assert!(has(&module, "only.by.string"), "字符串常量名引用必须入根");
    assert!(!has(&module, "dead"), "无关死函数应被删（防假绿）");
    assert_references_closed(&before, &module);
}

/// N3：跨模块限定名（lib.helper）从 init 引用 ⇒ 必须保留（导入面在 IR 层就是限定名字符串）。
#[test]
fn test_n3_cross_module_qualified_name_from_init_is_kept() {
    // Arrange
    let mut module = module_with(vec![
        code_fn("main", vec![ret_void()]),
        code_fn("lib.helper", vec![ret_void()]),
        code_fn("dead", vec![ret_void()]),
    ]);
    module.init = vec![call_name("lib.helper", None)];
    let before = module.clone();

    // Act
    eliminate(&mut module).expect("DCE 应成功");

    // Assert
    assert!(has(&module, "lib.helper"), "init 里的跨模块限定名必须入根");
    assert!(!has(&module, "dead"), "无关死函数应被删");
    assert_references_closed(&before, &module);
}

/// N2：只被 MakeClosure 引用（函数值/lambda）的函数必须保留。
#[test]
fn test_n2_make_closure_target_is_kept() {
    // Arrange
    let mut module = module_with(vec![
        code_fn("main", vec![make_closure("builder", 0), ret_void()]),
        code_fn("builder", vec![ret_void()]),
        code_fn("dead", vec![ret_void()]),
    ]);
    let before = module.clone();

    // Act
    eliminate(&mut module).expect("DCE 应成功");

    // Assert
    assert!(has(&module, "builder"), "MakeClosure 目标必须入根");
    assert!(!has(&module, "dead"), "无关死函数应被删");
    assert_references_closed(&before, &module);
}

/// N4：只被 CallVirt（裸方法名）可达的方法必须保留——方法面根 = TypeDecl 的 t. 前缀。
///
/// 语料没有 CallVirt 生成点（实测 interface_* 夹具走变体分发静态调用），故用合成 IR。
/// 本测试同时钉「不得从 CallVirt.method_name 拼 Type.method」：方法名只写 get_x，
/// 由 Point. 前缀入根。
#[test]
fn test_n4_callvirt_only_method_is_kept() {
    // Arrange
    let mut module = module_with(vec![
        code_fn(
            "main",
            vec![
                Instruction::CallVirt {
                    dst: None,
                    obj: Operand::Local(0),
                    method_name: "get_x".to_string(),
                    args: Vec::new(),
                    span: span(),
                },
                ret_void(),
            ],
        ),
        type_decl_fn("Point"),
        code_fn("Point.get_x", vec![ret_void()]),
        code_fn("dead", vec![ret_void()]),
    ]);
    let before = module.clone();

    // Act
    eliminate(&mut module).expect("DCE 应成功");

    // Assert
    assert!(has(&module, "Point"), "TypeDecl 恒留");
    assert!(
        has(&module, "Point.get_x"),
        "TypeDecl 的 t. 前缀方法必须入根（vtable 面），即使只有 CallVirt 可达"
    );
    assert!(!has(&module, "dead"), "无关死函数应被删");
    assert_references_closed(&before, &module);
}

/// N5：NativeFn 常量不是函数表键 ⇒ 不建根、不报错；同名表项按死码处理。
#[test]
fn test_n5_native_fn_name_is_not_a_root() {
    // Arrange：main 里有一个 NativeFn 常量；函数表里另有一个同名 Code 函数（构造性
    // 反例；生产 IR 里 extern-ref 体不会产生 FunctionIR）
    let mut module = module_with(vec![
        code_fn(
            "main",
            vec![
                Instruction::Load {
                    dst: Operand::Local(0),
                    src: Operand::Const(ConstValue::NativeFn("std.io.println".to_string())),
                    span: span(),
                },
                ret_void(),
            ],
        ),
        code_fn("std.io.println", vec![ret_void()]),
        code_fn("dead", vec![ret_void()]),
    ]);
    let before = module.clone();

    // Act
    let report = eliminate(&mut module).expect("NativeFn 不应让 DCE 报错");

    // Assert
    assert!(
        !has(&module, "std.io.println"),
        "NativeFn 常量不是表键，同名表项不得因它入根"
    );
    assert!(
        report.deleted.contains(&"std.io.println".to_string()),
        "删除归因应含它"
    );
    assert!(!has(&module, "dead"), "无关死函数应被删");
    assert_references_closed(&before, &module);
}

/// 类型名字符串（CreateStruct.type_name）不得当函数根。
#[test]
fn test_type_name_string_is_not_a_function_root() {
    // Arrange：CreateStruct 指向 Ghost；表里另有一个 Code 函数叫 Ghost（非 TypeDecl）
    let mut module = module_with(vec![
        code_fn(
            "main",
            vec![
                Instruction::CreateStruct {
                    dst: Operand::Local(0),
                    type_name: "Ghost".to_string(),
                    fields: Vec::new(),
                    span: span(),
                },
                ret_void(),
            ],
        ),
        code_fn("Ghost", vec![ret_void()]),
    ]);
    let before = module.clone();

    // Act
    eliminate(&mut module).expect("DCE 应成功");

    // Assert
    assert!(
        !has(&module, "Ghost"),
        "CreateStruct.type_name 是类型名、不是函数表键，同名 Code 函数不得入根"
    );
    assert_references_closed(&before, &module);
}

/// 导出面根：TypeDecl 的构造器（TypeDecl 自身）与 t. 前缀方法恒留。
#[test]
fn test_type_decl_and_prefix_methods_are_kept() {
    // Arrange
    let mut module = module_with(vec![
        code_fn("main", vec![ret_void()]),
        type_decl_fn("T"),
        code_fn("T.m", vec![ret_void()]),
        code_fn("T2.m", vec![ret_void()]),
    ]);
    let before = module.clone();

    // Act
    eliminate(&mut module).expect("DCE 应成功");

    // Assert
    assert!(has(&module, "T"), "TypeDecl 恒留");
    assert!(has(&module, "T.m"), "T. 前缀方法入根");
    assert!(
        !has(&module, "T2.m"),
        "非 TypeDecl 前缀的 T2.m 不得被 T. 前缀误收"
    );
    assert_references_closed(&before, &module);
}

/// 入口函数：entry_function 是根（即使 init/其它函数都不引用它）。
#[test]
fn test_entry_function_is_kept() {
    // Arrange
    let mut module = module_with(vec![
        code_fn("app.main", vec![ret_void()]),
        code_fn("dead", vec![ret_void()]),
    ]);
    module.entry_function = Some("app.main".to_string());
    let before = module.clone();

    // Act
    eliminate(&mut module).expect("DCE 应成功");

    // Assert
    assert!(has(&module, "app.main"), "入口函数必须入根");
    assert!(!has(&module, "dead"), "无关死函数应被删");
    assert_references_closed(&before, &module);
}

/// #476 契约：script 形态「main 已定义但未调用」也必须保留 main（提示/退出码依赖函数表）。
#[test]
fn test_script_main_is_kept_even_if_uncalled() {
    // Arrange：无 entry_function、init 为空、main 无任何引用
    let mut module = module_with(vec![code_fn("main", vec![ret_void()])]);

    // Act
    let report = eliminate(&mut module).expect("DCE 应成功");

    // Assert
    assert!(has(&module, "main"), "名字 main 恒为根（#476 契约）");
    assert!(report.deleted.is_empty(), "本夹具没有可删函数");
}

/// 限定名形态的 main（lib.main）同样是根。
#[test]
fn test_qualified_main_suffix_is_kept() {
    // Arrange
    let mut module = module_with(vec![
        code_fn("lib.main", vec![ret_void()]),
        code_fn("dead", vec![ret_void()]),
    ]);

    // Act
    eliminate(&mut module).expect("DCE 应成功");

    // Assert
    assert!(has(&module, "lib.main"), "endswith(.main) 的函数恒为根");
    assert!(!has(&module, "dead"), "无关死函数应被删");
}

/// DefId 面：名字陈旧但 def 指向存活函数时，必须按 def 保活。
#[test]
fn test_defid_only_target_is_kept() {
    // Arrange：main 的 Call 名字 stale_name 不在表里，但 def 指向 target
    let mut module = module_with(vec![
        code_fn(
            "main",
            vec![call_name("stale_name", Some(DefId(7))), ret_void()],
        ),
        code_fn_with_def("target", DefId(7), vec![ret_void()]),
    ]);
    let before = module.clone();

    // Act
    eliminate(&mut module).expect("DCE 应成功");

    // Assert
    assert!(
        has(&module, "target"),
        "DefId 目标必须入根（codegen 优先 def_idx）"
    );
    assert_references_closed(&before, &module);
}

/// 反向 MakeClosure 闭合：存活闭包体的唯一 MakeClosure 引用者不可达 ⇒ 引用者被保活。
#[test]
fn test_reverse_make_closure_keeps_only_referencer() {
    // Arrange：main 按名调 layer（layer 活）；creator 只通过 MakeClosure 引用 layer，
    // 自身没有任何入边（死）。若不反向闭合，creator 被删后 layer 的 env_len 反推会失效。
    let mut module = module_with(vec![
        code_fn("main", vec![call_name("layer", None), ret_void()]),
        code_fn("layer", vec![ret_void()]),
        code_fn("creator", vec![make_closure("layer", 0), ret_void()]),
        code_fn("dead", vec![ret_void()]),
    ]);
    let before = module.clone();

    // Act
    eliminate(&mut module).expect("DCE 应成功");

    // Assert
    assert!(has(&module, "layer"), "layer 被 main 按名引用，必须存活");
    assert!(
        has(&module, "creator"),
        "layer 的唯一 MakeClosure 引用者必须被反向闭合保活（env_len 反推不变量）"
    );
    assert!(!has(&module, "dead"), "闭环外的真死码仍应被删");
    assert_references_closed(&before, &module);
}

/// 反向闭合是无条件保守规则：活目标的所有引用者都留（含本身不可达的引用者），
/// 且重复运行/等价模块重跑的删除集合一致（单调规则、顺序无关）。
#[test]
fn test_reverse_make_closure_is_unconditional_and_deterministic() {
    // Arrange：main 按名调 a（a 活）；a 与 b 都 MakeClosure layer；b 自身不可达
    let mut module = module_with(vec![
        code_fn("main", vec![call_name("a", None), ret_void()]),
        code_fn("a", vec![make_closure("layer", 0), ret_void()]),
        code_fn("b", vec![make_closure("layer", 1), ret_void()]),
        code_fn("layer", vec![ret_void()]),
        code_fn("dead", vec![ret_void()]),
    ]);
    let before = module.clone();

    // Act
    let first = eliminate(&mut module).expect("DCE 应成功");
    let after_first = normalize_module(&module);
    let mut again = before.clone();
    let third = eliminate(&mut again).expect("DCE 应成功");
    let second = eliminate(&mut module).expect("二次 DCE 应成功");

    // Assert
    assert!(
        has(&module, "b"),
        "活目标 layer 的全部引用者必须保活（无条件保守，不可达引用者也留）"
    );
    assert!(!has(&module, "dead"), "闭环外的真死码仍应被删");
    assert!(second.deleted.is_empty(), "二次运行不得再删（幂等）");
    assert_eq!(
        normalize_module(&module),
        after_first,
        "二次运行不得改动 IR"
    );
    assert_eq!(
        third.deleted, first.deleted,
        "等价模块重跑的删除集合与顺序必须一致（确定性）"
    );
    assert_eq!(
        normalize_module(&again),
        after_first,
        "等价模块的结果必须一致"
    );
    assert_references_closed(&before, &module);
}

/// 索引形态调用目标：中止删除且 ModuleIR 逐字节不变。
#[test]
fn test_int_call_target_aborts_without_mutation() {
    // Arrange
    let mut module = module_with(vec![
        code_fn(
            "main",
            vec![
                Instruction::Call {
                    dst: None,
                    func: Operand::Const(ConstValue::Int(3)),
                    args: Vec::new(),
                    def: None,
                    span: span(),
                },
                ret_void(),
            ],
        ),
        code_fn("dead", vec![ret_void()]),
    ]);
    let before = normalize_module(&module);

    // Act
    let result = eliminate(&mut module);

    // Assert
    assert!(result.is_err(), "索引目标必须让 DCE 中止");
    let err = result.expect_err("已断言 Err");
    assert!(
        err.to_string().contains("Const(Int(3))") || format!("{err:?}").contains("Const(Int(3))"),
        "内部诊断应点名索引目标；实际 {err:?}"
    );
    assert_eq!(
        normalize_module(&module),
        before,
        "中止路径不得改动 ModuleIR"
    );
}

/// 回滚路径：DCE_ENABLED=false ⇒ 空报告且 ModuleIR 逐字节不变（含 function_files）。
#[test]
fn test_disabled_leaves_module_byte_identical() {
    // Arrange
    let mut module = module_with(vec![
        code_fn("main", vec![ret_void()]),
        code_fn("dead", vec![ret_void()]),
    ]);
    module.function_files.insert("main".to_string(), 0);
    module.function_files.insert("dead".to_string(), 0);
    let before_ir = normalize_module(&module);
    let before_files = module.function_files.clone();

    // Act
    let report: DceReport = eliminate_with(&mut module, false).expect("关闭开关不得失败");

    // Assert
    assert!(report.deleted.is_empty(), "关闭开关不得删任何函数");
    assert_eq!(
        normalize_module(&module),
        before_ir,
        "关闭开关必须逐字节不变"
    );
    assert_eq!(
        module.function_files, before_files,
        "function_files 也必须不变"
    );
}

/// function_files 修剪：删除函数的条目被清掉，存活条目保持。
#[test]
fn test_function_files_pruned_for_deleted_function() {
    // Arrange
    let mut module = module_with(vec![
        code_fn("main", vec![ret_void()]),
        code_fn("dead", vec![ret_void()]),
    ]);
    module.function_files.insert("main".to_string(), 0);
    module.function_files.insert("dead".to_string(), 1);

    // Act
    eliminate(&mut module).expect("DCE 应成功");

    // Assert
    assert_eq!(module.function_files.get("main"), Some(&0), "存活条目保持");
    assert!(
        !module.function_files.contains_key("dead"),
        "已删函数的条目必须修剪"
    );
}

/// 幂等：对输出再跑一次不删任何东西。
#[test]
fn test_dce_is_idempotent() {
    // Arrange
    let mut module = module_with(vec![
        code_fn("main", vec![make_closure("builder", 0), ret_void()]),
        code_fn("builder", vec![ret_void()]),
        code_fn("dead", vec![ret_void()]),
    ]);
    eliminate(&mut module).expect("首次 DCE 应成功");
    let after_first = normalize_module(&module);

    // Act
    let second = eliminate(&mut module).expect("二次 DCE 应成功");

    // Assert
    assert!(second.deleted.is_empty(), "二次运行不得再删（幂等）");
    assert_eq!(
        normalize_module(&module),
        after_first,
        "二次运行不得改动 IR"
    );
}

/// 真实 post-mono 直驱：mono 保留原件与共享层（75c28a09 起删除职责唯一归 DCE），
/// DCE 恰好删掉两者、保留实例与逐实例克隆，前后 verify_loose 均绿。
///
/// 判据门 = 病史 + 证明。病史：#542（共享层失去唯一 MakeClosure 引用者成孤儿，
/// verify 红）与 #542-D3（删除职责单点归 DCE，mono 不再按名删原件——75c28a09
/// 「build_output 保留全部原件」）。证明：mono 只生成不删除 ⇒ 每个共享层/克隆
/// 都有其 MakeClosure 引用者，pre-DCE 产物必须自洽（verify_loose 绿）——这是
/// #542 类孤儿缺陷在现架构下**唯一的单元级观察点**（接线测试只看 DCE 后终态，
/// DCE 会把孤儿静默清掉；生产入口测试无法区分「谁删的」）；本测试同时钉住
/// 职责划分：若 mono 重新自删原件或 DCE 根集破洞（漏删原件 / 误删实例），此处红。
///
/// 前身 = t1 设计包 §4 的「删除前 verify 红」判据——该前提随 75c28a09 的设计
/// 变更作废（原件保留后共享层有引用者、不再成孤儿），本测试随之 retarget 到
/// 新架构不变量（同提交对 mono 两条闸测试的 retarget 先例，不删测试）。
#[test]
fn test_post_mono_dce_removes_mono_retained_originals_and_keeps_instances() {
    // Arrange：语料本体（include_str，不复制源码，避免探针与语料漂移）。
    // 走 lex→parse→typecheck→ir_gen→mono 的真实生成序列而不是 Compiler 外观：
    // 只有直驱能取到「DCE 前」的 post-mono IR，pre-DCE 自洽性才可观察。
    let source =
        include_str!("../../../../../tests/yaoxiang/02-type-system/generics/fn_const_generic.yx");
    let tokens = crate::frontend::core::lexer::tokenize(source).expect("语料必须通过词法分析");
    let parsed = crate::frontend::core::parser::parse(&tokens);
    assert!(!parsed.has_errors, "语料必须通过语法分析");
    let ns = crate::frontend::core::sema::probe::TypeLevelNamespace::scan_one(&parsed.module);
    let mut checker = crate::frontend::core::sema::checker::TypeChecker::new("fn_const_generic.yx");
    let type_result = checker.check_module(&parsed.module, &ns);
    assert!(
        type_result.diagnostics.is_empty(),
        "语料必须通过类型检查；实际 {:?}",
        type_result.diagnostics
    );
    let mut generator = crate::middle::lower::AstToIrGenerator::new_with_type_result(
        &type_result,
        crate::frontend::module::registry::ModuleRegistry::with_std(),
        None,
        ns,
    );
    let ir_pre = generator
        .generate_module_ir(&parsed.module)
        .expect("语料必须生成 IR");
    let mut ir = crate::middle::passes::mono::Monomorphizer::new()
        .monomorphize(&ir_pre, &type_result.instantiation_requests)
        .expect("语料必须通过单态化");

    // Assert（pre-DCE，75c28a09 设计形态）：mono 保留全部原件——共享层与原件
    // 都在函数表里；每个 MakeClosure 目标都有引用者 ⇒ verify_loose 绿（#542 类
    // 孤儿缺陷的回归守卫：mono 一旦再产出孤儿，此处红）。
    assert!(
        has(&ir, "f"),
        "mono 必须保留泛型原件 f（删除职责唯一归 DCE）"
    );
    assert!(
        has(&ir, "__f_l0"),
        "mono 必须保留共享层 __f_l0（原件 f 的 MakeClosure 目标）"
    );
    assert!(
        verify_loose(&ir).is_ok(),
        "pre-DCE post-mono 产物必须自洽（mono 只生成不删除，不得有孤儿共享层；实际 {:?}）",
        verify_loose(&ir).err()
    );

    // Act：DCE
    let before = ir.clone();
    let report = eliminate(&mut ir).expect("真实 post-mono IR 上 DCE 应成功");

    // Assert（post-DCE）：mono 保留的原件/共享层恰好被删，实例与克隆保留，
    // 结构合法，删除归因闭合。
    assert!(
        verify_loose(&ir).is_ok(),
        "删除后 verify_loose 必须绿；实际 {:?}",
        verify_loose(&ir).err()
    );
    assert!(
        !has(&ir, "f"),
        "不可达泛型原件 f 必须由 DCE 删除（75c28a09：删除职责唯一归 DCE）；实际函数表: {:?}",
        ir.functions.iter().map(|fn_| &fn_.name).collect::<Vec<_>>()
    );
    assert!(
        !has(&ir, "__f_l0"),
        "失去引用者的共享层 __f_l0 必须随原件一起被删（#542 的修复对象）；实际函数表: {:?}",
        ir.functions.iter().map(|fn_| &fn_.name).collect::<Vec<_>>()
    );
    assert!(has(&ir, "f(int64::42)"), "实例必须保留（main 按名调用）");
    assert!(
        has(&ir, "__f_l0(int64::42)"),
        "逐实例克隆必须保留（实例的 MakeClosure 目标）"
    );
    // 直驱生成序列只含本单元函数（嵌入 std 由 driver 侧合并，不在本测试合同面）；
    // 通用死码裁剪行为由 N6/传递死链与接线轮的语料差分承担。
    assert!(
        report.deleted.iter().any(|n| n == "f") && report.deleted.iter().any(|n| n == "__f_l0"),
        "归因清单应恰含被删的原件与共享层；实际 {:?}",
        report.deleted
    );
    assert_references_closed(&before, &ir);
}
/// 红侧负控（直构 IR）：**没有 MakeClosure 站点**的孤儿共享层必须让 verify 红。
///
/// 这是上一条用例退役掉的「删除前红」的可控替身。它钉的性质与语料当年的形态同构、
/// 但不依赖 mono 的删除语义：共享层按 env=1 的帧布局生成（参数落在槽 1），而
/// `verify` 只能从 `MakeClosure` 反推 env 槽数——没有站点就按 env=0 算，
/// 槽 1 的读落在无定义路径上（支配性红）。判据有判别力这件事由此钉住。
#[test]
fn test_orphan_shared_layer_without_site_is_dominance_red() {
    // Arrange：共享层形如 [Move r0 <- r1, Ret]，无人以 MakeClosure 引用它
    let orphan = FunctionIR {
        name: "__f_l0".to_string(),
        def: None,
        params: vec![MonoType::Int(64)],
        return_type: MonoType::Void,
        generic_params: None,
        body: FunctionBody::Code {
            blocks: vec![BasicBlock {
                label: 0,
                instructions: vec![
                    Instruction::Move {
                        dst: Operand::Local(0),
                        src: Operand::Local(1),
                        span: span(),
                    },
                    ret_void(),
                ],
                successors: Vec::new(),
            }],
            entry: 0,
            // 具名槽才是支配性判定的面（temp 槽是帧语义的预初始化，永远不算未定义）：
            // 槽 1 是「env 槽」——真实共享层里它是捕获名，读它但没有指令定义它。
            locals: vec![LocalSlot::temp(MonoType::Int(64)), named_local("n")],
            values: Vec::new(),
            value_slots: Vec::new(),
        },
    };
    let m = module_with(vec![orphan]);

    // Act
    let result = verify_loose(&m);
    let detail = result
        .as_ref()
        .err()
        .map(|e| e.to_string())
        .unwrap_or_default();

    // Assert：负控必须红（且红在支配性，不是别的类别）
    assert!(
        result.is_err(),
        "无 MakeClosure 站点的共享层必须让 verify_loose 红（env 槽数反推失效）：{detail}"
    );
    assert!(
        detail.contains("use-before-def"),
        "红必须落在支配性不变量上（env 反推失效的形态）；实际：{detail}"
    );
}

/// 7b.2：DCE 只删函数——存活函数的 values / value_slots 原样保留且仍可索引。
#[test]
fn test_dce_preserves_values_and_value_slots_of_live_functions() {
    // Arrange：main 调用 live；dead 不可达。live 带一个值 + 槽映射。
    let mut live = code_fn(
        "live",
        vec![Instruction::Ret {
            value: Some(Operand::Value(ValueId(0))),
            span: span(),
        }],
    );
    if let FunctionBody::Code {
        values,
        value_slots,
        ..
    } = &mut live.body
    {
        values.push(ValueInfo {
            ty: MonoType::Int(64),
            debug_name: None,
            defining_block: 0,
        });
        value_slots.push(1);
    }
    let entry = code_fn("main", vec![call_name("live", None), ret_void()]);
    let dead = code_fn("dead", vec![ret_void()]);
    let mut module = module_with(vec![entry, live, dead]);

    // Act
    let report = eliminate(&mut module).expect("无索引目标/无重复名，DCE 应成功");

    // Assert：dead 被删，live 的值表与槽映射完好、ValueId 仍可索引
    assert_eq!(
        names(&module),
        vec!["main".to_string(), "live".to_string()],
        "只允许删除不可达的 dead"
    );
    assert_eq!(report.deleted, vec!["dead".to_string()]);
    let live = module
        .functions
        .iter()
        .find(|f| f.name == "live")
        .expect("live 必须存活");
    assert_eq!(live.values().len(), 1, "存活函数的值表不能被 DCE 破坏");
    assert_eq!(
        live.values()[0].ty,
        MonoType::Int(64),
        "值的类型必须原样保留"
    );
    assert_eq!(
        live.value_slots().to_vec(),
        vec![1],
        "值 → 槽映射必须原样保留"
    );
    assert!(
        live.blocks()[0].instructions.iter().any(|i| matches!(
            i,
            Instruction::Ret {
                value: Some(Operand::Value(ValueId(0))),
                ..
            }
        )),
        "指令里的 ValueId 必须仍在值表范围内"
    );
}
