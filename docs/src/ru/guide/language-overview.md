---
title: Шпаргалка по синтаксису
---

# Шпаргалка по синтаксису

Освойте основной синтаксис YaoXiang за 5 минут. Для углублённого изучения посетите [учебник](/tutorial/).

## Переменные

```yaoxiang
x = 42                    // неизменяемая (по умолчанию)
mut y = 0                 // мутабельная

name: String = "hello"    // явный тип
count: Int = 100          // аннотация типа

pub version = "1.0"       // публичный экспорт
```

## Функции

Всё есть `name: type = value`. Функция — тоже значение.

```yaoxiang
// Форма выражения (возвращает значение напрямую)
add: (a: Int, b: Int) -> Int = a + b

// Форма блока кода (явный return)
factorial: (n: Int) -> Int = {
    if n <= 1 { return 1 }
    return n * factorial(n - 1)
}

// Лямбда (при полной сигнатуре имена параметров можно опустить)
double = (x) => x * 2
add = (a, b) => a + b
inc = x => x + 1            // для одного параметра скобки можно опустить

// Внутри блока кода нужен return
process: (x: Int) -> Int = {
    a = x * 2
    b = a + 1
    return b
}

// Функции с Void не требуют return
greet: (name: String) -> Void = {
    io.println("Hello, " + name)
}
```

## Типы

Никаких ключевых слов `type`, `struct`, `trait`, `impl`. Одно универсальное объявление на все случаи.

```yaoxiang
// Тип записи
Point: Type = { x: Float, y: Float }
p = Point(1.0, 2.0)            // позиционные параметры
p = Point(x=1.0, y=2.0)        // именованные параметры

// Поля со значениями по умолчанию
Point: Type = { x: Float = 0, y: Float = 0 }
Point()                        // OK: x=0, y=0
Point(x=1.0)                   // OK: x=1.0, y=0

// Вариантный тип (перечисление)
Color: Type = { red | green | blue }

Option: (T: Type) -> Type = { some(T) | none }
Result: (T: Type, E: Type) -> Type = { ok(T) | err(E) }

// Интерфейс (тип записи, все поля которой имеют функциональный тип)
Drawable: Type = { draw: (Surface) -> Void }

// Композиция интерфейсов
DrawableSerializable: Type = Drawable & Serializable

// Объявление реализации интерфейса внутри типа
Circle: Type = {
    radius: Float,
    Drawable,              // реализует интерфейс Drawable
    Serializable,          // реализует интерфейс Serializable
}

// Дженерики
List: (T: Type) -> Type = {
    data: Array(T),
    length: Int,
    push: (self: List(T), item: T) -> Void,
    map: (R: Type) -> ((self: List(T), f: (T) -> R) -> List(R)),
}

// Ограничения дженериков
clone: (T: Clone)(value: T) -> T = value.clone()
sort: (T: Clone + PartialOrd)(list: List(T)) -> List(T)
```

## Методы

```yaoxiang
// Функция в пространстве имён (Type.method — лишь метка принадлежности, а не привязка)
Point.distance: (a: &Point, b: &Point) -> Float = {
    dx = a.x - b.x
    dy = a.y - b.y
    return (dx * dx + dy * dy).sqrt()
}

// Синтаксис вызова через . появляется только после явной привязки
Point.distance = distance[0]
// После этого p1.distance(p2) → distance(p1, p2)

// Быстрое определение + привязка
Point.draw: (self: &Point, surface: Surface) -> Void = {
    surface.plot(self.x, self.y)
}
```

## Управление потоком

```yaoxiang
// if — это выражение
grade = if score >= 90 { "A" } elif score >= 60 { "B" } else { "C" }

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

// Генератор списка
evens = [x for x in nums if x % 2 == 0]
doubled = [x * 2 for x in nums]
```

## Сопоставление с образцом

```yaoxiang
match shape {
    circle(r) => pi * r * r,
    rect(w, h) => w * h,
    point => 0,
}

// Образцы структур / кортежей
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

// Защитные выражения
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
// Move: по умолчанию — передача владения
p1 = Point(1.0, 2.0)
p2 = p1                   // p1 перемещён

// Заимствование &: токен создаётся автоматически (& указывать вручную не нужно)
distance: (a: &Point, b: &Point) -> Float = ...
d = distance(p1, p2)      // компилятор автоматически создаёт токен заимствования

// Мутабельное заимствование &mut
update: (p: &mut Point, x: Float) -> Void = { p.x = x }

// ref: разделяемое владение (компилятор сам выбирает Rc/Arc)
shared = ref data

// clone: явное глубокое копирование
backup = data.clone()
```

## Конкурентность

spawn — единственный примитив параллелизма. Никаких async/await, никаких Send/Sync.

```yaoxiang
// spawn-блок: подвыражения выполняются параллельно автоматически
result = spawn {
    user = fetch_user(1)
    posts = fetch_posts()
    return (user, posts)
}

// spawn for: параллелизм по данным
results = spawn for item in items {
    return process(item)
}

// spawn + ref: разделение данных между задачами
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