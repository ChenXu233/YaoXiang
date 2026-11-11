//! 闭项擦除的 post-mono 门禁（RFC-039 D62 / 6.9.2-C2c）
//!
//! # 钉住的四个性质
//!
//! 1. **实例不再有形参侧的 q=0 槽**：`f(int64::42)` / `f(int64::7)` 与它们的逐实例
//!    柯里层克隆里，`LocalSlot.quantity` 没有 `Zero`；实例的 `params` 为空。
//! 2. **调用点实参同位删除**：`main` 里对 `f(int64::42)` 的调用实参数为 0，与实例的
//!    `params.len()` 一致（两侧由同一份计划推出，任何一侧漏删都会在这里红）。
//! 3. **体内读编译期形参 ⇒ 物化成常量**：克隆体里出现 `Load <- Const(42)` / `Const(7)`，
//!    即值来自请求、落点是常量（不是「留一个 q=0 槽继续跑」）。
//! 4. **门禁本身有牙齿**：同一断言套在**未擦除的兜底件**（原件 `factorial` 与共享层
//!    `__factorial_l0`）上会命中 q=0 槽——所以「实例里没有」不是空断言。
//!
//! 为什么不按名字或 `generic_params` 反推「哪些函数是实例」：那是第二份身份判据
//! （实例身份只有 mono 自己知道）。这里直接点名夹具保证存在的实例函数做正向断言，
//! 未擦除的兜底件同样点名断言，反向证明判据会失败。

use crate::ast::Quantity;
use crate::frontend::compiler::Compiler;
use crate::frontend::core::sema::MonoType;
use crate::middle::ir::ir::{ConstValue, FunctionBody, Instruction, LocalSlot, ModuleIR, Operand};
use crate::middle::ir::verify::verify_loose;

/// 体内读 N 的直驱夹具：语料里没有这个形态，没有它物化步就是静默通道。
const BODY_READS_N: &str = concat!(
    "use std.assert\n",
    "f: (N: Int) -> (k: N) -> Int = {\n",
    "    x: Int = N\n",
    "    return x + k\n",
    "}\n",
    "main: () -> Void = {\n",
    "    a: Int = f(42)(42)\n",
    "    assert.assert(a == 84, \"f(42)(42) should be 84\")\n",
    "    b: Int = f(7)(7)\n",
    "    assert.assert(b == 14, \"f(7)(7) should be 14\")\n",
    "}\n",
);

/// 递归 const 泛型（语料同形）：`factorial(k - 1)` 是开项 ⇒ 请求不产生 ⇒ 原件与
/// 共享层作为**未擦除兜底**留在函数表里（它们仍带 q=0 槽，正好当门禁的反向对照）。
const RECURSIVE_FALLBACK: &str = concat!(
    "factorial: (N: Int) -> (k: N) -> Int = {\n",
    "    return match k {\n",
    "        0 => 1,\n",
    "        _ => k * factorial(k - 1)(k - 1)\n",
    "    }\n",
    "}\n",
    "main: () -> Void = {\n",
    "    r: Int = factorial(5)(5)\n",
    "}\n",
);

/// 三层柯里 + 编译期 binder：产生**两级**克隆（`__f_l0` 中间层 / `__f_l1` 最内层），
/// 中间层的 env 槽会进最内层克隆的参数区——单值层夹具碰不到这条记账。
const THREE_LAYER_BOUND: &str = concat!(
    "f: (N: Int) -> (m: Int) -> (k: N) -> Int = {\n",
    "    x: Int = N\n",
    "    return x + m + k\n",
    "}\n",
    "main: () -> Void = {\n",
    "    a: Int = f(5)(6)(5)\n",
    "}\n",
);

/// 编译到 post-mono（擦除发生在 mono 末尾，故这里看到的就是擦除后的形态）。
fn compile_post_mono(source: &str) -> ModuleIR {
    let mut compiler = Compiler::new();
    compiler
        .compile("curry_erasure.yx", source)
        .expect("夹具必须通过编译（擦除不改可编译性）")
}

/// 按名取函数下标（缺失即列出函数表——夹具的函数表是契约的一部分）。
fn function_index(
    ir: &ModuleIR,
    name: &str,
) -> usize {
    ir.functions
        .iter()
        .position(|f| f.name == name)
        .unwrap_or_else(|| {
            let names: Vec<&str> = ir.functions.iter().map(|f| f.name.as_str()).collect();
            panic!("函数表里没有 {name}；实际函数表: {names:?}")
        })
}

