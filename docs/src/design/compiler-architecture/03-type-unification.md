# 类型表示单一化

> **附属设计文档**。本文是 [RFC-039 编译器架构重构](../rfc/draft/039-compiler-architecture.md) 的附属文档。四层模型、验收判据分级与执行阶段顺序见 RFC-039 正文;各附属文档的定位见 [本目录索引](index.md)。

## 定位与范围

### 覆盖

把当前并行的多套类型表示收敛为一套，并消除由此派生的四类负担：

- **13 个生产零构造的 `ast::Type` 变体**（`ast.rs:427-541` 的 26 变体中，见 2.2）
- **一张手写的类型名同义词表**（`mono.rs:618-643`，见 2.3）
- **parser 对 typecheck 层的真实反向依赖**（2 处，见 2.4）
- **一个生产零构造的表达式变体** `Expr::FnDef`（`ast.rs:37-43`，见 2.6）

具体交付物是四样：26 变体逐个处置表（5.2）、`Type::Name` 的语义定种 `NameKind` 与同义词表消除（5.3）、`TypeEnvProbe` 注入式数据流（5.4）、类型表生成期门禁 T1-T3（5.7）。

### 目录改名与本文路径

RFC-039 决议 D1 决定执行目录改名（`typecheck/`→`sema/`、`middle/core/`→`middle/ir/`），**随 P5/P6 一并完成、不分阶段**。本文的 50 项改动清单与全部验收 grep **按改名前的现状路径书写**——它们是 P6 各批次在**改名批次之前**的施工基线；每批次收尾的目录改名是**纯搬移批次**（C1 zero-diff），单独 commit。基线在改名 commit 后失效属预期行为，以 git 历史为二分依据。

### 不覆盖

- **目录改名的施工方式**。`Type` 本阶段不搬离 `parser/ast.rs`；只在 `types/` 下新增 `lower.rs` 作为唯一转换点。目录改名（`typecheck/`→`sema/` 等）按 RFC-039 决议 D1 随 P5/P6 收尾执行，形态见 `01-routing.md` 的目标目录结构。
- **四层模型与依赖方向规范**。见 [01-routing.md](01-routing.md)。
- **等价性判据的分级与校验器实现**。C1-C6 的定义、三层判据（IR 校验器 / 规范化快照 / 语料差分）见 [07-equivalence-oracle.md](07-equivalence-oracle.md)。本文只引用其中 **C3（类型表示收敛）** 一条作为自己的验收判据。
- **SSA 化**。见 [04-ssa.md](04-ssa.md)。本文是它的前置，理由见下。
- **P1-P10 全局执行序列**。见 RFC-039 正文与 [02-stage-contract.md](02-stage-contract.md)。本文只给出自己这条线的阶段划分（见"实施要点"）。

### 与 RFC-039 的分工

RFC-039 是上位总纲：四层模型、判据分级、跨文档的阶段顺序、以及"为什么现在做这次重构"。本文是 RFC-039 中**类型表示收敛这一条**的施工图：具体删哪些变体、改哪些文件、按什么顺序改、靠什么门禁防退化。

本文的全部行号、变体清单与构造点统计均来自对 `src/` 的静态核实，文中逐条给出 `文件:行号`。**本文不记录执行过程，只记录代码事实与由此推出的处置。**

为便于与同批文档交叉引用，"目标设计"与"详细设计"两章沿用 `5.x` / `6.x` 的小节编号，其余章节按本目录统一体例。

### 为什么必须先于 SSA 化

`ir::Type` 并不是第三套独立类型，而是 `ast::Type` 的 `pub use` 别名（`ir.rs:3`）。[04-ssa.md](04-ssa.md) 要给 IR 引入寄存器类型标注与新指令，若不在此之前把表示收敛，SSA 的类型标注会顺着 `ir::Type` 别名直接长在 26 变体的语法类型上，等于坐实第三套表示。

当前 `ir::Type` 的能力边界由两个变体划死：

- `ast::Type::ConstExpr(Box<Expr>)`（`ast.rs:506`）把一个完整表达式树嵌进类型里。IR 指令的类型标注里出现 `Expr` 是不可能的。
- `ast::Type::Literal { name, base_type }`（`ast.rs:476-482`）携带源码名与 `Span`，是编译期字面量类型的语法形态，不是运行期类型。

在 26 变体之上做 SSA 类型标注，等于把这 13 个死变体和两个语法专属变体永久固化进 IR 契约。

## 现状

以下全部为代码事实陈述，不含提案。每条给出 `文件:行号` 证据。

### 2.1 三套并行表示与有损桥接

类型表示的多重性是四个独立缺陷的共同上游：

| 下游症状 | 与类型表示的因果关系 |
| --- | --- |
| 同一段类型文本有多种合法 AST 形态 | 因为 `Type::Int(64)`（`src/frontend/core/parser/ast.rs:432`）与 `Type::Name { name: "Int" }`（`ast.rs:428-431`）**语义等价、类型不同**，parser 选了后者，typecheck 必须补一张表把前者翻译回来 |
| 字节码层签名与 IR 层签名不一致 | 因为 `ir::Type` 是别名而非独立类型，`BytecodeFunction`（`src/middle/core/bytecode.rs:808`）只能拿到别名，于是 `params` 声明为 `Vec<ir::Type>`（`bytecode.rs:812`）而 `FunctionCode`（`src/middle/passes/codegen/bytecode.rs:275`）的 `params` 声明为 `Vec<MonoType>`（`codegen/bytecode.rs:277`） |
| 补一个内置类型名要改多处且无检查 | 因为名字→类型的映射散落在 `from_builtin_name`（`src/frontend/core/types/mono.rs:618-643`）与 `bytecode.rs:2371-2376` 的字符串 match 中，两处各写一遍且互不校验 |
| parser 里硬编码 `"Terminates"` | 因为 parser 拿不到"这是不是一个类型"或"这是一个运算符接口"的判定，于是只能把类型层的知识以字符串字面量内嵌进语法层（`src/frontend/core/parser/statements/declarations.rs:496`） |

**前三条是表示问题，第四条是表示问题的一个下游副作用。** 只做第 1 条（删死变体）会留下同义词表和反向依赖；只做第 4 条（给 parser 注入回调）会建立在仍然不一致的表示之上。

三套表示中有一套是别名幻觉：

| 名称 | 定义位置 | 行数 | 角色 |
| --- | --- | --- | --- |
| `ast::Type` | `src/frontend/core/parser/ast.rs:427-541` | 26 变体 | 语法层类型 |
| `MonoType` | `src/frontend/core/types/mono.rs` | 1077 行 | 语义层类型（单态化后） |
| `ir::Type` | `src/middle/core/ir.rs:3` | — | **`pub use crate::frontend::core::parser::ast::Type;`** |

`ir.rs:3` 是一行 `pub use`，`ir.rs:6` 随即 `use crate::frontend::core::typecheck::MonoType;`。IR 模块同时依赖两个"类型"符号，但其中一个只是别名。

序列化侧还有第四处类型载体：

| 载体 | 元素类型 | 位置 |
| --- | --- | --- |
| `BytecodeModule.type_table` | `Vec<ir::Type>` | `src/middle/core/bytecode.rs:854` |
| `FunctionCode.type_table` | `Vec<MonoType>` | `src/middle/passes/codegen/bytecode.rs:32` |
| 解释器模块 `type_table` | `Vec<ir::Type>` | `src/backends/interpreter/image.rs:44` |

**有损桥接。** `From<MonoType> for IrType`（`src/middle/core/bytecode.rs:2353-2390`）把 `MonoType` 降级为 `ast::Type`，其**信息损失是静默的**：

```rust
// bytecode.rs:2371-2376
MonoType::Generic { name, args } => match name.as_str() {
    "String" => IrType::String,
    "Bytes" => IrType::Bytes,
    "Tuple" => IrType::Tuple(...),
    _ => IrType::Void,          // ← 任何其他泛型类型静默变成 Void
},
// bytecode.rs:2378-2385
MonoType::Struct(_) | MonoType::Enum(_) | MonoType::Ref { .. }
    | MonoType::TypeVar(_) | MonoType::TypeRef(_) | MonoType::Union(_)
    | MonoType::Intersection(_) | MonoType::AssocType { .. } => IrType::Void,
```

8 个 `MonoType` 变体全部塌缩为 `IrType::Void`，加上一条 `_ => IrType::Void` 兜底（`bytecode.rs:2386-2387`，注释自认是"删除枚举变体前的过渡分支"）。

`IrType::Void` 在这里是**合法的类型值而非错误信号**。因此下游无法区分"这个函数确实返回 `Void`"和"这个函数的返回类型在桥接时丢失了"——这是 07 的第一层校验器"类型一致"不变量在当前代码上无法真正跑绿的原因之一。

同时 `IrType::String` / `IrType::Bytes` 指向的 `ast::Type::String`（`ast.rs:435`）与 `ast::Type::Bytes`（`ast.rs:436`）正是 2.2 要删的死变体——**有损桥接的两端都指向死代码**。

### 2.2 生产零构造变体

**判定口径**：构造点扫描范围为 `src/` 全量，排除 `tests/` 与 `*/tests/*`。`ast::Type` 的 26 个变体中，以下 13 个在生产代码中**只有 match 臂、没有构造点**：

| 变体 | 定义 | 全部生产命中 |
| --- | --- | --- |
| `Int(usize)` | `ast.rs:432` | `mono.rs:699`（臂） |
| `Float(usize)` | `ast.rs:433` | `mono.rs:700`（臂） |
| `Char` | `ast.rs:434` | `mono.rs:701`（臂） |
| `String` | `ast.rs:435` | `mono.rs:702`（臂） |
| `Bytes` | `ast.rs:436` | `mono.rs:703`（臂） |
| `Bool` | `ast.rs:437` | `mono.rs:707`（臂） |
| `Void` | `ast.rs:438` | `mono.rs:708`（臂）、`types.rs:836`（**唯一构造点**） |
| `Enum(Vec<String>)` | `ast.rs:449` | `mono.rs:747`（臂） |
| `Union(Vec<(String, Option<Type>)>)` | `ast.rs:448` | `mono.rs:743`（臂） |
| `Option(Box<Type>)` | `ast.rs:455` | `mono.rs:770`（臂） |
| `Result(Box<Type>, Box<Type>)` | `ast.rs:456` | `mono.rs:771`（臂） |
| `Sum(Vec<Type>)` | `ast.rs:472` | `mono.rs:814`（臂） |
| `AssocType { .. }` | `ast.rs:463-471` | `mono.rs:781-786`（臂）、`formatter/handlers/types.rs:84`（臂）、`semantic_tokens.rs:228`（臂）——**当前代码中未找到生产构造点** |

