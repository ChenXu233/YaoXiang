//! 多文件编排器（RFC-029）
//!
//! 将多个 `.yx` 文件编排为一次编译：
//!
//! 1. **发现**：从入口文件所在目录递归收集所有 `.yx` 文件
//! 2. **签名提取**：对每个文件跑 pass1（类型）+ pass2（签名），提取顶层绑定的真实 `MonoType`
//! 3. **构建 Registry**：用户模块（`ModuleSource::User`）+ std，合并为预构建注册表
//! 4. **逐文件类型检查**：每个文件带着完整 Registry 独立 typecheck——强制模块边界
//!    （必须 `use` 才能访问其他文件的绑定，`use` 保持 record 解构语义）
//! 5. **逐文件 IR 生成 + IR 层链接**：每个文件独立生成 `ModuleIR`（预注册跨文件的
//!    结构体布局/方法绑定/全局名，使跨文件字段访问与全局引用能解析），再在 IR 层
//!    拼接函数/全局/FFI。每个文件始终是独立编译单元——**不合并 AST**。
//!
//! 为什么 typecheck 与 IR 生成都逐文件：
//! - 逐文件 typecheck 保证模块边界（`use` 有意义）。
//! - 逐文件 IR 生成保持每个文件是独立编译单元；跨文件函数调用按名字在链接后解析
//!   （解释器用扁平 name→func 表），跨文件字段/全局靠预注册的上下文解析。
//!
//! ponytail: 发现策略是"目录递归"而非"use 追踪"——简单且天然支持包内循环。
//! 升级为按需 use 追踪是后续优化（子 RFC 029a）。
//! ponytail: 函数名带模块限定名（module=record 语义，a.helper 与 b.helper 是不同
//! record 的字段），在解释器扁平 name→func 表里天然共存，跨文件同名函数不冲突。

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use crate::frontend::core::parser::ast::{Expr, StmtKind};
use crate::frontend::core::parser::{self, Module};
use crate::frontend::core::tokenize;
use crate::frontend::core::typecheck::checker::TypeChecker;
use crate::frontend::core::typecheck::TypeCheckResult;
use crate::frontend::core::types::mono::MonoType;
use crate::frontend::core::types::PolyType;
use crate::frontend::module::registry::ModuleRegistry;
use crate::frontend::module::symbol::SymbolTable;
use crate::frontend::module::{Export, ExportKind, ModuleInfo, ModuleSource};
use crate::middle::core::ir::{ConstValue, Instruction, Operand};
use crate::middle::ModuleIR;

/// 一个已发现的源文件：模块键 + 磁盘路径 + 源码
struct DiscoveredFile {
    /// 模块键（相对入口目录的点分路径，如 `lib`、`math.geometry`）
    module_key: String,
    /// 磁盘路径
    path: PathBuf,
    /// 源码内容
    source: String,
}

/// 编排错误
#[derive(Debug, thiserror::Error)]
pub enum OrchestratorError {
    #[error("无法读取入口文件 {path}: {reason}")]
    Io { path: String, reason: String },
    #[error("解析 {path} 失败: {message}")]
    Parse { path: String, message: String },
    #[error("类型检查 {path} 失败:\n{message}")]
    TypeCheck { path: String, message: String },
    #[error("编译 {path} 失败: {message}")]
    Compile { path: String, message: String },
    #[error("模块限定名冲突 `{name}`（{first} 与 {second}）；通常是模块路径歧义——如 foo/bar.yx 与 foo/bar/mod.yx 同被解析为 foo.bar，请删除其中一个")]
    Collision {
        name: String,
        first: String,
        second: String,
    },
}

