//! RFC-027 编译期求值器
//!
//! 统一的编译期求值引擎，支持：
//! - 类型归约（If/Match/Nat 条件类型）
//! - ConstExpr 求值（编译期常量表达式）
//! - β-归约（类型级函数应用）
//!
//! # 示例
//! ```yaoxiang
//! type If[True, Int, String] => Int
//! type If[False, Int, String] => String
//!
//! type NonEmpty[T] = If[T != Void, T, Never]
//! ```

use std::collections::{HashMap, HashSet};

use crate::frontend::core::types::{MonoType, ConstValue};
use crate::frontend::core::types::const_data::{BinOp, ConstExpr, UnOp};
use super::TypeLevelError;
use super::TypeLevelResult;
use crate::frontend::core::typecheck::TypeEnvironment;
use crate::frontend::core::typecheck::proof::budget::BudgetTracker;
use super::dependent_types::DependentTypeEnv;

/// 类型求值错误
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EvalError {
    MaxDepthExceeded,
    CycleDetected(String),
    ArithmeticError(String),
    TypeMismatch(String),
}

/// 求值配置
#[derive(Debug, Clone, Default)]
pub struct EvalConfig {
    /// 最大求值深度
    pub max_depth: usize,

    /// 是否启用缓存
    pub enable_cache: bool,

    /// 是否启用循环检测
    pub cycle_detection: bool,
}

/// 编译期求值器
///
/// 负责在编译期对条件类型进行求值：
/// - If 条件类型：基于布尔条件选择类型
/// - Match 类型：基于模式匹配选择类型
/// - Nat 运算：自然数算术运算
/// - ConstExpr 求值：编译期常量表达式
/// - β-归约：类型级函数应用
pub struct Evaluator<'a> {
    /// 类型求值缓存
    /// 避免重复求值相同类型
    cache: HashMap<MonoType, Result<MonoType, EvalError>>,

    /// 依赖追踪
    /// 记录类型之间的依赖关系
    dependencies: HashMap<MonoType, HashSet<MonoType>>,

    /// 类型环境引用
    env: &'a TypeEnvironment,

    /// 求解预算追踪器
    budget: &'a BudgetTracker,

    /// 已访问类型（用于循环检测）
    visiting: HashSet<MonoType>,
    /// 求值配置
    config: EvalConfig,

    /// 依赖类型环境（用于解析类型族）
    dep_env: &'a DependentTypeEnv,
}

impl<'a> Evaluator<'a> {
    /// 创建新的求值器
    pub fn new(
        env: &'a TypeEnvironment,
        budget: &'a BudgetTracker,
        dep_env: &'a DependentTypeEnv,
    ) -> Self {
        Self {
            cache: HashMap::new(),
            dependencies: HashMap::new(),
            env,
            budget,
            dep_env,
            visiting: HashSet::new(),
            config: EvalConfig {
                max_depth: 100, // 设置合理的默认深度
                enable_cache: true,
                cycle_detection: true,
            },
        }
    }

    /// 创建带配置的求值器
    pub fn with_config(
        env: &'a TypeEnvironment,
        budget: &'a BudgetTracker,
        config: EvalConfig,
        dep_env: &'a DependentTypeEnv,
    ) -> Self {
        Self {
            cache: HashMap::new(),
            dependencies: HashMap::new(),
            env,
            budget,
            dep_env,
            visiting: HashSet::new(),
            config,
        }
    }

    // ============ ConstExpr 求值 ============

    /// 求值编译期表达式
    pub fn eval_expr(
        &mut self,
        expr: &ConstExpr,
        bindings: &HashMap<String, ConstValue>,
    ) -> Result<ConstValue, EvalError> {
        // 消费预算
        if !self.budget.spend() {
            return Err(EvalError::MaxDepthExceeded);
        }

        match expr {
            ConstExpr::Lit(val) => Ok(val.clone()),

            ConstExpr::NamedVar(name) => bindings
                .get(name)
                .cloned()
                .ok_or_else(|| EvalError::TypeMismatch(format!("未绑定变量: {}", name))),

            ConstExpr::Var(_const_var) => Err(EvalError::TypeMismatch(
                "ConstVar 求值需要类型环境上下文".into(),
            )),

            ConstExpr::BinOp { op, left, right } => {
                let l = self.eval_expr(left, bindings)?;
                let r = self.eval_expr(right, bindings)?;
                eval_binop(*op, &l, &r)
            }

            ConstExpr::UnOp { op, expr: inner } => {
                let v = self.eval_expr(inner, bindings)?;
                eval_unop(*op, &v)
            }

            ConstExpr::If {
                condition,
                then_branch,
                else_branch,
            } => {
                let cond = self.eval_expr(condition, bindings)?;
                match cond {
                    ConstValue::Bool(true) => self.eval_expr(then_branch, bindings),
                    ConstValue::Bool(false) => self.eval_expr(else_branch, bindings),
                    _ => Err(EvalError::TypeMismatch("If 条件必须为 Bool".into())),
                }
            }

            ConstExpr::Call { .. } | ConstExpr::Range { .. } => {
                Err(EvalError::TypeMismatch("阶段 1 不支持的表达式".into()))
            }
        }
    }

