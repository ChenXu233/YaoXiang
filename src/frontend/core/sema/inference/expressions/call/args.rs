//! 调用裁决族 · 命名/记录实参绑定。
//!
//! 自 `inference/expressions.rs` 纯搬移（WBS 7.0.4）：方法体与文档注释零改动，
//! 仅补跨模块可见性（`pub(super)`）与相对路径（`super::` 改指 crate 路径）。

use crate::frontend::core::sema::judgement::decide;
use crate::util::diagnostic::{ErrorCodeDefinition, Result};
use crate::frontend::core::types::MonoType;

use super::super::ExpressionInferrer;
use crate::frontend::core::sema::inference::arg_binding;

impl<'a> ExpressionInferrer<'a> {
    /// #452：把方法调用的实参绑定到形参槽位，并当场做规则校验。
    ///
    /// 绑定规则（用户 2026-10-14 裁决，主流约定）：
    /// ① 位置实参按声明顺序占前导槽位（第 i 个位置实参 → 第 i 个值形参）；
    /// ② 命名实参按形参名入槽；名字不匹配任何形参 → E1014；
    /// ③ 位置与命名落在同一槽位 → E1111（既不许命名覆盖位置，也不许丢弃位置实参）；
    /// ④ 命名实参之后不得再出现位置实参 → E1111；
    /// ⑤ 同一形参被重复命名 → E1015（字面重复，只有这一条是「重复指定」）；
    /// ⑥ 任一形参槽位空缺 → E1010（缺实参）。
    ///
    /// 返回**逐实参的形参位**（长度 = 位置实参数 + 命名实参数，顺序 = 位置实参
    /// 在前、命名实参按书写序），供类型检查逐参对齐与 IR 生成重排共用——
    /// 实参序两侧同源，不各自重算。
    ///
    /// 形参名对应的是**值形参**（不含接收者位 self）：接收者由字段访问形态提供，
    /// 不是实参，也不可能被命名实参指定。
    /// #452 规则⑥ / #466：候选形参名表与命名实参是否**名字相容**——
    /// 非空名表要求每个命名实参名都在其中；空名表 = 名字不可用，不构成
    /// 不相容证据（宁可不按名匹配，也不拿错位名字匹配）。
    ///
    /// 决议循环（候选淘汰）与回退路径的形参名选择（零命中调用）共用——
    /// 两处各写一份正是「按名筛候选」判据分叉的入口（#466 就是分叉后
    /// 回退侧只认首候选名表导致的误拒）。
    pub(super) fn names_support_named_args(
        names: &[String],
        named_args: &[(String, crate::ast::Expr)],
    ) -> bool {
        names.is_empty() || named_args.iter().all(|(n, _)| names.iter().any(|p| p == n))
    }

    /// #452 / F1：把候选形参表按**元数**分组（元数 = 值形参数 = 形参表长度 - 1），
    /// 每组取首候选的形参名作代表。**只用于元数结算**：组内首个非空名表给出
    /// 该元数的形参长度；同名下同元数候选的**名字可能互不相同**（#466），
    /// 名字层面的判定（按名筛候选 / 未知名报错）不得以本函数的输出为据，
    /// 须走 [`Self::names_support_named_args`] 与全候选名并集。
    ///
    /// **空表 = 名字不可用**（泛型方法声明里的类型级形参被剥离后名字表对不齐，
    /// 登记时留空；见 `add_method_overload_with_params`）：这种条目不构成元数
    /// 证据，不得参与分组——否则元数被误算成 0，`b.get()` 这类合法调用会报 E1010。
    pub(super) fn candidate_arity_groups(lists: &[Vec<String>]) -> Vec<(usize, Vec<String>)> {
        let mut out: Vec<(usize, Vec<String>)> = Vec::new();
        for names in lists.iter() {
            if names.is_empty() {
                continue; // 名字不可用：不构成元数证据
            }
            let arity = names.len() - 1;
            if out.iter().any(|(a, _)| *a == arity) {
                continue;
            }
            out.push((arity, names.clone()));
        }
        out
    }

    /// #466：回退路径（决议零命中 / 决议表缺席）的形参名与签名选择。
    ///
    /// `lists` 是候选声明期形参名表（含接收者位），`cands` 是同序的候选签名表
    /// （三张平行表同源，见 `append_overload_group`）。返回（候选形参名，
    /// 相容候选的签名）——签名仅在「元数相符且名字相容的候选唯一」时给出，
    /// 供逐参类型检查把 E1002 归因到用户所用名字所在的候选。
    ///
    /// 选择规则（调用点注释①–④的落点）：
    /// - 单候选：名字即它的（缺参/超参形态不按实参数筛，那一格也要有名字上下文）；
    /// - 元数相符组（空名表不构成元数证据）内按 [`Self::names_support_named_args`]
    ///   筛：唯一相容 ⇒ 名字与签名都随它；全不相容（名字存在于其它元数的候选）
    ///   ⇒ 取该组首候选的名字；多个相容且名字全同 ⇒ 取之（同形参名不同类型的
    ///   常规重载）；多个相容而名字互异 ⇒ 名字不可判定，返回 None 不猜；
    /// - 无元数相符组（缺参/超参）：取首个非空名表，让元数结算（E1010）有上下文。
    pub(super) fn fallback_named_arg_context(
        lists: &[Vec<String>],
        cands: Option<&Vec<MonoType>>,
        total_args: usize,
        named_args: &[(String, crate::ast::Expr)],
    ) -> (Option<Vec<String>>, Option<MonoType>) {
        if lists.len() == 1 {
            return (lists.first().cloned(), None);
        }
        let matching: Vec<usize> = lists
            .iter()
            .enumerate()
            .filter(|(_, l)| !l.is_empty() && l.len() - 1 == total_args)
            .map(|(i, _)| i)
            .collect();
        if matching.is_empty() {
            return (lists.iter().find(|l| !l.is_empty()).cloned(), None);
        }
        let compatible: Vec<usize> = matching
            .iter()
            .copied()
            .filter(|&i| Self::names_support_named_args(&lists[i], named_args))
            .collect();
        match compatible.as_slice() {
            // 全不相容：名字存在于其它元数的候选——E1014 报在元数相符的重载上
            [] => (Some(lists[matching[0]].clone()), None),
            // 唯一相容：名字与签名都随它（E1002 的归因候选由此定）
            [only] => (
                Some(lists[*only].clone()),
                cands.and_then(|c| c.get(*only)).cloned(),
            ),
            // 多个相容：名字全同才可信
            many @ [first, ..] => {
                let names = &lists[*first];
                if many.iter().all(|&i| &lists[i] == names) {
                    (Some(names.clone()), None)
                } else {
                    (None, None)
                }
            }
        }
    }

