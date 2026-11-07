//! WBS 4.5.2 / RFC-039 D20：overload_resolutions「产出 ⊆ 消费」判据（键 = 调用点 PlanId）
//!
//! 规范来源:
//! - docs/src/rfc/accepted/039-compiler-architecture.md D20（`ReleasePlan` /
//!   `overload_resolutions` 的 Span 键控 → `PlanId`，消除跨层 span 契约）
//! - docs/src/dev/architecture/07-equivalence-oracle.md §修复正确性漏洞的专门判据
//!   （同款判据形态：产出键集 ⊆ 消费键集，**无白名单，差集必须为空**）
//!
//! 判据形态（与 release_plan_spans 同款、同一套结算事实）：全语料
//!（tests/yaoxiang/ + src/std/tests/，剔 expect: compile-error 与 // skip:）逐文件走
//! lex→parse→typecheck→ir_gen **真实生成序列**，结算 `OverloadResolutions` 自己的
//! 两侧事实：产出键（`entries`）与消费台账（`get` 命中即记账）。差集 =
//! `OverloadResolutions::unconsumed()`，必须为空。
//!
//! # 红态证据（本判据建立时的实测，2026-10-13）
//!
//! 判据落地时消费侧仍带 `if named_args.is_empty()` 门（改动前 ir_gen 的行为）：
//! 命名实参调用若已被 typecheck 决议，ir_gen 不查表、**静默回落到未混编名**——
//! 实测差集即这一类的逐文件清单（见下方 `PRE_FIX_EXAMINED` 与提交记录）。
//! 处置：消费点取消该门（决议出的名字必须被真正使用），先红后绿。
//!
//! 范围说明：多文件路径（orchestrator 逐文件 AstToIrGenerator）共用同一台账，
//! 但生成器不跨编排边界暴露，本判据先覆盖单文件 pipeline 语料面。
