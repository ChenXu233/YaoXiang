//! Abstract Syntax Tree types

use std::collections::HashSet;

pub use crate::frontend::core::lexer::tokens::Literal;
pub use crate::util::plan_id::PlanId;
use crate::util::span::Span;

use crate::frontend::core::parser::probe::{NameKind, TypeEnvProbe};

#[derive(Debug, Clone)]
pub struct SpannedIdent {
    pub name: String,
    pub span: Span,
}

/// Expression
#[derive(Debug, Clone)]
pub enum Expr {
    Lit(Literal, Span),
    Var(String, Span),
    BinOp {
        op: BinOp,
        left: Box<Expr>,
        right: Box<Expr>,
        span: Span,
    },
    UnOp {
        op: UnOp,
        expr: Box<Expr>,
        span: Span,
    },
    Call {
        func: Box<Expr>,
        args: Vec<Expr>,
        /// RFC-010: 命名参数 `Point(x=1, y=2)`
        named_args: Vec<(String, Expr)>,
        span: Span,
        /// 调用点身份（RFC-039 D20 / WBS 4.5.2）：解析期由 `ParserState`
        /// 分配、随本调用节点走的 `PlanId`——重载决议（`overload_resolutions`）
        /// 的产出与消费读同一个节点上的同一个值，不再靠「Call 的 span 两处
        /// 各自算一遍」的位置巧合。
        ///
        /// 构造一律走 [`Expr::call`]（全仓唯一构造器）；解析器之外的合成调用
        /// 记 [`PlanId::UNPLANNED`]，消费侧不消费未分配身份（见
        /// `OverloadResolutions::get`）。
        plan_id: PlanId,
    },
    If {
        condition: Box<Expr>,
        then_branch: Box<Block>,
        else_if_branches: Vec<(Box<Expr>, Box<Block>)>,
        else_branch: Option<Box<Block>>,
        span: Span,
    },
    Match {
        expr: Box<Expr>,
        arms: Vec<MatchArm>,
        span: Span,
    },
    While {
        condition: Box<Expr>,
        body: Box<Block>,
        span: Span,
    },
    For {
        var: String,
        var_mut: bool, // 变量是否可变
        iterable: Box<Expr>,
        body: Box<Block>,
        span: Span,
    },
    /// spawn for 数据并行循环（RFC-024 §2.4）
    SpawnFor {
        var: String,
        var_mut: bool,
        iterable: Box<Expr>,
        body: Box<Block>,
        span: Span,
    },
    Block(Block),
    Return(Option<Box<Expr>>, Span),
    /// break 语句（#314：Python 风格定案——裸 break，无标签，仅语句位置）
    Break(Span),
    /// continue 语句（#314：裸 continue，无标签）
    Continue(Span),
    Cast {
        expr: Box<Expr>,
        target_type: Type,
        span: Span,
    },
    Tuple(Vec<Expr>, Span),
    List(Vec<Expr>, Span),
    ListComp {
        element: Box<Expr>,                 // 元素表达式 x * x
        generators: Vec<ListCompGenerator>, // 生成器子句，至少一个（parser 保证）
        span: Span,
    },
    Dict(Vec<(Expr, Expr)>, Span),
    /// membership 谓词（#299 §3）：`elem in container` → Bool
    /// 一等霍尔谓词：精化类型阶段作为编译期可证命题的基底
    In {
        elem: Box<Expr>,
        container: Box<Expr>,
        span: Span,
    },
    Index {
        expr: Box<Expr>,
        index: Box<Expr>,
        span: Span,
    },
    FieldAccess {
        expr: Box<Expr>,
        field: String,
        span: Span,
    },
    /// Error propagation operator: `expr?`
    /// Generated when the user writes `expr?` - propagates Err/None upward
    Try {
        expr: Box<Expr>,
        span: Span,
    },
    /// ref 关键字：创建 Arc（原子引用计数）
    /// `shared = ref p` 创建 p 的 Arc 副本
    Ref {
        expr: Box<Expr>,
        span: Span,
    },
    /// Borrow expression: `&expr` or `&mut expr`
    /// Creates a borrow token
    Borrow {
        mutable: bool,
        expr: Box<Expr>,
        span: Span,
    },
    /// unsafe 块：允许系统级操作
    /// `unsafe { *ptr = ... }`
    Unsafe {
        body: Box<Block>,
        span: Span,
    },
    /// Spawn a concurrent block: `spawn { ... }`
    ///
    /// RFC-024: The only parallel primitive in YaoXiang.
    Spawn {
        body: Box<Block>,
        span: Span,
    },
    /// Lambda expression: (params) => body
    /// Used for RFC-007 function syntax: name = (params) => body
    Lambda {
        params: Vec<Param>,
        body: Box<Block>,
        span: Span,
    },
    /// RFC-012: F-string template literal
    /// `f"Hello {name}"` → FString { segments: [Text("Hello "), Interpolation { expr, format_spec }] }
    FString {
        segments: Vec<FStringSegment>,
        span: Span,
    },
    /// 错误恢复占位符：表示解析失败的表达式
    ///
    /// 当解析器遇到无法解析的表达式时，插入此占位符而非 panic。
    /// 类型检查器遇到此节点时应报告错误但不 panic。
    /// 用于 LSP 错误恢复场景。
    Error(Span),
}

/// 列表推导式的生成器子句：`for <var> in <iterable> ('if' <condition>)?`
///
/// #401：单个推导式可携带多个子句（`[e for x in a for y in b if c]`），
/// 后续子句的 iterable/condition 可引用先前子句绑定的变量。
#[derive(Debug, Clone)]
pub struct ListCompGenerator {
    pub var: String,                  // 迭代变量名 x
    pub iterable: Box<Expr>,          // 可迭代对象
    pub condition: Option<Box<Expr>>, // 过滤条件 if x > 0
    pub span: Span,
}

/// RFC-012: F-string segment
#[derive(Debug, Clone)]
pub enum FStringSegment {
    /// Plain text fragment
    Text(String),
    /// Interpolation expression with optional format specifier
    Interpolation {
        expr: Box<Expr>,
        format_spec: Option<String>,
    },
}

/// Binary operators
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BinOp {
    Add,
    Sub,
    Mul,
    Div,
    Mod,
    Eq,
    Neq,
    Lt,
    Le,
    Gt,
    Ge,
    And,
    Or,
    Range,
    Assign,
    // #285: 位运算/移位（SPEC §2.2 级 7/8）
    BitAnd,
    BitOr,
    BitXor,
    Shl,
    Shr,
}

/// Unary operators
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UnOp {
    Neg,
    Pos,
    Not,
    /// Dereference: `*ptr`
    Deref,
}

/// Statement
#[derive(Debug, Clone)]
pub struct Stmt {
    pub kind: StmtKind,
    pub span: Span,
    /// 释放计划身份（RFC-039 D20 / WBS 4.5.1）：解析期由 `ParserState`
    /// **单调分配**、编译单元内唯一，随语句节点走。
    ///
    /// 它把 `ReleasePlan` 的键从「span 位置巧合」换成「AST 节点结构身份」：
    /// ownership 把释放点归属到释放边界所在的语句，ir_gen 在语句边界按同一
    /// 身份查表发射 `Instruction::Drop`。不经解析器的合成语句恒为
    /// [`PlanId::UNPLANNED`]（不构成释放计划义务）。
    pub plan_id: PlanId,
}

