//! RFC-039 D20 / WBS 4.5.2：`overload_resolutions` 键 = 调用点 `PlanId`
//! —— 决议形态覆盖与消费必达
//!
//! 规范来源:
//! - docs/src/rfc/accepted/039-compiler-architecture.md D20（Span 键控 → `PlanId`：
//!   `ReleasePlan` / `overload_resolutions` 的键改为随 AST 节点走的身份）
//! - RFC-011a §3（方法级重载：调用点按 实参兼容 + 期望返回 决议候选，IR 定义侧按
//!   声明序混编 `Temp.to` / `Temp.to#1`）
//!
//! # 键的同一性
//!
//! 产出（`inference/expressions.rs` 的 `infer_call_expr`）与消费（`ir_gen` 的两处
//! 方法调用命名）都读**同一个 `Expr::Call` 节点**上的 `plan_id`——不再靠「Call 的
//! span 两处各算一遍」的位置巧合。未分配身份（`PlanId::UNPLANNED`，解析器之外的
//! 合成调用）**不可被消费**（`get` 返回 `None`），故任何「合成调用进了决议表」都会
//! 以未消费条目在义务账本结算处响亮（W1090），不会静默落回未混编名。
//!
//! # 决议形态覆盖清单（来源：本文件逐条钉板）
//!
//! | 形态 | 产出条目 | 消费 | 钉板用例 |
//! | --- | --- | --- | --- |
//! | 期望返回过滤（匿名绑定双候选） | 2 | ✓ 两个混编名都进 IR | `expected_return` |
//! | 位置实参兼容过滤 | 1 | ✓ 决议出的混编名进 IR | `positional_arg` |
//! | 类型体内绑定（`pick = fn[0]`，有绑定消费点） | 1 | ✓ 决议名替换绑定名 | `type_body_binding` |
//! | `?`/Try 操作数中的调用 | 1 | ✓ | `try_operand` |
//! | 命名实参（纯命名） | 1 | ✓ 决议 + 消费（#452） | `named_args` |
//! | 命名实参（位置 + 命名混合） | 1 | ✓ 决议 + 消费（#452） | `mixed_args` |
//! | 歧义（无期望上下文） | 0 | —（E1107 报错） | `ambiguous` |
//! | 变体构造 `Color.red()` | 0 | —（走变体构造路径，另在相位） | `variant_ctor` |
//! | 未分配身份（合成调用） | 不可消费 | —（`get` 拒绝，结算响亮） | `unplanned_key` |
//!
//! 命名实参两条形态的语义边界（#452 修订）：命名实参按**形参名**绑定到候选形参位后
//! 与位置实参一起参与匹配 ⇒ 两条形态都产出条目且都被消费。改动前的实测红态：
//! `r: Int = t.pick(1, a = 2)` 决议候选被丢弃、落回默认绑定（静默给错答案）；
//! 4.5.2 把消费门落成响亮（W1090）；#452 把决议与消费两端一起补齐，
//! 冲突/缺参/未知名/重复命名一律编译期报错（不得静默择一）。

use crate::frontend::core::lexer::tokenize;
use crate::frontend::core::parser::parse;
use crate::frontend::core::typecheck::checker::TypeChecker;
use crate::frontend::core::typecheck::types::OverloadResolutions;
use crate::frontend::module::registry::ModuleRegistry;
use crate::middle::core::ir::Instruction;
use crate::middle::core::ir_gen::AstToIrGenerator;
use crate::util::plan_id::PlanId;

/// 真实链路探针结果：诊断码 / 决议条目数 / 未消费条目 / IR 里的被调名（排序后）。
struct OverloadProbe {
    diagnostics: Vec<String>,
    entries: usize,
    unconsumed: usize,
    call_targets: Vec<String>,
}