    // ============ 类型求值 ============

    /// 求值类型
    pub fn eval(
        &mut self,
        ty: &MonoType,
    ) -> Result<MonoType, EvalError> {
        self.eval_with_depth(ty, 0)
    }

    /// 带深度限制的求值
    fn eval_with_depth(
        &mut self,
        ty: &MonoType,
        depth: usize,
    ) -> Result<MonoType, EvalError> {
        // 检查深度限制
        if depth > self.config.max_depth {
            return Err(EvalError::MaxDepthExceeded);
        }

        // 检查缓存
        if self.config.enable_cache {
            if let Some(cached) = self.cache.get(ty).cloned() {
                return cached;
            }
        }

        // 循环检测
        if self.config.cycle_detection && self.visiting.contains(ty) {
            return Err(EvalError::CycleDetected(format!(
                "Cycle detected in type: {}",
                ty
            )));
        }
        self.visiting.insert(ty.clone());

        let result = self.eval_internal(ty, depth);

        // 移除循环检测标记
        self.visiting.remove(ty);

        // 缓存结果
        if self.config.enable_cache {
            self.cache.insert(ty.clone(), result.clone());
        }

        result
    }

    /// 内部求值逻辑
    fn eval_internal(
        &mut self,
        ty: &MonoType,
        depth: usize,
    ) -> Result<MonoType, EvalError> {
        match ty {
            // 处理 If 条件类型
            MonoType::TypeRef(name) if name == "If" => self.eval_if_type(ty, depth),

            // 处理 Match 类型
            MonoType::TypeRef(name) if name == "Match" => self.eval_match_type(ty, depth),

            // 处理 Nat 运算
            MonoType::TypeRef(name) if name == "Nat" => self.eval_nat_type(ty, depth),

            // 处理 IsTrue 类型族：IsTrue(true) => Void, IsTrue(false) => Never
            MonoType::TypeRef(name) if name == "IsTrue" || name.starts_with("IsTrue(") => {
                self.eval_istrue(ty, depth)
            }

            // 处理 Assert 类型族：Assert(x) => IsTrue(x) 别名
            MonoType::TypeRef(name) if name == "Assert" || name.starts_with("Assert(") => {
                self.eval_istrue(ty, depth)
            }

            // 处理类型引用
            MonoType::TypeRef(name) => self.eval_type_ref(name, depth),

            // 精化类型：只归约基类型——约束由 Layer 3 处理
            MonoType::Refined { base, .. } => self.eval_with_depth(base, depth + 1),

            // DepFn 不参与类型归约
            MonoType::DepFn { .. } => Ok(ty.clone()),

            // 其他类型直接返回
            _ => Ok(ty.clone()),
        }
    }

    // ============ If 条件类型求值 ============

    /// 求值 If[C, T, E] 类型
    ///
    /// 基于布尔条件 C 在编译期选择 T 或 E
    fn eval_if_type(
        &mut self,
        ty: &MonoType,
        depth: usize,
    ) -> Result<MonoType, EvalError> {
        // 尝试从类型引用中提取参数
        let (condition, true_branch, false_branch) = match self.extract_if_args(ty) {
            Some(args) => args,
            None => return Ok(ty.clone()),
        };

        // 递归求值条件
        let cond_result = self.eval_condition(&condition, depth);

        match cond_result {
            Ok(true) => {
                // 条件为 true，求值 true 分支
                self.eval_with_depth(&true_branch, depth + 1)
            }
            Ok(false) => {
                // 条件为 false，求值 false 分支
                self.eval_with_depth(&false_branch, depth + 1)
            }
            Err(e) => Err(e),
        }
    }

    /// 从 If 类型引用中提取参数
    fn extract_if_args(
        &self,
        ty: &MonoType,
    ) -> Option<(MonoType, MonoType, MonoType)> {
        // If 类型的参数格式: If(condition, true_branch, false_branch)
        // 通过解析类型引用名称来提取
        if let MonoType::TypeRef(name) = ty {
            if let Some(args) = Self::parse_generic_args(name) {
                if args.len() == 3 {
                    return Some((
                        self.parse_type(&args[0])?,
                        self.parse_type(&args[1])?,
                        self.parse_type(&args[2])?,
                    ));
                }
            }
        }
        None
    }

