# 前端范式：词法与语法

> **附属设计文档**。本文是 [RFC-039 编译器架构重构](../../rfc/accepted/039-compiler-architecture.md) 的附属文档。四层模型、验收判据分级与执行阶段顺序见 RFC-039 正文;各附属文档的定位见 [本目录索引](index.md)。

## 定位与范围

本文处理 L2 前端——**词法（`src/frontend/core/lexer/`）与语法（`src/frontend/core/parser/`）**。

**要解决的问题**：把"加一个二元运算符"的改动面，从当前的 **L2 内 6-7 处 + 下游 17 个生产文件**（其中编译器强制约 8-9 处，其余靠人工同步）降到 **1-2 处声明 + 7-8 处自动生成**。

本文落实 RFC-039 功能路由表 A 第一行"二元运算符"所标注的痛点。重构类别为 [07-equivalence-oracle.md](07-equivalence-oracle.md) 定义的 **C5 前端范式变更**（判据：AST 快照相同 + 诊断相同 + 行为相同，三者全用）与 **C6 纯删除**（无需等价性判据，只需确认无引用）。C6 项是 C5 的前置：删除 96 行死代码、消除三处裸魔数、复活 629 行从未运行的词法测试。

**范围边界**：`ast::Type`（26 变体）的处置与三套平行类型表示的收敛归 [03-type-unification.md](03-type-unification.md)，本文不重复处置，只在"详细设计"一节声明接口边界。

### 为什么这件事值得做

"加一个运算符"是前端最贵的一次改动。这个税有**三个恶性特征**：

**其一，成本不随经验下降。** 加过十次运算符的人并不会比加过一次的人少改一处——因为这些位置之间没有可推导的关系。`led.rs:40` 的 `(6,7)` 和 `nud.rs:1041` 的 `12` 之间没有任何联系，但它们必须彼此一致，否则表达式解析出错。

**其二，漏改的后果分两类，其中最坏的一类编译器抓不到。** 详见"加一个运算符的改动面"一节的分层表：穷尽 `match` 漏改会被编译器拦住，而**跨枚举漏改编译器永远抓不到**。

**其三，这个税在账上看不见。** `git log --oneline -- src/frontend/core/parser/pratt/led.rs` 里的每一次提交几乎都同时碰 3-4 个文件，但没有任何工具能把"这次提交漏了哪一处"指出来。

### 同仓库已有的正确范式

本项目有两处**同类问题的正确解法**，可直接复用：

- **错误码**：`build.rs:19-55` 用 `tools/code-tables` 在**构建期**解析注册表、校验唯一性、逐条比对 RFC-013 的 markdown 码表，**任一不一致直接 `panic!` 拒绝编译**。加一个错误码的成本因此是恒定的 1 处 + 文档。
- **标准库接口**：`gen_interfaces.rs` 逐字节比对、`gen_docs.rs` 标记区间漂移检测（RFC-039 路由表 B 末行）。

这两个机制的共同点是**把"必须同步修改"从人的记忆变成可执行的断言**。本文的全部目标都可以归结为同一句话：把运算符表、词法规则表、AST 变体表三者之间的对应关系变成可执行的断言。

### 为什么"只删死代码"不够

死代码确实存在（`precedence.rs` 96 行、`skip_old_function_syntax` 空函数、629 行孤儿测试），但删掉它们之后，"加一个运算符改十几处"一处都不会少。死代码是**存量债**，跨文件的改动面是**结构性成本**。二者必须分开处理（处置见"关键决策与理由"）。

## 现状

以下全部为**已核实事实**，每条附文件路径与行号。核实方法与逐项复核实测结果见"核实记录"。

### 词法层：`literals.rs` 的复制粘贴证据

`src/frontend/core/lexer/literals.rs` **共 1568 行**。这个体积不来自复杂度，来自三处结构性重复。

**重复一：四个基数扫描器几乎逐行相同。**

| 扫描器 | 行范围 | 行数 |
| --- | --- | --- |
| `scan_hex_number` | `literals.rs:77-161` | 85 |
| `scan_octal_number` | `literals.rs:164-247` | 84 |
| `scan_binary_number` | `literals.rs:250-333` | 84 |
| `scan_decimal_number` | `literals.rs:336-516` | 181 |

前三个各约 84 行，结构完全一致：累积 digit → `has_digits` 检查 → `overflow` 标志 → `checked_mul(base)` → `checked_add` → `try_into` → 报错。**唯一差异是 base 常量和 digit 提取表达式。** 三份 × 84 行里，**约 250 行是纯冗余**（`scan_decimal_number` 因需额外处理小数点与指数，不能合并，故单列）。

证据抽样——`scan_hex_number` 的溢出路径（`literals.rs:134-141`）：

```rust
if overflow {
    lexer.error = Some(crate::frontend::core::lexer::LexError::InvalidNumber(
        value,
        point(lexer),
    ));

    return Some(lexer.make_token(TokenKind::Error("Hex number too large".to_string())));
}
```

`scan_octal_number:220-227`、`scan_binary_number:306-313` 是同一段代码换 base 与消息文本。

**重复二：转义解码实现 3 遍。**

| 实现 | 位置 | 长度 |
| --- | --- | --- |
| `scan_string` 的 `\\` 分支 | `literals.rs:694-827` | 134 行 |
| `scan_char` 的 `\\` 分支 | `literals.rs:1015-1141` | 127 行 |
| `push_fstring_escape` | `literals.rs:1374` 起 | 已抽成独立函数 |

前两份是 127-134 行的近乎逐行复制（`'n'/'t'/'r'/'\\'/'"'/'\''/'0'` → push 相同，`\x` / `\u{...}` 逻辑相同，`c =>` 报错兜底相同），**约 250 行被写了两遍**。

值得单独指出的是：**f-string 版本已经被抽成了独立函数**（`push_fstring_escape:1374`）。这说明**已经有人意识到这个重复并局部修复了，但没回头收敛另外两处**。这比"三处都没意识到"更值得记录——它证明问题可解，只是缺一个强制机制。

**重复三：多行字符串扫描两份。** `scan_multi_line_string:869` 与 `scan_fstring_multi_line:1497` 又是两份同类实现。

**异常排版残留。** `literals.rs:777-787` 与 `1098-1108` 两处，各 11 行，形态是**结构体字面量的每个字段之间插入 3 个连续空行**。这是自动化改写残留。同一函数的其他位置（`1111-1115`）字段间只有单个空行，说明格式化被中断过。

> 这类残留本身无害，但它是**诊断信号**：同一处代码被不同工具改写过两遍，意味着历史上有人在这里做过局部修补而没有拉通上下文。

**结论**：`literals.rs` 中约 500 行（250 行基数扫描冗余 + 250 行转义双份）是**机械重复**，不是复杂度。

**f-string 插值触发嵌套编译。** 这是**运行时的重复编译**，不是重复实现，因此不能靠合并函数解决。

`src/frontend/core/parser/pratt/nud.rs:441-446`：

```rust
let tokens_result = crate::frontend::core::lexer::tokenize(expr_str_trimmed);
match tokens_result {
    Ok(tokens) => {
        let mut parser = crate::frontend::core::parser::ParserState::new(&tokens);
        if let Some(expr) =
            parser.parse_expression(crate::frontend::core::parser::pratt::BP_LOWEST)
```

**parser 层在运行中对每个 f-string 插值单独跑一次完整 `tokenize()`，并新建一个 `ParserState`。**

后果：含 N 个插值的 f-string **触发 N+1 次完整词法分析**（N 次插值 + 1 次外层）。同时，每个插值是一个**独立的解析上下文**，因此：

- 插值内的诊断 span 是相对插值文本的，重新映射到源码位置需要额外工作；
- 插值内无法访问外层的任何 parser 状态（当前无此需求，但结构上封死了未来的可能性，例如插值内引用外层隐式变量）；
- 词法错误与语法错误的产生位置被割裂在两个阶段。

这是**性能与结构双重缺陷**，必须处理（见"目标设计 · 词法"）。

### 语法层：两套 BP 阶梯、裸魔数、手写分派表

#### 两套阶梯不是"一套活一套死"，而是彼此交织

`src/frontend/core/parser/pratt/precedence.rs` 中同时存在**两套绑定力阶梯**，编号大面积重叠。

**第一套（`precedence.rs:6-17`）**：`BP_LOWEST=0` / `BP_ASSIGN=1` / `BP_LOGICAL_OR=2` / `BP_LOGICAL_AND=3` / `BP_EQUALITY=4` / `BP_COMPARISON=5` / `BP_TERM=6` / `BP_FACTOR=7` / `BP_UNARY=8` / `BP_CAST=8` / `BP_CALL=9` / `BP_HIGHEST=10`。

**第二套（`precedence.rs:22-31`）**：`BP_RANGE=7` / `BP_OR=1` / `BP_AND=2` / `BP_EQ=3` / `BP_CMP=4` / `BP_BIT=5` / `BP_SHIFT=6` / `BP_ADD=7` / `BP_MUL=8`。

**实测结论：两套都在生产代码中存活，而且从 `infix_info` 内部就已经交织。**

- `infix_info`（`led.rs:30-88`）引用**第一套的 5 个常量**：`BP_ASSIGN`（`:33`）、`BP_CALL`（`:75` / `:77` / `:79` / `:83`）、`BP_CAST`（`:81`）；
- 引用**第二套的 8 个常量**：`BP_OR`（`:42`）、`BP_AND`（`:44`）、`BP_EQ`（`:47`）、`BP_SHIFT`（`:51` / `:54`）、`BP_BIT`（`:58`）、`BP_CMP`（`:62` / `:65`）、`BP_ADD`（`:68`）、`BP_MUL`（`:72`）；
- **第二套里唯一的死成员是 `BP_RANGE`**——`..` 在 `led.rs:40` 用的是裸字面量 `(6, 7)` 而不是 `BP_RANGE`；`BP_RANGE` 在生产代码中只出现在 `led.rs:157-159` 的注释里。

