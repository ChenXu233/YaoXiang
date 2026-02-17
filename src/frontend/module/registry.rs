//! 模块注册表
//!
//! 管理所有已注册的模块，支持按路径查询和注册。
//! 提供统一的模块发现接口，合并 std 模块和用户模块。

use std::collections::HashMap;

use super::{Export, ExportKind, ModuleError, ModuleInfo, ModuleSource};

/// 模块注册表
///
/// 存储所有已注册的模块信息，支持按路径查询。
/// 对外提供统一的模块发现接口。
#[derive(Debug, Default)]
pub struct ModuleRegistry {
    /// 模块映射（path -> ModuleInfo）
    modules: HashMap<String, ModuleInfo>,
}

impl ModuleRegistry {
    /// 创建新的注册表
    pub fn new() -> Self {
        Self {
            modules: HashMap::new(),
        }
    }

    /// 创建包含 std 模块的注册表
    pub fn with_std() -> Self {
        let mut registry = Self::new();
        registry.register_std_modules();
        registry
    }

    /// 注册一个模块
    pub fn register(
        &mut self,
        module: ModuleInfo,
    ) {
        self.modules.insert(module.path.clone(), module);
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
        self.modules.get(&path).is_some()
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
    /// 从 `crate::std` 中的 `native_declarations()` 自动发现和注册所有标准库模块。
    fn register_std_modules(&mut self) {
        // 注册根 std 模块
        let mut std_root = ModuleInfo::new("std".to_string(), ModuleSource::Std);

        // 注册 std.io
        let io_module = Self::build_std_submodule(
            "std.io",
            crate::std::io::native_declarations()
                .into_iter()
                .filter(|d| d.implemented)
                .map(|d| (d.name, d.native_name, d.signature))
                .collect(),
        );
        std_root.add_submodule("io".to_string());
        std_root.add_export(Export {
            name: "io".to_string(),
            full_path: "std.io".to_string(),
            kind: ExportKind::SubModule,
            signature: "Module".to_string(),
        });
        self.register(io_module);

        // 注册 std.math
        let math_module = Self::build_std_submodule(
            "std.math",
            crate::std::math::native_declarations()
                .into_iter()
                .filter(|d| d.implemented)
                .map(|d| (d.name, d.native_name, d.signature))
                .collect(),
        );
        std_root.add_submodule("math".to_string());
        std_root.add_export(Export {
            name: "math".to_string(),
            full_path: "std.math".to_string(),
            kind: ExportKind::SubModule,
            signature: "Module".to_string(),
        });
        self.register(math_module);

        // 注册 std.net
        let net_module = Self::build_std_submodule(
            "std.net",
            crate::std::net::native_declarations()
                .into_iter()
                .filter(|d| d.implemented)
                .map(|d| (d.name, d.native_name, d.signature))
                .collect(),
        );
        std_root.add_submodule("net".to_string());
        std_root.add_export(Export {
            name: "net".to_string(),
            full_path: "std.net".to_string(),
            kind: ExportKind::SubModule,
            signature: "Module".to_string(),
        });
        self.register(net_module);

        // 注册 std.concurrent
        let concurrent_module = Self::build_std_submodule(
            "std.concurrent",
            crate::std::concurrent::native_declarations()
                .into_iter()
                .filter(|d| d.implemented)
                .map(|d| (d.name, d.native_name, d.signature))
                .collect(),
        );
        std_root.add_submodule("concurrent".to_string());
        std_root.add_export(Export {
            name: "concurrent".to_string(),
            full_path: "std.concurrent".to_string(),
            kind: ExportKind::SubModule,
            signature: "Module".to_string(),
        });
        self.register(concurrent_module);

        // 注册根 std 模块
        self.register(std_root);
    }

    /// 从 NativeDeclaration 列表构建 std 子模块
    fn build_std_submodule(
        module_path: &str,
        declarations: Vec<(&'static str, &'static str, &'static str)>,
    ) -> ModuleInfo {
        let mut module = ModuleInfo::new(module_path.to_string(), ModuleSource::Std);

        for (name, native_name, signature) in declarations {
            // 判断是函数还是常量
            let kind = if signature.starts_with('(') {
                ExportKind::Function
            } else {
                ExportKind::Constant
            };

            module.add_export(Export {
                name: name.to_string(),
                full_path: native_name.to_string(),
                kind,
                signature: signature.to_string(),
            });
        }

        module
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_registry_with_std() {
        let registry = ModuleRegistry::with_std();

        // 验证 std 根模块存在
        assert!(registry.has_module("std"));

        // 验证 std 子模块存在
        assert!(registry.has_module("std.io"));
        assert!(registry.has_module("std.math"));
        assert!(registry.has_module("std.net"));
        assert!(registry.has_module("std.concurrent"));
    }

    #[test]
    fn test_registry_exports() {
        let registry = ModuleRegistry::with_std();

        // 验证 std.io 有 println 导出
        let io_module = registry.get("std.io").unwrap();
        assert!(io_module.has_export("println"));
        assert!(io_module.has_export("print"));

        // 验证 std.math 有 PI 导出
        let math_module = registry.get("std.math").unwrap();
        assert!(math_module.has_export("PI"));
    }

    #[test]
    fn test_resolve_export() {
        let registry = ModuleRegistry::with_std();

        // 解析 std.io.println
        let export = registry.resolve_export("std.io.println").unwrap();
        assert_eq!(export.name, "println");
        assert_eq!(export.full_path, "std.io.println");
    }

    #[test]
    fn test_short_to_qualified_map() {
        let registry = ModuleRegistry::with_std();
        let map = registry.short_to_qualified_map();

        assert_eq!(map.get("println"), Some(&"std.io.println".to_string()));
        assert_eq!(map.get("print"), Some(&"std.io.print".to_string()));
    }

    #[test]
    fn test_is_native_name() {
        let registry = ModuleRegistry::with_std();

        assert!(registry.is_native_name("std.io.println"));
        assert!(registry.is_native_name("std.math.PI"));
        assert!(!registry.is_native_name("user.module.func"));
    }

    #[test]
    fn test_is_std_submodule() {
        let registry = ModuleRegistry::with_std();

        assert!(registry.is_std_submodule("io"));
        assert!(registry.is_std_submodule("math"));
        assert!(registry.is_std_submodule("net"));
        assert!(registry.is_std_submodule("concurrent"));
        assert!(!registry.is_std_submodule("user_module"));
    }
}
