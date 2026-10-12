---
title: '错误码'
description: 'YaoXiang 编译器与标准库的全部诊断码索引'
---

# 错误码参考

 本页由 `scripts/docs/gen-error-code-docs.py` 从 `src/util/diagnostic/codes/` 的 `define_codes!` 注册表与 `locales/zh.json` 生成，请勿手工编辑。改文案请改 `locales/*.json`，改码表请改 `define_codes!`，然后重跑 `python scripts/docs/gen-error-code-docs.py`。

YaoXiang 的每个诊断都带一个稳定码号（如 `E1001`），便于检索、在 issue 中引用、以及在 CI 里按码过滤。诊断的用户可见文案（标题 / 模板 / 帮助）由 `locales/*.json` 提供，可用 `yx explain <码号>` 在终端查看。

## 码号体系

码号由前缀（族）与四位序号组成，族即编译器阶段：

| 族前缀 | 类别 | 含义 | 注册码数 |
| --- | --- | --- | --- |
| `E0xxx` | `Lexer` / `Parser` | 词法与语法分析错误 | 11 |
| `E1xxx` | `TypeCheck` | 类型检查错误 | 53 |
| `E2xxx` | `Semantic` | 语义分析错误 | 23 |
| `E3xxx` | `Codegen` | 代码生成错误（含 IR 层一致性检查） | 13 |
| `E4xxx` | `Generic` | 泛型、特质与常量求值错误 | 14 |
| `E5xxx` | `Module` | 模块与导入错误 | 7 |
| `E6xxx` | `Runtime` | 运行时错误（VM 与 std 原生函数） | 7 |
| `E7xxx` | `Io` | I/O 与系统错误 | 4 |
| `E8xxx` | `Internal` | 内部编译器错误（编译器自身的缺陷） | 3 |
| `W1xxx` | `Warning` | 警告（不阻止编译） | 8 |

`E3xxx` 族此前未在任何文档中列出，现已并入本页；其码在 IR 生成期与入口点检查时产生。

## 统计

- `define_codes!` 注册表：**143** 项（135 个 `E` 码 + 8 个 `W` 码）。
- std 层运行时码：**5** 个（不在 `define_codes!` 注册表内，见下节）。
- 合计 **148** 个诊断码。
- 其中 **35** 个注册码当前在非测试代码里没有发射点，标注为「暂未发射」——它们的码号仍然保留，但用户暂时无法触发。

## 速查：按码号前缀检索

| 族前缀 | 类别 | 含义 | 详情 |
| --- | --- | --- | --- |
| `E0xxx` | `Lexer / Parser` | 词法与语法分析错误 | 分族页：[`E0xxx`](E0xxx.md) |
| `E1xxx` | `TypeCheck` | 类型检查错误 | 分族页：[`E1xxx`](E1xxx.md) |
| `E2xxx` | `Semantic` | 语义分析错误 | 分族页：[`E2xxx`](E2xxx.md) |
| `E4xxx` | `Generic` | 泛型与特质错误 | 分族页：[`E4xxx`](E4xxx.md) |
| `E5xxx` | `Module` | 模块与导入错误 | 分族页：[`E5xxx`](E5xxx.md) |
| `E6xxx` | `Runtime` | 运行时错误 | 分族页：[`E6xxx`](E6xxx.md) |
| `E7xxx` | `Io` | I/O 与系统错误 | 分族页：[`E7xxx`](E7xxx.md) |
| `E8xxx` | `Internal` | 内部编译器错误 | 分族页：[`E8xxx`](E8xxx.md) |
| `W1xxx` | `Warning` | 警告 | [`W1xxx`](../warning-code/warning-codes.md) |

`E3xxx` 族暂无独立分族页，全部码列在下方全量清单中。

## 全量码清单

「模板」列取自 `locales/zh.json`，即诊断正文里替换参数后的形态；「发射点」指该码在非测试代码中的实际调用处数量。