    /// Parse generic arguments from type name: `Name(T1, T2)`
    fn parse_generic_args(name: &str) -> Option<Vec<String>> {
        if !name.contains('(') || !name.ends_with(')') {
            return None;
        }

        let inner = &name[name.find('(').unwrap() + 1..name.len() - 1];
        let mut args = Vec::new();
        let mut current = String::new();
        let mut depth = 0;

        for c in inner.chars() {
            match c {
                ',' if depth == 0 => {
                    args.push(current.trim().to_string());
                    current = String::new();
                }
                '(' => {
                    depth += 1;
                    current.push(c);
                }
                ')' if depth > 0 => {
                    depth -= 1;
                    current.push(c);
                }
                _ => current.push(c),
            }
        }

        if !current.trim().is_empty() {
            args.push(current.trim().to_string());
        }

        if !args.is_empty() {
            Some(args)
        } else {
            None
        }
    }

    /// 解析类型字符串为 MonoType
    #[doc(hidden)]
    pub fn parse_type(
        &self,
        s: &str,
    ) -> Option<MonoType> {
        match s.trim() {
            "Void" => Some(MonoType::Void),
            "Bool" => Some(MonoType::Bool),
            "Int" => Some(MonoType::Int(64)),
            "Float" => Some(MonoType::Float(64)),
            "Char" => Some(MonoType::Char),
            "String" => Some(MonoType::String),
            "Never" => Some(MonoType::Never),
            "True" => Some(MonoType::TypeRef("True".to_string())),
            "False" => Some(MonoType::TypeRef("False".to_string())),
            s if s.starts_with("If(") => Some(MonoType::TypeRef(s.to_string())),
            s if s.starts_with("Match(") => Some(MonoType::TypeRef(s.to_string())),
            s if s.starts_with("Nat(") => Some(MonoType::TypeRef(s.to_string())),
            s => Some(MonoType::TypeRef(s.to_string())),
        }
    }

    /// 求值条件
    fn eval_condition(
        &mut self,
        condition: &MonoType,
        depth: usize,
    ) -> Result<bool, EvalError> {
        match condition {
            // 布尔字面量
            MonoType::TypeRef(name) if name == "True" => Ok(true),
            MonoType::TypeRef(name) if name == "False" => Ok(false),

            // 等式条件: L == R
            MonoType::TypeRef(name) if name.starts_with("Eq(") => {
                self.eval_eq_condition(name, depth)
            }

            // 不等条件: L != R
            MonoType::TypeRef(name) if name.starts_with("Neq(") => {
                self.eval_eq_condition(name, depth).map(|b| !b)
            }

            // 组合条件: And
            MonoType::TypeRef(name) if name.starts_with("And(") => {
                self.eval_and_condition(name, depth)
            }

            // 组合条件: Or
            MonoType::TypeRef(name) if name.starts_with("Or(") => {
                self.eval_or_condition(name, depth)
            }

            // 否定条件: Not
            MonoType::TypeRef(name) if name.starts_with("Not(") => self
                .eval_condition(&self.extract_inner_type(name), depth)
                .map(|b| !b),

            // 类型变量无法确定
            MonoType::TypeVar(_) => Ok(true),

            // 其他情况需要进一步检查
            _ => Ok(true),
        }
    }

    /// 求值等式条件
    fn eval_eq_condition(
        &mut self,
        name: &str,
        depth: usize,
    ) -> Result<bool, EvalError> {
        if let Some(args) = Self::parse_generic_args(name) {
            if args.len() == 2 {
                let left = self
                    .parse_type(&args[0])
                    .unwrap_or_else(|| MonoType::TypeRef(args[0].clone()));
                let right = self
                    .parse_type(&args[1])
                    .unwrap_or_else(|| MonoType::TypeRef(args[1].clone()));

                // 递归求值操作数
                let left_eval = self.eval_with_depth(&left, depth + 1);
                let right_eval = self.eval_with_depth(&right, depth + 1);

                // 如果两边都已确定值
                if let (Ok(l), Ok(r)) = (left_eval, right_eval) {
                    return Ok(l == r);
                }
            }
        }
        Ok(true)
    }

    /// 求值 And 条件
    fn eval_and_condition(
        &mut self,
        name: &str,
        depth: usize,
    ) -> Result<bool, EvalError> {
        if let Some(args) = Self::parse_generic_args(name) {
            if args.len() == 2 {
                let left = self
                    .parse_type(&args[0])
                    .unwrap_or_else(|| MonoType::TypeRef(args[0].clone()));
                let right = self
                    .parse_type(&args[1])
                    .unwrap_or_else(|| MonoType::TypeRef(args[1].clone()));

                let left_eval = self.eval_condition(&left, depth + 1);
                let right_eval = self.eval_condition(&right, depth + 1);

                match (left_eval, right_eval) {
                    (Ok(false), _) | (_, Ok(false)) => Ok(false),
                    (Ok(true), Ok(true)) => Ok(true),
                    (Err(e), _) | (_, Err(e)) => Err(e),
                }
            } else {
                Ok(true)
            }
        } else {
            Ok(true)
        }
    }

