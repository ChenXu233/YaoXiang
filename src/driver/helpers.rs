//! 阶段臂共用的自由函数（4.2.9 自 mod.rs 拆出）。
//!
//! 全部是纯函数：输入产物 → 输出产物，不接触 `State`。归属说明见各
//! 函数的「自 orchestrator 移入」注释（4.2.1/4.2.2）。

use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};

use crate::frontend::core::lexer;
use crate::frontend::core::parser::ast::{Expr, StmtKind};
use crate::frontend::core::parser::{self, Module};
use crate::frontend::core::typecheck::passes::dead_code::DeadCodeAnalyzer;
use crate::frontend::core::types::mono::MonoType;
use crate::frontend::module::orchestrator::OrchestratorError;
use crate::frontend::module::registry::ModuleRegistry;
use crate::frontend::module::roles;
use crate::frontend::module::symbol::SymbolTable;
use crate::middle::ModuleIR;
use crate::util::diagnostic::{Diagnostic, ErrorCodeDefinition};

// manifest 解析依赖 package 模块（wasm32 下不编译）——角色上下文在 wasm 降级
#[cfg(not(target_arch = "wasm32"))]
use crate::package::manifest::PackageManifest;

use super::unit::Unit;

/// 单文件模式：扫描入口源码中的 `use std.X`，把命中的嵌入 std 模块
/// （std.test）编译为独立 ModuleIR 并合并进入口 IR。
///
/// 移植自 pipeline.rs:349-389。词法扫描失败返回 Ok（真正的错误由
/// parse 阶段报告）。`registry` 与入口 IR 生成共用（共享 SymbolTable），
/// 保证 DefId 一致（#94）。
pub(in crate::driver) fn merge_embedded_std_ir(
    source: &str,
    ir: &mut ModuleIR,
    registry: &ModuleRegistry,
) -> Result<(), Diagnostic> {
    let mut use_paths = crate::frontend::module::orchestrator::scan_use_paths(source);
    // `for x in xs` 的脱糖会调用迭代协议 `std.list.iter/has_next/next`，
    // 该协议现由纯 yx 模块提供（`src/std/list.yx`，#117 硬切换）。
    // 源码未写 `use std.list` 时也必须纳入编译单元，否则 for 循环运行时
    // 报「Native function not found: std.list.iter」。
    if !use_paths.iter().any(|p| p == "std.list") {
        use_paths.push("std.list".to_string());
    }
    for use_path in use_paths {
        if use_path == "std"
            || !use_path.starts_with("std.")
            || crate::std::yx_sources::embedded_std_source(&use_path).is_none()
        {
            continue;
        }
        // 嵌入 std 模块：编译独立 IR 并合并（去重——入口可能多处 use 同一模块）
        let embedded = match crate::frontend::module::orchestrator::compile_embedded_module(
            &use_path, registry,
        ) {
            Ok(m) => m,
            Err(e) => {
                // #322 M3：E_INTERNAL 伪码收敛为注册码 E8001
                return Err(ErrorCodeDefinition::internal_error(&format!(
                    "嵌入 std 模块 {use_path} 编译失败: {e}"
                ))
                .build());
            }
        };
        for func in embedded.functions {
            if !ir.functions.iter().any(|f| f.name == func.name) {
                ir.functions.push(func);
            }
        }
    }
    Ok(())
}

/// 为所有文件分配不相交的全局槽位区间（T5）——自 orchestrator 移入
/// （4.2.1；入参由 owned 元组切片改为「单元 + AST 槽」引用视图，避免全量
/// AST clone——原签名里的 PathBuf 本就未被使用）。
///
/// 返回 `(限定名 → 绝对槽位号, 各文件的槽位基址)`。槽位号按发现顺序连续
/// 分配；限定名用 `{module_key}.{name}`（与 `SymbolTable::qualify` 同源），
/// 使 `use lib.{value}` 解析到 `lib.value`。只登记**真正的值绑定**
/// （非函数）——函数进函数表，不占全局槽位。
pub(in crate::driver) fn allocate_global_slots(
    units: &[Unit],
    asts: &[Option<Module>],
) -> (Vec<(String, usize)>, Vec<usize>) {
    let mut layout: Vec<(String, usize)> = Vec::new();
    let mut bases: Vec<usize> = Vec::with_capacity(units.len());
    let mut next_slot = 0usize;
    for (unit, ast) in units.iter().zip(asts.iter()) {
        bases.push(next_slot);
        let Some(ast) = ast else {
            continue; // 上游失败（FailFast 下 run 循环已中止，防御）
        };
        for (name, _ty) in extract_global_defs(ast) {
            layout.push((SymbolTable::qualify(&unit.key, &name), next_slot));
            next_slot += 1;
        }
    }
    (layout, bases)
}

