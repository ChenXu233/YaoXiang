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
    // W1090 义务已产出但无消费者（02-stage-contract §2 义务账本 / RFC-039 D5，
    // WBS 4.3.1：阶段表尾部结算的未排空义务，级别先 W 后 E）。
    // span_exempt=true 的理由（宏文档 codes/mod.rs:8-18 要求在此写明）：
    // 结算发生在 Driver::run 阶段表尾部，诊断粒度是「编译单元（文件/模块）
    // × 义务字段」——义务是阶段之间的契约，不存在用户可指的那一行源码，
    // 故不塞 dummy span；文件身份由 file 参数进消息文本。
    ("W1090", Warning, true, obligation_not_drained(field: &str, file: &str, detail: &str) => .param("field", field) .param("file", file) .param("detail", detail)),
});
