# Компиляторный вычислительный движок (CTE Engine) и статическая верификация по Хоару — план реализации

> **Задача**: Реализовать компиляторный вычислительный движок + канал статической верификации по Хоару для поддержки зависящих от значений типов и проверки размерностей на этапе компиляции
> **На основе RFC**: RFC-010 (унифицированный синтаксис типов), RFC-011 (обобщённая система/зависящие от значений типы), RFC-022 (статическая верификация по Хоару)
> **Дата**: 2026-05-10
> **Статус**: В разработке
> **Целевые вехи**:
> - M1: Скелет свёртывания констант + анализа чистоты
> - M2: Компиляторное вычисление чистых функций + проверка завершения
> - M3: Вычисления на уровне типов (`If`/`Assert`/`match` семейства типов)
> - M4: Верификация по Хоару (`//!` спецификации → генерация VC → подключение к SMT)

---

## Аннотация

Зависящие от значений типы YaoXiang (RFC-011) требуют, чтобы типы могли зависеть от значений, известных на этапе компиляции (например, `Vec(factorial(5))` → `Vec(120)`), а статическая верификация по Хоару (RFC-022) требует проверки спецификаций чистых функций на этапе компиляции. Обе системы разделяют одну ключевую потребность: **безопасное выполнение/анализ чистых функций на этапе компиляции**.

Данный план предлагает **унифицированный компиляторный вычислительный движок (CTE Engine)**, абстрагирующий анализ чистоты, проверку завершения и вычисление выражений в общую инфраструктуру, обслуживающую два потребителя: вычисления на уровне типов и верификацию по Хоару.

---

## Основные принципы проектирования

1. **Повторное использование системы владения для анализа чистоты**: `&mut` в YaoXiang — это маркер побочных эффектов — проверка заимствований уже сообщает нам, что будет изменено
2. **Чистая функция = вычислимая на этапе компиляции**: без ключевого слова `const fn`, компилятор автоматически выводит чистоту
3. **Доказательство завершения = основа типобезопасности**: вычисление в позиции типа должно доказать завершение (`decreases` спецификация), иначе система типов неразрешима
4. **Частичное вычисление лучше полного**: сколько параметров известно — столько и вычисляем, остальное оставляем для времени выполнения
5. **Разделение интерпретатора для компиляции и логики Хоара**: одно ядро вычисления выражений обслуживает оба типа потребителей
6. **Двухрежимное вычисление**: конкретное вычисление (известные параметры → результат `CTValue`) и символьное вычисление (неизвестные параметры → SMT выражение) используют общую структуру интерпретатора, различаясь только средой вычисления

---

## Общая архитектура

```
                            Исходный файл + //! спецификации
                                    ↓
                            ┌──────────────┐
                            │   Парсер     │
                            │ (распознаёт //! комментарии)│
                            └──────┬───────┘
                                   ↓
                            ┌──────────────┐
                            │  Проверка типов │
                            │ • сбор спецификаций │
                            │ • обнаружение зависимостей по значению │
                            └──────┬───────┘
                                   ↓
              ┌────────────────────┴────────────────────┐
              ↓                                         ↓
   ┌──────────────────────┐                ┌──────────────────────┐
   │   CTE движок          │                │  Верификатор Хоара    │
   │                      │                │                      │
   │  ┌────────────────┐  │                │  1. Сбор //! спецификаций│
   │  │  Анализатор чистоты│  │                │  2. Генерация условий верификации (VC)│
   │  │  (на основе владения)│  │                │  3. SMT решатель (Z3)│
   │  └───────┬────────┘  │                │  4. Отчёт о контрпримерах│
   │  ┌───────┴────────┐  │                └──────────┬───────────┘
   │  │  Проверка завершения│  │                           │
   │  │  (decreases)    │  │                           ↓
   │  └───────┬────────┘  │                ┌──────────────────────┐
   │  ┌───────┴────────┐  │                │  Результат верификации│
   │  │  AST интерпретатор│  │                │  • успех → кэш       │
   │  │                │  │                │  • неудача → блокировка release │
   │  │ ┌────────────┐ │  │                └──────────────────────┘
   │  │ │ Конкретное │ │  │                       ↑
   │  │ │ вычисление │ │  │                       │
   │  │ │ env: всё известно│  │  Общий интерпретатор   │
   │  │ │ → CTValue  │ │  │  (AST обход/инлайн/раскрытие циклов)│
   │  │ └────────────┘ │  │                       │
   │  │ ┌────────────┐ │  │                       │
   │  │ │ Символьное │─┼──┼───────────────────────┘
   │  │ │ вычисление │ │  │  (потребляется логикой Хоара)
   │  │ │ env: частично известно│  │
   │  │ │ → SMTExpr  │ │  │
   │  │ └────────────┘ │  │
   │  └───────┬────────┘  │
   │          ↓           │
   │  Встраивание результата в тип/мономорфизацию │
   └──────────────────────┘
```