| 码号 | 中文名 | 类别 | span 豁免 | 模板 | 发射点 |
| --- | --- | --- | --- | --- | --- |
| `E0001` | 无效字符 | `Lexer` | 否 | `无效字符：'{char}'` | ✅ 2 处 |
| `E0002` | 无效数字字面量 | `Lexer` | 否 | `无效数字字面量：'{literal}'` | ✅ 1 处 |
| `E0003` | 未终止的字符串 | `Lexer` | 否 | `字符串从第 {line} 行开始未终止` | ⚠ 暂未发射 |
| `E0004` | 无效字符字面量 | `Lexer` | 否 | `无效字符字面量：'{literal}'` | ⚠ 暂未发射 |
| `E0010` | 期望的令牌 | `Parser` | 否 | `期望 {expected}，找到 {found}` | ✅ 22 处 |
| `E0011` | 意外的令牌 | `Parser` | 否 | `意外的令牌：'{token}'` | ✅ 18 处 |
| `E0012` | 无效语法 | `Parser` | 否 | `无效语法：{reason}` | ✅ 13 处 |
| `E0013` | 不匹配的括号 | `Parser` | 否 | `不匹配的 {bracket_type}：在第 {open_line} 行、第 {open_col} 列打开，未闭合` | ⚠ 暂未发射 |
| `E0014` | 缺少分号 | `Parser` | 否 | `{statement} 后缺少分号` | ⚠ 暂未发射 |
| `E0016` | 期望表达式 | `Parser` | 否 | `期望表达式：{context}` | ✅ 4 处 |
| `E0018` | 关键字作名称 | `Parser` | 否 | `'{keyword}' 是关键字，不能用作名称` | ✅ 1 处 |
| `E1001` | 未知变量 | `TypeCheck` | 否 | `未知变量：'{name}'` | ✅ 4 处 |
| `E1002` | 类型不匹配 | `TypeCheck` | 否 | `期望类型 '{expected}'，实际类型 '{found}'` | ✅ 83 处 |
| `E1003` | 未知类型 | `TypeCheck` | 否 | `未知类型：'{type}'` | ✅ 3 处 |
| `E1010` | 参数数量不匹配 | `TypeCheck` | 否 | `函数 '{func}' 期望 {expected} 个参数，找到 {found} 个` | ✅ 18 处 |
| `E1011` | 参数类型不匹配 | `TypeCheck` | 否 | `参数类型不匹配：期望 '{expected}'，实际 '{found}'` | ⚠ 暂未发射 |
| `E1012` | 返回类型不匹配 | `TypeCheck` | 否 | `返回类型不匹配：期望 '{expected}'，实际 '{found}'` | ✅ 1 处 |
| `E1013` | 函数未找到 | `TypeCheck` | 否 | `函数未找到：'{func}'` | ✅ 5 处 |
| `E1014` | 命名参数名未知 | `TypeCheck` | 否 | `函数 '{func}' 没有名为 '{name}' 的参数（可用：{params}）` | ✅ 2 处 |
| `E1015` | 参数重复指定 | `TypeCheck` | 否 | `函数 '{func}' 的参数 '{name}' 被重复指定` | ✅ 2 处 |
| `E1020` | 无法推断类型 | `TypeCheck` | 否 | `无法推断 '{expr}' 的类型` | ⚠ 暂未发射 |
| `E1021` | 类型推断冲突 | `TypeCheck` | 否 | `类型推断冲突：{reason}` | ⚠ 暂未发射 |
| `E1030` | 模式不完整 | `TypeCheck` | 否 | `模式不完整：缺少 {patterns}` | ✅ 2 处 |
| `E1031` | 不可达模式 | `TypeCheck` | 否 | `不可达模式：'{pattern}'` | ✅ 3 处 |
| `E1032` | 模式重复绑定 | `TypeCheck` | 否 | `模式重复绑定：'{name}' 在同一模式中出现多次` | ✅ 2 处 |
| `E1033` | 或模式绑定不一致 | `TypeCheck` | 否 | `或模式绑定不一致：\`\|\` 两侧必须绑定相同的名字集` | ✅ 1 处 |
| `E1034` | 结构体模式缺字段 | `TypeCheck` | 否 | `结构体模式缺少字段 '{field}'（'{struct}' 的字段须完整覆盖）` | ✅ 1 处 |
| `E1040` | 操作不支持 | `TypeCheck` | 否 | `类型 '{type}' 不支持操作 '{op}'` | ⚠ 暂未发射 |
| `E1041` | 索引越界 | `TypeCheck` | 否 | `索引越界：有效范围是 0..{max}，找到 {index}` | ✅ 5 处 |
| `E1042` | 字段未找到 | `TypeCheck` | 否 | `在结构体 '{struct}' 中找不到字段 '{field}'` | ✅ 9 处 |
| `E1043` | 模块成员未找到 | `TypeCheck` | 否 | `模块 '{module}' 没有导出 '{name}'` | ✅ 1 处 |
| `E1050` | 需要布尔操作数 | `TypeCheck` | 否 | `逻辑运算需要布尔操作数，实际为 '{left}' 和 '{right}'` | ✅ 1 处 |
| `E1051` | 逻辑 NOT 需要布尔操作数 | `TypeCheck` | 否 | `逻辑 NOT 需要布尔操作数，实际为 '{type}'` | ✅ 1 处 |
| `E1052` | 无效解引用 | `TypeCheck` | 否 | `无法解引用类型 '{type}'，期望指针类型` | ✅ 1 处 |
| `E1053` | 非结构体字段访问 | `TypeCheck` | 否 | `无法在非结构体类型 '{type}' 上访问字段` | ✅ 2 处 |
| `E1054` | 条件类型不匹配 | `TypeCheck` | 否 | `条件必须是布尔类型，实际为 '{type}'` | ✅ 3 处 |
| `E1055` | 约束在非泛型上下文中 | `TypeCheck` | 否 | `约束类型 '{type}' 只能在泛型上下文中使用` | ⚠ 暂未发射 |
| `E1060` | 类型参数数量不匹配 | `TypeCheck` | 否 | `期望 {expected} 个类型参数，实际 {found} 个` | ⚠ 暂未发射 |
| `E1061` | 无法实例化泛型 | `TypeCheck` | 否 | `无法用给定参数实例化泛型类型` | ⚠ 暂未发射 |
| `E1062` | const 泛型约束失败 | `TypeCheck` | 否 | `const 泛型约束失败: \`{constraint}\` 不成立` | ✅ 1 处 |
| `E1064` | 绑定位置索引无效 | `TypeCheck` | 否 | `绑定位置索引无效：{positions}（函数共 {total} 个参数）` | ✅ 1 处 |
| `E1065` | 对非函数值调用 | `TypeCheck` | 否 | `类型 \`{type}\` 不可调用——它不是函数` | ✅ 1 处 |
| `E1071` | 类型定义只能在模块级 | `TypeCheck` | 否 | `类型定义 '{name}' 只能在模块级（模块顶层）` | ✅ 2 处 |
| `E1081` | `?` 仅允许在返回可传播类型的函数内使用 | `TypeCheck` | 否 | `\`?\` 仅允许在返回实现 \`Try\` 的类型的函数内使用` | ✅ 2 处 |
| `E1082` | `?` 只能用于实现 Try 的类型 | `TypeCheck` | 否 | `'{type}' 未实现 \`Try\` 接口，不能使用 \`?\`` | ✅ 1 处 |
| `E1083` | `?` 的错误类型不匹配 | `TypeCheck` | 否 | `\`?\` 的错误类型不匹配：期望 '{expected}'，实际 '{found}'` | ✅ 1 处 |
| `E1090` | ✨ 不可言说 ✨ | `TypeCheck` | 否 | `Type: Type = Type` | ⚠ 暂未发射 |
| `E1091` | 无效的泛型元类型 | `TypeCheck` | 否 | `泛型元类型自引用不允许：'{decl}'` | ✅ 1 处 |
| `E1092` | 精化类型实参形态非法 | `TypeCheck` | 否 | `'{name}' 要求编译期常量实参，但所给实参无法转换` | ✅ 3 处 |
| `E1093` | 精化实参个数不匹配 | `TypeCheck` | 否 | `'{name}' 期望 {expected} 个实参，实际 {found} 个` | ✅ 2 处 |
| `E1094` | 未使用的编译期值参数 | `TypeCheck` | 否 | `'{param}' 声明为 '{type}' 的编译期值参数，但未在类型体中被引用` | ✅ 1 处 |
| `E1095` | 未知接口 | `TypeCheck` | 否 | `未知接口：'{name}'（类型体引用的名字不是已注册的类型构造器）` | ✅ 1 处 |
| `E1096` | 接口参数数量不匹配 | `TypeCheck` | 否 | `接口 '{name}' 期望 {expected} 个类型参数，找到 {found} 个` | ✅ 3 处 |
| `E1097` | 接口成员命名冲突 | `TypeCheck` | 否 | `类型 '{type}' 的成员 '{member}' 与接口成员冲突（字段与方法共享命名空间，§1.2）` | ✅ 1 处 |
| `E1098` | 接口方法未实现 | `TypeCheck` | 否 | `类型 '{type}' 未实现接口 '{interface}' 的方法 '{method}'` | ✅ 1 处 |
| `E1099` | 接口方法签名不匹配 | `TypeCheck` | 否 | `类型 '{type}' 的方法 '{method}' 与接口签名不符：期望 {expected}，实际 {found}` | ✅ 2 处 |
| `E1100` | 接口方法重复实现 | `TypeCheck` | 否 | `类型 '{type}' 的方法 '{method}' 被重复实现（同签名覆盖被禁止，§3）` | ✅ 1 处 |
| `E1101` | 类型未实现接口 | `TypeCheck` | 否 | `类型 '{type}' 未实现接口 '{interface}'，不能进入该存在类型位置` | ✅ 5 处 |
| `E1102` | 循环控制语句出现在循环外 | `TypeCheck` | 否 | `'{keyword}' 出现在循环外` | ✅ 2 处 |
| `E1103` | 类型位置不能使用方括号 | `TypeCheck` | 是 | `'{name}[...]' 不是类型语法——类型实参要用圆括号` | ✅ 1 处 |
| `E1104` | 接口实现不在类型的定义模块 | `TypeCheck` | 否 | `类型 '{type}' 不由本模块定义，不能在本模块为其实现接口 '{interface}'` | ✅ 1 处 |
| `E1105` | 变体构造器不可作为字段访问 | `TypeCheck` | 否 | `'{variant}' 是和类型 '{type}' 的变体构造器，不是数据字段` | ✅ 1 处 |
| `E1106` | 约束未满足 | `TypeCheck` | 否 | `类型 '{type}' 未实现 \`{interface}\`——泛型参数 \`{param}\` 的约束 \`{param}: {interface}\` 在此调用点不成立` | ✅ 1 处 |
| `E1107` | 方法重载歧义 | `TypeCheck` | 否 | `方法重载歧义：'{key}' 有 {count} 个候选匹配且无法区分` | ✅ 1 处 |
| `E1108` | 空块落入容器期望位 | `TypeCheck` | 是 | `此处期望容器类型，但 \`{}\` 是空块（值 Void）` | ✅ 2 处 |
| `E2001` | 作用域错误 | `Semantic` | 否 | `变量 '{name}' 不在作用域中` | ⚠ 暂未发射 |
| `E2002` | 重复定义 | `Semantic` | 否 | `重复定义：'{name}' 已在当前作用域中定义` | ✅ 4 处 |
| `E2003` | 所有权错误 | `Semantic` | 否 | `所有权约束违反：{reason}` | ✅ 1 处 |
| `E2010` | 不可变赋值 | `Semantic` | 否 | `无法给不可变变量 '{name}' 赋值` | ✅ 2 处 |
| `E2011` | 使用未初始化变量 | `Semantic` | 否 | `使用未初始化的变量 '{name}'` | ⚠ 暂未发射 |
| `E2012` | 可变性冲突 | `Semantic` | 否 | `可变性冲突：无法在不可变上下文中使用可变引用` | ⚠ 暂未发射 |
| `E2013` | 变量遮蔽 | `Semantic` | 否 | `Cannot shadow existing variable '{name}'` | ✅ 3 处 |
| `E2014` | 使用已移动的值 | `Semantic` | 否 | `'{name}' 已被移动，无法再次使用` | ✅ 1 处 |
| `E2016` | 不可变赋值 | `Semantic` | 否 | `无法给不可变变量 '{name}' 赋值` | ✅ 1 处 |
| `E2018` | 可变/不可变借用冲突 | `Semantic` | 否 | `无法可变借用 '{name}'（已被不可变借用）` | ✅ 1 处 |
| `E2019` | 双重释放 | `Semantic` | 否 | `值 '{name}' 被释放了两次` | ✅ 1 处 |
| `E2020` | 释放后使用 | `Semantic` | 否 | `'{name}' 在释放后被使用` | ✅ 1 处 |
| `E2027` | unsafe 解引用 | `Semantic` | 否 | `无法在 unsafe 块外解引用裸指针` | ✅ 1 处 |
| `E2090` | 无效签名 | `Semantic` | 否 | `无效签名：{reason}` | ✅ 3 处 |
| `E2091` | 签名未知类型 | `Semantic` | 否 | `无效签名：未知类型 '{type_name}'` | ⚠ 暂未发射 |
| `E2092` | 签名缺少箭头 | `Semantic` | 否 | `无效签名：缺少 '->'` | ✅ 1 处 |
| `E2093` | 重复参数名 | `Semantic` | 否 | `无效签名：重复参数名 '{name}'` | ✅ 2 处 |
| `E2094` | 泛型参数遮蔽 | `Semantic` | 否 | `无效签名：泛型参数 '{name}' 遮蔽了外层泛型参数` | ⚠ 暂未发射 |
| `E2095` | 参数名遮蔽泛型 | `Semantic` | 否 | `无效签名：参数名 '{name}' 遮蔽了泛型参数` | ✅ 1 处 |
| `E2096` | 签名裸容器类型 | `Semantic` | 否 | `无效签名：容器类型 '{name}' 缺少类型实参` | ✅ 1 处 |
| `E2029` | spawn 内引用循环 | `Semantic` | 否 | `spawn 内引用循环: {cycle}` | ✅ 1 处 |
| `E2030` | 精化类型约束违反 | `Semantic` | 否 | `对 '{assigned}' 赋值后，'{var}' 不再满足精化类型约束：{constraint}（反例：{counterexample}）` | ✅ 1 处 |
| `E2031` | 精化约束无法证明 | `Semantic` | 否 | `对 '{assigned}' 赋值后，'{var}' 的精化类型约束：{constraint} 在证明内核内无法静态证明` | ✅ 3 处 |
| `E3004` | 不支持的迭代器 | `Codegen` | 否 | `不支持的迭代器类型：{iter_type}` | ✅ 2 处 |
| `E3005` | IR 生成错误 | `Codegen` | 否 | `IR 生成错误：{message}` | ✅ 19 处 |
| `E3006` | 未解析变量 | `Codegen` | 否 | `IR 生成内部错误：变量 '{name}' 未能解析` | ✅ 1 处 |
| `E3007` | 顶层绑定初始化必须为常量 | `Codegen` | 否 | `顶层绑定 '{name}' 的初始化必须是编译期常量表达式` | ⚠ 暂未发射 |
| `E3008` | 不支持的 match 模式 | `Codegen` | 否 | `match 模式 '{pattern}' 尚未实现：当前仅支持字面量与通配符（_）模式` | ✅ 3 处 |
| `E3018` | 单态化实例化失败 | `Codegen` | 否 | `泛型 '{func}' 实例化失败：{reason}` | ✅ 2 处 |
| `E3014` | 寄存器溢出 | `Codegen` | 是 | `寄存器溢出：{id} 超过限制 {limit}` | ✅ 4 处 |
| `E3017` | 无效操作数（代码生成） | `Codegen` | 否 | `代码生成：无效操作数：{reason}` | ✅ 3 处 |
| `E3019` | 顶层绑定循环依赖 | `Codegen` | 否 | `顶层绑定的初始化形成循环依赖：{cycle}` | ✅ 1 处 |
| `E3020` | 缺少程序入口 | `Codegen` | 否 | `可执行文件缺少 main 绑定（{path}）` | ✅ 2 处 |
| `E3021` | 入口不是函数 | `Codegen` | 否 | `程序入口 '{name}' 必须是函数，但它是值绑定` | ⚠ 暂未发射 |
| `E3022` | 入口 main 签名不符 | `Codegen` | 否 | `入口 '{name}' 的签名不符：运行期以零参调用，但它需要 '{expected}'，实际是 '{found}'` | ✅ 1 处 |
| `E3023` | 顶层不允许可执行语句 | `Codegen` | 是 | `Bin 角色（存在 yaoxiang.toml）下顶层不允许可执行语句——程序主体就是 main` | ✅ 1 处 |
| `E4001` | 泛型约束违反 | `Generic` | 否 | `类型 '{type}' 不满足特质约束 '{trait}'` | ✅ 1 处 |
| `E4002` | 特质未找到 | `Generic` | 否 | `特质 '{trait}' 未找到` | ⚠ 暂未发射 |
| `E4003` | 特质实现缺失 | `Generic` | 否 | `类型 '{type}' 缺少特质 '{trait}' 的实现` | ⚠ 暂未发射 |
| `E4004` | 特质实现冲突 | `Generic` | 否 | `特质 '{trait}' 的实现冲突` | ⚠ 暂未发射 |
| `E4005` | 关联类型未找到 | `Generic` | 否 | `在 '{container}' 中找不到关联类型 '{assoc_type}'` | ⚠ 暂未发射 |
| `E4010` | 常量除零 | `Generic` | 否 | `常量表达式除以零` | ✅ 3 处 |
| `E4011` | 常量溢出 | `Generic` | 否 | `常量表达式溢出` | ✅ 4 处 |
| `E4012` | 常量递归过深 | `Generic` | 否 | `常量求值超出最大递归深度 {limit}` | ✅ 1 处 |
| `E4014` | 常量求值失败 | `Generic` | 是 | `无法求值常量表达式：{reason}` | ✅ 10 处 |
| `E4018` | 精化谓词违反 | `Generic` | 是 | `精化谓词违反：{constraint}  反例： {counterexample}` | ✅ 6 处 |
| `E4019` | 类型等式不成立 | `Generic` | 否 | `类型等式不成立：期望 {expected}，实际 {found}` | ✅ 1 处 |
| `E4020` | 需要证明函数 | `Generic` | 否 | `需要证明函数来验证约束` | ⚠ 暂未发射 |
| `E4021` | 循环终止性无法自动证明 | `Generic` | 否 | `无法自动证明循环终止：未找到有效的递减度量` | ✅ 1 处 |
| `E4022` | 测度不成立 | `Generic` | 否 | `测度不成立：此调用点无法证明测度严格递减  反例： {counterexample}` | ✅ 1 处 |
| `E5001` | 模块未找到 | `Module` | 否 | `模块 '{module}' 未找到` | ✅ 1 处 |
| `E5002` | 导入错误 | `Module` | 否 | `导入模块 '{module}' 失败：{reason}` | ⚠ 暂未发射 |
| `E5003` | 导出未找到 | `Module` | 否 | `在模块 '{module}' 中找不到导出 '{export}'` | ✅ 1 处 |
| `E5004` | 循环依赖 | `Module` | 否 | `检测到循环依赖：{path}` | ⚠ 暂未发射 |
| `E5005` | 无效的模块路径 | `Module` | 否 | `无效的模块路径：'{path}'` | ⚠ 暂未发射 |
| `E5006` | 重复导入 | `Module` | 否 | `重复导入：'{name}' 已被导入` | ⚠ 暂未发射 |
| `E5007` | 模块导出 | `Module` | 否 | `模块 '{module}' 的导出：{available}` | ⚠ 暂未发射 |
| `E6001` | 除零错误 | `Runtime` | 是 | `表达式 {expr} 除以零` | ✅ 8 处 |
| `E6003` | 数组索引越界 | `Runtime` | 是 | `数组索引越界：有效范围是 0..{max}，找到 {index}` | ✅ 3 处 |
| `E6004` | 栈溢出 | `Runtime` | 是 | `栈溢出：递归深度超出限制 {limit}` | ✅ 3 处 |
| `E6005` | 断言失败 | `Runtime` | 是 | `断言失败：{condition}` | ✅ 3 处 |
| `E6006` | 函数未找到（运行时） | `Runtime` | 是 | `函数未找到：'{func}'` | ✅ 1 处 |
| `E6007` | 运行时错误 | `Runtime` | 是 | `运行时错误：{message}` | ✅ 5 处 |
| `E6008` | 键不存在 | `Runtime` | 是 | `键不存在：{key}` | ✅ 2 处 |
| `E7001` | 文件未找到 | `Io` | 否 | `文件未找到：'{path}'` | ⚠ 暂未发射 |
| `E7002` | 权限被拒绝 | `Io` | 否 | `权限被拒绝：'{path}'` | ⚠ 暂未发射 |
| `E7003` | I/O 错误 | `Io` | 否 | `I/O 错误：{reason}` | ⚠ 暂未发射 |
| `E7004` | 网络错误 | `Io` | 否 | `网络错误：{reason}` | ⚠ 暂未发射 |
| `E8001` | 内部编译器错误 | `Internal` | 是 | `内部编译器错误：{message}` | ✅ 27 处 |
| `E8002` | 意外 Panic | `Internal` | 否 | `意外编译器 panic：{reason}` | ⚠ 暂未发射 |
| `E8003` | 编译器阶段错误 | `Internal` | 否 | `编译器阶段错误：{phase} - {message}` | ⚠ 暂未发射 |
| `W1001` | 未使用的私有函数 | `Warning` | 否 | `未使用的函数：'{name}'` | ✅ 1 处 |
| `W1002` | 未使用的私有类型 | `Warning` | 否 | `未使用的类型：'{name}'` | ✅ 1 处 |
| `W1003` | 未使用的导入 | `Warning` | 否 | `未使用的导入：'{name}'` | ✅ 1 处 |
| `W1004` | 未使用的私有变量 | `Warning` | 否 | `未使用的变量：'{name}'` | ✅ 1 处 |
| `W1005` | 未使用的私有方法 | `Warning` | 否 | `未使用的方法：'{name}'` | ✅ 1 处 |
| `W1006` | 本地模块遮蔽依赖包 | `Warning` | 否 | `本地模块 '{module}' 遮蔽了依赖包 '{dependency}'` | ✅ 2 处 |
| `W1063` | const 泛型约束无法求值 | `Warning` | 否 | `const 泛型约束无法求值: \`{constraint}\`` | ✅ 1 处汇聚发射 |
| `W1081` | 终止性义务未判定 | `Warning` | 否 | `{count} 条终止性测度义务未判定（SMT 求解器不可用）` | ✅ 1 处汇聚发射 |
| `W1080` | 编译期证明降级 | `Warning` | 否 | `编译期无法证明约束，已降级为运行时检查` | ⚠ 暂未发射 |