/// 编排一个项目：发现源文件 → 构建 Registry → 逐文件 typecheck → 整体编译。
///
/// 返回合并后的 `ModuleIR`，可直接交给 codegen。入口文件的 `main` 函数即程序入口。
pub fn compile_project(entry: &Path) -> Result<ModuleIR, OrchestratorError> {
    let files = discover(entry)?;
    let registry = build_registry_from(&files)?;

    // 解析所有文件 → AST（只解析一次，Phase 1/2 复用）。携带模块键供限定名使用。
    let mut asts: Vec<(String, PathBuf, Module)> = Vec::new();
    for file in &files {
        let ast = parse_file(&file.path, &file.source)?;
        asts.push((file.module_key.clone(), file.path.clone(), ast));
    }

    // Phase 1: 逐文件 typecheck，强制模块边界（必须 use 才能跨文件访问）。
    // 保留每个文件的 TypeCheckResult，供 Phase 2 IR 生成复用。
    // RFC-029：注入所有模块的方法绑定，使跨文件方法调用（导入类型后调其方法）能解析。
    let method_bindings = registry.all_method_bindings();
    let mut type_results: Vec<TypeCheckResult> = Vec::new();
    for (_key, path, ast) in &asts {
        let mut checker = TypeChecker::new("<module>");
        checker.env().module_registry = registry.clone();
        checker.env().method_bindings = method_bindings.clone();
        let result = checker.check_module(ast);
        if !result.diagnostics.is_empty() {
            let msg = result
                .diagnostics
                .iter()
                .map(|d| d.to_string())
                .collect::<Vec<_>>()
                .join("\n");
            return Err(OrchestratorError::TypeCheck {
                path: path.display().to_string(),
                message: msg,
            });
        }
        type_results.push(result);
    }

    // 收集跨文件上下文：所有文件的类型（结构体布局/方法绑定）与全局变量名。
    let all_ast_refs: Vec<&Module> = asts.iter().map(|(_, _, ast)| ast).collect();
    let mut all_globals: Vec<(String, MonoType)> = Vec::new();
    for (_, _, ast) in &asts {
        all_globals.extend(extract_global_defs(ast));
    }

    // Phase 2: 逐文件 IR 生成（预注册跨文件上下文）→ 限定名重写 → IR 层链接。
    // 每个文件是独立编译单元——不合并 AST。函数名带模块限定名（module=record 语义），
    // 跨文件同名顶层函数因此天然共存，不再冲突。
    let mut module_irs: Vec<(String, ModuleIR)> = Vec::new();
    for ((key, path, ast), result) in asts.iter().zip(type_results.iter()) {
        let mut ir =
            crate::middle::generate_ir_with_context(ast, result, &all_ast_refs, &all_globals)
                .map_err(|diags| OrchestratorError::Compile {
                    path: path.display().to_string(),
                    message: diags
                        .iter()
                        .map(|d| d.to_string())
                        .collect::<Vec<_>>()
                        .join("\n"),
                })?;
        let aliases = build_import_aliases(ast, &registry);
        qualify_module_ir(&mut ir, key, &aliases);
        module_irs.push((path.display().to_string(), ir));
    }

    let entry_key = entry_module_key(entry);
    link_module_irs(module_irs, &entry_key)
}

/// 提取一个文件定义的全局变量（顶层非函数绑定）的名字与类型。
///
/// 镜像 `generate_stmt_ir` 的函数/全局分流：值为 Lambda/Block 视为函数，否则为全局。
/// 用于跨文件全局解析——其他文件引用这些名字时生成 `Call(访问器函数)`。
fn extract_global_defs(ast: &Module) -> Vec<(String, MonoType)> {
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
                let is_fn = matches!(
                    value.as_deref(),
                    Some(Expr::Lambda { .. }) | Some(Expr::Block(_))
                );
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

/// 链接多个文件的 `ModuleIR`：拼接函数/全局/FFI，合并 per-function 映射。
///
/// 各文件的函数已带模块限定名（`qualify_module_ir`），跨文件同名函数天然共存。
/// 仅当同一限定名出现两次（同名文件重复发现等病态情形）才报错。入口函数设为
/// `{entry_key}.main`，供 codegen 精确定位。
fn link_module_irs(
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
        mut_locals: HashMap::new(),
        loop_binding_locals: HashMap::new(),
        local_names: HashMap::new(),
        ffi_libs: Vec::new(),
        ffi_bindings: Vec::new(),
        entry_function: Some(format!("{}.main", entry_key)),
    };
    for (_, ir) in irs {
        merged.globals.extend(ir.globals);
        merged.functions.extend(ir.functions);
        merged.mut_locals.extend(ir.mut_locals);
        merged.loop_binding_locals.extend(ir.loop_binding_locals);
        merged.local_names.extend(ir.local_names);
        merged.ffi_libs.extend(ir.ffi_libs);
        merged.ffi_bindings.extend(ir.ffi_bindings);
    }
    Ok(merged)
}