    /// 求值 Or 条件
    fn eval_or_condition(
        &mut self,
        name: &str,
        depth: usize,
    ) -> Result<bool, EvalError> {
        if let Some(args) = Self::parse_generic_args(name) {
            if args.len() == 2 {
                let left = self
                    .parse_type(&args[0])
                    .unwrap_or_else(|| MonoType::TypeRef(args[0].clone()));
                let right = self
                    .parse_type(&args[1])
                    .unwrap_or_else(|| MonoType::TypeRef(args[1].clone()));

                let left_eval = self.eval_condition(&left, depth + 1);
                let right_eval = self.eval_condition(&right, depth + 1);

                match (left_eval, right_eval) {
                    (Ok(true), _) | (_, Ok(true)) => Ok(true),
                    (Ok(false), Ok(false)) => Ok(false),
                    (Err(e), _) | (_, Err(e)) => Err(e),
                }
            } else {
                Ok(true)
            }
        } else {
            Ok(true)
        }
    }

    /// 从类型引用中提取内部类型
    fn extract_inner_type(
        &self,
        name: &str,
    ) -> MonoType {
        if let Some(start) = name.find('(') {
            if let Some(end) = name.rfind(')') {
                let inner = &name[start + 1..end];
                return self
                    .parse_type(inner)
                    .unwrap_or_else(|| MonoType::TypeRef(inner.to_string()));
            }
        }
        MonoType::Void
    }

    // ============ Match 类型求值 ============

    /// 求值 Match[T] 类型
    ///
    /// 基于模式匹配选择类型
    fn eval_match_type(
        &mut self,
        ty: &MonoType,
        depth: usize,
    ) -> Result<MonoType, EvalError> {
        // 尝试从类型引用中提取参数
        let (target, arms) = match self.extract_match_args(ty) {
            Some(args) => args,
            None => return Ok(ty.clone()),
        };

        // 递归求值目标类型
        let target_eval = self.eval_with_depth(&target, depth + 1);

        match target_eval {
            Ok(target_ty) => {
                // 查找匹配的分支
                for (pattern, result) in arms {
                    if self.pattern_matches(&target_ty, &pattern) {
                        return self.eval_with_depth(&result, depth + 1);
                    }
                }
                // 没有匹配分支
                Err(EvalError::TypeMismatch(
                    "No matching arm in MatchType".to_string(),
                ))
            }
            Err(e) => Err(e),
        }
    }

    /// 从 Match 类型引用中提取参数
    fn extract_match_args(
        &self,
        ty: &MonoType,
    ) -> Option<(MonoType, Vec<(MonoType, MonoType)>)> {
        if let MonoType::TypeRef(name) = ty {
            if !name.starts_with("Match(") {
                return None;
            }

            // 简化实现：Match 的格式为 Match(target, pattern1 => result1, pattern2 => result2, ...)
            if let Some(args) = Self::parse_generic_args(name) {
                if args.len() >= 2 {
                    let target = self.parse_type(&args[0])?;
                    let mut arms = Vec::new();

                    for arg in &args[1..] {
                        if let Some((pattern, result)) = self.parse_match_arm(arg) {
                            arms.push((pattern, result));
                        }
                    }

                    return Some((target, arms));
                }
            }
        }
        None
    }

    /// 解析 match 分支
    fn parse_match_arm(
        &self,
        s: &str,
    ) -> Option<(MonoType, MonoType)> {
        // 格式: pattern => result
        if let Some((pattern_str, result_str)) = s.split_once("=>") {
            let pattern = self.parse_type(pattern_str.trim())?;
            let result = self.parse_type(result_str.trim())?;
            return Some((pattern, result));
        }
        None
    }

    /// 检查模式是否匹配目标类型
    fn pattern_matches(
        &self,
        target: &MonoType,
        pattern: &MonoType,
    ) -> bool {
        match pattern {
            // 通配符匹配任何类型
            MonoType::TypeRef(name) if name == "_" => true,

            // 类型变量需要类型检查确定
            MonoType::TypeVar(_) => false,

            // 精确类型匹配
            _ => target == pattern,
        }
    }

    // ============ Nat 算术求值 ============

    /// 求值 Nat 类型
    ///
    /// 执行自然数算术运算
    fn eval_nat_type(
        &mut self,
        ty: &MonoType,
        depth: usize,
    ) -> Result<MonoType, EvalError> {
        if let MonoType::TypeRef(name) = ty {
            if let Some((op, args)) = self.extract_nat_args(name) {
                return self.eval_nat_op(&op, &args, depth);
            }
        }
        Ok(ty.clone())
    }

    /// 提取 Nat 运算参数
    fn extract_nat_args(
        &self,
        name: &str,
    ) -> Option<(String, Vec<MonoType>)> {
        if !name.starts_with("Nat(") || !name.ends_with(')') {
            return None;
        }

        let inner = &name[4..name.len() - 1];

        // 解析操作和参数
        if let Some((op, args_str)) = inner.split_once(',') {
            let op = op.trim().to_string();
            let mut args = Vec::new();

            for arg in args_str.split(',') {
                if let Some(ty) = self.parse_type(arg.trim()) {
                    args.push(ty);
                }
            }

            return Some((op, args));
        }

        None
    }

