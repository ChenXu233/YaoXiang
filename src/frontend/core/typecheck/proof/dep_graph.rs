//! 类型依赖图：变量间的类型标注依赖关系（RFC-027 §6.1）
//!
//! `mut v: Pred(... x ...)` 的类型标注引用了 `x` 时，`x` 变更需要重验证 `v`。
//! 图同时持有 dependant 解析后的精化类型本体——重验证（VC 生成）直接读图，
//! 不回查 `TypeChecker.env`：函数体局部变量不在模块级 env 里（#379 根因），
//! 而建边时解析出的 `Refined` 类型才是约束的唯一事实源。

use std::collections::{HashMap, HashSet};

use crate::frontend::core::types::MonoType;
use crate::frontend::core::types::const_data::{ConstExpr, ConstValue};

/// 内置终止测度谓词名（RFC-027a）。
///
/// 测度是终止性见证，归 TerminationChecker 管，不参与数据依赖与重验证。
pub const TERMINATES_PREDICATE: &str = "Terminates";

/// 精化约束是否为终止测度（`Terminates(m)`）
pub fn constraint_is_terminates(refined: &MonoType) -> bool {
    matches!(
        refined,
        MonoType::Refined {
            constraint: ConstExpr::Call { func, .. },
            ..
        } if func == TERMINATES_PREDICATE
    )
}

#[derive(Debug, Default)]
pub struct TypeDepGraph {
    /// 被依赖变量 → 依赖它的变量集合
    /// 例：{ "i": {"s", "t"}, "j": {"s"} }
    edges: HashMap<String, HashSet<String>>,
    /// dependant → 解析后的精化类型（`MonoType::Refined`）
    constraints: HashMap<String, MonoType>,
}

impl TypeDepGraph {
    pub fn new() -> Self {
        Self {
            edges: HashMap::new(),
            constraints: HashMap::new(),
        }
    }

    /// 注册精化绑定：记录 dependant 的约束，并对其类型标注中的自由变量建边。
    ///
    /// 非精化类型是 no-op；`Terminates` 测度约束不注册（见 [`TERMINATES_PREDICATE`]）。
    /// 依赖边跳过 dependant 自身（`{ s: Int; s > 0 }` 不自依赖）。
    pub fn register_refined(
        &mut self,
        dependant: &str,
        refined: &MonoType,
    ) {
        if constraint_is_terminates(refined) {
            return;
        }
        let MonoType::Refined { constraint, .. } = refined else {
            return;
        };
        self.constraints
            .insert(dependant.to_string(), refined.clone());
        let mut free_vars = Vec::new();
        collect_free_vars(constraint, &mut free_vars);
        for dep in free_vars {
            if dep != dependant {
                self.add_dep(dependant, &dep);
            }
        }
    }

    /// 记录依赖：`dependant` 的类型标注引用了 `dependency`
    /// dependency 变更时 dependant 需要重验证
    fn add_dep(
        &mut self,
        dependant: &str,
        dependency: &str,
    ) {
        self.edges
            .entry(dependency.to_string())
            .or_default()
            .insert(dependant.to_string());
    }

    /// 查询被某个变量变更影响的变量集合
    pub fn affected_by(
        &self,
        var: &str,
    ) -> Vec<&str> {
        self.edges
            .get(var)
            .map(|set| set.iter().map(|s| s.as_str()).collect())
            .unwrap_or_default()
    }

    /// 取 dependant 注册的精化类型（重验证的 VC 来源）
    pub fn constraint_of(
        &self,
        dependant: &str,
    ) -> Option<&MonoType> {
        self.constraints.get(dependant)
    }

    /// 清除某个变量的依赖者（变量离开作用域时），连同其约束
    pub fn remove_dependant(
        &mut self,
        dependant: &str,
    ) {
        for deps in self.edges.values_mut() {
            deps.remove(dependant);
        }
        self.constraints.remove(dependant);
    }
}

/// 遍历约束表达式树，收集所有命名变量引用（NamedVar / Var）。
///
/// 重验证的闸门用：约束的自由变量只有**全部**有已知值，VC 才可判定；
/// 缺值时送 SMT 会退化成「对所有值成立」的全称检查，产生伪反例。
fn collect_free_vars(
    expr: &ConstExpr,
    out: &mut Vec<String>,
) {
    match expr {
        ConstExpr::NamedVar(name) => out.push(name.clone()),
        ConstExpr::Var(var) => out.push(var.to_string()),
        ConstExpr::BinOp { left, right, .. } => {
            collect_free_vars(left, out);
            collect_free_vars(right, out);
        }
        ConstExpr::UnOp { expr, .. } => collect_free_vars(expr, out),
        ConstExpr::Call { args, .. } => {
            for a in args {
                collect_free_vars(a, out);
            }
        }
        ConstExpr::If {
            condition,
            then_branch,
            else_branch,
        } => {
            collect_free_vars(condition, out);
            collect_free_vars(then_branch, out);
            collect_free_vars(else_branch, out);
        }
        ConstExpr::Range { start, end } => {
            collect_free_vars(start, out);
            collect_free_vars(end, out);
        }
        ConstExpr::Lit(_) => {}
    }
}

