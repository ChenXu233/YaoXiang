# E5xxx：模块与导入

> 模块解析和导入导出相关错误。

## E5001：Module not found

导入的模块不存在。

```yaoxiang
use "./nonexistent";  # 模块不存在
```

```
error[E5001]: Module not found: ./nonexistent
  --> example.yx:1:5
   |
 1 | use "./nonexistent";
   |     ^^^^^^^^^^^^^^^^^ file not found
```

## E5002：Cyclic import

模块间循环依赖。

```yaoxiang
# a.yx
use "./b";

pub a_func: () -> Void = b.b_func();
```

```yaoxiang
# b.yx
use "./a";

pub b_func: () -> Void = a.a_func();
```

```
error[E5002]: Cyclic import detected
  --> a.yx:1:1
   |
 1 | use "./b";
   | ^^^^^^^^^^^ circular dependency: a -> b -> a
```

## E5003：Symbol not exported

尝试访问未导出的符号。

```yaoxiang
# my_module.yx
internal = 10;  # 未导出

# main.yx
use "./my_module";
x = my_module.internal;
```

```
error[E5003]: Symbol not exported: internal
  --> main.yx:3:18
   |
 3 | x = my_module.internal;
   |                  ^^^^^^^^ `internal` is not exported by `./my_module`
```

## E5004：Invalid module path

模块路径格式错误。

```yaoxiang
use "invalid/path/../to/module";  # 路径包含 ..
```

```
error[E5004]: Invalid module path
  --> example.yx:1:5
   |
 1 | use "invalid/path/../to/module";
   |     ^^^^^^^^^^^^^^^^^^^^^^^^^^^^ invalid path format
```

## E5005：Private access

访问私有符号。

```yaoxiang
# other.yx
private_value = 42;

# main.yx
use "./other";
x = other.private_value;
```

```
error[E5005]: Private access: private_value is private
  --> main.yx:3:18
   |
 3 | x = other.private_value;
   |                  ^^^^^^^^^^^ cannot access private symbol
```

## 相关章节

- [E1xxx：类型检查](./E1xxx.md)
- [E6xxx：运行时错误](./E6xxx.md)
- [错误码总索引](./index.md)
