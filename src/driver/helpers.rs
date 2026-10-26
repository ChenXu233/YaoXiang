//! 阶段臂共用的自由函数（4.2.9 自 mod.rs 拆出）。
//!
//! 全部是纯函数：输入产物 → 输出产物，不接触 `State`。归属说明见各
//! 函数的「自 orchestrator 移入」注释（4.2.1/4.2.2）。

use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};

use crate::frontend::core::lexer;
use crate::frontend::core::parser::ast::{classify_generic_params, Expr, GenericParamKind, StmtKind};
use crate::frontend::core::parser::{self, Module};
use crate::frontend::core::typecheck::operator_interfaces;
use crate::frontend::core::typecheck::passes::dead_code::DeadCodeAnalyzer;
use crate::frontend::core::typecheck::TypeCheckResult;
use crate::frontend::core::types::mono::MonoType;
use crate::frontend::module::orchestrator;
use crate::frontend::module::orchestrator::OrchestratorError;
use crate::frontend::module::registry::ModuleRegistry;
use crate::frontend::module::roles::{self, FileRole};
use crate::frontend::module::symbol::SymbolTable;
use crate::middle::passes::mono::instance::{GenericFunctionId, InstantiationRequest};
use crate::middle::ModuleIR;
use crate::util::diagnostic::{Diagnostic, ErrorCodeDefinition};
use crate::util::span::Span;

// manifest 解析依赖 package 模块（wasm32 下不编译）——角色上下文在 wasm 降级
#[cfg(not(target_arch = "wasm32"))]
use crate::package::manifest::PackageManifest;

use super::unit::Unit;

