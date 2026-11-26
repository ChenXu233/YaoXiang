#![allow(clippy::result_large_err)]

//! 表达式类型推断（`inference/expressions/`）
//!
//! 实现各种表达式的类型推断。使用统一的 ScopeManager 管理变量作用域。
//!
//! 子模块（WBS 7.0.4 迁入；原 `inference/expressions.rs` 已删除）：
//! - `expr.rs`：除调用裁决族以外的全部方法（注入接口、作用域/变量状态面、表达式/语句推断）；
//! - `call/`：调用裁决族（`infer_call_expr` 入口，按方法簇分 `args` 实参绑定、
//!   `instantiation` 调用点泛型实例化、`method` 方法调用键三个子模块）。
//!
//! 本文件保留 `ExpressionInferrer` 的结构定义与构造族，以及被 expr 与 call 两侧共用、
//! 且被 `inference/statements.rs` 直接引用的模块级函数助手。
//!
//! 结构体字段与方法的可见性（`pub(super)`）是跨文件拆分的机械需要：这些字段与方法
//! 原先私有于单个 impl 文件，拆成子模块后仍要跨文件读写同一份推断器状态。

mod call;
mod expr;

use crate::frontend::core::sema::passes::overload;
use crate::frontend::core::types::{MonoType, PolyType, TypeConstraintSolver};
use crate::middle::passes::mono::instance::InstantiationRequest;
use std::collections::{HashMap, HashSet};

use super::call_ownership::CallOwnershipTable;
use super::scope::ScopeManager;

/// 空的 Native 签名表（默认值）
static EMPTY_SIGNATURES: std::sync::LazyLock<HashMap<String, MonoType>> =
    std::sync::LazyLock::new(HashMap::new);

/// 空的类型定义表（默认值）：与 `TypeEnvironment.types` 同型，判等判定点的 δ 表
static EMPTY_POLY_TYPES: std::sync::LazyLock<HashMap<String, PolyType>> =
    std::sync::LazyLock::new(HashMap::new);

/// #387：native arity 区间表空缺省
static EMPTY_ARITY: std::sync::LazyLock<HashMap<String, (usize, Option<usize>)>> =
    std::sync::LazyLock::new(HashMap::new);

/// #454：native 导出形参名表空缺省（未注入时使用）
static EMPTY_NATIVE_PARAM_NAMES: std::sync::LazyLock<
    HashMap<String, crate::frontend::core::sema::environment::ExportParamInfo>,
> = std::sync::LazyLock::new(HashMap::new);

/// E1113（6.9.2-B）：调用点形参名表空缺省（未注入时使用——直构测试 / 未走签名
/// 收集的入口）。缺省 = **不判** E1113，绝不按位置猜。
static EMPTY_CALL_SITE_PARAMS: std::sync::LazyLock<
    HashMap<String, crate::frontend::core::sema::checker::CallSiteSlots>,
> = std::sync::LazyLock::new(HashMap::new);

/// RFC-009 读视图：剥离借用层，算术/比较按 inner 类型判定（读穿透）
pub(super) fn read_view(ty: &MonoType) -> MonoType {
    let mut cur = ty;
    while let MonoType::Ref { inner, .. } = cur {
        cur = inner;
    }
    cur.clone()
}

/// E1002 的 expected 参数——Add/Sub/Mul/Div 白名单接受集的描述文（issue #509
/// 残留③）。
///
/// 两个遍历器（`ExpressionInferrer::infer_binary` 与 `StatementChecker` 的
/// 二元运算臂）报告的是**同一个接受集**，文面必须同源——各自硬编一份即
/// 「同一行为 ≥2 处复制」（禁令三 A，#509 前正是两份拷贝且带后缀漂移）。
/// `StatementChecker` 臂用基文；`ExpressionInferrer` 臂保持既有文面、拼接
/// 「或实现对应算术接口」后缀（本常量只收公共部分，两处输出逐字节不变）。
pub(super) const E1002_ARITH_EXPECTED: &str = "Int/Float/String/List（两侧同型）";

/// 二元运算 Never 短路的裁决（issue #509 残留④：判据收敛为共享纯函数）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum NeverShortCircuit {
    /// 两侧都是 `Never`：无具体类型可依，调用方发 fresh var 推迟
    Both,
    /// 仅左侧是 `Never`：结果取右侧的具体形态
    TakeRight,
    /// 仅右侧是 `Never`：结果取左侧的具体形态
    TakeLeft,
}

/// 二元运算 Never 短路判据——两个遍历器共用的**纯函数裁决**（issue #509
/// 残留④）。
///
/// #469 处 3：绑定失败以 `Never` 毒化登记（`StatementChecker` 与
/// `ExpressionInferrer` 各自的环境写入点），毒化引用参与二元运算不施加类型
/// 约束——unify 爆炸原理（`solver.rs` 的 `Never <: T`）在运算位的延伸，否则
/// E1001 消了、E1002 又冒出来。判据原为两份逐字拷贝（仅注释互指，#509），
/// 一处改另一处忘改即两路径裁决分歧，故收敛为本函数：按引用取操作数返回
/// 裁决，不持推断器状态——fresh var 的发放需 `&mut solver`，由调用方执行
/// （3.20.34 C 组 fn helper 因借用冲突不可编译的教训）。
///
/// 返回 `None` 表示两侧都非 `Never`，按正常推断走。
pub(super) fn never_short_circuit(
    left: &MonoType,
    right: &MonoType,
) -> Option<NeverShortCircuit> {
    match (left, right) {
        (MonoType::Never, MonoType::Never) => Some(NeverShortCircuit::Both),
        (MonoType::Never, _) => Some(NeverShortCircuit::TakeRight),
        (_, MonoType::Never) => Some(NeverShortCircuit::TakeLeft),
        _ => None,
    }
}