    /// 记录构造器的唯一绑定入口（B.2）：构造参数 = **数据字段**（witness 不占槽位），
    /// 位置/命名/争槽/未知名/重复名/缺参都交共享判据 `bind_args_to_params`。
    /// 额外结算「超参」——共享判据只查必需位，上限由各调用面自己声明。
    pub(super) fn bind_record_ctor_args(
        key: &str,
        data_field_names: &[String],
        optional: &[bool],
        args: &[crate::ast::Expr],
        named_args: &[(String, crate::ast::Expr)],
        call_span: crate::util::span::Span,
    ) -> Result<Vec<usize>> {
        let provided = args.len() + named_args.len();
        if provided > data_field_names.len() {
            return Err(ErrorCodeDefinition::argument_count_mismatch(
                key,
                data_field_names.len(),
                provided,
            )
            .at(call_span)
            .build());
        }
        arg_binding::bind_args_to_params(
            key,
            data_field_names,
            args,
            named_args,
            0,
            optional,
            call_span,
        )
    }

    /// 构造字段的逐字段类型检查（E1002；与泛型两层构造同源判据）。
    ///
    /// `Vec` 字段接受 List 字面量（RFC-011 容器命名分层，与变量声明落点同款豁免），
    /// 逐元素 unify 仍执行；其余走 6.9.3d 判定点 `decide`。
    pub(super) fn check_ctor_field_type(
        &mut self,
        field_ty: &MonoType,
        arg_ty: &MonoType,
        arg_expr: &crate::ast::Expr,
        span: crate::util::span::Span,
    ) -> Result<()> {
        // 引用字段（&T / &mut T）：构造实参到引用的桥接归所有权层，B.2 不判
        // （既有语料 Token(&mut Int) / PointView(&Point) 依赖该边界，见 09a B.2a 记录）。
        if matches!(field_ty, MonoType::Ref { .. }) {
            return Ok(());
        }
        // 容器命名分层：List 字面量可落 Vec/List 字段（RFC-011；与变量声明落点
        // 同款豁免），逐元素 unify 仍执行。此前只豁免 Vec 字段——List 字段的
        // 同类落点由本判据补齐（普通构造 arm 原先完全不查，故无回归面）。
        let vec_seed_ok = matches!(
            field_ty,
            MonoType::Generic { name, .. }
                if matches!(name.as_str(), "Vec" | "List" | "Array")
        ) && matches!(arg_expr, crate::ast::Expr::List(_, _));
        if vec_seed_ok {
            if let (
                Some(MonoType::Generic {
                    args: field_args, ..
                }),
                MonoType::Generic { args: arg_args, .. },
            ) = (Some(field_ty), &self.solver.resolve_type(arg_ty))
            {
                if let (Some(field_element), Some(arg_element)) =
                    (field_args.first(), arg_args.first())
                {
                    if let Err(failure) = decide(
                        self.solver,
                        field_element,
                        arg_element,
                        Some(span),
                        self.type_defs,
                        self.dep_env,
                    ) {
                        return Err(failure.or_site(
                            ErrorCodeDefinition::type_mismatch(
                                &format!("{}", field_element),
                                &format!("{}", arg_element),
                            )
                            .at(span)
                            .build(),
                        ));
                    }
                }
            }
            return Ok(());
        }
        if let Err(failure) = decide(
            self.solver,
            field_ty,
            arg_ty,
            Some(span),
            self.type_defs,
            self.dep_env,
        ) {
            return Err(failure.or_site(
                ErrorCodeDefinition::type_mismatch(
                    &format!("{}", field_ty),
                    &format!("{}", arg_ty),
                )
                .at(span)
                .build(),
            ));
        }
        Ok(())
    }

    /// 方法调用的实参绑定：收敛到共享判据 `arg_binding::bind_args_to_params`
    pub(super) fn bind_named_args(
        key: &str,
        param_names: &[String],
        args: &[crate::ast::Expr],
        named_args: &[(String, crate::ast::Expr)],
        call_span: crate::util::span::Span,
    ) -> Result<Vec<usize>> {
        arg_binding::bind_args_to_params(
            key,
            param_names,
            args,
            named_args,
            1, // 方法形态：param_names[0] 是接收者位 self
            &vec![false; param_names.len()],
            call_span,
        )
    }
}
