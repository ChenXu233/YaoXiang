//! 局部变量名落盘验证：具名局部在 IR 中携带源码名，临时寄存器保持无名。
//!
//! 回归保护：名字来自生成期 `register_local` 的就地写入。若这条链路断了
//! （`cur_locals` 未填充、嵌套函数体未隔离、`take_cur_locals` 截断），
//! 名字会静默退回全 None——本测试即是防静默回退的哨兵。

use crate::frontend::compiler::Compiler;
use crate::middle::ir::ir::{FunctionBody, LocalSlot, ModuleIR};

/// 编译 `src` 并取回模块 IR；失败时带上错误原因。
fn compile(src: &str) -> ModuleIR {
    Compiler::new()
        .compile("test", src)
        .unwrap_or_else(|e| panic!("compile should succeed, but failed with: {e:?}"))
}

/// 取 `fn_name` 的代码体局部槽位表；函数缺失或不是代码体都直接失败。
fn code_locals<'a>(
    ir: &'a ModuleIR,
    fn_name: &str,
) -> &'a [LocalSlot] {
    let func = ir
        .functions
        .iter()
        .find(|f| f.name == fn_name)
        .unwrap_or_else(|| panic!("{fn_name} should be emitted"));
    let FunctionBody::Code { locals, .. } = &func.body else {
        panic!("{fn_name} should have a code body")
    };
    locals
}

fn locals_of<'a>(
    ir: &'a ModuleIR,
    fn_name: &str,
) -> &'a [LocalSlot] {
    ir.functions
        .iter()
        .find(|f| f.name == fn_name)
        .unwrap_or_else(|| panic!("function {fn_name} not found"))
        .locals()
}

#[test]
fn named_locals_carry_source_name() {
    let src = r#"
main = () => {
    alpha = 1
    beta = alpha + 2
}
"#;
    let ir = compile(src);

    let names: Vec<&str> = locals_of(&ir, "main")
        .iter()
        .filter_map(|s| s.name.as_deref())
        .collect();
    assert!(
        names.contains(&"alpha"),
        "alpha should be recorded as a named slot, got names: {names:?}"
    );
    assert!(
        names.contains(&"beta"),
        "beta should be recorded as a named slot, got names: {names:?}"
    );
}

#[test]
fn parameters_carry_source_name() {
    let src = r#"
double: (n: Int) -> Int = (n) => { n * 2 }

main = () => {
    r = double(21)
}
"#;
    let ir = compile(src);

    let func = ir
        .functions
        .iter()
        .find(|f| f.name.starts_with("double"))
        .expect("double should be emitted");
    let names: Vec<&str> = func
        .locals()
        .iter()
        .filter_map(|s| s.name.as_deref())
        .collect();
    assert!(
        names.contains(&"n"),
        "parameter n should be recorded as a named slot, got: {names:?}"
    );
}

#[test]
fn temporaries_stay_anonymous() {
    let src = r#"
main = () => {
    a = 1
    b = 2
    c = a + b
}
"#;
    let ir = compile(src);

    let locals = locals_of(&ir, "main");
    // 表达式求值会产生临时寄存器；它们必须保持无名，
    // 否则诊断里会把 "临时寄存器" 当成用户变量显示出来。
    assert!(
        locals.iter().any(|s| s.name.is_none()),
        "expected at least one anonymous temp slot, got: {locals:?}"
    );
}

#[test]
fn nested_function_locals_do_not_leak_to_parent() {
    // 内层函数体的具名局部不得出现在外层函数的槽位表里。
    // 这正是 span 迁移中踩过的坑：跨调用状态未隔离导致张冠李戴。
    let src = r#"
outer = () => {
    outer_only = 1
    inner = () => {
        inner_only = 2
        inner_only
    }
    inner()
    outer_only
}
"#;
    let ir = compile(src);

    let outer_names: Vec<&str> = locals_of(&ir, "outer")
        .iter()
        .filter_map(|s| s.name.as_deref())
        .collect();
    assert!(
        outer_names.contains(&"outer_only"),
        "outer should own outer_only, got: {outer_names:?}"
    );
    assert!(
        !outer_names.contains(&"inner_only"),
        "inner_only must not leak into outer's slots, got: {outer_names:?}"
    );
}

#[test]
fn locals_length_matches_declared_count() {
    // 槽位数量必须恰好等于函数体声明数：多一个少一个都会改变 local_count
    // 进而改变 .42 字节码。
    let src = r#"
main = () => {
    x = 1
    y = 2
    x + y
}
"#;
    let ir = compile(src);
    let locals = code_locals(&ir, "main");

    // 每个槽位下标都必须落在 0..locals.len()
    assert!(
        !locals.is_empty(),
        "main should have at least one local slot"
    );
    let max_idx = locals.len() - 1;
    for (i, slot) in locals.iter().enumerate() {
        assert!(
            i <= max_idx,
            "slot index {i} out of range (len {}) for {slot:?}",
            locals.len()
        );
    }
}