/// 该函数的槽表（不是代码体即空表）。
fn slots(
    ir: &ModuleIR,
    name: &str,
) -> Vec<LocalSlot> {
    let index = function_index(ir, name);
    match &ir.functions[index].body {
        FunctionBody::Code { locals, .. } => locals.clone(),
        FunctionBody::TypeDecl { .. } => Vec::new(),
    }
}

/// 该函数是否还留着 q=0 槽（判据唯一来源 = `LocalSlot.quantity`）。
fn has_const_slot(
    ir: &ModuleIR,
    name: &str,
) -> bool {
    slots(ir, name)
        .iter()
        .any(|slot| slot.quantity == Some(Quantity::Zero))
}

/// 具名槽的（下标 → 名字）序列：用来钉「密排、没有留洞」。
fn named_slots(
    ir: &ModuleIR,
    name: &str,
) -> Vec<(usize, String)> {
    slots(ir, name)
        .iter()
        .enumerate()
        .filter_map(|(index, slot)| slot.name.clone().map(|n| (index, n)))
        .collect()
}

/// 形参数。
fn params_len(
    ir: &ModuleIR,
    name: &str,
) -> usize {
    let index = function_index(ir, name);
    ir.functions[index].params.len()
}

/// 体内所有 `Load <- Const` 的常量（物化落点的唯一形态）。
fn materialized_constants(
    ir: &ModuleIR,
    name: &str,
) -> Vec<ConstValue> {
    let index = function_index(ir, name);
    let FunctionBody::Code { blocks, .. } = &ir.functions[index].body else {
        panic!("{name} 不是代码体")
    };
    let mut out = Vec::new();
    for block in blocks {
        for instr in &block.instructions {
            if let Instruction::Load {
                src: Operand::Const(value),
                ..
            } = instr
            {
                out.push(value.clone());
            }
        }
    }
    out
}

/// 某容器里所有指向 `callee` 的静态调用的实参数。
fn call_arg_counts(
    ir: &ModuleIR,
    container: &str,
    callee: &str,
) -> Vec<usize> {
    let index = function_index(ir, container);
    let FunctionBody::Code { blocks, .. } = &ir.functions[index].body else {
        panic!("{container} 不是代码体")
    };
    let mut out = Vec::new();
    for block in blocks {
        for instr in &block.instructions {
            if let Instruction::Call {
                func: Operand::Const(ConstValue::String(target)),
                args,
                ..
            } = instr
            {
                if target == callee {
                    out.push(args.len());
                }
            }
        }
    }
    out
}

/// 该函数作为闭包体时的 env 槽数（`MakeClosure` 的 env 长度）。
///
/// IR 没有 env_len 字段，`verify` 的 FnView 也是从 `MakeClosure` 侧反推——同一来源。
fn env_len_of(
    ir: &ModuleIR,
    name: &str,
) -> usize {
    for func in &ir.functions {
        let FunctionBody::Code { blocks, .. } = &func.body else {
            continue;
        };
        for block in blocks {
            for instr in &block.instructions {
                if let Instruction::MakeClosure {
                    func: target, env, ..
                } = instr
                {
                    if target == name {
                        return env.len();
                    }
                }
            }
        }
    }
    0
}

/// 声明的形参类型表（clone；判据只读不改）。
fn param_types(
    ir: &ModuleIR,
    name: &str,
) -> Vec<MonoType> {
    let index = function_index(ir, name);
    ir.functions[index].params.clone()
}

/// `MonoType` 里出现的全部 `TypeRef` 名（只收容器，其余变体不可能携带名字）。
fn type_ref_names(ty: &MonoType) -> Vec<String> {
    match ty {
        MonoType::TypeRef(name) => vec![name.clone()],
        MonoType::Generic { args, .. } => args.iter().flat_map(type_ref_names).collect(),
        MonoType::Fn {
            params,
            return_type,
        } => params
            .iter()
            .flat_map(type_ref_names)
            .chain(type_ref_names(return_type))
            .collect(),
        MonoType::Ref { inner, .. } | MonoType::Refined { base: inner, .. } => {
            type_ref_names(inner)
        }
        MonoType::Union(items) | MonoType::Intersection(items) => {
            items.iter().flat_map(type_ref_names).collect()
        }
        _ => Vec::new(),
    }
}