根因链条可完整复核：

1. **parser 把所有原始类型都表示成 `Type::Name { name }`**——`src/frontend/core/parser/statements/types.rs:162-165` 的回退臂直接返回 `Some(Type::Name { name, span: name_span })`。
2. **`Result` / `Option` 不 lower 成专用节点**——`types.rs:392-396` 注释明写：「RFC-010: Result/Option 不 lower 成专用 AST 节点……与用户自定义泛型类型走同一 `Type::Generic` 路径。类型表示仍为 `Generic{"Result"/"Option", args}`」。
3. **`Type::Void` 唯一构造点是错误兜底**——`types.rs:836` 的 `_ => (Vec::new(), Type::Void)`，位于一个返回 `(Vec<Param>, Type)` 的 match 末尾。

`NamedStruct`（`ast.rs:443-447`）与 `Literal`（`ast.rs:476-482`）、`MetaType`（`ast.rs:497-504`）有生产构造点：`types.rs:364`、`types.rs:571`、`types.rs:100` 与 `declarations.rs:573`，不属于死变体。

**这 13 个变体在测试代码中有构造点**（`src/frontend/core/types/tests/mono.rs:27-40`、`97`、`119-145`、`221`；`src/frontend/core/typecheck/inference/tests/statements.rs:81`、`366`、`525`、`567`、`591`、`620`）。因此本文的表述是"**生产零构造**"而非"全局零引用"。这些测试本身就是被测行为的固化——它们断言的正是"这些语法形态能被 lower 成什么 MonoType"，而这些形态 parser 永不产出。删除变体时这些测试须一并删除或改写。

#### 口径边界：反向桥会重建其中 7 个

上表的"唯一构造点 / 无构造点"只在 **parser → AST 的前向构造**方向成立。反方向存在两处生产级 `MonoType → ast::Type` 重建，它们**实际构造出上表 7 个变体**：

| 反向桥 | 位置 | 重建的 13 变体成员 |
| --- | --- | --- |
| `From<MonoType> for IrType` | `bytecode.rs:2353-2390` | `Int`（`2357`）、`Float`（`2358`）、`Bool`（`2359`）、`Char`（`2360`）、`Void`（`2361`）、`String`（`2372`）、`Bytes`（`2373`） |
| `mono_to_ast_type`（`substitute_type_in_ast` 内的嵌套 fn） | `middle/passes/mono/function.rs:507-529` | `Int`（`512`）、`Float`（`513`）、`Bool`（`514`）、`Char`（`515`）、`Void`（`516`）、`String`（`522`）、`Bytes`（`523`） |

```
// middle/passes/mono/function.rs:511-527
match mono {
    MonoType::Int(n) => AstType::Int(*n),
    MonoType::Float(n) => AstType::Float(*n),
    MonoType::Bool => AstType::Bool,
    MonoType::Char => AstType::Char,
    MonoType::Void => AstType::Void,
    MonoType::TypeRef(name) => AstType::Name { name: name.clone(), span },
    // #299：String/Bytes 等容器类型现为 Generic，经 type_name 反解回 AstType
    MonoType::Generic { name, .. } if name == "String" => AstType::String,
    MonoType::Generic { name, .. } if name == "Bytes" => AstType::Bytes,
    _ => AstType::Name { name: mono.type_name(), span },
}
```

**这个口径边界对处置有直接影响**：

- `bytecode.rs` 那一处随 6.3 的 #28（删除 `From<MonoType> for IrType` 整个 impl）一并消失，不构成额外成本。
- `function.rs:507-529` **不在任何一条现有改动项的覆盖范围内**。它是一个 `MonoType → AstType` 的类型名反解器（注释标注 `#299`），把 `String` / `Bytes` 从 `Generic` 按名字反解回专用变体。删掉 `ast::Type::String` / `Bytes` 后这两条 guard 分支必须改写；删掉 `Int` / `Float` / `Bool` / `Char` / `Void` 后 `_` 臂的行为会变（原本落进 `_` 的部分类型会静默改走 `AstType::Name`）。这是 6.3 补充的第 47 项。
- **对门禁的直接后果**：一个只扫前向构造点的 T1 会把上述 7 个变体判为"有构造点"，从而把它们从可删除名单中排除。T1 必须显式地把反向桥纳入统计口径，否则门禁的结论与本文的处置表矛盾（见 5.7）。

### 2.3 手写同义词表

`MonoType::from_builtin_name`（`src/frontend/core/types/mono.rs:618-643`）用一张字符串表把类型名映射到 `MonoType`：

```
"Int" | "int" | "Int64" | "int64" | "i64"  => Some(MonoType::Int(64)),   // :620
"DateTime" | "datetime"                     => Some(MonoType::Int(64)),   // :627
"Int32" | "int32" | "i32"                   => Some(MonoType::Int(32)),   // :628
...
"Void" | "void" | "()"                      => Some(MonoType::Void),      // :640
```

**这张表存在的唯一理由是弥合 AST 层的不一致**：因为 `Type::Int(64)` 与 `Type::Name { name: "Int" }` 在 AST 层语义等价却类型不同（且 parser 只产出后者），`from_builtin_name` 才必须同时接受大小写、缩写、带符号名与 `DateTime` 别名。`"()"` 出现在 `Void` 行尤其说明问题——它在配合 `types.rs:836` 的 `Type::Void` 兜底。

同一份"哪些名字是内置类型名"的知识在仓库里共有四处副本，彼此无任何一致性校验：

| # | 位置 | 形态 |
| --- | --- | --- |
| 1 | `mono.rs:618-643` `from_builtin_name` | 字符串 match，名字 → `MonoType` |
| 2 | `bytecode.rs:2371-2376` | 字符串 match，抄了 `"String"` / `"Bytes"` / `"Tuple"` 三个 |
| 3 | `ast.rs:838-843` `CONST_PARAM_TYPES` | 15 个 const 泛型参数名（`"Int"` / `"Bool"` / `"Float"` / `"I8"`…`"F64"` / `"Char"` / `"String"`）的 `&[&str]` 常量 |
| 4 | `src/lsp/world.rs:176` | LSP 侧的内置类型名清单 |

第 3 项 `CONST_PARAM_TYPES` 被 `ast.rs:912` 的 `extract_generic_param_names` 用来判定 const 泛型参数——它枚举的是"可作 const 参数的类型名"，与 `from_builtin_name` 的全集是**部分重叠但不同一**的关系，且无任何机制保证两者同步。第 4 项是 LSP 语法高亮/悬停的第四份副本，同样无校验（T2 的比对范围纳入它）。

### 2.4 parser 对类型层的反向依赖

RFC-039 路由表 C 记为 3 条（`declarations.rs:27-80` 调用 `is_type_param_annotation` / `name_used_as_type`，这两个函数属于类型层）。**实测与该表述有出入**：这两个函数定义在 parser 自己内部，不在类型层。

- `is_type_param_annotation` 定义于 `src/frontend/core/parser/statements/declarations.rs:27-33`（全仓唯一 `fn` 定义）
- `name_used_as_type` 定义于 `declarations.rs:42-80`（全仓唯一 `fn` 定义）
- `declare_predicate` / `is_predicate_name` 定义于 `src/frontend/core/parser/parser_state.rs:46` / `54`，也在 parser 内

真正跨层引用 `typecheck` 的生产代码共 **2 处**，且调的是同一个函数 `crate::frontend::core::typecheck::operator_interfaces::spec`：

| # | 位置 | 所在函数 | 作用 |
| --- | --- | --- | --- |
| 1 | `src/frontend/core/parser/statements/declarations.rs:509` | 签名参数过滤闭包 | 判定约束位形参（`T: Add`），不占运行时参数 |
| 2 | `src/frontend/core/parser/ast.rs:930` | `extract_generic_param_names` | 判定约束位形参，产出带 `constraints` 的 `GenericParamName` |

第 2 处比第 1 处更靠内层——**`parser/ast.rs` 自身**（`ast.rs:847-948` 这一整块与 `StmtKind`/`Expr` 同文件）直接引用了 typecheck 符号。路由表 C 只记了 `declarations.rs` 一处。

它与 `declarations.rs:498-499` 的硬编码共同构成同一个判断：

```rust
// declarations.rs:498-499
// 随后续内置谓词增加而扩展（未来的开集方案：把谓词名下传给
// parser，或改成语法层可判定的形态）。
// parser 不持有类型环境，故看两处：内置谓词硬编码，
// 外加本遍已解析的声明累积（`declare_predicate`）。
let is_predicate_app =
    |n: &str| n == "Terminates" || state.is_predicate_name(n);
```

注释自认这是"未来的开集方案"的临时形态。`n == "Terminates"` 是**把类型层的知识以字符串字面量内嵌在语法层**。

**为什么这是类型表示问题的下游**：parser 之所以需要问"这是不是一个运算符接口名"，是因为它必须区分 `Type::Name` 到底指一个类型还是指一个接口约束。**`Type::Name { name: String }` 用一个无差别的字符串承载了至少 4 种语义**（具体类型 / 类型变量 / 约束名 / 谓词名），语法层无法自行判定，判定权被迫上移到类型层。

#### 重复实现：同名同义、行为已分叉

`name_used_as_type` 有两份实现。`ast.rs:843-846` 的注释自认这一点：

```rust
/// 参数名是否在给定类型中被当作类型引用（`(N: Int) -> (n: N)` 的 `N`）。
///
/// 与 `declarations.rs` 的 `name_used_as_type` 同义，此处独立实现以避免
/// parser 内部跨模块依赖。
pub fn name_used_as_type_in(   // ast.rs:847
```

两份实现**已经分叉**：

| 维度 | `declarations.rs:42-80` `name_used_as_type` | `ast.rs:847-885` `name_used_as_type_in` |
| --- | --- | --- |
| 签名 | 带 `is_predicate: &dyn Fn(&str) -> bool` 回调 | 无回调 |
| `Type::Generic` 分支 | 先 `is_predicate(app_name)` 短路返回 `false`（`:54-56`） | 无此短路，直接递归 `args` |
| `Type::Struct { body }` | **无**（落 `_ => false`） | **有**（`:871-879`，遍历 `Field` / `Expr`） |
| `Type::NamedStruct` | **无** | **有**（`:880-882`，遍历 `fields`） |

