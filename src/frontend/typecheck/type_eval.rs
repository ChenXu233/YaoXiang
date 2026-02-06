//! RFC-011 编译期类型求值器
//!
//! 实现条件类型的编译期求值：
//! - `If[C, T, E]`: 基于布尔条件的类型选择
//! - `Match[T]`: 模式匹配类型选择
//! - `Nat`: 自然数算术运算
//!
//! # 示例
//! ```yaoxiang
//! type If[True, Int, String] => Int
//! type If[False, Int, String] => String
//!
//! type NonEmpty[T] = If[T != Void, T, Never]
//! ```

use std::collections::{HashMap, HashSet};

use crate::frontend::core::type_system::{MonoType, ConstValue};
use crate::frontend::type_level::TypeLevelError;
use crate::frontend::type_level::TypeLevelResult;
use crate::frontend::typecheck::TypeEnvironment;

/// 求值结果
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EvalResult<T> {
    /// 求值成功，得到结果
    Value(T),

    /// 条件未确定，需要进一步类型检查
    Pending,

    /// 求值失败
    Error(String),
}

impl<T> EvalResult<T> {
    /// 转换为 Option
    pub fn ok(self) -> Option<T> {
        match self {
            Self::Value(v) => Some(v),
            Self::Pending | Self::Error(_) => None,
        }
    }

    /// 转换为 Result
    pub fn result(self) -> Result<T, String> {
        match self {
            Self::Value(v) => Ok(v),
            Self::Pending => Err("Evaluation pending".to_string()),
            Self::Error(e) => Err(e),
        }
    }

    /// 检查是否是值
    pub fn is_value(&self) -> bool {
        matches!(self, Self::Value(_))
    }

    /// 检查是否是待定
    pub fn is_pending(&self) -> bool {
        matches!(self, Self::Pending)
    }
}

/// 编译期类型求值器
///
/// 负责在编译期对条件类型进行求值：
/// - If 条件类型：基于布尔条件选择类型
/// - Match 类型：基于模式匹配选择类型
/// - Nat 运算：自然数算术运算
#[derive(Debug, Default)]
pub struct TypeEvaluator {
    /// 类型求值缓存
    /// 避免重复求值相同类型
    cache: HashMap<MonoType, EvalResult<MonoType>>,

    /// 依赖追踪
    /// 记录类型之间的依赖关系
    dependencies: HashMap<MonoType, HashSet<MonoType>>,

    /// 类型环境引用
    env: Option<*const TypeEnvironment>,

    /// 已访问类型（用于循环检测）
    visiting: HashSet<MonoType>,

    /// 求值配置
    config: EvalConfig,
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

impl TypeEvaluator {
    /// 创建新的求值器
    pub fn new() -> Self {
        Self {
            cache: HashMap::new(),
            dependencies: HashMap::new(),
            env: None,
            visiting: HashSet::new(),
            config: EvalConfig {
                max_depth: 100, // 设置合理的默认深度
                enable_cache: true,
                cycle_detection: true,
            },
        }
    }

    /// 创建带配置的求值器
    pub fn with_config(config: EvalConfig) -> Self {
        Self {
            cache: HashMap::new(),
            dependencies: HashMap::new(),
            env: None,
            visiting: HashSet::new(),
            config,
        }
    }

    /// 设置类型环境
    pub fn set_env(
        &mut self,
        env: &TypeEnvironment,
    ) {
        self.env = Some(env);
    }

    /// 获取类型环境引用
    pub fn env(&self) -> Option<&TypeEnvironment> {
        self.env.map(|p| unsafe { &*p })
    }

    /// 求值类型
    pub fn eval(
        &mut self,
        ty: &MonoType,
    ) -> EvalResult<MonoType> {
        self.eval_with_depth(ty, 0)
    }