第一套 12 个常量的存活状况（`grep` 全仓扫描结果）：

| 状态 | 常量 | 生产引用点 |
| --- | --- | --- |
| **死（6 个）** | `BP_LOGICAL_OR` / `BP_LOGICAL_AND` / `BP_EQUALITY` / `BP_TERM` / `BP_FACTOR` | 生产零命中。仅出现在 `precedence.rs` 自身与孤儿测试 `precedence_inline.rs` |
| 死（仅注释） | `BP_COMPARISON` | 生产零命中；仅在 `declarations.rs:742` 的注释里被提及 |
| **活（6 个）** | `BP_LOWEST` | `parser/mod.rs:80`、`statements/control_flow.rs` 9 处、`statements/declarations.rs` 6 处、`pratt/led.rs:229`/`:321`/`:327`/`:415`、`pratt/nud.rs:195`/`:263`/`:446`/`:516`/`:523` 等 |
| | `BP_ASSIGN` | `led.rs:33`、`led.rs:99`、`statements/declarations.rs:18`（import）、`:743`（`BP_ASSIGN + 1`） |
| | `BP_UNARY` | `nud.rs:32`、`:35`、`:83`、`:101`、`:223`、`:243` |
| | `BP_CALL` | `led.rs:75`、`:77`、`:79`、`:83` |
| | `BP_CAST` | `led.rs:81` |
| | `BP_HIGHEST` | `nud.rs:38-69` 约 20 处（每个字面量与关键字前缀各一处） |

**合计：活着的 6 个常量在生产代码中约 89 处引用，跨 8 个文件。**

> 这修正了一个很容易得出的错误结论："删掉第一套阶梯"**不是** 12 行删除，而是跨 8 个生产文件、89 处引用的重命名与归并。第一套里真正零引用的只有 6 个常量（外加仅存于注释的 `BP_COMPARISON`）。第二套也不完整——`BP_RANGE` 是死的。

**真正零引用的死代码是那两个类型**：`Precedence` 枚举（`precedence.rs:35-94`，60 行）与 `PrecedenceContext` 结构体（`precedence.rs:98-133`，36 行），合计 **96 行**。核实结果：这两个类型在生产代码中零使用——全仓 grep `PrecedenceContext` 与 `Precedence::` 的命中，除 `precedence.rs` 自身外，全部落在 `src/frontend/core/parser/pratt/tests/precedence_inline.rs`。

**这里有一个必须写进文档的细节**：那份唯一引用死代码的文件，**自己也是孤儿**。`pratt/tests/mod.rs:4-6` 只声明了 `mod led; mod nud; mod precedence;`，**没有声明 `precedence_inline`**。

> 即：96 行死代码之所以"看起来有测试覆盖"，是因为它被一个 95 行 / 6 test 的文件测着，而那个文件从未参与编译（详见"死代码与测试现状"）。**删除死代码必须同时处理这个孤儿测试文件**，否则删完就编译不过。

此外 `precedence.rs:33` 有一句自相矛盾的注释作为佐证：*"Precedence rules for the Pratt parser"*——这句注释描述的正是那个零引用的 `Precedence` 枚举，其阶梯与实际生效的阶梯不是同一套。枚举与实现已经脱节。

#### 数值碰撞

两套阶梯重叠区段的编号几乎逐位对应，因此**跨套碰撞成对出现**：

| 冲突 | 值 | 使用点 A（第一套） | 使用点 B（第二套） |
| --- | --- | --- | --- |
| 赋值 vs 逻辑或 | `1 == 1` | `BP_ASSIGN`（`led.rs:33`） | `BP_OR`（`led.rs:42`） |
| 类型转换 vs 乘法 | `8 == 8` | `BP_CAST`（`led.rs:81`） | `BP_MUL`（`led.rs:72`） |
| 移位 vs 项 | `6 == 6` | `BP_TERM`（死） | `BP_SHIFT`（`led.rs:51`/`:54`） |
| 按位 vs 比较 | `5 == 5` | `BP_COMPARISON`（仅注释） | `BP_BIT`（`led.rs:58`） |

**第一套内部还自撞一处**：`BP_UNARY=8` 与 `BP_CAST=8`（`precedence.rs:14` 与 `:15`）——**同一个套内两个不同语义共用一个数值**，而两者都活着（分别用于 `nud.rs` 的前缀绑定与 `led.rs:81` 的 cast）。第二套内部同样自撞：`BP_RANGE=7` 与 `BP_ADD=7`。

**目前之所以没有炸，靠的是一个未被写下来的约定**：几乎所有中缀都用 `bp_right = bp_left + 1`，使右操作数只接受**严格更高**绑定力的运算符——这实际产生的是**左结合**（`a - b - c` → `(a-b)-c`）。因此"比较 `bp_left`"这一唯一实际生效的比较不会因两个运算符 bp 相等而出错。`led.rs:85` 的 `FatArrow` 用 `(11, 1)`，`bp_right` 低于 `bp_left`，是唯一显式的"右操作数贪婪吞并"写法（lambda 体向右最大化）。

这是一个**偶然正确**：它依赖"`infix_info` 返回的 `bp_left` 是唯一的，且 `parse_expression_internal:100` 只比较 `bp_left`"这一事实。任何一处改成真正使用 `bp_left == bp_right` 判别结合性的写法，冲突立刻显形。

#### 三处裸魔数

| 魔数 | 位置 | 问题 |
| --- | --- | --- |
| `(6, 7)` | `led.rs:40` `TokenKind::DotDot` | 字面量绑定力对，无名字；且**没用同套的 `BP_RANGE`**，使 `BP_RANGE` 成为死常量 |
| `(11, 1)` | `led.rs:85` `TokenKind::FatArrow` | 11 **超过两套阶梯的任何上限**（第一套 `BP_HIGHEST=10`） |
| `12` | `nud.rs:1041` `parse_expression(12)` | **大于 `BP_HIGHEST=10`**，用于 RFC-010b 模式探测 |

`nud.rs:896-901` 的注释坦承了这个魔数的来历：*"Lambda binding power is 11, so we use 12 to stop before =>"*、*"parse_expression(12) 会把 `ok` 停成裸 Var、`(` 成意外 token"*。也就是说**这个 12 是试出来的**，不是推导出来的。

#### 分派是手写 match

**分派机制**：全部是手写 `match` 返回函数指针，**无宏、无表、无生成器**。

- `parse_prefix`（`nud.rs:14-19`）取 `prefix_info()` 闭包；
- `parse_expression_internal`（`pratt/mod.rs:79-113`）循环取 `infix_info()` 的 `(bp_left, bp_right, parser_fn)` 三元组；
- 两张手写表：`infix_info`（`led.rs:30-88`）、`prefix_info`（`nud.rs:26-75`）。

#### `parse_assign_after_target`：名字骗人的 507 行总派发器

`src/frontend/core/parser/statements/declarations.rs:136` 的 `parse_assign_after_target`，函数体至 `:642`（**507 行**，文件共 1015 行）。

名字叫 "assign after target"，**实际是声明形式的总派发器**，内含 8 个独立职责段落：旧语法探测 / 标注消歧三层嵌套前瞻 / 声明合法性 / 语义副作用 / 类型体解析回退 / RFC-010 lambda 签名参数重建 / MetaType 定义 / 普通初始化。

膨胀成因是三类叠加：

**成因一：死语言特性约 120 行。**

`is_old_function_syntax`（`declarations.rs:82-117`）、`skip_old_function_syntax`（`declarations.rs:120-122`）、以及 `:145-167` 的配套前瞻。

需要精确说明的是：**这两个函数不是未被引用**。它们在 `declarations.rs:719` 与 `:725` 被 `parse_identifier_stmt` 调用，路径是"检测到 `identifier(` 后跟类型参数和 `->` → 报 '旧语法已弃用' → 跳过 → 返回 None"。

因此准确表述是：**这是一个仍然在跑的"拒绝已移除语法"的门禁**，约 120 行开销用于对每个 `identifier(` 开头的语句做一次括号配平扫描。其中：

- `skip_old_function_syntax`（`:120-122`）**函数体只有一行注释** `// 旧语法已移除，此函数不再需要`，是**纯空函数**；
- 更值得注意的是，调用它之后（`:725-726`）直接 `return None`，**没有消费任何 token**。也就是说这个"skip"**实际上什么都没跳过**——后续 token 仍留在流里，只能靠调用方的错误恢复兜住。这是一个名实不符的接口。

**成因二：前瞻逻辑未抽象。** "跳过配对括号"在同一文件被**手写 4 遍**：

| 位置 | 行范围 |
| --- | --- |
| `is_old_function_syntax` 内 | `declarations.rs:96-105` |
| `parse_assign_after_target` 内 | `declarations.rs:147-156` |
| 同上 | `declarations.rs:191-200` |
| 同上 | `declarations.rs:224-233` |

每次约 12 行，结构一致：`paren_depth = 1` → `while paren_depth > 0 && !at_end()` → 遇 `LParen` 加一、遇 `RParen` 减一 → `bump()`。**四份之间没有共享任何代码。**

**成因三：类型层逻辑漏进 parser。** 详见 RFC-039 路由表 C：`declarations.rs:27-80` 调用类型层的 `is_type_param_annotation` / `name_used_as_type`；`declarations.rs:498-509` 硬编码 `"Terminates"` 字符串并调用 `typecheck::operator_interfaces::spec()`。**这部分归 03-type-unification.md / 02-stage-contract.md，本文不重复处置，只在"详细设计"一节标注边界。**

