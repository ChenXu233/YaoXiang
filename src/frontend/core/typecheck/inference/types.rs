//! 类型系统工具
//!
//! 提供类型统一、推导等工具函数

use crate::frontend::core::types::{MonoType, TypeConstraintSolver};

/// 类型系统工具
pub struct TypeSystem;

impl TypeSystem {
    /// 统一两个类型
    pub fn unify(
        solver: &mut TypeConstraintSolver,
        ty1: &MonoType,
        ty2: &MonoType,
    ) -> Result<(), String> {
        solver.unify(ty1, ty2).map_err(|e| format!("{:?}", e))
    }

    /// 从可迭代对象类型推导元素类型
    pub fn infer_element_type(
        solver: &mut TypeConstraintSolver,
        iter_ty: &MonoType,
    ) -> MonoType {
        match iter_ty {
            MonoType::List(elem) => *elem.clone(),
            MonoType::String => MonoType::Char,
            MonoType::Dict(key_ty, value_ty) => {
                MonoType::Tuple(vec![*key_ty.clone(), *value_ty.clone()])
            }
            _ => solver.new_var(),
        }
    }

    /// 构造列表类型
    pub fn make_list_type(elem_ty: MonoType) -> MonoType {
        MonoType::List(Box::new(elem_ty))
    }

    /// 检查类型是否可迭代
    pub fn is_iterable(ty: &MonoType) -> bool {
        matches!(
            ty,
            MonoType::List(_) | MonoType::String | MonoType::Dict(_, _) | MonoType::Tuple(_)
        )
    }
}
