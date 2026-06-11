# types/ — 类型系统 IR

**职责**：定义类型的数据结构和对类型的纯操作。不包含程序上下文、路径条件、证明逻辑。

**文件**：
- `mono.rs` — MonoType, PolyType, TypeBinding, UniverseLevel
- `var.rs` — TypeVar, ConstVar
- `const_data.rs` — ConstValue, ConstExpr, ConstKind
- `constraint.rs` — TypeConstraint
- `solver.rs` — TypeConstraintSolver
- `substitute.rs` — 类型替换
- `trait_data.rs` — Trait 定义体系
- `error.rs` — 类型错误
- `eval/` — 类型代数操作（见 eval/README.md）

**依赖**：无（不依赖 typecheck/、middle/、backends/）

**被依赖**：typecheck/、backends/、LSP、REPL、middle/
