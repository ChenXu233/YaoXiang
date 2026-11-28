//! 绑定身份类型与类型层受限视图（L3-0）
//!
//! 本模块是「身份」的中性叶：路径是事实、序号是本次编译的句柄。
//!
//! - [DefPath]：模块键 + 可选归属段 + 裸名（+ 消歧序号）的结构化身份；
//!   [DefPath::project] 是全仓库唯一的限定名投影实现，投影只出不进。
//! - [DefId]：本次编译按首现序分配的句柄，只在一次编译内有意义。
//! - [DefKind]：绑定类别（函数/类型/方法/常量）。
//!
//! 类型层只持它的 kind 受限视图：[TypePath] 拒绝 owner / 消歧序号 / 含点裸名，
//! 因而只能表示类型身份；[Nominal] 是它的新类型包装。视图不新增字段、不新增投影
//! ——[TypePath::project] 直接委托 [DefPath::project]。
//!
//! 显式不提供：Deref/DerefMut（不暴露可变路径）、Default（空身份无意义）、
//! Serialize/Deserialize（身份只能由解析产出，不能从任意字节流入）。
//! 只给 as_def_path(&self) -> &DefPath 只读访问器。

use std::fmt;
use std::hash::{Hash, Hasher};

/// 顶层绑定的唯一身份（本次编译内的序号）。
///
/// 由符号表分配，进程内唯一。身份来自分配序号而非字符串，故同名绑定只要 intern 的
/// 路径不同即拥有不同 DefId；同一路径重复 intern 复用同一 DefId。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct DefId(pub u32);

/// 绑定类别。
///
/// 方法在底层函数表里仍是函数（解释器以 lib.Point.get_x 这样的扁平名存储），
/// [DefKind::Method] 仅用于把它与所属类型关联、构建静态 vtable。
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

/// 绑定身份的路径：限定名的唯一构造者，符号表的权威存储形式。
///
/// 路径由三段组成，拼成限定名时只有 [DefPath::project] 这一个实现：
///
/// | 形态 | module | owner | name | 投影 |
/// | --- | --- | --- | --- | --- |
/// | 顶层绑定 | lib | None | helper | lib.helper |
/// | 方法 | lib | Some(Point) | get_x | lib.Point.get_x |
/// | 生成期扁平名 | 空串 | None | __T.m_l0 | __T.m_l0 |
///
/// 路径是事实、序号（[DefId]）是本次编译的句柄。
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct DefPath {
    /// 注册表模块键原样（std.io 这类整串不拆开，也不当成两级模块）。
    ///
    /// 空串 = 无模块归属：生成期扁平名与 native 的非「模块键 + 裸名」形状名。
    pub module: String,
    /// 归属段：方法所属类型的裸名（普通绑定为 None）。
    ///
    /// 只在定义侧 Type.method 键上有值——类型名是标识符、不含点。
    pub owner: Option<String>,
    /// 裸绑定名（不含模块键与归属段；生成期扁平名整串在此）。
    pub name: String,
    /// 同名消歧序号（本轮恒为 0，[DefPath::project] 不输出后缀）。
    pub disambiguator: u32,
}

impl DefPath {
    /// 顶层绑定的路径：{module}.{name}。module 传空串即无模块归属的平路径。
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

    /// 方法的路径：{module}.{type_bare}.{method}。
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

    /// 唯一的限定名投影实现：按 模块键 → 归属段 → 裸名 用点号连接。
    ///
    /// 全仓库由「模块键 + 裸名（+ 归属段）」拼出的绑定身份限定名都经由此函数；
    /// 在已投影名字上追加尾段/混编后缀的派生名不在此列（它们消费投影产物）。
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

/// 类型名的合法形态：非空、不含点（不含点才能与限定身份区分）。
fn is_valid_type_bare(name: &str) -> bool {
    !name.is_empty() && !name.contains('.')
}

/// 类型身份的受限视图：只接受「无归属段、无消歧序号、裸名不含点」的路径。
#[derive(Clone)]
pub struct TypePath {
    inner: DefPath,
}

impl TypePath {
    /// 匿名/合成形态的空名（无名义头的 Struct/Enum 用）：不是身份，仅保持今天的空名打印。
    pub fn anonymous() -> Self {
        Self {
            inner: DefPath::item("", ""),
        }
    }

    /// 无模块归属的类型名（内建/类型参数/合成名）。
    pub fn flat(name: &str) -> Option<Self> {
        Self::item("", name)
    }

    /// 有模块归属的类型名（module 可含点，如 std.io）。
    pub fn item(
        module: &str,
        name: &str,
    ) -> Option<Self> {
        if !is_valid_type_bare(name) {
            return None;
        }
        Some(Self {
            inner: DefPath::item(module, name),
        })
    }

    /// 从既有路径构造视图；方法路径（owner 非空）、带消歧序号、含点裸名一律拒绝。
    pub fn from_def_path(path: &DefPath) -> Option<Self> {
        if path.owner.is_none() && path.disambiguator == 0 && is_valid_type_bare(&path.name) {
            Some(Self {
                inner: path.clone(),
            })
        } else {
            None
        }
    }

    /// 限定名投影——直接委托 [DefPath::project]，不另写拼接。
    pub fn project(&self) -> String {
        self.inner.project()
    }

    /// 显示投影：裸名。
    pub fn bare(&self) -> &str {
        &self.inner.name
    }

    /// 模块键（无模块归属为空串）。
    pub fn module(&self) -> &str {
        &self.inner.module
    }