`ast.rs:868-870` 的注释记录了后两个分支的来历：「此前缺这两个分支，const 泛型参数在定义体内被引用时判不出『被当类型用』，注解校验会把合法 const 参数误报未知名」。

即：**为了避开跨模块依赖而复制的函数，其修复没有回流到原件。** `ast.rs` 版本有定义体内的字段递归，`declarations.rs` 版本没有——同一段源码在两个判定点上得到不同结论。这是 5.3 引入 `NameKind` 时必须一并合并的重复。

### 2.5 平行运算符枚举

运算符枚举有两份，互不引用且**成员已经分叉**：

| 概念 | 语法层 | 常量求值层 | 差异 |
| --- | --- | --- | --- |
| 二元运算符 | `ast::BinOp`（`ast.rs:191-213`） | `const_data::BinOp`（`src/frontend/core/types/const_data.rs:234-258`） | 语法层独有 `Neq`（`:197`）、`Range`（`:205`）、`Assign`（`:206`）；常量层用 `Ne`（`:243`）而非 `Neq` |
| 一元运算符 | `ast::UnOp`（`ast.rs:217-223`） | `const_data::UnOp`（`const_data.rs:321-330`） | 语法层独有 `Deref`（`:222`）；常量层独有 `BitNot`（`:329`） |

两份还各自实现了完整的 `Display`（`const_data.rs:291-317`）与分类谓词（`const_data.rs:260-289` 的 `is_arithmetic` / `is_comparison` / `is_logical` / `is_bitwise`）。

**为什么这是类型表示问题的一部分**：`ast::BinOp::Assign`（`ast.rs:206`）与 `ast::UnOp::Deref`（`ast.rs:222`）**不是类型**——它们出现在类型位置的表达式里时会落入 `Type::ConstExpr`（`ast.rs:506`），即"仅编译期变体"。这与 6.1 的 IR 层类型纪律直接相关：收敛后 `Type::ConstExpr` 在 `Type → MonoType` 后即消失，IR 永不可见，而 `Assign` / `Deref` 会经 `ir_gen.rs` 的 `BinOp::Assign => Ok(MonoType::Void)`（`inference/expressions.rs:875`）一类路径继续存在。两套枚举的分叉使得"哪些运算符能在类型位置出现"无法从任一侧单独判定。

本阶段不合并这两套枚举（其归属见 [05-frontend-paradigm.md](05-frontend-paradigm.md) 的"改运算符改动面"），但 T1 门禁的扫描对象应包含 `ast::BinOp` / `ast::UnOp`，使"新增零构造运算符"同样可见。

### 2.6 `Expr::FnDef`：生产零构造的表达式变体

`Expr::FnDef`（`ast.rs:37-43`）**生产零构造**属实——全仓仅两处构造：`src/frontend/core/parser/tests/ast.rs:837` 与 `src/frontend/core/typecheck/tests/checker.rs:111`，均为测试。

但**消费点远多于 4 处**。实测生产消费点 12 处：

| 文件 | 行 |
| --- | --- |
| `src/frontend/core/spawn/placement.rs` | `122` |
| `src/frontend/core/spawn/analysis.rs` | `912` |
| `src/formatter/handlers/expr.rs` | `43` |
| `src/frontend/module/orchestrator.rs` | `1492` |
| `src/middle/core/ir_gen.rs` | `4325`、`5097` |
| `src/frontend/core/typecheck/checker.rs` | `1642` |
| `src/frontend/core/typecheck/checker/semantic_tokens.rs` | `1416` |
| `src/frontend/core/typecheck/inference/expressions.rs` | `3374` |
| `src/frontend/core/typecheck/inference/existential.rs` | `55` |
| `src/frontend/core/typecheck/passes/dead_code.rs` | `305` |
| `src/frontend/core/typecheck/layers/ownership.rs` | `1207` |
| `src/frontend/core/typecheck/layers/termination.rs` | `752` |

另有 2 处穷举 match 臂被迫为它存在：`ast.rs:1026`（`Expr::span()`）与 `pratt/mod.rs:47`（`expr_end_line`）。

**函数定义实际走的路**是：`nud.rs:425-429` 构造 `Expr::Lambda` + `declarations.rs:462-479` 构造 `StmtKind::Assign`（把 `Lambda` 放进 `value`）。也就是说 `FnDef` 分支是**一条永不执行、但被 14 个位置维护着的并行路径**。

### 2.7 语法节点替类型层保存语义

**其一，`StmtKind::Assign` 携带 typechecker 专用字段。** `ast.rs:241-249` 的 `Assign` 变体含 `signature_params: Vec<Param>`（`ast.rs:244-245`），注释直言：

> `/// 签名第一组参数原样（含参数名），供 typechecker classify_generic_params 使用`

语句 AST 为类型检查器保留了一个专用槽位。构造点：`declarations.rs:472`。

**其二，表达式位置被当成参数列表。** `(a: Int, b: Int)` 在 `src/frontend/core/parser/pratt/nud.rs:505-514` 被解析为 **`Expr::Lambda { params, body: Box::new(Block { stmts: Vec::new(), .. }) }`**——一个**空 body 的 Lambda**。随后由 `src/frontend/core/parser/pratt/led.rs:466` 的 `Expr::Lambda { params, .. } => Some(params.clone())` 臂把它"捞回来"当作参数列表。

即：AST 的一个正常变体被用作**临时的参数列表携带者**，再由另一个模块的 match 臂还原语义。

**其三，`Type::NamedParen` 承载返回位 binder 名。** `ast.rs:519-540` 的注释明写：

> RFC-027 §3 的返回位精化靠它声明**返回形式参数名**（这里叫 binder）……**丢掉它，类型检查器只能猜**「约束里不在作用域的自由变量即返回值形参」，于是 `(r: P(m))` 会把未声明的 `m` 也当成 binder 静默代入（实测缺陷）。

语法节点在替类型层保存 binder 身份。

## 目标设计

### 5.1 目标表示的形态

**主张：单一表示 = `Type`（原 `ast::Type`）作为唯一的类型结构；`MonoType` 降级为类型检查器内部的工作表示（working form）；`ir::Type` 这个别名删除。**

三套表示收敛为**两套角色**，但只有一套是"类型"：

```
唯一类型结构  Type  ─────────────────────────────┐
   (26 → 13 变体, 住在 types/ 而非 parser/ast.rs)  │
                                                  │  From<Type> for MonoType  (total, 无语义损失)
                                                  │  ← 唯一转换点 types/lower.rs
类型检查工作表示  MonoType  ──────────────────────┘
   (带 TypeVar / 替换态, 只在 typecheck 内部流转)
   ×
ir::Type 别名删除, BytecodeFunction.params / type_table 改用 MonoType
```

**为什么不是"合并为 `MonoType`"**（这是最容易被提出的方案，此处否决）：

1. `Type` 携带 `Span`（`ast.rs:430`、`458`、`464`、`478`、`499`、`490`、`533-540` 等），`MonoType` 不带。07 的 C3 判据是"诊断码与消息相同"——诊断的位置信息必须能回溯到源码。把 `Span` 加进 `MonoType` 会让单态化后的类型带上一堆无意义的哨兵 span。
2. `MonoType` 含 `TypeVar` 与替换态（`types/substitute.rs:112`、`types/solver.rs:411` 的 `pub fn unify`）。这些是**类型检查器求解过程的中间态**，不是类型本身。把它们放进唯一的类型结构，等于让 IR 构造层需要理解类型变量。
3. `ast::Type::ConstExpr(Box<Expr>)`（`ast.rs:506`）与 `ast::Type::Literal`（`ast.rs:476-482`）是**编译期**概念（RFC-027），在 `MonoType` 中被 lower 掉。统一到 `MonoType` 意味着要给它加回这些语法形态。
4. **最关键的反向论证**：把 `MonoType` 提升为唯一表示，等于让 L2 语法层依赖 L3 语义层的类型。RFC-039 明确 L2 不得依赖 L3。本文的目标之一就是消除反向依赖，选一个会造成反向依赖的方案是自相矛盾的。

**为什么 `ir::Type` 必须直接删而不是"保留"**：

`ir.rs:3` 的别名使得 `BytecodeFunction.params`（`bytecode.rs:812`）与 `return_type`（`bytecode.rs:814`）使用 `ir::Type`，而序列化侧 `codegen/bytecode.rs:277-278` 使用 `MonoType`。两套字段类型之间靠 `bytecode.rs:2339` 的 `file.type_table.into_iter().map(|t| t.into())` 与 `bytecode.rs:2353-2390` 的有损 `From` 桥连接。

**`From` 桥存在的唯一理由是"两侧字段类型不同"**。把 `bytecode.rs:812`、`bytecode.rs:814`、`bytecode.rs:854`、`image.rs:44` 统一改为 `MonoType`，`From<MonoType> for IrType` 就失去存在理由，可整体删除（连带消除 2.2 记录的反向重建）。

**明确不做的事**：不把 `Type` 从 `parser/ast.rs` 搬到 `sema/types/`。本阶段只在 `types/` 下新增 `lower.rs` 作为唯一转换点，`Type` 的定义位置不动；目录改名按 D1 在 P6 收尾作为独立搬移批次执行——届时 `Type` 的家是顶层 `ast/type_.rs`（AST 顶层域），`sema/types/` 是 `MonoType` 工作表示的家。

### 5.2 26 变体逐个处置表

> 处置分类：**删除**（生产零构造）、**保留**（生产有构造点）、**迁移**（语义归位到别的节点）。