/// 单文件模式：扫描入口源码中的 `use std.X`，把命中的嵌入 std 模块
/// （std.test）编译为独立 ModuleIR 并合并进入口 IR。
///
/// 移植自 pipeline.rs:349-389。词法扫描失败返回 Ok（真正的错误由
/// parse 阶段报告）。`registry` 与入口 IR 生成共用（共享 SymbolTable），
/// 保证 DefId 一致（#94）。`config` 透传给 `compile_embedded_module`
/// （#471：嵌入 std 的单态化闸门与入口程序同源，不另设默认）。
pub(in crate::driver) fn merge_embedded_std_ir(
    source: &str,
    ir: &mut ModuleIR,
    registry: &ModuleRegistry,
    config: &crate::frontend::config::CompileConfig,
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
            &use_path,
            registry,
            config.clone(),
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

/// 多文件初始化顺序 = `use` 依赖图的**拓扑序**（#467）——返回单元下标的
/// **全排列**，调用方（Linking 臂）按它拼接各单元的 `ModuleIR.init`。
///
/// **依赖边从哪来**：发现期以 **use 路径**作模块键（`orchestrator::discover_with_used`
/// 的 `key_to_path`），故「单元源码里有 `use X` 且 X 命中某单元的模块键」即一条
/// 「本单元依赖 X 单元」的边。取边用的是发现期**同一个**词法扫描
/// `orchestrator::scan_use_refs`：编译集就是沿它的产物 BFS 拉进来的，初始化序
/// 必须按同一张图排。不用第二套判据（AST 顶层 `StmtKind::Use`）是因为两张图会
/// 不一致——`use std.{list, test}` 在该扫描里展开成子模块路径
/// `std.list`/`std.test`，而 AST 顶层只剩前缀 `std`，这类单元就会被漏掉而
/// 退回发现序，正是本缺陷的形状（「发现包含、排序不含」）。
///
/// **顺序判据**（只影响 `init` 段的拼接顺序，不改槽位编号——槽位仍由
/// `allocate_global_slots` 按发现序分配）：
/// - 被依赖单元的初始化先于导入它的单元（#467 的修复点）。
/// - 无依赖边的单元**保持发现顺序**（发现期已按路径字典序排序，见
///   `orchestrator::discover_with_used` 末尾的 `files.sort_by`）——这是
///   「无依赖信息时的确定性 tie-break」，不是静默乱序。
/// - **入口单元最后**：编译集是从入口出发沿 use 边 BFS 得到的（同一函数以入口为
///   队列种子），故除入口外每个单元都经 use 边从入口可达 ⇒ 拓扑序里它们都在入口
///   之前。唯一不经 use 边注入的单元是 `std.list`（#117 补齐，只有函数绑定、
///   `init` 为空），落在哪一侧都不影响语义。
/// - 模块导入环：Tarjan 求 SCC 后对缩点图拓扑排序；**SCC 内部按路径字典序**。
///   环内先后在语义上不可判定，此约定只为确定性——**不是语义保证**。该事实
///   不再静默（#491）：成员数 ≥2 的 SCC 一并经 `import_cycles` 出参，由
///   Linking 臂发 W1007（用户 2026-10-23 裁决 A）；本函数仍是**唯一检出点**。
/// - **无依赖信息**的三种情形一律不建边，单元按发现序落位（确定、可复现）：
///   ① `use` 目标不在单元集——未解析导入由 typecheck 报 E5001，std 原生模块没有
///   编译期初始化序列；② 单元没有 IR（parse/typecheck 失败）时它不进 `irs`，
///   没有可排的初始化段；③ 同一文件被两种拼写命中时模块键只记首次（发现期按
///   canonical 路径去重），第二种拼写不建边。
pub(in crate::driver) fn init_topological_order(units: &[Unit]) -> InitOrder {
    let n = units.len();
    if n <= 1 {
        // 单单元程序没有导入边，检出面为空（#491 的 W1007 恒不发射）
        return InitOrder {
            order: (0..n).collect(),
            import_cycles: Vec::new(),
        };
    }
    // 模块键 → 单元下标：发现期保证模块键与文件一一对应（同键不同文件在发现期
    // 即报 Ambiguous），这里取首个出现作防御性兜底。
    let mut key_to_idx: HashMap<&str, usize> = HashMap::with_capacity(n);
    for (i, unit) in units.iter().enumerate() {
        key_to_idx.entry(unit.key.as_str()).or_insert(i);
    }
    // 依赖边：`dependents[d]` = 直接依赖单元 d 的单元下标（同一单元重复 `use`
    // 同一路径只记一条边）。
    let mut dependents: Vec<Vec<usize>> = vec![Vec::new(); n];
    // #491：同一趟里记下每条边的源码位置（`use` 关键字到路径末段）——W1007
    // 要落在真实语句上。去重判据与 `dependents` 完全同一处（同一导入方重复
    // `use` 同一目标只留首条），不另建一套边集。
    let mut edges: Vec<ImportEdge> = Vec::new();
    for (i, unit) in units.iter().enumerate() {
        // 与发现期同一条扫描：整份源码的 `use` token
        for use_ref in orchestrator::scan_use_refs(&unit.source) {
            // 目标不在单元集：不建边（无依赖信息，见函数注释①）
            let Some(&dep) = key_to_idx.get(use_ref.path.as_str()) else {
                continue;
            };
            if dep != i && !dependents[dep].contains(&i) {
                dependents[dep].push(i);
                edges.push(ImportEdge {
                    importer: i,
                    imported: dep,
                    span: use_ref.span,
                });
            }
        }
    }
    let mut comps = tarjan_scc(&dependents);
    // 每个 SCC 的成员按**路径字典序**排序（路径相同再按原下标，保证环内顺序是
    // 确定性约定，而非发现顺序的偶然产物）。
    for comp in &mut comps {
        comp.sort_by(|&a, &b| units[a].path.cmp(&units[b].path).then_with(|| a.cmp(&b)));
    }
    // #491：成员数 ≥2 的 SCC 即导入环（`dep != i` 已排除自环——单元导入自己
    // 不是环，见上方无依赖信息来源①）。环的**检出**就在 `tarjan_scc`，这里只把
    // 它写成可读的路径与边，供 Linking 臂发射 W1007；不新增第二份环检测。
    let mut import_cycles: Vec<ImportCycle> = comps
        .iter()
        .filter(|comp| comp.len() >= 2)
        .map(|comp| {
            // 环内边：两端都落在同一分量里（跨分量的边属缩点图，与环无关）
            let cycle_edges: Vec<ImportEdge> = edges
                .iter()
                .filter(|e| comp.contains(&e.importer) && comp.contains(&e.imported))
                .copied()
                .collect();
            ImportCycle {
                path: cycle_path(comp, &cycle_edges),
                edges: cycle_edges,
            }
        })
        .collect();
    // 确定性：环按「环内首单元（分量内路径字典序最小者）」排序，与拓扑序的
    // tie-break 同源，不看 Tarjan 的产出顺序
    import_cycles.sort_by_key(|c| c.path.first().copied().unwrap_or(usize::MAX));
    let mut comp_of = vec![usize::MAX; n];
    for (c, members) in comps.iter().enumerate() {
        for &m in members {
            comp_of[m] = c;
        }
    }
    // SCC 依赖边（去重）：`comp_deps[c]` = c 直接依赖的 SCC
    let mut comp_deps: Vec<Vec<usize>> = vec![Vec::new(); comps.len()];
    for (dep, ds) in dependents.iter().enumerate() {
        for &i in ds {
            let (cd, ci) = (comp_of[dep], comp_of[i]);
            if cd != ci && !comp_deps[ci].contains(&cd) {
                comp_deps[ci].push(cd);
            }
        }
    }
    let mut comp_dependents: Vec<Vec<usize>> = vec![Vec::new(); comps.len()];
    let mut indegree: Vec<usize> = comp_deps.iter().map(|d| d.len()).collect();
    for (c, deps) in comp_deps.iter().enumerate() {
        for &d in deps {
            comp_dependents[d].push(c);
        }
    }
    // 稳定 tie-break：SCC 之间按「SCC 内最小原下标」择先——无依赖关系的单元
    // 因此保持发现顺序（路径字典序）。
    let priority: Vec<usize> = comps
        .iter()
        .map(|members| members.iter().copied().min().unwrap_or(usize::MAX))
        .collect();
    let mut ordered: Vec<usize> = Vec::with_capacity(n);
    let mut emitted = vec![false; comps.len()];
    for _ in 0..comps.len() {
        let next = (0..comps.len())
            .filter(|&c| !emitted[c] && indegree[c] == 0)
            .min_by_key(|&c| priority[c]);
        let Some(c) = next else {
            // 缩点图是 DAG，理论上必有 in-degree 0 的未处理节点。防御性兜底：
            // 按优先键追加剩余 SCC，绝不静默丢单元（顺序仍确定、可复现）。
            debug_assert!(false, "SCC 缩点图出现环（不可达）");
            let mut rest: Vec<usize> = (0..comps.len()).filter(|&c| !emitted[c]).collect();
            rest.sort_by_key(|&c| priority[c]);
            for c in rest {
                emitted[c] = true;
                ordered.extend(comps[c].iter().copied());
            }
            break;
        };
        emitted[c] = true;
        ordered.extend(comps[c].iter().copied());
        for &dependent in &comp_dependents[c] {
            indegree[dependent] -= 1;
        }
    }
    InitOrder {
        order: ordered,
        import_cycles,
    }
}

/// 初始化序产物（#467 拓扑序 + #491 导入环）。
///
/// 两个字段是**同一份** Tarjan SCC 结果的两个视图：`order` 是拼接各单元
/// `init` 段用的全排列，`import_cycles` 是其中成员数 ≥2 的强连通分量。
/// **环检测只有这一处**（4.10.4 起即如此），消费方不得另建判据。
pub(in crate::driver) struct InitOrder {
    /// 单元下标的全排列（调用方按它拼接各单元的 `ModuleIR.init`）。
    pub order: Vec<usize>,
    /// 导入环，按「环内首单元」排序（确定性）。
    pub import_cycles: Vec<ImportCycle>,
}

/// 一个模块导入环（#491）：成员之间互有导入路径的强连通分量。
///
/// 环内先后**不可判定**——本结构只陈述这个事实，不改初始化顺序（顺序仍是
/// 上方拓扑序的路径字典序 tie-break）。
pub(in crate::driver) struct ImportCycle {
    /// 环路径，**按 `use` 方向**（`a → b` 表示 a 导入 b），首尾相接：
    /// 末项导入首项。由 SCC 内 BFS 重建——SCC 只保证「存在环」，路径要写出来
    /// 才可读；拿成员下标排序冒充会印出并不存在的边。
    pub path: Vec<usize>,
    /// 环内的 `use` 边（含源码 span），W1007 的定位取自这里。
    pub edges: Vec<ImportEdge>,
}

/// 环内的一条 `use` 边：`importer` 导入 `imported`。
///
/// `span` 是 `use` 关键字到路径末段的源码位置（`UsePathRef.span`）——
/// 用户能指着的那一行，环诊断因此不必走 span 豁免。
#[derive(Debug, Clone, Copy)]
pub(in crate::driver) struct ImportEdge {
    pub importer: usize,
    pub imported: usize,
    pub span: Span,
}

/// 在 SCC 内重建一条**真实**的环路径（#491）：从分量首单元（路径字典序最小）
/// 出发，沿「导入方 → 被导入方」边 BFS，找到第一个「有边指回起点」的节点，
/// 路径 = 起点到它的 BFS 树路径，环由调用方补末项（回到起点）。
///
/// SCC 保证强连通，故指回起点的那条边必然存在；BFS 树路径保证路径简单
/// （无重复节点）。本函数**只做呈现**：环的存在性由 `tarjan_scc` 判定，
/// 它不参与检出判据（否则就是第二份环检测）。
fn cycle_path(
    members: &[usize],
    edges: &[ImportEdge],
) -> Vec<usize> {
    let Some(&start) = members.first() else {
        return Vec::new();
    };
    let mut parent: HashMap<usize, usize> = HashMap::new();
    let mut visited: HashSet<usize> = HashSet::new();
    let mut queue: std::collections::VecDeque<usize> = std::collections::VecDeque::new();
    visited.insert(start);
    queue.push_back(start);
    let mut closing: Option<usize> = None;
    while let Some(u) = queue.pop_front() {
        // 出边按 `edges` 的收集序（= 单元序 × 源码 `use` 序），确定性
        let nexts: Vec<usize> = edges
            .iter()
            .filter(|e| e.importer == u && members.contains(&e.imported))
            .map(|e| e.imported)
            .collect();
        for v in nexts {
            if v == start {
                closing = Some(u);
                break;
            }
            if visited.insert(v) {
                parent.insert(v, u);
                queue.push_back(v);
            }
        }
        if closing.is_some() {
            break;
        }
    }
    // 防御：SCC 强连通 ⇒ 必然找到回边；找不到时退化为单点路径，绝不 panic
    let Some(mut node) = closing else {
        return vec![start];
    };
    let mut path = vec![node];
    while let Some(&p) = parent.get(&node) {
        path.push(p);
        node = p;
    }
    path.reverse();
    path
}

/// Tarjan 强连通分量（#467 初始化顺序用）——返回每个 SCC 的成员下标。
///
/// `dependents[d]` 是「依赖 d 的单元」邻接表；SCC 是互达关系，与边方向无关
/// （对反向图求 SCC 得到同一组划分），故这里直接用它当 DFS 邻接表。
fn tarjan_scc(dependents: &[Vec<usize>]) -> Vec<Vec<usize>> {
    struct Tarjan<'a> {
        dependents: &'a [Vec<usize>],
        /// 节点的 DFS 序号（`usize::MAX` = 未访问）
        index: Vec<usize>,
        low: Vec<usize>,
        on_stack: Vec<bool>,
        stack: Vec<usize>,
        next_index: usize,
        comps: Vec<Vec<usize>>,
    }

    impl Tarjan<'_> {
        fn visit(
            &mut self,
            v: usize,
        ) {
            self.index[v] = self.next_index;
            self.low[v] = self.next_index;
            self.next_index += 1;
            self.stack.push(v);
            self.on_stack[v] = true;
            for &w in &self.dependents[v] {
                if self.index[w] == usize::MAX {
                    self.visit(w);
                    self.low[v] = self.low[v].min(self.low[w]);
                } else if self.on_stack[w] {
                    self.low[v] = self.low[v].min(self.index[w]);
                }
            }
            if self.low[v] == self.index[v] {
                let mut comp = Vec::new();
                loop {
                    let w = self
                        .stack
                        .pop()
                        .expect("Tarjan 栈非空：lowlink == index 的根必有成员");
                    self.on_stack[w] = false;
                    comp.push(w);
                    if w == v {
                        break;
                    }
                }
                self.comps.push(comp);
            }
        }
    }

    let n = dependents.len();
    let mut ctx = Tarjan {
        dependents,
        index: vec![usize::MAX; n],
        low: vec![0; n],
        on_stack: vec![false; n],
        stack: Vec::new(),
        next_index: 0,
        comps: Vec::new(),
    };
    while let Some(root) = ctx.index.iter().position(|&idx| idx == usize::MAX) {
        ctx.visit(root);
    }
    ctx.comps
}

