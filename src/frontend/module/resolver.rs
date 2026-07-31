//! 名字解析器 —— 「名字 → 限定名 → DefId」的唯一所有者（RFC-029 重构核心）
//!
//! 此前名字解析散落多处、各按隐式字符串语法各自为政：ir_gen 的 `is_namespace_call` /
//! `extract_namespace_path`、orchestrator 的导入别名构建。本解析器把这些语义收敛到一处，
//! 且**不依赖 AST**——只收字符串（命名空间头 + 字段段列表），AST 的扁平化（FieldAccess 链
//! → 段列表）留给消费方。这样解析逻辑可独立单测，并可被 **typecheck**（语义解析的归属地，
//! 而非只管语法的 parser）与 IR 生成共用。
//!
//! 解析规则（与既有约定一致，保证行为不变）：
//! - `std`            → `std.{field...}`
//! - std 子模块 `io`   → `std.io.{field...}`
//! - 用户命名空间别名  → 其模块限定键 + `{field...}`（`use math.geometry` 后 `geometry.foo` → `math.geometry.foo`）
//! - 其余裸名         → 视为模块/类型前缀直接限定
//!
//! 限定名生成统一经 [`SymbolTable::qualify`]；DefId 查 [`SymbolTable::def`]。

use std::collections::HashMap;

use super::registry::ModuleRegistry;
use super::symbol::{DefId, SymbolTable};

/// 名字解析器：在给定文件上下文（模块键 + 用户命名空间）下把名字解析为限定名 / DefId。
///
/// 全部字段借用，按解析上下文（通常一个文件）临时构造。
pub struct Resolver<'a> {
    /// 项目级符号表（限定名 → DefId）
    symbols: &'a SymbolTable,
    /// 模块注册表（用于判定 std 子模块）
    registry: &'a ModuleRegistry,
    /// 当前文件的模块限定键
    module_key: &'a str,
    /// 用户模块命名空间：别名 → 模块限定键（来自整体导入 `use lib` / `use lib as l`）
    user_namespaces: &'a HashMap<String, String>,
}

impl<'a> Resolver<'a> {
    pub fn new(
        symbols: &'a SymbolTable,
        registry: &'a ModuleRegistry,
        module_key: &'a str,
        user_namespaces: &'a HashMap<String, String>,
    ) -> Self {
        Self {
            symbols,
            registry,
            module_key,
            user_namespaces,
        }
    }

    /// `name` 是否是命名空间头（`std`、std 子模块、或用户模块别名）。
    pub fn is_namespace(
        &self,
        name: &str,
    ) -> bool {
        name == "std"
            || self.registry.is_std_submodule(name)
            || self.user_namespaces.contains_key(name)
    }

    /// 把命名空间路径（头 + 字段段列表）解析为限定名。
    ///
    /// 例：`resolve_namespace("io", &["println"])` → `std.io.println`；
    /// `resolve_namespace("geometry", &["foo"])`（geometry → math.geometry）→ `math.geometry.foo`。
    pub fn resolve_namespace(
        &self,
        head: &str,
        fields: &[&str],
    ) -> String {
        let mut base = if head == "std" {
            "std".to_string()
        } else if self.registry.is_std_submodule(head) {
            SymbolTable::qualify("std", head)
        } else if let Some(mk) = self.user_namespaces.get(head) {
            mk.clone()
        } else {
            head.to_string()
        };
        for f in fields {
            base = SymbolTable::qualify(&base, f);
        }
        base
    }

    /// 把命名空间路径解析为 DefId（先限定，再查符号表）。
    pub fn resolve_namespace_def(
        &self,
        head: &str,
        fields: &[&str],
    ) -> Option<DefId> {
        self.symbols.def(&self.resolve_namespace(head, fields))
    }

    /// 解析本文件顶层绑定（函数/类型/常量）的 DefId。
    pub fn resolve_local(
        &self,
        bare: &str,
    ) -> Option<DefId> {
        self.symbols
            .def(&SymbolTable::qualify(self.module_key, bare))
    }

