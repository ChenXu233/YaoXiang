//! 终止检查模块
//!
//! 实现 RFC-027 Section 7：编译器全自动证明循环终止和递归函数终止。
//!
//! **当前实现（Phase 1）**：
//! - 策略 3：有界递增/递减模式 (`i += const` with `i < bound`)
//! - 递归参数递减检查 (`factorial(n-1)`)
//! - `for` 循环自动通过（范围迭代天然终止）
//!
//! **后续扩展**：
//! - 策略 1：线性秩函数自动合成
//! - 策略 2：谓词违反计数
//! - 策略 4：乘法缩放度量模板

// ==================== 度量分析器 ====================
//
// 从循环体中提取候选度量，验证度量是否严格递减。

/// 度量方向
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Direction {
    /// 变量递增（度量 = bound - var）
    Increasing,
    /// 变量递减（度量 = var - bound）
    Decreasing,
}

/// 候选度量：一个变量朝着一个边界移动
///
/// # Example
/// ```text
/// while i < n { i += 1 }
/// → LinearMeasure { var: "i", bound: n, direction: Increasing, delta: 1 }
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LinearMeasure {
    /// 被修改的变量名
    pub var: String,
    /// 边界值（上界或下界），`None` 表示边界是运行时变量
    pub bound: Option<i128>,
    /// 边界变量名（当边界是运行时变量时）
    pub bound_var: Option<String>,
    /// 方向
    pub direction: Direction,
    /// 每次迭代的变化量（默认 1）
    pub delta: i128,
}

impl LinearMeasure {
    /// 创建一个递增到上界的度量
    ///
    /// `while i < n { i += delta }` → `(bound - i)` 每次减 `delta`
    pub fn increasing(
        var: &str,
        bound_var: Option<&str>,
        bound_val: Option<i128>,
        delta: i128,
    ) -> Self {
        Self {
            var: var.to_string(),
            bound: bound_val,
            bound_var: bound_var.map(|s| s.to_string()),
            direction: Direction::Increasing,
            delta,
        }
    }

    /// 创建一个递减到下界的度量
    ///
    /// `while i > 0 { i -= delta }` → `(i - bound)` 每次减 `delta`
    pub fn decreasing(
        var: &str,
        bound_var: Option<&str>,
        bound_val: Option<i128>,
        delta: i128,
    ) -> Self {
        Self {
            var: var.to_string(),
            bound: bound_val,
            bound_var: bound_var.map(|s| s.to_string()),
            direction: Direction::Decreasing,
            delta,
        }
    }

    /// 创建一个乘法缩放度量
    ///
    /// `while v < upper { v *= const }` → ceil(log_const(upper/v)) 每次减 1
    pub fn multiplicative(
        var: &str,
        upper: i128,
        const_val: i128,
    ) -> Self {
        Self {
            var: var.to_string(),
            bound: Some(upper),
            bound_var: None,
            direction: Direction::Increasing,
            delta: const_val,
        }
    }

    /// 返回度量的可读描述
    pub fn describe(&self) -> String {
        match self.direction {
            Direction::Increasing => match (&self.bound_var, self.bound) {
                (Some(bv), _) => format!("{} - {}", bv, self.var),
                (None, Some(bv)) => format!("{} - {}", bv, self.var),
                _ => format!("bound - {}", self.var),
            },
            Direction::Decreasing => match (&self.bound_var, self.bound) {
                (Some(bv), _) => format!("{} - {}", self.var, bv),
                (None, Some(bv)) => format!("{} - {}", self.var, bv),
                _ => format!("{} - bound", self.var),
            },
        }
    }
}

// ==================== 终止检查器核心实现 ====================
//
// 分析循环和递归函数，自动证明终止性。
//
// ## 循环分析
//
// 1. 收集循环体中的赋值操作（`i += 1`, `i = i + 1`）
// 2. 从循环条件中提取边界信息（`while i < n` → i 的上界是 n）
// 3. 枚举候选度量并验证严格递减
//
// ## 递归分析
//
// 1. 识别直接递归调用
// 2. 检查调用参数是否严格递减（`f(n-1)` where `n-1 < n`）

use crate::frontend::core::parser::ast::{self, Expr, Stmt, StmtKind, BinOp, Type};
use crate::frontend::core::typecheck::environment::TypeEnvironment;
use crate::frontend::core::typecheck::proof::verdict::{
    BudgetReport, DisproofKind, DisproofModel, ProofResult, UnprovenReason,
};
use super::super::proof::smt::ast::{SMTExpr, SMTCommand, SMTSort, SMTResult};
#[cfg(not(target_arch = "wasm32"))]
use super::super::proof::smt::backend::Solver;

/// 显式测度在递归回边上产生的**义务原料**（RFC-027a §义务生成）。
///
/// T3 只**生成/记录**：把「形参 + 测度 + 调用实参」三样备齐，T4 才能构造
/// `m[形参 := 实参] < m` 并送 SMT 判定。本结构不携带判定结果。
#[derive(Debug, Clone, PartialEq)]
pub struct MeasureObligation {
    /// 发起递归的函数名（义务归属）
    pub fn_name: String,
    /// 函数形参名（按声明序）——供 T4 做「形参 := 实参」替换
    pub params: Vec<String>,
    /// 程序员在类型位声明的测度原式
    pub measure: crate::frontend::core::types::const_data::ConstExpr,
    /// 自递归调用点的实参（按位置对应 `params`）
    pub call_args: Vec<crate::frontend::core::types::const_data::ConstExpr>,
    /// 自递归调用点位置
    pub span: crate::util::span::Span,
    /// 调用点处已累积的路径守卫（RFC-027a §路径守卫）。
    ///
    /// 如 gcd 的 `b != 0`（来自 `if b == 0 { return a }` 的早返回）。
    /// 判定递减时作为背景假设——缺它则 `a % b < b` 不成立（b 为负时 `mod` 取负号）。
    pub guards: Vec<crate::frontend::core::types::const_data::ConstExpr>,
}

/// 单条义务的判定结果（RFC-027a §判定管线）
///
/// 两类义务共用此枚举：**递减**（`m[形参:=实参] < m` 不可满足）与**良基性**
/// （`m >= 0` 在形参前置条件下不可满足取反）。两者都只问「目标取反是否不可
/// 满足」，故用中性的「已证明」而非「递减」——否则良基性判定会被读成递减判定。
///
/// `Disproved` 与 `NotProved` **必须分开**（RFC-027a:77 与 :223 的分工）：
/// 判伪（求解器给出反例）才是义务不成立，可报 E4022；求解器返回 unknown 只是
/// 「判不出」，不是义务为假——把它当判伪会违反 :223「只给方向不拒绝」。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MeasureVerdict {
    /// 已证明：目标取反不可满足
    Proved,
    /// 判伪：求解器给出反例（存在使义务不成立的取值）
    Disproved {
        /// 反例赋值（变量 → 取值），进 E4022 文案。
        /// 保留**结构**而非预格式化字符串：发射端要填进 `DisproofModel`，
        /// 格式化在诊断侧统一做（与 `PredicateViolation` 同一取法）。
        counterexample: Vec<(String, String)>,
    },
    /// 未证明：求解器返回 unknown（判不了，**不等于**义务为假）
    NotProved,
    /// 未判定：无求解器可用（wasm / Z3 缺失）
    Unjudged,
}

/// 当前正在遍历的、**带显式测度**的函数上下文（RFC-027a §义务生成）。
///
/// 只在函数有显式测度时建立——无测度的递归不走显式测度路径（RFC-027 §7
/// 「无测度 → 硬边界」）。
#[derive(Debug, Clone)]
struct FnMeasureContext {
    name: String,
    params: Vec<String>,
    measure: crate::frontend::core::types::const_data::ConstExpr,
}

