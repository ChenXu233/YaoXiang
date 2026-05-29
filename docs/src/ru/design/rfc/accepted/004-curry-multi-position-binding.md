```md
---
title: RFC-004：多позиционное объединённое связывание для каррированных методов
---

# RFC-004: 多позиционное объединённое связывание для каррированных методов

> **Статус**: Принято
> **Автор**: Чэнь Сюй
> **Дата создания**: 2025-01-05
> **Последнее обновление**: 2026-02-18 (добавлены встроенные绑定, постфиксный синтаксис связывания)

## Аннотация

Настоящий RFC предлагает принципиально новый синтаксис **многопозиционного объединённого связывания**, позволяющий точно привязать функцию к любому параметрическому положению типа, с поддержкой как однопозиционного, так и многопозиционного объединённого связывания,从根本上解决 каррированном связывании проблему «кто является вызывающей стороной», без введения ключевого слова `self`.

## Мотивация

### Зачем нужна эта функция?

При текущей дизайне языка связывание独立函数 в качестве методов типа сталкивается со следующими проблемами:

1. **Неподвижность положения вызывающей стороны**: традиционное связывание只能固定 `obj.method(args)` 中的 `obj` 为第一个参数
2. **Сложность связывания нескольких параметров**: 当方法需要接收多个同类型参数时，无法优雅表达
3. **Семантическая неоднозначность каррирования**: 部分应用时难以区分"绑定到哪个位置"

### Цели дизайна: унификация двух парадигм программирования

本设计旨在**统一函数式和 OOP 两种编程视角**:

```yaoxiang
# Функциональная перспектива: явная передача всех параметров
distance(p1, p2)

# ООП перспектива: неявное this
p1.distance(p2)

# [positions] 语法糖让两种写法等价，本质都是函数调用
Point.distance = distance[0]   # this 绑定到第 0 位
```

**Основные преимущества**:
- 底层是函数，上层是方法语法
- 不引入 `self` 关键字，保持语言简洁性
- 完全函数化：方法调用本质是参数传递
- `[0]`, `[1]`, `[-1]` 灵活控制 this 绑定位置
- **语法统一**: функциональное определение использует формат `name: (params) -> Return = body`

### Текущие проблемы

```yaoxiang
# 现有设计的问题：
Point: Type = { x: Float, y: Float }
Vector: Type = { x: Float, y: Float, z: Float }

distance: (a: Point, b: Point) -> Float = { ... }
transform: (p: Point, v: Vector) -> Point = { ... }

# 只能绑定到第一个参数
Point.distance = distance  # 等价于 distance[0]
# p1.distance(p2) → distance(p1, p2) ✓

# 但如果 transform 的签名是 transform(Vector, Point) 呢？
# 无法表达 p1.transform(v1) → transform(v1, p1) 的语义
```

## Предложение

### Основной дизайн: связывание по умолчанию + необязательное указание позиции

#### Привязка по умолчанию к первой позиции с совпадающим типом

**Поведение по умолчанию**: `Type.method = function` автоматически ищет первую позицию, совпадающую с этим типом, и выполняет привязку

```yaoxiang
# Связывание по умолчанию первой позиции с совпадающим типом
Point.distance = distance           # Компилятор自动查找第一个 Point 参数位置
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

**Обработка ошибок связывания**:
- **找不到匹配类型**: 如果函数参数中没有该类型，报错或警告
- **工厂函数模式**: 如果没有参数匹配，可能作为工厂函数使用

```yaoxiang
# 情况1：找不到匹配类型
create_point: () -> Point = { ... }
Point.create = create_point        # 错误：没有 Point 类型参数

# 情况2：工厂函数模式（可选）
Point.create = create_point        # 作为工厂函数，调用：Point.create()
```

**优点**:
- 智能绑定：根据类型自动匹配，符合直觉
- 类型安全：只有类型匹配才绑定，避免错误
- 灵活控制：当默认绑定不是期望行为时，可显式指定位置

#### Автоматическое каррированное связывание

当函数参数数量 > 绑定位置数量时，自动生成 каррированная функция:

```yaoxiang
Point: Type = { x: Float, y: Float }

# 基础函数：3 个参数
scale: (p: Point, factor: Float) -> Point = {
    return Point(p.x * factor, p.y * factor)
}

# 绑定时自动 каррирование
Point.scale = scale[0, 1]   # Point 绑定到第 0、1 位，第 2 位保留

# 调用时自动部分应用
p1 = Point(2.0, 3.0)
scaled = p1.scale(2.0)       # → scale(p1, 2.0) 直接调用
result = scaled              # → Point(4.0, 6.0)

# 链式调用更优雅
result = Point(2.0, 3.0).scale(2.0)  # → Point(4.0, 6.0)
```

### Синтаксис позиционной индексной привязки

Введён синтаксис `[position]` для точного управления связыванием функциональных параметров с типами:

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

### Подробное определение синтаксиса

```
绑定声明 ::= 类型 '.' 标识符 '=' 函数名 '[' 位置列表 ']'

位置列表 ::= 位置 (',' 位置)*
位置     ::= 整数                    # 占位符
           | '_'                    # 跳过此位置（占位符）
           | 整数 '..' 整数         # 位置范围（未来扩展）

函数名   ::= 标识符
类型     ::= 标识符 (泛型参数)?
```

### 内置绑定

Связывание можно записать прямо в определении типа, без отдельного оператора связывания:

```yaoxiang
# 方式1：在类型定义体内直接绑定
Point: Type = {
    x: Float = 0,
    y: Float = 0,
    distance = distance[0]           # 绑定到位置0
}

# 方式2：匿名函数 + 位置绑定
Point: Type = {
    x: Float = 0,
    y: Float = 0,
    distance: ((a: Point, b: Point) -> Float)[0] = ((a, b) => {
        dx = a.x - b.x
        dy = a.y - b.y
        return (dx * dx + dy * dy).sqrt()
    })
}
# 语法：((params) => body)[position]
```

**Каррированная семантика**:
- При связывании `distance = distance[0]` 原函数签名 `(a: Point, b: Point) -> Float`
- 生成 method 签名：`b: Point -> Float`（第0位由调用者填充）

### Примеры использования

```yaoxiang
# === 完整示例 ===

Point: Type = { x: Float, y: Float }
Vector: Type = { x: Float, y: Float, z: Float }

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
Circle: Type = { center: Point, radius: Float }

distance: (a: Circle, b: Circle) -> Float = {
    return a.center.distance(b.center) - a.radius - b.radius
}

# 将距离方法绑定到 Circle 类型
Circle.distance = distance[0, 1]
# 调用：c1.distance(c2) → distance(c1, c2)
```

### 支持 деструктуризации кортежей

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

Coord: Type = { x: Float, y: Float }

# 自动解构绑定：Coord -> (Float, Float)
Coord.describe = process_coordinates[1]
# 使用：coord.describe() → process_coordinates((coord.x, coord.y))
```

### 多返回值绑定

```yaoxiang
# === 多返回值绑定 ===

min_max: (list: List(Int)) -> (Int, Int) = {
    min = list.reduce(Int.MAX, (a, b) => if a < b then a else b)
    max = list.reduce(Int.MIN, (a, b) => if a > b then a else b)
    return (min, max)
}

