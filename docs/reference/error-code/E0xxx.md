# E0xxx：词法与语法分析

> 词法和解析阶段发生的错误。

## E0001：Invalid character

源代码包含非法字符。

```yaoxiang
x: Int = ¥100;  # ¥ 不是有效字符
```

```
error[E0001]: Invalid character
  --> example.yx:1:10
   |
 1 | x: Int = ¥100;
   |              ^ illegal character '¥'
```

## E0002：Invalid number literal

数字字面量格式不正确。

```yaoxiang
x: Int = 1_000_000_;  # 下划线位置错误
```

```
error[E0002]: Invalid number literal
  --> example.yx:1:10
   |
 1 | x = 1_000_000_;
   |          ^ invalid digit '_' in numeric literal
```

## E0003：Unterminated string

多行字符串缺少结束引号。

```yaoxiang
greeting = "Hello, World;
```

```
error[E0003]: Unterminated string
  --> example.yx:1:18
   |
 1 | greeting = "Hello, World;
   |                  ^ string never terminates
```

## E0004：Invalid character literal

字符字面量不正确。

```yaoxiang
c = 'ab';  # 字符只能包含一个字符
```

```
error[E0004]: Invalid character literal
  --> example.yx:1:10
   |
 1 | c = 'ab';
   |          ^^^ character literal must contain exactly one character
```

## E0010：Expected token

语法分析时期望特定 token。

```yaoxiang
add: (a: Int, b: Int)  Int = {  # 缺少 ->
    a + b
}
```

```
error[E0010]: Expected token
  --> example.yx:1:28
   |
 1 | add: (a: Int, b: Int)  Int = {
   |                       ^^ expected '->'
```

## E0011：Unexpected token

遇到意外的 token。

```yaoxiang
x = 10;
x +++;  # 意外的 ++
```

```
error[E0011]: Unexpected token
  --> example.yx:2:3
   |
 2 | x +++;
   |   ^^ unexpected token '++'
```

## E0012：Invalid syntax

表达式/语句语法错误。

```yaoxiang
if x > 0 {
    print(x)  # 缺少 {}
}
```

```
error[E0012]: Invalid syntax
  --> example.yx:2:5
   |
 2 |     print(x)
   |     ^ expected '{' after if condition
```

## E0013：Mismatched brackets

圆括号、方括号、花括号不匹配。

```yaoxiang
result = (1 + 2 * 3;
```

```
error[E0013]: Mismatched brackets
  --> example.yx:1:20
   |
 1 | result = (1 + 2 * 3;
   |                    ^ unclosed '('
```

## E0014：Missing semicolon

语句末尾缺少分号（对于需要分号的旧语法）。

```yaoxiang
x = 10;
y = 20;
```

```
error[E0014]: Missing semicolon
  --> example.yx:2:1
   |
 2 | y = 20
   | ^ expected ';' after variable declaration
```

## 相关章节

- [E1xxx：类型检查](./E1xxx.md)
- [错误码总索引](./index.md)
