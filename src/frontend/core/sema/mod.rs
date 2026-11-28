//! 类型检查器模块
//!
//! 实现 YaoXiang 语言的类型检查器，支持：
//! - Hindley-Milner 类型推断
//! - 泛型函数和泛型类型
//! - 完整的错误收集
//! - RFC-004/010/011 支撑

use crate::ast::Module;
use crate::frontend::core::types::identity::DefPath;
use crate::frontend::core::types::identity::Nominal;
use std::collections::HashMap;

// ============ 子模块声明 ============

// 编译期谓词解析器
pub mod predicate_resolver;

// 编译期值参数判定（RFC-011 §4.1 两步判定的唯一实现）
pub mod const_param;

// 导入推断模块
pub mod inference;

// 语义信息数据库
pub mod semantic_db;

// 类型环境
pub mod environment;

// 导入面（WBS 4.9.10 D-5）：类型名 → 提供模块的唯一来源
pub mod import_face;

// RFC-011b: 运算符接口与实现登记表
pub mod operator_interfaces;

// 类型环境探针的 L3 实现（RFC-039 6.4：parser 的数据流去类型依赖）
pub mod probe;

// 类型检查器
pub mod checker;

// 签名解析
pub mod signature;
// 类型定义
pub mod types;
// 依赖记录 Σ 的记录模型（RFC-027 §14 / SPEC §8.3；声明期一遍的产出）
pub mod record_model;
// 证明管道基础设施
pub mod proof;
// 证明层（本目录无调度器：调用先后由 checker::check_module_impl 决定）
pub mod layers;

/// 判定点协议（6.9.3d）：unify → 回滚 → 判等的**唯一**入口
pub mod judgement;
// 独立分析遍
pub mod passes;

// ============ 测试模块 ============
#[cfg(test)]
mod test_util;
#[cfg(test)]
mod tests;

// ============ 类型导出 ============

// 使用 core 层的类型系统（显式导出以避免 ambiguous glob re-exports）
pub use crate::frontend::core::types::{
    ConstExpr, ConstKind, ConstValue, ConstVarDef, EnumType, MonoType, PolyType, StructType,
    TypeBinding, TypeConstraint, TypeConstraintError, TypeConstraintSolver, TypeVar, UniverseLevel,
};

// 重新导出子模块
pub use crate::frontend::core::types::eval::evaluator::*;
pub use checker::*;
pub use environment::*;
pub use inference::*;
pub use probe::{is_builtin_predicate_name, is_constraint_name, SemanticProbe};
pub use signature::*;
pub use types::*;

// 导入诊断系统
pub use crate::util::diagnostic::{Diagnostic, ErrorCodeDefinition, ErrorCollector, I18nRegistry};

/// 类型推断结果
pub type TypeResult<T> = Result<T, Diagnostic>;

/// 类型错误收集器
pub type TypeErrorCollector = ErrorCollector;

// ============ 入口函数 ============

/// 检查模块
#[allow(unused_variables)]
pub fn check_module(
    ast: &Module,
    env: &mut Option<environment::TypeEnvironment>,
) -> types::TypeCheckResult {
    check_module_inner(ast, env, false)
}

/// 检查模块（收集所有错误模式）
///
/// 与 `check_module` 相同，但启用错误收集模式。
/// 类型检查器会尽可能多地收集错误，适用于 LSP 诊断。
#[allow(unused_variables)]
pub fn check_module_collect_all(
    ast: &Module,
    env: &mut Option<environment::TypeEnvironment>,
) -> types::TypeCheckResult {
    check_module_inner(ast, env, true)
}

