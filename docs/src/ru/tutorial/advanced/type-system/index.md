---
title: Типовая система
---

# Типовая система

В базовом руководстве вы научились использовать встроенные типы `Int`, `String`, `Bool` и другие. В этой главе мы углублённо рассмотрим типовую систему YaoXiang и научимся **определять собственные типы**.

## Унифицированная синтаксическая модель

Типовая система YaoXiang основана на унифицированном синтаксисе, определённом в RFC-010: **всё является `name: type = value`**.

| Концепция     | Запись                                           |
| ------------- | ------------------------------------------------ |
| Переменная    | `x: Int = 42`                                    |
| Функция       | `add: (a: Int, b: Int) -> Int = a + b`           |
| Записевой тип | `Point: Type = { x: Float, y: Float }`          |
| Интерфейс     | `Drawable: Type = { draw: (Surface) -> Void }`  |
| Обобщённый тип| `List: (T: Type) -> Type = { ... }`              |

Обратите внимание: **определение типа также является `name: Type = value`**.

## Записевые типы

Записевые типы (в других языках называемые "структурами") являются наиболее базовым способом организации данных в YaoXiang:

```yaoxiang
// Определение записевого типа
Point: Type = { x: Float, y: Float }

// Создание экземпляра
origin = Point(x: 0.0, y: 0.0)
p = Point(x: 3.0, y: 4.0)

// Доступ к полям
print(p.x)  // 3.0
print(p.y)  // 4.0
```

### Значения полей по умолчанию

Для полей можно указать значения по умолчанию, которые можно не предоставлять при конструировании:

```yaoxiang
User: Type = {
    name: String,
    age: Int = 0,
    active: Bool = true,
}

alice = User(name: "Alice", age: 25)        // active принимает значение по умолчанию true
bob = User(name: "Bob")                      // age=0, active=true
anonymous = User(name: "guest", active: false)  // age=0
```

### Определение методов

Для определения методов типа используется синтаксис `Type.method`:

```yaoxiang
Point: Type = { x: Float, y: Float }

// Определение метода: синтаксис Point.method
Point.length: (self: Point) -> Float = {
    return (self.x * self.x + self.y * self.y).sqrt()
}

p = Point(x: 3.0, y: 4.0)

// Оба способа вызова эквивалентны
print(Point.length(p))  // 5.0 — функциональный вызов
print(p.length())       // 5.0 — синтаксис с .
```

### Автоматическое связывание `pub`

В одном файле функции, объявленные как `pub`, автоматически связываются с типами, определёнными в том же файле:

```yaoxiang
Point: Type = { x: Float, y: Float }

// pub функция автоматически связывается с Point
pub distance: (p1: Point, p2: Point) -> Float = {
    dx = p1.x - p2.x
    dy = p1.y - p2.y
    return (dx * dx + dy * dy).sqrt()
}

p1 = Point(x: 0.0, y: 0.0)
p2 = Point(x: 3.0, y: 4.0)

// Автоматически связанные методы вызываются через .
print(p1.distance(p2))  // 5.0
```

## Перечислимые типы

Перечисления определяют набор взаимоисключающих вариантов. Варианты без данных пишутся строчными буквами, варианты с данными используют функциональный синтаксис:

```yaoxiang
// Простое перечисление
Color: Type = { red: () -> Color, green: () -> Color, blue: () -> Color }

// Перечисление с данными
Result: (T: Type, E: Type) -> Type = { ok: (T) -> Result(T, E), err: (E) -> Result(T, E) }

// Вложенные перечисления
Shape: Type = { circle: (Float) -> Shape, rect: (Float, Float) -> Shape, point: () -> Shape }
```

Ключевая идея перечислений: **каждый вариант сам по себе является типом**.

```yaoxiang
area: (s: Shape) -> Float = match s {
    circle(r) => 3.14159 * r * r,
    rect(w, h) => w * h,
    point => 0,
}

print(area(circle(5.0)))    // 78.53975
print(area(rect(3.0, 4.0))) // 12.0
```

## Интерфейсы

Интерфейсы — это **записевые типы, все поля которых являются функциональными типами**. Реализация интерфейса заключается в том, чтобы запись содержала имя этого интерфейса:

```yaoxiang
// Определение интерфейса
Drawable: Type = {
    draw: (Surface) -> Void,
    bounding_box: () -> Rect,
}

// Реализация интерфейса: включение имени интерфейса в записавой тип
Circle: Type = {
    x: Float,
    y: Float,
    radius: Float,
    Drawable,       // Реализация интерфейса Drawable
}

// Предоставление методов, требуемых интерфейсом
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

Интерфейсы обеспечивают полиморфизм — любой тип, реализующий `Drawable`, может быть передан функции, принимающей `Drawable`.

## Обобщённые типы

Обобщения позволяют записывать определения типов **без привязки к конкретным типам**:

```yaoxiang
// Обобщённая пара Pair
Pair: (T: Type, U: Type) -> Type = { first: T, second: U }

// Использование
string_pair = Pair(Int, String)(first: 1, second: "hello")
float_pair = Pair(Float, Float)(first: 3.14, second: 2.71)
```

Обобщённые функции:

```yaoxiang
// Обобщённая функция map: применение функции к каждому элементу списка
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

| Концепция      | Синтаксис                                                                       | Назначение                  |
| -------------- | ------------------------------------------------------------------------------- | --------------------------- |
| Записевой тип  | `Point: Type = { x: Float, y: Float }`                                          | Организация связанных данных|
| Перечисление   | `Color: Type = { red: () -> Color, green: () -> Color, blue: () -> Color }`     | Один из многих              |
| Интерфейс      | `Drawable: Type = { draw: ... }`                                                | Полиморфная абстракция      |
| Обобщение      | `List: (T: Type) -> Type = { ... }`                                             | Параметризация по типу      |
| Never          | `Never` — это встроенный базовый тип системы                                    | Расходящийся/никогда не возвращающийся путь кода |
| Метод          | `Type.method: (self: Type, ...) -> ...`                                         | Привязка поведения          |