#### AST 枚举全貌

`src/frontend/core/parser/ast.rs`：

| 枚举 | 位置 | 变体数 |
| --- | --- | --- |
| `Expr` | `ast.rs:16-163` | 22（含 `Error`） |
| `BinOp` | `ast.rs:191-213` | 20 |
| `UnOp` | `ast.rs:217-223` | 4 |
| `StmtKind` | `ast.rs:234` | 9（含 `Error`） |
| `Type` | `ast.rs:427-541` | 26 |
| `Pattern` | `ast.rs:794-814` | 8 |

`Expr::Error` 已核实存在（`pratt/mod.rs:72` 的 span 提取处理它，`ast.rs:1048` 同样）；`StmtKind::Error` 在 `parser/mod.rs:50` 被构造。**这两个错误占位符是向后兼容设计的基石，必须保留**（见"详细设计 · 向后兼容性"）。

### 加一个运算符的改动面

**下游的改动面比预期大得多。** `BinOp::` 在全仓命中 **534 次 / 47 个文件**，其中**生产文件 17 个**。更关键的是，这些文件消费的是**两个互不相关的 `BinOp` 枚举**：

| 枚举 | 定义处 | 变体数 | 生产消费者（括号内为该文件中 `BinOp::` 命中次数） |
| --- | --- | --- | --- |
| `ast::BinOp` | `parser/ast.rs:191-213` | 20 | `led.rs`(22) `nud.rs`(1) `checker.rs`(8) `inference/expressions.rs`(25) `inference/statements.rs`(5) `operator_interfaces.rs`(5) `ir_gen.rs`(23) `formatter/handlers/expr.rs`(25) `lsp/handlers/inlay_hint.rs`(4) `spawn/analysis.rs`(3) |
| `const_data::BinOp` | `types/const_data.rs:234` | 已核实存在（`Ne` 而非 `Neq`） | `const_data.rs`(36) `const_eval.rs`(64) `evaluator.rs`(13) `ownership.rs`(13) `termination.rs`(16) `proof/smt/translate.rs`(13) `proof/dep_graph.rs`(2) |

已核实：`src/backends/` 对 `BinOp` 与 `ast::` **零命中**——解释器消费 IR / 字节码，从不见 AST。因此 17 个生产文件全部落在 L2 → L3 链路上。

**两个枚举之间没有任何 `From`/`TryFrom`**——全仓 grep `impl From<...BinOp`、`-> BinOp`、`BinOp as` 均无转换实现。代价是**每个同时接触两者的文件都必须在 import 处起别名**：`const_eval.rs:19`（`BinOp as AstBinOp`）、`ir_gen.rs:2317`（`use ast::BinOp as B`）、测试文件里的 `CEBinOp` / `ConstBinOp`。

> **这是本文档最重要的发现。** `ast::BinOp` 加一个变体时，**编译器完全无法提醒你去改 `const_data::BinOp`**，因为它们是两个独立类型，各自都能独立编译通过。漏改的后果是：常量折叠路径不认识新运算符，而类型检查与 IR 生成路径认识——**同一个运算符在两个子系统里行为不同，没有任何诊断**。
>
> 这比"漏改一处 `match` 臂"严重一个量级。后者会被穷尽匹配拦住，前者不可能被拦住。

**关于"漏改是否无人能发现"，需要精确区分**：

| 漏改类型 | 是否被编译器发现 | 已核实证据 |
| --- | --- | --- |
| 穷尽 `match` 漏臂 | **会** | `const_data.rs:291-317` 的 `impl Display for BinOp` 是 18 臂穷尽匹配，无 `_` 兜底 |
| `matches!` 宏枚举漏项 | **不会** | `const_data.rs:263-288` 的 `is_arith` / `is_comparison` / `is_logical` / `is_bitwise` 全是 `matches!` 部分列举；新增变体 → 静默返回 `false` |
| `match` 带 `_` 兜底漏臂 | **不会** | `ownership.rs:687` 的 `_ => {}` 静默忽略新变体 |
| **跨 `ast::BinOp` / `const_data::BinOp` 漏改** | **永远不会** | 两者无转换、无共同父类型，编译器无从关联 |

所以准确的说法不是"11 处漏了没人知道"，而是：**穷尽匹配那部分编译器能兜住；真正静默的是 `matches!` 宏、`_` 兜底、以及跨枚举这三类。**

### 死代码与测试现状

这是本文档中**最需要立刻处理**的一项。

#### 死代码

- `precedence.rs:35-94` `Precedence` 枚举（60 行）与 `precedence.rs:98-133` `PrecedenceContext` 结构体（36 行）：生产零使用，唯一外部引用者是孤儿文件 `pratt/tests/precedence_inline.rs`（95 行 / 6 test，未被 `pratt/tests/mod.rs:4-6` 声明）。
- `skip_old_function_syntax`（`declarations.rs:120-122`）：纯空函数，调用点 `:725` 之后直接 `return None`，不消费 token。
- `BP_RANGE`（`precedence.rs:22`）：生产零使用（仅存于 `led.rs:157-159` 注释）。

#### 词法层只有一个测试文件在跑

**`lexer/mod.rs:104-106` 唯一的测试声明是：**

```rust
#[cfg(test)]
#[path = "tests/fstring.rs"]
mod fstring_tests;
```

`grep 'mod tests'` 在 `src/frontend/core/lexer/` 全目录只命中这一处。**`lexer/` 从未声明 `mod tests;`**。

`src/frontend/core/lexer/tests/` 整棵子树因此**从未参与编译**。该目录实际有 **14 个文件**（含 `mod.rs`）：

| 文件 | 行数 | 状态 |
| --- | --- | --- |
| `mod.rs` | 38 | 存在，声明 11 个子模块 |
| `literals.rs` | 222 | 真实测试，**孤儿** |
| `rfc010_lexer.rs` | 167 | 真实测试，**孤儿** |
| `fstring.rs` | 90 | 真实测试，**已通过 `#[path]` 接线** |
| `lexer_mod.rs` | 89 | 真实测试，**双重孤儿** |
| `rfc004_lexer.rs` | 81 | 真实测试，**孤儿** |
| `symbols.rs` | 70 | 真实测试，**双重孤儿** |
| `basic.rs` `comments.rs` `delimiters.rs` `errors.rs` `keywords.rs` `operators.rs` | 各 1-3 行 | 7 个空壳 |
| `debug_lexer.rs` | 1 | 空壳 |

**真实但从未运行的测试：629 行 / 5 个文件**（222+167+89+81+70）。词法层实际在跑的只有 f-string 一个文件。

**一个必须写清楚的接线陷阱：`tests/mod.rs` 存在，但它本身也没接全。**

`lexer/tests/mod.rs:15-25` 声明了 11 个子模块（`basic` / `literals` / `operators` / `delimiters` / `keywords` / `comments` / `errors` / `rfc004_lexer` / `rfc010_lexer` / `debug_lexer` / `fstring`），`:28-38` 再 `pub use` 全部做"向后兼容再导出"。

**但它没有声明 `lexer_mod` 和 `symbols`。** 因此：

> **只加一行 `mod tests;` 是不够的。** 复活 `lexer/tests/` 需要**两处**改动：`lexer/mod.rs` 加 `mod tests;`，**且** `tests/mod.rs` 加 `mod lexer_mod;` 与 `mod symbols;`。否则这 159 行仍然不会运行。

**7 个空壳的陷阱**：`tests/mod.rs:28-38` 的 `pub use basic::*;` 这类语句，**引用一个零断言的模块不会产生任何编译错误或警告**。加完 `mod tests;` 之后，CI 会显示"测试全部通过"，而实际上 `basic` / `comments` / `delimiters` / `errors` / `keywords` / `operators` / `debug_lexer` 七个模块**一条断言都没有**。**这会制造"词法层已覆盖"的假象**，比完全不接线更危险。

**未覆盖且风险最高的路径**（一旦接线就会立即暴露）：

| 路径 | 位置 | 风险 |
| --- | --- | --- |
| 四个基数扫描器的溢出路径 | `literals.rs:134-141` / `220-227` / `306-313` + decimal 对应段 | 整数溢出是静默错误 |
| `checked_mul` 溢出后"继续消费但不报错"分支 | 4 处同构 | 可能造成 token 流错位 |
| `scan_leading_dot` **整个函数** | `literals.rs:519-635`（117 行） | 零覆盖 |
| `\x` / `\u` 非法转义路径 | `literals.rs:730-749` / `1051-1070` | 错误路径难测 |

**parser 侧接线是正常的**：`parser/mod.rs:8-9`、`pratt/mod.rs:8-9`、`statements/mod.rs:11-12` 都有正确的 `mod tests;`。`pratt/tests/mod.rs:4-6` 声明了 `led`（335 行）、`nud`（269 行）、`precedence`（13 行）。

**但 parser 侧的覆盖质量同样有问题**：

- `pratt/tests/precedence.rs`（**13 行**）唯一断言是 `assert!(bp_lowest < bp_highest)`，**近乎空转**——它甚至不能发现上面列出的四处跨套碰撞。
- `pratt/tests/precedence_inline.rs`（**95 行 / 6 test**）未被 `pratt/tests/mod.rs` 声明，**从未运行**——而且它测的**全是那套零引用的 `Precedence` 枚举 + `PrecedenceContext`**。
- **真正在用的两套阶梯（`BP_OR`…`BP_MUL` 与 `BP_LOWEST`/`BP_ASSIGN`/`BP_UNARY`/`BP_CALL`/`BP_CAST`/`BP_HIGHEST`）没有任何直接单元测试。**

