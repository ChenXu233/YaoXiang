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
    // W1007 模块导入环（#491）：编译集的 `use` 依赖图里存在环（成员数 ≥2 的
    // 强连通分量）时，环内初始化顺序**不可判定**——读尚未初始化的顶层值会在
    // 运行期报 E6007，而这类程序在 4.10.4 之前只按「环内路径字典序」静默排序
    // （用户 2026-10-23 裁决 A：不得静默）。
    // **检出单源** = `driver::helpers::init_topological_order` 里那次 Tarjan SCC
    // （4.10.4 为初始化序已经算过，本码只消费它的出参，不新增第二份环检测）。
    // 不并入 E3019：E3019 判**一个单元内**顶层绑定之间的初始化引用成环，本码判
    // **模块之间**的导入成环——两类问题修法不同（前者改初始化表达式，后者拆 use）。
    // span 不豁免：定位落在环内一条真实 `use` 语句上（`UsePathRef.span`）。
    ("W1007", Warning, false, module_import_cycle(cycle: &str) => .param("cycle", cycle)),
    // W1063
    ("W1063", Warning, false, const_generic_unevaluable(constraint: &str) => .param("constraint", constraint)),
    // W1081 终止性测度义务未判定（WBS 3.4.2：无求解器时 Unjudged 不再静默——
    // 「未判」≠「成立」，义务存在而判不了必须让用户知情）
    ("W1081", Warning, false, termination_obligations_unjudged(count: usize) => .param("count", count.to_string())),
    // W1080 编译期无法证明约束，已降级为运行时检查
    ("W1080", Warning, false, constraint_demoted() => ),
    // W1082 所有权分析降级：无函数体检查器 ⇒ 空类型账本 + 空调用点所有权表，
    // 所有权判定退化为「什么都不冲突」（WBS 4.4.3 / 02 §S5 (d)；RFC-039 D59
    // C 类「静默不动作」——降级必须留痕）。生产路径上 init_body_checker 先于
    // 所有权阶段执行、该分支不可达，故本码是**阶段契约防线**：一旦发射即说明
    // 阶段顺序被破坏。不并入 W1081（后者专指求解器缺失）与 W1080（约束降级）。
    ("W1082", Warning, false, ownership_analysis_degraded() => ),
    // W1090 义务已产出但无消费者（02-stage-contract §2 义务账本 / RFC-039 D5，
    // WBS 4.3.1 阶段表尾部结算的未排空义务；WBS 4.3.2 同码升 E，2026-10-13 用户裁决甲）。
    // **码族与发射级别解耦**：码号留在 W 族（不新增 E 码——同一事实两个码即平行表示），
    // 发射级别由构造点的显式 `.severity(Severity::Error)` 决定
    // （driver/obligations.rs 的 `to_diagnostic`）；本表的 `Warning` 类目
    // 只表示编号归属，不决定级别（RFC-013「发射通道」节）。
    // span_exempt=true 的理由（宏文档 codes/mod.rs:8-18 要求在此写明）：
    // 结算发生在 Driver::run 阶段表尾部，诊断粒度是「编译单元（文件/模块）
    // × 义务字段」——义务是阶段之间的契约，不存在用户可指的那一行源码，
    // 故不塞 dummy span；文件身份由 file 参数进消息文本。
    ("W1090", Warning, true, obligation_not_drained(field: &str, file: &str, detail: &str) => .param("field", field) .param("file", file) .param("detail", detail)),
});
