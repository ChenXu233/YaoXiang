# Быстрый старт YaoXiang

> Это руководство поможет вам быстро освоить язык программирования YaoXiang.
>
> **Примечание**: Примеры кода в этом документе написаны в соответствии со спецификацией языка YaoXiang. Если вы обнаружите различия в синтаксисе при фактическом запуске, обратитесь к [спецификации языка](../design/language-spec.md).

## Установка

### Компиляция из исходного кода (рекомендуется)

```bash
# Клонирование репозитория
git clone https://github.com/yourusername/yaoxiang.git
cd yaoxiang

# Компиляция (отладочная версия, для разработки и тестирования)
cargo build

# Компиляция (релизная версия, рекомендуется для продакшена)
cargo build --release

# Запуск тестов
cargo test

# Проверка версии
./target/debug/yaoxiang --version
# или
./target/release/yaoxiang --version
```

**Проверка успешной установки**:
```bash
./target/debug/yaoxiang --version
# Должно вывести: yaoxiang x.y.z
```

## Первая программа

Создайте файл `hello.yx`:

```yaoxiang
# hello.yx
use std.io

# Определение функции: name: (param: Type, ...) -> return_type = { ... }
main: () -> Void = {
    println("Hello, YaoXiang!")
}
```

Запуск:

```bash
./target/debug/yaoxiang hello.yx
# или с релизной версией
./target/release/yaoxiang hello.yx
```

Вывод:

```
Hello, YaoXiang!
```

## Основные концепции

### Переменные и типы

```yaoxiang
# Автоматический вывод типов
x = 42                    # Выводится как Int
name = "YaoXiang"         # Выводится как String
pi = 3.14159              # Выводится как Float
is_valid = true           # Выводится как Bool

# Явные аннотации типов (рекомендуется использовать централизованные соглашения)
count: Int = 100

# По умолчанию неизменяемо (функция безопасности)
x = 10
x = 20                    # ❌ Ошибка компиляции! Неизменяемо

# Изменяемые переменные (требуют явного объявления)
mut counter = 0
counter = counter + 1     # ✅ OK
```

### Функции

```yaoxiang
# Синтаксис определения функции
add: (a: Int, b: Int) -> Int = a + b

# Вызов
result = add(1, 2)        # result = 3

# Функция с одним параметром
inc: (x: Int) -> Int = x + 1
```

### Определение типов

YaoXiang использует унифицированную модель синтаксиса `name: type = value`:

```yaoxiang
# Объявление переменной
x: Int = 42
name: String = "YaoXiang"

# Определение функции
add: (a: Int, b: Int) -> Int = a + b

# Определение типа (с использованием фигурных скобок)
type Point = { x: Float, y: Float }

# Использование типа
p: Point = Point(x: 1.0, y: 2.0)
p.x  # 1.0
p.y  # 2.0
```

#### Типы записей

```yaoxiang
# Структурные типы
type Point = { x: Float, y: Float }
type Rect = { x: Float, y: Float, width: Float, height: Float }

# Использование
p = Point(x: 3.0, y: 4.0)
r = Rect(x: 0.0, y: 0.0, width: 10.0, height: 20.0)
```

#### Определение интерфейсов

Интерфейсы — это типы записей, все поля которых являются функциональными типами:

```yaoxiang
# Определение интерфейса
type Drawable = {
    draw: (Surface) -> Void,
    bounding_box: () -> Rect
}

type Serializable = {
    serialize: () -> String
}

# Пустой интерфейс
type EmptyInterface = {}
```

#### Методы типов

Определение методов типов с использованием синтаксиса `Type.method: (Type, ...) -> Return = ...`:

```yaoxiang
# Определение типа
type Point = { x: Float, y: Float }

# Определение метода типа
Point.draw: (self: Point, surface: Surface) -> Void = {
    surface.plot(self.x, self.y)
}

Point.serialize: (self: Point) -> String = {
    "Point(${self.x}, ${self.y})"
}

# Использование методов (синтаксический сахар)
p = Point(x: 1.0, y: 2.0)
p.draw(screen)           # → Point.draw(p, screen)
str = p.serialize()      # → Point.serialize(p)
```

#### Автоматическое связывание

Функции, объявленные с ключевым словом `pub`, автоматически привязываются к типам, определённым в том же файле:

