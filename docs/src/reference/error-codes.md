# 错误码参考

YaoXiang 编译器使用错误码标识不同类型的诊断信息。错误码按编号范围分组，每个错误码对应一个特定的错误场景。

---

## E0xxx -- 词法和语法分析

词法分析器（Lexer）和语法分析器（Parser）阶段产生的错误。

| 错误码 | 模板 | 说明 |
|--------|------|------|
| E0001 | `Invalid character: '{char}'` | 无效字符 |
| E0002 | `Invalid number literal: '{literal}'` | 无效数字字面量 |
| E0003 | `Unterminated string starting at line {line}` | 未终止的字符串 |
| E0004 | `Invalid character literal: '{literal}'` | 无效字符字面量 |
| E0010 | `Expected {expected}, found {found}` | 期望的令牌 |
| E0011 | `Unexpected token: '{token}'` | 意外的令牌 |
| E0012 | `Invalid syntax: {reason}` | 无效语法 |
| E0013 | `Mismatched {bracket_type}: opened at line {open_line}, column {open_col}, not closed` | 不匹配的括号 |
| E0014 | `Missing semicolon after {statement}` | 缺少分号 |

## E1xxx -- 类型检查

类型检查阶段产生的错误，涵盖变量类型、函数调用、模式匹配、泛型实例化、并发语义和错误传播等。

| 错误码 | 模板 | 说明 |
|--------|------|------|
| E1001 | `Unknown variable: '{name}'` | 未知变量 |
| E1002 | `Expected type '{expected}', found type '{found}'` | 类型不匹配 |
| E1003 | `Unknown type: '{type}'` | 未知类型 |
| E1010 | `Function '{func}' expects {expected} arguments, found {found}` | 参数数量不匹配 |
| E1011 | `Parameter type mismatch: expected '{expected}', found '{found}'` | 参数类型不匹配 |
| E1012 | `Return type mismatch: expected '{expected}', found '{found}'` | 返回类型不匹配 |
| E1013 | `Function not found: '{func}'` | 函数未找到 |
| E1020 | `Cannot infer type for '{expr}'` | 无法推断类型 |
| E1021 | `Type inference conflict: {reason}` | 类型推断冲突 |
| E1030 | `Pattern non-exhaustive: missing patterns {patterns}` | 模式不完整 |
| E1031 | `Unreachable pattern: '{pattern}'` | 不可达模式 |
| E1040 | `Operation '{op}' is not supported for type '{type}'` | 操作不支持 |
| E1041 | `Index out of bounds: valid range is 0..{max}, found {index}` | 索引越界 |
| E1042 | `Field '{field}' not found in struct '{struct}'` | 字段未找到 |
| E1050 | `Logical operation requires boolean operands, found '{left}' and '{right}'` | 需要布尔操作数 |
| E1051 | `Logical NOT requires boolean operand, found '{type}'` | 逻辑 NOT 需要布尔操作数 |
| E1052 | `Cannot dereference type '{type}', expected pointer type` | 无效解引用 |
| E1053 | `Cannot access field on non-struct type '{type}'` | 非结构体字段访问 |
| E1054 | `Condition must be boolean, found '{type}'` | 条件类型不匹配 |
| E1055 | `Constraint type '{type}' can only be used in generic context` | 约束在非泛型上下文中 |
| E1060 | `Expected {expected} type argument(s), found {found}` | 类型参数数量不匹配 |
| E1061 | `Cannot instantiate generic type with given arguments` | 无法实例化泛型 |
| E1070 | `Unknown label: '{label}'` | 未知标签 |
| E1080 | `` `spawn` is only allowed inside @block scope (current: @{mode}) `` | spawn 仅允许在 @block 作用域内使用 |
| E1081 | `` `?` is only allowed inside functions returning Result `` | `?` 仅允许在返回 Result 的函数内使用 |
| E1082 | `` `?` requires a Result expression, found '{type}' `` | `?` 只能用于 Result 表达式 |
| E1083 | `` Result error type mismatch for `?`: expected '{expected}', found '{found}' `` | `?` 的错误类型不匹配 |
| E1090 | `Type: Type = Type` | 不可言说（彩蛋） |
| E1091 | `Generic meta-type self-reference is not allowed: '{decl}'` | 无效的泛型元类型 |

## E2xxx -- 语义分析

语义分析阶段产生的错误，涵盖作用域、变量生命周期、所有权和函数签名解析等。