## 目标设计

### 词法：声明式化

#### 三个方案的评估

| 方案 | 加一个 token 的成本 | 能否消除词法层的三处重复 | 能否解决 f-string 嵌套编译 | 引入的依赖 |
| --- | --- | --- | --- | --- |
| **A. `logos` 生成 DFA** | 1 处（token 声明） | **能**（转义 / 数字各一份） | 否（需手工处理插值） | 新增 `logos` |
| **B. `regex` 手写 DFA** | 1-2 处 | 部分 | 否 | 新增 `regex`（是否已是传递依赖**未核实**） |
| **C. 保留手写，强制合并重复** | 1 处 | **能**（手动抽公共函数） | **能**（需显式设计） | 无 |

**主张：采用 C 为基线，A 作为条件性备选，B 不采纳。**

理由：

1. **C 能以零新增依赖消除词法层全部约 500 行重复**，且完全在可控范围内。`scan_string` / `scan_char` / `push_fstring_escape` 三份转义解码合并为**唯一**的 `decode_escape(lexer, out) -> bool`，四个基数扫描器合并为**唯一**的 `scan_radix_number(lexer, base, digits_pred)`。这是纯机械重构，判据为 C5 的"AST 快照相同 + 诊断相同"。
2. **A（`logos`）的能力是真实的**——它能把转义解码与数字扫描各收敛为一份。**但它解决不了 f-string 嵌套编译**，而后者是词法层最重要的缺陷。同时引入一个新依赖会改变 `Literals` 的错误类型（`LexError` 的 `InvalidEscape` / `InvalidNumber` 变体需要重新映射），这会**直接违反 C5 的"诊断相同"判据**，除非额外写一层适配——那等于又引入了一次手工同步点。
3. **B（`regex` 手写 DFA）不采纳**：它要求手写状态机，工作量与 A 相当但可控性远不如 A。既然在 A 与 B 之间选，逻辑上必须选 A；既然本文主张 C 优先，就没有选 B 的理由。

> **设计判断（非已核实事实）**：A 在**长期**可能是更好的形态（声明式 token 表本身就是一种"改一处"的强制）。但它的迁移成本与"诊断相同"判据的冲突使它不适合作为本次 C5 的内容。若后续要采纳 A，应作为独立提案，并以本文的测试复活成果作为安全网。

#### f-string 嵌套编译必须显式处理

无论选 A 还是 C，嵌套编译都必须解决。**主张：取消运行时的嵌套 `tokenize()`，改为词法期一次性产出。**

具体做法：`scan_string` 遇到 `f"` 前缀时，**在同一次词法扫描内**把插值内容记为 `FStringSegment::RawText(String)`，同时**不**递归调用 `tokenize()`。插值的真正 tokenize + parse 推迟到 parser 首次需要该 f-string 节点时，且：

- **一次性完成**（对 N 个插值，只做 1 次插值段的 tokenize，把结果缓存为 `Vec<Vec<Token>>`），把 N+1 次降为 1 次；
- **span 映射在词法期就确定**（记录每个插值在源文件中的绝对偏移），不再依赖 `Point` 的相对语义。

这条改动**独立于范式选择**，即使不做 A 也应执行。

### 语法：完整文法驱动（LALRPOP）

> **决策（2026-10-03，ChenXu233）**：**采用 A（LALRPOP 完整文法驱动），不做半截的查表化。** A 的等价性风险（LALRPOP 的错误恢复模型与 `synchronize()` 不同、试出来的解析行为能否逐位复现）已通过下面的**双解析器差分**方案消解；查表化是不充分的中间态。

#### 方案评估

| 方案 | 加一个二元运算符的改动面 | 能否消除 BP 冲突 | 能否保住 `synchronize()` 与错误占位符 | 结论 |
| --- | --- | --- | --- | --- |
| **A. LALRPOP** | 1 处文法 + 1 处动作 | 能（LR 表自动算优先级，冲突在构建期暴露为文法冲突错误） | 需重写，但**错误产生式可显式声明** | **采用** |
| **B. tree-sitter** | 1 处 grammar.js | 能 | 不适用（产 CST 不产领域 AST） | **否决** |
| C. 保留 Pratt 查表化 | 2-3 处 | 能 | 能 | **否决**（不充分，见下） |

**B 否决理由**（不变）：tree-sitter 产通用语法树，本项目的 `Expr` / `StmtKind` 是**带语义信息的领域 AST**（`Expr::Cast` 带目标类型、`Expr::FString` 带 segment 列表、`Pattern::Struct` 带字段名）。CST → 领域 AST 仍需一层完整映射，**tree-sitter 省不掉它**。等于在 1326 行 `nud.rs` 之外再加同样体量的映射层。

**C 否决理由**：查表化解决的是**同步性**（改一处不漏改），没解决**可维护性**——`Expr` 的 22 个变体仍要手写 22 个构造动作，两套 BP 阶梯的归并仍要改约 89 处引用，而 `Expr::Lambda { body: 空 Block }` 被借用作参数列表（`nud.rs:505-514`）这类结构性问题完全没被触及。**它是从"完全手写"到"完整文法"之间的一个不必要中间站。**

#### 否决 A 的原始顾虑，以及它如何被消解

否决 A 的原始顾虑是**等价性**，不是能力不足：

- `ParserState::synchronize()`（`parser_state.rs:170-186`）与 `Expr::Error` / `StmtKind::Error` 占位符构成的"尽量继续解析"语义，LALRPOP 的错误恢复是另一套模型（插入 / 删除 token），**产出的诊断集合必然不同**
- `pratt/mod.rs:100-102` 的 `bp_left < min_bp` 退出条件、`nud.rs:1041` 的 `parse_expression(12)` 模式探测，都是**在特定输入上试出来的行为**，能否逐位复现需逐条证明

**消解方案：双解析器差分（double-parse diff）。** 不靠"判据应该一样"的论证，靠**跑出来**：

| 步骤 | 内容 | 证明什么 |
| --- | --- | --- |
| 1 | 建 LALRPOP 文法 + 动作代码，**与现有 Pratt 完全并存**，不改任何现有代码 | —— |
| 2 | 293 个语料 + `src/std/tests` 全部，**两套 parser 各跑一遍**，AST 按 [07](07-equivalence-oracle.md) 的规范化规则处理后**逐位比对** | **合法程序的 AST 等价性被实测证明，不靠论证** |
| 3 | 切流：`parser/mod.rs` 的 `parse()` 改调 LALRPOP；Pratt 保留为 `parse_legacy()` | —— |
| 4 | 删除 Pratt：`nud.rs`(1326) + `led.rs`(495) + `precedence.rs` 两套阶梯 + 约 89 处引用 | —— |
| 5 | 错误恢复：`synchronize()` 与 `Expr::Error` / `StmtKind::Error` 在 LALRPOP 侧用**显式错误产生式**重建 | 见下 |

**步骤 2 是整个方案的核心**。若 293 个语料的 AST 有任何一处不等价，就在切流前发现——这比"人工逐条论证 89 处引用能否复现"强得多，且成本可接受（离线跑一次）。

#### 错误恢复的处置：完整保留 C5，不放宽

**错误诊断必须逐条一致，这是硬要求，不是可协商的。** 处置方式是**在文法中显式建模现有行为**：

| 现有行为 | 在 LALRPOP 中的对应 |
| --- | --- |
| `ParserState::synchronize()`（`parser_state.rs:170-186`）：语句解析失败后向前跳过 token 直到同步点 | 文法中显式声明"错误 → 跳到同步点集合 → 继续"的产生式，**跳过集合必须与现有实现逐 token 一致** |
| `Expr::Error` / `StmtKind::Error` 占位符：使解析器能在错误后继续构造"部分 AST" | 文法中显式声明错误节点产生式，**下游 match 分支位置不变**（下游若把 `Error` 放在通配分支里会静默吞掉错误） |
| 诊断 code + span | **逐条相同。措辞不变。** |

**这不是"应该做得到"，是必须做到。** LALRPOP 允许写显式错误产生式；把 `synchronize()` 的同步点集合写进文法即可。若阶段 3a/3b 实测证明在 LALRPOP 下无法复现现有诊断，**正确动作是如实报告该发现并重新评估方案（含"维持 Pratt"），不是修改判据**。

迁移的落点与处置：

| 项 | 内容 |
| --- | --- |
| 文法文件 | `src/frontend/core/parser/grammar/yaoxiang.lalrpop`（新目录，见 [01](01-routing.md) 目标结构） |
| 动作代码 | `grammar/actions.rs`——把 `Expr` / `StmtKind` 的构造从 `nud.rs` / `led.rs` 搬过来，**每个变体一个函数** |
| 错误产生式 | 显式声明，配合 `Expr::Error` / `StmtKind::Error` 占位符保留"尽量继续解析"语义 |
| BP 阶梯归并 | 两套阶梯随文法删除；`BP_LOWEST` / `BP_ASSIGN` / `BP_UNARY` / `BP_CALL` / `BP_CAST` / `BP_HIGHEST` 这 6 个活着的常量，其**约 89 处生产引用**改由文法中的优先级声明承担 |
| 裸魔数 | `(6,7)` / `(11,1)` / `12` 随 Pratt 一并消失——LR 表自动计算优先级 |
| `is_old_function_syntax`（`declarations.rs:82-117`，36 行） | **自然删除**——文法驱动后 `f(Int) -> Int = ...` 无法匹配任何产生式，专门的"探测已移除语法"是反模式 |
| `Expr::Lambda { body: 空 Block }` 借用（`nud.rs:505-514`） | **自然消除**——参数列表在文法里就是参数列表，不需要借 AST 节点 |
| `synchronize()`（`parser_state.rs:170-186`） | 保留 API，改为由错误产生式驱动调用点 |

