#![allow(clippy::result_large_err)]

//! 子类型检查
//!
//! 实现子类型关系的检查
//!
//! 支持约束类型的结构化子类型检查：
//! 当目标类型是约束（接口）类型时，检查源类型是否满足约束的所有方法要求

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

            // 结构体字段协变（包括约束类型的结构化子类型检查）
            (MonoType::Struct(a), MonoType::Struct(b)) => {
                // 如果目标是约束类型（接口），执行结构化匹配
                if sup.is_constraint() {
                    return self.satisfies_constraint(sub, sup);
                }

                // 普通结构体：名称和字段必须匹配
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

            // 非结构体类型赋值给约束类型也可以尝试检查
            (_, MonoType::Struct(_)) if sup.is_constraint() => self.satisfies_constraint(sub, sup),

            // 其他情况根据类型提升规则
            _ => false,
        }
    }

    /// 检查具体类型是否满足约束类型（接口）的所有方法要求
    ///
    /// 实现结构化子类型（鸭子类型）规则：
    /// - 约束类型的每个函数字段都必须在具体类型中存在
    /// - 函数签名必须兼容（参数逆变，返回值协变）
    fn satisfies_constraint(
        &self,
        sub: &MonoType,
        constraint: &MonoType,
    ) -> bool {
        let constraint_fields = constraint.constraint_fields();

        // 空约束：任何类型都满足
        if constraint_fields.is_empty() {
            return true;
        }

        // 获取子类型的函数字段
        let sub_fn_fields: Vec<(String, &MonoType)> = match sub {
            MonoType::Struct(s) => s
                .fields
                .iter()
                .filter(|(_, ty)| matches!(ty, MonoType::Fn { .. }))
                .map(|(name, ty)| (name.clone(), ty))
                .collect(),
            _ => return false,
        };

        // 检查每个约束字段是否在子类型中存在且签名兼容
        for (field_name, constraint_fn) in &constraint_fields {
            let found = sub_fn_fields.iter().find(|(name, _)| name == field_name);

            match found {
                Some((_, found_fn)) => {
                    // 检查函数签名兼容性
                    if !self.fn_signature_compatible(found_fn, constraint_fn) {
                        return false;
                    }
                }
                None => return false,
            }
        }

        true
    }

    /// 检查两个函数签名是否兼容（用于约束满足检查）
    ///
    /// 约束签名通常不包含 self 参数，类型签名可能包含 self 作为第一个参数
    fn fn_signature_compatible(
        &self,
        found_fn: &MonoType,
        constraint_fn: &MonoType,
    ) -> bool {
        match (found_fn, constraint_fn) {
            (
                MonoType::Fn {
                    params: found_params,
                    return_type: found_return,
                    ..
                },
                MonoType::Fn {
                    params: constraint_params,
                    return_type: constraint_return,
                    ..
                },
            ) => {
                // 返回类型必须是协变的
                if !self.is_subtype(found_return, constraint_return) {
                    return false;
                }

                // 参数数量匹配：直接匹配或多一个 self 参数
                if found_params.len() == constraint_params.len() {
                    // 直接比较参数（逆变）
                    found_params
                        .iter()
                        .zip(constraint_params.iter())
                        .all(|(f, c)| self.is_subtype(c, f))
                } else if found_params.len() == constraint_params.len() + 1 {
                    // 跳过第一个参数（self），比较其余
                    found_params[1..]
                        .iter()
                        .zip(constraint_params.iter())
                        .all(|(f, c)| self.is_subtype(c, f))
                } else {
                    false
                }
            }
            _ => false,
        }
    }
}