---

## 一、编译期值（CTValue）

Компиляторные вычисления的核心数据类型。设计为编译器内部 IR 值，不与运行时值混淆。

```rust
/// 编译期求值结果
enum CTValue {
    /// 整数（涵盖 Bool 的所有编译期用途）
    Int(i64),

    /// 浮点数
    Float(f64),

    /// 字符串（错误消息、类型名等）
    String(SmolStr),

    /// 类型引用——类型级计算的核心
    /// YaoXiang 的类型自身是 Type1 层的"值"
    Type(TypeId),

    /// 异构元组
    Tuple(Vec<CTValue>),

    /// 同构数组
    Array(Vec<CTValue>),

    /// 结构化值
    Struct {
        type_id: TypeId,
        fields: HashMap<SmolStr, CTValue>,
    },

    /// 未求值的函数引用（部分求值时保留）
    /// 当所有参数已知时内联求值，否则保留为运行时调用
    Thunk {
        func: FunctionId,
        known_args: Vec<CTValue>,
        unknown_params: Vec<ParamId>,
    },
}
```

**关键设计**：`CTValue::Type(TypeId)` 让类型成为一等编译期值。`If(C, T, E)` 的 C 求值为 `CTValue::Bool`，T/E 求值为 `CTValue::Type`。

---

## 二、子系统 1：纯度分析器

### 2.1 设计思路

复用 YaoXiang 的所有权系统（RFC-009），副作用天然由类型签名表达：

| 参数模式 | 含义 | 编译期可求值？ |
|----------|------|---------------|
| `x: T` (owned) | 获取所有权，可自由修改 | ✅ |
| `x: &T` (共享引用) | 只读 | ✅ |
| `x: &mut T` (独占引用) | 可修改 | ⚠️ 取决于 T 来源 |
| I/O 调用 | 外部副作用 | ❌ |
| 调用非纯函数 | 传递性 | ❌ |

### 2.2 算法

```
analyze_purity(func: FunctionId, ctx: &mut PurityContext) -> PurityResult:
    // 1. 快速路径：已被标注
    if ctx.has_purity_annotation(func):
        return ctx.get_annotation(func)

    // 2. 检查直接副作用
    for op in func.body.operations():
        match op:
            Call(callee, _) if is_io_operation(callee):
                return Impure("IO operation")
            Call(callee, args) where has_mut_arg(args):
                if arg_escapes_function(args):
                    return Impure("mutation of external state via &mut")
            Call(callee, _):
                // 传递性：被调用者必须也是纯函数
                if analyze_purity(callee, ctx).is_impure():
                    return Impure("calls impure function: {callee}")

    // 3. 默认推断为纯函数
    return Pure
```

### 2.3 不提供显式纯度标注

**设计决策：不提供 `//! pure` 之类的显式标注。**

所有权系统（RFC-009）已经通过类型签名表达了副作用信息——`&mut T` 就是修改、I/O 操作就是外部副作用。编译器有能力自动推断纯度。

如果编译器误判一个纯函数为不纯，那是编译器的 bug，应该修编译器，而不是让用户打补丁。提供一个"相信我，这函数是纯的"的标注只会掩盖真实问题。

> *"不要写兼容、回退、临时、备用、特定模式生效的代码。让问题直接暴露。"*