/// 源码 → lex→parse→typecheck→ir_gen 真实序列的探针（与语料判据同一生成路径）。
fn probe_overload(source: &str) -> OverloadProbe {
    let tokens = tokenize(source).expect("夹具必须可 tokenize");
    let parsed = parse(&tokens);
    assert!(
        !parsed.has_errors,
        "夹具必须无语法错误: {:?}",
        parsed.errors
    );
    let mut checker = TypeChecker::new("<overload-probe>");
    let type_result = checker.check_module(&parsed.module);
    let diagnostics: Vec<String> = type_result
        .diagnostics
        .iter()
        .map(|d| d.code.clone())
        .collect();
    let entries = type_result.overload_resolutions.len();
    let mut generator =
        AstToIrGenerator::new_with_type_result(&type_result, ModuleRegistry::with_std(), None);
    let ir = generator.generate_module_ir(&parsed.module);
    let unconsumed = type_result.overload_resolutions.unconsumed().len();
    let mut call_targets: Vec<String> = match &ir {
        Ok(ir) => ir
            .functions
            .iter()
            .flat_map(|func| {
                func.all_instructions()
                    .filter_map(|inst| match inst {
                        Instruction::Call {
                            func:
                                crate::middle::core::ir::Operand::Const(
                                    crate::middle::core::ir::ConstValue::String(name),
                                ),
                            ..
                        } => Some(name.clone()),
                        _ => None,
                    })
                    .collect::<Vec<String>>()
            })
            .collect(),
        Err(errors) => vec![format!(
            "<IR 错误 {:?}>",
            errors.iter().map(|d| d.code.clone()).collect::<Vec<_>>()
        )],
    };
    call_targets.sort();
    OverloadProbe {
        diagnostics,
        entries,
        unconsumed,
        call_targets,
    }
}

const EXPECTED_RETURN_FIXTURE: &str = r#"
Temp: Type = { value: Float }
Temp.to: (self: &Temp) -> Float = (self) => { return self.value }
Temp.to: (self: &Temp) -> String = (self) => { return "hot" }
main: () -> Void = {
    t = Temp(1.0)
    a: Float = t.to()
    b: String = t.to()
}
main()
"#;

/// 期望返回过滤：两个候选各被一次调用决议命中——两条条目都必须被消费，
/// 且决议出的两个混编名真的进 IR（消费不是记账空转）。
#[test]
fn test_overload_resolution_expected_return_is_consumed_by_call_identity() {
    // Arrange / Act：真实链路（含两处带注解期望返回的调用）
    let probe = probe_overload(EXPECTED_RETURN_FIXTURE);

    // Assert
    assert!(
        probe.diagnostics.is_empty(),
        "夹具必须通过类型检查: {:?}",
        probe.diagnostics
    );
    assert_eq!(probe.entries, 2, "两次调用各决议一条（期望返回区分候选）");
    assert_eq!(
        probe.unconsumed, 0,
        "决议条目必须全部被 IR 生成消费（键同源自调用节点）"
    );
    assert!(
        probe.call_targets.contains(&"Temp.to".to_string())
            && probe.call_targets.contains(&"Temp.to#1".to_string()),
        "两个混编名都必须真的进 IR（消费不得空转）: {:?}",
        probe.call_targets
    );
}

const POSITIONAL_FIXTURE: &str = r#"
Temp: Type = { v: Int }
Temp.pick: (self: &Temp, a: Int) -> Int = (self, a) => { return a }
Temp.pick: (self: &Temp, a: String) -> String = (self, a) => { return a }
main: () -> Void = {
    t = Temp(1)
    r: Int = t.pick(1)
}
main()
"#;

/// 位置实参兼容过滤：只有 Int 候选匹配（String 候选被 unify 过滤）——条目被消费，
/// IR 用候选 0 的混编名（= 键本身）。
#[test]
fn test_overload_resolution_positional_arg_is_consumed() {
    // Arrange / Act
    let probe = probe_overload(POSITIONAL_FIXTURE);

    // Assert
    assert!(
        probe.diagnostics.is_empty(),
        "夹具必须通过类型检查: {:?}",
        probe.diagnostics
    );
    assert_eq!(probe.entries, 1, "实参类型只匹配一个候选");
    assert_eq!(probe.unconsumed, 0, "条目必须被消费");
    assert!(
        probe.call_targets.contains(&"Temp.pick".to_string()),
        "应调用决议出的候选 0 混编名: {:?}",
        probe.call_targets
    );
}

const TYPE_BODY_BINDING_FIXTURE: &str = r#"
fa: (self: &Temp, a: Int) -> Int = (self, a) => { return a }
fb: (self: &Temp, a: String) -> String = (self, a) => { return a }
Temp: Type = {
    v: Int,
    pick = fa[0],
    pick = fb[0],
}
main: () -> Void = {
    t = Temp(1)
    r: Int = t.pick(1)
}
main()
"#;