/// RFC-029 限定名：把一个文件 IR 里的顶层函数名重写为 `{module_key}.{name}`，
/// 并把函数体里对这些函数与导入函数的调用/闭包引用一并重写。
///
/// module=record 语义：`a.helper` 与 `b.helper` 是两个 record 的不同字段，本就不冲突。
/// 解释器函数表是扁平 name→func，故用限定名作键使其共存。
///
/// #244：TypeDecl 与方法派生函数也限定——`a.Point` 与 `b.Point` 是不同 record 的
/// 不同字段，构造器/方法/vtable 随之共存。方法派生函数（`Point.get_x`）按本地类型
/// 名前缀匹配限定为 `lib.Point.get_x`，与 codegen `CreateStruct.type_name` 及解释器
/// `build_vtable` 的前缀查找天然契合。
fn qualify_module_ir(
    ir: &mut ModuleIR,
    module_key: &str,
    aliases: &HashMap<String, String>,
) {
    // 先收集本文件的类型名（TypeDecl），供方法派生函数前缀匹配。
    let mut type_names: std::collections::HashSet<String> = std::collections::HashSet::new();
    for func in &ir.functions {
        if func.is_type_decl() {
            type_names.insert(func.name.clone());
        }
    }

    // 本文件要限定的函数：普通函数、TypeDecl、方法派生函数（前缀是本地类型）。
    // 限定名统一经 SymbolTable::qualify 生成——全仓库限定名语法的唯一所有者。
    let mut rename: HashMap<String, String> = HashMap::new();
    for func in &ir.functions {
        let bare = &func.name;
        if func.is_type_decl() {
            // TypeDecl：Point → lib.Point
            rename.insert(bare.clone(), SymbolTable::qualify(module_key, bare));
        } else if let Some(dot_pos) = bare.find('.') {
            // 带点名字：方法派生（Point.get_x）或外部限定名（std.io.println）。
            // 只有限定点是本地类型时才限定，避免误伤 std 等已限定名。
            let prefix = &bare[..dot_pos];
            if type_names.contains(prefix) {
                rename.insert(bare.clone(), SymbolTable::qualify(module_key, bare));
            }
        } else {
            // 普通代码函数：distance → lib.distance
            rename.insert(bare.clone(), SymbolTable::qualify(module_key, bare));
        }
    }

    // 重写函数定义名，及按函数名索引的 per-function 映射。
    for func in &mut ir.functions {
        if let Some(q) = rename.get(&func.name) {
            func.name = q.clone();
        }
    }
    ir.mut_locals = remap_keys(std::mem::take(&mut ir.mut_locals), &rename);
    ir.loop_binding_locals = remap_keys(std::mem::take(&mut ir.loop_binding_locals), &rename);
    ir.local_names = remap_keys(std::mem::take(&mut ir.local_names), &rename);

    // 重写调用/闭包引用：本文件函数（rename）+ 导入函数（aliases）。
    for func in &mut ir.functions {
        if func.is_type_decl() {
            continue;
        }
        for block in func.blocks_mut() {
            for instr in &mut block.instructions {
                rewrite_call_names(instr, &rename, aliases);
            }
        }
    }
}

/// 按重命名表替换一张以函数名为键的映射的键。
fn remap_keys<V>(
    map: HashMap<String, V>,
    rename: &HashMap<String, String>,
) -> HashMap<String, V> {
    map.into_iter()
        .map(|(k, v)| (rename.get(&k).cloned().unwrap_or(k), v))
        .collect()
}

/// 重写单条指令里的函数名引用（Call/TailCall 的字符串 func、MakeClosure 的 func）。
///
/// 解析顺序：完全匹配 rename（本文件函数）→ 完全匹配 aliases（导入函数）→
/// 前缀匹配（方法调用 `Point.get_x` 经 `Point` 的 rename/alias 限定为 `lib.Point.get_x`，#244）。
fn rewrite_call_names(
    instr: &mut Instruction,
    rename: &HashMap<String, String>,
    aliases: &HashMap<String, String>,
) {
    let resolve = |name: &str| -> Option<String> {
        rename
            .get(name)
            .cloned()
            .or_else(|| aliases.get(name).cloned())
            .or_else(|| {
                // 方法调用前缀匹配：Point.get_x → lib.Point.get_x
                let dot_pos = name.find('.')?;
                let prefix = &name[..dot_pos];
                let suffix = &name[dot_pos..]; // 含点
                rename
                    .get(prefix)
                    .or_else(|| aliases.get(prefix))
                    .map(|q| format!("{}{}", q, suffix))
            })
    };
    match instr {
        Instruction::Call { func, .. } | Instruction::TailCall { func, .. } => {
            if let Operand::Const(ConstValue::String(name)) = func {
                if let Some(q) = resolve(name) {
                    *name = q;
                }
            }
        }
        Instruction::MakeClosure { func, .. } => {
            if let Some(q) = resolve(func) {
                *func = q;
            }
        }
        _ => {}
    }
}

