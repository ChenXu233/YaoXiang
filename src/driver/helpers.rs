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
use crate::frontend::core::typecheck::TypeCheckResult;
use crate::frontend::core::types::mono::MonoType;
use crate::frontend::module::orchestrator;
use crate::frontend::module::orchestrator::OrchestratorError;
use crate::frontend::module::registry::ModuleRegistry;
use crate::frontend::module::roles;
use crate::frontend::module::symbol::SymbolTable;
use crate::middle::passes::mono::instance::{GenericFunctionId, InstantiationRequest};
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
///
/// 同名只登记一次（4.10.3）：Script 顶层对 `mut` 的**赋值语句**与声明同形
///（`counter = counter + 1` 也是 `Assign { target: Var, .. }`），逐语句计数
/// 会给同一变量重复配槽——与 ir_gen `register_global_slots` 的按名合并同一
/// 判据（裸名）。
pub(in crate::driver) fn extract_global_defs(ast: &Module) -> Vec<(String, MonoType)> {
    let mut out: Vec<(String, MonoType)> = Vec::new();
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
                if !is_fn && !out.iter().any(|(n, _)| n == name) {
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

/// 多文件形态（3.4.8 ②）：把各单元的实例化请求聚合为**全程序一张请求表**，
/// 并把 `containing_fn` 的裸名限定化为 merged IR 的函数表键。
///
/// 为什么需要限定化：`containing_fn` 是 mono 的 deferred 桶键（泛型体内的嵌套
/// 调用请求按所在函数暂存，待该函数特化时求值），也是调用点改写三元键的容器
/// 维度。typecheck 逐单元运行，`scope.fn_context()` 只知道**裸名**（`f`）；
/// 而 ir_gen 的 `qualify_names` 已把每个单元的顶层函数重命名为
/// `{module_key}.{bare}`，Linking 之后 merged IR 的函数表只有限定名。
/// 不限定化则 deferred 桶**永不排水**：桶建在裸名下、特化按限定名查桶——
/// 递归泛型（`f(x) = f([x])`）的整条特化链不展开，既不出深度诊断，
/// 原件又被 `restore_generics_with_uncovered_call_sites` 放回、以类型擦除
/// 执行到进程栈溢出（0xC00000FD）。
///
/// 判据「该单元是否定义了这个名字」：`containing_fn` 只可能是**本单元**的
/// 函数（AST 属于本单元），故按单元自己的函数名集合匹配。合并后重名不冲突
///（函数名已限定化）；嵌套函数名（`outer.inner`）不在单元顶层函数表里，
/// 保持原样——它们的 deferred 语义与单文件一致。
///
/// 命名前提（3.4.8 ③ 回归修复后收齐）：**所有多单元形态的单元 IR 定义键都已
/// 按单元键限定**——project 形态由 `ir_generation_orchestrated` 传单元键，
/// rootless（无项目根）check 形态由 `ir_generation_check` 传单元键 +
/// `with_script_mode(true)`（限定名与 Script 语义解耦）。因此本函数不再需要
/// 「限定名 ⇄ 扁平名」的桥：请求名要么已与 IR 调用名同源（限定名），要么经
/// `qualify(unit.key, name)` 兜底命中定义键——两条路径都落在同一张限定表上。
pub(in crate::driver) fn aggregate_instantiation_requests(
    units: &[Unit],
    unit_irs: &[Option<ModuleIR>],
    type_results: &[Option<&TypeCheckResult>],
    unit_asts: &[Option<Module>],
    registry: &ModuleRegistry,
) -> Vec<InstantiationRequest> {
    // 全程序函数名表：链接后 merged IR 的函数集 = 各单元函数集的并集
    // （`link_module_irs` 对同名限定键报 Collision，故并集即最终键集）。
    // 别名改写的目标（`pick` → `lib.pick`）落在**别的单元**，单看本单元函数表
    // 会误判为「不存在」而拒绝——故校验用并集。
    let program_functions: HashSet<&str> = unit_irs
        .iter()
        .flatten()
        .flat_map(|ir| ir.functions.iter().map(|f| f.name.as_str()))
        .collect();
    let mut aggregated: Vec<InstantiationRequest> = Vec::new();
    for (i, ir) in unit_irs.iter().enumerate() {
        let (Some(ir), Some(type_result)) = (ir, type_results.get(i).copied().flatten()) else {
            continue; // 上游已失败 / 无产物（FailFast 下 run 循环已中止，防御）
        };
        // 单元 IR 的函数名**已按单元键限定**（3.4.8 ③ 回归修复后，多单元形态
        // 一律限定：project 由 `ir_generation_orchestrated`、rootless check 由
        // `ir_generation_check` 传单元键）——故判据是「限定名是否在该单元函数
        // 表里」而非「裸名」；`containing_fn` 由 typecheck 给的是裸名。
        let top_level: HashSet<&str> = ir.functions.iter().map(|f| f.name.as_str()).collect();
        // 对应 `qualify_names` 里的 `rename` 表：本单元函数**已限定**，故这里是
        // 恒等映射（限定名 → 自身）。恒等而非「裸名 → 限定名」很重要：请求名若是
        // 裸名（具名导入 `use lib.{pick}` 的 `pick`），不得被本单元同名的限定名
        // 抢先命中——那种情况正是模块别名（`pick` → `lib.pick`）要接管的分支。
        let rename: HashMap<String, String> = top_level
            .iter()
            .map(|name| ((*name).to_string(), (*name).to_string()))
            .collect();
        // 模块别名表（本地名 → 限定名）：调 ir_gen 的**同一实现**
        // `import_aliases_from`（`qualify_names` 改写调用名用的就是它跑出来的表），
        // 传本单元 AST 与同一 registry——一处生产、一处消费，不在驱动侧另建表，
        // 也不复刻任何解析/优先级规则。
        let aliases: HashMap<String, String> = unit_asts
            .get(i)
            .and_then(|a| a.as_ref())
            .map(|ast| {
                crate::middle::core::ir_gen::AstToIrGenerator::import_aliases_from(ast, registry)
            })
            // 无 AST（上游失败）时无可解析：空表等价于纯单元键限定化
            .unwrap_or_default();
        let resolver = |name: &str| -> Option<String> {
            // 解析走 ir_gen 的**单点实现**（与生成期改写指令同一函数）
            let raw = crate::middle::core::ir_gen::AstToIrGenerator::resolve_call_target(
                name, &rename, &aliases,
            );
            let raw = raw.filter(|r| program_functions.contains(r.as_str()));
            // 别名未命中时按单元键限定化——这正是 `qualify_names` 对**本单元**
            raw.or_else(|| {
                let qualified = SymbolTable::qualify(&units[i].key, name);
                program_functions
                    .contains(qualified.as_str())
                    .then_some(qualified)
            })
        };
        for request in &type_result.instantiation_requests {
            let mut request = request.clone();
            // 被调方名字（`generic_id.name`）→ IR 调用指令里的名字。
            //
            // 规则与 `qualify_names` **同一个实现**（`resolve_call_target`）：
            // 完全匹配本单元函数名 / 完全匹配别名（具名导入 `use lib.{pick}`
            // 的 `pick` → `lib.pick`）/ 前缀匹配（`Point.get_x` → `lib.Point.get_x`）。
            // 请求名与 IR 调用指令名必须逐字一致（调用点改写三元键的名字维度）：
            // 只改一边会让改写落空、原件删除后调用点悬空（E6006），或反过来让
            // mono 认不出请求对象（E3018）。std 调用（`std.list.len`）与 native
            // 名不在本单元函数表里，保持原名（mono 侧显式跳过）。
            if let Some(resolved) = resolver(request.generic_id().name()) {
                request.generic_id =
                    GenericFunctionId::new(resolved, request.generic_id().type_params().to_vec());
            }
            // 所在函数名：裸名 → 本单元限定名，使 deferred 桶键与 merged IR 同源
            if let Some(bare) = request.containing_fn.as_deref() {
                let qualified = SymbolTable::qualify(&units[i].key, bare);
                if top_level.contains(qualified.as_str()) {
                    request.containing_fn = Some(qualified);
                }
            }
            aggregated.push(request);
        }
    }
    aggregated
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

/// 引用池的装配输入：模块键 + 源路径 + 已解析 AST（借用视图）
pub(in crate::driver) struct PoolModule<'a> {
    /// 模块键（限定名前缀）
    pub key: &'a str,
    /// 源路径（嵌入 std 模块为虚拟路径）
    pub path: &'a Path,
    /// 已解析 AST
    pub ast: &'a Module,
}

/// 引用池（RFC-029f 改进 1+2）：`模块键 → 该模块**对外引用**的标识符集合`。
///
/// **来源 = 编译面，不是目录**（缺陷 1 / #334「包内 use 图」）：池的根由调用方
/// 按 RFC-029f 判定三层给定——manifest 声明面的包目标（`[run].main` /
/// `[[bin]]` / `[lib]` / `[exports]`）优先（[collect_package_refs]），无声明面
/// 时退化为被检文件自身的编译集（[collect_compilation_refs]）；**两条路径都
/// 不为找入口递归扫目录**。从未被 `use` 的孤立文件、tests/ 下的测试文件因此
/// 不再构成生产代码的消费者。
///
/// **身份 = `(模块, 名字)`**（缺陷 2）：每个模块自有的名字（顶层定义与局部
/// 绑定，见 `DeadCodeAnalyzer::collect_refs_and_own_names`）从**它自己**的
/// 条目里剔除——同名标识符在不同模块各自计数，模块自己的 `foo`（定义或
/// 局部变量）不构成它对别的模块 `foo` 的消费。
///
/// 两个既有例外保留：vendor 依赖文件与嵌入 std 模块（虚拟路径）不入池
/// （外部包视角——它们不是本项目自己声明的消费面）。
///
/// 本函数是**唯一装配点**（两条来源路径共用，口径不得分歧）
pub(in crate::driver) fn collect_ref_pool(
    modules: &[PoolModule<'_>]
) -> HashMap<String, HashSet<String>> {
    let mut pool: HashMap<String, HashSet<String>> = HashMap::new();
    for module in modules {
        // 嵌入 std 模块：虚拟路径不是磁盘文件，也不属于本项目的消费面
        if module.path.to_str().is_some_and(|p| p.starts_with('<')) {
            continue;
        }
        // vendor 依赖：外部包视角（RFC-029f 原口径保留）
        if orchestrator::is_vendor_path(module.path) {
            continue;
        }
        let (refs, own) = DeadCodeAnalyzer::collect_refs_and_own_names(module.ast);
        pool.insert(
            module.key.to_string(),
            refs.difference(&own).cloned().collect(),
        );
    }
    pool
}

/// 退化为「被检文件自身的编译集」：Discovery 臂 `discover_with_used` 的产物
/// （`units`/`asts` 按下标对应）。只读传入的 AST——不做词法/语法/IO，也就
/// 没有「容错跳过」一说（解析失败的文件在 `asts` 里本就是 `None`）。
pub(in crate::driver) fn collect_compilation_refs(
    units: &[Unit],
    asts: &[Option<Module>],
) -> HashMap<String, HashSet<String>> {
    let modules: Vec<PoolModule<'_>> = units
        .iter()
        .zip(asts.iter())
        .filter_map(|(unit, ast)| {
            Some(PoolModule {
                key: unit.key.as_str(),
                path: unit.path.as_path(),
                ast: ast.as_ref()?,
            })
        })
        .collect();
    collect_ref_pool(&modules)
}

/// manifest 声明面优先的池来源（#334「包内 use 图不可达的报警」）：对每个包目标
/// root 跑一次 `discover_with_used`，把编译集里的文件现场词法+语法解析后装配池。
///
/// - root 取自 `roles::ExplicitSurfaces`（`[run].main`/`[[bin]]`/`[lib]`/
///   `[exports]` 指向的文件，路径已 canonicalize）——**不扫目录找入口**；
/// - 去重口径 = canonical 路径（与发现期同口径）；
/// - root 读不到（声明了却不存在）、词法/语法失败的文件静默跳过：池是辅助判定，
///   不允许它制造新的检查失败（解析失败的文件本就退出编译单元，4.10.1 同口径）；
/// - 全部 root 都跑不通时返回空池，调用方据此退化为被检文件自身的编译集。
pub(in crate::driver) fn collect_package_refs(
    roots: &[PathBuf]
) -> HashMap<String, HashSet<String>> {
    let mut seen: HashSet<PathBuf> = HashSet::new();
    let mut owned: Vec<(String, PathBuf, Module)> = Vec::new();
    for root in roots {
        let Ok((files, _, _)) = orchestrator::discover_with_used(root) else {
            continue;
        };
        for file in files {
            let canon = file
                .path
                .canonicalize()
                .unwrap_or_else(|_| file.path.clone());
            if !seen.insert(canon) {
                continue;
            }
            let Ok(tokens) = lexer::tokenize(&file.source) else {
                continue;
            };
            let parsed = parser::parse(&tokens);
            if parsed.has_errors {
                continue;
            }
            owned.push((file.module_key, file.path, parsed.module));
        }
    }
    let modules: Vec<PoolModule<'_>> = owned
        .iter()
        .map(|(key, path, ast)| PoolModule { key, path, ast })
        .collect();
    collect_ref_pool(&modules)
}
