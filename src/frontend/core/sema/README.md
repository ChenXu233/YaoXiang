# typecheck/ — 证明管道（编译器验证引擎）

**职责**：编排所有编译期验证。通过证明管道逐层检查程序，每层返回 ProofResult。

**结构**：
- `checker.rs` — TypeChecker：管道编排者
- `environment.rs` — TypeEnvironment：共享状态
- `signature.rs` — 签名解析工具
- `semantic_db.rs` — 语义信息收集
- `proof/` — 管道基础设施（ProofResult + ProofContext）
- `layers/` — 证明层（调用顺序 predicate → termination → ownership，由 `checker.rs::check_module_impl` 的调用点决定；详见 `layers/README.md`）
- `passes/` — 独立分析遍（无层间依赖）
- `inference/` — HM 类型推断引擎
- `traits/` — Trait 系统

**依赖**：types/、types/eval/、middle/（所有权检查）
nam
**被依赖**：LSP、REPL