static EMPTY_GENERIC_TYPE_DEFS: std::sync::LazyLock<
    HashMap<String, crate::frontend::core::sema::environment::GenericTypeDef>,
> = std::sync::LazyLock::new(HashMap::new);

/// RFC-011b: 空接口实现登记表（未注入时使用——测试/独立构造）
static EMPTY_INTERFACE_IMPL_REGISTRY: std::sync::LazyLock<
    HashMap<String, Vec<crate::frontend::core::sema::environment::InterfaceImplEntry>>,
> = std::sync::LazyLock::new(HashMap::new);

/// RFC-010: 空和类型登记表（未注入时使用）
static EMPTY_SUM_TYPES: std::sync::LazyLock<
    HashMap<String, Vec<crate::frontend::core::sema::environment::SumVariantDef>>,
> = std::sync::LazyLock::new(HashMap::new);

/// 表达式类型推断器
///
/// 使用统一的 ScopeManager 管理变量作用域，
/// 不再维护独立的作用域栈。
/// 空表：未注入声明序类型参数名时使用（测试/独立构造）。
static EMPTY_FN_TYPE_PARAMS: std::sync::LazyLock<HashMap<String, Vec<String>>> =
    std::sync::LazyLock::new(HashMap::new);

/// **编译期证据判定的三态结果**（RFC-039 D62 / issue #533，6.9.2-B）。
///
/// 此前用 `Option` 把「非 q=0 调用」与「目标调用缺证据」混成同一个 `None`：
/// 两者都静默按运行期传参，没有任何诊断——错的是第二条（`f(y)` 该被拒绝），
/// 不是第一条（`f(42)` 之外的普通调用本就与我无关）。
enum ConstArgOutcome {
    /// 非目标：被调不可解析 / 没有 q=0 形参 / 调用点没有需要证据的实参
    NotTarget,
    /// 证据齐备：`Some` = 已登记特化请求；`None` = 形态不支持特化（混合 /
    /// 命名 / deferred）但证据检查已通过——本轮不因「不支持特化」跳过检查，
    /// 也不谎称已特化
    Evidenced(Option<MonoType>),
    /// 目标但拿不到编译期证据 ⇒ E1113（调用点该实参处，不是 warning）
    MissingEvidence(crate::util::diagnostic::Diagnostic),
}

