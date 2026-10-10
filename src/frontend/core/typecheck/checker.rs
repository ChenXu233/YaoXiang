//! 类型检查器模块
//!
//! 包含 TypeChecker 的完整实现

use std::collections::{HashMap, HashSet};

use crate::frontend::core::parser::ast::{
    classify_generic_params, Expr, GenericParamKind, Module, Param, TypeBodyItem,
};
use crate::frontend::core::types::{MonoType, PolyType, TraitTable};
use crate::frontend::core::types::const_data::{ConstExpr, ConstValue, BinOp, ConstKind, ConstVarDef};
use crate::frontend::core::types::eval::const_eval::{ConstFunction, convert_expr_to_const_expr};
use crate::frontend::core::typecheck::predicate_resolver::PredicateResolver;
use crate::frontend::core::typecheck::proof::verdict::ProofResult;
use crate::frontend::core::typecheck::proof::dep_graph::{
    constraint_is_terminates, refined_free_vars, RefinedScopeUnit,
};
use crate::frontend::core::types::eval::dependent_types::DependentTypeEnv;
use crate::std::StdModule;

use super::inference;
use super::semantic_db;
use crate::frontend::core::spawn;
use super::types::TypeCheckResult;
use super::environment::TypeEnvironment;
use super::environment::ImplementationProof;
use super::{add_builtin_types, add_std_traits, add_native_function_types};
use super::Diagnostic;
use crate::util::diagnostic::ErrorCodeDefinition;

/// 类型检查器
///
/// 负责模块级类型检查编排，协调前置收集和函数体检查
pub struct TypeChecker {
    /// 当前环境
    env: TypeEnvironment,
    /// 本模块函数的声明期类型参数名（名字 → 按声明序的参数名）。
    /// 在签名收集阶段填充（`collect_signatures`），供 `embedded_std_module_info`
    /// 随导出暴露——跨模块单态化需要被调函数的声明名。
    declared_fn_type_params: HashMap<String, Vec<String>>,
    /// 语句检查器
    body_checker: Option<inference::StatementChecker>,
    /// RFC-014 §项目模式：vendor 根目录（`<project>/.yaoxiang/vendor`）。
    /// 有值时转发给 body_checker——`use` 缺依赖包的 E5001 追加 install 提示。
    vendor_root: Option<std::path::PathBuf>,
    /// 语义信息收集（typecheck 阶段同时产出）
    semantic_db: semantic_db::SemanticDB,
    /// 依赖类型环境（类型族注册与查找）
    pub dependent_type_env: DependentTypeEnv,
    /// 用户模块命名空间别名表（别名 → 模块限定键），由 `use lib` / `use lib as l` 整体导入登记。
    /// 模块解析归 typecheck 所有：IR 生成直接消费此表，不再自行从 AST 重新推导。
    module_namespaces: HashMap<String, String>,
    /// RFC-004: 类型体绑定的待登记队列（类型名, 绑定, span）。
    /// pass1 收集类型定义，此时被绑函数可能尚未注册（pass2 收集签名），
    /// 故 External/DefaultExternal 延迟到 pass2 之后统一登记。
    pending_body_bindings: Vec<(
        String,
        crate::frontend::core::parser::ast::TypeBodyBinding,
        crate::util::span::Span,
    )>,
    /// RFC-011a: 已注册类型定义的 AST 体项（接口展开需要原始应用项；
    /// MonoType 转换会丢弃 Expr/Binding 项，故单独留存）。
    type_definition_bodies: HashMap<String, Vec<crate::frontend::core::parser::ast::TypeBodyItem>>,
    /// RFC-011a: 类型体中的接口实例化待决记录（阶段 2 延迟完整性检查）。
    pending_interface_instantiations: Vec<PendingInterfaceInstantiation>,
    /// RFC-011a §3: 已声明方法签名 (类型名, 方法名) -> 签名。
    /// 同签名重复声明 = 覆盖 → E1100；不同签名 = 重载 → 放行。
    declared_methods: HashMap<(String, String), MonoType>,
    /// #321 W1003: 导入的本地名 → use 语句位置（pass2 use elaboration 登记，
    /// 供模块尾统一判定未使用导入；重复登记由发射处去重）。
    imported_names: Vec<(String, crate::util::span::Span)>,
    /// #321 W1003: 导入名监视集（本地名 → 报告名）。整体导入的导出成员
    /// （如 `use std.io` 后裸调 `print`）同样映射到模块别名，命中即视为导入已使用。
    import_watch: HashMap<String, String>,
    /// #321 W1003: pass2 解析路径标记的已使用名（类型/方法外部绑定等
    /// 不经 body_checker Var 推断臂的引用点），发射时与 body 侧并集。
    import_used: HashSet<String>,
    /// T3：pass2 收集的顶层值绑定占位类型（名字 → 类型）。
    ///
    /// 仅用于**函数体内的前向引用解析**——注入 body_checker 使后置绑定名可见。
    /// 不写入 `env.vars`：那里会被当作最终 `bindings` 输出，而占位类型
    /// （TypeVar）会遮蔽 pass3 推断出的真实类型。
    early_value_bindings: HashMap<String, PolyType>,
    /// #358：已声明的顶层函数名 → 签名（用于区分重复定义与合法重载）。
    declared_top_fns: HashMap<String, String>,
}

/// 取顶层语句的函数**名与签名**（值/HOF 绑定也算函数）。
///
/// 签名用字符串形态：只用于等值比较，不需要结构化。
impl TypeChecker {
    fn top_level_fn_signature(
        &self,
        stmt: &crate::frontend::core::parser::ast::Stmt,
    ) -> Option<(String, String)> {
        use crate::frontend::core::parser::ast::{Expr, StmtKind};
        let StmtKind::Assign {
            target,
            type_annotation,
            signature_params,
            value,
            ..
        } = &stmt.kind
        else {
            return None;
        };
        let Expr::Var(name, _) = target.as_ref() else {
            return None;
        };
        let is_fn = value.as_ref().is_some_and(|v| {
            matches!(v.as_ref(), Expr::Lambda { .. })
                || Expr::block_binding_is_function(type_annotation.as_ref(), Some(v.as_ref()))
        });
        if !is_fn {
            return None;
        }
        // 签名 = 参数类型（含返回位）。
        //
        // 优先取 `signature_params`（RFC-010 新语法把参数类型放在这里），
        // 它区分 `Array(Int)` 与 `Array(Float)`；无参数时退到类型注解自身。
        // 注意：不能用 `{:?}` 格式化 `Type`——它含 `Span`，同一签名在不同
        // 位置会得到不同字符串。用 `MonoType::type_name()` 取纯类型名。
        let type_name_of = |t: &crate::frontend::core::parser::ast::Type| {
            crate::frontend::core::types::MonoType::from(t.clone()).type_name()
        };
        let sig = if !signature_params.is_empty() {
            signature_params
                .iter()
                .map(|p| {
                    p.ty.as_ref()
                        .map(type_name_of)
                        .unwrap_or_else(|| "_".to_string())
                })
                .collect::<Vec<_>>()
                .join(",")
        } else {
            type_annotation
                .as_ref()
                .map(type_name_of)
                .unwrap_or_default()
        };
        Some((name.clone(), sig))
    }
}

/// RFC-011a: 类型体应用项 `Animal(Dog)` 的待决接口实例化。
struct PendingInterfaceInstantiation {
    impl_type: String,
    interface_name: String,
    args: Vec<crate::frontend::core::parser::ast::Type>,
    span: crate::util::span::Span,
}

impl TypeChecker {
    /// 创建新的类型检查器
    pub fn new(module_name: &str) -> Self {
        let mut env = TypeEnvironment::new_with_module(module_name.to_string());
        add_builtin_types(&mut env);
        add_std_traits(&mut env);
        // RFC-011b: 运算符接口声明（接口构造器形态）+ 核心默认登记
        super::operator_interfaces::register_interface_defs(&mut env);
        super::operator_interfaces::register_native_entries(&mut env);
        // RFC-011b 阶段 2: `?` 传播接口（四方法）
        super::operator_interfaces::register_try_interface_def(&mut env);
        // #391：std 签名是编译器静态资产，注册期对畸形签名硬失败——
        // 签名静态编译进二进制，坏签名等价于编译器自身 invariant 被破坏，
        // 任何测试/运行路径都必须立即大声失败，绝不静默退化。
        add_native_function_types(&mut env)
            .unwrap_or_else(|d| panic!("std native 签名注册失败: {} {}", d.code, d.message));
        Self::register_builtin_container_defs(&mut env);

        // std 注册表的方法绑定注入（与 orchestrator 逐文件注入同一规则）：
        // 嵌入 yx std 模块的方法（`Result.is_failure` 等）在单文件模式下
        // 也按方法调用解析。
        for (key, ty) in env.module_registry.all_method_bindings() {
            env.method_bindings.insert(key, ty);
        }

        // 注册预定义的 const 函数
        Self::register_predefined_const_functions(&mut env);

        // 初始化依赖类型环境并通过 std::assert 注册类型族
        let mut dependent_type_env = DependentTypeEnv::new();
        crate::std::assert::AssertModule.register_type_families(&mut dependent_type_env);

        Self {
            env,
            declared_fn_type_params: HashMap::new(),
            body_checker: None,
            vendor_root: None,
            semantic_db: semantic_db::SemanticDB::new(),
            dependent_type_env,
            module_namespaces: HashMap::new(),
            pending_body_bindings: Vec::new(),
            type_definition_bodies: HashMap::new(),
            pending_interface_instantiations: Vec::new(),
            declared_methods: HashMap::new(),
            imported_names: Vec::new(),
            import_watch: HashMap::new(),
            import_used: HashSet::new(),
            early_value_bindings: HashMap::new(),
            declared_top_fns: HashMap::new(),
        }
    }

    /// RFC-011a: 已通过的接口实现证明（编译期，运行时擦除）。
    /// LSP/阶段 3 动态分发的类型收集据此枚举某接口的全部实现类型。
    pub fn implementation_proofs(&self) -> &[ImplementationProof] {
        &self.env.implementation_proofs
    }

    /// RFC-011b: 接口实现登记表（带类型实参维度）。
    /// 运算符查询与约束求解的唯一判据。
    pub fn interface_impl_registry(
        &self
    ) -> &HashMap<String, Vec<super::environment::InterfaceImplEntry>> {
        &self.env.interface_impl_registry
    }

    /// 注册预定义的 const 函数
    /// 这些函数用于值依赖类型的编译期求值
    /// 注册内置泛型容器的类型构造器定义（RFC-011）。
    ///
    /// `Vec(T)` / `Array(T, N)` 是**内建**类型（与 `Int`/`String` 同类），
    /// 但它们带类型参数，且值位置可当构造器用（`Vec(Int)()`、`Vec(Int)(1,2,3)`）。
    /// 此前它们既不在 `env.types`（标量表）也不在 `generic_type_defs`
    ///（只由用户 `Type` 定义填充）——于是值位置的 `Vec(Int)()` 报
    /// E1001 unknown variable 'Vec'（D6.2），类型推断也拿不到构造器形参
    /// （`Vec(Int)(1,2,3)` 不写注解时 `func_ty` 悬空成 `t49`）。
    ///
    /// 构造器体的字段表只承担一个作用：让实例化能展开成 `Generic{name,args}`。
    fn register_builtin_container_defs(env: &mut TypeEnvironment) {
        use crate::frontend::core::typecheck::environment::GenericTypeDef;
        use crate::frontend::core::types::mono::PolyType;
        // `Vec(T)`：运行时长度的最小地基。无具名字段（长度是内建属性）。
        for (name, params) in [("Vec", vec!["T"]), ("Array", vec!["T", "N"])] {
            let type_binders: Vec<crate::frontend::core::types::TypeVar> = (0..params.len())
                .map(crate::frontend::core::types::TypeVar::new)
                .collect();
            let body = MonoType::Generic {
                name: name.to_string(),
                args: params
                    .iter()
                    .map(|p| MonoType::TypeRef(p.to_string()))
                    .collect(),
            };
            let def = GenericTypeDef {
                poly: PolyType {
                    type_binders,
                    const_binders: Vec::new(),
                    body,
                },
                type_param_names: params.iter().map(|p| p.to_string()).collect(),
            };
            env.add_generic_type_def(name.to_string(), def);
        }
    }

    fn register_predefined_const_functions(env: &mut TypeEnvironment) {
        // 注册 factorial 函数
        let factorial = ConstFunction::new(
            "factorial".to_string(),
            vec!["n".to_string()],
            ConstExpr::If {
                condition: Box::new(ConstExpr::BinOp {
                    op: BinOp::Le,
                    left: Box::new(ConstExpr::NamedVar("n".to_string())),
                    right: Box::new(ConstExpr::Lit(ConstValue::Int(1))),
                }),
                then_branch: Box::new(ConstExpr::Lit(ConstValue::Int(1))),
                else_branch: Box::new(ConstExpr::BinOp {
                    op: BinOp::Mul,
                    left: Box::new(ConstExpr::NamedVar("n".to_string())),
                    right: Box::new(ConstExpr::Call {
                        func: "factorial".to_string(),
                        args: vec![ConstExpr::BinOp {
                            op: BinOp::Sub,
                            left: Box::new(ConstExpr::NamedVar("n".to_string())),
                            right: Box::new(ConstExpr::Lit(ConstValue::Int(1))),
                        }],
                    }),
                }),
            },
        );
        env.add_const_function("factorial".to_string(), factorial);

        // 注册 fibonacci 函数
        let fibonacci = ConstFunction::new(
            "fibonacci".to_string(),
            vec!["n".to_string()],
            ConstExpr::If {
                condition: Box::new(ConstExpr::BinOp {
                    op: BinOp::Le,
                    left: Box::new(ConstExpr::NamedVar("n".to_string())),
                    right: Box::new(ConstExpr::Lit(ConstValue::Int(1))),
                }),
                then_branch: Box::new(ConstExpr::NamedVar("n".to_string())),
                else_branch: Box::new(ConstExpr::BinOp {
                    op: BinOp::Add,
                    left: Box::new(ConstExpr::Call {
                        func: "fibonacci".to_string(),
                        args: vec![ConstExpr::BinOp {
                            op: BinOp::Sub,
                            left: Box::new(ConstExpr::NamedVar("n".to_string())),
                            right: Box::new(ConstExpr::Lit(ConstValue::Int(1))),
                        }],
                    }),
                    right: Box::new(ConstExpr::Call {
                        func: "fibonacci".to_string(),
                        args: vec![ConstExpr::BinOp {
                            op: BinOp::Sub,
                            left: Box::new(ConstExpr::NamedVar("n".to_string())),
                            right: Box::new(ConstExpr::Lit(ConstValue::Int(2))),
                        }],
                    }),
                }),
            },
        );
        env.add_const_function("fibonacci".to_string(), fibonacci);
    }

    /// 获取环境引用
    /// 本模块声明的**函数类型参数名表**快照（名字 → 按声明序的参数名）。
    ///
    /// 跨模块调用的单态化需要被调函数的声明名（否则签名里的 `TypeRef("A")`
    /// 无人绑定）；`embedded_std_module_info` 借此把名字随导出一并暴露。
    pub fn generic_fn_type_params_snapshot(&self) -> HashMap<String, Vec<String>> {
        // 签名收集阶段（`collect_signatures`）填的是 `declared_fn_type_params`；
        // 函数体检查后 `body_checker` 里也有一份更全的。两处合并，后者优先。
        let mut out = self.declared_fn_type_params.clone();
        if let Some(c) = self.body_checker.as_ref() {
            out.extend(c.generic_fn_type_params_snapshot());
        }
        out
    }

    pub fn env(&mut self) -> &mut TypeEnvironment {
        &mut self.env
    }

    /// 注入 vendor 根目录（RFC-014 §项目模式）。
    ///
    /// 须在 `check_module` 前调用；`check_module` 初始化 body_checker 时
    /// 转发，`use` 缺依赖包的 E5001 据此追加 `yaoxiang install` 提示。
    pub fn set_vendor_root(
        &mut self,
        root: std::path::PathBuf,
    ) {
        self.vendor_root = Some(root);
    }

    /// 获取模块名称
    pub fn module_name(&self) -> &str {
        &self.env.module_name
    }

    /// 添加错误
    fn add_error(
        &mut self,
        error: Diagnostic,
    ) {
        self.env.errors.add_error(error);
    }

    /// 名是否是「返回 `Type` 的函数」（精化谓词 / 证明函数）或是内置谓词 `Terminates`。
    ///
    /// 这类名字在类型位置合法：`val: IsPositive(5)`、`acc: Terminates(n)`——
    /// 其实参是**值表达式/度量**，不是类型引用（RFC-028 §6.9、RFC-027a）。
    /// 判定与 `body_checker` 的 `proof_fn_bases` 同源（`Fn` 且返回 `MetaType`）。
    fn is_type_returning_fn(
        &self,
        name: &str,
    ) -> bool {
        if name == "Terminates" {
            return true;
        }
        self.env.vars.get(name).is_some_and(|poly| {
            matches!(
                &poly.body,
                MonoType::Fn { return_type, .. }
                    if matches!(return_type.as_ref(), MonoType::MetaType { .. })
            )
        })
    }

    /// 从一条语句里抽出所有类型注解并校验（#371/#372）。
    ///
    /// 覆盖位置：绑定/函数的类型注解（含返回位）、各层形参、类型定义体。
    /// 泛型参数名（`(A: Type)` 形态）先剔出来置顶跳过。
    fn check_annotation_type_names(
        &mut self,
        stmt: &crate::frontend::core::parser::ast::Stmt,
    ) {
        self.check_annotation_type_names_with(stmt, &[]);
    }

    /// `check_annotation_type_names` 的带继承参数版（B6）。
    ///
    /// `inherited`：外层作用域已声明的类型参数名（如外层函数的 `T`/`N`）。
    /// 函数体内的绑定/lambda 形参注解可以引用它们，不能当未知名报错。
    fn check_annotation_type_names_with(
        &mut self,
        stmt: &crate::frontend::core::parser::ast::Stmt,
        inherited: &[String],
    ) {
        use crate::frontend::core::parser::ast::{StmtKind, Type as T};

        let (type_annotation, signature_params, definition) = match &stmt.kind {
            StmtKind::Assign {
                type_annotation,
                signature_params,
                ..
            } => (type_annotation.as_ref(), signature_params.as_slice(), None),
            StmtKind::TypeDefinition {
                signature_params,
                definition,
                ..
            } => (None, signature_params.as_slice(), Some(definition)),
            _ => return,
        };

        // 签名参数名：**类型参数**（`A: Type`）与**编译期值参数**（`N: Int`，靠
        // `(n: N)` 这种「形参被当类型用」的形态识别）都是声明处绑定的局部名字，
        // 在类型位置出现时不算未知名，先行剔除。
        //
        // ⚠ 不能只看「有注解」：那会把普通值形参（`x: Int` 的 `x`）也算作类型名，
        // 于是 `f: (bogus: Int) -> bogus = ...` 的返回位 `bogus` 被当合法类型
        // 放行，该函数的返回类型检查形同虚设（B5）。
        let generic_names: Vec<String> = {
            use crate::frontend::core::parser::ast::name_used_as_type_in;
            let mut names: Vec<String> = inherited.to_vec();
            for p in signature_params {
                let Some(ty) = p.ty.as_ref() else { continue };
                // 类型参数（`A: Type` 形态）直接计入
                if matches!(ty, T::MetaType { .. }) {
                    names.push(p.name.clone());
                    continue;
                }
                // 值参数：其名在**签名注解或类型定义体的类型位**被引用时才算类型名
                // （`(N: Int) -> (n: N) -> Int` 的内层 `N`；
                //   `SafeArray: (T: Type, N: Int) -> Type = { data: Array(T, N) }` 的 `N`）
                let referenced = signature_params.iter().any(|q| {
                    q.ty.as_ref()
                        .is_some_and(|t| name_used_as_type_in(&p.name, t))
                }) || type_annotation
                    .as_ref()
                    .is_some_and(|t| name_used_as_type_in(&p.name, t))
                    || definition
                        .as_ref()
                        .is_some_and(|t| name_used_as_type_in(&p.name, t));
                if referenced {
                    names.push(p.name.clone());
                }
            }
            names
        };

        // 各层形参注解（`A: Type` 形态的类型参数位自身不引用类型，跳过）
        for p in signature_params {
            if let Some(ty) = p.ty.as_ref() {
                if !matches!(ty, T::MetaType { .. }) {
                    self.check_type_names_in(ty, &generic_names);
                }
            }
        }
        // 顶层注解（含函数返回位）
        if let Some(ty) = type_annotation {
            self.check_type_names_in(ty, &generic_names);
        }
        // 类型定义体（字段类型就是 #372 的另一半）。
        //
        // 体内声明的**成员名**（RFC-011 §3.1 关联类型，如 `IteratorType: Iterator(Item)`）
        // 在本体内是合法的类型名：`iter: (Self) -> IteratorType` 要能引用它。
        // 这是体级作用域——不得溢出到其他类型体或形参（见 `collect_body_member_names`）。
        if let Some(def) = definition {
            let mut in_body = generic_names.clone();
            Self::collect_member_names(def, &mut in_body);
            self.check_type_names_in(def, &in_body);
        }

        // B6：体递归——本条语句的函数体/块/分支/循环里的绑定与 lambda
        // 形参注解同样受校验，且可引用外层已声明的类型参数（`inherited` +
        // 本条签名自身的 generic_names 向下传递）。
        //
        // 此前只扫 module.items，函数体内的错拼完全逃逸
        // （`main = { g: (a: BogusType) -> Int = ... }` 编译通过）。
        let mut nested_inherited = inherited.to_vec();
        nested_inherited.extend(generic_names.iter().cloned());
        self.check_nested_annotations(stmt, &nested_inherited);
    }

    /// 下钻语句的嵌套作用域校验注解（B6）。只下钻承载函数体/块的位置。
    fn check_nested_annotations(
        &mut self,
        stmt: &crate::frontend::core::parser::ast::Stmt,
        inherited: &[String],
    ) {
        use crate::frontend::core::parser::ast::StmtKind;
        match &stmt.kind {
            StmtKind::Assign { value, .. } => {
                if let Some(e) = value.as_deref() {
                    self.check_nested_annotations_in_expr(e, inherited);
                }
            }
            StmtKind::Expr(e) => self.check_nested_annotations_in_expr(e, inherited),
            StmtKind::Return(Some(e)) => self.check_nested_annotations_in_expr(e, inherited),
            StmtKind::If {
                then_branch,
                else_if_branches,
                else_branch,
                ..
            } => {
                for s in &then_branch.stmts {
                    self.check_annotation_type_names_with(s, inherited);
                }
                for (_, body) in else_if_branches {
                    for s in &body.stmts {
                        self.check_annotation_type_names_with(s, inherited);
                    }
                }
                if let Some(b) = else_branch {
                    for s in &b.stmts {
                        self.check_annotation_type_names_with(s, inherited);
                    }
                }
            }
            _ => {}
        }
    }

    /// 表达式内的嵌套作用域下钻（块 / lambda / 循环 / 分支）。
    fn check_nested_annotations_in_expr(
        &mut self,
        expr: &crate::frontend::core::parser::ast::Expr,
        inherited: &[String],
    ) {
        use crate::frontend::core::parser::ast::Expr;
        match expr {
            Expr::Block(b) => {
                for s in &b.stmts {
                    self.check_annotation_type_names_with(s, inherited);
                }
            }
            Expr::While {
                condition, body, ..
            } => {
                self.check_nested_annotations_in_expr(condition, inherited);
                for s in &body.stmts {
                    self.check_annotation_type_names_with(s, inherited);
                }
            }
            Expr::For { iterable, body, .. } => {
                self.check_nested_annotations_in_expr(iterable, inherited);
                for s in &body.stmts {
                    self.check_annotation_type_names_with(s, inherited);
                }
            }
            Expr::If {
                condition,
                then_branch,
                else_if_branches,
                else_branch,
                ..
            } => {
                self.check_nested_annotations_in_expr(condition, inherited);
                for s in &then_branch.stmts {
                    self.check_annotation_type_names_with(s, inherited);
                }
                for (c, body) in else_if_branches {
                    self.check_nested_annotations_in_expr(c, inherited);
                    for s in &body.stmts {
                        self.check_annotation_type_names_with(s, inherited);
                    }
                }
                if let Some(b) = else_branch {
                    for s in &b.stmts {
                        self.check_annotation_type_names_with(s, inherited);
                    }
                }
            }
            Expr::Lambda { body, .. } => {
                for s in &body.stmts {
                    self.check_annotation_type_names_with(s, inherited);
                }
            }
            _ => {}
        }
    }

    /// 收集类型体内声明的成员名（RFC-011 §3.1 关联类型）。
    ///
    /// 背景：类型体里的 `Name: Type` 在 AST 里**只有一种形态**
    /// （`TypeBodyItem::Field`），关联类型成员与运行时数据字段**同形**，无从区分。
    /// 因此这里把全部字段名都当作潜在的体内类型名——这是 AST 表达力所限，
    /// 不是偷懒（见下方「残留宽松」）。
    ///
    /// **作用域**：仅用于定义体自身的校验。调用方只把结果传给
    /// `check_type_names_in(def, ..)`，不会写入 `env`，所以不会溢出到
    /// 别的类型体、形参或返回值。
    ///
    /// 残留宽松：`S: Type = { a: Int, b: a }` 里的 `a` 会被当作合法类型名。
    /// 这拦不了——`a` 确实被引用，而「引用」正是 RFC-011 判定成员的依据；
    /// 要根治需给关联类型一个**区别于数据字段的语法形态**（如 `type Name: T`），
    /// 属语言设计变更，不在本修复范围。
    fn collect_member_names(
        ty: &crate::frontend::core::parser::ast::Type,
        out: &mut Vec<String>,
    ) {
        use crate::frontend::core::parser::ast::{Type as T, TypeBodyItem};
        match ty {
            T::Struct { body } => {
                for item in body {
                    if let TypeBodyItem::Field(f) = item {
                        out.push(f.name.clone());
                    }
                }
            }
            T::NamedStruct { fields, .. } => {
                for f in fields {
                    out.push(f.name.clone());
                }
            }
            _ => {}
        }
    }

    /// 若表达式是索引形态（`Name[...]`），返回被索引的名字与源码位置（#371）。
    ///
    /// 用于把 `List[Int]` 这类**错写法**变成一条指向注解本身的诊断：
    /// 它在类型位置无合法解释（编译期值参数不用下标），而默认路径会把它
    /// 静默变成一个名叫 `<const-expr>` 的占位类型，直到使用处才报出
    /// 完全无关的错误。
    fn index_expr_head(
        expr: &crate::frontend::core::parser::ast::Expr
    ) -> Option<(String, crate::util::span::Span)> {
        use crate::frontend::core::parser::ast::Expr as E;
        match expr {
            E::Index { expr, .. } => match expr.as_ref() {
                E::Var(name, span) => Some((name.clone(), *span)),
                // 链式索引 `A[0][1]`：递归到最内层
                other => Self::index_expr_head(other),
            },
            _ => None,
        }
    }