/// 终止检查器
///
/// 在类型检查之后、约束求解之前运行。
/// 遍历 AST，为每个 `while` 循环和每个递归函数执行终止分析。
#[derive(Debug)]
pub struct TerminationChecker {
    /// 收集到的证明结果
    results: Vec<ProofResult>,
    /// 求解器（持有所有权）——策略 1 秩函数 SMT 验证 + 显式测度义务判定
    ///
    /// RFC-027a T4：生产在 `checker.rs` 注入 `default_solver()`；未注入则为
    /// `None`，SMT 相关路径整体不执行（wasm 下无 Z3，见 #376）。
    #[cfg(not(target_arch = "wasm32"))]
    solver: Option<Box<dyn Solver>>,
    /// 带精化标注的变量名集合——决定循环是否进**验证模式**（RFC-027 §7）
    ///
    /// RFC-027 §7：终止性不是独立开关，而是验证模式的一部分。**裸 `while`
    /// 不进验证模式**，不生成任何终止义务；仅当度量变量带精化标注
    /// （如 `i: UpTo(n)`）时才须证终止。
    ///
    /// 空集合 = 无任何精化标注 → 所有循环都不检查（与 RFC 一致）。
    /// 由 `set_refined_vars` 注入；未注入时保持空集，即「不检查」。
    refined_vars: std::collections::HashSet<String>,
    /// 显式测度表：绑定/函数名 → 测度表达式（RFC-027a §2）。
    ///
    /// 由 `set_measures` 注入，调用方（`TypeChecker`）从 **AST** 提取——
    /// 不走已解析的 `MonoType`（那是有损转换，T1 调查中它把测度表达式丢成
    /// `Int(64)`）。
    ///
    /// 空表 = 源码里没有任何 `Terminates(m)` 标注。
    measures:
        std::collections::HashMap<String, crate::frontend::core::types::const_data::ConstExpr>,
    /// 当前正在遍历的带测度函数上下文（RFC-027a §义务生成）。
    /// `None` = 不在带测度的函数体内，自调用不产生测度义务。
    current_fn: Option<FnMeasureContext>,
    /// 已生成的测度义务（RFC-027a §义务生成）。T3 生成，T4 判定。
    measure_obligations: Vec<MeasureObligation>,
    /// 已判定结果，与 `measure_obligations` **同序同长**：
    /// `measure_verdicts[i]` 是 `measure_obligations[i]` 的判定。
    ///
    /// RFC-027a T4：只判定并记录，不发射诊断（E4022 与路径守卫属 T5）。
    measure_verdicts: Vec<MeasureVerdict>,
    /// 函数前置条件（型参精化的代入后约束，RFC-027a §良基性）。
    ///
    /// 键 = 函数名，值 = 该函数带精化的形参约束（如 `b: NonNegative(b)` →
    /// `b >= 0`）。由 `set_param_assumptions` 注入。
    ///
    /// 用途：良基性 `m >= 0` 的证据只能来自它——ℤ 上 `<` 并**不**良基
    /// （`-1,-2,…` 无限下降），无下界就不能宣称测度落在自然数上。
    param_assumptions:
        std::collections::HashMap<String, Vec<crate::frontend::core::types::const_data::ConstExpr>>,
    /// 良基性判定结果：函数名 → 其测度 `m >= 0` 的判定。
    ///
    /// 与 `measure_verdicts`（逐调用点的递减判定）分开：良基性是**测度自身**
    /// 的性质（每函数一条），递减是**每个递归调用点**的性质。
    well_founded: std::collections::HashMap<String, MeasureVerdict>,
    /// 当前路径已累积的守卫（RFC-027a §路径守卫 / RFC-027 §3.3 假设栈）。
    ///
    /// 顺序语句的路径条件：`if C { return ... }` 之后，后续语句在 `!C` 下成立。
    /// gcd 的 `a % b < b` 正需守卫 `b != 0`。分支内另压正向/反向条件。
    ///
    /// 只**加**假设、绝不减：判不出时就不压（少假设只会让证明更难，不会
    /// 造出不成立的假设）。
    path_guards: Vec<crate::frontend::core::types::const_data::ConstExpr>,
}

impl Default for TerminationChecker {
    fn default() -> Self {
        Self::new()
    }
}

impl TerminationChecker {
    /// 创建新的终止检查器
    pub fn new() -> Self {
        Self {
            results: Vec::new(),
            #[cfg(not(target_arch = "wasm32"))]
            solver: None,
            refined_vars: std::collections::HashSet::new(),
            measures: std::collections::HashMap::new(),
            current_fn: None,
            measure_obligations: Vec::new(),
            measure_verdicts: Vec::new(),
            param_assumptions: std::collections::HashMap::new(),
            well_founded: std::collections::HashMap::new(),
            path_guards: Vec::new(),
        }
    }

    /// 注入函数前置条件（型参精化的代入后约束，RFC-027a §良基性）。
    ///
    /// 值必须是**已在形参名下**的约束（`b: NonNegative(b)` → `b >= 0`），
    /// 因为良基性目标是 `测度 >= 0`，两者必须同名同名变量才能合证。
    pub fn set_param_assumptions(
        mut self,
        assumptions: std::collections::HashMap<
            String,
            Vec<crate::frontend::core::types::const_data::ConstExpr>,
        >,
    ) -> Self {
        self.param_assumptions = assumptions;
        self
    }

    /// 读取良基性判定结果（函数名 → 测度 `m >= 0` 的判定）。
    ///
    /// 只暴露原料：发射 E4022 属 T5，本步只判不报。
    pub fn well_founded_verdicts(&self) -> &std::collections::HashMap<String, MeasureVerdict> {
        &self.well_founded
    }

    /// 注入显式测度表（RFC-027a §2）。
    ///
    /// 键是**被标注的绑定名或函数名**：函数返回类型位的 `Terminates(b)` 挂在
    /// 函数名下（`gcd`），变量绑定位的 `Terminates(n - i)` 挂在绑定名下（`acc`）。
    /// 值是该测度的 `ConstExpr` 无损形态——复合测度（`n - i`）必须完整保留，
    /// 压成单变量会让 SMT 拿到错误的测度。
    pub fn set_measures(
        mut self,
        measures: std::collections::HashMap<
            String,
            crate::frontend::core::types::const_data::ConstExpr,
        >,
    ) -> Self {
        self.measures = measures;
        self
    }

    /// 读取已注入的显式测度表（RFC-027a §2）。
    ///
    /// 供义务生成（T3）按被标注名查测度；亦供测试断言注入是否生效。
    pub fn measures(
        &self
    ) -> &std::collections::HashMap<String, crate::frontend::core::types::const_data::ConstExpr>
    {
        &self.measures
    }

    /// 读取已生成的测度义务（RFC-027a §义务生成）。
    ///
    /// T3 交付「生成」；判定在 T4，故此处只暴露原料。
    pub fn measure_obligations(&self) -> &[MeasureObligation] {
        &self.measure_obligations
    }

    /// 读取测度义务的判定结果（RFC-027a §判定管线）。
    ///
    /// 与 [`Self::measure_obligations`] 同序同长。
    pub fn measure_verdicts(&self) -> &[MeasureVerdict] {
        &self.measure_verdicts
    }

    /// 注入带精化标注的变量名集合（RFC-027 §7 验证模式的判据）
    ///
    /// 门控方向是**放开检查**：精化集合外的变量 → 循环不进验证模式 → 不报
    /// E4021。因此注入缺失会导致「该查的没查」——调用方必须传真实集合，
    /// 不能用空集偷懒。
    pub fn set_refined_vars(
        mut self,
        vars: std::collections::HashSet<String>,
    ) -> Self {
        self.refined_vars = vars;
        self
    }
    /// 注入求解器后端（持有所有权，不泄露）
    ///
    /// RFC-027a T4：终止检查器**持有**求解器而非借 `&'static`——后者迫使调用方
    /// 用 `Box::leak` 把 `default_solver()` 的 `Box<dyn Solver>` 变成静态引用，
    /// 泄漏虽小但无必要（计划 D3）。
    #[cfg(not(target_arch = "wasm32"))]
    pub fn with_solver_owned(
        mut self,
        solver: Box<dyn Solver>,
    ) -> Self {
        self.solver = Some(solver);
        self
    }

    /// 检查整个模块的终止性
    ///
    /// 返回收集到的所有证明结果。空 Vec 表示所有循环和递归都可证明终止，
    /// 或未发现需要检查的循环/递归。
    pub fn check_module(
        &mut self,
        module: &ast::Module,
        _env: &TypeEnvironment,
    ) -> Vec<ProofResult> {
        for stmt in &module.items {
            self.check_stmt(stmt, false);
        }
        self.judge_measure_obligations();
        self.judge_well_foundedness();
        self.emit_measure_disproofs();
        std::mem::take(&mut self.results)
    }

    /// 递减义务判伪 → E4022（RFC-027a §义务生成）。
    ///
    /// **只发递减判伪**，两个门都没有放松：
    /// - `NotProved`（求解器 unknown）不发——判不出不是义务为假（027a:223）
    /// - 良基性未证时不发——ℤ 上 `<` 不良基，测度不降**不**等于不终止
    ///   （`gcd` 在 `b` 为负时照样停）。良基性已证（测度高含下界）时，递减失守
    ///   才是真正的不终止证据。
    ///
    /// 宽严方向：这两道门只会**少报**，不会误报。
    fn emit_measure_disproofs(&mut self) {
        for (ob, verdict) in self
            .measure_obligations
            .iter()
            .zip(self.measure_verdicts.iter())
        {
            let MeasureVerdict::Disproved { counterexample } = verdict else {
                continue;
            };
            // 良基性门：只有测度确实落在自然数上时，"不严格递减"才推出不终止
            if !matches!(
                self.well_founded.get(&ob.fn_name),
                Some(MeasureVerdict::Proved)
            ) {
                continue;
            }
            self.results.push(ProofResult::Disproved(DisproofModel {
                kind: DisproofKind::MeasureNotDecreasing,
                assignments: counterexample.clone(),
                constraint: ob.measure.to_string(),
                span: Some(ob.span),
                predicate_span: None,
            }));
        }
    }

