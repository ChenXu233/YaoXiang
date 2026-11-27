//! 纯查询族（`AstToIrGenerator` 的只读判定与提取）
//!
//! 共同形态：`&self` 或关联函数，只读生成器状态 / AST / `MonoType`，不写槽位、
//! 不发 IR。方法体逐字搬自原 `middle/ir/ir_gen.rs`（WBS 7.0.1），语义未改。

use super::AstToIrGenerator;
use crate::ast::{self, Expr};
use crate::frontend::core::sema::{MonoType, PolyType};
use crate::frontend::module::registry::QualifiedNameConcern;
use crate::frontend::module::resolver::Resolver;
use crate::middle::ir::ir::{ConstValue, Operand};
use crate::util::diagnostic::{Diagnostic, ErrorCodeDefinition};
use crate::util::span::Span;

/// 把命名空间表达式拆成「头 + 字段段列表」，供 [`Resolver`] 解析。
///
/// `std.io.println`（FieldAccess 链）→ `("std", ["io", "println"])`；
/// 非 Var 结尾的表达式（如方法调用结果再取字段）返回 None。
/// 纯 AST 机械操作；「头是不是命名空间」「拼成什么限定名」的语义归 [`Resolver`]。
fn flatten_namespace(expr: &ast::Expr) -> Option<(&str, Vec<&str>)> {
    let mut segs: Vec<&str> = Vec::new();
    let mut cur = expr;
    loop {
        match cur {
            ast::Expr::Var(name, _) => {
                segs.push(name.as_str());
                segs.reverse();
                let head = segs[0];
                return Some((head, segs[1..].to_vec()));
            }
            ast::Expr::FieldAccess { expr, field, .. } => {
                segs.push(field.as_str());
                cur = expr;
            }
            _ => return None,
        }
    }
}

impl AstToIrGenerator {
    /// 当前文件的模块限定键（单文件模式默认 `main`，与 TypeChecker 一致）。
    fn module_key_or_main(&self) -> &str {
        self.module_key.as_deref().unwrap_or("main")
    }

