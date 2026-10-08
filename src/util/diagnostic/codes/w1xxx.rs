//! 错误码定义（define_codes! 单源条目，#326）

use super::{ErrorCategory, ErrorCodeDefinition, DiagnosticBuilder};

define_codes!(W1XXX, {
    // W1001 未使用的私有函数（pub = 对外接口永不报，#321 定案 B）
    ("W1001", Warning, false, unused_function(name: &str) => .param("name", name)),
    // W1002 未使用的私有类型
    ("W1002", Warning, false, unused_type(name: &str) => .param("name", name)),
    // W1003 未使用的导入（typecheck use elaboration 检出）
    ("W1003", Warning, false, unused_import(name: &str) => .param("name", name)),
    // W1004 未使用的私有变量
    ("W1004", Warning, false, unused_variable(name: &str) => .param("name", name)),
    // W1005 未使用的私有方法
    ("W1005", Warning, false, unused_method(name: &str) => .param("name", name)),
    // W1006 本地模块遮蔽依赖包（RFC-014 §项目模式：本地最高优先级，
    // 覆盖 vendor 中同名依赖时提示；--deny-shadowing 可升级为错误）
    ("W1006", Warning, false, module_shadows_dependency(module: &str, dependency: &str) => .param("module", module) .param("dependency", dependency)),
    // W1063
    ("W1063", Warning, false, const_generic_unevaluable(constraint: &str) => .param("constraint", constraint)),
    // W1081 终止性测度义务未判定（WBS 3.4.2：无求解器时 Unjudged 不再静默——
    // 「未判」≠「成立」，义务存在而判不了必须让用户知情）
    ("W1081", Warning, false, termination_obligations_unjudged(count: usize) => .param("count", count.to_string())),
    // W1080 编译期无法证明约束，已降级为运行时检查
    ("W1080", Warning, false, constraint_demoted() => ),
});
