//! 除调用裁决族以外的全部方法（WBS 7.0.4 甲案）。
//!
//! 方法体与文档注释逐字搬自原 `inference/expressions.rs`，语义未改；仅补跨模块可见性
//! （`pub(super)`）与相对路径（`super::` 改指 crate 路径）。调用裁决族见 `super::call`。

use crate::ast::{BinOp, Literal, UnOp};
use crate::frontend::core::sema::environment::MethodOverloadCandidates;
use crate::frontend::core::sema::import_face::ImportFace;
use crate::frontend::core::sema::judgement::decide;
use crate::frontend::core::types::identity::DefPath;
use crate::frontend::core::types::{MonoType, PolyType, TypeConstraintSolver};
use crate::util::diagnostic::{ErrorCodeDefinition, Result};
use std::collections::{HashMap, HashSet};

use super::ExpressionInferrer;
use super::{
    extract_namespace_path, is_builtin_type_name, local_pattern_label, never_short_circuit,
    read_view, NeverShortCircuit, E1002_ARITH_EXPECTED,
};
use crate::frontend::core::sema::inference::existential;
use crate::frontend::core::sema::inference::mono_type_is_witness_bearing;
use crate::frontend::core::sema::inference::statements;
use crate::frontend::core::sema::operator_interfaces;
use crate::frontend::core::types::identity::Nominal;

impl<'a> ExpressionInferrer<'a> {
    /// 获取求解器引用（可变）
    pub fn solver(&mut self) -> &mut TypeConstraintSolver {
        self.solver
    }

    /// #321 W1003：注入导入名监视集（StatementChecker 委托表达式检查前调用）
    pub fn set_import_watch(
        &mut self,
        names: &HashMap<String, String>,
    ) {
        self.import_watch
            .extend(names.iter().map(|(k, v)| (k.clone(), v.clone())));
    }

    /// #321 W1003：取走已使用名集合（委托方回收合并）
    pub fn take_import_used(&mut self) -> HashSet<String> {
        std::mem::take(&mut self.imported_used)
    }

    /// 注入模块别名集合（#396）：FieldAccess 在模块别名上取成员失败时按
    /// 模块语义报 E1043，而非 struct 语义的 E1042。
    pub fn set_module_aliases(
        &mut self,
        aliases: &'a std::collections::HashSet<String>,
    ) {
        self.module_aliases = Some(aliases);
    }

    /// 注入模块别名 → 模块限定键表（限定调用的实例化请求用，见字段注释）。
    pub fn set_module_namespaces(
        &mut self,
        namespaces: &'a HashMap<String, String>,
    ) {
        self.module_namespaces = Some(namespaces);
    }

    /// #321 W1003：变量成功解析时调用——命中监视集的名字记为已使用
    fn note_import_use(
        &mut self,
        name: &str,
    ) {
        if let Some(report_as) = self.import_watch.get(name) {
            let report_as = report_as.clone();
            self.imported_used.insert(report_as);
        }
    }

    /// RFC-011a §6：在"具体→存在"兼容判定通过的位置收集包装点。
    /// 成员违规（具体类型未实现接口）以 E1101 返回。
    pub(crate) fn collect_existential_coercions(
        &mut self,
        expr: &crate::ast::Expr,
        expected: &MonoType,
    ) -> Result<()> {
        let (mut coercions, errors) = existential::collect_existential_coercions(
            self.solver,
            self.scope,
            self.type_defs,
            self.generic_type_defs,
            expr,
            expected,
        );
        if let Some(err) = errors.into_iter().next() {
            return Err(err);
        }
        self.existential_coercions.append(&mut coercions);
        Ok(())
    }

    /// 设置方法绑定表（键是方法身份）
    pub fn set_method_bindings(
        &mut self,
        bindings: &'a HashMap<DefPath, MonoType>,
    ) {
        self.method_bindings = bindings;
    }

    /// 设置导入面（D-5）：方法身份的解析只经它
    fn mono_from_annotation(
        &self,
        ty: &crate::ast::Type,
    ) -> MonoType {
        match self.import_face {
            Some(face) => face.mono_from_annotation(ty),
            None => MonoType::from(ty.clone()),
        }
    }

    pub fn set_import_face(
        &mut self,
        face: &'a ImportFace,
    ) {
        self.import_face = Some(face);
    }

    /// 写出类型名 → 类型身份：委托导入面的**唯一实现**（D64）。
    ///
    /// 只有经 TypeDefinition 声明的类型解析出身份；幻名/类型族/内建/类型参数
    /// 返回 None（调用方保持平路径）。
    pub(super) fn type_identity(
        &self,
        written: &str,
    ) -> Option<crate::frontend::core::types::identity::Nominal> {
        self.import_face?.type_identity(written)
    }

    /// 把刚由**写出名**构造出的平路径类型升格为身份（构造器/变体/枚举路径）。
    ///
    /// 与 [`Self::mono_from_annotation`] 同一个解析器：构造器产生的类型名与
    /// 注解侧的类型名必须成对注入，否则 `T(...)` 与 `x: T` 在判等点分属
    /// 平路径与身份、合法程序被误拒。
    pub(super) fn resolve_identity(
        &self,
        ty: MonoType,
    ) -> MonoType {
        match self.import_face {
            Some(face) => {
                crate::frontend::core::types::substitute::resolve_nominal_identities(&ty, &|w| {
                    face.type_identity(w)
                })
            }
            None => ty,
        }
    }

    /// 写出名 → 名义类型（解析不到保持平路径）：查表/判等前的统一形状。
    pub(super) fn nominal_for(
        &self,
        written: &str,
    ) -> crate::frontend::core::types::identity::Nominal {
        self.type_identity(written).unwrap_or_else(|| {
            crate::frontend::core::types::identity::Nominal::flat(written)
                .expect("type name must be bare and non-empty")
        })
    }

