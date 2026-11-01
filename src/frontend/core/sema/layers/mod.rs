//! 证明层
//!
//! 本文件只声明模块，不声明层序——调用顺序由调用点决定：父模块的 `checker.rs`
//! 里，`check_module_impl` 先跑精化走查（`predicate::check_predicate`），再跑
//! `termination`，最后跑 `ownership`；本目录内部互不引用。
//! 「下层失败上层不跑」不成立（无此实现）：任一层证伪/证不出只转成诊断，不阻断后续层。
//! 各层证明入口返回 `ProofResult` 或 `Vec<ProofResult>`；调用点清单见 `README.md`。

pub mod equivalence;
pub mod ownership;
pub mod predicate;
pub mod termination;

#[cfg(test)]
mod tests;