/// Statement kind
#[derive(Debug, Clone)]
pub enum StmtKind {
    Expr(Box<Expr>),
    /// 赋值语句：target = 值 或 target: 类型 = 值
    /// target 是表达式（Var("x")、FieldAccess(Var("File"), "open")、...）
    /// parser 不区分 target 能否当左值——由 L3 的类型检查判断
    /// type_annotation 是可选的类型标注
    /// value 是可选的初始化表达式（extern 绑定 / 方法绑定有，纯声明无）
    Assign {
        target: Box<Expr>,
        /// 类型标注；函数定义时它承载 `Type::Fn`——形参名与类型同栖在
        /// `Type::Fn.params`（`Type::fn_params_flat` 是唯一取用入口）。
        type_annotation: Option<Type>,
        value: Option<Box<Expr>>,
        is_mut: bool,
        span: Span,
    },
    /// For loop: `for [mut] item in iterable { body }`
    For {
        var: String,
        /// 变量名的源码位置（用于代码染色等）
        var_span: Span,
        var_mut: bool, // 变量是否可变
        iterable: Box<Expr>,
        body: Box<Block>,
    },
    /// 类型定义：`Point: Type = { x: Float, y: Float }`
    /// 或泛型：`Option: (T: Type) -> Type = { some(T) | none }`
    TypeDefinition {
        /// 类型名
        name: String,
        /// 声明签名：`(T: Type) -> Type`（`Point: Type` 记 `MetaType`）。
        /// 泛型形参从 `Type::Fn` 载体取（`Type::fn_params_flat`），与 `Assign`
        /// 的 `type_annotation` 同一表示——不再另存并行形参表。
        type_annotation: Option<Type>,
        /// 解析后的类型体（必有）
        definition: Type,
    },
    /// Use statement: `use module.path` / `use module.{a, b}` /
    /// `use module as m` / `use module.{a as x}`（#245）
    Use {
        path: String,
        /// Module path span
        path_span: Span,
        /// Spans of each identifier in the module path (dot-separated)
        path_parts: Vec<SpannedIdent>,
        /// 花括号导入项（带名源位置，供 LSP 导入项跳转定位绑定）
        items: Option<Vec<SpannedIdent>>,
        /// 位置别名 `use path as m`（带名源位置，同上）
        alias: Option<Vec<SpannedIdent>>,
        /// 花括号内联别名（#245）：与 `items` 逐项对齐，`None` 表示该项无别名。
        /// `use lib.{helper as h}` → items=["helper"], item_aliases=[Some("h")]。
        /// 全项均无别名时为 `None`。
        item_aliases: Option<Vec<Option<SpannedIdent>>>,
    },
    /// If statement: `if condition { then_branch } elif branches else_branch`
    If {
        condition: Box<Expr>,
        then_branch: Box<Block>,
        else_if_branches: Vec<(Box<Expr>, Box<Block>)>,
        else_branch: Option<Box<Block>>,
        span: Span,
    },
    /// 元组解构赋值: `a, b = (1, 2)` 或 `(a, b) = (1, 2)`
    DestructureAssign {
        names: Vec<SpannedIdent>,
        rhs: Box<Expr>,
        span: Span,
    },
    /// Return statement: `return expr` or `return`
    Return(Option<Box<Expr>>),
    /// 错误恢复占位符：表示解析失败的语句
    ///
    /// 当解析器遇到无法解析的语句时，插入此占位符而非 panic。
    /// 类型检查器遇到此节点时应报告错误但不 panic。
    /// 用于 LSP 错误恢复场景。
    Error(Span),
}

/// 结构体字段定义
///
/// 用于表示类型定义中的字段，包含可变性标记和可选默认值
#[derive(Debug, Clone)]
pub struct StructField {
    pub name: String,
    pub is_mut: bool,
    pub ty: Type,
    /// 可选的默认值表达式
    pub default: Option<Box<Expr>>,
}

impl StructField {
    /// 创建新的结构体字段（无默认值）
    pub fn new(
        name: String,
        is_mut: bool,
        ty: Type,
    ) -> Self {
        Self {
            name,
            is_mut,
            ty,
            default: None,
        }
    }

    /// 创建带默认值的结构体字段
    pub fn with_default(
        name: String,
        is_mut: bool,
        ty: Type,
        default: Expr,
    ) -> Self {
        Self {
            name,
            is_mut,
            ty,
            default: Some(Box::new(default)),
        }
    }
}

/// 类型体项 — 类型定义 `{}` 是有序代码块，每项按序求值
///
/// parser 只按语法区分，不认识任何类型族名字。
#[derive(Debug, Clone)]
pub enum TypeBodyItem {
    /// `name: Type` 或 `name: Type = default` — 运行时数据字段
    Field(StructField),
    /// `name = function[pos]` 或 `name = function` 或匿名函数绑定 — 保留现有 TypeBodyBinding 语义
    Binding(TypeBodyBinding),
    /// 接口约束名 `InterfaceName`
    Interface(String),
    /// 匿名类型表达式 `Assert(N > 0)` / `SomeProofType(t)` — 结果未命名
    Expr(Type),
}

/// 类型体内置绑定
///
/// 在类型定义体内绑定方法到字段
#[derive(Debug, Clone)]
pub struct TypeBodyBinding {
    pub name: String,
    pub kind: BindingKind,
}

/// 绑定方式
#[derive(Debug, Clone)]
pub enum BindingKind {
    /// 外部函数绑定: `name = function[positions]`
    External {
        function: String,
        positions: Vec<i64>,
    },
    /// 匿名函数绑定: `name: ((params) -> Return)[position] = ((params) => body)`
    Anonymous {
        params: Vec<Param>,
        return_type: Box<Type>,
        positions: Vec<i64>,
        body: Box<Expr>,
    },
    /// RFC-004: 默认绑定（无位置）: `name = function`
    /// 自动查找函数参数中第一个类型匹配的位置
    DefaultExternal { function: String },
}

/// Generic parameter kind: Type parameter or Const parameter
#[derive(Debug, Clone)]
pub enum GenericParamKind {
    /// Type parameter: `T`
    Type,
    /// Const parameter: [N: Int]
    Const {
        /// The type of the const parameter (e.g., Int)
        const_type: Box<Type>,
    },
}

/// Generic parameter with constraints: `[T: Clone]` or `[N: Int]`
#[derive(Debug, Clone)]
pub struct GenericParam {
    pub name: String,
    pub kind: GenericParamKind,
    pub constraints: Vec<Type>,
}
/// Generic parameter with name and constraints only (no Type/Const classification).
/// Used by consumers that only need structural info, not kind dispatch.
#[derive(Debug, Clone)]
pub struct GenericParamName {
    pub name: String,
    pub constraints: Vec<Type>,
}

/// Type
#[derive(Debug, Clone)]
pub enum Type {
    Name {
        name: String,
        span: Span,
    },
    Struct {
        /// 有序类型体项（字段/绑定/接口/表达式按源码顺序）
        body: Vec<TypeBodyItem>,
    },
    NamedStruct {
        name: String,
        name_span: Span,
        fields: Vec<StructField>,
    },
    Tuple(Vec<Type>),
    /// 函数类型：`(a: Int, b: Int) -> Ret`。
    ///
    /// `params` 复用 [`Param`]——名字与类型同栖（RFC-039 6.8.1）：
    /// 签名位置（`parse_fn_type_with_names`）与 lambda 头一样带真实名字；
    /// 纯类型位置/类型反解没有名字可写时用 [`Param::anonymous`] 合成
    /// （`name` 空串，`is_named()` 为 false）。**不得**再引入并行的
    /// 名字数组（如 `param_names: Vec<Option<String>>`）。
    Fn {
        params: Vec<Param>,
        return_type: Box<Type>,
    },
    Generic {
        name: String,
        name_span: Span,
        args: Vec<Type>,
    },
    /// Literal type: a compile-time constant value used as a type
    /// e.g., "5" in `[n: Int](n: n)` - n is a literal type "5"
    /// Used for const generics with literal value parameters
    Literal {
        /// The literal name (e.g., "5")
        name: String,
        name_span: Span,
        /// The underlying type (e.g., Int)
        base_type: Box<Type>,
    },
    /// Raw pointer type: `*T`
    /// Only usable inside unsafe blocks
    Ptr(Box<Type>),
    /// Reference type: `&T` (immutable) or `&mut T` (mutable)
    /// Borrow token — zero-sized compile-time type
    Ref {
        mutable: bool,
        inner: Box<Type>,
        span: Span,
    },
    /// Meta-type: `Type` or `Type[T]` or `Type[K, V]`
    /// RFC-010: Used in unified syntax `Name: Type = { ... }`
    /// `Type` is the only meta-type keyword in the language
    /// Supports infinite universe levels: `Type[Type[T]]` → Type2, etc.
    MetaType {
        /// `Type` 关键字的源码位置
        name_span: Span,
        /// Generic type parameters (empty for plain `Type`)
        /// e.g., `Type[T]` has args = `T`, `Type[K, V]` has args = `K, V`
        /// e.g., `Type[Type[T]]` has args = `MetaType { args: T }`
        args: Vec<Type>,
    },
    /// 编译期表达式（泛型参数位置的值表达式，如 Assert(N > 0) 中的 N > 0）
    ConstExpr(Box<Expr>),
    /// 括号类型：`(T)` —— **完整类型，链条终止**（RFC-004 柯里化语义）。
    ///
    /// 仅在「括号未构成函数参数组」时产生，即括号后不跟 `->`。因此：
    ///
    /// - `(a: Int) -> Int` 参数组：内层 `(...)` 后跟 `->` → **不**产生 `Paren`
    /// - `() -> ((a: Int) -> Int)` 返回位置裸括号 → **产生** `Paren`
    ///
    /// `split_curry` 遇 `Paren` 即停止拆层：括号声明“这是一个完整类型”，
    /// 它是函数的**值**，不是下一个参数组。
    ///
    /// 其余环节（类型检查 / 单态化 / 解释器）视 `Paren` 为透明，递归内层即可。
    Paren(Box<Type>),
    /// **具名括号**：`(r: P(...))` —— 只有一个具名参数、且括号后不跟 `->`。
    ///
    /// RFC-027 §3 的返回位精化靠它声明**返回形式参数名**（这里叫 binder）：
    ///
    /// - `f: (b: Int) -> (r: IsPositive(r + 1))` 的 `r` 就是 binder——
    ///   它「仅存在于类型签名中、仅被谓词引用，**不进入函数体作用域**」，
    ///   由 `return` 的值供给。故解析层必须留下这个名字：丢掉它，类型检查器
    ///   只能猜「约束里不在作用域的自由变量即返回值形参」，于是
    ///   `(r: P(m))` 会把未声明的 `m` 也当成 binder 静默代入（实测缺陷）。
    /// - 语义与 `Paren` 同构：`MonoType` 层透明（只递归内层）、
    ///   `split_curry` 到此终止（括号声明“这是一个完整类型”），只多携带 binder 名。
    ///
    /// 参数位的形参名归 `Type::Fn.params`（名字与类型同栖，RFC-039 6.8.1）；
    /// 本节点只承载**返回位**单具名括号的 binder——那里没有 `->` 后的参数组，
    /// 名字无处可栖，只能由语法节点携带、由类型检查器显式解释。
    NamedParen {
        /// 声明的具名（返回形式参数名）
        param: String,
        /// 名字的源码位置
        param_span: Span,
        /// 内层类型（谓词应用）
        inner: Box<Type>,
    },
}