## std 层运行时码

下列码**不在 `define_codes!` 注册表内**：它们由标准库原生函数在运行期经 `error_new("码号", …)` 构造，因此不出现在 `ErrorCodeDefinition::find()` 的查询结果里，但 `locales/*.json` 有对应文案，用户终端上仍可能看到。

| 码号 | 中文名 | 类别 | 模板 | 发射点 |
| --- | --- | --- | --- | --- |
| `E6009` | Range 步长非法 | std 运行时 | `Range 步长非法：step = {step}` | `src/std/range.rs:213`、`src/std/range.rs:344` 等 3 处 |
| `E6010` | 整数解析失败 | std 运行时 | `整数解析失败：{input}` | `src/std/result.rs:28`、`src/std/string.rs:551` |
| `E6011` | 浮点解析失败 | std 运行时 | `浮点解析失败：{input}` | `src/std/result.rs:29`、`src/std/string.rs:576` 等 3 处 |
| `E6012` | 码点非法 | std 运行时 | `码点非法：{input}` | `src/std/result.rs:30`、`src/std/string.rs:621` |
| `E6013` | JSON 解析失败 | std 运行时 | `JSON 解析失败：{detail}` | `src/std/json.yx:483`、`src/std/result.rs:31` |

## 标记说明

- **span 豁免**：为 `true` 时该诊断允许没有源码位置（内部错误、运行期诊断等）。非豁免码在既无显式 `.at()` 又无 walk 上下文时会被拒绝（debug 构建 panic，release 落 `E8001`）。
- **发射点 ⚠ 暂未发射**：该码在注册表中保留，但当前非测试代码里没有调用处，用户无法触发。码号不回收，避免旧产物里的诊断被复用成别的含义。
- **std 层运行时码**：见上一节，不参与注册表统计。

## 查看单个码

```bash
yx explain E1001
yx explain E1001 --json
```

## 在 Rust 代码中查询

```rust
use yaoxiang::util::diagnostic::{ErrorCodeDefinition, I18nRegistry};

let i18n = I18nRegistry::default();

if let Some(code) = ErrorCodeDefinition::find("E1001") {
    println!("Title: {}", i18n.get_title(&code));
    if let Some(help) = i18n.get_help(&code) {
        println!("Help: {}", help);
    }
}
```
