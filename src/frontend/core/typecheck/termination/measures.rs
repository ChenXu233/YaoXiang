//! 度量分析器
//!
//! 从循环体中提取候选度量，验证度量是否严格递减。

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
