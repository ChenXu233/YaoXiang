//! 符号表：路径 ↔ 序号/类别/归属的双向映射，限定名的唯一生产者（RFC-029 #243/#244）
//!
//! 身份类型（DefPath / DefId / DefKind）在 frontend::core::types::identity（L3-0 下沉）；
//! 本模块只保留 SymbolTable 及其直接实现。权威存储是 DefPath（paths，下标即 DefId），
//! 反查表 ids 也以路径为键；names / by_projection 是投影缓存与投影索引——两表都只由
//! SymbolTable::alloc 这一处写入、都由 DefPath::project 派生，不是第二权威。
//!
//! 方法通过 DefKind::Method + parent 链接到所属类型，SymbolTable::vtable 在 intern 期
//! 增量构建静态 vtable——编译期定死，替代运行时前缀扫描。

use std::collections::HashMap;

use crate::frontend::core::types::identity::{DefId, DefKind, DefPath};
/// 符号表：路径 ↔ 序号/类别/归属的双向映射，限定名的唯一生产者。
///
/// 权威存储是 [`DefPath`]（`paths`，下标即 DefId），反查表 `ids` 也以路径为键；
/// `names` / `by_projection` 是**投影缓存与投影索引**——两表都只由
/// [`SymbolTable::alloc`] 这一处写入、都由 [`DefPath::project`] 派生，**不是第二权威**。
#[derive(Debug, Default, Clone)]
pub struct SymbolTable {
    /// 权威存储：DefId（下标）→ 路径
    paths: Vec<DefPath>,
    /// 权威反查：路径 → DefId
    ids: HashMap<DefPath, DefId>,
    /// 投影缓存（与 paths 等长；唯一写入点 `alloc`，由 [`DefPath::project`] 生成）
    names: Vec<String>,
    /// **投影索引**：投影字符串 → DefId。
    ///
    /// 不是第二权威：只由 `alloc` 在登记时写入（值一律是 [`DefPath::project`] 的
    /// 产物），读取只经 [`SymbolTable::def_by_projection`]，且**从不反解**字符串
    ///（任何把限定名拆回模块/归属/裸名的动作都不在这里发生）。
    ///
    /// 不变量：**一条投影只对应一条路径**（路径是身份，投影是它的像）。两条路径若
    /// 投影相同，投影索引会以后登记者为准——构造点必须保证「模块/归属/裸名」三段与
    /// 投影一一对应，不得借同一个投影登记两种身份。
    by_projection: HashMap<String, DefId>,
    /// DefId → 类别（与 paths 等长）
    kinds: Vec<DefKind>,
    /// DefId → 所属类型（仅 Method 为 Some；与 paths 等长）
    parents: Vec<Option<DefId>>,
    /// 静态 vtable：类型 DefId → 有序方法列表（方法名, 方法 DefId）。
    /// intern 方法时增量维护——编译期定死，替代运行时 `build_vtable` 前缀扫描。
    vtables: HashMap<DefId, Vec<(String, DefId)>>,
}

impl SymbolTable {
    pub fn new() -> Self {
        Self::default()
    }

    /// 登记（或复用）一个路径，返回其 DefId。幂等：同路径复用同一 DefId。
    pub fn intern(
        &mut self,
        path: DefPath,
        kind: DefKind,
    ) -> DefId {
        self.alloc(path, kind, None)
    }

    /// 登记方法（`path` 必须带归属段），链接到所属类型并维护静态 vtable。幂等。
    ///
    /// 所属类型（`{module}.{owner}` 的同路径项）不存在时先以 [`DefKind::Type`]
    /// 登记，故方法登记可独立于类型登记顺序。归属段缺失的路径不是方法路径：
    /// 按普通绑定登记（无 parent、无 vtable 条目）——[`DefPath::method`] 的构造点
    /// 保证不发生。
    pub fn intern_method(
        &mut self,
        path: DefPath,
    ) -> DefId {
        let Some(owner) = path.owner.clone() else {
            return self.alloc(path, DefKind::Method, None);
        };
        let type_def = self.alloc(DefPath::item(&path.module, &owner), DefKind::Type, None);
        // 幂等：已存在则直接复用，不重复登记 vtable 条目。
        if let Some(&id) = self.ids.get(&path) {
            return id;
        }
        let method_name = path.name.clone();
        let method_def = self.alloc(path, DefKind::Method, Some(type_def));
        self.vtables
            .entry(type_def)
            .or_default()
            .push((method_name, method_def));
        method_def
    }

    /// 内部：登记一个路径（锁步维护所有平行表），返回 DefId。幂等。
    ///
    /// **唯一写入点**：`paths` / `ids` 两个权威表与 `names` / `by_projection`
    /// 两个投影表都在这里同步写入——投影只在这个位置由 [`DefPath::project`] 生成一次。
    fn alloc(
        &mut self,
        path: DefPath,
        kind: DefKind,
        parent: Option<DefId>,
    ) -> DefId {
        if let Some(&id) = self.ids.get(&path) {
            return id;
        }
        let projection = path.project();
        let id = DefId(self.paths.len() as u32);
        self.paths.push(path.clone());
        self.names.push(projection.clone());
        self.kinds.push(kind);
        self.parents.push(parent);
        self.ids.insert(path, id);
        self.by_projection.insert(projection, id);
        id
    }

    /// DefId → 路径（权威存储的读取口；L2 内部与 L3 只读消费）。
    pub fn path(
        &self,
        id: DefId,
    ) -> &DefPath {
        &self.paths[id.0 as usize]
    }

    /// DefId → 限定名（路径的**投影缓存**，与 `path(id).project()` 同值）。
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

    /// 路径 → DefId（权威反查）。
    pub fn def_path(
        &self,
        path: &DefPath,
    ) -> Option<DefId> {
        self.ids.get(path).copied()
    }

    /// **投影索引查询**：本项目自己投影过的限定名 → DefId。
    ///
    /// 只接受**本次编译的投影产物**（[`DefPath::project`] 的输出，如 IR 里的调用名、
    /// 注册表导出的 `full_path`）：它不解析字符串、也不替代 [`Self::def_path`]。
    /// 凡持有 `module` / `owner` / `name` 三段的地方一律走 [`Self::def_path`]。
    pub fn def_by_projection(
        &self,
        projection: &str,
    ) -> Option<DefId> {
        self.by_projection.get(projection).copied()
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
        self.paths.len()
    }

    pub fn is_empty(&self) -> bool {
        self.paths.is_empty()
    }
}
