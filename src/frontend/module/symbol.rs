//! 符号表与 DefId —— 绑定身份的唯一所有者（RFC-029 #243/#244 后续重构）
//!
//! 此前模块限定名 `{module}.{name}` 由多处各自 `format!` 拼接、又被多处各自解析：
//! orchestrator 的 `make_export` / `qualify_module_ir`、ir_gen 的 `extract_namespace_path`、
//! 解释器的 `build_vtable` 前缀扫描与 `call_static_by_name` 的 `_constructor` 回退。
//! 五个解析器共用一套隐式字符串语法，靠约定耦合，任一处改动都在**运行时**炸成
//! "function not found"，而非编译期报错。
//!
//! 本模块把「绑定身份」收敛到一处：
//! - 每个顶层绑定（函数/类型/方法/常量）分配唯一 [`DefId`]，身份由分配序号保证，
//!   与名字无关——两个同名类型（`a.Point` / `b.Point`）拿到不同 DefId，天然共存；
//! - 限定名是 DefId 的**投影**（intern 时登记的字符串），由 [`SymbolTable::qualify`]
//!   这一处统一生成，全仓库不再各自 `format!`；
//! - 方法通过 [`DefKind::Method`] + parent 链接到所属类型，[`SymbolTable::vtable`]
//!   在 intern 期增量构建**静态 vtable**——编译期定死，替代运行时前缀扫描。
//!
//! # 阶段定位
//!
//! 这是「把名字解析从运行时五处收敛为编译期一处」重构的地基（阶段 0）。本模块只提供
//! 身份基础设施；「名字 → DefId」的语义解析归属 **typecheck**（而非 parser——parser 只管
//! 抽象语法），由后续阶段把 typecheck 已有的名字解析改为产出 DefId，IR gen 直接消费。
//!
//! ponytail: 后端（字节码/解释器）暂时仍按限定名分发——那是有意保留的安全缝隙。把 DefId
//! 推进到字节码（函数索引、删除运行时 `build_vtable` 扫描）是阶段 3/4，不在本次。

use std::collections::HashMap;

/// 顶层绑定的唯一身份。
///
/// 由 [`SymbolTable`] 分配，进程内唯一。身份来自分配序号而非字符串，故同名绑定
/// 只要 intern 的限定名不同即拥有不同 DefId。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct DefId(pub u32);

/// 绑定类别。
///
/// 方法在底层函数表里仍是函数（解释器以 `lib.Point.get_x` 这样的扁平名存储），
/// [`DefKind::Method`] 仅用于把它与所属类型关联、构建静态 vtable。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DefKind {
    /// 普通顶层函数
    Function,
    /// 类型定义（结构体/记录）
    Type,
    /// 绑定到类型的方法（parent 指向所属类型的 DefId）
    Method,
    /// 常量
    Constant,
}

/// 符号表：DefId ↔ 限定名/类别/归属的双向映射，限定名的唯一生产者。
///
/// 各 `Vec` 与 `names` 等长、按 DefId 下标对齐（`intern` 时锁步 push）。
#[derive(Debug, Default, Clone)]
pub struct SymbolTable {
    /// DefId → 限定名（DefId 即槽位下标）
    names: Vec<String>,
    /// 限定名 → DefId（反查）
    ids: HashMap<String, DefId>,
    /// DefId → 类别（与 names 等长）
    kinds: Vec<DefKind>,
    /// DefId → 所属类型（仅 Method 为 Some；与 names 等长）
    parents: Vec<Option<DefId>>,
    /// 静态 vtable：类型 DefId → 有序方法列表（方法名, 方法 DefId）。
    /// intern 方法时增量维护——编译期定死，替代运行时 `build_vtable` 前缀扫描。
    vtables: HashMap<DefId, Vec<(String, DefId)>>,
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

    /// 内部：登记一个限定名（锁步维护所有平行 Vec），返回 DefId。幂等。
    fn alloc(
        &mut self,
        qualified: &str,
        kind: DefKind,
        parent: Option<DefId>,
    ) -> DefId {
        if let Some(&id) = self.ids.get(qualified) {
            return id;
        }
        let id = DefId(self.names.len() as u32);
        self.names.push(qualified.to_string());
        self.kinds.push(kind);
        self.parents.push(parent);
        self.ids.insert(qualified.to_string(), id);
        id
    }

    /// 登记（或复用）一个限定名，返回其 DefId。幂等：同名复用同一 DefId。
    ///
    /// 默认类别为 [`DefKind::Function`]、无 parent——兼容既有调用点。需要类别/归属信息
    /// 时用 [`intern_kind`](Self::intern_kind) / [`intern_method`](Self::intern_method)。
    pub fn intern(
        &mut self,
        qualified: &str,
    ) -> DefId {
        self.alloc(qualified, DefKind::Function, None)
    }