pub struct ExpressionInferrer<'a> {
    /// 共享的作用域管理器
    scope: &'a mut ScopeManager,
    /// 约束求解器
    solver: &'a mut TypeConstraintSolver,
    /// 循环嵌套深度（#311）：while/for 体内才允许 break/continue。
    /// 函数体（Lambda）边界重置——闭包体不继承外层循环上下文；
    /// spawn for 体是逐元素闭包执行，不计入可 break 的循环上下文。
    loop_depth: usize,
    /// 函数边界嵌套深度（RFC-010a 规则②）：0 = 模块初始化层（Script 顶层），
    /// ≥1 = 函数体内。命名函数体经语句侧 is_top_level 镜像下传（set_fn_depth）；
    /// lambda / spawn / 带返回类型的函数体在本 walker 进体时 +1。
    /// `return` 在深度 0 处编译期拒绝（E1109）。
    fn_depth: usize,
    /// 重载候选存储引用
    overload_candidates: &'a HashMap<String, Vec<overload::OverloadCandidate>>,
    /// Native 函数签名引用
    native_signatures: &'a HashMap<String, MonoType>,
    /// 模块别名集合（#396）：Some 时，模块别名 Struct 上取成员失败报
    /// E1043（模块语义）而非 E1042（struct 语义）。测试等直构场景为 None。
    module_aliases: Option<&'a std::collections::HashSet<String>>,
    /// 模块别名 → 模块限定键（`use lib` → `lib`；`use std.list` → `std.list`；
    /// `use std.io as printer` → `std.io`）。由 StatementChecker 加工 use 时登记，
    /// 随委托注入。
    ///
    /// 用途（RFC-039 §3.4.8 ①）：限定调用 `lib.f(x)` 的实例化请求必须携带
    /// **与 merged-IR 同源的限定名** `{module_key}.{f}`——ir_gen 的
    /// `qualify_names` 把每个模块的顶层函数重写为这一格式，下游 mono 消费
    /// merged_ir 时按名匹配泛型定义表；用裸 `f` 匹配不到 `lib.f`，请求被静默
    /// 跳过。限定名拼接一律经 [`crate::frontend::module::symbol::DefPath::project`]（唯一拼接实现）。
    ///
    /// None（测试直构 / 独立表达式推断）→ 限定调用不发请求，行为与注入前一致。
    module_namespaces: Option<&'a HashMap<String, String>>,
    /// 当前函数的 Result 错误类型（若为 None，则不允许使用 `?`）
    result_err: Option<MonoType>,
    /// 当前函数的预期返回类型（用于 return 语句的类型检查）
    expected_return_type: Option<MonoType>,
    /// 当前函数**声明注解**的位置（#353 口径 / WBS 6.8.2a D54 ②）：return 值
    /// 与声明返回位不符时指向注解，而不是体内那个（可能完全正确的）值——与语句
    /// 形态的 check_return_value_type 同一落点（两种 return AST 形状同一诊断、
    /// 同一位置）。None = 当前期望返回位不是本地声明给的（匿名 lambda / spawn 体），
    /// 此时退回 return 语句自身的位置。
    return_annotation_span: Option<crate::util::span::Span>,
    /// 当前嵌套的 `unsafe {}` 深度（RFC-010：unsafe 块内允许类型定义）
    unsafe_depth: usize,
    /// 方法绑定表: "Type.method" -> MonoType(Fn)
    /// 用于方法调用语法糖解析: p.draw(screen) → Point.draw(p, screen)
    method_bindings: &'a HashMap<String, MonoType>,
    /// RFC-011b: 接口实现登记表（运算符查询唯一判据，不经名字解析）
    interface_impl_registry:
        &'a HashMap<String, Vec<crate::frontend::core::sema::environment::InterfaceImplEntry>>,
    /// RFC-010: 和类型登记表（类型名 → 变体定义，声明序）
    sum_types: &'a HashMap<String, Vec<crate::frontend::core::sema::environment::SumVariantDef>>,
    /// 类型定义表: type_name -> PolyType（与 `TypeEnvironment.types` 同型）
    ///
    /// 用于 TypeRef → Struct 解析（字段访问等）；6.9.3d 起同时是判等判定点的
    /// δ 展开表（`decide` 把它交给 `DefeqContext.types`）。
    type_defs: &'a HashMap<String, PolyType>,
    /// 泛型类型定义表
    /// 用于 List(1, 2, 3) 等泛型类型构造调用的实例化
    generic_type_defs:
        &'a HashMap<String, crate::frontend::core::sema::environment::GenericTypeDef>,
    /// RFC-011b：当前函数的约束形参（形参 → 接口名）——定义体内
    /// 约束类型上的运算符延迟派发依据
    current_fn_param_constraints: Option<&'a HashMap<String, String>>,
    /// RFC-011a §3：重载候选签名与 IR 混编名（方法级重载决议）
    method_overloads: Option<&'a HashMap<String, Vec<MonoType>>>,
    method_overload_ir_names: Option<&'a HashMap<String, Vec<String>>>,
    /// RFC-011a §3 / #452：重载候选的声明期形参名（与候选序一一对应，含接收者位）。
    /// 命名实参按形参名绑定到候选参数位——决议与冲突判定都读它。
    method_overload_param_names: Option<&'a HashMap<String, Vec<Vec<String>>>>,
    /// 期望类型（带注解绑定传入，方法重载按期望返回决议）
    current_expected: Option<MonoType>,
    /// RFC-011a §3：重载决议点（**调用点 PlanId** → 混编 IR 名；RFC-039 D20）
    pub overload_resolutions: Vec<(crate::util::plan_id::PlanId, String)>,
    /// 实例化请求（收集遇到的所有泛型函数实例化需求）
    pub instantiation_requests: Vec<InstantiationRequest>,
    /// 泛型函数的声明序类型参数名：函数名 -> ["T", "Acc", ...]（由 StatementChecker 注入）。
    /// 调用点据此区分「真类型参数」与「恰好未在本地注册的普通类型名」（std 的 `Error` 等）。
    generic_fn_type_params: &'a HashMap<String, Vec<String>>,
    /// 最近一次 `monomorphize` 解出的类型实参（按声明顺序），供紧随其后的
    /// `collect_instantiation_request` 取用。
    last_type_args: Vec<MonoType>,
    /// FieldAccess 臂解析到的**方法绑定键**（`Box.get` 形态；与
    /// `method_bindings` 的查找键同一构造）。写入纪律：FieldAccess 臂进体先
    /// 清空、命中方法绑定时写入（臂是唯一生产者）。消费在**调用边界**：
    /// `infer_call_expr` 在 `infer_expr(func)` 之后立即 `take` 快照——被调方
    /// 是 FieldAccess 时快照值经参数传入调用点证据判定（E1113 的方法面，
    /// issue #538），否则丢弃。判定只消费快照、不读状态位——实参推断对状态
    /// 位的覆盖不影响本次判定，残留在各调用边界被清除。
    last_method_binding_key: Option<String>,
    /// RFC-011a §6 存在类型强制点（具体→存在包装点，ir_gen 按 span 查表注入包装）
    pub existential_coercions: Vec<super::existential::ExistentialCoercion>,
    /// RFC-011b: 运算符派发点（显式接口实现命中处，ir_gen 按 span 注入方法调用）
    pub try_expr_impls: Vec<(crate::util::span::Span, String)>,
    pub operator_dispatches:
        Vec<crate::frontend::core::sema::operator_interfaces::OperatorDispatch>,
    /// 依赖类型环境（效应查询）—— 由 StatementChecker 注入
    dep_env: Option<&'a crate::frontend::core::types::eval::dependent_types::DependentTypeEnv>,
    /// 流敏感假设集 Γ（效应注入）—— 由 StatementChecker 注入
    gamma: Option<&'a mut crate::frontend::core::sema::proof::assumptions::FlowSensitiveGamma>,
    /// #321 W1003：导入名监视集（StatementChecker 委托时拷贝注入）
    import_watch: HashMap<String, String>,
    /// #321 W1003：已使用的监视名（委托方经 take_import_used 回收）
    imported_used: HashSet<String>,
    /// #335 G3 类型信息流接口：调用点所有权解析表（按调用 span 键控）
    pub call_ownership: CallOwnershipTable,
    /// #387：native arity 区间表（随签名表同源注入，缺省空表=无区间）
    native_arity: &'a HashMap<String, (usize, Option<usize>)>,
    /// #454：native 导出形参名表（全限定名 + 短名 → 声明序形参名）。
    /// 命名空间限定的原生调用据此按名绑定/判定未知名（此前整段静默跳过）。
    native_param_names:
        &'a HashMap<String, crate::frontend::core::sema::environment::ExportParamInfo>,
    /// E1113（6.9.2-B）：本模块函数的**调用点实参序列**形参名表（函数名/方法键 →
    /// 名字；非类型形参、q=0 保留；条目见 `CallSiteSlots`）。声明面由 checker 的
    /// 签名收集登记，经 StatementChecker 同一条注入链传入；导入函数面由
    /// `import_binding` 按 Export 载荷登记（#539）。缺项 = 事实不足
    /// （直构场景 / 方法面剩余形态）⇒ 不判、不猜。
    declared_call_site_params:
        &'a HashMap<String, crate::frontend::core::sema::checker::CallSiteSlots>,
    /// #389：match scrutinee 推断类型（match 节点 span 键控）——IR 生成期
    /// generate_match_expr_ir 按 span 回查，替代对复合 scrutinee 的 AST 猜测
    pub match_scrutinee_types: HashMap<crate::util::span::Span, MonoType>,
    /// WBS 3.4.1：编译期不可求值的 const 泛型约束（span + 约束描述）——
    /// 事实收集，经 StatementChecker 回收、checker 发射 W1063（先例同构：
    /// instantiation_requests 等字段的同一条汇聚链）。
    pub unevaluable_const_constraints: Vec<(crate::util::span::Span, String)>,
    /// 证明函数基类型表（RFC-027 Phase 2.5）：「返回 Type 的函数名 → 其基类型」。
    /// #523：调用点逐参统一消费——curry 内层参数位的精化应用注册期不剥
    /// （`check_fn_stmt` 只剥外层形参位），原始形态 `IsPositive(N)` 若不按
    /// base 统一，基类型实参被假拒绝（E1002）。剥除判据**单源**：与注册期
    /// 的 `statements::strip_refined_value_type` 共用同一实现
    /// （`strip_refined_with_proof_fns`），本表由 StatementChecker 随委托注入。
    proof_fn_bases: &'a HashMap<String, MonoType>,
}