/// 声明与槽自洽（C2c 复审 F2 的门禁）。判据只读 `LocalSlot.quantity` 与声明本身，
/// 与既有 q=0 判据同源。
///
/// 「形参数 == 参数区槽数」在本 IR 里等价于「参数区 = 帧铺入的具名绑定」——帧前段是
/// `env 槽 + 形参位`，形参位个数就是声明个数，故真正能失败的是**边界位置**而不是
/// 算术：声明被多裁/少裁/只代入不裁，边界就会落到 temp 或落到未擦除的编译期位。
/// 三条断言：
///
/// ① 声明不超过帧容量（`locals.len() >= env_len + 形参数`）；
/// ② 参数区 `0..env_len+形参数` 每个槽都**具名**且**非 q=0**；
/// ③ 声明形参类型不得提到本函数里 q=0 绑定的名字——那正是「已不存在的编译期
///    泛型名被带进函数表」的形态。
///
/// `ERASE_CONST_PARAMS` 关掉时 ② 必红（env 里的 q=0 槽还在参数区）；只代入不裁声明
/// 时 ①/② 必红（声明的位次压到 temp 上）。
fn assert_declaration_matches_slots(
    ir: &ModuleIR,
    name: &str,
) {
    let env_len = env_len_of(ir, name);
    let params = param_types(ir, name);
    let slots = slots(ir, name);
    let param_region = env_len + params.len();

    // ① 声明不得超过帧容量
    assert!(
        slots.len() >= param_region,
        "{name} 声明了 {param_region} 个帧前段位（env {env_len} + 形参 {}），但槽表只有 {} 个",
        params.len(),
        slots.len()
    );

    // ② 参数区每个槽都是帧铺入的具名绑定，且不是 q=0
    for (index, slot) in slots.iter().take(param_region).enumerate() {
        assert!(
            slot.name.is_some(),
            "{name} 的参数区槽 {index} 无名：帧会往无名槽铺实参（声明与槽裁剪不同源）"
        );
        assert!(
            slot.quantity != Some(Quantity::Zero),
            "{name} 的参数区槽 {index}（{}）仍是 q=0：编译期位没被裁掉，却还占着形参区",
            slot.name.as_deref().unwrap_or("<无名>")
        );
    }

    // ③ 声明类型不得引用本函数的 q=0 绑定名
    let const_names: Vec<String> = slots
        .iter()
        .filter(|slot| slot.quantity == Some(Quantity::Zero))
        .filter_map(|slot| slot.name.clone())
        .collect();
    for ty in &params {
        for referenced in type_ref_names(ty) {
            assert!(
                !const_names.contains(&referenced),
                "{name} 的声明形参类型 {ty:?} 仍引用编译期绑定名 `{referenced}`（实例化后它在本函数内已无绑定）"
            );
        }
    }
}

/// ① 实例与逐实例克隆都不再留 q=0 槽，且实例的形参表为空。
#[test]
fn test_instance_and_clone_have_no_const_param_slot() {
    // Arrange
    let ir = compile_post_mono(BODY_READS_N);

    // Act / Assert：实例确实被建出（点名断言，不靠名字反推身份）
    for name in [
        "f(int64::42)",
        "f(int64::7)",
        "__f_l0(int64::42)",
        "__f_l0(int64::7)",
    ] {
        assert!(
            !has_const_slot(&ir, name),
            "{name} 仍带 q=0 槽（LocalSlot.quantity == Zero）：擦除没发生"
        );
    }
    assert_eq!(
        params_len(&ir, "f(int64::42)"),
        0,
        "闭项实例的形参表必须为空：N 是编译期值，不该占运行期位"
    );
    assert_eq!(
        params_len(&ir, "__f_l0(int64::42)"),
        1,
        "逐实例克隆只留运行期形参 k（N 的 env 位已删）"
    );
}

/// ② 调用点实参与被调实例的形参表一致（任一侧漏删都会不对称）。
#[test]
fn test_call_site_args_match_erased_params() {
    // Arrange
    let ir = compile_post_mono(BODY_READS_N);

    // Act / Assert
    for name in ["f(int64::42)", "f(int64::7)"] {
        let counts = call_arg_counts(&ir, "main", name);
        assert_eq!(
            counts,
            vec![0],
            "{name} 在 main 里应恰有一处调用且实参为 0（期望 [0]，实际 {counts:?}）"
        );
        assert_eq!(
            counts[0],
            params_len(&ir, name),
            "{name} 的调用实参数与形参数必须相等"
        );
    }
}