/// 链接多个文件的 `ModuleIR`：拼接函数/全局/FFI，合并 per-function 映射
/// ——自 orchestrator 移入（4.2.1）。
///
/// 各文件的函数已带模块限定名（`qualify_module_ir`），跨文件同名函数天然共存。
/// 仅当同一限定名出现两次（同名文件重复发现等病态情形）才报错。入口函数：
/// main 为函数绑定时设 `{entry_key}.main`；值 main 时为 None——程序体就是
/// 初始化序列（#388 定案：值 main 初始化期求值即执行）。
///
/// **两个顺序必须分开**（#467）：
/// - `irs` 的顺序是**文件表序**（调用方按 `state.units` 给，即发现序）。
///   `source_files`、`init_file_ids` 的下标空间、`function_files` 的归属值、
///   全局槽位表都建在它上面——这些是「文件身份」，与执行先后无关；消费方
///   （debug span 的 file_id、错误定位重建 SourceMap）按同一张表索引，文件表序
///   一换只会让两侧一起漂。
/// - `init_order` 是 `init` 段的**执行序**（`use` 依赖拓扑序，见
///   [`init_topological_order`]），只决定初始化指令的拼接先后：被依赖单元先跑、
///   导入它的单元后跑。旧实现按文件表序（路径字典序）拼接，入口文件路径排在前面
///   时，它的顶层绑定初始化会读到被依赖单元尚未写入的槽位（void：静默错值，或
///   运行期 E6007）。
///
/// `init_order` 必须是 `0..irs.len()` 的置换（唯一调用方按构造保证）。
pub(in crate::driver) fn link_module_irs(
    irs: Vec<(String, ModuleIR)>,
    entry_key: &str,
    init_order: &[usize],
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
    // 文件级产物按**文件表序**合并；初始化段先摘出来，等下面按执行序拼
    //（#467：两者的顺序不同，见函数注释）。
    let mut pending_init = Vec::with_capacity(irs.len());
    for (_, mut ir) in irs {
        merged.globals.extend(ir.globals);
        merged.functions.extend(ir.functions);
        pending_init.push(std::mem::take(&mut ir.init));
        // 不合并 init_locals：各文件槽位号从 0 起算，直接拼接会错位（它是 init
        // 序列的局部槽位表，#368 调试视角；多文件下不做重编号合并，本函数不消费）。
        merged.ffi_libs.extend(ir.ffi_libs);
        merged.ffi_bindings.extend(ir.ffi_bindings);
    }
    // #467：初始化序列按**执行序**拼接——被依赖单元先于导入它的单元。
    // #368：每条指令记下所属文件表下标，多文件下按它给 debug span 定 file_id，
    // 否则所有段的错误都指向同一个（错的）文件；`init_file_ids` 与 `init` 等长。
    for &file_idx in init_order {
        let seg = pending_init
            .get_mut(file_idx)
            .expect("init_order 必须只引用已链接的文件表下标（调用方契约）");
        merged
            .init_file_ids
            .extend(std::iter::repeat_n(file_idx, seg.len()));
        merged.init.append(seg);
    }
    // 防御网（按构造不可达）：`init_order` 是全排列，上面已把每段各取一次。
    // 万一将来调用方漏了某个单元，绝不静默丢掉它的初始化——debug 构建下断言
    // 暴露契约破坏，release 下按发现序把残余段补到尾部（完整优先于顺序完美）。
    let missing: Vec<usize> = (0..pending_init.len())
        .filter(|&i| !pending_init[i].is_empty())
        .collect();
    debug_assert!(
        missing.is_empty(),
        "init_order 必须是 0..irs.len() 的置换：未覆盖文件 {missing:?} 的初始化段"
    );
    for file_idx in missing {
        let seg = &mut pending_init[file_idx];
        merged
            .init_file_ids
            .extend(std::iter::repeat_n(file_idx, seg.len()));
        merged.init.append(seg);
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

/// 引用池的装配输入：源路径 + 角色 + 已解析 AST（借用视图）
///
/// 不持模块键：池按**定义模块**记账（`consumed_from` 的键来自 import 路径，
/// 见 `resolve_import_module`），装配方自己的模块键只用于查表不参与装配。
pub(in crate::driver) struct PoolModule<'a> {
    /// 源路径（嵌入 std 模块为虚拟路径）
    pub path: &'a Path,
    /// 该模块的角色（改进 5：Test 不贡献消费面）
    pub role: FileRole,
    /// 已解析 AST
    pub ast: &'a Module,
}

/// 包内引用池（RFC-029f 改进 1/2/4/5）：按**定义模块**记账的消费面。
///
/// **来源 = 编译面，不是目录**（缺陷 1 / #334「包内 use 图」）：池的根由调用方
/// 按 RFC-029f 判定三层给定——manifest 声明面的包目标（`[run].main` /
/// `[[bin]]` / `[lib]` / `[exports]`）优先（[collect_package_refs]），无声明面
/// 时退化为被检文件自身的编译集（[collect_compilation_refs]）；**两条路径都
/// 不为找入口递归扫目录**。
///
/// **消费判据 = 「被 import 后真实使用」**（改进 4）：一条消费记录 = 「某模块
/// 用 `use` 把名字 n 从模块 M 带进作用域、且在**使用点**真的引用了 n」，
/// 只记到 M（`consumed_from`）——仅出现在 use 项里而没有使用点（未使用导入，
/// 另由 W1003 报告）、以及别的模块对**同名**标识符的引用（名字撞名）都不算
/// 消费。import 的目标模块由 `module_keys`（本次编译面的模块键集合，发现期
/// 同一份解析产物）判定，不另起一套路径解析。
///
/// **Test 不贡献**（改进 5）：`FileRole::Test` 模块的引用单独成池——Test 自身
/// 不参与死代码判定，故其引用不外溢去豁免生产代码的定义。
///
/// **字段/方法位残差**（改进 4 的残面，归 #450 另案）：`obj.render()` 只出现
/// 短名 `render`，AST 层归因不到接收者类型所属模块；**只有基名命中命名空间
/// 导入**（`use lib` + `lib.foo()`）时才按 import 归因，其余按名字维度保守
/// 计入 `field_names`（宁漏报方向）。
///
/// 两个既有例外保留：vendor 依赖文件与嵌入 std 模块（虚拟路径）不入池
/// （外部包视角——它们不是本项目自己声明的消费面）。
///
/// 本函数是**唯一装配点**（两条来源路径共用，口径不得分歧）
#[derive(Default)]
pub(in crate::driver) struct RefPool {
    /// 定义模块键 → 被别的模块 import 后真实使用的名字（改进 4 的消费面）
    pub consumed_from: HashMap<String, HashSet<String>>,
    /// 字段/方法位引用（归因不到模块的残差，按名字维度保守计入）
    pub field_names: HashSet<String>,
}

pub(in crate::driver) fn collect_ref_pool(
    modules: &[PoolModule<'_>],
    module_keys: &HashSet<String>,
) -> RefPool {
    let mut pool = RefPool::default();
    for module in modules {
        // 嵌入 std 模块：虚拟路径不是磁盘文件，也不属于本项目的消费面
        if module.path.to_str().is_some_and(|p| p.starts_with('<')) {
            continue;
        }
        // vendor 依赖：外部包视角（RFC-029f 原口径保留）
        if orchestrator::is_vendor_path(module.path) {
            continue;
        }
        // 改进 5：测试引用单独成池，不豁免生产代码（Test 自身不参与死代码判定）
        if module.role == FileRole::Test {
            continue;
        }
        let names = DeadCodeAnalyzer::collect_module_names(module.ast);
        // 改进 4（花括号项）：use 项带入的名字**且**在使用点被引用 ⇒ 记到目标模块
        for (local, target, use_path) in &names.imported {
            if !names.refs.contains(local) {
                continue; // 只有 use 项、没有使用点（未使用导入）不算消费
            }
            if let Some(key) = resolve_import_module(use_path, target, module_keys) {
                pool.consumed_from
                    .entry(key)
                    .or_default()
                    .insert(target.clone());
            }
        }
        // 改进 4（命名空间形态）：use lib + lib.foo() ⇒ foo 记到 lib
        let mut residual: HashSet<String> = names.field_refs_free.clone();
        for (base, field) in &names.namespaced_fields {
            let attributed = names
                .namespaces
                .iter()
                .find(|(ns, _)| ns == base)
                .and_then(|(_, use_path)| resolve_import_module(use_path, field, module_keys));
            match attributed {
                Some(key) => {
                    pool.consumed_from
                        .entry(key)
                        .or_default()
                        .insert(field.clone());
                }
                // 值上的方法调用 / 未解析的命名空间：名字维度保守计入
                None => {
                    residual.insert(field.clone());
                }
            }
        }
        // 模块自有名不是对别的模块的消费（改进 2）
        pool.field_names
            .extend(residual.difference(&names.own).cloned());
    }
    pool
}

/// import 路径 → 编译面里的模块键（`module_keys` = 发现期同一份解析产物，
/// 不另起一套路径解析）：名字是子模块时键为 `<use 路径>.<名字>`，否则为
/// `<use 路径>`；两者都不在编译面里（E5001 未解析）时不算消费。
fn resolve_import_module(
    use_path: &str,
    name: &str,
    module_keys: &HashSet<String>,
) -> Option<String> {
    let submodule = format!("{use_path}.{name}");
    if module_keys.contains(&submodule) {
        return Some(submodule);
    }
    if module_keys.contains(use_path) {
        return Some(use_path.to_string());
    }
    None
}

/// 退化为「被检文件自身的编译集」：Discovery 臂 `discover_with_used` 的产物
/// （`units`/`asts` 按下标对应；`roles` 由 RoleClassification 臂给出，
/// 与包目标路径同用 `roles::classify` 单点判定）。只读传入的 AST——不做
/// 词法/语法/IO，也就没有「容错跳过」一说（解析失败的文件在 `asts` 里本就是
/// `None`）。
pub(in crate::driver) fn collect_compilation_refs(
    units: &[Unit],
    asts: &[Option<Module>],
    roles: &[Option<FileRole>],
) -> RefPool {
    let module_keys: HashSet<String> = units.iter().map(|unit| unit.key.clone()).collect();
    let modules: Vec<PoolModule<'_>> = units
        .iter()
        .zip(asts.iter())
        .enumerate()
        .filter_map(|(i, (unit, ast))| {
            Some(PoolModule {
                path: unit.path.as_path(),
                // 角色缺省（防御路径）按 Internal 计入：贡献消费面是「宁漏报」方向
                role: roles
                    .get(i)
                    .copied()
                    .flatten()
                    .unwrap_or(FileRole::Internal),
                ast: ast.as_ref()?,
            })
        })
        .collect();
    collect_ref_pool(&modules, &module_keys)
}

/// 方法可达性标记（WBS 4.9.6 / #450 + WBS 4.9.7 / issue #464），按**定义单元**归因：
///
/// - `by_unit`：定义单元键 → 该单元内**被决议命中**的 `(基名, 声明序)`；
/// - `groups`：出现过决议条目的**名字组**（基名）——池的名字级豁免
///   （`RefPool::field_names` 全局平铺）对这些组不成立，由驱动侧交给分析器
///   （`DeadCodeAnalyzer::set_resolved_groups`）；
/// - 定义单元归属：`OverloadResolutions` 每单元一份（条目记在**记录方**单元），
///   名字形如 `{Type}.{method}#k`——`{Type}` 是**定义侧**的裸类型名且**不带单元**
///   （见 `typecheck/environment.rs` 的 `add_method_overload_with_params`）⇒ 一条
///   条目可能指向**多个单元**的同名类型（同名类型跨单元，见 `typed_owners`）；
///   故归因按「记录方能拼出该类型名的模块集合」给出，而不是单值反查；
/// - 只读：经 `OverloadResolutions::iter`（不触达 `get`、不计入消费台账）。
#[derive(Default)]
pub(in crate::driver) struct MethodMarks {
    pub by_unit: HashMap<String, HashSet<(String, usize)>>,
    pub groups: HashSet<String>,
    /// 定义单元键 → 该单元内被**算子派发**命中的 `Type.method` 组
    /// （WBS 3.20.3 / issue #505；整组——派发名不分重载候选）。
    /// 来源是 `TypeCheckResult::operator_dispatches`（RFC-011b span 键控
    /// 派发点，与 D20 重载决议表平行互不重叠的义务通道）。
    pub operator_by_unit: HashMap<String, HashSet<String>>,
}

/// 记录方单元的**类型名面**：定义侧裸类型名 → 该单元能拼出这个名字的模块键集合
/// （WBS 4.9.7 / issue #464 的归因输入）。
///
/// 三类来源（全部复用既有单点 `DeadCodeAnalyzer::collect_module_names` +
/// `resolve_import_module`，不另起路径解析）：
/// ① 花括号项 `use M.{T}`：`T` 是调用方写出的裸名，也是 t19 注入面的来源——
///    候选真从这里来，归属**确定**；
/// ② 内联别名 `use M.{T as X}` 与模块限定构造/访问 `ns.T`：调用点键仍是**定义侧**
///    `T`（实测：两种形态的 `X(..).to()` / `ns.T(..).to()` 都落键 `T.to`），
///    而 t19 的注入面按收窄规则**不**注入它们（`arms.rs::visible_method_overloads`
///    规则②）⇒ 条目与定义单元不再一一对应。这些模块按**可能归属**并入：漏并会让
///    「经别名/限定名真实被调用」的同名方法被 W1005 误报（误报防线，宁漏报方向）；
/// ③ 记录方自身：本单元定义的类型（含自有键位不外并的形态）。
///
/// 未在记录方名字面出现的模块**不**并入——这正是 4.9.7 收的缺口：跨单元同名同序的
/// **未调用**定义此前靠池的名字级豁免（短名全局平铺）蒙混过关。
fn typed_owners(
    ast: Option<&Module>,
    recorder_unit: &str,
    module_keys: &HashSet<String>,
) -> HashMap<String, HashSet<String>> {
    let mut owners: HashMap<String, HashSet<String>> = HashMap::new();
    let Some(ast) = ast else { return owners };
    let names = DeadCodeAnalyzer::collect_module_names(ast);
    // ① + ②：花括号项（含内联别名）—— local 是调用方写出的名字，target 是定义侧类型名
    for (_, target, use_path) in &names.imported {
        if let Some(key) = resolve_import_module(use_path, target, module_keys) {
            owners.entry(target.clone()).or_default().insert(key);
        }
    }
    // ②：模块限定访问 `ns.T`——字段名即定义侧类型名，命名空间已解析到模块
    for (base, field) in &names.namespaced_fields {
        let Some((_, use_path)) = names.namespaces.iter().find(|(ns, _)| ns == base) else {
            continue; // 基名不是命名空间导入（值上的方法调用）——不是类型名面
        };
        if let Some(key) = resolve_import_module(use_path, field, module_keys) {
            owners.entry(field.clone()).or_default().insert(key);
        }
    }
    // ③：记录方自身
    for key in owners.values_mut() {
        key.insert(recorder_unit.to_string());
    }
    owners
}

pub(in crate::driver) fn resolved_method_marks(
    units: &[Unit],
    asts: &[Option<Module>],
    type_results: &[Option<&TypeCheckResult>],
) -> MethodMarks {
    let module_keys: HashSet<String> = units.iter().map(|u| u.key.clone()).collect();
    // WBS 3.20.4 / issue #510：先**第一遍扫全部 type_results** 建全局实例化
    // 映射（形参名 → 具体类型名）——实例化请求记在**调用方**单元的
    // TypeCheckResult 里，派发记录（约束形参名）在**定义方**单元：映射必须
    // 跨单元聚合，派发循环里才能把 "T.add" 展开成 "Vec3.add" 再归因。
    // 形参名表从**全部单元的 AST** 提取一并传入：跨单元兜底路径的请求不带
    // type_params（探针实测 params=[]），名字只能回定义单元的声明里取。
    let fn_type_params = declared_type_param_names(
        type_results
            .iter()
            .enumerate()
            .filter_map(|(i, r)| Some((asts.get(i).and_then(|a| a.as_ref())?, (*r)?))),
    );
    let inst_map =
        generic_instantiation_map(type_results.iter().filter_map(|r| *r), &fn_type_params);
    let mut marks = MethodMarks::default();
    for (i, result) in type_results.iter().enumerate() {
        let Some(result) = result else { continue };
        let recorder_unit = units.get(i).map(|u| u.key.clone()).unwrap_or_default();
        let owners = typed_owners(
            asts.get(i).and_then(|a| a.as_ref()),
            &recorder_unit,
            &module_keys,
        );
        for (_, ir_name) in result.overload_resolutions.iter() {
            let (base, ordinal) = split_overload_ir_name(ir_name);
            let type_name = base.split('.').next().unwrap_or(&base);
            // 名字面里没有这个类型名（记录方自有类型且未出现在 import/限定访问里）时，
            // 归记录方自身——本地定义的候选由本单元现场登记，条目必来自本单元
            let mut defining: HashSet<String> = owners.get(type_name).cloned().unwrap_or_default();
            defining.insert(recorder_unit.clone());
            for key in defining {
                marks
                    .by_unit
                    .entry(key)
                    .or_default()
                    .insert((base.clone(), ordinal));
            }
            marks.groups.insert(base);
        }
        // 算子派发（WBS 3.20.3 / issue #505）：`u[0]` / `u == t` 的具名决议
        // `{type_name, method}` 同样按**定义单元**归因——归因面与 overload
        // 条目同一套（`typed_owners` 的可能归属并入 + 记录方自身）；派发名
        // `Type.method` 不分重载候选 ⇒ 记**整组**，分析器按组命中判活。
        //
        // WBS 3.20.4 / issue #510：派发名可能是**约束形参名**（泛型体内
        // `a + b` ⇒ 组 "T.add"）——先按实例化映射（上方跨单元聚合的
        // `inst_map`）展开成具体类型组（原组保留），再逐组走同一套归因：
        // 展开出的 "Vec3.add" 以其自身类型名查 `owners` 归到定义单元，
        // 原 "T.add" 照旧归记录方。
        let dispatch_bases = result
            .operator_dispatches
            .iter()
            .map(|d| format!("{}.{}", d.type_name, d.method));
        for base in expand_dispatch_groups(dispatch_bases, &inst_map) {
            let type_name = base.split('.').next().unwrap_or(&base);
            let mut defining: HashSet<String> = owners.get(type_name).cloned().unwrap_or_default();
            defining.insert(recorder_unit.clone());
            for key in defining {
                marks
                    .operator_by_unit
                    .entry(key)
                    .or_default()
                    .insert(base.clone());
            }
        }
    }
    marks
}

/// 形参名 → 已见具体类型名集合的**实例化映射**（WBS 3.20.4 / issue #510，
/// 映射构建的**单点**，两臂共用）。
///
/// typecheck 记录的算子派发名是**约束形参名**（`OperatorDispatch{type_name:"T"}`
/// ⇒ 组 "T.add"），而 `T → Vec3` 的具名化发生在 mono 特化（middle 层）——死代码
/// 分析在 typecheck 后运行，看不到实例化。但 typecheck 自己已记录全部实例化：
/// `TypeCheckResult::instantiation_requests` 的每条请求带 `generic_id.type_params`
/// （形参名表）与 `type_args`（实参表），按位 zip 即 形参名 → 实参类型名。
/// 同一形参多次实例化（sum@Int、sum@Vec3）取并集；跨泛型同名形参也取并集
/// （都表示「这个名字被绑定过的具体类型面」）。
///
/// `fn_type_params`（函数名 → 形参名表，见 [`declared_type_param_names`]）服务
/// **跨单元兜底路径**的请求：限定/花括号导入的调用走签名形态反推
/// （expressions.rs 的 `collect_instantiation_request` 第二条路径），发出的请求
/// `type_params` 为**空**（探针实测 params=[]）——此时按函数名查表补名再 zip。
/// 函数名做**尾段**匹配（`lib.sum` → `sum`）：AST 表按绑定裸名建，两种调用
/// 形态都命中；同名跨模块的短名误配方向 = 展开出多余组 ⇒ 至多漏报 W1005
/// （宁漏报方向，登记为边界）。
///
/// 聚合口径由调用方决定：**调用点在别的单元时请求记在那里的 type_result 里**
/// ——编排臂必须传全部单元的 TypeCheckResult（跨单元聚合），单文件臂传本单元
/// 一条（单文件形态无跨单元）。
///
/// 只收**具体类型实参**（Struct/Enum/Int/Float/Bool/Char 值形态）；
/// `MonoType::Generic`/`TypeRef` 等符号化实参不入映射——嵌套泛型深链不求值
/// （issue #510 登记边界）。`MonoType::type_name()` 的行为（`types/mono.rs`）：
/// Struct → 裸名（"Vec3"）、`Int(64)` → "int64"——展开出的 "int64.add" 无用户
/// 方法定义，组不命中任何定义，天然无害。
pub(in crate::driver) fn generic_instantiation_map<'a>(
    type_results: impl IntoIterator<Item = &'a TypeCheckResult>,
    fn_type_params: &HashMap<String, Vec<String>>,
) -> HashMap<String, HashSet<String>> {
    let mut map: HashMap<String, HashSet<String>> = HashMap::new();
    for result in type_results {
        for request in &result.instantiation_requests {
            let from_request = request.generic_id.type_params();
            let params: &[String] = if from_request.is_empty() {
                let bare = request
                    .generic_id
                    .name()
                    .rsplit('.')
                    .next()
                    .unwrap_or(request.generic_id.name());
                match fn_type_params.get(bare) {
                    Some(names) => names.as_slice(),
                    None => continue,
                }
            } else {
                from_request
            };
            for (param, arg) in params.iter().zip(request.type_args()) {
                let concrete = match arg {
                    MonoType::Struct(_)
                    | MonoType::Enum(_)
                    | MonoType::Bool
                    | MonoType::Char
                    | MonoType::Int(_)
                    | MonoType::Float(_) => arg.type_name(),
                    _ => continue,
                };
                map.entry(param.clone()).or_default().insert(concrete);
            }
        }
    }
    map
}

/// 泛型函数名（绑定裸名）→ 声明序类型形参名表，从**定义单元的 AST** 提取
/// （WBS 3.20.4 / issue #510）。
///
/// 为什么需要第二来源：形参名随导出携带的登记口（`checker.rs::collect_signatures`
/// 的 Assign 臂）只认 `(T: Type)`（MetaType）形态，约束泛型 `(T: Add)`
/// （`Type::Name` 约束名）不入表 ⇒ 导出面不带 ⇒ 跨单元请求的
/// `generic_id.type_params` 为空（探针实测），单一 zip 通路展不开。这里回到
/// 声明现场补读：判据用**现成单点** `ast::classify_generic_params`（与
/// typecheck 侧同一函数，语义不复制），is_trait 闭包按其调用点同源判据组装
/// （`trait_table.has_trait` + 内建算子接口 `operator_interfaces::spec`——
/// checker.rs 处理 `(T: Add)` 的同一对来源）。
///
/// 只扫顶层 `Assign`（`sum: (T: Add) -> … = …` 是顶层泛型绑定的承载形态，
/// `signature_params` 即第一组括号段）；普通值绑定的该段为空、值参数类型名
/// （`Vec3` 等）不满足 Type/ trait 判据 ⇒ 分类不出 Type 参数，天然无条目。
pub(in crate::driver) fn declared_type_param_names<'a>(
    units: impl IntoIterator<Item = (&'a Module, &'a TypeCheckResult)>
) -> HashMap<String, Vec<String>> {
    let mut out: HashMap<String, Vec<String>> = HashMap::new();
    for (ast, result) in units {
        let is_trait = |name: &str| {
            result.trait_table.has_trait(name) || operator_interfaces::spec(name).is_some()
        };
        for stmt in &ast.items {
            if let StmtKind::Assign {
                target,
                signature_params,
                ..
            } = &stmt.kind
            {
                if let Expr::Var(name, _) = target.as_ref() {
                    let names: Vec<String> = classify_generic_params(signature_params, &is_trait)
                        .into_iter()
                        .filter(|p| matches!(p.kind, GenericParamKind::Type))
                        .map(|p| p.name)
                        .collect();
                    if !names.is_empty() {
                        out.entry(name.clone()).or_insert(names);
                    }
                }
            }
        }
    }
    out
}

