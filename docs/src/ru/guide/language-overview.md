---
title: Краткий справочник по синтаксису
---

# Краткий справочник по синтаксису

Основные концепции YaoXiang за 5 минут. Для углублённого изучения посетите [руководство](/tutorial/).

## Переменные

```yaoxiang
x = 42                    // Неизменяемые (по умолчанию)
mut y = 0                 // Изменяемые

name: String = "hello"    // Явный тип
count: Int = 100          // Аннотация типа

pub version = "1.0"       // Публичный экспорт
```

## Функции

Всё есть `name: type = value`. Функции тоже являются значениями.

```yaoxiang
// Выражение (возвращает значение напрямую)
add: (a: Int, b: Int) -> Int = a + b

// Блок кода (явный return)
factorial: (n: Int) -> Int = {
    if n <= 1 { return 1 }
    return n * factorial(n - 1)
}

// Lambda (имя параметра можно опустить при полной сигнатуре)
double = (x) => x * 2
add = (a, b) => a + b
inc = x => x + 1            // Однопараметрный можно без скобок

// В блоке кода нужен return
process: (x: Int) -> Int = {
    a = x * 2
    b = a + 1
    return b
}

// Void функции не нуждаются в return
greet: (name: String) -> Void = {
    io.println("Hello, " + name)
}
```

## Типы

Нет ключевых слов `type`, `struct`, `trait`, `impl`. Всё решается единым объявлением.

```yaoxiang
// Тип записи
Point: Type = { x: Float, y: Float }
p = Point(1.0, 2.0)            // Позиционные аргументы
p = Point(x=1.0, y=2.0)        // Именованные аргументы

// Поля со значениями по умолчанию
Point: Type = { x: Float = 0, y: Float = 0 }
Point()                        // OK: x=0, y=0
Point(x=1.0)                   // OK: x=1.0, y=0

// Вариантный тип (перечисление)
Color: Type = { red: () -> Color, green: () -> Color, blue: () -> Color }

Option: (T: Type) -> Type = { some: (T) -> Option(T), none: () -> Option(T) }
Result: (T: Type, E: Type) -> Type = { ok: (T) -> Result(T, E), err: (E) -> Result(T, E) }

// Интерфейс (тип записи, где все поля — функции)
Drawable: Type = { draw: (Surface) -> Void }

// Композиция интерфейсов
DrawableSerializable: Type = Drawable & Serializable

// Объявление реализации интерфейса внутри типа
Circle: Type = {
    radius: Float,
    Drawable,              // Реализует интерфейс Drawable
    Serializable,          // Реализует интерфейс Serializable
}

// Обобщённый тип
List: (T: Type) -> Type = {
    data: Array(T),
    length: Int,
    push: (self: List(T), item: T) -> Void,
    map: (R: Type) -> ((self: List(T), f: (T) -> R) -> List(R)),
}

// Обобщённые ограничения
clone: (T: Clone)(value: T) -> T = value.clone()
sort: (T: Clone + PartialOrd)(list: List(T)) -> List(T)
```

## Методы

```yaoxiang
// Namespace-функция (Type.method — лишь маркер принадлежности, не связывание)
Point.distance: (a: &Point, b: &Point) -> Float = {
    dx = a.x - b.x
    dy = a.y - b.y
    return (dx * dx + dy * dy).sqrt()
}

// После явного связывания появляется синтаксис .
Point.distance = distance[0]
// Теперь p1.distance(p2) → distance(p1, p2)

// Быстрое определение + связывание
Point.draw: (self: &Point, surface: Surface) -> Void = {
    surface.plot(self.x, self.y)
}
```

## Управление потоком

```yaoxiang
// if — выражение
grade = if score >= 90 { "A" } else if score >= 60 { "B" } else { "C" }

// match
result = match value {
    ok(v) => "success: {v}",
    err(e) => "error: {e}",
    _ => "unknown",
}

// Циклы
for i in 0..5 { io.println(i) }
for item in items { io.println(item) }

mut n = 0
while n < 5 { io.println(n); n = n + 1 }
```

## Структуры данных

```yaoxiang
// Список
nums = [1, 2, 3, 4, 5]
first = nums[0]           // 1

// Словарь
scores = {"Alice": 90, "Bob": 85}
a = scores["Alice"]       // 90

// List comprehension
evens = [x for x in nums if x % 2 == 0]
doubled = [x * 2 for x in nums]
```

## Pattern matching

```yaoxiang
match shape {
    circle(r) => pi * r * r,
    rect(w, h) => w * h,
    point => 0,
}

// Структурный/кортежный паттерны
match p {
    { x: 0, y: 0 } => "origin",
    { x, y } => "({x}, {y})",
}
match t {
    (0, 0) => "origin",
    (x, y) => "({x}, {y})",
}

// Деструктурирующее присваивание
a, b = (1, 2)              // a=1, b=2

// Guard-выражение
match age {
    n if n >= 18 => true,
    _ => false,
}
```

## Модули и импорт

```yaoxiang
use std.io
use std.math.{sqrt, sin, cos}
use std.{io, list}

io.println("hello")
result = sqrt(16)         // 4.0

// Псевдонимы
use std.math as math
use std.{io as print}

// Публичный экспорт
pub add: (a: Int, b: Int) -> Int = a + b
pub Point: Type = { x: Float, y: Float }
```

## Владение

```yaoxiang
// Move: семантика передачи по умолчанию
p1 = Point(1.0, 2.0)
p2 = p1                   // p1 перемещён

// Заимствование &: автоматическое создание токена (без ручного &)
distance: (a: &Point, b: &Point) -> Float = ...
d = distance(p1, p2)      // Компилятор автоматически создаёт токен заимствования

// Изменяемое заимствование &mut
update: (p: &mut Point, x: Float) -> Void = { p.x = x }

// ref: разделяемое владение (компилятор автоматически выбирает Rc/Arc)
shared = ref data

// clone: явное глубокое копирование
backup = data.clone()
```

## Параллелизм

Spawn — единственный примитив параллелизма. Нет async/await, нет Send/Sync.

```yaoxiang
// Spawn-блок: подвыражения автоматически параллельны
result = spawn {
    user = fetch_user(1)
    posts = fetch_posts()
    return (user, posts)
}

// Spawn for: данные параллельны
results = spawn for item in items {
    return process(item)
}

// Spawn + ref: разделяемые данные между задачами
main = {
    shared = ref data
    result = spawn {
        a = shared
        return a
    }
}
```

## F-string

```yaoxiang
name = "YaoXiang"
io.println(f"Hello {name}")          // Hello YaoXiang
io.println(f"Sum: {10 + 20}")        // Sum: 30
io.println(f"Pi: {pi:.2f}")          // Pi: 3.14
```