    /// 求值 Nat 运算
    fn eval_nat_op(
        &mut self,
        op: &str,
        args: &[MonoType],
        depth: usize,
    ) -> Result<MonoType, EvalError> {
        // 递归求值参数
        let mut eval_args = Vec::new();
        for arg in args {
            match self.eval_with_depth(arg, depth + 1) {
                Ok(v) => eval_args.push(v),
                Err(e) => return Err(e),
            }
        }

        match op {
            // 加法: Nat<Add, a, b>
            "Add" if eval_args.len() == 2 => self.nat_add(&eval_args[0], &eval_args[1]),

            // 减法: Nat<Sub, a, b>
            "Sub" if eval_args.len() == 2 => self.nat_sub(&eval_args[0], &eval_args[1]),

            // 乘法: Nat<Mul, a, b>
            "Mul" if eval_args.len() == 2 => self.nat_mul(&eval_args[0], &eval_args[1]),

            // 除法: Nat<Div, a, b>
            "Div" if eval_args.len() == 2 => self.nat_div(&eval_args[0], &eval_args[1]),

            // 取模: Nat<Mod, a, b>
            "Mod" if eval_args.len() == 2 => self.nat_mod(&eval_args[0], &eval_args[1]),

            // 比较: Nat<Eq, a, b> -> Bool
            "Eq" if eval_args.len() == 2 => self.nat_eq(&eval_args[0], &eval_args[1]).map(|b| {
                MonoType::TypeRef(if b {
                    "True".to_string()
                } else {
                    "False".to_string()
                })
            }),

            // 小于: Nat<Lt, a, b> -> Bool
            "Lt" if eval_args.len() == 2 => self.nat_lt(&eval_args[0], &eval_args[1]).map(|b| {
                MonoType::TypeRef(if b {
                    "True".to_string()
                } else {
                    "False".to_string()
                })
            }),

            _ => Err(EvalError::ArithmeticError(format!(
                "Unknown Nat operation: {}",
                op
            ))),
        }
    }

    /// Nat 加法
    fn nat_add(
        &self,
        a: &MonoType,
        b: &MonoType,
    ) -> Result<MonoType, EvalError> {
        match (self.extract_nat_value(a), self.extract_nat_value(b)) {
            (Some(na), Some(nb)) => {
                let result = na + nb;
                self.nat_literal(result)
            }
            _ => Ok(MonoType::TypeRef(format!("Nat(Add, {:?}, {:?})", a, b))),
        }
    }

    /// Nat 减法
    fn nat_sub(
        &self,
        a: &MonoType,
        b: &MonoType,
    ) -> Result<MonoType, EvalError> {
        match (self.extract_nat_value(a), self.extract_nat_value(b)) {
            (Some(na), Some(nb)) => {
                if nb > na {
                    Err(EvalError::ArithmeticError("Nat underflow".to_string()))
                } else {
                    let result = na - nb;
                    self.nat_literal(result)
                }
            }
            _ => Ok(MonoType::TypeRef(format!("Nat(Sub, {:?}, {:?})", a, b))),
        }
    }

    /// Nat 乘法
    fn nat_mul(
        &self,
        a: &MonoType,
        b: &MonoType,
    ) -> Result<MonoType, EvalError> {
        match (self.extract_nat_value(a), self.extract_nat_value(b)) {
            (Some(na), Some(nb)) => {
                let result = na * nb;
                self.nat_literal(result)
            }
            _ => Ok(MonoType::TypeRef(format!("Nat(Mul, {:?}, {:?})", a, b))),
        }
    }

    /// Nat 除法
    fn nat_div(
        &self,
        a: &MonoType,
        b: &MonoType,
    ) -> Result<MonoType, EvalError> {
        match (self.extract_nat_value(a), self.extract_nat_value(b)) {
            (Some(na), Some(nb)) => {
                if nb == 0 {
                    Err(EvalError::ArithmeticError(
                        "Nat division by zero".to_string(),
                    ))
                } else {
                    let result = na / nb;
                    self.nat_literal(result)
                }
            }
            _ => Ok(MonoType::TypeRef(format!("Nat(Div, {:?}, {:?})", a, b))),
        }
    }

    /// Nat 取模
    fn nat_mod(
        &self,
        a: &MonoType,
        b: &MonoType,
    ) -> Result<MonoType, EvalError> {
        match (self.extract_nat_value(a), self.extract_nat_value(b)) {
            (Some(na), Some(nb)) => {
                if nb == 0 {
                    Err(EvalError::ArithmeticError("Nat modulo by zero".to_string()))
                } else {
                    let result = na % nb;
                    self.nat_literal(result)
                }
            }
            _ => Ok(MonoType::TypeRef(format!("Nat(Mod, {:?}, {:?})", a, b))),
        }
    }