/// 派发组按实例化映射展开（WBS 3.20.4 / issue #510，展开逻辑的**单点**，
/// 两臂共用）：组 "{P}.{m}" 若 P 在映射中 ⇒ 追加全部 "{X}.{m}"（X ∈ map[P]）；
/// **原组保留**——用户类型可能真叫 "T"，且展开是追加不是替换。
pub(in crate::driver) fn expand_dispatch_groups(
    groups: impl IntoIterator<Item = String>,
    map: &HashMap<String, HashSet<String>>,
) -> HashSet<String> {
    let mut expanded = HashSet::new();
    for group in groups {
        expanded.insert(group.clone());
        if let Some((param, method)) = group.split_once('.') {
            if let Some(type_names) = map.get(param) {
                for type_name in type_names {
                    expanded.insert(format!("{type_name}.{method}"));
                }
            }
        }
    }
    expanded
}

/// `Type.method#k` → `(Type.method, k)`；无 `#` 后缀 ⇒ 声明序 0
fn split_overload_ir_name(ir_name: &str) -> (String, usize) {
    match ir_name.rsplit_once('#') {
        Some((base, idx)) => (base.to_string(), idx.parse().unwrap_or(0)),
        None => (ir_name.to_string(), 0),
    }
}

/// manifest 声明面优先的池来源（#334「包内 use 图不可达的报警」）：对每个包目标
/// root 跑一次 `discover_with_used`，把编译集里的文件现场词法+语法解析后装配池。
///
/// - root 取自 `roles::ExplicitSurfaces`（`[run].main`/`[[bin]]`/`[lib]`/
///   `[exports]` 指向的文件，路径已 canonicalize）——**不扫目录找入口**；
/// - 去重口径 = canonical 路径（与发现期同口径）；
/// - root 读不到（声明了却不存在）、词法/语法失败的文件静默跳过：池是辅助判定，
///   不允许它制造新的检查失败（解析失败的文件本就退出编译单元，4.10.1 同口径）；
/// - 模块键集合取各 root 的发现结果（import 归因的解析判据，改进 4）；角色用
///   `roles::classify` 单点判定（Test 不贡献消费面，改进 5）；
/// - 全部 root 都跑不通时返回 `None`，调用方据此退化为被检文件自身的编译集。
pub(in crate::driver) fn collect_package_refs(
    roots: &[PathBuf],
    project_root: Option<&Path>,
    surfaces: Option<&roles::ExplicitSurfaces>,
    test_rules: Option<&roles::TestRules>,
) -> Option<RefPool> {
    let mut seen: HashSet<PathBuf> = HashSet::new();
    let mut module_keys: HashSet<String> = HashSet::new();
    let mut owned: Vec<(String, PathBuf, Module, FileRole)> = Vec::new();
    let mut reached = false;
    for root in roots {
        let Ok((files, used_by, _)) = orchestrator::discover_with_used(root) else {
            continue;
        };
        reached = true;
        for file in files {
            let canon = file
                .path
                .canonicalize()
                .unwrap_or_else(|_| file.path.clone());
            // 模块键先记：语法失败的文件仍是一个已解析的 import 目标（无定义可豁免）
            module_keys.insert(file.module_key.clone());
            if !seen.insert(canon.clone()) {
                continue;
            }
            let Ok(tokens) = lexer::tokenize(&file.source) else {
                continue;
            };
            let parsed = parser::parse(&tokens);
            if parsed.has_errors {
                continue;
            }
            // 角色单点判定（roles::classify）：Test 不贡献消费面（改进 5）
            let role = roles::classify(
                &canon,
                project_root,
                surfaces,
                test_rules,
                used_by.contains(&canon),
                ast_has_main(&parsed.module),
            );
            owned.push((file.module_key, file.path, parsed.module, role));
        }
    }
    if !reached {
        return None; // 一个 root 都没跑通：调用方退化为被检文件自身的编译集
    }
    let modules: Vec<PoolModule<'_>> = owned
        .iter()
        .map(|(_, path, ast, role)| PoolModule {
            path,
            role: *role,
            ast,
        })
        .collect();
    Some(collect_ref_pool(&modules, &module_keys))
}
