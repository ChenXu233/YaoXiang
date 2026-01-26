//! 条件类型系统 (RFC-011 Phase 5)
//!
//! 实现 If 类型、模式匹配和类型级分支计算

use super::{TypeLevelError, TypeLevelValue, TypeLevelComputer};
use crate::frontend::typecheck::types::MonoType;

/// 条件类型
#[derive(Debug, Clone)]
pub enum ConditionalType {
    /// If 类型：If[C, T, E]
    If {
        condition: Box<ConditionalType>,
        then_type: Box<ConditionalType>,
        else_type: Box<ConditionalType>,
    },
    /// 模式匹配类型
    Match {
        expr: Box<ConditionalType>,
        arms: Vec<TypeMatchArm>,
    },
    /// 泛型变量
    Var { name: String, index: usize },
    /// 具体类型
    Concrete { ty: MonoType },
    /// 联合类型
    Union { types: Vec<ConditionalType> },
    /// 交集类型
    Intersection { types: Vec<ConditionalType> },
    /// 类型族应用
    App {
        family: String,
        args: Vec<ConditionalType>,
    },
}

/// 类型匹配分支
#[derive(Debug, Clone)]
pub struct TypeMatchArm {
    /// 匹配的模式
    pub pattern: TypeMatch,
    /// 匹配的类型
    pub ty: ConditionalType,
    /// 是否为默认分支
    pub is_default: bool,
}

/// 类型匹配模式
#[derive(Debug, Clone)]
pub enum TypeMatch {
    /// 变量模式
    Var(String),
    /// 类型模式
    Type(MonoType),
    /// 构造器模式
    Constructor { name: String, args: Vec<TypeMatch> },
    /// 通配符模式
    Wildcard,
    /// 模式列表
    Tuple(Vec<TypeMatch>),
    /// 模式选项
    Or(Vec<TypeMatch>),
}

/// 条件类型求值结果
#[derive(Debug, Clone)]
pub enum EvalResult {
    /// 计算得到的类型
    Type(MonoType),
    /// 计算得到的值（用于条件判断）
    Value(TypeLevelValue),
    /// 未计算完成（需要更多上下文）
    Pending,
    /// 错误
    Error(TypeLevelError),
}

impl ConditionalType {
    /// 创建 If 类型
    pub fn if_type<C, T, E>(
        condition: C,
        then_type: T,
        else_type: E,
    ) -> Self
    where
        C: Into<ConditionalType>,
        T: Into<ConditionalType>,
        E: Into<ConditionalType>,
    {
        ConditionalType::If {
            condition: Box::new(condition.into()),
            then_type: Box::new(then_type.into()),
            else_type: Box::new(else_type.into()),
        }
    }

    /// 创建 Match 类型
    pub fn match_type<E>(
        expr: E,
        arms: Vec<TypeMatchArm>,
    ) -> Self
    where
        E: Into<ConditionalType>,
    {
        ConditionalType::Match {
            expr: Box::new(expr.into()),
            arms,
        }
    }

    /// 创建变量类型
    pub fn var(
        name: &str,
        index: usize,
    ) -> Self {
        ConditionalType::Var {
            name: name.to_string(),
            index,
        }
    }

    /// 创建具体类型
    pub fn concrete(ty: MonoType) -> Self {
        ConditionalType::Concrete { ty }
    }

    /// 创建联合类型
    pub fn union(types: Vec<ConditionalType>) -> Self {
        ConditionalType::Union { types }
    }

    /// 创建交集类型
    pub fn intersection(types: Vec<ConditionalType>) -> Self {
        ConditionalType::Intersection { types }
    }

    /// 创建类型族应用
    pub fn app(
        family: &str,
        args: Vec<ConditionalType>,
    ) -> Self {
        ConditionalType::App {
            family: family.to_string(),
            args,
        }
    }