### 2.4 与 RFC-022 的关系

纯度分析器同时服务于：
- **CTE**：非纯函数不能在类型位置使用
- **霍尔逻辑**：规约表达式（`requires`/`ensures` 的右侧）必须是纯函数调用

---

## 三、子系统 2：终止检查器

### 3.1 设计思路

类型位置的编译期求值必须保证终止，否则类型系统就不可判定。YaoXiang 通过 `//! decreases` 规约来证明终止。

```
//! decreases: <expr>
```

其中 `<expr>` 是一个有下界的良基值（通常是 `Int` 类型的自然数）。

### 3.2 算法

```
check_termination(func: FunctionId, ctx: &mut TermContext) -> TermResult:
    // 1. 查找 decreases 规约
    let decreases_expr = find_decreases_spec(func)
        .or_else(|| infer_decreases(func))

    match decreases_expr:
        None if has_recursive_call(func):
            return TermError::NoDecreasesAnnotation
        None:
            return TermOk  // 无递归，无需证明

        Some(decreases):
            // 2. 验证每个递归调用点
            for call in func.recursive_calls():
                let dec_at_call = eval_decreases_at(call, decreases)
                let dec_at_entry = eval_decreases_at(func.entry, decreases)

                if !strictly_less_than(dec_at_call, dec_at_entry):
                    return TermError::NotDecreasing {
                        at: call.location,
                        expected_less_than: dec_at_entry,
                        actual: dec_at_call,
                    }

            // 3. 验证下界
            if !has_lower_bound(decreases):
                return TermError::NoLowerBound

            return TermOk
```

### 3.3 自动推断

一些明显的终止情况无需标注：

```yaoxiang
// 无需 decreases——编译器看到循环有已知上界 n
sum: (arr: Array(Int, n)) -> Int = {
    s = 0; i = 0
    while i < n { s += arr[i]; i += 1 }
    return s
}
```

需要标注的情况：
```yaoxiang
// 必须标注 decreases——递归调用 n-1
factorial: (n: Int) -> Int = {
    //! decreases: n
    if n <= 1 { return 1 }
    return n * factorial(n - 1)
}
```

### 3.4 与 RFC-022 的关系

终止检查器同时服务于：
- **CTE**：decreases 是编译期求值的准入门槛
- **霍尔逻辑**：循环不变式的 decreases 变体（`/*! decreases: n - i !*/`）同样由终止检查器验证

---

## 四、子系统 3：AST 解释器

### 4.1 设计思路

解释器基于 AST 遍历，维护一个求值环境（变量名 → CTValue 映射）。核心能力是**部分求值**：已知参数就计算，未知参数就保留。

```
eval(expr: &Expr, env: &mut EvalEnv) -> EvalResult<CTValue>:
    match expr:
        // 字面量 → 直接转换
        Literal(lit) => lit.into_ctvalue()

        // 变量 → 查找环境
        Variable(name) => env.get(name).ok_or(NotInScope)

        // 二元运算
        BinaryOp(l, op, r) =>
            let lv = eval(l, env)?; let rv = eval(r, env)?
            apply_op(op, lv, rv)

        // 条件分支
        If(cond, then, else) =>
            match eval(cond, env)? {
                Bool(true) => eval(then, env),
                Bool(false) => eval(else, env),
                _ => Err(ExpectedBool),
            }

        // 函数调用——核心逻辑
        Call(func, args) =>
            let known_args = args.filter_map(|a| eval(a, env).ok())
            if known_args.len() == args.len():
                // 全参数已知 → 内联求值
                inline_and_eval(func, known_args, env)
            else if known_args.len() > 0:
                // 部分已知 → 部分求值（产出一元化代码）
                partial_eval(func, known_args, env)
            else:
                // 全部未知 → Thunk
                CTValue::Thunk { func, known_args: vec![], unknown_params: args }

        // 模式匹配
        Match(scrutinee, arms) =>
            let val = eval(scrutinee, env)?
            for arm in arms:
                if arm.pattern.matches(val):
                    return eval(arm.body, env.with_bindings(arm.bindings))
            Err(NoMatch)

        // 代码块
        Block(stmts) =>
            if stmts.is_empty():
                return CTValue::Void
            for stmt in stmts[..len-1]:
                eval(stmt, env)?
            eval(stmts.last(), env)

        // 循环
        While(cond, body) =>
            let mut result = CTValue::Void
            while eval(cond, env)? == Bool(true):
                check_step_limit()?  // 防止死循环
                result = eval(body, env)?
            result
```

