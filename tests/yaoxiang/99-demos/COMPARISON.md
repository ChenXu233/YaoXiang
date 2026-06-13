# PLDI SRC 对比：Rust vs YaoXiang

## 对比 1：结构体引用字段 — 消除生命周期标注

**论文论点**: YaoXiang 的令牌系统消除了生命周期标注 `'a`

### Rust（需要 `'a`）
```rust
struct Point { x: f64, y: f64 }

struct Window<'a> {     // ← 需要生命周期参数
    target: Point,
    view: &'a Point,    // ← 需要显式标注 'a
}

fn main() {
    let p = Point { x: 1.0, y: 2.0 };
    let w = Window { target: p, view: &p };
    //                                ^ 需要显式 &
    println!("{}", w.view.x);
}
```

### YaoXiang（无 `'a`，无显式 `&`）
```yaoxiang
Point: Type = { x: Float, y: Float }

Window: Type = {
    target: Point,
    view: &Point   // 零大小令牌，无生命周期标注
}

main = {
    p = Point(1.0, 2.0)
    w = Window(p, p)   // 自动创建令牌，无 &
    io.println(w.view.x)
}
```

**差异**: Rust 12 行含 `'a`，YaoXiang 10 行无 `'a`。令牌是类型属性，生命周期由 Move/RAII 自动管理。

---

## 对比 2：自由函数调用 — 自动借用

**论文论点**: 编译器自动创建令牌，用户不需要手动标注 `&`

### Rust（需要显式 `&`）
```rust
fn distance(a: &Point, b: &Point) -> f64 {
    (a.x - b.x).abs() + (a.y - b.y).abs()
}

fn main() {
    let p1 = Point { x: 0.0, y: 0.0 };
    let p2 = Point { x: 3.0, y: 4.0 };
    let d = distance(&p1, &p2); // ← 需要显式 &p1, &p2
    //              ^         手动标注借用
}
```

### YaoXiang（自动借用）
```yaoxiang
distance: (a: &Point, b: &Point) -> Float = {
    dx = a.x - b.x
    dy = a.y - b.y
    return dx * dx + dy * dy
}

main = {
    p1 = Point(0.0, 0.0)
    p2 = Point(3.0, 4.0)
    d = distance(p1, p2)  // ← 自动创建 &Point 令牌
    io.println(d)
}
```

**差异**: Rust 调用时需要 `&p1, &p2`，YaoXiang 调用时 `p1, p2` 自动借用。签名已声明权限需求，编译器自动处理。

---

## 对比 3：并发共享 — 消除 Send/Sync/Arc/Clone/async

**论文论点**: YaoXiang `ref` + `spawn` 消除了 Rust 的五个概念

### Rust（需要 Arc + Clone + Send + Sync + tokio::spawn + async move）
```rust
use std::sync::Arc;

#[derive(Clone)]
struct Data { value: i32 }
// Data: Send + Sync 必须实现（编译器自动推导）

fn main() {
    let data = Data { value: 100 };
    // 跨线程共享需要 Arc + 两次 clone
    let shared = Arc::new(data);
    let s1 = Arc::clone(&shared);  // 引用计数 +1
    let s2 = Arc::clone(&shared);  // 引用计数 +1

    let h1 = std::thread::spawn(move || { s1.value });
    let h2 = std::thread::spawn(move || { s2.value });

    let a = h1.join().unwrap();
    let b = h2.join().unwrap();
    println!("{} {}", a, b);
}
```

### YaoXiang（ref + spawn，零 ceremony）
```yaoxiang
main = {
    data = 100

    // ref: 编译器自动选 Rc 或 Arc
    shared = ref data

    result = spawn {
        a = shared    // ref 是 Dup 类型，自由复制
        b = shared    // 编译器判断跨线程 → Arc
        return (a, b)
    }

    io.println(result)
}
```

**差异**: Rust 需要 5 个概念（Arc, Clone, Send, Sync, move closure），YaoXiang 需要 2 个（ref, spawn）。

---

## 复杂度对比总结

| 概念 | Rust | YaoXiang | 减少 |
|------|------|----------|------|
| 生命周期标注 `'a` | 必需 | 不需要 | ✅ 消除 |
| 显式借用 `&x` | 必需 | 自动 | ✅ 消除 |
| `Send` / `Sync` trait | 必需 | 不需要 | ✅ 消除 |
| `Arc<T>` / `Rc<T>` | 手动选择 | 编译器选 | ✅ 消除 |
| 手动 `.clone()` | 必需 | ref 自动 | ✅ 消除 |
| `async` / `.await` | 必需 | spawn{} | ✅ 消除 |
| `Mutex<T>` / `RwLock<T>` | 必需 | ref 替代 | ✅ 消除 |