/// 提取顶层**值绑定**（非函数）——自 orchestrator 移入（4.2.1）。
pub(in crate::driver) fn extract_global_defs(ast: &Module) -> Vec<(String, MonoType)> {
    let mut out = Vec::new();
    for stmt in &ast.items {
        if let StmtKind::Assign {
            target,
            type_annotation,
            value,
            ..
        } = &stmt.kind
        {
            if let Expr::Var(name, _) = target.as_ref() {
                // RFC-010a 附录D + B 方案（33f2ebbe）：Lambda → 函数；注解是
                // Fn → 函数；其余（含无注解块，内容决定类型）→ 块值。
                // 此前这里写「Block 就是函数」，把 `x: Int = { 5 }`
                // 误判为函数——与 ir_gen 的注册口径不一致，导致跨文件引用
                // `use lib.{x}` 找不到槽位（T5）。
                let is_fn =
                    Expr::block_binding_is_function(type_annotation.as_ref(), value.as_deref())
                        || matches!(value.as_deref(), Some(Expr::Lambda { .. }));
                if !is_fn {
                    let ty = type_annotation
                        .as_ref()
                        .map(|t| MonoType::from(t.clone()))
                        .unwrap_or(MonoType::Int(64));
                    out.push((name.clone(), ty));
                }
            }
        }
    }
    out
}

/// 链接多个文件的 `ModuleIR`：拼接函数/全局/FFI，合并 per-function 映射
/// ——自 orchestrator 移入（4.2.1）。
///
/// 各文件的函数已带模块限定名（`qualify_module_ir`），跨文件同名函数天然共存。
/// 仅当同一限定名出现两次（同名文件重复发现等病态情形）才报错。入口函数：
/// main 为函数绑定时设 `{entry_key}.main`；值 main 时为 None——程序体就是
/// 初始化序列（#388 定案：值 main 初始化期求值即执行）。
pub(in crate::driver) fn link_module_irs(
    irs: Vec<(String, ModuleIR)>,
    entry_key: &str,
) -> Result<ModuleIR, OrchestratorError> {
    let mut seen: HashMap<String, String> = HashMap::new();
    for (path, ir) in &irs {
        for func in &ir.functions {
            if let Some(prev) = seen.get(&func.name) {
                return Err(OrchestratorError::Collision {
                    name: func.name.clone(),
                    first: prev.clone(),
                    second: path.clone(),
                });
            }
            seen.insert(func.name.clone(), path.clone());
        }
    }

    let mut merged = ModuleIR {
        globals: Vec::new(),
        functions: Vec::new(),
        init: Vec::new(),
        init_locals: Vec::new(),
        init_file_ids: Vec::new(),
        ffi_libs: Vec::new(),
        ffi_bindings: Vec::new(),
        entry_function: None,
        source_files: irs.iter().map(|(p, _)| p.clone()).collect(),
        function_files: HashMap::new(),
    };
    for (i, (_, ir)) in irs.iter().enumerate() {
        for func in &ir.functions {
            merged.function_files.insert(func.name.clone(), i);
        }
    }
    for (file_idx, (_, ir)) in irs.into_iter().enumerate() {
        merged.globals.extend(ir.globals);
        merged.functions.extend(ir.functions);
        // T5：各文件的初始化序列按发现顺序拼接（被依赖模块先于入口文件）。
        // #368：每条指令记下所属文件——多文件下按它给 debug span 定 file_id，
        // 否则所有段的错误都指向同一个（错的）文件。
        merged
            .init_file_ids
            .extend(std::iter::repeat_n(file_idx, ir.init.len()));
        merged.init.extend(ir.init);
        // 不合并 init_locals：各文件槽位号从 0 起算，直接拼接会错位。
        // 多文件下顶层只有声明（可执行语句被 E3023 拒），具名局部仅出现在
        // Script 模式，故此处不需要它。
        merged.ffi_libs.extend(ir.ffi_libs);
        merged.ffi_bindings.extend(ir.ffi_bindings);
    }
    // 槽位号已由 allocate_global_slots 全局唯一，此处仅按索引稳定排序，
    // 便于调试与运行时按索引直取。
    merged.globals.sort_by_key(|g| g.index);

    // 入口函数仅在 main 为**函数**绑定时设置：值 main 的程序体就是初始化
    // 序列（求值即执行），无需入口调用——与 Script 同款执行形态（#356
    // 防双跑：初始化与入口调用绝不叠加）。
    let entry_fn = format!("{}.main", entry_key);
    if merged.functions.iter().any(|f| f.name == entry_fn) {
        merged.entry_function = Some(entry_fn);
    }

    Ok(merged)
}