impl<'a> ExpressionInferrer<'a> {
    /// 创建新的表达式推断器
    pub fn new(
        scope: &'a mut ScopeManager,
        solver: &'a mut TypeConstraintSolver,
        overload_candidates: &'a HashMap<String, Vec<overload::OverloadCandidate>>,
    ) -> Self {
        Self {
            scope,
            solver,
            loop_depth: 0,
            fn_depth: 0,
            overload_candidates,
            native_signatures: &EMPTY_SIGNATURES,
            module_aliases: None,
            module_namespaces: None,
            result_err: None,
            expected_return_type: None,
            return_annotation_span: None,
            unsafe_depth: 0,
            method_bindings: &EMPTY_SIGNATURES,
            type_defs: &EMPTY_POLY_TYPES,
            generic_type_defs: &EMPTY_GENERIC_TYPE_DEFS,
            current_fn_param_constraints: None,
            method_overloads: None,
            method_overload_ir_names: None,
            method_overload_param_names: None,
            current_expected: None,
            overload_resolutions: Vec::new(),
            instantiation_requests: Vec::new(),
            generic_fn_type_params: &EMPTY_FN_TYPE_PARAMS,
            last_type_args: Vec::new(),
            last_method_binding_key: None,
            existential_coercions: Vec::new(),
            interface_impl_registry: &EMPTY_INTERFACE_IMPL_REGISTRY,
            sum_types: &EMPTY_SUM_TYPES,
            try_expr_impls: Vec::new(),
            operator_dispatches: Vec::new(),
            call_ownership: CallOwnershipTable::new(),
            native_arity: &EMPTY_ARITY,
            native_param_names: &EMPTY_NATIVE_PARAM_NAMES,
            declared_call_site_params: &EMPTY_CALL_SITE_PARAMS,
            match_scrutinee_types: HashMap::new(),
            unevaluable_const_constraints: Vec::new(),
            proof_fn_bases: &EMPTY_SIGNATURES,
            dep_env: None,
            gamma: None,
            import_watch: HashMap::new(),
            imported_used: HashSet::new(),
        }
    }

