#![allow(clippy::result_large_err)]

//! 子类型检查
//!
//! 实现子类型关系的检查

use crate::util::diagnostic::Result;
use crate::frontend::core::type_system::MonoType;

/// 子类型检查器
pub struct SubtypeChecker;

impl Default for SubtypeChecker {
    fn default() -> Self {
        Self::new()
    }
}

impl SubtypeChecker {
    /// 创建新的子类型检查器
    pub fn new() -> Self {
        Self
    }

    /// 检查子类型关系
    pub fn check_subtype(
        &self,
        sub: &MonoType,
        sup: &MonoType,
    ) -> Result<bool> {
        if self.is_subtype(sub, sup) {
            Ok(true)
        } else {
            Ok(false)
        }
    }

    /// 检查是否为有效子类型
    #[allow(clippy::only_used_in_recursion)]
    pub fn is_subtype(
        &self,
        sub: &MonoType,
        sup: &MonoType,
    ) -> bool {
        match (sub, sup) {
            // 相同类型是子类型
            (a, b) if a == b => true,

            // Int可以强制转换为Float
            (MonoType::Int(_), MonoType::Float(_)) => true,

            // 子类型是协变的
            (MonoType::List(a), MonoType::List(b)) => self.is_subtype(a, b),

            // 函数是逆变的
            (
                MonoType::Fn {
                    params: a_params,
                    return_type: a_ret,
                    is_async: a_async,
                },
                MonoType::Fn {
                    params: b_params,
                    return_type: b_ret,
                    is_async: b_async,
                },
            ) => {
                // 异步属性必须匹配
                if a_async != b_async {
                    return false;
                }

                // 参数是逆变的：b_param是a_param的子类型
                let params_ok = a_params.len() == b_params.len()
                    && a_params
                        .iter()
                        .zip(b_params.iter())
                        .all(|(a, b)| self.is_subtype(b, a));

                // 返回值是协变的：a_ret是b_ret的子类型
                let ret_ok = self.is_subtype(a_ret, b_ret);

                params_ok && ret_ok
            }

            // 结构体字段协变
            (MonoType::Struct(a), MonoType::Struct(b)) => {
                if a.name != b.name || a.fields.len() != b.fields.len() {
                    return false;
                }
                a.fields
                    .iter()
                    .zip(b.fields.iter())
                    .all(|(a_field, b_field)| {
                        a_field.0 == b_field.0 && self.is_subtype(&a_field.1, &b_field.1)
                    })
            }

            // 其他情况根据类型提升规则
            _ => false,
        }
    }
}