    /// 评估条件类型
    pub fn evaluate(
        &self,
        computer: &mut TypeLevelComputer,
    ) -> Result<EvalResult, TypeLevelError> {
        match self {
            ConditionalType::Concrete { ty } => Ok(EvalResult::Type(ty.clone())),
            ConditionalType::Var { index, .. } => {
                // 从计算机的变量映射中获取值
                if let Some(value) = computer.var_mapping.get(index) {
                    Ok(EvalResult::Value(value.clone()))
                } else {
                    Err(TypeLevelError::ConditionalTypeError {
                        reason: format!("Unbound conditional type variable {}", index),
                        span: crate::util::span::Span::default(),
                    })
                }
            }
            ConditionalType::If {
                condition,
                then_type,
                else_type,
            } => {
                // 评估条件
                let cond_result = condition.evaluate(computer)?;
                match cond_result {
                    EvalResult::Value(TypeLevelValue::Bool(true)) => then_type.evaluate(computer),
                    EvalResult::Value(TypeLevelValue::Bool(false)) => else_type.evaluate(computer),
                    EvalResult::Value(TypeLevelValue::Nat(n)) => {
                        // 非零自然数视为真
                        if n.is_zero() {
                            else_type.evaluate(computer)
                        } else {
                            then_type.evaluate(computer)
                        }
                    }
                    EvalResult::Value(TypeLevelValue::Type(ty)) => {
                        // 非Void类型视为真
                        if matches!(ty, MonoType::Void) {
                            else_type.evaluate(computer)
                        } else {
                            then_type.evaluate(computer)
                        }
                    }
                    EvalResult::Pending => Ok(EvalResult::Pending),
                    EvalResult::Error(err) => Err(err),
                    EvalResult::Type(_) => Err(TypeLevelError::ConditionalTypeError {
                        reason: "Cannot use Type as boolean condition".to_string(),
                        span: crate::util::span::Span::default(),
                    }),
                }
            }
            ConditionalType::Match { expr, arms } => {
                // 评估表达式
                let expr_result = expr.evaluate(computer)?;
                // 匹配分支
                for arm in arms {
                    if arm.matches(&expr_result, computer)? {
                        return arm.ty.evaluate(computer);
                    }
                }
                Err(TypeLevelError::ConditionalTypeError {
                    reason: "No matching arm in type-level match".to_string(),
                    span: crate::util::span::Span::default(),
                })
            }
            ConditionalType::Union { types } => {
                // Union 简化：如果所有类型相同，返回该类型
                if let Some(first) = types.first() {
                    let first_result = first.evaluate(computer)?;
                    for other in types.iter().skip(1) {
                        let other_result = other.evaluate(computer)?;
                        if !first_result.compatible_with(&other_result) {
                            return Ok(EvalResult::Type(MonoType::Union(
                                types
                                    .iter()
                                    .map(|t| match t.evaluate(computer) {
                                        Ok(EvalResult::Type(ty)) => ty,
                                        _ => MonoType::Void,
                                    })
                                    .collect(),
                            )));
                        }
                    }
                    return Ok(first_result);
                }
                Ok(EvalResult::Pending)
            }
            ConditionalType::Intersection { types } => {
                // Intersection 简化
                if let Some(first) = types.first() {
                    let first_result = first.evaluate(computer)?;
                    for other in types.iter().skip(1) {
                        let other_result = other.evaluate(computer)?;
                        if !first_result.compatible_with(&other_result) {
                            return Ok(EvalResult::Type(MonoType::Intersection(
                                types
                                    .iter()
                                    .map(|t| match t.evaluate(computer) {
                                        Ok(EvalResult::Type(ty)) => ty,
                                        _ => MonoType::Void,
                                    })
                                    .collect(),
                            )));
                        }
                    }
                    return Ok(first_result);
                }
                Ok(EvalResult::Pending)
            }
            ConditionalType::App { family, args } => {
                // 处理预定义类型族
                match family.as_str() {
                    "Option" => {
                        if let Some(arg) = args.first() {
                            let arg_result = arg.evaluate(computer)?;
                            if let EvalResult::Type(_ty) = arg_result {
                                // Option[T] = Some(T) | None
                                return Ok(EvalResult::Type(MonoType::TypeRef(
                                    "Option".to_string(),
                                )));
                            }
                        }
                    }
                    "List" => {
                        if let Some(arg) = args.first() {
                            let arg_result = arg.evaluate(computer)?;
                            if let EvalResult::Type(_ty) = arg_result {
                                // List[T] = Nil | Cons(T, List[T])
                                return Ok(EvalResult::Type(MonoType::TypeRef("List".to_string())));
                            }
                        }
                    }
                    "Vector" => {
                        if args.len() == 2 {
                            let ty_result = args[0].evaluate(computer)?;
                            let len_result = args[1].evaluate(computer)?;
                            if let (
                                EvalResult::Type(_ty),
                                EvalResult::Value(TypeLevelValue::Nat(_n)),
                            ) = (ty_result, len_result)
                            {
                                // Vector[T, N] 根据长度构造
                                return Ok(EvalResult::Type(MonoType::TypeRef(
                                    "Vector".to_string(),
                                )));
                            }
                        }
                    }
                    _ => {}
                }
                Ok(EvalResult::Pending)
            }
        }
    }