    /// 创建带 native 函数签名的表达式推断器
    pub fn with_native_signatures(
        scope: &'a mut ScopeManager,
        solver: &'a mut TypeConstraintSolver,
        overload_candidates: &'a HashMap<String, Vec<overload::OverloadCandidate>>,
        native_signatures: &'a HashMap<String, MonoType>,
    ) -> Self {
        Self {
            scope,
            solver,
            loop_depth: 0,
            fn_depth: 0,
            overload_candidates,
            native_signatures,
            module_aliases: None,
            module_namespaces: None,
            result_err: None,
            expected_return_type: None,
            return_annotation_span: None,
            unsafe_depth: 0,
            method_bindings: &EMPTY_SIGNATURES,
            type_defs: &EMPTY_POLY_TYPES,
            generic_type_defs: &EMPTY_GENERIC_TYPE_DEFS,
            current_fn_param_constraints: None,
            method_overloads: None,
            method_overload_ir_names: None,
            method_overload_param_names: None,
            current_expected: None,
            overload_resolutions: Vec::new(),
            instantiation_requests: Vec::new(),
            generic_fn_type_params: &EMPTY_FN_TYPE_PARAMS,
            last_type_args: Vec::new(),
            last_method_binding_key: None,
            existential_coercions: Vec::new(),
            interface_impl_registry: &EMPTY_INTERFACE_IMPL_REGISTRY,
            sum_types: &EMPTY_SUM_TYPES,
            try_expr_impls: Vec::new(),
            operator_dispatches: Vec::new(),
            call_ownership: CallOwnershipTable::new(),
            native_arity: &EMPTY_ARITY,
            native_param_names: &EMPTY_NATIVE_PARAM_NAMES,
            declared_call_site_params: &EMPTY_CALL_SITE_PARAMS,
            match_scrutinee_types: HashMap::new(),
            unevaluable_const_constraints: Vec::new(),
            proof_fn_bases: &EMPTY_SIGNATURES,
            dep_env: None,
            gamma: None,
            import_watch: HashMap::new(),
            imported_used: HashSet::new(),
        }
    }

    /// 创建带 native 函数签名 + Result 错误上下文的表达式推断器
    pub fn with_native_signatures_and_result_err(
        scope: &'a mut ScopeManager,
        solver: &'a mut TypeConstraintSolver,
        overload_candidates: &'a HashMap<String, Vec<overload::OverloadCandidate>>,
        native_signatures: &'a HashMap<String, MonoType>,
        result_err: Option<MonoType>,
    ) -> Self {
        Self {
            scope,
            solver,
            loop_depth: 0,
            fn_depth: 0,
            overload_candidates,
            native_signatures,
            module_aliases: None,
            module_namespaces: None,
            result_err,
            expected_return_type: None,
            return_annotation_span: None,
            unsafe_depth: 0,
            method_bindings: &EMPTY_SIGNATURES,
            type_defs: &EMPTY_POLY_TYPES,
            generic_type_defs: &EMPTY_GENERIC_TYPE_DEFS,
            current_fn_param_constraints: None,
            method_overloads: None,
            method_overload_ir_names: None,
            method_overload_param_names: None,
            current_expected: None,
            overload_resolutions: Vec::new(),
            instantiation_requests: Vec::new(),
            generic_fn_type_params: &EMPTY_FN_TYPE_PARAMS,
            last_type_args: Vec::new(),
            last_method_binding_key: None,
            existential_coercions: Vec::new(),
            interface_impl_registry: &EMPTY_INTERFACE_IMPL_REGISTRY,
            sum_types: &EMPTY_SUM_TYPES,
            try_expr_impls: Vec::new(),
            operator_dispatches: Vec::new(),
            call_ownership: CallOwnershipTable::new(),
            native_arity: &EMPTY_ARITY,
            native_param_names: &EMPTY_NATIVE_PARAM_NAMES,
            declared_call_site_params: &EMPTY_CALL_SITE_PARAMS,
            match_scrutinee_types: HashMap::new(),
            unevaluable_const_constraints: Vec::new(),
            proof_fn_bases: &EMPTY_SIGNATURES,
            dep_env: None,
            gamma: None,
            import_watch: HashMap::new(),
            imported_used: HashSet::new(),
        }
    }

    /// 创建带完整上下文（native 签名 + Result + 预期返回类型 + 方法绑定）的表达式推断器
    pub fn with_full_context(
        scope: &'a mut ScopeManager,
        solver: &'a mut TypeConstraintSolver,
        overload_candidates: &'a HashMap<String, Vec<overload::OverloadCandidate>>,
        native_signatures: &'a HashMap<String, MonoType>,
        result_err: Option<MonoType>,
        expected_return_type: Option<MonoType>,
        method_bindings: &'a HashMap<String, MonoType>,
    ) -> Self {
        Self {
            scope,
            solver,
            loop_depth: 0,
            fn_depth: 0,
            overload_candidates,
            native_signatures,
            module_aliases: None,
            module_namespaces: None,
            result_err,
            expected_return_type,
            return_annotation_span: None,
            unsafe_depth: 0,
            method_bindings,
            type_defs: &EMPTY_POLY_TYPES,
            generic_type_defs: &EMPTY_GENERIC_TYPE_DEFS,
            current_fn_param_constraints: None,
            method_overloads: None,
            method_overload_ir_names: None,
            method_overload_param_names: None,
            current_expected: None,
            overload_resolutions: Vec::new(),
            instantiation_requests: Vec::new(),
            generic_fn_type_params: &EMPTY_FN_TYPE_PARAMS,
            last_type_args: Vec::new(),
            last_method_binding_key: None,
            existential_coercions: Vec::new(),
            interface_impl_registry: &EMPTY_INTERFACE_IMPL_REGISTRY,
            sum_types: &EMPTY_SUM_TYPES,
            try_expr_impls: Vec::new(),
            operator_dispatches: Vec::new(),
            call_ownership: CallOwnershipTable::new(),
            native_arity: &EMPTY_ARITY,
            native_param_names: &EMPTY_NATIVE_PARAM_NAMES,
            declared_call_site_params: &EMPTY_CALL_SITE_PARAMS,
            match_scrutinee_types: HashMap::new(),
            unevaluable_const_constraints: Vec::new(),
            proof_fn_bases: &EMPTY_SIGNATURES,
            dep_env: None,
            gamma: None,
            import_watch: HashMap::new(),
            imported_used: HashSet::new(),
        }
    }
}