### 4.2 内联求值

当所有参数已知时，解释器将函数体内联到求值上下文：

```
inline_and_eval(func, args, env):
    // 1. 检查纯度
    purity.check(func)?

    // 2. 检查缓存
    if let Some(cached) = cache.get(func, args):
        return cached

    // 3. 创建内联环境
    let mut inline_env = env.child()
    for (param, arg) in func.params.zip(args):
        inline_env.bind(param.name, arg)

    // 4. 求值函数体
    let result = eval(&func.body, &mut inline_env)?

    // 5. 缓存结果
    cache.insert(func, args, result.clone())
    result
```

### 4.3 步数限制

编译期求值必须有硬限制，防止即使有 `decreases` 也意外超时的场景：

```
const MAX_EVAL_STEPS: u64 = 1_000_000;  // 一百万步硬上限

struct EvalEnv {
    variables: HashMap<SmolStr, CTValue>,
    step_count: u64,
    step_limit: u64,
}
```

### 4.4 双模式求值：具体 vs 符号

解释器核心框架（AST 遍历、内联展开、模式匹配）是统一的，但**求值环境**决定了两种模式：

#### 4.4.1 具体求值（Concrete Evaluation）

**消费者**：CTE 引擎 → 类型级求值、单态化

**特征**：
- 环境中所有变量都有具体的 `CTValue`
- 函数调用参数全部已知 → 内联求值
- 产出：`CTValue`（具体值或类型引用）
- 失败 = 编译错误

```
// 场景：Vec(factorial(5))
// env = { factorial → Function(...) }
eval(Call("factorial", [Literal(5)]), env):
    → inline_and_eval(factorial, [CTValue::Int(5)], env)
    → CTValue::Int(120)
// 类型替换：Vec(120)
```

#### 4.4.2 符号求值（Symbolic Evaluation）

**消费者**：霍尔逻辑验证器 → SMT 求解

**特征**：
- 环境中存在**符号变量**（如函数参数 `n`、`arr`，编译期未知）
- 已知子表达式求值为具体值，未知部分保留为 SMT 符号
- 函数调用不会内联——而是展开为逻辑公式
- 产出：`SMTExpr`（一阶逻辑表达式），交给 Z3
- 失败 = 验证不通过（非编译错误）

```
// 场景：验证 max 的 ensures:
//   //! ensures: GreaterOrEqual(result, arr[0..n]) = result >= forall arr[i]
// env = { result → Symbol("result"), arr → Symbol("arr"), n → Symbol("n") }
eval(BinaryOp(Variable("result"), GtEq, Call("arr_max", [Symbol("arr"), Symbol("n")]))):
    // result 是符号 → 保留
    // arr_max(arr, n) 是纯函数但参数未知 → 展开为逻辑定义
    → SMTExpr::Forall(i in 0..n, Symbol("result") >= Symbol("arr")[i])
// 交给 Z3：∀arr, n, result. (n > 0 ∧ ...) → result >= arr[0] ∧ ... ∧ result >= arr[n-1]
```

#### 4.4.3 两种模式的关键区别

| 维度 | 具体求值 | 符号求值 |
|------|----------|----------|
| 环境 | `HashMap<Name, CTValue>` | `HashMap<Name, SMTTerm>` |
| 变量未知时 | 报错 | 保留为符号 |
| 函数调用 | 内联 + 求值函数体 | 展开为逻辑定义（不执行） |
| 循环 | 实际迭代（带步数限制） | 转换为循环不变式 VC |
| 产出类型 | `Result<CTValue, CTError>` | `Result<SMTExpr, SMError>` |
| 失败语义 | 编译错误 | 验证失败（可降级为 Runtime Check） |
| 性能特征 | 快（直接计算） | 慢（SMT 求解） |