    /// 校验注解引用的类型名是否都可解析（#371/#372）。
    ///
    /// 为何需要：`MonoType::from(ast::Type)` 对 `Type::Name` 不查表（直接包成
    /// `TypeRef(name)`）。于是形参/字段里错拼的类型名会静默成为一个「合法类型」，
    /// 而 solver 对两个不同的 `TypeRef` 判不等的分支根本走不到——**该参数上的
    /// 类型检查随之全部失效**（任意实参都能传）。
    ///
    /// 必须放在**所有注册完成之后**跑（见调用处）：`use` 导入的类型、接口、
    /// 泛型构造器、std 导出都陆续进 env，提前跑会把它们误判为未知。
    ///
    /// `generic_names` 是本签名的类型参数名（如 `(A: Type)` 里的 `A`）——
    /// 它们是声明处绑定的局部名字，不是可解析类型。
    fn check_type_names_in(
        &mut self,
        ty: &crate::frontend::core::parser::ast::Type,
        generic_names: &[String],
    ) {
        let mut unknown: Vec<(String, crate::util::span::Span, bool)> = Vec::new();
        Self::collect_unknown_type_names(ty, generic_names, &self.env, &mut unknown);
        for (name, span, was_bracket) in unknown {
            // 值空间兼容（合法形态三类）：
            //
            // 1. **编译期值函数**：`Array(Int, factorial(5))` 的 `factorial`。
            // 2. **精化谓词 / 证明函数**：`IsPositive(5)`、`Terminates(n)`——
            //    其类型位写的是「返回 Type 的函数名（类型宇宙）或内置谓词」，
            //    实参是值表达式或度量。 RFC-027 §6.9 / RFC-027a。
            // 3. **顶层值绑定**：pass2 只进 `early_value_bindings`（pass3 才入 env），
            //    模块级依赖精化 `mut s: SumUpTo(n, 6)` 的 `n` 由此容错（#379）。
            //
            // ⚠ 不放行「任何已声明值」（`env.vars` 通查）：那会让
            // `f: (x: println) -> Int` 通过（`println` 是值位置的函数名），
            // 该形参的类型检查形同虚设（B5）。#379 需要的是**用户顶层绑定**，
            // 由 `early_value_bindings` 精确覆盖，无需放宽到全量值的并集。
            if self.env.is_const_function(&name)
                || self.is_type_returning_fn(&name)
                || self.early_value_bindings.contains_key(&name)
            {
                continue;
            }
            // 方括号写法（`List[Int]`）→ 专用码，直接给出 `List(...)` 的写法；
            // 其余未知名 → E1003。两者都不再让错误拖到使用处（#371）。
            let code = if was_bracket {
                ErrorCodeDefinition::bracket_in_type_position(&name)
            } else {
                ErrorCodeDefinition::unknown_type(&name)
            };
            self.add_error(code.at(span).build());
        }
    }

    /// 递归收集注解中无法解析的类型名（#372）。
    fn collect_unknown_type_names(
        ty: &crate::frontend::core::parser::ast::Type,
        generic_names: &[String],
        env: &crate::frontend::core::typecheck::environment::TypeEnvironment,
        out: &mut Vec<(String, crate::util::span::Span, bool)>,
    ) {
        use crate::frontend::core::parser::ast::Type as T;
        match ty {
            T::Name { name, span } => {
                if !generic_names.iter().any(|g| g == name) && !env.resolves_type_name(name) {
                    out.push((name.clone(), *span, false));
                }
            }
            T::Generic {
                name,
                name_span,
                args,
            } => {
                if !generic_names.iter().any(|g| g == name) && !env.resolves_type_name(name) {
                    out.push((name.clone(), *name_span, false));
                }
                // 精化谓词 / 证明函数的**实参位是值**（`Terminates(n)` 的测度 n、
                // `IsPositive(5)` 的 5），不是类型引用——不下钻，否则测度里的
                // 局部变量名会被误报为未知名类型（RFC-027a）。
                if is_predicate_head(name, env) {
                    return;
                }
                for a in args {
                    Self::collect_unknown_type_names(a, generic_names, env, out);
                }
            }
            T::Fn {
                params,
                return_type,
            } => {
                for p in params {
                    Self::collect_unknown_type_names(p, generic_names, env, out);
                }
                Self::collect_unknown_type_names(return_type, generic_names, env, out);
            }
            T::Tuple(items) | T::Sum(items) => {
                for i in items {
                    Self::collect_unknown_type_names(i, generic_names, env, out);
                }
            }
            T::Option(inner) | T::Ptr(inner) => {
                Self::collect_unknown_type_names(inner, generic_names, env, out);
            }
            T::Result(ok, err) => {
                Self::collect_unknown_type_names(ok, generic_names, env, out);
                Self::collect_unknown_type_names(err, generic_names, env, out);
            }
            T::Ref { inner, .. } => {
                Self::collect_unknown_type_names(inner, generic_names, env, out);
            }
            T::MetaType { args, .. } => {
                for a in args {
                    Self::collect_unknown_type_names(a, generic_names, env, out);
                }
            }
            T::Literal { base_type, .. } => {
                Self::collect_unknown_type_names(base_type, generic_names, env, out);
            }
            T::Struct { body } => {
                for item in body {
                    if let crate::frontend::core::parser::ast::TypeBodyItem::Field(f) = item {
                        Self::collect_unknown_type_names(&f.ty, generic_names, env, out);
                    }
                }
            }
            T::NamedStruct { fields, .. } => {
                for f in fields {
                    Self::collect_unknown_type_names(&f.ty, generic_names, env, out);
                }
            }
            T::AssocType {
                host_type,
                assoc_args,
                ..
            } => {
                Self::collect_unknown_type_names(host_type, generic_names, env, out);
                for a in assoc_args {
                    Self::collect_unknown_type_names(a, generic_names, env, out);
                }
            }
            T::Union(members) => {
                for (_, m) in members {
                    if let Some(m) = m {
                        Self::collect_unknown_type_names(m, generic_names, env, out);
                    }
                }
            }
            // ConstExpr（RFC-027 编译期值参数）：合法形态是**值表达式**
            // （`Assert(N > 0)`、`Array(Int, factorial(5))`）。
            //
            // 但 `List[Int]` 这类**误解**也会落到这里：类型位置的标识符后跟 `[`
            // 会被当索引表达式，于是注解变成一个「名字叫 `<const-expr>` 的类型」。
            // 那是个凭空造出的占位类型，谁也认不出（#371）。
            //
            // 索引形态的表达式在**类型位置**没有任何合法解释（编译期值参数
            // 不用下标），所以一律报未知名——指向被索引的名字，并给出正确写法。
            T::ConstExpr(expr) => {
                if let Some((name, span)) = Self::index_expr_head(expr) {
                    // 下标形态在**类型位置**无任何合法解释（编译期值参数不用下标），
                    // 一律报专用码，指向注解并给出正确写法。
                    out.push((name, span, true));
                }
            }
            // 括号类型、Range、字面量类型、通配等：不引用具名类型。
            _ => {}
        }
    }

    /// 检查是否有错误
    pub fn has_errors(&self) -> bool {
        self.env.errors.has_errors()
    }

    /// 添加变量绑定
    pub fn add_var(
        &mut self,
        name: String,
        poly: PolyType,
    ) {
        self.env.add_var(name, poly);
    }

    /// 获取错误列表
    pub fn errors(&self) -> &[Diagnostic] {
        self.env.errors.errors()
    }

    /// 检查单个语句（委托给 StatementChecker）
    pub fn check_stmt(
        &mut self,
        stmt: &crate::frontend::core::parser::ast::Stmt,
    ) -> Result<(), Box<Diagnostic>> {
        self.body_checker_mut().check_stmt(stmt)
    }

    /// 检查整个模块
    ///
    /// 在收集模式下，将收集所有错误后统一返回。
    pub fn check_module(
        &mut self,
        module: &Module,
    ) -> TypeCheckResult {
        self.check_module_impl(module, false)
    }

    /// 检查整个模块（收集所有错误模式）
    ///
    /// 启用错误收集模式后，类型检查器会尽可能多地收集错误，
    /// 而不是在第一个错误处停止。适用于 LSP 诊断场景。
    pub fn check_module_collect_all(
        &mut self,
        module: &Module,
    ) -> TypeCheckResult {
        self.check_module_impl(module, true)
    }

    /// 仅收集模块的顶层签名（RFC-029 多文件编排）。
    ///
    /// 跑 pass1（类型定义）+ pass2（函数/绑定签名），**不检查函数体**。
    /// 用于在构建 Registry 时提取每个文件顶层绑定的真实 `MonoType`。
    /// 函数体里的跨文件引用留到完整三遍（pass3）才检查，彼时 Registry 已就绪。
    pub fn collect_signatures(
        &mut self,
        module: &Module,
    ) {
        // pass1: 类型定义
        for stmt in &module.items {
            // #324：模块级阶段挂当前语句 span，诊断自动获得位置
            let _module_span_guard = crate::util::diagnostic::push_current_span(stmt.span);
            if let crate::frontend::core::parser::ast::StmtKind::TypeDefinition {
                name,
                signature_params,
                definition,
                ..
            } = &stmt.kind
            {
                self.add_type_definition(name, definition, signature_params, stmt.span);
            }
            // RFC-010：`unsafe {}` 内的类型定义提升到本作用域（块外可用）
            let mut hoisted = Vec::new();
            self.collect_unsafe_type_defs(stmt, &mut hoisted);
            for (name, definition, sig, span) in hoisted {
                self.add_type_definition(&name, &definition, &sig, span);
            }
        }
        // pass2: 函数/绑定签名（使其可被前向引用）
        for stmt in &module.items {
            // #324：模块级阶段挂当前语句 span，诊断自动获得位置
            let _module_span_guard = crate::util::diagnostic::push_current_span(stmt.span);
            self.collect_function_signature(stmt);
        }
        // 记录本模块每个函数的**声明期类型参数名**（按声明序）。
        //
        // 放在签名收集阶段：`embedded_std_module_info` 只跑这一步（不跑函数体），
        // 却需要把名字随导出一并暴露——跨模块调用（`list.len(v)`）的单态化
        // 依赖它们绑定签名里的 `TypeRef("A")`。
        {
            use crate::frontend::core::parser::ast::StmtKind;
            for stmt in &module.items {
                if let StmtKind::Assign {
                    target,
                    signature_params,
                    ..
                } = &stmt.kind
                {
                    if let crate::frontend::core::parser::ast::Expr::Var(name, _) = target.as_ref()
                    {
                        // 参数位是 `(A: Type)` 形态即类型参数（同
                        // `StatementChecker` 的 classify_generic_params 判据）。
                        let names: Vec<String> = signature_params
                            .iter()
                            // 类型参数的语法形态是 `A: Type`，解析为 `Type::MetaType`
                            // （与 `StatementChecker::classify_generic_params` 判据一致）。
                            .filter(|p| {
                                matches!(
                                    p.ty.as_ref(),
                                    Some(crate::frontend::core::parser::ast::Type::MetaType { .. })
                                )
                            })
                            .map(|p| p.name.clone())
                            .collect();
                        if !names.is_empty() {
                            self.declared_fn_type_params
                                .entry(name.clone())
                                .or_insert(names);
                        }
                    }
                }
            }
        }

        // RFC-004: 函数签名就位后登记类型体绑定
        self.flush_pending_body_bindings();
    }

    /// 构建并安装函数体检查器（check_module_impl 与 #397 导出面收割共用）。
    ///
    /// 除检查器本体的各张表注入外，同步三样东西：
    /// ① `env.vars`（native 短名标注为「导入」，避免 E2010 误报）；
    /// ② T3 顶层值绑定占位（前向引用名字可见，pass3/收割执行到时覆盖）；
    /// ③ W1003 模块级导入名监视集。
    fn init_body_checker(
        &mut self,
        collect_all: bool,
    ) {
        let trait_table = self.env.trait_table.clone();
        let mut body_checker = inference::StatementChecker::new(
            self.env.solver(),
            None,
            self.dependent_type_env.clone(),
            trait_table,
        );
        // 设置 native 函数签名表
        body_checker.set_native_signatures(self.env.native_signatures.clone());
        // #387：native arity 区间表随签名表同源传递
        body_checker.set_native_arities(self.env.native_arity.clone());
        // 设置模块注册表，支持函数体/块作用域 use
        body_checker.set_module_registry(self.env.module_registry.clone());
        // #396：模块别名集合——模块成员缺失报 E1043 而非 E1042
        body_checker.set_module_aliases(self.env.module_aliases.clone());
        // RFC-014：vendor 根注入——缺依赖包的 E5001 追加 install 提示
        if let Some(vendor_root) = self.vendor_root.clone() {
            body_checker.set_vendor_root(vendor_root);
        }
        // 设置泛型类型定义模板表
        body_checker.set_generic_type_defs(self.env.generic_type_defs.clone());
        // 设置方法绑定表
        body_checker.set_method_bindings(self.env.method_bindings.clone());
        body_checker.method_overloads = self.env.method_overloads.clone();
        body_checker.method_overload_ir_names = self.env.method_overload_ir_names.clone();
        // RFC-011b: 接口实现登记表（finalize_interface_instantiations 已落表）
        body_checker.set_interface_impl_registry(self.env.interface_impl_registry.clone());
        body_checker.set_sum_types(self.env.sum_types.clone());
        // 设置类型定义表（用于 TypeRef → Struct 解析）
        let type_defs: HashMap<String, MonoType> = self
            .env
            .types
            .iter()
            .map(|(name, poly)| (name.clone(), poly.body.clone()))
            .collect();
        body_checker.set_type_defs(type_defs);
        // RFC-027 Phase 2.5: 构建证明函数基类型表
        let proof_fn_bases: HashMap<String, MonoType> = self
            .env
            .vars
            .iter()
            .filter_map(|(name, poly)| {
                if let MonoType::Fn {
                    params,
                    return_type,
                } = &poly.body
                {
                    if matches!(return_type.as_ref(), MonoType::MetaType { .. }) {
                        return params.first().map(|base| (name.clone(), base.clone()));
                    }
                }
                None
            })
            .collect();
        body_checker.set_proof_fn_bases(proof_fn_bases);
        // 如果启用收集模式，设置收集所有错误
        if collect_all {
            body_checker.set_collect_all_errors(true);
        }
        *self.body_checker_mut() = body_checker;

        // 将环境中的变量同步到 body_checker。
        //
        // 标注为「导入」：`env.vars` 里既有本文件的声明，也有
        // `add_native_function_types` 注入的 std native 短名（如 `ok`/`err`）。
        // 后者不是本文件的绑定——若不区分，用户写 `ok = ...` 会被 spec §4.3
        // 判定看成「重赋值不可变变量」→ 误报 E2010。
        // #414：use 登记的本地名（imported_names）同样按导入注入——pass3 的
        // 导入冲突判定（E5006/E5008）靠 `imported` 旗标区分「本文件绑定」与
        // 「本 use 语句的 pass2 预登记」，否则首条 use 就会撞上自己的同步产物。
        let use_local_names: Vec<String> =
            self.imported_names.iter().map(|(n, _)| n.clone()).collect();
        for (name, poly) in self.env.vars.clone() {
            let is_native = self.env.native_signatures.contains_key(&name);
            if is_native || use_local_names.iter().any(|n| n == &name) {
                self.body_checker_mut().add_imported_var(name, poly);
            } else {
                self.body_checker_mut().add_var(
                    name,
                    poly,
                    false,
                    crate::util::span::Span::default(),
                );
            }
        }

        // T3：把 pass2 收集的顶层值绑定占位也注入 body_checker，
        // 使函数体内对后置绑定的引用（前向引用）能解析到名字。
        // pass3 执行到该语句时会重新推断并覆盖占位类型。
        for (name, poly) in self.early_value_bindings.clone() {
            self.body_checker_mut().add_forward_declared_var(name, poly);
        }

        // #321 W1003：模块级导入名监视集注入（函数体级 use 由 process_use_stmt 自行登记）
        let module_import_watch = self.import_watch.clone();
        self.body_checker_mut()
            .set_import_watch(module_import_watch);
    }

    /// #397：导出面收割——对无标注的顶层值绑定跑与 pass3 相同的语句检查，
    /// 使推断出的具体类型落进 `env.vars`。此前这类绑定既不在签名表（函数
    /// 专属）也无标注可读，`extract_module_info` 的导出面直接丢弃它们，
    /// 导入方只能得到误导性报错。
    ///
    /// 收割前先在 body_checker 作用域里加工 `use`（块级 use 同一入口），
    /// 使初始化式引用其他模块名字的绑定（`use other;` 后 `v = other.greet()`）
    /// 也能推断。收割期间的诊断一律丢弃：这里只取类型，错误报告属于模块
    /// 自身的完整检查。
    pub fn harvest_untyped_value_bindings(
        &mut self,
        module: &crate::frontend::core::parser::ast::Module,
    ) {
        use crate::frontend::core::parser::ast::{Expr, StmtKind};
        // T3 占位先行：前向引用（`b = a + 1` 中 a 在后）名字可见，
        // 且 init_body_checker 的占位同步依赖这一步
        for stmt in &module.items {
            self.collect_value_binding_signature(stmt);
        }
        self.init_body_checker(false);

        // use 加工先行：块级 use 同一入口，模块别名进 body_checker 作用域
        for stmt in &module.items {
            if let StmtKind::Use {
                path,
                path_span,
                items,
                alias,
                item_aliases,
                ..
            } = &stmt.kind
            {
                let _ = self.body_checker_mut().process_use_stmt(
                    path,
                    *path_span,
                    items,
                    alias,
                    item_aliases,
                );
            }
        }

        // 收割目标：裸名、无标注、非函数、签名表里没有的顶层绑定
        let targets: Vec<&crate::frontend::core::parser::ast::Stmt> = module
            .items
            .iter()
            .filter(|stmt| match &stmt.kind {
                StmtKind::Assign {
                    target,
                    type_annotation,
                    value,
                    ..
                } => {
                    let Expr::Var(name, _) = target.as_ref() else {
                        return false;
                    };
                    if type_annotation.is_some() || self.env.vars.contains_key(name) {
                        return false;
                    }
                    let is_fn = value.as_ref().is_some_and(|v| {
                        matches!(v.as_ref(), Expr::Lambda { .. })
                            || Expr::block_binding_is_function(
                                type_annotation.as_ref(),
                                Some(v.as_ref()),
                            )
                    });
                    !is_fn
                }
                _ => false,
            })
            .collect();

        for stmt in targets {
            let _module_span_guard = crate::util::diagnostic::push_current_span(stmt.span);
            // 诊断丢弃：收割只取类型；错误由模块自身编译时的完整检查报告
            let _ = self.body_checker_mut().check_stmt(stmt);
            if let StmtKind::Assign { target, .. } = &stmt.kind {
                let Expr::Var(name, _) = target.as_ref() else {
                    continue;
                };
                // 推断类型落在 body_checker 的 global 链（#295：模块级绑定）
                if let Some(poly) = self.body_checker_global_type(name) {
                    self.env.vars.entry(name.clone()).or_insert(poly);
                }
            }
        }
    }

    /// body_checker global 链上某绑定的类型（收割读回用）。
    fn body_checker_global_type(
        &self,
        name: &str,
    ) -> Option<PolyType> {
        self.body_checker
            .as_ref()
            .and_then(|bc| bc.scope_globals().get(name))
            .map(|info| info.poly.clone())
    }