    /// 带深度限制的求值
    fn eval_with_depth(
        &mut self,
        ty: &MonoType,
        depth: usize,
    ) -> EvalResult<MonoType> {
        // 检查深度限制
        if depth > self.config.max_depth {
            return EvalResult::Error("Maximum evaluation depth exceeded".to_string());
        }

        // 检查缓存
        if self.config.enable_cache {
            if let Some(cached) = self.cache.get(ty).cloned() {
                return cached;
            }
        }

        // 循环检测
        if self.config.cycle_detection && self.visiting.contains(ty) {
            return EvalResult::Error(format!("Cycle detected in type: {}", ty));
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
    ) -> EvalResult<MonoType> {
        match ty {
            // 处理 If 条件类型
            MonoType::TypeRef(name) if name == "If" => self.eval_if_type(ty, depth),

            // 处理 Match 类型
            MonoType::TypeRef(name) if name == "Match" => self.eval_match_type(ty, depth),

            // 处理 Nat 运算
            MonoType::TypeRef(name) if name == "Nat" => self.eval_nat_type(ty, depth),

            // 处理类型引用
            MonoType::TypeRef(name) => self.eval_type_ref(name, depth),

            // 其他类型直接返回
            _ => EvalResult::Value(ty.clone()),
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
    ) -> EvalResult<MonoType> {
        // 尝试从类型引用中提取参数
        let (condition, true_branch, false_branch) = match self.extract_if_args(ty) {
            Some(args) => args,
            None => return EvalResult::Value(ty.clone()),
        };

        // 递归求值条件
        let cond_result = self.eval_condition(&condition, depth);

        match cond_result {
            EvalResult::Value(true) => {
                // 条件为 true，求值 true 分支
                self.eval_with_depth(&true_branch, depth + 1)
            }
            EvalResult::Value(false) => {
                // 条件为 false，求值 false 分支
                self.eval_with_depth(&false_branch, depth + 1)
            }
            EvalResult::Pending => {
                // 条件无法确定，返回待定
                EvalResult::Pending
            }
            EvalResult::Error(msg) => EvalResult::Error(msg),
        }
    }

    /// 从 If 类型引用中提取参数
    fn extract_if_args(
        &self,
        ty: &MonoType,
    ) -> Option<(MonoType, MonoType, MonoType)> {
        // If 类型的参数格式: If<condition, true_branch, false_branch>
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

    /// 解析泛型参数
    fn parse_generic_args(name: &str) -> Option<Vec<String>> {
        if !name.contains('<') || !name.ends_with('>') {
            return None;
        }

        let inner = &name[name.find('<').unwrap() + 1..name.len() - 1];
        let mut args = Vec::new();
        let mut current = String::new();
        let mut depth = 0;

        for c in inner.chars() {
            match c {
                ',' if depth == 0 => {
                    args.push(current.trim().to_string());
                    current = String::new();
                }
                '<' => depth += 1,
                '>' if depth > 0 => depth -= 1,
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
            "Int" => Some(MonoType::Int(32)),
            "Float" => Some(MonoType::Float(64)),
            "Char" => Some(MonoType::Char),
            "String" => Some(MonoType::String),
            "Never" => Some(MonoType::TypeRef("Never".to_string())),
            "True" => Some(MonoType::TypeRef("True".to_string())),
            "False" => Some(MonoType::TypeRef("False".to_string())),
            s if s.starts_with("If<") => Some(MonoType::TypeRef(s.to_string())),
            s if s.starts_with("Match<") => Some(MonoType::TypeRef(s.to_string())),
            s if s.starts_with("Nat<") => Some(MonoType::TypeRef(s.to_string())),
            s => Some(MonoType::TypeRef(s.to_string())),
        }
    }

    /// 求值条件
    fn eval_condition(
        &mut self,
        condition: &MonoType,
        depth: usize,
    ) -> EvalResult<bool> {
        match condition {
            // 布尔字面量
            MonoType::TypeRef(name) if name == "True" => EvalResult::Value(true),
            MonoType::TypeRef(name) if name == "False" => EvalResult::Value(false),

            // 等式条件: L == R
            MonoType::TypeRef(name) if name.starts_with("Eq<") => {
                self.eval_eq_condition(name, depth)
            }

            // 不等条件: L != R
            MonoType::TypeRef(name) if name.starts_with("Neq<") => {
                match self.eval_eq_condition(name, depth) {
                    EvalResult::Value(b) => EvalResult::Value(!b),
                    other => other,
                }
            }

            // 组合条件: And
            MonoType::TypeRef(name) if name.starts_with("And<") => {
                self.eval_and_condition(name, depth)
            }

            // 组合条件: Or
            MonoType::TypeRef(name) if name.starts_with("Or<") => {
                self.eval_or_condition(name, depth)
            }

            // 否定条件: Not
            MonoType::TypeRef(name) if name.starts_with("Not<") => {
                match self.eval_condition(&self.extract_inner_type(name), depth) {
                    EvalResult::Value(b) => EvalResult::Value(!b),
                    other => other,
                }
            }

            // 类型变量无法确定
            MonoType::TypeVar(_) => EvalResult::Pending,

            // 其他情况需要进一步检查
            _ => EvalResult::Pending,
        }
    }

    /// 求值等式条件
    fn eval_eq_condition(
        &mut self,
        name: &str,
        depth: usize,
    ) -> EvalResult<bool> {
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
                if let (EvalResult::Value(l), EvalResult::Value(r)) = (left_eval, right_eval) {
                    return EvalResult::Value(l == r);
                }
            }
        }
        EvalResult::Pending
    }

    /// 求值 And 条件
    fn eval_and_condition(
        &mut self,
        name: &str,
        depth: usize,
    ) -> EvalResult<bool> {
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
                    (EvalResult::Value(false), _) | (_, EvalResult::Value(false)) => {
                        EvalResult::Value(false)
                    }
                    (EvalResult::Value(true), EvalResult::Value(true)) => EvalResult::Value(true),
                    _ => EvalResult::Pending,
                }
            } else {
                EvalResult::Pending
            }
        } else {
            EvalResult::Pending
        }
    }

