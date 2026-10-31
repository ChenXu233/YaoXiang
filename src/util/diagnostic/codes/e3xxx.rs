//! 错误码定义（define_codes! 单源条目，#326）

use super::{ErrorCategory, ErrorCodeDefinition, DiagnosticBuilder};

define_codes!(E3XXX, {
    // E3004 不支持的迭代器类型
    ("E3004", Codegen, false, ir_unsupported_iterator(iter_type: &str) => .param("iter_type", iter_type)),
    // E3005 IR 内部错误
    ("E3005", Codegen, false, ir_internal_error(message: &str) => .param("message", message)),
    // E3006 未解析变量（#271 静默归零清单 #3：typecheck 漏网变量不再静默 Load 0）
    ("E3006", Codegen, false, unresolved_variable(name: &str) => .param("name", name)),
    // E3007 顶层绑定初始化非编译期常量（码号保留，**已不再产生**）。
    //
    // #271 清单 #2 引入时：初始化折叠不到常量则硬错误（取代静默填 0）。
    // 顶层绑定重构 T2 后：初始化改为运行时求值（全局槽位 + 模块初始化序列），
    // 常量折叠降为**优化**而非正确性门槛，故本码无生产调用点。
    // 保留定义以避免码号复用撞上旧产物（.42/日志）的诊断。
    ("E3007", Codegen, false, top_level_init_not_const(name: &str) => .param("name", name)),
    // E3008 不支持的 match 模式（#330 安全网：非字面量/通配符模式无 IR 编码，
    // 原 stub 加载 0 永不匹配、scrutinee 为 0 时误匹配，宁编译期拒绝不静默错译）
    ("E3008", Codegen, false, ir_unsupported_pattern(pattern: &str) => .param("pattern", pattern)),
    // E3018 单态化实例化失败（#335：此前静默跳过，下游表现为 E6006 函数表缺失）
    ("E3018", Codegen, false, ir_instantiation_failed(func: &str, reason: &str) => .param("func", func).param("reason", reason)),
    // E3014 寄存器溢出（span 豁免：寄存器索引上限 u8=255 是编译器内部资源限制，
    // 与具体源码位置无关；两个触发点（IR 层局部变量总数、OperandResolver 单操作数）
    // 都拿不到函数 span。此前标 false → debug 构建直接 panic，用户看到内部堆栈）
    ("E3014", Codegen, true, register_overflow(id: &str, limit: &str) => .param("id", id).param("limit", limit)),
    // E3017 无效操作数（代码生成）
    ("E3017", Codegen, false, codegen_invalid_operand(reason: &str) => .param("reason", reason)),
    // E3019 顶层绑定循环依赖（T3）：a 依赖 b、b 依赖 a 时列出环上的名字。
    // 此前这类写法报 E1001（前向引用不可用）或静默取到未初始化值。
    ("E3019", Codegen, false, global_init_cycle(cycle: &str) => .param("cycle", cycle)),
    // E3020 入口点语义（T4，RFC-029f Bin 角色）。
    // 无 main ⇒ 全部函数不可达（Bin 没有外部消费者，main 是唯一可达性根），
    // 属编译错误；此前静默执行函数表第一个函数（#271 静默错误族）。
    ("E3020", Codegen, false, bin_missing_main(path: &str) => .param("path", path)),
    // E3021 入口非函数（码号保留，**已不再产生**）。
    //
    // T4 引入时：Bin 入口要求 main 必须是函数，值绑定 main 报本码。
    // #388 定案：入口按**绑定存在性**判定（函数已是值，类型层统一）——
    // 值 main 在初始化期求值即执行，是合法入口，本码失去语义。
    // 保留定义以避免码号复用撞上旧产物（.42/日志）的诊断。
    ("E3021", Codegen, false, bin_main_not_function(name: &str) => .param("name", name)),
    // E3022 入口 main 签名不符（运行期以零参调用）
    ("E3022", Codegen, false, bin_main_signature(name: &str, expected: &str, found: &str) => .param("name", name).param("expected", expected).param("found", found)),
    // E3023 Bin 角色顶层不允许可执行语句（规范 §3.11）。
    // 此前这类**用户写法错误**落入 E3005「IR 内部错误，请报告此问题」，
    // 并把 `Discriminant(0)` 泄给用户。同族：#360（索引赋值）/#311（break）。
    ("E3023", Codegen, true, bin_top_level_statement() => ),
    // E3024 项目配置存在但不可读（#503，WBS 3.23.1；#520 扩面，2026-10-31
    // 用户裁决甲案）：yaoxiang.toml 存在却解析失败——**声明面**（非法 TOML /
    // 缺 [package] 必填字段，#503：此前被 `.ok()` 静默吞掉，跨文件引用池失效
    // 并误报 W1002）或 **[tool.*] 投影**（[tool.test] 类型错误穿透
    // PackageManifest 后被 `.ok()` 静默吞掉，显式 patterns 静默回退内建约定，
    // RFC-015:282 / RFC-036:146）——任一解析失败即报本码中止，不静默退化；
    // 无配置文件（合法单文件模式）不受影响。文案取「项目配置」（非「清单」）
    // 以同时覆盖两面。
    // span_exempt=true：错误位置在 yaoxiang.toml（非 .yx 源码），诊断条目
    // 挂入口文件名下只是逐文件通道的载体——无源码位置可指，#324 契约禁止
    // 塞 dummy span。
    ("E3024", Codegen, true, manifest_unreadable(path: &str, detail: &str) => .param("path", path).param("detail", detail)),
});
