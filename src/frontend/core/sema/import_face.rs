//! 导入面——本单元可见的**类型名 → 提供模块**（WBS 4.9.10 / RFC-039 L1 方法面接入 D-5）。
//!
//! 方法身份的解析只需要一个问题：「这个被写出来的类型名，定义在哪个模块？」。
//! 答案的**唯一来源**是本模块：本单元声明（自有）、花括号导入（含内联别名）、
//! 整体导入的命名空间前缀，加上 std 的类型基线。此前这四种来源分散在五处
//! 各自判定（driver 的候选注入、lower 的 `imported_type_owner`、
//! `std_method_binding_qualified_map`、typecheck 的两张改绑表），任一处判错都
//! 表现为**静默错配**——调用名与定义名分属不同单元却同名。
//!
//! 本表按 D-5 只做**一张表**：不做优先级推理、不做相容判断。
//! - `use lib` 整体导入只登记**命名空间前缀**（`lib.Temp` 这种写法要从前缀解析），
//!   裸类型名不入表——`use lib` 后的裸 `Temp` 仍按 E1001 响亮拒绝；
//! - 同一个裸名被多个模块带进作用域 = 源程序名字层已歧义，本表记入
//!   `conflicted` 并**不给来源**（调用点因此不产身份，保持拒绝面不变）。
//!
//! 表在 `TypeChecker::check_module` 入口建一次（`ImportFace::build`），
//! 之后 method key 的构造点只问 `TypeEnvironment::resolve_method_key`。

use std::collections::{HashMap, HashSet};

use crate::ast::{Module, StmtKind};
use crate::frontend::core::sema::passes::dead_code::DeadCodeAnalyzer;
use crate::frontend::core::types::identity::DefPath;
use crate::frontend::module::registry::ModuleRegistry;
use crate::frontend::module::ExportKind;

/// 一个类型名在导入面上的来源。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TypeOrigin {
    /// 提供模块键（本单元类型 = 本单元键；单文件形态下为空串，见 `build`）
    pub module: String,
    /// 定义侧类型裸名（内联别名时与本地名不同）
    pub target: String,
}

/// 导入面：类型名 → 来源，加整体导入的命名空间前缀。
#[derive(Debug, Clone, Default)]
pub struct ImportFace {
    /// 本地名 → 来源
    types: HashMap<String, TypeOrigin>,
    /// 名字层歧义集（同名被多个模块带进作用域）：不给来源，落不进身份
    conflicted: HashSet<String>,
    /// 命名空间前缀 → 模块键（`use lib` / `use lib as l`）
    namespaces: HashMap<String, String>,
}

impl ImportFace {
    /// 建表：本单元声明 + 花括号导入 + 命名空间前缀 + std 类型基线。
    ///
    /// `own_module` 是本单元类型的模块键——多文件形态 = 单元键，单文件形态 = 空串
    /// （与 lower 的 `module_key` 开关同源：单文件的 IR 函数名就是扁平名）。
    pub fn build(
        ast: &Module,
        registry: &ModuleRegistry,
        own_module: &str,
    ) -> Self {
        let mut face = Self::default();
        // ① 本单元声明的类型（AST 的 TypeDefinition 是断言的唯一来源）
        for stmt in &ast.items {
            if let StmtKind::TypeDefinition { name, .. } = &stmt.kind {
                face.types.insert(
                    name.clone(),
                    TypeOrigin {
                        module: own_module.to_string(),
                        target: name.clone(),
                    },
                );
            }
        }
        // ② 花括号导入项：只收**类型导出**（函数/常量不是类型名空间）
        let names = DeadCodeAnalyzer::collect_module_names(ast);
        for (local, target, use_path) in &names.imported {
            if face.types.contains_key(local) {
                continue; // 自有优先：同名裸名的双导入由 E5006 拒绝
            }
            let Some(exports) = registry.get_exports(use_path) else {
                continue;
            };
            if !exports
                .get(target)
                .is_some_and(|export| export.kind == ExportKind::Type)
            {
                continue;
            }
            let origin = TypeOrigin {
                module: use_path.clone(),
                target: target.clone(),
            };
            let register =
                |name: &str, face: &mut Self, origin: &TypeOrigin| match face.types.get(name) {
                    Some(existing) if existing != origin => {
                        // 同名被两个模块带进作用域：整名歧义（不猜、不 last-wins）
                        face.conflicted.insert(name.to_string());
                        face.types.remove(name);
                    }
                    Some(_) => {}
                    None => {
                        face.types.insert(name.to_string(), origin.clone());
                    }
                };
            register(local, &mut face, &origin);
        }
        // ③ 整体导入：只有命名空间前缀（`lib.Temp` 的 `lib`）
        for (local, module_path) in &names.namespaces {
            face.namespaces.insert(local.clone(), module_path.clone());
        }
        // ④ std 类型基线：std 的类型名在每个单元都可见（与 `TypeChecker::new`
        //    注入全部 std 方法绑定同源）。排序后再插入 ⇒ 同输入同结果，
        //    且不复活已判歧义的名字。
        for (type_name, module_key) in registry.std_type_origins() {
            if face.types.contains_key(&type_name) || face.conflicted.contains(&type_name) {
                continue;
            }
            face.types.insert(
                type_name.clone(),
                TypeOrigin {
                    module: module_key,
                    target: type_name,
                },
            );
        }
        face
    }

