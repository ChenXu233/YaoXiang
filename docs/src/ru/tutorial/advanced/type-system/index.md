---
title: Система типов
---

# Система типов

В базовом руководстве вы научились использовать встроенные типы, такие как `Int`, `String`, `Bool`. В этой главе мы углублённо рассмотрим систему типов YaoXiang и научимся **определять собственные типы**.

## Единая синтаксическая модель

Система типов YaoXiang построена на унифицированном синтаксисе, определённом в RFC-010: **всё является `name: type = value`**.

| Концепция | Синтаксис |
|------|------|
| Переменная | `x: Int = 42` |
| Функция | `add: (a: Int, b: Int) -> Int = a + b` |
| Запись типа | `Point: Type = { x: Float, y: Float }` |
| Интерфейс | `Drawable: Type = { draw: (Surface) -> Void }` |
| Обобщённый тип | `List: (T: Type) -> Type = { ... }` |

Обратите внимание: **определение типа само по себе также является `name: Type = value`**.

## Записи типов

Записи типов (в других языках называемые "структурами") — это наиболее фундаментальный способ организации данных в YaoXiang:

```yaoxiang
// Определение записи типа
Point: Type = { x: Float, y: Float }

// Создание экземпляра
origin = Point(x: 0.0, y: 0.0)
p = Point(x: 3.0, y: 4.0)

// Доступ к полям
print(p.x)  // 3.0
print(p.y)  // 4.0
```

### Значения полей по умолчанию

Поля могут иметь значения по умолчанию, которые можно переопределить при конструировании:

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

Используйте синтаксис `Type.method` для определения методов типа:

```yaoxiang
Point: Type = { x: Float, y: Float }

// Определение метода: синтаксис Point.method
Point.length: (self: Point) -> Float = {
    return (self.x * self.x + self.y * self.y).sqrt()
}

p = Point(x: 3.0, y: 4.0)

// Оба способа вызова эквивалентны
print(Point.length(p))  // 5.0 — функциональный вызов
print(p.length())       // 5.0 — вызов через точку
```

### Автоматическая привязка pub

В пределах одного файла функции, объявленные как `pub`, автоматически привязываются к типам, определённым в том же файле:

```yaoxiang
Point: Type = { x: Float, y: Float }

// pub функции автоматически привязываются к Point
pub distance: (p1: Point, p2: Point) -> Float = {
    dx = p1.x - p2.x
    dy = p1.y - p2.y
    return (dx * dx + dy * dy).sqrt()
}

p1 = Point(x: 0.0, y: 0.0)
p2 = Point(x: 3.0, y: 4.0)

// Автопривязанные методы вызываются через точку
print(p1.distance(p2))  // 5.0
```

## Перечисления типов

Перечисления определяют набор взаимно исключающих вариантов. Варианты без данных записываются строчными буквами, варианты с данными — в функциональном синтаксисе:

```yaoxiang
// Простое перечисление
Color: Type = { red | green | blue }

// Перечисление с данными
Result: (T: Type, E: Type) -> Type = { ok(T) | err(E) }

// Вложенные перечисления
Shape: Type = { circle(Float) | rect(Float, Float) | point }
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

Интерфейсы — это **записи типов, все поля которых являются функциональными типами**. Реализация интерфейса означает включение имени интерфейса в запись:

```yaoxiang
// Определение интерфейса
Drawable: Type = {
    draw: (Surface) -> Void,
    bounding_box: () -> Rect,
}

// Реализация интерфейса: включение имени интерфейса в запись типа
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

Интерфейсы обеспечивают полиморфизм — любой тип, реализующий `Drawable`, можно передать функции, ожидающей `Drawable`.

## Обобщённые типы

Обобщения позволяют писать **определения типов, не привязанные к конкретным типам**:

```yaoxiang
// Обобщённая пара
Pair: (T: Type, U: Type) -> Type = { first: T, second: U }

// Использование
string_pair = Pair(Int, String)(first: 1, second: "hello")
float_pair = Pair(Float, Float)(first: 3.14, second: 2.71)
```

Обобщённые функции:

```yaoxiang
// Обобщённая функция map: применяет функцию к каждому элементу списка
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

| Концепция | Синтаксис | Назначение |
|------|------|------|
| Запись типа | `Point: Type = { x: Float, y: Float }` | Организация связанных данных |
| Перечисление | `Color: Type = { red \| green \| blue }` | Один из нескольких вариантов |
| Интерфейс | `Drawable: Type = { draw: ... }` | Абстракция для полиморфизма |
| Обобщения | `List: (T: Type) -> Type = { ... }` | Параметризация по типу |
| Never | `Never` — системный встроенный нижний тип | Расходящиеся/никогда не возвращающиеся пути кода |
| Методы | `Type.method: (self: Type, ...) -> ...` | Привязка поведения |