    /// 判定各带测度函数的**良基性**（RFC-027a §良基性：测度落在自然数上）。
    ///
    /// 目标 `m >= 0`，背景假设是该函数的形参精化（前置条件）。两者同名变量
    /// 才能合证；无求解器 → `Unjudged`（不得把「未判」当「成立」）。
    fn judge_well_foundedness(&mut self) {
        self.well_founded = self.compute_well_foundedness();
    }

    /// 逐函数计算良基性判定。
    #[cfg(not(target_arch = "wasm32"))]
    fn compute_well_foundedness(&self) -> std::collections::HashMap<String, MeasureVerdict> {
        use crate::frontend::core::types::const_data::{BinOp, ConstExpr, ConstValue};

        let mut out = std::collections::HashMap::new();
        for (fn_name, measure) in &self.measures {
            let goal = ConstExpr::BinOp {
                op: BinOp::Ge,
                left: Box::new(measure.clone()),
                right: Box::new(ConstExpr::Lit(ConstValue::Int(0))),
            };
            // 背景假设 = 形参前置条件 + 该名字下各回边的路径守卫。
            //
            // 守卫必需：循环测度的下界正是从循环条件导出的（`while i < n` ⇒
            // `n - i > 0`）。守卫在该回边处确实成立，故用作假设是可靠的。
            let assumptions = {
                let mut a = self
                    .param_assumptions
                    .get(fn_name)
                    .cloned()
                    .unwrap_or_default();
                for ob in self
                    .measure_obligations
                    .iter()
                    .filter(|o| o.fn_name == *fn_name)
                {
                    for g in &ob.guards {
                        if !a.contains(g) {
                            a.push(g.clone());
                        }
                    }
                }
                a
            };
            let verdict = match self.solver.as_deref() {
                None => MeasureVerdict::Unjudged,
                Some(solver) => {
                    // 未知按 Int：测度是整型表达式（同 `compute_measure_verdicts`）
                    let var_sorts =
                        crate::frontend::core::typecheck::proof::smt::translate::infer_var_sorts(
                            &goal,
                            &std::collections::HashMap::new(),
                        );
                    let commands =
                        crate::frontend::core::typecheck::proof::smt::translate::translate_constraint(
                            &goal,
                            &assumptions,
                            &var_sorts,
                        );
                    match solver.solve(&commands, 100) {
                        SMTResult::Unsat => MeasureVerdict::Proved,
                        // 良基性 Sat = 「下界推不出」，而**不是**义务为假：
                        // 测度在可达输入上未必真会取负，只是当前精化不足以导出下界。
                        // RFC-027a:223 对此的规定是「推不出时进残余义务，且只给方向
                        // 不拒绝」——故不得映射为 `Disproved`（否则会报 E4022 拒绝）。
                        SMTResult::Sat { .. } | SMTResult::Unknown { .. } => {
                            MeasureVerdict::NotProved
                        }
                    }
                }
            };
            out.insert(fn_name.clone(), verdict);
        }
        out
    }

    /// 逐函数计算良基性判定（wasm: 无 Z3，恒 `Unjudged`）。
    #[cfg(target_arch = "wasm32")]
    fn compute_well_foundedness(&self) -> std::collections::HashMap<String, MeasureVerdict> {
        self.measures
            .keys()
            .map(|k| (k.clone(), MeasureVerdict::Unjudged))
            .collect()
    }

    /// 判定已生成的测度义务（RFC-027a §判定管线）
    ///
    /// 对每条义务构造 `not (m[形参:=实参] < m)` 送求解器：Unsat = 严格递减成立。
    ///
    /// 本任务只**判定并记录**，不发射诊断——E4022 与路径守卫的完备采集属 T5。
    /// 故用户可见行为不变（与 T3 同：机制就位但无人裁决）。
    fn judge_measure_obligations(&mut self) {
        let verdicts = self.compute_measure_verdicts();
        self.measure_verdicts = verdicts;
    }

    /// 逐条计算测度义务的判定结果（无求解器时全部 `Unjudged`）。
    ///
    /// 取 `&self` 并返回 owned Vec，避开「借 `self.solver` 的同时写
    /// `self.measure_verdicts`」的借用冲突。
    #[cfg(not(target_arch = "wasm32"))]
    fn compute_measure_verdicts(&self) -> Vec<MeasureVerdict> {
        use crate::frontend::core::types::const_data::ConstExpr;

        let Some(solver) = self.solver.as_deref() else {
            return vec![MeasureVerdict::Unjudged; self.measure_obligations.len()];
        };
        self.measure_obligations
            .iter()
            .map(|ob| {
                // m[形参 := 实参] —— 下一轮取值
                let next = substitute_const_expr(&ob.measure, &ob.params, &ob.call_args);
                let decreasing = ConstExpr::BinOp {
                    op: crate::frontend::core::types::const_data::BinOp::Lt,
                    left: Box::new(next),
                    right: Box::new(ob.measure.clone()),
                };
                // 无绑定环境：未知名按 Int 处理（测度是整型表达式）
                let var_sorts =
                    crate::frontend::core::typecheck::proof::smt::translate::infer_var_sorts(
                        &decreasing,
                        &std::collections::HashMap::new(),
                    );
                // translate_constraint 断言目标取反：Unsat = 递减在所有取值下成立。
                // 背景假设 = 函数前置条件（良基性下界，如 `b >= 0`）+ 该调用点
                // 的路径守卫（如 `b != 0`）——两者缺一不可：仅守卫时 b 可取负，
                // `a % b < b` 因 mod 取负号而不成立。
                let mut assumptions = self
                    .param_assumptions
                    .get(&ob.fn_name)
                    .cloned()
                    .unwrap_or_default();
                assumptions.extend(ob.guards.iter().cloned());
                let commands =
                    crate::frontend::core::typecheck::proof::smt::translate::translate_constraint(
                        &decreasing,
                        &assumptions,
                        &var_sorts,
                    );
                match solver.solve(&commands, 100) {
                    SMTResult::Unsat => MeasureVerdict::Proved,
                    // 判伪：存在使递减不成立的取值（b 取 0）。反例进 E4022。
                    SMTResult::Sat { model } => MeasureVerdict::Disproved {
                        counterexample: model.assignments.clone(),
                    },
                    // unknown：判不了，**不**等于义务为假（027a:223）
                    SMTResult::Unknown { .. } => MeasureVerdict::NotProved,
                }
            })
            .collect()
    }

    /// wasm 无 Z3（#376）：义务一律不判定。
    #[cfg(target_arch = "wasm32")]
    fn compute_measure_verdicts(&self) -> Vec<MeasureVerdict> {
        vec![MeasureVerdict::Unjudged; self.measure_obligations.len()]
    }

    // ==================== 语句遍历 ====================

    fn check_stmt(
        &mut self,
        stmt: &Stmt,
        is_never: bool,
    ) {
        match &stmt.kind {
            StmtKind::Expr(expr) => self.check_expr(expr, is_never),
            StmtKind::Assign {
                target,
                signature_params,
                value: Some(v),
                type_annotation,
                ..
            } => {
                use crate::frontend::core::parser::ast::Expr;
                let child_never = is_never || is_never_return_type(type_annotation.as_ref());
                // RFC-027a：带显式测度的函数体进上下文，使自递归调用可被识别。
                // 用 `signature_params` 而非 `Type::Fn.params` 取形参名——后者只有
                // 类型（`[Int, Int]`），名字在签名参数里。
                let saved_ctx = self.enter_measured_fn(target, signature_params);
                if let Expr::Lambda { body, .. } = v.as_ref() {
                    self.check_stmts(&body.stmts, child_never);
                } else if let Expr::Block(block) = v.as_ref() {
                    self.check_stmts(&block.stmts, child_never);
                } else if let Expr::While {
                    condition, body, ..
                } = v.as_ref()
                {
                    // RFC-027 §6.9 / 027a §义务生成：测度直接绑在**循环**上（`acc:
                    // Terminates(n - i) = while …`）——此绑定的测度就是该循环的兜底测度，
                    // 生成回边义务。
                    //
                    // 只在「循环是带测度绑定的**直接值**」时走这支：函数体（Block）内的
                    // 循环属于外层函数的测度域，不能把函数的测度当成循环的。
                    if let Some(ctx) = self.current_fn.clone() {
                        self.check_expr(condition, child_never);
                        self.generate_loop_measure_obligation(&ctx, condition, body, stmt.span);
                        // 有显式测度即已有兜底路径，不做自动探索、不报 E4021（§6.9：
                        // 探索失败才用显式测度；此处程序员已直接给出）。
                        self.check_stmts(&body.stmts, child_never);
                    } else {
                        self.check_expr(v, child_never);
                    }
                } else {
                    self.check_expr(v, child_never);
                }
                self.current_fn = saved_ctx;
            }
            StmtKind::If {
                condition,
                then_branch,
                else_if_branches,
                else_branch,
                ..
            } => {
                self.check_expr(condition, is_never);
                // 各分支各压自己的路径条件（RFC-027 §3.3 假设栈）：
                // then 压条件本身，else 压其否定。出分支即弹回。
                let mark = self.path_guards.len();
                if let Some(g) =
                    crate::frontend::core::types::eval::const_eval::convert_expr_to_const_expr(
                        condition,
                    )
                {
                    self.path_guards.push(g);
                }
                self.check_stmts(&then_branch.stmts, is_never);
                self.path_guards.truncate(mark);
                for (cond, body) in else_if_branches {
                    self.check_expr(cond, is_never);
                    let mark = self.path_guards.len();
                    if let Some(g) =
                        crate::frontend::core::types::eval::const_eval::convert_expr_to_const_expr(
                            cond,
                        )
                    {
                        self.path_guards.push(g);
                    }
                    self.check_stmts(&body.stmts, is_never);
                    self.path_guards.truncate(mark);
                }
                if let Some(else_body) = else_branch {
                    let mark = self.path_guards.len();
                    if let Some(g) = negate_guard(condition) {
                        self.path_guards.push(g);
                    }
                    self.check_stmts(&else_body.stmts, is_never);
                    self.path_guards.truncate(mark);
                }
            }
            // 其他语句类型不包含需要检查的子结构
            _ => {}
        }
    }