    /// Nat 相等比较
    fn nat_eq(
        &self,
        a: &MonoType,
        b: &MonoType,
    ) -> Result<bool, EvalError> {
        match (self.extract_nat_value(a), self.extract_nat_value(b)) {
            (Some(na), Some(nb)) => Ok(na == nb),
            _ => Ok(true),
        }
    }

    /// Nat 小于比较
    fn nat_lt(
        &self,
        a: &MonoType,
        b: &MonoType,
    ) -> Result<bool, EvalError> {
        match (self.extract_nat_value(a), self.extract_nat_value(b)) {
            (Some(na), Some(nb)) => Ok(na < nb),
            _ => Ok(true),
        }
    }

    /// 从 MonoType 提取 Nat 值
    fn extract_nat_value(
        &self,
        ty: &MonoType,
    ) -> Option<i128> {
        match ty {
            // 字面量整数值
            MonoType::Literal {
                value: ConstValue::Int(n),
                ..
            } => Some(*n),
            // 类型引用形式的 Nat 字面量: Nat<5>
            MonoType::TypeRef(name) if name.starts_with("Nat(") => {
                if let Some(inner) = Self::extract_nat_literal(name) {
                    return Some(inner);
                }
                None
            }
            // 直接的整数值
            MonoType::Int(n) => Some(*n as i128),
            _ => None,
        }
    }

    /// 从类型引用中提取 Nat 字面量值
    fn extract_nat_literal(name: &str) -> Option<i128> {
        if let Some(start) = name.find('(') {
            if let Some(end) = name.rfind(')') {
                let inner = &name[start + 1..end];
                return inner.parse::<i128>().ok();
            }
        }
        None
    }

    /// 创建 Nat 字面量类型
    fn nat_literal(
        &self,
        n: i128,
    ) -> Result<MonoType, EvalError> {
        Ok(MonoType::TypeRef(format!("Nat({})", n)))
    }

    // ============ 类型引用求值 ============

    /// 求值类型引用
    fn eval_type_ref(
        &mut self,
        name: &str,
        depth: usize,
    ) -> Result<MonoType, EvalError> {
        // 先尝试解析为类型族调用
        if let Some(result) = self.eval_type_family_call(name, depth)? {
            return Ok(result);
        }

        // 检查类型环境中的类型定义
        if let Some(poly) = self.env.types.get(name) {
            return self.eval_with_depth(&poly.body, depth + 1);
        }

        // 类型引用本身
        Ok(MonoType::TypeRef(name.to_string()))
    }

    /// 尝试求值类型族调用
    ///
    /// 检查 name 是否为类型族调用（如 "factorial(Zero)"），
    /// 如果是且类型族已注册，则进行实例化归约。
    fn eval_type_family_call(
        &mut self,
        name: &str,
        depth: usize,
    ) -> Result<Option<MonoType>, EvalError> {
        // 解析函数名和参数
        let (family_name, arg_strs) = match Self::parse_type_family_call(name) {
            Some(pair) => pair,
            None => return Ok(None),
        };

        // 查找类型族
        let family = match self.dep_env.get_type_family(&family_name) {
            Some(f) => f,
            None => return Ok(None),
        };

        // 递归归约参数
        let mut reduced_args = Vec::new();
        for arg_str in &arg_strs {
            let parsed = match self.parse_type(arg_str) {
                Some(t) => t,
                None => return Ok(None),
            };
            let reduced = self.eval_with_depth(&parsed, depth + 1)?;
            reduced_args.push(reduced);
        }

        // 实例化类型族
        match family.instantiate(&reduced_args) {
            Some(def) => {
                let result = def.into_type();
                // 递归归约结果（可能产生新的类型族调用）
                self.eval_with_depth(&result, depth + 1).map(Some)
            }
            None => Ok(None),
        }
    }

    /// 解析类型族调用字符串
    ///
    /// 将 "factorial(Zero)" 解析为 ("factorial", ["Zero"])
    fn parse_type_family_call(name: &str) -> Option<(String, Vec<String>)> {
        let args = Self::parse_generic_args(name)?;
        if args.is_empty() {
            return None;
        }
        // 函数名是 name 中括号前的部分
        let paren_pos = name.find('(')?;
        let family_name = name[..paren_pos].to_string();
        Some((family_name, args))
    }

