//! 嵌入二进制的纯 YaoXiang 标准库源文件（RFC-036 §4）
//!
//! `use std.test` 等命中本表的模块以虚拟路径作种子模块注入 orchestrator，
//! 与用户模块走同一前端管道（parse → typecheck → IR）。标准库版本与二进制
//! 严格绑定，单文件模式可用，无需用户配置标准库路径。
//!
//! ponytail: `include_str!` 即嵌入——RFC 提到的 build.rs 清单生成，等 .yx std
//! 多到需要自动化时再上。

/// (文件路径, 源码文本)。路径形如 `std/test.yx`（use 路径点转斜杠 + .yx）。
/// native 模块（std.assert 等）不在此表；同名不得同时存在两种实现。
pub const STD_YX_FILES: &[(&str, &str)] = &[
    ("std/list.yx", include_str!("list.yx")),
    ("std/test.yx", include_str!("test.yx")),
    ("std/result.yx", include_str!("result.yx")),
    ("std/option.yx", include_str!("option.yx")),
];

/// use 路径（`std.test`）查嵌入源；未命中（native 模块或用户模块）返回 None。
pub fn embedded_std_source(use_path: &str) -> Option<&'static str> {
    let file = format!("{}.yx", use_path.replace('.', "/"));
    STD_YX_FILES
        .iter()
        .find(|(path, _)| *path == file)
        .map(|(_, src)| *src)
}

/// 虚拟路径（`<std/test>`，orchestrator 发现阶段构造的形态）查嵌入源文本。
/// 运行时错误渲染回填 SourceMap 用（#327）：虚拟路径读盘必失败，源文本只能来自嵌入表。
pub fn embedded_source_by_virtual_path(virtual_path: &str) -> Option<&'static str> {
    let inner = virtual_path.strip_prefix('<')?.strip_suffix('>')?;
    embedded_std_source(inner)
}

/// 编译嵌入 std 模块的签名，构造 ModuleInfo（供 Registry 注册）。
///
/// `extract_module_info`（orchestrator）的嵌入版：解析源码 → 签名收集 → 按 AST 顶层
/// 定义导出。**只导出模块自身定义的绑定**——TypeChecker::new 会预载 native 签名到 env.vars，
/// 直接遍历 vars 会把全部 std native 误标成 `std.test.*` 导出。
pub fn embedded_std_module_info(use_path: &str) -> Option<crate::frontend::module::ModuleInfo> {
    use crate::frontend::core::parser;
    use crate::frontend::core::tokenize;
    use crate::frontend::core::typecheck::checker::TypeChecker;
    use crate::frontend::core::types::mono::MonoType;
    use crate::frontend::module::symbol::SymbolTable;
    use crate::frontend::module::{Export, ExportKind, ModuleInfo, ModuleSource};

    let source = embedded_std_source(use_path)?;
    let tokens = tokenize(source).ok()?;
    let parsed = parser::parse(&tokens);
    if parsed.has_errors {
        return None;
    }
    let mut checker = TypeChecker::new(use_path);
    checker.collect_signatures(&parsed.module);
    let vars = checker.env().vars.clone();
    let types = checker.env().types.clone();
    // 声明期类型参数名（按声明序）：跨模块调用的单态化需要它们绑定签名里的
    // `TypeRef("A")`。随导出一并携带，调用方不必从签名形态反推
    // （反推区分不了类型参数与普通类型名，会误伤 `Dict(K,V)` / `File` 等）。
    let fn_type_params = checker.generic_fn_type_params_snapshot();

    let mut info = ModuleInfo::new(use_path.to_string(), ModuleSource::Std);
    info.method_bindings = checker.env().method_bindings.clone();

    for stmt in &parsed.module.items {
        match &stmt.kind {
            parser::ast::StmtKind::TypeDefinition {
                name, definition, ..
            } => {
                if let Some(ty) = types.get(name).map(|p| p.body.clone()) {
                    // 跨模块类型传播：泛型模板 + 和类型变体 + 接口实现随导出携带
                    let type_payload = checker.type_def_export_payload(name, definition);
                    info.add_export(Export {
                        name: name.clone(),
                        full_path: SymbolTable::qualify(use_path, name),
                        kind: ExportKind::Type,
                        signature: String::new(),
                        mono_type: Some(ty),
                        type_params: None,
                        param_names: None,
                        type_payload,
                    });
                }
            }
            parser::ast::StmtKind::Assign { target, value, .. } => {
                if let parser::ast::Expr::Var(name, _) = target.as_ref() {
                    if let Some(ty) = vars.get(name).map(|p| p.body.clone()) {
                        // 声明期形参名：命名参数调用（`list.push(item = 9, list = v)`）
                        // 需按名字重排实参。名字只存在于 AST 的 Lambda 形参里
                        // （`MonoType::Fn` 只存类型），所以这里从 AST 取。
                        let param_names = value
                            .as_deref()
                            .map(|v| v.callable_parts().0)
                            .filter(|ps| !ps.is_empty())
                            .map(|ps| ps.iter().map(|p| p.name.clone()).collect());
                        info.add_export(Export {
                            name: name.clone(),
                            full_path: SymbolTable::qualify(use_path, name),
                            kind: if matches!(ty, MonoType::Fn { .. }) {
                                ExportKind::Function
                            } else {
                                ExportKind::Constant
                            },
                            signature: String::new(),
                            mono_type: Some(ty),
                            type_params: fn_type_params.get(name).cloned(),
                            param_names,
                            type_payload: None,
                        });
                    }
                }
            }
            _ => {}
        }
    }
    Some(info)
}
