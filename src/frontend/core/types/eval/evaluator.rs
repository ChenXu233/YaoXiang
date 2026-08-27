//! RFC-027 编译期求值器
//!
//! 统一的编译期求值引擎，支持：
//! - 类型归约（类型族调用/类型别名展开）
//! - ConstExpr 求值（编译期常量表达式）
//! - β-归约（类型级函数应用）

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
            "String" => Some(MonoType::make_string()),
            "Never" => Some(MonoType::Never),
            "True" => Some(MonoType::TypeRef("True".to_string())),
            "False" => Some(MonoType::TypeRef("False".to_string())),
            s => Some(MonoType::TypeRef(s.to_string())),
        }
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

    // ============ 公共 API ============

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
