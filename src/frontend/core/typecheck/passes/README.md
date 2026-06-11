# typecheck/passes/ — 独立分析遍

**职责**：不参加证明管道排队的独立检查。每个遍独立执行，互不依赖。

**文件**：
- `dead_code.rs` — 死代码检测
- `spawn_placement.rs` — spawn 块位置合法性检查
- `overload.rs` — 重载解析

**规则**：
- passes 之间不互相调用
- 可以依赖 proof/ 的类型定义
- 不能依赖 layers/（passes 不是管道的层）
- 新增遍只需在 mod.rs 加一行 `pub mod xxx;`
