# typecheck/layers/ — 有序证明层

**职责**：证明管道的检查逻辑。按层序执行，下层失败上层不跑。

**层序**：
| 层 | 文件 | 证明的命题 | 输入 | 依赖 |
|----|------|-----------|------|------|
| 0 | equivalence.rs | 类型等式 | 两个类型 | types/eval |
| 1 | ownership.rs | 所有权/令牌无冲突 | 变量操作 | Layer 0 |
| 2 | termination.rs | 循环/递归有限步退出 | 循环体 AST | Layer 0, 1 |
| 3 | predicate.rs | 精化谓词满足 | 表达式 + 上下文 | Layer 0, 1, 2 |

**规则**：
- 每层返回 `ProofResult`
- 每层只依赖序号更小的层和 proof/
- 层间不跨层调用（Layer 2 不直接调 Layer 0 的内部函数）
- 新增层必须在 mod.rs 中声明顺序
