# E0xxx: Lexer & Parser

> Errors occurring during lexical and parsing stages.

## E0001: Invalid character

Source code contains illegal characters.

```yaoxiang
x = ¥100;  # ¥ is not a valid character
```

```
error[E0001]: Invalid character
  --> example.yx:1:10
   |
 1 | x = ¥100;
   |          ^ illegal character '¥'
```

## E0002: Invalid number literal

Number literal format is incorrect.

```yaoxiang
x = 1_000_000_;  # Underscore in wrong position
```

```
error[E0002]: Invalid number literal
  --> example.yx:1:10
   |
 1 | x = 1_000_000_;
   |          ^ invalid digit '_' in numeric literal
```

## E0003: Unterminated string

Multi-line string missing closing quote.

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

## E0004: Invalid character literal

Character literal is incorrect.

```yaoxiang
c = 'ab';  # Character can only contain one character
```

```
error[E0004]: Invalid character literal
  --> example.yx:1:10
   |
 1 | c = 'ab';
   |          ^^^ character literal must contain exactly one character
```

## E0010: Expected token

Parser expects a specific token.

```yaoxiang
add: (a: Int, b: Int)  Int = {  # Missing ->
    a + b
}
```

```
error[E0010]: Expected token
  --> example.yx:1:28
   |
 1 | add: (a: Int, b: Int)  Int = {
   |                      ^^ expected '->'
```

## E0011: Unexpected token

Encountered an unexpected token.

```yaoxiang
x = 10;
x +++;  # Unexpected ++
```

```
error[E0011]: Unexpected token
  --> example.yx:2:3
   |
 2 | x +++;
   |   ^^ unexpected token '++'
```

## E0012: Invalid syntax

Expression/statement syntax error.

```yaoxiang
if x > 0 {
    print(x)  # Missing {}
}
```

```
error[E0012]: Invalid syntax
  --> example.yx:2:5
   |
 2 |     print(x)
   |     ^ expected '{' after if condition
```

## E0013: Mismatched brackets

Parentheses, brackets, braces do not match.

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

## E0014: Missing semicolon

Statement missing semicolon (for old syntax requiring semicolons).

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

## Related

- [E1xxx: Type Checking](./E1xxx.md)
- [Error Code Index](./index.md)