```yaoxiang
type Point = { x: Float, y: Float }

# Объявление pub автоматически привязывается к Point
pub distance: (p1: Point, p2: Point) -> Float = {
    dx = p1.x - p2.x
    dy = p1.y - p2.y
    (dx * dx + dy * dy).sqrt()
}

# Использование
p1 = Point(x: 3.0, y: 4.0)
p2 = Point(x: 1.0, y: 2.0)

# Функциональный вызов
d = distance(p1, p2)           # 3.606...

# ООП синтаксический сахар (автоматическая привязка к Point.distance)
d2 = p1.distance(p2)           # → distance(p1, p2)
```

#### Типы перечислений

```yaoxiang
# Простое перечисление
type Color = red | green | blue

# Перечисление с данными
Result: (T: Type, E: Type) -> Type = ok(T) | err(E)

# Использование обобщений
success: Result(Int, String) = ok(42)
failure: Result(Int, String) = err("not found")
```

#### Обобщённые типы

```yaoxiang
# Определение обобщённого типа
List: (T: Type) -> Type = {
    data: Array(T),
    length: Int,
    push: (List(T), T) -> Void
}

# Конкретная реализация
type IntList = List(Int)
type StringList = List(String)
```

### Управление потоком выполнения

```yaoxiang
# Условные выражения
if x > 0 {
    "positive"
} elif x == 0 {
    "zero"
} else {
    "negative"
}

# Циклы
for i in 0..5 {
    print(i)
}

# Цикл while
mut n = 0
while n < 5 {
    print(n)
    n = n + 1
}
```

### Списки и словари

```yaoxiang
# Списки
numbers = [1, 2, 3, 4, 5]
first = numbers[0]         # 1

# Словари
scores = {"Alice": 90, "Bob": 85}
alice_score = scores["Alice"]  # 90

# Добавление элемента
mut list = [1, 2, 3]
list.append(4)
```

### Сопоставление с образцом

```yaoxiang
# Выражение match
result: Result(Int, String) = ok(42)

message = match result {
    ok(value) => "Success: " + value.to_string()
    err(error) => "Error: " + error
}
```

## Параллельное программирование (асинхронное)

Уникальная особенность YaoXiang: функции, помеченные `spawn`, автоматически получают асинхронные возможности.

```yaoxiang
# Определение параллельной функции (автоматическое асинхронное выполнение)
fetch_data: (url: String) -> JSON spawn = {
    HTTP.get(url).json()
}

# Вызов параллельной функции (автоматический параллелизм, без await)
main: () -> Void = {
    # Два вызова выполняются автоматически параллельно
    user = fetch_user(1)     # Автоматический параллелизм
    posts = fetch_posts()    # Автоматический параллелизм

    # Автоматическое ожидание при необходимости получения результата
    print(user.name)
    print(posts.length)
}
```

## Система модулей

```yaoxiang
# Импорт стандартной библиотеки
use std.io
use std.math

# Использование импортированных функций
result = math.sqrt(16)      # 4.0
println("Hello!")
```

## Часто задаваемые вопросы

### В: Переменные по умолчанию неизменяемы, как изменить переменную?

```yaoxiang
# Используйте ключевое слово mut для объявления изменяемой переменной
mut x = 10
x = 20                       # ✅ OK
```

### В: Как определить функцию?

```yaoxiang
# Полная форма (рекомендуется)
add: (a: Int, b: Int) -> Int = a + b

# Краткая форма (с выводом типа)
add = (a, b) => a + b
```

### В: Как обрабатывать ошибки?

```yaoxiang
# Использование типа Result
Result: (T: Type, E: Type) -> Type = ok(T) | err(E)

# Обработка с помощью сопоставления с образцом
result = risky_operation()
match result {
    ok(value) => print("Success: " + value)
    err(error) => print("Error: " + error)
}
```

## Следующие шаги

- 📖 Прочтите [Руководство по YaoXiang](../YaoXiang-book.md), чтобы узнать о основных возможностях
- 📚 Ознакомьтесь со [спецификацией языка](../YaoXiang-language-specification.md), чтобы узнать полный синтаксис
- 🏗️ Изучите [архитектурную документацию](../architecture/), чтобы понять детали реализации
- 💡 Прочитайте [манифест дизайна](../YaoXiang-design-manifesto.md), чтобы понять основные принципы

## Связанные ресурсы

- [GitHub репозиторий](https://github.com/yourusername/yaoxiang)
- [Сообщения об ошибках](https://github.com/yourusername/yaoxiang/issues)
- [Руководство по содействию](../guides/dev/)