    /// 转换到具体类型
    pub fn to_mono_type(
        &self,
        computer: &mut TypeLevelComputer,
    ) -> Result<MonoType, TypeLevelError> {
        match self.evaluate(computer)? {
            EvalResult::Type(ty) => Ok(ty),
            EvalResult::Value(value) => Err(TypeLevelError::ConditionalTypeError {
                reason: format!("Expected Type but got {:?}", value),
                span: crate::util::span::Span::default(),
            }),
            EvalResult::Pending => Err(TypeLevelError::ConditionalTypeError {
                reason: "Cannot evaluate conditional type to a concrete type".to_string(),
                span: crate::util::span::Span::default(),
            }),
            EvalResult::Error(err) => Err(err),
        }
    }
}

impl TypeMatchArm {
    /// 创建新分支
    pub fn new(
        pattern: TypeMatch,
        ty: ConditionalType,
        is_default: bool,
    ) -> Self {
        TypeMatchArm {
            pattern,
            ty,
            is_default,
        }
    }

    /// 检查是否匹配
    pub fn matches(
        &self,
        result: &EvalResult,
        computer: &mut TypeLevelComputer,
    ) -> Result<bool, TypeLevelError> {
        self.pattern.matches(result, computer)
    }
}

impl TypeMatch {
    /// 创建变量模式
    pub fn var(name: &str) -> Self {
        TypeMatch::Var(name.to_string())
    }

    /// 创建类型模式
    pub fn ty(ty: MonoType) -> Self {
        TypeMatch::Type(ty)
    }

    /// 创建构造器模式
    pub fn constructor(
        name: &str,
        args: Vec<TypeMatch>,
    ) -> Self {
        TypeMatch::Constructor {
            name: name.to_string(),
            args,
        }
    }

    /// 创建通配符模式
    pub fn wildcard() -> Self {
        TypeMatch::Wildcard
    }

    /// 创建元组模式
    pub fn tuple(args: Vec<TypeMatch>) -> Self {
        TypeMatch::Tuple(args)
    }

    /// 创建或模式
    pub fn or(patterns: Vec<TypeMatch>) -> Self {
        TypeMatch::Or(patterns)
    }

    /// 检查是否匹配
    pub fn matches(
        &self,
        result: &EvalResult,
        _computer: &mut TypeLevelComputer,
    ) -> Result<bool, TypeLevelError> {
        match (self, result) {
            (TypeMatch::Wildcard, _) => Ok(true),
            (TypeMatch::Var(_), _) => Ok(true),
            (TypeMatch::Type(pat_ty), EvalResult::Type(res_ty)) => {
                Ok(pat_ty == res_ty || matches!(pat_ty, MonoType::Void))
            }
            (TypeMatch::Type(pat_ty), EvalResult::Value(TypeLevelValue::Type(res_ty))) => {
                Ok(pat_ty == res_ty || matches!(pat_ty, MonoType::Void))
            }
            (
                TypeMatch::Constructor { name: pat_name, .. },
                EvalResult::Type(MonoType::TypeRef(res_name)),
            ) => Ok(pat_name == res_name),
            (
                TypeMatch::Constructor {
                    name: pat_name,
                    args: _,
                },
                EvalResult::Value(TypeLevelValue::Type(MonoType::TypeRef(res_name))),
            ) => Ok(pat_name == res_name),
            (TypeMatch::Or(patterns), result) => {
                for pattern in patterns {
                    if pattern.matches(result, _computer)? {
                        return Ok(true);
                    }
                }
                Ok(false)
            }
            _ => Ok(false),
        }
    }
}

impl EvalResult {
    /// 检查兼容性
    fn compatible_with(
        &self,
        other: &Self,
    ) -> bool {
        match (self, other) {
            (EvalResult::Type(t1), EvalResult::Type(t2)) => t1 == t2,
            (EvalResult::Value(v1), EvalResult::Value(v2)) => v1 == v2,
            _ => false,
        }
    }
}

