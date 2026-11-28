//! 运行期形参槽位口径 — 基于 RFC-039 6.5（表示重塑）与 8.9.1（参数位形态归一，D53）
//!
//! 判据单点：`ast::Param::occupies_runtime_param_slot` = **带类型标注**（`ty.is_some()`）。
//!
//! 历史与裁定（WBS 8.9.1，D53）：6.5 重塑的基线口径里，参数位**裸标识符**
//! （`(Int64)` → `Param.name` 是标识符、`ty = None`）不占运行期形参位；
//! D53 裁决该形态是**未命名带型参数**，裸标识符按类型解析并写进 `ty`，
//! 于是它现在**占**位。`ty = None` 只剩无标注值形参（lambda 头 / HM 推断位）。
//!
//! 判据本身未变（仍 = 带类型标注），变的是该形态的 `ty`；故本文件不再有
//! 「裸标识符不占位」的锁定用例——解析形状由
//! `parser::statements::tests::types::test_bare_type_params_parse_as_unnamed_typed`
//! 钉住，槽位口径由 `Param::occupies_runtime_param_slot` 的文档与本注记同源。
