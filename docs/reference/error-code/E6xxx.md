# E6xxx：运行时错误

> 执行期间发生的错误。

## E6001：Division by zero

整数除以零。

```yaoxiang
main: () -> Void = {
    x = 10 / 0;
}
```

```
error[E6001]: Division by zero
  --> example.yx:2:17
   |
  2 |     x = 10 / 0;
   |            ^ division by zero
```

## E6002：Assertion failed

assert! 宏失败。

```yaoxiang
main: () -> Void = {
    x = 10;
    assert!(x > 100, "x must be greater than 100");  # assert! 宏
}
```

```
error[E6002]: Assertion failed: x must be greater than 100
  --> example.yx:3:5
   |
  3 |     assert!(x > 100, "x must be greater than 100");
   |     ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ assertion failed
```

## E6003：Arithmetic overflow

算术运算溢出。

```yaoxiang
main: () -> Void = {
    max_int = 9223372036854775807;  # 最大 Int 值
    overflow = max_int + 1;  # 溢出
}
```

```
error[E6003]: Arithmetic overflow
  --> example.yx:3:21
   |
  3 |     overflow = max_int + 1;
   |                     ^^^^^^^^^^^ arithmetic overflow
```

## E6004：Stack overflow

栈空间耗尽（递归深度过大）。

```yaoxiang
fibonacci: (n: Int) -> Int = {
    if n <= 1 {
        return n;
    }
    return fibonacci(n - 1) + fibonacci(n - 2);
}

main: () -> Void = {
    result = fibonacci(10000);
}
```

```
error[E6004]: Stack overflow: maximum recursion depth exceeded
  --> example.yx:7:22
   |
  7 |     result = fibonacci(10000);
   |                      ^^^^^^^^^^^^ recursion depth too large
```

## E6005：Heap allocation failed

内存分配失败。

```yaoxiang
main: () -> Void = {
    huge_list = List::new(1_000_000_000_000);
}
```

```
error[E6005]: Heap allocation failed
  --> example.yx:2:19
   |
  2 |     huge_list = List::new(1_000_000_000_000);
   |                   ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ memory allocation failed
```

## E6006：Runtime index out of bounds

运行时索引越界。

```yaoxiang
main: () -> Void = {
    arr = [1, 2, 3];  # 数组
    index = get_index();  # 动态索引
    value = arr[index];
}
```

```
error[E6006]: Runtime index out of bounds
  --> example.yx:4:18
   |
  4 |     value = arr[index];
   |                  ^^^^^^^^^^ index out of bounds at runtime
```

## E6007：Type cast failed

尝试将类型断言为不兼容类型。

```yaoxiang
main: () -> Void = {
    value: Any = 42;
    string_value = value as String;
}
```

```
error[E6007]: Type cast failed
  --> example.yx:3:22
   |
  3 |     string_value = value as String;
   |                      ^^^^^^^^^^^^^^^^ cannot cast `Int` to `String`
```

## 相关章节

- [E5xxx：模块与导入](./E5xxx.md)
- [E7xxx：I/O 与系统错误](./E7xxx.md)
- [错误码总索引](./index.md)