/// 从一个文件的 `use` 语句构建导入别名表：本地短名 → 源模块限定名（registry full_path）。
///
/// 供限定名重写解析跨文件调用目标。std 调用已由 IR 生成限定为 `std.*`，不在此表。
fn build_import_aliases(
    ast: &Module,
    registry: &ModuleRegistry,
) -> HashMap<String, String> {
    let mut aliases = HashMap::new();
    for stmt in &ast.items {
        if let StmtKind::Use { path, items, .. } = &stmt.kind {
            let Some(exports) = registry.get_exports(path) else {
                continue;
            };
            if let Some(names) = items {
                for name in names {
                    if let Some(export) = exports.get(name) {
                        // 别名函数/常量/类型（调用目标）。#244 后类型也限定：构造调用
                        // `Point(...)` 的函数名被限定为 `lib.Point`，导入方须把本地短名
                        // `Point` 别名到 `lib.Point` 才能解析；方法调用 `Point.get_x` 经
                        // rewrite_call_names 的前缀匹配同样受益于此别名。
                        if matches!(
                            export.kind,
                            ExportKind::Function | ExportKind::Constant | ExportKind::Type
                        ) {
                            aliases.insert(name.clone(), export.full_path.clone());
                        }
                    }
                }
            }
        }
    }
    aliases
}

/// 计算入口文件的模块键（与 `module_key_for` 一致：文件 stem，`mod.yx` 折叠为目录名）。
fn entry_module_key(entry: &Path) -> String {
    let stem = entry
        .file_stem()
        .map(|s| s.to_string_lossy().to_string())
        .unwrap_or_else(|| "main".to_string());
    if stem == "mod" {
        entry
            .parent()
            .and_then(|p| p.file_name())
            .map(|s| s.to_string_lossy().to_string())
            .unwrap_or(stem)
    } else {
        stem
    }
}

/// 构建预构建 Registry：std + 所有用户模块（含真实签名）。
pub fn build_registry(entry: &Path) -> Result<ModuleRegistry, OrchestratorError> {
    let files = discover(entry)?;
    build_registry_from(&files)
}

fn build_registry_from(files: &[DiscoveredFile]) -> Result<ModuleRegistry, OrchestratorError> {
    let mut registry = ModuleRegistry::with_std();
    for file in files {
        let ast = parse_file(&file.path, &file.source)?;
        let info = extract_module_info(&file.module_key, &ast);
        registry.register(info);
    }
    Ok(registry)
}

/// 从入口文件所在目录递归发现所有 `.yx` 文件。
fn discover(entry: &Path) -> Result<Vec<DiscoveredFile>, OrchestratorError> {
    let base = entry
        .parent()
        .unwrap_or_else(|| Path::new("."))
        .to_path_buf();

    let mut paths: Vec<PathBuf> = Vec::new();
    collect_yx_files(&base, &mut paths)?;
    // 稳定顺序，保证编译/合并可复现
    paths.sort();

    let mut files = Vec::new();
    for path in paths {
        let module_key = module_key_for(&base, &path);
        let source = std::fs::read_to_string(&path).map_err(|e| OrchestratorError::Io {
            path: path.display().to_string(),
            reason: e.to_string(),
        })?;
        files.push(DiscoveredFile {
            module_key,
            path,
            source,
        });
    }
    Ok(files)
}

/// 递归收集目录下所有 `.yx` 文件。
fn collect_yx_files(
    dir: &Path,
    out: &mut Vec<PathBuf>,
) -> Result<(), OrchestratorError> {
    let entries = std::fs::read_dir(dir).map_err(|e| OrchestratorError::Io {
        path: dir.display().to_string(),
        reason: e.to_string(),
    })?;
    for entry in entries {
        let entry = entry.map_err(|e| OrchestratorError::Io {
            path: dir.display().to_string(),
            reason: e.to_string(),
        })?;
        let path = entry.path();
        if path.is_dir() {
            collect_yx_files(&path, out)?;
        } else if path.extension().map(|e| e == "yx").unwrap_or(false) {
            out.push(path);
        }
    }
    Ok(())
}