impl Type {
    /// RFC-004 括号语义：`self` 是否为「括号声明的完整类型」（`Paren` / `NamedParen`）。
    ///
    /// 括号 = 柯里链条终止 = 该类型是函数的**值**，不是下一层参数组。
    /// 两处共用同一判定，防止判据漂移：
    /// - parser 的 paren_return 包装（`parser/statements/declarations.rs`）；
    /// - typecheck 的值级分解（`sema/inference/statements.rs`，issue #501）。
    pub fn is_paren_complete(&self) -> bool {
        matches!(self, Type::Paren(_) | Type::NamedParen { .. })
    }

    /// 从 Type::Struct 的 body 中提取字段引用（顺序保留）
    pub fn struct_fields(&self) -> Vec<&StructField> {
        match self {
            Type::Struct { body } => body
                .iter()
                .filter_map(|it| {
                    if let TypeBodyItem::Field(f) = it {
                        Some(f)
                    } else {
                        None
                    }
                })
                .collect(),
            _ => Vec::new(),
        }
    }

    /// 提取接口约束名引用
    pub fn struct_interfaces(&self) -> Vec<&String> {
        match self {
            Type::Struct { body } => body
                .iter()
                .filter_map(|it| {
                    if let TypeBodyItem::Interface(s) = it {
                        Some(s)
                    } else {
                        None
                    }
                })
                .collect(),
            _ => Vec::new(),
        }
    }

    /// 提取绑定引用
    pub fn struct_bindings(&self) -> Vec<&TypeBodyBinding> {
        match self {
            Type::Struct { body } => body
                .iter()
                .filter_map(|it| {
                    if let TypeBodyItem::Binding(b) = it {
                        Some(b)
                    } else {
                        None
                    }
                })
                .collect(),
            _ => Vec::new(),
        }
    }

    /// 收集类型树中出现的名字引用（类型名/绑定函数名/编译期表达式的标识符）。
    ///
    /// 供死代码分析（可达性）与未使用导入检测共享：
    /// 类型引用只出现在注解/类型体位置，普通表达式遍历看不到它们。
    /// 保守策略——拿不准的名字一律收集（宁可漏报也不误报死代码/未用导入）。
    pub fn collect_name_refs(
        &self,
        out: &mut HashSet<String>,
    ) {
        match self {
            Type::Name { name, .. } => {
                out.insert(name.clone());
            }
            Type::Struct { body } => {
                for item in body {
                    collect_type_body_item_refs(item, out);
                }
            }
            Type::NamedStruct { fields, .. } => {
                for f in fields {
                    f.ty.collect_name_refs(out);
                    if let Some(default) = &f.default {
                        collect_expr_ident_refs(default, out);
                    }
                }
            }
            Type::Tuple(items) => {
                for ty in items {
                    ty.collect_name_refs(out);
                }
            }
            Type::Fn {
                params,
                return_type,
            } => {
                for p in params {
                    if let Some(ty) = &p.ty {
                        ty.collect_name_refs(out);
                    }
                }
                return_type.collect_name_refs(out);
            }
            Type::Ptr(inner) => inner.collect_name_refs(out),
            Type::Generic { name, args, .. } => {
                out.insert(name.clone());
                for ty in args {
                    ty.collect_name_refs(out);
                }
            }
            Type::Literal { base_type, .. } => base_type.collect_name_refs(out),
            Type::Ref { inner, .. } => inner.collect_name_refs(out),
            Type::MetaType { args, .. } => {
                for ty in args {
                    ty.collect_name_refs(out);
                }
            }
            Type::ConstExpr(expr) => collect_expr_ident_refs(expr, out),
            _ => {}
        }
    }
}

/// 收集类型体项（字段/绑定/接口/匿名类型表达式）中的名字引用
fn collect_type_body_item_refs(
    item: &TypeBodyItem,
    out: &mut HashSet<String>,
) {
    match item {
        TypeBodyItem::Field(f) => {
            f.ty.collect_name_refs(out);
            if let Some(default) = &f.default {
                collect_expr_ident_refs(default, out);
            }
        }
        TypeBodyItem::Binding(b) => collect_binding_kind_refs(&b.kind, out),
        TypeBodyItem::Expr(ty) => ty.collect_name_refs(out),
        TypeBodyItem::Interface(_) => {}
    }
}

/// 收集类型体绑定（外部/匿名/默认绑定）中的名字引用
fn collect_binding_kind_refs(
    kind: &BindingKind,
    out: &mut HashSet<String>,
) {
    match kind {
        BindingKind::External { function, .. } | BindingKind::DefaultExternal { function } => {
            out.insert(function.clone());
        }
        BindingKind::Anonymous {
            params,
            return_type,
            body,
            ..
        } => {
            for p in params {
                if let Some(ty) = &p.ty {
                    ty.collect_name_refs(out);
                }
            }
            return_type.collect_name_refs(out);
            collect_expr_ident_refs(body, out);
        }
    }
}

/// 收集表达式中出现的标识符（仅 Var 叶子，不深入求值语义）。
///
/// 用于 ConstExpr 等表达式位置的名字保守收集；
/// 完整的引用分析在 L3 的类型检查与死代码分析各自的遍历中完成。
fn collect_expr_ident_refs(
    expr: &Expr,
    out: &mut HashSet<String>,
) {
    match expr {
        Expr::Var(name, _) => {
            out.insert(name.clone());
        }
        Expr::Call { func, args, .. } => {
            collect_expr_ident_refs(func, out);
            for arg in args {
                collect_expr_ident_refs(arg, out);
            }
        }
        Expr::FieldAccess { expr, .. } => collect_expr_ident_refs(expr, out),
        Expr::BinOp { left, right, .. } => {
            collect_expr_ident_refs(left, out);
            collect_expr_ident_refs(right, out);
        }
        Expr::UnOp { expr, .. } => collect_expr_ident_refs(expr, out),
        Expr::Cast { expr, .. } => collect_expr_ident_refs(expr, out),
        Expr::Index { expr, index, .. } => {
            collect_expr_ident_refs(expr, out);
            collect_expr_ident_refs(index, out);
        }
        Expr::Tuple(items, _) | Expr::List(items, _) => {
            for item in items {
                collect_expr_ident_refs(item, out);
            }
        }
        Expr::Dict(fields, _) => {
            for (key, value) in fields {
                collect_expr_ident_refs(key, out);
                collect_expr_ident_refs(value, out);
            }
        }
        _ => {}
    }
}

/// Block
///
/// All blocks use `return` for return values. No trailing expression semantics.
#[derive(Debug, Clone)]
pub struct Block {
    pub stmts: Vec<Stmt>,
    pub span: Span,
}