/// 类型体内绑定（RFC-004 `pick = fa[0]`，ir_gen 的**有绑定**消费点）：决议名替换
/// 后定义覆盖的绑定名——条目被消费，IR 调用决议出的混编名而非后定义的 `fb`。
#[test]
fn test_overload_resolution_type_body_binding_is_consumed() {
    // Arrange / Act
    let probe = probe_overload(TYPE_BODY_BINDING_FIXTURE);

    // Assert
    assert!(
        probe.diagnostics.is_empty(),
        "夹具必须通过类型检查: {:?}",
        probe.diagnostics
    );
    assert_eq!(probe.entries, 1, "类型体内两条不同签名绑定 = 重载候选");
    assert_eq!(probe.unconsumed, 0, "有绑定消费点必须消费条目");
    assert!(
        probe.call_targets.contains(&"Temp.pick".to_string()),
        "应调用决议出的混编名（不是后定义的 fb）: {:?}",
        probe.call_targets
    );
}

const TRY_OPERAND_FIXTURE: &str = r#"
use std.option
Temp: Type = { v: Int }
Temp.get: (self: &Temp, s: String) -> Option(Int) = (self, s) => { return Option(Int).some(1) }
Temp.get: (self: &Temp, s: Int) -> Option(Int) = (self, s) => { return Option(Int).some(2) }
f: (t: &Temp) -> Option(Int) = (t) => {
    v = t.get("x")?
    return Option(Int).some(v)
}
main: () -> Void = { t = Temp(1) }
main()
"#;

/// `?`/Try 操作数里的方法调用：调用节点仍是决议键的主人——条目被消费。
/// （实参取 string 以与首候选一一对应：两候选若都相容会命中 E1107 歧义，
/// 那是正确行为但不是本条要钉的形态。）
#[test]
fn test_overload_resolution_try_operand_is_consumed() {
    // Arrange / Act
    let probe = probe_overload(TRY_OPERAND_FIXTURE);

    // Assert
    assert!(
        probe.diagnostics.is_empty(),
        "夹具必须通过类型检查: {:?}",
        probe.diagnostics
    );
    assert_eq!(probe.entries, 1, "Try 操作数里的调用同样参与决议");
    assert_eq!(probe.unconsumed, 0, "条目必须被消费");
    assert!(
        probe.call_targets.contains(&"Temp.get".to_string()),
        "应调用 String 候选（裸名首定义）: {:?}",
        probe.call_targets
    );
}

const MIXED_ARGS_FIXTURE: &str = r#"
Temp: Type = { v: Int }
Temp.pick: (self: &Temp, n: Int, a: Int) -> Int = (self, n, a) => { return n * 100 + a }
Temp.pick: (self: &Temp, n: String, a: Int) -> Int = (self, n, a) => { return a }
main: () -> Void = {
    t = Temp(1)
    r: Int = t.pick(1, a = 2)
}
main()
"#;

/// 位置 + 命名混合实参（#452 后）：命名实参加入候选匹配 ⇒ 决议唯一命中 Int 候选，
/// 且 ir_gen 的消费点不再带 `named_args.is_empty()` 门 ⇒ 条目被真实消费。
/// 改动前该形态是**静默给错答案**（决议出的候选被丢弃、落回默认绑定），
/// 4.5.2 曾把消费门落成响亮 W1090；#452 把两端一起补齐。
#[test]
fn test_overload_resolution_mixed_named_args_is_consumed() {
    // Arrange / Act
    let probe = probe_overload(MIXED_ARGS_FIXTURE);

    // Assert
    assert!(
        probe.diagnostics.is_empty(),
        "混合实参夹具必须通过类型检查: {:?}",
        probe.diagnostics
    );
    assert_eq!(probe.entries, 1, "位置 + 命名实参共同决议出一条条目");
    assert_eq!(
        probe.unconsumed, 0,
        "消费门取消 ⇒ 决议出的混编名必须被真实消费（#452）"
    );
    assert!(
        probe.call_targets.contains(&"Temp.pick".to_string()),
        "应命中首候选（n: Int，与位置实参 1 相容）: {:?}",
        probe.call_targets
    );
    assert!(
        !probe.call_targets.contains(&"Temp.pick#1".to_string()),
        "不得调用 n: String 的候选（位置实参 1 是 Int）: {:?}",
        probe.call_targets
    );
}