    /// 检查整个模块的内部实现
    fn check_module_impl(
        &mut self,
        module: &Module,
        collect_all: bool,
    ) -> TypeCheckResult {
        // 第零遍：登记编译期谓词定义（#377-3）。
        // 必须先于 pass1/pass2（签名与类型定义的精化标注要能解析到谓词体），
        // 且独立成遍，使谓词的定义与使用不受声明序约束。
        self.collect_predicate_defs(module);

        // 第一遍：收集所有类型定义
        for stmt in &module.items {
            // #324：模块级阶段挂当前语句 span，诊断自动获得位置
            let _module_span_guard = crate::util::diagnostic::push_current_span(stmt.span);
            if let crate::frontend::core::parser::ast::StmtKind::TypeDefinition {
                name,
                signature_params,
                definition,
                ..
            } = &stmt.kind
            {
                self.add_type_definition(name, definition, signature_params, stmt.span);
            }
            // RFC-010：`unsafe {}` 内的类型定义提升到本作用域（块外可用）
            let mut hoisted = Vec::new();
            self.collect_unsafe_type_defs(stmt, &mut hoisted);
            for (name, definition, sig, span) in hoisted {
                self.add_type_definition(&name, &definition, &sig, span);
            }
        }

        // 第二遍：收集所有函数签名（使其可被前向引用）
        for stmt in &module.items {
            // #324：模块级阶段挂当前语句 span，诊断自动获得位置
            let _module_span_guard = crate::util::diagnostic::push_current_span(stmt.span);
            self.collect_function_signature(stmt);
        }

        // 第二遍（补）：收集顶层值绑定的类型（T3 前向引用）。
        // 必须在函数签名之后（避免把函数当值绑定），且在函数体检查之前
        // （使函数体内的引用能解析到后置绑定）。
        for stmt in &module.items {
            let _module_span_guard = crate::util::diagnostic::push_current_span(stmt.span);
            self.collect_value_binding_signature(stmt);
        }

        // RFC-004: 函数签名就位后登记类型体绑定
        self.flush_pending_body_bindings();

        // RFC-011a 阶段2: 接口实例化完整性检查（Self 替换 + 签名匹配 + 实现证明）
        self.finalize_interface_instantiations();

        // 收集所有导出项

        // #371/#372：注解里的类型名校验。
        //
        // 放在此处（而非签名收集时）是因为 `use` 导入的类型、接口、泛型构造器、
        // std 导出都在前面各步才陆续进 env——提前校验会把 `Vec`/`Iterator`/
        // `Error` 这类合法名误判为未知（实测踩过）。
        //
        // 递归进函数体（B6）：此前只扫 module.items，函数体内同样的错拼
        // （`main = { g: (a: BogusType) -> Int = ... }`）完全逃逸校验。
        for stmt in &module.items {
            let _module_span_guard = crate::util::diagnostic::push_current_span(stmt.span);
            self.check_annotation_type_names(stmt);
        }

        // RFC-024: spawn 位置检查
        for err in spawn::placement::check_spawn_placement(module) {
            self.add_error(err);
        }

        // 初始化函数体检查器（构建 + 三张表同步，与导出面收割共用）
        self.init_body_checker(collect_all);

        // 第三遍：检查所有语句（包括函数体）
        for stmt in &module.items {
            // #324：模块级阶段挂当前语句 span，诊断自动获得位置
            let _module_span_guard = crate::util::diagnostic::push_current_span(stmt.span);
            if let Err(e) = self.body_checker_mut().check_stmt(stmt) {
                self.add_error(*e);
            }
        }

        // Phase 2.5: 检查精化类型绑定
        // 遍历所有语句，对变量绑定的精化类型执行证明检查
        let mut proof_calls = Vec::new();
        self.collect_refined_binding_checks(module, &mut proof_calls);

        // 收集 body_checker 中累积的错误（收集模式下产生的）
        if let Some(ref mut bc) = self.body_checker {
            for err in bc.drain_collected_errors() {
                self.env.errors.add_error(err);
            }
        }

        // RFC-027: 终止检查 — 在类型检查之后、约束求解之前运行
        // 分析循环和递归函数，自动证明终止性
        //
        // §7 验证模式门控：只有带精化标注的变量参与「度量变量」判定，裸 `while`
        // 不进验证模式。此处先收集精化变量名，再注入检查器。
        let refined_vars = self.collect_refined_var_names(module);
        // RFC-027a §2：显式测度（`Terminates(m)`）从 AST 提取后注入，供义务
        // 生成按被标注名查测度。与 `refined_vars` 同源（同一遍 AST 遍历）。
        let measures = self.collect_termination_measures(module);
        // RFC-027a §良基性：函数前置条件（形参精化代入后的约束）——良基性
        // `m >= 0` 的证据只能来自它（ℤ 上 `<` 不良基）。
        let param_assumptions = self.collect_param_refinements();
        // WBS 3.4.2：终止性义务未判定数（无求解器时 Unjudged）随结果一并取出——
        // T4「只判定并记录，不发射诊断」是静默通道，计数在 warnings 汇聚点发 W1081。
        let (term_results, unjudged_termination_obligations) = {
            #[allow(unused_mut)]
            let mut term_checker = super::layers::termination::TerminationChecker::new()
                .set_refined_vars(refined_vars)
                .set_measures(measures)
                .set_param_assumptions(param_assumptions);
            // RFC-027a T4（即 #377-1）：生产在此注入求解器后端。此前从不调用
            // 注入点，`self.solver` 恒为 `None`，SMT 相关路径在任何平台都不执行。
            // `default_solver()` 返回 `None`（初始化失败）时保持不注入——
            // wasm 自 #435 起同带 Z3（libz3.a 静态链接进产物）。
            if let Some(solver) = super::proof::smt::backend::default_solver() {
                term_checker = term_checker.with_solver_owned(solver);
            }
            let results = term_checker.check_module(module, self.env());
            let unjudged = super::layers::termination::count_unjudged_obligations(
                term_checker.measure_verdicts(),
                term_checker.well_founded_verdicts(),
            );
            (results, unjudged)
        };
        for result in term_results {
            match result.into_result() {
                Ok(()) => {} // 证明通过，无需诊断
                Err(diag) => self.add_error(diag),
            }
        }

        // RFC-027: 所有权检查 — 在终止检查之后、约束求解之前运行
        // 分析借用令牌冲突、Move/Drop/Clone/Mut 语义（RFC-009a §系统谓词清单）
        // #256：类型账本从推断层移交，供 Move/Dup 分类
        // #265：经证明管线入口 check_ownership，分支守卫注入假设栈
        let (release_plan, escaped_refs) = {
            let ledger = self
                .body_checker
                .as_ref()
                .map(|bc| bc.var_type_ledger().clone())
                .unwrap_or_default();
            // #335 G3 类型信息流接口：调用点所有权解析表随 ledger 一并移交
            let call_ownership = self
                .body_checker
                .as_ref()
                .map(|bc| bc.call_ownership.clone())
                .unwrap_or_default();
            let mut proof_ctx =
                crate::frontend::core::typecheck::proof::context::ProofContext::new(&self.env);
            let (ownership_results, plan, escaped_refs) = super::layers::ownership::check_ownership(
                &mut proof_ctx,
                module,
                &self.env,
                &ledger,
                &call_ownership,
            );
            for result in ownership_results {
                match result {
                    ProofResult::Proved => {}
                    ProofResult::Disproved(model) => {
                        // SpawnCycleViolation 的检测发生在全模块 ref 图上，模型无
                        // 语句级 span——挂模块 span 兜底，避免 spanless 构造在
                        // build() 走 debug panic / release 降级 E8001（显式 .at() 仍优先）
                        let _model_guard = crate::util::diagnostic::push_current_span(module.span);
                        self.add_error(model.into_diagnostic());
                    }
                    // 02-stage-contract §漏洞证据链第 8 步（WBS 3.2.1）：所有权层
                    // Unproven 不得被空臂吞掉——记账（proof_calls 上抛，与
                    // 5179/5318/5448 三处精化分支同一契约）+ 诊断（into_result
                    // 既定转换路径，不新造第二个 Unproven→Diagnostic 实现）。
                    result @ ProofResult::Unproven { .. } => {
                        if let ProofResult::Unproven {
                            proof_calls: calls, ..
                        } = &result
                        {
                            proof_calls.extend(calls.iter().cloned());
                        }
                        // 同 Disproved 臂：spanless 构造前挂模块 span 兜底（#324）
                        let _model_guard = crate::util::diagnostic::push_current_span(module.span);
                        if let Err(diag) = result.into_result() {
                            self.add_error(diag);
                        }
                    }
                }
            }
            (plan, escaped_refs)
        };

        // 求解所有约束
        let solve_result = self.env.solver().solve();
        if let Err(constraint_errors) = solve_result {
            for e in constraint_errors {
                let mut diag = e.error;
                diag.span = Some(e.span);
                self.add_error(diag);
            }
        }

        // 语义收集：遍历 AST 构建 SemanticDB
        // 即便类型检查存在错误（如语法或类型错误），我们也要尽可能收集当前的语义 token，保证代码染色等功能
        self.collect_semantic_tokens(module);

        // 语义事件排空（#433/D55）：绑定期采集的定义/引用/导入事件落 SemanticDB。
        // 定义与引用经 VarInfo.definition_span 同源衔接（resolves_to → DefId），
        // dummy 定义（导入名/占位）不入列，对应引用的 resolves_to 落空 DefId，
        // 跳转按 precise-only 返回 None。同 (名字, span) 去重——同一绑定
        // 经多条路径重复登记时保第一条。
        if let Some(bc) = self.body_checker.as_mut() {
            let file_path = self.env.module_name.clone();
            let mut seen_defs: HashSet<(String, crate::util::span::Span)> = HashSet::new();
            for ev in bc.take_binding_events() {
                if seen_defs.insert((ev.name.clone(), ev.span)) {
                    let def = semantic_db::DefinitionInfo {
                        def_id: semantic_db::DefId {
                            file_path: file_path.clone(),
                            span: ev.span,
                        },
                        name: ev.name,
                        kind: ev.kind,
                        span: ev.span,
                        file_path: file_path.clone(),
                        type_info: ev.type_info,
                        signature: None,
                    };
                    self.semantic_db.add_definition(&file_path, def);
                }
            }
            let (refs, imports) = (bc.take_reference_events(), bc.take_import_events());
            for r in refs {
                let r#ref = semantic_db::ReferenceInfo {
                    name: r.name,
                    span: r.span,
                    file_path: file_path.clone(),
                    resolves_to: semantic_db::DefId {
                        file_path: file_path.clone(),
                        span: r.resolves_to_span,
                    },
                };
                self.semantic_db.add_reference(&file_path, r#ref);
            }
            for imp in imports {
                self.semantic_db.add_import(
                    &file_path,
                    semantic_db::ImportInfo {
                        module_path: imp.module_path,
                        imported_names: imp.imported_names,
                        span: imp.span,
                    },
                );
            }
        }

        // 收集错误（无论有无错误都收进 result.diagnostics）。
        //
        // 4.2.7 去重：同一诊断事实（同码同 span 同消息）可经多条通道重复
        // 汇入——收集点的 collected_errors/Err 双通道（pass-3 与 drain 各
        // 收一份，嵌套作用域还会经外层收集点再收）、注解校验的签名形参与
        // 整体注解双访（E1003/E1103）、所有权层同点双发（E2014/E2018）。
        // 诊断是位置事实，重复报告不携带信息——模块结果边界统一去重，
        // 保首次出现、顺序不变（4.2.5 基线实证 64 个语料条目带重复副本）。
        let mut seen: HashSet<(String, Option<crate::util::span::Span>, String)> = HashSet::new();
        let mut diagnostics = Vec::new();
        for d in self.errors() {
            if seen.insert((d.code.clone(), d.span, d.message.clone())) {
                diagnostics.push(d.clone());
            }
        }

        // 构建类型检查结果
        // 合并 StatementChecker 中的局部变量类型到 bindings
        let mut bindings = self.env.vars.clone();
        let mut local_var_types = HashMap::new();

        // 从 body_checker.vars 获取局部变量类型，并合并三链模型的 globals（#295 重构：
        // 模块级绑定（如 `result = id(42)`）在 ScopeManager.globals，不在 vars() 中）
        if let Some(ref bc) = self.body_checker {
            for (name, poly) in bc.vars() {
                // 只添加 env.vars 中不存在的局部变量类型
                if !bindings.contains_key(&name) {
                    bindings.insert(name.clone(), poly.clone());
                }
                // 收集局部变量的 MonoType（用于 IR 生成器错误消息）
                local_var_types.insert(name, poly.body);
            }
            // 补入**已退出作用域**的块内声明（#D1）。
            //
            // `vars()` 遍历的是当前存活的 `local_scopes`，而 `exit_block`
            // 会 pop 掉块层——`while`/`if` 等体内声明的变量因此不在 `vars()` 中。
            // 但 IR 生成发生在类型检查**之后**，`for` 的迭代器派发需要这些变量的
            // 静态类型（`generate_for_loop_ir` 取不到类型即报 E3004 `<unknown>`）。
            //
            // `type_ledger` 正是为此存在的机制：注释标明「作用域 pop 后条目保留，
            // 供下游按位置查询」（scope.rs）。此处据它补齐类型表，
            // 使块内声明对 IR 生成可见。
            //
            // 按语句位置排序后覆盖写入：同名变量取**最后**一条（即最内层/最强制的
            // 那次声明），与词法遮蔽的直觉一致。
            let mut ledger_entries: Vec<_> = bc.var_type_ledger().iter().collect();
            ledger_entries.sort_by_key(|((stmt_key, _), _)| *stmt_key);
            for ((_stmt_key, name), poly) in ledger_entries {
                local_var_types
                    .entry(name.clone())
                    .or_insert_with(|| poly.body.clone());
            }
            for (name, info) in bc.scope_globals() {
                if !bindings.contains_key(name) {
                    bindings.insert(name.clone(), info.poly.clone());
                }
            }
        }

        // 同时从 env.vars 收集非全局绑定（函数）的局部变量
        for (name, poly) in &self.env.vars {
            // 排除函数（函数名首字母小写或者是已知的函数）
            let is_function =
                matches!(poly.body, crate::frontend::core::types::MonoType::Fn { .. });
            if !is_function && !local_var_types.contains_key(name) {
                local_var_types.insert(name.clone(), poly.body.clone());
            }
        }

        // 注意：由于 body_checker.solver 是克隆的，无法通过 solver.resolve() 来解析类型变量。
        // 幸运的是，assign_var 方法已经将更新后的类型写回到了 scope 中，
        // 所以这里直接使用 scope 中的类型即可，不需要额外 resolve。
        // （注：如果后续需要支持更复杂的泛型推导，可能需要重新设计 solver 的共享机制）

        // 从 body_checker 收集实例化请求
        let instantiation_requests = if let Some(ref bc) = self.body_checker {
            bc.instantiation_requests.clone()
        } else {
            Vec::new()
        };

        // RFC-011a §6: 从 body_checker 收集存在类型强制点（ir_gen 包装注入用）
        let variant_ctor_calls = if let Some(ref bc) = self.body_checker {
            bc.variant_ctor_calls.clone()
        } else {
            Vec::new()
        };
        let try_expr_impls = if let Some(ref bc) = self.body_checker {
            bc.try_expr_impls.clone()
        } else {
            Vec::new()
        };
        let operator_dispatches = if let Some(ref bc) = self.body_checker {
            bc.operator_dispatches.clone()
        } else {
            Vec::new()
        };
        let existential_coercions = if let Some(ref bc) = self.body_checker {
            bc.existential_coercions.clone()
        } else {
            Vec::new()
        };
        // #389：match scrutinee 推断类型（ir_gen 按 span 回查）
        let match_scrutinee_types = if let Some(ref bc) = self.body_checker {
            bc.match_scrutinee_types.clone()
        } else {
            HashMap::new()
        };

        // #321 W1003：未使用导入警告（Warning 级，不阻断编译，经 warnings 通道流出）
        let import_warnings = self.collect_unused_import_warnings(module);

        // WBS 3.4.2：求解器缺失时终止性测度义务判 Unjudged——「未判」≠「成立」，
        // 义务存在而判不了必须让用户知情（Warning 级，不阻断——同 W1003 契约）
        let mut warnings = import_warnings;
        if unjudged_termination_obligations > 0 {
            warnings.push(
                ErrorCodeDefinition::termination_obligations_unjudged(
                    unjudged_termination_obligations,
                )
                .build(),
            );
        }

        // WBS 3.4.1：编译期不可求值的 const 泛型约束——事实链
        //（environment → ExpressionInferrer → StatementChecker）在此汇聚，
        // 发射 W1063（Warning 级，不阻断——与 W1003 同一契约）
        if let Some(ref bc) = self.body_checker {
            for (span, constraint) in &bc.unevaluable_const_constraints {
                warnings.push(
                    ErrorCodeDefinition::const_generic_unevaluable(constraint)
                        .at(*span)
                        .build(),
                );
            }
        }

        TypeCheckResult {
            module_name: self.env.module_name.clone(),
            diagnostics,
            bindings,
            local_var_types,
            semantic_db: std::mem::take(&mut self.semantic_db),
            trait_table: self.env.trait_table.clone(),
            proof_calls, // Phase 2.5 预留：证明调用收集
            release_plan,
            escaped_refs,
            instantiation_requests,
            existential_coercions,
            try_expr_impls,
            match_scrutinee_types,
            method_overload_ir_names: self.env.method_overload_ir_names.clone(),
            overload_resolutions: if let Some(ref bc) = self.body_checker {
                bc.overload_resolutions.clone()
            } else {
                Vec::new()
            },
            operator_dispatches,
            variant_ctor_calls,
            implementation_proofs: self.env.implementation_proofs.clone(),
            interface_impl_registry: self.env.interface_impl_registry.clone(),
            sum_types: self.env.sum_types.clone(),
            sum_type_param_names: self
                .env
                .generic_type_defs
                .iter()
                .filter(|(name, _)| self.env.sum_types.contains_key(*name))
                .map(|(name, d)| (name.clone(), d.type_param_names.clone()))
                .collect(),
            module_namespaces: std::mem::take(&mut self.module_namespaces),
            warnings,
        }
    }

    /// 获取 body_checker 的可变引用
    fn body_checker_mut(&mut self) -> &mut inference::StatementChecker {
        if self.body_checker.is_none() {
            let trait_table = self.env.trait_table.clone();
            let mut body_checker = inference::StatementChecker::new(
                self.env.solver(),
                None,
                self.dependent_type_env.clone(),
                trait_table,
            );
            // 设置 native 函数签名表
            body_checker.set_native_signatures(self.env.native_signatures.clone());
            // #387：native arity 区间表随签名表同源传递
            body_checker.set_native_arities(self.env.native_arity.clone());
            // 设置模块注册表，支持函数体/块作用域 use
            body_checker.set_module_registry(self.env.module_registry.clone());
            // #396：模块别名集合——模块成员缺失报 E1043 而非 E1042
            body_checker.set_module_aliases(self.env.module_aliases.clone());
            self.body_checker = Some(body_checker);
        }
        self.body_checker.as_mut().unwrap()
    }

    /// 收集函数签名（第一遍扫描）
    /// 收集 `unsafe { ... }` 块内直接嵌套的类型定义（RFC-010：块内类型定义
    /// 交给上一作用域，使其在块外可用）。
    ///
    /// 递归处理嵌套的 unsafe 块；只取块体**直接**包含的 TypeDefinition
    /// （更深层的块由各自的父块负责）。返回 (name, definition, signature_params, span)
    /// 以便调用方在 pass1 中统一注册。
    fn collect_unsafe_type_defs(
        &self,
        stmt: &crate::frontend::core::parser::ast::Stmt,
        out: &mut Vec<(
            String,
            crate::frontend::core::parser::ast::Type,
            Vec<Param>,
            crate::util::span::Span,
        )>,
    ) {
        fn walk_expr(
            e: &crate::frontend::core::parser::ast::Expr,
            out: &mut Vec<(
                String,
                crate::frontend::core::parser::ast::Type,
                Vec<Param>,
                crate::util::span::Span,
            )>,
        ) {
            use crate::frontend::core::parser::ast::{Expr, StmtKind};
            match e {
                Expr::Unsafe { body, .. } => {
                    for s in &body.stmts {
                        if let StmtKind::TypeDefinition {
                            name,
                            signature_params,
                            definition,
                            ..
                        } = &s.kind
                        {
                            out.push((
                                name.clone(),
                                definition.clone(),
                                signature_params.clone(),
                                s.span,
                            ));
                        }
                        collect_unsafe_type_defs_pub(s, out);
                    }
                }
                Expr::If {
                    then_branch,
                    else_if_branches,
                    else_branch,
                    ..
                } => {
                    for s in &then_branch.stmts {
                        collect_unsafe_type_defs_pub(s, out);
                    }
                    for (_, b) in else_if_branches {
                        for s in &b.stmts {
                            collect_unsafe_type_defs_pub(s, out);
                        }
                    }
                    if let Some(b) = else_branch {
                        for s in &b.stmts {
                            collect_unsafe_type_defs_pub(s, out);
                        }
                    }
                }
                Expr::Block(b) => {
                    for s in &b.stmts {
                        collect_unsafe_type_defs_pub(s, out);
                    }
                }
                _ => {}
            }
        }
        fn collect_unsafe_type_defs_pub(
            stmt: &crate::frontend::core::parser::ast::Stmt,
            out: &mut Vec<(
                String,
                crate::frontend::core::parser::ast::Type,
                Vec<Param>,
                crate::util::span::Span,
            )>,
        ) {
            use crate::frontend::core::parser::ast::StmtKind;
            match &stmt.kind {
                StmtKind::Expr(e) => walk_expr(e, out),
                StmtKind::Assign { value: Some(v), .. } => walk_expr(v, out),
                StmtKind::If {
                    then_branch,
                    else_if_branches,
                    else_branch,
                    ..
                } => {
                    for s in &then_branch.stmts {
                        collect_unsafe_type_defs_pub(s, out);
                    }
                    for (_, b) in else_if_branches {
                        for s in &b.stmts {
                            collect_unsafe_type_defs_pub(s, out);
                        }
                    }
                    if let Some(b) = else_branch {
                        for s in &b.stmts {
                            collect_unsafe_type_defs_pub(s, out);
                        }
                    }
                }
                StmtKind::For { body, .. } => {
                    for s in &body.stmts {
                        collect_unsafe_type_defs_pub(s, out);
                    }
                }
                _ => {}
            }
        }

        collect_unsafe_type_defs_pub(stmt, out);
    }

    fn collect_function_signature(
        &mut self,
        stmt: &crate::frontend::core::parser::ast::Stmt,
    ) {
        // #358：顶层函数**重复定义**检测（同名**同签名**才是重复）。
        //
        // 此前只有「与结构体重名」会报 E2002——同一文件写两次 `main`（或任何
        // 顶层函数）两个定义都留在函数表里，静默取第一个。下游按名解析
        //（`CallStatic` / vtable / `find_entry_point`）可能取到非预期的那个，
        // 而用户看不到任何提示。
        //
        // 重载（RFC-011 §3.15）是合法特性：`sum: (Array(Int)) -> Int` 与
        // `sum: (Array(Float)) -> Float` 共存不报——故只在**签名完全相同**时判重。
        if let Some((dup_name, sig)) = self.top_level_fn_signature(stmt) {
            match self.declared_top_fns.get(&dup_name) {
                Some(prev) if *prev == sig => {
                    let diag = ErrorCodeDefinition::duplicate_definition(&dup_name)
                        .at(stmt.span)
                        .build();
                    self.env.errors.add_error(diag);
                }
                Some(_) => {} // 不同签名 → 重载，放行
                None => {
                    self.declared_top_fns.insert(dup_name, sig);
                }
            }
        }
        match &stmt.kind {
            crate::frontend::core::parser::ast::StmtKind::Expr(expr) => {
                // 处理函数定义表达式
                if let crate::frontend::core::parser::ast::Expr::FnDef {
                    name,
                    params,
                    return_type,
                    ..
                } = expr.as_ref()
                {
                    let fn_ty = MonoType::Fn {
                        params: params
                            .iter()
                            .map(|p| {
                                p.ty.as_ref()
                                    .map(|t| MonoType::from(t.clone()))
                                    .unwrap_or_else(|| self.env.solver().new_var())
                            })
                            .collect(),
                        return_type: Box::new(
                            return_type
                                .as_ref()
                                .map(|t| MonoType::from(t.clone()))
                                .unwrap_or_else(|| self.env.solver().new_var()),
                        ),
                    };

                    // RFC-027: 解析类型标注中的编译期谓词（#263：诊断汇入后上报）
                    let mut refined_diags = Vec::new();
                    let fn_ty = match fn_ty {
                        MonoType::Fn {
                            params,
                            return_type,
                        } => MonoType::Fn {
                            params: params
                                .into_iter()
                                .map(|p| self.resolve_type_annotation(&p, &mut refined_diags))
                                .collect(),
                            return_type: Box::new(
                                self.resolve_type_annotation(&return_type, &mut refined_diags),
                            ),
                        },
                        other => other,
                    };
                    let fn_ty = self.resolve_terminates_base(fn_ty);
                    self.env.errors.extend_errors(refined_diags);

                    self.env.add_var(name.clone(), PolyType::mono(fn_ty));
                }
                // 处理 Lambda 赋值 (name = (params) => body)
                else if let crate::frontend::core::parser::ast::Expr::BinOp {
                    op: crate::frontend::core::parser::ast::BinOp::Assign,
                    left,
                    right,
                    ..
                } = expr.as_ref()
                {
                    if let crate::frontend::core::parser::ast::Expr::Var(name, _) = left.as_ref() {
                        if let crate::frontend::core::parser::ast::Expr::Lambda { params, .. } =
                            right.as_ref()
                        {
                            let fn_ty = MonoType::Fn {
                                params: params
                                    .iter()
                                    .map(|p| {
                                        p.ty.as_ref()
                                            .map(|t| MonoType::from(t.clone()))
                                            .unwrap_or_else(|| self.env.solver().new_var())
                                    })
                                    .collect(),
                                return_type: Box::new(self.env.solver().new_var()),
                            };

                            // RFC-027: 解析类型标注中的编译期谓词
                            let mut refined_diags = Vec::new();
                            let fn_ty =
                                match fn_ty {
                                    MonoType::Fn {
                                        params,
                                        return_type,
                                    } => MonoType::Fn {
                                        params: params
                                            .into_iter()
                                            .map(|p| {
                                                self.resolve_type_annotation(&p, &mut refined_diags)
                                            })
                                            .collect(),
                                        return_type: Box::new(self.resolve_type_annotation(
                                            &return_type,
                                            &mut refined_diags,
                                        )),
                                    },
                                    other => other,
                                };
                            self.env.errors.extend_errors(refined_diags);

                            self.env.add_var(name.clone(), PolyType::mono(fn_ty));
                        }
                    }
                }
            }
            crate::frontend::core::parser::ast::StmtKind::Assign {
                target,
                type_annotation,
                signature_params,
                value,
                ..
            } if value.as_ref().is_some_and(|v| {
                matches!(
                    v.as_ref(),
                    crate::frontend::core::parser::ast::Expr::Lambda { .. }
                ) || crate::frontend::core::parser::ast::Expr::block_binding_is_function(
                    type_annotation.as_ref(),
                    Some(v.as_ref()),
                )
            }) || (value.is_none() && type_annotation.is_some()) =>
            {
                // 从 target 提取 name 和 type_name
                let (name, type_name) = match target.as_ref() {
                    crate::frontend::core::parser::ast::Expr::Var(n, _) => (n.clone(), None),
                    crate::frontend::core::parser::ast::Expr::FieldAccess {
                        expr, field, ..
                    } => {
                        if let crate::frontend::core::parser::ast::Expr::Var(tn, _) = expr.as_ref()
                        {
                            {
                                // 语义分流（issue #180 F 组）：base 是类型才是方法定义；
                                // base 是值（实例）→ 字段赋值，pass3 处理，不当方法定义收集。
                                let ty_name = matches!(
                                    self.env.resolve_base_kind(tn),
                                    crate::frontend::core::typecheck::environment::BaseKind::TypeSpace
                                )
                                .then(|| tn.clone());
                                (field.clone(), ty_name)
                            }
                        } else {
                            (field.clone(), None)
                        }
                    }
                    _ => {
                        // 无法从 target 提取名字，跳过
                        return;
                    }
                };
                // 从 value 提取 params 和 body（Lambda 形式）
                let (params, _): (Vec<_>, Vec<_>) = match value {
                    Some(expr) => {
                        if let crate::frontend::core::parser::ast::Expr::Lambda {
                            params: p,
                            body: b,
                            ..
                        } = expr.as_ref()
                        {
                            (p.clone(), b.stmts.clone())
                        } else if let crate::frontend::core::parser::ast::Expr::Block(b) =
                            expr.as_ref()
                        {
                            (Vec::new(), b.stmts.clone())
                        } else {
                            (Vec::new(), Vec::new())
                        }
                    }
                    None => (Vec::new(), Vec::new()),
                };
                let method_type = type_annotation.as_ref();
                let generic_params = classify_generic_params(
                    signature_params,
                    // RFC-011b: 运算符接口名也是合法约束名（T: Add）——
                    // 约束求解在实例化点查接口实现登记表
                    &|name| {
                        self.env.has_trait(name) || super::operator_interfaces::spec(name).is_some()
                    },
                );
                // 处理统一函数语法
                // 方法绑定使用 method_type，普通函数使用 type_annotation
                let (param_types, return_type) = if let Some(meth_ty) = method_type {
                    // 方法绑定：优先使用 method_type 中的签名
                    if let crate::frontend::core::parser::ast::Type::Fn {
                        params: param_tys,
                        return_type,
                    } = meth_ty
                    {
                        let pts: Vec<MonoType> = param_tys
                            .iter()
                            .map(|t| MonoType::from(t.clone()))
                            .collect();
                        (pts, MonoType::from(*return_type.clone()))
                    } else {
                        // method_type 不是 Fn 类型，回退到 type_annotation 或 params
                        if let Some(type_ann) = type_annotation {
                            if let crate::frontend::core::parser::ast::Type::Fn {
                                params: param_tys,
                                return_type,
                            } = type_ann
                            {
                                let pts: Vec<MonoType> = param_tys
                                    .iter()
                                    .map(|t| MonoType::from(t.clone()))
                                    .collect();
                                (pts, MonoType::from(*return_type.clone()))
                            } else {
                                let pts: Vec<MonoType> = params
                                    .iter()
                                    .map(|p| {
                                        p.ty.as_ref()
                                            .map(|t| MonoType::from(t.clone()))
                                            .unwrap_or_else(|| self.env.solver().new_var())
                                    })
                                    .collect();
                                (pts, self.env.solver().new_var())
                            }
                        } else {
                            let pts: Vec<MonoType> = params
                                .iter()
                                .map(|p| {
                                    p.ty.as_ref()
                                        .map(|t| MonoType::from(t.clone()))
                                        .unwrap_or_else(|| self.env.solver().new_var())
                                })
                                .collect();
                            (pts, self.env.solver().new_var())
                        }
                    }
                } else if let Some(type_ann) = type_annotation {
                    if let crate::frontend::core::parser::ast::Type::Fn {
                        params: param_tys,
                        return_type,
                    } = type_ann
                    {
                        let pts: Vec<MonoType> = param_tys
                            .iter()
                            .map(|t| MonoType::from(t.clone()))
                            .collect();
                        (pts, MonoType::from(*return_type.clone()))
                    } else {
                        let pts: Vec<MonoType> = params
                            .iter()
                            .map(|p| {
                                p.ty.as_ref()
                                    .map(|t| MonoType::from(t.clone()))
                                    .unwrap_or_else(|| self.env.solver().new_var())
                            })
                            .collect();
                        (pts, self.env.solver().new_var())
                    }
                } else {
                    let pts: Vec<MonoType> = params
                        .iter()
                        .map(|p| {
                            p.ty.as_ref()
                                .map(|t| MonoType::from(t.clone()))
                                .unwrap_or_else(|| self.env.solver().new_var())
                        })
                        .collect();
                    (pts, self.env.solver().new_var())
                };

                // 泛型函数处理：
                // 当 generic_params 包含 Type 级别的参数时，外层 Fn 的前 N 个参数
                // 是类型级参数（如 (T: Type)），return_type 才是实际的值级函数类型。
                // 需要剥离类型级参数，并将 TypeRef("T") 替换为新的类型变量。
                let type_generic_params: Vec<_> = generic_params
                    .iter()
                    .filter(|p| {
                        matches!(
                            p.kind,
                            crate::frontend::core::parser::ast::GenericParamKind::Type
                        )
                    })
                    .collect();

                // === 函数 const 泛型判定（用途分析） ===
                // 候选 = generic_params 中 annotation 为具体类型的参数（形态粗筛产物）。
                // 扫描 curry 后续组的类型标注（内层 Fn 的 params），
                // 被引用的候选 → const。精筛唯一实现在 const_param.rs（RFC-011 §4.1）。
                let candidate_names: HashSet<String> = generic_params
                    .iter()
                    .filter(|p| {
                        matches!(
                            p.kind,
                            crate::frontend::core::parser::ast::GenericParamKind::Const { .. }
                        )
                    })
                    .map(|p| p.name.clone())
                    .collect();

                let mut used_as_const = HashSet::new();
                if !candidate_names.is_empty() {
                    // 扫描内层 Fn 的 params：对于 (N: Int) -> (n: N) -> Int，
                    // type_annotation.return_type 是 Fn { params: [Type::Name("N")], ... }。
                    // N 出现在内层 params 中 → const。
                    //
                    // RFC-027a：扫描需排除**谓词应用的实参位置**——`Terminates(b)`
                    // 的 `b`、`IsPositive(n)` 的 `n` 是编译期表达式（值），不是类型
                    // 引用。不排除则形参名被误判为 const 泛型参数并替换成底层类型，
                    // 精化约束/测度在到达消费端之前就丢了（E1092）。
                    let is_predicate = |n: &str| self.is_predicate_application(n);
                    if let Some(crate::frontend::core::parser::ast::Type::Fn {
                        return_type, ..
                    }) = type_annotation
                    {
                        if let crate::frontend::core::parser::ast::Type::Fn {
                            params: inner_params,
                            ..
                        } = return_type.as_ref()
                        {
                            for p in inner_params {
                                collect_used_in_type(
                                    p,
                                    &candidate_names,
                                    &mut used_as_const,
                                    &is_predicate,
                                );
                            }
                        }
                        // 也扫描 return_type 自身（深度嵌套场景）
                        collect_used_in_type(
                            return_type,
                            &candidate_names,
                            &mut used_as_const,
                            &is_predicate,
                        );
                    }
                }

                let resolved_const =
                    crate::frontend::core::typecheck::const_param::resolve_const_candidates(
                        &generic_params,
                        signature_params,
                        &used_as_const,
                        type_generic_params.len(),
                    );
                let const_binders: Vec<ConstVarDef> = resolved_const.const_binders;

                let (final_param_types, final_return_type) = if !type_generic_params.is_empty()
                    && param_types.len() >= type_generic_params.len()
                {
                    // 为每个泛型类型参数创建新的类型变量
                    let mut subst = HashMap::new();
                    for gp in &type_generic_params {
                        let fresh_var = self.env.solver().new_var();
                        subst.insert(gp.name.clone(), fresh_var);
                    }

                    // 添加 const 参数名到 subst，使内层 Fn 中的 TypeRef("N") 解析为底层类型
                    for cb in &const_binders {
                        let base_ty = match cb.kind {
                            ConstKind::Int(_) => MonoType::Int(64),
                            ConstKind::Bool => MonoType::Bool,
                            ConstKind::Float(_) => MonoType::Float(64),
                        };
                        subst.insert(cb.name.clone(), base_ty);
                    }

                    // 剥离类型级参数，使用 return_type 作为实际函数类型
                    let inner_fn_ty = return_type.clone().substitute(&subst);

                    match inner_fn_ty {
                        MonoType::Fn {
                            params: inner_params,
                            return_type: inner_ret,
                            ..
                        } => (inner_params, *inner_ret),
                        // return_type 不是 Fn（可能是单值泛型），保持原样
                        _ => (param_types, return_type),
                    }
                } else if !const_binders.is_empty() {
                    // 没有 Type 泛型但有 const 泛型（如 factorial: (N: Int) -> ...）
                    // 替换 return_type 中的 TypeRef("N") 为 Int
                    let mut subst = HashMap::new();
                    for cb in &const_binders {
                        let base_ty = match cb.kind {
                            ConstKind::Int(_) => MonoType::Int(64),
                            ConstKind::Bool => MonoType::Bool,
                            ConstKind::Float(_) => MonoType::Float(64),
                        };
                        subst.insert(cb.name.clone(), base_ty);
                    }
                    let substituted_ret = return_type.clone().substitute(&subst);
                    (param_types, substituted_ret)
                } else {
                    (param_types, return_type)
                };

                let fn_ty = MonoType::Fn {
                    params: final_param_types.clone(),
                    return_type: Box::new(final_return_type),
                };
                // RFC-027: 解析类型标注中的编译期谓词（如 Positive(5) -> Refined）
                let mut refined_diags = Vec::new();
                let fn_ty = match fn_ty {
                    MonoType::Fn {
                        params,
                        return_type,
                    } => MonoType::Fn {
                        params: params
                            .into_iter()
                            .map(|p| self.resolve_type_annotation(&p, &mut refined_diags))
                            .collect(),
                        return_type: Box::new(
                            self.resolve_type_annotation(&return_type, &mut refined_diags),
                        ),
                    },
                    other => other,
                };
                let fn_ty = self.resolve_terminates_base(fn_ty);
                self.env.errors.extend_errors(refined_diags);

                // 如果有 type_name（显式方法绑定），使用 add_fn_binding
                if type_name.is_some() {
                    // RFC-011a §3: 同签名重复声明 = 覆盖 → E1100；不同签名 = 重载 → 放行
                    let tn = type_name.as_deref().unwrap_or_default();
                    let key = (tn.to_string(), name.clone());
                    if let Some(prev) = self.declared_methods.get(&key) {
                        if *prev == fn_ty {
                            self.add_error(
                                ErrorCodeDefinition::interface_method_duplicate(tn, &name)
                                    .at(stmt.span)
                                    .build(),
                            );
                        }
                    }
                    self.declared_methods.insert(key, fn_ty.clone());
                    self.env
                        .add_fn_binding(&name, type_name.as_deref(), fn_ty.clone());
                    // RFC-011b: 重载候选表（接口完整性检查按候选集匹配）
                    if let Some(tn) = type_name.as_deref() {
                        let okey = format!("{}.{}", tn, name);
                        self.env.add_method_overload(&okey, fn_ty.clone());
                    }
                } else {
                    // 如果函数有 const 泛型参数，存进 PolyType.const_binders
                    let poly = if const_binders.is_empty() {
                        PolyType::mono(fn_ty.clone())
                    } else {
                        // type_binders 从 solver 的新变量管理，PolyType 中 type_binders 留空
                        // （函数类型 Level 的泛型由 solver 处理，PolyType 仅存 const_binders）
                        PolyType::new_with_const(Vec::new(), const_binders.clone(), fn_ty.clone())
                    };
                    self.env.add_var(name.clone(), poly);
                }
            }
            crate::frontend::core::parser::ast::StmtKind::Use {
                path,
                items,
                alias,
                item_aliases,
                ..
            } => {
                // 计算导入模式
                // use std.io → register as "io.print"
                // use std.io as str → register as "str.print"
                // use std.{print} → register as "print"
                // use std.{print as p} → register as "p"（#245 内联别名）
                // use std.{print, read} → register as "print", "read"
                let import_all = items.is_none();
                let aliases = alias.as_ref();

                // 通过 ModuleRegistry 查找模块导出，不再硬编码特定模块
                if let Some(module) = self.env.module_registry.get(path).cloned() {
                    let items_ref = items.as_ref();

                    // 收集需要导入的导出
                    let mut exports_to_import: Vec<&crate::frontend::module::Export> = Vec::new();
                    for export in module.exports.values() {
                        let should_import = import_all
                            || items_ref.is_some_and(|i| i.iter().any(|s| s.name == export.name));
                        if should_import {
                            exports_to_import.push(export);
                        }
                    }

                    // 根据别名情况注册
                    match (items.as_ref(), aliases) {
                        // use path (无 items，无 alias) → 提取 path 最后部分作为模块别名
                        (None, None) => {
                            let module_alias = path.split('.').next_back().unwrap_or(path);
                            // #321 W1003：登记导入本地名与导出成员监视
                            self.record_import_name(module_alias, stmt.span);
                            self.watch_import_members(module_alias, &module);
                            // 登记用户模块命名空间别名（std 走 is_std_submodule 机制，不入此表）
                            if !(path == "std" || path.starts_with("std.")) {
                                self.module_namespaces
                                    .insert(module_alias.to_string(), path.clone());
                            }
                            // 首先将模块本身注册为 Struct 类型（包含所有导出作为字段）
                            self.register_module_as_struct(path, module_alias, &module);
                            // 然后注册每个导出。带载荷的类型导出额外按**自身名**
                            // 补镜像：整体导入此前只注册模块 Struct，类型裸名
                            // （`use std.result` 后写 `Result(Int, String)`）不可用。
                            // 仅限 type_payload 非空的导出——native 类型家族桩
                            // （mono_type None）经 with_std 预载已有解析路径，
                            // 用宽松类型注册成值变量会污染值位置的类型名解析。
                            for export in exports_to_import {
                                if matches!(export.kind, crate::frontend::module::ExportKind::Type)
                                    && export.type_payload.is_some()
                                {
                                    self.register_use_export(&export.name, export, false);
                                }
                                self.register_use_export(module_alias, export, true);
                            }
                        }
                        // use path as alias → 整个模块用别名注册
                        // #414：单/多别名统一走此臂——每个别名各绑一份模块 record
                        (None, Some(aliases)) => {
                            for alias_name in aliases {
                                // #321 W1003：登记导入本地名与导出成员监视
                                self.record_import_name(alias_name, stmt.span);
                                self.watch_import_members(alias_name, &module);
                                // 命名空间别名登记。#410：std 路径不再豁免——裸子模块名由
                                // is_std_submodule 兜住，别名只有这张表能救；漏登记则
                                // ir_gen 把 `m.sqrt` 当闭包值调用 → E8001 ICE
                                self.module_namespaces
                                    .insert(alias_name.clone(), path.clone());
                                for export in &exports_to_import {
                                    if matches!(
                                        export.kind,
                                        crate::frontend::module::ExportKind::Type
                                    ) && export.type_payload.is_some()
                                    {
                                        self.register_use_export(&export.name, export, false);
                                    }
                                    self.register_use_export(alias_name, export, true);
                                }
                            }
                        }
                        // use path.{a, b} / use path.{a as x}（#245：仅内联别名）。
                        // 按 item 名查导出——exports 是 HashMap 无序，zip 会错配。
                        (Some(item_names), _) => {
                            for (i, item) in item_names.iter().enumerate() {
                                let Some(export) = module.exports.get(&item.name) else {
                                    continue;
                                };
                                let local_name = item_aliases
                                    .as_ref()
                                    .and_then(|v| v.get(i))
                                    .and_then(|a| a.as_ref())
                                    .map(|a| a.name.as_str());
                                // #415：子模块导出经内联别名绑定时（`use std.{io as printer}`），
                                // is_std_submodule 只认裸子模块名，别名必须入 namespace 表，
                                // 否则 ir_gen 解析 `printer.print` → E3006。非别名条目入表与
                                // 既有机制重合（io → std.io），无害。
                                if matches!(
                                    export.kind,
                                    crate::frontend::module::ExportKind::SubModule
                                ) {
                                    let ns_name = local_name.unwrap_or(&item.name);
                                    self.module_namespaces
                                        .insert(ns_name.to_string(), export.full_path.clone());
                                }
                                // #321 W1003：登记导入本地名（内联别名优先）
                                match local_name {
                                    Some(local) => self.record_import_name(local, stmt.span),
                                    None => self.record_import_name(&item.name, stmt.span),
                                }
                                match local_name {
                                    Some(local) => self.register_use_export(local, export, true),
                                    None => self.register_use_export(&item.name, export, false),
                                }
                            }
                        }
                    }
                }
            }
            // 外部绑定: Type.method = function 或 Type.method = function[pos]
            crate::frontend::core::parser::ast::StmtKind::Assign {
                target,
                value: Some(val),
                span,
                ..
            } => {
                let (type_name, method_name) = match target.as_ref() {
                    Expr::FieldAccess { expr, field, .. } => {
                        if let Expr::Var(tn, _) = expr.as_ref() {
                            (tn.clone(), field.clone())
                        } else {
                            return;
                        }
                    }
                    _ => return,
                };
                // 语义分流（issue #180 F 组）：base 必须是已注册类型才在 pass2 登记类型空间方法。
                // base 是值（实例）→ 不在此登记，由 pass3 check_stmt 做 schema 校验。
                if !matches!(
                    self.env.resolve_base_kind(&type_name),
                    crate::frontend::core::typecheck::environment::BaseKind::TypeSpace
                ) {
                    return;
                }
                // 从 value 提取 func_name 和 positions
                let (func_name, positions) = match val.as_ref() {
                    Expr::Var(fn_name, _) => (fn_name.clone(), vec![0]),
                    Expr::Index {
                        expr: inner, index, ..
                    } => {
                        let fn_name = if let Expr::Var(n, _) = inner.as_ref() {
                            n.clone()
                        } else {
                            return;
                        };
                        let positions = Self::extract_positions(index);
                        (fn_name, positions)
                    }
                    _ => return,
                };
                let found = self.env.get_var(&func_name).map(|poly| {
                    let total = match &poly.body {
                        MonoType::Fn { params, .. } => params.len(),
                        _ => 0,
                    };
                    (total, poly.body.clone())
                });
                if let Some((total, fn_ty)) = found {
                    // #321 W1003：绑定为方法的导入函数视为已使用（pass2 解析，
                    // 不经 body_checker 的 Var 推断臂）
                    self.note_import_use(&func_name);
                    if let Some(positions) =
                        self.normalize_binding_positions(&positions, total, *span)
                    {
                        let method_ty = Self::method_type_after_binding(&fn_ty, &positions);
                        self.env
                            .add_method_binding(&type_name, &method_name, method_ty);
                    }
                }
            }
            _ => {}
        }
    }