/// Function parameter（形参：名字与类型同栖）
///
/// 三类使用点共用同一个结构（RFC-039 6.8.1）：lambda 头、函数签名的
/// `Type::Fn.params`、以及需要形参名/类型对的语义消费点。**唯一**的形参
/// 表示——不得新增 Parameter/Arg 第二结构，也不得用
/// `param_names: Vec<Option<String>>` 一类平行数组过渡。
///
/// # 名字槽的三种形态（事实描述，不是承诺）
///
/// 1. **有名形参**：签名/lambda 头里写出了名字（`a: Int`）——`name` 是真实
///    形参标识符；
/// 2. **无名形参（anonymous）**：纯类型位 / 类型反解合成——`name` 为空串，
///    由 [`Param::anonymous`] / [`Param::from_type`] 构造；
/// 3. **参数位裸标识符**：`(Int64)` / `(a)` 这类没有 `:` 的写法——`name` 是
///    那个标识符、`ty = None`。它**可能是类型名而不是形参**（`(Int64)`），
///    parser 不区分（类型名/参数名的歧义正是 6.4 探针注入要解决的问题）。
///    该形态**不占运行期形参位**（基线口径，见
///    [`Param::occupies_runtime_param_slot`]）；它究竟算不算参数由
///    WBS 8.9.1（参数位形态归一，D53）裁决，6.5 只做零行为差分的表示重塑。
///
/// 空串名字不参与任何按名查找（泛型分类入口显式跳过无名形参，见
/// `classify_generic_params` / `extract_generic_param_names`）。形态 0（无名）
/// 与形态 3 在 AST 上都可判定；但形态 3 与「无标注的值形参」**同形**，不可区分。
#[derive(Clone)]
pub struct Param {
    pub name: String,
    pub ty: Option<Type>,
    pub is_mut: bool,
    pub span: Span,
}

impl Param {
    /// 合成**无名形参**：只有类型可写的位置（类型位置、类型反解）使用。
    pub fn anonymous(
        ty: Option<Type>,
        span: Span,
    ) -> Self {
        Self {
            name: String::new(),
            ty,
            is_mut: false,
            span,
        }
    }

    /// 由类型合成无名形参（span 取 dummy：合成形的源码位置不存在）。
    pub fn from_type(ty: Type) -> Self {
        Self::anonymous(Some(ty), Span::dummy())
    }

    /// `name` 槽是否**非空**——只回答这一个事实，**不**表示「携带真实形参名」：
    /// 参数位裸标识符（形态 3）也非空，但它是标识符、未必是形参。
    pub fn is_named(&self) -> bool {
        !self.name.is_empty()
    }

    /// 是否占**运行期形参位**（IR 参数槽）：按基线口径 = **带类型标注**。
    /// 参数位裸标识符（形态 3，`ty = None`）不占位——此口径由 WBS 8.9.1
    /// 重新裁定；改它必须同时改 `middle/ir/tests/fn_param_slots.rs` 的
    /// 锁定测试。
    pub fn occupies_runtime_param_slot(&self) -> bool {
        self.ty.is_some()
    }
}

/// `Debug` 手写（不 derive）：**无名形参**按其所承载的 `Type` 打印——与
/// RFC-039 6.8.1 重塑前 `Type::Fn.params: Vec<Type>` 的 Debug 文本逐字一致；
/// 有名形参按完整结构打印（名字是语义，必须可见）。
///
/// **为什么必须手写而不是再生基线**：规范化 IR 快照的 `type_decl` 行打印
/// AST 类型的 `Debug` 文本（`middle/ir/normalize.rs`），并已入库 216 条。
/// 纯表示重塑（6.8.1）不改变 IR 语义，若靠 `UPDATE_SNAPSHOTS=1` 再生基线，
/// 就会把「形参携带名字」这件事伪装成一次 IR 行为面变化，违反 07 §128
/// 「漂移必须能说清来源」——复审时也无法区分「表示搬运」与「真实 IR 变化」。
/// 故把文本一致性收在**打印边界**（此处），而不是改写基线。无名形参的裸 `Type`
/// 文本正是旧 `Vec<Type>` 的文本，故快照零漂移。（6.5 时代另有
/// `Type::without_fn_param_names` 在入 IR 前剥类型位形参名；WBS 6.6.12 把 IR 的
/// 类型声明载体换成 IR 原生收窄形状后，AST 不再入 IR，该函数成为死代码并已删除。）
impl std::fmt::Debug for Param {
    fn fmt(
        &self,
        f: &mut std::fmt::Formatter<'_>,
    ) -> std::fmt::Result {
        match (&self.ty, self.is_named(), self.is_mut) {
            (Some(ty), false, false) => std::fmt::Debug::fmt(ty, f),
            _ => f
                .debug_struct("Param")
                .field("name", &self.name)
                .field("ty", &self.ty)
                .field("is_mut", &self.is_mut)
                .field("span", &self.span)
                .finish(),
        }
    }
}

impl Type {
    /// 声明里**运行期值位**（**拍平**口径：声明序、跨所有柯里层、按调用序）。
    ///
    /// 它回答的问题是「调用时要按序给几个值」——消费点：E1112 的元数对照、
    /// IR 的形参类型与调用侧可用名（`fn_param_slots` 的 `declaration_runtime_param_slots`）。
    ///
    /// **不是**「实现体所绑的那一层」：那是 [`Type::impl_head_runtime_params`]（层内口径，
    /// 供 parser 合成实现头、typecheck 按位取声明来源）。两者不可互换——多层柯里声明上
    /// 拍平取前 N 会把内层实现头的 `x` 填成外层 `A` 的 Int（实测误报 E1011）。
    ///
    /// 过滤：仅「名字层编译期位」（`T: Type`、类型级函数）
    /// 与「全裸名字并集且名字出现在类型位置」的多约束写法（`T: Add + Multiply`）。
    /// 按**用途**判定的编译期位（`N: Int` 被当常量用）需要层与使用域信息，不在此过滤：
    /// 拍平口径把它们当值位（`f: (N: Int) -> (n: N) -> Int` 平铺后是 2 位），
    /// 故元数对照只可能偏宽、不会误报。
    pub fn runtime_value_params(&self) -> Vec<Param> {
        Self::value_slots(&self.fn_params_flat())
    }

    /// 最内层层（值组）的 Fn 节点（停层判据见 [`Type::impl_head_layer`]）。
    fn innermost_fn(ty: &Type) -> Option<&Type> {
        let mut cur = ty;
        loop {
            let Type::Fn { return_type, .. } = cur else {
                return None;
            };
            // 返回位可以是括号包起来的函数类型（`(T) -> ((self: &R(T)) -> Bool)`）：
            // 它表示「返回的是一个函数值」，而**实现头绑的是这个内层 Fn 的形参**
            // （`= (self) => ...`）——与 `curry` 不同，不沿内层继续下钻。
            let mut next: &Type = return_type;
            loop {
                match next {
                    Type::Paren(inner) => next = inner,
                    Type::NamedParen { inner, .. } => next = inner,
                    _ => break,
                }
            }
            if matches!(next, Type::Fn { .. }) {
                cur = next;
            } else {
                return Some(cur);
            }
        }
    }

    /// 实现头（**合成**形参表：块体 / 表达式体没有作者写的头）绑定**哪一组**声明形参。
    ///
    /// 返回该组的原始形参（顺序即声明序、**不做编译期位过滤**——过滤要按用途判定，
    /// 只有消费方判得出：parser 的探针问答 / sema 的 const 用途分析）。要「这一层
    /// 有哪些运行期值位」请直接用 [`Type::impl_head_runtime_params`]（层选择 + 过滤
    /// 的单一入口）；本方法只回答「哪一组」，供 parser 的括号包装分支按组取形参。
    ///
    /// 分组的唯一判据在**括号返回位**：`mk: (a: Int) -> ((b: Int) -> R)` 的链条在
    /// 括号处终止（`Paren` 声明「返回的是一个函数值」），体是**外层**函数本体
    ///（它产出被返回的函数值），故取**首组**；其余（裸 Fn 链柯里 / 普通函数）
    /// 体是最内层值函数。与 typecheck 的值级分层（`check_fn_stmt`：闭包包装或
    /// 括号返回 ⇒ 外层，否则剥层到最内层）是同一条分层规则（WBS 6.8.3 ③ 的口径），
    /// 故 parser 合成出的形参与 typecheck 的逐位对齐按构造同源。
    pub fn impl_head_layer(&self) -> Vec<Param> {
        match self.impl_head_fn() {
            Some(Type::Fn { params, .. }) => params.clone(),
            _ => Vec::new(),
        }
    }