/// 向后兼容：ExprInferrer 是 ExpressionInferrer 的类型别名
pub type ExprInferrer<'a> = ExpressionInferrer<'a>;

/// 提取命名空间路径前缀：Var("io") → "io"，FieldAccess(FieldAccess(a,b),c) → "a.b.c"
/// （FieldAccess 推断臂与 #317 方法调用探测共用）
fn extract_namespace_path(expr: &crate::ast::Expr) -> Option<String> {
    match expr {
        crate::ast::Expr::Var(name, _) => Some(name.clone()),
        crate::ast::Expr::FieldAccess { expr, field, .. } => {
            extract_namespace_path(expr).map(|p| format!("{}.{}", p, field))
        }
        _ => None,
    }
}

/// Extract a string literal from an AST expression (compile-time evaluation helper)
fn extract_string_literal_from_expr(expr: &crate::ast::Expr) -> Option<String> {
    match expr {
        crate::ast::Expr::Lit(crate::frontend::core::lexer::tokens::Literal::String(s), _) => {
            Some(s.clone())
        }
        _ => None,
    }
}

/// 从表达式提取编译期常量值（用于 const 泛型参数）
fn extract_const_value_from_expr(
    expr: &crate::ast::Expr
) -> Option<crate::frontend::core::types::const_data::ConstValue> {
    use crate::frontend::core::types::const_data::ConstValue;
    match expr {
        crate::ast::Expr::Lit(crate::frontend::core::lexer::tokens::Literal::Int(n), _) => {
            Some(ConstValue::Int(*n))
        }
        crate::ast::Expr::Lit(crate::frontend::core::lexer::tokens::Literal::Bool(b), _) => {
            Some(ConstValue::Bool(*b))
        }
        crate::ast::Expr::Lit(crate::frontend::core::lexer::tokens::Literal::Float(f), _) => {
            Some(ConstValue::Float(*f as f32))
        }
        _ => None,
    }
}

/// 检查名称是否为内置类型名（Type 宇宙的值）
///
/// 单源：标量走 `MonoType::from_builtin_name`（与类型位置同一份表，
/// 不再手抄第二份名单）；容器/泛型构器（`Vec` / `Array` / `Dict` /
/// `Tuple` / `Option` / `Result` / `Range` / `Iter` / `Bytes`）
/// 在类型位置由 `Type::Generic` 分支无条件接受，此处补上对应的值位置识别。
///
/// 这两类名字在**值位置**都是 Type 宇宙的值：`Vec(Int)()` 是类型构造
/// 调用（D6.2 修的就是它报 E1001 unknown variable 'Vec'）。
fn is_builtin_type_name(name: &str) -> bool {
    MonoType::from_builtin_name(name).is_some()
        || is_builtin_generic_type_name(name)
        || name == "Type"
        // RFC-010: std 预置类型名（封闭集合）——Any 顶类型、
        // Error 错误载体（RFC-013 类型族），值位置实参位合法
        || name == "Any"
        || name == "Error"
}

/// 内置泛型容器的类型名（值位置可作类型构造器用）。
///
/// **仅含 RFC-011 规格化了两层构造的 `Vec(T)` / `Array(T, N)`**。
/// 其余容器名（`Dict`/`Tuple`/`Option`/`Result`/`Range`/`Iter`/`Bytes`）
/// 在**值位置**没有类型构造器语义（无 `Dict(Int,Int)()` 这样的写法），
/// 列入它们会让类型检查放行、IR 层却无对应生成器，
/// 结果从清楚的 `E1001 unknown variable` 退化成 `E3006` 编译器内部一致性错。
/// 类型实参是否为**构造器引用**（用户泛型类型名 / 内置泛型容器名）。
///
/// 单一实现（复审轮 1 F1）：`inference::expressions` 与 `inference::statements`
/// 两处消费同经此函数——此前两份逐字副本（禁令三 A：同义重复）。
pub(super) fn is_ctor_reference(
    ty: &MonoType,
    generic_type_defs: &HashMap<String, crate::frontend::core::sema::environment::GenericTypeDef>,
) -> bool {
    match ty {
        MonoType::TypeRef(n) => {
            generic_type_defs.contains_key(n) || is_builtin_generic_type_name(n)
        }
        _ => false,
    }
}

pub(super) fn is_builtin_generic_type_name(name: &str) -> bool {
    matches!(name, "Vec" | "Array")
}