    /// T3：pass2 收集**顶层值绑定**的类型（含非函数的 `name = expr`）。
    ///
    /// 此前 pass2 只收函数签名，值绑定只在 pass3 执行到语句时才入 scope——
    /// 导致前向引用（`main` 中引用后置绑定、`b = a + 1` 且 `a` 在后）报 E1001。
    /// 现在值绑定也在 pass2 登记，使其在整个模块内可见。
    ///
    /// 类型来源优先级：显式标注 > 初始化表达式的字面量类型 > 类型变量（待 pass3 统一）。
    fn collect_value_binding_signature(
        &mut self,
        stmt: &crate::frontend::core::parser::ast::Stmt,
    ) {
        use crate::frontend::core::parser::ast::{Expr, StmtKind};
        let StmtKind::Assign {
            target,
            type_annotation,
            value,
            ..
        } = &stmt.kind
        else {
            return;
        };
        // 只处理裸名绑定（`Type.method` 归方法路径）
        let Expr::Var(name, _) = target.as_ref() else {
            return;
        };
        // 函数绑定已在 collect_function_signature 登记，不重复
        let is_fn = value.as_ref().is_some_and(|v| {
            matches!(v.as_ref(), Expr::Lambda { .. })
                || Expr::block_binding_is_function(type_annotation.as_ref(), Some(v.as_ref()))
        });
        if is_fn {
            return;
        }
        // 已有该名字（函数签名或前面登记的值）则不覆盖
        if self.env.vars.contains_key(name) || self.early_value_bindings.contains_key(name) {
            return;
        }
        // 类型：显式标注优先；否则从字面量初始化推断一个保守类型
        let ty = match type_annotation {
            Some(t) => MonoType::from(t.clone()),
            None => match value.as_deref() {
                Some(Expr::Lit(lit, _)) => match lit {
                    crate::frontend::core::lexer::tokens::Literal::Int(_) => MonoType::Int(64),
                    crate::frontend::core::lexer::tokens::Literal::Float(_) => MonoType::Float(64),
                    crate::frontend::core::lexer::tokens::Literal::String(_) => {
                        MonoType::make_string()
                    }
                    crate::frontend::core::lexer::tokens::Literal::Char(_) => MonoType::Char,
                    crate::frontend::core::lexer::tokens::Literal::Bool(_) => MonoType::Bool,
                    _ => self.env.solver().new_var(),
                },
                // 其余形态（块值、调用、引用、运算）：类型变量。
                // pass3 执行到该语句时统一出具体类型；这里只求「名字可见」。
                _ => self.env.solver().new_var(),
            },
        };
        self.early_value_bindings
            .insert(name.clone(), PolyType::mono(ty));
    }

    /// RFC-004: 归一化绑定位置（负索引从末尾计数，[-1] = 最后一个参数）并校验有效性。
    /// total 为被绑函数参数个数；未知（0）时跳过归一化与校验。无效位置报 E1064。
    fn normalize_binding_positions(
        &mut self,
        positions: &[i64],
        total: usize,
        span: crate::util::span::Span,
    ) -> Option<Vec<i64>> {
        if total == 0 {
            return Some(positions.to_vec());
        }
        let total_i = total as i64;
        let normalized: Vec<i64> = positions
            .iter()
            .map(|&p| if p < 0 { p + total_i } else { p })
            .collect();
        if normalized.iter().any(|&p| p < 0 || p >= total_i) {
            self.add_error(
                ErrorCodeDefinition::invalid_binding_position(&format!("{:?}", positions), total)
                    .at(span)
                    .build(),
            );
            return None;
        }
        Some(normalized)
    }

    /// RFC-004: 绑定后的方法类型——单位绑定时保留全签名（实例占住绑定参数位），
    /// 多位绑定时挖掉被绑参数，剩余参数由调用点填充。
    fn method_type_after_binding(
        fn_ty: &MonoType,
        positions: &[i64],
    ) -> MonoType {
        if positions.len() <= 1 {
            return fn_ty.clone();
        }
        match fn_ty {
            MonoType::Fn {
                params,
                return_type,
            } => {
                let new_params: Vec<MonoType> = params
                    .iter()
                    .enumerate()
                    .filter(|(i, _)| !positions.contains(&(*i as i64)))
                    .map(|(_, p)| p.clone())
                    .collect();
                MonoType::Fn {
                    params: new_params,
                    return_type: return_type.clone(),
                }
            }
            other => other.clone(),
        }
    }

    /// RFC-004: 登记类型体绑定的待处理队列（须在函数签名收集 pass2 之后调用）。
    pub fn flush_pending_body_bindings(&mut self) {
        let pending = std::mem::take(&mut self.pending_body_bindings);
        for (type_name, binding, span) in pending {
            match &binding.kind {
                crate::frontend::core::parser::ast::BindingKind::External {
                    function,
                    positions,
                } => {
                    self.register_external_method_binding(
                        &type_name,
                        &binding.name,
                        function,
                        positions,
                        span,
                    );
                }
                crate::frontend::core::parser::ast::BindingKind::DefaultExternal { function } => {
                    self.register_external_method_binding(
                        &type_name,
                        &binding.name,
                        function,
                        &[0],
                        span,
                    );
                }
                crate::frontend::core::parser::ast::BindingKind::Anonymous { .. } => {}
            }
        }
    }

    /// RFC-011a 阶段1: 判断类型体应用项是否为接口实例化形态——
    /// head 命中已注册类型构造器（generic_type_defs）且实参全为类型形态。
    fn is_interface_application(
        &self,
        ty: &crate::frontend::core::parser::ast::Type,
    ) -> bool {
        use crate::frontend::core::parser::ast::Type;
        if let Type::Generic {
            name: head, args, ..
        } = ty
        {
            let all_type_args = args.iter().all(|a| {
                matches!(
                    a,
                    Type::Name { .. }
                        | Type::Generic { .. }
                        | Type::MetaType { .. }
                        | Type::Ref { .. }
                )
            });
            return all_type_args && self.env.generic_type_defs.contains_key(head);
        }
        false
    }

    /// RFC-011a: 类型实参是否引用给定参数名（抽象引用 → 继承链接，不记待决实例化）
    fn type_arg_references(
        ty: &crate::frontend::core::parser::ast::Type,
        params: &[String],
    ) -> bool {
        use crate::frontend::core::parser::ast::Type;
        match ty {
            Type::Name { name, .. } => params.contains(name),
            Type::Generic { args, .. } => args.iter().any(|a| Self::type_arg_references(a, params)),
            Type::Ref { inner, .. } => Self::type_arg_references(inner, params),
            _ => false,
        }
    }

    /// RFC-011a 阶段2: 接口实例化完整性检查（须在 pass2 与类型体绑定登记之后调用）。
    /// 五步流程（§1）：识别（pass1 已记录）→ Self 替换展开 → 签名匹配 →
    /// 通过则登记 ImplementationProof → 失败报编译错误。
    pub fn finalize_interface_instantiations(&mut self) {
        let pending = std::mem::take(&mut self.pending_interface_instantiations);
        for inst in pending {
            let args: Vec<MonoType> = inst
                .args
                .iter()
                .map(|a| MonoType::from(a.clone()))
                .collect();
            let _ = self.check_interface_instantiation(
                &inst.impl_type,
                &inst.interface_name,
                &args,
                inst.span,
            );
        }
    }

    /// 构造类型定义导出载荷（跨模块类型传播）。
    ///
    /// 在**签名收集期**的 checker env 上读取三张表：泛型模板、和类型变体、
    /// 类型体接口应用。接口应用条目与 `pending_interface_instantiations`
    /// 同源同构（AST `Expr(Generic)` 应用项，`is_interface_application` 判定），
    /// 区别仅在签名期不做完整性检查（那要在方法绑定齐备后进行，属定义方
    /// 模块自身的 finalize 职责）——分发查询只匹配 impl_type + args，导入方
    /// 据此恢复登记表即可。三张表全空的普通记录类型返回 None（导出退化为
    /// 既有 mono_type 快照，零开销）。
    /// 类型表达式是否引用指定类型名（模板 Self 位判定：
    /// `Index(Box(T), Int, T)` 的 `Box(T)` 引用 Box 自身）
    fn type_mentions_type(
        ty: &crate::frontend::core::parser::ast::Type,
        name: &str,
    ) -> bool {
        use crate::frontend::core::parser::ast::Type;
        match ty {
            Type::Name { name: n, .. } => n == name,
            Type::Generic { name: n, args, .. } => {
                n == name || args.iter().any(|a| Self::type_mentions_type(a, name))
            }
            Type::Ref { inner, .. } => Self::type_mentions_type(inner, name),
            _ => false,
        }
    }

    /// 模板条目的成员名：编译器侧规格接口取固定方法名，用户接口取
    /// 声明体的函数字段名（best-effort——完整性检查在方法定义处另行把关）
    fn interface_template_methods(
        head: &str,
        env: &TypeEnvironment,
    ) -> Vec<String> {
        if let Some(spec) = super::operator_interfaces::spec(head) {
            return vec![spec.method.to_string()];
        }
        env.generic_type_defs
            .get(head)
            .map(|d| match &d.poly.body {
                MonoType::Struct(s) => s
                    .fields
                    .iter()
                    .filter(|(_, t)| matches!(t, MonoType::Fn { .. }))
                    .map(|(n, _)| n.clone())
                    .collect(),
                _ => Vec::new(),
            })
            .unwrap_or_default()
    }

    pub fn type_def_export_payload(
        &self,
        name: &str,
        definition: &crate::frontend::core::parser::ast::Type,
    ) -> Option<crate::frontend::module::TypeDefPayload> {
        use crate::frontend::core::parser::ast::{Type, TypeBodyItem};
        let type_def = self.env.generic_type_defs.get(name).cloned();
        let sum_variants = self.env.sum_types.get(name).cloned().unwrap_or_default();
        let mut interface_impls: Vec<(String, super::environment::InterfaceImplEntry)> = Vec::new();
        if let Type::Struct { body } = definition {
            for item in body {
                let TypeBodyItem::Expr(ty) = item else {
                    continue;
                };
                if !self.is_interface_application(ty) {
                    continue;
                }
                let Type::Generic {
                    name: head,
                    args: ast_args,
                    ..
                } = ty
                else {
                    continue;
                };
                let mono_args: Vec<MonoType> =
                    ast_args.iter().map(|a| MonoType::from(a.clone())).collect();
                // 尽力收集成员名（接口模板体的函数字段名）；完整性检查
                // 仍由定义方 finalize 负责，分发查询只匹配 impl_type + args
                let methods = self
                    .env
                    .generic_type_defs
                    .get(head)
                    .map(|d| match &d.poly.body {
                        MonoType::Struct(s) => s
                            .fields
                            .iter()
                            .filter(|(_, t)| matches!(t, MonoType::Fn { .. }))
                            .map(|(n, _)| n.clone())
                            .collect(),
                        _ => Vec::new(),
                    })
                    .unwrap_or_default();
                interface_impls.push((
                    head.clone(),
                    super::environment::InterfaceImplEntry {
                        impl_type: name.to_string(),
                        args: mono_args,
                        methods,
                        native: false,
                    },
                ));
            }
        }
        if type_def.is_none() && sum_variants.is_empty() && interface_impls.is_empty() {
            return None;
        }
        Some(crate::frontend::module::TypeDefPayload {
            type_def,
            sum_variants,
            interface_impls,
        })
    }

    /// RFC-011a: 展开接口成员列表（递归支持接口继承，Self 延迟替换）。
    /// 返回 (方法名, 替换后签名)；同名成员后者覆盖前者。
    fn expand_interface_members(
        &mut self,
        interface_name: &str,
        args: &[MonoType],
        visiting: &mut Vec<String>,
        span: crate::util::span::Span,
    ) -> Result<Vec<(String, MonoType)>, ()> {
        use crate::frontend::core::parser::ast::Type;
        if visiting.iter().any(|v| v == interface_name) {
            // 循环继承：终止该分支展开
            return Ok(Vec::new());
        }
        visiting.push(interface_name.to_string());

        // RFC-011b 阶段 2: Try 接口（四成员）编译器侧展开
        if interface_name == super::operator_interfaces::TRY_NAME {
            if args.len() != 3 {
                self.add_error(
                    ErrorCodeDefinition::interface_arity_mismatch(interface_name, 3, args.len())
                        .at(span)
                        .build(),
                );
                return Err(());
            }
            visiting.pop();
            return Ok(super::operator_interfaces::expand_try_members(args));
        }

        // RFC-011b: 七个运算符接口走编译器侧规格（成员签名模板），
        // 不依赖用户源码声明体；其余接口照旧从 generic_type_defs + 声明体展开
        if let Some(spec) = super::operator_interfaces::spec(interface_name) {
            let param_names: Vec<String> = spec.params.iter().map(|s| s.to_string()).collect();
            if param_names.len() != args.len() {
                self.add_error(
                    ErrorCodeDefinition::interface_arity_mismatch(
                        interface_name,
                        param_names.len(),
                        args.len(),
                    )
                    .at(span)
                    .build(),
                );
                return Err(());
            }
            let sig = super::operator_interfaces::member_signature(spec);
            let substituted = TypeEnvironment::replace_type_params(&sig, &param_names, args);
            visiting.pop();
            return Ok(vec![(spec.method.to_string(), substituted)]);
        }

        let Some(param_names) = self
            .env
            .generic_type_defs
            .get(interface_name)
            .map(|d| d.type_param_names.clone())
        else {
            self.add_error(
                ErrorCodeDefinition::unknown_interface(interface_name)
                    .at(span)
                    .build(),
            );
            return Err(());
        };
        if param_names.len() != args.len() {
            self.add_error(
                ErrorCodeDefinition::interface_arity_mismatch(
                    interface_name,
                    param_names.len(),
                    args.len(),
                )
                .at(span)
                .build(),
            );
            return Err(());
        }
        let Some(body) = self.type_definition_bodies.get(interface_name).cloned() else {
            visiting.pop();
            return Ok(Vec::new());
        };

        let mut members: Vec<(String, MonoType)> = Vec::new();
        for item in &body {
            match item {
                TypeBodyItem::Field(f) => {
                    let ty = MonoType::from(f.ty.clone());
                    let ty = TypeEnvironment::replace_type_params(&ty, &param_names, args);
                    if !matches!(ty, MonoType::Fn { .. }) {
                        // RFC-011a §1: 接口成员必须是方法（函数类型字段）
                        self.add_error(
                            ErrorCodeDefinition::interface_method_mismatch(
                                interface_name,
                                &f.name,
                                "函数类型",
                                &ty.to_string(),
                            )
                            .at(span)
                            .build(),
                        );
                        return Err(());
                    }
                    members.push((f.name.clone(), ty));
                }
                TypeBodyItem::Expr(Type::Generic {
                    name: head,
                    args: nested_args,
                    ..
                }) => {
                    // 接口继承：先替换当前接口类型参数再递归展开（Self 延迟替换）
                    let nested: Vec<MonoType> = nested_args
                        .iter()
                        .map(|a| {
                            let m = MonoType::from(a.clone());
                            TypeEnvironment::replace_type_params(&m, &param_names, args)
                        })
                        .collect();
                    match self.expand_interface_members(head, &nested, visiting, span) {
                        Ok(sub) => members.extend(sub),
                        Err(()) => return Err(()),
                    }
                }
                _ => {}
            }
        }
        visiting.pop();
        Ok(members)
    }

