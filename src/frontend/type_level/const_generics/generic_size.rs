//! RFC-011 泛型尺寸计算
//!
//! 实现泛型类型的尺寸计算，用于Const泛型和数组类型。

use crate::frontend::core::type_system::MonoType;

/// 泛型尺寸计算器
///
/// 计算泛型类型的编译期尺寸
#[derive(Debug, Clone, Default)]
pub struct GenericSize {
    /// 基础类型大小（字节）
    base_sizes: std::collections::HashMap<&'static str, usize>,
}

impl GenericSize {
    /// 创建新的尺寸计算器
    pub fn new() -> Self {
        let mut base_sizes = std::collections::HashMap::new();
        base_sizes.insert("Bool", 1);
        base_sizes.insert("Int", 8);
        base_sizes.insert("Float", 8);
        base_sizes.insert("String", 8); // 指针
        base_sizes.insert("Void", 0);

        Self { base_sizes }
    }

    /// 计算类型的尺寸
    pub fn size_of(
        &self,
        ty: &MonoType,
    ) -> Result<usize, String> {
        match ty {
            MonoType::Bool => self
                .base_sizes
                .get("Bool")
                .cloned()
                .ok_or("Bool not found".to_string()),
            MonoType::Int(_) => self
                .base_sizes
                .get("Int")
                .cloned()
                .ok_or("Int not found".to_string()),
            MonoType::Float(_) => self
                .base_sizes
                .get("Float")
                .cloned()
                .ok_or("Float not found".to_string()),
            MonoType::String => self
                .base_sizes
                .get("String")
                .cloned()
                .ok_or("String not found".to_string()),
            MonoType::Void => self
                .base_sizes
                .get("Void")
                .cloned()
                .ok_or("Void not found".to_string()),
            MonoType::TypeRef(name) => {
                // 检查是否是 Array<T, N> 类型
                if let Some((elem_type, count)) = self.parse_array_type(name) {
                    return self.size_of_array(elem_type.as_ref(), count);
                }
                // 对于类型引用，尝试查找基础大小
                self.base_sizes
                    .get(name.as_str())
                    .cloned()
                    .ok_or_else(|| format!("TypeRef {} not found", name))
            }
            MonoType::Tuple(types) => {
                let mut total = 0;
                for ty in types {
                    total += self.size_of(ty)?;
                }
                Ok(total)
            }
            MonoType::List(_elem_type) => {
                // List<T> 大小未知（动态大小），返回错误
                Err("Cannot compute size of dynamic List type".to_string())
            }
            MonoType::TypeVar(_) => Err("Cannot compute size of type variable".to_string()),
            MonoType::Fn { .. } => Ok(8), // 指针
            _ => Err(format!("Unknown type: {:?}", ty)),
        }
    }

    /// 解析 Array<T, N> 类型的元素类型和数量
    fn parse_array_type(
        &self,
        type_name: &str,
    ) -> Option<(Box<MonoType>, usize)> {
        if !type_name.starts_with("Array<") {
            return None;
        }

        // 提取泛型参数部分
        let args_str = &type_name["Array<".len()..type_name.len().saturating_sub(1)];

        // 分割参数，找到元素类型和数量
        let mut args = Vec::new();
        let mut current = String::new();
        let mut depth = 0;

        for c in args_str.chars() {
            match c {
                ',' if depth == 0 => {
                    if !current.trim().is_empty() {
                        args.push(current.trim().to_string());
                    }
                    current = String::new();
                }
                '<' => {
                    depth += 1;
                    current.push(c);
                }
                '>' => {
                    if depth == 0 {
                        break;
                    }
                    depth -= 1;
                    current.push(c);
                }
                _ => current.push(c),
            }
        }

        if !current.trim().is_empty() {
            args.push(current.trim().to_string());
        }

        if args.len() < 2 {
            return None;
        }

        // 解析元素类型
        let elem_type = Box::new(MonoType::TypeRef(args[0].clone()));

        // 解析数组长度（尝试解析为整数）
        let count = args[1].parse::<usize>().ok()?;

        Some((elem_type, count))
    }

    /// 计算数组的尺寸
    fn size_of_array(
        &self,
        elem_type: &MonoType,
        count: usize,
    ) -> Result<usize, String> {
        let elem_size = self.size_of(elem_type)?;
        Ok(elem_size.saturating_mul(count))
    }
}

/// 尺寸表达式
///
/// 用于表示类型尺寸的表达式
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum SizeExpr {
    /// 常量
    Const(usize),

    /// 乘法
    Mul(Box<SizeExpr>, Box<SizeExpr>),

    /// 加法
    Add(Box<SizeExpr>, Box<SizeExpr>),
}

impl SizeExpr {
    /// 计算表达式
    pub fn eval(&self) -> Result<SizeResult, String> {
        match self {
            SizeExpr::Const(n) => Ok(SizeResult::new(*n, true)),
            SizeExpr::Mul(a, b) => {
                let a_result = a.eval()?;
                let b_result = b.eval()?;
                Ok(SizeResult::new(
                    a_result.size.saturating_mul(b_result.size),
                    a_result.is_const && b_result.is_const,
                ))
            }
            SizeExpr::Add(a, b) => {
                let a_result = a.eval()?;
                let b_result = b.eval()?;
                Ok(SizeResult::new(
                    a_result.size.saturating_add(b_result.size),
                    a_result.is_const && b_result.is_const,
                ))
            }
        }
    }
}

/// 泛型尺寸计算结果
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SizeResult {
    /// 尺寸值
    pub size: usize,

    /// 是否是常量
    pub is_const: bool,
}

impl SizeResult {
    /// 创建成功结果
    pub fn new(
        size: usize,
        is_const: bool,
    ) -> Self {
        Self { size, is_const }
    }
}

/// 预定义的尺寸计算
pub mod predefined {
    use super::*;

    /// 计算类型数组的尺寸
    pub fn array_type_size(
        elem_type: MonoType,
        count: usize,
    ) -> Result<SizeResult, String> {
        let elem_size = GenericSize::new().size_of(&elem_type)?;
        Ok(SizeResult::new(elem_size * count, true))
    }

    /// 计算元组的尺寸
    pub fn tuple_size(types: &[MonoType]) -> Result<SizeResult, String> {
        let mut total = 0;
        let mut all_const = true;

        for ty in types {
            let size = GenericSize::new().size_of(ty)?;
            total += size;
            if matches!(ty, MonoType::TypeVar(_)) {
                all_const = false;
            }
        }

        Ok(SizeResult::new(total, all_const))
    }
}