| # | 变体 | 定义行 | 处置 | 理由 |
| --- | --- | --- | --- | --- |
| 1 | `Name { name, span }` | `ast.rs:428-431` | **保留**（加强） | parser 的**唯一**原始类型出口（`types.rs:162-165`）。删除 12 个原始类型变体后，所有原始类型都走它。需新增"名字的语义种类"标注（见 5.3） |
| 2 | `Int(usize)` | `ast.rs:432` | **删除** | 生产零构造（`mono.rs:699` 为唯一臂）。位宽由 `from_builtin_name` 的 `Name` 路径承载 |
| 3 | `Float(usize)` | `ast.rs:433` | **删除** | 生产零构造（`mono.rs:700`） |
| 4 | `Char` | `ast.rs:434` | **删除** | 生产零构造（`mono.rs:701`） |
| 5 | `String` | `ast.rs:435` | **删除** | 生产零构造（`mono.rs:702`）。**注意**：`bytecode.rs:2372` 的 `IrType::String` 消费它，删除后该臂必须改写 |
| 6 | `Bytes` | `ast.rs:436` | **删除** | 生产零构造（`mono.rs:703-706`）。同上有 `bytecode.rs:2373` 消费 |
| 7 | `Bool` | `ast.rs:437` | **删除** | 生产零构造（`mono.rs:707`） |
| 8 | `Void` | `ast.rs:438` | **删除** | 生产唯一构造是错误兜底 `types.rs:836`。该兜底随 5.6 的错误路径改造一并消除 |
| 9 | `Struct { body }` | `ast.rs:439-442` | **保留** | 有生产消费（`mono.rs:709`、`checker.rs:3167` 等） |
| 10 | `NamedStruct { name, name_span, fields }` | `ast.rs:443-447` | **保留** | 有生产构造点 `types.rs:364` |
| 11 | `Union(Vec<(String, Option<Type>)>)` | `ast.rs:448` | **删除** | 生产零构造（`mono.rs:743-746`） |
| 12 | `Enum(Vec<String>)` | `ast.rs:449` | **删除** | 生产零构造（`mono.rs:747`）。枚举走 `Struct` + `TypeBodyItem`，与 `mono.rs:2378` 的 `MonoType::Enum(_) => IrType::Void` 无关 |
| 13 | `Tuple(Vec<Type>)` | `ast.rs:450` | **保留** | `bytecode.rs:2374` 消费 |
| 14 | `Fn { params, return_type }` | `ast.rs:451-454` | **保留** | `bytecode.rs:2366` 消费 |
| 15 | `Option(Box<Type>)` | `ast.rs:455` | **删除** | 生产零构造（`mono.rs:770`）。`types.rs:392-396` 已说明走 `Generic` 路径 |
| 16 | `Result(Box<Type>, Box<Type>)` | `ast.rs:456` | **删除** | 生产零构造（`mono.rs:771`）。同上 |
| 17 | `Generic { name, name_span, args }` | `ast.rs:457-461` | **保留** | `Result` / `Option` / `String` / `Bytes` 的实际表示都走它（`types.rs:397-399`） |
| 18 | `AssocType { host_type, assoc_name, assoc_name_span, assoc_args }` | `ast.rs:463-471` | **删除** | 生产零构造（全仓唯一构造在测试 `types/tests/mono.rs:178`；语法层没有产出它的 `::` 路径）。未来若启用关联类型语法，由届时的新提案重新定义（决议 D8） |
| 19 | `Sum(Vec<Type>)` | `ast.rs:472` | **删除** | 生产零构造（`mono.rs:814`） |
| 20 | `Literal { name, name_span, base_type }` | `ast.rs:476-482` | **保留** | 有生产构造点 `types.rs:571`；RFC-027 const generics |
| 21 | `Ptr(Box<Type>)` | `ast.rs:485` | **保留** | unsafe 块内的裸指针类型 |
| 22 | `Ref { mutable, inner, span }` | `ast.rs:488-492` | **保留** | 借用记号 |
| 23 | `MetaType { name_span, args }` | `ast.rs:497-504` | **保留** | 有生产构造点 `types.rs:100`、`declarations.rs:573`；RFC-010 |
| 24 | `ConstExpr(Box<Expr>)` | `ast.rs:506` | **保留（并标记为仅编译期）** | 有生产构造点 `types.rs:160`。是 `Type` 中唯一嵌入 `Expr` 的变体，IR 层禁止携带（见 6.1） |
| 25 | `Paren(Box<Type>)` | `ast.rs:518` | **保留** | RFC-004 柯里化的层终止符，`split_curry` 依赖其存在性 |
| 26 | `NamedParen { param, param_span, inner }` | `ast.rs:533-540` | **迁移** | 语义（binder 名）归位到类型检查器，AST 只保留语法。详见 5.6 |

**净效果：26 → 14 变体**（删 12、迁 1、保留 13）。

> **删变体前必须先处置反向桥**：2.2 记录的 `function.rs:507-529` 会重建本表第 2-8 项中的 7 个。删除顺序上它必须与 #1-#8 同批处理，否则该函数的 `_` 臂会静默改变行为（见 6.3 的 #47）。

> **`AssocType` 的处理方式**：按 D8 直接删除（5.2 表第 18 项）。T1 门禁（5.7）在未改动代码上的首次报告**必须独立复现这一结论**——若报告显示 `AssocType` 存在生产构造点，说明本节核实有误，**正确动作是回到 RFC 决议表改 D8 并说明原因**，不是把变体留在白名单里。

### 5.3 `NameKind` 与同义词表消除

**问题**：`Type::Name { name: String }` 用一个无差别的字符串承载了至少 4 种语义——具体类型 / 类型变量 / 约束名（运算符接口）/ 谓词名。`declarations.rs:498-499` 需要判定"是不是谓词"、`declarations.rs:508-511` 需要判定"是不是运算符接口"、`ast.rs:930` 需要判定"是不是运算符接口"、`declarations.rs:42-80` 与 `ast.rs:847-885` 两份 `name_used_as_type*` 需要判定"是不是类型引用"，全靠字符串比较。

**提案：给 `Type::Name` 增加一个 `kind` 字段，由 parser 一次性定种。**

```rust
// 目标形态（示意；行号为改动后）
Name {
    name: String,
    kind: NameKind,   // 新增
    span: Span,
}

enum NameKind {
    Builtin,      // 内置类型名：Int / Float / Bool / ... → 交给 from_builtin_name
    UserType,     // 用户声明的类型名（含 Generic 的 std.result / std.option）
    TypeVar,      // 类型变量：T / K / V
    Constraint,   // 运算符接口 / 约束名：Add / Ord / ...
    Predicate,    // 编译期谓词：Terminates / Sorted / ...
}
```

**关键设计点：`NameKind` 的判定需要的正是 parser 现在拿不到的信息**（类型环境、已声明的运算符接口表、已声明的谓词表）。因此本字段**不能在 parser 内独立定出**——它必须由注入的环境定出。这直接连到 5.4。

**同义词表的处置**：

`from_builtin_name`（`mono.rs:618-643`）**保留，但语义降级**。改造前它是"弥合 AST 层两种等价表示"的消歧器；改造后 `Type::Int(64)` 已删除，"Int" 只有一条表示路径，它就退化为**唯一的名字→类型解析表**。

保留它的具体理由（不能删）：

- `"DateTime" | "datetime" => Some(MonoType::Int(64))`（`mono.rs:627`）的注释记录了一个真实修复（#338 连带问题：`now()` 的返回值传不进 `format_time`）。这是**别名**语义，与 `Int`/`int` 的大小写别名不是同一回事。
- 它是标准库类型名与语言内置名的**唯一切点**。

但表内的**冗余可以砍**。改造后 parser 只会产出规范大小写（`Int` / `Int32` / `Float` / `Bool` / ...），因此：

| 类别 | 现状（`mono.rs:620-642`） | 目标 |
| --- | --- | --- |
| 大小写别名（`"int"` / `"i64"` / `"int64"`） | 保留 | **删除**——parser 规范化后不会产出 |
| 缩写别名（`"i64"` / `"i32"` / `"f64"`） | 保留 | **移到 lexer 层**（`src/frontend/core/lexer/state.rs` 关键字表登记为 `Int` 的等价写法），`from_builtin_name` 只认规范名 |
| `DateTime` 别名 | 保留（`mono.rs:627`） | **保留**——语义别名，附注释 |
| `"()"` （`mono.rs:640`） | 保留 | **删除**——`Type::Void` 兜底（`types.rs:836`）删除后无处产生 |

**净效果**：`from_builtin_name` 从 12 个 match 臂收敛为约 6 个规范名，语义从"消歧表"变为"解析表"，且**成为类型名的单一事实源**（5.7 门禁校验它）。

**两份 `name_used_as_type*` 的合并**：2.4 记录的两份重复实现必须合并为一份，否则 `NameKind` 提供的定种信息会被两份各自独立的字符串比较逻辑绕开。合并后的实现放在 `parser/ast.rs`（数据源在 `Type` 附近），`declarations.rs` 改为调用它；`is_predicate` 短路改为读 `NameKind::Predicate`，`Struct` / `NamedStruct` 的字段递归按 `ast.rs:871-882` 的版本（较新、修复更全）取。

**`CONST_PARAM_TYPES` 的归属**：`ast.rs:838-841` 的 const 泛型参数名表与 `from_builtin_name` 的全集部分重叠（见 2.3）。`NameKind::Builtin` 引入后，`ast.rs:912` 的判定可改为"`kind == Builtin` 且该名字在 const 参数可接受集合内"，把两份知识显式接上；const 参数的子集定义随 T2 门禁校验（见 5.7）。

> **设计判断**：`.yx` 语料中若存在写 `i64` / `int` 的类型标注，把它们规范化到 `Int` 会改变这些文件的**诊断位置**（不改变诊断码）。按 C3 判据这是允许的（消息与顺序可规范化），但需在阶段 1 记录基线。

### 5.4 谓词/类型名的数据流

**当前数据流**（parser 内部闭环 + 2 处越界）：

```
ast.rs (extract_generic_param_names, :930)  ─┐
declarations.rs:509                          ─┴─→ typecheck::operator_interfaces::spec()   ✗
declarations.rs
  ├─ declare_predicate()      ← parser_state.rs:46（parser 自持，仅本遍已解析的声明）
  ├─ n == "Terminates"        ← 硬编码字面量（declarations.rs:499）
  └─ （上述越界调用）
```

**目标数据流**（注入，不越界）：

```
L1 编排层（入口处一次性构造）
   │  ① 编译内置运算符接口表
   │  ② 编译 std 的 .yx 声明 → 谓词名集合
   ▼
TypeEnvProbe（trait，定义在 L2，src/frontend/core/parser/probe.rs）
   │  fn is_predicate(&self, name: &str) -> bool
   │  fn is_constraint(&self, name: &str) -> bool
   ▼
ParserState（parser_state.rs:46/54 处扩展，改为委派给 probe）
   ▼
declarations.rs:498-511 与 ast.rs:930 改为
   let is_predicate_app = |n: &str| state.probe().is_predicate(n);
   if state.probe().is_constraint(n) { ... }
```

**逐文件改动**：

