//! 模块注册表
//!
//! 管理所有已注册的模块，支持按路径查询和注册。
//! 提供统一的模块发现接口，合并 std 模块和用户模块。

use std::collections::HashMap;

use super::symbol::{DefKind, SymbolTable};
use super::{Export, ExportKind, ModuleError, ModuleInfo, ModuleSource};
use crate::frontend::core::types::mono::MonoType;

/// 模块注册表
///
/// 存储所有已注册的模块信息，支持按路径查询。
/// 对外提供统一的模块发现接口。
///
/// 同时拥有项目级 [`SymbolTable`]：每个模块 `register` 时其顶层绑定（函数/类型/常量/方法）
/// 被增量 intern 为唯一 DefId。注册表是「项目有哪些模块」的权威，也是「绑定身份」的权威。
#[derive(Debug, Default, Clone)]
pub struct ModuleRegistry {
    /// 模块映射（path -> ModuleInfo）
    modules: HashMap<String, ModuleInfo>,
    /// 项目级符号表：所有已注册模块的顶层绑定身份，随 register 增量构建。
    symbols: SymbolTable,
}

impl ModuleRegistry {
    /// 创建新的注册表
    pub fn new() -> Self {
        Self {
            modules: HashMap::new(),
            symbols: SymbolTable::new(),
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
    pub fn symbols(&self) -> &SymbolTable {
        &self.symbols
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
            self.symbols.intern_full(&export.full_path, kind);
        }
        for method_key in module.method_bindings.keys() {
            if let Some((type_bare, method)) = method_key.split_once('.') {
                self.symbols.intern_method(&module.path, type_bare, method);
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
            });

            // 注册模块信息
            self.register(module_info);
        }

        // 注册根 std 模块
        self.register(std_root);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::frontend::core::types::mono::MonoType;

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
        }
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
