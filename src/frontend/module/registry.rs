//! 模块注册表
//!
//! 管理所有已注册的模块，支持按路径查询和注册。
//! 提供统一的模块发现接口，合并 std 模块和用户模块。

use std::collections::HashMap;
use std::cell::RefCell;
use std::rc::Rc;

use super::symbol::{DefKind, SymbolTable};
use super::{Export, ExportKind, ModuleError, ModuleInfo, ModuleSource};
use crate::frontend::core::typecheck::environment::MethodOverloadCandidates;
use crate::frontend::core::types::mono::MonoType;

/// 模块注册表
///
/// 存储所有已注册的模块信息，支持按路径查询。
/// 对外提供统一的模块发现接口。
///
/// 同时拥有项目级 [`SymbolTable`]：每个模块 `register` 时其顶层绑定（函数/类型/常量/方法）
/// 被增量 intern 为唯一 DefId。注册表是「项目有哪些模块」的权威，也是「绑定身份」的权威。
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
    /// 限定名直接采用 `export.full_path`（即 `{module_key}.{bare}`，与 SymbolTable::qualify 一致）。
    /// 子模块不是绑定，跳过。方法从 `method_bindings`（键 `Type.method`）登记，
    /// 链接到所属类型并构建静态 vtable。
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
            self.symbols
                .borrow_mut()
                .intern_full(&export.full_path, kind);
        }
        for method_key in module.method_bindings.keys() {
            if let Some((type_bare, method)) = method_key.split_once('.') {
                self.symbols
                    .borrow_mut()
                    .intern_method(&module.path, type_bare, method);
            }
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
    pub fn all_method_bindings(&self) -> HashMap<String, MonoType> {
        let mut bindings = HashMap::new();
        for module in self.modules.values() {
            bindings.extend(module.method_bindings.clone());
        }
        bindings
    }

    /// 方法**重载候选**表的确定性合并（RFC-029f 改进 4 / WBS 4.9.7 / issue #464）：
    /// 对 `scoped` 给出的模块键**升序**逐模块追加候选组——模块间按键序、模块内按
    /// 定义序，任一键位下的候选顺序因此与 HashMap 迭代序无关。
    ///
    /// 为什么带作用域参数而不是无条件"全注册表并集"：候选按**裸类型名**键控
    /// （`{Type}.{method}`）。两个单元各自定义同名类型时键相同，把调用单元
    /// **命名不到**的单元的候选并进来会让同一调用点匹配到两个签名相同的候选 ⇒
    /// 匹配数虚高、按「歧义即报错」规则报 E1107 误报（实测形态见
    /// `driver::helpers::visible_method_overloads` 的注释）。作用域由调用方按
    /// import face 单点给出，本函数不做任何名字解析。
    pub fn merge_method_overloads(
        &self,
        scoped: &[String],
    ) -> HashMap<String, MethodOverloadCandidates> {
        let mut modules: Vec<&String> = scoped.iter().collect();
        modules.sort();
        modules.dedup();
        let mut merged: HashMap<String, MethodOverloadCandidates> = HashMap::new();
        for path in modules {
            let Some(module) = self.modules.get(path) else {
                continue; // 不在注册表里的模块键不贡献候选（宁漏勿误）
            };
            for (key, group) in &module.method_overloads {
                merged.entry(key.clone()).or_default().merge_from(group);
            }
        }
        merged
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

    /// 方法绑定的**限定名**映射（`"Result.is_failure"` → `"std.result.Result.is_failure"`）。
    ///
    /// yx std 模块的方法（`Type.method` 定义）在方法调用点按 `{类型名}.{方法}`
    /// 命名，但 IR 函数表里的名字是「模块路径 + 方法全键」限定的
    /// （`std.result.Result.is_failure`，模块内原名含类型前缀）——此映射补齐。
    /// 仅收录 std 模块（用户模块方法本就在入口 IR 里以 `Type.method` 原名
    /// 存在，限定会错分发）。
    pub fn std_method_binding_qualified_map(&self) -> HashMap<String, String> {
        let mut map = HashMap::new();
        for module in self.modules.values() {
            if module.source != ModuleSource::Std {
                continue;
            }
            for key in module.method_bindings.keys() {
                map.entry(key.clone())
                    .or_insert_with(|| format!("{}.{}", module.path, key));
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

    /// 检查完整路径是否是已注册的 native 函数/常量
    pub fn is_native_name(
        &self,
        full_path: &str,
    ) -> bool {
        // 尝试从路径中解析模块路径和导出名
        if let Some(dot_pos) = full_path.rfind('.') {
            let module_path = &full_path[..dot_pos];
            let export_name = &full_path[dot_pos + 1..];
            if let Some(module) = self.modules.get(module_path) {
                return module.has_export(export_name) && module.source == ModuleSource::Std;
            }
        }
        false
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
        let Some((module_path, export_name)) = full_path.rsplit_once('.') else {
            return false;
        };
        self.modules
            .get(module_path)
            .is_some_and(|module| module.is_yx_defined_export(export_name))
    }

    /// 检查名称是否是 std 子模块（如 io, math, net, concurrent）
    pub fn is_std_submodule(
        &self,
        name: &str,
    ) -> bool {
        let path = format!("std.{}", name);
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
        for method in crate::frontend::core::typecheck::operator_interfaces::TRY_METHODS {
            let key = format!("Result.{method}");
            assert!(
                m.method_bindings.contains_key(&key),
                "missing yx Try method {key}"
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
        }
    }

    /// WBS 4.9.7 / #464：跨单元候选合并的**确定性**与**作用域**（单一合并入口的钉板）。
    ///
    /// 判据：① 模块间按键升序（与调用方给出的顺序无关）；② 作用域外的模块不贡献候选；
    /// ③ 候选三元组（候选/混编名/形参名）同序对齐——配对不得错位。
    #[test]
    fn merge_method_overloads_is_deterministic_and_scoped() {
        // Arrange：a / b 两个模块各自登记同名键 "Temp.to" 的候选组（a 的候选为 Int，b 为 Bool）
        let mut registry = ModuleRegistry::new();
        let mut a = ModuleInfo::new("a".to_string(), ModuleSource::User);
        a.method_overloads.insert(
            "Temp.to".to_string(),
            MethodOverloadCandidates {
                candidates: vec![MonoType::Int(64)],
                ir_names: vec!["Temp.to".to_string()],
                param_names: vec![vec!["self".to_string()]],
            },
        );
        let mut b = ModuleInfo::new("b".to_string(), ModuleSource::User);
        b.method_overloads.insert(
            "Temp.to".to_string(),
            MethodOverloadCandidates {
                candidates: vec![MonoType::Bool],
                ir_names: vec!["Temp.to".to_string()],
                param_names: vec![vec!["self".to_string()]],
            },
        );
        registry.register(a);
        registry.register(b);

        // Act：同一作用域集合按两种给出顺序各并一次；再各取「只含 a」「只含 b」「空」三种作用域
        let by_ba = registry.merge_method_overloads(&["b".to_string(), "a".to_string()]);
        let by_ab = registry.merge_method_overloads(&["a".to_string(), "b".to_string()]);
        let only_a = registry.merge_method_overloads(&["a".to_string()]);
        let empty = registry.merge_method_overloads(&[]);

        // Assert：合并序 = 模块键升序（a 先 b 后），与调用方顺序无关
        let merged = by_ba
            .get("Temp.to")
            .expect("a/b 都在作用域内 ⇒ 键位必须存在");
        assert_eq!(
            merged.candidates,
            vec![MonoType::Int(64), MonoType::Bool],
            "候选必须按模块键升序合并（a 先、b 后），实际: {:?}",
            merged.candidates
        );
        assert_eq!(
            by_ab.get("Temp.to").map(|group| group.candidates.clone()),
            Some(merged.candidates.clone()),
            "合并结果不得依赖调用方给出的模块顺序"
        );
        // Assert：三元组同序对齐（配对错位 ⇒ 调用点名与定义名不再对应）
        assert_eq!(
            (merged.ir_names.len(), merged.param_names.len()),
            (merged.candidates.len(), merged.candidates.len()),
            "混编名/形参名必须与候选逐个对齐，实际: names={:?} params={:?}",
            merged.ir_names,
            merged.param_names
        );
        // Assert：作用域是唯一入口——作用域外的模块不贡献候选
        assert_eq!(
            only_a.get("Temp.to").map(|group| group.candidates.clone()),
            Some(vec![MonoType::Int(64)]),
            "作用域只含 a 时不得并入 b 的候选"
        );
        assert!(empty.is_empty(), "空作用域必须得到空表（无候选来源）");
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
            .insert("Point.get_x".to_string(), MonoType::Int(64));
        lib.method_bindings
            .insert("Point.get_y".to_string(), MonoType::Int(64));
        registry.register(lib);

        let symbols = registry.symbols();
        // 导出按类别登记
        let helper = symbols.def("lib.helper").expect("function interned");
        let point = symbols.def("lib.Point").expect("type interned");
        let pi = symbols.def("lib.PI").expect("constant interned");
        assert_eq!(symbols.kind(helper), DefKind::Function);
        assert_eq!(symbols.kind(point), DefKind::Type);
        assert_eq!(symbols.kind(pi), DefKind::Constant);

        // 方法登记并链接到类型，构建静态 vtable
        let get_x = symbols.def("lib.Point.get_x").expect("method interned");
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
        assert_eq!(registry.symbols().def("lib.helper").map(|d| d.0), Some(0));
        assert_eq!(registry.symbols().len(), 1);
    }

    #[test]
    fn with_std_interns_std_exports() {
        let registry = ModuleRegistry::with_std();
        // std.io.println 应被 intern 为函数
        let println = registry.symbols().def("std.io.println");
        assert!(println.is_some(), "std.io.println interned");
        assert_eq!(registry.symbols().kind(println.unwrap()), DefKind::Function);
    }
}