/// 计算模块键：相对 base 的点分路径，去掉 `.yx`。`mod.yx` 折叠为所在目录名。
///
/// - `lib.yx` → `lib`
/// - `math/geometry.yx` → `math.geometry`
/// - `math/mod.yx` → `math`
fn module_key_for(
    base: &Path,
    path: &Path,
) -> String {
    let rel = path.strip_prefix(base).unwrap_or(path);
    let mut parts: Vec<String> = rel
        .components()
        .map(|c| c.as_os_str().to_string_lossy().to_string())
        .collect();
    // 去掉文件名扩展
    if let Some(last) = parts.last_mut() {
        if let Some(stem) = last.strip_suffix(".yx") {
            *last = stem.to_string();
        }
    }
    // mod.yx 折叠：移除末尾的 "mod"
    if parts.last().map(|s| s.as_str()) == Some("mod") {
        parts.pop();
    }
    if parts.is_empty() {
        // 入口目录下的 mod.yx → 用目录名
        base.file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_else(|| "main".to_string())
    } else {
        parts.join(".")
    }
}

/// 解析单个文件为 AST。
fn parse_file(
    path: &Path,
    source: &str,
) -> Result<Module, OrchestratorError> {
    let tokens = tokenize(source).map_err(|e| OrchestratorError::Parse {
        path: path.display().to_string(),
        message: format!("{:?}", e),
    })?;
    let result = parser::parse(&tokens);
    if result.has_errors {
        let msg = result
            .errors
            .iter()
            .map(|e| e.to_string())
            .collect::<Vec<_>>()
            .join("\n");
        return Err(OrchestratorError::Parse {
            path: path.display().to_string(),
            message: msg,
        });
    }
    Ok(result.module)
}

/// 提取一个文件的模块信息：跑签名收集，导出所有顶层绑定的真实类型。
fn extract_module_info(
    module_key: &str,
    ast: &Module,
) -> ModuleInfo {
    let mut checker = TypeChecker::new(module_key);
    checker.collect_signatures(ast);

    // TypeEnvironment 不实现 Clone，只克隆需要的几张表。
    let vars = checker.env().vars.clone();
    let types = checker.env().types.clone();
    let mut info = ModuleInfo::new(module_key.to_string(), ModuleSource::User);
    // RFC-029：导出本文件的方法绑定（如 Point.get_x），供其他文件导入类型后调用。
    info.method_bindings = checker.env().method_bindings.clone();

    for stmt in &ast.items {
        match &stmt.kind {
            StmtKind::TypeDefinition { name, .. } => {
                if let Some(ty) = lookup(&types, name) {
                    info.add_export(make_export(module_key, name, ExportKind::Type, ty));
                }
            }
            StmtKind::Assign {
                target,
                type_annotation,
                ..
            } => {
                if let Expr::Var(name, _) = target.as_ref() {
                    if let Some(ty) = lookup(&vars, name) {
                        // 函数/lambda 绑定由 collect_signatures 收集 → 在 vars 中。
                        let kind = if matches!(ty, MonoType::Fn { .. }) {
                            ExportKind::Function
                        } else {
                            ExportKind::Constant
                        };
                        info.add_export(make_export(module_key, name, kind, ty));
                    } else if let Some(type_ann) = type_annotation {
                        // 普通常量（如 `value: Int = 7`）不会被当作签名收集，
                        // 从类型标注导出其类型。
                        let ty = MonoType::from(type_ann.clone());
                        info.add_export(make_export(module_key, name, ExportKind::Constant, ty));
                    }
                }
            }
            StmtKind::Expr(expr) => {
                if let Expr::FnDef { name, .. } = expr.as_ref() {
                    if let Some(ty) = lookup(&vars, name) {
                        info.add_export(make_export(module_key, name, ExportKind::Function, ty));
                    }
                }
            }
            _ => {}
        }
    }

    info
}

fn make_export(
    module_key: &str,
    name: &str,
    kind: ExportKind,
    ty: MonoType,
) -> Export {
    Export {
        name: name.to_string(),
        full_path: SymbolTable::qualify(module_key, name),
        kind,
        signature: String::new(),
        mono_type: Some(ty),
    }
}

fn lookup(
    map: &HashMap<String, PolyType>,
    name: &str,
) -> Option<MonoType> {
    map.get(name).map(|p| p.body.clone())
}