    /// 名义类型 → `env.types` 的本地表项（L3-1b S4 的唯一消费入口）。
    ///
    /// 平路径（内建/类型参数/单文件类型）本地键就是裸名——与今天同一条事实；
    /// 带模块身份查派生索引（身份→本地键）。**带模块但没有索引项 = 查不到**，
    /// 不回落到裸名：同名不同模块的两个类型正是在裸名上重合的。
    pub(super) fn type_def(
        &self,
        nominal: &crate::frontend::core::types::identity::Nominal,
    ) -> Option<&'a PolyType> {
        let defs = self.type_defs;
        if nominal.module().is_empty() {
            return defs.get(nominal.bare());
        }
        let key = self.type_identity_index?.get(nominal)?;
        defs.get(key.as_str())
    }

    /// 名义类型 → 泛型类型定义模板（与 [`Self::type_def`] 同一本地键口径）。
    pub(super) fn generic_def(
        &self,
        nominal: &crate::frontend::core::types::identity::Nominal,
    ) -> Option<&'a crate::frontend::core::sema::environment::GenericTypeDef> {
        let defs = self.generic_type_defs;
        if nominal.module().is_empty() {
            return defs.get(nominal.bare());
        }
        let key = self.type_identity_index?.get(nominal)?;
        defs.get(key.as_str())
    }

    /// 名义类型 → 和类型变体表（与 [`Self::type_def`] 同一本地键口径）。
    pub(super) fn sum_variants(
        &self,
        nominal: &crate::frontend::core::types::identity::Nominal,
    ) -> Option<&'a Vec<crate::frontend::core::sema::environment::SumVariantDef>> {
        let defs = self.sum_types;
        if nominal.module().is_empty() {
            return defs.get(nominal.bare());
        }
        let key = self.type_identity_index?.get(nominal)?;
        defs.get(key.as_str())
    }

    /// 方法身份的解析入口：委托导入面的**唯一实现**（不在这里另判一份）
    pub fn method_key(
        &self,
        written_type: &str,
        method: &str,
    ) -> Option<DefPath> {
        self.import_face?.method_key(written_type, method)
    }

    /// Nominal→方法键：委托导入面的唯一实现；仅由 receiver_method_key 使用。
    fn method_key_for_nominal(
        &self,
        nominal: &crate::frontend::core::types::identity::Nominal,
        method: &str,
    ) -> Option<DefPath> {
        self.import_face?.method_key_for_nominal(nominal, method)
    }

    /// 方法键：**接收者身份优先**（D-6），否则按静态类型名解析（D-5）。
    ///
    /// 为什么需要这一档：跨单元**同名类型**（`a.Temp` / `b.Temp`）的接收者在
    /// 类型层面都是 `Struct{name:"Temp"}`，静态类型名在导入面里是歧义名；只有
    /// 绑定登记时写出的名字（`TempA` / `TempB`）能区分它们。非 `Var` 接收者
    /// 没有绑定表可查，退回静态类型名（不猜）。
    fn receiver_method_key(
        &self,
        receiver: &crate::ast::Expr,
        nominal: &crate::frontend::core::types::identity::Nominal,
        method: &str,
    ) -> Option<DefPath> {
        if let crate::ast::Expr::Var(name, _) = receiver {
            if let Some(identity) = self.scope.type_identity(name) {
                return Some(DefPath::method(&identity.module, &identity.name, method));
            }
        }
        // 身份优先：模块非空时直接用 (module, 定义名) 取路径；module 为空（flat/
        // 单文件/幻名）必须回落到字符串路径——那里还承担 `is_conflicted` 拒绝面。
        if !nominal.module().is_empty() {
            if let Some(key) = self.method_key_for_nominal(nominal, method) {
                return Some(key);
            }
        }
        self.method_key(nominal.bare(), method)
    }

    /// 该身份下的候选签名（与会话内的形参名组同序）
    pub(super) fn overload_candidates(
        &self,
        key: &DefPath,
    ) -> Option<&'a Vec<MonoType>> {
        self.method_overloads
            .and_then(|m| m.get(key))
            .map(|group| &group.candidates)
    }

    /// 该身份下的声明期形参名组（缺表 = 名字不可用，不按名匹配）
    pub(super) fn overload_param_names(
        &self,
        key: &DefPath,
    ) -> Option<&'a Vec<Vec<String>>> {
        self.method_overloads
            .and_then(|m| m.get(key))
            .map(|group| &group.param_names)
    }

    /// RFC-011b: 注入接口实现登记表（运算符查询的唯一判据）
    pub fn set_interface_impl_registry(
        &mut self,
        registry: &'a HashMap<
            String,
            Vec<crate::frontend::core::sema::environment::InterfaceImplEntry>,
        >,
    ) {
        self.interface_impl_registry = registry;
    }

    /// RFC-010: 注入和类型登记表
    pub fn set_sum_types(
        &mut self,
        sum_types: &'a HashMap<
            String,
            Vec<crate::frontend::core::sema::environment::SumVariantDef>,
        >,
    ) {
        self.sum_types = sum_types;
    }

    /// 设置类型定义表
    pub fn set_type_defs(
        &mut self,
        defs: &'a HashMap<String, PolyType>,
    ) {
        self.type_defs = defs;
    }

    /// 注入 `env.types` 的派生索引（身份→本地键）；与 `set_type_defs` 同源，
    /// 由 StatementChecker 的 macro 借用传入。本批不查询，S4 的字段/方法解析用。
    pub fn set_type_identity_index(
        &mut self,
        index: &'a HashMap<crate::frontend::core::types::identity::Nominal, String>,
    ) {
        self.type_identity_index = Some(index);
    }

    /// 派生索引只读访问（S3 建面，S4 查询）。
    #[allow(dead_code)]
    pub fn type_identity_index(
        &self
    ) -> Option<&'a HashMap<crate::frontend::core::types::identity::Nominal, String>> {
        self.type_identity_index
    }

    /// 设置泛型类型定义表
    pub fn set_generic_type_defs(
        &mut self,
        defs: &'a HashMap<String, crate::frontend::core::sema::environment::GenericTypeDef>,
    ) {
        self.generic_type_defs = defs;
    }

    /// 设置证明函数基类型表（RFC-027 Phase 2.5 / #523）。
    /// StatementChecker 随委托注入——与注册期 `proof_fn_bases` 同一张表，
    /// 剥精化判据因此单源（见 `strip_refined_with_proof_fns`）。
    pub fn set_proof_fn_bases(
        &mut self,
        bases: &'a HashMap<String, MonoType>,
    ) {
        self.proof_fn_bases = bases;
    }

    /// 设置 native 导出形参名表（#454）
    pub fn set_native_param_names(
        &mut self,
        names: &'a HashMap<String, crate::frontend::core::sema::environment::ExportParamInfo>,
    ) {
        self.native_param_names = names;
    }

    /// 设置调用点形参名表（E1113 的槽位判据，见字段文档）
    pub fn set_declared_call_site_params(
        &mut self,
        params: &'a HashMap<String, crate::frontend::core::sema::checker::CallSiteSlots>,
    ) {
        self.declared_call_site_params = params;
    }

    /// 设置 native arity 区间表（#387）
    pub fn set_native_arities(
        &mut self,
        arities: &'a HashMap<String, (usize, Option<usize>)>,
    ) {
        self.native_arity = arities;
    }

    /// RFC-011b：当前函数的约束形参（定义体延迟派发）
    pub fn set_current_fn_param_constraints(
        &mut self,
        constraints: Option<&'a HashMap<String, String>>,
    ) {
        self.current_fn_param_constraints = constraints;
    }

    /// RFC-011a §3：方法重载候选（候选签名 + 声明期形参名同组）
    pub fn set_method_overloads(
        &mut self,
        overloads: &'a HashMap<DefPath, MethodOverloadCandidates>,
    ) {
        self.method_overloads = Some(overloads);
    }

    /// 注入当前函数**声明注解**的位置（#353 口径）：return 值类型不符的诊断
    /// 指向注解，与语句形态 check_return_value_type 同一落点（WBS 6.8.2a D54 ②）。
    pub fn set_return_annotation_span(
        &mut self,
        span: Option<crate::util::span::Span>,
    ) {
        self.return_annotation_span = span;
    }

    /// 期望类型（方法重载按期望返回决议）
    pub fn set_current_expected(
        &mut self,
        expected: Option<MonoType>,
    ) {
        self.current_expected = expected;
    }

    /// 注入泛型函数的声明序类型参数名表（调用点做类型参数替换的依据）。
    pub fn set_generic_fn_type_params(
        &mut self,
        params: &'a HashMap<String, Vec<String>>,
    ) {
        self.generic_fn_type_params = params;
    }

    /// 设置依赖类型环境（效应查询）
    pub fn set_dep_env(
        &mut self,
        dep_env: &'a crate::frontend::core::types::eval::dependent_types::DependentTypeEnv,
    ) {
        self.dep_env = Some(dep_env);
    }

    /// 设置流敏感假设集 Γ（效应注入）
    pub fn set_gamma(
        &mut self,
        gamma: &'a mut crate::frontend::core::sema::proof::assumptions::FlowSensitiveGamma,
    ) {
        self.gamma = Some(gamma);
    }

    /// 添加变量到当前作用域
    pub fn add_var(
        &mut self,
        name: String,
        poly: PolyType,
        is_mut: bool,
    ) {
        self.scope
            .add_var(name, poly, is_mut, crate::util::span::Span::default());
    }

    /// 添加参数（lambda 参数，lambda 体可继承）
    pub fn add_param(
        &mut self,
        name: String,
        poly: PolyType,
        is_mut: bool,
        definition_span: crate::util::span::Span,
    ) {
        self.scope.add_param(name, poly, is_mut, definition_span);
    }

    /// 检查变量是否存在于任何作用域中
    pub fn var_exists_in_any_scope(
        &self,
        name: &str,
    ) -> bool {
        self.scope.var_in_any_scope(name)
    }

    /// 尝试添加变量到当前作用域
    ///
    /// `span` 转发给 `definition_span`（#433/D55）：for/推导式等表达层级
    /// 绑定的定义事件需要真实声明位置，跳转定义靠它落 `DefId`。
    pub fn try_add_var(
        &mut self,
        name: String,
        poly: PolyType,
        span: crate::util::span::Span,
        is_mut: bool,
    ) -> Result<()> {
        self.scope.add_var(name, poly, is_mut, span);
        Ok(())
    }

    /// 检查变量是否存在于当前作用域
    pub fn var_exists_in_current_scope(
        &self,
        name: &str,
    ) -> bool {
        self.scope.var_in_current_scope(name)
    }

    /// 获取变量（从最内层作用域开始查找）
    pub fn get_var(
        &self,
        name: &str,
    ) -> Option<&PolyType> {
        self.scope.get_var(name)
    }

    /// 获取所有变量（从所有作用域）
    pub fn get_all_vars(&self) -> HashMap<String, PolyType> {
        self.scope.vars()
    }

    /// 变量赋值操作 - 统一处理变量赋值并写回 scope
    ///
    /// 统一变量类型并写回 scope，确保后续类型推断能获取最新类型。
    /// 如果变量不存在，则创建新变量。
    /// 这是修复 for 循环等场景类型丢失的关键方法。
    /// 关键：直接使用右侧表达式的类型（new_ty），而不是依赖 solver.resolve()。
    ///
    /// `enforce_unify`：右值是否必须与变量当前类型统一（与带注解初值同口径，
    /// 失败报 E1002）。`x = <右值>` 形式的重赋值必须为 true——此前直接覆写导致
    /// 类型可变（Int 变量可赋 String），漏检到运行时 E6007 才爆（工作流验证 Bug 1）。
    /// false 仅用于无初值注解绑定的占位覆写路径（顶层预注册占位类型随后被
    /// 真实注解类型覆写，覆写是该流程的承重墙，见 rfc011 编译期求值测试）。
    pub fn assign_var(
        &mut self,
        name: &str,
        new_ty: crate::frontend::core::types::MonoType,
        span: crate::util::span::Span,
        enforce_unify: bool,
    ) -> Result<()> {
        if enforce_unify {
            if let Some(poly) = self.scope.get_var(name) {
                let declared = poly.body.clone();
                if let Err(failure) = decide(
                    self.solver,
                    &new_ty,
                    &declared,
                    Some(span),
                    self.type_defs,
                    self.dep_env,
                ) {
                    return Err(failure.or_site(
                        ErrorCodeDefinition::type_mismatch(
                            &format!("{}", declared),
                            &format!("{}", new_ty),
                        )
                        .at(span)
                        .build(),
                    ));
                }
            }
        }
        // 直接使用右侧表达式的类型更新变量（变量不存在则新建，保持原行为）
        // 注意：new_ty 已经是解析后的正确类型（如 List<Int>），不需要额外 resolve
        self.scope
            .update_var(name, crate::frontend::core::types::PolyType::mono(new_ty));
        Ok(())
    }

    /// 退出循环作用域时，将内部声明的变量提升到外层作用域
    ///
    /// 解决循环退出后变量丢失的问题，确保 IR 生成阶段能获取变量类型。
    fn promote_loop_vars_to_parent_scope(&mut self) {
        let current_scope_vars = self.scope.current_scope_vars();

        // 退出当前 scope
        self.scope.exit_block();

        // 将循环内声明的变量添加到外层 scope，保留可变性
        for (name, info) in current_scope_vars {
            self.scope.add_var(
                name,
                info.poly,
                info.is_mut,
                crate::util::span::Span::default(),
            );
        }
    }

    /// 进入新的作用域
    pub fn enter_scope(&mut self) {
        self.scope.enter_block();
    }

    /// 退出当前作用域
    pub fn exit_scope(&mut self) {
        self.scope.exit_block();
    }

    /// 获取当前作用域层级
    pub fn scope_level(&self) -> usize {
        self.scope.scope_level()
    }

    /// #311：语句检查器（StatementChecker）自己递归走 for 体，委托表达式检查时
    /// 把 checker 侧循环深度传入，保证 E1102 判定跨两个 walker 一致
    pub fn set_loop_depth(
        &mut self,
        depth: usize,
    ) {
        self.loop_depth = depth;
    }

    /// 语句侧镜像 set_loop_depth：委托表达式检查时把函数边界深度传入
    /// （命名函数体内 is_top_level=false → 1，模块初始化层 → 0），
    /// 保证 E1109（return 函数外）判定跨两个 walker 同源
    pub fn set_fn_depth(
        &mut self,
        depth: usize,
    ) {
        self.fn_depth = depth;
    }

    /// 推断字面量表达式类型
    pub fn infer_literal(
        &mut self,
        lit: &crate::frontend::core::lexer::tokens::Literal,
    ) -> Result<MonoType> {
        // 6.8.7：期望类型驱动的字面量定型——单值类型（int64::3）要有居民。
        //
        // 判据只有一条：当前期望类型是 Literal{v} 且本字面量取值等于 v。
        // 否则（无期望 / 期望不是单值类型 / 值不等）仍取基类型——值不等的情形
        // 由后续形参/标注对照按单值类型报 E1002，不在这里改判。
        if let Some(expected) = self.current_expected.clone() {
            if let Some(ty) = Self::literal_matches_single_value_type(
                lit,
                Some(&self.solver.resolve_type(&expected)),
            ) {
                return Ok(ty);
            }
        }
        let ty = match lit {
            crate::frontend::core::lexer::tokens::Literal::Int(_) => MonoType::Int(64),
            crate::frontend::core::lexer::tokens::Literal::Float(_) => MonoType::Float(64),
            crate::frontend::core::lexer::tokens::Literal::Bool(_) => MonoType::Bool,
            crate::frontend::core::lexer::tokens::Literal::Char(_) => MonoType::Char,
            crate::frontend::core::lexer::tokens::Literal::String(_) => MonoType::make_string(),
            crate::frontend::core::lexer::tokens::Literal::Void => MonoType::Void,
        };
        Ok(ty)
    }

    /// #300 I 项：Range 构造的类型推断（表达式级，能看到嵌套与字面量）
    ///
    /// - `a..b`：Range(Int)，元素类型必须是 Int（for 逐次 +1、in 界推理都要求整数域）
    /// - `(a..b)..c`：step 形态，c 必须是 Int；字面量 0 编译期拒绝（Never 不可居留）；
    ///   动态 step 的零检查在 ir_gen 生成运行时断言（未来错误系统落地后升格 Result，#301）
    /// - 左操作数已是 Range 而右侧再嵌套 Range（`a..b..c..d`）：拒绝，区间是三分量构造
    fn infer_range_expr(
        &mut self,
        left: &crate::ast::Expr,
        right: &crate::ast::Expr,
        span: crate::util::span::Span,
    ) -> Result<MonoType> {
        use crate::ast::Expr;
        use crate::frontend::core::lexer::tokens::Literal;

        let left_ty = self.infer_expr(left)?;
        let right_ty = self.infer_expr(right)?;

        if left_ty.is_range() {
            // step 形态：右侧是步长
            if right_ty.is_range() {
                return Err(ErrorCodeDefinition::type_mismatch(
                    "Int (range step)",
                    &format!("{right_ty}"),
                )
                .at(span)
                .build());
            }
            if let Expr::Lit(Literal::Int(0), _) = right {
                return Err(ErrorCodeDefinition::type_mismatch(
                    "non-zero Int (range step)",
                    "int64::0",
                )
                .at(span)
                .build());
            }
            return match right_ty {
                MonoType::Int(_) => Ok(left_ty),
                _ => Err(
                    ErrorCodeDefinition::type_mismatch("Int", &format!("{right_ty}"))
                        .at(span)
                        .build(),
                ),
            };
        }

        // 基础形态：两端必须 Int
        match (left_ty, right_ty) {
            (MonoType::Int(_), MonoType::Int(_)) => Ok(MonoType::Generic {
                name: Nominal::flat(AsRef::<str>::as_ref(&("Range")))
                    .expect("type name must be bare and non-empty"),
                args: vec![MonoType::Int(64)],
            }),
            (l, r) => Err(ErrorCodeDefinition::type_mismatch(
                "Int range bounds",
                &format!("{l}..{r}"),
            )
            .at(span)
            .build()),
        }
    }

    /// 推断二元操作符表达式类型
    ///
    /// RFC-011b: 算术运算双路——白名单快路径（含 Int~Float 混合 widening）
    /// 先行，miss 后查接口实现登记表：native 条目取登记结果类型（如
    /// `Add(Int, Float, Float)`），用户条目额外记录 OperatorDispatch 供
    /// ir_gen 派发方法调用。都 miss 报 E1002 并提示需实现对应接口。
    pub fn infer_binary(
        &mut self,
        op: &BinOp,
        left: &MonoType,
        right: &MonoType,
        span: crate::util::span::Span,
        receiver: &crate::ast::Expr,
    ) -> Result<MonoType> {
        // 底部类型 vacuous（issue #469 处 3）：绑定失败毒化登记（`Never` 占位）
        // 或不可达表达式里的运算不施加任何类型约束——unify 的爆炸原理
        // （`solver.rs` 的 `Never <: T`）在运算位的同款延伸。毒化的引用参与
        // 算术/比较时必须静默通过，否则 E1001 消了、E1002 又冒出来。
        // 判据收敛为共享纯函数 `never_short_circuit`（issue #509 残留④，与
        // StatementChecker 二元运算臂同源）；结果取另一侧的具体形态，两侧都
        // 是 Never 时无信息可依，由调用方发 fresh var 推迟。
        if let Some(verdict) = never_short_circuit(left, right) {
            return Ok(match verdict {
                NeverShortCircuit::Both => self.solver.new_var(),
                NeverShortCircuit::TakeRight => right.clone(),
                NeverShortCircuit::TakeLeft => left.clone(),
            });
        }
        match op {
            BinOp::Add | BinOp::Sub | BinOp::Mul | BinOp::Div => {
                // RFC-009 读透明：借用值参与算术按 inner 类型判定（读穿透）
                let l = read_view(left);
                let r = read_view(right);
                if let (MonoType::Int(_), MonoType::Int(_)) = (&l, &r) {
                    Ok(l)
                } else if let (MonoType::Float(_), MonoType::Float(_)) = (&l, &r) {
                    Ok(l)
                } else if let (MonoType::Int(_), MonoType::Float(_))
                | (MonoType::Float(_), MonoType::Int(_)) = (&l, &r)
                {
                    // RFC-011b：Int~Float 混合 widening（native 登记
                    // Add(Int, Float, Float) 的快路径），结果恒 Float
                    Ok(MonoType::Float(64))
                } else if l.is_string() && r.is_string() {
                    Ok(MonoType::make_string())
                } else if l.is_list() && r.is_list() {
                    let left_elem = &l.generic_args().expect("List args")[0];
                    let right_elem = &r.generic_args().expect("List args")[0];
                    let _ = self.solver.unify(left_elem, right_elem);
                    let elem_ty = self.solver.resolve_type(left_elem);
                    Ok(MonoType::make_list(elem_ty))
                } else {
                    // 未绑定类型变量（无标注 lambda 参数等）：把变量与**另一侧的
                    // 具体类型**统一，结果取该具体类型。这样 `x * 2` 里
                    // `x: TypeVar` 会被绑成 `Int`，`fn(x: Int) -> Int` 的返回类型
                    // 得以定型（调用方 `map(.., f: (item: T) -> R)` 的 `R` 才能解出）。
                    //
                    // 仅当**两侧都是**未绑定变量时才真的无法判定，返回 fresh 推迟
                    // ——此时无具体类型可依，强行 unify 会把两个独立参数错误地
                    // 绑在一起（破坏 `fn(Any, Any)` 的独立性）。
                    let (lv, rv) = (
                        matches!(left, MonoType::TypeVar(_)),
                        matches!(right, MonoType::TypeVar(_)),
                    );
                    if lv && !rv {
                        let _ = self.solver.unify(left, right);
                        return Ok(self.solver.resolve_type(right));
                    }
                    if rv && !lv {
                        let _ = self.solver.unify(left, right);
                        return Ok(self.solver.resolve_type(left));
                    }
                    if lv && rv {
                        return Ok(self.solver.new_var());
                    }
                    // RFC-011b：登记表查询（用户实例化的运算符重载）
                    if let Some(result) = self.query_operator_dispatch(op, &l, &r, span, receiver) {
                        return Ok(result);
                    }
                    // 类型层不认的组合宁拒不静默：fresh var 兜底会让
                    // `1 + "a"` 纸面通过、运行期错译（#271 同款纪律）
                    Err(ErrorCodeDefinition::type_mismatch(
                        // 接受集文面与 StatementChecker 同源（issue #509 残留③，
                        // 基文见 expressions::E1002_ARITH_EXPECTED）；本臂 miss
                        // 后仍查接口派发，保留既有后缀
                        &format!("{E1002_ARITH_EXPECTED}或实现对应算术接口"),
                        &format!("{l} 与 {r}"),
                    )
                    .build())
                }
            }
            BinOp::Mod => {
                let l = read_view(left);
                let r = read_view(right);
                if let (MonoType::Int(_), MonoType::Int(_)) = (&l, &r) {
                    Ok(l)
                } else if let (MonoType::Float(_), MonoType::Float(_)) = (&l, &r) {
                    Ok(l)
                } else if let (MonoType::Int(_), MonoType::Float(_))
                | (MonoType::Float(_), MonoType::Int(_)) = (&l, &r)
                {
                    // RFC-011b：混合 widening，结果恒 Float
                    Ok(MonoType::Float(64))
                } else if matches!(left, MonoType::TypeVar(_))
                    || matches!(right, MonoType::TypeVar(_))
                {
                    // 未绑定类型变量延后判定（同 Add/Sub/Mul/Div 臂注释）
                    Ok(self.solver.new_var())
                } else {
                    // RFC-011b：登记表查询（用户实例化的 % 重载）
                    if let Some(result) = self.query_operator_dispatch(op, &l, &r, span, receiver) {
                        return Ok(result);
                    }
                    Err(ErrorCodeDefinition::type_mismatch(
                        // #509 残留③同族登记：Mod 的白名单接受集与 Add 族不同
                        // （Int/Float，无 String/List），不与 E1002_ARITH_EXPECTED
                        // 收敛——同一常量承载两个接受集才是错。文面单点，无拷贝。
                        "Int/Float（两侧同型）或实现 Modulo 接口",
                        &format!("{l} 与 {r}"),
                    )
                    .build())
                }
            }
            BinOp::Eq | BinOp::Neq | BinOp::Lt | BinOp::Le | BinOp::Gt | BinOp::Ge => {
                // 比较两侧必须同型：`unify` 失败即报错，不再丢弃。
                //
                // 此前 `let _ = unify(...)` 吞掉错误，`1 < "x"` / `1 < 2.5`
                // 编译期放行、运行期报 E6007；而常量折叠路径（ir_gen）却能
                // 折出结果——同一表达式写在顶层能折、写在函数体内就炸（B9）。
                // RFC-011b 只承诺混合**算术**（Add/Sub/Mul/Div），比较保持
                // 「两侧同型」的原生快路径，故此处按纪律拒绝。
                if let Err(failure) = decide(
                    self.solver,
                    left,
                    right,
                    Some(span),
                    self.type_defs,
                    self.dep_env,
                ) {
                    return Err(failure.or_site(
                        ErrorCodeDefinition::type_mismatch(
                            &format!("{}", self.solver.resolve_type(left)),
                            &format!("{}", self.solver.resolve_type(right)),
                        )
                        .at(span)
                        .build(),
                    ));
                }
                Ok(MonoType::Bool)
            }
            BinOp::And | BinOp::Or => {
                if let (MonoType::Bool, MonoType::Bool) = (left, right) {
                    Ok(MonoType::Bool)
                } else {
                    Err(ErrorCodeDefinition::logical_operand_type_mismatch(
                        &format!("{}", left),
                        &format!("{}", right),
                    )
                    .build())
                }
            }
            BinOp::Range => {
                // #300 I 项：表达式级入口（infer_expr BinOp 臂）已拦截，此处仅为
                // 直接调用 infer_binary 的路径兜底——无表达式形状可查，按基础形态处理
                let elem_ty = if left == right {
                    left.clone()
                } else {
                    let _ = self.solver.unify(left, right);
                    left.clone()
                };
                Ok(MonoType::Generic {
                    name: Nominal::flat(AsRef::<str>::as_ref(&("Range")))
                        .expect("type name must be bare and non-empty"),
                    args: vec![elem_ty],
                })
            }
            // #285: 位运算/移位仅限 Int（SPEC §2.2 级 7/8）
            BinOp::BitAnd | BinOp::BitOr | BinOp::BitXor | BinOp::Shl | BinOp::Shr => {
                if let (MonoType::Int(_), MonoType::Int(_)) = (left, right) {
                    Ok(left.clone())
                } else {
                    Err(ErrorCodeDefinition::type_mismatch("Int", &format!("{}", left)).build())
                }
            }
            BinOp::Assign => Ok(MonoType::Void),
        }
    }

    /// RFC-010b: match 变体解构检查（和类型 scrutinee）。
    ///
    /// - 变体集按 scrutinee 的和类型消歧（裸变体名 `ok(v)` 无歧义）；
    /// - 载荷绑定进臂作用域（单载荷 Identifier；多载荷/复杂载荷暂 E3008）；
    /// - 穷尽性：Union 臂覆盖全部变体，或存在 Wildcard/Identifier 兜底臂，
    ///   否则 E1030（列出缺失变体）；兜底臂之后的臂、重复变体 → E1031；
    /// - 各臂 body 类型 unify 为 match 表达式的结果类型。
    fn check_match_expr(
        &mut self,
        scrutinee: &crate::ast::Expr,
        arms: &[crate::ast::MatchArm],
        match_span: crate::util::span::Span,
    ) -> Result<MonoType> {
        use crate::ast::Pattern;
        let scrutinee_ty = self.infer_expr(scrutinee)?;
        let scrutinee_ty = self.solver.resolve_type(&scrutinee_ty);
        let mut resolved = scrutinee_ty.clone();
        while let MonoType::Ref { inner, .. } = resolved {
            resolved = *inner;
        }
        let resolved = self.solver.resolve_type(&resolved);

        // #389：scrutinee 推断类型按 match 节点 span 落表——IR 生成期
        // generate_match_expr_ir 的 AST 猜测器（get_expr_mono_type）对
        // 调用/内联构造/字段访问形态返回 None，Union 模式解析随之失败
        // 被 #330 安全网误拦（E3008）；checker 的推断结果是唯一权威。
        self.match_scrutinee_types
            .insert(match_span, resolved.clone());

        // 和类型判定（Generic 形态 / scope 镜像 Struct 形态）
        let sum_name: Option<String> = match &resolved {
            MonoType::Generic { name, .. } if self.sum_variants(name).is_some() => {
                Some(name.clone().bare().to_string())
            }
            MonoType::Struct(st) if self.sum_variants(&st.name).is_some() => {
                Some(st.name.clone().bare().to_string())
            }
            // 非泛型和类型的方法体（`self: &Maybe`）：Ref 剥离后是 TypeRef，
            // 名义命中 sum_types 即和类型（经类型镜像复核）
            MonoType::TypeRef(n) => {
                if self.sum_variants(n).is_some() {
                    Some(n.clone().bare().to_string())
                } else {
                    match self.type_def(n).map(|p| &p.body) {
                        Some(MonoType::Struct(st)) if self.sum_variants(&st.name).is_some() => {
                            Some(st.name.bare().to_string())
                        }
                        _ => None,
                    }
                }
            }
            _ => None,
        };
        // RFC-010b：统一模式检查——非和类型 scrutinee 走同一条臂循环
        //（字面量/元组/结构体/或模式/守卫/标识符绑定全部可用），Union
        // 模式仅在和类型上有定义（check_pattern 内按类型自足解析）
        let sum_nominal = sum_name.as_ref().map(|n| self.nominal_for(n));
        let variants = sum_nominal
            .as_ref()
            .and_then(|n| self.sum_variants(n))
            .cloned()
            .unwrap_or_default();
        let param_names = sum_nominal
            .as_ref()
            .and_then(|n| self.generic_def(n))
            .map(|d| d.type_param_names.clone())
            .unwrap_or_default();
        let _ = (&variants, &param_names);
        let concrete = match &resolved {
            MonoType::Generic { args, .. } if sum_name.is_some() => args.clone(),
            _ => Vec::new(),
        };
        let _ = &concrete;

        let mut seen_variants: HashSet<String> = HashSet::new();
        let mut seen_literals: HashSet<String> = HashSet::new();
        // issue #405：范围臂的精确重复检测（同端点 ⇒ E1031）。重叠/包含
        // 分析（如 80..89 与 85）首版不做——E1031 的语义保持「精确重复」，
        // 区间覆盖证明登记为后续扩展。
        let mut seen_ranges: HashSet<(i128, i128)> = HashSet::new();
        let mut fallback_seen = false;
        let mut result_ty: Option<MonoType> = None;

        for arm in arms {
            // RFC-010b：守卫解包——`pat if cond => body`，绑定对守卫可见
            let (pat, guard): (&Pattern, Option<&crate::ast::Expr>) = match &arm.pattern {
                Pattern::Guard { pattern, condition } => (pattern, Some(condition)),
                p => (p, None),
            };

            // 兜底臂之后的任何臂都不可达（E1031）。兜底 = 无条件命中的
            // 不可驳斥模式（Identifier/Wildcard/Tuple/Struct，未带守卫）
            if fallback_seen {
                return Err(
                    ErrorCodeDefinition::unreachable_pattern(&local_pattern_label(pat))
                        .at(arm.span)
                        .build(),
                );
            }

            // 无守卫臂的重复可达性：变体重复、字面量重复 → E1031
            //（带守卫的重复臂可达——它是对前臂的细化，Rust 同规）
            if guard.is_none() {
                let alts: Vec<&Pattern> = match pat {
                    Pattern::Or(ps) => ps.iter().collect(),
                    p => vec![p],
                };
                for alt in alts {
                    match alt {
                        Pattern::Union { variant, .. } => {
                            if seen_variants.contains(variant) {
                                return Err(ErrorCodeDefinition::unreachable_pattern(variant)
                                    .at(arm.span)
                                    .build());
                            }
                        }
                        Pattern::Literal(lit) => {
                            let key = Self::literal_pattern_type(lit).to_string();
                            if !seen_literals.insert(format!("{key}:{lit:?}")) {
                                return Err(ErrorCodeDefinition::unreachable_pattern(
                                    &local_pattern_label(alt),
                                )
                                .at(arm.span)
                                .build());
                            }
                        }
                        // issue #405：同端点的重复范围臂不可达（E1031）。
                        // 端点已由 check_pattern 限定 Int——此处折叠 i128 判重；
                        // 非整数字面量端点走下方空臂（check_pattern 已先行报错）。
                        Pattern::Range {
                            lo: Literal::Int(lo),
                            hi: Literal::Int(hi),
                        } if !seen_ranges.insert((*lo, *hi)) => {
                            return Err(ErrorCodeDefinition::unreachable_pattern(
                                &local_pattern_label(alt),
                            )
                            .at(arm.span)
                            .build());
                        }
                        _ => {}
                    }
                }
            }

            self.scope.enter_block();
            let arm_ty = {
                let mut binds: Vec<(String, MonoType)> = Vec::new();
                let checked = match self.check_pattern_alts(pat, &resolved, &mut binds, arm.span) {
                    Err(e) => Err(e),
                    // 绑定先注册再查守卫——守卫表达式对模式绑定可见
                    Ok(()) => {
                        for (bn, bty) in &binds {
                            self.scope.add_var(
                                bn.clone(),
                                PolyType::mono(bty.clone()),
                                false,
                                arm.span,
                            );
                        }
                        Ok(())
                    }
                };
                match checked {
                    Err(e) => Err(e),
                    Ok(()) => {
                        // F-1（6.9.4 修复）：臂的模式绑定**遮蔽**同名外层绑定。Γ 的等式
                        // 读取口按**绑定身份**校验（`const_equalities_resolved` 逐名比对
                        // 注入时的 `VarInfo::definition_span`，与值环境同一条解析链），
                        // 故这里不需要额外动作：臂内的 `y` 解析到臂绑定，外层守卫事实
                        // 自然不产值；出臂后解析回外层绑定，事实照旧可用（不误杀）。
                        match guard {
                            Some(cond) => self.check_guard_condition(cond),
                            None => Ok(()),
                        }
                        .and_then(|()| self.infer_block(&arm.body, true, None))
                    }
                }
            };
            self.scope.exit_block();
            let arm_ty = arm_ty?;

            // 穷尽性记账：带守卫的臂不计入（守卫可能不命中，Rust 同规）；
            // Union 臂标记变体覆盖，不可驳斥臂视为兜底
            if guard.is_none() {
                let alts: Vec<&Pattern> = match pat {
                    Pattern::Or(ps) => ps.iter().collect(),
                    p => vec![p],
                };
                for alt in alts {
                    match alt {
                        Pattern::Union { variant, .. } => {
                            seen_variants.insert(variant.clone());
                        }
                        Pattern::Wildcard
                        | Pattern::Identifier(_)
                        | Pattern::Tuple(_)
                        | Pattern::Struct { .. } => {
                            fallback_seen = true;
                        }
                        _ => {}
                    }
                }
            }

            // 各臂类型必须互相兼容，否则报错——此前 `let _ = unify(...)` 丢掉
            // 错误，`match o { some(v) => v, none() => "str" }` 能编译通过，
            // 与实际声明的返回类型不一致，拖到运行时才炸（E6007）。
            // 与函数尾表达式/if-else 的纪律一致：类型层不认的组合宁拒不静默。
            result_ty = Some(match result_ty {
                None => arm_ty,
                Some(prev) => {
                    if let Err(failure) = decide(
                        self.solver,
                        &prev,
                        &arm_ty,
                        Some(match_span),
                        self.type_defs,
                        self.dep_env,
                    ) {
                        let prev_r = self.solver.resolve_type(&prev);
                        let arm_r = self.solver.resolve_type(&arm_ty);
                        return Err(failure.or_site(
                            ErrorCodeDefinition::type_mismatch(
                                &format!("{}", prev_r),
                                &format!("{}", arm_r),
                            )
                            .at(arm.span)
                            .build(),
                        ));
                    }
                    self.solver.resolve_type(&prev)
                }
            });
        }

        // 穷尽性（全类型，2026-09-27 定案：不该隐式的地方不隐式）。
        // 和类型：无兜底臂时变体集必须被 Union 臂全覆盖；
        // 非和类型：无兜底臂时，字面量臂必须覆盖全部可居留值——仅 Bool
        //（true+false）可枚举；Int/String/Char 等值域开放，必须有兜底臂。
        // Void 空类型无可居留值，vacuously 穷尽。
        if let Some(sum_name) = &sum_name {
            if !fallback_seen {
                let missing: Vec<String> = self.sum_types[sum_name]
                    .iter()
                    .filter(|v| !seen_variants.contains(&v.name))
                    .map(|v| v.name.clone())
                    .collect();
                if !missing.is_empty() {
                    return Err(
                        ErrorCodeDefinition::pattern_non_exhaustive(&missing.join(", ")).build(),
                    );
                }
            }
        } else if !fallback_seen {
            let is_void = matches!(resolved, MonoType::Void);
            let bool_covered = resolved == MonoType::Bool
                && ["bool:Bool(true)", "bool:Bool(false)"]
                    .iter()
                    .all(|k| seen_literals.contains(*k));
            if !is_void && !bool_covered {
                return Err(ErrorCodeDefinition::pattern_non_exhaustive("`_`").build());
            }
        }

        Ok(result_ty.unwrap_or_else(|| self.solver.new_var()))
    }

    /// RFC-010b：或模式检查——各备选逐一检查，绑定名集必须一致（E1033）。
    /// 首个备选的绑定进入 `binds`（同名绑定的类型必然一致：同一被匹配类型）。
    fn check_pattern_alts(
        &mut self,
        pattern: &crate::ast::Pattern,
        ty: &MonoType,
        binds: &mut Vec<(String, MonoType)>,
        span: crate::util::span::Span,
    ) -> Result<()> {
        use crate::ast::Pattern;
        let alts: Vec<&Pattern> = match pattern {
            Pattern::Or(ps) => ps.iter().collect(),
            p => vec![p],
        };
        let mut consensus: Option<HashSet<String>> = None;
        for alt in &alts {
            let mut scratch: Vec<(String, MonoType)> = Vec::new();
            self.check_pattern(alt, ty, &mut scratch, span)?;
            let names: HashSet<String> = scratch.iter().map(|(n, _)| n.clone()).collect();
            match &consensus {
                None => {
                    consensus = Some(names);
                    *binds = scratch;
                }
                Some(prev) if *prev != names => {
                    return Err(ErrorCodeDefinition::or_pattern_binding_mismatch()
                        .at(span)
                        .build());
                }
                _ => {}
            }
        }
        Ok(())
    }

    /// RFC-010b：单备选模式检查——按被匹配类型递归。Union 的和类型上下文
    /// 从 `ty` 自足解析（载荷位可能是另一和类型，如 `ok(some(v))`）。
    fn check_pattern(
        &mut self,
        pattern: &crate::ast::Pattern,
        ty: &MonoType,
        binds: &mut Vec<(String, MonoType)>,
        span: crate::util::span::Span,
    ) -> Result<()> {
        use crate::ast::Pattern;
        let resolved = self.solver.resolve_type(ty);
        match pattern {
            Pattern::Wildcard => Ok(()),
            Pattern::Identifier(bn) => {
                if binds.iter().any(|(n, _)| n == bn) {
                    return Err(ErrorCodeDefinition::duplicate_pattern_binding(bn)
                        .at(span)
                        .build());
                }
                binds.push((bn.clone(), resolved.clone()));
                Ok(())
            }
            Pattern::Literal(lit) => {
                let lit_ty = Self::literal_pattern_type(lit);
                if let Err(failure) = decide(
                    self.solver,
                    &resolved,
                    &lit_ty,
                    Some(span),
                    self.type_defs,
                    self.dep_env,
                ) {
                    return Err(failure.or_site(
                        ErrorCodeDefinition::type_mismatch(
                            &resolved.to_string(),
                            &lit_ty.to_string(),
                        )
                        .at(span)
                        .build(),
                    ));
                }
                Ok(())
            }
            Pattern::Range { lo, hi } => {
                // issue #405：范围模式——端点必须都是整数字面量（负端点已在
                // parser 归一化为负 Int；Float/Bool 等端点语法成立、在此拒绝），
                // 端点类型与被匹配值一致（同 Literal 模式的 decide 判据）。
                // 端点是编译期字面量，倒序范围（lo > hi）在此判空：空范围
                // 恒不命中，该臂不可达（E1031）。
                let lo_ty = Self::literal_pattern_type(lo);
                let hi_ty = Self::literal_pattern_type(hi);
                if lo_ty != MonoType::Int(64) {
                    return Err(ErrorCodeDefinition::type_mismatch(
                        "Int (range pattern bound)",
                        &lo_ty.to_string(),
                    )
                    .at(span)
                    .build());
                }
                if hi_ty != MonoType::Int(64) {
                    return Err(ErrorCodeDefinition::type_mismatch(
                        "Int (range pattern bound)",
                        &hi_ty.to_string(),
                    )
                    .at(span)
                    .build());
                }
                let (Literal::Int(lo_v), Literal::Int(hi_v)) = (lo, hi) else {
                    return Err(ErrorCodeDefinition::type_mismatch(
                        "Int (range pattern bound)",
                        "non-int literal",
                    )
                    .at(span)
                    .build());
                };
                if let Err(failure) = decide(
                    self.solver,
                    &resolved,
                    &MonoType::Int(64),
                    Some(span),
                    self.type_defs,
                    self.dep_env,
                ) {
                    return Err(failure.or_site(
                        ErrorCodeDefinition::type_mismatch(
                            &resolved.to_string(),
                            &MonoType::Int(64).to_string(),
                        )
                        .at(span)
                        .build(),
                    ));
                }
                if lo_v > hi_v {
                    return Err(ErrorCodeDefinition::unreachable_pattern(&format!(
                        "{}..{}",
                        lo_v, hi_v
                    ))
                    .at(span)
                    .build());
                }
                Ok(())
            }
            Pattern::Tuple(ps) => {
                let args: Vec<MonoType> = match &resolved {
                    MonoType::Generic { name, args } if name == "Tuple" => args.clone(),
                    other => {
                        return Err(
                            ErrorCodeDefinition::type_mismatch("元组", &other.to_string())
                                .at(span)
                                .build(),
                        );
                    }
                };
                if args.len() != ps.len() {
                    return Err(ErrorCodeDefinition::argument_count_mismatch(
                        "元组模式",
                        args.len(),
                        ps.len(),
                    )
                    .at(span)
                    .build());
                }
                for (p, t) in ps.iter().zip(args.iter()) {
                    self.check_pattern(p, t, binds, span)?;
                }
                Ok(())
            }
            Pattern::Struct { name, fields } => {
                // 解析结构体类型：Struct 直接用；TypeRef 经类型镜像复核
                let struct_ty: crate::frontend::core::types::mono::StructType = match &resolved {
                    MonoType::Struct(s) if &s.name == name => s.clone(),
                    MonoType::TypeRef(n) if n == name => match self.type_def(n).map(|p| &p.body) {
                        Some(MonoType::Struct(s)) => s.clone(),
                        _ => {
                            return Err(ErrorCodeDefinition::type_mismatch(
                                name,
                                &resolved.to_string(),
                            )
                            .at(span)
                            .build());
                        }
                    },
                    other => {
                        return Err(ErrorCodeDefinition::type_mismatch(name, &other.to_string())
                            .at(span)
                            .build());
                    }
                };
                // 声明字段必须被完整覆盖（`..` 略名尚未支持）；未知字段拒绝
                for field_info in &struct_ty.fields {
                    let fname = &field_info.name;
                    let fty = &field_info.ty;
                    let matched: Vec<&(String, bool, Box<Pattern>)> =
                        fields.iter().filter(|(n, _, _)| n == fname).collect();
                    if matched.is_empty() {
                        return Err(
                            ErrorCodeDefinition::struct_pattern_missing_field(name, fname)
                                .at(span)
                                .build(),
                        );
                    }
                    if matched.len() > 1 {
                        return Err(ErrorCodeDefinition::duplicate_pattern_binding(fname)
                            .at(span)
                            .build());
                    }
                    self.check_pattern(&matched[0].2, fty, binds, span)?;
                }
                for (fname, _, _) in fields {
                    if !struct_ty.fields.iter().any(|f| &f.name == fname) {
                        return Err(ErrorCodeDefinition::field_not_found(fname, name)
                            .at(span)
                            .build());
                    }
                }
                Ok(())
            }
            Pattern::Union {
                variant, pattern, ..
            } => {
                let (sum_name, variants, param_names, concrete) = match &resolved {
                    MonoType::Generic { name, args } if self.sum_variants(name).is_some() => {
                        let param_names = self
                            .generic_type_defs
                            .get(name.bare())
                            .map(|d| d.type_param_names.clone())
                            .unwrap_or_default();
                        (
                            name.clone(),
                            self.sum_types[name.bare()].clone(),
                            param_names,
                            args.clone(),
                        )
                    }
                    MonoType::Struct(s) if self.sum_variants(&s.name).is_some() => (
                        s.name.clone(),
                        self.sum_variants(&s.name).cloned().unwrap_or_default(),
                        Vec::new(),
                        Vec::new(),
                    ),
                    MonoType::TypeRef(n) if self.sum_variants(n).is_some() => (
                        n.clone(),
                        self.sum_types[n.bare()].clone(),
                        Vec::new(),
                        Vec::new(),
                    ),
                    other => {
                        return Err(ErrorCodeDefinition::type_mismatch(
                            "和类型（变体解构要求 scrutinee 实现该变体集）",
                            &other.to_string(),
                        )
                        .at(span)
                        .build());
                    }
                };
                // WBS 3.20.2：变体解构消费的是该类型的**变体集**——变体集
                // 随 use 导入成文（register_type_payload 写入 sum_types），
                // 和类型名经成员监视集回指到导入（note_import_use），于是
                // `match some(v) / none()` 算作消费了 `use std.option`，
                // 不再误报 W1003（docs/src/reference/stdlib/weak.md 自然
                // 写法；变体名不走 Var 解析臂，此臂是它唯一的语义消费点）
                self.note_import_use(sum_name.bare());
                let Some(vdef) = variants.iter().find(|v| &v.name == variant) else {
                    return Err(
                        ErrorCodeDefinition::field_not_found(variant, sum_name.bare())
                            .at(span)
                            .build(),
                    );
                };
                match pattern.as_deref() {
                    // 零载荷模式（`none()`）：变体必须真的无参数
                    None => {
                        if !vdef.params.is_empty() {
                            return Err(ErrorCodeDefinition::argument_count_mismatch(
                                variant,
                                vdef.params.len(),
                                0,
                            )
                            .at(span)
                            .build());
                        }
                        Ok(())
                    }
                    Some(p) => {
                        if vdef.params.is_empty() {
                            return Err(ErrorCodeDefinition::argument_count_mismatch(
                                variant, 0, 1,
                            )
                            .at(span)
                            .build());
                        }
                        let substituted: Vec<MonoType> = vdef
                            .params
                            .iter()
                            .map(|pty| {
                                crate::frontend::core::sema::TypeEnvironment::replace_type_params(
                                    pty,
                                    &param_names,
                                    &concrete,
                                )
                            })
                            .collect();
                        // 载荷类型：1 参原样；多参 Tuple（与构造打包对称）
                        let payload_ty = match substituted.len() {
                            1 => substituted.into_iter().next().unwrap(),
                            _ => MonoType::make_tuple(substituted),
                        };
                        self.check_pattern(p, &payload_ty, binds, span)
                    }
                }
            }
            Pattern::Or(_) => self.check_pattern_alts(pattern, ty, binds, span),
            Pattern::Guard { .. } => Err(ErrorCodeDefinition::ir_unsupported_pattern("guard")
                .at(span)
                .build()),
        }
    }

    /// 守卫条件检查：必须产出 Bool（绑定已在臂作用域内，check_expr 期间可见）
    fn check_guard_condition(
        &mut self,
        cond: &crate::ast::Expr,
    ) -> Result<()> {
        let cond_ty = self.infer_expr(cond)?;
        let cond_ty = self.solver.resolve_type(&cond_ty);
        if !matches!(cond_ty, MonoType::Bool) {
            return Err(ErrorCodeDefinition::type_mismatch("Bool", &cond_ty.to_string()).build());
        }
        Ok(())
    }

    /// 字面量模式的类型（模式位置的字面量按字面类型对齐 scrutinee）
    fn literal_pattern_type(lit: &crate::frontend::core::lexer::tokens::Literal) -> MonoType {
        match lit {
            crate::frontend::core::lexer::tokens::Literal::Int(_) => MonoType::Int(64),
            crate::frontend::core::lexer::tokens::Literal::Float(_) => MonoType::Float(64),
            crate::frontend::core::lexer::tokens::Literal::Bool(_) => MonoType::Bool,
            crate::frontend::core::lexer::tokens::Literal::String(_) => MonoType::make_string(),
            crate::frontend::core::lexer::tokens::Literal::Char(_) => MonoType::Char,
            crate::frontend::core::lexer::tokens::Literal::Void => MonoType::Void,
        }
    }

    /// RFC-011b: 算术运算的登记表查询。命中用户实例化时记录 OperatorDispatch
    /// （ir_gen 据此生成 `Call "Type.method"`），native 命中直接取登记结果类型
    /// （运算符保持原生指令，不走方法调用）。
    fn query_operator_dispatch(
        &mut self,
        op: &BinOp,
        l: &MonoType,
        r: &MonoType,
        span: crate::util::span::Span,
        receiver: &crate::ast::Expr,
    ) -> Option<MonoType> {
        use crate::frontend::core::sema::operator_interfaces as ops;
        let iface = ops::arithmetic_interface(op)?;
        let hit = ops::query_operator_template(
            self.interface_impl_registry,
            self.generic_type_defs,
            self.solver,
            iface,
            &[l.clone(), r.clone()],
        );
        let (entry, remaining) = match hit {
            Some(hit) => hit,
            None => {
                // RFC-011b：约束形参延迟派发——操作数是带匹配约束的
                // 当前函数类型形参（TypeRef("T")）→ 记形参名派发，ir_gen
                // 发 `Call "T.add"`，mono 特化时按类型实参改写
                if let Some(pn) = self.constrained_operand_param(op, l, r) {
                    let method = ops::spec(iface)
                        .map(|s| s.method.to_string())
                        .unwrap_or_else(|| "add".to_string());
                    // 约束形参名不是类型身份：名字就是形参占位 `T.add`，mono 特化
                    // 时按类型实参改写（不经导入面——形参名不在类型名空间里）
                    let dispatch_name = format!("{pn}.{method}");
                    // 约束形参延迟派发：结果位就是形参自身（mono 特化时改写）。
                    let result_ty = MonoType::TypeRef(
                        Nominal::flat(AsRef::<str>::as_ref(&(pn)))
                            .expect("type name must be bare and non-empty"),
                    );
                    self.operator_dispatches.push(ops::OperatorDispatch {
                        span,
                        type_name: pn.clone(),
                        method,
                        dispatch_name,
                        negate: false,
                        result_type: Some(result_ty.clone()),
                    });
                    return Some(result_ty);
                }
                return None;
            }
        };
        let result_ty = remaining.first()?.clone();
        if !entry.native {
            let type_name = entry.impl_type.clone();
            let method = entry
                .methods
                .first()
                .cloned()
                .unwrap_or_else(|| "add".to_string());
            let dispatch_name = super::operator_dispatch_name(
                self.scope,
                self.import_face,
                receiver,
                &type_name,
                &method,
            );
            self.operator_dispatches.push(ops::OperatorDispatch {
                span,
                type_name,
                method,
                dispatch_name,
                negate: false,
                result_type: Some(result_ty.clone()),
            });
        }
        Some(result_ty)
    }

    /// 约束形参判定：操作数是 TypeRef("T") 且 T 带与运算符匹配的约束接口
    /// → 返回形参名（延迟派发）
    fn constrained_operand_param(
        &self,
        op: &BinOp,
        l: &MonoType,
        r: &MonoType,
    ) -> Option<String> {
        use crate::frontend::core::sema::operator_interfaces as ops;
        let iface = ops::arithmetic_interface(op)?;
        let constraints = self.current_fn_param_constraints.as_ref()?;
        for operand in [l, r] {
            if let MonoType::TypeRef(pn) = operand {
                if constraints.get(pn.bare()).map(|s| s.as_str()) == Some(iface) {
                    return Some(pn.clone().bare().to_string());
                }
            }
        }
        None
    }

    /// RFC-011b: `==` / `!=` 的类型判定与派发记录。
    ///
    /// 非记录形态按比较「两侧同型」纪律判定（issue #549，2026-11-20 用户裁决
    /// 甲案；与 `infer_binary` 排序比较臂同款 `decide` 入口）：两侧 unify 失败
    /// 报 E1002，不再吞错误放行异型比较。放行面按既有规则收窄——`Any`/`Never`
    /// 侧走 unify 顶部/底部类型规则放行（std/test.yx 的 `Any == Any` 槽位），
    /// TypeVar 一侧延后判定（泛型体内合法），借用操作数按 RFC-009 读穿透剥层
    /// （std/list.yx 的 `list[i] == item` 形态 (A, &A) 剥借用后同型）。至少一侧
    /// 为记录类型时进接口判定：显式 `Equal(T, T)` 实例化优先（记 OperatorDispatch
    /// 供 ir_gen 派发方法调用），否则结构推导——全字段可比且不含 `&mut` 线性
    /// 令牌则原生逐字段比较，失败报 E1101（指明拖后腿的字段）。
    pub fn infer_equality(
        &mut self,
        op: &BinOp,
        left_ty: &MonoType,
        right_ty: &MonoType,
        span: crate::util::span::Span,
        receiver: &crate::ast::Expr,
    ) -> Result<MonoType> {
        use crate::frontend::core::sema::operator_interfaces as ops;
        let negate = matches!(op, BinOp::Neq);
        let l = read_view(left_ty);
        let r = read_view(right_ty);

        // 未定型类型变量一侧：延后判定（保持既有行为，泛型体内合法）
        if matches!(left_ty, MonoType::TypeVar(_)) || matches!(right_ty, MonoType::TypeVar(_)) {
            let _ = self.solver.unify(left_ty, right_ty);
            return Ok(MonoType::Bool);
        }

        let l_record = self.as_user_record(&l);
        let r_record = self.as_user_record(&r);

        // RFC-010：和类型相等 = 类型身份 + 变体序 + 载荷（RFC-010「运行时表示与相等」），
        // **不走结构推导**——和类型的字段全是变体构造器（函数），结构推导必然判
        // 「字段不可比较」而拒绝。此前 `Color.green() == Color.green()`（两侧都是
        // 构造器产物 Generic）走本函数的早退分支能过，但 `mk() == Color.green()`
        // （一侧是注解形态 TypeRef，经 type_defs 落成 Struct）会掉进结构推导报
        // E1101——同一表达式因变量来源不同而结果不同。名义命中即放行。
        //
        // 同名还必须**实参兼容**：`Option(Int).none() == Option(String).none()`
        // 名义都是 "Option"，只比名字会静默判等（实为不同类型身份的变体）。
        let l_sum = self.as_sum_type_name(&l);
        let r_sum = self.as_sum_type_name(&r);
        if let (Some(ln), Some(rn)) = (&l_sum, &r_sum) {
            if ln == rn {
                if self.solver.unify(left_ty, right_ty).is_err() {
                    return Err(ErrorCodeDefinition::type_mismatch(
                        &format!("{}", self.solver.resolve_type(left_ty)),
                        &format!("{}", self.solver.resolve_type(right_ty)),
                    )
                    .at(span)
                    .build());
                }
                return Ok(MonoType::Bool);
            }
            // 不同名和类型（如 Maybe vs Color）：身份不同，不可比
            return Err(ErrorCodeDefinition::type_mismatch(
                &format!("{}", self.solver.resolve_type(left_ty)),
                &format!("{}", self.solver.resolve_type(right_ty)),
            )
            .at(span)
            .build());
        }

        if l_record.is_none() && r_record.is_none() {
            // 「两侧同型」纪律（issue #549，2026-11-20 用户裁决甲案）：与
            // infer_binary 排序比较臂（Lt/Le/Gt/Ge）同款——两侧 unify 失败即报
            // E1002，不再吞错误放行异型比较。此前 `let _ = unify(...)` 恒放行，
            // `1 == "a"` / `1 == 2.5` / `[1] == ["x"]` 编译期全绿、运行期才落
            // E6007（#500 查证实测）；容器异型族（`(1,2) == [1,2]` 等）运行
            // 成功打印 false——D59 D 类（改变原先成功程序的可观察行为），经
            // 用户裁决全量对齐。判定走 `decide`（与排序比较臂同一入口）：失败
            // 且 ground 时仍给 RFC-027 判等一次机会；操作数取读视图（RFC-009
            // 比较读穿透），诊断文面与排序比较臂同族同文（resolve 后的两侧
            // 原始形态，E1002）。
            if let Err(failure) = decide(
                self.solver,
                &l,
                &r,
                Some(span),
                self.type_defs,
                self.dep_env,
            ) {
                return Err(failure.or_site(
                    ErrorCodeDefinition::type_mismatch(
                        &format!("{}", self.solver.resolve_type(left_ty)),
                        &format!("{}", self.solver.resolve_type(right_ty)),
                    )
                    .at(span)
                    .build(),
                ));
            }
            return Ok(MonoType::Bool);
        }

        // 显式 Equal 实例化优先（登记表查询，名义结构匹配；
        // native 条目只含基础类型，Struct 查询命中的必为用户条目）
        if let Some(entry) = ops::query_exact_template(
            self.interface_impl_registry,
            self.generic_type_defs,
            self.solver,
            ops::EQUAL_INTERFACE,
            &[l.clone(), r.clone()],
        ) {
            let type_name = entry.impl_type.clone();
            let method = entry
                .methods
                .first()
                .cloned()
                .unwrap_or_else(|| "equal".to_string());
            let dispatch_name = super::operator_dispatch_name(
                self.scope,
                self.import_face,
                receiver,
                &type_name,
                &method,
            );
            self.operator_dispatches.push(ops::OperatorDispatch {
                span,
                type_name,
                method,
                dispatch_name,
                negate,
                // `equal` 的返回位是 Bool；`!=` 取反后仍是 Bool。
                result_type: Some(MonoType::Bool),
            });
            return Ok(MonoType::Bool);
        }

        // 同名记录 → 结构推导（递归字段，&mut 拒绝）
        if let (Some(ls), Some(rs)) = (&l_record, &r_record) {
            if ls.name == rs.name {
                match self.struct_comparable(&MonoType::Struct(ls.clone()), 0) {
                    Ok(()) => return Ok(MonoType::Bool),
                    Err(field_desc) => {
                        return Err(ErrorCodeDefinition::type_does_not_implement_interface(
                            &format!("{}（字段 {} 不可比较）", ls.name, field_desc),
                            ops::EQUAL_INTERFACE,
                        )
                        .at(span)
                        .build());
                    }
                }
            }
        }

        // 异型 / 单侧记录 / 字段不可比
        let name = l_record
            .as_ref()
            .or(r_record.as_ref())
            .map(|s| s.name.clone())
            .unwrap_or_else(|| {
                Nominal::flat(AsRef::<str>::as_ref(&("Struct")))
                    .expect("type name must be bare and non-empty")
            });
        Err(ErrorCodeDefinition::type_does_not_implement_interface(
            &format!("{}（{} 与 {}）", name, l, r),
            ops::EQUAL_INTERFACE,
        )
        .at(span)
        .build())
    }

    /// RFC-010: 和类型名义名——`Generic`/`Struct`/`TypeRef` 三形态统一判定入口。
    ///
    /// 和类型的相等判定不得依赖 `as_user_record`（它只认 Struct/TypeRef，
    /// 会把 Generic 形态的构造器产物漏掉），故单立此查询。判定口径与
    /// `check_match_expr` / `match_scrutinee_sum_type` 同源（三形态 + 类型镜像）。
    fn as_sum_type_name(
        &self,
        ty: &MonoType,
    ) -> Option<String> {
        match ty {
            MonoType::Generic { name, .. } if self.sum_variants(name).is_some() => {
                Some(name.clone().bare().to_string())
            }
            MonoType::Struct(s) if self.sum_variants(&s.name).is_some() => {
                Some(s.name.clone().bare().to_string())
            }
            MonoType::TypeRef(n) => {
                if self.sum_variants(n).is_some() {
                    Some(n.clone().bare().to_string())
                } else {
                    // 注解形态经类型镜像复核（与 match 的和类型判定同款）
                    match self.type_def(n).map(|p| &p.body) {
                        Some(MonoType::Struct(st)) if self.sum_variants(&st.name).is_some() => {
                            Some(st.name.bare().to_string())
                        }
                        _ => None,
                    }
                }
            }
            _ => None,
        }
    }

    /// RFC-011b: 解析为用户记录类型（Struct 或 type_defs 可解析的 TypeRef）
    fn as_user_record(
        &self,
        ty: &MonoType,
    ) -> Option<crate::frontend::core::types::StructType> {
        match ty {
            MonoType::Struct(s) => Some(s.clone()),
            MonoType::TypeRef(name) => match self.type_def(name).map(|p| &p.body) {
                Some(MonoType::Struct(s)) => Some(s.clone()),
                _ => None,
            },
            _ => None,
        }
    }

    /// RFC-011b Equal 结构推导：类型是否可比（基础类型、String、可比记录/
    /// 元组/列表；`&mut` 线性令牌读一次即消耗，不可比）。返回 Err(首个不可比
    /// 字段的描述)。
    fn struct_comparable(
        &self,
        ty: &MonoType,
        depth: usize,
    ) -> Result<(), String> {
        if depth > 16 {
            return Err("类型嵌套过深（疑似循环引用）".to_string());
        }
        // &mut 线性令牌：读一次即消耗，无法同时取出两个值来比——
        // 必须在 read_view 穿透之前判定（穿透会把 &mut T 剥成 T）
        if let MonoType::Ref {
            mutable: true,
            inner,
        } = ty
        {
            return Err(format!("&mut {}", inner));
        }
        let ty = read_view(ty);
        match &ty {
            MonoType::Int(_)
            | MonoType::Float(_)
            | MonoType::Bool
            | MonoType::Char
            | MonoType::Void
            | MonoType::Never => Ok(()),
            MonoType::Struct(s) => {
                for field_info in &s.fields {
                    self.struct_comparable(&field_info.ty, depth + 1)
                        .map_err(|d| format!("{}: {}", field_info.name, d))?;
                }
                Ok(())
            }
            MonoType::Generic { name, args } if name == "String" || name == "Bytes" => {
                let _ = args;
                Ok(())
            }
            MonoType::Generic { name, args } if name == "Range" => args
                .iter()
                .try_for_each(|a| self.struct_comparable(a, depth + 1)),
            MonoType::Generic { name, args }
                if name == "List" || name == "Array" || name == "Vec" || name == "Tuple" =>
            {
                args.iter()
                    .try_for_each(|a| self.struct_comparable(a, depth + 1))
                    .map_err(|d| format!("{}<{}>", name, d))
            }
            MonoType::TypeRef(name) => {
                // 内建名（字段经 from_builtin_name 已物化；此处兜底别名形态）
                if crate::frontend::core::types::MonoType::from_builtin_name(name.bare()).is_some()
                {
                    return Ok(());
                }
                match self.type_def(name).map(|p| &p.body) {
                    Some(resolved) => self.struct_comparable(resolved, depth + 1),
                    _ => Err(name.clone().bare().to_string()),
                }
            }
            other => Err(other.to_string()),
        }
    }

    /// 推断一元操作符表达式类型
    pub fn infer_unary(
        &mut self,
        op: &UnOp,
        expr: &MonoType,
    ) -> Result<MonoType> {
        match op {
            UnOp::Neg => Ok(expr.clone()),
            UnOp::Pos => Ok(expr.clone()),
            UnOp::Not => {
                if *expr == MonoType::Bool {
                    Ok(MonoType::Bool)
                } else {
                    Err(
                        ErrorCodeDefinition::logical_not_type_mismatch(&format!("{}", expr))
                            .build(),
                    )
                }
            }
            UnOp::Deref => {
                if let MonoType::TypeRef(inner) = expr {
                    let inner_type = inner.bare().trim_start_matches('*').to_string();
                    Ok(MonoType::TypeRef(
                        Nominal::flat(AsRef::<str>::as_ref(&(inner_type)))
                            .expect("type name must be bare and non-empty"),
                    ))
                } else {
                    Err(ErrorCodeDefinition::invalid_deref(&format!("{}", expr)).build())
                }
            }
        }
    }

    /// 泛型类型构造器名在**值位置**的类型：类型级函数 (T: Type, ...) -> Type
    /// （RFC-010 元类型层级——构造器本身是 Type1 值，Box 的类型是 (T: Type) -> Type，
    /// 而 Int 的类型是 Type）。
    ///
    /// 仅泛型类型构造器（generic_type_defs）有此形态；非泛型类型名在值位置
    /// 仍是记录构造器调用（Point(1.0, 2.0)），返回 None 不改写。
    fn type_ctor_value_type(
        &self,
        name: &str,
    ) -> Option<MonoType> {
        let def = self.generic_def(&self.nominal_for(name))?;
        let meta = MonoType::MetaType {
            universe_level: crate::frontend::core::types::mono::UniverseLevel::type0(),
            type_params: Vec::new(),
        };
        let mut params: Vec<MonoType> = vec![meta.clone(); def.type_param_names.len()];
        for binder in &def.poly.const_binders {
            params.push(match binder.kind {
                crate::frontend::core::types::const_data::ConstKind::Int(_) => MonoType::Int(64),
                crate::frontend::core::types::const_data::ConstKind::Bool => MonoType::Bool,
                crate::frontend::core::types::const_data::ConstKind::Float(_) => {
                    MonoType::Float(64)
                }
            });
        }
        if params.is_empty() {
            return None;
        }
        Some(MonoType::Fn {
            params,
            return_type: Box::new(meta),
        })
    }

    /// 作用域里的类型名条目是否为「该类型自身的记录镜像」：add_type_definition
    /// 把类型名同时登记为值名（env.add_var），其类型就是该记录类型本身。
    /// 同名真值变量（mut Box = ...）不是镜像，按普通值处理。
    fn is_self_type_mirror(
        name: &str,
        body: &MonoType,
    ) -> bool {
        matches!(body, MonoType::Struct(s) if s.name == name)
    }

    /// 用实际类型 `actual` 解出 `shape` 里未绑定的 `TypeRef` 类型参数，返回替换后的类型。
    ///
    /// 只在**结构对齐**处绑定（泛型名相同、实参个数相同、以及 Ref/Fn 逐位），
    /// 因此 `List(TypeRef "A")` vs `List(Int)` → `List(Int)`。
    /// 无绑定则原样返回，交由常规 unify 决定。
    fn bind_type_params_from(
        shape: &MonoType,
        actual: &MonoType,
    ) -> MonoType {
        match (shape, actual) {
            (MonoType::TypeRef(_), act) => {
                if matches!(act, MonoType::TypeRef(_)) {
                    shape.clone()
                } else {
                    act.clone()
                }
            }
            (
                MonoType::Generic { name: n1, args: a1 },
                MonoType::Generic { name: n2, args: a2 },
            ) if n1 == n2 && a1.len() == a2.len() => MonoType::Generic {
                name: n1.clone(),
                args: a1
                    .iter()
                    .zip(a2.iter())
                    .map(|(s, a)| Self::bind_type_params_from(s, a))
                    .collect(),
            },
            (MonoType::Ref { mutable, inner: i1 }, MonoType::Ref { inner: i2, .. }) => {
                MonoType::Ref {
                    mutable: *mutable,
                    inner: Box::new(Self::bind_type_params_from(i1, i2)),
                }
            }
            (
                MonoType::Fn {
                    params: p1,
                    return_type: r1,
                },
                MonoType::Fn {
                    params: p2,
                    return_type: r2,
                },
            ) if p1.len() == p2.len() => MonoType::Fn {
                params: p1
                    .iter()
                    .zip(p2.iter())
                    .map(|(s, a)| Self::bind_type_params_from(s, a))
                    .collect(),
                return_type: Box::new(Self::bind_type_params_from(r1, r2)),
            },
            _ => shape.clone(),
        }
    }

    /// 6.8.7：期望类型是**单值类型** Literal{v} 且字面量取值等于 v 时，返回该
    /// 单值类型（单值类型的居民）；否则 None（调用方按既有基类型定型）。
    ///
    /// 只在「期望明确是 Literal{v} 且值相等」时命中——不是「字面量恒为 Literal」
    /// （后者会让全部整数字面量改型、224 份快照大面积漂移，本轮不选）。
    fn literal_matches_single_value_type(
        lit: &crate::frontend::core::lexer::tokens::Literal,
        expected: Option<&MonoType>,
    ) -> Option<MonoType> {
        let MonoType::Literal {
            value: want,
            base_type,
            name,
        } = expected?
        else {
            return None;
        };
        let have = crate::frontend::core::sema::const_param::literal_const_value(lit)?;
        (have == *want).then(|| MonoType::Literal {
            name: name.clone(),
            base_type: base_type.clone(),
            value: want.clone(),
        })
    }

    /// 带期望类型的表达式推断：只有匿名 lambda 消费期望——被调签名的
    /// 形参位就是它的权威签名（议题 1 机制的 Call 臂延伸）。其余表达式
    /// 形态与 [`Self::infer_expr`] 完全一致。
    pub fn infer_expr_with_expected(
        &mut self,
        expr: &crate::ast::Expr,
        expected: Option<&MonoType>,
    ) -> Result<MonoType> {
        match expr {
            crate::ast::Expr::Lambda {
                params,
                body,
                plan_id,
                ..
            } => {
                let ty = self.infer_lambda(params, body, expected)?;
                self.record_lambda_result_type(*plan_id, &ty);
                Ok(ty)
            }
            // 6.8.7：字面量同样消费期望——期望是单值类型且取值相等时定型为该
            // 单值类型。调用实参位因此能拿到 Literal{v} 型实参：
            // f(42)(42) 的 42 定型为 int64::42，与形参 n: N 的单值类型对齐。
            crate::ast::Expr::Lit(lit, _) => {
                let expected_resolved = expected.map(|e| self.solver.resolve_type(e));
                match Self::literal_matches_single_value_type(lit, expected_resolved.as_ref()) {
                    Some(ty) => Ok(ty),
                    None => self.infer_literal(lit),
                }
            }
            _ => self.infer_expr(expr),
        }
    }

    /// 类型是否是 witness-bearing 记录（6.9.5-B.1c / 5A）。形状归一在
    /// `inference::mono_type_is_witness_bearing`（唯一读取点）。
    pub(super) fn is_witness_bearing_type(
        &self,
        ty: &MonoType,
    ) -> bool {
        mono_type_is_witness_bearing(ty, self.type_defs, self.type_identity_index)
    }

    /// Any 多态槽位判定：`TypeRef("Any")`（可带 Ref 壳）。求解器对
    /// `unify(Any, T)` 恒放行，期望传播时此类位退回 fresh var。
    fn is_any_slot(ty: &MonoType) -> bool {
        match ty {
            MonoType::Ref { inner, .. } => Self::is_any_slot(inner),
            MonoType::TypeRef(n) => n == "Any",
            _ => false,
        }
    }

    /// 期望 Fn 类型剥纯类型参数层（议题 1 已拍板规则）：只有「参数全是
    /// 类型位（MetaType）」的层才剥；混合层无法安全剥除 → None，调用方
    /// 退回无期望推断。期望不是 Fn 同样 None。
    fn value_level_expected(expected: &MonoType) -> Option<MonoType> {
        let mut cur = expected.clone();
        loop {
            match cur {
                MonoType::Fn {
                    ref params,
                    ref return_type,
                } => {
                    if params.is_empty() {
                        return Some(cur);
                    }
                    let meta_flags = params
                        .iter()
                        .map(|p| matches!(p, MonoType::MetaType { .. }))
                        .collect::<Vec<_>>();
                    if meta_flags.iter().any(|&m| m) {
                        if !meta_flags.iter().all(|&m| m) {
                            return None;
                        }
                        cur = *return_type.clone();
                    } else {
                        return Some(cur);
                    }
                }
                _ => return None,
            }
        }
    }

    /// 闭包结果类型落表（RFC-039 D20：与调用点同一张表、同一身份形态）。
    ///
    /// 存**原样**推断结果；收敛与「未收敛即不落」在导出点做一次（与调用点
    /// 共用 `finalize_expr_result_types`）。身份未分配（合成 lambda）不进表。
    fn record_lambda_result_type(
        &mut self,
        plan_id: crate::util::plan_id::PlanId,
        ty: &MonoType,
    ) {
        if plan_id.is_planned() {
            self.expr_result_types.insert(plan_id, ty.clone());
        }
    }

    /// 匿名 lambda 推断（从 infer_expr 的 Lambda 臂抽出，供期望下传共用）。
    ///
    /// `expected`：被调签名的形参位类型（None = 无期望，行为与历史一致）。
    /// 有期望时剥掉纯类型参数层，参数按期望位取（调用方签名是匿名 lambda
    /// 的权威对齐来源，自带标注与期望的分歧由后续 unify 暴露），体的
    /// return 检查以期望返回位为准。
    fn infer_lambda(
        &mut self,
        params: &[crate::ast::Param],
        body: &crate::ast::Block,
        expected: Option<&MonoType>,
    ) -> Result<MonoType> {
        let value_sig: Option<(Vec<MonoType>, MonoType)> = expected
            .and_then(Self::value_level_expected)
            .and_then(|ty| match ty {
                MonoType::Fn {
                    params: ps,
                    return_type,
                } if ps.len() == params.len() => Some((ps, *return_type)),
                _ => None,
            });

        self.scope.enter_fn();
        // 6.9.4 P5 / T17(b)：lambda 体是**Γ 证据边界**。闭包体延迟执行，外层分支守卫
        // 的事实到调用时刻未必仍成立 ⇒ 体外事实不入体；体内自产的事实只在本体内可见
        // （restore_barrier 同时退 scope ⇒ 随体亡）。
        let saved_gamma_barrier = self.gamma.as_deref_mut().map(|gamma| gamma.enter_barrier());
        // #295：三链模型——enter_fn 推新局部层，外层函数局部变量不在链上（闭包不捕获），
        // 参数链跨边界累积可见（柯里化固化）。
        for (i, param) in params.iter().enumerate() {
            // 两类期望位不传播（退回标注/fresh var，保留旧路径行为）：
            // - Any 多态槽位（std range/list 的 `fn(Any) -> Any` 回调位）：
            //   钉死会让体推断失去自由度；fresh var 与 Any 的 unify 恒成立
            //   且发生在体检查之后（延迟绑定）
            // - 含未解析参数（TypeVar / 声明名 TypeRef，如泛型被调的 `T`，
            //   调用点尚未实例化）：钉死悬空名字会让 `x * 2` 立即报错
            let param_ty = match value_sig.as_ref().and_then(|(ps, _)| ps.get(i)) {
                Some(ty)
                    if !Self::is_any_slot(ty) && !statements::contains_unresolved_param(ty) =>
                {
                    ty.clone()
                }
                _ => match &param.ty {
                    Some(t) => self.mono_from_annotation(&t.clone()),
                    None => self.solver.new_var(),
                },
            };
            self.add_param(
                param.name.clone(),
                PolyType::mono(param_ty),
                param.is_mut,
                param.span,
            );
        }

        // Lambda is a function boundary: it must not inherit outer `Result` context.
        let saved_result_err = self.result_err.take();
        self.result_err = None;
        // Lambda is also a return type boundary
        let saved_expected_ret = self.expected_return_type.take();
        // 期望返回位：体的 return 检查以被调签名的返回类型为准
        self.expected_return_type = value_sig.as_ref().map(|(_, r)| r.clone());
        // 期望位来自**调用点**（被调签名）而非本地声明：注解位置同样隔离，
        // 否则 lambda 体内的 return 不符会指向外层那个无关的绑定注解。
        let saved_ann_span = self.return_annotation_span.take();
        // #311：函数体同样是循环上下文边界
        let saved_loop_depth = self.loop_depth;
        self.loop_depth = 0;
        // Lambda 也是函数边界（RFC-010a 规则②）：体内 return 退出的是本 lambda
        let saved_fn_depth = self.fn_depth;
        self.fn_depth += 1;
        let body_ty = self.infer_block(body, true, None);
        // 箭头 lambda（`(x) => expr`）的体被解析成只含 `return expr` 的块，
        // 块值按规则恒为 `Never`（`return : Never`）。此处**在退出 lambda 作用域前**
        // 取出载荷类型——退出后形参已不在 scope，重推会失败。
        let arrow_payload_ty = match &body_ty {
            Ok(MonoType::Never) if body.stmts.len() == 1 => match &body.stmts[0].kind {
                crate::ast::StmtKind::Return(Some(e)) => self.infer_expr(e).ok(),
                _ => None,
            },
            _ => None,
        };
        self.loop_depth = saved_loop_depth;
        self.fn_depth = saved_fn_depth;
        self.expected_return_type = saved_expected_ret;
        self.return_annotation_span = saved_ann_span;
        self.result_err = saved_result_err;
        // 6.9.4 P5：退出 lambda 体的证据边界（与上方 enter_barrier 成对；
        // 箭头 lambda 的载荷重推也在边界内，故放在其后）
        if let (Some(gamma), Some(saved)) = (self.gamma.as_deref_mut(), saved_gamma_barrier) {
            gamma.restore_barrier(saved);
        }

        self.scope.exit_fn();
        let body_ty = body_ty?;

        // 返回的 Fn 形态与参数注册同源：期望位优先（Any 槽位同上退回），
        // 否则标注/fresh var
        let param_types: Vec<MonoType> = params
            .iter()
            .enumerate()
            .map(
                |(i, p)| match value_sig.as_ref().and_then(|(ps, _)| ps.get(i)) {
                    Some(ty)
                        if !Self::is_any_slot(ty) && !statements::contains_unresolved_param(ty) =>
                    {
                        ty.clone()
                    }
                    _ => match &p.ty {
                        Some(t) => self.mono_from_annotation(&t.clone()),
                        None => self.solver.new_var(),
                    },
                },
            )
            .collect();

        let return_type = arrow_payload_ty.unwrap_or_else(|| body_ty.clone());

        Ok(MonoType::Fn {
            params: param_types,
            return_type: Box::new(return_type),
        })
    }

    /// 推断表达式的类型
    pub fn infer_expr(
        &mut self,
        expr: &crate::ast::Expr,
    ) -> Result<MonoType> {
        // #324：挂当前节点 span，walk 内诊断构造自动获得位置
        let _current_span = crate::util::diagnostic::push_current_span(expr.span());
        match expr {
            // 字面量
            crate::ast::Expr::Lit(lit, _) => self.infer_literal(lit),

            // 变量
            crate::ast::Expr::Var(name, span) => {
                // 语义引用登记（#433/D55）：get_var_info 一次拿全类型与定义位置，
                // resolves_to 与定义侧 BindingEvent 同源（同走 VarInfo.definition_span）
                let resolved = self
                    .scope
                    .get_var_info(name)
                    .map(|info| (info.poly.clone(), info.definition_span));
                if let Some((poly, def_span)) = resolved {
                    // RFC-010 元类型层级：泛型类型构造器名在**值位置**表示构造器
                    // 本身（Type1 值），其类型是类型级函数 (T: Type, ...) -> Type，
                    // 而不是它构造出的记录类型。kind_of(Box) 把构造器当高阶参数
                    // 传递时，实参 Box 必须匹配形参 (T: Type) -> Type（WBS 3.5.1）。
                    // 只改写「类型名自身的记录镜像」；同名真值变量优先。
                    let ctor_value = self
                        .type_ctor_value_type(name)
                        .filter(|_| Self::is_self_type_mirror(name, &poly.body));
                    self.scope.record_reference(name, *span, def_span);
                    // #321 W1003：命中监视集的导入名记为已使用
                    self.note_import_use(name);
                    // 关键：直接使用 scope 中存储的类型！
                    // 因为 assign_var 已经将更新后的类型写入了 scope
                    // 不需要再通过 solver 解析（solver 不知道 scope 的更新）
                    Ok(ctor_value.unwrap_or(poly.body))
                } else if self.sum_variants(&self.nominal_for(name)).is_some() {
                    // RFC-010: 和类型名在表达式位置 — Type 宇宙的值
                    // （`Result(Int, String)` 的内层 Var；构造分派在 Call 臂）
                    Ok(crate::frontend::core::types::MonoType::MetaType {
                        universe_level: crate::frontend::core::types::mono::UniverseLevel::type0(),
                        type_params: Vec::new(),
                    })
                } else if is_builtin_type_name(name) {
                    // 内置类型名在表达式位置 — 当作 Type 宇宙的值
                    Ok(crate::frontend::core::types::MonoType::MetaType {
                        universe_level: crate::frontend::core::types::mono::UniverseLevel::type0(),
                        type_params: Vec::new(),
                    })
                } else {
                    Err(ErrorCodeDefinition::unknown_variable(name)
                        .at(*span)
                        .build())
                }
            }

            // 二元运算
            crate::ast::Expr::BinOp {
                op,
                left,
                right,
                span,
            } => {
                // #300 I 项：Range 构造在表达式层拦截（需要看表达式形状，不只是类型）
                if matches!(op, BinOp::Range) {
                    return self.infer_range_expr(left, right, *span);
                }
                let right_ty = self.infer_expr(right)?;

                if matches!(op, BinOp::Assign) {
                    if let crate::ast::Expr::Var(var_name, _) = left.as_ref() {
                        // 局部程序变量重赋值强制统一（E1002）；仅全局（std/模块导出）
                        // 撞名时保持旧覆写行为，见 assign_var 文档
                        let enforce = self.scope.var_in_local_scopes(var_name);
                        self.assign_var(var_name, right_ty, *span, enforce)?;
                    }
                    return Ok(MonoType::Void);
                }

                let left_ty = self.infer_expr(left)?;
                // RFC-011b: `==`/`!=` 走相等判定（记录类型参与时查 Equal 接口
                // 或结构推导，并记录显式派发点）；其余比较保持既有行为
                if matches!(op, BinOp::Eq | BinOp::Neq) {
                    return self.infer_equality(op, &left_ty, &right_ty, *span, left);
                }
                self.infer_binary(op, &left_ty, &right_ty, *span, left)
            }

            // 一元运算
            crate::ast::Expr::UnOp { op, expr, .. } => {
                let expr_ty = self.infer_expr(expr)?;
                self.infer_unary(op, &expr_ty)
            }

            // 元组
            crate::ast::Expr::Tuple(elems, _) => {
                let types: Result<Vec<_>> = elems.iter().map(|e| self.infer_expr(e)).collect();
                Ok(MonoType::make_tuple(types?))
            }

            // 列表
            crate::ast::Expr::List(elems, _) => {
                if elems.is_empty() {
                    let elem_ty = self.solver.new_var();
                    Ok(MonoType::make_list(elem_ty))
                } else {
                    let mut iter = elems.iter();
                    let first = iter.next().expect("non-empty list must have first element");
                    let mut elem_ty = self.infer_expr(first)?;
                    for e in iter {
                        let ty = self.infer_expr(e)?;
                        let _ = self.solver.unify(&elem_ty, &ty);
                        elem_ty = self.solver.resolve_type(&elem_ty);
                    }
                    Ok(MonoType::make_list(elem_ty))
                }
            }

            // 字典
            crate::ast::Expr::Dict(pairs, _) => {
                if pairs.is_empty() {
                    let key_ty = self.solver.new_var();
                    let value_ty = self.solver.new_var();
                    Ok(MonoType::make_dict(key_ty, value_ty))
                } else {
                    let mut key_ty = None;
                    let mut value_ty = None;
                    for (k, v) in pairs {
                        let k_type = self.infer_expr(k)?;
                        let v_type = self.infer_expr(v)?;
                        if key_ty.is_none() {
                            key_ty = Some(k_type);
                        }
                        if value_ty.is_none() {
                            value_ty = Some(v_type);
                        }
                    }
                    Ok(MonoType::make_dict(
                        key_ty.unwrap_or_else(|| self.solver.new_var()),
                        value_ty.unwrap_or_else(|| self.solver.new_var()),
                    ))
                }
            }

            // 下标访问
            // #299 §3: membership 谓词 `elem in container` → Bool
            // 右操作数：List/Array/Dict(键)/Tuple/String 子串/Range 区间
            crate::ast::Expr::In {
                elem,
                container,
                span,
            } => {
                let elem_ty = self.infer_expr(elem)?;
                let container_ty = self.infer_expr(container)?;
                // #300 A 项：真校验替代三臂同返 Bool 的摆设 match。
                // 元素-容器兼容性："a" in [1,2,3]（String 探 Int 列表）编译期拒绝。
                // Range 字面量（1..10）类型是 Generic{"Range"}，区间检查合法
                // #300 决策4：Set 除名——无运行时表示（HeapValue/std.set 均不存在），
                // 待真实需求出现时照 Dict 模式补全
                let member_ty: Option<MonoType> = match &container_ty {
                    MonoType::Generic { name, args, .. } => match name.as_str() {
                        "List" | "Vec" | "Array" | "Dict" => args.first().cloned(),
                        // ponytail: Tuple 异构成员，精确检查需逐成员回退试探，
                        // 真实误报面极小，需要时再加
                        "Tuple" => return Ok(MonoType::Bool),
                        "Range" => Some(MonoType::Int(64)),
                        "String" => Some(MonoType::make_string()),
                        // 其他泛型（Struct 实例化等）不是容器
                        _ => None,
                    },
                    // 未解析类型变量：推迟到后续绑定，不拦
                    MonoType::TypeVar(_) => return Ok(MonoType::Bool),
                    _ => None,
                };
                match member_ty {
                    Some(member) => {
                        if let Err(failure) = decide(
                            self.solver,
                            &elem_ty,
                            &member,
                            Some(*span),
                            self.type_defs,
                            self.dep_env,
                        ) {
                            return Err(failure.or_site(
                                ErrorCodeDefinition::type_mismatch(
                                    &format!("{}", member),
                                    &format!("{}", elem_ty),
                                )
                                .at(*span)
                                .build(),
                            ));
                        }
                        Ok(MonoType::Bool)
                    }
                    None => Err(ErrorCodeDefinition::type_mismatch(
                        "List/Array/Dict/Tuple/String/Range",
                        &format!("{}", container_ty),
                    )
                    .at(*span)
                    .build()),
                }
            }
            crate::ast::Expr::Index {
                expr: container,
                index,
                span: idx_span,
            } => {
                let container_ty = self.infer_expr(container)?;
                // 剥掉 Ref 层：`&Vec(T)` 上下标读取合法（RFC-009 §2.8 读透明，
                // 与 `r.n` 字段读同款）。此前未剥离，带借用的形参里
                // `list[i]` 报「不可索引」。
                let mut container_ty = container_ty;
                while let MonoType::Ref { inner, .. } = container_ty {
                    container_ty = *inner;
                }
                let container_ty = self.solver.resolve_type(&container_ty);
                match container_ty {
                    MonoType::Generic { name, args } if name == "List" => Ok(args[0].clone()),
                    MonoType::Generic { name, args } if name == "Vec" => Ok(args[0].clone()),
                    MonoType::Generic { name, args } if name == "Array" => Ok(args[0].clone()),
                    MonoType::Generic { name, args } if name == "Dict" => Ok(args[1].clone()),
                    MonoType::Generic { name, args } if name == "Tuple" => {
                        if let crate::ast::Expr::Lit(
                            crate::frontend::core::lexer::tokens::Literal::Int(i),
                            _,
                        ) = index.as_ref()
                        {
                            if *i >= 0 && (*i as usize) < args.len() {
                                Ok(args[*i as usize].clone())
                            } else {
                                Err(
                                    ErrorCodeDefinition::index_out_of_bounds(args.len(), *i as i64)
                                        .build(),
                                )
                            }
                        } else {
                            Ok(self.solver.new_var())
                        }
                    }
                    // #385 定案 D3：String 下标放开——标量值（码点）下标，
                    // 产出单字符 String（定案 C：单字符只用 String 表示，
                    // 与 chars()/substring 同域）；越界/负索引由运行时
                    // StringGetChar 报 E6003（索引失败契约）。此前 String
                    // 落入下方 Index 接口查询臂被整体静态拒绝。
                    MonoType::Generic { name, .. } if name == "String" => {
                        self.infer_expr(index)?;
                        Ok(MonoType::make_string())
                    }
                    MonoType::Fn { .. } => {
                        // RFC-004 多位置绑定语法 f[0] / f[1,2]：索引结果由语句层
                        // 方法绑定机制消费，此处类型不收敛（既有宽松语义，非兜底洞）
                        Ok(self.solver.new_var())
                    }
                    // RFC-011b：登记表查询——实现 Index 接口的用户容器。
                    // 命中用户实例化时记录 OperatorDispatch（ir_gen 派发
                    // `Call "Type.index"`），native 容器不受影响（走上面快路径）。
                    other => {
                        use crate::frontend::core::sema::operator_interfaces as ops;
                        let key_ty = self.infer_expr(index)?;
                        let key_ty = self.solver.resolve_type(&key_ty);
                        let (entry, remaining) = match ops::query_operator_template(
                            self.interface_impl_registry,
                            self.generic_type_defs,
                            self.solver,
                            ops::INDEX_INTERFACE,
                            &[other.clone(), key_ty],
                        ) {
                            Some(hit) => hit,
                            None => {
                                return Err(ErrorCodeDefinition::type_mismatch(
                                    "List/Array/Dict/Tuple/String（可索引）或实现 Index 接口",
                                    &format!("{other}"),
                                )
                                .at(container.span())
                                .build())
                            }
                        };
                        let Some(value_ty) = remaining.first() else {
                            return Err(ErrorCodeDefinition::type_mismatch(
                                "Index 接口需要三实参 (Self, Key, Value)",
                                &format!("{}", other),
                            )
                            .at(container.span())
                            .build());
                        };
                        if !entry.native {
                            let type_name = entry.impl_type.clone();
                            let method = entry
                                .methods
                                .first()
                                .cloned()
                                .unwrap_or_else(|| "index".to_string());
                            let dispatch_name = super::operator_dispatch_name(
                                self.scope,
                                self.import_face,
                                container,
                                &type_name,
                                &method,
                            );
                            self.operator_dispatches.push(ops::OperatorDispatch {
                                span: *idx_span,
                                type_name,
                                method,
                                dispatch_name,
                                negate: false,
                                result_type: Some(value_ty.clone()),
                            });
                        }
                        Ok(value_ty.clone())
                    }
                }
            }

            // 字段访问
            crate::ast::Expr::FieldAccess {
                expr: obj,
                field,
                span,
                ..
            } => {
                // 方法绑定键状态位（`last_method_binding_key`）：本臂是它唯一的
                // 写入点——进体先清空，命中方法绑定的每个返回点写入。消费在
                // 调用边界：`infer_call_expr` 在 `infer_expr(func)` 之后立即快照
                // （take）并经参数传入本次调用的证据判定（E1113 方法面，
                // issue #538）；实参推断对状态位的覆盖不参与判定，非
                // FieldAccess 被调方在调用边界丢弃并清残留。
                self.last_method_binding_key = None;
                let obj_ty = self.infer_expr(obj)?;
                let obj_ty = self.solver.resolve_type(&obj_ty);

                // 解包所有 Ref 层，用于字段/方法查找
                // 例如 &Point -> Point, &&Point -> Point
                let mut resolved = obj_ty.clone();
                while let MonoType::Ref { inner, .. } = resolved {
                    resolved = *inner;
                }
                let resolved = self.solver.resolve_type(&resolved);

                let namespace_path = extract_namespace_path(obj);

                // RFC-010: 和类型的字段访问拒绝——变体名升格（全有或全无），
                // 运行时 tagged union 不携带字段表；构造用 `类型.变体(...)`（在
                // Call 臂拦截），取回载荷用 match 变体解构（RFC-039）。
                // 方法调用（`r.is_failure()`，Try 接口方法）除外：先查方法绑定表
                // （含跨模块导入的 yx std 方法），命中即按方法返回，不落字段拒绝。
                if let MonoType::Generic { name: sum_name, .. } = &resolved {
                    if let Some(sum_variants) = self.sum_variants(sum_name).cloned() {
                        if sum_variants.iter().any(|v| v.name == *field) {
                            return Err(ErrorCodeDefinition::variant_used_as_field(
                                sum_name.bare(),
                                field,
                            )
                            .build());
                        }
                        // 方法身份由导入面解析（D-5）：拼字符串不再产生身份，无身份即不查表
                        if let Some(key) = self.receiver_method_key(obj, sum_name, field) {
                            if let Some(method_ty) = self.method_bindings.get(&key) {
                                self.last_method_binding_key = Some(key);
                                return Ok(method_ty.clone());
                            }
                        }
                        return Err(
                            ErrorCodeDefinition::field_not_found(field, sum_name.bare()).build()
                        );
                    }
                }

                // 泛型类型实例展开：`Box(T)` 是 `Generic{name:"Box", args:[T]}`，
                // 字段定义存在 generic_type_defs 里而非 struct 表。若目标名命中
                // 泛型类型定义，先实例化成 Struct 再做字段/方法查找——
                // 非泛型结构体（Point）走原有 Struct 路径不受影响。
                let resolved = match &resolved {
                    MonoType::Generic { name, args }
                        if self.generic_type_defs.contains_key(name.bare()) =>
                    {
                        match crate::frontend::core::sema::TypeEnvironment::instantiate_generic_type(
                            &self.generic_type_defs[name.bare()],
                            args,
                        ) {
                            Ok(inst) => {
                                // 3.4.1：不可求值 const 约束事实上达（W1063 汇聚链）
                                self.unevaluable_const_constraints.extend(
                                    inst.unevaluable_constraints
                                        .iter()
                                        .map(|c| (*span, c.clone())),
                                );
                                self.solver.resolve_type(&inst.ty)
                            }
                            Err(_) => resolved,
                        }
                    }
                    _ => resolved,
                };
                if let Some(ns_path) = namespace_path {
                    let full_path = format!("{}.{}", ns_path, field);
                    if let Some(sig) = self.native_signatures.get(&full_path).cloned() {
                        return Ok(sig);
                    }
                    if self
                        .native_signatures
                        .keys()
                        .any(|k| k.starts_with(&full_path))
                    {
                        let fn_ty = MonoType::Fn {
                            params: vec![self.solver.new_var()],
                            return_type: Box::new(MonoType::Void),
                        };
                        return Ok(fn_ty);
                    }
                }

                // 底部类型 vacuous（issue #469 处 3）：毒化登记（`Never` 占位）的
                // 变量上取字段不施加约束——与 infer_binary 的 Never 短路同款，
                // 防止 E1001 消了、E1053「non-struct type 'never'」又冒出来
                // （实测 generic_ctor_type_mismatch_err 语料的 `a.value`）。
                // 字段类型无信息可依，取 fresh 推迟。
                if matches!(resolved, MonoType::Never) {
                    return Ok(self.solver.new_var());
                }

                match resolved {
                    // 元组下标：`t.0` / `t.1`。解析期已把数字下标转成十进制字段名，
                    // 这里按下标取元素类型。此前只支持 `t[0]` 形态（见索引表达式臂）。
                    MonoType::Generic { ref name, ref args } if name == "Tuple" => {
                        match field.parse::<usize>() {
                            Ok(i) if i < args.len() => Ok(args[i].clone()),
                            _ => Err(ErrorCodeDefinition::index_out_of_bounds(
                                args.len(),
                                field.parse::<i64>().unwrap_or(-1),
                            )
                            .build()),
                        }
                    }
                    // RFC-011 容器命名分层：`Vec(T)` / `Array(T, N)` 的长度。
                    // 两者以 `length: Int` 暴露长度（与 Range 具名字字段同款处理）：
                    // - `Array(T, N)`：N 编译期已知，但统一走同一读取路径，避免两套语义。
                    // - `Vec(T)`：运行时长度，底层缓冲的长度。
                    // - `List(T)`：库类型自己维护的 length 字段，不经此处。
                    MonoType::Generic { ref name, .. }
                        if (name == "Vec" || name == "Array") && field == "length" =>
                    {
                        Ok(MonoType::Int(64))
                    }
                    // #302：Range 具名字段（start/end/step）
                    MonoType::Generic { ref name, .. } if name == "Range" => {
                        if matches!(field.as_str(), "start" | "end" | "step") {
                            Ok(MonoType::Int(64))
                        } else {
                            // 回退方法查找（std.range 协议面）
                            // 方法身份由导入面解析（D-5）：拼字符串不再产生身份
                            if let Some(key) = self.receiver_method_key(obj, name, field) {
                                if let Some(method_ty) = self.method_bindings.get(&key) {
                                    self.last_method_binding_key = Some(key);
                                    return Ok(method_ty.clone());
                                }
                            }
                            Err(ErrorCodeDefinition::field_not_found(field, "Range").build())
                        }
                    }
                    MonoType::Struct(struct_type) => {
                        for field_info in &struct_type.fields {
                            if &field_info.name == field {
                                // 6.9.5-B.1b：witness 字段无运行期数据槽，值级读取
                                // 一律拒绝（E1114）；类型级投影留 B.3。
                                if field_info.is_witness() {
                                    return Err(ErrorCodeDefinition::witness_field_not_readable(
                                        struct_type.name.bare(),
                                        field,
                                    )
                                    .at(*span)
                                    .build());
                                }
                                return Ok(field_info.ty.clone());
                            }
                        }
                        // Field not found in struct — try method lookup
                        // 方法身份由导入面解析（D-5）：拼字符串不再产生身份，无身份即不查表
                        if let Some(key) = self.receiver_method_key(obj, &struct_type.name, field) {
                            if let Some(method_ty) = self.method_bindings.get(&key) {
                                self.last_method_binding_key = Some(key);
                                return Ok(method_ty.clone());
                            }
                        }
                        // #396/#289：模块别名上取成员失败——模块不是 struct，
                        // 报 E1043（模块语义 + 可用导出清单），不得借道 E1042。
                        if self
                            .module_aliases
                            .is_some_and(|aliases| aliases.contains(struct_type.name.bare()))
                        {
                            let available = struct_type
                                .fields
                                .iter()
                                .map(|f| f.name.as_str())
                                .collect::<Vec<_>>()
                                .join(", ");
                            return Err(ErrorCodeDefinition::module_has_no_export(
                                struct_type.name.bare(),
                                field,
                                &available,
                            )
                            .build());
                        }
                        Err(
                            ErrorCodeDefinition::field_not_found(field, struct_type.name.bare())
                                .build(),
                        )
                    }
                    MonoType::TypeRef(ref type_name) => {
                        // Try to resolve TypeRef → Struct via type_defs for field lookup
                        if let Some(def_ty) = self.type_def(type_name).map(|p| &p.body) {
                            let def_ty = self.solver.resolve_type(def_ty);
                            if let MonoType::Struct(ref struct_type) = def_ty {
                                for field_info in &struct_type.fields {
                                    if &field_info.name == field {
                                        // 6.9.5-B.1b：注解类型通常是 TypeRef，经
                                        // type_defs 解析后才看得到字段量词；witness
                                        // 字段同样在值级拒绝。
                                        if field_info.is_witness() {
                                            return Err(
                                                ErrorCodeDefinition::witness_field_not_readable(
                                                    struct_type.name.bare(),
                                                    field,
                                                )
                                                .at(*span)
                                                .build(),
                                            );
                                        }
                                        return Ok(field_info.ty.clone());
                                    }
                                }
                                // Field not found in resolved struct — try method lookup
                                // 方法身份由导入面解析（D-5）：拼字符串不再产生身份，无身份即不查表
                                if let Some(key) =
                                    self.receiver_method_key(obj, &struct_type.name, field)
                                {
                                    if let Some(method_ty) = self.method_bindings.get(&key) {
                                        self.last_method_binding_key = Some(key);
                                        return Ok(method_ty.clone());
                                    }
                                }
                                return Err(ErrorCodeDefinition::field_not_found(
                                    field,
                                    struct_type.name.bare(),
                                )
                                .build());
                            }
                        }
                        // Try method lookup on TypeRef (generic type or forward reference)
                        // 方法身份由导入面解析（D-5）：拼字符串不再产生身份，无身份即不查表
                        if let Some(key) = self.receiver_method_key(obj, type_name, field) {
                            if let Some(method_ty) = self.method_bindings.get(&key) {
                                self.last_method_binding_key = Some(key);
                                return Ok(method_ty.clone());
                            }
                        }
                        Err(
                            ErrorCodeDefinition::field_access_on_non_struct(&format!("{}", obj_ty))
                                .build(),
                        )
                    }
                    _ => Err(ErrorCodeDefinition::field_access_on_non_struct(&format!(
                        "{}",
                        obj_ty
                    ))
                    .build()),
                }
            }

            // 函数调用
            crate::ast::Expr::Call {
                func,
                args,
                named_args,
                span,
                plan_id,
            } => {
                let ty = self.infer_call_expr(func, args, named_args, *span, *plan_id)?;
                // 调用点结果类型落表（RFC-039 D20：与 `overload_resolutions` 同一身份）。
                // 限定名 / native / 方法 / 泛型调用的**最终**类型只有 sema 有：IR 侧按
                // 被调名反查原生签名取不到，值表遂落显式未知。
                //
                // 这里存**原样**推断结果：此刻 zonk 太早——`dict.new()` 的
                // `Dict(K, V)` 要等后续使用点（`dict.set(d, "a", 1)`）才收敛。收敛
                // 与「未收敛即不落」的过滤在导出点
                // （`StatementChecker::finalize_expr_result_types`）做一次；身份未分配
                // 的合成调用（`PlanId::UNPLANNED`）不进表。
                if plan_id.is_planned() {
                    self.expr_result_types.insert(*plan_id, ty.clone());
                }
                Ok(ty)
            }

            // If 表达式
            crate::ast::Expr::If {
                condition,
                then_branch,
                else_if_branches,
                else_branch,
                ..
            } => self.infer_if_expr(
                condition,
                then_branch,
                else_if_branches,
                else_branch.as_deref(),
            ),

            // While 表达式
            crate::ast::Expr::While {
                condition, body, ..
            } => {
                let cond_ty = self.infer_expr(condition)?;
                if cond_ty != MonoType::Bool {
                    return Err(ErrorCodeDefinition::condition_type_mismatch(&format!(
                        "{}",
                        cond_ty
                    ))
                    .build());
                }

                self.loop_depth += 1;

                self.scope.enter_block();
                let result = self.infer_block(body, true, None);
                // 退出循环作用域时，将内部变量提升到外层，避免变量丢失
                self.promote_loop_vars_to_parent_scope();

                self.loop_depth -= 1;

                result?;
                Ok(MonoType::Void)
            }

            // For 循环
            crate::ast::Expr::For {
                var,
                var_mut,
                iterable,
                body,
                span,
            } => self.infer_for_loop(var, *var_mut, iterable, body, *span),

            // Return 表达式
            crate::ast::Expr::Return(expr, span) => {
                // RFC-010a 规则② 边界条件：模块初始化层（Script 顶层）没有函数
                // 边界可退——编译期拒绝（E1109），不再运行期静默终止初始化、
                // 跳过后续顶层语句
                if self.fn_depth == 0 {
                    return Err(ErrorCodeDefinition::return_outside_function()
                        .at(*span)
                        .build());
                }
                if let Some(e) = expr {
                    let ret_ty = self.infer_expr(e)?;
                    // If we know the expected return type, check that the return
                    // expression type matches it via unification.
                    let expected = self.expected_return_type.clone();
                    if let Some(ref expected) = expected {
                        // 诊断口径与语句形态的 return 收一叉（WBS 6.8.2a / D54 ②）：
                        // `return X` 有两个 AST 形状——箭头糖 `=> X` 成 StmtKind::Return
                        //（parser/pratt/led.rs），显式 `return X` 成 StmtKind::Expr(Expr::Return)
                        //（parser/statements/control_flow.rs）。两者是**同一语义**，却各走
                        // 一个检查函数：语句形态报 E1012（指向注解，check_return_value_type），
                        // 表达式形态此前报 E1002（指向载荷）。实测同一 `f: () -> Int = ...`
                        // 的 `{ "hello" }` / `{ return "hello" }` 两格：一个 E1012、一个
                        // E1002。此处改用 E1012 并把位置指向**注解**（#353 口径：真正该改的是
                        // 声明，不是体里那个可能完全正确的值）。
                        let at = self.return_annotation_span.unwrap_or(*span);
                        // 返回位若含**未绑定的类型参数**（`List(TypeRef "A")`，A 是所在
                        // 泛型的参数），先按名把该参数绑定到实际返回类型，再做常规 unify。
                        // 否则 `unify(List(Int), List(TypeRef "A"))` 因「TypeRef 与具体类型
                        // 无统一规则」而报错——而这里 A := Int 是合法的。
                        let bound_expected = Self::bind_type_params_from(expected, &ret_ty);
                        if bound_expected != *expected {
                            decide(
                                self.solver,
                                &ret_ty,
                                &bound_expected,
                                Some(at),
                                self.type_defs,
                                self.dep_env,
                            )
                            .map_err(|failure| {
                                failure.or_site(
                                    ErrorCodeDefinition::return_type_mismatch(
                                        &format!("{}", bound_expected),
                                        &format!("{}", ret_ty),
                                    )
                                    .at(at)
                                    .build(),
                                )
                            })?;
                        } else {
                            decide(
                                self.solver,
                                &ret_ty,
                                expected,
                                Some(at),
                                self.type_defs,
                                self.dep_env,
                            )
                            .map_err(|failure| {
                                failure.or_site(
                                    ErrorCodeDefinition::return_type_mismatch(
                                        &format!("{}", expected),
                                        &format!("{}", ret_ty),
                                    )
                                    .at(at)
                                    .build(),
                                )
                            })?;
                        }
                        // RFC-011a §6: 具体值返回进存在类型返回位 → 检查实现并记录包装点
                        self.collect_existential_coercions(e, expected)?;
                    }
                    Ok(ret_ty)
                } else {
                    Ok(MonoType::Void)
                }
            }

            // Break 表达式
            crate::ast::Expr::Break(span) => {
                // #311：循环控制流仅允许在 while/for 体内（spawn for 体在函数边界已重置深度）
                if self.loop_depth == 0 {
                    return Err(ErrorCodeDefinition::break_outside_loop("break")
                        .at(*span)
                        .build());
                }
                Ok(MonoType::Void)
            }

            // Continue 表达式
            crate::ast::Expr::Continue(span) => {
                if self.loop_depth == 0 {
                    return Err(ErrorCodeDefinition::break_outside_loop("continue")
                        .at(*span)
                        .build());
                }
                Ok(MonoType::Void)
            }

            // Cast 表达式
            crate::ast::Expr::Cast {
                expr, target_type, ..
            } => {
                let _ = self.infer_expr(expr)?;
                let target_mono: MonoType = target_type.clone().into();
                Ok(target_mono)
            }

            // Block 表达式
            crate::ast::Expr::Block(block) => {
                // #394：空块 `{}` 落入容器期望位——B 方案定案 `{}` = 空块（值
                // Void），空字典钦定写法是 `dict.new()`（SPEC syntax §1.6.4）。
                // 期望类型可得不匹配时直接给出引导，不让用户在
                // E1002 "found void" 上空转（期望非容器时不干预）。
                if block.stmts.is_empty() {
                    if let Some(expected) = &self.current_expected {
                        if Self::is_container_family(&self.solver.resolve_type(expected)) {
                            return Err(ErrorCodeDefinition::empty_block_as_container()
                                .at(block.span)
                                .build());
                        }
                    }
                }
                self.infer_block(block, true, None)
            }

            // Lambda 表达式
            crate::ast::Expr::Lambda {
                params,
                body,
                span: _span,
                plan_id,
                ..
            } => {
                let ty = self.infer_lambda(params, body, None)?;
                self.record_lambda_result_type(*plan_id, &ty);
                Ok(ty)
            }

            // Match 表达式
            crate::ast::Expr::Match { expr, arms, span } => {
                self.check_match_expr(expr, arms, *span)
            }

            // Try 表达式: expr?（RFC-011b 阶段 2 定案：接口驱动，不硬绑 Result）
            crate::ast::Expr::Try { expr, span } => self.infer_try_expr(expr, *span),

            // Ref 表达式
            crate::ast::Expr::Ref { expr, .. } => {
                let expr_ty = self.infer_expr(expr)?;
                Ok(MonoType::Generic {
                    name: Nominal::flat(AsRef::<str>::as_ref(&("Arc")))
                        .expect("type name must be bare and non-empty"),
                    args: vec![expr_ty],
                })
            }

            // Unsafe 块
            crate::ast::Expr::Unsafe { body, .. } => {
                // RFC-010：块内定义的类型名提升到当前作用域（供尾表达式引用），
                // 须在检查块体**之前**注册，否则 `T` 报 E1001。
                for st in &body.stmts {
                    if let crate::ast::StmtKind::TypeDefinition { name, .. } = &st.kind {
                        if self.scope.get_var(name).is_none() {
                            // 类型值的运行时表示是 Void（编译期构造）
                            self.scope.add_var(
                                name.clone(),
                                PolyType::mono(MonoType::Void),
                                false,
                                st.span,
                            );
                        }
                    }
                }
                self.unsafe_depth += 1;
                let r = self.infer_block(body, true, None);
                self.unsafe_depth -= 1;
                r
            }

            // spawn 块：spawn { ... }
            crate::ast::Expr::Spawn { body, .. } => {
                // `spawn {}` 引入**新的函数边界**：块内 `return` 退出的是 spawn 体
                // 而非外层函数（运行时已验证：`r = spawn { return 3 + 4 }` 得 r == 7）。
                //
                // 此前 `expected_return_type` 未隔离，保持外层函数声明的类型，
                // 导致 `f: () -> Void = { r = spawn { return 1 + 2 } }` 误报
                // E1002（把 spawn 体的 return 当外层 void 函数的 return）。
                let saved = self.expected_return_type.take();
                // spawn 体的 return 也不属外层声明：注解位置随之隔离（同 expected）
                let saved_ann_span = self.return_annotation_span.take();
                // spawn 体是函数边界（RFC-010a 规则②）：体内 return 退出 spawn（E1109 判定）
                let saved_fn_depth = self.fn_depth;
                self.fn_depth += 1;
                // 6.9.4 P5 / T17(c)：spawn 体也是**Γ 证据边界**（新函数边界，体内
                // return 退出的是 spawn 体）——外层守卫事实不入体，体内自产事实随体亡。
                let saved_gamma_barrier =
                    self.gamma.as_deref_mut().map(|gamma| gamma.enter_barrier());
                let result = self.infer_block(body, true, None);
                if let (Some(gamma), Some(saved)) = (self.gamma.as_deref_mut(), saved_gamma_barrier)
                {
                    gamma.restore_barrier(saved);
                }
                self.fn_depth = saved_fn_depth;
                self.expected_return_type = saved;
                self.return_annotation_span = saved_ann_span;
                result
            }

            // ListComp 表达式
            // #401：按生成器子句顺序绑定——第 i 个子句的 iterable/condition
            // 可引用先前子句的迭代变量；过滤条件强制 Bool（同 if 推断）
            crate::ast::Expr::ListComp {
                element,
                generators,
                ..
            } => {
                self.scope.enter_block();

                for gen in generators {
                    let iter_ty = self.infer_expr(&gen.iterable)?;
                    // 循环变量类型从可迭代对象取（此前硬编码 Char，靠
                    // 算术 fresh-var 兜底蒙混；硬化后按 check_for_stmt 同款分发）
                    let loop_var_ty = match &iter_ty {
                        m if m.is_list() || m.is_vec() || m.is_array() => {
                            m.generic_args().unwrap()[0].clone()
                        }
                        m if m.is_string() => MonoType::Char,
                        m if m.is_dict() => {
                            let args = m.generic_args().unwrap();
                            MonoType::make_tuple(vec![args[0].clone(), args[1].clone()])
                        }
                        _ => self.solver.resolve_type(&iter_ty),
                    };

                    self.scope.add_var(
                        gen.var.clone(),
                        PolyType::mono(loop_var_ty),
                        false,
                        gen.span,
                    );

                    if let Some(cond) = &gen.condition {
                        let cond_ty = self.infer_expr(cond)?;
                        if cond_ty != MonoType::Bool {
                            return Err(ErrorCodeDefinition::condition_type_mismatch(&format!(
                                "{}",
                                cond_ty
                            ))
                            .build());
                        }
                    }
                }

                let elem_ty = self.infer_expr(element)?;

                self.scope.exit_block();

                Ok(MonoType::make_list(elem_ty))
            }

            // RFC-012: F-string 类型推断
            // f-string 总是返回 String 类型
            crate::ast::Expr::FString { segments, .. } => {
                // 验证每个插值表达式的类型
                for segment in segments {
                    if let crate::ast::FStringSegment::Interpolation { expr, .. } = segment {
                        let _expr_ty = self.infer_expr(expr)?;
                        // 所有类型都支持转换为 String（通过 format()）
                    }
                }
                Ok(MonoType::make_string())
            }

            // 错误恢复占位符：返回新类型变量，不会导致 panic
            crate::ast::Expr::Error(span) => Err(ErrorCodeDefinition::invalid_syntax("缺失表达式")
                .at(*span)
                .build()),

            // 借用表达式：&expr 或 &mut expr
            // TODO: 详细类型检查将在后续任务中实现
            crate::ast::Expr::Borrow {
                mutable,
                expr: inner,
                span,
            } => {
                let inner_ty = self.infer_expr(inner)?;
                // 6.9.4 P7：`&mut name` 把名字借出去做可变访问 ⇒ 该名的常量等式事实不再
                // 成立（保守方向：kill 多了只需重新 assert，不会用旧证明保证新值）。
                // 只有直接写形态（Var）进 kill set；索引/字段/别名的写面不在本批。
                if *mutable {
                    if let crate::ast::Expr::Var(name, _) = inner.as_ref() {
                        if let Some(gamma) = self.gamma.as_deref_mut() {
                            gamma.kill(name);
                        }
                    }
                }
                // 6.9.5-B.1c / 5A：witness-bearing 记录整体不可变——`&mut` 拒绝。
                // witness 记录的证据依赖「构造后字段不变」；可变借出即可写回。
                if *mutable && self.is_witness_bearing_type(&inner_ty) {
                    return Err(ErrorCodeDefinition::witness_record_immutable(&format!(
                        "{}",
                        inner_ty
                    ))
                    .at(*span)
                    .build());
                }
                Ok(MonoType::Ref {
                    mutable: *mutable,
                    inner: Box::new(inner_ty),
                })
            }

            // spawn for 数据并行循环（RFC-024 §2.4）
            crate::ast::Expr::SpawnFor {
                var,
                var_mut,
                iterable,
                body,
                span,
                ..
            } => {
                // 1. 检查 iterable 类型，推导元素类型
                let iter_ty = self.infer_expr(iterable)?;

                let element_type = match &iter_ty {
                    MonoType::Generic { name, args } if name == "List" => args[0].clone(),
                    MonoType::Generic { name, args } if name == "Range" && args.len() == 1 => {
                        args[0].clone()
                    }
                    MonoType::Generic { name, .. } if name == "String" => MonoType::Char,
                    MonoType::Generic { name, .. } if name == "Tuple" => self.solver.new_var(),
                    MonoType::Generic { name, args } if name == "Dict" => {
                        MonoType::make_tuple(vec![args[0].clone(), args[1].clone()])
                    }
                    _ => self.solver.new_var(),
                };

                // 2. 进入循环作用域，注册迭代变量
                // #311：spawn for 体是逐元素闭包执行，不是可 break/continue 的循环上下文
                self.scope.enter_block();
                let body_ty = self
                    .try_add_var(var.clone(), PolyType::mono(element_type), *span, *var_mut)
                    .and_then(|_| self.infer_block(body, true, None));

                self.promote_loop_vars_to_parent_scope();

                match body_ty {
                    Ok(ty) => {
                        // spawn for 返回 List(T)，T 是循环体返回类型
                        if matches!(ty, MonoType::Void) {
                            Ok(MonoType::make_list(MonoType::Void))
                        } else {
                            Ok(MonoType::make_list(ty))
                        }
                    }
                    Err(e) => Err(e),
                }
            }
        }
    }

    /// `e?` 的类型判定（自 `infer_expr` 的 Try 臂抽出）。
    ///
    /// RFC-011b 阶段 2：`?` 由 `Try` 接口驱动——接收者须实现 Try，
    /// 外层函数返回类型也须实现，且 E 位接得住残余类型。
    fn infer_try_expr(
        &mut self,
        expr: &crate::ast::Expr,
        span: crate::util::span::Span,
    ) -> Result<MonoType> {
        let inner_ty = self.infer_expr(expr)?;
        let mut resolved = self.solver.resolve_type(&inner_ty);
        while let MonoType::Ref { inner, .. } = resolved {
            resolved = *inner;
        }
        let resolved = self.solver.resolve_type(&resolved);

        // 1) 接收者类型必须实现 Try（登记表查询，Self 位名义匹配 +
        //    抽象条目按 scrutinee 实参实例化）
        let Some((impl_name, ok_ty, err_ty)) =
            operator_interfaces::query_try_impl(self.interface_impl_registry, &resolved)
        else {
            return Err(
                ErrorCodeDefinition::try_requires_result(&resolved.to_string())
                    .at(span)
                    .build(),
            );
        };

        // 2) 外层函数返回类型必须也实现 Try，且 E 位接得住 err_ty
        //    （`?` 的失败值经 from_error 桥沿返回类型传播）
        let Some(expected_ret) = self.expected_return_type.clone() else {
            return Err(ErrorCodeDefinition::try_only_allowed_in_result()
                .at(span)
                .build());
        };
        let mut expected = self.solver.resolve_type(&expected_ret);
        while let MonoType::Ref { inner, .. } = expected {
            expected = *inner;
        }
        let expected = self.solver.resolve_type(&expected);
        match operator_interfaces::query_try_impl(self.interface_impl_registry, &expected) {
            Some((_, _, ret_err_ty)) => {
                if let Err(failure) = decide(
                    self.solver,
                    &ret_err_ty,
                    &err_ty,
                    Some(span),
                    self.type_defs,
                    self.dep_env,
                ) {
                    return Err(failure.or_site(
                        ErrorCodeDefinition::try_error_type_mismatch(
                            &self.solver.resolve_type(&ret_err_ty).to_string(),
                            &self.solver.resolve_type(&err_ty).to_string(),
                        )
                        .at(span)
                        .build(),
                    ));
                }
            }
            None => {
                return Err(ErrorCodeDefinition::try_only_allowed_in_result()
                    .at(span)
                    .build());
            }
        }

        // 记录实现的**类型身份**（ir_gen 按同一 AST span 查表命名四方法调用）：
        // 接收者类型自身携带名义头（跨模块同名 Try 类型按定义模块限定），
        // 拿不到名义头时才退回登记名（接口实现表的 impl_type 是定义侧裸名）。
        let impl_identity = match &resolved {
            MonoType::Generic { name, .. } => Some(name.clone()),
            MonoType::Struct(s) => Some(s.name.clone()),
            MonoType::TypeRef(n) => Some(n.clone()),
            _ => None,
        }
        .unwrap_or_else(|| self.nominal_for(&impl_name));
        self.try_expr_impls.push((span, impl_identity));

        Ok(ok_ty)
    }

    /// #394：容器族判定——泛型容器构造器及其借用形态（`&Dict(K, V)` 等）。
    /// 期望为 Fn 的块作值形态（`main = { ... }`）天然排除在外。
    pub(super) fn is_container_family(ty: &MonoType) -> bool {
        match ty {
            MonoType::Generic { name, .. } => {
                matches!(
                    name.as_str(),
                    "Dict" | "List" | "Vec" | "Array" | "Set" | "Tuple"
                )
            }
            MonoType::Ref { inner, .. } => Self::is_container_family(inner),
            _ => false,
        }
    }

    /// 推断代码块的类型
    ///
    /// 语义（RFC-010）：
    /// - `{}` 块的值 = 块内 `return expr` 的 `expr` 的值
    /// - 没有 `return` = Void
    /// - 尾随表达式不影响块的返回类型
    pub fn infer_block(
        &mut self,
        block: &crate::ast::Block,
        _allow_unit: bool,
        _expected_type: Option<&MonoType>,
    ) -> Result<MonoType> {
        // spec §2.15：每个 `{}` 块创建一个作用域——
        // 内层可读外层，**外层不可读内层**。
        //
        // 此前本函数不开作用域，导致内层块变量泄漏到外层可见
        //（`{ inner = 1 }; print(inner)` 不报未定义），进而使 spec §4.3 的
        // 跨作用域规则（E2010 / E2013）无从判定。
        self.scope.enter_block();
        let result = (|| {
            // RFC-010a 规则①：块的值 = 尾表达式（唯一出口）。
            // 空块 `{}` → Void；末位为语句（如赋值）→ Void；否则为尾表达式类型。
            //
            // 与 `return` 的关系（规则②）：`return : Never`。带 `return` 的分支经
            // `join` 被忽略（`Never` 不参与合并），故不再需要单独收集 `return` 类型。
            let mut block_ty: MonoType = MonoType::Void;

            let last_idx = block.stmts.len().checked_sub(1);
            for (i, stmt) in block.stmts.iter().enumerate() {
                let ty = self.infer_stmt(stmt)?;
                if Some(i) == last_idx {
                    block_ty = ty;
                }
            }

            Ok(block_ty)
        })();
        self.scope.exit_block();
        result
    }

    /// #313：for 循环推断——`Expr::For`（表达式位置）与 `StmtKind::For`
    /// （infer_stmt，spawn 体/循环体内）共用，保证两条路径的元素类型推导
    /// 与循环上下文（E1102）一致。
    fn infer_for_loop(
        &mut self,
        var: &str,
        var_mut: bool,
        iterable: &crate::ast::Expr,
        body: &crate::ast::Block,
        span: crate::util::span::Span,
    ) -> Result<MonoType> {
        let iter_ty = self.infer_expr(iterable)?;

        let element_type = match &iter_ty {
            MonoType::Generic { name, args } if name == "List" => args[0].clone(),
            MonoType::Generic { name, args } if name == "Range" && args.len() == 1 => {
                args[0].clone()
            }
            MonoType::Generic { name, .. } if name == "String" => MonoType::Char,
            MonoType::Generic { name, .. } if name == "Tuple" => self.solver.new_var(),
            MonoType::Generic { name, args } if name == "Dict" => {
                MonoType::make_tuple(vec![args[0].clone(), args[1].clone()])
            }
            _ => self.solver.new_var(),
        };

        self.loop_depth += 1;

        self.scope.enter_block();
        let result = self
            .try_add_var(var.to_string(), PolyType::mono(element_type), span, var_mut)
            .and_then(|_| self.infer_block(body, true, None));

        // 退出循环作用域时，将内部变量提升到外层，避免变量丢失
        self.promote_loop_vars_to_parent_scope();

        self.loop_depth -= 1;
        result
    }

    /// #313：if 推断——`Expr::If`（表达式位置）与 `StmtKind::If`
    /// （infer_stmt，spawn 体/循环体内）共用。
    fn infer_if_expr(
        &mut self,
        condition: &crate::ast::Expr,
        then_branch: &crate::ast::Block,
        else_if_branches: &[(Box<crate::ast::Expr>, Box<crate::ast::Block>)],
        else_branch: Option<&crate::ast::Block>,
    ) -> Result<MonoType> {
        let cond_ty = self.infer_expr(condition)?;
        if cond_ty != MonoType::Bool {
            return Err(
                ErrorCodeDefinition::condition_type_mismatch(&format!("{}", cond_ty)).build(),
            );
        }

        self.scope.enter_block();
        let then_result = self.infer_block(then_branch, true, None);
        self.scope.exit_block();
        let _then_ty = then_result?;

        for (else_if_cond, else_if_block) in else_if_branches {
            let else_if_cond_ty = self.infer_expr(else_if_cond)?;
            if else_if_cond_ty != MonoType::Bool {
                return Err(ErrorCodeDefinition::condition_type_mismatch(&format!(
                    "{}",
                    else_if_cond_ty
                ))
                .build());
            }
            self.scope.enter_block();
            let else_if_result = self.infer_block(else_if_block, true, None);
            self.scope.exit_block();
            let _ = else_if_result?;
        }

        if let Some(else_block) = else_branch {
            self.scope.enter_block();
            let else_result = self.infer_block(else_block, true, None);
            self.scope.exit_block();
            else_result
        } else {
            Ok(MonoType::Void)
        }
    }

    /// 推断语句的类型
    ///
    /// RFC-010a 规则①：返回该语句作为块尾表达式时贡献的值类型——
    /// 表达式语句取表达式类型；赋值语句为 `Void`；`return` 为 `Never`（规则②）；
    /// 控制流语句取其块值。
    pub fn infer_stmt(
        &mut self,
        stmt: &crate::ast::Stmt,
    ) -> Result<MonoType> {
        // #324：挂当前节点 span，walk 内诊断构造自动获得位置
        let _current_span = crate::util::diagnostic::push_current_span(stmt.span);
        match &stmt.kind {
            // RFC-010a 规则①：语句的值——表达式语句取其表达式类型；赋值语句为
            // Void；`return` 为 Never（规则②）；控制流语句取其块值/运算值。
            crate::ast::StmtKind::Expr(expr) => self.infer_expr(expr),
            crate::ast::StmtKind::Assign {
                target,
                type_annotation,
                value,
                is_mut,
                span: stmt_span,
                ..
            } => {
                use crate::ast::Expr;
                let name = match target.as_ref() {
                    Expr::Var(n, _) => n.clone(),
                    _ => return Ok(MonoType::Void),
                };
                // 如果 value 是 Lambda，走函数推断
                if let Some(v) = value {
                    if let Expr::Lambda { params, .. } = v.as_ref() {
                        let param_types: Vec<MonoType> =
                            params.iter().map(|_| self.solver.new_var()).collect();
                        let return_type = type_annotation
                            .as_ref()
                            .map_or(MonoType::Void, |t| t.clone().into());
                        let fn_type = MonoType::Fn {
                            params: param_types,
                            return_type: Box::new(return_type),
                        };
                        self.try_add_var(
                            name.clone(),
                            PolyType::mono(fn_type),
                            *stmt_span,
                            *is_mut,
                        )?;
                        // #313：注册后仍需推断 lambda 体（Lambda 臂带 enter_fn/参数/
                        // 函数边界语义）——此前直接 return，spawn 体/循环体内嵌套
                        // lambda 的体从未被类型检查。注册类型来自注解，保持不变。
                        let _ = self.infer_expr(v)?;
                        return Ok(MonoType::Void);
                    }
                    // RFC-010a 附录D：`name = { ... }` 无注解时按内容推断（块值是尾表达式）；
                    // 非 Fn 注解→块值（立即求值，变量绑定块值类型）。
                    // 此前不论注解一律注册为 0 参函数，与 ir_gen 分流脱节（#343）。
                    if let Expr::Block(..) = v.as_ref() {
                        if crate::ast::Expr::block_binding_is_function(
                            type_annotation.as_ref(),
                            Some(v.as_ref()),
                        ) {
                            let fn_type = MonoType::Fn {
                                params: vec![],
                                return_type: Box::new(
                                    type_annotation
                                        .as_ref()
                                        .map_or(MonoType::Void, |t| t.clone().into()),
                                ),
                            };
                            self.try_add_var(
                                name.clone(),
                                PolyType::mono(fn_type),
                                *stmt_span,
                                *is_mut,
                            )?;
                            // #313：匿名绑定体仍须检查
                            let _ = self.infer_expr(v)?;
                            return Ok(MonoType::Void);
                        }
                        // 块值：走普通变量路径（下方按 value 推断 init_ty）
                    }
                }
                // 普通变量
                let init_ty = if let Some(expr) = value {
                    self.infer_expr(expr)?
                } else {
                    type_annotation
                        .as_ref()
                        .map_or_else(|| self.solver.new_var(), |t| t.clone().into())
                };
                // #313：注解与初值的 unify 校验（镜像 checker 侧 enforce_unify 语义）。
                // 此前 inferrer 侧丢弃注解，spawn 体/循环体内 `t: Int = "hello"`
                // 静默通过并按推断类型注册。
                if let (Some(ann), Some(_)) = (type_annotation, value) {
                    let ann_mono: MonoType = ann.clone().into();
                    // RFC-027 Phase 2.5：绑定注解位的谓词精化正格化——与
                    // `check_var_stmt_inner` 共用同一转换（refine_binding_annotation
                    // 判据单源）。#550：此前本臂直接用 raw Generic 统一，块局部
                    // 绑定（无注解块值 / if 分支 / 循环体 / spawn 体经 infer_block
                    // 走查到这里）的谓词注解不发生 Refined 转换，可满足值被 E1002
                    // 假拒绝。Refined 对统一透明（unify 入口剥基，RFC-027 §6.1）。
                    let ann_mono = statements::refine_binding_annotation(
                        ann,
                        ann_mono,
                        self.proof_fn_bases,
                        self.solver,
                    );
                    decide(
                        self.solver,
                        &init_ty,
                        &ann_mono,
                        Some(*stmt_span),
                        self.type_defs,
                        self.dep_env,
                    )
                    .map_err(|failure| {
                        failure.or_site(
                            ErrorCodeDefinition::type_mismatch(
                                &format!("{}", ann_mono),
                                &format!("{}", init_ty),
                            )
                            .at(*stmt_span)
                            .build(),
                        )
                    })?;
                }
                // spec §4.3「赋值优先」：与 `StatementChecker` 同一判定
                // （`ScopeManager::classify_binding`）——此前两处各写一份，
                // 导致规则不一致（E2010/E2013 在这里有、在那里没有）。
                use crate::frontend::core::sema::inference::scope::BindingAction;
                match self.scope.classify_binding(&name, *is_mut, value.is_none()) {
                    BindingAction::ImmutableReassign => {
                        return Err(ErrorCodeDefinition::immutable_assignment(&name)
                            .at(*stmt_span)
                            .build());
                    }
                    BindingAction::DuplicateDefinition => {
                        return Err(ErrorCodeDefinition::duplicate_definition(&name)
                            .at(*stmt_span)
                            .build());
                    }
                    BindingAction::Shadowing => {
                        return Err(ErrorCodeDefinition::variable_shadowing(&name)
                            .at(*stmt_span)
                            .build());
                    }
                    BindingAction::Reassign => {
                        // 6.9.4 P7 的镜像执行点：本 walker（spawn / lambda 体内的语句）
                        // 与 `StatementChecker::check_var_stmt` 共用**同一** `classify_binding`
                        // 判定，重赋值的 kill 也必须同口径——否则体内自产的等式事实在
                        // `y = …` 之后仍被当作证据（旧值证新值）。
                        if let Some(gamma) = self.gamma.as_deref_mut() {
                            gamma.kill(&name);
                        }
                        self.assign_var(&name, init_ty, *stmt_span, true)?;
                        return Ok(MonoType::Void);
                    }
                    BindingAction::Declare => {}
                }
                self.try_add_var(name.clone(), PolyType::mono(init_ty), *stmt_span, *is_mut)?;
                Ok(MonoType::Void)
            }
            // #313：以下语句种类此前落 `_ => Ok(())` 静默跳过——spawn 体/循环体经
            // infer_block 走到这里，If/For 中的语句从未被类型检查（类型错误编译通过，
            // 仅 IR 层内部错误兜底）。语义与 StatementChecker::check_stmt 对齐；
            // 无兜底臂：新增 StmtKind 变体将编译期报非穷尽 match。
            crate::ast::StmtKind::For {
                var,
                var_mut,
                iterable,
                body,
                ..
            } => {
                self.infer_for_loop(var, *var_mut, iterable, body, stmt.span)?;
                Ok(MonoType::Void)
            }
            crate::ast::StmtKind::If {
                condition,
                then_branch,
                else_if_branches,
                else_branch,
                ..
            } => self.infer_if_expr(
                condition,
                then_branch,
                else_if_branches,
                else_branch.as_deref(),
            ),
            // 元组解构赋值（镜像 StatementChecker::check_stmt 的 DestructureAssign 臂）
            crate::ast::StmtKind::DestructureAssign { names, rhs, span } => {
                let rhs_ty = self.infer_expr(rhs)?;
                let resolved_ty = self.solver.resolve_type(&rhs_ty);
                match &resolved_ty {
                    MonoType::Generic { name, args } if name == "Tuple" => {
                        if args.len() != names.len() {
                            return Err(ErrorCodeDefinition::type_mismatch(
                                &format!("Tuple({})", names.len()),
                                &format!("Tuple({})", args.len()),
                            )
                            .at(*span)
                            .build());
                        }
                        for (name, elem_ty) in names.iter().zip(args.iter()) {
                            self.try_add_var(
                                name.name.clone(),
                                PolyType::mono(elem_ty.clone()),
                                *span,
                                false,
                            )?;
                        }
                        Ok(MonoType::Void)
                    }
                    _ => {
                        for name in names {
                            let ty = self.solver.new_var();
                            self.try_add_var(name.name.clone(), PolyType::mono(ty), *span, false)?;
                        }
                        Ok(MonoType::Void)
                    }
                }
            }
            // RFC-010a 规则②：return : (T) -> Never（非局部退出，退出最近的函数边界）。
            // 值本身仍被推断（与 expected_return_type 统一），但语句类型是 Never，
            // 使其在 join 中被忽略。
            crate::ast::StmtKind::Return(Some(expr)) => {
                self.infer_expr(expr)?;
                Ok(MonoType::Never)
            }
            crate::ast::StmtKind::Return(None) => Ok(MonoType::Never),
            // 类型定义仅模块级合法（E1071，#295）；infer_stmt 只会在 spawn 体/
            // 循环体内遇到它——必为函数上下文，一律报错。
            // 例外（RFC-010）：`unsafe {}` 内允许类型定义（不透明类型封装）。
            crate::ast::StmtKind::TypeDefinition { name, .. } => {
                if self.unsafe_depth > 0 {
                    Ok(MonoType::Void)
                } else {
                    Err(ErrorCodeDefinition::type_def_only_at_module_level(name)
                        .at(stmt.span)
                        .build())
                }
            }
            // use 语句的模块注册依赖 StatementChecker 的环境（process_use_stmt），
            // inferrer 无模块上下文。真实代码中 use 已由 checker 处理；表达式位置的
            // 循环体内出现时显式接受，import 未注册时下游报 E1001（响亮失败）。
            crate::ast::StmtKind::Use { .. } => Ok(MonoType::Void),
            // 错误恢复占位符：报告错误但不 panic（镜像 StatementChecker::check_stmt）
            crate::ast::StmtKind::Error(span) => {
                Err(ErrorCodeDefinition::invalid_syntax("缺失语句")
                    .at(*span)
                    .build())
            }
        }
    }
}
