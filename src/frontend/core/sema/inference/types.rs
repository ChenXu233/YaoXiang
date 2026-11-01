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
            m if m.is_list() => m.generic_args().unwrap()[0].clone(),
            m if m.is_string() => MonoType::Char,
            m if m.is_dict() => {
                let args = m.generic_args().unwrap();
                MonoType::make_tuple(vec![args[0].clone(), args[1].clone()])
            }
            _ => solver.new_var(),
        }
    }

    /// 构造列表类型
    pub fn make_list_type(elem_ty: MonoType) -> MonoType {
        MonoType::make_list(elem_ty)
    }

    /// 检查类型是否可迭代
    pub fn is_iterable(ty: &MonoType) -> bool {
        ty.is_list() || ty.is_string() || ty.is_dict() || ty.is_tuple()
    }
}
