//! 调用裁决族 · 方法调用绑定键与接收者静态类型。
//!
//! 自 `inference/expressions.rs` 纯搬移（WBS 7.0.4）：方法体与文档注释零改动，
//! 仅补跨模块可见性（`pub(super)`）。

use crate::frontend::core::types::MonoType;

use super::super::ExpressionInferrer;
use super::super::extract_namespace_path;

impl<'a> ExpressionInferrer<'a> {
    /// #317：FieldAccess 调用目标是否解析自 method_bindings（impl 方法绑定形态）
    ///
    /// 返回 Some(方法键) 时，签名 params[0] 是接收者占位，实参对应 params[1..]。
    /// 判定链与字段访问推断臂同序：native 命名空间形态（assert.assert 等，
    /// receiver-less）与结构体同名函数字段（记录形态，d.draw）均返回 None；
    /// 动态分发/链式访问（obj 非局部变量）不在本检查范围，返回 None。
    pub(super) fn method_binding_call_key(
        &self,
        func: &crate::ast::Expr,
    ) -> Option<String> {
        let crate::ast::Expr::FieldAccess {
            expr: obj, field, ..
        } = func
        else {
            return None;
        };
        let crate::ast::Expr::Var(obj_name, _) = &**obj else {
            return None;
        };

        // native 命名空间形态优先（同推断臂顺序）：命中即非方法绑定
        if let Some(ns_path) = extract_namespace_path(obj) {
            let full_path = format!("{}.{}", ns_path, field);
            if self.native_signatures.contains_key(&full_path)
                || self
                    .native_signatures
                    .keys()
                    .any(|k| k.starts_with(&full_path))
            {
                return None;
            }
        }

        // clippy manual_let_else：Option 上下文等价改写为 ?（CI Check 修复）
        let poly = self.scope.get_var(obj_name)?;
        let mut resolved = self.solver.resolve_type(&poly.body);
        while let MonoType::Ref { inner, .. } = resolved {
            resolved = *inner;
        }
        let resolved = self.solver.resolve_type(&resolved);

        // 结构字段优先（同推断臂顺序）：同名函数字段是记录形态调用，非方法绑定
        let (type_name, has_field) = match &resolved {
            MonoType::Struct(s) => (s.name.clone(), s.fields.iter().any(|f| &f.name == field)),
            MonoType::TypeRef(n) => {
                if let Some(def_ty) = self.type_defs.get(n).map(|p| &p.body) {
                    let def_ty = self.solver.resolve_type(def_ty);
                    if let MonoType::Struct(s) = def_ty {
                        (s.name.clone(), s.fields.iter().any(|fi| &fi.name == field))
                    } else {
                        (n.clone(), false)
                    }
                } else {
                    (n.clone(), false)
                }
            }
            MonoType::Generic { name, .. } => (name.clone(), false),
            _ => return None,
        };
        if has_field {
            return None;
        }

        let key = format!("{}.{}", type_name, field);
        let binding = self.method_bindings.get(&key)?;
        let MonoType::Fn { params, .. } = binding else {
            return None;
        };
        // 接收者校验：params[0] 解包 Ref 后的类型名须等于接收者类型名或 "Self"，
        // 确保首参确是接收者占位（auto_bind/显式 Type.method 两条注册路径的约定），
        // 而非恰好首参为 TypeRef 的普通函数
        let recv_ty = params.first()?;
        let mut recv = self.solver.resolve_type(recv_ty);
        while let MonoType::Ref { inner, .. } = recv {
            recv = *inner;
        }
        let recv_name = match self.solver.resolve_type(&recv) {
            MonoType::Struct(s) => s.name.clone(),
            MonoType::TypeRef(n) => n.clone(),
            MonoType::Generic { name, .. } => name.clone(),
            _ => return None,
        };
        if recv_name != type_name && recv_name != "Self" {
            return None;
        }
        Some(key)
    }

    /// 调用接收者的**静态类型**（无副作用）：只覆盖变量接收者。
    ///
    /// 5A 的方法接收者判定只需要「接收者是不是 witness-bearing 记录」；为此
    /// 重新推断接收者会产生引用登记/实例化请求等副作用，故这里只做 scope
    /// 查表（拿不到就返回 None——宁漏检不误报，嵌套接收者留待 B.2 的探针）。
    pub(super) fn call_receiver_static_type(
        &self,
        func: &crate::ast::Expr,
    ) -> Option<MonoType> {
        let crate::ast::Expr::FieldAccess { expr: receiver, .. } = func else {
            return None;
        };
        let crate::ast::Expr::Var(name, _) = receiver.as_ref() else {
            return None;
        };
        self.get_var(name).map(|poly| poly.body.clone())
    }
}
