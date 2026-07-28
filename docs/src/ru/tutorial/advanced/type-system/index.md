---
title: Система типов
---

# Система типов

В базовом руководстве вы научились использовать встроенные типы `Int`, `String`, `Bool` и другие.
Эта глава углубляется в систему типов YaoXiang, научит вас **определять собственные типы**.

## Единая синтаксическая модель

Система типов YaoXiang построена на едином синтаксисе, определённом в RFC-010: **всё есть
`name: type = value`**.

| Понятие        | Синтаксис                                      |
| -------------- | ---------------------------------------------- |
| Переменная     | `x: Int = 42`                                  |
| Функция        | `add: (a: Int, b: Int) -> Int = a + b`         |
| Тип записи     | `Point: Type = { x: Float, y: Float }`         |
| Интерфейс      | `Drawable: Type = { draw: (Surface) -> Void }` |
| Обобщённый тип | `List: (T: Type) -> Type = { ... }`            |

Обратите внимание: **само определение типа также имеет вид `name: Type = value`**.

## Тип записи

Тип записи (в других языках называется «структурой») — это самый базовый способ организации данных в
YaoXiang:

```yaoxiang
// 定义记录类型
Point: Type = { x: Float, y: Float }

// 创建实例
origin = Point(x: 0.0, y: 0.0)
p = Point(x: 3.0, y: 4.0)

// 访问字段
print(p.x)  // 3.0
print(p.y)  // 4.0
```

### Значения полей по умолчанию

Поля могут иметь значения по умолчанию, указываемые при создании необязательно:

```yaoxiang
User: Type = {
    name: String,
    age: Int = 0,
    active: Bool = true,
}

alice = User(name: "Alice", age: 25)        // active 取默认值 true
bob = User(name: "Bob")                      // age=0, active=true
anonymous = User(name: "guest", active: false)  // age=0
```

### Определение методов

Для определения методов типа используется синтаксис `Type.method`:

```yaoxiang
Point: Type = { x: Float, y: Float }

// 定义方法：Point.method 语法
Point.length: (self: Point) -> Float = {
    return (self.x * self.x + self.y * self.y).sqrt()
}

p = Point(x: 3.0, y: 4.0)

// 两种调用方式等价
print(Point.length(p))  // 5.0 — 函数式调用
print(p.length())       // 5.0 — .调用语法
```

### Автоматическая привязка pub

В одном файле функции, объявленные с `pub`, автоматически привязываются к типам, определённым в этом
же файле:

```yaoxiang
Point: Type = { x: Float, y: Float }

// pub 函数自动绑定到 Point
pub distance: (p1: Point, p2: Point) -> Float = {
    dx = p1.x - p2.x
    dy = p1.y - p2.y
    return (dx * dx + dy * dy).sqrt()
}

p1 = Point(x: 0.0, y: 0.0)
p2 = Point(x: 3.0, y: 4.0)

// 自动绑定的方法用 . 调用
print(p1.distance(p2))  // 5.0
```

## Тип перечисления

Перечисление определяет набор взаимоисключающих вариантов. Варианты без данных записываются в нижнем
регистре, варианты с данными — в функциональном синтаксисе:

```yaoxiang
// 简单枚举
Color: Type = { red: () -> Color, green: () -> Color, blue: () -> Color }

// 带数据的枚举
Result: (T: Type, E: Type) -> Type = { ok: (T) -> Result(T, E), err: (E) -> Result(T, E) }

// 嵌套枚举
Shape: Type = { circle: (Float) -> Shape, rect: (Float, Float) -> Shape, point: () -> Shape }
```

Ключевая идея перечислений: **каждый вариант сам по себе также является типом**.

```yaoxiang
area: (s: Shape) -> Float = match s {
    circle(r) => 3.14159 * r * r,
    rect(w, h) => w * h,
    point => 0,
}

print(area(circle(5.0)))    // 78.53975
print(area(rect(3.0, 4.0))) // 12.0
```

## Интерфейс

Интерфейс — это **тип записи, все поля которого имеют функциональный тип**. Реализация интерфейса
означает, что запись содержит имя интерфейса:

```yaoxiang
// 定义接口
Drawable: Type = {
    draw: (Surface) -> Void,
    bounding_box: () -> Rect,
}

// 实现接口：在记录类型中包含接口名
Circle: Type = {
    x: Float,
    y: Float,
    radius: Float,
    Drawable,       // 实现 Drawable 接口
}

// 提供接口要求的方法
Circle.draw: (self: Circle, surface: Surface) -> Void = {
    surface.draw_circle(self.x, self.y, self.radius)
}

Circle.bounding_box: (self: Circle) -> Rect = {
    return Rect(
        x: self.x - self.radius,
        y: self.y - self.radius,
        width: self.radius * 2.0,
        height: self.radius * 2.0,
    )
}
```

Интерфейс обеспечивает полиморфизм — любой тип, реализующий `Drawable`, может быть передан функции,
принимающей `Drawable`.

## Обобщённые типы

Обобщения позволяют писать определения типов, **не привязанные к конкретному типу**:

```yaoxiang
// 泛型 Pair
Pair: (T: Type, U: Type) -> Type = { first: T, second: U }

// 使用
string_pair = Pair(Int, String)(first: 1, second: "hello")
float_pair = Pair(Float, Float)(first: 3.14, second: 2.71)
```

Обобщённые функции:

```yaoxiang
// 泛型 map：对列表的每个元素应用函数
map: (T: Type, R: Type) -> ((list: List(T), f: (T) -> R) -> List(R)) = {
    mut result: List(R) = []
    for item in list {
        result.append(f(item))
    }
    return result
}

numbers = [1, 2, 3, 4]
doubled = map(Int, Int)(numbers, (x) => x * 2)
print(doubled)  // [2, 4, 6, 8]
```

## Итоги

| Понятие      | Синтаксис                                                                   | Назначение                                       |
| ------------ | --------------------------------------------------------------------------- | ------------------------------------------------ |
| Тип записи   | `Point: Type = { x: Float, y: Float }`                                      | Организация связанных данных                     |
| Перечисление | `Color: Type = { red: () -> Color, green: () -> Color, blue: () -> Color }` | Выбор одного из вариантов                        |
| Интерфейс    | `Drawable: Type = { draw: ... }`                                            | Полиморфная абстракция                           |
| Обобщения    | `List: (T: Type) -> Type = { ... }`                                         | Параметризация типов                             |
| Never        | `Never` — встроенный нижний тип системы                                     | Расходящиеся/никогда не возвращающиеся пути кода |
| Метод        | `Type.method: (self: Type, ...) -> ...`                                     | Присоединение поведения                          |