| 文件 | 改动 |
| --- | --- |
| `src/frontend/core/parser/probe.rs`（新增） | 定义 `trait TypeEnvProbe`，提供 `is_predicate` / `is_constraint` / `is_type_var` / `is_builtin_type` 四个纯查询方法。**trait 定义在 L2，实现由 L3 提供**——这是合法的依赖倒置 |
| `src/frontend/core/parser/parser_state.rs:46-58` | `declare_predicate` / `is_predicate_name` 保留（供本遍累积），新增 `probe: Box<dyn TypeEnvProbe>` 字段与 `probe()` 访问器 |
| `src/frontend/core/parser/statements/declarations.rs:498-499` | 删 `n == "Terminates"`；改为 `state.probe().is_predicate(n) \|\| state.is_predicate_name(n)` |
| `src/frontend/core/parser/statements/declarations.rs:508-511` | 删 `crate::frontend::core::typecheck::operator_interfaces::spec(n)` 调用；改为 `state.probe().is_constraint(n)` |
| `src/frontend/core/parser/ast.rs:930` | 删 `crate::frontend::core::typecheck::operator_interfaces::spec(name)` 调用。`ast.rs` 是 `Type` / `Expr` / `StmtKind` 的定义处，不应持有环境依赖——该函数需改为接收 `&dyn TypeEnvProbe` 参数（见下） |
| 全部 5 个 parser 构造点（`parser.rs` 内的 `Parser::new` / `ParserState::new`） | 接收 `probe: Box<dyn TypeEnvProbe>` 参数。**默认实现 `NullProbe`（全返回 false）** 保证 parser 单元测试不受影响 |

**`extract_generic_param_names` 的签名问题**：`ast.rs:891` 的 `pub fn extract_generic_param_names(params: &[Param]) -> Vec<GenericParamName>` 是不接收任何上下文的自由函数。删除 `ast.rs:930` 的越界调用后它需要 `is_constraint` 判定，因此签名须增加 `probe: &dyn TypeEnvProbe`。调用方（含 `parser/tests/ast.rs`）同步改传 `&NullProbe`。

**`NameKind` 的定种时机**：`types.rs:162-165` 构造 `Type::Name` 时，用 `state.probe()` 定出 `kind`，一次定死，后续所有 match 臂不再做字符串比较。

**这消除了 RFC-039 路由表 C 的全部 3 条**——其中 2 条按 2.4 的口径本来就只涉及 parser 内部函数，真正的跨层引用是 `declarations.rs:509` 与 `ast.rs:930` 两处，均由本节覆盖。

### 5.5 `Expr::FnDef` 合并

**主张：删除 `Expr::FnDef`（`ast.rs:37-43`），函数定义统一走 `Expr::Lambda` + `StmtKind::Assign`。**

**这是已在运行的路径**：`nud.rs:425-429` + `declarations.rs:462-479`（见 2.6）。`FnDef` 分支是永不执行的并行实现。

**改动清单**（12 处消费 + 2 处穷举臂，逐个列出）：

| 位置 | 现有臂 | 目标 |
| --- | --- | --- |
| `spawn/placement.rs:122` | `Expr::FnDef { body, .. } => self.check_block(body)` | 并入既有 `Expr::Lambda` 臂（`body` 同为 `Box<Block>`，**直接删此臂**） |
| `spawn/analysis.rs:912` | `Expr::FnDef { body, .. } => { ... }` | 同上，并入 Lambda 臂 |
| `formatter/handlers/expr.rs:43` | `Expr::FnDef { ... }` | 改为格式化 `Assign` 中的 Lambda；该 arm 删除 |
| `orchestrator.rs:1492` | `if let Expr::FnDef { name, .. }` | 改为从 `StmtKind::Assign { target, .. }` 取名字 |
| `ir_gen.rs:4325` | `ast::Expr::FnDef { span, .. } => *span` | 并入 Lambda 臂（`Lambda` 也带 `span`） |
| `ir_gen.rs:5097` | `\| ast::Expr::FnDef { span, .. }` | 删此 or-pattern |
| `checker.rs:1642` | `if let ...::Expr::FnDef {` | 改判 `Expr::Lambda` |
| `checker/semantic_tokens.rs:1416` | `Expr::FnDef {` | 删此臂 |
| `inference/expressions.rs:3374` | `...::Expr::FnDef {` | 改判 `Expr::Lambda` |
| `inference/existential.rs:55` | `\| Expr::FnDef { span, .. }` | 删此 or-pattern |
| `passes/dead_code.rs:305` | `Expr::FnDef {` | 改判 `Expr::Lambda` |
| `layers/ownership.rs:1207` | `\| Expr::FnDef { span, .. }` | 删此 or-pattern |
| `layers/termination.rs:752` | `Expr::FnDef {` | 改判 `Expr::Lambda`；`termination.rs:2303` 的注释「`fn(): Never` — FnDef.return_type 是裸返回类型」同步更新 |
| `ast.rs:1026` | `\| Expr::FnDef { span, .. }` | 删此 or-pattern |
| `pratt/mod.rs:47` | `Expr::FnDef { span, .. } => *span` | 删此臂 |
| `typecheck/tests/checker.rs:109-111` | 测试构造 `Expr::FnDef` | 改写为 `Expr::Lambda` + `Assign` |
| `parser/tests/ast.rs:837-847` | 测试构造 `Expr::FnDef` | 删此测试 |

**不改变行为**：`Expr::Lambda` 有 `params: Vec<Param>`、`body: Box<Block>`、`span`，与 `FnDef` 前三者同构；`FnDef` 独有的 `name` 字段在 `Expr` 层无处可用（名字在 `StmtKind::Assign.target` 上）。

**风险与缓解**：`checker.rs:1642`、`inference/expressions.rs:3374` 这类**非穷举**的 `if let` 匹配在改造后变成永不匹配——不报错、不告警。缓解：类型表门禁（5.7）增加"零构造变体"检查，`Expr::FnDef` 删除后若有人重新写 `if let Expr::FnDef` 会直接编译失败（变体不存在），**这是编译期保证，无需门禁**。

### 5.6 `signature_params` 与 `NamedParen` 的处置

**`StmtKind::Assign.signature_params`（`ast.rs:244-245`）→ 删除。**

理由：它的存在理由（`ast.rs:244` 注释）是"供 typechecker `classify_generic_params` 使用"——**语句 AST 为类型检查器保留的专用槽位**。但 `declarations.rs:500-511` 的 `extracted_params` 在同一函数内已经算出了 `value_params`（已完成类型位/约束位过滤），把它随语句传下去是重复搬运。

改动：`ast.rs:244-245` 删字段 → `declarations.rs:472` 删实参 → `classify_generic_params` 的数据来源改为就地重算（它拿得到 `value: Option<Box<Expr>>` 里的 `Lambda.params`）。**净删除一行字段声明、一处构造、一条搬运路径。**

> **风险**：`signature_params` 是**未经谓词过滤的原始**列表，而 `Lambda.params` 是**已过滤**的（`declarations.rs:500-511` 的 filter 去掉了类型位与约束位）。`classify_generic_params` 若依赖未过滤版本，改造后行为会变。**必须在阶段 1 核实 `classify_generic_params` 实际使用哪一份**，不能假定等价。

**`Type::NamedParen`（`ast.rs:533-540`）→ 保留节点，移出 binder 语义。**

`ast.rs:519-532` 的注释记录了它存在的**充分理由**（见 2.7）：`(r: P(m))` 中若丢掉 `r`，类型检查器只能猜"约束里不在作用域的自由变量即返回值形参"，会**把未声明的 `m` 也当成 binder 静默代入**（注释标注为"实测缺陷"）。

因此**不能简单删除**——那会重新引入一个已被修复的正确性缺陷。正确处置是：

1. `NamedParen { param, param_span, inner }` 保留为**纯语法节点**，`param`/`param_span` 只作为源码事实携带，不承担类型层 binder 语义。
2. binder 的解释权移交给 `sema/`：类型检查器在消费 `NamedParen` 时**显式**提取 `param` 为 binder。这是"读语法节点的字段"，与 `ConstExpr` 内嵌 `Expr` 同性质——语法节点提供事实，语义层做解释。
3. 注释从"类型检查器只能猜"改写为"本节点提供 binder 名，语义解释在 `sema/` 内"。

**为什么这是"迁移"而不是"删除"**：语法树必须能无损表示源码。`r` 在源码里确实存在，AST 不能假装它不存在。问题不在节点存在，而在**语义解释的责任没被明确定义**。

### 5.7 类型表生成期门禁

**参照对象已在本仓库验证**：`build.rs:19-37` 调用 `tools/code-tables` 的 `parse_registry` / `validate`，对 145 个错误码做唯一性、段位、zh 完备性校验，`is_ok()` 为假时直接 `panic!` 拒绝编译；`build.rs:39-54` 进一步逐条比对 RFC-013 的 markdown 码表，不一致同样 `panic!`。**RFC-013 文档与代码因此始终一致**——`tools/code-tables/Cargo.toml:15-16` 只有一个 `serde_json` 依赖，校验逻辑是文本级解析，不依赖 syn。

类型表目前**没有**任何类似门禁。本文要补的机制形态可直接沿用。

**新增 crate**：`tools/type-tables`（`Cargo.toml` 依赖仅 `serde_json`，与 `code-tables` 对称）。四个检查：

| # | 检查 | 输入 | 失败条件 | 治法 |
| --- | --- | --- | --- | --- |
| **T1 零构造变体** | 扫描 `src/frontend/core/parser/ast.rs` 的 `pub enum Type`（及 `Expr` / `StmtKind` / `BinOp` / `UnOp`）变体清单；在 `src/` 下统计每个变体的**构造点**（排除 `tests/` 与 `*/tests/*`），**并单独标记反向桥中的重建点** | 变体清单 + 命中表 | 某变体**零构造点**且不在显式白名单内 | 删变体，或移入白名单并注明理由 |
| **T2 同义词表与全仓名字清单一致** | `types/mono.rs` 的 `from_builtin_name` match 表 + `ast.rs:838-843` `CONST_PARAM_TYPES` + `src/lsp/world.rs:176` 的 LSP 内置类型名清单 | 三张表 | 某个类型名在一张表中存在而另一张不认（按各自语义的映射规则比对） | `type-tables --fix` 双向补齐 |
| **T3 内置类型名与 RFC-011 文档一致** | `from_builtin_name` 的规范名集合 + `docs/src/design/rfc/accepted/011-generic-type-system.md` 的类型表 | 两张表 | 码表区间/名字与注册表不一致 | `cargo run --manifest-path tools/type-tables/Cargo.toml -- --fix` |
| **T4 `Type → MonoType` 穷尽性** | `Type` 变体清单 vs `types/lower.rs` 的 match 臂 | 两张表 | 有变体无对应臂，或有臂无对应变体 | 补臂 / 删臂 |

