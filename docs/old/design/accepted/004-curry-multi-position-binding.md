# RFC-004: 柯里化方法的多位置联合绑定设计

> **状态**: 已接受
> **作者**: 晨煦
> **创建日期**: 2025-01-05
> **最后更新**: 2026-02-03（与 RFC-010 统一语法对齐：参数名在签名中声明）

## 摘要

本 RFC 提出一种全新的**多位置联合绑定**语法，允许将函数精确绑定到类型的任意参数位置，支持单位置绑定和多位置联合绑定，从根本上解决柯里化绑定中"谁是调用者"的问题，无需引入 `self` 关键字。

## 动机

### 为什么需要这个特性？

当前语言设计中，将独立函数绑定为类型方法时面临以下问题：

1. **调用者位置不灵活**：传统绑定只能固定 `obj.method(args)` 中的 `obj` 为第一个参数
2. **多参数绑定困难**：当方法需要接收多个同类型参数时，无法优雅表达
3. **柯里化语义歧义**：部分应用时难以区分"绑定到哪个位置"

### 设计目标：统一两种编程视角

本设计旨在**统一函数式和 OOP 两种编程视角**：

```yaoxiang
# 函数视角：显式传递所有参数
distance(p1, p2)

# OOP 视角：隐式 this
p1.distance(p2)

# [positions] 语法糖让两种写法等价，本质都是函数调用
Point.distance = distance[0]   # this 绑定到第 0 位
```

**核心价值**：
- 底层是函数，上层是方法语法
- 不引入 `self` 关键字，保持语言简洁性
- 完全函数化：方法调用本质是参数传递
- `[0]`, `[1]`, `[-1]` 灵活控制 this 绑定位置
- **语法统一**：函数定义使用 `name: (params) -> Return = body` 格式

### 当前的问题

```yaoxiang
# 现有设计的问题：
type Point = { x: Float, y: Float }
type Vector = { x: Float, y: Float, z: Float }

distance: (a: Point, b: Point) -> Float = { ... }
transform: (p: Point, v: Vector) -> Point = { ... }

# 只能绑定到第一个参数
Point.distance = distance  # 等价于 distance[0]
# p1.distance(p2) → distance(p1, p2) ✓

# 但如果 transform 的签名是 transform(Vector, Point) 呢？
# 无法表达 p1.transform(v1) → transform(v1, p1) 的语义
```

## 提案

### 核心设计：默认绑定 + 可选位置指定

#### 默认绑定到第一个类型匹配的位置

**默认行为**：`Type.method = function` 自动查找第一个和该类型匹配的位置并绑定

```yaoxiang
# 默认绑定第一个类型匹配的位置
Point.distance = distance           # 编译器自动查找第一个 Point 参数位置
p1.distance(p2)                     # → distance(p1, p2)

# 如果函数有两个 Point 参数，绑定到第一个匹配的位置
distance: (a: Point, b: Point) -> Float = {
    dx = a.x - b.x
    dy = a.y - b.y
    return (dx * dx + dy * dy).sqrt()
}
# 绑定：Point.distance = distance
# 调用：p1.distance(p2) → distance(p1, p2) ✓

# 只有需要特殊位置（不是第一个匹配）时才显式指定
Point.compare = distance[1]        # 绑定到第二个 Point 参数
p1.compare(p2)                    # → distance(p2, p1)
```

**绑定失败处理**：
- **找不到匹配类型**：如果函数参数中没有该类型，报错或警告
- **工厂函数模式**：如果没有参数匹配，可能作为工厂函数使用

```yaoxiang
# 情况1：找不到匹配类型
create_point: () -> Point = { ... }
Point.create = create_point        # 错误：没有 Point 类型参数

# 情况2：工厂函数模式（可选）
Point.create = create_point        # 作为工厂函数，调用：Point.create()
```

**好处**：
- 智能绑定：根据类型自动匹配，符合直觉
- 类型安全：只有类型匹配才绑定，避免错误
- 灵活控制：当默认绑定不是期望行为时，可显式指定位置

#### 自动柯里化绑定

当函数参数数量 > 绑定位置数量时，自动生成柯里化函数：