    /// RFC-011a 阶段2: 对单个接口实例化做完整性检查并登记实现证明。
    fn check_interface_instantiation(
        &mut self,
        impl_type: &str,
        interface_name: &str,
        args: &[MonoType],
        span: crate::util::span::Span,
    ) -> Result<(), ()> {
        // RFC-011b 孤儿规则：实现跟随类型的定义模块。类型体内实例化的
        // impl_type 恒为外层类型，执法落点因此在 Self 位——运算符接口
        // `Add(X, R, O)` 的 X 必须与声明类型名义相同，用户无法借自己
        // 类型体的实例化给内建类型（Int/List 等）当 Self 补运算符。
        // （native 登记走 register_native_entries，不经此检查。）
        if super::operator_interfaces::spec(interface_name).is_some() {
            let self_head = args.first().and_then(|a| match a {
                MonoType::Struct(s) => Some(s.name.clone()),
                MonoType::TypeRef(n) => Some(n.clone()),
                MonoType::Generic { name, .. } => Some(name.clone()),
                // 内建标量经 from_builtin_name 物化（Add(Int,..) 的 Int → Int(64)）
                MonoType::Int(_) => Some("Int".to_string()),
                MonoType::Float(_) => Some("Float".to_string()),
                MonoType::Bool => Some("Bool".to_string()),
                MonoType::Char => Some("Char".to_string()),
                MonoType::Void => Some("Void".to_string()),
                _ => None,
            });
            if self_head.as_deref() != Some(impl_type) {
                let shown = self_head.unwrap_or_else(|| "?".to_string());
                self.add_error(
                    ErrorCodeDefinition::interface_impl_outside_defining_module(
                        &shown,
                        interface_name,
                    )
                    .at(span)
                    .build(),
                );
                return Err(());
            }
        }
        let mut visiting = Vec::new();
        let members = self.expand_interface_members(interface_name, args, &mut visiting, span)?;

        // §1.2 命名空间共享：接口方法名与实现类型已有字段同名 → 冲突
        let impl_field_names: Vec<String> = match self.env.types.get(impl_type) {
            Some(poly) => match &poly.body {
                MonoType::Struct(s) => s.fields.iter().map(|(n, _)| n.clone()).collect(),
                _ => Vec::new(),
            },
            None => Vec::new(),
        };
        for (member, _) in &members {
            if impl_field_names.contains(member) {
                self.add_error(
                    ErrorCodeDefinition::interface_member_conflict(impl_type, member)
                        .at(span)
                        .build(),
                );
                return Err(());
            }
        }

        // 完整性检查：每个接口成员须有同名实现且 Self 替换后签名一致。
        // RFC-011b: 按重载候选集匹配——同名方法存在多个签名（实例化级重载）
        // 时任一匹配即可；method_bindings 单值表会互相覆盖，不可用。
        let mut methods: Vec<String> = Vec::new();
        for (member, expected) in &members {
            let overload_key = format!("{}.{}", impl_type, member);
            let candidates = self
                .env
                .get_method_overloads(&overload_key)
                .cloned()
                .unwrap_or_default();
            if candidates.is_empty() {
                self.add_error(
                    ErrorCodeDefinition::interface_method_missing(
                        impl_type,
                        interface_name,
                        member,
                    )
                    .at(span)
                    .build(),
                );
                return Err(());
            }
            // impl 签名中的 Self 是 impl 类型的别名（RFC-011a §3）——
            // 两侧用同一实参替换后再比较，impl 写 &Self 或 &Dog 均匹配
            let mut matched = false;
            let mut last_found = None;
            for candidate in &candidates {
                let found =
                    TypeEnvironment::replace_type_params(candidate, &["Self".to_string()], args);
                if &found == expected {
                    matched = true;
                    break;
                }
                last_found = Some(found);
            }
            if !matched {
                let found = last_found.unwrap_or_else(|| candidates[0].clone());
                self.add_error(
                    ErrorCodeDefinition::interface_method_mismatch(
                        impl_type,
                        member,
                        &expected.to_string(),
                        &found.to_string(),
                    )
                    .at(span)
                    .build(),
                );
                return Err(());
            }
            methods.push(member.clone());
        }

        // 通过 → 生成实现证明（纯编译期概念，运行时擦除，§5.3/§6.4）
        self.env.implementation_proofs.push(ImplementationProof {
            type_name: impl_type.to_string(),
            interface_name: interface_name.to_string(),
            methods: methods.clone(),
        });

        // RFC-011b: 写入接口实现登记表（带类型实参维度）——
        // 运算符查询与约束求解的唯一判据，与 proof 同步落表
        self.env.add_interface_impl(
            interface_name,
            super::environment::InterfaceImplEntry {
                impl_type: impl_type.to_string(),
                args: args.to_vec(),
                methods,
                native: false,
            },
        );

        // 接口名追加进实现类型的 interfaces 面（LSP/阶段3 类型收集消费）。
        // types 与 vars 必须同步：构造器调用（Dog("Rex")）返回的是 vars 里的
        // StructType，只改 types 会让 structured-subtyping 臂（solver.rs:626）
        // 拿到空 interfaces，List(Animal)/标量存在类型赋值全部 E1002。
        let poly = self.env.types.get(impl_type);
        if let Some(poly) = poly {
            if let MonoType::Struct(s) = &poly.body {
                let mut s = s.clone();
                if !s.interfaces.iter().any(|i| i == interface_name) {
                    s.interfaces.push(interface_name.to_string());
                    let updated = PolyType {
                        type_binders: poly.type_binders.clone(),
                        const_binders: poly.const_binders.clone(),
                        body: MonoType::Struct(s),
                    };
                    self.env
                        .types
                        .insert(impl_type.to_string(), updated.clone());
                    self.env.vars.insert(impl_type.to_string(), updated);
                }
            }
        }
        Ok(())
    }

    /// RFC-004: 将 `Type.method = fn[pos]` 形态的绑定登记到 method_bindings。
    /// 函数尚未注册（前向引用）时静默跳过——与既有宽松行为一致。
    fn register_external_method_binding(
        &mut self,
        type_name: &str,
        method_name: &str,
        func_name: &str,
        positions: &[i64],
        span: crate::util::span::Span,
    ) {
        let found = self.env.get_var(func_name).map(|poly| {
            (
                match &poly.body {
                    MonoType::Fn { params, .. } => params.len(),
                    _ => 0,
                },
                poly.body.clone(),
            )
        });
        let Some((total, fn_ty)) = found else {
            return;
        };
        // #321 W1003：类型体外部绑定解析的导入函数视为已使用（pass2 解析）
        self.note_import_use(func_name);
        if let Some(positions) = self.normalize_binding_positions(positions, total, span) {
            let method_ty = Self::method_type_after_binding(&fn_ty, &positions);
            self.env
                .add_method_binding(type_name, method_name, method_ty.clone());
            // RFC-011b: 绑定形态同样进重载候选表（接口完整性检查按候选集匹配）
            let okey = format!("{}.{}", type_name, method_name);
            self.env.add_method_overload(&okey, method_ty);
        }
    }

    /// 从 Index 表达式的 index 部分提取位置列表
    fn extract_positions(index: &crate::frontend::core::parser::ast::Expr) -> Vec<i64> {
        crate::frontend::core::parser::ast::Expr::extract_binding_positions(index)
    }

    /// 将模块注册为 Struct 类型（包含所有导出作为字段）
    /// native/未知导出的宽松类型占位（fresh var -> Void）。
    ///
    /// ponytail: std native FFI 边界本就丢类型精度，用宽松占位是诚实的永久状态，
    /// 不是临时 hack。用户模块走 export.mono_type 的完整真实类型。
    fn loose_native_type(&mut self) -> MonoType {
        MonoType::Fn {
            params: vec![self.env.solver().new_var()],
            return_type: Box::new(MonoType::Void),
        }
    }

    /// 导出的注册类型：有真实类型用真实的（用户模块），否则走宽松占位（native）。
    fn export_register_type(
        &mut self,
        export: &crate::frontend::module::Export,
    ) -> MonoType {
        export
            .mono_type
            .clone()
            .unwrap_or_else(|| self.loose_native_type())
    }

    fn register_module_as_struct(
        &mut self,
        _module_path: &str,
        module_alias: &str,
        module: &crate::frontend::module::ModuleInfo,
    ) {
        let mut fields = Vec::new();
        for export in module.exports.values() {
            let field_ty = self.export_register_type(export);
            fields.push((export.name.clone(), field_ty));
        }
        let module_ty = MonoType::Struct(crate::frontend::core::types::mono::StructType {
            name: module_alias.to_string(),
            fields,
            methods: HashMap::new(),
            field_mutability: Vec::new(),
            field_has_default: Vec::new(),
            interfaces: vec![],
        });
        self.env.module_aliases.insert(module_alias.to_string());
        self.env
            .add_var(module_alias.to_string(), PolyType::mono(module_ty));
    }

    /// 注册单个导出
    /// - `use_alias`: 是否使用别名模式，为 true 时注册名为 prefix，否则为 export.name
    fn register_use_export(
        &mut self,
        prefix: &str,
        export: &crate::frontend::module::Export,
        use_alias: bool,
    ) {
        let register_name = if use_alias {
            prefix.to_string()
        } else {
            export.name.clone()
        };

        match export.kind {
            crate::frontend::module::ExportKind::SubModule => {
                // 子模块作为命名空间
                let sub_module_path = export.full_path.clone();
                let mut fields = Vec::new();
                let mut payload_type_exports = Vec::new();
                if let Some(sub_module) = self.env.module_registry.get(&sub_module_path).cloned() {
                    for sub_export in sub_module.exports.values() {
                        let field_ty = self.export_register_type(sub_export);
                        fields.push((sub_export.name.clone(), field_ty));
                        // 分组 use（`use std.{result}`）与整模块 use 同权：子模块内
                        // 带载荷的 Type 导出按裸名补镜像——裸名构造
                        // （`Result(Int, String)`）与 match 变体解构的变体集
                        // （sum_types）都依赖这份注册，与整模块导入臂同一规则。
                        if matches!(sub_export.kind, crate::frontend::module::ExportKind::Type)
                            && sub_export.type_payload.is_some()
                        {
                            payload_type_exports.push(sub_export.clone());
                        }
                    }
                }
                let module_ty = MonoType::Struct(crate::frontend::core::types::mono::StructType {
                    name: export.name.clone(),
                    fields,
                    methods: HashMap::new(),
                    field_mutability: Vec::new(),
                    field_has_default: Vec::new(),
                    interfaces: vec![],
                });
                // 分组导入的子模块命名空间（`use std.{result}` 后 `result.nope`）
                // 同样按模块语义报 E1043。
                self.env.module_aliases.insert(register_name.clone());
                self.env.add_var(register_name, PolyType::mono(module_ty));
                for sub_export in &payload_type_exports {
                    self.register_use_export(&sub_export.name, sub_export, false);
                }
            }
            crate::frontend::module::ExportKind::Type => {
                // 类型导出：镜像本地类型定义，同时注册到类型空间与值空间，
                // 使构造（Point(...)）与字段访问（p.x）能解析结构体。
                // 类型空间已有同名（native 类型家族经 with_std 预载，如 std.result
                // 的 Error）时不得用宽松类型覆盖值空间——否则该名字在值位置
                // 不再解析为类型名，和类型实参报 E1002。
                if self.env.get_var(&register_name).is_some()
                    || self.env.types.contains_key(&register_name)
                {
                    return;
                }
                let ty = self.export_register_type(export);
                let poly = PolyType::mono(ty);
                self.env.add_type(register_name.clone(), poly.clone());
                self.env.add_var(register_name.clone(), poly);
                // 跨模块类型传播：恢复定义方三张表（泛型模板 / 和类型变体 /
                // 接口实现登记）——变体构造、match 变体解构、`?` 与运算符
                // 分发的唯一判据都在这三张表里
                self.register_type_payload(&register_name, export);
            }
            _ => {
                // 如果变量已存在（比如已经是 Struct 类型），则跳过
                if self.env.get_var(&register_name).is_some() {
                    return;
                }
                let ty = self.export_register_type(export);
                self.env.add_var(register_name, PolyType::mono(ty));
            }
        }
    }

    /// 跨模块类型传播：把类型导出载荷写入导入方 env 的三张表
    /// （`generic_type_defs` / `sum_types` / `interface_impl_registry`）。
    /// 无载荷（普通记录类型导出、native 导出）为空操作。
    fn register_type_payload(
        &mut self,
        name: &str,
        export: &crate::frontend::module::Export,
    ) {
        let Some(payload) = &export.type_payload else {
            return;
        };
        if let Some(def) = &payload.type_def {
            self.env.add_generic_type_def(name.to_string(), def.clone());
        }
        if !payload.sum_variants.is_empty() {
            self.env
                .sum_types
                .insert(name.to_string(), payload.sum_variants.clone());
        }
        for (interface_name, entry) in &payload.interface_impls {
            self.env.add_interface_impl(interface_name, entry.clone());
        }
    }

    /// #321 W1003：登记导入的本地名（pass2 use elaboration 处调用；
    /// 重复登记由发射处按名去重，首次出现的位置优先呈现）
    fn record_import_name(
        &mut self,
        name: &str,
        span: crate::util::span::Span,
    ) {
        self.imported_names.push((name.to_string(), span));
        self.import_watch.insert(name.to_string(), name.to_string());
    }

    /// #321 W1003：整体导入（`use std.io` / `use std.io as i`）的成员监视——
    /// 导出成员名（print 等）解析命中同样使模块别名视为已使用，
    /// 覆盖"导入模块后裸调导出函数"的常见形态（native 注册路径）。
    fn watch_import_members(
        &mut self,
        alias: &str,
        module: &crate::frontend::module::ModuleInfo,
    ) {
        for name in module.exports.keys() {
            self.import_watch.insert(name.clone(), alias.to_string());
        }
    }

    /// #321 W1003：pass2 解析点命中监视集时标记对应导入已使用
    fn note_import_use(
        &mut self,
        name: &str,
    ) {
        if let Some(report_as) = self.import_watch.get(name) {
            let report_as = report_as.clone();
            self.import_used.insert(report_as);
        }
    }

    /// #321 W1003：汇总未使用导入（模块级 + 函数体级），产出 Warning 诊断。
    ///
    /// 使用判定三层并集：① 表达式解析（body_checker 导入监视集命中，作用域
    /// 感知）；② pass2 解析点（外部绑定等）；③ 语法级兜底
    /// [`DeadCodeAnalyzer::collect_ident_refs`]——match 模式、spawn 体、类型
    /// 注解等 Var 推断臂看不到的引用位置，宁多收（漏报方向）不漏收（误报方向）。
    /// 诊断进 TypeCheckResult.warnings 通道——混入 diagnostics 会被管线按错误
    /// 计数，破坏非阻断契约。
    fn collect_unused_import_warnings(
        &mut self,
        module: &crate::frontend::core::parser::ast::Module,
    ) -> Vec<Diagnostic> {
        let (body_imports, used_refs) = match self.body_checker.as_mut() {
            Some(bc) => bc.drain_import_tracking(),
            None => (Vec::new(), HashSet::new()),
        };
        let mut used = used_refs;
        // pass2 解析点（外部绑定等）标记的已使用名与 body 侧并集
        used.extend(std::mem::take(&mut self.import_used));
        // 语法级兜底：模式/类型注解/spawn 体等位置的标识符引用
        used.extend(super::passes::dead_code::DeadCodeAnalyzer::collect_ident_refs(module));
        let mut seen = HashSet::new();
        let mut warnings = Vec::new();
        for (name, span) in self.imported_names.iter().chain(body_imports.iter()) {
            if used.contains(name) || !seen.insert(name.clone()) {
                continue;
            }
            warnings.push(ErrorCodeDefinition::unused_import(name).at(*span).build());
        }
        warnings
    }

    /// 添加类型定义
    fn add_type_definition(
        &mut self,
        name: &str,
        definition: &crate::frontend::core::parser::ast::Type,
        signature_params: &[Param],
        span: crate::util::span::Span,
    ) {
        let generic_params = classify_generic_params(
            signature_params,
            // RFC-011b: 运算符接口名也是合法约束名（T: Add）——
            // 约束求解在实例化点查接口实现登记表
            &|name| self.env.has_trait(name) || super::operator_interfaces::spec(name).is_some(),
        );
        let param_names: Vec<String> = generic_params.iter().map(|p| p.name.clone()).collect();
        // RFC-010 Easter Egg: Type: Type = Type
        // 当用户尝试定义 Type 自身时，触发彩蛋
        if name == "Type" {
            // 检查 definition 是否引用了 Type
            let is_type_self_ref = match definition {
                // 情况1: definition 是 MetaType（Type: Type = ...）
                crate::frontend::core::parser::ast::Type::MetaType { .. } => true,
                // 情况2: definition 是 Name("Type")（Type: Type = Type）
                crate::frontend::core::parser::ast::Type::Name { name, .. } => name == "Type",
                // 情况3: definition 是 Generic { name: "Type", ... }（Type: Type = Type[T]）
                crate::frontend::core::parser::ast::Type::Generic {
                    name: type_name, ..
                } => type_name == "Type",
                // 情况4: definition 是 NamedStruct { name: "Type", ... }
                crate::frontend::core::parser::ast::Type::NamedStruct {
                    name: type_name, ..
                } => type_name == "Type",
                _ => false,
            };

            if is_type_self_ref {
                // 检查 type_annotation 是否有泛型参数（这表示 Type: Type[T] = ...）
                if !generic_params.is_empty() {
                    let decl = format!("Type: Type({}) = ...", param_names.join(", "));
                    self.add_error(
                        ErrorCodeDefinition::invalid_generic_self_reference(&decl)
                            .at(span)
                            .build(),
                    );
                    return;
                }
                // 无泛型参数 → 静默跳过（#161 宇宙分层决策）
                return;
            }
        }

        let poly = PolyType::mono(MonoType::from(definition.clone()));
        // Inject the type name into StructType if it's missing (plain Type::Struct has no name)
        let poly = PolyType::mono(match &poly.body {
            MonoType::Struct(s) if s.name.is_empty() => {
                MonoType::Struct(crate::frontend::core::types::mono::StructType {
                    name: name.to_string(),
                    fields: s.fields.clone(),
                    methods: s.methods.clone(),
                    field_mutability: s.field_mutability.clone(),
                    field_has_default: s.field_has_default.clone(),
                    interfaces: s.interfaces.clone(),
                })
            }
            _ => poly.body.clone(),
        });
        self.env.add_type(name.to_string(), poly.clone());
        // 同时注册到 vars，使类型名可以在表达式中使用（如 Point(1.0, 2.0) 构造器调用）
        self.env.add_var(name.to_string(), poly.clone());

        // RFC-004: 类型体内的方法绑定登记到 method_bindings（与语句级 Type.method = fn[pos]
        // 同一语义）。此前类型体 Binding 项只有 IR 侧登记，方法调用在类型检查层报 E1042。
        if let crate::frontend::core::parser::ast::Type::Struct { body } = definition {
            for item in body {
                if let crate::frontend::core::parser::ast::TypeBodyItem::Binding(b) = item {
                    match &b.kind {
                        crate::frontend::core::parser::ast::BindingKind::External { .. }
                        | crate::frontend::core::parser::ast::BindingKind::DefaultExternal {
                            ..
                        } => {
                            // 被绑函数可能在其后定义，延迟到 pass2 之后登记
                            self.pending_body_bindings
                                .push((name.to_string(), b.clone(), span));
                        }
                        crate::frontend::core::parser::ast::BindingKind::Anonymous {
                            params,
                            return_type,
                            positions,
                            ..
                        } => {
                            // 匿名绑定：函数类型来自内置声明的注解
                            let fn_ty = MonoType::Fn {
                                params: params
                                    .iter()
                                    .filter_map(|p| p.ty.as_ref())
                                    .map(|t| MonoType::from(t.clone()))
                                    .collect(),
                                return_type: Box::new(MonoType::from((**return_type).clone())),
                            };
                            if let Some(positions) =
                                self.normalize_binding_positions(positions, params.len(), span)
                            {
                                let method_ty = Self::method_type_after_binding(&fn_ty, &positions);
                                self.env.add_method_binding(name, &b.name, method_ty);
                            }
                        }
                    }
                }
            }
        }

        // RFC-027 Phase 2.5: 带约束表达式的类型定义同时注册为 proof 函数
        // IsPositive: (x: Int) -> Type = { x > 0 }
        // → 注册 `IsPositive` 为 `Fn { params: [Int], return_type: MetaType }`
        if let crate::frontend::core::parser::ast::Type::Struct { body } = definition {
            let has_constraint = body.iter().any(|item| {
                matches!(
                    item,
                    crate::frontend::core::parser::ast::TypeBodyItem::Expr(
                        crate::frontend::core::parser::ast::Type::ConstExpr(_)
                    )
                )
            });
            if has_constraint && !signature_params.is_empty() {
                let param_types: Vec<MonoType> = signature_params
                    .iter()
                    .filter_map(|p| p.ty.as_ref().map(|t| MonoType::from(t.clone())))
                    .collect();
                let fn_ty = MonoType::Fn {
                    params: param_types,
                    return_type: Box::new(MonoType::MetaType {
                        universe_level: crate::frontend::core::types::mono::UniverseLevel::type0(),
                        type_params: vec![],
                    }),
                };
                self.env.add_var(name.to_string(), PolyType::mono(fn_ty));
            }
        }

        // RFC-011a 阶段1: 类型体 AST 项留存（接口展开需要原始应用项，MonoType 转换会丢弃）
        if let crate::frontend::core::parser::ast::Type::Struct { body } = definition {
            self.type_definition_bodies
                .insert(name.to_string(), body.clone());
        }

        // RFC-011a 阶段1: 类型体应用项语义改判。
        // `Name(args)`（Expr(Generic)）命中已注册类型构造器且实参全为类型 → 接口实例化，
        // 记入待决队列，阶段 2 统一做 Self 替换 + 完整性检查；实参引用本声明的类型参数
        // （如接口继承体内的 Animal(Self)）为抽象链接，不记待检查（展开时延迟替换）。
        // 其余应用项维持 const 约束路径（Assert(N > 0) 等），行为不变。
        if let crate::frontend::core::parser::ast::Type::Struct { body } = definition {
            let enclosing_params: Vec<String> =
                generic_params.iter().map(|p| p.name.clone()).collect();
            for item in body {
                if let crate::frontend::core::parser::ast::TypeBodyItem::Expr(ty) = item {
                    if self.is_interface_application(ty) {
                        if let crate::frontend::core::parser::ast::Type::Generic {
                            name: head,
                            args,
                            ..
                        } = ty
                        {
                            let abstract_ref = args
                                .iter()
                                .any(|a| Self::type_arg_references(a, &enclosing_params));
                            // Self 位引用类型自身名字（`Index(Box(T), Int, T)` 的
                            // `Box(T)`）→ 泛型类型**自身的**接口实例化：注册为模板
                            // 条目（TypeRef 形参原样保留，派发查询按 scrutinee
                            // 实参结构对齐绑定后替换）。abstract_ref 的原本判定
                            // 是为接口继承链接（`Animal(Self)`）设计的，会误伤
                            // 此形态——Box(T) index 端到端缺口（#341）的根因。
                            let self_is_own_shape = args
                                .first()
                                .is_some_and(|a| Self::type_mentions_type(a, name));
                            if !abstract_ref {
                                self.pending_interface_instantiations.push(
                                    PendingInterfaceInstantiation {
                                        impl_type: name.to_string(),
                                        interface_name: head.clone(),
                                        args: args.clone(),
                                        span,
                                    },
                                );
                            } else if self_is_own_shape {
                                let mono_args: Vec<MonoType> =
                                    args.iter().map(|a| MonoType::from(a.clone())).collect();
                                let methods = Self::interface_template_methods(head, &self.env);
                                self.env.add_interface_impl(
                                    head,
                                    super::environment::InterfaceImplEntry {
                                        impl_type: name.to_string(),
                                        args: mono_args,
                                        methods,
                                        native: false,
                                    },
                                );
                            }
                        }
                    }
                }
            }
        }

        // 如果是泛型类型构造器（有泛型参数），存储模板信息用于类型实例化
        if !generic_params.is_empty() {
            use crate::frontend::core::typecheck::environment::GenericTypeDef;
            use crate::frontend::core::types::var::TypeVar;

            let type_param_names: Vec<String> = generic_params
                .iter()
                .filter(|p| matches!(p.kind, GenericParamKind::Type))
                .map(|p| p.name.clone())
                .collect();

            // const 判定：用途分析是唯一裁判（RFC-011 §4.1 精筛唯一实现在 const_param.rs）。
            let candidate_names: HashSet<String> = generic_params
                .iter()
                .filter(|p| matches!(p.kind, GenericParamKind::Const { .. }))
                .map(|p| p.name.clone())
                .collect();
            let used_as_const = collect_used_const_names(&candidate_names, definition);

            let resolved_const =
                crate::frontend::core::typecheck::const_param::resolve_const_candidates(
                    &generic_params,
                    signature_params,
                    &used_as_const,
                    type_param_names.len(),
                );

            // #297/F：落空 const 候选（标注具体类型但未在类型体任何类型位置引用）
            // 不能静默丢弃——否则实例化 arity 对不上，调用侧报风马牛不相及的 E1010。
            // 按 RFC-011 §4.1 编译期值参数判定：声明侧直接报错。
            for p in &resolved_const.fallen {
                self.add_error(
                    ErrorCodeDefinition::unused_const_param(&p.name, name)
                        .at(span)
                        .build(),
                );
            }

            let mut const_binders: Vec<ConstVarDef> = resolved_const.const_binders;

            // 从类型体收集待定证明义务到 const 参数
            // （接口实例化应用项已被分流，不走 const 约束路径）
            if let crate::frontend::core::parser::ast::Type::Struct { body } = definition {
                for item in body {
                    match item {
                        TypeBodyItem::Expr(ty) if !self.is_interface_application(ty) => {
                            process_body_expr_item(ty, &mut const_binders);
                        }
                        TypeBodyItem::Expr(_) => {}
                        TypeBodyItem::Field(f) => {
                            process_body_expr_item(&f.ty, &mut const_binders);
                        }
                        _ => {}
                    }
                }
            }

            let type_binders: Vec<TypeVar> =
                (0..type_param_names.len()).map(TypeVar::new).collect();

            let def = GenericTypeDef {
                poly: PolyType {
                    type_binders,
                    const_binders,
                    body: poly.body.clone(),
                },
                type_param_names,
            };
            self.env.add_generic_type_def(name.to_string(), def);
        }

        // 自动为 Record 类型派生标准库 traits
        self.auto_derive_traits(name, definition);

        // RFC-010: 记录式和类型判定（全有或全无，见「变体构造（权威定义）」节）
        if let Some(variants) = detect_sum_type(name, &param_names, definition) {
            self.env.sum_types.insert(name.to_string(), variants);
            // 和类型的泛型模板 body 归一化为 Generic 自身形态：值表示是
            // tagged union，不是函数字段 Struct——保留定义体会让注解实例化
            // `Option(Arc(Int))` 展开成构造器字段 Struct，与 Enum 值形态
            // 冲突（坑⑦，预置登记时代同一约定）。非泛型和类型无模板，跳过。
            if let Some(def) = self.env.generic_type_defs.get_mut(name) {
                def.poly.body = MonoType::Generic {
                    name: name.to_string(),
                    args: def
                        .type_param_names
                        .iter()
                        .map(|p| MonoType::TypeRef(p.clone()))
                        .collect(),
                };
            }
        }
    }
}

/// 精化谓词 / 证明函数的头名判定：`Terminates` 或「返回 `Type` 的函数」。
///
/// 这类应用（`Terminates(n)`、`IsPositive(5)`）的**实参位是值**，解析器与
/// 谓词解析器都按值处理；注解类型名校验因此不得下钻其实参，否则测度里的
/// 局部变量名会被误报为未知名类型（RFC-027 §6.9 / RFC-027a）。
fn is_predicate_head(
    name: &str,
    env: &crate::frontend::core::typecheck::environment::TypeEnvironment,
) -> bool {
    if name == "Terminates" {
        return true;
    }
    env.get_var(name).is_some_and(|poly| {
        matches!(
            &poly.body,
            crate::frontend::core::types::MonoType::Fn { return_type, .. }
                if matches!(return_type.as_ref(), crate::frontend::core::types::MonoType::MetaType { .. })
        )
    })
}