    /// 实现头所绑那一层的 **Fn 节点**（层选择单点；`impl_head_layer` 与编译期位
    /// 判据的使用域都从这里取）。
    fn impl_head_fn(&self) -> Option<&Type> {
        match self {
            Type::Fn {
                params,
                return_type,
            } if return_type.is_paren_complete() => {
                // 首组全是**编译期位**（`(T: Type) -> ((self: &R(T)) -> Bool)`）时本层
                // 不占运行期位，paren 层就是值层——作者写的 lambda 即值级函数本体
                // （与 parser 的包装判定、typecheck 的类型泛型剥离同一条规则）。
                if Self::value_slots(params).is_empty() {
                    Self::paren_inner_fn(return_type)
                } else {
                    Some(self)
                }
            }
            _ => Self::innermost_fn(self),
        }
    }

    /// 括号返回位内层的 Fn 节点（`-> ((a: Int) -> R)`）；内层不是 Fn
    ///（`-> (Int)` 一类）时为空。
    fn paren_inner_fn(return_type: &Type) -> Option<&Type> {
        let mut inner: &Type = return_type;
        loop {
            match inner {
                Type::Paren(t) => inner = t,
                Type::NamedParen { inner: t, .. } => inner = t,
                Type::Fn { .. } => return Some(inner),
                _ => return None,
            }
        }
    }

    /// 实现头所绑那一层的**运行期值位**——这一份切法的**单一实现**
    ///（WBS 6.8.2 / D54 F1 收尾，6.8.3 F1d）。
    ///
    /// 消费点（各自不得再写过滤循环）：
    /// - parser：块体 / 表达式体**合成**实现头的形参表（没有作者写的头时才合成）；
    /// - typecheck：实现头第 i 位的**声明来源**——体内绑定与 E1011 标注对照按位对齐
    ///   （`check_fn_stmt` 的 `declared_value_params`）。
    ///
    /// 四类编译期位在此一次剔除：
    /// 1. **类型位**：`T: Type`、类型级函数 `(F: (T: Type) -> Type)`；
    /// 2. **约束位**：标注是约束名（`T: Add`）——是不是约束由调用方注入
    ///    （parser 传探针、sema 传 trait 表）；谓词应用（`Terminates(b)`）的实参位
    ///    是**值**不是类型，故「被当类型用」的扫描另需调用方注入谓词问答；
    /// 3. **多约束并集且名字出现在组内类型位置**（`T: Clone + Add`）；
    /// 4. **被当编译期常量/类型用**：名字出现在**本层之外**的类型位置
    ///    （`f: (N: Int, s: String) -> Array(Int, N)` 的 `N`，由唯一分类器
    ///    `classify_compile_time_value_params` 判定）。
    ///
    /// **依赖类型的值形参不剔除**：`f: (a: Int, b: Vec(a)) -> Int` 的 `a` 名字虽出现在
    /// `Vec(a)` 里，但那是**本组**形参标注——它的值参与类型，仍是运行期值位。剔掉它
    /// 会让实现头少一位、位次整体前移（块体形态实测因此误报 E1011）。
    pub fn impl_head_runtime_params(
        &self,
        is_constraint: &dyn Fn(&str) -> bool,
        cls: &ParamClassifier<'_>,
    ) -> Vec<Param> {
        let Some(Type::Fn { .. }) = self.impl_head_fn() else {
            return Vec::new();
        };
        let layer = self.impl_head_layer();
        // 用途判定交给唯一分类器（SPEC §7.1）：名字在**任一** Type 子树被引用 ⇒ 编译期
        // 值参数。使用域不再是「本层之外」——那是 F-1 的位置特判，§7.1 明写「遍历全部
        // 子树、不按位置分情况」。
        let const_used = classify_compile_time_value_params(&self.fn_params_flat(), &[self], cls);
        // 组内类型位置出现的名字——多约束并集判据用（`T: Clone + Add`）。
        let mut type_position_names: std::collections::HashSet<String> =
            std::collections::HashSet::new();
        for p in &layer {
            if let Some(ty) = p.ty.as_ref() {
                ty.collect_name_refs(&mut type_position_names);
            }
        }
        layer
            .into_iter()
            .filter(|p| {
                let Some(ty) = p.ty.as_ref() else {
                    return true;
                };
                if Self::is_name_level_type_slot(ty) {
                    return false;
                }
                if matches!(ty, Type::Name { name, .. } if is_constraint(name)) {
                    return false;
                }
                if let Type::Tuple(items) = ty {
                    let all_names =
                        !items.is_empty() && items.iter().all(|t| matches!(t, Type::Name { .. }));
                    if all_names && type_position_names.contains(p.name.as_str()) {
                        return false;
                    }
                }
                !const_used.contains(p.name.as_str())
            })
            .collect()
    }

    /// 标注本身是否**类型位**：`T: Type`、名字恰为 `Type`、类型级函数
    ///（`(F: (T: Type) -> Type)` 的构造器变量）。判据单点——`value_slots` 与
    /// `impl_head_runtime_params` 共用，不得各写一份 `matches!`。
    fn is_name_level_type_slot(ty: &Type) -> bool {
        matches!(ty, Type::MetaType { .. })
            || matches!(ty, Type::Name { name, .. } if name == "Type")
            || matches!(ty, Type::Fn { params, return_type } if crate::ast::is_type_level_fn(params, return_type))
    }

    /// **实现头绑定序列**（拍平口径）：形参 − 类型参数 − 编译期值参数（SPEC §7.1）。
    ///
    /// 与**调用点实参序列**（形参 − 类型参数；编译期值参数仍占实参位）是两个序列，
    /// 不得并成一个。本函数只回答「实现头第 i 位能绑到哪个声明位」，供 E1112 / E1011；
    /// 调用点实参分配不走这里。
    pub fn runtime_value_params_cls(
        &self,
        cls: &ParamClassifier<'_>,
    ) -> Vec<Param> {
        let flat = self.fn_params_flat();
        let compile_time = classify_compile_time_value_params(&flat, &[self], cls);
        Self::value_slots(&flat)
            .into_iter()
            .filter(|p| !compile_time.contains(p.name.as_str()))
            .collect()
    }
    /// **拍平**口径的编译期位过滤（`runtime_value_params` 用；不含按用途判定的
    /// 编译期常量位——那些要层与使用域信息，见 `impl_head_runtime_params`）。
    fn value_slots(params: &[Param]) -> Vec<Param> {
        let mut type_position_names: std::collections::HashSet<String> =
            std::collections::HashSet::new();
        for p in params {
            if let Some(ty) = p.ty.as_ref() {
                ty.collect_name_refs(&mut type_position_names);
            }
        }
        params
            .iter()
            .filter(|p| {
                let Some(ty) = p.ty.as_ref() else {
                    return true;
                };
                if Self::is_name_level_type_slot(ty) {
                    return false;
                }
                if let Type::Tuple(items) = ty {
                    let all_names =
                        !items.is_empty() && items.iter().all(|t| matches!(t, Type::Name { .. }));
                    if all_names && type_position_names.contains(p.name.as_str()) {
                        return false;
                    }
                }
                true
            })
            .cloned()
            .collect()
    }

    /// 从 `Type::Fn` 链**拍平**出全部形参（curry 各组依次展开；`Paren` /
    /// `NamedParen` 视为透明继续内层）。
    ///
    /// 这是「语句声明里的原始签名形参」的**唯一实现**（RFC-039 6.5.2）：
    /// `Assign`/`TypeDefinition` 不再另存形参表，消费点一律经本方法取；非函数
    /// 类型返回空。`NamedParen` 的 binder 名不在此列——它由该节点自身承载、
    /// 由类型检查器显式解释（返回位精化）。
    pub fn fn_params_flat(&self) -> Vec<Param> {
        fn walk(
            ty: &Type,
            out: &mut Vec<Param>,
        ) {
            match ty {
                Type::Fn {
                    params,
                    return_type,
                } => {
                    out.extend(params.iter().cloned());
                    walk(return_type, out);
                }
                Type::Paren(inner) | Type::NamedParen { inner, .. } => walk(inner, out),
                _ => {}
            }
        }
        let mut out = Vec::new();
        walk(self, &mut out);
        out
    }
}

/// Match arm
#[derive(Debug, Clone)]
pub struct MatchArm {
    pub pattern: Pattern,
    pub body: Block,
    pub span: Span,
}

