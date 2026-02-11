# E8xxx：内部编译器错误

> 编译器内部错误，通常是 bug。

## E8001：Internal compiler error

编译器内部错误。

```
error[E8001]: Internal compiler error
  --> example.yx:1:1
   |
 1 | [compiler internal error]
   |
   = note: This is a bug in the YaoXiang compiler
   = please file an issue at https://github.com/yaoxiang-lang/yaoxiang/issues
```

## E8002：Codegen error

IR/字节码生成失败。

```
error[E8002]: Codegen error
  --> example.yx:5:1
   |
 5 | [code generation failed]
   |
   = note: Failed to generate bytecode for function
```

## E8003：Unimplemented feature

使用未实现的功能。

```yaoxiang
main: () -> Void = {
    # 某个尚未实现的功能
}
```

```
error[E8003]: Unimplemented feature
  --> example.yx:2:5
   |
 2 |     [feature not yet implemented]
   |
   = note: This feature is not yet implemented
```

## E8004：Optimization error

编译器优化错误。

```
error[E8004]: Optimization error
  --> example.yx:1:1
   |
 1 | [compiler optimization failed]
   |
   = note: This is a bug in the compiler's optimizer
```

## 报告内部错误

遇到 E8xxx 错误时，请：

1. 确认错误可复现
2. 收集最小复现代码
3. 在 [GitHub Issues](https://github.com/yaoxiang-lang/yaoxiang/issues) 报告

## 相关章节

- [E6xxx：运行时错误](./E6xxx.md)
- [E7xxx：I/O 与系统错误](./E7xxx.md)
- [错误码总索引](./index.md)