#### 4.4.4 共享的解释器框架

两种模式共享同一套 AST 遍历骨架：

```rust
/// 解释器 trait：具体求值和符号求值各自实现
trait Interpreter {
    type Value;       // CTValue 或 SMTExpr
    type Error;       // CTError 或 SMError

    fn eval_literal(&mut self, lit: &Literal) -> Result<Self::Value, Self::Error>;
    fn eval_variable(&mut self, name: &str) -> Result<Self::Value, Self::Error>;
    fn eval_binary_op(&mut self, op: BinOp, l: Self::Value, r: Self::Value) -> Result<Self::Value, Self::Error>;
    fn eval_call(&mut self, func: FunctionId, args: &[Expr]) -> Result<Self::Value, Self::Error>;
    fn eval_if(&mut self, cond: &Expr, then: &Expr, else_: &Expr) -> Result<Self::Value, Self::Error>;
    fn eval_match(&mut self, scrutinee: &Expr, arms: &[MatchArm]) -> Result<Self::Value, Self::Error>;
    fn eval_while(&mut self, cond: &Expr, body: &Expr) -> Result<Self::Value, Self::Error>;
}

/// 统一的 AST 遍历器，委托给具体实现
fn eval_ast<I: Interpreter>(interp: &mut I, expr: &Expr) -> Result<I::Value, I::Error> {
    match expr {
        Expr::Literal(lit) => interp.eval_literal(lit),
        Expr::Variable(name) => interp.eval_variable(name),
        Expr::BinaryOp { op, left, right } => {
            let l = eval_ast(interp, left)?;
            let r = eval_ast(interp, right)?;
            interp.eval_binary_op(*op, l, r)
        }
        Expr::Call { func, args } => interp.eval_call(*func, args),
        Expr::If { cond, then, else_ } => interp.eval_if(cond, then, else_),
        // ... 其余 AST 节点同理
    }
}
```

**关键洞察**：具体求值和符号求值的 AST 遍历逻辑完全相同，差异仅在于：
- **用什么表示值**（`CTValue` vs `SMTExpr`）
- **函数调用怎么处理**（内联执行 vs 逻辑展开）
- **未知变量怎么处理**（报错 vs 保留符号）

---

## 五、CTE 与其他组件的交互

### 5.1 类型级求值用例

CTE 用于在编译时计算以下类型表达式：

1. 类型标注位置
   `Vec(factorial(5))` → `CTE::eval(factorial(5))` → `CTValue::Int(120)`
   类型替换为 `Vec(120)`

2. 泛型值参数
   `Array(Int, factorial(3))` → `CTE::eval(factorial(3))` → `CTValue::Int(6)`
   实例化为 `Array(Int, 6)`

3. Assert 类型
   `Assert(N > 0)` → `CTE::eval(N > 0)` → `CTValue::Bool(true/false)`
   True → Void, False → compile_error("N must be > 0")

4. If 条件类型
   `If(C, T, E)` → `CTE::eval(C)` → `CTValue::Bool(b)`
   True → T, False → E

5. Match 类型族
   `AsString(Int)` → match Int { Int => String, ... } → String

### 5.2 与单态化的交互

```
单态化在以下位置使用 CTE 结果：

1. 已知泛型值参数 → 生成具体实例
   List(Int) 的 push 方法 → 生成 push_List_Int

2. 已知值依赖类型 → 展开为具体类型
   Matrix(Float, 3, 3).data → Array(Array(Float, 3), 3)

3. 部分求值 → 生成一元化代码
   map(Int, String) → 生成 map_Int_String，其中 T=Int, R=String 已固定
```

### 5.3 与霍尔逻辑验证器的交互

```
验证器在以下位置使用 CTE：

1. 规约表达式的部分求值
   //! requires: n > 0 && factorial(n) < MAX
   CTE::eval(factorial(n)) → 如果 n 编译期已知 → 常量
                           → 如果 n 未知 → 保留为符号，交给 SMT

2. 规约条件简化
   //! ensures: result >= 0 && result < n
   CTE 尝试简化已知子表达式，减少 SMT 求解负担

3. 规约类型实例化
   NonEmpty(n) = n > 0
   CTE 将规约类型展开为布尔表达式
```