    /// 解析本文件类型上的方法 DefId（经静态 vtable 按名查槽）。
    pub fn resolve_method(
        &self,
        type_bare: &str,
        method: &str,
    ) -> Option<DefId> {
        let type_def = self.resolve_local(type_bare)?;
        self.symbols
            .vtable(type_def)
            .iter()
            .find(|(n, _)| n == method)
            .map(|(_, d)| *d)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::frontend::module::symbol::DefKind;

    fn ctx() -> (SymbolTable, ModuleRegistry, HashMap<String, String>) {
        let mut symbols = SymbolTable::new();
        // 本文件 lib 的顶层绑定与方法
        symbols.intern_kind("lib", "helper", DefKind::Function);
        symbols.intern_method("lib", "Point", "get_x");
        // std 导出
        symbols.intern_full("std.io.println", DefKind::Function);
        symbols.intern_full("std.math.geometry.foo", DefKind::Function);
        // 另一用户模块的导出（供跨文件命名空间解析）
        symbols.intern_full("other.helper", DefKind::Function);

        let registry = ModuleRegistry::with_std();

        let mut user_namespaces = HashMap::new();
        user_namespaces.insert("lib".to_string(), "lib".to_string());
        user_namespaces.insert("geometry".to_string(), "std.math.geometry".to_string());
        user_namespaces.insert("other".to_string(), "other".to_string());

        (symbols, registry, user_namespaces)
    }

    #[test]
    fn namespace_detection() {
        let (symbols, registry, un) = ctx();
        let r = Resolver::new(&symbols, &registry, "lib", &un);
        assert!(r.is_namespace("std"));
        assert!(r.is_namespace("io")); // std 子模块
        assert!(r.is_namespace("lib")); // 用户模块别名
        assert!(r.is_namespace("geometry"));
        assert!(!r.is_namespace("helper")); // 普通函数名
        assert!(!r.is_namespace("Point"));
    }

    #[test]
    fn resolve_std_paths() {
        let (symbols, registry, un) = ctx();
        let r = Resolver::new(&symbols, &registry, "lib", &un);
        assert_eq!(
            r.resolve_namespace("std", &["io", "println"]),
            "std.io.println"
        );
        assert_eq!(r.resolve_namespace("io", &["println"]), "std.io.println");
        assert_eq!(
            r.resolve_namespace("geometry", &["foo"]),
            "std.math.geometry.foo"
        );
    }

    #[test]
    fn resolve_namespace_to_def() {
        let (symbols, registry, un) = ctx();
        let r = Resolver::new(&symbols, &registry, "lib", &un);
        let d = r
            .resolve_namespace_def("io", &["println"])
            .expect("interned");
        assert_eq!(symbols.name(d), "std.io.println");
        // 跨文件用户模块命名空间
        let d2 = r
            .resolve_namespace_def("other", &["helper"])
            .expect("interned");
        assert_eq!(symbols.name(d2), "other.helper");
        // 未登记 → None（解析失败可在编译期报错，而非运行时 function not found）
        assert!(r.resolve_namespace_def("io", &["nope"]).is_none());
    }

    #[test]
    fn resolve_local_and_method() {
        let (symbols, registry, un) = ctx();
        let r = Resolver::new(&symbols, &registry, "lib", &un);
        let helper = r.resolve_local("helper").expect("local fn");
        assert_eq!(symbols.name(helper), "lib.helper");
        let get_x = r.resolve_method("Point", "get_x").expect("method");
        assert_eq!(symbols.name(get_x), "lib.Point.get_x");
        assert_eq!(symbols.kind(get_x), DefKind::Method);
        // 不存在的方法
        assert!(r.resolve_method("Point", "get_y").is_none());
        assert!(r.resolve_local("missing").is_none());
    }

    #[test]
    fn bare_prefix_falls_back_to_direct_qualify() {
        let (symbols, registry, un) = ctx();
        let r = Resolver::new(&symbols, &registry, "lib", &un);
        // 非命名空间头视为模块/类型前缀直接限定（与 extract_namespace_path else 分支一致）
        assert_eq!(r.resolve_namespace("Point", &["get_x"]), "Point.get_x");
    }
}
