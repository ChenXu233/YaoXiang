# 并发模型规范

> **状态：可能废弃，未实现。** 此文档不代表当前设计。实际所有权模型见 [类型系统规范](./type-system.md) 第十二章（借用令牌）和第十一章（类型属性）。`@block`/`@eager` 注解、`Mutex[T]`/`Atomic[T]`/`RwLock[T]` 等内容为早期草案，不反映当前设计方向。

本文件暂存，待并发模型设计确定后重写或删除。

**函数注解**：

| 注解 | 位置 | 行为 |
本文件暂存，待并发模型设计确定后重写或删除。

---

## 所有权模型（摘要）

完整模型见 [类型系统规范](./type-system.md) 第十一、十二章。此处仅保留安全模型摘要：

```yaoxiang
// Move（默认，零拷贝）
p2 = p

// ref（编译器自动选 Rc 或 Arc）
shared = ref p

// clone（显式深复制）
p2 = p.clone()

// unsafe（裸指针）
unsafe {
    ptr: *Point = &p
    (*ptr).x = 0.0
}
```

| 语义 | 说明 | 开销 |
|------|------|------|
| Move | 默认，所有权转移 | 零 |
| `&T` / `&mut T` | 借用令牌，零大小编译期权限证明 | 零 |
| `ref` | 编译器自动选 Rc（单任务）/ Arc（跨任务） | 按需 |

## spawn 语法（摘录）

完整语法见 [语法规范](./syntax.md) 3.10 节。

```yaoxiang
// spawn 块
(result_a, result_b) = spawn {
    parse(fetch("url1")),
    parse(fetch("url2"))
}

// spawn 循环
results = spawn for item in items {
    process(item)
}
```
