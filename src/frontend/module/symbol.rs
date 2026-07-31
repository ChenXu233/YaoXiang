//! 符号表与 DefId —— 模块限定名的唯一所有者（RFC-029 #243/#244 后续重构）
//!
//! 此前模块限定名 `{module}.{name}` 由多处各自 `format!` 拼接：orchestrator 的
//! `make_export` / `qualify_module_ir`、ir_gen 的 `extract_namespace_path`。格式靠约定
//! 耦合，任一处改动都在**运行时**炸成 "function not found"，而非编译期报错。
//!
//! 本模块把「绑定身份」与「限定名生成」收敛到一处：
//! - 每个顶层绑定分配唯一 [`DefId`]，身份由分配序号保证，与名字无关——两个同名类型
//!   （`a.Point` / `b.Point`）拿到不同 DefId，天然共存；
//! - 限定名是 DefId 的**投影**（intern 时登记的字符串），由 [`SymbolTable::qualify`]
//!   这一处统一生成，全仓库不再各自 `format!`；
//! - 名字解析（本地名/命名空间名 → 限定名）统一走本表。
//!
//! ponytail: 后端（字节码/解释器）仍按限定名分发——那是有意保留的安全缝隙。把 DefId
//! 推进到字节码（函数索引、静态 vtable、删除运行时前缀扫描）是独立后续工作，不在本次。

use std::collections::HashMap;

/// 顶层绑定的唯一身份。
///
/// 由 [`SymbolTable`] 分配，进程内唯一。身份来自分配序号而非字符串，故同名绑定
/// 只要 intern 的限定名不同即拥有不同 DefId。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct DefId(pub u32);

/// 符号表：DefId ↔ 限定名的双向映射，限定名的唯一生产者。
#[derive(Debug, Default, Clone)]
pub struct SymbolTable {
    /// DefId → 限定名（DefId 即槽位下标）
    names: Vec<String>,
    /// 限定名 → DefId（反查）
    ids: HashMap<String, DefId>,
}

impl SymbolTable {
    pub fn new() -> Self {
        Self::default()
    }

    /// 限定名的**唯一**生成规则：`{module}.{bare}`。
    ///
    /// 全仓库的限定名拼接都应经由此函数，不再各自 `format!("{}.{}", ...)`。
    pub fn qualify(
        module: &str,
        bare: &str,
    ) -> String {
        format!("{}.{}", module, bare)
    }

    /// 登记（或复用）一个限定名，返回其 DefId。幂等：同名复用同一 DefId。
    pub fn intern(
        &mut self,
        qualified: &str,
    ) -> DefId {
        if let Some(&id) = self.ids.get(qualified) {
            return id;
        }
        let id = DefId(self.names.len() as u32);
        self.names.push(qualified.to_string());
        self.ids.insert(qualified.to_string(), id);
        id
    }

    /// 登记 `{module}.{bare}` 并返回 DefId（等价 `intern(&qualify(module, bare))`）。
    pub fn intern_qualified(
        &mut self,
        module: &str,
        bare: &str,
    ) -> DefId {
        let q = Self::qualify(module, bare);
        self.intern(&q)
    }

    /// DefId → 限定名。
    pub fn name(
        &self,
        id: DefId,
    ) -> &str {
        &self.names[id.0 as usize]
    }

    /// 限定名 → DefId。
    pub fn def(
        &self,
        qualified: &str,
    ) -> Option<DefId> {
        self.ids.get(qualified).copied()
    }

    /// 是否已登记该限定名。
    pub fn contains(
        &self,
        qualified: &str,
    ) -> bool {
        self.ids.contains_key(qualified)
    }

    pub fn len(&self) -> usize {
        self.names.len()
    }

    pub fn is_empty(&self) -> bool {
        self.names.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn qualify_is_single_source_of_format() {
        assert_eq!(SymbolTable::qualify("lib", "Point"), "lib.Point");
        assert_eq!(
            SymbolTable::qualify("math.geometry", "foo"),
            "math.geometry.foo"
        );
    }

    #[test]
    fn intern_is_idempotent_and_unique() {
        let mut t = SymbolTable::new();
        let a = t.intern_qualified("a", "Point");
        let b = t.intern_qualified("b", "Point");
        let a2 = t.intern_qualified("a", "Point");
        // 同名类型不同模块 → 不同 DefId（#244 共存的基础）
        assert_ne!(a, b);
        // 幂等：重复 intern 复用
        assert_eq!(a, a2);
        assert_eq!(t.name(a), "a.Point");
        assert_eq!(t.name(b), "b.Point");
        assert_eq!(t.def("a.Point"), Some(a));
        assert_eq!(t.len(), 2);
    }
}
