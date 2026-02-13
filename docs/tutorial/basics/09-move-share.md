---
title: 第九章：移动与共享
---

# 第九章：移动与共享

> 本章目标：深入理解 Move、ref 和 clone 三种共享方式，掌握所有权的高级用法


## 9.1 三种共享方式回顾

| 方式 | 语法 | 含义 | 线程安全 | 性能 |
|------|------|------|----------|------|
| **移动** | `p2 = p` | 所有权转移 | - | 零开销 |
| **引用** | `ref p` | 共享（Arc） | ✅ 安全 | 中等 |
| **克隆** | `.clone()` | 显式复制 | - | 视类型 |

## 9.2 深入移动（Move）

### 9.2.1 赋值 = 移动

```yaoxiang
# 赋值就是移动
original: String = "Hello"
copy = original              # 移动！original 变空
```

### 9.2.2 函数参数 = 移动

```yaoxiang
# 函数参数默认是移动
process: (data: Data) -> Void = {
    print(data)              # 使用 data
    # 函数结束，data 被释放
}

main: () -> Void = {
    big_data: Data = load_data()
    process(big_data)       # 移动！big_data 变空
}
```

### 9.2.3 返回值 = 移动

```yaoxiang
# 函数返回值也是移动
create_point: () -> Point = {
    p = Point(1.0, 2.0)
    return p                 # 移动！p 变空，返回新值
}

main: () -> Void = {
    my_point = create_point()  # 接收返回值
}
```

## 9.3 深入引用（ref）

### 9.3.1 创建引用

```yaoxiang
# ref 关键字创建共享引用（Arc）
data: Point = Point(1.0, 2.0)
shared: ref Point = ref data    # Arc 引用计数

# ref 自动推断类型
shared2 = ref data             # 自动推导为 ref Point
```

### 9.3.2 引用的特点

```yaoxiang
data: Point = Point(1.0, 2.0)

# 创建多个引用
ref1 = ref data
ref2 = ref data
ref3 = ref data

# 引用计数 = 3
# ref1、ref2、ref3 都指向同一个 data
```

### 9.3.3 引用计数自动管理

```yaoxiang
{
    data = Point(1.0, 2.0)    # 引用计数 = 1

    shared = ref data           # 引用计数 = 2

    # shared 离开作用域，引用计数 = 1

}                               # data 离开作用域，引用计数 = 0，自动释放
```

### 9.3.4 并发共享

```yaoxiang
# ref = Arc，线程安全
shared_data = ref my_data

spawn(() => {
    # 在新任务中使用共享数据
    print(shared_data.x)
})

spawn(() => {
    # 另一个任务
    print(shared_data.y)
})
```


## 9.4 深入克隆（clone）

### 9.4.1 显式复制

```yaoxiang
# 克隆 = 显式复制
original: Point = Point(1.0, 2.0)
copy = original.clone()        # 创建一个新副本

# 现在有两个独立的 Point
original.x = 0.0              # ✅ 不影响 copy
copy.x = 10.0                 # ✅ 不影响 original
```

### 9.4.2 什么时候用 clone？

| 场景 | 是否用 clone |
|------|--------------|
| 需要保留原值 | ✅ 用 |
| 原值不再需要 | ❌ 用 Move |
| 需要多个副本 | ✅ 用 |

```yaoxiang
# 场景1：需要保留原值
data: Config = load_config()
backup = data.clone()          # ✅ 保留备份
process(data)                  # 处理原数据

# 场景2：原值不再需要
processed = data               # ✅ 直接移动
```


## 9.5 三种方式对比

```yaoxiang
# 数据
p: Point = Point(1.0, 2.0)

# === 方式1：Move ===
p2 = p          # p 变空，p2 是新所有者
# print(p.x)     # ❌ 错误！p 变空了
# print(p2.x)    # ✅ 正确！2.0

# === 方式2：ref ===
p: Point = Point(1.0, 2.0)  # 重新创建
shared = ref p      # 共享，p 仍然是所有者
# print(shared.x)   # ✅ 可以访问
# print(p.x)        # ✅ p 仍然是所有者

# === 方式3：clone ===
p: Point = Point(1.0, 2.0)  # 重新创建
copy = p.clone()  # 创建独立副本
# print(p.x)       # ✅ p 仍然有效
# print(copy.x)    # ✅ copy 是独立副本
```


## 9.6 所有权回流

当函数修改参数后返回，可以"回流"所有权：

```yaoxiang
# 所有权回流示例
p: Point = Point(1.0, 2.0)

# p 被修改后返回
p = p.translate(10.0, 10.0)   # p.translate 返回新的 Point

# 等价于：
# temp = p.translate(10.0, 10.0)
# p = temp                      # 回流
```

**链式调用**：

```yaoxiang
p: Point = Point(1.0, 2.0)

# 链式调用
p = p.translate(10.0, 10.0)
      .rotate(90)
      .scale(2.0)
```


## 9.7 循环引用

**注意**：循环引用可能导致内存泄漏！

```yaoxiang
# ❌ 循环引用（编译器会报错）
a = Node("A")
b = Node("B")
a.child = ref b    # a -> b
b.child = ref a    # b -> a（循环！）
```

**解决方法**：YaoXiang 编译器会检测跨任务循环：

```yaoxiang
# ✅ 单任务内循环（编译器允许，泄漏可控）
{
    a = Node("A")
    b = Node("B")
    a.child = ref b
    b.child = ref a

    # 任务结束后一起释放，泄漏消失
}
```


## 9.8 unsafe 模式

对于底层操作，可以用 `unsafe` 绕过检查：

```yaoxiang
# unsafe：裸指针操作
p: Point = Point(1.0, 2.0)

unsafe {
    ptr: *Point = &p       # 获取裸指针
    (*ptr).x = 0.0          # 直接修改内存
}
```

**注意**：`unsafe` 需要自己保证安全！


## 9.9 本章小结

| 概念 | 语法 | 说明 |
|------|------|------|
| 移动 | `p2 = p` | 所有权转移，零开销 |
| 引用 | `ref p` | 共享（Arc），线程安全 |
| 克隆 | `.clone()` | 显式复制，需要时用 |
| 回流 | `p = p.method()` | 链式调用 |
| unsafe | `unsafe { ... }` | 裸指针操作 |


## 9.10 易经引言

> 「反者道之动，弱者道之用。
>  天下万物生于有，有生于无。」
> —— 《道德经》
>
> 移动与共享，亦是阴阳之道：
> - **动**：Move 是阳，转让所有权
> - **静**：ref 是阴，维持共享
> - **变**：clone 是化，创造新象
>
> 三者相生相克，循环往复。
> 懂得了"动"与"静"的平衡，方能驾驭所有权之剑。
