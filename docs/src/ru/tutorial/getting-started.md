# YaoXiang — быстрый старт

> Это руководство поможет вам быстро освоить язык программирования YaoXiang.
>
> **Примечание**: Примеры кода в этом документе написаны в соответствии со спецификацией языка YaoXiang. Если вы столкнётесь с синтаксическими различиями при фактическом запуске, обратитесь к
> [спецификации языка](../reference/language-spec/index.md).

## Установка

### Компиляция из исходного кода (рекомендуется)

```bash
# Клонировать репозиторий
git clone https://github.com/ChenXu233/YaoXiang.git
cd yaoxiang

# Компиляция (отладочная версия, для разработки и тестирования)
cargo build

# Компиляция (релизная версия, рекомендуется для продакшена)
cargo build --release

# Запустить тесты
cargo test

# Проверить версию
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

// Определение функции: name: (param: Type, ...) -> return_type = { return ... }  # Блок кода должен явно возвращать значение
// Выражение: name: (param: Type, ...) -> return_type = expr                      # Выражение возвращает значение напрямую
main: () -> Void = {
    print("Hello, YaoXiang!")
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
// Автоматический вывод типов
x = 42  // Выводится как Int
name = "YaoXiang"  // Выводится как String
pi = 3.14159  // Выводится как Float
is_valid = true  // Выводится как Bool

// Явные аннотации типов (рекомендуется использовать договорённость о централизации типов)
count: Int = 100

// По умолчанию неизменяемые (функция безопасности)
x = 10
x = 20  // ❌ Ошибка компиляции! Неизменяемость

// Изменяемые переменные (нужно явное объявление)
mut counter = 0
counter = counter + 1  // ✅ OK
```

### Функции

```yaoxiang
// Синтаксис определения функции
// Выражение: возвращает значение напрямую, без return
add: (a: Int, b: Int) -> Int = a + b

// Блок кода: необходимо использовать return для возврата значения
// add: (a: Int, b: Int) -> Int = { return a + b }

// Вызов
result = add(1, 2)  // result = 3

// Функция с одним параметром (выражение)
inc: (x: Int) -> Int = x + 1
```

### Определение типов

YaoXiang использует унифицированную модель синтаксиса `name: type = value`:

```yaoxiang
// Объявление переменной
x: Int = 42
name: String = "YaoXiang"

// Определение функции
add: (a: Int, b: Int) -> Int = a + b

// Определение типа (использование фигурных скобок)
Point: Type = { x: Float, y: Float }

// Использование типа
p: Point = Point(x=1.0, y=2.0)
p.x  // 1.0
p.y  // 2.0
```

#### Типы записей

```yaoxiang
// Структурные типы
Point: Type = { x: Float, y: Float }
Rect: Type = { x: Float, y: Float, width: Float, height: Float }

// Использование
p = Point(x=3.0, y=4.0)
r = Rect(x=0.0, y=0.0, width=10.0, height=20.0)
```

#### Определение интерфейсов

Интерфейсы — это типы записей, все поля которых являются функциями:

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

#### Методы типов

Используйте синтаксис `Type.method: (Type, ...) -> Return = ...` для определения методов типа:

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

// Использование методов (синтаксический сахар)
p = Point(x=1.0, y=2.0)
p.draw(screen)  // → Point.draw(p, screen)
str = p.serialize()  // → Point.serialize(p)
```

#### Автоматическое связывание

Функции, объявленные с ключевым словом `pub`, автоматически связываются с типами, определёнными в том же файле:

```yaoxiang
Point: Type = { x: Float, y: Float }

// Публичная функция автоматически связывается с Point
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

// Синтаксис ООП (автосвязывание с Point.distance)
d2 = p1.distance(p2)  // → distance(p1, p2)
```

#### Перечисления

```yaoxiang
// Простое перечисление
Color: Type = { red: () -> Color, green: () -> Color, blue: () -> Color }

// Перечисление с данными
Result: (T: Type, E: Type) -> Type = { ok: (T) -> Result(T, E), err: (E) -> Result(T, E) }

// Использование с generics
success: Result(Int, String) = ok(42)
failure: Result(Int, String) = err("not found")
```

#### Обобщённые типы

```yaoxiang
// Определение обобщённого типа
List: (T: Type) -> Type = {
    data: Array(T),
    length: Int,
    push: (List(T), T) -> Void
}

// Конкретные экземпляры
IntList: Type = List(Int)
StringList: Type = List(String)
```

### Управление потоком выполнения

```yaoxiang
// Условное выражение
if x > 0 {
    "positive"
} else if x == 0 {
    "zero"
} else {
    "negative"
}

// Цикл
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
// Списки
numbers = [1, 2, 3, 4, 5]
first = numbers[0]  // 1

// Словари
scores = {"Alice": 90, "Bob": 85}
alice_score = scores["Alice"]  // 90

// Добавление элементов
mut list = [1, 2, 3]
list.append(4)
```

### Сопоставление с образцом

```yaoxiang
// Выражение match
result: Result(Int, String) = ok(42)

message = match result {
    ok(value) => "Success: " + value.to_string()
    err(error) => "Error: " + error
}
```

## Параллельное программирование (конкурентность)

Модель конкурентности YaoXiang строится вокруг примитива `spawn <expr>` — это единственная точка входа для параллелизма.

```yaoxiang
// spawn модифицирует любое выражение, автоматически выполняя параллельно
main: () -> Void = {
    user = spawn fetch_user(1)   // Выполняется в фоне
    posts = spawn fetch_posts()  // Параллельный следующий шаг

    // При необходимости получить результат — автоматическая блокировка
    print(user.name)
    print(posts.length)
}
```

**Основное правило**: выражения, модифицированные `spawn`, выполняются в фоновом режиме, а внешний код синхронно блокируется в ожидании результата. Независимые задачи выполняются автоматически параллельно, с планированием по модели GMP среды выполнения.

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

### В: Переменные по умолчанию неизменяемы, как тогда изменять переменные?

```yaoxiang
// Используйте ключевое слово mut для объявления изменяемой переменной
mut x = 10
x = 20  // ✅ OK
```

### В: Как определять функции?

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

// Обработка через сопоставление с образцом
result = risky_operation()
match result {
    ok(value) => print("Success: " + value)
    err(error) => print("Error: " + error)
}
```

## Что дальше

- 📚 Изучите [спецификацию языка](../reference/language-spec/index.md) для понимания полного синтаксиса
- 🏗️ Просмотрите [документацию по дизайну](../design/) для понимания деталей реализации
- 💡 Прочитайте [манифест дизайна](../design/manifesto.md) для ознакомления с основными идеями

## Связанные ресурсы

- [GitHub репозиторий](https://github.com/yourusername/yaoxiang)
- [Сообщить о проблеме](https://github.com/yourusername/yaoxiang/issues)
- [Руководство по внесению вклада](../dev/contributing.md)