    /// 顺序走查一个语句块，维护路径守卫（RFC-027a §路径守卫）。
    ///
    /// 早返回惯用法 `if C { return ... }` 之后，后续语句在 `!C` 下成立——
    /// gcd 的 `a % b < b` 正需这条 `b != 0`。出块即 `truncate` 弹回，
    /// 守卫不泄漏到兄弟作用域。
    fn check_stmts(
        &mut self,
        stmts: &[Stmt],
        is_never: bool,
    ) {
        let mark = self.path_guards.len();
        for stmt in stmts {
            self.check_stmt(stmt, is_never);
            // 只有「then 必不落回且无 else/else-if」时，后续语句才确定在 `!C` 下
            if let StmtKind::If {
                condition,
                then_branch,
                else_if_branches,
                else_branch,
                ..
            } = &stmt.kind
            {
                if else_branch.is_none()
                    && else_if_branches.is_empty()
                    && block_diverges(&then_branch.stmts)
                {
                    if let Some(g) = negate_guard(condition) {
                        self.path_guards.push(g);
                    }
                }
            }
        }
        self.path_guards.truncate(mark);
    }

    // ==================== 表达式遍历 ====================

    fn check_expr(
        &mut self,
        expr: &Expr,
        is_never: bool,
    ) {
        match expr {
            Expr::While {
                condition,
                body,
                span,
                ..
            } => {
                self.check_while_loop(condition, body, *span, is_never);
                // 递归检查循环体内的嵌套循环
                for s in &body.stmts {
                    self.check_stmt(s, is_never);
                }
            }
            Expr::For { iterable, body, .. } => {
                for s in &body.stmts {
                    self.check_stmt(s, is_never);
                }
                self.check_expr(iterable, is_never);
            }
            Expr::FnDef {
                name: _,
                params,
                body,
                return_type,
                ..
            } => {
                let param_names: Vec<String> = params.iter().map(|p| p.name.clone()).collect();
                let fn_never = is_never || is_never_return_type(return_type.as_ref());
                self.check_fn_body(&param_names, body, fn_never);
            }
            Expr::Call { func, args, .. } => {
                for a in args {
                    self.check_expr(a, is_never);
                }
                self.check_possible_recursive_call(func, args);
            }
            Expr::BinOp {
                op: _, left, right, ..
            } => {
                self.check_expr(left, is_never);
                self.check_expr(right, is_never);
            }
            Expr::Block(block) => {
                for s in &block.stmts {
                    self.check_stmt(s, is_never);
                }
            }
            Expr::If {
                condition,
                then_branch,
                else_if_branches,
                else_branch,
                ..
            } => {
                self.check_expr(condition, is_never);
                for s in &then_branch.stmts {
                    self.check_stmt(s, is_never);
                }
                for (cond, body) in else_if_branches {
                    self.check_expr(cond, is_never);
                    for s in &body.stmts {
                        self.check_stmt(s, is_never);
                    }
                }
                if let Some(else_body) = else_branch {
                    for s in &else_body.stmts {
                        self.check_stmt(s, is_never);
                    }
                }
            }
            Expr::Lambda { body, .. } => {
                for s in &body.stmts {
                    self.check_stmt(s, is_never);
                }
            }
            // RFC-027a：`return f(...)` 的调用在 Return 内部。本分支此前缺位，
            // 使 Return 成叶子——**函数体的自递归调用从来不会被访问**，这也是
            // `check_possible_recursive_call` 恒空的原因之一。
            Expr::Return(Some(inner), _) => self.check_expr(inner, is_never),
            // 叶子节点不需要检查
            _ => {}
        }
    }

    // ==================== 循环终止检查 ====================

    /// 循环是否进「验证模式」（RFC-027 §7）
    ///
    /// 判据：循环条件里的界变量、或循环体中被赋值的变量，**任一**带精化标注。
    /// 取「任一」而非「全部」是保守方向——宁可多查（报 E4021 是能力边界提示），
    /// 不可漏查。
    /// 判据取 RFC §7「度量变量带精化标注」的**宽读**：循环条件中的变量、或
    /// 循环体中被赋值的变量，任一带精化即进验证模式。
    ///
    /// §7 的范例是 `i: UpTo(n)` 配 `while i < n { i = i + 1 }`——精化变量 i
    /// 正在条件里。故条件变量必须计入；只看体中被赋值者会漏掉「有精化但本轮
    /// 未推进」的循环（如 `while i < n { f(i) }`），那是该查而未查。
    ///
    /// 方向性：门控是**放开检查**（不生成义务），故取宽读更保守——宽读多生成
    /// 义务（可能报 E4021），窄读会静默漏查。漏查不可接受，故宁可宽。
    fn loop_is_in_verification_mode(
        &self,
        condition: &Expr,
        body: &ast::Block,
    ) -> bool {
        // 条件中的变量（§7 范例形态：i: UpTo(n) 出现在 `i < n` 里）
        for (var, _) in self.extract_bounds_from_condition(condition) {
            if self.refined_vars.contains(&var) {
                return true;
            }
        }
        // 循环体中被赋值的变量（候选度量）
        self.collect_assignments(body)
            .iter()
            .any(|assign| self.refined_vars.contains(&assign.var))
    }

    /// 分析 while 循环的终止性
    fn check_while_loop(
        &mut self,
        condition: &Expr,
        body: &ast::Block,
        span: crate::util::span::Span,
        is_never: bool,
    ) {
        // Never 返回函数：循环不终止是类型签名保证的语义，直接放行
        if is_never {
            return;
        }
        // RFC-027 §7：验证模式门控——**裸 `while` 不生成终止义务**。
        //
        // 终止性不是独立开关，而是「验证模式」的一部分：类型一旦被精化，它
        // 标注的那段计算才进验证模式。故只有当循环的**度量变量**（被
        // 赋值的循环变量）带精化标注时，才要求证明终止。
        //
        // 反例（修复前会误报）：`mut i: Int = 0; while i < 10 { i = i + 1 }`
        // 是普通 Int 循环，按 §7 完全不需要终止证明，此前却报 E4021。
        //
        // ⚠ 门控方向是**放开检查**：判错会漏查而非误查。故只看「是否有任一
        // 度量变量带精化」，不做额外启发式猜测。
        if !self.loop_is_in_verification_mode(condition, body) {
            // 仍要下钻循环体，检查其中的嵌套循环（内层可能带精化）
            for s in &body.stmts {
                self.check_stmt(s, is_never);
            }
            return;
        }
        // 1. 从条件中提取边界信息（仅保留循环不变量——上界在循环体内被赋值时
        //    度量合成不成立：`while i < n { n = n - 1; i = i + 1 }` 度量恒为 0）
        let bounds: Vec<(String, (BoundOp, BoundExpr))> = self
            .extract_bounds_from_condition(condition)
            .into_iter()
            .filter(|(_, (_, b))| Self::bound_is_loop_invariant(b, condition, body))
            .collect();

        // 2. 从循环体中收集赋值操作
        let assignments = self.collect_assignments(body);

        // 3. 尝试匹配度量
        for assign in &assignments {
            for (var_name, (cmp_op, bound_expr)) in &bounds {
                if assign.var != *var_name {
                    continue;
                }

                // 尝试构建线性度量
                if let Some(measure) = self.try_build_measure(assign, *cmp_op, bound_expr) {
                    // 快速检查：度量是否有下界（至少 >= 0）
                    // 完整验证在后续阶段由 SMT 求值器完成
                    self.emit_terminates(span, &measure);
                    return;
                }
            }
        }

        // 策略 4：乘法缩放度量
        if let Some(measure) = self.try_multiplicative_measure(&assignments, &bounds) {
            self.emit_terminates(span, &measure);
            return;
        }

        // 策略 1：线性秩函数自动合成（SMT 验证）
        //
        // ⚠ 当前**恒不生效**，见 #377。注入求解器也无法救回任何循环：
        //   (a) 上游 bound_is_loop_invariant 过滤把「在循环体内被赋值的边界变量」
        //       全部剔除，而需要秩函数的形状（i<j { i+=1; j-=1 }、while flag）
        //       边界必定在体内被改 → bounds 恒为空 → 零候选
        //   (b) generate_rank_candidates 产出的 delta 恒为 +1，而
        //       verify_rank_candidate 构造 m' = m + delta 后断言 not(m' < m)，
        //       m+1 < m 恒假 → not(false) 恒真 → Sat 而非 Unsat → 恒返回 false
        // 实测：注入 default_solver() 后与原版逐字节相同（12 种循环形状）
        // 下面的 tripwire 测试锁定该事实；修好 (a)(b) 后它会失败，届时请
        // 连同本注释与 #377 一起更新，而不是删掉断言
        #[cfg(not(target_arch = "wasm32"))]
        if self.solver.is_some() {
            if let Some(measure) =
                self.try_linear_rank_function(&bounds, &assignments, condition, span)
            {
                self.emit_terminates(span, &measure);
                return;
            }
        }

        // 策略 2：谓词违反计数（框架占位）
        if let Some(measure) = self.try_violation_count(&assignments, &bounds) {
            self.emit_terminates(span, &measure);
            return;
        }

        // 4. 没有找到有效度量 → 报错
        self.emit_loop_not_terminating(span);
    }