    /// 登记（或复用）一个**已限定**名并指定类别，无 parent。幂等。
    ///
    /// 与 [`intern_kind`](Self::intern_kind) 不同：此函数收的是完整限定名（如 `std.io.println`），
    /// 不再经 `qualify` 拼接。供从注册表 `export.full_path` 批量登记使用。
    pub fn intern_full(
        &mut self,
        qualified: &str,
        kind: DefKind,
    ) -> DefId {
        self.alloc(qualified, kind, None)
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

    /// 登记带类别的 `{module}.{bare}`（函数/类型/常量），无 parent。幂等。
    pub fn intern_kind(
        &mut self,
        module: &str,
        bare: &str,
        kind: DefKind,
    ) -> DefId {
        let q = Self::qualify(module, bare);
        self.alloc(&q, kind, None)
    }

    /// 登记方法 `{module}.{type_bare}.{method}`，链接到所属类型并维护静态 vtable。幂等。
    ///
    /// 所属类型 `{module}.{type_bare}` 不存在时先以 [`DefKind::Type`] intern。
    /// 方法名限定沿用既有约定（`Point.get_x` → `lib.Point.get_x`），与 codegen
    /// `CreateStruct.type_name` 及解释器函数表的扁平名天然契合。
    pub fn intern_method(
        &mut self,
        module: &str,
        type_bare: &str,
        method: &str,
    ) -> DefId {
        let type_def = self.intern_kind(module, type_bare, DefKind::Type);
        let method_qualified = Self::qualify(module, &format!("{}.{}", type_bare, method));
        // 幂等：已存在则直接复用，不重复登记 vtable 条目。
        if let Some(&id) = self.ids.get(&method_qualified) {
            return id;
        }
        let method_def = self.alloc(&method_qualified, DefKind::Method, Some(type_def));
        self.vtables
            .entry(type_def)
            .or_default()
            .push((method.to_string(), method_def));
        method_def
    }

    /// DefId → 限定名。
    pub fn name(
        &self,
        id: DefId,
    ) -> &str {
        &self.names[id.0 as usize]
    }

    /// DefId → 类别。
    pub fn kind(
        &self,
        id: DefId,
    ) -> DefKind {
        self.kinds[id.0 as usize]
    }

    /// DefId → 所属类型（仅方法为 Some）。
    pub fn parent(
        &self,
        id: DefId,
    ) -> Option<DefId> {
        self.parents[id.0 as usize]
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

    /// 类型的静态 vtable：有序方法列表（方法名, 方法 DefId）。
    ///
    /// 顺序即 intern 顺序（源码定义顺序）。类型无方法或非类型 DefId 返回空切片。
    pub fn vtable(
        &self,
        type_def: DefId,
    ) -> &[(String, DefId)] {
        self.vtables
            .get(&type_def)
            .map(|v| v.as_slice())
            .unwrap_or(&[])
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

    #[test]
    fn intern_records_kind() {
        let mut t = SymbolTable::new();
        let f = t.intern_kind("m", "helper", DefKind::Function);
        let ty = t.intern_kind("m", "Point", DefKind::Type);
        let c = t.intern_kind("m", "PI", DefKind::Constant);
        assert_eq!(t.kind(f), DefKind::Function);
        assert_eq!(t.kind(ty), DefKind::Type);
        assert_eq!(t.kind(c), DefKind::Constant);
        // 普通绑定无 parent
        assert_eq!(t.parent(f), None);
        assert_eq!(t.parent(ty), None);
    }

    #[test]
    fn intern_method_links_parent_and_builds_vtable() {
        let mut t = SymbolTable::new();
        let get_x = t.intern_method("lib", "Point", "get_x");
        let get_y = t.intern_method("lib", "Point", "get_y");

        // 方法限定名沿用既有约定
        assert_eq!(t.name(get_x), "lib.Point.get_x");
        assert_eq!(t.name(get_y), "lib.Point.get_y");
        assert_eq!(t.kind(get_x), DefKind::Method);

        // parent 指向自动 intern 的类型
        let point = t.def("lib.Point").expect("type auto-interned");
        assert_eq!(t.kind(point), DefKind::Type);
        assert_eq!(t.parent(get_x), Some(point));
        assert_eq!(t.parent(get_y), Some(point));

        // 静态 vtable 按 intern 顺序排列
        let vt = t.vtable(point);
        assert_eq!(vt.len(), 2);
        assert_eq!(vt[0], ("get_x".to_string(), get_x));
        assert_eq!(vt[1], ("get_y".to_string(), get_y));
    }

    #[test]
    fn intern_method_is_idempotent() {
        let mut t = SymbolTable::new();
        let a = t.intern_method("lib", "Point", "get_x");
        let b = t.intern_method("lib", "Point", "get_x");
        assert_eq!(a, b);
        // vtable 不重复登记
        let point = t.def("lib.Point").unwrap();
        assert_eq!(t.vtable(point).len(), 1);
    }

    #[test]
    fn same_type_name_coexist_with_independent_vtables() {
        // #244：a.Point 与 b.Point 是不同类型的不同方法，vtable 各自独立。
        let mut t = SymbolTable::new();
        let a_get = t.intern_method("a", "Point", "get");
        let b_get = t.intern_method("b", "Point", "get");
        assert_ne!(a_get, b_get);

        let a_point = t.def("a.Point").unwrap();
        let b_point = t.def("b.Point").unwrap();
        assert_ne!(a_point, b_point);
        assert_eq!(t.vtable(a_point), &[("get".to_string(), a_get)]);
        assert_eq!(t.vtable(b_point), &[("get".to_string(), b_get)]);
    }

    #[test]
    fn vtable_empty_for_typeless_or_function_def() {
        let mut t = SymbolTable::new();
        let f = t.intern_kind("m", "helper", DefKind::Function);
        assert!(t.vtable(f).is_empty());
    }
}
