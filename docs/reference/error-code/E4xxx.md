# E4xxx：泛型与特质

> 泛型参数和特质约束相关错误。

## E4001：Generic parameter mismatch

泛型参数数量或类型不匹配。

```yaoxiang
identity: [T](x: T) -> T = x;
result = identity[Int, String](10);  # 太多泛型参数
```

```
error[E4001]: Generic parameter mismatch: expected 1, found 2
  --> example.yx:2:27
   |
 2 | result = identity[Int, String](10);
   |                            ^^^^^^^^ too many generic parameters
```

## E4002：Trait bound violated

不满足 trait 约束。

```yaoxiang
double: [T: Addable](x: T) -> T = x + x;
s = double("hello");  # String 不实现 Addable
```

```
error[E4002]: Trait bound violated: String does not implement Addable
  --> example.yx:2:18
   |
 2 | s = double("hello");
   |                  ^^^^^ trait bound `Addable` not satisfied
```

## E4003：Associated type error

关联类型定义或使用错误。

```yaoxiang
# 使用 type 定义接口（RFC-010 语法）
type Container = {
    type Item;  # 关联类型
    get: () -> Item,
}
```

```
error[E4003]: Associated type error
  --> example.yx:3:5
   |
 3 |     type Item;
   |           ^^^ associated type definition error
```

## E4004：Duplicate trait implementation

重复实现同一 trait（RFC-010 使用接口组合语法）。

```yaoxiang
# 接口定义
type Printable = {
    print: () -> Void,
}

# 实现接口（在类型定义末尾列出接口名）
type IntPrinter = {
    value: Int,
    Printable,  # 实现 Printable 接口
}

type IntPrinter2 = {
    value: Int,
    Printable,  # 重复实现
}
```

```
error[E4004]: Duplicate trait implementation: Printable is already implemented for Int
  --> example.yx:10:5
   |
10 |     Printable,  # 重复实现
   |     ^^^^^^^^^^ conflicting implementation
```

## E4005：Trait not found

找不到要求的 trait。

```yaoxiang
print_all: [T: MyPrintable](items: List[T]) -> Void = {
    for item in items {
        item.print();
    }
}
```

```
error[E4005]: Trait not found: MyPrintable
  --> example.yx:1:16
   |
 1 | print_all: [T: MyPrintable](items: List[T]) -> Void = {
   |                ^^^^^^^^^^^ trait not defined
```

## E4006：Sized bound violated

Sized 约束不满足。

```yaoxiang
store: [T](value: T) -> Void = {
    # T 必须是 Sized
}
```

```
error[E4006]: Sized bound violated: T may not be sized
  --> example.yx:1:14
   |
 1 | store: [T](value: T) -> Void = {
   |            ^^^^^^^^^ T does not satisfy `Sized` bound
```

## 相关章节

- [E1xxx：类型检查](./E1xxx.md)
- [E5xxx：模块与导入](./E5xxx.md)
- [错误码总索引](./index.md)