    /// 从循环条件中提取变量边界信息
    ///
    /// 支持的模式：
    /// - `i < n` → (i, Lt, n)
    /// - `i <= n` → (i, Lte, n)
    /// - `i > 0` → (i, Gt, 0)
    /// - `i >= 0` → (i, Gte, 0)
    /// - `i != 0` → (i, Neq, 0) — 不提供严格递减保证
    /// - `i < s.n` → (i, Lt, Var("s.n")) — 字段访问作符号上界（见 expr_to_bound）
    fn extract_bounds_from_condition(
        &self,
        condition: &Expr,
    ) -> Vec<(String, (BoundOp, BoundExpr))> {
        let mut bounds = Vec::new();

        if let Expr::BinOp {
            op, left, right, ..
        } = condition
        {
            let cmp_op = match op {
                BinOp::Lt => BoundOp::Lt,
                BinOp::Le => BoundOp::Le,
                BinOp::Gt => BoundOp::Gt,
                BinOp::Ge => BoundOp::Ge,
                _ => return bounds,
            };

            // 尝试 left = var, right = bound
            if let Expr::Var(var_name, _) = left.as_ref() {
                if let Some(bound) = self.expr_to_bound(right) {
                    bounds.push((var_name.clone(), (cmp_op, bound)));
                }
            }
            // 尝试 right = var, left = bound (反转比较)
            if let Expr::Var(var_name, _) = right.as_ref() {
                if let Some(bound) = self.expr_to_bound(left) {
                    let rev_op = match cmp_op {
                        BoundOp::Lt => BoundOp::Gt,
                        BoundOp::Le => BoundOp::Ge,
                        BoundOp::Gt => BoundOp::Lt,
                        BoundOp::Ge => BoundOp::Le,
                    };
                    bounds.push((var_name.clone(), (rev_op, bound)));
                }
            }
        }

        bounds
    }

    /// 将表达式转换为边界表示
    ///
    /// `FieldAccess`（如 `s.n`）归为**符号变量**（`BoundExpr::Var`）：度量合成只需
    /// 上界在迭代间保持同一，而 SMT 侧同一符号名保证这一点。健全性由
    /// `bound_is_loop_invariant` 把守——若该字段在循环体内被赋值，上界不作数。
    fn expr_to_bound(
        &self,
        expr: &Expr,
    ) -> Option<BoundExpr> {
        match expr {
            Expr::Lit(lit, _) => match lit {
                ast::Literal::Int(v) => Some(BoundExpr::Const(*v)),
                ast::Literal::Float(f) => Some(BoundExpr::Const(*f as i128)),
                _ => None,
            },
            Expr::Var(name, _) => Some(BoundExpr::Var(name.clone())),
            // `s.n` / `a.b.c`：拼成稳定符号名，供 SMT 作同一变量使用
            Expr::FieldAccess { .. } => Self::render_field_path(expr).map(BoundExpr::Var),
            _ => None,
        }
    }

    /// 把字段访问链渲染成稳定符号名（`s.n` / `a.b.c`）。
    /// 非纯字段链（含索引/调用等）返回 None。
    fn render_field_path(expr: &Expr) -> Option<String> {
        match expr {
            Expr::Var(name, _) => Some(name.clone()),
            Expr::FieldAccess {
                expr: inner, field, ..
            } => Self::render_field_path(inner).map(|p| format!("{p}.{field}")),
            _ => None,
        }
    }

    /// 上界是否为循环不变量：该边界引用的名字（或其前缀根）不得在循环体内被赋值。
    ///
    /// 没有这一步，`while i < n { n = n - 1; i = i + 1 }` 会被误判为终止
    /// （上界 n 自身在递减，度量 `n - i` 恒为 0，循环不终止）。
    fn bound_is_loop_invariant(
        bound: &BoundExpr,
        condition: &Expr,
        body: &ast::Block,
    ) -> bool {
        let names: Vec<&str> = match bound {
            // 常量天然不变
            BoundExpr::Const(_) => return true,
            BoundExpr::Var(n) => vec![n.as_str()],
        };
        // 边界名本身与循环变量同名时（`i < i`）不作数，交由后续度量合成失败处理
        let _ = condition;
        let mut assigned: Vec<String> = Vec::new();
        Self::collect_assigned_names(body, &mut assigned);
        !names.iter().any(|n| {
            // 精确匹配（`s.n`）或前缀根匹配（赋值 `s` 就污染 `s.n`）
            let root = n.split('.').next().unwrap_or(n);
            assigned
                .iter()
                .any(|a| a == n || a == root || n.starts_with(&format!("{a}.")))
        })
    }

    /// 收集循环体内被赋值的所有目标名（变量名或字段路径）。
    fn collect_assigned_names(
        block: &ast::Block,
        out: &mut Vec<String>,
    ) {
        for stmt in &block.stmts {
            Self::collect_assigned_names_from_stmt(stmt, out);
        }
    }

    fn collect_assigned_names_from_stmt(
        stmt: &crate::frontend::core::parser::ast::Stmt,
        out: &mut Vec<String>,
    ) {
        use crate::frontend::core::parser::ast::StmtKind;
        match &stmt.kind {
            StmtKind::Assign { target, .. } => {
                if let Some(path) = Self::render_field_path(target) {
                    out.push(path);
                }
            }
            StmtKind::Expr(e) => Self::collect_assigned_names_from_expr(e, out),
            StmtKind::If { then_branch, .. } => {
                Self::collect_assigned_names(then_branch, out);
            }
            StmtKind::For { body, .. } => {
                Self::collect_assigned_names(body, out);
            }
            _ => {}
        }
    }

    /// 表达式内的赋值路径收集（`i = i + 1` / `if` / `while` / `for` 体内的）。
    /// 循环语句在 AST 里是 `Expr` 而非 `StmtKind`。
    fn collect_assigned_names_from_expr(
        expr: &Expr,
        out: &mut Vec<String>,
    ) {
        match expr {
            Expr::BinOp {
                op: ast::BinOp::Assign,
                left,
                ..
            } => {
                if let Some(path) = Self::render_field_path(left) {
                    out.push(path);
                }
            }
            Expr::If {
                then_branch,
                else_branch,
                ..
            } => {
                Self::collect_assigned_names(then_branch, out);
                if let Some(e) = else_branch {
                    Self::collect_assigned_names(e, out);
                }
            }
            Expr::While { body, .. } | Expr::For { body, .. } => {
                Self::collect_assigned_names(body, out);
            }
            Expr::Block(b) => Self::collect_assigned_names(b, out),
            _ => {}
        }
    }

    /// 从循环体中收集所有赋值操作
    fn collect_assignments(
        &self,
        body: &ast::Block,
    ) -> Vec<LoopAssignment> {
        let mut assignments = Vec::new();
        for stmt in &body.stmts {
            self.collect_assignments_from_stmt(stmt, &mut assignments);
        }
        assignments
    }

