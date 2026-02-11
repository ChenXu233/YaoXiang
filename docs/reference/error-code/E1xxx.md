# E1xxx：类型检查

> 类型系统相关错误。

## E1001：Unknown variable

引用的变量未定义。

```yaoxiang
x = 10;
print(y);  # y 未定义
```

```
error[E1001]: Unknown variable: y
  --> example.yx:2:8
   |
 2 | print(y);
   |        ^ not found in this scope
```

**修复**：检查变量名是否拼写正确，或先定义该变量。

## E1002：Type mismatch

期望类型与实际类型不符。

```yaoxiang
x: Int = "hello";
```

```
error[E1002]: Type mismatch: expected `Int`, found `String`
  --> example.yx:1:12
   |
 1 | x: Int = "hello";
   |            ^^^^^^^^ expected `Int`, found `String`
```

**修复**：使用正确的类型，或添加类型转换。

## E1003：Unknown type

引用的类型不存在。

```yaoxiang
x: MyType = 10;
```

```
error[E1003]: Unknown type: MyType
  --> example.yx:1:10
   |
 1 | x: MyType = 10;
   |          ^^^^^^ not defined
```

## E1010：Parameter count mismatch

函数调用参数数量与定义不符。

```yaoxiang
add: (a: Int, b: Int) -> Int = a + b;
result = add(10);  # 只传了 1 个参数
```

```
error[E1010]: Parameter count mismatch: expected 2, found 1
  --> example.yx:2:16
   |
 2 | result = add(10);
   |                ^^^ expected 2 arguments
```

## E1011：Parameter type mismatch

参数类型检查失败。

```yaoxiang
greet: (name: String) -> Void = print(name);
greet(123);  # 传递了 Int 而不是 String
```

```
error[E1011]: Parameter type mismatch: expected `String`, found `Int`
  --> example.yx:2:8
   |
 2 | greet(123);
   |        ^^^ expected `String`, found `Int`
```

## E1012：Return type mismatch

函数返回值类型错误。

```yaoxiang
getNumber: () -> Int = "hello";
```

```
error[E1012]: Return type mismatch: expected `Int`, found `String`
  --> example.yx:1:22
   |
 1 | getNumber: () -> Int = "hello";
   |                      ^ expected `Int`, found `String`
```

## E1013：Function not found

调用未定义的函数。

```yaoxiang
result = my_function(10);
```

```
error[E1013]: Function not found: my_function
  --> example.yx:1:13
   |
 1 | result = my_function(10);
   |              ^^^^^^^^^^^ not defined
```

## E1020：Cannot infer type

上下文无法推断类型。

```yaoxiang
identity: (x: T) -> T = x;  # 泛型函数需要类型参数
```

```
error[E1020]: Cannot infer type for parameter `x`
  --> example.yx:1:13
   |
 1 | identity: (x: T) -> T = x;
   |             ^ parameter must have type annotation
```

## E1021：Type inference conflict

多处约束导致类型矛盾。

```yaoxiang
x: Int = 10;
y: String = x;  # Int 无法转换为 String
```

```
error[E1021]: Type inference conflict
  --> example.yx:2:14
   |
 2 | y: String = x;
   |              ^ cannot convert `Int` to `String`
```

## E1030：Pattern non-exhaustive

match 表达式未覆盖所有情况。

```yaoxiang
result: Result[Int, String] = ok(42);
match result {
    ok(value) => print(value),
    # 缺少 err 分支
};
```

```
error[E1030]: Pattern non-exhaustive: missing `err` pattern
  --> example.yx:3:1
   |
 3 | match result {
   | ^ not all patterns are covered
```

## E1031：Unreachable pattern

永远无法匹配的模式。

```yaoxiang
x: Int = 10;
match x {
    0 => print("zero"),
    1 => print("one"),
    10 => print("ten"),  # 这个分支永远无法到达
    _ => print("other"),
};
```

```
warning[E1031]: Unreachable pattern
  --> example.yx:4:5
   |
 4 |     10 => print("ten"),
   |     ^^ pattern can never match
```

## E1040：Operation not supported

类型不支持该操作。

```yaoxiang
a: String = "hello";
b: String = "world";
c: String = a * b;  # String 不支持 * 操作
```

```
error[E1040]: Operation not supported: `String * String`
  --> example.yx:3:13
   |
 3 | c: String = a * b;
   |             ^ operator '*' is not defined for `String`
```

## E1041：Index out of bounds

数组/列表索引超出范围。

```yaoxiang
arr: List[Int] = [1, 2, 3];
x: Int = arr[10];  # 索引超出数组大小
```

```
error[E1041]: Index out of bounds: index 10, size 3
  --> example.yx:2:13
   |
 2 | x: Int = arr[10];
   |             ^^ index 10 out of bounds for array of size 3
```

## E1042：Field not found

访问不存在的结构体字段。

```yaoxiang
type Point = { x: Int, y: Int };
p: Point = Point(10, 20);  # 构造器语法
z: Int = p.z;  # Point 没有 z 字段
```

```
error[E1042]: Field not found: `z` in `Point`
  --> example.yx:3:11
   |
 3 | z: Int = p.z;
   |           ^ field `z` does not exist in `Point`
```

## 相关章节

- [E0xxx：词法与语法分析](./E0xxx.md)
- [E2xxx：语义分析](./E2xxx.md)
- [错误码总索引](./index.md)
