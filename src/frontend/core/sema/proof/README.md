# typecheck/proof/ — 管道基础设施

**职责**：定义证明管道的共享类型——ProofResult 和 ProofContext。所有 layers/ 和 checker.rs 都使用这些类型。

**文件**：
- `verdict.rs` — ProofResult (Proved / Disproved(Model) / Unproven)
- `context.rs` — ProofContext（路径条件栈 + 假设集合 + 类型依赖图 + 类型环境引用）

**依赖**：types/（MonoType）, typecheck/environment（TypeEnvironment）

**被依赖**：typecheck/layers/*, typecheck/checker.rs, typecheck/passes/*

**规则**：
- proof/ 只定义类型，不包含检查逻辑
- ProofContext 是唯一的状态传递载体
- 不依赖任何 layer，不依赖 passes