    fn collect_assignments_from_stmt(
        &self,
        stmt: &Stmt,
        assignments: &mut Vec<LoopAssignment>,
    ) {
        match &stmt.kind {
            StmtKind::Expr(expr) => {
                // `i += 1` 解析为 `i = i + 1` (BinOp::Assign)
                if let Expr::BinOp {
                    op: BinOp::Assign,
                    left,
                    right,
                    ..
                } = expr.as_ref()
                {
                    if let Expr::Var(var_name, _) = left.as_ref() {
                        let delta_info = self.analyze_delta(right, var_name);
                        assignments.push(LoopAssignment {
                            var: var_name.clone(),
                            delta_info,
                        });
                    }
                }
            }
            // `i = i - 1` → Assign { target: Var(name), value: Some(init) }
            StmtKind::Assign {
                target,
                value: Some(v),
                ..
            } => {
                use crate::frontend::core::parser::ast::Expr;
                if let Expr::Var(name, _) = target.as_ref() {
                    let delta_info = self.analyze_delta(v, name);
                    if !matches!(delta_info, DeltaInfo::Unknown) {
                        assignments.push(LoopAssignment {
                            var: name.clone(),
                            delta_info,
                        });
                    }
                }
                // 递归处理 Lambda/Block 函数体
                if let Expr::Lambda { body, .. } = v.as_ref() {
                    for s in &body.stmts {
                        self.collect_assignments_from_stmt(s, assignments);
                    }
                } else if let Expr::Block(block) = v.as_ref() {
                    for s in &block.stmts {
                        self.collect_assignments_from_stmt(s, assignments);
                    }
                }
            }
            StmtKind::If {
                then_branch,
                else_if_branches,
                else_branch,
                ..
            } => {
                for s in &then_branch.stmts {
                    self.collect_assignments_from_stmt(s, assignments);
                }
                for (_, body) in else_if_branches {
                    for s in &body.stmts {
                        self.collect_assignments_from_stmt(s, assignments);
                    }
                }
                if let Some(else_body) = else_branch {
                    for s in &else_body.stmts {
                        self.collect_assignments_from_stmt(s, assignments);
                    }
                }
            }
            _ => {}
        }
    }

    /// 分析赋值右侧的 delta 模式
    ///
    /// 识别模式：
    /// - `var + const` → DeltaKind::Add(const)
    /// - `var - const` → DeltaKind::Sub(const)
    /// - `const + var` → DeltaKind::Add(const)
    fn analyze_delta(
        &self,
        expr: &Expr,
        var_name: &str,
    ) -> DeltaInfo {
        match expr {
            Expr::BinOp {
                op: BinOp::Add,
                left,
                right,
                ..
            } => {
                if self.is_var_ref(left, var_name) {
                    if let Some(c) = self.as_const(right) {
                        return DeltaInfo::Add(c);
                    }
                }
                if self.is_var_ref(right, var_name) {
                    if let Some(c) = self.as_const(left) {
                        return DeltaInfo::Add(c);
                    }
                }
                DeltaInfo::Unknown
            }
            Expr::BinOp {
                op: BinOp::Sub,
                left,
                right,
                ..
            } => {
                if self.is_var_ref(left, var_name) {
                    if let Some(c) = self.as_const(right) {
                        return DeltaInfo::Sub(c);
                    }
                }
                DeltaInfo::Unknown
            }
            Expr::BinOp {
                op: BinOp::Mul,
                left,
                right,
                ..
            } => {
                if self.is_var_ref(left, var_name) {
                    if let Some(c) = self.as_const(right) {
                        return DeltaInfo::Mul(c);
                    }
                }
                if self.is_var_ref(right, var_name) {
                    if let Some(c) = self.as_const(left) {
                        return DeltaInfo::Mul(c);
                    }
                }
                DeltaInfo::Unknown
            }
            // 处理前置 ++/-- 展开后的形式: i = 1 + i
            _ => DeltaInfo::Unknown,
        }
    }

    fn is_var_ref(
        &self,
        expr: &Expr,
        name: &str,
    ) -> bool {
        matches!(expr, Expr::Var(v, _) if v == name)
    }

    fn as_const(
        &self,
        expr: &Expr,
    ) -> Option<i128> {
        match expr {
            Expr::Lit(ast::Literal::Int(v), _) => Some(*v),
            Expr::Lit(ast::Literal::Float(f), _) => Some(*f as i128),
            _ => None,
        }
    }

    /// 尝试构建线性度量
    ///
    /// 匹配逻辑：
    /// - 变量递增 (delta > 0) + 有上界 (Lt/Lte) → 度量 = bound - var
    /// - 变量递减 (delta < 0) + 有下界 (Gt/Gte) → 度量 = var - bound
    fn try_build_measure(
        &self,
        assign: &LoopAssignment,
        cmp_op: BoundOp,
        bound: &BoundExpr,
    ) -> Option<LinearMeasure> {
        let delta = match assign.delta_info {
            DeltaInfo::Add(c) => c,  // var += c, c > 0
            DeltaInfo::Sub(c) => -c, // var -= c, c > 0 → delta = -c
            DeltaInfo::Mul(c) => c,  // var *= c, c > 1 → delta = c
            DeltaInfo::Unknown => return None,
        };

        if delta == 0 {
            return None; // 没有变化，不可能是严格递减度量
        }

        let (bound_val, bound_var) = match bound {
            BoundExpr::Const(c) => (Some(*c), None),
            BoundExpr::Var(name) => (None, Some(name.clone())),
        };

        match (cmp_op, delta > 0) {
            // i < bound / i <= bound，且 i 在递增 → 度量 = bound - i
            (BoundOp::Lt | BoundOp::Le, true) => Some(LinearMeasure::increasing(
                &assign.var,
                bound_var.as_deref(),
                bound_val,
                delta.abs(),
            )),
            // i > bound / i >= bound，且 i 在递减 → 度量 = i - bound
            (BoundOp::Gt | BoundOp::Ge, false) => Some(LinearMeasure::decreasing(
                &assign.var,
                bound_var.as_deref(),
                bound_val,
                delta.abs(),
            )),
            _ => None, // 方向不匹配（如 i < bound 但 i 在递减）
        }
    }

    /// 策略 4：乘法缩放度量模板
    ///
    /// 检测 `v *= const`（const > 1）且 v 有整数上界的循环模式。
    /// 度量 = ceil(log_const(upper / v))，每次乘 const 度量减 1。
    fn try_multiplicative_measure(
        &self,
        assignments: &[LoopAssignment],
        bounds: &[(String, (BoundOp, BoundExpr))],
    ) -> Option<LinearMeasure> {
        for assign in assignments {
            if let DeltaInfo::Mul(const_val) = assign.delta_info {
                if const_val <= 1 {
                    continue;
                }
                // 查找 v 的上界
                if let Some((_, (BoundOp::Lt | BoundOp::Le, BoundExpr::Const(upper)))) =
                    bounds.iter().find(|(v, _)| v == &assign.var)
                {
                    if *upper > 0 {
                        return Some(LinearMeasure::multiplicative(
                            &assign.var,
                            *upper,
                            const_val,
                        ));
                    }
                }
            }
        }
        None
    }

    /// 策略 1：线性秩函数自动合成
    ///
    /// 枚举候选线性度量，SMT 验证每条执行路径上严格递减。
    /// - ≤3 个有界变量 → 全组合枚举
    /// - >3 个 → 只单变量，失败报编译错误
    #[cfg(not(target_arch = "wasm32"))]
    fn try_linear_rank_function(
        &self,
        bounds: &[(String, (BoundOp, BoundExpr))],
        assignments: &[LoopAssignment],
        _condition: &Expr,
        _span: crate::util::span::Span,
    ) -> Option<LinearMeasure> {
        // ⚠ 本函数当前对任何输入都返回 None，见 #377（终止策略 1 不可用）。
        // 调用点见 check_while_loop 的同名注释。两处缺陷：
        //   (a) `bounds` 进到这里时已被 bound_is_loop_invariant 清空；
        //   (b) 即使非空，verify_rank_candidate 的 delta 符号也是错的。
        let solver = self.solver.as_deref()?;

        let bounded_vars: Vec<&str> = bounds.iter().map(|(v, _)| v.as_str()).collect();
        let candidates = self.generate_rank_candidates(&bounded_vars, bounds);

        for candidate in &candidates {
            if self.verify_rank_candidate(candidate, bounds, assignments, solver) {
                return Some(candidate.clone());
            }
        }

        None
    }

