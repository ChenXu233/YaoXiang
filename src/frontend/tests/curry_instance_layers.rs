//! 逐实例柯里层与实例容器身份（RFC-039 D62 / 6.9.2-C2b，P1 + P2）
//!
//! # 钉住的两个性质
//!
//! 1. **逐实例包装层**：泛型柯里函数的包装层（`__k2_l0` 这类）按实例各克隆一份
//!    （`__k2_l0(int64)` / `__k2_l0(string)`），实例体的 `MakeClosure` 指向**自己**
//!    的克隆，不指向共享层。
//! 2. **克隆体内的调用点落到该实例的被调实例**：`__k2_l0(int64)` 调 `mk(int64)`、
//!    `__k2_l0(string)` 调 `mk(string)`——容器维度是「这段代码所属的实例」，不是
//!    IR 函数名（P2）。
//!
//! ## 为什么断言在**字节码**上
//!
//! IR 名字被改写不等于调用真的换了目标：`Instruction::Call` / `MakeClosure` 同时携带
//! DefId 与名字，翻译器优先 DefId。6.9.2-C2b 之前实测：`factorial(5)(5)` 的调用点
//! 名字已改成 `factorial(int64::5)`，字节码仍指向原件 `factorial`——特化实例是死码，
//! 语料靠擦除后的原件跑通。断言 IR 名字的测试对这一半**没有牙齿**；断言「调用落到
//! 哪个函数表下标」才是性质本身（换一种实现方式——改 def 或改名字——都不会误红）。
//!
//! ## 夹具为什么是「两条值层」
//!
//! 类型层在 IR 层已擦除（`A: Type` 不占运行期槽），柯里包装层只在**值层 ≥ 2** 时出现
//! （`ir_gen::generate_curry_function_ir` 按值层切层）；单值层的泛型函数直接就是目标
//! 函数本身，没有包装层可克隆。

use crate::frontend::compiler::Compiler;
use crate::middle::bytecode::file::{BytecodeInstruction, FunctionCode};
use crate::middle::bytecode::opcode;
use crate::middle::passes::codegen::translator::Translator;

/// 两条值层 + 一个类型参数的泛型柯里函数：`k2(Int)(1)(5)` 与 `k2(String)(2)("s")`
/// 产生两个实例，各自的包装层内部都要调**自己那一份** `mk`。
const TWO_VALUE_LAYER_GENERIC: &str = concat!(
    "mk: (A: Type) -> (x: A) -> A = x\n",
    "k2: (A: Type) -> (a: Int) -> (x: A) -> A = { mk(x) }\n",
    "main: () -> Void = {\n",
    "    a: Int = k2(Int)(1)(5)\n",
    "    b: String = k2(String)(2)(\"s\")\n",
    "}\n",
);

/// 语料同形（`tests/yaoxiang/02-type-system/generics/const_generic_recursive_factorial.yx`）：
/// 体内递归调用的编译期实参是**开项** `k - 1`。
const RECURSIVE_CONST_GENERIC: &str = concat!(
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

/// 编译到字节码函数表（编译管线 → 单态化 → 翻译，与 `dump` 同一条路）。
fn compile_to_bytecode(source: &str) -> Vec<FunctionCode> {
    let mut compiler = Compiler::new();
    let ir = compiler
        .compile("curry_instance_layers.yx", source)
        .expect("夹具必须通过编译（本批只改 emit 形状，不改可编译性）");
    Translator::new()
        .translate_module(&ir)
        .expect("翻译不应失败")
        .code_section
        .functions
}

/// 函数名 → 字节码函数表下标（`CallStatic` / `MakeClosure` 的操作数就是这个下标）。
fn index_of(
    functions: &[FunctionCode],
    name: &str,
) -> usize {
    functions
        .iter()
        .position(|f| f.name == name)
        .unwrap_or_else(|| {
            let names: Vec<&str> = functions.iter().map(|f| f.name.as_str()).collect();
            panic!("函数表里没有 {name}；实际函数表: {names:?}")
        })
}

/// 取某函数里第一条指定 opcode 的静态目标下标。
///
/// `CallStatic` 与 `MakeClosure` 的操作数布局都是 `dst(1) + func_id(4, LE) + …`，
/// 故一处解码两种指令。
fn static_target(
    functions: &[FunctionCode],
    owner: &str,
    opcode_want: u8,
) -> u32 {
    let func = &functions[index_of(functions, owner)];
    let instr = func
        .instructions
        .iter()
        .find(|i| i.opcode == opcode_want)
        .unwrap_or_else(|| {
            panic!(
                "{owner} 里没有 opcode {opcode_want:#04x} 的指令；实际指令: {:?}",
                func.instructions
                    .iter()
                    .map(|i| i.opcode)
                    .collect::<Vec<_>>()
            )
        });
    decode_func_id(instr)
}

/// `dst(1) + func_id(4, LE)` ⇒ 函数表下标。
fn decode_func_id(instr: &BytecodeInstruction) -> u32 {
    let bytes = &instr.operands[1..5];
    u32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]])
}