```yaoxiang
type Point = { x: Float, y: Float }

# 基础函数：3 个参数
scale: (p: Point, factor: Float) -> Point = {
    return Point(p.x * factor, p.y * factor)
}

# 绑定时自动柯里化
Point.scale = scale[0, 1]   # Point 绑定到第 0、1 位，第 2 位保留

# 调用时自动部分应用
p1 = Point(2.0, 3.0)
scaled = p1.scale(2.0)       # → scale(p1, 2.0) 直接调用
result = scaled              # → Point(4.0, 6.0)

# 链式调用更优雅
result = Point(2.0, 3.0).scale(2.0)  # → Point(4.0, 6.0)
```

### 位置索引绑定语法

引入 `[position]` 语法精确控制函数参数与类型的绑定关系：

```yaoxiang
# 语法格式：Type.method = function[positions]

# === 基础绑定 ===

# 单位置绑定
Point.distance = distance[1]           # 绑定到第1参数（索引从0开始）
# 使用：p1.distance(p2) → distance(p2, p1)

# 多位置联合绑定（元组解构）
Point.transform = transform[1, 2]      # 绑定到第1,2参数
# 使用：p1.transform(v1) → transform(v1, p1)
# 原函数签名：transform(Point, Vector) → Point
# 绑定后：Point.transform(Vector) → Point
```

### 详细语法定义

```
绑定声明 ::= 类型 '.' 标识符 '=' 函数名 '[' 位置列表 ']'

位置列表 ::= 位置 (',' 位置)*
位置     ::= 整数                    # 占位符
           | '_'                    # 跳过此位置（占位符）
           | 整数 '..' 整数         # 位置范围（未来扩展）

函数名   ::= 标识符
类型     ::= 标识符 (泛型参数)?
```

### 使用示例

```yaoxiang
# === 完整示例 ===

type Point = { x: Float, y: Float }
type Vector = { x: Float, y: Float, z: Float }

# 1. 基础距离计算
distance: (a: Point, b: Point) -> Float = {
    dx = a.x - b.x
    dy = a.y - b.y
    return (dx * dx + dy * dy).sqrt()
}

# 绑定：Point.distance = distance[1]
# 调用：p1.distance(p2) → distance(p2, p1)
# 但我们想要 p1.distance(p2) → distance(p1, p2)，所以：
Point.distance = distance[0]

# 2. 变换操作（多位置绑定）
transform: (p: Point, v: Vector) -> Point = {
    return Point(p.x + v.x, p.y + v.y)
}

# 绑定 Point.transform = transform[1]
# 调用：p.transform(v) → transform(v, p) ❌
# 绑定 Point.transform = transform[0]
# 调用：p.transform(v) → transform(p, v) ✓

# 3. 复杂多参数函数
multiply: (a: Point, s: Float) -> Point = {
    return Point(a.x * s, a.y * s)
}

# 只绑定第1参数（Point类型），保留第3参数
Point.scale = multiply[0, _]
# 调用：p.scale(2.0) → multiply(p, 2.0)

# 4. 跨类型绑定
type Circle = { center: Point, radius: Float }

distance: (a: Circle, b: Circle) -> Float = {
    return a.center.distance(b.center) - a.radius - b.radius
}

# 将距离方法绑定到 Circle 类型
Circle.distance = distance[0, 1]
# 调用：c1.distance(c2) → distance(c1, c2)
```

### 元组解构支持

```yaoxiang
# === 元组解构绑定 ===

# 函数接收元组参数
process_coordinates: (coord: (Float, Float)) -> String = {
    return match coord {
        (0.0, 0.0) -> "origin"
        (x, 0.0) -> "on x-axis at ${x}"
        (0.0, y) -> "on y-axis at ${y}"
        (x, y) -> "point at (${x}, ${y})"
    }
}

type Coord = { x: Float, y: Float }

# 自动解构绑定：Coord -> (Float, Float)
Coord.describe = process_coordinates[1]
# 使用：coord.describe() → process_coordinates((coord.x, coord.y))
```

### 多返回值绑定

```yaoxiang
# === 多返回值绑定 ===

min_max: (list: List[Int]) -> (Int, Int) = {
    min = list.reduce(Int.MAX, (a, b) => if a < b then a else b)
    max = list.reduce(Int.MIN, (a, b) => if a > b then a else b)
    return (min, max)
}

List.range: [T](self: List[T]) -> (T, T) = min_max[1]
# 使用：(min_val, max_val) = list.range()
```

## 详细设计

### 编译器实现

#### 绑定解析算法