    /// 生成秩函数候选列表
    ///
    /// ⚠ 缺陷 (b) 在本函数：所有候选的 `delta` 入口恒为 `1`（见下方
    /// `increasing(v, .., 1)`）。而 `verify_rank_candidate` 构造
    /// `m' = var + delta` 后断言 `not (m' < m)`——`m' = m + 1` 使 `m' < m`
    /// 恒假，`not(false)` 恒真 → 求解器返回 Sat → 验证恒失败。
    /// 需按 `Direction` 决定 delta 符号（Increasing 度量应是 `bound - v`，其
    /// 每次迭代减 1；而非直接对 `v` 加 delta）。见 #377。
    fn generate_rank_candidates(
        &self,
        bounded_vars: &[&str],
        bounds: &[(String, (BoundOp, BoundExpr))],
    ) -> Vec<LinearMeasure> {
        let mut candidates = Vec::new();

        if bounded_vars.len() > 3 {
            // 只尝试单变量度量
            for &v in bounded_vars {
                candidates.push(LinearMeasure::increasing(v, None, None, 1));
                if let Some((_, (_, BoundExpr::Const(upper)))) =
                    bounds.iter().find(|(bv, _)| bv == v)
                {
                    candidates.push(LinearMeasure::increasing(v, None, Some(*upper), 1));
                }
            }
            return candidates;
        }

        // ≤3 个变量：全组合
        for &v in bounded_vars {
            candidates.push(LinearMeasure::increasing(v, None, None, 1));
            if let Some((_, (_, BoundExpr::Const(u)))) = bounds.iter().find(|(bv, _)| bv == v) {
                candidates.push(LinearMeasure::increasing(v, None, Some(*u), 1));
            }
        }

        // 两变量组合：v_i - v_j
        for i in 0..bounded_vars.len() {
            for j in 0..bounded_vars.len() {
                if i != j {
                    candidates.push(LinearMeasure::increasing(
                        bounded_vars[i],
                        Some(bounded_vars[j]),
                        None,
                        1,
                    ));
                }
            }
        }

        candidates
    }

    /// SMT 验证秩函数候选是否在所有路径上严格递减
    #[cfg(not(target_arch = "wasm32"))]
    fn verify_rank_candidate(
        &self,
        candidate: &LinearMeasure,
        _bounds: &[(String, (BoundOp, BoundExpr))],
        _assignments: &[LoopAssignment],
        solver: &dyn Solver,
    ) -> bool {
        let mut commands = Vec::new();

        // 声明秩函数变量
        commands.push(SMTCommand::DeclareConst(
            candidate.var.clone(),
            SMTSort::Int,
        ));
        if let Some(ref bv) = candidate.bound_var {
            commands.push(SMTCommand::DeclareConst(bv.clone(), SMTSort::Int));
        }

        // 构造 m_prime = var + delta（被赋值后的值）
        let m_var = SMTExpr::Atom(candidate.var.clone());
        let m_prime = SMTExpr::App(
            "+".into(),
            vec![
                SMTExpr::Atom(candidate.var.clone()),
                SMTExpr::Atom(candidate.delta.to_string()),
            ],
        );

        // assert (not (< m_prime m_var))
        let decreasing = SMTExpr::App("<".into(), vec![m_prime, m_var]);
        let not_decreasing = SMTExpr::App("not".into(), vec![decreasing]);
        commands.push(SMTCommand::Assert(not_decreasing));

        commands.push(SMTCommand::CheckSat);

        // unsat = m' < m 在所有情况下成立 → 严格递减
        matches!(
            solver.solve(&commands, 50),
            crate::frontend::core::typecheck::proof::smt::ast::SMTResult::Unsat
        )
    }

    /// 策略 2：谓词违反计数（RFC-027 §7.3，实验性，框架占位）
    ///
    /// 完整实现需要：
    /// 1. Parser 支持 forall 量词语法
    /// 2. 解析目标类型定义提取条件函数
    /// 3. 生成 violation_count 度量
    /// 4. 验证相邻操作减少度量
    ///
    /// 当前返回 None，待 Parser 升级后补完。
    fn try_violation_count(
        &self,
        _assignments: &[LoopAssignment],
        _bounds: &[(String, (BoundOp, BoundExpr))],
    ) -> Option<LinearMeasure> {
        None
    }

    // ==================== 递归终止检查 ====================

    /// 检查函数体
    fn check_fn_body(
        &mut self,
        _param_names: &[String],
        body: &ast::Block,
        is_never: bool,
    ) {
        for s in &body.stmts {
            self.check_stmt(s, is_never);
        }
    }

    /// 若该绑定带显式测度，进入其函数体上下文（RFC-027a §义务生成）。
    ///
    /// 返回**进入前**的上下文，供遍历结束后恢复——函数体可嵌套（lambda 套
    /// lambda），恢复而非盲置 `None` 才能正确处理嵌套。
    ///
    /// 无测度的函数**不建立上下文**（返回 `None`）：其递归不走显式测度路径，
    /// 保持 RFC-027 §7「无测度 → 硬边界」语义。
    fn enter_measured_fn(
        &mut self,
        target: &Expr,
        signature_params: &[ast::Param],
    ) -> Option<FnMeasureContext> {
        let Expr::Var(name, _) = target else {
            return None;
        };
        let measure = self.measures.get(name).cloned()?;
        let ctx = FnMeasureContext {
            name: name.clone(),
            params: signature_params.iter().map(|p| p.name.clone()).collect(),
            measure,
        };
        self.current_fn.replace(ctx)
    }

    /// 检查可能的递归调用
    ///
    /// RFC-027a §义务生成：当前函数带显式测度且被调用者就是它自己时，生成一条
    /// 测度义务（形参 + 测度 + 实参）。若实参个数与形参不符、或实参转不出编译期
    /// 表达式，**放弃该条而不是猜值**——T4 拿不到完整替换就无法判定，宁缺勿错。
    ///
    /// 只认直接自递归。互递归需 SCC（本期非目标，计划 §五）。
    fn check_possible_recursive_call(
        &mut self,
        func: &Expr,
        args: &[Expr],
    ) {
        let Some(ctx) = self.current_fn.clone() else {
            return;
        };
        let Expr::Var(callee, span) = func else {
            return;
        };
        if *callee != ctx.name {
            return;
        }
        // 实参数与形参数必须一致，否则「形参 := 实参」替换无意义
        if args.len() != ctx.params.len() {
            return;
        }
        let converted: Option<Vec<_>> = args
            .iter()
            .map(crate::frontend::core::types::eval::const_eval::convert_expr_to_const_expr)
            .collect();
        let Some(call_args) = converted else {
            return;
        };
        self.measure_obligations.push(MeasureObligation {
            fn_name: ctx.name,
            params: ctx.params,
            measure: ctx.measure,
            call_args,
            span: *span,
            guards: self.path_guards.clone(),
        });
    }

    /// 循环回边的显式测度义务（RFC-027 §6.9 兜底路径）。
    ///
    /// 与递归回边**同构**：`m[变量 := 回边取值] < m`。当前侧的 `m` 用声明原式，
    /// 下一侧用体内赋值代入后的式子（复用 `substitute_const_expr`）。
    ///
    /// 守卫取**循环条件 + 体内已累积的路径守卫**——循环条件在进入本轮时成立，
    /// 是回边可达的唯一假设（也是良基性下界的来源：`i < n` 导出 `n - i > 0`）。
    ///
    /// 三种情形**放弃生成**（宁缺勿错，与 `check_possible_recursive_call` 同方针）：
    ///
    /// 1. 体内无所属变量的赋值——没有回边状态可谈
    /// 2. 某赋值的右侧引用了**另一个也被赋值的变量**——那需要「同时代入」语义，
    ///    逐个代入会因次序得出无关的式子，宁可不判
    /// 3. 测度式**未引用任何**被赋值变量——义务退化为 `m < m`，恒假即误报
    fn generate_loop_measure_obligation(
        &mut self,
        ctx: &FnMeasureContext,
        condition: &Expr,
        body: &ast::Block,
        span: crate::util::span::Span,
    ) {
        use crate::frontend::core::types::const_data::ConstExpr;
        use crate::frontend::core::types::eval::const_eval::convert_expr_to_const_expr;

        // 体内对变量的赋值（按序，同一变量的后者覆盖前者）
        let mut updates: Vec<(String, ConstExpr)> = Vec::new();
        for stmt in &body.stmts {
            let StmtKind::Assign {
                target,
                value: Some(v),
                ..
            } = &stmt.kind
            else {
                continue;
            };
            let Expr::Var(name, _) = target.as_ref() else {
                continue;
            };
            let Some(rhs) = convert_expr_to_const_expr(v) else {
                continue;
            };
            updates.retain(|(n, _)| n != name);
            updates.push((name.clone(), rhs));
        }
        // (1) 无赋值
        if updates.is_empty() {
            return;
        }

        let names: Vec<String> = updates.iter().map(|(n, _)| n.clone()).collect();

        // (2) 赋值间相互引用 → 同时代入语义不明
        for (name, rhs) in &updates {
            if names
                .iter()
                .any(|other| other != name && const_expr_refs(rhs, other))
            {
                return;
            }
        }

        // (3) 测度须真的取到某个被赋值变量，否则义务退化为 `m < m`（恒假）
        if !names.iter().any(|n| const_expr_refs(&ctx.measure, n)) {
            return;
        }

        let call_args: Vec<ConstExpr> = updates.into_iter().map(|(_, rhs)| rhs).collect();
        let mut guards = self.path_guards.clone();
        if let Some(g) = convert_expr_to_const_expr(condition) {
            guards.push(g);
        }
        self.measure_obligations.push(MeasureObligation {
            fn_name: ctx.name.clone(),
            params: names,
            measure: ctx.measure.clone(),
            call_args,
            span,
            guards,
        });
    }