    /// 求值 IsTrue/Assert 类型族
    ///
    /// IsTrue(true) => Void
    /// IsTrue(false) => Never
    /// IsTrue(x) 当 x 不可归约时保持不变
    /// Assert(x) 委托给 IsTrue(x)
    fn eval_istrue(
        &mut self,
        ty: &MonoType,
        _depth: usize,
    ) -> Result<MonoType, EvalError> {
        let name = match ty {
            MonoType::TypeRef(n) => n,
            _ => return Ok(ty.clone()),
        };
        let args = Self::parse_generic_args(name);
        let arg = match args {
            Some(a) if a.len() == 1 => a.into_iter().next().unwrap(),
            _ => return Ok(ty.clone()),
        };
        let arg_ty = match self.parse_type(&arg) {
            Some(t) => t,
            None => return Ok(ty.clone()),
        };
        match &arg_ty {
            MonoType::TypeRef(n) if n == "true" => Ok(MonoType::Void),
            MonoType::TypeRef(n) if n == "false" => Ok(MonoType::Never),
            _ => {
                // 尝试归约参数——可能是可计算的表达式
                let reduced_arg = self.eval(&arg_ty)?;
                match &reduced_arg {
                    MonoType::TypeRef(n) if n == "true" => Ok(MonoType::Void),
                    MonoType::TypeRef(n) if n == "false" => Ok(MonoType::Never),
                    _ => Ok(ty.clone()),
                }
            }
        }
    }

    // ============ 公共 API ============

    /// 快速求值 If 条件类型
    ///
    /// 传入条件、true 分支和 false 分支，直接返回求值结果
    pub fn eval_if(
        &mut self,
        condition: &MonoType,
        true_branch: &MonoType,
        false_branch: &MonoType,
    ) -> Result<MonoType, EvalError> {
        let cond_result = self.eval_condition(condition, 0);

        match cond_result {
            Ok(true) => self.eval(true_branch),
            Ok(false) => self.eval(false_branch),
            Err(e) => Err(e),
        }
    }

    /// 快速求值 Match 表达式
    ///
    /// 传入目标类型和分支列表，返回匹配的结果类型
    pub fn eval_match(
        &mut self,
        target: &MonoType,
        arms: Vec<(MonoType, MonoType)>,
    ) -> Result<MonoType, EvalError> {
        let target_eval = self.eval(target);

        match target_eval {
            Ok(target_ty) => {
                for (pattern, result) in arms {
                    if self.pattern_matches(&target_ty, &pattern) {
                        return self.eval(&result);
                    }
                }
                Err(EvalError::TypeMismatch(
                    "No matching arm in MatchType".to_string(),
                ))
            }
            Err(e) => Err(e),
        }
    }

    /// 快速求值 Nat 算术表达式
    ///
    /// 传入操作和参数列表，返回 Nat 结果类型
    pub fn eval_nat(
        &mut self,
        op: &str,
        args: &[MonoType],
    ) -> Result<MonoType, EvalError> {
        self.eval_nat_op(op, args, 0)
    }

    /// 清空缓存
    pub fn clear_cache(&mut self) {
        self.cache.clear();
    }

    /// 清空依赖追踪
    pub fn clear_dependencies(&mut self) {
        self.dependencies.clear();
    }

    /// 获取缓存统计
    pub fn cache_stats(&self) -> (usize, usize) {
        (self.cache.len(), self.dependencies.len())
    }
}

// ============ ConstValue 辅助方法 ============

impl ConstValue {
    pub fn as_int(&self) -> Result<i128, EvalError> {
        match self {
            ConstValue::Int(n) => Ok(*n),
            _ => Err(EvalError::TypeMismatch(format!(
                "期望 Int，实际: {:?}",
                self
            ))),
        }
    }

    pub fn as_bool(&self) -> Result<bool, EvalError> {
        match self {
            ConstValue::Bool(b) => Ok(*b),
            _ => Err(EvalError::TypeMismatch(format!(
                "期望 Bool，实际: {:?}",
                self
            ))),
        }
    }
}

// ============ 自由函数 ============

/// 二元运算求值
fn eval_binop(
    op: BinOp,
    left: &ConstValue,
    right: &ConstValue,
) -> Result<ConstValue, EvalError> {
    match op {
        BinOp::Add => Ok(ConstValue::Int(left.as_int()? + right.as_int()?)),
        BinOp::Sub => Ok(ConstValue::Int(left.as_int()? - right.as_int()?)),
        BinOp::Mul => Ok(ConstValue::Int(left.as_int()? * right.as_int()?)),
        BinOp::Div => {
            let r = right.as_int()?;
            if r == 0 {
                return Err(EvalError::ArithmeticError("除零".into()));
            }
            Ok(ConstValue::Int(left.as_int()? / r))
        }
        BinOp::Mod => {
            let r = right.as_int()?;
            if r == 0 {
                return Err(EvalError::ArithmeticError("模零".into()));
            }
            Ok(ConstValue::Int(left.as_int()? % r))
        }
        BinOp::Gt => Ok(ConstValue::Bool(left.as_int()? > right.as_int()?)),
        BinOp::Ge => Ok(ConstValue::Bool(left.as_int()? >= right.as_int()?)),
        BinOp::Lt => Ok(ConstValue::Bool(left.as_int()? < right.as_int()?)),
        BinOp::Le => Ok(ConstValue::Bool(left.as_int()? <= right.as_int()?)),
        BinOp::Eq => Ok(ConstValue::Bool(left == right)),
        BinOp::Ne => Ok(ConstValue::Bool(left != right)),
        BinOp::And => Ok(ConstValue::Bool(left.as_bool()? && right.as_bool()?)),
        BinOp::Or => Ok(ConstValue::Bool(left.as_bool()? || right.as_bool()?)),
        _ => Err(EvalError::TypeMismatch(format!(
            "不支持的二元运算: {:?}",
            op
        ))),
    }
}