/// 模块检查内部实现
#[allow(unused_variables)]
fn check_module_inner(
    ast: &Module,
    env: &mut Option<environment::TypeEnvironment>,
    collect_all: bool,
) -> types::TypeCheckResult {
    // 使用 TypeChecker 进行完整的模块检查
    let mut checker = checker::TypeChecker::new("main");

    // 如果提供了外部环境，将其变量和类型导入到 checker 中
    if let Some(ref mut ext_env) = env {
        for (name, poly) in &ext_env.vars {
            checker.env().add_var(name.clone(), poly.clone());
        }
        for (name, poly) in &ext_env.types {
            checker.env().add_type(name.clone(), poly.clone());
        }
        // RFC-029: 继承外部环境的模块注册表（多文件编排预构建的用户模块）
        checker.env().module_registry = ext_env.module_registry.clone();
    }

    // 谓词名可见面（WBS 3.20.9 / #522）：本自由函数是单文件入口，
    // scan 被检模块自身
    let namespace = probe::TypeLevelNamespace::scan_one(ast);

    // 执行模块检查
    let result = if collect_all {
        checker.check_module_collect_all(ast, &namespace)
    } else {
        checker.check_module(ast, &namespace)
    };

    // 将 method_bindings 导回传入的环境
    if let Some(ref mut ext_env) = env {
        ext_env.method_bindings = checker.env().method_bindings.clone();
    }

    result
}

/// 添加内置类型到环境
///
/// **不含标量内置名**（WBS 6.1.2）：`Int` / `Bool` / `String` … 由
/// `MonoType::from_builtin_name` 单源解析（`TypeEnvironment::resolves_type_name`
/// 第一条就是查它）。此处曾登记 `int` / `float` / `bool` / `string` / `void` /
/// `never` / `char` 七个小写拼写，等于在类型表里留了第二份别名表——名字虽能
/// 解析，却与解析表给的 `MonoType` 不一致（`int` 登记的是 `Int(32)`，
/// `from_builtin_name("Int")` 给 `Int(64)`）。删除后小写拼写在类型位置一律
/// 落成未知类型，与 SPEC §2.1 的 15 名清单一致。
pub fn add_builtin_types(env: &mut environment::TypeEnvironment) {
    // RFC-024: 内置资源类型
    for resource_type in &["FilePath", "HttpUrl", "DBUrl", "Console"] {
        env.types.insert(
            resource_type.to_string(),
            PolyType::mono(MonoType::TypeRef(
                Nominal::flat(AsRef::<str>::as_ref(&(resource_type)))
                    .expect("type name must be bare and non-empty"),
            )),
        );
    }
}