**净效果**：删除 `nud.rs`(1326) + `led.rs`(495) + `precedence.rs` 的两套阶梯 + 约 120 行死旧语法探测，新增文法文件与动作代码。**"加一个二元运算符"的改动面从十几处降到 1 处文法 + 1 处动作**，且 BP 冲突在**文法构建期**就报错，不会再像现在这样"偶然正确"。

#### 前置条件

1. **`lexer/tests/` 的 629 行 / 55 个测试必须先复活**（[09](09-execution-wbs.md) 的 P1）。没有前端测试安全网就换掉整个解析器，无法满足 07 "判据必须先于重构"
2. **AST 规范化快照必须先入库**（P2 的第二层），否则步骤 2 无从比对
3. **f-string 嵌套编译消除**（`literals.rs` 改造）**独立于本项**，即使不上文法驱动也应执行

### `parse_assign_after_target` 的处置

**主张：拆成 8 个具名函数，函数体只做派发。**

507 行、8 段职责的函数，任何一次局部修改都要在 500 行里定位。拆分后的目标形态：

| 拆分出的函数 | 职责 | 大致行数 | 归属 |
| --- | --- | --- | --- |
| `probe_old_function_syntax` | 旧语法探测 | 36 | **本文删除** |
| `resolve_annotation_ambiguity` | 标注消歧（三层嵌套前瞻） | 待核实 | 本文（抽出独立前瞻工具后大幅缩短） |
| `check_declaration_legality` | 声明合法性 | 待核实 | 本文 |
| `apply_semantic_side_effects` | 语义副作用 | 待核实 | **03-type-unification.md**（含 `498-509` 的 `"Terminates"` 硬编码） |
| `parse_type_body` | 类型体解析回退 | 待核实 | 本文 |
| `rebuild_lambda_params` | RFC-010 lambda 签名参数重建 | 待核实 | 本文 |
| `parse_meta_type_def` | MetaType 定义 | 待核实 | 本文 |
| `parse_plain_init` | 普通初始化 | 待核实 | 本文 |

配套：把 4 份"跳过配对括号"（`96-105` / `147-156` / `191-200` / `224-233`）收敛为 `ParserState` 上的唯一方法 `skip_balanced_parens()`，4 处改为调用它。

**净效果（设计判断）**：507 行 → 8 个单一职责函数 + 1 个前瞻工具；前瞻代码从 48 行降到约 12 行；类型层逻辑移出后（约 80 行）实际归属本文的部分约 300 行。

### 死代码清理（C6，无需等价性判据）

按 07-equivalence-oracle.md 的 C6 定义（"纯删除，无需等价性判据，只需确认无引用"）：

| 清理项 | 位置 | 行数 | 前置条件 |
| --- | --- | --- | --- |
| `Precedence` 枚举 | `precedence.rs:35-94` | 60 | **必须同时处理** `pratt/tests/precedence_inline.rs`（唯一引用者，本身是孤儿） |
| `PrecedenceContext` 结构体 | `precedence.rs:98-133` | 36 | 同上 |
| 第一套中零引用的 6 个常量 | `precedence.rs:8-13`、`:11`（仅注释） | 6 | 生产零引用，直接删 |
| `BP_RANGE` | `precedence.rs:22` | 1 | 生产零引用（仅存于 `led.rs:157-159` 注释），先改 `..` 用具名常量 |
| `skip_old_function_syntax` | `declarations.rs:120-122` | 3 | 空函数；`declarations.rs:725` 的调用点一并删除 |
| `is_old_function_syntax` | `declarations.rs:82-117` | 36 | 见下 |
| `declarations.rs:145-167` | 同文件 | 23 | 同上 |
| 裸魔数 `(6,7)` | `led.rs:40` | — | 提升为 `BP_RANGE_L` / `BP_RANGE_R` 具名常量 |
| 裸魔数 `(11,1)` | `led.rs:85` | — | 提升为 `BP_LAMBDA_L` / `BP_LAMBDA_R` |
| 裸魔数 `12` | `nud.rs:1041` | — | 提升为 `BP_PATTERN_PROBE`，并**在注释中记录它是试出来的** |

> **注意**：第一套阶梯中 `BP_LOWEST` / `BP_ASSIGN` / `BP_UNARY` / `BP_CALL` / `BP_CAST` / `BP_HIGHEST` **不在本表的纯删除范围内**——它们在生产代码中有约 89 处引用，其归并属于上面的"语法：文法驱动 · C 方案"（C5 范畴），不是 C6 删除。

**关于 `is_old_function_syntax` 的处置选择（设计判断）**：

它不是未被引用的死代码，而是**一个仍在运行的"拒绝已移除语法"门禁**（`declarations.rs:719`）。两个选项：

- **选项 1（激进）**：整段删除。已移除的语法，其"拒绝"应交给**语法定义本身**——文法驱动后 `f(Int) -> Int = ...` 根本无法匹配任何产生式，自然报错，不需要专门探测。**这是范式变更带来的红利**：约 120 行在文法驱动后自然消失。
- **选项 2（保守）**：保留门禁但删掉空函数 `skip_old_function_syntax`，并把"不消费 token 就 return None"改为真正跳过。

**主张选项 1**，因为它与"保留 Pratt 查表化"的主张自洽：既然分派表是唯一事实源，专门手写"探测某个已移除语法"就是反模式。**前提是 C5 判据证明诊断集不变**——需实测"删除后 `f(Int) -> Int = x => x` 的诊断 code 与 span 是否与删除前一致"。若不一致，退回选项 2 并记为开放问题。

### 测试重建

**这是第一优先级，必须先于任何范式变更。**

**阶段 1：复活 `lexer/tests/`。** 需要**两处**改动，不是一处：

1. `src/frontend/core/lexer/mod.rs` 在 `:106` 之后加 `#[cfg(test)] mod tests;`
2. `src/frontend/core/lexer/tests/mod.rs` 补 `mod lexer_mod;` 与 `mod symbols;`（否则这 159 行仍不运行）

**阶段 2：处置 7 个空壳。** 关键约束：`tests/mod.rs:28-38` 的 `pub use xxx::*;` 引用空模块**不产生任何编译错误或警告**，接线成功后 CI 会显示"全绿"而实际零覆盖。处置方式：

| 方案 | 做法 | 评价 |
| --- | --- | --- |
| **删除（主张）** | 删掉 7 个空壳文件 + 对应 `mod` 声明 + 对应 `pub use` 行 | **干净。** 空壳的唯一价值是"以后填"，而 `literals.rs` / `rfc004_lexer.rs` 已经承担了这些职责 |
| 填充 | 给每个空壳写真实断言 | 7 个文件 × 内容未知，**无法核实应该测什么**（未核实） |
| 保留 + 标记 | 保留但加 `// TODO` 与 CI 计数告警 | 制造"已覆盖"假象，**反对** |

**主张删除**，并在死代码清理的同时把 `tests/mod.rs:28-38` 的 `pub use` 再导出**全部删除**（它本身就是为了配合一个不存在的再导出设计，且正是它掩盖了空壳）。

**阶段 3：补结合性用例。** 新增 `pratt/tests/binding_power.rs`，对**实际生效的两套阶梯**断言：

- 合并后的层内 bp 严格递增、无重复（**这条会在改造前就抓出上述四处碰撞**）
- 每一对相邻层各写一个最小表达式，断言其结合方向（如 `a - b - c` → `(a-b)-c`）
- `DotDot` / `FatArrow` / 模式探测三个特例各一组用例，**替代目前空转的 `pratt/tests/precedence.rs:6-13`**

**阶段 4：补字面量错误路径。** 针对未覆盖路径表中的四类逐条加用例：四基数溢出、`checked_mul` 后继续消费分支、`scan_leading_dot` 整体、`\x` / `\u` 非法转义。

**阶段 5：删除 `precedence_inline.rs`**（随死代码删除），并在 `pratt/tests/mod.rs` 补一条**接线自检**断言，防止再出现孤儿。

### 改一个运算符要改几处：前后对比

| # | 现状 | 现状的强制力 | 目标 | 目标的强制力 |
| --- | --- | --- | --- | --- |
| 1 | `lexer/state.rs:21` `keyword_from_str` 加 match 臂（仅关键字形式） | 无 | 运算符声明表 1 行 | 编译期 |
| 2 | `lexer/tokens.rs:81-162` `TokenKind` 加变体 | 编译期 | **由表生成** | 编译期 |
| 3 | `pratt/precedence.rs:6-17` 与 `22-31` 选一套加 BP 常量 | 无 | **由表生成** | 编译期（层内唯一性断言） |
| 4 | `pratt/led.rs:30-88` `infix_info` 加臂 | 编译期（穷尽） | **由表生成** | 编译期 |
| 5 | `pratt/led.rs:116-136` `parse_binary` 加 op 映射臂 | 编译期（穷尽） | **由表生成** | 编译期 |
| 6 | `ast.rs:191-213` `BinOp` 加变体 | 编译期 | **由表生成** | 编译期 |
| 7 | 新符号字符：`tokenizer.rs:218` `next_token_inner` | 无 | **由表生成** | 编译期 |
| 8 | `const_data.rs:234` `const_data::BinOp` 加变体 | **无（跨枚举，编译器抓不到）** | **与 `ast::BinOp` 同源生成** | 编译期 |
| 9 | 17 个下游生产文件中依赖 `matches!` / `_` 兜底的站点 | **无（静默）** | 生成的访问器 | 编译期 |

**汇总：**