const AMBIGUOUS_FIXTURE: &str = r#"
Temp: Type = { value: Float }
Temp.to: (self: &Temp) -> Float = (self) => { return self.value }
Temp.to: (self: &Temp) -> String = (self) => { return "hot" }
main: () -> Void = {
    t = Temp(1.0)
    x = t.to()
}
main()
"#;

/// 无期望上下文的歧义：E1107 报错、不产出条目（不该隐式的地方不隐式）。
#[test]
fn test_overload_resolution_ambiguous_call_is_rejected_without_entry() {
    // Arrange / Act
    let probe = probe_overload(AMBIGUOUS_FIXTURE);

    // Assert
    assert!(
        probe.diagnostics.contains(&"E1107".to_string()),
        "歧义必须报 E1107: {:?}",
        probe.diagnostics
    );
    assert_eq!(probe.entries, 0, "歧义不得产出决议条目");
}

const NAMED_ONLY_FIXTURE: &str = r#"
Temp: Type = { v: Int }
Temp.pick: (self: &Temp, a: Int) -> Int = (self, a) => { return a * 10 }
Temp.pick: (self: &Temp, a: String) -> Int = (self, a) => { return 0 }
main: () -> Void = {
    t = Temp(1)
    s: Int = t.pick(a = 2)
}
main()
"#;

/// 纯命名实参（#452 后）：命名实参按形参名绑定后与位置实参一起匹配 ⇒ 决议命中
/// Int 候选并被消费。改动前不产出条目、按默认绑定检查（名字与候选无关）。
#[test]
fn test_overload_resolution_named_only_is_consumed() {
    // Arrange / Act
    let probe = probe_overload(NAMED_ONLY_FIXTURE);

    // Assert
    assert!(
        probe.diagnostics.is_empty(),
        "夹具必须通过类型检查: {:?}",
        probe.diagnostics
    );
    assert_eq!(probe.entries, 1, "纯命名实参参与决议（#452）");
    assert_eq!(probe.unconsumed, 0, "条目必须被消费");
    assert!(
        probe.call_targets.contains(&"Temp.pick".to_string()),
        "实参名 a 与两个候选都相容，而实参类型 Int 只与首候选相容: {:?}",
        probe.call_targets
    );
}

const VARIANT_CTOR_FIXTURE: &str = r#"
Color: Type = { red: () -> Color, blue: () -> Color }
main: () -> Void = {
    c = Color.red()
    d = Color.blue()
}
main()
"#;

/// 变体构造调用：不产出决议条目（走变体构造路径，不在重载候选表里）——
/// 「无条目」是这一形态不构成欠消费的证据。
#[test]
fn test_overload_resolution_variant_ctor_produces_no_entry() {
    // Arrange / Act
    let probe = probe_overload(VARIANT_CTOR_FIXTURE);

    // Assert
    assert!(
        probe.diagnostics.is_empty(),
        "夹具必须通过类型检查: {:?}",
        probe.diagnostics
    );
    assert_eq!(probe.entries, 0, "变体构造不产生重载决议条目");
    assert_eq!(probe.unconsumed, 0, "无条目即无欠消费");
}

/// 未分配身份（合成调用）**不可被消费**：`get` 拒绝、条目留在差集里——
/// 「消费侧遇未分配必须响亮」的单元级席位（响亮由义务账本结算给出）。
#[test]
fn test_overload_resolution_unplanned_key_is_never_consumed() {
    // Arrange：解析器之外的合成调用节点（UNPLANNED）上登记一条决议
    let mut table = OverloadResolutions::default();
    table.resolve(PlanId::UNPLANNED, "Temp.synth".to_string());

    // Act
    let consumed = table.get(PlanId::UNPLANNED);
    let unconsumed = table.unconsumed();

    // Assert
    assert!(
        consumed.is_none(),
        "未分配身份不得被消费（否则合成调用会静默取到别人的决议）"
    );
    assert_eq!(
        unconsumed,
        vec![PlanId::UNPLANNED],
        "未分配身份的条目必须留在差集里（结算据此响亮）"
    );
}