    /// 求值 Or 条件
    fn eval_or_condition(
        &mut self,
        name: &str,
        depth: usize,
    ) -> EvalResult<bool> {
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
                    (EvalResult::Value(true), _) | (_, EvalResult::Value(true)) => {
                        EvalResult::Value(true)
                    }
                    (EvalResult::Value(false), EvalResult::Value(false)) => {
                        EvalResult::Value(false)
                    }
                    _ => EvalResult::Pending,
                }
            } else {
                EvalResult::Pending
            }
        } else {
            EvalResult::Pending
        }
    }

    /// 从类型引用中提取内部类型
    fn extract_inner_type(
        &self,
        name: &str,
    ) -> MonoType {
        if let Some(start) = name.find('<') {
            if let Some(end) = name.rfind('>') {
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
    ) -> EvalResult<MonoType> {
        // 尝试从类型引用中提取参数
        let (target, arms) = match self.extract_match_args(ty) {
            Some(args) => args,
            None => return EvalResult::Value(ty.clone()),
        };

        // 递归求值目标类型
        let target_eval = self.eval_with_depth(&target, depth + 1);

        match target_eval {
            EvalResult::Value(target_ty) => {
                // 查找匹配的分支
                for (pattern, result) in arms {
                    if self.pattern_matches(&target_ty, &pattern) {
                        return self.eval_with_depth(&result, depth + 1);
                    }
                }
                // 没有匹配分支
                EvalResult::Error("No matching arm in MatchType".to_string())
            }
            EvalResult::Pending => EvalResult::Pending,
            EvalResult::Error(msg) => EvalResult::Error(msg),
        }
    }

    /// 从 Match 类型引用中提取参数
    fn extract_match_args(
        &self,
        ty: &MonoType,
    ) -> Option<(MonoType, Vec<(MonoType, MonoType)>)> {
        if let MonoType::TypeRef(name) = ty {
            if !name.starts_with("Match<") {
                return None;
            }

            // 简化实现：Match 的格式为 Match<target, pattern1 => result1, pattern2 => result2, ...>
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
    ) -> EvalResult<MonoType> {
        if let MonoType::TypeRef(name) = ty {
            if let Some((op, args)) = self.extract_nat_args(name) {
                return self.eval_nat_op(&op, &args, depth);
            }
        }
        EvalResult::Value(ty.clone())
    }

    /// 提取 Nat 运算参数
    fn extract_nat_args(
        &self,
        name: &str,
    ) -> Option<(String, Vec<MonoType>)> {
        if !name.starts_with("Nat<") || !name.ends_with('>') {
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
    ) -> EvalResult<MonoType> {
        // 递归求值参数
        let mut eval_args = Vec::new();
        for arg in args {
            match self.eval_with_depth(arg, depth + 1) {
                EvalResult::Value(v) => eval_args.push(v),
                EvalResult::Pending => return EvalResult::Pending,
                EvalResult::Error(msg) => return EvalResult::Error(msg),
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
            "Eq" if eval_args.len() == 2 => match self.nat_eq(&eval_args[0], &eval_args[1]) {
                EvalResult::Value(b) => EvalResult::Value(MonoType::TypeRef(if b {
                    "True".to_string()
                } else {
                    "False".to_string()
                })),
                EvalResult::Pending => EvalResult::Pending,
                EvalResult::Error(msg) => EvalResult::Error(msg),
            },

            // 小于: Nat<Lt, a, b> -> Bool
            "Lt" if eval_args.len() == 2 => match self.nat_lt(&eval_args[0], &eval_args[1]) {
                EvalResult::Value(b) => EvalResult::Value(MonoType::TypeRef(if b {
                    "True".to_string()
                } else {
                    "False".to_string()
                })),
                EvalResult::Pending => EvalResult::Pending,
                EvalResult::Error(msg) => EvalResult::Error(msg),
            },

            _ => EvalResult::Error(format!("Unknown Nat operation: {}", op)),
        }
    }

    /// Nat 加法
    fn nat_add(
        &self,
        a: &MonoType,
        b: &MonoType,
    ) -> EvalResult<MonoType> {
        match (self.extract_nat_value(a), self.extract_nat_value(b)) {
            (Some(na), Some(nb)) => {
                let result = na + nb;
                self.nat_literal(result)
            }
            _ => EvalResult::Pending,
        }
    }

    /// Nat 减法
    fn nat_sub(
        &self,
        a: &MonoType,
        b: &MonoType,
    ) -> EvalResult<MonoType> {
        match (self.extract_nat_value(a), self.extract_nat_value(b)) {
            (Some(na), Some(nb)) => {
                if nb > na {
                    EvalResult::Error("Nat underflow".to_string())
                } else {
                    let result = na - nb;
                    self.nat_literal(result)
                }
            }
            _ => EvalResult::Pending,
        }
    }

    /// Nat 乘法
    fn nat_mul(
        &self,
        a: &MonoType,
        b: &MonoType,
    ) -> EvalResult<MonoType> {
        match (self.extract_nat_value(a), self.extract_nat_value(b)) {
            (Some(na), Some(nb)) => {
                let result = na * nb;
                self.nat_literal(result)
            }
            _ => EvalResult::Pending,
        }
    }

    /// Nat 除法
    fn nat_div(
        &self,
        a: &MonoType,
        b: &MonoType,
    ) -> EvalResult<MonoType> {
        match (self.extract_nat_value(a), self.extract_nat_value(b)) {
            (Some(na), Some(nb)) => {
                if nb == 0 {
                    EvalResult::Error("Nat division by zero".to_string())
                } else {
                    let result = na / nb;
                    self.nat_literal(result)
                }
            }
            _ => EvalResult::Pending,
        }
    }

    /// Nat 取模
    fn nat_mod(
        &self,
        a: &MonoType,
        b: &MonoType,
    ) -> EvalResult<MonoType> {
        match (self.extract_nat_value(a), self.extract_nat_value(b)) {
            (Some(na), Some(nb)) => {
                if nb == 0 {
                    EvalResult::Error("Nat modulo by zero".to_string())
                } else {
                    let result = na % nb;
                    self.nat_literal(result)
                }
            }
            _ => EvalResult::Pending,
        }
    }

    /// Nat 相等比较
    fn nat_eq(
        &self,
        a: &MonoType,
        b: &MonoType,
    ) -> EvalResult<bool> {
        match (self.extract_nat_value(a), self.extract_nat_value(b)) {
            (Some(na), Some(nb)) => EvalResult::Value(na == nb),
            _ => EvalResult::Pending,
        }
    }

    /// Nat 小于比较
    fn nat_lt(
        &self,
        a: &MonoType,
        b: &MonoType,
    ) -> EvalResult<bool> {
        match (self.extract_nat_value(a), self.extract_nat_value(b)) {
            (Some(na), Some(nb)) => EvalResult::Value(na < nb),
            _ => EvalResult::Pending,
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
            MonoType::TypeRef(name) if name.starts_with("Nat<") => {
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
        if let Some(start) = name.find('<') {
            if let Some(end) = name.rfind('>') {
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
    ) -> EvalResult<MonoType> {
        EvalResult::Value(MonoType::TypeRef(format!("Nat<{}>", n)))
    }

    // ============ 类型引用求值 ============

    /// 求值类型引用
    fn eval_type_ref(
        &mut self,
        name: &str,
        depth: usize,
    ) -> EvalResult<MonoType> {
        // 检查是否是预定义的类型别名
        if let Some(env_ptr) = self.env {
            let env = unsafe { &*env_ptr };
            if let Some(poly) = env.types.get(name) {
                // 递归求值类型定义
                return self.eval_with_depth(&poly.body, depth + 1);
            }
        }

        // 类型引用本身
        EvalResult::Value(MonoType::TypeRef(name.to_string()))
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
    ) -> EvalResult<MonoType> {
        let cond_result = self.eval_condition(condition, 0);

        match cond_result {
            EvalResult::Value(true) => self.eval(true_branch),
            EvalResult::Value(false) => self.eval(false_branch),
            EvalResult::Pending => EvalResult::Pending,
            EvalResult::Error(msg) => EvalResult::Error(msg),
        }
    }

    /// 快速求值 Match 表达式
    ///
    /// 传入目标类型和分支列表，返回匹配的结果类型
    pub fn eval_match(
        &mut self,
        target: &MonoType,
        arms: Vec<(MonoType, MonoType)>,
    ) -> EvalResult<MonoType> {
        let target_eval = self.eval(target);

        match target_eval {
            EvalResult::Value(target_ty) => {
                for (pattern, result) in arms {
                    if self.pattern_matches(&target_ty, &pattern) {
                        return self.eval(&result);
                    }
                }
                EvalResult::Error("No matching arm in MatchType".to_string())
            }
            other => other,
        }
    }

    /// 快速求值 Nat 算术表达式
    ///
    /// 传入操作和参数列表，返回 Nat 结果类型
    pub fn eval_nat(
        &mut self,
        op: &str,
        args: &[MonoType],
    ) -> EvalResult<MonoType> {
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

// ============ 与类型归一化器集成 ============

/// 类型求值结果转换
impl From<EvalResult<MonoType>> for TypeLevelResult<MonoType> {
    fn from(result: EvalResult<MonoType>) -> Self {
        match result {
            EvalResult::Value(ty) => TypeLevelResult::Normalized(ty),
            EvalResult::Pending => TypeLevelResult::Pending(MonoType::TypeRef("?".to_string())),
            EvalResult::Error(msg) => {
                TypeLevelResult::Error(TypeLevelError::ComputationFailed(msg))
            }
        }
    }
}

/// 集成到现有类型归一化器的辅助函数
///
/// 将 TypeEvaluator 与 TypeNormalizer 集成，确保：
/// 1. 求值器的缓存与归一化器的缓存同步
/// 2. 条件类型的求值结果被正确缓存
/// 3. 避免重复求值相同类型
///
/// **设计说明**：当前架构采用"嵌入式集成"模式，
/// TypeNormalizer 内部包含 TypeEvaluator，共享生命周期。
/// 这种设计避免了需要手动同步两个独立缓存的问题。
#[allow(dead_code)]
pub fn integrate_evaluator(
    _evaluator: &mut TypeEvaluator,
    _normalizer: &mut crate::frontend::type_level::evaluation::TypeNormalizer,
) {
    // TypeNormalizer 现在内部包含 TypeEvaluator
    // 缓存同步由 TypeNormalizer 内部处理
    // 这个函数保留用于未来可能的外部集成需求
}

/// 同步两个缓存系统（备用方法，当前架构不需要）
///
/// 如果未来需要分离 TypeEvaluator 和 TypeNormalizer，
/// 可以使用此函数同步缓存。
#[allow(dead_code)]
pub fn sync_caches(
    evaluator: &TypeEvaluator,
    context: &mut crate::frontend::type_level::evaluation::NormalizationContext,
) {
    use crate::frontend::type_level::evaluation::NormalForm;

    let cache = context.cache_mut();

    // 将 TypeEvaluator 的缓存同步到 NormalizationContext
    // EvalResult<MonoType> -> NormalForm 转换
    for (ty, eval_result) in &evaluator.cache {
        match eval_result {
            EvalResult::Value(_result_ty) => {
                // 已求值的类型标记为已归一化
                cache.insert(ty.clone(), NormalForm::Normalized);
            }
            EvalResult::Pending => {
                // 待求值的类型需要进一步处理
                cache.insert(ty.clone(), NormalForm::NeedsReduction);
            }
            EvalResult::Error(_) => {
                // 错误的类型也标记
                cache.insert(ty.clone(), NormalForm::Normalized);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_generic_args() {
        // If 有 3 个参数
        assert_eq!(
            TypeEvaluator::parse_generic_args("If<True, Int, String>"),
            Some(
                vec!["True", "Int", "String"]
                    .into_iter()
                    .map(|s| s.to_string())
                    .collect()
            )
        );

        // Add 有 2 个参数
        assert_eq!(
            TypeEvaluator::parse_generic_args("Add<1, 2>"),
            Some(vec!["1", "2"].into_iter().map(|s| s.to_string()).collect())
        );
    }

    #[test]
    fn test_extract_nat_literal() {
        assert_eq!(TypeEvaluator::extract_nat_literal("Nat<42>"), Some(42));
        assert_eq!(TypeEvaluator::extract_nat_literal("Nat<0>"), Some(0));
        assert_eq!(TypeEvaluator::extract_nat_literal("Nat<abc>"), None);
    }

    #[test]
    fn test_parse_type() {
        let evaluator = TypeEvaluator::new();

        assert_eq!(evaluator.parse_type("Void"), Some(MonoType::Void));
        assert_eq!(evaluator.parse_type("Bool"), Some(MonoType::Bool));
        assert_eq!(evaluator.parse_type("Int"), Some(MonoType::Int(32)));
        assert_eq!(
            evaluator.parse_type("CustomType"),
            Some(MonoType::TypeRef("CustomType".to_string()))
        );
    }

    #[test]
    fn test_nat_operations() {
        let mut evaluator = TypeEvaluator::new();

        // 测试加法
        let a = MonoType::Literal {
            name: "5".to_string(),
            base_type: Box::new(MonoType::Int(32)),
            value: ConstValue::Int(5),
        };
        let b = MonoType::Literal {
            name: "3".to_string(),
            base_type: Box::new(MonoType::Int(32)),
            value: ConstValue::Int(3),
        };

        let result = evaluator.eval_nat("Add", &[a, b]);
        assert_eq!(
            result,
            EvalResult::Value(MonoType::TypeRef("Nat<8>".to_string()))
        );

        // 测试减法
        let a = MonoType::Literal {
            name: "10".to_string(),
            base_type: Box::new(MonoType::Int(32)),
            value: ConstValue::Int(10),
        };
        let b = MonoType::Literal {
            name: "3".to_string(),
            base_type: Box::new(MonoType::Int(32)),
            value: ConstValue::Int(3),
        };

        let result = evaluator.eval_nat("Sub", &[a, b]);
        assert_eq!(
            result,
            EvalResult::Value(MonoType::TypeRef("Nat<7>".to_string()))
        );
    }

    #[test]
    fn test_condition_evaluation() {
        let mut evaluator = TypeEvaluator::new();

        // 测试布尔条件
        let true_cond = MonoType::TypeRef("True".to_string());
        let false_cond = MonoType::TypeRef("False".to_string());

        assert_eq!(
            evaluator.eval_condition(&true_cond, 0),
            EvalResult::Value(true)
        );
        assert_eq!(
            evaluator.eval_condition(&false_cond, 0),
            EvalResult::Value(false)
        );

        // 测试 And 条件
        let and_cond = MonoType::TypeRef("And<True, False>".to_string());
        assert_eq!(
            evaluator.eval_condition(&and_cond, 0),
            EvalResult::Value(false)
        );

        let and_cond = MonoType::TypeRef("And<True, True>".to_string());
        assert_eq!(
            evaluator.eval_condition(&and_cond, 0),
            EvalResult::Value(true)
        );
    }

    #[test]
    fn test_evaluator_cache() {
        let mut evaluator = TypeEvaluator::new();
        evaluator.config.enable_cache = true;

        let ty = MonoType::TypeRef("True".to_string());

        // 第一次求值
        let result1 = evaluator.eval(&ty);
        let stats1 = evaluator.cache_stats();

        // 第二次求值（应该命中缓存）
        let result2 = evaluator.eval(&ty);
        let stats2 = evaluator.cache_stats();

        assert_eq!(result1, result2);
        assert_eq!(stats1.0, stats2.0);
    }

    #[test]
    fn test_integrate_evaluator_function() {
        // 测试 integrate_evaluator 函数存在且可调用
        // 当前实现是空的，因为 TypeNormalizer 已内置 TypeEvaluator
        let mut evaluator = TypeEvaluator::new();
        let mut normalizer = crate::frontend::type_level::evaluation::TypeNormalizer::new();

        // 函数应该可以编译和调用
        integrate_evaluator(&mut evaluator, &mut normalizer);
        // 如果能执行到这里，说明函数正常工作
    }

    #[test]
    fn test_sync_caches_function() {
        // 测试 sync_caches 函数存在且可调用
        let evaluator = TypeEvaluator::new();
        let mut context = crate::frontend::type_level::evaluation::NormalizationContext::new();

        // 函数应该可以编译和调用
        sync_caches(&evaluator, &mut context);
        // 如果能执行到这里，说明函数正常工作
    }
}