```rust
struct Binding {
    type_name: String,
    method_name: String,
    function_name: String,
    positions: Vec<usize>,      // 绑定位置列表
    is_partial: bool,           // 是否为部分绑定
}

fn parse_binding(expr: Expr) -> Binding {
    // 格式：Type.method = function[positions]
    // 或：Type.method = function（默认自动查找类型匹配位置）

    let (type_name, method_name) = parse_left_side(expr.left);
    let (function_name, positions) = parse_right_side(expr.right);

    // 如果没有指定位置，自动查找第一个类型匹配的位置
    let final_positions = match positions {
        Some(pos) => normalize_positions(pos),
        None => auto_find_matching_positions(type_name, function_name),
    };

    Binding {
        type_name,
        method_name,
        function_name,
        positions: final_positions,
        is_partial: has_remaining_params(positions, function_name),
    }
}

fn auto_find_matching_positions(type_name: &str, function_name: &str) -> Vec<usize> {
    // 查找函数签名中第一个匹配 type_name 的位置
    let func = find_function(function_name);

    for (idx, param) in func.params.iter().enumerate() {
        if param.type_ == type_name {
            return vec![idx];
        }
    }

    // 如果没找到匹配，返回错误或空向量
    // 编译器将根据配置决定是报错、警告还是视为工厂函数
    vec![]
}
```

#### 调用代码生成

```rust
fn generate_method_call(
    obj_type: Type,
    method_name: String,
    args: Vec<Expr>
) -> Expr {
    let binding = find_binding(obj_type, method_name);
    let func = find_function(binding.function_name);

    let param_count = func.params.len();
    let mut new_args = vec![];
    let mut user_arg_idx = 0;

    for pos in 0..param_count {
        if binding.positions.contains(&pos) {
            // 这个位置由调用者对象（this）提供
            new_args.push(Expr::This(obj_type));
        } else {
            // 这个位置由用户提供的参数填充
            if user_arg_idx < args.len() {
                new_args.push(args[user_arg_idx].clone());
                user_arg_idx += 1;
            } else {
                // 参数不足，创建部分应用
                return Expr::PartialApply(binding, args);
            }
        }
    }

    Expr::Call(binding.function_name, new_args)
}
```

### 类型检查规则

```rust
fn check_binding_type_compatibility(
    binding: &Binding,
    func: &Function
) -> Result<(), TypeError> {
    // 1. 如果是自动查找位置（未显式指定），检查是否找到匹配
    if binding.positions.is_empty() {
        return Err(TypeError::NoMatchingParameter(
            binding.type_name.clone(),
            func.name.clone()
        ));
    }

    // 2. 验证所有位置索引有效
    for pos in &binding.positions {
        if *pos >= func.params.len() {
            return Err(TypeError::InvalidBindingPosition(*pos));
        }
    }

    // 3. 检查绑定位置的类型兼容性
    for pos in &binding.positions {
        let param_type = &func.params[*pos].type_;
        let binding_type = &binding.type_name;

        if !isAssignable(binding_type, param_type) {
            return Err(TypeError::IncompatibleTypes(
                binding_type, param_type
            ));
        }
    }

    // 4. 检查方法调用参数与剩余参数匹配
    Ok(())
}
```

### 运行时行为

| 场景 | 绑定语法 | 调用 | 转换为 |
|------|---------|------|--------|
| 默认绑定 | `Point.distance = distance` | `p1.distance(p2)` | `distance(p1, p2)` |
| 自动匹配 | `Point.transform = transform` | `p.transform(v)` | `transform(p, v)` |
| 单位置 | `Point.distance = distance[1]` | `p1.distance(p2)` | `distance(p2, p1)` |
| 单位置 | `Point.test = func[-1]` | `p.test(a, b)` | `func(a, b, p)` |
| 多位置 | `Point.transform = transform[0, _]` | `p.transform(v)` | `transform(p, v)` |
| 自动柯里化 | `Point.scale = scale[0, _]` | `p.scale(2.0)` | `scale(p, 2.0)` |
| 占位符 | `Type.method = func[1, _]` | `obj.method(arg)` | `func(arg, obj)` |

**说明**：
- **默认绑定**：自动查找第一个类型匹配的位置
- `[0]`：this 绑定到第 0 位（第一个参数）
- `[1]`：this 绑定到第 1 位（第二个参数）
- `[-1]`：this 绑定到最后一位（从末尾计数）

