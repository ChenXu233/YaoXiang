# inference/ — HM 类型推断引擎

**职责**：实现 Hindley-Milner 类型推断。推断 expr/stmt 的具体类型，维护 TypeEnvironment。

**结构**：
- `expressions.rs` — 表达式类型推断
- `statements.rs` — 语句类型检查
- `scope.rs` — 作用域管理
- `assignment.rs` — 赋值兼容性检查
- `bounds.rs` — trait 边界检查
- `patterns.rs` — 模式匹配推断
- `types.rs` — 类型系统核心

**不做**：
- 不做证明（那是 `layers/`）
- 不做死代码检测（那是 `passes/dead_code`）
- 不做类型关系判断（那是 `layers/equivalence`）

**依赖**：`types/`、`typecheck/environment`

**被依赖**：`checker.rs`、`layers/`（部分）
