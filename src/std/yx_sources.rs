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
pub const STD_YX_FILES: &[(&str, &str)] = &[("std/test.yx", include_str!("test.yx"))];

/// use 路径（`std.test`）查嵌入源；未命中（native 模块或用户模块）返回 None。
pub fn embedded_std_source(use_path: &str) -> Option<&'static str> {
    let file = format!("{}.yx", use_path.replace('.', "/"));
    STD_YX_FILES
        .iter()
        .find(|(path, _)| *path == file)
        .map(|(_, src)| *src)
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
    let mut info = ModuleInfo::new(use_path.to_string(), ModuleSource::Std);
    info.method_bindings = checker.env().method_bindings.clone();

    for stmt in &parsed.module.items {
        match &stmt.kind {
            parser::ast::StmtKind::TypeDefinition { name, .. } => {
                if let Some(ty) = types.get(name).map(|p| p.body.clone()) {
                    info.add_export(Export {
                        name: name.clone(),
                        full_path: SymbolTable::qualify(use_path, name),
                        kind: ExportKind::Type,
                        signature: String::new(),
                        mono_type: Some(ty),
                    });
                }
            }
            parser::ast::StmtKind::Assign { target, .. } => {
                if let parser::ast::Expr::Var(name, _) = target.as_ref() {
                    if let Some(ty) = vars.get(name).map(|p| p.body.clone()) {
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
                        });
                    }
                }
            }
            _ => {}
        }
    }
    Some(info)
}