**T4 的实现说明**：Rust 的穷尽 match 已在编译期保证"变体 → 臂"的方向。T4 的价值在**反向**——检测 `lower.rs` 里存在**指向已删除变体**的陈旧臂（这类代码在删除变体时就编译失败了，故 T4 实为冗余）。**T4 因此降级为一条 CI 断言（`cargo build` 成功即通过），不进 `build.rs` 的 `panic!` 路径。** T1-T3 才是真正需要文本解析的三项。

**T1 的两个实现难点与缓解**：

1. **构造点 vs match 臂的区分**需要语法分析。仓库根 `Cargo.toml` **无 `syn` 依赖**，`tools/code-tables` 亦只有 `serde_json`。缓解方案：
   - **首选**：给 `tools/type-tables` 引入 `syn`（仅该 crate，根 crate 不动）。`syn` 是标准做法，且门禁 crate 独立于编译器。
   - **退路**：复用已有的 `tools/extract_arm.py`（仓库内既有 Python 工具）的行级启发式——构造点位于 `= ` 右侧或函数实参位，match 臂位于 `=>` 左侧。用 `=>` 出现次数近似判断。
   - **保守起点**：T1 首次上线时**只报告不失败**（`cargo:warning`），取一次全量基线确认启发式无误后再改为 `panic!`。这与"CI 门禁初期会红、需要先建立基线"的处理一致。
2. **反向桥的构造点必须单独归类**。2.2 记录了 `bytecode.rs:2353-2390` 与 `function.rs:507-529` 两处 `MonoType → ast::Type` 重建。若 T1 把它们的重建点与 parser 的前向构造混为一谈，`Int` / `Float` / `Bool` / `Char` / `Void` / `String` / `Bytes` 七个变体会被判为"有构造点"而免于删除，与 5.2 的处置表直接矛盾。T1 的报告须把两类构造点分列，并在白名单机制中要求对反向桥重建单独说明。

**门禁覆盖面**：`build.rs` 中新增 `type_tables::validate(root, &entries)` 调用，位置紧邻 `build.rs:19-37` 现有块之后，沿用 `panic!("类型表校验失败（{} 项 error ...）", ...)` 形态。

## 详细设计

### 6.1 类型系统影响

| 影响面 | 判断 |
| --- | --- |
| 类型推断（`inference/`） | **无影响**。inference 全程消费 `MonoType`，不接触 `ast::Type` 变体 |
| 求解器（`types/solver.rs`） | **无影响**。`pub fn unify`（`solver.rs:411`）、`types/substitute.rs:112` 的 `substitute`、`types/mono.rs:649` 的 `substitute` 均在 `MonoType` 上操作 |
| 常量求值（`types/eval/`） | **无影响**。`eval/const_eval.rs`（1128 行）、`eval/dependent_types.rs`（`check_structural_termination` 在 `478`）、`eval/reducer.rs:572` 的 `unify` 都在 `MonoType` 上 |
| 单态化（`middle/passes/mono/`） | **需改**。`mono/function.rs:358`、`362`、`451`、`456`、`554`、`558`、`618`、`623` 8 处 match `AstType::NamedStruct` / `AstType::AssocType`——`AssocType` 的处置若定为删除，这 8 处需同步。**另需改 `function.rs:507-529` 的 `mono_to_ast_type`**（见 6.3 的 #47） |
| IR 构造（`middle/core/ir_gen.rs`） | **需改**。`ir_gen.rs:1266`、`1399` 的 `ast::Type::NamedStruct` match；`ir_gen.rs:1380` 的 `let params: Vec<MonoType> = signature_params`（这是 `signature_params` 的另一个消费点，5.6 需一并核实） |
| 字节码序列化（`middle/passes/codegen/bytecode.rs`） | **需改**。`codegen/bytecode.rs:492-495` 的 `type_id_to_monotype`、`351-352` 的编码循环、`632` 的组装 |
| 解释器（`backends/interpreter/`） | **需改**。`image.rs:44` 的 `type_table: Vec<ir::Type>` 改 `Vec<MonoType>`；`repl/eval.rs:369-370` 按 `type_table` 下标格式化类型名，元素类型变化会影响 `{:?}` 的输出格式 |
| RFC-027 编译期类型 | **需改**。`Type::ConstExpr` 保留但标记为**仅编译期**，`Type → MonoType`（`types/lower.rs`）后即消失，IR 层永不可见 |

**IR 层的类型纪律（供 04-ssa 继承）**：收敛后 IR 的类型标注只能取 `MonoType` 中的**已解析子集**。`Type::ConstExpr` / `Type::Literal` / `Type::MetaType` / `Type::Paren` / `Type::NamedParen` 是**仅编译期**变体，不得出现在 `BytecodeFunction`、指令操作数或 `type_table` 中。这条纪律应由 07 第一层校验器的"类型一致"不变量强制。`ast::BinOp::Assign` / `ast::UnOp::Deref`（2.5）是同类问题的表达式侧形态——它们可出现在 `ConstExpr` 内，因而同样不得下沉到 IR 的类型标注中。

### 6.2 运行时行为

**本文不改变任何运行时行为。** 依据：

1. 删除的 11 个变体在生产代码中无**前向**构造点 → 无源码可产出它们（反向桥的处置见 6.3 的 #47/#28）。
2. `ir::Type` 是 `ast::Type` 的别名 → 把它改回 `MonoType` 不改变类型集合，只改变**载体**。
3. `From<MonoType> for IrType` 的**损失**（8 变体 → `Void`）在改动后不再发生——这**可能**改变某些程序的行为（原本丢失的类型现在保留了）。**这是唯一的行为变化方向，且是修复而非回归**，但必须由 07 第三层语料差分确认无回归。

**需要重点关注的语义变化点**：

| 位置 | 现状 | 收敛后 |
| --- | --- | --- |
| `bytecode.rs:2371-2376` | 非 `String`/`Bytes`/`Tuple` 的 `Generic` → `IrType::Void` | 保留真实 `MonoType::Generic` |
| `bytecode.rs:2378-2385` | `Struct`/`Enum`/`Ref`/`TypeVar`/`TypeRef`/`Union`/`Intersection`/`AssocType` → `IrType::Void` | 保留真实 `MonoType` |
| `bytecode.rs:2386-2387` | `_ => IrType::Void` 兜底 | **删除**（穷尽 match 即可） |

> **设计判断**：这 8 个变体当前全部塌缩为 `Void` 意味着**依赖签名字节码的代码路径（解释器的 arity/类型检查、序列化格式）目前看到的一律是 `Void`**。收敛后这些位置会看到真实类型。**这是本文唯一可能的行为变化来源，且方向是"恢复正确信息"**。若语料差分出现差异，应视为**暴露既有缺陷**而非本文引入的回归——判定方法：单独构造一个含 `Ref` 参数的 `.yx` 程序，在改动前后分别 `dump_bytecode` 对比。

### 6.3 编译器改动清单

