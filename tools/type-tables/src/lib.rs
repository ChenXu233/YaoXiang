//! 类型表生成期门禁（P6 / RFC-039 §5.7 的 T1–T2）
//!
//! 两个消费者（形态与 `tools/code-tables` 对称）：
//! - `build.rs`（build-dependencies 引本 lib）：每次 `cargo build` 只跑 **T1**，**已是构建硬失败**
//!   （6.6.2，2026-10-30）——违规即 `panic!` 拒绝编译：零构造变体 / 白名单与实现不一致 /
//!   登记豁免点未命中 / 未登记的兜底点 / **勘察基线漂移（T1.6）**，五类走同一条失败路径；
//!   通过时只打一行摘要（`cargo:warning` 信息行，不是违规）。
//! - `type-tables` bin：`--report` 输出 T1/T2 两节全量报告，`--json` 输出 T1 结构化结果。
//!
//! 两项检查（失败条件见 [03-type-unification.md] §5.7 的表）：
//! - **T1 构造点分列**：`ast::Type` 的每个变体，统计三类构造点（parser 前向 / AST→AST 重建 /
//!   反向桥重建），三类不得混算；前向零构造与全局零构造分别报。
//! - **T2 三表一致**：`from_builtin_name`、`CONST_PARAM_TYPES`、LSP 内置类型名清单逐名比对。
//!   三张表是内置类型名的**全部代码侧载体**；文档侧没有载体（RFC-011 是泛型系统设计，
//!   不枚举内置类型名），故门禁不把任何文档当比对对象（2026-10-30 用户裁决：比对对象不存在，
//!   原「内置类型名 vs RFC-011 文档」一项已删除）。
//!
//! [03-type-unification.md]: docs/src/dev/architecture/03-type-unification.md

pub mod baseline;
pub mod dispositions;
pub mod scan;
pub mod t1;
pub mod tables;

use std::path::Path;

/// `ast::Type` 的定义家（T1 的变体清单来源；T2 的 `CONST_PARAM_TYPES` 亦在此）
pub const AST_REL: &str = "src/frontend/core/parser/ast.rs";
/// `MonoType::from_builtin_name` 的所在文件（T2 的规范名来源）
pub const MONO_REL: &str = "src/frontend/core/types/mono.rs";
/// LSP 内置类型名清单的所在文件（T2 第三张表）
pub const LSP_REL: &str = "src/lsp/world.rs";
/// 反向桥 A（`impl From<MonoType> for IrType`）的所在文件（T1 分列）
pub const BYTECODE_REL: &str = "src/middle/core/bytecode.rs";
/// AST→AST 重建与反向桥 B（`mono_to_ast_type`）的所在文件（T1 分列）
pub const FUNCTION_REL: &str = "src/middle/passes/mono/function.rs";
/// T1 的扫描根（递归，排除 `tests` 目录）
pub const SCAN_ROOT_REL: &str = "src";

/// `build.rs` 入口：T1 **门禁**判定（6.6.2 起是构建硬失败）。
///
/// - `Ok(摘要)`：通过——`build.rs` 打印为一行 `cargo:warning` 信息（不是违规）；
/// - `Err(明细)`：违规——`build.rs` 必须 `panic!` 使构建失败，明细含变体名与 `file:line`。
///
/// 扫描本身出错也算 `Err`：口径不完整时判定不成立，**不得**降级成「通过」（防静默降级）。
pub fn build_gate(root: &Path) -> Result<String, String> {
    match scan::scan(root) {
        Ok(result) => t1::ForwardReport::new(result).gate(),
        Err(msg) => Err(format!(
            "type-tables T1 扫描未完成（口径不完整 ⇒ 判定不成立）：{msg}"
        )),
    }
}

/// `--report`：完整文本报告（T1/T2）。
pub fn report(root: &Path) -> Result<String, String> {
    let scan = scan::scan(root)?;
    let forward = t1::ForwardReport::new(scan);
    let names = tables::check(root)?;

    let mut out = String::new();
    out.push_str("=== P6 类型表门禁报告（tools/type-tables --report）===\n");
    out.push('\n');
    out.push_str("**T1 已是构建门禁**（6.6.2，2026-10-30）：违规即 build.rs panic 拒绝编译——\n");
    out.push_str("零构造变体 / 白名单与实现不一致 / 登记豁免未命中 / 未登记兜底点 /\n");
    out.push_str(
        "勘察基线漂移（T1.6）走同一条失败路径；通过时只打一行摘要。T2 仍只报告、不接构建路径。\n",
    );
    out.push_str(&format!(
        "扫描根：{SCAN_ROOT_REL}/（排除 tests/ 与 */tests/*）\n"
    ));
    out.push('\n');
    out.push_str(&forward.render());
    out.push_str(&names.render());
    Ok(out)
}

/// `--json`：T1 的结构化结果（供 CI / 后续任务消费；不改变文本报告）。
pub fn report_json(root: &Path) -> Result<String, String> {
    let scan = scan::scan(root)?;
    let forward = t1::ForwardReport::new(scan);
    let value = forward.to_json();
    serde_json::to_string_pretty(&value).map_err(|e| format!("序列化 T1 结果失败: {e}"))
}