    /// 只读访问底层路径（不提供可变访问器）。
    pub fn as_def_path(&self) -> &DefPath {
        &self.inner
    }
}

impl fmt::Debug for TypePath {
    fn fmt(
        &self,
        f: &mut fmt::Formatter<'_>,
    ) -> fmt::Result {
        // 与今天 String 的 Debug 逐字相同（带引号裸名）；限定身份也只打印裸名。
        write!(f, "{:?}", self.bare())
    }
}

impl fmt::Display for TypePath {
    fn fmt(
        &self,
        f: &mut fmt::Formatter<'_>,
    ) -> fmt::Result {
        f.write_str(self.bare())
    }
}

impl PartialEq for TypePath {
    fn eq(
        &self,
        other: &Self,
    ) -> bool {
        self.module() == other.module() && self.bare() == other.bare()
    }
}

impl PartialEq<str> for TypePath {
    fn eq(
        &self,
        other: &str,
    ) -> bool {
        self.bare() == other
    }
}

impl PartialEq<&str> for TypePath {
    fn eq(
        &self,
        other: &&str,
    ) -> bool {
        self.bare() == *other
    }
}

impl Eq for TypePath {}

impl Hash for TypePath {
    fn hash<H: Hasher>(
        &self,
        state: &mut H,
    ) {
        // R-H1：flat 只写 bare 的 String 哈希原序（与今天 String::hash 一致，保 pin）；
        // 非 flat 先 module 后 bare（固定写入序，域分离靠顺序而非拼接串）。
        if self.module().is_empty() {
            self.bare().hash(state);
        } else {
            self.module().hash(state);
            self.bare().hash(state);
        }
    }
}

/// 名义类型身份：[TypePath] 的新类型包装（L3-1 的 MonoType 名义面用它）。
#[derive(Clone)]
pub struct Nominal(TypePath);

impl Nominal {
    /// 匿名/合成形态的空名（无名义头的 Struct/Enum 用）：不是身份，仅保持今天的空名打印。
    pub fn anonymous() -> Self {
        Self(TypePath::anonymous())
    }

    /// 无模块归属的名义类型（内建/类型参数/合成名）。
    pub fn flat(name: &str) -> Option<Self> {
        TypePath::flat(name).map(Self)
    }

    /// 有模块归属的名义类型。
    pub fn item(
        module: &str,
        name: &str,
    ) -> Option<Self> {
        TypePath::item(module, name).map(Self)
    }

    /// 从路径构造；方法与带消歧序号的路径一律拒绝。
    pub fn from_def_path(path: &DefPath) -> Option<Self> {
        TypePath::from_def_path(path).map(Self)
    }

    /// 限定名投影（委托 DefPath::project）。
    pub fn project(&self) -> String {
        self.0.project()
    }

    /// 显示投影：裸名。
    pub fn bare(&self) -> &str {
        self.0.bare()
    }

    /// 模块键（无模块归属为空串）。
    pub fn module(&self) -> &str {
        self.0.module()
    }

    /// 只读访问底层类型路径。
    pub fn as_type_path(&self) -> &TypePath {
        &self.0
    }
}

impl fmt::Debug for Nominal {
    fn fmt(
        &self,
        f: &mut fmt::Formatter<'_>,
    ) -> fmt::Result {
        write!(f, "{:?}", self.bare())
    }
}

impl fmt::Display for Nominal {
    fn fmt(
        &self,
        f: &mut fmt::Formatter<'_>,
    ) -> fmt::Result {
        f.write_str(self.bare())
    }
}

impl PartialEq for Nominal {
    fn eq(
        &self,
        other: &Self,
    ) -> bool {
        self.0 == other.0
    }
}

impl PartialEq<str> for Nominal {
    fn eq(
        &self,
        other: &str,
    ) -> bool {
        self.bare() == other
    }
}

impl PartialEq<&str> for Nominal {
    fn eq(
        &self,
        other: &&str,
    ) -> bool {
        self.bare() == *other
    }
}

impl Eq for Nominal {}

impl Hash for Nominal {
    fn hash<H: Hasher>(
        &self,
        state: &mut H,
    ) {
        self.0.hash(state);
    }
}

impl TypePath {
    /// 裸名（与 bare 同义，供既有 as_str 调用点使用）。
    pub fn as_str(&self) -> &str {
        self.bare()
    }
}

impl AsRef<str> for TypePath {
    fn as_ref(&self) -> &str {
        self.bare()
    }
}

impl std::borrow::Borrow<str> for TypePath {
    fn borrow(&self) -> &str {
        self.bare()
    }
}

impl Nominal {
    /// 裸名（与 bare 同义，供既有 as_str 调用点使用）。
    pub fn as_str(&self) -> &str {
        self.bare()
    }
}

impl std::borrow::Borrow<str> for Nominal {
    fn borrow(&self) -> &str {
        self.bare()
    }
}

impl PartialEq<String> for TypePath {
    fn eq(
        &self,
        other: &String,
    ) -> bool {
        self.bare() == other.as_str()
    }
}

impl PartialEq<String> for Nominal {
    fn eq(
        &self,
        other: &String,
    ) -> bool {
        self.bare() == other.as_str()
    }
}

impl PartialEq<Nominal> for String {
    fn eq(
        &self,
        other: &Nominal,
    ) -> bool {
        self.as_str() == other.bare()
    }
}

impl PartialEq<&Nominal> for String {
    fn eq(
        &self,
        other: &&Nominal,
    ) -> bool {
        self.as_str() == other.bare()
    }
}

impl PartialEq<Nominal> for str {
    fn eq(
        &self,
        other: &Nominal,
    ) -> bool {
        self == other.bare()
    }
}