| # | 文件 | 行号 | 改动 | 类别 |
| --- | --- | --- | --- | --- |
| 1 | `src/frontend/core/parser/ast.rs` | `435-438`、`448-449`、`455-456`、`472` | 删除 `String`/`Bytes`/`Bool`/`Void`/`Union`/`Enum`/`Option`/`Result`/`Sum`/`Int`/`Float` 变体 | 纯删除 |
| 2 | `src/frontend/core/parser/ast.rs` | `428-431` | `Name` 增加 `kind: NameKind` 字段 | 数据流 |
| 3 | `src/frontend/core/parser/ast.rs` | `37-43` | 删除 `Expr::FnDef` | 纯删除 |
| 4 | `src/frontend/core/parser/ast.rs` | `244-245` | 删除 `Assign.signature_params` | 纯删除 |
| 5 | `src/frontend/core/parser/ast.rs` | `1026` | 删 `Expr::FnDef` or-pattern | 随 #3 |
| 6 | `src/frontend/core/parser/ast.rs` | `519-532` | `NamedParen` 注释改为"语法提供 binder 名，语义解释在 `sema/`" | 文档 |
| 7 | `src/frontend/core/parser/ast.rs` | `795-814` 后 | 新增 `pub enum NameKind` | 新增 |
| 8 | `src/frontend/core/parser/probe.rs` | 新增 | `trait TypeEnvProbe` + `NullProbe` | 新增 |
| 9 | `src/frontend/core/parser/parser_state.rs` | `46-58` | 增加 `probe` 字段与访问器 | 数据流 |
| 10 | `src/frontend/core/parser/statements/types.rs` | `162-165` | 用 `state.probe()` 定 `NameKind` | 数据流 |
| 11 | `src/frontend/core/parser/statements/types.rs` | `392-396` | 注释更新（`Result`/`Option` 走 `Generic`，已与事实一致，仅补变体删除说明） | 文档 |
| 12 | `src/frontend/core/parser/statements/types.rs` | `836` | 删 `_ => (Vec::new(), Type::Void)` 兜底，改为显式错误返回 | 行为 |
| 13 | `src/frontend/core/parser/statements/declarations.rs` | `498-499` | 删 `n == "Terminates"`，改 `state.probe().is_predicate(n)` | 反向依赖 |
| 14 | `src/frontend/core/parser/statements/declarations.rs` | `508-511` | 删 `typecheck::operator_interfaces::spec()` 调用 | 反向依赖 |
| 15 | `src/frontend/core/parser/statements/declarations.rs` | `472` | 删 `signature_params` 实参 | 随 #4 |
| 16 | `src/frontend/core/parser/pratt/mod.rs` | `47` | 删 `Expr::FnDef` 臂 | 随 #3 |
| 17 | `src/frontend/core/parser/pratt/led.rs` | `466` | 保留（是参数列表捞回点，随 5.5 改为直接读 Lambda） | 随 #3 |
| 18 | `src/frontend/core/types/mono.rs` | `699-708`、`743-747`、`770-771`、`814` | 删 11 个 match 臂 | 随 #1 |
| 19 | `src/frontend/core/types/mono.rs` | `620-642` | 砍同义词（大小写别名、`i64` 类缩写、`"()"`） | 收敛 |
| 20 | `src/frontend/core/types/mono.rs` | `627` | `DateTime` 别名保留 | 保留 |
| 21 | `src/frontend/core/types/lower.rs` | 新增 | 迁移 `From<Type> for MonoType`（现于 `mono.rs:692-830`）为唯一转换点 | 新增 |
| 22 | `src/frontend/core/lexer/state.rs` | 关键字表 | 登记 `i64`/`int` 等为 `Int` 的等价写法 | 收敛 |
| 23 | `src/middle/core/ir.rs` | `3` | 删 `pub use ... ast::Type` | 纯删除 |
| 24 | `src/middle/core/ir.rs` | `678` | `FunctionCode.params` 保持 `Vec<MonoType>`（已是） | — |
| 25 | `src/middle/core/bytecode.rs` | `812`、`814` | `BytecodeFunction.params` / `return_type` 改 `MonoType` | 收敛 |
| 26 | `src/middle/core/bytecode.rs` | `854` | `BytecodeModule.type_table` 改 `Vec<MonoType>` | 收敛 |
| 27 | `src/middle/core/bytecode.rs` | `2339` | `type_table` 转换改 `.collect()`（元素已同型） | 随 #26 |
| 28 | `src/middle/core/bytecode.rs` | `2352-2390` | **删除** `From<MonoType> for IrType` 整个 impl | 纯删除 |
| 29 | `src/backends/interpreter/image.rs` | `44` | `type_table` 改 `Vec<MonoType>` | 收敛 |
| 30 | `src/middle/passes/mono/function.rs` | `358`-`623`（8 处） | `AstType::AssocType` 臂随门禁结论处理 | 待定 |
| 31 | `src/frontend/core/lexer/state.rs` / `tools/type-tables/` / `build.rs` | 新增 / `build.rs:19-37` 后 | 类型表门禁 T1-T3 | 门禁 |
| 32 | `spawn/placement.rs` | `122` | 删 `Expr::FnDef` 臂 | 随 #3 |
| 33 | `spawn/analysis.rs` | `912` | 同上 | 随 #3 |
| 34 | `formatter/handlers/expr.rs` | `43` | 同上 | 随 #3 |
| 35 | `orchestrator.rs` | `1492` | 同上，改从 `Assign.target` 取名 | 随 #3 |
| 36 | `ir_gen.rs` | `4325`、`5097` | 同上 | 随 #3 |
| 37 | `typecheck/checker.rs` | `1642` | 同上 | 随 #3 |
| 38 | `typecheck/checker/semantic_tokens.rs` | `1416` | 同上 | 随 #3 |
| 39 | `typecheck/inference/expressions.rs` | `3374` | 同上 | 随 #3 |
| 40 | `typecheck/inference/existential.rs` | `55` | 同上 | 随 #3 |
| 41 | `typecheck/passes/dead_code.rs` | `305` | 同上 | 随 #3 |
| 42 | `typecheck/layers/ownership.rs` | `1207` | 同上 | 随 #3 |
| 43 | `typecheck/layers/termination.rs` | `752`、`2303` | 同上 + 注释更新 | 随 #3 |
| 44 | `parser/tests/ast.rs` | `837-847` | 删 `Expr::FnDef` 测试 | 测试 |
| 45 | `typecheck/tests/checker.rs` | `109-111` | 改写为 Lambda + Assign | 测试 |
| 46 | `types/tests/mono.rs` | `27-40`、`97`、`119-145`、`221` | 删除 11 个死变体的 lower 断言 | 测试 |
| 47 | `src/middle/passes/mono/function.rs` | `507-529` | `mono_to_ast_type` 删 `AstType::Int`/`Float`/`Bool`/`Char`/`Void`/`String`/`Bytes` 分支，统一走 `AstType::Name` + `mono.type_name()`。**必须与 #1 同批处理** | 收敛 |
| 48 | `src/frontend/core/parser/ast.rs` | `891`、`930` | `extract_generic_param_names` 签名增 `probe: &dyn TypeEnvProbe`；删 `typecheck::operator_interfaces::spec()` 调用改 `probe.is_constraint(name)`。调用方（含 `parser/tests/ast.rs`）传 `&NullProbe` | 反向依赖 |
| 49 | `src/frontend/core/parser/statements/declarations.rs` | `42-80` | 删本文件内的 `name_used_as_type`，改为调用 `ast.rs` 的合并版；`is_predicate` 短路改读 `NameKind::Predicate` | 重复实现 |
| 50 | `src/frontend/core/parser/ast.rs` | `912` | `CONST_PARAM_TYPES` 判定改经 `NameKind::Builtin` 接线（见 5.3） | 收敛 |

> **门禁本身也必须门禁**：`tools/type-tables` 与 `code-tables` 同样需要 `mod` 声明接线（本仓已有 8 棵孤儿测试树的教训）。新建 crate 时同步在根 `Cargo.toml` 登记。

### 6.4 向后兼容性策略

**约束**：`tests/yaoxiang/` 293 个 `.yx` 语料（单文件）+ P2 建立的多文件语料层（`tests/yaoxiang-multifile/`）+ `src/std/tests/*.yx` + 标准库，**必须全部行为不变**（诊断码与消息相同、顺序可规范化）。

| 风险 | 判断 | 缓解 |
| --- | --- | --- |
| 源码里写 `i64` / `int` / `f64` 等类型标注 | 规范化到 `Int` 后 `from_builtin_name` 只认规范名 | **阶段 1 第一件事**：扫全语料统计非规范类型名出现次数。若为 0，可直接删同义词；若非 0，按 5.3 移入 lexer 别名表（**语法层接受、lower 层归一**，对诊断码无影响） |
| 源码里写 `DateTime` | 保留别名（`mono.rs:627`） | 无影响 |
| `("()")` 出现在类型位置 | `types.rs:167` 起 `LParen` 分支处理元组/参数组，产出 `Tuple` 而非 `Name{"()"}` | 删 `"()"` 前先统计全语料 `"()"` 作为类型名的出现次数 |
| `dump_bytecode` 输出变化 | `type_table` 元素类型从 `ir::Type` 改 `MonoType`，`{:?}` 格式不同 | **C3 判据允许**（07 规定 `dump_bytecode` 仅在 C1/C2 阶段比对）。但需在阶段 2 记录输出 diff 供人工 review |
| 有损桥接修复后行为变化 | 见 6.2 | 语料差分 + 定向 `dump_bytecode` 对拍 |
| `mono_to_ast_type` 改写后类型替换结果变化 | `function.rs:507-529` 是单态化类型替换的出口（`substitute_type_in_ast:532-537` 把替换后的 `MonoType` 转回 `AstType`）。删变体后若走 `_` 臂，替换结果从 `AstType::Int(64)` 变为 `AstType::Name { name: "i64" }` 一类 | 阶段 3 定向对拍：选含泛型替换的语料，比对改动前后单态化产物的 `type_name()`。**不得假定 `{:?}` 输出等价** |
| `.yx` 源码本身 | **零修改** | 本文不要求改任何语料或 std 源文件。这条是硬验收项 |

**硬性验收条件**：`git diff --stat tests/ src/std/` 必须为空（除多文件语料层与测试基线文件的新增）。

## 实施要点

判据统一采用 07 的 **C3：诊断码与消息相同（顺序可规范化）**。以下是本文这条线的阶段划分；跨文档的全局先后关系见 RFC-039。

| 阶段 | 内容 | 验收判据 | 回滚点 |
| --- | --- | --- | --- |
| **0. 门禁先行** | 新建 `tools/type-tables`，实现 T1/T2/T3，T1 首版**只报告不失败**。在**未改动**的代码上取全量基线 | T1 报告准确列出 13 个零构造变体（含 `AssocType` 的定性结论），并**分列出 2.2 记录的反向桥重建点**；T2/T3 全绿或给出可修清单 | 无代码改动，无需回滚 |
| **1. 名称规范化** | 统计语料中非规范类型名；`from_builtin_name`（`mono.rs:620-642`）砍同义词，移入 lexer 别名表。**同步核实 `classify_generic_params` 用的是 `signature_params` 还是 `Lambda.params`** | C3：全语料诊断集与阶段 0 基线**逐条相同** | 单独 commit，可 revert |
| **2. 字节码类型统一** | 改 `bytecode.rs:812`/`814`/`854`、`image.rs:44` 为 `MonoType`；删 `From<MonoType> for IrType`（`bytecode.rs:2352-2390`） | C3 + 定向 `dump_bytecode` 对拍（6.2 表格逐项） | 单独 commit，可 revert |
| **3. 死变体删除** | 按 5.2 表删 11 个变体；清 `mono.rs:699-708` 等 match 臂、`types.rs:836` 兜底；**同批处理 `function.rs:507-529`（#47）**；同步 `types/tests/mono.rs` | C3 + `cargo build` 全绿（穷尽 match 强制补全所有臂）+ 泛型替换定向对拍（6.4 末行） | 单独 commit，可 revert |
| **4. parser 数据流** | 新增 `probe.rs`；改 `parser_state.rs:46-58`；改 `declarations.rs:498-511` 与 `ast.rs:930`（#48）；`types.rs:162-165` 定 `NameKind`；合并两份 `name_used_as_type*`（#49） | C3 + **新增断言**：`grep 'typecheck' src/frontend/core/parser/` 应为 0 | 单独 commit，可 revert |
| **5. AST 死变体与专用字段** | 删 `Expr::FnDef`（16 处）+ `Assign.signature_params`；`NamedParen` 语义迁移 | C3 + `tests/integration/` 18 模块全绿 | 分两个 commit（FnDef / signature_params） |
| **6. 门禁转硬** | T1 从 warning 改为 `panic!`；T4 降级为 CI 断言 | `cargo build` 在故意引入一个零构造变体时**必须失败**（红测试先行） | 单独 commit |

**依赖顺序**：0 → 1 → 2 → 3 → 4 → 5 → 6。**阶段 3 与 5 必须在阶段 2 之后**——`bytecode.rs:2372-2373` 消费 `IrType::String` / `IrType::Bytes`，若先删变体会破坏编译。阶段 3 的 #47 必须与 #1 同批，否则 `mono_to_ast_type` 静默改行为。

**对下游的依赖**：本文全部阶段完成后，[04-ssa.md](04-ssa.md) 方可开工。此时 `BytecodeFunction.params`、`FunctionCode.params`、`type_table` 三者元素类型一致，SSA 的类型标注有唯一落点。

