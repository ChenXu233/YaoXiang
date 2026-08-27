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
//! 发现策略：沿 `use` 追踪（#247，RFC-036 隔离需求驱动）——从入口 BFS 可达模块，
//! 双根解析（导入者目录优先，项目根兑底）。包内循环由 visited 集天然支持。
//! ponytail: 函数名带模块限定名（module=record 语义，a.helper 与 b.helper 是不同
//! record 的字段），在解释器扁平 name→func 表里天然共存，跨文件同名函数不冲突。

use std::collections::{HashMap, HashSet, VecDeque};
use std::path::{Path, PathBuf};

use crate::frontend::core::parser::ast::{Expr, StmtKind};
use crate::util::diagnostic::Diagnostic;
use crate::frontend::core::parser::{self, Module};
use crate::frontend::core::tokenize;
use crate::frontend::core::typecheck::checker::TypeChecker;
use crate::frontend::core::typecheck::TypeCheckResult;
use crate::frontend::core::types::mono::MonoType;
use crate::frontend::core::types::PolyType;
use crate::frontend::module::registry::ModuleRegistry;
use crate::frontend::module::symbol::SymbolTable;
use crate::frontend::module::{Export, ExportKind, ModuleInfo, ModuleSource};
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
    TypeCheck {
        path: String,
        message: String,
        /// 结构化诊断（span 相对 path 文件），供 CLI 按源文件渲染
        diagnostics: Vec<Diagnostic>,
    },
    #[error("编译 {path} 失败: {message}")]
    Compile {
        path: String,
        message: String,
        diagnostics: Vec<Diagnostic>,
    },
    #[error("模块限定名冲突 `{name}`（{first} 与 {second}）；通常是模块路径歧义——如 foo/bar.yx 与 foo/bar/mod.yx 同被解析为 foo.bar，请删除其中一个")]
    Collision {
        name: String,
        first: String,
        second: String,
    },
    #[error("模块 `{key}` 解析歧义：{first} 与 {second}（导入者目录与项目根存在同名模块，请重命名其一）")]
    Ambiguous {
        key: String,
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
                diagnostics: result.diagnostics,
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
        let ir = crate::middle::generate_ir_with_context(
            ast,
            result,
            &all_ast_refs,
            &all_globals,
            &registry,
            key,
        )
        .map_err(|diags| {
            let message = diags
                .iter()
                .map(|d| d.to_string())
                .collect::<Vec<_>>()
                .join("\n");
            OrchestratorError::Compile {
                path: path.display().to_string(),
                message,
                diagnostics: diags,
            }
        })?;
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
        ffi_libs: Vec::new(),
        ffi_bindings: Vec::new(),
        entry_function: Some(format!("{}.main", entry_key)),
        source_files: irs.iter().map(|(p, _)| p.clone()).collect(),
        function_files: HashMap::new(),
    };
    for (i, (_, ir)) in irs.iter().enumerate() {
        for func in &ir.functions {
            merged.function_files.insert(func.name.clone(), i);
        }
    }
    for (_, ir) in irs {
        merged.globals.extend(ir.globals);
        merged.functions.extend(ir.functions);
        merged.ffi_libs.extend(ir.ffi_libs);
        merged.ffi_bindings.extend(ir.ffi_bindings);
    }
    Ok(merged)
}

/// 计算入口文件的模块键（文件 stem，`mod.yx` 折叠为目录名）。
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