/// ③ 体内读编译期形参 ⇒ 物化成常量，且值来自**该实例**的请求（42 vs 7）。
#[test]
fn test_const_param_read_is_materialized_as_constant() {
    // Arrange
    let ir = compile_post_mono(BODY_READS_N);

    // Act
    let forty_two = materialized_constants(&ir, "__f_l0(int64::42)");
    let seven = materialized_constants(&ir, "__f_l0(int64::7)");

    // Assert：`x: Int = N` 的 N 读点变成常量（值取自请求，不是 0 / 占位）
    assert!(
        forty_two.contains(&ConstValue::Int(42)),
        "f(int64::42) 的克隆体里应有常量 42（实际 {forty_two:?}）"
    );
    assert!(
        seven.contains(&ConstValue::Int(7)),
        "f(int64::7) 的克隆体里应有常量 7（实际 {seven:?}）"
    );
    assert!(
        !forty_two.contains(&ConstValue::Int(7)),
        "两个实例的编译期值不得串味（42 的实例里出现 7 就说明计划对错了槽）"
    );
}

/// ④ 槽位密排：删掉 N 之后 `k` / `x` 从 0 起连续，没有留洞、也没有凭空 +1。
#[test]
fn test_named_slots_are_dense_after_erasure() {
    // Arrange
    let ir = compile_post_mono(BODY_READS_N);

    // Act
    let named = named_slots(&ir, "__f_l0(int64::42)");

    // Assert
    assert_eq!(
        named,
        vec![(0, "k".to_string()), (1, "x".to_string())],
        "擦除后具名槽必须从 0 起密排（原来 N 占 0、k 占 1、x 占 2）；实际 {named:?}"
    );
}

/// ⑤ 门禁有牙齿：未擦除的兜底件（原件与共享层）仍然带 q=0 槽。
///
/// `factorial(k - 1)` 是开项 ⇒ 请求不产生 ⇒ 原件与共享层保留为**未擦除兜底**，
/// 它们按名被调用、按运行期值执行。同一断言在它们身上必须命中——否则上面那些
/// 「实例里没有 q=0 槽」就是空断言（例如判据恒假时）。
#[test]
fn test_unerased_fallback_still_has_const_slot() {
    // Arrange
    let ir = compile_post_mono(RECURSIVE_FALLBACK);

    // Act / Assert：实例已擦除
    assert!(
        !has_const_slot(&ir, "factorial(int64::5)"),
        "开项闭包的实例 factorial(int64::5) 应已擦除 q=0 形参"
    );
    assert!(
        !has_const_slot(&ir, "__factorial_l0(int64::5)"),
        "逐实例克隆 __factorial_l0(int64::5) 应已擦除 q=0 env 槽"
    );
    // Act / Assert：兜底件未擦除（判据在此**会**命中，证明它不是空断言）
    assert!(
        has_const_slot(&ir, "factorial"),
        "原件 factorial 是未擦除的兜底件，应仍带 q=0 形参槽（开项调用点按原名调它）"
    );
    assert!(
        has_const_slot(&ir, "__factorial_l0"),
        "共享层 __factorial_l0 是未擦除的兜底件，应仍带 q=0 env 槽"
    );
}