/// 精化类型的约束是否可判定：自由变量集合
///
/// （暴露给走查层做缺值闸门；空集 = 无变量约束，恒可判定）
pub fn refined_free_vars(refined: &MonoType) -> Vec<String> {
    let mut vars = Vec::new();
    if let MonoType::Refined { constraint, .. } = refined {
        collect_free_vars(constraint, &mut vars);
    }
    vars
}

/// 精化依赖走查的作用域单元（模块层或一个函数体）
///
/// RFC-027 §6.1 的依赖以函数体为边界：跨函数同名变量不互相触发重验证
/// （旧实现是模块级共享一张名字键图，跨函数误报源，#379 微重构修正）。
/// `values` 是走查中的字面量值环境——顺序常量传播，只认字面量/负字面量；
/// 非字面量 RHS 的赋值使变量值回到未知。
#[derive(Default)]
pub struct RefinedScopeUnit {
    pub deps: TypeDepGraph,
    pub values: HashMap<String, ConstValue>,
}

impl RefinedScopeUnit {
    pub fn new() -> Self {
        Self::default()
    }

    /// 记录变量的已知字面量值；None（非字面量初始化/RHS）作废旧值——
    /// 用陈旧值参与判定会产生伪 Proved
    pub fn track(
        &mut self,
        name: &str,
        value: Option<ConstValue>,
    ) {
        match value {
            Some(v) => {
                self.values.insert(name.to_string(), v);
            }
            None => {
                self.values.remove(name);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::frontend::core::types::const_data::ConstValue;

    fn refined_of(expr: ConstExpr) -> MonoType {
        MonoType::Refined {
            base: Box::new(MonoType::Int(64)),
            constraint: expr,
        }
    }

    #[test]
    fn register_stores_constraint_and_edges() {
        // s: { s == n * 2 } → 约束入图，边 n → s（跳过自身）
        let refined = refined_of(ConstExpr::BinOp {
            op: crate::frontend::core::types::const_data::BinOp::Eq,
            left: Box::new(ConstExpr::NamedVar("s".into())),
            right: Box::new(ConstExpr::BinOp {
                op: crate::frontend::core::types::const_data::BinOp::Mul,
                left: Box::new(ConstExpr::NamedVar("n".into())),
                right: Box::new(ConstExpr::Lit(ConstValue::Int(2))),
            }),
        });
        let mut graph = TypeDepGraph::new();
        graph.register_refined("s", &refined);

        assert_eq!(graph.affected_by("n"), vec!["s"]);
        assert!(graph.affected_by("s").is_empty(), "不自依赖");
        assert!(graph.constraint_of("s").is_some());
    }

    #[test]
    fn register_ignores_non_refined() {
        let mut graph = TypeDepGraph::new();
        graph.register_refined("x", &MonoType::Int(64));
        assert!(graph.constraint_of("x").is_none());
        assert!(graph.affected_by("x").is_empty());
    }

    #[test]
    fn register_ignores_terminates_measure() {
        // Terminates(m) 是终止测度，不是数据约束
        let measure = refined_of(ConstExpr::Call {
            func: TERMINATES_PREDICATE.into(),
            args: vec![ConstExpr::NamedVar("i".into())],
        });
        let mut graph = TypeDepGraph::new();
        graph.register_refined("m", &measure);
        assert!(graph.constraint_of("m").is_none());
        assert!(graph.affected_by("i").is_empty());
    }

    #[test]
    fn remove_dependant_clears_edges_and_constraint() {
        let refined = refined_of(ConstExpr::NamedVar("n".into()));
        let mut graph = TypeDepGraph::new();
        graph.register_refined("s", &refined);
        assert_eq!(graph.affected_by("n"), vec!["s"]);

        graph.remove_dependant("s");
        assert!(graph.affected_by("n").is_empty());
        assert!(graph.constraint_of("s").is_none());
    }

    #[test]
    fn free_vars_cover_if_and_range() {
        // If/Range 子表达式同样含变量引用（旧实现漏收集）
        let refined = refined_of(ConstExpr::If {
            condition: Box::new(ConstExpr::NamedVar("c".into())),
            then_branch: Box::new(ConstExpr::Range {
                start: Box::new(ConstExpr::NamedVar("a".into())),
                end: Box::new(ConstExpr::Lit(ConstValue::Int(9))),
            }),
            else_branch: Box::new(ConstExpr::Lit(ConstValue::Bool(true))),
        });
        let mut vars = refined_free_vars(&refined);
        vars.sort();
        assert_eq!(vars, vec!["a", "c"]);
    }
}