impl From<MonoType> for ConditionalType {
    fn from(ty: MonoType) -> Self {
        ConditionalType::Concrete { ty }
    }
}

/// 条件类型检查器
#[derive(Debug)]
pub struct ConditionalTypeChecker {
    computer: TypeLevelComputer,
}

impl ConditionalTypeChecker {
    /// 创建新的条件类型检查器
    pub fn new() -> Self {
        ConditionalTypeChecker {
            computer: TypeLevelComputer::new(),
        }
    }

    /// 检查条件类型
    pub fn check(
        &mut self,
        cond_type: &ConditionalType,
    ) -> Result<MonoType, TypeLevelError> {
        cond_type.to_mono_type(&mut self.computer)
    }

    /// 绑定变量
    pub fn bind(
        &mut self,
        index: usize,
        value: TypeLevelValue,
    ) {
        self.computer.bind_var(index, value);
    }

    /// 取消绑定变量
    pub fn unbind(
        &mut self,
        index: usize,
    ) {
        self.computer.unbind_var(index);
    }

    /// 清空缓存
    pub fn clear_cache(&mut self) {
        self.computer.clear_cache();
    }

    /// 获取计算器
    pub fn computer(&self) -> &TypeLevelComputer {
        &self.computer
    }
}

impl Default for ConditionalTypeChecker {
    fn default() -> Self {
        ConditionalTypeChecker::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::frontend::typecheck::types::MonoType;

    #[test]
    fn test_conditional_type_if_true() {
        let mut checker = ConditionalTypeChecker::new();
        checker.bind(0, TypeLevelValue::Bool(true));

        let cond_type = ConditionalType::if_type(
            ConditionalType::var("x", 0),
            ConditionalType::concrete(MonoType::Int(64)),
            ConditionalType::concrete(MonoType::String),
        );

        let result = checker.check(&cond_type).unwrap();
        assert_eq!(result, MonoType::Int(64));
    }

    #[test]
    fn test_conditional_type_if_false() {
        let mut checker = ConditionalTypeChecker::new();
        checker.bind(0, TypeLevelValue::Bool(false));

        let cond_type = ConditionalType::if_type(
            ConditionalType::var("x", 0),
            ConditionalType::concrete(MonoType::Int(64)),
            ConditionalType::concrete(MonoType::String),
        );

        let result = checker.check(&cond_type).unwrap();
        assert_eq!(result, MonoType::String);
    }

    #[test]
    fn test_conditional_type_match() {
        let mut checker = ConditionalTypeChecker::new();
        checker.bind(0, TypeLevelValue::Type(MonoType::Int(64)));

        let arms = vec![
            TypeMatchArm::new(
                TypeMatch::ty(MonoType::Int(64)),
                ConditionalType::concrete(MonoType::Bool),
                false,
            ),
            TypeMatchArm::new(
                TypeMatch::wildcard(),
                ConditionalType::concrete(MonoType::Void),
                true,
            ),
        ];

        let cond_type = ConditionalType::match_type(ConditionalType::var("x", 0), arms);

        let result = checker.check(&cond_type).unwrap();
        assert_eq!(result, MonoType::Bool);
    }

    #[test]
    fn test_type_match_pattern() {
        let pattern = TypeMatch::ty(MonoType::Int(64));
        let result = EvalResult::Type(MonoType::Int(64));
        let mut checker = ConditionalTypeChecker::new();

        assert!(pattern.matches(&result, &mut checker.computer).unwrap());
    }

    #[test]
    fn test_or_pattern() {
        let pattern = TypeMatch::or(vec![
            TypeMatch::ty(MonoType::Int(32)),
            TypeMatch::ty(MonoType::Int(64)),
        ]);

        let result1 = EvalResult::Type(MonoType::Int(32));
        let result2 = EvalResult::Type(MonoType::Int(64));
        let result3 = EvalResult::Type(MonoType::String);

        let mut checker = ConditionalTypeChecker::new();

        assert!(pattern.matches(&result1, &mut checker.computer).unwrap());
        assert!(pattern.matches(&result2, &mut checker.computer).unwrap());
        assert!(!pattern.matches(&result3, &mut checker.computer).unwrap());
    }
}