| 错误码 | 模板 | 说明 |
|--------|------|------|
| E2001 | `Variable '{name}' is not in scope` | 作用域错误 |
| E2002 | `Duplicate definition: '{name}' is already defined in this scope` | 重复定义 |
| E2003 | `Ownership constraint violated: {reason}` | 所有权错误 |
| E2010 | `Cannot assign to immutable variable '{name}'` | 不可变赋值 |
| E2011 | `Use of uninitialized variable '{name}'` | 使用未初始化变量 |
| E2012 | `Mutability conflict: cannot use mutable reference in immutable context` | 可变性冲突 |
| E2013 | `Cannot shadow existing variable '{name}'` | 变量遮蔽 |
| E2014 | `Function calls are not allowed in top-level variable initializers` | 顶层变量不支持函数调用 |
| E2090 | `Invalid signature: {reason}` | 无效签名 |
| E2091 | `Invalid signature: unknown type '{type_name}'` | 签名未知类型 |
| E2092 | `Invalid signature: missing '->'` | 签名缺少箭头 |
| E2093 | `Invalid signature: duplicate parameter '{name}'` | 重复参数名 |
| E2094 | `Invalid signature: generic '{name}' shadows outer generic` | 泛型参数遮蔽 |
| E2095 | `Invalid signature: parameter '{name}' shadows generic` | 参数名遮蔽泛型 |

## E4xxx -- 泛型与特质

泛型约束和特质系统相关错误。

| 错误码 | 模板 | 说明 |
|--------|------|------|
| E4001 | `Type '{type}' does not satisfy the trait bound '{trait}'` | 泛型约束违反 |
| E4002 | `Trait '{trait}' not found` | 特质未找到 |
| E4003 | `Missing implementation for trait '{trait}' for type '{type}'` | 特质实现缺失 |
| E4004 | `Conflicting trait implementations for '{trait}'` | 特质实现冲突 |
| E4005 | `Associated type '{assoc_type}' not found in '{container}'` | 关联类型未找到 |

## E5xxx -- 模块与导入

模块系统和导入相关错误。

| 错误码 | 模板 | 说明 |
|--------|------|------|
| E5001 | `Module '{module}' not found` | 模块未找到 |
| E5002 | `Failed to import module '{module}': {reason}` | 导入错误 |
| E5003 | `Export '{export}' not found in module '{module}'` | 导出未找到 |
| E5004 | `Circular dependency detected: {path}` | 循环依赖 |
| E5005 | `Invalid module path: '{path}'` | 无效的模块路径 |
| E5006 | `Duplicate import: '{name}' is already imported` | 重复导入 |
| E5007 | `Module '{module}' exports: {available}` | 模块导出提示 |

## E6xxx -- 运行时

运行时阶段产生的错误。

| 错误码 | 模板 | 说明 |
|--------|------|------|
| E6001 | `Division by zero in expression: {expr}` | 除零错误 |
| E6002 | `Null pointer dereference at {location}` | 空指针解引用 |
| E6003 | `Array index out of bounds: valid range is 0..{max}, found {index}` | 数组索引越界 |
| E6004 | `Stack overflow: recursion depth exceeded limit {limit}` | 栈溢出 |
| E6005 | `Assertion failed: {condition}` | 断言失败 |
| E6006 | `Function not found: '{func}'` | 函数未找到（运行时） |
| E6007 | `Runtime error: {message}` | 运行时错误 |

## E7xxx -- I/O 与系统

I/O 操作和系统级错误。

| 错误码 | 模板 | 说明 |
|--------|------|------|
| E7001 | `File not found: '{path}'` | 文件未找到 |
| E7002 | `Permission denied: '{path}'` | 权限被拒绝 |
| E7003 | `I/O error: {reason}` | I/O 错误 |
| E7004 | `Network error: {reason}` | 网络错误 |

## E8xxx -- 内部编译器错误

编译器内部错误，通常表示编译器本身的 bug。遇到此类错误请在 [GitHub Issues](https://github.com/yaoxiang/yaoxiang/issues) 报告。

| 错误码 | 模板 | 说明 |
|--------|------|------|
| E8001 | `Internal compiler error: {message}` | 内部编译器错误 |
| E8002 | `Unexpected compiler panic: {reason}` | 意外 Panic |
| E8003 | `Compiler phase error: {phase} - {message}` | 编译器阶段错误 |

## W1xxx -- 警告

死代码检测相关警告。警告不会阻止编译，但表示代码中可能存在的问题。

| 错误码 | 模板 | 说明 |
|--------|------|------|
| W1001 | `Unused exported function: '{name}'` | 未使用的导出函数 |
| W1002 | `Unused exported type: '{name}'` | 未使用的导出类型 |
| W1003 | `Unused import: '{name}'` | 未使用的导入 |
| W1004 | `Unused exported variable: '{name}'` | 未使用的导出变量 |
| W1005 | `Unused exported method: '{name}'` | 未使用的导出方法 |

---

共计 **83** 个诊断码（78 个错误码 + 5 个警告码）。