/// RFC-010 记录式和类型判定：字段全为函数且返回自身 → 变体定义表（声明序）。
/// 全有或全无——存在绑定项即不判定；任一字段返回非自身 → 普通记录（零诊断）。
fn detect_sum_type(
    name: &str,
    param_names: &[String],
    definition: &crate::frontend::core::parser::ast::Type,
) -> Option<Vec<super::environment::SumVariantDef>> {
    use crate::frontend::core::parser::ast::{Type, TypeBodyItem};
    let body = match definition {
        Type::Struct { body } => body,
        _ => return None,
    };
    let mut fields = Vec::new();
    for item in body {
        match item {
            TypeBodyItem::Field(f) => fields.push(f),
            // 方法绑定 = 携带行为的普通记录，不判定
            TypeBodyItem::Binding(_) => return None,
            _ => {}
        }
    }
    if fields.is_empty() {
        return None;
    }
    // ast 名字提取（只认裸名；形参引用在声明处即裸名）
    fn head_name(t: &Type) -> Option<&str> {
        match t {
            Type::Name { name, .. } => Some(name),
            _ => None,
        }
    }
    // 「返回自身」：return 的实参名字序列与形参表一致
    let returns_self = |ret: &Type| -> bool {
        let params_match = |args: &[Type]| -> bool {
            args.len() == param_names.len()
                && args
                    .iter()
                    .zip(param_names.iter())
                    .all(|(a, p)| head_name(a) == Some(p.as_str()))
        };
        match ret {
            // `Result(T, E)` 被 parser 特判为 Type::Result
            Type::Result(a, b) => {
                param_names.len() == 2
                    && head_name(a) == param_names.first().map(|s| s.as_str())
                    && head_name(b) == param_names.get(1).map(|s| s.as_str())
            }
            // `Option(T)` 特判为 Type::Option
            Type::Option(inner) => {
                param_names.len() == 1
                    && head_name(inner) == param_names.first().map(|s| s.as_str())
            }
            Type::Generic { name: n, args, .. } => n == name && params_match(args),
            // 非泛型：`Color` 或 `Self`
            Type::Name { name: n, .. } => param_names.is_empty() && (n == name || n == "Self"),
            _ => false,
        }
    };
    let mut variants = Vec::new();
    for f in fields {
        let Type::Fn {
            params,
            return_type,
        } = &f.ty
        else {
            return None;
        };
        if !returns_self(return_type) {
            return None;
        }
        variants.push(super::environment::SumVariantDef {
            name: f.name.clone(),
            params: params.iter().map(|t| MonoType::from(t.clone())).collect(),
        });
    }
    Some(variants)
}

/// 处理类型体中的类型表达式项，收集 const 约束到对应的 const binder。
/// 遍历类型表达式中的 Generic 类型（如 Assert(N < 100)），
/// 将含自由 const 变量的 ConstExpr 参数关联到对应 const binder 的 constraints。
fn process_body_expr_item(
    ty: &crate::frontend::core::parser::ast::Type,
    const_binders: &mut [ConstVarDef],
) {
    use crate::frontend::core::parser::ast::Type;
    if let Type::Generic { args, .. } = ty {
        for arg in args {
            if let Type::ConstExpr(expr) = arg {
                if let Some(const_expr) = convert_expr_to_const_expr(expr) {
                    if let Some(cv) = find_const_var_in_expr(expr, const_binders) {
                        const_binders[cv.index()].constraints.push(const_expr);
                    }
                }
            }
        }
    }
}

/// 在 Expr 树中查找引用的 const 变量名，返回对应的 ConstVar index
fn find_const_var_in_expr(
    expr: &Expr,
    const_binders: &[ConstVarDef],
) -> Option<crate::frontend::core::types::var::ConstVar> {
    match expr {
        Expr::Var(name, _) => const_binders
            .iter()
            .position(|b| b.name == *name)
            .map(crate::frontend::core::types::var::ConstVar::new),
        Expr::BinOp { left, right, .. } => find_const_var_in_expr(left, const_binders)
            .or_else(|| find_const_var_in_expr(right, const_binders)),
        Expr::UnOp { expr, .. } => find_const_var_in_expr(expr, const_binders),
        _ => None,
    }
}

/// 用途分析：扫描类型定义体，找出候选参数中哪些**被当编译期值/类型使用**。
///
/// 这是 const 判定的唯一裁判——取代旧的"CONST_PARAM_TYPES 名单 + 大小写猜测"。
/// 用途 = 参数名出现在以下任一编译期位置：
/// - `Array(T, N)` 里作为泛型实参的裸 `Name`（Generic.args）
/// - `Assert(N > 0)` 里的 `ConstExpr` 表达式引用
/// - `length: N` 里作为字段类型标注的裸 `Name`
fn collect_used_const_names(
    candidates: &HashSet<String>,
    definition: &crate::frontend::core::parser::ast::Type,
) -> HashSet<String> {
    use crate::frontend::core::parser::ast::Type;
    let mut found = HashSet::new();
    if let Type::Struct { body } = definition {
        for item in body {
            match item {
                TypeBodyItem::Expr(ty) => {
                    collect_used_in_type(ty, candidates, &mut found, &|_| false)
                }
                TypeBodyItem::Field(f) => {
                    collect_used_in_type(&f.ty, candidates, &mut found, &|_| false)
                }
                _ => {}
            }
        }
    }
    found
}

/// 递归扫描类型表达式，收集被引用的候选参数名。
///
/// `is_predicate` 判定「该类型应用名是否解析为编译期谓词」——谓词应用的
/// 实参位是**编译期表达式操作数**（`Terminates(b)` 的 `b`），不是类型引用，
/// 必须跳过（RFC-027a T1）。
pub(crate) fn collect_used_in_type(
    ty: &crate::frontend::core::parser::ast::Type,
    candidates: &HashSet<String>,
    found: &mut HashSet<String>,
    is_predicate: &dyn Fn(&str) -> bool,
) {
    use crate::frontend::core::parser::ast::Type;
    match ty {
        Type::Name { name, .. } if candidates.contains(name) => {
            found.insert(name.clone());
        }
        Type::ConstExpr(expr) => collect_used_in_expr(expr, candidates, found),
        Type::Generic { name, args, .. } => {
            // 谓词/证明函数应用：实参是编译期表达式，不是类型引用。
            //
            // `Terminates(b)` 的 `b`、`IsPositive(n)` 的 `n` 都是**值**——
            // 把它们计入 const 泛型用途会让形参名被当作 const 绑定并被替换成
            // 底层具体类型（`b` → `Int(64)`），精化约束/测度在到达消费端
            // 前就丢了（E1092）。对照 `Array(T, N)`：`Array` 不是谓词，
            // 其 `N` 确实是 const 泛型引用，照常计入。
            if is_predicate(name) {
                return;
            }
            for arg in args {
                collect_used_in_type(arg, candidates, found, is_predicate);
            }
        }
        Type::Fn {
            params,
            return_type,
        } => {
            for p in params {
                collect_used_in_type(p, candidates, found, is_predicate);
            }
            collect_used_in_type(return_type, candidates, found, is_predicate);
        }
        Type::Option(inner) | Type::Ptr(inner) => {
            collect_used_in_type(inner, candidates, found, is_predicate);
        }
        Type::Ref { inner, .. } => collect_used_in_type(inner, candidates, found, is_predicate),
        Type::Result(a, b) => {
            collect_used_in_type(a, candidates, found, is_predicate);
            collect_used_in_type(b, candidates, found, is_predicate);
        }
        Type::Tuple(types) | Type::Sum(types) => {
            for t in types {
                collect_used_in_type(t, candidates, found, is_predicate);
            }
        }
        _ => {}
    }
}

/// 递归扫描编译期表达式（如 `N > 0`），收集被引用的候选参数名。
fn collect_used_in_expr(
    expr: &Expr,
    candidates: &HashSet<String>,
    found: &mut HashSet<String>,
) {
    match expr {
        Expr::Var(name, _) if candidates.contains(name) => {
            found.insert(name.clone());
        }
        Expr::BinOp { left, right, .. } => {
            collect_used_in_expr(left, candidates, found);
            collect_used_in_expr(right, candidates, found);
        }
        Expr::UnOp { expr, .. } => collect_used_in_expr(expr, candidates, found),
        _ => {}
    }
}
impl TypeChecker {
    /// 为 Record 类型自动派生标准库 traits
    ///
    /// 规则：
    /// 1. Record 的所有字段都实现了某 trait → 自动派生该 trait
    /// 2. 显式定义的方法会覆盖自动派生
    fn auto_derive_traits(
        &mut self,
        type_name: &str,
        definition: &crate::frontend::core::parser::ast::Type,
    ) {
        // 提取字段列表
        let fields: Vec<crate::frontend::core::parser::ast::StructField> = match definition {
            crate::frontend::core::parser::ast::Type::NamedStruct { fields, .. } => fields.clone(),
            crate::frontend::core::parser::ast::Type::Struct { body } => body
                .iter()
                .filter_map(|it| {
                    if let crate::frontend::core::parser::ast::TypeBodyItem::Field(f) = it {
                        Some(f.clone())
                    } else {
                        None
                    }
                })
                .collect(),
            _ => return,
        };

        // 获取 trait 表的引用（用于检查）
        let trait_table = &self.env.trait_table;

        // 为每个内置可派生 trait 尝试自动派生
        let mut impls_to_add = Vec::new();

        for trait_name in TraitTable::BUILTIN_DERIVES {
            // 检查是否可以自动派生
            let can_derive = trait_table.can_auto_derive(trait_name, &fields);

            if can_derive {
                // 检查是否已有显式实现
                if !self.env.has_trait_impl(trait_name, type_name) {
                    // 生成自动派生实现
                    if let Some(impl_) = TraitTable::generate_auto_derive(type_name, trait_name) {
                        impls_to_add.push(impl_);
                    }
                }
            }
        }

        // 批量添加实现（避免借用冲突）
        for impl_ in impls_to_add {
            // auto_derive 已有 has_trait_impl 前置检查，这里不会冲突
            let _ = self.env.add_trait_impl(impl_);
        }
    }

    // ============ 语义信息收集 ============

    /// 从已完成类型检查的 AST 收集语义 tokens
    ///
    /// 利用 typecheck 阶段已有的类型信息，一次遍历产出语义数据。
    /// 收集规则：
    /// - StmtKind::Binding   → Function/Type (定义，区分 type_annotation)
    /// - StmtKind::Var       → Variable (定义)
    /// - StmtKind::Binding   → Method/Type/Function (通过字段区分)
    /// - StmtKind::Use       → Namespace (引用)
    /// - Param               → Parameter (定义)
    /// - Expr::Var           → Variable (引用)
    /// - Expr::Call          → Function (引用)
    /// - Expr::FieldAccess   → Property (引用)
    /// - Expr::Cast          → Type (引用)
    fn constructor_names_from_module(_module: &Module) -> HashSet<String> {
        // Variant 语法已废弃（RFC-010，issue #203）。
        // 类型定义的构造器识别由下游类型系统统一处理，此处保留空集合作占位，
        // 避免破坏 semantic_tokens 中 EnumMember 识别链路。
        HashSet::new()
    }

    fn add_use_module_root(
        &self,
        imported_module_roots: &mut HashSet<String>,
        path: &str,
        items: &Option<Vec<crate::frontend::core::parser::ast::SpannedIdent>>,
        alias: &Option<Vec<String>>,
    ) {
        if items.is_some() {
            return;
        }

        if self.env.module_registry.has_module(path) {
            if let Some(aliases) = alias {
                if aliases.len() == 1 {
                    imported_module_roots.insert(aliases[0].clone());
                    return;
                }
            }

            if let Some(last) = path.split('.').next_back() {
                if !last.is_empty() {
                    imported_module_roots.insert(last.to_string());
                }
            }
            return;
        }

        // use std.io.print / use std.io.print as p 属于符号导入，不是命名空间根
        if let Some(dot_pos) = path.rfind('.') {
            let module_path = &path[..dot_pos];
            if self.env.module_registry.has_module(module_path) {
                return;
            }
        }

        // 回退策略：未知路径按旧行为处理
        if let Some(aliases) = alias {
            if aliases.len() == 1 {
                imported_module_roots.insert(aliases[0].clone());
                return;
            }
        }
        if let Some(last) = path.split('.').next_back() {
            if !last.is_empty() {
                imported_module_roots.insert(last.to_string());
            }
        }
    }
    // ============ RFC-027 阶段 1：编译期谓词集成 ============

    /// 判定一个类型应用名是否解析为**编译期谓词**（RFC-027 §6.9 / RFC-027a）。
    ///
    /// 谓词应用的实参位是编译期表达式（值），不是类型引用——`Terminates(b)` 的
    /// `b`、`IsPositive(n)` 的 `n` 都是值。这些位置不得参与 const 泛型用途分析。
    ///
    /// 判据取两条同源路径（与 `resolve_type_annotation` 一致）：
    /// 1. 已在 `predicate_defs` 注册的谓词
    /// 2. 返回 `Type` 的证明函数（如 `IsPositive: (x: Int) -> Type = { x > 0 }`）
    /// 3. `Terminates`——内置谓词，无源码声明（RFC-027 §6.9）
    fn is_predicate_application(
        &self,
        name: &str,
    ) -> bool {
        if name == "Terminates" {
            return true;
        }
        if self.env.predicate_defs.contains_key(name) {
            return true;
        }
        matches!(
            self.env.get_var(name).map(|p| &p.body),
            Some(MonoType::Fn { return_type, .. })
                if matches!(return_type.as_ref(), MonoType::MetaType { .. })
        )
    }

    /// 把 `Terminates` 精化的占位 base 换成**新类型变量**（RFC-027a）。
    ///
    /// `resolve_type_annotation` 是 `&self`，不能 mint 类型变量，故用 `Void`
    /// 作占位；此处（`&mut self`）就地替换，使 `Terminates` 标注**不约束**
    /// 被标注计算的值类型：`gcd` 返 `Int`、`is_even` 返 `Bool` 都能与 base 统一。
    /// 若不用占位而取固定 `Int`，返回 `Bool` 的函数会撞 E1002。
    fn resolve_terminates_base(
        &mut self,
        ty: MonoType,
    ) -> MonoType {
        match ty {
            MonoType::Refined { base, constraint } => {
                let is_terminates = matches!(
                    &constraint,
                    ConstExpr::Call { func, .. } if func == "Terminates"
                );
                if is_terminates && matches!(base.as_ref(), MonoType::Void) {
                    return MonoType::Refined {
                        base: Box::new(self.env.solver().new_var()),
                        constraint,
                    };
                }
                MonoType::Refined { base, constraint }
            }
            MonoType::Fn {
                params,
                return_type,
            } => MonoType::Fn {
                params: params
                    .into_iter()
                    .map(|p| self.resolve_terminates_base(p))
                    .collect(),
                return_type: Box::new(self.resolve_terminates_base(*return_type)),
            },
            other => other,
        }
    }

    /// 解析类型标注：如果是编译期谓词调用，正格化为 Refined（#263：非法用法写诊断汇入 diags，不静默）
    ///
    /// Generic("Positive", [arg]) -> 尝试 PredicateResolver::try_resolve
    /// 如果不是已知的编译期谓词，检查是否是证明函数
    fn resolve_type_annotation(
        &self,
        ty: &MonoType,
        diags: &mut Vec<Diagnostic>,
    ) -> MonoType {
        match ty {
            MonoType::Generic { name, args } if !args.is_empty() => {
                // RFC-027a：`Terminates(m)` —— 终止测度标注（一元形态）。
                //
                // `Terminates` 本身不是类型，而是**内置谓词**（RFC-027 §6.9，与
                // `Int`/`Never` 同属核心原语）：它把测度 `m` 绑定到所在类型位标注的
                // 那段计算上，声明该计算终止。故正格化为 `Refined`，基类型取
                // **`Void`**——测度是编译期见证，随 witness 擦除，不参与运行时类型。
                //
                // 为何是 `Void` 而非 `Int`：测度返回类型不受限（RFC-027a §非目标：
                // 不强制自然数），且 `Terminates` 标注的计算**不产出值**
                // （函数形态下它就是函数自身的终止性声明）。取 `Int` 会让
                // `gcd(...) -> Terminates(b)` 的函数返回类型错变成 Int。
                //
                // 归档到 `Refined` 的收益：`resolves_type_name` 不必再特判，
                // 且下游已有「`Refined` 取 `base` 做 unify」的成熟路径
                // （inference/statements.rs），无需新机制。
                if name == "Terminates" {
                    if args.len() != 1 {
                        // 元数不对：二元形态（`Terminates(fn_ty, m)`）尚未实现
                        // （见 RFC-027a D1 park），多余实参不得静默忽略。
                        // 复用 E1093（精化类型实参个数不匹配）——同为「精化谓词
                        // 实参数量不对」类，不新增码位。
                        diags.push(
                            ErrorCodeDefinition::refined_arity_mismatch(
                                "Terminates",
                                1,
                                args.len(),
                            )
                            .build(),
                        );
                        return ty.clone();
                    }
                    // 测度实参转 `ConstExpr`（与证明函数路径同一转换）
                    let const_args: Option<Vec<ConstExpr>> = args
                        .iter()
                        .map(|a| self.mono_type_to_const_expr(a))
                        .collect();
                    let Some(const_args) = const_args else {
                        // 实参形态不可转换（如嵌套泛型）——与证明函数路径同码 #263
                        diags
                            .push(ErrorCodeDefinition::refined_arg_not_const("Terminates").build());
                        return ty.clone();
                    };
                    return MonoType::Refined {
                        // 占位 base：`Terminates(b)` 写在返回位时**没有**声明值类型
                        //（测度只是终止性见证，不约束返回值——RFC-027 §6.9：
                        // `Terminates(m)` 精化的是「该计算的值类型」自身）。
                        // 调用方（`collect_function_signature`）会把此占位 base
                        // 换成新类型变量，使其可与函数体的真实返回类型统一。
                        base: Box::new(MonoType::Void),
                        constraint: ConstExpr::Call {
                            func: "Terminates".into(),
                            args: const_args,
                        },
                    };
                }
                // 尝试原有 PredicateResolver（三值结果，#263）
                match PredicateResolver::try_resolve(&self.env, name, args) {
                    Some(Ok(refined)) => return refined,
                    Some(Err(err)) => {
                        // #263：已注册谓词非法用法——汇入诊断，绝不静默放行
                        diags.push(Self::refined_usage_diagnostic(name, &err));
                        return ty.clone();
                    }
                    None => {} // 不是谓词——继续证明函数路径
                }
                // Phase 2.5: 检查是否是证明函数（源码定义的返回 Type 的函数）
                match self.lookup_proof_fn_base_type(name, args) {
                    Some(Ok(base)) => {
                        // 将参数转换为 ConstExpr
                        let const_args: Option<Vec<ConstExpr>> = args
                            .iter()
                            .map(|a| self.mono_type_to_const_expr(a))
                            .collect();
                        match const_args {
                            Some(const_args) => {
                                let constraint = ConstExpr::Call {
                                    func: name.clone(),
                                    args: const_args,
                                };
                                MonoType::Refined {
                                    base: Box::new(base),
                                    constraint,
                                }
                            }
                            None => {
                                // #263：证明函数实参形态不可转换——约束无法生成，汇入诊断（E1092）
                                diags
                                    .push(ErrorCodeDefinition::refined_arg_not_const(name).build());
                                ty.clone()
                            }
                        }
                    }
                    Some(Err(err)) => {
                        // #263：证明函数实参个数不匹配——汇入诊断（E1093）
                        diags.push(Self::refined_usage_diagnostic(name, &err));
                        ty.clone()
                    }
                    None => ty.clone(), // 非证明函数——保持原样
                }
            }
            _ => ty.clone(),
        }
    }

    /// 将谓词/证明函数用法非法映射为诊断（#263：结构化错误，i18n 文案不混排）
    fn refined_usage_diagnostic(
        name: &str,
        err: &crate::frontend::core::typecheck::predicate_resolver::PredicateResolveError,
    ) -> Diagnostic {
        use crate::frontend::core::typecheck::predicate_resolver::PredicateResolveError;
        match err {
            PredicateResolveError::ArityMismatch { expected, found } => {
                ErrorCodeDefinition::refined_arity_mismatch(name, *expected, *found).build()
            }
            PredicateResolveError::ArgNotConst => {
                ErrorCodeDefinition::refined_arg_not_const(name).build()
            }
        }
    }

    /// 查找证明函数的基类型（三值结果，#263）
    ///
    /// 检查 `name` 是否在环境中定义为返回 Type 的函数：
    /// - `None`：不是证明函数（调用方继续其他解析路径）
    /// - `Some(Ok(base))`：是证明函数，返回第一个参数的类型作为基类型
    /// - `Some(Err(err))`：是证明函数但实参个数不匹配（#263：不得静默放行）
    fn lookup_proof_fn_base_type(
        &self,
        name: &str,
        args: &[MonoType],
    ) -> Option<
        Result<
            MonoType,
            crate::frontend::core::typecheck::predicate_resolver::PredicateResolveError,
        >,
    > {
        // 查找函数定义
        let poly = self.env.get_var(name)?;
        let fn_ty = &poly.body;

        // 检查是否是函数类型，且返回类型是 MetaType（Type）
        if let MonoType::Fn {
            params,
            return_type,
        } = fn_ty
        {
            // 检查返回类型是否是 MetaType（表示返回 Type）
            if matches!(return_type.as_ref(), MonoType::MetaType { .. })
                || matches!(return_type.as_ref(), MonoType::TypeRef(ref name) if name == "Type")
            {
                // #263：实参个数不匹配是非法用法，不得与「不是证明函数」混淆
                if params.len() != args.len() {
                    return Some(Err(
                        crate::frontend::core::typecheck::predicate_resolver::PredicateResolveError::ArityMismatch {
                            expected: params.len(),
                            found: args.len(),
                        },
                    ));
                }
                // 返回第一个参数的类型作为基类型（此处 params 与 args 等长且非空）
                return Some(Ok(params[0].clone()));
            }
        }
        None
    }

    /// 将 MonoType 转换为 ConstExpr
    ///
    /// 用于将类型参数转换为约束表达式中的常量表达式。
    fn mono_type_to_const_expr(
        &self,
        ty: &MonoType,
    ) -> Option<ConstExpr> {
        match ty {
            // 字面量值
            MonoType::Literal { value, .. } => Some(ConstExpr::Lit(value.clone())),
            // 变量引用
            MonoType::TypeRef(name) => Some(ConstExpr::NamedVar(name.clone())),
            // 递归处理 Generic 中的参数
            MonoType::Generic { name: _, args } if args.len() == 1 => {
                self.mono_type_to_const_expr(&args[0])
            }
            _ => None,
        }
    }

    /// 常量折叠：表达式在字面量值环境下求成具体值（精化值环境的数据源）
    ///
    /// 复用 convert_expr_to_const_expr + Evaluator——与 check_predicate
    /// 第 1 级同一台求值器，折叠语义与约束判定严格一致。转换/求值失败
    /// （变量值未知、调用等非内核形态）→ None：值回到「未知」，走
    /// 「缺值不送检」路径，绝不用陈旧值凑数。
    fn fold_value(
        ctx: &crate::frontend::core::typecheck::proof::context::ProofContext<'_>,
        expr: &Expr,
        values: &HashMap<String, ConstValue>,
    ) -> Option<ConstValue> {
        let const_expr = convert_expr_to_const_expr(expr)?;
        let mut evaluator = crate::frontend::core::types::eval::evaluator::Evaluator::new(
            ctx.env,
            &ctx.budget,
            &ctx.dep_env,
        );
        evaluator.eval_expr(&const_expr, values).ok()
    }

    /// 条件在值环境下可判定为 Bool（if 链守卫裁剪的判据）
    fn fold_bool(
        ctx: &crate::frontend::core::typecheck::proof::context::ProofContext<'_>,
        expr: &Expr,
        values: &HashMap<String, ConstValue>,
    ) -> Option<bool> {
        match Self::fold_value(ctx, expr, values)? {
            ConstValue::Bool(b) => Some(b),
            _ => None,
        }
    }

    /// 两阶段精化类型检查（RFC-027 Phase 3.1）
    ///
    /// 阶段 1：遍历模块，构建 TypeDepGraph + 检查初始化绑定
    /// 阶段 2：遍历赋值点，查询依赖图，生成 VC
    /// 收集带精化标注的**变量名**（RFC-027 §7 验证模式的判据）
    ///
    /// 只做「该绑定是否解析为精化类型」的判定，不复用
    /// `collect_refined_binding_checks`——后者会顺带执行证明调用与依赖图构建，
    /// 此处重复调用会产生两遍副作用。故独立走一遍轻量遍历。
    ///
    /// 只处理变量绑定（`y: T = ...`）与函数签名（参数/返回类型精化，供递归
    /// 终止使用）。判据是 `resolve_type_annotation` 的结果为 `Refined`——与
    /// 生成 E4018 的那条路径同一真相源。
    fn collect_refined_var_names(
        &self,
        module: &Module,
    ) -> std::collections::HashSet<String> {
        let mut names = std::collections::HashSet::new();
        for stmt in &module.items {
            // #324：本遍历会调用 resolve_type_annotation，它会发 E1092/E1093 类
            // 需 span 的诊断。这些诊断此处丢弃（另一遍负责报出），但**构造**过程
            // 在 debug 下会因缺 span 而 panic——故须挂 walk 上下文，与
            // collect_refined_binding_checks 同款。
            let _span_guard = crate::util::diagnostic::push_current_span(stmt.span);
            self.collect_refined_var_names_in_stmt(stmt, &mut names);
        }
        names
    }

    /// 递归收集单条语句中的精化变量名（含函数体内的嵌套绑定）
    fn collect_refined_var_names_in_stmt(
        &self,
        stmt: &crate::frontend::core::parser::ast::Stmt,
        names: &mut std::collections::HashSet<String>,
    ) {
        use crate::frontend::core::parser::ast::{Expr, StmtKind};

        // 轻量解析：只关心「注解是否解析为 Refined」，诊断丢弃（另一遍会报）
        let mut sink = Vec::new();

        match &stmt.kind {
            StmtKind::Assign {
                target,
                type_annotation: Some(type_ann),
                value,
                ..
            } => {
                if let Expr::Var(name, _) = target.as_ref() {
                    let mono_ty = MonoType::from(type_ann.clone());
                    let resolved = self.resolve_type_annotation(&mono_ty, &mut sink);
                    if matches!(resolved, MonoType::Refined { .. }) {
                        names.insert(name.clone());
                    }
                }
                // 函数体内的嵌套精化绑定同样计入
                if let Some(expr) = value.as_deref() {
                    self.collect_refined_var_names_in_expr(expr, names);
                }
            }
            StmtKind::Expr(expr) => self.collect_refined_var_names_in_expr(expr, names),
            StmtKind::If {
                then_branch,
                else_if_branches,
                else_branch,
                ..
            } => {
                for s in &then_branch.stmts {
                    self.collect_refined_var_names_in_stmt(s, names);
                }
                for (_, body) in else_if_branches {
                    for s in &body.stmts {
                        self.collect_refined_var_names_in_stmt(s, names);
                    }
                }
                if let Some(eb) = else_branch {
                    for s in &eb.stmts {
                        self.collect_refined_var_names_in_stmt(s, names);
                    }
                }
            }
            _ => {}
        }
    }

    /// 递归收集表达式内的精化绑定（块/循环体/函数体）
    fn collect_refined_var_names_in_expr(
        &self,
        expr: &crate::frontend::core::parser::ast::Expr,
        names: &mut std::collections::HashSet<String>,
    ) {
        use crate::frontend::core::parser::ast::Expr;

        match expr {
            Expr::Block(block) => {
                for s in &block.stmts {
                    self.collect_refined_var_names_in_stmt(s, names);
                }
            }
            Expr::While { body, .. } | Expr::For { body, .. } => {
                for s in &body.stmts {
                    self.collect_refined_var_names_in_stmt(s, names);
                }
            }
            Expr::Lambda { body, .. } => {
                for s in &body.stmts {
                    self.collect_refined_var_names_in_stmt(s, names);
                }
            }
            _ => {}
        }
    }