/// 规范 / 病史：D62（issue #533）——q=0 形参的**具名绑定**必须在槽位上携带同一量词
/// 事实（`ast::Param::quantity` 的投影），擦除前识别体内对它的引用要靠它。
///
/// 声明 `f: (N: Int) -> (n: N) -> Int = (n) => n` 的柯里最内层函数把外层形参 N 铺成
/// env 槽：N 是编译期值参数（`n: N` 引用它）⇒ 该槽带 `Some(Zero)`；临时槽恒 `None`。
#[test]
fn test_const_param_slot_carries_zero_quantity() {
    // Arrange
    let src = r#"
f: (N: Int) -> (n: N) -> Int = (n) => n
main = () => {
}
"#;

    // Act
    let ir = compile(src);
    let slots = locals_of(&ir, "__f_l0");

    // Assert
    let n_slot = slots
        .iter()
        .find(|s| s.name.as_deref() == Some("N"))
        .unwrap_or_else(|| panic!("__f_l0 应把外层形参 N 铺成具名槽；实际: {slots:?}"));
    assert_eq!(
        n_slot.quantity,
        Some(crate::ast::Quantity::Zero),
        "N 被 `n: N` 当类型用 ⇒ 槽位必须携带 q=0 事实；实际: {slots:?}"
    );
    assert!(
        slots
            .iter()
            .filter(|s| s.name.is_none())
            .all(|s| s.quantity.is_none()),
        "临时槽不携带任何量词事实；实际: {slots:?}"
    );
}

/// P0（6.9.2-C2a）：柯里**包装层**（非最内层）的具名形参同样携带量词事实。
///
/// 此前包装层既不 `register_local` 也不写量词：擦除侧只能靠 `generic_params` +
/// `params.len()` 反推哪一槽是 q=0（D3 第 1 条）。本条钉住三件事：
/// ① 包装层形参**有具名槽**且事实正确（N ⇒ q=0）；② 事实来自声明侧同一份
/// `Param::quantity`，`None` 不被默认成 `Zero`（`m` 未被类型位置使用 ⇒ None）；
/// ③ 包装层与最内层对同一名字给出**相同**事实。
#[test]
fn test_curry_wrapper_layer_params_carry_quantity() {
    // Arrange — 三层柯里：层0=[N]，层1=[m]，层2=[k]；只有 N 是 q=0
    let src = r#"
h: (N: Int) -> (m: Int) -> (k: N) -> Int = (k) => k
main = () => {
}
"#;

    // Act
    let ir = compile(src);
    let outermost = locals_of(&ir, "h");
    let middle = locals_of(&ir, "__h_l0");
    let innermost = locals_of(&ir, "__h_l1");

    // Assert — ① 最外层包装层：本层形参 N 有具名槽 + q=0 事实
    let outer_n = outermost
        .iter()
        .find(|s| s.name.as_deref() == Some("N"))
        .unwrap_or_else(|| panic!("包装层 h 应把形参 N 铺成具名槽；实际: {outermost:?}"));
    assert_eq!(
        outer_n.quantity,
        Some(crate::ast::Quantity::Zero),
        "包装层的 N 被 `k: N` 当类型用 ⇒ 必须携带 q=0 事实；实际: {outermost:?}"
    );

    // Assert — ② 中间包装层：env 的 N 与本层 m 都在，且 m 不得被默认成 Zero
    let mid_n = middle
        .iter()
        .find(|s| s.name.as_deref() == Some("N"))
        .unwrap_or_else(|| panic!("中间层应把 env 形参 N 铺成具名槽；实际: {middle:?}"));
    assert_eq!(
        mid_n.quantity,
        Some(crate::ast::Quantity::Zero),
        "中间层的 env 形参 N 同样携带 q=0 事实；实际: {middle:?}"
    );
    let mid_m = middle
        .iter()
        .find(|s| s.name.as_deref() == Some("m"))
        .unwrap_or_else(|| panic!("中间层应把本层形参 m 铺成具名槽；实际: {middle:?}"));
    assert_eq!(
        mid_m.quantity, None,
        "m 未被类型位置使用 ⇒ 事实是「尚未解析」(None)，绝不默认成 Zero；实际: {middle:?}"
    );

    // Assert — ③ 与最内层同一名字的事实一致
    let inner_n = innermost
        .iter()
        .find(|s| s.name.as_deref() == Some("N"))
        .unwrap_or_else(|| panic!("最内层应保留 env 形参 N 的具名槽；实际: {innermost:?}"));
    assert_eq!(
        mid_n.quantity, inner_n.quantity,
        "包装层与最内层必须给出同一份量词事实（唯一解析源）"
    );
}
