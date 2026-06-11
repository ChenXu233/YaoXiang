# types/eval/ — 纯类型代数

**职责**：对类型的纯函数操作。输入类型，输出归约后的类型或错误。不涉及程序上下文、不涉及证明。

**文件**：
- `evaluator.rs` — TypeEvaluator：常量折叠、If/Match/Nat 归约
- `normalizer.rs` — TypeNormalizer：βδι-规约 + 范式化
- `conditional.rs` — 条件类型 + MatchType
- `const_eval.rs` — Const 泛型求值 + ConstFunction
- `reducer.rs` — TypeReducer + TypeUnifier
- `operations.rs` — 类型级运算
- `type_families.rs` — 类型族
- `dependent_types.rs` — 依赖类型

**依赖**：types/（mono, var, const_data, substitute）

**被依赖**：typecheck/layers/equivalence（通过 evaluator 做确定性归约）

**规则**：
- 所有函数是纯函数，不持有可变状态
- 返回 `Result<MonoType, EvalError>`，不返回 ProofResult
- 归约不了就返回原类型（不是错误）