/// ⑥ 擦除后的模块仍满足宽松校验（槽位越界 / 支配性 / 跳转目标这些结构性判据）。
///
/// 用递归夹具而不是 [`BODY_READS_N`]：后者两个调用点都被改写，驱动侧的覆盖闸会把
/// 原件 `f` 删掉，于是**共享层 `__f_l0` 失去唯一的 `MakeClosure` 引用**；宽松校验的
/// env 槽数只能从 `MakeClosure` 侧反推（IR 没有 env_len 字段），无人引用的层因此
/// 被判成「env 长度 0」，它自己的第 1 号槽就成了无定义路径。这条与擦除无关（把
/// [`Monomorphizer::ERASE_CONST_PARAMS`] 关掉后同样红），是 C2b 保共享层 + 覆盖闸
/// 删原件共同留下的**孤立死 IR**；处理它属于死代码消除，不在本轮范围。
#[test]
fn test_post_mono_module_passes_loose_verify() {
    // Arrange：原件仍被保留（开项调用点按原名调它），共享层因此仍有引用
    let ir = compile_post_mono(RECURSIVE_FALLBACK);

    // Act
    let result = verify_loose(&ir);

    // Assert：删槽 + 重排 + 入口插常量 Load（含跳转目标整体平移）后仍结构合法
    assert!(
        result.is_ok(),
        "擦除后的 post-mono IR 应通过宽松校验；实际 {:?}",
        result.err()
    );
    // 顺带钉住本夹具里实例确实被擦除（否则本测试只证「模块没坏」）
    assert!(
        !has_const_slot(&ir, "__factorial_l0(int64::5)"),
        "本夹具的实例应已擦除，否则本测试对擦除没有覆盖"
    );
}

/// ⑦（F2）克隆的声明形参类型必须带**该实例**的身份：`k: N` 在 `f(int64::42)` 的克隆
/// 里是 `i64::42`，在 `f(int64::7)` 里是 `i64::7`——不是悬空名 `N`，也不串味。
#[test]
fn test_clone_declaration_types_carry_instance_identity() {
    // Arrange
    let ir = compile_post_mono(BODY_READS_N);

    // Act
    let forty_two = param_types(&ir, "__f_l0(int64::42)");
    let seven = param_types(&ir, "__f_l0(int64::7)");

    // Assert
    assert_eq!(
        forty_two.len(),
        1,
        "克隆只声明一个运行期形参（k）；实际 {forty_two:?}"
    );
    for (ty, want) in [(&forty_two[0], 42i128), (&seven[0], 7i128)] {
        let MonoType::Literal { value, .. } = ty else {
            panic!("克隆的声明形参类型应是单值类型 Literal，实际 {ty:?}");
        };
        // `MonoType::Literal` 的载荷是编译期值（`const_data`），与 IR 常量同名不同型
        assert!(
            matches!(
                value,
                crate::frontend::core::types::const_data::ConstValue::Int(n) if *n == want
            ),
            "克隆的声明形参类型应是该实例的单值类型 int64::{want}，实际 {ty:?}"
        );
    }
}

/// ⑧（F2 门禁）声明与槽自洽：形参数 == 参数区具名非 q=0 槽数、参数区无 q=0 槽、
/// 声明类型不指向编译期绑定名。实例与克隆一起过（实例形参为 0，克隆剩运行期形参）。
///
/// 开关 `ERASE_CONST_PARAMS` 关掉时第 ② 条必红——见本文件头的牙齿说明。
#[test]
fn test_declaration_matches_slots_for_instance_and_clone() {
    // Arrange
    let ir = compile_post_mono(BODY_READS_N);

    // Act / Assert
    for name in [
        "f(int64::42)",
        "f(int64::7)",
        "__f_l0(int64::42)",
        "__f_l0(int64::7)",
    ] {
        assert_declaration_matches_slots(&ir, name);
    }
    // 递归夹具的实例/克隆同样自洽（factorial 是开项递归面的兜底形态）
    let recursive = compile_post_mono(RECURSIVE_FALLBACK);
    for name in ["factorial(int64::5)", "__factorial_l0(int64::5)"] {
        assert_declaration_matches_slots(&recursive, name);
    }
    // 三层夹具：两级克隆。中间层克隆的 env 槽会进最内层的参数区——单值层夹具碰不到
    // 这条记账，而它正是「声明与槽同源裁剪」最容易错位的地方。
    let three = compile_post_mono(THREE_LAYER_BOUND);
    for name in ["f(int64::5)", "__f_l0(int64::5)", "__f_l1(int64::5)"] {
        assert_declaration_matches_slots(&three, name);
    }
    // 最内层克隆的声明形参已代入成该实例的单值类型（`k: N` → `int64::5`）
    let innermost = param_types(&three, "__f_l1(int64::5)");
    assert_eq!(
        innermost.len(),
        1,
        "最内层克隆只声明一个运行期形参（k）；实际 {innermost:?}"
    );
    assert!(
        matches!(innermost[0], MonoType::Literal { .. }),
        "最内层克隆的声明形参类型应已代入成单值类型，实际 {:?}",
        innermost[0]
    );
}
