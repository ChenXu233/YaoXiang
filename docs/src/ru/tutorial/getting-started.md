# YaoXiang: быстрый старт

> Это руководство поможет вам быстро освоить язык программирования YaoXiang.
>
> **Внимание**: примеры кода в этом документе написаны на основе спецификации языка YaoXiang. Если
> при фактическом выполнении вы столкнётесь с синтаксическими различиями, обратитесь к
> [спецификации языка](../reference/language-spec/index.md).

## Установка

### Сборка из исходного кода (рекомендуется)

```bash
# Клонировать репозиторий
git clone https://github.com/ChenXu233/YaoXiang.git
cd yaoxiang

# Сборка (отладочная версия, для разработки и тестирования)
cargo build

# Сборка (релизная версия, рекомендуется для продакшена)
cargo build --release

# Запуск тестов
cargo test

# Просмотр версии
./target/debug/yaoxiang --version
# или
./target/release/yaoxiang --version
```

**Проверка успешной установки**:

```bash
./target/debug/yaoxiang --version
# Должно вывести что-то вроде: yaoxiang x.y.z
```

## Первая программа

Создайте файл `hello.yx`:

```yaoxiang
// hello.yx
use std.io

// Определение функции: name: (param: Type, ...) -> return_type = { return ... }  # В блоке кода требуется явный return
// Форма выражения: name: (param: Type, ...) -> return_type = expr           # Выражение возвращает значение напрямую
main: () -> Void = {
    print("Hello, YaoXiang!")
}
```

Запуск:

```bash
./target/debug/yaoxiang hello.yx
# или релизная версия
./target/release/yaoxiang hello.yx
```

Вывод:

```
Hello, YaoXiang!
```

## Основные концепции

### Переменные и типы

```yaoxiang
// Автоматический вывод типов
x = 42  // выводится как Int
name = "YaoXiang"  // выводится как String
pi = 3.14159  // выводится как Float
is_valid = true  // выводится как Bool

// Явные аннотации типов (рекомендуется использовать соглашение о централизованных типах)
count: Int = 100

// По умолчанию неизменяемые (свойство безопасности)
x = 10
x = 20  // ❌ Ошибка компиляции! Неизменяемое

// Изменяемые переменные (требуется явное объявление)
mut counter = 0
counter = counter + 1  // ✅ OK
```

### Функции

```yaoxiang
// Синтаксис определения функции
// Форма выражения: возвращает значение напрямую, без return
add: (a: Int, b: Int) -> Int = a + b

// Форма блока кода: требуется return для возврата значения
// add: (a: Int, b: Int) -> Int = { return a + b }

// Вызов
result = add(1, 2)  // result = 3

// Функция с одним параметром (форма выражения)
inc: (x: Int) -> Int = x + 1
```

### Определение типов

YaoXiang использует унифицированную синтаксическую модель `name: type = value`:

```yaoxiang
// Объявление переменной
x: Int = 42
name: String = "YaoXiang"

// Определение функции
add: (a: Int, b: Int) -> Int = a + b

// Определение типа (с использованием фигурных скобок)
Point: Type = { x: Float, y: Float }

// Использование типа
p: Point = Point(x=1.0, y=2.0)
p.x  // 1.0
p.y  // 2.0
```

#### Record type

```yaoxiang
// Тип-структура
Point: Type = { x: Float, y: Float }
Rect: Type = { x: Float, y: Float, width: Float, height: Float }

// Использование
p = Point(x=3.0, y=4.0)
r = Rect(x=0.0, y=0.0, width=10.0, height=20.0)
```

#### Определение интерфейса

Интерфейс — это record type, все поля которого имеют function type:

```yaoxiang
// Определение интерфейса
Drawable: Type = {
    draw: (Surface) -> Void,
    bounding_box: () -> Rect
}

Serializable: Type = {
    serialize: () -> String
}

// Пустой интерфейс
EmptyInterface: Type = {}
```

#### Методы типа

Для определения методов типа используется синтаксис `Type.method: (Type, ...) -> Return = ...`:

```yaoxiang
// Определение типа
Point: Type = { x: Float, y: Float }

// Определение метода типа
Point.draw: (self: Point, surface: Surface) -> Void = {
    surface.plot(self.x, self.y)
}

Point.serialize: (self: Point) -> String = {
    "Point({self.x}, {self.y})"
}

// Использование метода (синтаксический сахар)
p = Point(x=1.0, y=2.0)
p.draw(screen)  // → Point.draw(p, screen)
str = p.serialize()  // → Point.serialize(p)
```

#### Автоматическое связывание