/// 声明体是否在**应用头部**使用了构造器变量（`F(A)`，F 是声明的类型参数名）。
///
/// 只有这类泛型类型的实例化产物里才可能出现「具体应用」（`Box(Int)`、`Vec(Int)`），
/// 需要展开成记录形态才能与实际记录值比较；普通泛型（字段是 `T`、`Container(T)`）
/// 的既有表示不动，避免改变无关程序的行为。
fn body_uses_ctor_head(
    body: &MonoType,
    type_params: &[String],
) -> bool {
    match body {
        MonoType::Generic { name, args } => {
            type_params.iter().any(|p| p == name)
                || args.iter().any(|a| body_uses_ctor_head(a, type_params))
        }
        MonoType::Struct(s) => s
            .fields
            .iter()
            .any(|f| body_uses_ctor_head(&f.ty, type_params)),
        MonoType::Fn {
            params,
            return_type,
        } => {
            params.iter().any(|t| body_uses_ctor_head(t, type_params))
                || body_uses_ctor_head(return_type, type_params)
        }
        MonoType::Ref { inner, .. } => body_uses_ctor_head(inner, type_params),
        MonoType::Union(ts) | MonoType::Intersection(ts) => {
            ts.iter().any(|t| body_uses_ctor_head(t, type_params))
        }
        _ => false,
    }
}

/// 把「具体应用」展开成记录形态：`Box(Int)` → `Struct{Box, v: Int}`。
///
/// 构造器变量在头部的替换（`F(A)` → `Box(Int)`）产出的是应用形态，而实际值是
/// 记录形态 `Struct{Box, ...}`；两者必须同形才能 unify/取字段。展开只对
/// 声明模板已知、实参元数吻合的**用户**类型构造器做（内置容器 `Vec`/`Array`
/// 是运行时长度的表示，保持应用形态）。
///
/// 消费面（同源展开，无第二份实现）：
/// - 调用点构造器变量的头部替换后展开（`specialize`）；
/// - 泛型构造器直造的字段类型校验（`specialize` 内直调）；
/// - 值构造臂的类型实参展开（`instantiate_with_ctor_heads`）；
/// - 绑定校验（#442）：init 是具体应用而注解是同名展开记录时，展开 init
///   再统一（`check_var_stmt_inner`）。
pub(super) fn expand_ctor_applications(
    ty: &MonoType,
    defs: &HashMap<String, crate::frontend::core::sema::environment::GenericTypeDef>,
) -> MonoType {
    match ty {
        MonoType::Generic { name, args } => {
            let expanded_args: Vec<MonoType> = args
                .iter()
                .map(|a| expand_ctor_applications(a, defs))
                .collect();
            if !is_builtin_generic_type_name(name) {
                if let Some(def) = defs.get(name) {
                    let total = def.type_param_names.len() + def.poly.const_binders.len();
                    if expanded_args.len() == total {
                        if let Ok(inst) =
                            crate::frontend::core::sema::TypeEnvironment::instantiate_generic_type(
                                def,
                                &expanded_args,
                            )
                        {
                            return expand_ctor_applications(&inst.ty, defs);
                        }
                    }
                }
            }
            MonoType::Generic {
                name: name.clone(),
                args: expanded_args,
            }
        }
        MonoType::Struct(s) => MonoType::Struct(crate::frontend::core::types::mono::StructType {
            fields: s
                .fields
                .iter()
                .map(
                    |field_info| crate::frontend::core::types::mono::StructFieldInfo {
                        ty: expand_ctor_applications(&field_info.ty, defs),
                        ..field_info.clone()
                    },
                )
                .collect(),
            ..s.clone()
        }),
        MonoType::Fn {
            params,
            return_type,
        } => MonoType::Fn {
            params: params
                .iter()
                .map(|p| expand_ctor_applications(p, defs))
                .collect(),
            return_type: Box::new(expand_ctor_applications(return_type, defs)),
        },
        MonoType::Ref { mutable, inner } => MonoType::Ref {
            mutable: *mutable,
            inner: Box::new(expand_ctor_applications(inner, defs)),
        },
        MonoType::Union(ts) => MonoType::Union(
            ts.iter()
                .map(|t| expand_ctor_applications(t, defs))
                .collect(),
        ),
        MonoType::Intersection(ts) => MonoType::Intersection(
            ts.iter()
                .map(|t| expand_ctor_applications(t, defs))
                .collect(),
        ),
        _ => ty.clone(),
    }
}

/// 泛型类型的实例化产物：声明体用了构造器变量头部时，把产物里的具体应用
/// 展开成记录形态（见 expand_ctor_applications）；否则原样返回。
pub(super) fn instantiate_with_ctor_heads(
    inst_ty: MonoType,
    def: &crate::frontend::core::sema::environment::GenericTypeDef,
    defs: &HashMap<String, crate::frontend::core::sema::environment::GenericTypeDef>,
) -> MonoType {
    if body_uses_ctor_head(&def.poly.body, &def.type_param_names) {
        expand_ctor_applications(&inst_ty, defs)
    } else {
        inst_ty
    }
}