/// 一元运算求值
fn eval_unop(
    op: UnOp,
    val: &ConstValue,
) -> Result<ConstValue, EvalError> {
    match op {
        UnOp::Not => Ok(ConstValue::Bool(!val.as_bool()?)),
        _ => Err(EvalError::TypeMismatch(format!(
            "不支持的一元运算: {:?}",
            op
        ))),
    }
}

/// 替换类型中的类型引用：将 body 中所有 TypeRef(name) 替换为 replacement
#[allow(dead_code)]
fn substitute_type(
    body: &MonoType,
    param_name: &str,
    replacement: &MonoType,
) -> MonoType {
    match body {
        MonoType::TypeRef(name) if name == param_name => replacement.clone(),
        MonoType::Fn {
            params,
            return_type,
        } => MonoType::Fn {
            params: params
                .iter()
                .map(|p| substitute_type(p, param_name, replacement))
                .collect(),
            return_type: Box::new(substitute_type(return_type, param_name, replacement)),
        },
        MonoType::List(inner) => {
            MonoType::List(Box::new(substitute_type(inner, param_name, replacement)))
        }
        MonoType::Option(inner) => {
            MonoType::Option(Box::new(substitute_type(inner, param_name, replacement)))
        }
        MonoType::Tuple(elems) => MonoType::Tuple(
            elems
                .iter()
                .map(|e| substitute_type(e, param_name, replacement))
                .collect(),
        ),
        MonoType::Ref { mutable, inner } => MonoType::Ref {
            mutable: *mutable,
            inner: Box::new(substitute_type(inner, param_name, replacement)),
        },
        MonoType::Refined { base, constraint } => MonoType::Refined {
            base: Box::new(substitute_type(base, param_name, replacement)),
            constraint: constraint.clone(), // ConstExpr 暂不替换
        },
        MonoType::DepFn {
            params,
            return_type,
        } => MonoType::DepFn {
            params: params
                .iter()
                .map(|p| crate::frontend::core::types::mono::DepParam {
                    name: p.name.clone(),
                    ty: substitute_type(&p.ty, param_name, replacement),
                })
                .collect(),
            return_type: Box::new(substitute_type(return_type, param_name, replacement)),
        },
        // 叶子类型和不含类型参数的类型直接返回
        _ => body.clone(),
    }
}

// ============ 与类型归一化器集成 ============

/// 类型求值结果转换
impl From<Result<MonoType, EvalError>> for TypeLevelResult<MonoType> {
    fn from(result: Result<MonoType, EvalError>) -> Self {
        match result {
            Ok(ty) => TypeLevelResult::Normalized(ty),
            Err(e) => TypeLevelResult::Error(TypeLevelError::ComputationFailed(format!("{:?}", e))),
        }
    }
}

/// 集成到现有类型归一化器的辅助函数
///
/// 将 Evaluator 与 TypeNormalizer 集成，确保：
/// 1. 求值器的缓存与归一化器的缓存同步
/// 2. 条件类型的求值结果被正确缓存
/// 3. 避免重复求值相同类型
///
/// **设计说明**：当前架构采用"嵌入式集成"模式，
/// TypeNormalizer 内部包含 Evaluator，共享生命周期。
/// 这种设计避免了需要手动同步两个独立缓存的问题。
#[allow(dead_code)]
pub fn integrate_evaluator(
    _evaluator: &mut Evaluator<'_>,
    _normalizer: &mut super::normalizer::TypeNormalizer,
) {
    // TypeNormalizer 现在内部包含 Evaluator
    // 缓存同步由 TypeNormalizer 内部处理
    // 这个函数保留用于未来可能的外部集成需求
}

/// 同步两个缓存系统（备用方法，当前架构不需要）
///
/// 如果未来需要分离 Evaluator 和 TypeNormalizer，
/// 可以使用此函数同步缓存。
#[allow(dead_code)]
pub fn sync_caches(
    evaluator: &Evaluator<'_>,
    context: &mut super::normalizer::NormalizationContext,
) {
    use super::normalizer::NormalForm;

    let cache = context.cache_mut();

    // 将 Evaluator 的缓存同步到 NormalizationContext
    // Result<MonoType, EvalError> -> NormalForm 转换
    for (ty, eval_result) in &evaluator.cache {
        match eval_result {
            Ok(_result_ty) => {
                // 已求值的类型标记为已归一化
                cache.insert(ty.clone(), NormalForm::Normalized);
            }
            Err(_) => {
                // 错误的类型也标记
                cache.insert(ty.clone(), NormalForm::Normalized);
            }
        }
    }
}