Функции, объявленные с ключевым словом `pub`, автоматически привязываются к типам, определённым в
том же файле:

```yaoxiang
Point: Type = { x: Float, y: Float }

// pub объявление автоматически привязывается к Point
pub distance: (p1: Point, p2: Point) -> Float = {
    dx = p1.x - p2.x
    dy = p1.y - p2.y
    (dx * dx + dy * dy).sqrt()
}

// Использование
p1 = Point(x=3.0, y=4.0)
p2 = Point(x=1.0, y=2.0)

// Функциональный вызов
d = distance(p1, p2)  // 3.606...

// Синтаксический сахар ООП (автоматическая привязка к Point.distance)
d2 = p1.distance(p2)  // → distance(p1, p2)
```

#### Enum type

```yaoxiang
// Простой enum
Color: Type = { red: () -> Color, green: () -> Color, blue: () -> Color }

// Enum с данными
Result: (T: Type, E: Type) -> Type = { ok: (T) -> Result(T, E), err: (E) -> Result(T, E) }

// Использование generics
success: Result(Int, String) = ok(42)
failure: Result(Int, String) = err("not found")
```

#### Generics типы

```yaoxiang
// Определение generic-типа
List: (T: Type) -> Type = {
    data: Array(T),
    length: Int,
    push: (List(T), T) -> Void
}

// Конкретная инстанциация
IntList: Type = List(Int)
StringList: Type = List(String)
```

### Управление потоком

```yaoxiang
// Условное выражение
if x > 0 {
    "positive"
} else if x == 0 {
    "zero"
} else {
    "negative"
}

// Цикл for
for i in 0..5 {
    print(i)
}

// Цикл while
mut n = 0
while n < 5 {
    print(n)
    n = n + 1
}
```

### Списки и словари

```yaoxiang
// Список
numbers = [1, 2, 3, 4, 5]
first = numbers[0]  // 1

// Словарь
scores = {"Alice": 90, "Bob": 85}
alice_score = scores["Alice"]  // 90

// Добавление элемента
mut list = [1, 2, 3]
list.append(4)
```

### Pattern matching

```yaoxiang
// Выражение match
result: Result(Int, String) = ok(42)

message = match result {
    ok(value) => "Success: " + value.to_string()
    err(error) => "Error: " + error
}
```

## Параллельное программирование (конкурентность)

Модель конкурентности YaoXiang построена вокруг примитива `spawn <expr>` — это единственная точка
входа в параллелизм.

```yaoxiang
// spawn модифицирует любое выражение, выполняя его параллельно
main: () -> Void = {
    user = spawn fetch_user(1)   // Выполняется в фоне
    posts = spawn fetch_posts()  // Ещё один параллельный шаг

    // При обращении к результату происходит автоматическая блокировка и ожидание
    print(user.name)
    print(posts.length)
}
```

**Ключевое правило**: выражение, модифицированное `spawn`, выполняется в фоне, а внешний синхронный
код блокируется в ожидании результата. Независимые задачи выполняются параллельно автоматически, а
их调度 осуществляется средой исполнения по модели GMP.

## Система модулей

```yaoxiang
// Импорт стандартной библиотеки
use std.io
use std.math

// Использование импортированных функций
result = math.sqrt(16)  // 4.0
print("Hello!")
```

## Часто задаваемые вопросы

### В: Переменные по умолчанию неизменяемы, как их изменять?

```yaoxiang
// Используйте ключевое слово mut для объявления изменяемой переменной
mut x = 10
x = 20  // ✅ OK
```

### В: Как определить функцию?

```yaoxiang
// Полная форма (рекомендуется)
add: (a: Int, b: Int) -> Int = a + b

// Краткая форма (вывод типов)
add = (a, b) => a + b
```

### В: Как обрабатывать ошибки?

```yaoxiang
// Используйте тип Result
Result: (T: Type, E: Type) -> Type = { ok(T) | err(E) }

// Обработка с помощью pattern matching
result = risky_operation()
match result {
    ok(value) => print("Success: " + value)
    err(error) => print("Error: " + error)
}
```

## Следующие шаги

- 📚 Изучите [спецификацию языка](../reference/language-spec/index.md) для полного описания
  синтаксиса
- 🏗️ Ознакомьтесь с [документацией по проектированию](../design/) для了解 деталей реализации
- 💡 Прочитайте [манифест проектирования](../design/manifesto.md) для понимания основных идей

## Связанные ресурсы

- [Репозиторий на GitHub](https://github.com/yourusername/yaoxiang)
- [Сообщить о проблеме](https://github.com/yourusername/yaoxiang/issues)
- [Руководство для контрибьюторов](../dev/contributing.md)
