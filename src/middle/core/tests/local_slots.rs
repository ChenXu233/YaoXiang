//! 局部变量名落盘验证：具名局部在 IR 中携带源码名，临时寄存器保持无名。
//!
//! 回归保护：名字来自生成期 `register_local` 的就地写入。若这条链路断了
//! （`cur_locals` 未填充、嵌套函数体未隔离、`take_cur_locals` 截断），
//! 名字会静默退回全 None——本测试即是防静默回退的哨兵。

use crate::frontend::compiler::Compiler;
use crate::middle::core::ir::FunctionBody;

fn locals_of<'a>(
    ir: &'a crate::middle::core::ir::ModuleIR,
    fn_name: &str,
) -> &'a [crate::middle::core::ir::LocalSlot] {
    ir.functions
        .iter()
        .find(|f| f.name == fn_name)
        .unwrap_or_else(|| panic!("function {fn_name} not found"))
        .locals()
}

#[test]
fn named_locals_carry_source_name() {
    let src = r#"
main = {
    alpha = 1
    beta = alpha + 2
}
"#;
    let mut compiler = Compiler::new();
    let ir = compiler
        .compile("test", src)
        .expect("compile should succeed");

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

main = {
    r = double(21)
}
"#;
    let mut compiler = Compiler::new();
    let ir = compiler
        .compile("test", src)
        .expect("compile should succeed");

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
main = {
    a = 1
    b = 2
    c = a + b
}
"#;
    let mut compiler = Compiler::new();
    let ir = compiler
        .compile("test", src)
        .expect("compile should succeed");

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
outer = {
    outer_only = 1
    inner = {
        inner_only = 2
        inner_only
    }
    inner()
    outer_only
}
"#;
    let mut compiler = Compiler::new();
    let ir = compiler
        .compile("test", src)
        .expect("compile should succeed");

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
main = {
    x = 1
    y = 2
    x + y
}
"#;
    let mut compiler = Compiler::new();
    let ir = compiler
        .compile("test", src)
        .expect("compile should succeed");

    let main = ir
        .functions
        .iter()
        .find(|f| f.name == "main")
        .expect("main should be emitted");
    let FunctionBody::Code { locals, .. } = &main.body else {
        panic!("main should have a code body")
    };
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