| 阶段 | 手工改动处数 | 自动生成处数 |
| --- | --- | --- |
| 现状 | **11-12**（L2 内 6-7 + 下游约 5-6 个必须同步的生产文件） | 0 |
| 目标（纯新符号运算符） | **1-2**（运算符声明表 1 行；若为关键字形式再加 `keyword_from_str` 1 行） | 7-8 |
| 目标（复用已有 token） | **1** | 6-7 |

且目标的 1-2 处**全部是声明式数据行**，不是逻辑代码——审查成本从"逐个 match 臂推演语义"降为"读一行声明"。

## 详细设计

### 与类型系统的接口边界

**边界声明（重要）**：`ast::Type`（26 变体，`ast.rs:427-541`）的收敛、平行类型表示的消除、`is_type_param_annotation` / `name_used_as_type` 的归属，**全部属于 [03-type-unification.md](03-type-unification.md)**。本文只处理三处与前端范式直接相关的接口：

**接口一：`const_data::BinOp` 与 `ast::BinOp` 的关系。** 归**本文**——因为它是"加一个运算符"改动面的直接组成部分。处置：由运算符声明表**同时生成**两个枚举（或生成一个、派生另一个），消灭 `const_eval.rs:19` / `ir_gen.rs:2317` 的 import 别名。判定归属本文而非 03 的理由：它是 L2 的产物被 L3 平行复制，不是类型表示本身的问题。

**接口二：`declarations.rs:498-509` 的硬编码 `"Terminates"` 与 `typecheck::operator_interfaces::spec()` 调用。** 归 **03-type-unification.md / 02-stage-contract.md**（消除 parser → typecheck 反向依赖，RFC-039 路由表 C 第一行）。本文在函数拆分中把这段代码**隔离到 `apply_semantic_side_effects`**，作为交接点，但不改其内容。

**接口三：运算符的优先级层是否需要类型层信息。** 当前不需要——`infix_info` 是纯语法函数。**设计判断**：查表化后应保持这一性质，运算符声明表中**不得**出现任何类型相关字段，否则 L2 会重新依赖 L3（违反 RFC-039 的依赖方向规范）。

### 运行时行为

**目标：运行时行为逐位不变。** 依据 07-equivalence-oracle.md 的 C5 判据，第三层（端到端语料差分）比对 `tests/yaoxiang/` 293 个 `.yx` + `src/std/tests/*.yx` 的退出码与 stdout/stderr。

三处**已识别的运行时影响**必须显式处理：

| 改动 | 运行时影响 | 处置 |
| --- | --- | --- |
| f-string 插值合并为 1 次 tokenize | **性能改善**，输出不变 | 需实测耗时收益并记录 |
| 运算符表生成 BP / 两套阶梯合并重排 | **可能改变结合性** | **必须为每个层对写等价性用例**（测试重建 · 阶段 3） |
| 删除 `is_old_function_syntax` | 旧语法的诊断 code/span 可能变化 | 需语料差分证明一致 |

**明确不做的事**：不改变运算符的优先级**语义**。BP 编号可以重排（消除死代码与碰撞），但"乘法绑定得比加法紧"这类**关系**必须不变。

### 编译器改动清单

逐文件、带行号。C6 清理（阶段 0，可独立提交）：

| 文件 | 行号 | 改动 |
| --- | --- | --- |
| `src/frontend/core/parser/pratt/precedence.rs` | `8-13`、`11` | 删除零引用的 6 个第一套常量 |
| 同上 | `22` | `BP_RANGE` 改为具名 `..` 绑定力（`:40` 同步），否则删除 |
| 同上 | `35-94` | 删除 `Precedence` 枚举 |
| 同上 | `98-133` | 删除 `PrecedenceContext` 结构体 |
| 同上 | `6-17`、`22-31` | 保留存活常量，与第二套合并为层内无冲突编号 |
| `src/frontend/core/parser/pratt/led.rs` | `33` | `BP_ASSIGN` 改指到重排后的赋值层常量 |
| 同上 | `40` | `(6,7)` → `BP_RANGE_L` / `BP_RANGE_R` |
| 同上 | `75`、`77`、`79`、`83` | `BP_CALL` 改指到重排后的后缀层常量 |
| 同上 | `81` | `BP_CAST` 改指到重排后的转换层常量 |
| 同上 | `85` | `(11,1)` → `BP_LAMBDA_L` / `BP_LAMBDA_R` |
| `src/frontend/core/parser/pratt/nud.rs` | `32`、`35`、`83`、`101`、`223`、`243` | `BP_UNARY` 改指到重排后的前缀层常量 |
| 同上 | `38-69` | `BP_HIGHEST` 改指到重排后的最高层常量 |
| 同上 | `1041` | `parse_expression(12)` → `parse_expression(BP_PATTERN_PROBE)`，并补注释说明该值来源 |
| `src/frontend/core/parser/statements/declarations.rs` | `120-122` | 删除 `skip_old_function_syntax` |
| 同上 | `725` | 删除其调用点（`is_old_function_syntax` 的删除见阶段 2） |
| `src/frontend/core/parser/pratt/tests/precedence_inline.rs` | 全文 | 删除（唯一引用死代码的孤儿文件） |

配套改名（BP 归并的连带面，约 89 处）：`parser/mod.rs:80`、`statements/control_flow.rs` 9 处、`statements/declarations.rs:18` / `321` / `601` / `743` / `818` / `850` / `966` / `994`、`statements/functions.rs:9` / `:110`、`statements/bindings.rs:168`、`statements/types.rs:15` / `:641` / `:654`、`pratt/led.rs:99` / `:229` / `:321` / `:327` / `:415`、`pratt/nud.rs:195` / `:263` / `:446` / `:516` / `:523`。

测试复活（阶段 1，可独立提交）：

| 文件 | 行号 | 改动 |
| --- | --- | --- |
| `src/frontend/core/lexer/mod.rs` | `106` 之后 | 新增 `#[cfg(test)] mod tests;` |
| `src/frontend/core/lexer/tests/mod.rs` | `15-25` | 新增 `mod lexer_mod;` 与 `mod symbols;` |
| 同上 | 全文 | 删除 7 个空壳的 `mod` 声明与 `pub use` 行 |
| 同上 | 全文 | 删除 `pub use ... ::*;` 再导出块（`28-38`） |
| `src/frontend/core/lexer/tests/` | 7 个空壳文件 | 删除文件 |
| `src/frontend/core/parser/pratt/tests/mod.rs` | `4-6` | 新增接线自检断言 |
| `src/frontend/core/parser/pratt/tests/precedence.rs` | `6-13` | 用真实结合性断言替换空转断言 |

范式变更（阶段 2-4）：

| 文件 | 改动 |
| --- | --- |
| `src/frontend/core/lexer/literals.rs` | 四个基数扫描器合并为 `scan_radix_number(lexer, base, digit_pred)`；三处转义解码合并为唯一 `decode_escape(lexer, out)`；`scan_multi_line_string:869` 与 `scan_fstring_multi_line:1497` 合并 |
| `src/frontend/core/parser/pratt/nud.rs:441-446` | 移除运行时嵌套 `tokenize()`，改由词法期一次性产出 |
| `src/frontend/core/parser/pratt/led.rs:30-88` / `nud.rs:26-75` | 两张手写表替换为生成的映射表 |
| `src/frontend/core/parser/ast.rs:191-213` | `BinOp` 与 `const_data::BinOp` 改为同源生成 |
| `src/frontend/core/types/const_data.rs:234` | 同上 |
| `src/frontend/core/parser/statements/declarations.rs:136-642` | 拆为 8 个函数 + `ParserState::skip_balanced_parens()`（`96-105`/`147-156`/`191-200`/`224-233` 四处改为调用） |
| `src/frontend/core/parser/parser_state.rs` | 新增 `skip_balanced_parens()`（`synchronize()` 在 `:170`，保持不变） |

**本文零改动的文件**：`src/middle/core/ir_gen.rs`、`src/frontend/core/typecheck/**`、`src/backends/**`、`src/frontend/core/formatter/**` 的**主体**。它们只在"生成的访问器"替代 `matches!` 宏时才改动，且改动是机械替换。

### 向后兼容性

查表化最容易破坏的是**错误恢复**。本项目的恢复契约由三部分组成，必须逐一保持：

**契约一：`ParserState::synchronize()`（`parser_state.rs:170-186`）。** 它在语句解析失败后向前跳过 token 直到同步点。**C5 判据要求保留它，且要求它触发的时机与跳过集合完全一致。** 迁移到 LALRPOP 后该函数的调用点改为由显式错误产生式驱动，函数本体与同步点集合**不改一行**——语义由文法复现，实现原样保留。

**契约二：`Expr::Error`（`ast.rs:1048`、`pratt/mod.rs:72` 消费）与 `StmtKind::Error`（`parser/mod.rs:50` 构造）。** 这两个占位符使解析器能在错误后继续构造一棵"部分 AST"，下游按 `Error` 分支跳过。**必须保留变体，且必须保留它们在下游 match 中的位置**（下游若把 `Expr::Error` 放在通配分支里，错误会被静默吞掉）。

**契约三：诊断的 code、span 与顺序。** 07-equivalence-oracle.md 第三层的比对规则是"诊断列表按 `(code, span.file, span.line)` 排序，**消息文本不参与比对**"。这给本文一定余量（措辞可变），但 **code 与 span 不可变**。

**f-string 特别说明**：插值改造会把 span 从"相对插值文本"改为"源文件绝对偏移"。为满足契约三，**span 必须在词法期就按绝对偏移记录**（决议 D25）——插值内诊断的 `span.file/line` 不得因本次改造改变。