/// 从入口沿 `use` 追踪发现可达模块（#247/RFC-036：替代目录递归——
/// 不相关文件的编译错误不再阻塞运行，测试文件的进程隔离才成立）。
///
/// 解析双根：`use a.b` 先按**导入者所在目录**解析，未命中再按**项目根**
/// （最近的 yaoxiang.toml 祖先）解析。模块键 = use 路径，与解析自哪个根无关。
fn discover(entry: &Path) -> Result<Vec<DiscoveredFile>, OrchestratorError> {
    let project_root = find_project_root(entry);
    let mut files: Vec<DiscoveredFile> = Vec::new();
    let mut visited: HashSet<PathBuf> = HashSet::new();
    let mut key_to_path: HashMap<String, PathBuf> = HashMap::new();
    // (路径, 模块键, 嵌入源)——嵌入 std 模块（std.test）的路径是虚拟的，源随身带
    let mut queue: VecDeque<(PathBuf, String, Option<&'static str>)> = VecDeque::new();
    queue.push_back((entry.to_path_buf(), entry_module_key(entry), None));

    while let Some((path, key, embedded)) = queue.pop_front() {
        let canon = path.canonicalize().unwrap_or_else(|_| path.clone());
        if !visited.insert(canon) {
            continue; // 包内循环导入：已处理过
        }
        if let Some(prev) = key_to_path.get(&key) {
            // 同一模块键映射到两个不同文件：双根产生歧义，显式报错而非静默遮蔽
            return Err(OrchestratorError::Ambiguous {
                key: key.clone(),
                first: prev.display().to_string(),
                second: path.display().to_string(),
            });
        }
        let source = match embedded {
            Some(src) => src.to_string(),
            None => std::fs::read_to_string(&path).map_err(|e| OrchestratorError::Io {
                path: path.display().to_string(),
                reason: e.to_string(),
            })?,
        };
        key_to_path.insert(key.clone(), path.clone());

        let importer_dir = path.parent().map(|p| p.to_path_buf());
        for use_path in scan_use_paths(&source) {
            // std 整体或 native std 模块不走文件解析
            if use_path == "std" {
                continue;
            }
            if use_path.starts_with("std.") {
                // RFC-036 §4：native 未命中时查嵌入源（std.test 等纯 YaoXiang std）
                if let Some(src) = crate::std::yx_sources::embedded_std_source(&use_path) {
                    let virtual_path = PathBuf::from(format!("<{}>", use_path.replace('.', "/")));
                    queue.push_back((virtual_path, use_path, Some(src)));
                }
                continue;
            }
            // 未命中留给 typecheck 报「模块未找到」，发现阶段不判死
            if let Some(resolved) =
                resolve_module_path(&use_path, importer_dir.as_deref(), project_root.as_deref())
            {
                queue.push_back((resolved, use_path, None));
            }
        }
        files.push(DiscoveredFile {
            module_key: key,
            path,
            source,
        });
    }

    // 稳定顺序，保证编译/合并可复现
    files.sort_by(|a, b| a.path.cmp(&b.path));
    Ok(files)
}

/// 从 entry 向上找最近的含 yaoxiang.toml 的目录（项目根）。
fn find_project_root(entry: &Path) -> Option<PathBuf> {
    let mut dir = entry.parent();
    while let Some(d) = dir {
        if d.join("yaoxiang.toml").exists() {
            return Some(d.to_path_buf());
        }
        dir = d.parent();
    }
    None
}

/// 解析模块路径为文件：`a.b` → `<base>/a/b.yx` 或 `<base>/a/b/mod.yx`。
/// 双根顺序：导入者目录优先，项目根兜底。
fn resolve_module_path(
    use_path: &str,
    importer_dir: Option<&Path>,
    project_root: Option<&Path>,
) -> Option<PathBuf> {
    let rel: PathBuf = use_path.split('.').collect();
    for base in [importer_dir, project_root].into_iter().flatten() {
        for cand in [
            base.join(&rel).with_extension("yx"),
            base.join(&rel).join("mod.yx"),
        ] {
            if cand.is_file() {
                return Some(cand);
            }
        }
    }
    None
}

/// 只读 use 行：词法级扫描源码中的模块路径（不解析函数体——RFC-029 发现协议）。
/// 词法失败的文件返回空，真正的错误由后续 parse 阶段报告。
pub(crate) fn scan_use_paths(source: &str) -> Vec<String> {
    use crate::frontend::core::lexer::TokenKind;
    let Ok(tokens) = tokenize(source) else {
        return Vec::new();
    };
    let mut paths = Vec::new();
    let mut i = 0;
    while i < tokens.len() {
        if !matches!(tokens[i].kind, TokenKind::KwUse) {
            i += 1;
            continue;
        }
        i += 1;
        let mut segments: Vec<String> = Vec::new();
        while let Some(TokenKind::Identifier(name)) = tokens.get(i).map(|t| &t.kind) {
            segments.push(name.clone());
            i += 1;
            match tokens.get(i).map(|t| &t.kind) {
                // `use lib.{x}`：模块路径到 { 为止
                Some(TokenKind::Dot)
                    if matches!(tokens.get(i + 1).map(|t| &t.kind), Some(TokenKind::LBrace)) =>
                {
                    break;
                }
                Some(TokenKind::Dot) => {
                    i += 1;
                }
                _ => break,
            }
        }
        if !segments.is_empty() {
            paths.push(segments.join("."));
        }
    }
    paths
}

/// 编译一个嵌入 std 模块（std.test 等纯 YaoXiang std，RFC-036 §4）为独立 ModuleIR。
///
/// 单文件模式（pipeline）在入口 IR 生成后调用，把嵌入模块的函数体合并进来——
/// 入口调用经 `short_to_qualified_map` 命名成 `std.test.assert_eq`，合并后即可解析。
/// `registry` 必须与入口 IR 生成共用（共享 SymbolTable），否则嵌入函数与入口调用点的
/// DefId 分属两张表，数值撞车会导致字节码按 DefId 分发到错误函数（#94）。
pub fn compile_embedded_module(
    key: &str,
    registry: &ModuleRegistry,
) -> Result<ModuleIR, OrchestratorError> {
    let source =
        crate::std::yx_sources::embedded_std_source(key).ok_or_else(|| OrchestratorError::Io {
            path: key.to_string(),
            reason: format!("不是嵌入 std 模块: {key}"),
        })?;
    let ast = parse_file(Path::new(&format!("<{}>", key.replace('.', "/"))), source)?;
    let mut checker = TypeChecker::new("<embedded>");
    checker.env().module_registry = registry.clone();
    let result = checker.check_module(&ast);
    if !result.diagnostics.is_empty() {
        let msg = result
            .diagnostics
            .iter()
            .map(|d| d.to_string())
            .collect::<Vec<_>>()
            .join("\n");
        return Err(OrchestratorError::TypeCheck {
            path: format!("<{key}> (embedded std)"),
            message: msg,
            diagnostics: result.diagnostics,
        });
    }
    crate::middle::generate_ir_with_context(&ast, &result, &[], &[], registry, key).map_err(
        |diags| OrchestratorError::Compile {
            path: format!("<{key}> (embedded std)"),
            message: diags
                .iter()
                .map(|d| d.to_string())
                .collect::<Vec<_>>()
                .join("\n"),
            diagnostics: diags,
        },
    )
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