### 每阶段的强制验证

| 阶段 | 必须执行 |
| --- | --- |
| 0 | `cargo run --manifest-path tools/type-tables/Cargo.toml -- --report`，人工核对 T1 报告（**含反向桥重建点分列**） |
| 1-5 | `cargo test`；`tests/yaoxiang/` 全量语料差分（07 第三层）；C3 判据逐条比对 |
| 2 | 额外：6.2 表格中每个变体各构造一个 `.yx` 样本，改动前后 `dump_bytecode` 对拍 |
| 3 | 额外：含泛型替换的语料，比对改动前后单态化产物的 `type_name()` |
| 4 | 额外：`rg 'typecheck' src/frontend/core/parser/` 返回 0 |
| 6 | 额外：故意写一个零构造变体，确认 `cargo build` 失败（红→绿） |

## 关键决策与理由

### 决策

| 决策 | 决定 | 理由 |
| --- | --- | --- |
| 目标表示形态 | `Type` 作唯一类型结构 + `MonoType` 作工作表示；`ir::Type` 别名删除 | `ir.rs:3` 的别名是 `BytecodeFunction.params` 与 `FunctionCode.params` 字段类型不一致的直接原因（2.1）。别名删除后两套字段类型统一，`From` 桥失去存在理由 |
| 否决"统一为 `MonoType`" | 不采纳 | 会导致 L2→L3 反向依赖（违反 RFC-039 分层），且需给 `MonoType` 加回 `Span` / `ConstExpr`。完整论证见 5.1 |
| `NamedParen` 不删 | 保留为纯语法节点，binder 语义迁到 `sema/` | `ast.rs:519-532` 记录了删除会重新引入一个已修复的正确性缺陷：`(r: P(m))` 会把未声明的 `m` 当 binder 静默代入 |
| 目录改名 | **做**（D1），随 P5/P6 收尾以纯搬移批次执行；本文正文路径按改名前现状书写 | 2026-10-03 决议（ChenXu233）；2026-10-04 补充施工方式 |

### 被否决的替代方案

四类替代方案均未采纳：**保留三套表示只加转换层**不解决任何一项根因，且 `ir::Type` 仍非独立类型、死变体与同义词表与反向依赖一项都不会消失；**用宏生成同义词表**治的是症状不是病因——`Type::Int(usize)` 删除后宏就没有存在理由了，改由门禁 T2 承担一致性保证（比宏的展开正确性更可验证）；**只删死变体不做统一**成本最低但把问题留在原地，且无门禁防止下一个人加 `Type::Int128(usize)` 时同样的死变体再长出来（若只能做一个 commit，应当明确记为该方案的子集并把 T1 门禁一并交付）；**统一为 `MonoType`** 见 5.1。

### 收益

- **消除 14 个位置的维护负担**。`Expr::FnDef` 的 12 处消费 + 2 处穷举臂是一条永不执行的并行路径，删掉后 `ir_gen.rs` / `ownership.rs` / `termination.rs` 三处都少一个分支。
- **消除一份已分叉的重复实现**。`name_used_as_type` 的两份副本（2.4）在定义体字段递归上已有分歧，合并后只剩一处。
- **有损桥接消失后，`IrType::Void` 不再是"合法类型值"**。这是 07 第一层校验器"类型一致"不变量能真正跑绿的前提。
- **门禁让退化不可能**。T1 使"新增零构造变体"从当前的静默通过变成构建失败——本仓库已有的 `build.rs` 机制证明这条路走得通。
- **零源码改动**。293 个 `.yx` 语料与 std 库不需要任何修改，这是 C3 判据能成立的根本原因。

## 已知局限与风险

### 风险

- **阶段 2 可能有行为变化**。有损桥接修复后，8 个 `MonoType` 变体现在能看到真实类型而非 `Void`。方向是修复，但会改变依赖签名字节码的路径。这是本文最大的不确定性。
- **`AssocType` 的定性依赖门禁准确性**。T1 若用行级启发式（`=>` 计数）而非 `syn`，可能误判。这是阶段 0 必须人工核对的原因。
- **T1 漏算反向桥会使处置表与门禁互相矛盾**。2.2 已证明 7 个"死变体"在反向桥里有生产重建点。若 T1 不分列构造点方向，这 7 个会被判为"有构造点"而免于删除——门禁将成为删除工作的阻力而非保障。
| **2. `signature_params` 删除有语义风险**。它携带**未过滤**的参数列表，而 `Lambda.params` 是**已过滤**的。若 `classify_generic_params` 依赖前者，改造会改变泛型分类结果。**必须先核实，不能假定等价**（ir_gen 侧 `ir_gen.rs:1380` 的 `signature_params` 消费点一并核实）。
| **3. 阶段 3 触及 `middle/passes/mono/function.rs`**。除 `AssocType` 的 8 处 match 外，`507-529` 的 `mono_to_ast_type` 是单态化类型替换的出口，改写后替换结果的 `type_name()` 可能变化（6.4 末行）。
- **`.yx` 语料的非规范类型名可能非零**。若语料大量使用 `i64` / `int`，阶段 1 的"直接删同义词"不可行，退化为"移入 lexer 别名表"，`from_builtin_name` 的收敛幅度减小。
- **门禁初期会红**。T1 首次上线若直接 `panic!` 会立刻阻塞编译。必须先只报告取基线。

### 未决问题

> **本节原列的开放问题已全部裁决。** 逐条决定见 [RFC-039 决议登记](../rfc/draft/039-compiler-architecture.md)（D1–D50）。**本文不留任何待定项。**
>
## 参见

### 上位与同批文档

- [RFC-039 编译器功能路由目录设计（总纲）](../rfc/draft/039-compiler-architecture.md) — 上位总纲；四层模型、判据分级、路由表 C（3 条类型表示相关条目归属本文）
- [01-routing.md](01-routing.md) — 功能路由表 A/B/C、依赖方向规范、目标目录结构
- [04-ssa.md](04-ssa.md) — SSA 化；本文的前置与下游消费者
- [05-frontend-paradigm.md](05-frontend-paradigm.md) — 前端范式；运算符改动面收敛
- [07-equivalence-oracle.md](07-equivalence-oracle.md) — C3 类别定义、诊断集比对规范、第一层 IR 校验器的类型一致不变量

### 其他 RFC

- [RFC-010 统一类型语法](../rfc/accepted/010-unified-type-syntax.md) — `Type::Generic` 统一路径的依据
- [RFC-011 泛型类型系统](../rfc/accepted/011-generic-type-system.md) — 内置类型表（门禁 T3 的比对对象）
- [RFC-013 错误码规范](../rfc/accepted/013-error-code-specification.md) — `build.rs` 生成期门禁的范例
- [RFC-027 编译期求值与类型](../rfc/accepted/027-compile-time-evaluation-types.md) — `Type::NamedParen` 与 `Type::ConstExpr` 的来源

### 代码位置

- `src/frontend/core/parser/ast.rs:427-541` — `ast::Type` 26 变体定义
- `src/frontend/core/parser/ast.rs:519-540` — `Type::NamedParen` 的 binder 语义说明（删除会重引入缺陷的证据）
- `src/frontend/core/parser/ast.rs:241-249` — `StmtKind::Assign.signature_params` 的 typechecker 专用字段
- `src/frontend/core/parser/ast.rs:838-841` — `CONST_PARAM_TYPES`：const 泛型参数名的第三份副本
- `src/frontend/core/parser/ast.rs:843-885` — `name_used_as_type_in`：`declarations.rs:42` 的重复实现（已分叉）
- `src/frontend/core/parser/ast.rs:891`、`930` — `extract_generic_param_names` 与其中的越界调用
- `src/frontend/core/parser/ast.rs:191-213`、`217-223` — `ast::BinOp` / `ast::UnOp`
- `src/frontend/core/parser/statements/types.rs:162-165` — parser 把所有原始类型表示为 `Type::Name`
- `src/frontend/core/parser/statements/types.rs:392-396` — `Result`/`Option` 不 lower 成专用 AST 节点
- `src/frontend/core/parser/statements/types.rs:836` — `Type::Void` 的唯一前向构造点（错误兜底）
- `src/frontend/core/parser/statements/declarations.rs:42-80` — `name_used_as_type` 原件
- `src/frontend/core/parser/statements/declarations.rs:494-511` — 硬编码 `"Terminates"` 与 `typecheck::operator_interfaces::spec()` 调用
- `src/frontend/core/parser/pratt/nud.rs:505-514` — 表达式位置被表示为空 body `Expr::Lambda`
- `src/frontend/core/parser/pratt/led.rs:466` — `expr_to_params` 臂把参数列表捞回
- `src/frontend/core/types/mono.rs:618-643` — `from_builtin_name` 同义词表
- `src/frontend/core/types/mono.rs:692-830` — `From<Type> for MonoType` 实现（13 个 match 臂中的 12 个在此）
- `src/frontend/core/types/mono.rs` — `MonoType` 定义（约 22 变体，`mono.rs:167` 起；1077 行文件）
- `src/frontend/core/types/const_data.rs:234-258`、`321-330` — `const_data::BinOp` / `const_data::UnOp`
- `src/frontend/core/types/solver.rs:411` — `pub fn unify`
- `src/frontend/core/types/eval/const_eval.rs` — 编译期常量求值（1128 行）
- `src/frontend/core/types/eval/dependent_types.rs:478` — `check_structural_termination`
- `src/middle/core/ir.rs:3` — `ir::Type` 作为 `ast::Type` 别名的证据
- `src/middle/core/bytecode.rs:808-814` — `BytecodeFunction` 的 `Vec<ir::Type>` 字段
- `src/middle/core/bytecode.rs:2352-2390` — 有损 `From<MonoType> for IrType` 桥
- `src/middle/passes/codegen/bytecode.rs:32`、`275-278` — `FunctionCode` 的 `Vec<MonoType>` 字段
- `src/middle/passes/mono/function.rs:507-529` — `mono_to_ast_type`：重建 7 个"死变体"的反向桥
- `src/backends/interpreter/image.rs:44` — 解释器模块的 `type_table`
- `build.rs:19-37` — 错误码注册表构建期门禁（类型表门禁的形态参照）
- `build.rs:39-54` — RFC-013 码表一致性比对
- `tools/code-tables/Cargo.toml:15-16` — 门禁 crate 仅依赖 `serde_json`（根 crate 无 `syn`；`tools/type-tables` 引入 `syn` 是首个例外）