/// #287: 将泛型构造器字段类型中的 TypeRef(类型参数名) 替换为对应 TypeVar，
/// 供 unify 推断类型参数的具体值。非参数 TypeRef 原样保留。
///
/// 两步合成（复审轮 1 F2）：先做 TypeRef/变量结构遍历，再对结果按
/// 「参数名 → 构造器名」改头部——头部改名只有共享实现
/// `types::substitute::substitute_ctor_heads` 一处，本函数不再内联。
fn substitute_type_params_with_vars(
    ty: &MonoType,
    param_vars: &HashMap<String, MonoType>,
) -> MonoType {
    // 绑定值是构造器引用（`F := Box` / `F := Vec`）的参数名才进改名表；
    // 普通类型参数绑定具体类型时头部不动（等价：内联版也只在 `ctor_head_name`
    // 命中时改名）。
    let head_map: HashMap<String, String> = param_vars
        .iter()
        .filter_map(|(name, bound)| {
            crate::frontend::core::types::substitute::ctor_head_name(bound)
                .map(|head| (name.clone(), head))
        })
        .collect();
    let walked = substitute_type_params_with_vars_walk(ty, param_vars);
    crate::frontend::core::types::substitute::substitute_ctor_heads(&walked, &head_map)
}

/// `substitute_type_params_with_vars` 的结构遍历体：只替换 TypeRef/变量位，
/// 头部改名由上面的共享实现负责（同一规则不再有第三份内联）。
fn substitute_type_params_with_vars_walk(
    ty: &MonoType,
    param_vars: &HashMap<String, MonoType>,
) -> MonoType {
    match ty {
        MonoType::TypeRef(name) => param_vars.get(name).cloned().unwrap_or_else(|| ty.clone()),
        MonoType::Struct(s) => MonoType::Struct(crate::frontend::core::types::mono::StructType {
            fields: s
                .fields
                .iter()
                .map(
                    |field_info| crate::frontend::core::types::mono::StructFieldInfo {
                        ty: substitute_type_params_with_vars_walk(&field_info.ty, param_vars),
                        ..field_info.clone()
                    },
                )
                .collect(),
            ..s.clone()
        }),
        MonoType::Generic { name, args } => MonoType::Generic {
            name: name.clone(),
            args: args
                .iter()
                .map(|a| substitute_type_params_with_vars_walk(a, param_vars))
                .collect(),
        },
        MonoType::Fn {
            params,
            return_type,
        } => MonoType::Fn {
            params: params
                .iter()
                .map(|p| substitute_type_params_with_vars_walk(p, param_vars))
                .collect(),
            return_type: Box::new(substitute_type_params_with_vars_walk(
                return_type,
                param_vars,
            )),
        },
        MonoType::Ref { mutable, inner } => MonoType::Ref {
            mutable: *mutable,
            inner: Box::new(substitute_type_params_with_vars_walk(inner, param_vars)),
        },
        MonoType::Union(v) => MonoType::Union(
            v.iter()
                .map(|t| substitute_type_params_with_vars_walk(t, param_vars))
                .collect(),
        ),
        MonoType::Intersection(v) => MonoType::Intersection(
            v.iter()
                .map(|t| substitute_type_params_with_vars_walk(t, param_vars))
                .collect(),
        ),
        _ => ty.clone(),
    }
}

/// 从类型构造实参表达式提取具体类型：类型名在表达式位置 infer 成 MetaType 空壳
/// （不存具体类型名），实例化泛型类型前需从 AST 名称解包成具体 MonoType。
fn concrete_type_from_expr_arg(
    expr: &crate::ast::Expr,
    type_defs: &HashMap<String, PolyType>,
    generic_type_defs: &HashMap<String, crate::frontend::core::sema::environment::GenericTypeDef>,
) -> Option<MonoType> {
    match expr {
        crate::ast::Expr::Var(name, _) => {
            if is_builtin_type_name(name) {
                // 内置**泛型**容器名（Vec / Array）是类型构造器：值位置是 Type
                // 宇宙的值，类型实参位要的是构造器名（TypeRef）而非具体标量类型。
                MonoType::from_builtin_name(name).or_else(|| {
                    is_builtin_generic_type_name(name).then(|| MonoType::TypeRef(name.clone()))
                })
            } else if generic_type_defs.contains_key(name) {
                // 泛型类型名（未实例化引用）作类型实参是**构造器引用**：
                // `Holder(Box, Int)` 的 `Box` 要传构造器名（TypeRef），不是它
                // 构造出的记录形态（记录形态会丢头部，无法再做构造器变量替换）。
                // 泛型类型名同时登记在类型表里，故这一支必须排在 type_defs 之前。
                Some(MonoType::TypeRef(name.clone()))
            } else {
                type_defs.get(name).map(|p| p.body.clone())
            }
        }
        _ => None,
    }
}

/// RFC-010b: 模式标签（诊断显示用，typecheck 侧本地版本）
fn local_pattern_label(p: &crate::ast::Pattern) -> String {
    use crate::ast::Pattern;
    use crate::frontend::core::lexer::tokens::Literal;
    match p {
        Pattern::Wildcard => "_".to_string(),
        Pattern::Identifier(n) => n.clone(),
        Pattern::Literal(l) => format!("{:?}", l),
        Pattern::Tuple(_) => "tuple".to_string(),
        Pattern::Struct { name, .. } => name.clone(),
        Pattern::Union { variant, .. } => variant.clone(),
        Pattern::Or(_) => "or".to_string(),
        // issue #405：范围臂标签还原源码写法（Int 域负端点直出负号）
        Pattern::Range {
            lo: Literal::Int(lo),
            hi: Literal::Int(hi),
        } => format!("{}..{}", lo, hi),
        Pattern::Range { .. } => "range".to_string(),
        Pattern::Guard { .. } => "guard".to_string(),
    }
}
