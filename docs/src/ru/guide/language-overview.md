```markdown
---
title: Краткий справочник по синтаксису
---

# Краткий справочник по синтаксису

Изучите основной синтаксис YaoXiang за 5 минут. Для углублённого изучения посетите [учебник](/tutorial/).

## Переменные

```yaoxiang
x = 42                    # неизменяемое (по умолчанию)
mut y = 0                 # изменяемое

name: String = "hello"    # явный тип
count: Int = 100          # аннотация типа
```

## Функции

```yaoxiang
# форма выражения (возвращает значение напрямую)
add: (a: Int, b: Int) -> Int = a + b

# форма блока кода (явный return)
factorial: (n: Int) -> Int = {
    if n <= 1 { return 1 }
    return n * factorial(n - 1)
}
```

## Типы

```yaoxiang
# тип записи
type Point = { x: Float, y: Float }
p = Point(x: 1.0, y: 2.0)

# перечисление
type Result(T, E) = ok(T) | err(E)
type Color = red | green | blue

# интерфейс
type Drawable = { draw: (Surface) -> Void }

# обобщённые типы
List: (T: Type) -> Type = { data: Array(T), length: Int }
```

## Управление потоком

```yaoxiang
# if — это выражение
grade = if score >= 90 { "A" } elif score >= 60 { "B" } else { "C" }

# match
result = match value {
    ok(v) => "success: ${v}",
    err(e) => "error: ${e}",
}

# циклы
for i in 0..5 { println(i) }

mut n = 0
while n < 5 { println(n); n = n + 1 }
```

## Структуры данных

```yaoxiang
# список
nums = [1, 2, 3, 4, 5]
first = nums[0]           # 1

# словарь
scores = {"Alice": 90, "Bob": 85}
a = scores["Alice"]       # 90

# множество
colors = {"red", "green", "blue"}

# генератор списка
evens = [x for x in nums if x % 2 == 0]
```

## Сопоставление с образцом

```yaoxiang
match shape {
    circle(r) => pi * r * r,
    rect(w, h) => w * h,
    point => 0,
}

# шаблон структуры
match p {
    { x: 0, y: 0 } => "origin",
    { x, y } => "(${x}, ${y})",
}

# сторожевые выражения
match age {
    adult(n) if n >= 18 => true,
    _ => false,
}
```

## Lambda

```yaoxiang
double = (x) => x * 2
add = (a, b) => a + b
apply = (list, op) => [op(x) for x in list]
```

## F-string

```yaoxiang
name = "YaoXiang"
println(f"Hello {name}")          # Hello YaoXiang
println(f"Sum: {10 + 20}")        # Sum: 30
println(f"Pi: {pi:.2f}")          # Pi: 3.14
```

## Модули

```yaoxiang
use std.io
use std.math

println("hello")
result = math.sqrt(16)    # 4.0
```

## Владение

```yaoxiang
# Move: передача владения по умолчанию
p1 = Point(1.0, 2.0)
p2 = p1                   # p1 перемещён

# ref: совместное владение
shared = ref data         # компилятор автоматически выбирает Rc/Arc

# clone: явное глубокое копирование
backup = data.clone()
```

## Конкурентность

```yaoxiang
# функции, помеченные spawn, автоматически асинхронны
fetch_data: (url: String) -> JSON spawn = {
    HTTP.get(url).json()
}

# автоматически параллельно, без await
user = fetch_user(1)
posts = fetch_posts()
```
```