    /// 收集全模块的**显式测度**表：被标注的绑定/函数名 → 测度表达式（RFC-027a §2）。
    ///
    /// 从 **AST** 提取而不从已解析的 `MonoType` 取：`MonoType::from` 是有损
    /// 转换，T1 调查中它把 `Terminates(b)` 的测度解析成 `Int(64)`，测度表达式
    /// 在到达消费端前就丢了。AST 是测度的无损来源。
    ///
    /// 两种标注位都收（RFC-027a §2.1）：
    /// - 函数返回类型位 `gcd: (a: Int, b: Int) -> Terminates(b)` → 键 `gcd`
    /// - 变量绑定位 `acc: Terminates(n - i) = ...` → 键 `acc`
    ///
    /// 无 `Terminates` 标注时返回空表。
    /// 收集函数**前置条件**：各函数带精化的形参约束（RFC-027a §良基性）。
    ///
    /// 源是 env 里已解析的函数类型——形参位已由 `resolve_type_annotation` 规范化
    /// 为 `Refined`，且谓词定义注册后约束是**谓词体**（`b: NonNegative(b)` →
    /// `b >= 0`）。故这里取的与判定同一份形态，不必再从 AST 提取。
    ///
    /// 键 = 函数名，值 = 其精化形参的约束；`Terminates` 除外（那是终止测度，
    /// 不是值约束）。
    pub(crate) fn collect_param_refinements(
        &self
    ) -> std::collections::HashMap<String, Vec<crate::frontend::core::types::const_data::ConstExpr>>
    {
        let mut out: std::collections::HashMap<String, Vec<_>> = std::collections::HashMap::new();
        for (name, poly) in &self.env.vars {
            let MonoType::Fn { params, .. } = &poly.body else {
                continue;
            };
            let constraints: Vec<_> = params
                .iter()
                .filter(|p| matches!(p, MonoType::Refined { .. }) && !constraint_is_terminates(p))
                .filter_map(|p| match p {
                    MonoType::Refined { constraint, .. } => Some(constraint.clone()),
                    _ => None,
                })
                .collect();
            if !constraints.is_empty() {
                out.insert(name.clone(), constraints);
            }
        }
        out
    }

    /// 登记编译期谓词定义（#377-3）。
    ///
    /// 谓词 = 返回 `Type` 的**单参数**声明，其体表达式即约束模板：
    ///
    /// ```text
    /// IsPositive: (x: Int) -> Type = { x > 0 }
    /// ```
    ///
    /// 登记后 `IsPositive(-5)` 解析为 `Refined { constraint: -5 > 0 }` 而非
    /// 不透明应用 `IsPositive(-5)`。这是精化约束第一次**可符号推理**的前提
    /// （SMT 与假设合证；终止检查良基性 `b >= 0` 就依赖它）。
    ///
    /// 两种声明形态都收（与 pipeline 执行证明函数的取法同源）：
    /// - `TypeDefinition`：`= { x > 0 }` 被解析成类型体，约束在
    ///   `Type::Struct` 的 `TypeBodyItem::Expr(Type::ConstExpr)` 里（主形态）
    /// - `Assign`：函数体形态
    ///
    /// 只登记单参数形态：`PredicateResolver::try_resolve` 的阶段 1 模型即单参数，
    /// 登记多参数谓词会使其调用一律撞 ArityMismatch（E1093）——保持其走证明
    /// 函数路径（现行为）。约束体不可转为常量表达式的声明不登记也不报错
    /// （它只是普通函数）。
    fn collect_predicate_defs(
        &mut self,
        module: &Module,
    ) {
        use crate::frontend::core::parser::ast::{Expr, StmtKind, Type, TypeBodyItem};
        for stmt in &module.items {
            let (name, signature_params, constraint) = match &stmt.kind {
                StmtKind::TypeDefinition {
                    name,
                    signature_params,
                    definition,
                    ..
                } => {
                    let Type::Struct { body } = definition else {
                        continue;
                    };
                    let Some(expr) = body.iter().find_map(|item| match item {
                        TypeBodyItem::Expr(Type::ConstExpr(e)) => Some(e.as_ref()),
                        _ => None,
                    }) else {
                        continue;
                    };
                    let Some(c) = convert_expr_to_const_expr(expr) else {
                        continue;
                    };
                    (name, signature_params, c)
                }
                StmtKind::Assign {
                    target,
                    type_annotation: Some(Type::Fn { return_type, .. }),
                    signature_params,
                    value: Some(v),
                    ..
                } => {
                    let Expr::Var(name, _) = target.as_ref() else {
                        continue;
                    };
                    // 返回 Type 才是谓词（`MetaType` 与 `Name{Type}` 两形态同义）
                    let returns_type = matches!(return_type.as_ref(), Type::MetaType { .. })
                        || matches!(return_type.as_ref(), Type::Name { name: n, .. } if n == "Type");
                    if !returns_type {
                        continue;
                    }
                    let Some(body) = extract_fn_body(Some(v.as_ref())) else {
                        continue;
                    };
                    let Some(expr) = body.stmts.iter().find_map(|s| match &s.kind {
                        StmtKind::Expr(e) => Some(e.as_ref()),
                        StmtKind::Return(Some(e)) => Some(e.as_ref()),
                        _ => None,
                    }) else {
                        continue;
                    };
                    let Some(c) = convert_expr_to_const_expr(expr) else {
                        continue;
                    };
                    (name, signature_params, c)
                }
                _ => continue,
            };
            // 参数名在 signature_params（`Type::Fn::params` 只有类型没有名）
            let [param] = signature_params.as_slice() else {
                continue;
            };
            let param_type = param
                .ty
                .as_ref()
                .map(|t| MonoType::from(t.clone()))
                .unwrap_or(MonoType::Int(64));
            self.env.predicate_defs.insert(
                name.clone(),
                crate::frontend::core::typecheck::predicate_resolver::PredicateDef {
                    param_name: param.name.clone(),
                    param_type,
                    constraint,
                },
            );
        }
    }
    pub(crate) fn collect_termination_measures(
        &self,
        module: &Module,
    ) -> std::collections::HashMap<String, crate::frontend::core::types::const_data::ConstExpr>
    {
        let mut measures = std::collections::HashMap::new();
        for stmt in &module.items {
            self.collect_termination_measures_in_stmt(stmt, &mut measures);
        }
        measures
    }

    /// 递归收集单条语句中的显式测度（含函数体内的嵌套绑定）
    fn collect_termination_measures_in_stmt(
        &self,
        stmt: &crate::frontend::core::parser::ast::Stmt,
        measures: &mut std::collections::HashMap<
            String,
            crate::frontend::core::types::const_data::ConstExpr,
        >,
    ) {
        use crate::frontend::core::parser::ast::{Expr, StmtKind, Type};

        match &stmt.kind {
            StmtKind::Assign {
                target,
                type_annotation: Some(type_ann),
                value,
                ..
            } => {
                if let Expr::Var(name, _) = target.as_ref() {
                    // 函数签名形态：测度在返回类型位
                    let annotated = match type_ann {
                        Type::Fn { return_type, .. } => return_type.as_ref(),
                        other => other,
                    };
                    if let Some(measure) = Self::terminates_measure(annotated) {
                        measures.insert(name.clone(), measure);
                    }
                }
                // 函数体内的嵌套绑定同样计入
                if let Some(expr) = value.as_deref() {
                    self.collect_termination_measures_in_expr(expr, measures);
                }
            }
            StmtKind::Expr(expr) => self.collect_termination_measures_in_expr(expr, measures),
            StmtKind::If {
                then_branch,
                else_if_branches,
                else_branch,
                ..
            } => {
                for s in &then_branch.stmts {
                    self.collect_termination_measures_in_stmt(s, measures);
                }
                for (_, body) in else_if_branches {
                    for s in &body.stmts {
                        self.collect_termination_measures_in_stmt(s, measures);
                    }
                }
                if let Some(eb) = else_branch {
                    for s in &eb.stmts {
                        self.collect_termination_measures_in_stmt(s, measures);
                    }
                }
            }
            _ => {}
        }
    }

    /// 递归收集表达式内的显式测度（块/循环体/函数体）
    fn collect_termination_measures_in_expr(
        &self,
        expr: &crate::frontend::core::parser::ast::Expr,
        measures: &mut std::collections::HashMap<
            String,
            crate::frontend::core::types::const_data::ConstExpr,
        >,
    ) {
        use crate::frontend::core::parser::ast::Expr;

        match expr {
            Expr::Block(block) => {
                for s in &block.stmts {
                    self.collect_termination_measures_in_stmt(s, measures);
                }
            }
            Expr::While { body, .. } | Expr::For { body, .. } => {
                for s in &body.stmts {
                    self.collect_termination_measures_in_stmt(s, measures);
                }
            }
            Expr::Lambda { body, .. } => {
                for s in &body.stmts {
                    self.collect_termination_measures_in_stmt(s, measures);
                }
            }
            _ => {}
        }
    }

    /// 从类型注解中取出 `Terminates(m)` 的测度表达式 `m`（RFC-027a §2）。
    ///
    /// 只认一元形态；非 `Terminates` 或元数不符返回 `None`（元数错误由
    /// `resolve_type_annotation` 报 E1093，此处不重复报）。
    ///
    /// 测度实参在 AST 里两种形态（探针实测）：
    /// - `Type::Name` —— 单变量测度（`Terminates(b)`）
    /// - `Type::ConstExpr` —— 表达式测度（`Terminates(n - i)`）
    ///
    /// `Type::Generic` 形态（`Terminates(f(n))`，解析器读作类型应用）不是测度
    /// 表达式，需要 const fn 求值才能定值，暂不支持——返回 `None` 而非猜一个值。
    fn terminates_measure(
        ty: &crate::frontend::core::parser::ast::Type
    ) -> Option<crate::frontend::core::types::const_data::ConstExpr> {
        use crate::frontend::core::parser::ast::Type;
        use crate::frontend::core::types::const_data::ConstExpr;
        use crate::frontend::core::types::eval::const_eval::convert_expr_to_const_expr;

        let Type::Generic { name, args, .. } = ty else {
            return None;
        };
        if name != "Terminates" || args.len() != 1 {
            return None;
        }
        match &args[0] {
            Type::Name { name, .. } => Some(ConstExpr::NamedVar(name.clone())),
            Type::ConstExpr(expr) => convert_expr_to_const_expr(expr),
            _ => None,
        }
    }

    /// Phase 2.5：精化类型绑定与依赖重验证（RFC-027 §6.1，#379 微重构）
    ///
    /// 单遍顺序走查，按 [`RefinedScopeUnit`] 切分作用域（模块层一个单元，
    /// 每个函数体一个新单元）。声明序即验证序：赋值点只重验证**当时已声明**
    /// 的依赖者——旧实现两遍全模块共享一张名字键图，跨函数同名误报与
    /// 「赋值先于声明」误触发都源于此。
    ///
    /// 初始校验与重验证统一经 check_predicate 送证明管道：
    /// - Proved → 通过
    /// - Disproved → E2030，反例进诊断
    /// - Unproven{calls} → 上抛 proof_calls，由 pipeline 编译期执行；
    ///   空调用集 → E2031（RFC-027 §9：Unproven 不得静默放行）
    /// - 约束自由变量缺已知值时**不送检**：缺值送 SMT 会退化成「对所有值
    ///   成立」的全称检查，产生伪反例。值环境由常量折叠供给（`n = n + 1`
    ///   以旧值折叠出新值）；折叠不出的（函数参数、I/O 等运行期值）是
    ///   当前机制的已知边界——精确处刑需假设注入 + 路径条件（RFC-027 §6
    ///   Floyd-Hoare 全貌，随数据流推理落地，#377 同盘）
    fn collect_refined_binding_checks(
        &mut self,
        module: &Module,
        proof_calls: &mut Vec<crate::frontend::core::typecheck::proof::verdict::ProofFunctionCall>,
    ) {
        let mut shared_ctx =
            crate::frontend::core::typecheck::proof::context::ProofContext::new(&self.env);
        // #263：shared_ctx 持有 &self.env，精化解析诊断先汇入 sink，
        // 待 shared_ctx 释放后再写入 env.errors
        let mut refined_diags = Vec::new();
        {
            let mut ctx = RefinedWalkCtx {
                shared_ctx: &mut shared_ctx,
                proof_calls,
                diags: &mut refined_diags,
                fn_return_refined: None,
                fn_return_binder: None,
                fn_name: None,
            };
            let mut module_unit = RefinedScopeUnit::new();
            self.refined_walk_stmts(&module.items, &mut module_unit, &mut ctx);
        }

        // #263：精化解析诊断汇入（shared_ctx 已不再被使用，借用结束）
        self.env.errors.extend_errors(refined_diags);
    }

    /// 精化走查：一个语句列表 = 一个词法块。
    ///
    /// 块内注册的精化 dependant 在块退出时从依赖图移除（块局部变量出块即亡）；
    /// 函数体/模块层作为最外层块，单元随走查结束整体丢弃，移除只是兜底。
    fn refined_walk_stmts(
        &self,
        stmts: &[crate::frontend::core::parser::ast::Stmt],
        unit: &mut RefinedScopeUnit,
        ctx: &mut RefinedWalkCtx<'_, '_>,
    ) {
        let mut registered: Vec<String> = Vec::new();
        for stmt in stmts {
            // #324：嵌套语句逐条挂当前 span，诊断落到真实语句行
            let _span_guard = crate::util::diagnostic::push_current_span(stmt.span);
            registered.extend(self.refined_walk_stmt(stmt, unit, ctx));
        }
        for name in &registered {
            unit.deps.remove_dependant(name);
        }
    }

    /// 精化走查：单条语句。返回本语句注册的精化 dependant 名（供块退出清理）。
    fn refined_walk_stmt(
        &self,
        stmt: &crate::frontend::core::parser::ast::Stmt,
        unit: &mut RefinedScopeUnit,
        ctx: &mut RefinedWalkCtx<'_, '_>,
    ) -> Vec<String> {
        use crate::frontend::core::parser::ast::{Expr, StmtKind};

        match &stmt.kind {
            // 注解绑定 `x: T = v`：解析精化 → 注册依赖 + 初始校验 → 记录字面量值
            StmtKind::Assign {
                target,
                type_annotation: Some(type_ann),
                signature_params,
                value,
                ..
            } => {
                let mut registered = Vec::new();
                if let Expr::Var(name, _) = target.as_ref() {
                    let mono_ty = MonoType::from(type_ann.clone());
                    let resolved_ty = self.resolve_type_annotation(&mono_ty, ctx.diags);
                    // 初始值常量折叠（对前置值环境求值）——初始校验与值追踪共用
                    let init_value = value
                        .as_deref()
                        .and_then(|v| Self::fold_value(&*ctx.shared_ctx, v, &unit.values));
                    if matches!(resolved_ty, MonoType::Refined { .. })
                        && !constraint_is_terminates(&resolved_ty)
                    {
                        unit.deps.register_refined(name, &resolved_ty);
                        registered.push(name.clone());
                        // 初始校验：bindings 代入 name 自身的初始值（若可折叠）
                        self.revalidate_refined(
                            name,
                            name,
                            &resolved_ty,
                            (name, init_value.clone()),
                            &unit.values,
                            ctx,
                        );
                    }
                    unit.track(name, init_value.clone());
                }
                // 绑定值是函数体 → 嵌套函数体独立单元走查（判定与
                // `block_binding_is_function` 同源，#363：注解是推荐的函数写法）
                if crate::frontend::core::parser::ast::Expr::block_binding_is_function(
                    Some(type_ann),
                    value.as_deref(),
                ) {
                    if let Some(body) = extract_fn_body(value.as_deref()) {
                        let fn_name = match target.as_ref() {
                            Expr::Var(n, _) => Some(n.as_str()),
                            _ => None,
                        };
                        // RFC-027 §3：返回位精化的 binder 名与谓词实参只在 **AST** 里
                        // 无损（`MonoType` 已把符号实参降级成占位），故在此按声明解析，
                        // 走查只消费结果。
                        let ret_refinement =
                            self.resolve_return_refinement(type_ann, signature_params, ctx.diags);
                        self.refined_walk_fn_body(body, fn_name, ret_refinement, ctx);
                    }
                }
                // RFC-027 §3.4：绑定初始值若为调用，校验其实参
                if let Some(v) = value.as_deref() {
                    self.check_call_arg_refinements(v, unit, ctx);
                }
                registered
            }
            // 无注解赋值/绑定 `x = v`：更新值环境 + 重验证依赖者
            StmtKind::Assign {
                target,
                value: Some(rhs),
                ..
            } => {
                if let Expr::Var(name, _) = target.as_ref() {
                    // RHS 对**前置**值环境折叠：`n = n + 1` 以旧 n 值算出新值
                    let new_value = Self::fold_value(&*ctx.shared_ctx, rhs, &unit.values);
                    // RFC-027 §6.1：x 变更 → 对每个依赖 x 的变量生成 VC 重验证
                    for dependant in unit.deps.affected_by(name) {
                        let dependant = dependant.to_string();
                        if let Some(refined) = unit.deps.constraint_of(&dependant) {
                            self.revalidate_refined(
                                name,
                                &dependant,
                                refined,
                                (name, new_value.clone()),
                                &unit.values,
                                ctx,
                            );
                        }
                    }
                    unit.track(name, new_value);
                    // 变量重赋值 ⇒ 依赖它的假设作废（否则用旧值判新值）
                    ctx.shared_ctx.assumptions.kill(name);
                }
                // 复杂赋值目标（索引/解构）：v1 不建模对依赖者的影响（与旧实现一致）
                // RHS 内嵌函数体：`f = () => {...}` 同样独立单元
                match rhs.as_ref() {
                    Expr::Lambda { body, .. } => {
                        // 无注解绑定：无签名可依，后置条件与形参前置条件皆空
                        self.refined_walk_fn_body(body, None, None, ctx);
                    }
                    // 无注解的块值在当前作用域顺序执行：同单元走查
                    Expr::Block(block) => {
                        self.refined_walk_stmts(&block.stmts, unit, ctx);
                    }
                    // RFC-027 §3.4：调用点的形参精化义务
                    Expr::Call { .. } => {
                        self.check_call_arg_refinements(rhs, unit, ctx);
                    }
                    _ => {}
                }
                Vec::new()
            }
            StmtKind::If {
                condition,
                then_branch,
                else_if_branches,
                else_branch,
                ..
            } => {
                // 分支敏感 + 守卫裁剪：条件在当前值环境下可判定时只走会执行
                // 的分支（不可达分支不产生验证义务、不并进汇合）；不可判定时
                // 保守展开。各路径从同一前置值环境出发，汇合取等值交集——
                // 仅部分分支改写（或声明）的变量出块即值未知
                let paths = self.refined_walk_if_paths(
                    condition,
                    &then_branch.stmts,
                    else_if_branches,
                    else_branch.as_deref(),
                    unit,
                    ctx,
                );
                unit.values = meet_value_paths(paths);
                Vec::new()
            }
            StmtKind::For { var, body, .. } => {
                self.refined_walk_loop_body(Some(var), body, unit, ctx);
                Vec::new()
            }
            StmtKind::Expr(expr) => {
                match expr.as_ref() {
                    // 值块：当前作用域顺序执行
                    Expr::Block(block) => {
                        self.refined_walk_stmts(&block.stmts, unit, ctx);
                    }
                    Expr::While { body, .. } => {
                        self.refined_walk_loop_body(None, body, unit, ctx);
                    }
                    Expr::For { var, body, .. } | Expr::SpawnFor { var, body, .. } => {
                        self.refined_walk_loop_body(Some(var), body, unit, ctx);
                    }
                    // RFC-027 §3.4：调用点的形参精化义务
                    Expr::Call { .. } => {
                        self.check_call_arg_refinements(expr, unit, ctx);
                    }
                    // RFC-027：返回位精化在 `return` 点验证（后置条件）
                    Expr::Return(Some(v), return_span) => {
                        self.check_return_refinement(v, *return_span, unit, ctx);
                    }
                    _ => {}
                }
                Vec::new()
            }
            _ => Vec::new(),
        }
    }

    /// 在**守卫成立**的假设下走查一个分支（RFC-027 §3.4）。
    ///
    /// §3.4 的帮助文案自己举的例子就是这条路径：
    /// `if y > 0 { divide(x, y) }`——守卫 `y > 0` 正是精化实参所需的静态证据。
    /// 不注入守卫会把这类**合法**代码判伪（y 无约束时可取 0），是误报。
    ///
    /// 作用域配对保证守卫不泄漏到分支之外（`else` 不该看到 `then` 的条件）。
    fn refined_walk_branch_with_guard(
        &self,
        guard: Option<crate::frontend::core::types::const_data::ConstExpr>,
        stmts: &[crate::frontend::core::parser::ast::Stmt],
        unit: &mut RefinedScopeUnit,
        ctx: &mut RefinedWalkCtx<'_, '_>,
    ) {
        ctx.shared_ctx.assumptions.enter_scope();
        if let Some(g) = guard {
            ctx.shared_ctx.assumptions.inject(g);
        }
        self.refined_walk_stmts(stmts, unit, ctx);
        ctx.shared_ctx.assumptions.exit_scope();
    }

    /// 走查 if 链：返回所有**可达**路径的出口值环境（进入态不被污染）。
    ///
    /// 守卫裁剪：条件以当前值环境折叠可判定（`Some(true/false)`）时，只走
    /// 会执行的分支——不可达分支内的赋值不产生保守误报，其内的精化声明
    /// 不产生验证义务（块局部，出块即清理）。不可判定时保守展开为多路径。
    fn refined_walk_if_paths(
        &self,
        condition: &crate::frontend::core::parser::ast::Expr,
        then_stmts: &[crate::frontend::core::parser::ast::Stmt],
        else_ifs: &[(
            Box<crate::frontend::core::parser::ast::Expr>,
            Box<crate::frontend::core::parser::ast::Block>,
        )],
        else_branch: Option<&crate::frontend::core::parser::ast::Block>,
        unit: &mut RefinedScopeUnit,
        ctx: &mut RefinedWalkCtx<'_, '_>,
    ) -> Vec<HashMap<String, crate::frontend::core::types::ConstValue>> {
        match Self::fold_bool(&*ctx.shared_ctx, condition, &unit.values) {
            Some(true) => {
                self.refined_walk_branch_with_guard(None, then_stmts, unit, ctx);
                vec![unit.values.clone()]
            }
            Some(false) => match else_ifs.split_first() {
                Some(((cond, body), rest)) => {
                    self.refined_walk_if_paths(cond, &body.stmts, rest, else_branch, unit, ctx)
                }
                None => match else_branch {
                    Some(eb) => {
                        self.refined_walk_branch_with_guard(None, &eb.stmts, unit, ctx);
                        vec![unit.values.clone()]
                    }
                    None => vec![unit.values.clone()],
                },
            },
            None => {
                // then 臂：条件成立是一条假设（§3.4 的守卫证据）
                let then_guard =
                    crate::frontend::core::types::eval::const_eval::convert_expr_to_const_expr(
                        condition,
                    );
                let pre = unit.values.clone();
                self.refined_walk_branch_with_guard(then_guard, then_stmts, unit, ctx);
                let mut paths = vec![std::mem::replace(&mut unit.values, pre)];
                // 余下链递归（后续条件同样剪枝）；无 else-if 时
                // 「全部条件都不中」的路径 = else 臂（有 else）或进入态原样
                match else_ifs.split_first() {
                    Some(((cond, body), rest)) => paths.extend(self.refined_walk_if_paths(
                        cond,
                        &body.stmts,
                        rest,
                        else_branch,
                        unit,
                        ctx,
                    )),
                    None => match else_branch {
                        Some(eb) => {
                            // else 臂：条件不成立
                            self.refined_walk_branch_with_guard(
                                crate::frontend::core::typecheck::layers::termination::negate_guard(
                                    condition,
                                ),
                                &eb.stmts,
                                unit,
                                ctx,
                            );
                            paths.push(unit.values.clone());
                        }
                        None => paths.push(unit.values.clone()),
                    },
                }
                paths
            }
        }
    }

    /// 走查循环体：迭代间被改写的变量值未知——入环与出环都作废其字面量值
    /// （第 2+ 次迭代的取值与入口值不同，入口值对体内 VC 不安全）
    fn refined_walk_loop_body(
        &self,
        loop_var: Option<&str>,
        body: &crate::frontend::core::parser::ast::Block,
        unit: &mut RefinedScopeUnit,
        ctx: &mut RefinedWalkCtx<'_, '_>,
    ) {
        let mut assigned = collect_assigned_var_names(&body.stmts);
        if let Some(var) = loop_var {
            assigned.insert(var.to_string());
        }
        for name in &assigned {
            unit.values.remove(name);
        }
        self.refined_walk_stmts(&body.stmts, unit, ctx);
        for name in &assigned {
            unit.values.remove(name);
        }
    }

    /// RFC-027 §3：从函数注解 **AST** 解析返回位精化（后置条件）。
    ///
    /// 与 `resolve_type_annotation` 承担同一义务，区别只在**来源**：那里拿到的
    /// 是已降级的 `MonoType`，而 `MonoType::from` 会把不可折叠的谓词实参
    ///（`r + 100`）压成占位 `TypeRef("<const-expr>")`。实参一旦丢失，
    /// `-> (r: P(任意实参))` 就退化成 `-> (r: P(<const-expr>))`，再被旧的
    ///「不在作用域的自由变量即返回值形参」启发式代入返回值——实测 d1
    ///（`-> (r: IsPositive(r + 100))` 返 `-5`）报出的约束因此是 `-5 > 0`。
    ///
    /// 返回形式参数严格按**声明的名字**识别：有声明（`(r: ...)`）时该名字绑定
    /// `return` 的值；**其余不在作用域的自由变量不再被当成 binder**，而是报
    /// 未定义标识符（`(r: Eq2(m, r))` 的 `m` 此前被静默代入返回值，约束退化成
    /// `7 == 7` 恒真）。
    ///
    /// 失败路径（非谓词应用、实参不可转换、元数不符）一律静默返回 `None`：
    /// 那几条诊断归注册路径（`resolve_type_annotation`），此处不重复报。
    /// 唯独「有未声明的自由变量」是本函数新增的义务，必须报、不得静默放行。
    ///
    /// 作用域名单 = 本函数签名形参（`signature_params`，含 curry 各组与类型/
    /// const 参数）+ 模块级绑定（`env.vars`）。嵌套函数体引用**外层**函数的
    /// 形参时不在名单内——那类约束没有静态证据可判，报未定义标识符比静默
    /// 代入返回值安全。
    fn resolve_return_refinement(
        &self,
        type_ann: &crate::frontend::core::parser::ast::Type,
        params: &[Param],
        diags: &mut Vec<Diagnostic>,
    ) -> Option<ReturnRefinement> {
        use crate::frontend::core::parser::ast::Type as AstType;

        let AstType::Fn { return_type, .. } = type_ann else {
            return None;
        };
        let (binder, annotated) = match return_type.as_ref() {
            AstType::NamedParen { param, inner, .. } => (Some(param.clone()), inner.as_ref()),
            other => (None, other),
        };
        let AstType::Generic { name, args, .. } = strip_type_parens(annotated) else {
            return None;
        };
        // `Terminates(m)` 是终止测度而非值约束（RFC-027 §6.9）
        if name == "Terminates" {
            return None;
        }
        let refined = self.refine_predicate_from_ast_args(name, args)?;

        // RFC-027 §3：除声明名之外，不在作用域（形参/模块级绑定）的自由变量是
        // 未定义标识符。旧启发式把它们一并代入返回值，使约束退化成恒真。
        let mut undeclared: Vec<String> = Vec::new();
        for free in refined_free_vars(&refined) {
            if binder.as_deref() == Some(free.as_str()) {
                continue;
            }
            if params.iter().any(|p| p.name == free) || self.env.vars.contains_key(&free) {
                continue;
            }
            if !undeclared.contains(&free) {
                undeclared.push(free);
            }
        }
        if !undeclared.is_empty() {
            for name in &undeclared {
                diags.push(ErrorCodeDefinition::unknown_variable(name).build());
            }
            // 注解已被拒：不再生成返回点义务（避免连环诊断）
            return None;
        }
        Some(ReturnRefinement { binder, refined })
    }