---

## 六、霍尔逻辑静态验证（RFC-022 实现设计）

### 6.1 规约解析

`//!` 和 `/*! ... !*/` 由解析器识别为特殊注释节点附加到 AST：

```rust
struct SpecAnnotation {
    kind: SpecKind,        // Requires | Ensures | Invariant | Decreases
    name: Option<SmolStr>, // 规约名称（可选的用户命名）
    spec_type: TypeExpr,   // 规约类型表达式
    expr: Expr,            // 布尔表达式
    span: Span,
}

enum SpecKind {
    Requires,
    Ensures,
    Invariant,
    Decreases,
}
```

### 6.2 验证条件生成（VCGen）

采用最弱前置条件（Weakest Precondition）演算：

```
generate_vc(func: FunctionId) -> Vec<VerificationCondition>:
    let requires = collect_requires(func)
    let ensures = collect_ensures(func)
    let invariants = collect_invariants(func)
    let decreases = collect_decreases(func)

    let mut vcs = Vec::new()

    // VC1: 前置条件的一致性
    vcs.push(VC::PreconditionConsistency(requires))

    // VC2: 后置条件的验证（对每条执行路径）
    for path in func.paths():
        let wp = compute_wp(path.body, ensures)
        vcs.push(VC::Postcondition {
            path: path.id,
            formula: implies(requires, wp),
        })

    // VC3: 循环不变式
    for (loop_, invariant) in invariants:
        // 进入循环前成立
        vcs.push(VC::InvariantEntry { loop_, invariant })
        // 每次迭代保持
        vcs.push(VC::InvariantPreservation { loop_, invariant })
        // 退出后蕴含后置条件
        vcs.push(VC::InvariantExit { loop_, invariant, post: ensures })

    vcs
```

### 6.3 SMT 求解器集成

```
┌─────────────┐     SMT-LIB 格式      ┌───────────┐
│  VC Generator │ ──────────────────→ │  Z3 求解器 │
└─────────────┘                       └─────┬─────┘
                                            │
                          ┌─────────────────┴──────────────┐
                          ↓                                ↓
                      unsat                            sat
                          ↓                                ↓
                    ┌──────────┐                   ┌──────────────┐
                    │ 验证通过  │                   │ 提取反例模型  │
                    │ 缓存结果  │                   │ 转换为可读格式│
                    └──────────┘                   └──────┬───────┘
                                                         ↓
                                                  ┌──────────────┐
                                                  │ 编译错误报告  │
                                                  │ • 输入值       │
                                                  │ • 违反的规约   │
                                                  └──────────────┘
```

### 6.4 编译模式

| 模式 | 行为 | CLI |
|------|------|-----|
| **Debug Build** | 解析规约，生成 VC，调用 Z3 证明；验证通过后才能 Release Build | `yaoxiangc --debug` |
| **Release Build** | 忽略所有 `//!` 注释，零开销，清除验证缓存 | `yaoxiangc --release` |
| **Runtime Checks** | 规约降级为 `assert` 语句，违规时 panic | `yaoxiangc --runtime-checks` |

---

## 七、实现阶段

### Phase 1：常量折叠 + 纯度分析骨架

**目标**：建立 CTE 基础设施，支持最基本的编译期求值

**内容**：
- [ ] 定义 `CTValue` 枚举和 `EvalEnv` 结构
- [ ] 实现 `eval()` 的基本路径：字面量、变量、二元运算、条件、代码块
- [ ] 实现纯度分析器的第一版：识别 I/O 调用为不纯，其他默认为纯
- [ ] 在类型检查器中插入 CTE 调用点（类型标注位置）
- [ ] 常量折叠：`1 + 2 * 3` 在编译期算成 `7`
- [ ] 死分支消除：`if true { ... } else { ... }` → 直接取 then 分支
- [ ] 单元测试：字面量求值、简单表达式、常量折叠