> **处置**：阶段 2 前先建立"f-string 插值内诊断 span"的基线快照，改造后逐条比对。`span.file/line` 不一致即视为实现缺陷并修复，不存在"接受 span 变化"的选项。（若实测证明原实现的插值 span 本就是错的，基线建立阶段就会暴露——那是既有的独立缺陷，单独修复，不并入本阶段，也不构成放宽判据的理由。）

## 实施要点

本文对应五个阶段。**每阶段可独立提交、独立回滚。** 阶段 0 与 1 是 C6 与测试复活，不需要等价性判据；阶段 2-4 是 C5，全程受判据约束。

| 阶段 | 内容 | 类别 | 验收判据 | 回滚点 |
| --- | --- | --- | --- | --- |
| **0** | 死代码清理（`Precedence` 枚举 + `PrecedenceContext` 96 行） | C6 | `cargo build` 通过；`cargo clippy` 无新增警告 | 单个提交，`git revert` 即可 |
| **1** | 测试复活与补全 | — | 词法层实际运行测试数 ≥ 5 文件 / 629 行；7 个空壳已删；结合性用例**先红后绿** | 单个提交 |
| **2** | 词法重复收敛 + f-string 嵌套编译消除 | C5 | AST 快照相同 + 诊断相同 + 行为相同；`literals.rs` 减少约 500 行 | 与阶段 3 分开提交 |
| **3a** | 建 LALRPOP 文法 + 动作代码，**与 Pratt 完全并存** | — | `cargo build` 通过；新 parser 对 293 语料能产出 AST（不比对） | 与阶段 2 分开提交 |
| **3b** | **双解析器差分**（两套 parser 各跑 293 语料，**AST + 诊断 code+span** 规范化后逐位比对） | **完整 C5** | **AST 与诊断均逐条等价——这一步是整个文法迁移的等价性证明** | 离线跑一次，可重复 |
| **3c** | 切流：`parser/mod.rs` 的 `parse()` 改调 LALRPOP，Pratt 留为 `parse_legacy()` | **完整 C5** | 全量语料行为等价 + 诊断 code+span 逐条相同 | 独立提交 |
| **3d** | 删 Pratt：`nud.rs`(1326) + `led.rs`(495) + `precedence.rs` 两套阶梯 + 约 89 处引用 | **完整 C5** | 语料全绿；`git grep BP_` 零命中 | 独立提交 |
| **4** | `parse_assign_after_target` 拆分 | C5 | 该函数只做派发；`skip_balanced_parens` 单一实现 | 独立提交 |

**本文内部的依赖顺序**：阶段 1 **必须先于**阶段 2/3/4——"先重构再补测试"是本项目已踩过的坑。**阶段 3b（双解析器差分）是不可跳过的**：它必须在 3c 切流之前跑出全绿，否则整个文法迁移没有等价性依据。

**3b/3c 的 go/no-go 关口**：若错误产生式的条数明显超出"把 `synchronize()` 的同步点集合建模出来"所需（parser 内嵌的语义判定——`declare_predicate` 累积、const 泛型参数过滤——是最难平移的部分），或差分连续两个迭代仍有不等价项，**停止迁移并重新评估**，回落到"保留 Pratt + 单套声明式优先级表 + 两套 `BinOp` 同源生成"的中间方案。该方案已能拿到运算符改动面收敛的大部分收益；LALRPOP 是手段，不是目标。回落属于 D24 预留的"重新评估"分支，**不是判据放宽**。

**阶段 0 的一个说明**：`BP_LOWEST` / `BP_ASSIGN` / `BP_UNARY` / `BP_CALL` / `BP_CAST` / `BP_HIGHEST` 这 6 个**活着的**第一套常量**不在阶段 0 的删除范围**——它们由阶段 3d 随 Pratt 一并删除。阶段 0 只删 `Precedence` 枚举与 `PrecedenceContext`（96 行，零生产引用）。

**判据执行方式**：三层判据的定义与回归门禁见 [07-equivalence-oracle.md](07-equivalence-oracle.md)。**全部走完整 C5，包括诊断 code+span。不存在任何放宽。**

**范围限定**：本文**不包含** `logos` 迁移（词法保持手写 + 合并重复，理由见「关键决策与理由」）。**LALRPOP 语法迁移是本文的组成部分**，见「语法：完整文法驱动」一节。

## 关键决策与理由

### 已选定的三条决策

| 决策 | 决定 | 理由 |
| --- | --- | --- |
| 词法方案 | 采用"保留手写 + 强制合并重复"；`logos` 列为条件性备选 | 零新增依赖即可消除约 500 行重复；`logos` 改变 `LexError` 错误类型，且解决不了 f-string 嵌套编译 |
| **语法方案** | **采用 LALRPOP 完整文法驱动**；否决 tree-sitter 与"保留 Pratt 查表化" | tree-sitter 产 CST 不产领域 AST，仍需一层完整映射代码；查表化只解决同步性、解决不了可维护性，是不充分的中间站。LALRPOP 的等价性风险由**双解析器差分**消解（见上），不靠论证 |
| `const_data::BinOp` 归属 | 归本文 | 它是"加一个运算符"改动面的直接组成部分；与 `ast::Type` 收敛（03）分开处理 |

### 关于"做完整"这件事的说明

查表化是一个不充分的中间站：它解决同步性，不解决可维护性；而"等判据放宽后重新评估"是消极等待，不是工程决策。因此本文采用**完整文法驱动**，等价性由双解析器差分实测保证（见上），诊断层不做任何放宽。

**这个转变的代价必须说清**：

- 施工量显著上升：需建文法文件、搬 22 个 `Expr` 变体的构造动作、归并约 89 处 BP 引用、重写错误恢复
- **前置条件变硬**：629 行前端测试必须先复活（09 的 P1），AST 规范化快照必须先入库（09 的 P2）。没有这两样，步骤 2 的差分无从做起
- **引入一个新依赖**（LALRPOP），与词法层"零新增依赖"的取向不一致

**收益是确定的**："加一个运算符"从 11 处降到 2 处，BP 冲突从"偶然正确"变成"文法构建期报错"，`Expr::Lambda` 借用、`parse_expression(12)` 魔数、约 120 行死旧语法探测**自然消失**。

### 被否决或降级的方案

**A. 只删死代码，不做范式变更。** 已核实可行，但不足以解决问题。死代码清理（约 130 行 + 3 处魔数）可以在**零判据成本**下完成，且应立即做。但它对"加一个运算符改 11 处"的影响是：L2 的改动面**一处都不会减少**；跨枚举漏改**完全未被触及**。**结论：采纳为阶段 0 与阶段 1，但必须继续做文法驱动。把 A 当作完整方案是错误的自我安慰。**

**B. 保留 Pratt，只加测试。** **未采纳为完整方案，但采纳为前置。** 测试**不能发现跨枚举漏改**——两个枚举各自都能编译通过，测试通过与否与它们是否同步无关。**结论：作为阶段 1 采纳；作为完整方案否决。**

**C. 保留 Pratt 并查表化。** **已否决**（2026-10-03）。它是"完全手写"与"完整文法"之间的不必要中间站：`Expr` 的 22 个变体仍要手写 22 个构造动作，`Expr::Lambda { body: 空 Block }` 的借用问题、`parse_expression(12)` 的魔数、两套阶梯归并的 89 处引用，全部仍在。

**D. 采用 tree-sitter。** **否决。** tree-sitter 的增量解析能力对本项目（每次全量编译、293 个语料规模）没有实质收益；且产 CST 不产领域 AST，仍需一层完整映射代码，等于在 `nud.rs` 之外净增同样体量。

**E. 采用 `logos` 替换手写词法器。** **有条件未采纳。** **重新评估的前置条件**：阶段 2 完成（转义解码已收敛为唯一实现）后，把该实现替换为 `logos` 回调的工作量会大幅下降。

### 方案的净收益

- **改动面从"十几处逻辑代码"降到"1-2 处声明数据 + 7-8 处生成"。** 且生成的 7-8 处由编译器保证同步，漏改不再是可能事件。
- **消灭一类编译器永远抓不到的缺陷。** 跨枚举漏改（`ast::BinOp` vs `const_data::BinOp`）是当前唯一**没有任何工具能发现**的缺陷类型，因为两个类型彼此独立。同源生成直接消灭它。
- **把 BP 碰撞从"偶然正确"变成"编译失败"。** 现有四处跨套碰撞与两处套内自撞目前靠 `bp_right = bp_left + 1` 的未写明约定侥幸不炸；层内唯一性断言让这类问题此后不可能沉默存在。
- **前端测试从"1 个文件在跑"变成"全部在跑"。** 629 行既有测试复活后，阶段 2-4 才有安全网；且这批测试**本来就存在**，只是一行 `mod` 没写。
- **阶段 0 与阶段 1 零风险、零判据成本。** 死代码删除与测试复活可以直接先做，不阻塞任何其他工作。

## 已知局限与风险

### 设计上的风险

- **运算符表是新的"必须维护的东西"。** 它把"改 11 处"变成"改 1 处 + 维护一张表"。**若表本身的字段设计不当，会退化成新的集中式维护负担**。这是本方案的主要风险，缓解手段是让表的每一列都由编译期断言保护。
- **BP 编号重排的风险不对称。** 两套阶梯合并涉及约 89 处生产引用的改名，若某处遗漏，**结合性会静默改变**（不报错，只是表达式解析结果不同）。这是本文最可能引入的静默回归，必须靠阶段 1 的层对结合性用例兜住。
- **阶段 2-4 的 C5 判据成本很高。** 三层判据全用意味着每阶段都要跑全量语料（293 个 `.yx`）+ AST 快照 + 诊断比对。这会使阶段 2-4 的每次提交都较重。
- **f-string 的 span 变更可能无法完全避免。** 见"向后兼容性"，可能需要接受一处已知行为变更并更新基线。

### 未决问题

