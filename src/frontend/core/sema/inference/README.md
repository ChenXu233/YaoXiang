# inference/ — HM 类型推断引擎

**职责**：实现 Hindley-Milner 类型推断。推断 expr/stmt 的具体类型，维护 TypeEnvironment。

**结构**：
- `expressions/` — 表达式类型推断（`ExpressionInferrer` 的方法按簇拆分）
  - `mod.rs` — 构造器与共享非方法项（只读视图、构造器应用与类型参数代换）
  - `expr.rs` — 表达式与语句推断、模式匹配及注入接口
  - `call/mod.rs` — 调用推断入口 `infer_call_expr` 与变体构造器调用
  - `call/args.rs` — 命名实参与构造器字段绑定
  - `call/instantiation.rs` — 泛型实例化请求与类型变量收集
  - `call/method.rs` — 方法调用接收者静态类型与绑定键
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

**依赖**：`types/`、`sema/environment`

**被依赖**：`checker.rs`、`layers/`（部分）
