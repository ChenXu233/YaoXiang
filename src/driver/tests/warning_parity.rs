//! WBS 3.4.6 判据 C2：警告面归一——**多文件与单文件两条形态**对同一份源码的
//! W 码集合逐条一致。
//!
//! 修复前缺口（红态来源）：`MULTI_FILE_STAGES` 无 `RoleClassification` /
//! `DeadCodeAnalysis` 两阶段，且 `typecheck_orchestrated` 的 FailFast 分支
//! 从不读 `result.warnings` ⇒ 多文件 `run` 恒零警告（02-stage-contract
//! §11 处阶段覆盖不一致表第 2/4/5 行；决策 B1 已于 WBS 3.4.6 撤销）。
//!
//! 判据口径（07 §三层判据的诊断列表归一规则）：W 码集合按 `(码, 行)` 排序后
//! 逐条比对——**消息文本不参与比对**；文件身份两形态构造性不同（单文件是内存
//! 源码 `test.yx`，多文件是落盘路径），故文件维度不进判据。
//!
//! 两形态的警告 sink 现状（本模块的存在理由就是让它们给出同一集合）：
//! - `SingleFile`（pipeline 通道）：`CompilationResult.warnings`；
//! - `MultiFile`（编排通道）：逐文件收集 `DriverOutcome.check_diagnostics`。
