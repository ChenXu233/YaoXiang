//! 模块注册表
//!
//! 管理所有已注册的模块，支持按路径查询和注册。
//! 提供统一的模块发现接口，合并 std 模块和用户模块。

use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

use super::symbol::{DefKind, DefPath, SymbolTable};
use super::{Export, ExportKind, ModuleError, ModuleInfo, ModuleSource};
use crate::frontend::core::sema::environment::MethodOverloadCandidates;
use crate::frontend::core::types::mono::MonoType;

/// 「限定名指向什么」的关注面——issue #494（WBS 3.6.6）三臂收敛后的查询参数。
///
/// 收敛前「这个限定名指向什么」在三处各答一次：原生名判定（原
/// `ModuleRegistry::is_native_name`）/ 命名空间臂（lower 侧
/// `registry.symbols().def_by_projection + DefKind` 分派）/ 别名判据（lower 侧
/// `qualified_export_is_code`，#479 引入、#493 改导出粒度）。按禁令三
/// （同一行为 ≥2 处复制 ⇒ 收敛到共享层），判据本体只保留在
/// [`ModuleRegistry::qualified_name_is`] 一处，调用点按本枚举传关注面；
/// **新增第四处消费走同一接口，不得再各写一份拆名/查表**。
///
/// 三个变体**不是同一谓词的三个别名**：对同一形态答案可以相反——
/// native 函数导出（`std.io.println`）对 [`QualifiedNameConcern::NativeName`]
/// 是「是」（Std 模块的已注册导出）、对 [`QualifiedNameConcern::FunctionCodeExport`]
/// 是「否」（native 导出没有 yx 函数体，不可物化为闭包）、对
/// [`QualifiedNameConcern::FunctionOrMethodDef`] 又是「是」（注册期已 intern
/// 为 Function def）。合并成单一布尔答案属行为变更；各臂语义与收敛前逐字节
/// 一致，形态全集 × 三关注面的答案表钉在 `src/frontend/module/tests/registry.rs`。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum QualifiedNameConcern {
    /// **原生名**（原 `ModuleRegistry::is_native_name` 判据，语义原样）：
    /// 末点拆名后命中**已注册 Std 模块**的导出——函数/常量/子模块导出都算
    /// （不看导出类别与实现来源）；用户/vendor 模块导出、未知名、无点裸名不算。
    NativeName,
    /// **函数代码**（原 lower 侧 `qualified_export_is_code` 判据，#479/#493，
    /// 语义原样）：导出类别是 Function **且**实现是 yx 源码——用户/vendor
    /// 模块的函数导出，或嵌入 yx 源（`src/std/yx_sources.rs`）的 std 函数导出；
    /// native 导出（#492 自己的形态）、常量、类型、子模块导出不算。
    /// 是「可物化为闭包的函数表条目」判据：作值时发 `MakeClosure`，零参调用
    /// 会把被调体的返回值装进变量（#479 的静默错值）。
    FunctionCodeExport,
    /// **可调用 def**（原 lower 侧命名空间臂判据，语义原样）：项目符号表
    /// （[`SymbolTable`]）里已 intern 的 Function/Method 绑定——不看模块表、
    /// 不看实现来源；方法 def（`Type.method`）与生成期合成 def 不是导出，
    /// 只有这一面能答。
    FunctionOrMethodDef,
}

/// 模块注册表
///
/// 存储所有已注册的模块信息，支持按路径查询。
/// 对外提供统一的模块发现接口。
///
/// 同时拥有项目级 [`SymbolTable`]：每个模块 `register` 时其顶层绑定（函数/类型/常量/方法）
/// 被增量 intern 为唯一 DefId。注册表是「项目有哪些模块」的权威，也是「绑定身份」的权威。
///
/// 符号表经 `Rc<RefCell>` 共享：`clone()` 的注册表（如传入各文件 IR 生成器）共享同一张表，
/// IR 生成期合成的闭包/匿名绑定 intern 进同一序号空间，DefId 全项目唯一。
#[derive(Debug, Default, Clone)]
pub struct ModuleRegistry {
    /// 模块映射（path -> ModuleInfo）
    modules: HashMap<String, ModuleInfo>,
    /// 项目级符号表：所有已注册模块的顶层绑定身份，随 register 增量构建。
    /// `Rc<RefCell>`：克隆体共享同一表，生成期 intern 全项目唯一。
    symbols: Rc<RefCell<SymbolTable>>,
}