    /// 本地名 → 来源（歧义/未知名 → None）
    pub fn type_origin(
        &self,
        local: &str,
    ) -> Option<&TypeOrigin> {
        self.types.get(local)
    }

    /// 名字是否因**多个模块同名导入**而失去来源
    pub fn is_conflicted(
        &self,
        local: &str,
    ) -> bool {
        self.conflicted.contains(local)
    }

    /// 方法身份的**唯一解析实现**（D-1/D-2/D-5/D-7）：写出形态 → 方法路径。
    ///
    /// 每一档都是单一事实源，不做优先级推理：
    /// 1. 命名空间限定写法（`lib.Temp` / `l.Temp`）：前缀经命名空间表解析为
    ///    模块键（`use lib as l` 的别名在此还原），后一段是类型裸名；
    /// 2. 本表给出的类型（本单元声明 / 花括号导入 / std 基线）：提供模块 + 定义侧裸名；
    /// 3. 其余（内建类型、类型参数、未知名）：无模块归属（D-1 的 `""`，投影即
    ///    `Type.method`，与既有表键逐字相同）；
    /// 4. 名字层歧义（多个模块带进同一裸名）或前缀解析不到：**无身份**——调用点因此
    ///    不产决议，保持今天的行为（D-7 不扩大拒绝面）。
    ///
    /// 全仓只有这一处把「写出来的类型名」变成身份：`TypeEnvironment` 与表达式
    /// 推断器都委托到这里，不存在第二份判定。
    pub fn method_key(
        &self,
        written_type: &str,
        method: &str,
    ) -> Option<DefPath> {
        if written_type.is_empty() || method.is_empty() {
            return None;
        }
        if let Some((prefix, rest)) = written_type.split_once('.') {
            // 类型名是标识符：限定写法只剩「前缀 + 一段」
            if rest.is_empty() || rest.contains('.') {
                return None;
            }
            let module = self.namespace_module(prefix)?;
            return Some(DefPath::method(module, rest, method));
        }
        if let Some(origin) = self.type_origin(written_type) {
            return Some(DefPath::method(&origin.module, &origin.target, method));
        }
        if self.is_conflicted(written_type) {
            return None;
        }
        Some(DefPath::method("", written_type, method))
    }

    /// 类型名 → **类型路径**（`{module}.{Type}`）：绑定登记时记类型身份用（D-6）。
    ///
    /// 歧义名 / 未知名 → None（宁可不记身份，也不用猜的来源）。
    pub fn type_path(
        &self,
        written: &str,
    ) -> Option<DefPath> {
        if let Some((prefix, rest)) = written.split_once('.') {
            if rest.is_empty() || rest.contains('.') {
                return None;
            }
            let module = self.namespace_module(prefix)?;
            return Some(DefPath::item(module, rest));
        }
        let origin = self.type_origin(written)?;
        Some(DefPath::item(&origin.module, &origin.target))
    }

    /// **唯一有上下文的 AST 类型 → MonoType 入口**（L3-1b1a）。
    ///
    /// 本批与 `MonoType::from` 同义（身份解析留到下一批）；有上下文的调用点
    /// 必须走这里，env-free 的点保持 `MonoType::from`，不得再写第二套转换。
    pub fn mono_from_annotation(
        &self,
        ty: &crate::ast::Type,
    ) -> crate::frontend::core::types::MonoType {
        crate::frontend::core::types::MonoType::from(ty.clone())
    }

    /// 命名空间前缀（`use lib` 的 `lib` / `use lib as l` 的 `l`）→ 模块键
    pub fn namespace_module(
        &self,
        prefix: &str,
    ) -> Option<&str> {
        self.namespaces.get(prefix).map(String::as_str)
    }
}
