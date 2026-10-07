//! 终止检查模块
//!
//! 实现 RFC-027 Section 7：编译器全自动证明循环终止和递归函数终止。
//!
//! **当前实现**：
//! - 策略 1：线性秩函数自动合成（`v < b → b - v`；`v > b → v - b`，SMT 验证递减）
//! - 策略 1b：标志循环（`while flag { v = v ∓ k; if v ⋛ c { flag = false } }`）
//! - 策略 3：有界递增/递减模式 (`i += const` with `i < bound`，边界须循环不变量)
//! - 策略 4：乘法缩放度量模板 (`v *= const` with `v < const`)
//! - 递归参数递减检查 (`factorial(n-1)`)
//! - `for` 循环自动通过（范围迭代天然终止）
//!
//! **后续扩展**：
//! - 策略 2：谓词违反计数（框架占位，需 forall 量词语法）

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
    /// `None`，SMT 相关路径整体不执行（wasm 自 #435 起同带 Z3）。
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
        // 1. 从条件中提取边界信息。**原始集合**留给策略 1（秩函数）——它逐赋值
        //    验证候选，不依赖边界在迭代间不变（见 `try_linear_rank_function`）。
        let all_bounds: Vec<(String, (BoundOp, BoundExpr))> =
            self.extract_bounds_from_condition(condition);

        // 策略 3/4 的度量形如 `bound - var`，其**边界必须循环不变量**：上界自身
        // 在体内被赋值时该度量不成立（`while i < n { n = n - 1; i = i + 1 }` 的
        // `n - i` 恒为 0，循环并不终止）。故这两条策略只吃过滤后的子集；
        // 策略 1 吃 `all_bounds`。
        let bounds: Vec<(String, (BoundOp, BoundExpr))> = all_bounds
            .iter()
            .filter(|(_, (_, b))| Self::bound_is_loop_invariant(b, condition, body))
            .cloned()
            .collect();

        // 每次迭代**必执行**的赋值位移表（策略 1 的递减论证前提：只有无条件赋值
        // 才能保证「每次迭代都朝边界走」）
        let unconditional = self.collect_unconditional_deltas(body);

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
        // 输入用**未过滤**的 `all_bounds`：需要秩函数的形状（`i < j { i += 1;
        // j -= 1 }`、`while flag`）其边界按定义就在体内被改，被不变性过滤清空后
        // 零候选。健全性改由逐赋值验证承担：候选的测度变化按**实际位移**算
        //（`δv` 来自无条件赋值、`δbound` 取守卫内赋值的保守极值），边界是不是
        // 不变量不再重要——边界自己动，测度只会减得更快。
        //
        // 旧缺陷 (b)：delta 恒 +1 且对 v 直接加，导致 m' = m + 1 恒不小于 m。
        // 现按 Direction 推导：Increasing（m = bound - v）用 Δm = δbound - δv，
        // Decreasing（m = v - bound）用 Δm = δv - δbound（见 verify_rank_candidate）。
        if self.solver.is_some() {
            if let Some(measure) = self.try_linear_rank_function(
                &all_bounds,
                &assignments,
                &unconditional,
                condition,
                span,
            ) {
                self.emit_terminates(span, &measure);
                return;
            }
        }

        // 策略 1b：**标志循环**（guarded exit）——条件不是比较式，边界提取无从下手，
        // 走「阈值守卫 + 无条件位移」的专门模式。位移是已知常量，故不需要求解器。
        if let Some(measure) =
            self.try_flag_loop_measure(condition, body, &assignments, &unconditional)
        {
            self.emit_terminates(span, &measure);
            return;
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
    /// **为什么需要这一步**：策略 3/4 的度量是 `bound - var`（或 `ceil(log(bound/var))`）
    /// 这类**假设边界固定**的量。没有过滤，`while i < n { n = n - 1; i = i + 1 }`
    /// 会被误判为终止（上界 n 自身在递减，`n - i` 恒为 0，循环其实不终止）。
    ///
    /// **适用范围**：只过滤策略 3/4 的输入。策略 1（线性秩函数）**必须**吃未过滤
    /// 的边界：需要秩函数的形状（`i < j { i += 1; j -= 1 }`）其边界按定义就在
    /// 体内被改，过滤后候选恒空（这正是 #377 缺陷 a：策略 1 在生产中恒不生效）。
    /// 策略 1 的健全性不靠边界不变，而靠**逐赋值验证**——它按实际位移算测度变化
    /// （见 `verify_rank_candidate`：边界自己动时测度只会减得更快）。
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
    ///
    /// ⚠ 已知边界（本轮未改，供后续收口）：本函数把**守卫内**的赋值也一并收进来，
    /// 不记录其条件。策略 3/4 据此把「有条件执行」当作「每轮执行」，例如
    /// `while i < n { if c { i = i + 1 } }` 在 `c` 恒假时并不终止，却会被策略 3
    /// 判为终止。策略 1 不依赖这一步：它另用 `collect_unconditional_deltas`（只收
    /// 体顶层语句）保证「每轮都动」，并把守卫内位移计入最坏情况极值。
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
                    // 位移**未知**的赋值同样要收进来：整条丢弃会让下游以为该变量
                    // 只有已知位移，于是「未知重绑定可能把变量改到反方向」时错误地
                    // 接受（策略 1/1b 的 delta_bounds 会把未知位移当作不可判并拒绝）。
                    assignments.push(LoopAssignment {
                        var: name.clone(),
                        delta_info,
                    });
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

    /// 位移值：`DeltaInfo` → 带符号整数（`Add(c)` = `+c`、`Sub(c)` = `-c`）。
    ///
    /// `Mul` 不是线性位移、`Unknown` 不可判 ⇒ `None`（调用方据此拒绝论证）。
    fn signed_delta(info: &DeltaInfo) -> Option<i128> {
        match info {
            DeltaInfo::Add(c) => Some(*c),
            DeltaInfo::Sub(c) => Some(-*c),
            DeltaInfo::Mul(_) | DeltaInfo::Unknown => None,
        }
    }

    /// 每次迭代**必执行**的赋值位移表（变量 → 带符号位移）。
    ///
    /// 只收循环体**顶层**语句：`if`/`while`/`for` 体内的赋值是**有条件**执行的，
    /// 不能用来论证「每次迭代都朝边界走」——那正是 `verify_rank_candidate` 的
    /// 健全性前提（守卫内的位移另有其用途：只参与最坏情况极值）。
    /// 位移未知或非线性的目标不入表。
    fn collect_unconditional_deltas(
        &self,
        block: &ast::Block,
    ) -> std::collections::HashMap<String, i128> {
        let mut out = std::collections::HashMap::new();
        for stmt in &block.stmts {
            // `i = i + 1` 的两种 AST 形态都要收：
            // `StmtKind::Expr(BinOp::Assign)` 与 `StmtKind::Assign { value }`
            let (target, value) = match &stmt.kind {
                StmtKind::Expr(expr) => match expr.as_ref() {
                    Expr::BinOp {
                        op: BinOp::Assign,
                        left,
                        right,
                        ..
                    } => (left.as_ref(), right.as_ref()),
                    _ => continue,
                },
                StmtKind::Assign {
                    target,
                    value: Some(v),
                    ..
                } => (target.as_ref(), v.as_ref()),
                _ => continue,
            };
            let Expr::Var(name, _) = target else {
                continue;
            };
            if let Some(delta) = Self::signed_delta(&self.analyze_delta(value, name)) {
                out.insert(name.clone(), delta);
            }
        }
        out
    }

    /// 变量在循环体内**全部**赋值（含守卫内）的位移极值 `(min, max)`。
    ///
    /// 未出现在任何赋值里 ⇒ `(0, 0)`（循环不变量）。任一赋值位移未知/非线性
    /// ⇒ `None`：说不准就拒绝论证（宁可报「无法证明」，不可误判终止）。
    fn delta_bounds(
        assignments: &[LoopAssignment],
        var: &str,
    ) -> Option<(i128, i128)> {
        let mut min: Option<i128> = None;
        let mut max: Option<i128> = None;
        for assign in assignments.iter().filter(|a| a.var == var) {
            let delta = Self::signed_delta(&assign.delta_info)?;
            min = Some(min.map_or(delta, |m: i128| m.min(delta)));
            max = Some(max.map_or(delta, |m: i128| m.max(delta)));
        }
        Some((min.unwrap_or(0), max.unwrap_or(0)))
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

    /// 策略 1：线性秩函数自动合成（RFC-027 §7.2）
    ///
    /// 从循环条件的每个边界生成一条候选线性测度（方向由算子定，见
    /// `generate_rank_candidates`），逐条交 SMT 验证「每次迭代严格递减」。
    ///
    /// `bounds` 是**未过滤**的边界集合（调用点传 `all_bounds`）：秩函数形状的
    /// 边界本来就在循环体内被改，不能按「循环不变量」剔除。健全性来自逐赋值
    /// 验证，而不是来自边界不变。
    fn try_linear_rank_function(
        &self,
        bounds: &[(String, (BoundOp, BoundExpr))],
        assignments: &[LoopAssignment],
        unconditional: &std::collections::HashMap<String, i128>,
        _condition: &Expr,
        _span: crate::util::span::Span,
    ) -> Option<LinearMeasure> {
        let solver = self.solver.as_deref()?;

        let candidates = self.generate_rank_candidates(bounds);

        for candidate in &candidates {
            if self.verify_rank_candidate(candidate, unconditional, assignments, solver) {
                return Some(candidate.clone());
            }
        }

        None
    }

    /// 生成秩函数候选：**每个边界一条，方向由边界算子决定**
    ///
    /// - `v < b` / `v <= b`：v 朝**上界** b 走 ⇒ 测度 `b - v`（direction = Increasing）
    /// - `v > b` / `v >= b`：v 朝**下界** b 走 ⇒ 测度 `v - b`（direction = Decreasing）
    ///
    /// 方向必须由算子给出——旧实现一律 `increasing(v, .., 1)` 是**不健全**的：
    /// 对 `while i > 0 { i = i + 1 }` 它会给出「测度 0 - i 递减」的假证明。
    /// `delta` 字段只是描述性默认值：策略 1 的实际位移从循环体赋值读取
    ///（`δv`、`δbound`），测度变化按 Direction 组合（见 `verify_rank_candidate`），
    /// 不再对 `v` 直接加一个恒为 +1 的 delta。
    fn generate_rank_candidates(
        &self,
        bounds: &[(String, (BoundOp, BoundExpr))],
    ) -> Vec<LinearMeasure> {
        let mut candidates = Vec::new();

        for (var, (op, bound)) in bounds {
            let (bound_val, bound_var) = match bound {
                BoundExpr::Const(c) => (Some(*c), None),
                BoundExpr::Var(n) => (None, Some(n.clone())),
            };
            match op {
                // 上界 + 递增 ⇒ 测度 bound - var
                BoundOp::Lt | BoundOp::Le => candidates.push(LinearMeasure::increasing(
                    var,
                    bound_var.as_deref(),
                    bound_val,
                    1,
                )),
                // 下界 + 递减 ⇒ 测度 var - bound
                BoundOp::Gt | BoundOp::Ge => candidates.push(LinearMeasure::decreasing(
                    var,
                    bound_var.as_deref(),
                    bound_val,
                    1,
                )),
            }
        }

        candidates
    }

    /// SMT 验证秩函数候选：**每次迭代测度严格递减**（RFC-027 §7.2）
    ///
    /// 测度与位移（按 Direction 推导，这是缺陷 (b) 的修法）：
    ///
    /// | direction | 测度 m | 迭代后 m' | 测度变化 Δm |
    /// |---|---|---|---|
    /// | Increasing | `bound - v` | `bound' - v'` | `δbound - δv` |
    /// | Decreasing | `v - bound` | `v' - bound'` | `δv - δbound` |
    ///
    /// 旧实现把 m' 写成 `v + delta`（delta 恒 +1）并断言 `not (m' < m)`：
    /// `v + 1 < v` 恒假 ⇒ `not(false)` 恒真 ⇒ 求解器给 Sat ⇒ 恒验证失败。
    /// 现在断言的是 `not (m + Δm < m)`：Δm < 0 时 unsat（严格递减成立）。
    ///
    /// 位移来源（健全性关键）：
    /// - `δv` 必须来自**无条件**赋值——只有每次迭代都执行才保证「每轮都动」；
    /// - 该变量的取值取全部赋值（含守卫内）的**最不利一端**（δv 最小 / 最大）；
    /// - `δbound`：常量边界为 0；变量边界取全部赋值的最不利一端（含守卫内），
    ///   任一赋值位移未知则拒绝论证；未被赋值即循环不变量（δbound = 0）。
    fn verify_rank_candidate(
        &self,
        candidate: &LinearMeasure,
        unconditional: &std::collections::HashMap<String, i128>,
        assignments: &[LoopAssignment],
        solver: &dyn Solver,
    ) -> bool {
        // 测度变量：必须每次迭代都动（无条件赋值），否则测度可能原地不动
        let Some(&dv_unconditional) = unconditional.get(&candidate.var) else {
            return false;
        };
        if dv_unconditional == 0 {
            return false;
        }
        // 该变量全部赋值的极值（含守卫内）——保守一侧参差
        let Some((dv_min, dv_max)) = Self::delta_bounds(assignments, &candidate.var) else {
            return false; // 有未知/乘法赋值：位移说不准
        };
        // 边界位移：常量边界恒 0；变量边界取保守极值
        let (db_min, db_max) = match &candidate.bound_var {
            None => (0, 0),
            Some(bv) => match Self::delta_bounds(assignments, bv) {
                Some(range) => range,
                None => return false,
            },
        };
        // 测度变化的最不利一端（越小越有利于「严格递减」）
        let delta_m = match candidate.direction {
            // m = bound - v：边界涨得最多、v 涨得最少
            Direction::Increasing => db_max - dv_min,
            // m = v - bound：v 涨得最多、边界跌得最少
            Direction::Decreasing => dv_max - db_min,
        };

        // 合成侧健全性闸门：Δm ≥ 0 的候选**明显不严格递减**（`m + Δm < m` 可满足），
        // 不交给求解器——否则「恒 Unsat」的桩求解器会把方向配错的候选也放过去。
        // 求解器仍是递减义务的判定通道（下方断言），只是候选得先指对方向。
        if delta_m >= 0 {
            return false;
        }

        let mut commands = Vec::new();
        commands.push(SMTCommand::DeclareConst("m".into(), SMTSort::Int));
        let m = SMTExpr::Atom("m".into());
        // `m + Δm`：Δm < 0 时写成减法，避免把负数字面量塞进 `+` 的参数位
        let m_prime = SMTExpr::App(
            "-".into(),
            vec![m.clone(), SMTExpr::Atom((-delta_m).to_string())],
        );
        // assert (not (< m' m))：m' = m + Δm
        let not_decreasing = SMTExpr::App(
            "not".into(),
            vec![SMTExpr::App("<".into(), vec![m_prime, m])],
        );
        commands.push(SMTCommand::Assert(not_decreasing));
        commands.push(SMTCommand::CheckSat);

        // unsat = m + Δm < m 在所有情况下成立 → 严格递减
        matches!(
            solver.solve(&commands, 50),
            crate::frontend::core::typecheck::proof::smt::ast::SMTResult::Unsat
        )
    }

    /// 策略 1b：**标志循环**（guarded exit）的秩函数
    ///
    /// 形状：`while flag { v = v ∓ k; if v ⋛ c { flag = false } }`
    ///
    /// 1. 循环条件是**裸标志变量**（不是比较式，故边界提取拿不到东西）；
    /// 2. 体内**直接**在阈值守卫内把标志置 `false`（循环出口）；
    /// 3. 标志在体内**只被写这一次**，且那次就是清零（否则标志可能被置回 true，
    ///    出口前提被破坏）；
    /// 4. `v` 每轮**无条件**动，且所有位移都在阈值方向（含守卫内也不反向）。
    ///
    /// 测度取到阈值的距离：`v ≤ c` / `v < c` 型用 `v - c`（Decreasing），
    /// `v ≥ c` / `v > c` 型用 `c - v`（Increasing）。标志未清零 ⇒ 该轮守卫未命中
    /// ⇒ 测度 ≥ 1 且每轮至少减 1 ⇒ 有限轮内命中 ⇒ 标志清零 ⇒ 下一轮条件为假 ⇒ 退出。
    ///
    /// 四条前置任一不满足即返回 `None`（宁可报「无法证明」E4021，不可误判终止）。
    fn try_flag_loop_measure(
        &self,
        condition: &Expr,
        body: &ast::Block,
        assignments: &[LoopAssignment],
        unconditional: &std::collections::HashMap<String, i128>,
    ) -> Option<LinearMeasure> {
        // 1) 条件是裸标志变量
        let Expr::Var(flag, _) = condition else {
            return None;
        };
        // 2) 标志只被写一次，且那次是字面量 false
        let (writes, has_non_false_write) = Self::flag_write_summary_block(body, flag)?;
        if writes != 1 || has_non_false_write {
            return None;
        }
        // 3) 找「阈值守卫内直接清零标志」的出口
        let (var, op, threshold) = self.find_guarded_flag_exit(body, flag)?;
        // 4) v 每轮必动（无条件），且全部位移都不朝反方向
        if !unconditional.contains_key(&var) {
            return None;
        }
        let (dv_min, dv_max) = Self::delta_bounds(assignments, &var)?;
        match op {
            // `v ≤ c` / `v < c`：v 必须递减（守卫未命中 ⇒ v - c ≥ 0）
            BoundOp::Le | BoundOp::Lt => {
                if dv_max >= 0 {
                    return None;
                }
                Some(LinearMeasure::decreasing(
                    &var,
                    None,
                    Some(threshold),
                    -dv_min,
                ))
            }
            // `v ≥ c` / `v > c`：v 必须递增（守卫未命中 ⇒ c - v ≥ 0）
            BoundOp::Gt | BoundOp::Ge => {
                if dv_min <= 0 {
                    return None;
                }
                Some(LinearMeasure::increasing(
                    &var,
                    None,
                    Some(threshold),
                    dv_max,
                ))
            }
        }
    }

    /// 在循环体里找「阈值守卫内**直接**清零标志」的出口。
    ///
    /// 两条健全性要求：
    /// 1. 只认直接写在 `if` 分支里的 `flag = false`：藏在更内层守卫里的清零只代表
    ///    「更弱的条件也会清零」，用它当出口会高估触发范围（不健全）；
    /// 2. **守卫块内不得重绑测度变量**（含写路径不可静态判定者）：1b 的测度论证
    ///    前提是「该变量每轮都朝阈值走」，守卫块里再动它就让方向判定看到的位移
    ///    不再代表整轮。
    ///
    /// 只认**常量阈值**的比较守卫（变量阈值留待后续）。
    fn find_guarded_flag_exit(
        &self,
        body: &ast::Block,
        flag: &str,
    ) -> Option<(String, BoundOp, i128)> {
        for stmt in &body.stmts {
            let (guard, branches): (&Expr, Vec<&ast::Block>) = match &stmt.kind {
                StmtKind::If {
                    condition,
                    then_branch,
                    else_if_branches,
                    ..
                } => (
                    condition.as_ref(),
                    std::iter::once(then_branch.as_ref())
                        .chain(else_if_branches.iter().map(|(_, b)| b.as_ref()))
                        .collect(),
                ),
                StmtKind::Expr(expr) => match expr.as_ref() {
                    Expr::If {
                        condition,
                        then_branch,
                        else_if_branches,
                        ..
                    } => (
                        condition.as_ref(),
                        std::iter::once(then_branch.as_ref())
                            .chain(else_if_branches.iter().map(|(_, b)| b.as_ref()))
                            .collect(),
                    ),
                    _ => continue,
                },
                _ => continue,
            };
            if !branches.iter().any(|b| Self::directly_clears_flag(b, flag)) {
                continue;
            }
            // 守卫必须是「变量 ⋛ 常量」，且**守卫块内不得再动这个变量**：1b 的
            // 测度论证前提是「该变量每轮朝阈值走」，守卫块里的重绑定会破坏它
            //（如 `if i <= 0 { i = i + 20; flag = false }`：方向判定看到的位移不再
            // 代表整轮）。保守取「守卫块内对该变量无任何赋值」，含写路径不可静态
            // 判定的形态（`flag_write_summary_*` 返回 None）也一并拒绝。
            for (var, (op, bound)) in self.extract_bounds_from_condition(guard) {
                let BoundExpr::Const(c) = bound else {
                    continue;
                };
                let guard_rewrites_var = branches
                    .iter()
                    .any(|b| !matches!(Self::flag_write_summary_block(b, &var), Some((0, _))));
                if guard_rewrites_var {
                    continue;
                }
                return Some((var, op, c));
            }
        }
        None
    }

    /// 该分支的**直接**语句里是否有 `flag = false`（两种赋值 AST 形态都认）。
    fn directly_clears_flag(
        block: &ast::Block,
        flag: &str,
    ) -> bool {
        block.stmts.iter().any(|stmt| {
            let (target, value) = match &stmt.kind {
                StmtKind::Assign {
                    target,
                    value: Some(v),
                    ..
                } => (target.as_ref(), Some(v.as_ref())),
                StmtKind::Expr(expr) => match expr.as_ref() {
                    Expr::BinOp {
                        op: BinOp::Assign,
                        left,
                        right,
                        ..
                    } => (left.as_ref(), Some(right.as_ref())),
                    _ => return false,
                },
                _ => return false,
            };
            matches!(
                (target, value),
                (Expr::Var(name, _), Some(Expr::Lit(ast::Literal::Bool(false), _))) if name == flag
            )
        })
    }

    /// 递归统计对**指定名字**的写：`(写次数, 是否含写路径不可静态判定者)`。
    ///
    /// 名字参数是通用的：1b 既用它统计标志（要求「只被清零这一条写路径」），
    /// 也用它判定**守卫块是否重绑测度变量**（要求「守卫块内不动该变量」）。
    ///
    /// `None` = 遇到本函数不建模的形态（解构赋值、字典/区间等表达式）⇒ 结论不可用，
    /// 调用方必须保守拒绝（把「说不准」当「有写」处理）。
    fn flag_write_summary_block(
        block: &ast::Block,
        flag: &str,
    ) -> Option<(usize, bool)> {
        let mut total = (0usize, false);
        for stmt in &block.stmts {
            total = Self::merge_write_summary(total, Self::flag_write_summary_stmt(stmt, flag)?);
        }
        Some(total)
    }

    fn merge_write_summary(
        a: (usize, bool),
        b: (usize, bool),
    ) -> (usize, bool) {
        (a.0 + b.0, a.1 || b.1)
    }

    fn flag_write_summary_stmt(
        stmt: &Stmt,
        flag: &str,
    ) -> Option<(usize, bool)> {
        match &stmt.kind {
            StmtKind::Assign { target, value, .. } => {
                let mut total = Self::flag_write_summary_target(target, flag, value.as_deref());
                if let Some(v) = value.as_deref() {
                    total =
                        Self::merge_write_summary(total, Self::flag_write_summary_expr(v, flag)?);
                }
                Some(total)
            }
            StmtKind::Expr(expr) => Self::flag_write_summary_expr(expr, flag),
            StmtKind::If {
                condition,
                then_branch,
                else_if_branches,
                else_branch,
                ..
            } => {
                let mut total = Self::flag_write_summary_expr(condition, flag)?;
                total = Self::merge_write_summary(
                    total,
                    Self::flag_write_summary_block(then_branch, flag)?,
                );
                for (cond, branch) in else_if_branches {
                    total = Self::merge_write_summary(
                        total,
                        Self::flag_write_summary_expr(cond, flag)?,
                    );
                    total = Self::merge_write_summary(
                        total,
                        Self::flag_write_summary_block(branch, flag)?,
                    );
                }
                if let Some(branch) = else_branch {
                    total = Self::merge_write_summary(
                        total,
                        Self::flag_write_summary_block(branch, flag)?,
                    );
                }
                Some(total)
            }
            StmtKind::For { iterable, body, .. } => {
                let total = Self::flag_write_summary_expr(iterable, flag)?;
                Some(Self::merge_write_summary(
                    total,
                    Self::flag_write_summary_block(body, flag)?,
                ))
            }
            StmtKind::Return(Some(expr)) => Self::flag_write_summary_expr(expr, flag),
            // 声明类语句不含赋值
            StmtKind::TypeDefinition { .. } | StmtKind::Use { .. } | StmtKind::Return(None) => {
                Some((0, false))
            }
            // 解构赋值等未建模形态：保守不可判
            _ => None,
        }
    }

    /// 单个赋值目标对 `flag` 的写摘要（`value` 为右值，用于判定是否字面量 false）。
    fn flag_write_summary_target(
        target: &Expr,
        flag: &str,
        value: Option<&Expr>,
    ) -> (usize, bool) {
        if matches!(target, Expr::Var(name, _) if name == flag) {
            let is_literal_false = matches!(value, Some(Expr::Lit(ast::Literal::Bool(false), _)));
            return (1, !is_literal_false);
        }
        (0, false)
    }

    fn flag_write_summary_expr(
        expr: &Expr,
        flag: &str,
    ) -> Option<(usize, bool)> {
        match expr {
            Expr::BinOp {
                op: BinOp::Assign,
                left,
                right,
                ..
            } => {
                let mut total = Self::flag_write_summary_target(left, flag, Some(right));
                total =
                    Self::merge_write_summary(total, Self::flag_write_summary_expr(left, flag)?);
                Some(Self::merge_write_summary(
                    total,
                    Self::flag_write_summary_expr(right, flag)?,
                ))
            }
            Expr::BinOp { left, right, .. } => {
                let total = Self::flag_write_summary_expr(left, flag)?;
                Some(Self::merge_write_summary(
                    total,
                    Self::flag_write_summary_expr(right, flag)?,
                ))
            }
            Expr::UnOp { expr, .. }
            | Expr::Cast { expr, .. }
            | Expr::Try { expr, .. }
            | Expr::Ref { expr, .. }
            | Expr::Borrow { expr, .. }
            | Expr::FieldAccess { expr, .. } => Self::flag_write_summary_expr(expr, flag),
            Expr::Call { func, args, .. } => {
                let mut total = Self::flag_write_summary_expr(func, flag)?;
                for arg in args {
                    total =
                        Self::merge_write_summary(total, Self::flag_write_summary_expr(arg, flag)?);
                }
                Some(total)
            }
            Expr::Index { expr, index, .. } => {
                let total = Self::flag_write_summary_expr(expr, flag)?;
                Some(Self::merge_write_summary(
                    total,
                    Self::flag_write_summary_expr(index, flag)?,
                ))
            }
            Expr::Tuple(items, _) | Expr::List(items, _) => {
                let mut total = (0usize, false);
                for item in items {
                    total = Self::merge_write_summary(
                        total,
                        Self::flag_write_summary_expr(item, flag)?,
                    );
                }
                Some(total)
            }
            Expr::If {
                condition,
                then_branch,
                else_if_branches,
                else_branch,
                ..
            } => {
                let mut total = Self::flag_write_summary_expr(condition, flag)?;
                total = Self::merge_write_summary(
                    total,
                    Self::flag_write_summary_block(then_branch, flag)?,
                );
                for (cond, branch) in else_if_branches {
                    total = Self::merge_write_summary(
                        total,
                        Self::flag_write_summary_expr(cond, flag)?,
                    );
                    total = Self::merge_write_summary(
                        total,
                        Self::flag_write_summary_block(branch, flag)?,
                    );
                }
                if let Some(branch) = else_branch {
                    total = Self::merge_write_summary(
                        total,
                        Self::flag_write_summary_block(branch, flag)?,
                    );
                }
                Some(total)
            }
            Expr::While {
                condition, body, ..
            } => {
                let total = Self::flag_write_summary_expr(condition, flag)?;
                Some(Self::merge_write_summary(
                    total,
                    Self::flag_write_summary_block(body, flag)?,
                ))
            }
            Expr::For { iterable, body, .. } | Expr::SpawnFor { iterable, body, .. } => {
                let total = Self::flag_write_summary_expr(iterable, flag)?;
                Some(Self::merge_write_summary(
                    total,
                    Self::flag_write_summary_block(body, flag)?,
                ))
            }
            Expr::Block(block) => Self::flag_write_summary_block(block, flag),
            Expr::Lambda { body, .. } | Expr::Unsafe { body, .. } => {
                Self::flag_write_summary_block(body, flag)
            }
            Expr::Return(Some(inner), _) => Self::flag_write_summary_expr(inner, flag),
            // 无写路径的叶子
            Expr::Lit(..)
            | Expr::Var(..)
            | Expr::Return(None, _)
            | Expr::Break(_)
            | Expr::Continue(_) => Some((0, false)),
            // 其余形态（Dict/In/Spawn/FString/ListComp/Error…）：保守不可判
            _ => None,
        }
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
pub(crate) fn substitute_const_expr(
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
pub(crate) fn negate_guard(
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