// 嵌入 std 注册的重入保护（embedded_std_module_info → TypeChecker::new → with_std 会再入）。
thread_local! {
    static REGISTERING_EMBEDDED: std::cell::Cell<bool> = const { std::cell::Cell::new(false) };
}

impl ModuleRegistry {
    /// 创建新的注册表
    pub fn new() -> Self {
        Self {
            modules: HashMap::new(),
            symbols: Rc::new(RefCell::new(SymbolTable::new())),
        }
    }

    /// 创建包含 std 模块的注册表
    pub fn with_std() -> Self {
        let mut registry = Self::new();
        registry.register_std_modules();
        registry
    }

    /// 注册一个模块
    ///
    /// 同时把该模块的顶层绑定（导出 + 方法）增量 intern 进项目符号表。
    pub fn register(
        &mut self,
        module: ModuleInfo,
    ) {
        self.intern_module(&module);
        self.modules.insert(module.path.clone(), module);
    }

    /// 项目级符号表（所有已注册模块的绑定身份）。
    pub fn symbols(&self) -> std::cell::Ref<'_, SymbolTable> {
        self.symbols.borrow()
    }

    /// 项目级符号表（可变）：IR 生成期合成的闭包/匿名绑定经此 intern。
    pub fn symbols_mut(&self) -> std::cell::RefMut<'_, SymbolTable> {
        self.symbols.borrow_mut()
    }

    /// 把一个模块的导出与方法登记进项目符号表。
    ///
    /// 导出项按**路径**登记：模块键 + 导出裸名。用户模块与嵌入 yx std 的
    /// `full_path` 就是该路径的投影（`orchestrator::make_export` /
    /// `embedded_std_module_info` 两处都经 `DefPath::item(..).project()` 生成）；
    /// native 的 FFI 名允许是任意字面量，那类按**无模块扁平名**登记（见循环内注释），
    /// 投影仍与 `full_path` 逐字相同。子模块不是绑定，跳过。方法从
    /// `method_bindings`（键 `Type.method`）登记，链接到所属类型并构建静态 vtable。
    fn intern_module(
        &mut self,
        module: &ModuleInfo,
    ) {
        for export in module.exports.values() {
            let kind = match export.kind {
                ExportKind::Function => DefKind::Function,
                ExportKind::Type => DefKind::Type,
                ExportKind::Constant => DefKind::Constant,
                ExportKind::SubModule => continue,
            };
            // 导出面的 `full_path` 是权威投影：一般形态即 `{module}.{bare}`。native
            // 的 FFI 名允许是任意字面量（VM 内建 `int.to_string` 就注册在 std.convert
            // 下），那类不是「模块 + 裸名」的投影 —— 按**无模块扁平名**登记，
            // 投影仍与 `full_path` 逐字相同（否则调用侧按 full_path 查投影索引会落空）。
            let canonical = DefPath::item(&module.path, &export.name);
            let path = if canonical.project() == export.full_path {
                canonical
            } else {
                DefPath::item("", &export.full_path)
            };
            self.symbols.borrow_mut().intern(path, kind);
        }
        for method_key in module.method_bindings.keys() {
            // 键即方法路径：模块键 / 归属段 / 裸名三段在 typecheck 侧已经由
            // `TypeEnvironment::resolve_method_key` 定死（WBS 4.9.10 D-1/D-2），
            // 这里只登记身份，不再拆字符串（拆名是反解，投影语法只归 `DefPath`）。
            self.symbols.borrow_mut().intern_method(method_key.clone());
        }
    }

    /// 获取模块信息
    pub fn get(
        &self,
        path: &str,
    ) -> Option<&ModuleInfo> {
        self.modules.get(path)
    }

    /// 检查模块是否已注册
    pub fn has_module(
        &self,
        path: &str,
    ) -> bool {
        self.modules.contains_key(path)
    }

    /// 获取模块的导出项
    pub fn get_exports(
        &self,
        path: &str,
    ) -> Option<&HashMap<String, Export>> {
        self.modules.get(path).map(|m| &m.exports)
    }

    /// 获取所有模块的方法绑定（RFC-029 跨文件方法调用）。
    ///
    /// 键是方法身份 [`DefPath`]（WBS 4.9.10 D-1/D-2）：不同单元的同名类型各有
    /// 一条路径，合并**不会互相覆盖**——确定性来自身份而不是装配顺序，故本表
    /// 没有排序依赖（旧实现按模块键升序 + 后者覆盖，是字符串键空间的补丁）。
    pub fn all_method_bindings(&self) -> HashMap<DefPath, MonoType> {
        let mut bindings = HashMap::new();
        for module in self.modules.values() {
            for (key, ty) in &module.method_bindings {
                bindings.insert(key.clone(), ty.clone());
            }
        }
        bindings
    }

    /// 跨单元方法候选的**注入面**（WBS 4.9.10 D-1）：注册表全部模块**除本单元**。
    ///
    /// 键是方法身份，同名类型分属两个单元也各有路径 ⇒ 不需要按名字裁剪作用域
    /// （旧的 `merge_method_overloads(scoped)` 是字符串键空间的补丁）。本单元
    /// 自己的候选由本单元签名收集现场登记——重复注入会把候选翻倍。
    pub fn injectable_method_overloads(
        &self,
        own_module_key: &str,
    ) -> Vec<(DefPath, MethodOverloadCandidates)> {
        let mut out = Vec::new();
        for (path, module) in &self.modules {
            if path == own_module_key {
                continue;
            }
            for (key, group) in &module.method_overloads {
                out.push((key.clone(), group.clone()));
            }
        }
        out
    }

    /// 裸方法键（`Type.method`）→ **std** 方法身份：算子派发面的身份查询。
    ///
    /// 旧 `std_method_binding_qualified_map` 只做字符串拼接（`Type.method` →
    /// `{module}.Type.method`）；本查询用同一张身份表回答同一个问题：按
    /// **归属段 + 裸名**找 std 模块里的那条路径。仅收 std 模块（用户模块的裸方法键
    /// 由入口单元自己承载，跨单元限定会错分发）。模块键升序 ⇒ 同输入同结果。
    pub fn std_method_path(
        &self,
        type_bare: &str,
        method: &str,
    ) -> Option<DefPath> {
        let mut module_keys: Vec<&String> = self.modules.keys().collect();
        module_keys.sort();
        for module_key in module_keys {
            let module = self.modules.get(module_key)?;
            if module.source != ModuleSource::Std {
                continue;
            }
            let path = DefPath::method(&module.path, type_bare, method);
            if module.method_bindings.contains_key(&path) {
                return Some(path);
            }
        }
        None
    }

    /// std 模块导出的**类型名 → 提供模块键**（导入面的 std 基线，D-5 ④）。
    ///
    /// std 的类型名在每个单元都可见（与 `TypeChecker::new` 注入全部 std 方法
    /// 绑定同源），故它们不是「导入」而是**基线**。按（类型名，模块键）升序返回：
    /// 两个 std 模块导出同名类型时由建表侧 first-wins，升序保证同输入同结果。
    pub fn std_type_origins(&self) -> Vec<(String, String)> {
        let mut out: Vec<(String, String)> = Vec::new();
        for (path, module) in &self.modules {
            if module.source != ModuleSource::Std {
                continue;
            }
            for (name, export) in &module.exports {
                if export.kind == ExportKind::Type {
                    out.push((name.clone(), path.clone()));
                }
            }
        }
        out.sort();
        out
    }

    /// 解析模块路径，查找指定的导出项
    ///
    /// 支持以下模式：
    /// - `std.io` -> 返回 std.io 模块的所有导出
    /// - `std.io.println` -> 返回 std.io 模块中的 println
    pub fn resolve_export(
        &self,
        full_path: &str,
    ) -> Result<&Export, ModuleError> {
        // 尝试直接查找模块
        if let Some(module) = self.modules.get(full_path) {
            // 该路径是一个模块，如果它是命名空间模块则不报错
            if module.is_namespace() {
                // 返回第一个子模块的导出（若有）
                if let Some(first_export) = module.exports.values().next() {
                    return Ok(first_export);
                }
            }
        }

        // 尝试分割路径：最后一部分是导出名，前面是模块路径
        if let Some(dot_pos) = full_path.rfind('.') {
            let module_path = &full_path[..dot_pos];
            let export_name = &full_path[dot_pos + 1..];

            if let Some(module) = self.modules.get(module_path) {
                if let Some(export) = module.get_export(export_name) {
                    return Ok(export);
                }
                return Err(ModuleError::ExportNotFound {
                    name: export_name.to_string(),
                    module_path: module_path.to_string(),
                    available: module
                        .export_names()
                        .into_iter()
                        .map(String::from)
                        .collect(),
                });
            }
        }

        Err(ModuleError::NotFound {
            path: full_path.to_string(),
            searched_paths: self.modules.keys().cloned().collect(),
        })
    }

    /// 获取模块路径下所有可用的短名称到完整路径的映射
    ///
    /// 用于 IR 生成时解析函数调用。
    pub fn short_to_qualified_map(&self) -> HashMap<String, String> {
        let mut map = HashMap::new();
        for module in self.modules.values() {
            if module.source == ModuleSource::Std {
                for export in module.exports.values() {
                    if export.kind == ExportKind::Function || export.kind == ExportKind::Constant {
                        map.insert(export.name.clone(), export.full_path.clone());
                    }
                }
            }
        }
        map
    }

    /// 获取所有 native 函数名列表（用于 IR 生成的快速查找）
    pub fn native_names(&self) -> Vec<String> {
        let mut names = Vec::new();
        for module in self.modules.values() {
            if module.source == ModuleSource::Std {
                for export in module.exports.values() {
                    if export.kind == ExportKind::Function || export.kind == ExportKind::Constant {
                        names.push(export.full_path.clone());
                    }
                }
            }
        }
        names
    }

    /// 限定名指向什么——registry 侧单一查询接口（issue #494 / WBS 3.6.6）。
    ///
    /// 收敛前三份判据（原生名 / lower 侧 `qualified_export_is_code`（#479/#493）/
    /// lower 侧命名空间臂 `symbols().def_by_projection + DefKind`）各答一次「这个限定名指向
    /// 什么」，拆名与查表的机械面随之复制——按禁令三收拢为本接口；调用点各自
    /// 传关注面参数（[`QualifiedNameConcern`]），**新增第四处消费不得再各写一份**。
    /// 三臂语义与收敛前逐字节一致（形态全集 × 三关注面的答案表钉在
    /// `src/frontend/module/tests/registry.rs` 的面板 pin）。
    pub fn qualified_name_is(
        &self,
        qualified: &str,
        concern: QualifiedNameConcern,
    ) -> bool {
        match concern {
            QualifiedNameConcern::NativeName => {
                self.split_qualified(qualified)
                    .is_some_and(|(module, leaf)| {
                        module.has_export(leaf) && module.source == ModuleSource::Std
                    })
            }
            QualifiedNameConcern::FunctionCodeExport => {
                // 原 lower 侧 qualified_export_is_code 判据原样：Function 导出 +
                // 实现来源按导出粒度追问（只有 std 面需要这次追问——用户/vendor
                // 导出都是本仓源码）。
                let Some((module, leaf)) = self.split_qualified(qualified) else {
                    return false;
                };
                if !matches!(
                    module.exports.get(leaf).map(|export| &export.kind),
                    Some(ExportKind::Function)
                ) {
                    return false;
                }
                module.source != ModuleSource::Std || self.is_yx_std_export(qualified)
            }
            QualifiedNameConcern::FunctionOrMethodDef => {
                // 原 lower 侧命名空间臂判据原样：项目符号表 def 类别分派。
                let symbols = self.symbols.borrow();
                // 入参是本编译期的**投影产物**（调用名/限定名），故走投影索引；
                // 索引只读、不反解字符串（权威反查是 `SymbolTable::def_path`）。
                symbols.def_by_projection(qualified).is_some_and(|def| {
                    matches!(symbols.kind(def), DefKind::Function | DefKind::Method)
                })
            }
        }
    }

    /// 限定名 → (所属已注册模块, 叶子名) 的唯一拆名基底（issue #494）。
    ///
    /// 「按末点拆名 → 查模块表」此前在 is_native_name / is_yx_std_export /
    /// is_native_fn_export / lower 侧 qualified_export_is_code 四处各写一份
    /// （`rfind` 与 `rsplit_once` 两种写法并存，语义等价：都取**最后一个**点）；
    /// 收敛后经本基底，不得再各写一份。
    ///
    /// 与 [`Self::resolve_export`] 语义**不同、不可互替**：后者先试「整名即模块
    /// 路径」的命名空间分支（命中时返回该模块的第一个导出），本基底只按末点
    /// 拆名查所属模块——模块不存在（含无点裸名）即 `None`。
    fn split_qualified<'a>(
        &'a self,
        qualified: &'a str,
    ) -> Option<(&'a ModuleInfo, &'a str)> {
        let (module_path, leaf) = qualified.rsplit_once('.')?;
        let module = self.modules.get(module_path)?;
        Some((module, leaf))
    }

    /// 限定名导出是否**纯 yx 实现**（导出粒度，#493）。
    ///
    /// 数据源唯一：嵌入 YaoXiang 源（crate::std::yx_sources）里定义的导出集，
    /// 由 `register_std_modules` 在注册时逐名标记；未标记者
    /// （native 导出、用户模块导出、未知路径）一律 false。
    ///
    /// 为什么必须按导出粒度：std.result 是 native（is_ok/unwrap…）与纯 yx
    /// （Result 类型 + Try 方法）的混合面，而嵌入模块本身也带
    /// ModuleSource::Std——模块粒度区分不了「这一条导出是谁实现的」。
    pub fn is_yx_std_export(
        &self,
        full_path: &str,
    ) -> bool {
        self.split_qualified(full_path)
            .is_some_and(|(module, leaf)| module.is_yx_defined_export(leaf))
    }

    /// 限定名导出是否是**原生函数**导出（#492：原生函数作值）。
    ///
    /// 判据 = 已注册的 native 导出（[`Self::qualified_name_is`] 的
    /// [`QualifiedNameConcern::NativeName`] 面）**且** 不是纯 yx 实现
    /// （[`Self::is_yx_std_export`] 的补集）**且** 类别是 Function。
    /// 三个来源都是既有判据，这里只做组合，不在别处再写一份名字判断。
    ///
    /// native **常量**（std.math.PI）不算：它们作值的既有表示是零参调用臂，
    /// 行为必须逐字节不变。
    pub fn is_native_fn_export(
        &self,
        full_path: &str,
    ) -> bool {
        if !self.qualified_name_is(full_path, QualifiedNameConcern::NativeName)
            || self.is_yx_std_export(full_path)
        {
            return false;
        }
        let Some((module, leaf)) = self.split_qualified(full_path) else {
            return false;
        };
        matches!(
            module.exports.get(leaf).map(|export| &export.kind),
            Some(ExportKind::Function)
        )
    }

    /// 检查名称是否是 std 子模块（如 io, math, net, concurrent）
    pub fn is_std_submodule(
        &self,
        name: &str,
    ) -> bool {
        let path = DefPath::item("std", name).project();
        self.modules.contains_key(&path)
    }

    /// 获取所有 std 子模块的名称
    pub fn std_submodule_names(&self) -> Vec<String> {
        if let Some(std_module) = self.modules.get("std") {
            std_module.submodules.clone()
        } else {
            Vec::new()
        }
    }

    /// 注册所有 std 模块
    ///
    /// 使用 `StdModule` trait 自动发现和注册所有标准库模块。
    fn register_std_modules(&mut self) {
        // 注册根 std 模块
        let mut std_root = ModuleInfo::new("std".to_string(), ModuleSource::Std);
        // 从 std 模块自动获取所有子模块信息
        for module_info in crate::std::all_module_infos() {
            // 提取子模块名称（从 "std.io" -> "io"）
            let submodule_name = module_info
                .path
                .strip_prefix("std.")
                .unwrap_or(&module_info.path)
                .to_string();

            // 注册子模块
            std_root.add_submodule(submodule_name.clone());
            std_root.add_export(Export {
                name: submodule_name,
                full_path: module_info.path.clone(),
                kind: ExportKind::SubModule,
                signature: "Module".to_string(),
                mono_type: None,
                type_params: None,
                param_names: None,
                type_payload: None,
                definition_span: None, // std 虚拟模块无源码定义位置（4.6.1）
                predicate_def: None,   // std 虚拟子模块无谓词 def（#524 载荷面）
                proof_fn_params: None, // 同上（#527 载荷面）
                const_binders: Vec::new(), // native 导出无 yx 登记面（#539 载荷面）
                call_site_params: Vec::new(),
            });

            // 注册模块信息
            self.register(module_info);
        }

        // 嵌入的纯 YaoXiang std 模块（std.test 等，RFC-036 §4）：与 native 同级注册，
        // 使 `use std.test` 在单文件模式下也能解析并得到限定名。
        // 重入保护：embedded_std_module_info 内部会 TypeChecker::new → with_std()
        // 再入本函数——重入时跳过嵌入注册（内层 registry 仅作签名收集环境，
        // native 已就绪；std.test 自身无 native 依赖缺口）。
        if !REGISTERING_EMBEDDED.with(|f| f.get()) {
            REGISTERING_EMBEDDED.with(|f| f.set(true));
            for (file, _) in crate::std::yx_sources::STD_YX_FILES {
                let use_path = file.strip_suffix(".yx").unwrap_or(file).replace('/', ".");
                if let Some(mut info) = crate::std::yx_sources::embedded_std_module_info(&use_path)
                {
                    // 导出粒度判据的数据源（#493）：嵌入源里定义的每个导出都是
                    // **纯 yx 实现**——先整体标记，再走下面的合并/注册（两条分支
                    // 都带着标记走）。标记只在这里产生（见 ModuleInfo::mark_yx_defined_export）。
                    let yx_defined: Vec<String> = info.exports.keys().cloned().collect();
                    for name in &yx_defined {
                        info.mark_yx_defined_export(name);
                    }
                    // 同名路径的 native 模块（如 std.result 已有 is_ok/unwrap 等
                    // native 导出）：**表面合并**而非整体覆盖——yx 导出与 native
                    // 导出按名字并集，同名冲突 yx 优先（纯 YaoXiang 定义是权威
                    // 表面），方法绑定同规则。注册顺序在 native 之后，直接
                    // insert 会把 native 导出整个顶掉（result.is_ok 报 E1042）。
                    if let Some(existing) = self.modules.get_mut(&info.path) {
                        // 标记随表面合并一起来：is_yx_std_export 必须能对
                        // std.result 里的 yx 导出说「是」、对 native 导出说「否」。
                        for name in &yx_defined {
                            existing.mark_yx_defined_export(name);
                        }
                        for (name, export) in info.exports {
                            existing.exports.insert(name, export);
                        }
                        for (key, ty) in info.method_bindings {
                            existing.method_bindings.insert(key, ty);
                        }
                        let submodule_name = info
                            .path
                            .strip_prefix("std.")
                            .unwrap_or(&info.path)
                            .to_string();
                        std_root.add_submodule(submodule_name.clone());
                        continue;
                    }
                    let submodule_name = info
                        .path
                        .strip_prefix("std.")
                        .unwrap_or(&info.path)
                        .to_string();
                    std_root.add_submodule(submodule_name.clone());
                    // 与 native 子模块同款：`std` 根也要**导出**子模块名，
                    // 否则 `use std.{io, list}` 的内联导入形式查 `std` 的 exports
                    // 时报 E5003「Export 'list' not found in module 'std'」。
                    // 此前只 add_submodule 未 add_export，故 `use std.list` 可用
                    // 而 `use std.{list}` 不可用。
                    std_root.add_export(Export {
                        name: submodule_name,
                        full_path: info.path.clone(),
                        kind: ExportKind::SubModule,
                        signature: "Module".to_string(),
                        mono_type: None,
                        type_params: None,
                        param_names: None,
                        type_payload: None,
                        definition_span: None, // std 虚拟模块无源码定义位置（4.6.1）
                        predicate_def: None,   // std 虚拟子模块无谓词 def（#524 载荷面）
                        proof_fn_params: None, // 同上（#527 载荷面）
                        const_binders: Vec::new(), // native 导出无 yx 登记面（#539 载荷面）
                        call_site_params: Vec::new(),
                    });
                    self.register(info);
                }
            }
            REGISTERING_EMBEDDED.with(|f| f.set(false));
        }

        // 注册根 std 模块
        self.register(std_root);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::frontend::core::types::mono::MonoType;

    /// 表面锁：std.result 合并表面 = yx（Result 类型 + Try 四方法）∪
    /// native（is_ok/unwrap 等工具族）；ok/err 构造器已退役（变体构造
    /// 语法 `Result(T, E).ok(v)` 是唯一构造通道）。
    #[test]
    fn std_result_merged_surface() {
        let registry = ModuleRegistry::with_std();
        let m = registry
            .get("std.result")
            .expect("std.result should be registered");
        assert!(m.exports.contains_key("Result"), "yx type export missing");
        for method in crate::frontend::core::sema::operator_interfaces::TRY_METHODS {
            // 键是方法身份（D-1）：std.result 的 Result 方法归该模块键
            let key = DefPath::method("std.result", "Result", method);
            assert!(
                m.method_bindings.contains_key(&key),
                "missing yx Try method {}",
                key.project()
            );
        }
        for f in [
            "is_ok",
            "is_err",
            "unwrap",
            "unwrap_or",
            "unwrap_err",
            "code",
            "message",
        ] {
            assert!(m.exports.contains_key(f), "missing native export {f}");
        }
        assert!(!m.exports.contains_key("ok"), "native ok must stay retired");
        assert!(
            !m.exports.contains_key("err"),
            "native err must stay retired"
        );
    }

    fn export(
        name: &str,
        full_path: &str,
        kind: ExportKind,
    ) -> Export {
        Export {
            name: name.to_string(),
            full_path: full_path.to_string(),
            kind,
            signature: String::new(),
            mono_type: Some(MonoType::Int(64)),
            type_params: None,
            param_names: None,
            type_payload: None,
            definition_span: None,
            predicate_def: None,
            proof_fn_params: None,
            const_binders: Vec::new(), // 测试桩导出无登记面（#539 载荷面）
            call_site_params: Vec::new(),
        }
    }

    /// WBS 4.9.10 D-1：**同名类型分属两个单元**时，候选按方法身份各归各家。
    ///
    /// 判据（取代字符串键空间的 `merge_method_overloads(scoped)` 钉板）：
    /// ① 注入面 = 注册表全部模块**除本单元**——本单元自己的候选由本单元登记，
    ///    重复注入会翻倍；② `a.Temp.to` 与 `b.Temp.to` 是**两条身份**，既不合并
    ///    也不互相覆盖（旧的字符串键空间里它们撞同一个键，只能靠作用域裁剪补）；
    /// ③ 候选与形参名同组同序（配对不得错位）。
    ///
    /// 过**证明**门：换一个正确的替代实现（例如改成按键排序后合并）本测试仍绿——
    /// 它守的是「身份区分同名类型」这条与实现选择无关的性质。
    #[test]
    fn injectable_method_overloads_keeps_same_name_types_distinct() {
        // Arrange：a / b 两个模块各自登记**同名类型** Temp 的 to 方法（签名可区分）
        let mut registry = ModuleRegistry::new();
        let mut a = ModuleInfo::new("a".to_string(), ModuleSource::User);
        let a_key = DefPath::method("a", "Temp", "to");
        a.method_overloads.insert(
            a_key.clone(),
            MethodOverloadCandidates {
                candidates: vec![MonoType::Int(64)],
                param_names: vec![vec!["self".to_string()]],
            },
        );
        let mut b = ModuleInfo::new("b".to_string(), ModuleSource::User);
        let b_key = DefPath::method("b", "Temp", "to");
        b.method_overloads.insert(
            b_key.clone(),
            MethodOverloadCandidates {
                candidates: vec![MonoType::Bool],
                param_names: vec![vec!["self".to_string()]],
            },
        );
        registry.register(a);
        registry.register(b);

        // Act：两个单元各自的注入面
        let from_a = registry.injectable_method_overloads("a");
        let from_b = registry.injectable_method_overloads("b");

        // Assert：注入面 = 全部模块除本单元（含顺序无关，因为键是身份）
        assert_eq!(
            from_a.len(),
            1,
            "a 的注入面只应含 b 的一组候选，实际: {from_a:?}"
        );
        assert_eq!(from_a[0].0, b_key, "a 的注入面不得含自己的候选");
        assert_eq!(
            from_a[0].1.candidates,
            vec![MonoType::Bool],
            "b 的候选必须原样带出"
        );
        assert_eq!(from_b[0].0, a_key, "b 的注入面不得含自己的候选");
        assert_eq!(
            from_b[0].1.candidates,
            vec![MonoType::Int(64)],
            "a 的候选必须原样带出"
        );
        // Assert：两条身份互不覆盖——同名类型在两个单元里各有自己的路径
        assert_ne!(
            a_key.project(),
            b_key.project(),
            "同名类型分属两单元必须投影不同（a.Temp.to / b.Temp.to）"
        );
        // Assert：配对同组同序（候选 ↔ 形参名）
        assert_eq!(
            from_a[0].1.candidates.len(),
            from_a[0].1.param_names.len(),
            "候选与形参名必须逐个对齐"
        );
    }
    /// WBS 4.9.10 D-1：`all_method_bindings` 的**同名类型不互相覆盖**。
    ///
    /// 旧判据（病史门）钉的是字符串键空间里的覆盖顺序缺陷：两个模块定义同名
    /// `Temp.tag` 时按 HashMap 迭代序后者覆盖前者，同一输入跨进程给出不同单绑定。
    /// 键改成方法身份后该缺陷**不可表达**：`a.Temp.tag` 与 `b.Temp.tag` 是两条
    /// 路径，各自留下自己的单绑定——排序依赖随之消失（无覆盖即无顺序）。
    ///
    /// 过**证明**门：换一个正确的替代实现（例如仍按模块键升序合并）本测试仍绿；
    /// 它守的是「同名类型各有身份、合并不得丢一条」这条与实现选择无关的性质。
    #[test]
    fn test_all_method_bindings_keeps_same_name_types_of_two_modules() {
        // Arrange：a / b 两个模块各自定义同名类型 Temp 的 tag，单绑定类型可区分
        let mut registry = ModuleRegistry::new();
        let a_key = DefPath::method("a", "Temp", "tag");
        let b_key = DefPath::method("b", "Temp", "tag");
        let mut a = ModuleInfo::new("a".to_string(), ModuleSource::User);
        a.method_bindings.insert(a_key.clone(), MonoType::Bool);
        let mut b = ModuleInfo::new("b".to_string(), ModuleSource::User);
        b.method_bindings.insert(b_key.clone(), MonoType::Int(64));
        registry.register(a);
        registry.register(b);

        // Act
        let merged = registry.all_method_bindings();

        // Assert：两条身份各自留下自己的单绑定（同名类型不互相覆盖）
        assert_eq!(
            merged.get(&a_key).cloned(),
            Some(MonoType::Bool),
            "a.Temp.tag 的单绑定必须留在合并结果里，实际: {merged:?}"
        );
        assert_eq!(
            merged.get(&b_key).cloned(),
            Some(MonoType::Int(64)),
            "b.Temp.tag 的单绑定必须留在合并结果里，实际: {merged:?}"
        );
    }

    #[test]
    fn register_interns_exports_and_methods() {
        let mut registry = ModuleRegistry::new();
        let mut lib = ModuleInfo::new("lib".to_string(), ModuleSource::User);
        lib.add_export(export("helper", "lib.helper", ExportKind::Function));
        lib.add_export(export("Point", "lib.Point", ExportKind::Type));
        lib.add_export(export("PI", "lib.PI", ExportKind::Constant));
        // 方法绑定：Point.get_x / Point.get_y
        lib.method_bindings
            .insert(DefPath::method("lib", "Point", "get_x"), MonoType::Int(64));
        lib.method_bindings
            .insert(DefPath::method("lib", "Point", "get_y"), MonoType::Int(64));
        registry.register(lib);

        let symbols = registry.symbols();
        // 导出按类别登记
        let helper = symbols
            .def_by_projection("lib.helper")
            .expect("function interned");
        let point = symbols
            .def_by_projection("lib.Point")
            .expect("type interned");
        let pi = symbols
            .def_by_projection("lib.PI")
            .expect("constant interned");
        assert_eq!(symbols.kind(helper), DefKind::Function);
        assert_eq!(symbols.kind(point), DefKind::Type);
        assert_eq!(symbols.kind(pi), DefKind::Constant);

        // 方法登记并链接到类型，构建静态 vtable
        let get_x = symbols
            .def_by_projection("lib.Point.get_x")
            .expect("method interned");
        assert_eq!(symbols.kind(get_x), DefKind::Method);
        assert_eq!(symbols.parent(get_x), Some(point));
        let vt = symbols.vtable(point);
        assert_eq!(vt.len(), 2);
        assert!(vt.iter().any(|(n, d)| n == "get_x" && *d == get_x));
    }

    #[test]
    fn register_is_idempotent_for_symbols() {
        let mut registry = ModuleRegistry::new();
        let mut lib = ModuleInfo::new("lib".to_string(), ModuleSource::User);
        lib.add_export(export("helper", "lib.helper", ExportKind::Function));
        registry.register(lib.clone());
        registry.register(lib);
        // 重复注册不产生重复 DefId
        assert_eq!(
            registry
                .symbols()
                .def_by_projection("lib.helper")
                .map(|d| d.0),
            Some(0)
        );
        assert_eq!(registry.symbols().len(), 1);
    }

    #[test]
    fn with_std_interns_std_exports() {
        let registry = ModuleRegistry::with_std();
        // std.io.println 应被 intern 为函数
        let println = registry.symbols().def_by_projection("std.io.println");
        assert!(println.is_some(), "std.io.println interned");
        assert_eq!(registry.symbols().kind(println.unwrap()), DefKind::Function);
    }
}