    /// 把谓词应用正格化为 `Refined`，实参取自 **AST**。
    ///
    /// 与 `resolve_type_annotation` 的 `Generic` 分支同源：编译期谓词代入谓词体、
    /// 证明函数留作不透明 `Call`。唯一区别是实参来源——这里直接读 AST，
    /// `r + 100` 这类符号实参不再被降级成占位。
    ///
    /// 取不出实参（不是谓词、形态不可转换、元数不符）时返回 `None` 且**不报诊断**：
    /// 那几条路径的诊断归属注册路径，此处只负责「按 AST 无损代入」。
    fn refine_predicate_from_ast_args(
        &self,
        name: &str,
        args: &[crate::frontend::core::parser::ast::Type],
    ) -> Option<MonoType> {
        // 1. 编译期谓词定义（单参数）：实参代入谓词体模板
        if let Some(def) = self.env.predicate_defs.get(name) {
            if args.len() != 1 {
                return None;
            }
            let arg_expr = self.ast_type_to_const_expr(&args[0])?;
            let constraint =
                crate::frontend::core::typecheck::layers::termination::substitute_const_expr(
                    &def.constraint,
                    std::slice::from_ref(&def.param_name),
                    std::slice::from_ref(&arg_expr),
                );
            return Some(MonoType::Refined {
                base: Box::new(def.param_type.clone()),
                constraint,
            });
        }
        // 2. 证明函数（源码定义、返回 `Type`）：约束是不透明应用
        let mono_args: Vec<MonoType> = args.iter().map(|a| MonoType::from(a.clone())).collect();
        let base = match self.lookup_proof_fn_base_type(name, &mono_args) {
            Some(Ok(base)) => base,
            _ => return None,
        };
        let const_args: Option<Vec<ConstExpr>> = args
            .iter()
            .map(|a| self.ast_type_to_const_expr(a))
            .collect();
        Some(MonoType::Refined {
            base: Box::new(base),
            constraint: ConstExpr::Call {
                func: name.to_string(),
                args: const_args?,
            },
        })
    }

    /// 谓词实参（AST 形态）→ `ConstExpr`。
    ///
    /// **先取 AST**：`r + 100` 在 AST 里是原样的编译期表达式，降级成 `MonoType`
    /// 后才变成占位 `"<const-expr>"`。取不出时退回 `MonoType` 形态（与
    /// `mono_type_to_const_expr` 同源：裸名 → `NamedVar`、字面量 → `Lit`），
    /// 使实参覆盖面与既有路径一致。
    fn ast_type_to_const_expr(
        &self,
        ty: &crate::frontend::core::parser::ast::Type,
    ) -> Option<ConstExpr> {
        use crate::frontend::core::parser::ast::Type as AstType;
        match ty {
            AstType::ConstExpr(expr) => convert_expr_to_const_expr(expr),
            AstType::Paren(inner) | AstType::NamedParen { inner, .. } => {
                self.ast_type_to_const_expr(inner)
            }
            other => self.mono_type_to_const_expr(&MonoType::from(other.clone())),
        }
    }

    /// 走查函数体：独立单元——依赖与值环境以函数体为边界，
    /// 不与外层或兄弟函数互通（跨函数同名不互相触发重验证）
    fn refined_walk_fn_body(
        &self,
        body: &crate::frontend::core::parser::ast::Block,
        fn_name: Option<&str>,
        ret_refinement: Option<ReturnRefinement>,
        ctx: &mut RefinedWalkCtx<'_, '_>,
    ) {
        // RFC-027 §3.3：当前函数的**形参精化**进假设集 Γ——同函数的每个 `return`
        // 都在它的前置条件下成立，缺它则 `succ` 这类后置条件恒推不出。
        // 只注入本函数自己的： 跨函数同名不能互相作证。
        let assumptions: Vec<crate::frontend::core::types::const_data::ConstExpr> = fn_name
            .and_then(|n| self.collect_param_refinements().get(n).cloned())
            .unwrap_or_default();
        let saved_ret = ctx.fn_return_refined.take();
        let saved_binder = ctx.fn_return_binder.take();
        let saved_name = ctx.fn_name.take();
        // RFC-027 §3：后置条件在**声明处**由 AST 解析（见 `resolve_return_refinement`）。
        // 走查只消费结果——注册路径那份 `MonoType` 已把不可折叠的谓词实参
        //（`r + 100`）压成 `"<const-expr>"` 占位，据此判定会把实参丢掉。
        let (refined, binder) = match ret_refinement {
            Some(r) => (Some(r.refined), r.binder),
            None => (None, None),
        };
        ctx.fn_return_refined = refined;
        ctx.fn_return_binder = binder;
        ctx.fn_name = fn_name.map(str::to_string);

        ctx.shared_ctx.assumptions.enter_scope();
        for a in assumptions {
            ctx.shared_ctx.assumptions.inject(a);
        }

        let mut fn_unit = RefinedScopeUnit::new();
        self.refined_walk_stmts(&body.stmts, &mut fn_unit, ctx);

        // RFC-010a 规则①：块的值 = 尾表达式；而函数体的值即返回值。
        // 故**尾表达式是隐式 `return`**，后置条件同样要在它上面验证
        //（`bad: (b: IsPositive(b)) -> (r: IsPositive(r)) = { b - 1 }` 无 `return` 关键字）。
        // 显式 `return` 已在 `refined_walk_stmt` 里查过，不重复。
        if let Some(last) = body.stmts.last() {
            if let crate::frontend::core::parser::ast::StmtKind::Expr(e) = &last.kind {
                if !matches!(
                    e.as_ref(),
                    crate::frontend::core::parser::ast::Expr::Return(..)
                ) {
                    self.check_return_refinement(e, last.span, &fn_unit, ctx);
                }
            }
        }

        ctx.shared_ctx.assumptions.exit_scope();
        ctx.fn_return_refined = saved_ret;
        ctx.fn_return_binder = saved_binder;
        ctx.fn_name = saved_name;
    }

    /// 重验证 dependant 的精化约束（VC 生成 → 证明管道）
    ///
    /// `trigger` 进 E2030 文案（赋值场景 = 被赋值变量名，初始校验 = 声明名）。
    /// `changed`：本次变更 (变量, 新值)——已知则代入 bindings；未知（非字面量
    /// RHS）则作废旧值，用陈旧值判约束会产生伪 Proved。
    fn revalidate_refined(
        &self,
        trigger: &str,
        dependant: &str,
        refined: &MonoType,
        changed: (&str, Option<crate::frontend::core::types::ConstValue>),
        values: &HashMap<String, crate::frontend::core::types::ConstValue>,
        ctx: &mut RefinedWalkCtx<'_, '_>,
    ) {
        let mut bindings = values.clone();
        match changed.1 {
            Some(v) => {
                bindings.insert(changed.0.to_string(), v);
            }
            None => {
                bindings.remove(changed.0);
            }
        }
        // 缺值闸门：约束的自由变量不全有已知值则不送检（防全称检查伪反例）
        if !refined_free_vars(refined)
            .iter()
            .all(|v| bindings.contains_key(v))
        {
            return;
        }
        match crate::frontend::core::typecheck::layers::predicate::check_predicate(
            ctx.shared_ctx,
            refined,
            &bindings,
        ) {
            ProofResult::Proved => {}
            ProofResult::Disproved(model) => {
                if let MonoType::Refined { constraint, .. } = refined {
                    // 闭合约束（无自由变量）为假 ⇒ **注解自身不成立**，与传给它
                    // 的值无关（`IsPositive(-5)`：`-5 > 0` 恒假）。这一支报 E4018
                    // 且**不**展示反例：bindings 里那个变量与闭合约束无关，展示
                    // 它（实测 `y=Int(5)`）会把人引向 RHS。
                    if refined_free_vars(refined).is_empty() {
                        ctx.diags.push(
                            ErrorCodeDefinition::refinement_violated(&constraint.to_string())
                                .param("counterexample", "（约束不含自由变量，注解自身不成立）")
                                .build(),
                        );
                    } else {
                        // 开约束：依赖变量的当前取值使其为假 ⇒ 重验证失败
                        let counterexample = model
                            .assignments
                            .iter()
                            .map(|(k, v)| format!("{k}={v}"))
                            .collect::<Vec<_>>()
                            .join(", ");
                        ctx.diags.push(
                            ErrorCodeDefinition::refined_constraint_violated(
                                trigger,
                                dependant,
                                &constraint.to_string(),
                                &counterexample,
                            )
                            .build(),
                        );
                    }
                }
            }
            ProofResult::Unproven {
                proof_calls: calls, ..
            } => {
                if calls.is_empty() {
                    // RFC-027 §4/§9：Unproven → 编译错误，无降级、无 silent pass。
                    // 此分支 = 绑定完备仍证不出（约束形态超出证明内核：If/Range
                    // 形约束、SMT unknown、超预算）。谓词定义注册后（#377-3）
                    // 约束已可以是谓词体的任意形态（不再是清一色 Call），本分支
                    // 因此成为真防线而非前瞻兑底：不得让 silent pass 复活。
                    ctx.diags.push(
                        ErrorCodeDefinition::refined_unproven(
                            trigger,
                            dependant,
                            &refined.to_string(),
                        )
                        .build(),
                    );
                } else {
                    ctx.proof_calls.extend(calls.clone());
                }
            }
        }
    }

    /// RFC-027 §3.3/§3.4：调用点实参对**形参精化**的验证。
    ///
    /// 被调函数签名里的精化形参（`b: IsPositive(b)`）是调用方义务：实参值
    /// 编译期可知且使约束为假时报 E4018。与绑定位共用同一检查器
    /// （`check_predicate`），故两处判定一致。
    ///
    /// **本版只做证伪**：实参不可折叠、约束自由变量不为单个、或结果非
    /// `Disproved` 时一律放行。§3.4 要求的「无静态证据即拒绝」是更强的半场
    /// ——它会把 `divide(x, y)`（实参无界、无标注）一并拒掉，需单独决策后
    /// 另行落地，故此处不默认启用。
    ///
    /// 局限：只在调用点两种形态触发（裸调用语句、赋值 RHS 为调用）；嵌套在
    /// 更复杂表达式里的调用不查（无通用表达式遍历器）。
    fn check_call_arg_refinements(
        &self,
        expr: &Expr,
        unit: &RefinedScopeUnit,
        ctx: &mut RefinedWalkCtx<'_, '_>,
    ) {
        let Expr::Call {
            func, args, span, ..
        } = expr
        else {
            return;
        };
        let Expr::Var(fn_name, _) = func.as_ref() else {
            return;
        };
        // 签名取自 env：形参位已由 `resolve_type_annotation` 规范化为 Refined
        let Some(sig) = self.env.vars.get(fn_name) else {
            return;
        };
        let MonoType::Fn { params, .. } = &sig.body else {
            return;
        };
        for (i, param_ty) in params.iter().enumerate() {
            if !matches!(param_ty, MonoType::Refined { .. }) || constraint_is_terminates(param_ty) {
                continue;
            }
            let Some(arg) = args.get(i) else { continue };
            let MonoType::Refined { base, constraint } = param_ty.clone() else {
                continue;
            };
            let free = refined_free_vars(param_ty);
            if free.len() != 1 {
                continue;
            }
            // 实参**符号代入**约束（RFC-027 §3.3）：不要求实参可折成常量。
            //
            // 折常量版只能查 `f(-5)` 这类字面量，而 §3.4 要的是「运行时值进入
            // 精化类型参数必须提供静态证据」——`f(y)`（y 无界）与 `f(n)`（n 带
            // 精化标注）正是它的两行范例。符号代入把 `y > 0` / `n > 0` 形式化后
            // 交判定管道：Γ 里有证据则成立，无则判伪/判不出。
            let Some(arg_expr) =
                crate::frontend::core::types::eval::const_eval::convert_expr_to_const_expr(arg)
            else {
                continue;
            };
            let substituted = MonoType::Refined {
                base: base.clone(),
                constraint:
                    crate::frontend::core::typecheck::layers::termination::substitute_const_expr(
                        &constraint,
                        std::slice::from_ref(&free[0]),
                        &[arg_expr],
                    ),
            };
            // RFC-027 §3.3：实参**自身的精化标注**是证据（#395 第三行）。
            //
            // 在**本检查局部**注入实参变量已注册的精化约束，查完即弹：不放进
            // 声明处的全局 Γ。原因是重验证路径会自命中——它验的正是同一条约束，
            // 若该约束已在 Γ，判定会在 level 2（精确匹配）平凡通过，**掩盖真违反**
            //（实测：4 个「依赖变量重赋值」负例语料会被静默放过）。
            let arg_refined: Option<crate::frontend::core::types::const_data::ConstExpr> = match arg
            {
                Expr::Var(name, _) => unit.deps.constraint_of(name).and_then(|ty| match ty {
                    MonoType::Refined { constraint, .. } if !constraint_is_terminates(ty) => {
                        Some(constraint.clone())
                    }
                    _ => None,
                }),
                _ => None,
            };
            let scoped = arg_refined.is_some();
            if scoped {
                ctx.shared_ctx.assumptions.enter_scope();
                if let Some(c) = arg_refined {
                    ctx.shared_ctx.assumptions.inject(c);
                }
            }
            let bindings = unit.values.clone();
            let outcome = crate::frontend::core::typecheck::layers::predicate::check_predicate(
                ctx.shared_ctx,
                &substituted,
                &bindings,
            );
            if scoped {
                ctx.shared_ctx.assumptions.exit_scope();
            }
            match outcome {
                ProofResult::Proved => {}
                ProofResult::Disproved(model) => {
                    let counterexample = model
                        .assignments
                        .iter()
                        .map(|(k, v)| format!("  {k} = {v}"))
                        .collect::<Vec<_>>()
                        .join("\n");
                    ctx.diags.push(
                        ErrorCodeDefinition::refinement_violated(&model.constraint)
                            .param("counterexample", &counterexample)
                            .at(*span)
                            .build(),
                    );
                }
                // 具名证明函数形态的约束（`IsPositive(b)`）当下判不出真假：
                // 交由 pipeline 编译并执行证明函数，返回 false 即 E4018。
                // 与绑定位（`revalidate_refined`）同一通道，故两处判定一致。
                ProofResult::Unproven {
                    proof_calls: calls, ..
                } => {
                    if calls.is_empty() {
                        // 无证法可执行且证不出：报错，不静默放行（§3.4）
                        ctx.diags.push(
                            ErrorCodeDefinition::refined_unproven(
                                fn_name,
                                &free[0],
                                &param_ty.to_string(),
                            )
                            .at(*span)
                            .build(),
                        );
                    } else {
                        ctx.proof_calls.extend(calls);
                    }
                }
            }
        }
    }
    /// RFC-027 §3：返回位精化在 `return` 点验证（后置条件）。
    ///
    /// 形参位与返回位是**同一个规则**的两侧（§3「统一性」）：`形参名:
    /// 谓词调用(形参名)`，区别仅在于值由调用方提供还是由 `return` 提供。
    /// 故两处判定走同一管道，仅**绑定来源**不同：
    ///
    /// - 形参位 → 调用点义务（`check_call_arg_refinements`）：自由变量绑到实参
    /// - 返回位 → 返回点义务（本函数）：自由变量按下列规则绑定
    ///
    /// **返回值形参的识别**：严格按**声明的名字**——`resolve_return_refinement`
    /// 从 `-> (r: P(...))` 取出 binder（§3：它「仅存在于类型签名中、仅被谓词
    /// 引用，**不进入函数体作用域**」），只有它绑到 `return` 表达式的值。
    /// 作用域内的自由变量（形参/模块级绑定）用自己的值或符号；既非声明名、
    /// 又不在作用域的自由变量在声明处已报未定义标识符，不再被代入返回值。
    /// 裸形态（`-> P(b - 1)`，约束只涉及形参）无声明名，约束与返回值无关地
    /// 成立/不成立——两个形态因此归一条规则。
    ///
    /// 保守方向：返回表达式不可折叠时**不判**（无静态证据即不报）——强半场
    /// （§3.4「无静态证据则编译错误」）是单独的开关，见 #395。
    fn check_return_refinement(
        &self,
        value: &Expr,
        span: crate::util::span::Span,
        unit: &RefinedScopeUnit,
        ctx: &mut RefinedWalkCtx<'_, '_>,
    ) {
        let Some(ret_ty) = ctx.fn_return_refined.clone() else {
            return;
        };
        let MonoType::Refined { base, constraint } = ret_ty.clone() else {
            return;
        };
        if constraint_is_terminates(&ret_ty) {
            return;
        }
        let free = refined_free_vars(&ret_ty);
        // 无自由变量的闭合约束与返回表达式无关（它只是一个退化的精化类型，
        // 成立性由注解自身决定）——不在本义务范围内。
        if free.is_empty() {
            return;
        }
        // 约束里**符号代入**声明的返回形式参数（§3「值由 `return` 提供」）。
        //
        // 用符号代入而非折常量：后置条件的价值正在于形参未知时也能判——
        // `r > 0` 代入 `b - 1` 得 `b - 1 > 0`，在 Γ={b>0} 下可判伪（b = 1）。
        // 折常量会把这类全漏掉（`b - 1` 含未知形参，折不出值）。
        let mut constraint = constraint;
        if let Some(binder) = ctx.fn_return_binder.clone() {
            // 只有声明名由 `return` 供给；约束未引用它时（如 `-> (r: P(5))`）
            // 无需代入，照常判定。
            if free.iter().any(|name| name == &binder) {
                let Some(returned) =
                    crate::frontend::core::types::eval::const_eval::convert_expr_to_const_expr(
                        value,
                    )
                else {
                    // 返回表达式转不出常量表达式（如含调用）：无从代入，保守不判
                    return;
                };
                constraint =
                    crate::frontend::core::typecheck::layers::termination::substitute_const_expr(
                        &constraint,
                        std::slice::from_ref(&binder),
                        &[returned],
                    );
            }
        }
        // 作用域内变量的当前值环境进 bindings；形参不在其中，留给 SMT 作符号
        let bindings = unit.values.clone();
        // 代入后的约束文本（含 `return` 值的符号形态），进诊断比未代入形态更可读。
        // 它是**约束表达式**（不是自然语言句子），故与 locale 无关。
        let constraint_text = constraint.to_string();
        let substituted = MonoType::Refined { base, constraint };
        match crate::frontend::core::typecheck::layers::predicate::check_predicate(
            ctx.shared_ctx,
            &substituted,
            &bindings,
        ) {
            ProofResult::Proved => {}
            ProofResult::Disproved(model) => {
                let counterexample = model
                    .assignments
                    .iter()
                    .map(|(k, v)| format!("  {k} = {v}"))
                    .collect::<Vec<_>>()
                    .join("\n");
                ctx.diags.push(
                    ErrorCodeDefinition::refinement_violated(&model.constraint)
                        .param("counterexample", &counterexample)
                        .at(span)
                        .build(),
                );
            }
            ProofResult::Unproven {
                proof_calls: calls, ..
            } => {
                if calls.is_empty() {
                    let fn_name = ctx
                        .fn_name
                        .clone()
                        .unwrap_or_else(|| "<anonymous>".to_string());
                    // RFC-027 §3：返回点义务的对象是**返回位**，不是签名形参——
                    // 此前这里用 `free[0]`（多参数谓词时那是形参名），文案把约束
                    // 挂到了错误对象上（实测：`SumUpTo(b, r)` 报成 "'b' 的精化
                    // 类型约束"）。
                    //
                    // 本槽只传 **标识符**：声明了 binder 就传它的名字（`r`——它是
                    // 返回形式参数，不是签名形参）；裸形态没有声明名，按 RFC-027 §2
                    // 取返回值形参的规范名 `result`（见 `DEFAULT_RETURN_BINDER`）。
                    // 任何自然语言措辞都由 locale 模板负责，code 侧不得写某语言的词。
                    //
                    // 这里**不覆盖 help**：E2031 的本地化 help 已完整给出可操作的
                    // 替代写法（实参可静态取值 / 提供返回 Type 的证明函数），
                    // 位置化文案要走 locales，不在 code 里拼。
                    let subject = match &ctx.fn_return_binder {
                        Some(binder) => binder.clone(),
                        None => DEFAULT_RETURN_BINDER.to_string(),
                    };
                    ctx.diags.push(
                        ErrorCodeDefinition::refined_unproven(&fn_name, &subject, &constraint_text)
                            .at(span)
                            .build(),
                    );
                } else {
                    ctx.proof_calls.extend(calls);
                }
            }
        }
    }
}

/// 裸形态返回值形参的**规范名**：`-> P(...)` 在语义上就是 `-> (result: P(...))`
/// 的简写（RFC-027 §2：spec 的 `max` 例子即 `-> (result: IsMax(T, arr, result))`，
/// 「`result` 是返回值形参，值由 `return` 提供」）。
///
/// 用途：裸形态没有声明名，E2031 的 `{var}` 槽取它作标识——与「按声明名识别」
/// 同一口径、是合法标识符（无任何语言的散文），跨 6 语言模板都自然：
/// en 渲染 `the refinement type constraint of 'result'`，zh 渲染 `'result' 的精化类型约束`。
const DEFAULT_RETURN_BINDER: &str = "result";

/// RFC-027 §3：返回位精化（后置条件）的解析结果。
///
/// `binder` 是**声明的**返回形式参数名（`-> (r: P(r))` 的 `r`）：只有它由
/// `return` 的值供给；`refined` 的约束已按 AST 实参代入——`r + 100` 保持符号
/// 形态，而不是被降级成占位 `<const-expr>`。
struct ReturnRefinement {
    /// `-> (r: P(...))` 声明的返回形式参数名；裸精化形态（`-> P(b - 1)`）为 `None`
    binder: Option<String>,
    /// 精化类型（`Refined { base, constraint }`）
    refined: MonoType,
}

/// 剥离类型外侧的括号：精化解析里 `Paren`（RFC-004）与 `NamedParen`
///（RFC-027 §3 具名形态）同为透明，只递归内层。
fn strip_type_parens(
    ty: &crate::frontend::core::parser::ast::Type
) -> &crate::frontend::core::parser::ast::Type {
    use crate::frontend::core::parser::ast::Type as AstType;
    match ty {
        AstType::Paren(inner) => strip_type_parens(inner),
        AstType::NamedParen { inner, .. } => strip_type_parens(inner),
        other => other,
    }
}

/// 精化走查的共享管道三件套：证明上下文 / 证明调用收集 / 诊断汇。
/// 走查各环节（初始校验、重验证、分支与循环下钻）都经它流转，
/// 不再逐层透传散参数。
struct RefinedWalkCtx<'a, 'env> {
    shared_ctx: &'a mut crate::frontend::core::typecheck::proof::context::ProofContext<'env>,
    proof_calls: &'a mut Vec<crate::frontend::core::typecheck::proof::verdict::ProofFunctionCall>,
    diags: &'a mut Vec<Diagnostic>,
    /// 当前函数的**返回位精化类型**（RFC-027 后置条件）。
    ///
    /// 函数体边界保存/恢复（lambda 可嵌套）——外层函数的后置条件不得
    /// 泄漏到内层函数体的 `return` 上。`None` = 无后置条件或无函数上下文。
    fn_return_refined: Option<MonoType>,
    /// 当前函数**声明的返回形式参数名**（`-> (r: P(r))` 的 `r`）。
    ///
    /// RFC-027 §3：只有这个名字由 `return` 的值供给；其余自由变量必须在
    /// 作用域内（形参/模块级绑定），否则在解析处报未定义标识符。
    /// 裸形态（`-> P(b - 1)`，约束只涉及形参）无声明名 ⇒ `None`。
    fn_return_binder: Option<String>,
    /// 当前函数名（进 E4018 的未证明文案）。
    fn_name: Option<String>,
}

/// 提取绑定值的函数体（判定已由调用方完成，这里只取形态）：
/// `Lambda` 恒为函数体；`Block` 在是函数定义时才是（值块返回 None）
fn extract_fn_body(
    value: Option<&crate::frontend::core::parser::ast::Expr>
) -> Option<&crate::frontend::core::parser::ast::Block> {
    match value? {
        crate::frontend::core::parser::ast::Expr::Lambda { body, .. } => Some(body.as_ref()),
        crate::frontend::core::parser::ast::Expr::Block(block) => Some(block),
        _ => None,
    }
}

/// 分支汇合：仅保留所有路径出口**等值**的变量。
/// 被任一路径改写（或仅部分路径声明）的变量出块即值未知——
/// 保守作废是 sound 的：宁可少判，不可用错值判。
fn meet_value_paths(
    paths: Vec<HashMap<String, crate::frontend::core::types::ConstValue>>
) -> HashMap<String, crate::frontend::core::types::ConstValue> {
    let mut meet = HashMap::new();
    let Some(first) = paths.first() else {
        return meet;
    };
    'outer: for (name, value) in first {
        for path in &paths[1..] {
            if path.get(name) != Some(value) {
                continue 'outer;
            }
        }
        meet.insert(name.clone(), value.clone());
    }
    meet
}

/// 收集语句列表中被**改写**的变量名（无注解赋值 `x = v` 的目标），
/// 供循环体作废字面量值。注解绑定（`mut x: T = ...`）每次迭代重新
/// 初始化且字面量值恒定，不在收集之列。递归覆盖嵌套控制流与值块。
fn collect_assigned_var_names(
    stmts: &[crate::frontend::core::parser::ast::Stmt]
) -> std::collections::HashSet<String> {
    use crate::frontend::core::parser::ast::{Expr, StmtKind};
    let mut assigned = std::collections::HashSet::new();
    for stmt in stmts {
        match &stmt.kind {
            StmtKind::Assign {
                target,
                type_annotation: None,
                value,
                ..
            } => {
                if let Expr::Var(name, _) = target.as_ref() {
                    assigned.insert(name.clone());
                }
                if let Some(v) = value.as_deref() {
                    collect_assigned_in_expr(v, &mut assigned);
                }
            }
            StmtKind::Assign { value, .. } => {
                if let Some(v) = value.as_deref() {
                    collect_assigned_in_expr(v, &mut assigned);
                }
            }
            StmtKind::If {
                then_branch,
                else_if_branches,
                else_branch,
                ..
            } => {
                assigned.extend(collect_assigned_var_names(&then_branch.stmts));
                for (_, body) in else_if_branches {
                    assigned.extend(collect_assigned_var_names(&body.stmts));
                }
                if let Some(eb) = else_branch {
                    assigned.extend(collect_assigned_var_names(&eb.stmts));
                }
            }
            StmtKind::For { var, body, .. } => {
                assigned.insert(var.clone());
                assigned.extend(collect_assigned_var_names(&body.stmts));
            }
            StmtKind::Expr(expr) => collect_assigned_in_expr(expr, &mut assigned),
            _ => {}
        }
    }
    assigned
}

/// 表达式形态的控制流体内改写收集（值块 / While / For 表达式）
fn collect_assigned_in_expr(
    expr: &crate::frontend::core::parser::ast::Expr,
    assigned: &mut std::collections::HashSet<String>,
) {
    use crate::frontend::core::parser::ast::Expr;
    match expr {
        Expr::Block(block) => {
            assigned.extend(collect_assigned_var_names(&block.stmts));
        }
        Expr::While { body, .. } => {
            assigned.extend(collect_assigned_var_names(&body.stmts));
        }
        Expr::For { var, body, .. } => {
            assigned.insert(var.clone());
            assigned.extend(collect_assigned_var_names(&body.stmts));
        }
        _ => {}
    }
}

include!("checker/semantic_tokens.rs");