/// Pattern
#[derive(Debug, Clone)]
pub enum Pattern {
    Wildcard,
    Identifier(String),
    Literal(Literal),
    Tuple(Vec<Pattern>),
    Struct {
        name: String,
        /// 字段模式列表：(字段名, 是否可变, 模式)
        fields: Vec<(String, bool, Box<Pattern>)>,
    },
    Union {
        name: String,
        variant: String,
        pattern: Option<Box<Pattern>>,
    },
    Or(Vec<Pattern>),
    Guard {
        pattern: Box<Pattern>,
        condition: Expr,
    },
}

/// Module
#[derive(Debug, Clone)]
pub struct Module {
    pub items: Vec<Stmt>,
    pub span: Span,
}

impl Default for Module {
    fn default() -> Self {
        Self {
            items: Vec::new(),
            span: Span::dummy(),
        }
    }
}

/// Returns true if a type is the `Type` meta keyword form.
pub fn is_meta_type(ty: &Type) -> bool {
    matches!(ty, Type::MetaType { .. })
}

/// 参数名是否在给定类型中被当作**类型引用**（`(N: Int) -> (n: N)` 的 `N`）。
///
/// **全仓唯一实现**（RFC-039 P6 6.4 / 03 §5.3，6.8.8 补齐扫描面）：parser 与 sema
/// 侧的同义遍历已全部收拢到本函数——「形参名是否在类型位置被引用」只有这一处判定。
///
/// 扫描面 = 调用方传入的任意 `Type` 子树（SPEC type-system §7.1：「类型位置」指
/// 声明中的全部 `Type` 子树，不按位置分情况）。判据要点：
///
/// - `Generic` 的**头部**同样是类型引用（`F(A)` 的 `F` 是构造器变量）；
/// - `Generic` 的**谓词应用**短路：头名判定为编译期谓词时实参位是**值**
///   （`Terminates(b)` 的 `b`），整个应用不构成类型引用（RFC-027a）；
/// - `Struct` / `NamedStruct` 的字段类型递归；
/// - `ConstExpr` 递归表达式里的名字引用（`Assert(N > 0)` 的 `N`），遍历复用
///   `collect_expr_ident_refs`——不新写第二份表达式遍历；
/// - `MetaType` 递归 `args`：`Type` 关键字本身不是可判定的候选名，只有实参是；
/// - `Paren` / `NamedParen` 递归 `inner`（RFC-004：括号声明完整类型，内层同义）。
///   `NamedParen` 的 `param` **不计**：它是**返回形式参数名的声明**（binder，
///   `(r: IsPositive(r + 1))` 的 `r`），不是类型引用——把它计入会让声明名冒充引用；
/// - `Type::Struct` 的 `TypeBodyItem::Binding` **不扫**：绑定项是类型体内的
///   **方法绑定**（外部函数引用 / 匿名实现体），其签名属于被绑方法而非构造器的
///   参数化面；把它计入会把「方法签名里提到的名字」等同于「类型参数的用途」，
///   让 E1094（构造器级落空候选）对只在方法签名出现的名字漏报。绑定内的名字
///   引用由 `collect_name_refs`（可达性 / 未用导入）负责，与本判定不同问题。
///
/// `is_predicate` 是调用方注入的环境问答：parser 侧传「探针 ∪ 本遍已声明谓词」，
/// L3 侧传语义探针。两边都不是字符串常量表。
///
/// 本函数只回答「名字是否出现在类型位置」，**不做**命名空间解析——「该名字是否
/// 解析到已声明类型」（SPEC §7.1 第 1、2 条）是调用方注入的上游问答，见
/// `ParamClassifier::is_type_name` 与 `const_param::used_as_const_in_annotation`。
pub fn name_used_as_type(
    name: &str,
    ty: &Type,
    is_predicate: &dyn Fn(&str) -> bool,
) -> bool {
    match ty {
        Type::Name { name: n, .. } => n == name,
        Type::Literal { name: n, .. } => n == name,
        // 头部位置同样是类型引用：`F(A)` 的应用头部 `F` 指的是**构造器变量**
        // （高阶类型参数，RFC-010 元类型层级的 Type1/Type2+ 值）——与 `F` 单独
        // 出现在类型位同义。此前只下钻实参位，头部被漏掉，于是类型体内的
        // `field: F(A)` 判不出 F 被当类型用，注解校验误报 E1003。
        //
        // 谓词应用反过来：头名是编译期谓词时**实参位是值**（`Terminates(b)`），
        // 整个应用不构成类型引用。
        Type::Generic {
            name: head, args, ..
        } => {
            if is_predicate(head) {
                return false;
            }
            head == name
                || args
                    .iter()
                    .any(|a| name_used_as_type(name, a, is_predicate))
        }
        Type::Fn {
            params,
            return_type,
        } => {
            params.iter().any(|p| {
                p.ty.as_ref()
                    .is_some_and(|t| name_used_as_type(name, t, is_predicate))
            }) || name_used_as_type(name, return_type, is_predicate)
        }
        Type::Ptr(inner) => name_used_as_type(name, inner, is_predicate),
        Type::Ref { inner, .. } => name_used_as_type(name, inner, is_predicate),
        Type::Tuple(types) => types
            .iter()
            .any(|t| name_used_as_type(name, t, is_predicate)),
        // 类型定义体：字段类型里的引用也算（`SafeArray = { data: Array(T, N) }`
        // 的 `N`）。此前缺这两个分支，const 泛型参数在定义体内被引用时
        // 判不出「被当类型用」，注解校验会把合法 const 参数误报未知名。
        Type::Struct { body } => body.iter().any(|item| match item {
            TypeBodyItem::Field(f) => name_used_as_type(name, &f.ty, is_predicate),
            TypeBodyItem::Expr(e) => name_used_as_type(name, e, is_predicate),
            _ => false,
        }),
        Type::NamedStruct { fields, .. } => fields
            .iter()
            .any(|f| name_used_as_type(name, &f.ty, is_predicate)),
        // 编译期表达式（`Assert(N > 0)` 的 `N > 0`）：名字引用只认 `Expr::Var` 叶子，
        // 表达式遍历复用 `collect_expr_ident_refs`（与 `collect_name_refs` 的
        // `ConstExpr` 臂同一份）——不新写第二份表达式遍历。
        Type::ConstExpr(expr) => {
            let mut refs = HashSet::new();
            collect_expr_ident_refs(expr, &mut refs);
            refs.contains(name)
        }
        // `Type[T]` / `Type[K, V]`：`Type` 关键字本身不是可判定的候选名，只有实参是。
        Type::MetaType { args, .. } => args
            .iter()
            .any(|arg| name_used_as_type(name, arg, is_predicate)),
        // 括号透明（RFC-004：括号声明完整类型，内层同义）。
        Type::Paren(inner) => name_used_as_type(name, inner, is_predicate),
        // 具名括号：`param` 是返回形式参数名的**声明**（binder），不是引用；
        // 只有内层类型是（见函数文档）。
        Type::NamedParen { inner, .. } => name_used_as_type(name, inner, is_predicate),
    }
}

/// 标注是否为**类型级函数**：`(T: Type) -> Type` —— 参数位与返回位都是 `Type`。
///
/// 这是高阶类型参数（构造器变量）的声明形态：F 的取值范围是类型构造器，
/// 而非某个具体类型（RFC-010 元类型层级 Type1/Type2+）。
pub fn is_type_level_fn(
    params: &[Param],
    return_type: &Type,
) -> bool {
    fn is_type(t: &Type) -> bool {
        match t {
            Type::MetaType { .. } => true,
            Type::Name { name, .. } => name == "Type",
            _ => false,
        }
    }
    !params.is_empty()
        && is_type(return_type)
        && params.iter().all(|p| p.ty.as_ref().is_some_and(is_type))
}

/// 分类注入：SPEC §7.1 的三个问答都由调用方提供（L2 不持有任何名字表）。
pub struct ParamClassifier<'a> {
    /// 该类型名可作编译期值参数候选（非 `Type` 的具体类型；权威表在 L3）。
    pub is_const_param_type: &'a dyn Fn(&str) -> bool,
    /// 该名字在**类型命名空间**里可解析（已声明类型 + 类型参数）。
    pub is_type_name: &'a dyn Fn(&str) -> bool,
    /// 该名字是编译期谓词（谓词应用的实参位是**值**不是类型）。
    pub is_predicate: &'a dyn Fn(&str) -> bool,
}