**产出**：`src/middle/cte/` 模块，包含 `value.rs`、`eval.rs`、`purity.rs`

### Phase 2：纯函数编译期求值 + 终止检查

**目标**：支持纯函数在编译期的完整求值

**内容**：
- [ ] 实现函数内联求值：已知所有参数 → 展开函数体求值
- [ ] 实现 `//! decreases` 解析和终止验证
- [ ] 实现递归函数编译期求值（带步数限制）
- [ ] 实现求值结果缓存（Memoization）
- [ ] 完善纯度分析：利用所有权信息识别 `&mut` 副作用
- [ ] 部分求值：已知部分参数时的代码生成优化
- [ ] 集成测试：`factorial(5)` 在类型位置求值为 `120`

**产出**：`src/middle/cte/interpreter.rs`、`src/middle/cte/termination.rs`

### Phase 3：类型级计算

**目标**：支持 `If`/`Assert`/`match` 类型族

**内容**：
- [ ] 实现 `CTValue::Type(TypeId)` 的类型级操作
- [ ] 实现 `If: (C: Bool, T: Type, E: Type) -> Type` 的条件类型求值
- [ ] 实现 `Assert(C)` → `True → Void, False → compile_error`
- [ ] 实现类型级 `match`：`AsString: (T: Type) -> Type = match T { ... }`
- [ ] 值依赖类型的完整实例化：`Matrix(Float, 3, 3)` → 具体类型
- [ ] 编译期维度验证：矩阵乘法维度不匹配 → 编译错误
- [ ] 与单态化（mono pass）的集成

**产出**：`src/middle/cte/type_level.rs`，更新 `src/middle/passes/mono/`

### Phase 4：霍尔逻辑静态验证

**目标**：完整的规约解析、VC 生成、SMT 验证通道

**内容**：
- [ ] 解析器扩展：识别 `//!` 和 `/*! ... !*/` 为规约节点
- [ ] 规约类型定义（`NonEmpty`、`Sorted`、`GreaterOrEqual` 等标准库规约类型）
- [ ] 用户自定义规约类型支持
- [ ] VC 生成器：最弱前置条件演算
- [ ] Z3 SMT 求解器集成（通过 `z3` crate）
- [ ] SMT-LIB 格式翻译
- [ ] 反例提取与可读化报告
- [ ] Debug/Release/RuntimeChecks 三种编译模式切换
- [ ] 集成测试：验证 `max`、`binary_search` 等函数的规约

**产出**：`src/middle/verification/` 模块

---

## 八、模块规划

```
src/middle/
├── cte/                          # Компиляторный вычислительный движок
│   ├── mod.rs                    # Входная точка CTE, координация трёх подсистем
│   ├── value.rs                  # Определение CTValue + базовые операции
│   ├── eval.rs                   # Унифицированный AST обходчик (Interpreter trait + eval_ast)
│   ├── concrete.rs               # Реализация конкретного вычисления (ConcreteInterpreter → CTValue)
│   ├── symbolic.rs               # Реализация символьного вычисления (SymbolicInterpreter → SMTExpr)
│   ├── env.rs                    # EvalEnv (среда вычисления + лимит шагов)
│   ├── purity.rs                 # Анализатор чистоты
│   ├── termination.rs            # Проверка завершения (верификация decreases)
│   ├── type_level.rs             # Вычисления на уровне типов (семейства типов If/Assert/match)
│   └── cache.rs                  # Кэширование результатов вычисления
│
├── verification/                 # Статическая верификация по Хоару
│   ├── mod.rs                    # Входная точка верификации
│   ├── spec_parser.rs            # Парсинг //! спецификаций
│   ├── spec_types.rs             # Встроенные определения типов спецификаций
│   ├── vcgen.rs                  # Генератор условий верификации (WP исчисление)
│   ├── smt.rs                    # Интерфейс SMT решателя Z3
│   └── counterexample.rs         # Форматирование контрпримеров
│
└── passes/
    └── mono/                     # Существующая мономорфизация (расширенная интеграция CTE)
        └── ...                   # Использование результатов CTE для инстанцирования
```

---