> **本节原列的开放问题已全部裁决。** 逐条决定见 [RFC-039 决议登记](../../rfc/accepted/039-compiler-architecture.md)（D1–D50）。**本文不留任何待定项。**
>
## 核实记录

本文所有代码事实均经**直接读盘**核实。核实方法：`read` 工具直读目标文件行区间；行数用 `Get-Content` 计数；引用面用 `grep` 工具全仓扫描并按生产 / 测试 / 孤儿三类分别归类。**未使用 `cargo test` / `cargo build`**——本文为纯文档，判据执行属于实施阶段。

### 逐项实测结果

| 核实项 | 实测结果 |
| --- | --- |
| `literals.rs` 总行数 | **1568** |
| `tokenizer.rs` 总行数 | **554** |
| `next_token_inner` 行号 | **`:218`** |
| 四个基数扫描器行段 | `77-161`（85 行）/ `164-247`（84）/ `250-333`（84）/ `336-516`（181）——与本文表一致 |
| 三处转义解码 | `694-827`（134 行）/ `1015-1141`（127 行）/ `push_fstring_escape:1374`——与本文表一致 |
| `scan_leading_dot` | `519-635`，**117 行** |
| `Expr` 变体数 | **22**（含 `Error`），`ast.rs:16-163` |
| `lexer/tests/` 文件数 | **14**（含 `mod.rs`） |
| 孤儿真实测试行数 | **629**（222+167+89+81+70）；另有 90 行 `fstring.rs` 已通过 `#[path]` 接线运行，不计入孤儿 |
| 复活 `lexer/tests/` 所需改动 | **两处**：`lexer/mod.rs` 加 `mod tests;` **且** `tests/mod.rs` 加 `mod lexer_mod;` `mod symbols;`。只加一处则后 159 行仍不运行 |
| `precedence.rs` 死代码 | **96 行**（`Precedence` 枚举 `35-94` + `PrecedenceContext` `98-133`）生产零使用**成立**；但唯一外部引用者 `precedence_inline.rs`（95 行 / 6 test）自身是孤儿——**删除死代码必须同时删该文件** |
| 7 个空壳行数 | 各 1-3 行（`debug_lexer.rs` 1 行，其余 3 行） |
| 空行残留形态 | **每个字段间插 3 个连续空行，跨 11 行**（`777-787` / `1098-1108`） |
| `pratt/tests/precedence.rs` 行数 | **13**，唯一断言 `assert!(bp_lowest < bp_highest)` |
| "跳过配对括号"第 4 处 | `224-233` |
| 下游改动面 | **17 个生产文件**匹配 `BinOp::`（全仓 **534 次 / 47 文件**） |
| `src/backends/` 是否为 `BinOp` 消费者 | **否**——`src/backends/` 对 `BinOp` 与 `ast::` **零命中**；解释器消费 IR / 字节码，从不见 AST |
| "漏改没有任何检查能发现" | **需分层**：穷尽 `match` 漏改**会被编译器发现**；静默的是 `matches!` 宏、`_` 兜底、以及**跨枚举**三类。本文已按实测分层表述 |
| `const_data.rs` 的 `matches!` 块 | `263-288`（`is_arith` / `is_comparison` / `is_logical` / `is_bitwise` 四函数区间） |
| `impl Display for BinOp` | `291` 起，18 臂穷尽匹配，无 `_` 兜底 |
| `parse_assign_after_target` 中两函数 | **不是死代码**：两函数仍在 `declarations.rs:719` / `:725` 被调用，是运行中的"拒绝已移除语法"门禁；且 `skip_old_function_syntax`（`120-122`，函数体仅一行注释）调用后**不消费任何 token**（`:725-726` 直接 `return None`） |
| 第二枚 `BinOp` 枚举 | `const_data.rs:234` 确实存在，与 `ast::BinOp` **无任何 `From` 转换**，迫使 `const_eval.rs:19` / `ir_gen.rs:2317` 等文件在 import 处起别名 |
| `lexer/tests/mod.rs` 再导出块 | 存在（38 行文件），`:28-38` 带 `pub use ... ::*;`，是空壳得以伪装成覆盖的直接原因 |

### 本轮复核实测修正的三处

以下三点与"第一套 BP 阶梯已死"的直觉不符，按实测为准：

1. **两套阶梯是交织的，不是"一套活一套死"。** `infix_info`（`led.rs:30-88`）自身就同时引用第一套的 5 个常量（`BP_ASSIGN:33`、`BP_CALL:75/77/79/83`、`BP_CAST:81`）与第二套的 8 个。第一套 12 个常量中**只有 6 个真正零引用**（`BP_LOGICAL_OR` / `BP_LOGICAL_AND` / `BP_EQUALITY` / `BP_TERM` / `BP_FACTOR`，外加仅存于 `declarations.rs:742` 注释的 `BP_COMPARISON`）；另外 6 个（`BP_LOWEST` / `BP_ASSIGN` / `BP_UNARY` / `BP_CALL` / `BP_CAST` / `BP_HIGHEST`）在生产代码中合计约 **89 处引用、跨 8 个文件**。因此"删除第一套 BP 常量"不是 12 行 C6 删除，而是 C5 范畴的跨文件重命名。
2. **第二套自己也有死成员。** `BP_RANGE`（`precedence.rs:22`）在生产代码中零使用——`..` 在 `led.rs:40` 用的是裸字面量 `(6, 7)`，`BP_RANGE` 仅出现在 `led.rs:157-159` 的注释里。
3. **那句"自相矛盾的注释"在 `precedence.rs:33`，不是 `:11`。** `*:11` 是 `pub const BP_COMPARISON: u8 = 5;`。该注释（*"Precedence rules for the Pratt parser"*）描述的是零引用的 `Precedence` 枚举。

附带修正：`bp_right = bp_left + 1` 产生的是**左结合**（`a - b - c` → `(a-b)-c`），而非右结合；`led.rs:85` 的 `(11, 1)` 是唯一显式的右操作数贪婪吞并写法。碰撞也不止跨套两处，另有 `BP_UNARY=8 == BP_CAST=8`（第一套内自撞，两者都活着）与 `BP_RANGE=7 == BP_ADD=7`（第二套内自撞）。

### 未核实事项

正文中已逐处标注「未核实」：

- `pratt/nud.rs` 的 `prefix_info` 表体（`26-75`）的具体臂数与逐条绑定力
- `declarations.rs:136-642` 中 8 个职责段落各自的精确行边界
- `const_data::BinOp`（`const_data.rs:234`）的完整变体集
- `logos` / `regex` 是否已在 `Cargo.toml` 中（作为传递依赖）
- `parser_state.rs:170-186` 之外 `synchronize()` 的全部调用点
- 本项目是否为已发布的库（影响删除公开面的风险评估）

## 参见

### 同批附属设计文档

- [01-routing.md](01-routing.md) — 加一个特性该改哪里（功能路由表 A/B/C、依赖方向规范）
- [03-type-unification.md](03-type-unification.md) — `ast::Type` 收敛归属，与本文的接口边界
- [04-ssa.md](04-ssa.md) — 同为 C4/C5 判据使用者
- [06-cleanup-inventory.md](06-cleanup-inventory.md) — 8 棵孤儿测试树的完整清单
- [07-equivalence-oracle.md](07-equivalence-oracle.md) — C5/C6 类别定义、三层判据、回归门禁

### RFC 正文与相关提案

- [RFC-039 编译器架构重构](../../rfc/accepted/039-compiler-architecture.md) — 上位总纲；四层模型、术语表、功能路由表 A/B/C
- [RFC-013 错误码规范](../../rfc/accepted/013-error-code-specification.md) — `build.rs` 生成期门禁，本文借鉴的门禁范例
- [RFC-010 统一类型语法](../../rfc/accepted/010-unified-type-syntax.md) — `ast::Type` 的来源

### 代码位置索引

- `src/frontend/core/lexer/literals.rs:77-161` / `164-247` / `250-333` / `336-516` — 四个基数扫描器
- `src/frontend/core/lexer/literals.rs:694-827` / `1015-1141` / `1374` — 三处转义解码实现
- `src/frontend/core/lexer/literals.rs:519-635` — `scan_leading_dot`（零覆盖）
- `src/frontend/core/lexer/mod.rs:104-106` — 词法层唯一的测试声明
- `src/frontend/core/lexer/tests/mod.rs:15-25` / `28-38` — 子模块声明与 `pub use` 再导出块
- `src/frontend/core/parser/pratt/nud.rs:441-446` — f-string 插值的运行时嵌套 `tokenize()`
- `src/frontend/core/parser/pratt/nud.rs:896-901` / `1041` — 魔数 `12` 的来源自述与使用点
- `src/frontend/core/parser/pratt/precedence.rs:6-17` 与 `22-31` — 两套 BP 阶梯
- `src/frontend/core/parser/pratt/precedence.rs:33` / `35-94` / `98-133` — 脱节的注释、零引用的枚举与结构体
- `src/frontend/core/parser/pratt/led.rs:30-88` — `infix_info` 手写表
- `src/frontend/core/parser/pratt/tests/mod.rs:4-6` — 缺少 `precedence_inline` 声明的孤儿接线
- `src/frontend/core/parser/statements/declarations.rs:719-727` — 旧语法拒绝门禁及其"跳过但不消费"缺陷
- `src/frontend/core/types/const_data.rs:234` — 与 `ast::BinOp` 平行的第二个运算符枚举
- `build.rs:19-55` — 错误码生成期门禁，本仓库现有的强制机制范例
- `src/middle/core/tests/bytecode.rs:369-657` — `test_every_opcode_roundtrips_not_silently_nop`，本仓库现有的"逐项往返"判据范例