/// 编译期值参数分类——**唯一**分类器（SPEC type-system §7.1 / RFC-011 §4.1）。
///
/// 此前有三套独立判据（`extract_generic_param_names` 只扫其它形参、漏返回位；
/// `Type::const_used_layer_names` 只看本层之外；`ir_gen::split_curry` 自带游标判据），
/// 三者在「名字出现在类型位置」的形态上给出不同位次。本函数是它们的共同上游：
/// 输出**编译期值参数的名单**，其余形参是运行期值参数（推论见 SPEC §7.1 末段：
/// 编译期值参数不占**实现头绑定序列**的位；调用点实参序列另算）。
///
/// 判定按规范的顺序：形态粗筛（非 `Type` 的具体类型）→ 用途精筛（名字在**任一**
/// `Type` 子树被引用）→ 命名空间解析（解析到类型即与形参无关）→ 单调不动点。
/// 谓词应用的实参位是**值**不是类型（`Terminates(b)` 的 `b`），故扫描按 `is_predicate`
/// 排除——否则值参数会被误判成编译期值参数。
pub fn classify_compile_time_value_params(
    params: &[Param],
    type_positions: &[&Type],
    cls: &ParamClassifier<'_>,
) -> std::collections::HashSet<String> {
    let is_const_param_type = cls.is_const_param_type;
    let is_type_name = cls.is_type_name;
    let is_predicate = cls.is_predicate;
    // ① 形态粗筛：非 Type 的具体类型标注 + 有名形参
    let candidates: Vec<String> = params
        .iter()
        .filter(|p| p.is_named())
        .filter(|p| match p.ty.as_ref() {
            Some(Type::Name { name, .. }) => name != "Type" && is_const_param_type(name),
            _ => false,
        })
        .map(|p| p.name.clone())
        .collect();
    if candidates.is_empty() {
        return std::collections::HashSet::new();
    }

    // ②③④ 用途精筛 + 命名空间解析 + 单调不动点
    //
    // **注意：这个循环在候选集内与单趟扫描等价，不是真迭代。**
    // `is_type_name` 是注入的固定闭包，`confirmed` 集只参与循环头的自身检查、从不进入
    // `is_type_name` 查询 ⇒ 第 2 轮对未确认候选的判定谓词逐字同第 1 轮，必无新增。
    // 实测：把循环强制改成单趟，全量 lib 判据全绿（见 09a §P6 6.8.4 §六）。
    //
    // SPEC §7.1 第 3 条要求「已确认者进入类型命名空间」，**此处尚未实现**：候选标注必须
    // 是具体类型（`Int`/`Bool`/…），任一候选的标注不可能引用另一候选，故该并集在当前
    // 候选集下不会改变任何判定。若将来放宽候选形态（例如允许候选标注引用另一候选），
    // 这里必须改成真迭代——否则会静默退化成单趟。
    let mut confirmed: std::collections::HashSet<String> = std::collections::HashSet::new();
    loop {
        let mut grew = false;
        for name in &candidates {
            if confirmed.contains(name) {
                continue;
            }
            // 解析到类型命名空间 ⇒ 该名字在类型位置上是类型引用，与形参无关。
            // 命名空间 = 注入的声明面（`is_type_name`），不含 `confirmed`——见上面的说明。
            if is_type_name(name) {
                continue;
            }
            if type_positions
                .iter()
                .any(|t| name_used_as_type(name, t, is_predicate))
            {
                confirmed.insert(name.clone());
                grew = true;
            }
        }
        if !grew {
            break;
        }
    }
    confirmed
}
/// 从签名形参里抽出**结构可判定**的泛型参数（名字 + 约束）。
///
/// 三类来源：`Type`/`MetaType` 标注、探针判为 const 参数类型的注解、
/// 约束名（探针判定的运算符接口）、类型级函数标注（构造器变量）。其余
/// `Name` 标注（用户类型的值参数）保守跳过——trait 表不在 L2，带 trait 约束的
/// 分类由 L3 的 `classify_generic_params` 另做。
pub fn extract_generic_param_names(
    params: &[Param],
    probe: &dyn TypeEnvProbe,
) -> Vec<GenericParamName> {
    params
        .iter()
        .filter_map(|p| {
            // 无名形参（形态 2：name 空串）不得产出泛型名——空串不是可检索的
            // 名字，任何按名查找都永不命中（守卫单点，见 Param 的三形态说明）。
            if !p.is_named() {
                return None;
            }
            let ty = p.ty.as_ref()?;
            match ty {
                // (T: Type) — MetaType
                Type::MetaType { .. } => Some(GenericParamName {
                    name: p.name.clone(),
                    constraints: Vec::new(),
                }),
                Type::Name { name, .. } if name == "Type" => Some(GenericParamName {
                    name: p.name.clone(),
                    constraints: Vec::new(),
                }),
                // `(N: Int)` 是 const 泛型，但 `(a: Int)` 是普通值参数——
                // 两者类型标注都是 `Int`，仅凭类型无法区分。
                // 判据：参数名是否在**同一签名**的其它位置被当作类型引用
                //（如 `f: (N: Int) -> (n: N) -> Int` 的 `N`）。
                // 不做此判别会把 `plain: (a: Int) -> Int` 误标为泛型函数，
                // 单态化据此将其删除且永不重建（实测 E6006，#351）。
                //
                // 「哪些类型名可作 const 参数」由**注入探针**回答（WBS 6.6.9 把该表
                // 移到 L3 的 `sema::probe::CONST_PARAM_TYPES`）：parser 不再
                // 持有自己的表、也不再用内置名表二次复核——探针说不就是不。
                Type::Name { name, .. } if probe.is_const_param_type(name) => {
                    let used_as_type = params.iter().any(|q| {
                        q.ty.as_ref().is_some_and(|qt| {
                            name_used_as_type(&p.name, qt, &|n| {
                                probe.kind_of(n) == NameKind::Predicate
                            })
                        })
                    });
                    if used_as_type {
                        Some(GenericParamName {
                            name: p.name.clone(),
                            constraints: Vec::new(),
                        })
                    } else {
                        None
                    }
                }
                // RFC-011 §2.1：约束形参（`T: Add`——标注为约束名）。
                // 是不是约束名由注入探针判定（权威表在 L3 的运算符接口表）；
                // 其余 Name（用户类型标注的值参数）仍保守跳过。
                Type::Name { name, .. } if probe.is_constraint(name) => Some(GenericParamName {
                    name: p.name.clone(),
                    constraints: vec![ty.clone()],
                }),
                // 高阶类型参数（构造器变量）：标注是类型级函数 `(T: Type) -> Type`。
                // 与 `classify_generic_params` 同一判据（is_type_level_fn），否则
                // 声明侧把它当类型参数、IR 侧却少登记一个泛型参数——单态化请求的
                // 类型实参数与 IR 的 generic_params 对不上，直接 E3018。
                Type::Fn {
                    params,
                    return_type,
                } if is_type_level_fn(params, return_type) => Some(GenericParamName {
                    name: p.name.clone(),
                    constraints: Vec::new(),
                }),
                Type::Name { .. } => {
                    // 无法确认是否为 trait → 保守不下泛型参数
                    None
                }
                // (T: Clone + Add) — 无法确认所有元素都是约束 → 保守忽略（L3 类型检查负责）
                Type::Tuple(_types) => None,
                _ => None,
            }
        })
        .collect()
}

