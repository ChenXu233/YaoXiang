# E2xxx：语义分析

> 作用域、生命周期等语义相关错误。

## E2001：Scope error

变量不在当前作用域。

```yaoxiang
outer: () -> Int = {
    x = 10;
    inner: () -> Int = {
        return x;  # x 不在 inner 的作用域中
    };
    return inner();
}
```

```
error[E2001]: Scope error: x is not in scope
  --> example.yx:4:16
   |
 4 |         return x;
   |                ^ not defined in this scope
```

## E2002：Duplicate definition

同一作用域内重复定义。

```yaoxiang
x = 10;
x = 20;  # 重复定义 x
```

```
error[E2002]: Duplicate definition: x is already defined
  --> example.yx:2:5
   |
 2 | x = 20;
   |     ^ previously defined here
```

## E2003：Ownership error

所有权约束不满足（YaoXiang 使用 Move 语义而非借用）。

```yaoxiang
dangling: () -> Int = {
    x = 10;
    return x;  # x 的所有权在返回时转移
}
```

## E2010：Immutable assignment

尝试修改不可变变量。

```yaoxiang
x = 10;
x = 20;  # x 是不可变的
```

```
error[E2010]: Immutable assignment: cannot assign to immutable variable
  --> example.yx:2:3
   |
 2 | x = 20;
   | ^ variable `x` is immutable
```

**修复**：使用 `mut` 声明可变变量。

## E2011：Uninitialized use

使用未初始化的变量。

```yaoxiang
x: Int;
print(x);  # x 未初始化
```

```
error[E2011]: Uninitialized use: x may be used here
  --> example.yx:2:8
   |
 2 | print(x);
   |        ^ variable `x` is not initialized
```

## E2012：Mutability conflict

不可变上下文中使用可变引用。

```yaoxiang
mut x = 10;
ref y = x;  # ref 创建共享引用
x = 20;  # y 是共享引用，不能修改
```

```
error[E2012]: Mutability conflict
  --> example.yx:3:3
   |
 3 | x = 20;
   | ^ cannot mutate through shared reference
```

## 相关章节

- [E1xxx：类型检查](./E1xxx.md)
- [E4xxx：泛型与特质](./E4xxx.md)
- [错误码总索引](./index.md)