    /// 构造当前文件上下文的名字解析器（名字 → 限定名 → DefId 的唯一所有者）。
    pub(in crate::middle) fn resolver(&self) -> Resolver<'_> {
        Resolver::new(
            self.registry.symbols(),
            &self.registry,
            self.module_key_or_main(),
            &self.user_namespaces,
        )
    }

    /// `recv` 是否是命名空间调用的接收者（头为 std / std 子模块 / 用户模块别名）。
    /// 机械扁平化后委托 [`Resolver::is_namespace`]——语义归 Resolver。
    pub(in crate::middle) fn is_namespace_receiver(
        &self,
        recv: &ast::Expr,
    ) -> bool {
        flatten_namespace(recv)
            .map(|(head, _)| self.resolver().is_namespace(head))
            .unwrap_or(false)
    }

    /// 解析 `recv.field` 的限定名（如 `std.io.println`），经 [`Resolver`] 统一解析。
    /// 非 Var 结尾的接收者退回 `field` 本身（与既有行为一致）。
    pub(in crate::middle) fn resolve_field_path(
        &self,
        recv: &ast::Expr,
        field: &str,
    ) -> String {
        match flatten_namespace(recv) {
            Some((head, mut fields)) => {
                fields.push(field);
                self.resolver().resolve_namespace(head, &fields)
            }
            None => field.to_string(),
        }
    }

    /// #396：表达式位置命名空间数据访问的判定——接收者链（Var/FieldAccess 链）
    /// 扁平化后头为命名空间，且头未被本文件局部绑定/闭包捕获遮蔽时，返回全链
    /// 限定名（`lib.v` → `lib.v`，`io.println` → `std.io.println`）。
    /// 语义归 [`Resolver`]（is_namespace / resolve_namespace），与调用位置同判据。
    pub(in crate::middle) fn resolve_namespace_data_access(
        &self,
        expr: &ast::Expr,
        field: &str,
    ) -> Option<String> {
        let (head, fields) = flatten_namespace(expr)?;
        if self.lookup_upvalue(head).is_some() || self.lookup_local(head).is_some() {
            return None;
        }
        if !self.resolver().is_namespace(head) {
            return None;
        }
        let mut segs = fields;
        segs.push(field);
        Some(self.resolver().resolve_namespace(head, &segs))
    }

    /// 从限定/混编调用名取叶子段（形参名表的键就是声明处的短名）。
    ///
    /// `Temp.pick` / `Temp.pick#1` → `pick`；无分隔符时原样返回。
    pub(in crate::middle) fn call_name_leaf(name: &str) -> &str {
        name.rsplit('.')
            .next()
            .and_then(|leaf| leaf.split('#').next())
            .unwrap_or(name)
    }

    /// 共享判据给出的「逐实参形参位」→ **按形参槽位序**重排的实参下标。
    ///
    /// #452 / #456：实参绑定的**判定**只有一处——`sema::inference::arg_binding::
    /// `bind_args_to_params`（位置占前导槽、命名按名入槽、未知名/争槽/命名后接位置/
    /// 重复命名/缺参六条规则全在那边）。本函数**只搬运**：把「逐实参的形参位」
    /// 转成 IR 侧要的「按槽位序的实参序」，不判位、不判名、不判可选位。
    ///
    /// 此前 ir_gen 另有一份 `build_arg_slots`（约 60 行）在同一条生产路径上重判，
    /// 与共享判据已经分歧（同一语义两个码：用户函数重复命名 E1015、方法/原生路径
    /// E1010/E1111）——本次合并后全仓只剩共享判据一处（#456）。
    ///
    /// 槽位不重复由判据保证（同槽冲突已在那侧拒绝），故排序无并列。
    pub(in crate::middle) fn slot_order_arg_indices(positions: &[usize]) -> Vec<usize> {
        let mut order: Vec<usize> = (0..positions.len()).collect();
        order.sort_by_key(|&arg_idx| positions[arg_idx]);
        order
    }

    /// 查一个函数的**声明期形参名 + 必需下限**（供命名参数重排与「可选位放行」）。
    ///
    /// 两个来源，按就近/精确优先：
    /// 1. 本模块的 `fn_param_names`（定义与调用同文件）——同模块函数的形参表
    ///    不含可选位语义，故 `required_min` 取全长（全必需）
    /// 2. 模块注册表里的导出：优先用导出携带的 `param_names`（无必需下限信息，
    ///    同样按全必需），否则走**与 typecheck 原生臂同一出口**
    ///    `signature::export_signature_param_info`（同一签名串给出形参名与必需下限）。
    ///    这层同源是 t14 的要点：ir_gen 曾硬编全必需位 ⇒ 合法省略可选位的命名调用
    ///    被共享判据误报 E1010（实测 `assert.assert(cond = 1 == 1)`）。
    ///
    /// **方法面的名字不从这里取**（WBS 4.9.10 D-3）：方法是类型成员、不占模块
    /// 导出位，名字随重载决议载荷（`MethodResolution::param_names`）直达 IR 生成。
    /// 旧「来源 2」（`method_param_info`：按 IR 调用名反查投影索引再回候选组）
    /// 已随之删除——名字只有一条通路，不再有第二份来源。
    ///
    /// `full_path` 是解析后的限定名（如 `std.list.push`；跨单元方法形如
    /// `lib.Point.scale_by`，由 typecheck 的方法身份直出，见 WBS 4.9.10 D-8）；
    /// `leaf` 是短名。
    pub(in crate::middle) fn lookup_param_info(
        &self,
        full_path: &str,
        leaf: &str,
    ) -> Option<crate::frontend::core::sema::environment::ExportParamInfo> {
        // #466：先查**全名/混编名**键——重载候选的形参名按混编名精确登记，
        // 叶子名键在同名重载下末次覆盖，会让决议命中的候选拿错名字表。
        // 非重载路径全名不在本表（fn_param_names 只登记本模块定义），落回叶名。
        if let Some(names) = self.fn_param_names.get(full_path) {
            let names = names.clone();
            let required_min = names.len();
            return Some(crate::frontend::core::sema::environment::ExportParamInfo {
                names,
                required_min,
            });
        }

        if let Some(names) = self.fn_param_names.get(leaf) {
            let names = names.clone();
            let required_min = names.len();
            return Some(crate::frontend::core::sema::environment::ExportParamInfo {
                names,
                required_min,
            });
        }
        let export = self.registry.resolve_export(full_path).ok()?;
        if let Some(names) = export.param_names.clone() {
            let required_min = names.len();
            return Some(crate::frontend::core::sema::environment::ExportParamInfo {
                names,
                required_min,
            });
        }
        // #454 / t14：原生导出的注册点（src/std/mod.rs）只带签名串、不带形参名表，
        // 故第三个来源 = 从**签名串**提取形参名与必需下限——与 typecheck 原生臂
        // 同一出口，避免两侧各算一份而分歧。
        let (names, required_min) =
            crate::frontend::core::sema::signature::export_signature_param_info(&export.signature)?;
        Some(crate::frontend::core::sema::environment::ExportParamInfo {
            names,
            required_min,
        })
    }

    /// 声明**返回位**若是完整 Fn（括号返回 / 闭包包装形态），取该内层 Fn 的
    /// 运行期形参（名字, 类型）；否则空表。
    ///
    /// 用途：闭包 IR 的形参定型。parser 不再往实现头填声明类型（D54 修复轮 F1），
    /// 闭包自己拿不到外层声明，只能由正在生成的函数把「返回位声明的形参」递下来。
    /// 编译期槽的剔除口径与别处同源（`extract_generic_param_names`）。
    /// **探针注入**：谓词问答取本单元声明面（`self.type_level`），不取静态
    /// `SemanticProbe`——用户声明的谓词不在内置名表里（6.9.1 观察项）。
    pub(in crate::middle) fn paren_return_value_params(
        &self,
        type_annotation: Option<&ast::Type>,
    ) -> Vec<(String, MonoType)> {
        let Some(ast::Type::Fn { return_type, .. }) = type_annotation else {
            return Vec::new();
        };
        // 返回位可能是裸括号（`() -> ((a: Int) -> Int)`）——`Paren` 在语义层透明，
        // 递归内层即可（见 `ast::Type::is_paren_complete` 的口径）。
        let mut inner_ty: &ast::Type = return_type.as_ref();
        while let ast::Type::Paren(inner) = inner_ty {
            inner_ty = inner.as_ref();
        }
        let ast::Type::Fn { params: inner, .. } = inner_ty else {
            return Vec::new();
        };
        let flat: Vec<ast::Param> = inner.to_vec();
        // 扫描面 = 内层 Fn 的整棵声明（形参标注 ∪ 返回位）：被返回闭包的编译期
        // 形参若只在它的返回位被引用，也必须从运行期形参里剔除。
        let generic = crate::ast::extract_generic_param_names(&flat, &[inner_ty], &self.type_level);
        flat.iter()
            .filter(|p| !generic.iter().any(|g| g.name == p.name))
            .map(|p| {
                let ty =
                    p.ty.clone()
                        .map(MonoType::from)
                        .unwrap_or_else(|| MonoType::Int(64));
                (p.name.clone(), ty)
            })
            .collect()
    }

    /// 实现头形参的**声明来源**：运行期值位的前 N 位（N = 实现头形参数）。
    ///
    /// 与 typecheck 的绑定位对齐**同一条规则**（值级签名前 N 位），切法单点在
    /// `ast::Type::runtime_value_params`（沿柯里链走到最内层值组、再剔除编译期位）。
    /// IR 只**渲染**声明写下的类型与名字，不参与判定（WBS 6.8.2 / D54 修复轮 F1）。
    ///
    /// 一切「实现头第 i 位的声明信息」都从这里取：形参**类型**（`impl_param_types`）与
    /// **调用侧可用名**（`self.fn_param_names` 的登记）——同一 helper、同一顺序，
    /// 不得各自再切一次。
    ///
    /// （形参「写出类型名」那条链已随 WBS 4.9.10 F1 退役：方法身份改由 typecheck 的
    /// `ImportFace`（D-5/D-6）判定，IR 生成侧不再留任何「写出的类型名」存储。）
    /// SPEC §7.1 分类器的**真注入**：候选类型取 L3 权威表；类型命名空间与谓词集
    /// 由 `generate_module_ir` 入口从 AST 扫出，不使用保守假值。
    /// 实现头绑定序列第 i 位在**调用点实参序列**里的下标（SPEC §7.1「两条序列」）。
    ///
    /// 绑定序列 = 形参 − 类型参数 − 编译期值参数；调用点序列 = 形参 − 类型参数（编译期值
    /// 参数**仍占**实参位）。故绑定第 i 位常常**不等于**运行期第 i 位：
    /// `f: (N: Int, M: N) -> Int = (m) => m` 的 `m` 是绑定第 0 位、运行期第 1 位（第 0 位
    /// 是 N）。名字在声明里唯一，故按名取下标；取不到时退回 i。
    ///
    /// 与 typecheck 侧同源：两侧都用 `runtime_value_params`（调用点序列）与
    /// `declaration_runtime_param_slots`（绑定序列）这两个既有 helper，不另切一刀。
    pub(in crate::middle) fn impl_head_runtime_indices(
        &self,
        type_annotation: Option<&ast::Type>,
    ) -> Vec<usize> {
        let Some(ann) = type_annotation else {
            return Vec::new();
        };
        let call_seq: Vec<String> = ann
            .runtime_value_params()
            .into_iter()
            .map(|p| p.name)
            .collect();
        let bindable = self.declaration_runtime_param_slots(Some(ann), call_seq.len());
        // 两条序列等长 ⇒ 恒等映射（没有编译期值参数，或它们不与值参数同层）。只有确实
        // 存在「不占绑定序列的编译期值参数」时才按下标重映射——否则柯里层/无标注形参
        // 的既有槽位顺序会被打乱（语料 `01-syntax/basics/statement-termination.yx` 实测
        // 因此 E6005）。
        if bindable.len() == call_seq.len() {
            return (0..bindable.len()).collect();
        }
        // 按名对位必须**带游标**：未命名形参（D53 的裸类型项）名字槽是空串，
        // 多个空串用 `position` 会全落到第一个空名下标上（同签名里两个
        // `Array(Int, N)` 位会得到同一个槽）。游标让同名/同空名的多个位按
        // 声明序一位一位往后对。（未命名位不可能是编译期值参数——const 候选
        // 要求形参有名，见 `classify_compile_time_value_params`。）
        let mut out = Vec::with_capacity(bindable.len());
        let mut cursor = 0usize;
        for (i, p) in bindable.into_iter().enumerate() {
            match call_seq
                .iter()
                .enumerate()
                .skip(cursor)
                .find(|(_, n)| **n == p.name)
                .map(|(idx, _)| idx)
            {
                Some(idx) => {
                    out.push(idx);
                    cursor = idx + 1;
                }
                None => out.push(i),
            }
        }
        out
    }

    pub(in crate::middle) fn declaration_runtime_param_slots(
        &self,
        type_annotation: Option<&ast::Type>,
        count: usize,
    ) -> Vec<ast::Param> {
        type_annotation
            .map(|ann| self.with_param_classifier(|cls| ann.runtime_value_params_cls(cls)))
            .unwrap_or_default()
            .into_iter()
            .take(count)
            .collect()
    }

    /// 以本单元注入的问答**借用**分类器：三个问答的取值只在这里写一次。
    ///
    /// - `is_const_param_type`：L3 权威表（`sema::probe::CONST_PARAM_TYPES`）；
    /// - `is_type_name`：本单元声明的类型名（生成入口扫出）；
    /// - `is_predicate`：本单元声明面的谓词名（`TypeLevelNamespace`，**不**用静态
    ///   `SemanticProbe` 独立判谓词——用户声明的谓词不在内置名表里，6.9.1 观察项）。
    ///
    /// `ParamClassifier` 借用闭包，故用回调把「组装」与「消费」隔开——返回值
    /// 不能直接产出分类器（悬垂引用）。
    pub(in crate::middle) fn with_param_classifier<R>(
        &self,
        f: impl FnOnce(&ast::ParamClassifier<'_>) -> R,
    ) -> R {
        let is_const_param_type =
            |n: &str| crate::frontend::core::sema::probe::is_const_param_type_name(n);
        let is_type_name = |n: &str| self.declared_type_names.contains(n);
        let is_predicate = |n: &str| self.type_level.predicates.contains(n);
        let cls = ast::ParamClassifier {
            is_const_param_type: &is_const_param_type,
            is_type_name: &is_type_name,
            is_predicate: &is_predicate,
        };
        f(&cls)
    }

    /// 实现头形参的 IR 类型表：作者标注优先，缺标注按声明同类位渲染，都没有时
    /// 按 WBS 2.1.2 的惯例填 `Int(64)` 占位（`filter_map` 丢弃会让 params 长度与
    /// 帧布局错位——verify 实测）。
    pub(in crate::middle) fn impl_param_types(
        &self,
        params: &[ast::Param],
        type_annotation: Option<&ast::Type>,
    ) -> Vec<MonoType> {
        let declared = self.declaration_runtime_param_slots(type_annotation, params.len());
        params
            .iter()
            .enumerate()
            .map(|(i, p)| {
                p.ty.clone()
                    .or_else(|| declared.get(i).and_then(|d| d.ty.clone()))
                    .map(MonoType::from)
                    .unwrap_or(MonoType::Int(64))
            })
            .collect()
    }

    /// 获取约束变量的具体类型名（如果编译期可确定）
    ///
    /// 用于方法调用时选择直接调用（零开销）而非 vtable 查找
    pub(in crate::middle) fn get_constraint_var_concrete_type(
        &self,
        var_name: &str,
    ) -> Option<&String> {
        self.constraint_var_concrete_types.get(var_name)
    }

    /// 查找变量的类型
    /// 该名字是否为「函数值」——即 typecheck 的 bindings 里其类型是 `Fn`。
    ///
    /// 用于 `Expr::Var` 的函数名物化（#348）：函数的类型是 Fn，所以名字就是函数值。
    pub(in crate::middle) fn binding_is_function(
        &self,
        name: &str,
    ) -> bool {
        matches!(
            self.lookup_var_type(name).map(|p| &p.body),
            Some(MonoType::Fn { .. })
        )
    }

    fn lookup_var_type(
        &self,
        name: &str,
    ) -> Option<&PolyType> {
        if let Some(ref type_result) = self.type_result {
            // 调试：打印所有绑定
            tracing::debug!("Looking for variable '{}' in bindings", name);
            tracing::debug!("All bindings: {:?}", type_result.bindings);

            if let Some(poly_type) = type_result.bindings.get(name) {
                // 使用 debug 日志记录类型信息
                tracing::debug!("Found type for variable {}: {:?}", name, poly_type);
                return Some(poly_type);
            }
        } else {
            tracing::debug!("type_result is None!");
        }
        tracing::debug!("Type not found for variable: {}", name);
        None
    }

    // 删除的函数：extract_type_name_from_poly
    // 原因：根据设计文档，不再需要复杂的类型名提取逻辑
    // 方法调用现在直接生成简单函数名（方法名）

    /// 表达式结果的 `MonoType`（7b.2 值表类型来源，提交信息）。
    ///
    /// **只取已有可信来源，不新造推断**：变量取类型检查结果 / 绑定表的真实类型；
    /// 构造器调用取类型名；普通函数调用取绑定函数类型的返回类型；其余返回
    /// `None`（调用方落显式未知 `TypeRef("?")`，不静默猜类型）。
    pub(in crate::middle) fn expr_value_type(
        &self,
        expr: &ast::Expr,
    ) -> Option<MonoType> {
        match expr {
            ast::Expr::Var(name, _) => {
                if let Some(mono_type) = self
                    .type_result
                    .as_ref()
                    .and_then(|tr| tr.local_var_types.get(name))
                {
                    return Some(mono_type.clone());
                }
                self.lookup_var_type(name).map(|poly| poly.body.clone())
            }
            ast::Expr::Call { func, .. } => {
                let ast::Expr::Var(name, _) = func.as_ref() else {
                    return None;
                };
                if self.struct_definitions.contains_key(name) {
                    return Some(MonoType::TypeRef(name.clone()));
                }
                match self.lookup_var_type(name).map(|poly| &poly.body) {
                    Some(MonoType::Fn { return_type, .. }) => Some((**return_type).clone()),
                    _ => None,
                }
            }
            _ => None,
        }
    }

    /// 解析字段索引（6.9.5-B.1b / H2：返回**运行期数据索引**）
    ///
    /// 从类型信息和结构体定义中动态查找字段在结构体中的位置。记录字段命中后
    /// 不返回声明序下标，而是走 `runtime_field_index`（唯一映射点）换算成
    /// 运行期槽下标——witness 字段运行期无槽，命中即 `None`（值级访问在
    /// 类型检查层已被 E1114 拒绝，这里只保证 IR 不会为 witness 造槽）。
    /// 非记录分支（元组下标 / Vec.length / Range）保持恒等。
    ///
    /// 查找顺序：
    /// 1. 从表达式的类型推导出结构体名，再从 struct_definitions 查找字段索引
    /// 2. 遍历所有结构体定义查找匹配的字段名（兜底）
    pub(in crate::middle) fn resolve_field_index(
        &self,
        expr: &ast::Expr,
        field_name: &str,
    ) -> Option<usize> {
        // 0. 元组下标（`t.0` / `t.1`）：解析期已把数字下标转成十进制字段名。
        // 元组的“字段索引”就是位置下标本身，直接解析返回。
        if let Some(t) = self.get_expr_mono_type(expr) {
            // 剥掉 Ref 层：`it: &(A, B)` 取 `it.1` 时类型是 `Ref{Tuple}`。
            let mut resolved = t.clone();
            while let crate::frontend::core::types::mono::MonoType::Ref { inner, .. } = resolved {
                resolved = *inner;
            }
            let is_tuple = matches!(
                &resolved,
                crate::frontend::core::types::mono::MonoType::Generic { name, .. }
                    if name == "Tuple"
            );
            if is_tuple {
                if let Ok(i) = field_name.parse::<usize>() {
                    return Some(i);
                }
            }
        }

        // 0. `Vec(T)` 的 `length`（RFC-011 容器命名分层）：内建缓冲的长度。
        // 运行时以 `RuntimeValue::List/Array` 表示，长度即底层条目数；
        // 写 `v.length = n` 语义为「把缓冲调整到 n 个元素」（截断或补 Void）。
        // 用字段索引 0 作占位——执行器按**运行时值类型**与结构体字段区分。
        if field_name == "length" {
            if let Some(t) = self.get_expr_mono_type(expr) {
                let mut resolved = t.clone();
                while let crate::frontend::core::types::mono::MonoType::Ref { inner, .. } = resolved
                {
                    resolved = *inner;
                }
                if matches!(
                    &resolved,
                    crate::frontend::core::types::mono::MonoType::Generic { name, .. }
                        if name == "Vec" || name == "Array"
                ) {
                    return Some(0);
                }
            }
        }

        // 0. #302：Range 具名字段（start=0, end=1, step=2）
        if matches!(field_name, "start" | "end" | "step")
            && self
                .get_expr_mono_type(expr)
                .map(|t| t.is_range())
                .unwrap_or(false)
        {
            return match field_name {
                "start" => Some(0),
                "end" => Some(1),
                _ => Some(2),
            };
        }

        // 1. 尝试从表达式类型推导结构体名，精确查找
        if let Some(type_name) = self.get_expr_struct_type_name(expr) {
            if let Some(fields) = self.struct_definitions.get(&type_name) {
                for (i, field) in fields.iter().enumerate() {
                    if field.name == field_name {
                        return crate::frontend::core::types::mono::runtime_field_index(
                            fields.iter().map(|f| f.quantity),
                            i,
                        );
                    }
                }
            }
        }

        // 2. 兜底：遍历所有结构体定义查找字段名（当类型推导不可用时）
        for fields in self.struct_definitions.values() {
            for (i, field) in fields.iter().enumerate() {
                if field.name == field_name {
                    return crate::frontend::core::types::mono::runtime_field_index(
                        fields.iter().map(|f| f.quantity),
                        i,
                    );
                }
            }
        }

        // 3. 未找到，返回 None
        None
    }

    /// 从表达式推导其结构体类型名称
    ///
    /// 用于 resolve_field_index 等需要知道表达式类型的场景
    pub(in crate::middle) fn get_expr_struct_type_name(
        &self,
        expr: &ast::Expr,
    ) -> Option<String> {
        match expr {
            ast::Expr::Var(name, _) => {
                // 从类型检查结果查找变量类型
                if let Some(ref type_result) = self.type_result {
                    if let Some(mono_type) = type_result.local_var_types.get(name) {
                        return Self::mono_type_to_struct_name(mono_type);
                    }
                }
                // 从 bindings 查找
                if let Some(poly_type) = self.lookup_var_type(name) {
                    let mono_type = poly_type.body.clone();
                    return Self::mono_type_to_struct_name(&mono_type);
                }
                // 从 IR 生成器追踪的类型查找
                if let Some(type_name) = self.local_var_types.get(name) {
                    // #266: &mut 令牌穿透——"&mut Point" 取底层 "Point"
                    let base = Self::strip_ref_prefix(type_name);
                    if self.struct_definitions.contains_key(base) {
                        return Some(base.to_string());
                    }
                }
                None
            }
            ast::Expr::Call { func, .. } => {
                // 构造器调用：Point(...) -> 类型名为 "Point"
                if let ast::Expr::Var(name, _) = func.as_ref() {
                    if self.struct_definitions.contains_key(name) {
                        return Some(name.clone());
                    }
                }
                None
            }
            _ => None,
        }
    }

    /// 从 MonoType 提取结构体类型名
    fn mono_type_to_struct_name(mono_type: &MonoType) -> Option<String> {
        match mono_type {
            MonoType::TypeRef(name) => Some(name.clone()),
            MonoType::Struct(st) => Some(st.name.clone()),
            // RFC-011 §4.2：泛型类型实例（`ValList(Int)`）— 方法绑定注册在类型名上
            // （`ValList.push`），故用 Generic 的名字取绑定；不给名字则方法调用
            // 落到 native 查找（`m.push` → “Native function not found”）。
            MonoType::Generic { name, .. } => Some(name.clone()),
            // #266: &mut 令牌穿透——Ref 递归取 inner（m = &mut q 的类型是
            // Ref { mutable, inner: Point }，方法派发应基于 Point）
            MonoType::Ref { inner, .. } => Self::mono_type_to_struct_name(inner),
            _ => None,
        }
    }

    /// #266: 穿透 `&` / `&mut` 引用前缀取底层类型名
    /// `"&mut Point"` → `"Point"`；`"&Point"` → `"Point"`；无前缀原样返回
    pub(in crate::middle) fn strip_ref_prefix(type_name: &str) -> &str {
        let t = type_name.trim();
        if let Some(rest) = t.strip_prefix("&mut ") {
            rest
        } else if let Some(rest) = t.strip_prefix('&') {
            rest.trim_start()
        } else {
            t
        }
    }

    /// 声明期分类的量词投影（`TypeCheckResult.record_models` 是唯一来源）。
    ///
    /// `struct_definitions` 与 `TypeDeclBody` 都从这里取字段：AST 的
    /// `StructField.quantity` 生产恒为 `None`，量词只在记录模型里。
    pub(in crate::middle) fn classified_fields(
        &self,
        name: &str,
        fields: &[ast::StructField],
    ) -> Vec<ast::StructField> {
        let mut patched = fields.to_vec();
        if let Some(model) = self
            .type_result
            .as_ref()
            .and_then(|result| result.record_models.get(name))
        {
            for (field, classified) in patched.iter_mut().zip(model.fields.iter()) {
                field.quantity = classified.quantity;
            }
        }
        patched
    }

    /// 构造参数/布局的**数据字段投影**：witness 字段运行期无槽位、也不是构造实参。
    pub(in crate::middle) fn data_ctor_fields(
        fields: &[ast::StructField]
    ) -> Vec<(usize, ast::StructField)> {
        fields
            .iter()
            .enumerate()
            .filter(|(_, field)| {
                !crate::frontend::core::types::mono::quantity_is_witness(field.quantity)
            })
            .map(|(index, field)| (index, field.clone()))
            .collect()
    }

    /// 类型声明在**字节码类型表**里的项。
    ///
    /// 等价于原 `codegen::type_from_ast`（WBS 6.6.12 前移）：载体只带 `MonoType`，
    /// codegen 不再接触 AST。逐臂与原实现一致——`Name`→`TypeRef`、`Tuple`→`make_tuple`、
    /// `Fn`→`Fn`（丢无名形参，与原实现相同）、`Generic`→`TypeRef("Name(arg, …)")`，
    /// 其余（含 `Struct`/`NamedStruct`/`MetaType` 等）→ `Void`。
    pub(in crate::middle) fn type_decl_self_type(ast_type: &ast::Type) -> MonoType {
        match ast_type {
            ast::Type::Name { name, .. } => MonoType::TypeRef(name.clone()),
            ast::Type::Tuple(types) => {
                MonoType::make_tuple(types.iter().map(Self::type_decl_self_type).collect())
            }
            ast::Type::Fn {
                params,
                return_type,
                ..
            } => MonoType::Fn {
                params: params
                    .iter()
                    .filter_map(|p| p.ty.as_ref())
                    .map(Self::type_decl_self_type)
                    .collect(),
                return_type: Box::new(Self::type_decl_self_type(return_type)),
            },
            ast::Type::Generic { name, args, .. } => MonoType::TypeRef(format!(
                "{}({})",
                name,
                args.iter()
                    .map(|t| Self::type_decl_self_type(t).type_name())
                    .collect::<Vec<_>>()
                    .join(", ")
            )),
            _ => MonoType::Void,
        }
    }

    /// 字段注解是否为**函数类型**（= 方法字段）。
    ///
    /// 剥掉透明包装（`Paren` / `NamedParen`）后再判——与 `From<ast::Type> for MonoType`
    /// 的透明口径一致（RFC-004 / RFC-027 §3）。SPEC `type-system.md:443`：函数类型字段
    /// 不生成构造参数；当前 `translator::synthesize_constructor` 仍把两类都算进构造参数
    /// （已知不一致，登记见提交信息，本轮不改行为）。
    pub(in crate::middle) fn field_is_method(ty: &ast::Type) -> bool {
        let mut t = ty;
        loop {
            match t {
                ast::Type::Paren(inner) => t = inner,
                ast::Type::NamedParen { inner, .. } => t = inner,
                ast::Type::Fn { .. } => return true,
                _ => return false,
            }
        }
    }

    // 迭代器协议已通过 Call 指令实现，不再需要独立的 IR 指令
    // 保留指令定义以供将来使用

    /// 获取表达式的 span
    pub(in crate::middle) fn get_expr_span(expr: &ast::Expr) -> Span {
        match expr {
            ast::Expr::Lit(_, span) => *span,
            ast::Expr::Var(_, span) => *span,
            ast::Expr::BinOp { span, .. } => *span,
            ast::Expr::UnOp { span, .. } => *span,
            ast::Expr::Call { span, .. } => *span,
            ast::Expr::If { span, .. } => *span,
            ast::Expr::Match { span, .. } => *span,
            ast::Expr::While { span, .. } => *span,
            ast::Expr::For { span, .. } => *span,
            ast::Expr::SpawnFor { span, .. } => *span,
            ast::Expr::Block(block) => block.span,
            ast::Expr::Return(_, span) => *span,
            ast::Expr::Break(span) => *span,
            ast::Expr::Continue(span) => *span,
            ast::Expr::Cast { span, .. } => *span,
            ast::Expr::Tuple(_, span) => *span,
            ast::Expr::List(_, span) => *span,
            ast::Expr::ListComp { span, .. } => *span,
            ast::Expr::Dict(_, span) => *span,
            ast::Expr::Index { span, .. } => *span,
            ast::Expr::FieldAccess { span, .. } => *span,
            ast::Expr::Try { span, .. } => *span,
            ast::Expr::Ref { span, .. } => *span,
            ast::Expr::Unsafe { span, .. } => *span,
            ast::Expr::Spawn { span, .. } => *span,
            ast::Expr::Lambda { span, .. } => *span,
            ast::Expr::FString { span, .. } => *span,
            ast::Expr::Error(span) => *span,
            ast::Expr::Borrow { span, .. } => *span,
            ast::Expr::In { span, .. } => *span,
        }
    }

    /// 获取表达式的实际类型名称（用于错误消息）
    ///
    /// 通过查询类型检查结果获取表达式的真正类型，而不是仅描述 AST 节点结构。
    /// 例如对于变量 `nums`，返回 `List<int64>` 而非 `变量 \`nums\``。
    pub(in crate::middle) fn get_expr_type_name(
        &self,
        expr: &ast::Expr,
    ) -> String {
        // 如果表达式是变量，尝试从多个来源查找其类型
        if let ast::Expr::Var(name, _) = expr {
            // 1. 从类型检查结果中的 local_var_types 查找（最准确，包含具体类型）
            if let Some(ref type_result) = self.type_result {
                if let Some(mono_type) = type_result.local_var_types.get(name) {
                    return mono_type.type_name();
                }
            }
            // 2. 从 bindings 中查找全局绑定
            if let Some(poly_type) = self.lookup_var_type(name) {
                let mono_type = poly_type.body.clone();
                return mono_type.type_name();
            }
            // 3. 从 IR 生成器本地追踪的类型中查找
            if let Some(type_name) = self.local_var_types.get(name) {
                return type_name.clone();
            }
        }

        // 构造器调用：Point(1.0, 2.0) → 类型名为 "Point"
        if let ast::Expr::Call { func, .. } = expr {
            if let ast::Expr::Var(name, _) = func.as_ref() {
                if self.struct_definitions.contains_key(name) {
                    return name.clone();
                }
            }
        }

        // 对于其他表达式，不做 AST 猜测
        "<unknown>".to_string()
    }

    /// 解析函数表达式为函数名 Operand（用于普通函数调用）
    pub(in crate::middle) fn resolve_function_name(
        &self,
        func: &ast::Expr,
    ) -> Result<Operand, Diagnostic> {
        if let Expr::Var(name, _) = func {
            // 用户**本模块**定义的类型/结构体优先于 `use` 导入的 std 名字。
            // 否则 `use std.math` 后用户写 `PI: Type = { .. }; PI(7)` 会被解析成
            // `std.math.PI`（圆周率常量）——调用常量得到 void，静默错值（D10）。
            // 要访问被遮蔽的 std 名，写限定名 `math.PI` 即可。
            //
            // 同理（issue #477 / WBS 3.8.1，用户裁决乙案），用户**本模块**定义的
            // **函数**也优先于 std native 短名：check 面（sema）按作用域解析到
            // 用户函数、按用户签名检查通过；lower 若把同名裸名改写成 std native
            // 限定名，同一调用点 check 绿、run 派发 native——修前实测两形态：
            // 实参运行期兼容时 native 静默执行给错误值（exit 0），不兼容时
            // 运行期 E6007（exit 1）。此处返回裸名，随后 `qualify_names` /
            // `resolve_call_target`（module_ir.rs）按本单元 rename 表把它限定成
            // 本模块函数——运行期命中用户函数，与 check 面一致。
            // 要访问被遮蔽的 std 名，写限定名 `os.read` 即可。
            // std 嵌入单元各自做 IR 生成（`declared_fn_names` 按单元填充），
            // std 模块内部互调命中的是 std 自己的函数名，语义零变化。
            let locally_defined = self.declared_type_names.contains(name);
            let locally_defined_fn = self.declared_fn_names.contains(name);
            let resolved_name = if locally_defined || locally_defined_fn {
                name.clone()
            } else if let Some(qualified) = self.use_aliases.get(name) {
                qualified.clone()
            } else if self
                .registry
                .qualified_name_is(name, QualifiedNameConcern::NativeName)
            {
                name.clone()
            } else if let Some(qualified) = self.registry.short_to_qualified_map().get(name) {
                qualified.clone()
            } else {
                name.clone()
            };
            Ok(Operand::Const(ConstValue::String(resolved_name)))
        } else if let Expr::Call {
            func: inner,
            args: inner_args,
            ..
        } = func
        {
            // 两层调用 `X(T)(...)`：内层类型实参运行期擦除。
            //
            // - 结构体构造器（`Container(Int)(42, 43)`）：按类型名生成，
            //   字段填充由调用点实参决定。
            // - **泛型函数**（`mk(Int)(5)`）：内层是类型应用，运行期完全无表示。
            //   单态化器把 `mk` 特化成 `mk(int64)` 并**剥掉未特化的 `mk`**，
            //   所以这里必须发**特化名**，否则运行期找不到 `mk`（E6006，D6.3）。
            if let Expr::Var(name, _) = inner.as_ref() {
                if self.struct_definitions.contains_key(name) {
                    return Ok(Operand::Const(ConstValue::String(name.clone())));
                }
                if self.lookup_local(name).is_none() {
                    let base = self.resolve_function_name(inner)?;
                    let base_name = match &base {
                        Operand::Const(ConstValue::String(s)) => s.clone(),
                        _ => name.clone(),
                    };
                    // 仅在实参确为类型值时叠特化后缀（值实参=部分应用，不在此列）
                    if !inner_args.is_empty()
                        && inner_args.iter().all(|a| self.expr_is_type_value(a))
                    {
                        let args_str = inner_args
                            .iter()
                            .map(Self::type_value_ir_name)
                            .collect::<Vec<_>>()
                            .join(", ");
                        return Ok(Operand::Const(ConstValue::String(format!(
                            "{}({})",
                            base_name, args_str
                        ))));
                    }
                    return Ok(base);
                }
            }
            Err(ErrorCodeDefinition::ir_internal_error(&format!(
                "无法解析函数名：非结构体构造器调用 {:?}",
                func
            ))
            .at(Self::get_expr_span(func))
            .build())
        } else {
            Err(ErrorCodeDefinition::ir_internal_error(&format!(
                "无法解析函数名：非变量表达式 {:?}",
                func
            ))
            .at(Self::get_expr_span(func))
            .build())
        }
    }

    /// 判断函数表达式是否可在编译期解析为静态函数名。
    ///
    /// 只有全局函数声明（通过 let/fn 定义的具名函数）才是静态的。
    /// 局部变量、闭包表达式、函数调用返回值等全部走动态分发。
    /// 该表达式是否是**类型值**（类型宇宙的值）：`Int`、`Vec(Int)` 等。
    ///
    /// 用于区分「类型实参层」（擦除）与「部分应用」（真实调用）：
    /// `mk(Int)(5)` 的 `Int` 是类型值，内层不生成运行时代码；
    /// `f(10)(5)` 的 `10` 是普通值，内层必须真的调用并返回闭包。
    fn expr_is_type_value(
        &self,
        expr: &ast::Expr,
    ) -> bool {
        let ast::Expr::Var(name, _) = expr else {
            return false;
        };
        // 本模块声明的局部变量/参数优先——变量叫 `Int` 也是值
        if self.lookup_local(name).is_some() {
            return false;
        }
        crate::frontend::core::types::mono::MonoType::from_builtin_name(name).is_some()
            || matches!(
                name.as_str(),
                "Vec" | "Array" | "Dict" | "Tuple" | "Option" | "Result" | "Range" | "Iter"
            )
            || self.declared_type_names.contains(name)
    }

    /// 类型值表达式 → `MonoType`（`Int` → `Int(64)`，`Vec(Int)` → `Generic`）。
    ///
    /// 与 `typecheck` 侧的 `type_arg_of_expr` 同口径（两边都必须把类型名解成
    /// 真实类型，而非笼统的 `MetaType`）。
    fn type_arg_of_expr_ir(expr: &ast::Expr) -> MonoType {
        match expr {
            ast::Expr::Var(name, _) => {
                crate::frontend::core::types::mono::MonoType::from_builtin_name(name)
                    .unwrap_or_else(|| {
                        crate::frontend::core::types::mono::MonoType::TypeRef(name.clone())
                    })
            }
            ast::Expr::Call { func, args, .. } => {
                if let ast::Expr::Var(cname, _) = func.as_ref() {
                    MonoType::Generic {
                        // #403：与 typecheck 侧 `type_arg_of_expr` 同判据——源名进
                        // 类型域落点归一（单源 [`MonoType::source_container_face`]）。
                        // `Vec(List(Int))(…)` 直造形态的内层 `List(Int)` 走本臂
                        // 递归，不过归一会在 L3 IR 视图产出 `Generic{"List"}`，
                        // 与 L2 表示劈叉。
                        name: MonoType::source_container_face(cname),
                        args: args.iter().map(Self::type_arg_of_expr_ir).collect(),
                    }
                } else {
                    MonoType::TypeRef("_".to_string())
                }
            }
            _ => MonoType::TypeRef("_".to_string()),
        }
    }

    /// 把类型值表达式转成单态化器的类型名（`Int` → `int64`，`String` → `string`）。
    ///
    /// 必须与 `middle::passes::mono` 生成特化名的口径**完全一致**
    /// （那边用 `MonoType::type_name()`）：`mk(Int)(5)` 要调的函数叫
    /// `mk(int64)`，名字对不上就 E6006。
    fn type_value_ir_name(expr: &ast::Expr) -> String {
        let ast::Expr::Var(name, _) = expr else {
            return "_".to_string();
        };
        crate::frontend::core::types::mono::MonoType::from_builtin_name(name)
            .unwrap_or_else(|| crate::frontend::core::types::mono::MonoType::TypeRef(name.clone()))
            .type_name()
    }

    /// 该调用点是否为**静态调用**——callee 是函数表里的一个名字（代码），而非某个值。
    ///
    /// 名字解析与 [`Self::generate_var_expr_ir`] 同一条优先级（局部变量 → 全局
    /// 槽位 → `use` 引入的跨文件值 → …）：命中就是**变量**，调用必须走 `CallDyn`
    /// （间接调用，机制已在：闭包表达式路径即此）；全未命中才是函数表条目，按名 `Call`。
    ///
    /// 两条臂都是「值被当成函数名」的实证缺陷（WBS 3.6.1 / 3.6.2）：
    /// - 顶层值绑定（`f = double`，查 [`Self::lookup_global`]）：它不是局部变量、
    ///   却在 typecheck 的 bindings 里（类型是 `Fn`），于是被当成函数名。而函数表
    ///   里没有叫 `f` 的代码——`f = double` 不进函数表，只占一个全局槽位（由
    ///   `register_global_slots` 预分配，见其注释）。按名 `Call` 在运行期查扁平
    ///   函数表落空，报 `E6006 Native function not found: f`。
    /// - `use lib.{f}` / `use lib.{f as g}` 导入的函数类型值（查
    ///   [`Self::use_alias_value_slot`]）：别名解析到的是对方单元的全局槽位，函数表
    ///   里同样没有这个值，按名 `Call` 报 `E6006 Native function not found: lib.f`。
    pub(in crate::middle) fn is_static_fn_name(
        &self,
        func: &ast::Expr,
    ) -> bool {
        match func {
            ast::Expr::Var(name, _) => {
                if self.lookup_local(name).is_some() {
                    return false;
                }
                // 顶层值绑定是变量不是代码：与 `generate_var_expr_ir` 的
                // 「局部 → 全局」优先级一致，走间接调用。
                if self.lookup_global(name).is_some() {
                    return false;
                }
                // 跨文件导入的值（`use lib.{f}` / `use lib.{f as g}`）：别名落在
                // 跨文件值绑定槽位即是变量。联查与值路径同经 `use_alias_value_slot`
                // ——同一实现，两条路径不会各猜一个答案。
                if self.use_alias_value_slot(name).is_some() {
                    return false;
                }
                self.lookup_var_type(name).is_some()
            }
            // 两层调用 `X(T)(...)`：
            // - 内层是 struct 构造器（`Container(Int)(42, 43)`）——类型实参擦除
            // - 内层是**普通函数**且实参均为**类型**（`mk(Int)(5)`）——擦除类型实参层，
            //   外层才是真实调用。
            //
            // 关键判据：内层实参必须是类型（`MetaType`）而非值。否则会误伤
            // 部分应用（`f(10)(5)` 里 `f(10)` 是**真的返回闭包**，不是类型实参层
            // ——那会让 `f(10)(5)` 变成单次调用 `f(10, 5)` 而错值）。
            ast::Expr::Call {
                func: inner,
                args: inner_args,
                ..
            } => match inner.as_ref() {
                ast::Expr::Var(name, _) => {
                    if self.struct_definitions.contains_key(name) {
                        return true;
                    }
                    !inner_args.is_empty()
                        && inner_args.iter().all(|a| self.expr_is_type_value(a))
                        && self.lookup_local(name).is_none()
                        && self.lookup_var_type(name).is_some()
                }
                _ => false,
            },
            _ => false,
        }
    }

    /// 获取表达式的推断类型（用于 IR 生成阶段的分支）
    /// 该字段访问是否为容器的 `.length`（`Vec(T)` / `Array(T, N)`）。
    ///
    /// 剥掉 `Ref` 层：`&Vec(A)` 的 `.length` 也是长度读取（RFC-009 §2.8 读透明）。
    pub(in crate::middle) fn is_container_length_field(
        &self,
        expr: &Expr,
        field: &str,
    ) -> bool {
        field == "length"
            && matches!(self.get_expr_mono_type(expr), Some(ref t) if {
                let mut r = t.clone();
                while let crate::frontend::core::types::mono::MonoType::Ref { inner, .. } = r {
                    r = *inner;
                }
                r.is_vec() || r.is_array()
            })
    }

    /// 生成一个表达式的静态类型（IR 期视图）
    pub(in crate::middle) fn get_expr_mono_type(
        &self,
        expr: &ast::Expr,
    ) -> Option<MonoType> {
        match expr {
            ast::Expr::BinOp {
                op: ast::BinOp::Range,
                ..
            } => {
                // #300 I 项：Range 是一等值（start/end/step 三槽），类型恒为 Range(Int)
                Some(MonoType::Generic {
                    name: "Range".into(),
                    args: vec![MonoType::Int(64)],
                })
            }
            ast::Expr::Var(name, _) => {
                if let Some(ref type_result) = self.type_result {
                    if let Some(mono_type) = type_result.local_var_types.get(name) {
                        return Some(mono_type.clone());
                    }
                }

                self.lookup_var_type(name)
                    .map(|poly_type| poly_type.body.clone())
            }
            // RFC-011a §6: 索引表达式——列表/数组元素的静态类型（存在类型
            // 接收者解析依赖此臂：animals[0] → 元素类型 TypeRef(接口)）
            ast::Expr::Index { expr: base, .. } => {
                let base_ty = self.get_expr_mono_type(base);
                match base_ty {
                    Some(MonoType::Generic { name, args })
                        if (name == "List" || name == "Vec" || name == "Array")
                            && args.len() == 1 =>
                    {
                        Some(args[0].clone())
                    }
                    _ => None,
                }
            }
            ast::Expr::List(_, _) => Some(MonoType::make_list(MonoType::Void)),
            // `Vec(Int)(...)` / `Array(Int, 3)(...)` 的静态类型。
            //
            // 链式取字段（`Vec(Int)().length`）走 `get_expr_mono_type` 判类型，
            // 没有本臂就落到「普通字段访问」分支 → 报
            // E3005「无法解析字段索引: 'length'」。类型实参在 AST 里就是
            // `Call{func: Var(容器名), args: [类型表达式...]}`，直接解出来。
            ast::Expr::Call { func, .. } => {
                if let ast::Expr::Call {
                    func: inner,
                    args: type_args,
                    ..
                } = func.as_ref()
                {
                    if let ast::Expr::Var(cname, _) = inner.as_ref() {
                        if matches!(cname.as_str(), "Vec" | "Array")
                            && !self.declared_type_names.contains(cname)
                        {
                            return Some(MonoType::Generic {
                                name: cname.clone(),
                                args: type_args.iter().map(Self::type_arg_of_expr_ir).collect(),
                            });
                        }
                    }
                }
                None
            }
            ast::Expr::Tuple(items, _) => {
                let elems = vec![MonoType::Void; items.len()];
                Some(MonoType::make_tuple(elems))
            }
            ast::Expr::Dict(_, _) => Some(MonoType::make_dict(MonoType::Void, MonoType::Void)),
            _ => None,
        }
    }

    // ==================== RFC-011a §6 存在类型变体分发 ====================

    /// 接收者表达式是否为存在类型（接口构造器名）→ 接口名。
    /// 支持 Var 与 Index（animals[0]；二段式 d = animals[0] 的 d 是 Var）。
    pub(in crate::middle) fn existential_receiver_interface(
        &self,
        expr: &ast::Expr,
    ) -> Option<String> {
        match expr {
            ast::Expr::Var(_, _) => {
                let mono = self.get_expr_mono_type(expr);
                if let Some(t) = mono.and_then(|t| self.existential_interface_of(&t)) {
                    return Some(t);
                }
                // MonoType 表缺失时回退字符串类型名表（带注解 let 填充）
                let var = Self::var_name(expr)?;
                let name = self.local_var_types.get(&var)?;
                self.interface_from_name(Self::strip_ref_prefix(name))
            }
            ast::Expr::Index { expr: base, .. } => {
                let base_ty = self.receiver_base_mono_type(base)?;
                match base_ty {
                    MonoType::Generic { name, args }
                        if (name == "List" || name == "Vec" || name == "Array")
                            && args.len() == 1 =>
                    {
                        self.existential_interface_of(&args[0])
                    }
                    _ => None,
                }
            }
            _ => None,
        }
    }

    pub(in crate::middle) fn var_name(expr: &ast::Expr) -> Option<String> {
        match expr {
            ast::Expr::Var(n, _) => Some(n.clone()),
            _ => None,
        }
    }

    /// Index 链基底的静态类型（嵌套 Index 递归取元素类型）
    fn receiver_base_mono_type(
        &self,
        expr: &ast::Expr,
    ) -> Option<MonoType> {
        match expr {
            ast::Expr::Index { expr: base, .. } => self.receiver_base_mono_type(base),
            other => self.get_expr_mono_type(other),
        }
    }

    /// MonoType 是否为 RFC-011a 存在类型 → 接口名。
    /// 判据：接口构造器在 bindings 中注册为全函数字段的记录（is_constraint）。
    /// 遗留 trait 约束同样满足 is_constraint，但由调用方以 interface_variants
    /// 成员性门控隔离（无 ImplementationProof 的约束类型不走变体分发）。
    fn existential_interface_of(
        &self,
        ty: &MonoType,
    ) -> Option<String> {
        match ty {
            MonoType::TypeRef(n) => self.interface_from_name(n),
            MonoType::Struct(s) if !s.name.is_empty() && Self::is_constraint_struct(s) => {
                self.interface_from_name(&s.name)
            }
            _ => None,
        }
    }

    /// 按名字查接口构造器（bindings），确认是约束记录形态
    fn interface_from_name(
        &self,
        name: &str,
    ) -> Option<String> {
        let body = self.type_result.as_ref()?.bindings.get(name)?.body.clone();
        match body {
            MonoType::Struct(s) if Self::is_constraint_struct(&s) => Some(name.to_string()),
            _ => None,
        }
    }

    /// mono.rs `MonoType::is_constraint` 的 Struct 臂本地镜像（该方法在
    /// MonoType 上，这里只有 &StructType）
    fn is_constraint_struct(s: &crate::frontend::core::types::mono::StructType) -> bool {
        s.fields.iter().all(|f| matches!(f.ty, MonoType::Fn { .. }))
    }

    /// span → 接口名查表
    pub(in crate::middle) fn lookup_existential_wrap(
        &self,
        expr: &ast::Expr,
    ) -> Option<String> {
        let span = Self::expr_span(expr);
        self.existential_wraps.get(&span).cloned()
    }

    /// 从表达式提取源码 span（无法定位的形态返回 dummy——dummy 永远查不中
    /// 包装表，安全方向失败）
    pub(in crate::middle) fn expr_span(expr: &ast::Expr) -> Span {
        match expr {
            ast::Expr::Lit(_, s)
            | ast::Expr::Var(_, s)
            | ast::Expr::Return(_, s)
            | ast::Expr::Break(s)
            | ast::Expr::Continue(s) => *s,
            ast::Expr::BinOp { span, .. }
            | ast::Expr::UnOp { span, .. }
            | ast::Expr::Call { span, .. }
            | ast::Expr::If { span, .. }
            | ast::Expr::Match { span, .. }
            | ast::Expr::While { span, .. }
            | ast::Expr::For { span, .. }
            | ast::Expr::SpawnFor { span, .. }
            | ast::Expr::Borrow { span, .. }
            | ast::Expr::FieldAccess { span, .. }
            | ast::Expr::Index { span, .. }
            | ast::Expr::Tuple(_, span)
            | ast::Expr::List(_, span)
            | ast::Expr::Cast { span, .. }
            | ast::Expr::Try { span, .. } => *span,
            ast::Expr::Block(block) => block.span,
            _ => Span::dummy(),
        }
    }

    /// 收集模式引入的绑定名（去重，声明序）——臂级预分配寄存器用
    pub(in crate::middle) fn collect_binding_names(pattern: &ast::Pattern) -> Vec<String> {
        fn walk(
            p: &ast::Pattern,
            out: &mut Vec<String>,
        ) {
            match p {
                ast::Pattern::Identifier(n) => {
                    if !out.contains(n) {
                        out.push(n.clone());
                    }
                }
                ast::Pattern::Tuple(ps) | ast::Pattern::Or(ps) => {
                    for c in ps {
                        walk(c, out);
                    }
                }
                ast::Pattern::Struct { fields, .. } => {
                    for (_, _, c) in fields {
                        walk(c, out);
                    }
                }
                ast::Pattern::Union { pattern, .. } => {
                    if let Some(c) = pattern.as_deref() {
                        walk(c, out);
                    }
                }
                _ => {}
            }
        }
        let mut out = Vec::new();
        walk(pattern, &mut out);
        out
    }

    /// 内置容器的类型构造器名：`Vec(Int)(...)` 的内层 `Vec`。
    ///
    /// 只认 RFC-011 明确规格化的 `Vec(T)` / `Array(T, N)`。用户自有同名类型
    /// 优先（D10 同款纪律）。
    pub(in crate::middle) fn builtin_container_ctor_name(
        &self,
        inner: &Expr,
    ) -> Option<String> {
        let Expr::Var(name, _) = inner else {
            return None;
        };
        if self.declared_type_names.contains(name) {
            return None;
        }
        matches!(name.as_str(), "Vec" | "Array").then(|| name.clone())
    }

    /// RFC-010: 变体构造形态检测——`Result(Int, String).ok(5)` / `Color.red()`。
    /// 与 typecheck 的变体构造识别同一 AST 形态；span 键跨系统不可靠（两端
    /// 对 Call 节点 span 的语义不同），故就地检测并从 sum_type_variants 取变体序。
    pub(in crate::middle) fn detect_variant_ctor_call(
        &self,
        func: &Expr,
    ) -> Option<(String, usize, usize)> {
        use crate::ast::Expr;
        if let Expr::FieldAccess {
            expr: base_expr,
            field: variant_name,
            ..
        } = func
        {
            // 两种 base 形态：泛型 `Call(Var(名), 类型实参)` / 非泛型 `Var(名)`
            let type_name = match base_expr.as_ref() {
                Expr::Call {
                    func: inner_func, ..
                } => match inner_func.as_ref() {
                    Expr::Var(n, _) => Some(n),
                    _ => None,
                },
                Expr::Var(n, _) => Some(n),
                _ => None,
            }?;
            let variants = self.sum_type_variants.get(type_name)?;
            let idx = variants.iter().position(|v| v.0 == *variant_name)?;
            return Some((type_name.clone(), idx, 0));
        }
        None
    }
}