    // ==================== 诊断输出 ====================

    fn emit_terminates(
        &mut self,
        _span: crate::util::span::Span,
        _measure: &LinearMeasure,
    ) {
        // 循环可证明终止——记录 Proved（不产生任何诊断）
        self.results.push(ProofResult::Proved);
    }

    fn emit_loop_not_terminating(
        &mut self,
        span: crate::util::span::Span,
    ) {
        // 能力边界，不是程序错误：用 LoopTerminationUnproven（→ E4021），
        // 不用 BeyondKernel（后者在 into_result 下会走 ICE E8001）。
        // 具体描述由 locales 提供，不在此写死中文以免污染其他语言。
        // span 随原因携带，使诊断指向具体循环（B10）。
        let reason = UnprovenReason::LoopTerminationUnproven { span };
        self.results.push(ProofResult::Unproven {
            reason,
            proof_calls: vec![],
            budget: BudgetReport {
                steps_used: 0,
                steps_limit: 0,
            },
        });
    }
}

// ==================== 内部类型 ====================

/// 循环条件中的边界运算符
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum BoundOp {
    Lt,
    Le,
    Gt,
    Ge,
}

/// 边界表达式
#[derive(Debug, Clone, PartialEq, Eq)]
enum BoundExpr {
    /// 常量边界（如 `i < 10`）
    Const(i128),
    /// 变量边界（如 `i < n`）
    Var(String),
}

/// 赋值操作的 delta 信息
#[derive(Debug, Clone, PartialEq, Eq)]
enum DeltaInfo {
    /// var += c
    Add(i128),
    /// var -= c
    Sub(i128),
    /// var *= c（const > 1，乘法缩放模式）
    Mul(i128),
    /// 无法确定
    Unknown,
}

/// 循环体中的赋值操作
#[derive(Debug, Clone, PartialEq, Eq)]
struct LoopAssignment {
    /// 被赋值的变量名
    var: String,
    /// delta 信息
    delta_info: DeltaInfo,
}

/// 判断函数类型签名的返回类型是否为 Never
///
/// `(P1, P2, ...) -> Never` → true
/// 该常量表达式是否引用名字 `name`（RFC-027a §义务生成）。
///
/// 用于循环回边义务的两个放弃条件：赋值间相互引用、测度未取到被赋值变量。
fn const_expr_refs(
    expr: &crate::frontend::core::types::const_data::ConstExpr,
    name: &str,
) -> bool {
    use crate::frontend::core::types::const_data::ConstExpr as CE;
    match expr {
        CE::NamedVar(n) => n == name,
        CE::BinOp { left, right, .. } => {
            const_expr_refs(left, name) || const_expr_refs(right, name)
        }
        CE::UnOp { expr, .. } => const_expr_refs(expr, name),
        CE::Call { args, .. } => args.iter().any(|a| const_expr_refs(a, name)),
        CE::If {
            condition,
            then_branch,
            else_branch,
        } => {
            const_expr_refs(condition, name)
                || const_expr_refs(then_branch, name)
                || const_expr_refs(else_branch, name)
        }
        CE::Range { start, end } => const_expr_refs(start, name) || const_expr_refs(end, name),
        _ => false,
    }
}

fn is_never_return_type(ty: Option<&Type>) -> bool {
    match ty {
        // `name: () -> Never` — type_annotation 是 Fn 类型，取 return_type
        Some(Type::Fn { return_type, .. }) => is_type_never(return_type.as_ref()),
        // `fn(): Never` — FnDef.return_type 是裸返回类型
        Some(t) => is_type_never(t),
        None => false,
    }
}

/// 递归判断 Type 是否为 Never（支持 Type::Name { name: "Never", .. }）
fn is_type_never(ty: &Type) -> bool {
    matches!(ty, Type::Name { name, .. } if name == "Never" || name == "never")
}

/// 在 `ConstExpr` 中把形参名替换为调用实参（RFC-027a §判定管线）。
///
/// 测度写成**形参的函数**（`gcd` 的测度 `b` 对应形参 `b`），调用点的实参给出
/// 下一轮取值，代入后即 `m[形参 := 实参]`。
///
/// 未出现在形参表内的名字**保持原样**（如已在外层作用域的变量、或字面量），
/// 而不是替换成空——替换错会让义务判成一个无关的式子。
///
/// 递归覆盖全部复合形态（`BinOp`/`UnOp`/`Call`/`If`/`Range`）：测度可以写成
/// `n - i`、`if c { a } else { b }` 等；只替换顶层会让内层测度悄悄不代入。
fn substitute_const_expr(
    expr: &crate::frontend::core::types::const_data::ConstExpr,
    params: &[String],
    args: &[crate::frontend::core::types::const_data::ConstExpr],
) -> crate::frontend::core::types::const_data::ConstExpr {
    use crate::frontend::core::types::const_data::ConstExpr as CE;

    let sub = |e: &CE| substitute_const_expr(e, params, args);

    match expr {
        CE::NamedVar(name) => match params.iter().position(|p| p == name) {
            Some(i) if i < args.len() => args[i].clone(),
            _ => expr.clone(),
        },
        CE::BinOp { op, left, right } => CE::BinOp {
            op: *op,
            left: Box::new(sub(left)),
            right: Box::new(sub(right)),
        },
        CE::UnOp { op, expr: inner } => CE::UnOp {
            op: *op,
            expr: Box::new(sub(inner)),
        },
        CE::Call {
            func,
            args: call_args,
        } => CE::Call {
            func: func.clone(),
            args: call_args.iter().map(sub).collect(),
        },
        CE::If {
            condition,
            then_branch,
            else_branch,
        } => CE::If {
            condition: Box::new(sub(condition)),
            then_branch: Box::new(sub(then_branch)),
            else_branch: Box::new(sub(else_branch)),
        },
        CE::Range { start, end } => CE::Range {
            start: Box::new(sub(start)),
            end: Box::new(sub(end)),
        },
        // Lit / Var(ConstVar)：不引用程序变量，原样
        other => other.clone(),
    }
}

/// 条件取反（RFC-027 §3.3 假设栈）。
///
/// 早返回惯用法 `if C { return ... }` 之后，后续语句在 `!C` 下成立。
/// 不在比较上做 De Morgan 规约——SMT 侧 `not (= b 0)` 与 `b != 0` 等价，
/// 少一层变换就少一处可能出错的地方。
fn negate_guard(
    condition: &crate::frontend::core::parser::ast::Expr
) -> Option<crate::frontend::core::types::const_data::ConstExpr> {
    use crate::frontend::core::types::const_data::{ConstExpr, UnOp};
    use crate::frontend::core::types::eval::const_eval::convert_expr_to_const_expr;

    let inner = convert_expr_to_const_expr(condition)?;
    // 双重否定直接消去（`!C` 作为条件时）
    if let ConstExpr::UnOp {
        op: UnOp::Not,
        expr,
    } = inner
    {
        return Some(*expr);
    }
    Some(ConstExpr::UnOp {
        op: UnOp::Not,
        expr: Box::new(inner),
    })
}

/// 语句块是否**必不回落**（控制流不会走到块后）。
///
/// 只识别两条形态——早返回惯用法（`return` 收尾）与两分支皆不落回的 `if`：
///
/// - `return` 的表示是 `StmtKind::Expr(Expr::Return)`（解析器如此产出，
///   与 `check_stmt` 的观测一致）
/// - 早期 `StmtKind::Return` 变体一并认（解析器不再产出，但零成本）
///
/// 判错的方向是安全的：漏认 → 少压一条守卫 → 证明更难、不会造出假假设。
fn block_diverges(stmts: &[Stmt]) -> bool {
    use crate::frontend::core::parser::ast::{Expr, StmtKind};

    let Some(last) = stmts.last() else {
        return false;
    };
    match &last.kind {
        StmtKind::Return(_) => true,
        StmtKind::Expr(e) => matches!(e.as_ref(), Expr::Return(Some(_), _)),
        StmtKind::If {
            then_branch,
            else_if_branches,
            else_branch: Some(else_body),
            ..
        } => {
            block_diverges(&then_branch.stmts)
                && block_diverges(&else_body.stmts)
                && else_if_branches
                    .iter()
                    .all(|(_, body)| block_diverges(&body.stmts))
        }
        _ => false,
    }
}