/// P1：包装层按实例各克隆一份（共享层只有在「实例 × 层」两两不同时才有意义）。
#[test]
fn test_curry_wrapper_layer_is_cloned_per_instance() {
    // Arrange / Act
    let functions = compile_to_bytecode(TWO_VALUE_LAYER_GENERIC);

    // Assert
    let names: Vec<&str> = functions.iter().map(|f| f.name.as_str()).collect();
    assert!(
        names.contains(&"__k2_l0(int64)"),
        "逐实例包装层 __k2_l0(int64) 必须存在（P1：包装层按实例克隆）；实际函数表: {names:?}"
    );
    assert!(
        names.contains(&"__k2_l0(string)"),
        "逐实例包装层 __k2_l0(string) 必须存在（P1：包装层按实例克隆）；实际函数表: {names:?}"
    );
}

/// P1 + P2 的第一半：实例体的 `MakeClosure` 指向**自己的**包装层克隆。
#[test]
fn test_instance_make_closure_points_to_its_own_layer_clone() {
    // Arrange
    let functions = compile_to_bytecode(TWO_VALUE_LAYER_GENERIC);

    // Act / Assert
    for (instance, layer) in [
        ("k2(int64)", "__k2_l0(int64)"),
        ("k2(string)", "__k2_l0(string)"),
    ] {
        let want = index_of(&functions, layer) as u32;
        let got = static_target(&functions, instance, opcode::MAKE_CLOSURE);
        assert_eq!(
            got, want,
            "实例 {instance} 的 MakeClosure 必须指向它自己的包装层克隆 {layer}（下标 {want}），\
             不是共享层、也不是原件——否则该实例的闭包走共享层，逐实例改写全部落空"
        );
    }
}

/// P2：克隆体内的调用点落到**该实例**的被调实例（不是原件、也不是另一实例）。
///
/// 这条同时是 DefId 口径的牙齿：只改名字不清 DefId 时，翻译器按陈旧的 def 分派，
/// 两个克隆都会指回原件 `mk`——断言失败。
#[test]
fn test_layer_clone_calls_the_instance_of_its_owner() {
    // Arrange
    let functions = compile_to_bytecode(TWO_VALUE_LAYER_GENERIC);

    // Act / Assert
    for (layer, callee) in [
        ("__k2_l0(int64)", "mk(int64)"),
        ("__k2_l0(string)", "mk(string)"),
    ] {
        let want = index_of(&functions, callee) as u32;
        let got = static_target(&functions, layer, opcode::CALL_STATIC);
        assert_eq!(
            got, want,
            "包装层克隆 {layer} 的调用点必须落到 {callee}（下标 {want}），\
             不是原件 mk、也不是另一实例的 mk"
        );
    }
}

/// tripwire（**当前事实**，勿读成「正确行为」）：开项递归调用的实例化仍未接通。
///
/// `factorial(k - 1)` 的编译期实参是**开项**（自由名 `k` 只在本帧内成立），
/// 6.9.2-C1 只让**闭项**进实例化请求（`collect_const_arg_instantiation_request`：
/// 开项 ⇒ `Evidenced(None)`），故 `factorial(int64::4)` **不存在**，逐实例包装层的
/// 内部调用点仍指向原件。
///
/// 这是 C3（开项开火 + `fire_deferred` 归一化）的剩余面，不是 P1/P2 的缺陷：
/// P1/P2 提供的是「逐实例层存在且可被独立改写」，开项接通后本条的断言必须改成
/// 「指向 `factorial(int64::4)`」并删掉本 tripwire。
#[test]
fn test_open_term_recursive_call_is_currently_not_specialized() {
    // Arrange
    let functions = compile_to_bytecode(RECURSIVE_CONST_GENERIC);
    let names: Vec<&str> = functions.iter().map(|f| f.name.as_str()).collect();

    // Assert 1：逐实例包装层已存在（P1 已生效）
    assert!(
        names.contains(&"__factorial_l0(int64::5)"),
        "逐实例包装层 __factorial_l0(int64::5) 应已存在（P1）；实际函数表: {names:?}"
    );
    // Assert 2：tripwire——开项实例尚未生成
    assert!(
        !names.iter().any(|n| n.starts_with("factorial(int64::4")),
        "tripwire 失效：开项递归已开始产生实例（C3 已落地）⇒ 请把本测试改成\
         「克隆的调用点指向 factorial(int64::4)」并删掉 tripwire；实际函数表: {names:?}"
    );
    // Assert 3：于是克隆的调用点仍指原件（不是「指向了别的实例」）
    let original = index_of(&functions, "factorial") as u32;
    let got = static_target(&functions, "__factorial_l0(int64::5)", opcode::CALL_STATIC);
    assert_eq!(
        got, original,
        "开项未接通时，逐实例包装层的调用点应仍指向原件 factorial（下标 {original}）——\
         指向任何实例都说明本 tripwire 已过期"
    );
}

/// 负控：单值层的泛型函数**没有**包装层可克隆，不应凭空多出 `__mk_l0` 之类函数。
#[test]
fn test_single_value_layer_generic_is_not_cloned() {
    // Arrange / Act
    let functions = compile_to_bytecode(TWO_VALUE_LAYER_GENERIC);

    // Assert
    let names: Vec<&str> = functions.iter().map(|f| f.name.as_str()).collect();
    assert!(
        !names.iter().any(|n| n.starts_with("__mk_l0")),
        "mk 只有一条值层（类型层不占运行期槽），不存在包装层，不得凭空克隆；实际函数表: {names:?}"
    );
}