/// 注册标准库 native 函数类型签名到类型环境
///
/// 这些签名用于类型检查 `native("...")` 表达式，确保调用签名匹配。
/// 通过 ModuleRegistry 自动发现所有 std 模块的 native 函数。
///
/// #391：std 签名是编译器静态资产，畸形签名在此处硬拒绝（`Err` 传播），
/// 不再打印后降级——退化签名会以短名注册进 native_signatures 劫持消费者
/// 调用点，或让签名错误在用户代码处以无关错误码暴露。
pub fn add_native_function_types(
    env: &mut environment::TypeEnvironment
) -> Result<(), crate::util::diagnostic::Diagnostic> {
    use crate::frontend::module::registry::ModuleRegistry;
    use crate::frontend::module::ExportKind;

    let registry = ModuleRegistry::with_std();
    // std **已声明**类型名 → 模块键（D64）：native 签名串里的 `Result(T, E)` /
    // `Json` / `Spanned` 必须与注解侧解析到同一身份，否则「按规范写的调用」
    // 会在 unify 处被判异（std 声明类型在单文件也带模块身份）。幻名
    // （File / Error）与类型族不在此表，保持平路径。
    let std_declared = registry.std_type_origins();
    let resolve_std_identity = |written: &str| -> Option<Nominal> {
        std_declared
            .iter()
            .find(|(name, _)| name == written)
            .and_then(|(name, module)| Nominal::item(module, name))
    };

    // 裸名歧义短名的唯一生产端（issue #577，用户裁决乙案）：先枚举
    // 「≥2 个 Std 单元同时导出同一 Function/Constant 名」的交集合。这些短名
    // 没有权威赢家——lower 侧 short_to_qualified_map 按注册表 HashMap 迭代序
    // 随机覆盖、run 期随机派发（静默错值 / E6007），check 面按静态序赢家签名
    // 检查通过造成「check 绿、run 二义」。判定与注册共用同一信息源，且都在
    // 本函数内先枚举后注册：歧义短名跳过全部三条短名键（签名 / arity / 形参
    // 名——表间必须对齐，否则「arity 有表项而签名无」又是一份平行事实），
    // 全限定名键照常；裸名引用在 Var 臂查不到时按
    // `env.ambiguous_std_short_names` 发 E1116。
    //
    // 分组键 = `export.name`，单元归属 = 模块路径（模块身份），与 lower 侧
    // `short_to_qualified_map` 的实际键同源（`registry.std_code_exports()` 是
    // 两侧共用的唯一迭代源）。**不得**从 `full_path` 末段反推短名或单元——
    // 点式 FFI 导出（如 `int.to_string`，src/std/convert.rs）的 full_path 就是
    // native FFI 名本身，其前缀（`int`）不是模块；按末段分组会把
    // `int.to_string` / `float.to_string` 等整名都拆成裸名 `to_string`，把只由
    // std.convert 一个单元导出的 to_string 误判为 11 单元歧名——裸名
    // to_string 本确定解析到 std.convert.to_string（修后首版曾因按末段分组
    // 误拒它，与 lower 侧 map 的实际键集不符；按 `export.name` 重枚举交集合
    // 可复现对账：16 名 → 15 名，to_string 退出）。乙案边界 = 「裸名命中
    // ≥2 个 Std 单元的同名导出」，`export.name` 为整名的点式 FFI 导出不构成
    // 裸名命中。
    let mut ambiguous: HashMap<String, Vec<String>> = HashMap::new();
    for (unit, short, _) in registry.std_code_exports() {
        let units = ambiguous.entry(short.to_string()).or_default();
        if !units.iter().any(|u| u == unit) {
            units.push(unit.to_string());
        }
    }
    ambiguous.retain(|_, units| units.len() >= 2);
    // E1116 文案把单元列表插值进错误消息——registry.modules 的 HashMap 迭代序
    // （std_code_exports 的枚举序）是随机的，不排序则同一调用点的错误文案逐字
    // 不稳定（#577 的病根就是序）。
    for units in ambiguous.values_mut() {
        units.sort();
    }

    // 遍历所有 std 子模块，自动注册导出的函数
    for submodule_name in registry.std_submodule_names() {
        let module_path = DefPath::item("std", &submodule_name).project();
        if let Some(module) = registry.get(&module_path) {
            for export in module.exports.values() {
                // 使用签名字符串解析出正确的函数类型
                let fn_ty = match export.kind {
                    ExportKind::Function | ExportKind::Constant => {
                        if export.signature.is_empty() {
                            // 嵌入 yx std 模块（RFC-036 §4，如 std.test）没有签名字符串契约，
                            // 类型由 export.mono_type 承载；空串 parse 出的坏签名会以短名
                            // 注册进 native_signatures，劫持消费者调用点（std.test 首次行使暴露）
                            continue;
                        }
                        let parsed = signature::parse_signature(&export.signature, env)?;
                        crate::frontend::core::types::substitute::resolve_nominal_identities(
                            &parsed,
                            &resolve_std_identity,
                        )
                    }
                    _ => continue,
                };

                let bare_is_ambiguous = ambiguous.contains_key(&export.name);

                // 注册完全限定名
                env.native_signatures
                    .insert(export.full_path.clone(), fn_ty.clone());

                // #454 / t14：同源灌装「形参名 + 必需下限」（同一签名串、同一解析出口）——
                // 命名实参的按名绑定与「可选位允许缺席」都据此判定，两侧不再各算一份
                if let Some((names, required_min)) =
                    signature::export_signature_param_info(&export.signature)
                {
                    let info = crate::frontend::core::sema::environment::ExportParamInfo {
                        names,
                        required_min,
                    };
                    env.native_param_names
                        .insert(export.full_path.clone(), info.clone());
                    if !bare_is_ambiguous {
                        env.native_param_names.insert(export.name.clone(), info);
                    }
                }

                // 注册短名称。歧义短名不注册（#577：没有权威赢家就不该有静默赢家）。
                if !bare_is_ambiguous {
                    env.native_signatures.insert(export.name.clone(), fn_ty);
                }

                // #387：同源灌装 arity 区间（? 可选 / ... 变参），键与签名表对齐
                let arity = signature::parse_signature_arity(&export.signature);
                env.native_arity.insert(export.full_path.clone(), arity);
                if !bare_is_ambiguous {
                    env.native_arity.insert(export.name.clone(), arity);
                }
            }
        }
    }

    env.ambiguous_std_short_names = ambiguous;

    // Register Native.c — the C ABI FFI entry point
    // Native.c: (lib: String) -> LibraryRef
    env.native_signatures.insert(
        "Native.c".to_string(),
        MonoType::Fn {
            params: vec![MonoType::make_string()],
            return_type: Box::new(MonoType::LibraryRef {
                mechanism: "c".to_string(),
                lib: String::new(), // placeholder, filled at IR gen compile-time
            }),
        },
    );

    // Register Native.rs — the Rust ABI std function dispatch
    // Native.rs: (sym: String) -> ExternRef
    env.native_signatures.insert(
        "Native.rs".to_string(),
        MonoType::Fn {
            params: vec![MonoType::make_string()],
            return_type: Box::new(MonoType::ExternRef {
                mechanism: "rs".to_string(),
                lib: String::new(),    // placeholder
                symbol: String::new(), // placeholder
            }),
        },
    );

    // 显式注册 Native 模块变量，使 Native.c / Native.rs 在 FieldAccess 中可解析
    // 注意：add_var 循环（line 280-282）只注册"Native.c"和"Native.rs"这样的点分全名，
    // 不注册"Native"本身。而 FieldAccess（`inference/expressions/expr.rs` 的 `infer_expr` 分支）通过 extract_namespace_path
    // 从 Var("Native") 开始构建路径，因此"Native"必须在作用域中且类型为 Struct（有字段）。
    env.add_var(
        "Native".to_string(),
        PolyType::mono(MonoType::Struct(crate::frontend::core::types::StructType {
            name: Nominal::flat(AsRef::<str>::as_ref(&("Native")))
                .expect("type name must be bare and non-empty"),
            fields: vec![
                crate::frontend::core::types::mono::StructFieldInfo {
                    name: "c".to_string(),
                    ty: MonoType::Fn {
                        params: vec![MonoType::make_string()],
                        return_type: Box::new(MonoType::LibraryRef {
                            mechanism: "c".to_string(),
                            lib: String::new(),
                        }),
                    },
                    quantity: None,
                    is_mut: false,
                    has_default: false,
                },
                crate::frontend::core::types::mono::StructFieldInfo {
                    name: "rs".to_string(),
                    ty: MonoType::Fn {
                        params: vec![MonoType::make_string()],
                        return_type: Box::new(MonoType::ExternRef {
                            mechanism: "rs".to_string(),
                            lib: String::new(),
                            symbol: String::new(),
                        }),
                    },
                    quantity: None,
                    is_mut: false,
                    has_default: false,
                },
            ],
            methods: std::collections::HashMap::new(),
            interfaces: Vec::new(),
        })),
    );

    // 同时将这些 native 函数注册为变量，使其可在类型推断时查找
    for (name, sig) in &env.native_signatures.clone() {
        env.add_var(name.clone(), PolyType::mono(sig.clone()));
    }

    Ok(())
}

/// 添加标准库 traits 到环境
pub fn add_std_traits(env: &mut environment::TypeEnvironment) {
    env.trait_table = crate::frontend::core::types::TraitTable::with_std();
}