## 权衡

### 优点

- **智能默认绑定**：默认绑定第一个类型匹配的位置，无需显式指定 `[positions]`
- **精确控制**：可以绑定到任意参数位置，灵活度高
- **类型安全**：编译时完全类型检查，只有类型匹配才绑定
- **语法简洁**：`[position]` 语法直观易懂
- **无 `self` 关键字**：保持语言简洁性
- **柯里化友好**：天然支持部分应用和链式调用
- **OOP 友好**：自动柯里化让 OOP 程序员无脑迁移

### 缺点

- **学习成本**：需要理解位置索引概念
- **编译复杂度**：绑定解析和类型检查增加编译器复杂度
- **调试难度**：错误信息需要清晰指出绑定位置问题

## 替代方案

| 方案 | 描述 | 为什么不选 |
|------|------|-----------|
| `self` 关键字 | 引入 Python/Rust 风格的 `self` | 违反 YaoXiang 无隐式 `self` 的设计哲学 |
| 命名参数绑定 | 使用命名参数 `func(a=obj)` | 需要修改函数签名定义，增加复杂性 |
| 宏系统 | 用宏实现绑定 | 运行时开销大，类型安全性降低 |
| 运算符重载 | 限制 `self` 在特定位置 | 语法不统一，语义混乱 |

## 实现策略

### 阶段划分

1. **Phase 1: 基础绑定**（v0.3）
   - 实现单位置 `[n]` 绑定语法（n 从 0 开始，支持负数）
   - 基本的类型检查和代码生成
   - 单元测试覆盖

2. **Phase 2: 多位置绑定**（v0.4）
   - 实现多位置 `[n, m, ...]` 联合绑定
   - 占位符 `_` 支持
   - 支持元组解构和部分应用

3. **Phase 3: 高级特性**（v0.5）
   - 支持范围语法 `[n..m]`
   - 编译时位置计算优化

### 依赖关系

- 无外部依赖
- 与 RFC-001（错误处理）无直接关联
- 可独立实现

### 风险

- 与现有绑定语法的兼容性处理
- 性能优化策略（编译期展开 vs 运行时查找）

## 开放问题

以下问题已在设计中解决，记录在附录A：

- ~~位置索引从 0 开始~~ → 已决定：从 0 开始
- ~~负数索引~~ → 已决定：支持
- ~~占位符~~ → 已决定：使用 `_`
- ~~范围语法~~ → 已决定：实现

**剩余开放问题**：

- [ ] 与现有绑定语法的兼容性处理
- [ ] 性能优化策略（编译期展开 vs 运行时查找）

---

## 附录

### 附录A：设计决策记录

| 决策 | 决定 | 理由 |
|------|------|------|
| 索引基准 | 从 0 开始 | 与元组/参数列表索引一致 |
| 负数索引 | 支持 | 灵活，从末尾计数 |
| 占位符 | `_` | 简洁，通用符号 |
| 范围语法 | 实现 | 批量绑定，如 `[0..2]` |
| 语法风格 | 中缀 `Type.method = func[positions]` | 与 RFC-010 统一 |
| **默认绑定逻辑** | **绑定第一个类型匹配的位置** | **更智能、更安全，符合直觉** |
| **绑定失败处理** | **找不到匹配时报错/警告/工厂函数** | **根据上下文灵活处理** |
| **函数语法** | **参数名在签名中 `name: (params) -> Return`** | **与 RFC-010 统一** |

### 附录B：术语表

| 术语 | 定义 |
|------|------|
| 绑定位置 | 函数参数列表中的索引位置 |
| 联合绑定 | 将类型绑定到多个参数位置 |
| 部分应用 | 只提供部分参数，返回待完成调用的函数 |
| **统一语法** | **`name: (params) -> Return = body`，参数名在签名中声明** |
| **类型匹配绑定** | **默认绑定逻辑：自动查找第一个与调用者类型匹配的位置** |
| **工厂函数绑定** | **当函数参数中没有匹配类型时，作为构造器使用** |

---

## 参考文献

- [Rust impl 语法](https://doc.rust-lang.org/book/ch05-03-method-syntax.html)
- [Haskell 类型类](https://wiki.haskell.org/Type_class)
- [Kotlin 扩展函数](https://kotlinlang.org/docs/extensions.html)