/// 用 trait_table 判定泛型参数分类（替代 extract_generic_params 的旧大小写猜测）。
///
/// MetaType 或 Name("Type") → Type 参数（无约束）；
/// 探针判为 const 参数类型的注解（L3 的 `CONST_PARAM_TYPES`）→ Const 参数；
/// Name 且 `is_trait(name)` → Type 参数（约束=该 annotation）；
/// Tuple 且所有元素都是 trait → Type 参数（多约束）；
/// 其余 → 跳过（值参数）。
pub fn classify_generic_params(
    params: &[Param],
    is_trait: &dyn Fn(&str) -> bool,
    probe: &dyn TypeEnvProbe,
) -> Vec<GenericParam> {
    params
        .iter()
        .filter_map(|p| {
            // 无名形参（形态 2：name 空串）不得产出泛型名——空串不是可检索的
            // 名字，任何按名查找都永不命中（守卫单点，见 Param 的三形态说明）。
            if !p.is_named() {
                return None;
            }
            let ty = p.ty.as_ref()?;
            match ty {
                // (T: Type) — MetaType
                Type::MetaType { .. } => Some(GenericParam {
                    name: p.name.clone(),
                    kind: GenericParamKind::Type,
                    constraints: Vec::new(),
                }),
                Type::Name { name, .. } if name == "Type" => Some(GenericParam {
                    name: p.name.clone(),
                    kind: GenericParamKind::Type,
                    constraints: Vec::new(),
                }),
                Type::Name { name, .. } if probe.is_const_param_type(name) => Some(GenericParam {
                    name: p.name.clone(),
                    kind: GenericParamKind::Const {
                        const_type: Box::new(ty.clone()),
                    },
                    constraints: Vec::new(),
                }),
                Type::Name { name, .. } if is_trait(name) => Some(GenericParam {
                    name: p.name.clone(),
                    kind: GenericParamKind::Type,
                    constraints: vec![ty.clone()],
                }),
                // 高阶类型参数（构造器变量）：标注是**类型级函数** `(T: Type) -> Type`——
                // 参数位与返回位都是 `Type`。F 的取值范围是类型构造器（RFC-010
                // 元类型层级 Type1/Type2+），故按类型参数登记，而不是值参数。
                Type::Fn {
                    params,
                    return_type,
                } if is_type_level_fn(params, return_type) => Some(GenericParam {
                    name: p.name.clone(),
                    kind: GenericParamKind::Type,
                    constraints: Vec::new(),
                }),
                // annotation 为具体类型（非 trait、非探针认的 const 参数类型）→ 非泛型参数
                Type::Name { .. } => None,
                Type::Tuple(types) => {
                    let all_traits = types.iter().all(|t| match t {
                        Type::Name { name, .. } => is_trait(name),
                        _ => false,
                    });
                    if all_traits && !types.is_empty() {
                        Some(GenericParam {
                            name: p.name.clone(),
                            kind: GenericParamKind::Type,
                            constraints: types.clone(),
                        })
                    } else {
                        None
                    }
                }
                _ => None,
            }
        })
        .collect()
}

impl Expr {
    /// 调用表达式的**唯一构造器**（RFC-039 D20 / WBS 4.5.2）。
    ///
    /// 全仓调用点一律经此构造：`func` 收 `Expr` 或 `Box<Expr>` 皆可（`Box` 有
    /// 恒等 `Into`）。`plan_id` 由解析器分配（`ParserState::alloc_plan_id`）；
    /// 解析器之外的合成调用（常量求值、IR 内建、测试夹具）记
    /// [`PlanId::UNPLANNED`]。
    pub fn call(
        func: impl Into<Box<Expr>>,
        args: Vec<Expr>,
        named_args: Vec<(String, Expr)>,
        span: Span,
        plan_id: PlanId,
    ) -> Expr {
        Expr::Call {
            func: func.into(),
            args,
            named_args,
            span,
            plan_id,
        }
    }

    /// 表达式的源码 span（穷举变体，新增变体时编译器强制补全）
    pub fn span(&self) -> Span {
        match self {
            Expr::Lit(_, s)
            | Expr::Var(_, s)
            | Expr::Return(_, s)
            | Expr::Break(s)
            | Expr::Continue(s) => *s,
            Expr::BinOp { span, .. }
            | Expr::UnOp { span, .. }
            | Expr::Call { span, .. }
            | Expr::If { span, .. }
            | Expr::Match { span, .. }
            | Expr::While { span, .. }
            | Expr::For { span, .. }
            | Expr::SpawnFor { span, .. }
            | Expr::Borrow { span, .. }
            | Expr::FieldAccess { span, .. }
            | Expr::Index { span, .. }
            | Expr::Tuple(_, span)
            | Expr::List(_, span)
            | Expr::Cast { span, .. }
            | Expr::Try { span, .. } => *span,
            Expr::Block(block) => block.span,
            Expr::ListComp { span, .. }
            | Expr::Dict(_, span)
            | Expr::In { span, .. }
            | Expr::Lambda { span, .. }
            | Expr::Ref { span, .. }
            | Expr::Unsafe { span, .. }
            | Expr::Spawn { span, .. }
            | Expr::FString { span, .. }
            | Expr::Error(span) => *span,
        }
    }

    /// 解构 Assign 目标：`x` → (x, None)；`Type.field` → (field, Some(Type))；其他 → None
    pub fn receiver_parts(&self) -> Option<(String, Option<String>)> {
        match self {
            Expr::Var(n, _) => Some((n.clone(), None)),
            Expr::FieldAccess { expr, field, .. } => {
                if let Expr::Var(tn, _) = expr.as_ref() {
                    Some((field.clone(), Some(tn.clone())))
                } else {
                    Some((field.clone(), None))
                }
            }
            _ => None,
        }
    }

    /// 解构函数值：Lambda → (params, body)；Block → (空, body)；其他 → (空, 空)
    pub fn callable_parts(&self) -> (Vec<Param>, Vec<Stmt>) {
        match self {
            Expr::Lambda { params, body, .. } => (params.clone(), body.stmts.clone()),
            Expr::Block(block) => (Vec::new(), block.stmts.clone()),
            _ => (Vec::new(), Vec::new()),
        }
    }

    /// `name = <expr>` 中，`mut` 绑定是否为类型/方法的**元绑定**——
    /// 这类左值不引入变量，不适用 spec §4.3 的声明/赋值判定。
    ///
    /// 两种形态：
    /// - **类型定义绑定**（RFC-010）：`Db = unsafe { Db: Type = {...}; Db }`
    ///   块内定义类型、尾表达式交回类型名——编译期构造，无运行时绑定。
    /// - 调用方自有的其他形态（如 `Type.method = f` 方法绑定）由调用方判定。
    pub fn is_type_def_binding(value: Option<&Expr>) -> bool {
        let Some(Expr::Unsafe { body, .. }) = value else {
            return false;
        };
        // 块内**直接**包含类型定义，且尾表达式引用该类型名
        let mut def_names: Vec<&str> = Vec::new();
        for st in &body.stmts {
            if let crate::ast::StmtKind::TypeDefinition { name, .. } = &st.kind {
                def_names.push(name.as_str());
            }
        }
        if def_names.is_empty() {
            return false;
        }
        matches!(
            body.stmts.last().map(|s| &s.kind),
            Some(crate::ast::StmtKind::Expr(e))
                if matches!(e.as_ref(), Expr::Var(n, _) if def_names.contains(&n.as_str()))
        )
    }

    /// `name = <value>` 是**函数定义**还是**值绑定**？（B 方案）
    ///
    /// # 判据：注解
    ///
    /// | 情形 | 结果 | 依据 |
    /// | ---- | ---- | ---- |
    /// | `value` 是 `Lambda`（`=>`） | 函数 | `=>` 是显式函数构造子 |
    /// | 注解是 `Fn` | 函数 | 声明了函数类型 |
    /// | 注解是非 `Fn` 类型 | 值 | 注解即类型 |
    /// | **无注解** | **值** | **内容决定类型** |
    ///
    /// # 无注解时按内容推断
    ///
    /// 无注解的 `f = { 5 }` 是**值** `5`，不是函数——块的值由内容给出，
    /// 不由注解存在与否决定。要定义函数就写注解：`f: () -> Int = { 5 }`。
    /// 这条规则与 `d = {"a": 1}` 同源：类型由内容决定。
    pub fn block_binding_is_function(
        type_annotation: Option<&Type>,
        value: Option<&Expr>,
    ) -> bool {
        match value {
            // `=>` 是显式函数构造子，注释无关
            Some(Expr::Lambda { .. }) => true,
            // 块：只有 Fn 注解才是函数；无注解按值处理（内容决定类型）
            Some(Expr::Block(_)) => matches!(type_annotation, Some(Type::Fn { .. })),
            // 非块值的绑定不参与此判定
            _ => false,
        }
    }

    /// RFC-004: 从绑定语句 `Type.method = fn[pos]` 的 index 部分提取位置列表。
    /// 支持单个整数与整数元组；负索引（`-1` 从末尾计数）原样保留，
    /// 由调用方按被绑函数元数归一化。无法识别时回退 `[0]`（与既有行为一致）。
    pub fn extract_binding_positions(index: &Expr) -> Vec<i64> {
        fn one(e: &Expr) -> Option<i64> {
            match e {
                Expr::Lit(Literal::Int(n), _) => Some(*n as i64),
                Expr::UnOp {
                    op: UnOp::Neg,
                    expr,
                    ..
                } => match expr.as_ref() {
                    Expr::Lit(Literal::Int(n), _) => Some(-(*n as i64)),
                    _ => None,
                },
                _ => None,
            }
        }
        match index {
            Expr::Tuple(items, _) => {
                let parsed: Vec<i64> = items.iter().filter_map(one).collect();
                if parsed.is_empty() {
                    vec![0]
                } else {
                    parsed
                }
            }
            other => one(other).map(|p| vec![p]).unwrap_or_else(|| vec![0]),
        }
    }
}

#[cfg(test)]
mod tests;