List.range: (T:Type)->((self: List(T)) -> (T, T)) = min_max[1]
# 使用：(min_val, max_val) = list.range()
```

## Детальный дизайн

### Реализация компилятора

### Правила проверки типов

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

### Поведение во время выполнения

| 场景 | 绑定语法 | 调用 | 转换为 |
|------|---------|------|--------|
| 默认绑定 | `Point.distance = distance` | `p1.distance(p2)` | `distance(p1, p2)` |
| 自动匹配 | `Point.transform = transform` | `p.transform(v)` | `transform(p, v)` |
| 单位置 | `Point.distance = distance[1]` | `p1.distance(p2)` | `distance(p2, p1)` |
| 单位置 | `Point.test = func[-1]` | `p.test(a, b)` | `func(a, b, p)` |
| 自动 каррирование | `Point.scale = scale[0, _]` | `p.scale(2.0)` | `scale(p, 2.0)` |
| 占位符 | `Type.method = func[1, _]` | `obj.method(arg)` | `func(arg, obj)` |

**说明**:
- **默认绑定**: 自动查找第一个类型匹配的位置
- `[0]`: this 绑定到第 0 位（第一个参数）
- `[1]`: this 绑定到第 1 位（第二个参数）
- `[-1]`: this 绑定到最后一位（从末尾计数）

## Компромиссы

### Преимущества

- **智能默认绑定**: 默认绑定第一个类型匹配的位置，无需显式指定 `[positions]`
- **精确控制**: 可以绑定到任意参数位置，灵活度高
- **类型安全**: 编译时完全类型检查，只有类型匹配才绑定
- **语法简洁**: `[position]` 语法直观易懂
- **无 `self` 关键字**: 保持语言简洁性
- **Каррирование友好**: 天然支持 частичное применение и цепочечные вызовы
- **ООП友好**: 自动 каррирование 让 ООП程序员无脑迁移

### Недостатки

- **学习成本**: 需要理解 позиция索引概念
- **编译复杂度**: 绑定解析 и 类型检查 增加 компилятор复杂度
- **调试难度**: 错误信息需要清晰指出绑定位置问题

## Альтернативные решения

| 方案 | 描述 | 为什么不选 |
|------|------|-----------|
| `self` 关键字 | 引入 Python/Rust 风格的 `self` | 违反 YaoXiang 无隐式 `self` 的设计哲学 |
| 命名参数绑定 | 使用命名参数 `func(a=obj)` | 需要修改函数签名定义，增加复杂性 |
| 宏系统 | 用宏实现绑定 | 运行时开销大，类型安全性降低 |
| 运算符重载 | 限制 `self` 在特定位置 | 语法不统一，语义混乱 |

## Стратегия реализации

### Фазы разделения

1. **Фаза 1: 基础绑定** (v0.3)
   - Реализация однопозиционного `[n]` синтаксиса связывания (n от 0, с поддержкой отрицательных чисел)
   - Базовая проверка типов и кодогенерация
   - Покрытие unit-тестами

2. **Фаза 2: 高级特性** (v0.5)
   - Поддержка синтаксиса диапазона `[n..m]`
   - Оптимизация вычисления позиций на этапе компиляции

### Зависимости

- Нет внешних зависимостей
- Не связано напрямую с RFC-001 (обработка ошибок)
- Может быть реализовано независимо

### Риски

- Совместимость с существующим синтаксисом связывания
- Стратегии оптимизации производительности (展开 на этапе компиляции vs поиск во время выполнения)

## Открытые вопросы

Следующие вопросы уже решены в дизайне, записаны в Приложении A:

- ~~索引基准从 0 开始~~ → 已决定：从 0 开始
- ~~负数索引~~ → 已决定：支持
- ~~占位符~~ → 已决定：使用 `_`
- ~~范围语法~~ → 已决定：实现

**Оставшиеся открытые вопросы**:

- [ ] Совместимость с существующим синтаксисом связывания
- [ ] Стратегии оптимизации производительности (展开 на этапе компиляции vs поиск во время выполнения)

---

## Приложения

### Приложение A: Журнал решений по дизайну

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

### Приложение B: Глоссарий

| 术语 | 定义 |
|------|------|
| 绑定位置 | 函数参数列表中的索引位置 |
| 联合绑定 | 将类型绑定到多个参数位置 |
| 部分应用 | 只提供部分参数，返回待完成调用的函数 |
| **统一语法** | **`name: (params) -> Return = body`，参数名在签名中声明** |
| **类型匹配绑定** | **默认绑定逻辑：自动查找第一个与调用者类型匹配的位置** |
| **工厂函数绑定** | **当函数参数中没有匹配类型时，作为构造器使用** |

---

## Ссылки

- [Rust impl 语法](https://doc.rust-lang.org/book/ch05-03-method-syntax.html)
- [Haskell 类型类](https://wiki.haskell.org/Type_class)
- [Kotlin 扩展函数](https://kotlinlang.org/docs/extensions.html)
```