//! 符号表与 DefId —— 绑定身份的唯一所有者（RFC-029 #243/#244 后续重构）
//!
//! 此前模块限定名 `{module}.{name}` 由多处各自 `format!` 拼接、又被多处各自解析：
//! orchestrator 的 `make_export` / `qualify_module_ir`、ir_gen 的 `extract_namespace_path`、
//! 解释器的 `build_vtable` 前缀扫描与 `call_static_by_name` 的 `_constructor` 回退。
//! 五个解析器共用一套隐式字符串语法，靠约定耦合，任一处改动都在**运行时**炸成
//! "function not found"，而非编译期报错。
//!
//! 本模块把「绑定身份」收敛到一处，用**两层代表**共同承载：
//!
//! - **路径**（[`DefPath`]）：与本次编译无关的事实——模块键 + 可选归属段 + 裸名。
//!   限定名是它的**投影**，[`DefPath::project`] 是全仓库唯一的限定名拼接实现；
//! - **序号**（[`DefId`]）：本次编译按**首次出现序**分配的句柄。IR 规范化
//!   （`middle::ir::normalize`）按首现序给 DefId 重新编号，故序号只在一次编译内有意义。
//!
//! **路径是事实、序号是本次编译的句柄**：路径不同即序号不同（`a.Point` / `b.Point`
//! 天然共存），同一路径在幂等登记下恒得同一序号。符号表以 [`DefPath`] 为权威键存储，
//! 投影字符串只是它的像（见 [`SymbolTable`] 的字段说明）。
//!
//! 方法通过 [`DefKind::Method`] + parent 链接到所属类型，[`SymbolTable::vtable`]
//! 在 intern 期增量构建**静态 vtable**——编译期定死，替代运行时前缀扫描。
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

/// 顶层绑定的唯一身份（本次编译内的序号）。
///
/// 由 [`SymbolTable`] 分配，进程内唯一。身份来自分配序号而非字符串，故同名绑定
/// 只要 intern 的路径不同即拥有不同 DefId；同一路径重复 intern 复用同一 DefId。
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

/// 绑定身份的**路径**：限定名的唯一构造者，符号表的权威存储形式。
///
/// 路径由三段组成，拼成限定名时**只有** [`DefPath::project`] 这一个实现：
///
/// | 形态 | `module` | `owner` | `name` | 投影 |
/// | --- | --- | --- | --- | --- |
/// | 顶层绑定 | `lib` | `None` | `helper` | `lib.helper` |
/// | 方法 | `lib` | `Some("Point")` | `get_x` | `lib.Point.get_x` |
/// | 生成期扁平名 | 空串 | `None` | `__T.m_l0` | `__T.m_l0` |
///
/// 路径是事实、序号（[`DefId`]）是本次编译的句柄——详见模块文档。
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct DefPath {
    /// 注册表模块键**原样**（`std.io` 这类整串不拆开，也不当成两级模块）。
    ///
    /// 空串 = **无模块归属**，两种名字落在这一类：
    /// 1. 本单元生成期合成的扁平名（curry 内层 `__T.m_l0`）——没有注册表模块键可归属；
    /// 2. native 导出里**不是**「模块键 + 裸名」形状的 FFI 名（如 `int.to_string` 注册在
    ///    `std.convert` 下，见 `ModuleRegistry::intern_module`）——挂到某个模块键下会
    ///    改变其投影。
    ///
    /// 这类路径的投影即名字本身，与历史 `intern` 按整串登记逐字相同。
    pub module: String,
    /// 归属段：方法所属类型的**裸名**（普通绑定为 `None`）。
    ///
    /// 只在**定义侧** `Type.method` 键上有值——类型名是标识符、不含点，
    /// 故该键的首段可以无歧义地当作归属段（见 [`DefPath::method`]）。
    pub owner: Option<String>,
    /// 裸绑定名（不含模块键与归属段；生成期扁平名整串在此）。
    pub name: String,
    /// 同名消歧序号（**本轮恒为 0**，[`DefPath::project`] 不输出后缀）。
    ///
    /// 预留给方法面接入（L1）与类型身份（L3）：同名重载候选需要各自的身份槽位时，
    /// 序号记进本字段、由 [`DefPath::project`] 追加后缀——届时拼接规则仍只改一处。
    pub disambiguator: u32,
}

impl DefPath {
    /// 顶层绑定的路径：`{module}.{name}`。
    ///
    /// `module` 传空串即「无模块归属的生成期扁平名」，投影为 `name` 本身
    ///（见 [`DefPath::module`] 的说明）。
    pub fn item(
        module: &str,
        name: &str,
    ) -> Self {
        Self {
            module: module.to_string(),
            owner: None,
            name: name.to_string(),
            disambiguator: 0,
        }
    }

    /// 方法的路径：`{module}.{type_bare}.{method}`。
    ///
    /// `type_bare` 是方法所属类型的裸名（不含模块键）。方法名限定沿用既有约定
    /// （`Point.get_x` 得到 `lib.Point.get_x`），与 codegen `CreateStruct.type_name`
    /// 及解释器函数表的扁平名天然契合。
    pub fn method(
        module: &str,
        type_bare: &str,
        method: &str,
    ) -> Self {
        Self {
            module: module.to_string(),
            owner: Some(type_bare.to_string()),
            name: method.to_string(),
            disambiguator: 0,
        }
    }

    /// **唯一**的限定名投影实现：按 模块键 → 归属段 → 裸名 用点号连接。
    ///
    /// 全仓库由「模块键 + 裸名（+ 归属段）」拼出的**绑定身份限定名**都经由此函数：
    /// `{module}.{bare}` 与 `{module}.{owner}.{bare}` 两种形态只在这里产生（L3 侧
    /// 按名拼装的存量点已一并迁移）。
    ///
    /// 明确排除的两类**不是**这里说的限定名投影：
    /// 1. **定义侧 `Type.method` 查表键**（`method_bindings`、方法重载候选等表的
    ///    键）：由各自表的所有者拼接，不参与 DefId 登记，也不得当作路径使用；
    /// 2. 在**已投影名字**上追加尾段/混编后缀的派生名（方法派发名、单态化混编名）：
    ///    它们消费的是投影产物，不重新构造「模块键 + 裸名」。
    ///
    /// `disambiguator` 本轮恒为 0 且**不参与投影**——L1/L3 需要同名多身份时在这里
    /// 追加后缀，调用点无需改动。
    pub fn project(&self) -> String {
        let mut out = String::with_capacity(self.module.len() + self.name.len() + 8);
        if !self.module.is_empty() {
            out.push_str(&self.module);
            out.push('.');
        }
        if let Some(owner) = &self.owner {
            out.push_str(owner);
            out.push('.');
        }
        out.push_str(&self.name);
        out
    }
}

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