## 九、Ключевые проектные решения

| Решение | Варианты | Выбор | Обоснование |
|---------|----------|-------|-------------|
| Способ определения чистоты | Явные аннотации vs автоматический вывод vs комбинация | **Автоматический вывод** | Система владения уже предоставляет достаточно информации; без аварийного люка с явными аннотациями |
| Компиляторный вычислитель | Ограниченное подмножество vs полный язык | **Полный язык (с лимитом шагов)** | Согласуется с унифицированным синтаксисом типов "всё есть `name: type = value`" |
| Доказательство завершения | Обязательные аннотации vs автоматический вывод | **В типовой позиции обязательно, в остальном автоматически** | Невозможность разрешения в типовой позиции = ошибка компиляции, в остальных местах можно放宽 |
| Генерация VC | WP исчисление vs SP исчисление | **WP исчисление** | WP проще и прямее, ошибки локализуются чётче |
| SMT решатель | Z3 vs CVC5 vs собственная разработка | **Z3** | Наиболее зрелый, лучшие Rust binding, наибольшее сообщество |
| Стратегия кэширования | Без кэша vs кэш между модулями | **LRU кэш + инкрементное аннулирование** | Результаты компиляторного вычисления — детерминированные чистые функции, идеально для кэширования |
| Режим компиляции | Единый режим vs разделение Debug/Release | **Debug верификация → Release с нулевыми накладными расходами** | Верификация дорогостояща, Release не должен её нести |

---

## 十、Риски и их снижение

| Риск | Влияние | Снижение |
|------|---------|----------|
| Сложность интеграции Z3 | Задержка Phase 4 | Использование зрелого crate `z3`; начать с простой арифметики, расширять постепенно |
| Таймаут компиляторного вычисления | Плохой UX | Лимит шагов + понятные сообщения об ошибках таймаута + предложения по упрощению выражений |
| Ошибка определения чистоты | Несоответствие результатов компиляции и выполнения | Система владения обеспечивает сильные гарантии; если ошибка — это баг компилятора, нужно чинить компилятор |
| Сложность отладки неудачной SMT верификации | Пользователь не понимает почему спецификация не выполняется | Извлечение контрпримера + отображение конкретных входных значений + подсветка пути выполнения |
| Значительное увеличение времени компиляции | Замедление CI | Инкрементная верификация + модульный кэш + файлы результатов верификации (аналогично `.o` файлам) |

---

## 十一、交叉引用现有 RFC

| RFC | 关系 | 本计划如何满足 |
|------|------|---------------|
| RFC-010 §统一语法 | CTValue 需支持所有类型表达式 | `CTValue::Type(TypeId)` + `CTValue::Struct` 覆盖 |
| RFC-011 §4.2 编译期计算 | 值依赖类型的核心机制 | Phase 2/3 实现 |
| RFC-011 §6 类型级计算 | `If`/`Assert`/`match` 类型族 | Phase 3 实现 |
| RFC-011 §终止检查机制 | decreases 规约 | Phase 2 的终止检查器实现 |
| RFC-022 §1 规约注释语法 | `//!` 解析 + 规约类型 | Phase 4 实现 |
| RFC-022 §3 验证机制 | VC 生成 + SMT 对接 | Phase 4 的 VCGen + SMT 模块 |
| RFC-009 §所有权模型 | 纯度分析的基础 | Phase 1/2 复用所有权信息 |

---

## 参考文献

- [RFC-010: 统一类型语法](../design/rfc/accepted/010-unified-type-syntax.md)
- [RFC-011: 泛型系统设计](../design/rfc/accepted/011-generic-type-system.md)
- [RFC-022: 霍尔逻辑静态验证](../design/rfc/draft/022-hoare-logic-static-verification.md)
- [RFC-009: 所有权模型](../design/rfc/accepted/009-ownership-model.md)
- [Z3 Prover](https://github.com/Z3Prover/z3)
- [SMT-LIB Standard](https://smtlib.cs.uiowa.edu/)
- [Weakest Precondition Calculus (Dijkstra)](https://en.wikipedia.org/wiki/Predicate_transformer_semantics)