/// T4：入口文件是否必须定义 `main`（Bin 要求）——自 orchestrator 移入（4.2.1）。
///
/// 判据：项目里有 manifest（`yaoxiang.toml`）。
///
/// 为何不用 `roles::classify`：那个模型回答「这个文件被谁消费」（服务于死代码
/// 分析），而这里要回答的是「这个文件能不能当程序跑」——两回事。有 manifest
/// 却没 `main` 的文件在 classify 里是 Internal（合理：它不被别的文件 use），
/// 但用户刚把它当入口跑了，此时必须有入口，否则静默什么都不做。
///
/// 对无 manifest 的单文件直跑（Script 角色）：顶层语句即程序主体，
/// 无需 main——这是「默认情况零门槛」。
pub(in crate::driver) fn is_bin_role(entry: &std::path::Path) -> bool {
    crate::frontend::module::orchestrator::find_project_root(entry).is_some()
}

/// 顶层是否存在 `main` 绑定（角色推断的 Bin 信号，RFC-029f）——自
/// orchestrator 移入（4.2.2；唯一消费端是 RoleClassification 臂）。
pub(in crate::driver) fn ast_has_main(ast: &Module) -> bool {
    ast.items.iter().any(|stmt| {
        matches!(
            &stmt.kind,
            StmtKind::Assign { target, .. }
                if matches!(target.as_ref(), Expr::Var(name, _) if name == "main")
        )
    })
}

/// 项目角色上下文（RFC-029f）：显式声明面 + 测试发现规则——自
/// orchestrator 移入（4.2.2；唯一消费端是 RoleClassification 臂）。
/// manifest 读取解析依赖 `crate::package`——wasm32 下不编译，降级为
/// `(None, None)`（全体 Script 态、无 patterns 规则，行为与引入模型前一致）。
#[cfg(not(target_arch = "wasm32"))]
pub(in crate::driver) fn role_context(
    project_root: Option<&Path>
) -> (Option<roles::ExplicitSurfaces>, Option<roles::TestRules>) {
    let Some(root) = project_root else {
        return (None, None);
    };
    let Ok(src) = std::fs::read_to_string(root.join(crate::package::manifest::MANIFEST_FILE))
    else {
        return (None, None);
    };
    let surfaces = toml::from_str::<PackageManifest>(&src)
        .ok()
        .map(|manifest| roles::TargetViews {
            bins: manifest
                .bin
                .iter()
                .map(|b| b.path.clone())
                .chain(manifest.run.as_ref().and_then(|r| r.main.clone()))
                .collect(),
            libs: manifest
                .exports
                .values()
                .cloned()
                .chain(manifest.lib.as_ref().map(|l| l.path.clone()))
                .collect(),
        })
        .map(|views| roles::explicit_surfaces(&views, root));
    let test_rules = toml::from_str::<crate::util::config::ProjectConfig>(&src)
        .ok()
        .map(|config| roles::TestRules::from_config(&config.tool.test));
    (surfaces, test_rules)
}

#[cfg(target_arch = "wasm32")]
pub(in crate::driver) fn role_context(
    _project_root: Option<&Path>
) -> (Option<roles::ExplicitSurfaces>, Option<roles::TestRules>) {
    (None, None)
}

/// 收集项目根下全部 `.yx` 文件的标识符引用并集（RFC-029f Phase 2 引用池）
/// ——自 orchestrator 移入（4.2.2；唯一消费端是 DeadCodeAnalysis 臂的
/// Check 形态）。
///
/// 跳过 `.git`/`.yaoxiang`（vendor）/`target` 目录；词法或语法失败的文件
/// 静默跳过——引用池是容错的辅助判定，不允许它制造新的检查失败。
pub(in crate::driver) fn collect_project_refs(project_root: &Path) -> HashSet<String> {
    fn is_excluded_dir(dir: &Path) -> bool {
        dir.file_name().is_some_and(|name| {
            let name = name.to_string_lossy();
            name == ".git" || name == ".yaoxiang" || name == "target"
        })
    }

    fn collect_yx(
        dir: &Path,
        out: &mut Vec<PathBuf>,
    ) {
        let Ok(entries) = std::fs::read_dir(dir) else {
            return;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                if !is_excluded_dir(&path) {
                    collect_yx(&path, out);
                }
            } else if path.extension().is_some_and(|e| e == "yx") {
                out.push(path);
            }
        }
    }

    let mut refs = HashSet::new();
    let mut files = Vec::new();
    collect_yx(project_root, &mut files);
    for path in files {
        let Ok(source) = std::fs::read_to_string(&path) else {
            continue;
        };
        let Ok(tokens) = lexer::tokenize(&source) else {
            continue;
        };
        let parsed = parser::parse(&tokens);
        refs.extend(DeadCodeAnalyzer::collect_ident_refs(&parsed.module));
    }
    refs
}
