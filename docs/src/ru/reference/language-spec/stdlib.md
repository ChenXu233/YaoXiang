# Спецификация стандартной библиотеки

В данном документе определяется спецификация стандартной библиотеки языка программирования YaoXiang, включая основную библиотеку, библиотеку ввода-вывода и математическую библиотеку.

---

## Глава 1: Основная библиотека

### 1.1 Базовые типы

Стандартная библиотека предоставляет реализации следующих базовых типов:

| Тип | Модуль | Описание |
|------|------|------|
| `Option(T)` | `std.option` | тип опционального значения |
| `Result(T, E)` | `std.result` | тип для обработки ошибок |
| `List(T)` | `std.collection` | динамический массив |
| `Map(K, V)` | `std.collection` | хеш-отображение |
| `String` | `std.string` | строковый тип |
| `Array(T, N)` | `std.array` | массив фиксированного размера |

### 1.2 Тип Option

```
Option: (T: Type) -> Type = { some: (T) -> Option(T), none: () -> Option(T) }
```

**Конструкторы вариантов**:

| Вариант | Синтаксис | Описание |
|------|------|------|
| `Option.some` | `Option.some(value)` | значение присутствует |
| `Option.none` | `Option.none()` | значение отсутствует |

**Основные методы**:

```yaoxiang
// Проверка наличия значения
is_some: (self: Option(T)) -> Bool
is_none: (self: Option(T)) -> Bool

// Получение значения (может вызвать panic)
unwrap: (self: Option(T)) -> T

// Получение значения или значения по умолчанию
unwrap_or: (self: Option(T), default: T) -> T

// Отображение значения
map: (R: Type) -> ((self: Option(T), f: (T) -> R) -> Option(R))
```

### 1.3 Тип Result

```
Result: (T: Type, E: Type) -> Type = { ok: (T) -> Result(T, E), err: (E) -> Result(T, E) }
```

**Конструкторы вариантов**:

| Вариант | Синтаксис | Описание |
|------|------|------|
| `Result.ok` | `Result.ok(value)` | значение успеха |
| `Result.err` | `Result.err(error)` | значение ошибки |

**Основные методы**:

```yaoxiang
// Проверка успешности
is_ok: (self: Result(T, E)) -> Bool
is_err: (self: Result(T, E)) -> Bool

// Получение значения (может вызвать panic)
unwrap: (self: Result(T, E)) -> T

// Получение значения или значения по умолчанию
unwrap_or: (self: Result(T, E), default: T) -> T

// Отображение значения успеха
map: (R: Type) -> ((self: Result(T, E), f: (T) -> R) -> Result(R, E))

// Отображение значения ошибки
map_err: (F: Type) -> ((self: Result(T, E), f: (E) -> F) -> Result(T, F))
```

### 1.4 Распространение ошибок

```
ErrorPropagate ::= Expr '?'
```

Оператор `?` автоматически распространяет ошибки типа Result:

```
// При успехе возвращает значение, при неудаче пробрасывает err наверх
data = fetch_data()?

// Эквивалентно
data = match fetch_data() {
    ok(v) => v
    err(e) => return err(e)
}
```

### 1.5 Утверждения (std.assert)

Модуль `std.assert` предоставляет единый механизм утверждений — `assert` времени выполнения и уточняющий тип `Assert` времени компиляции являются двумя сторонами одного и того же примитива.

```yaoxiang
# IsTrue: функция-мост от значения к типу
IsTrue: (b: Bool) -> Type = match b {
    true => Void,      # ⊤, программа продолжается
    false => Never,    # ⊥, расходится
}

# Assert: примитив уточняющего типа времени компиляции
Assert: (cond: Bool) -> Type = IsTrue(cond)

# assert: утверждение времени выполнения (форма введения значения Assert)
assert: (cond: Bool, ?msg: String | Error) -> Assert(IsTrue(cond))

# Перегрузка для Result
assert: (result: Result) -> Assert(IsTrue(is_ok(result)))
```

**Диспетчеризация dispatch**:

| Условие | Поведение |
|------|------|
| все свободные переменные cond известны на этапе компиляции | компилятор вычисляет: true → стирается, false → ошибка компиляции |
| присутствуют свободные переменные времени выполнения | вставляется проверка времени выполнения, внедряется потокочувствительный набор предположений Γ |

`assert(false, "msg")` эквивалентно raise — отдельные ключевые слова throw/raise не требуются.

---

## Глава 2: Библиотека ввода-вывода

### 2.1 Стандартный ввод-вывод

```yaoxiang
// Стандартный вывод
print: (msg: String) -> Void
println: (msg: String) -> Void

// Стандартный ввод
read_line: () -> String
read_char: () -> Char
```

### 2.2 Файловые операции

```yaoxiang
// Тип файла
File: Type = {
    path: String,
    read: (self: File) -> Result(String, Error),
    write: (self: File, content: String) -> Result(Void, Error),
    append: (self: File, content: String) -> Result(Void, Error),
    close: (self: File) -> Void
}

// Операции с файлами
open: (path: String) -> Result(File, Error)
create: (path: String) -> Result(File, Error)
delete: (path: String) -> Result(Void, Error)
```

### 2.3 Операции с каталогами

```yaoxiang
// Тип каталога
Dir: Type = {
    path: String,
    entries: (self: Dir) -> Result(List(String), Error),
    create: (self: Dir) -> Result(Void, Error),
    delete: (self: Dir) -> Result(Void, Error)
}

// Операции с каталогами
read_dir: (path: String) -> Result(Dir, Error)
create_dir: (path: String) -> Result(Void, Error)
delete_dir: (path: String) -> Result(Void, Error)
```

---

## Глава 3: Математическая библиотека

### 3.1 Основные математические функции

```yaoxiang
// Абсолютное значение
abs: (x: Int) -> Int
abs: (x: Float) -> Float

// Максимум и минимум
max: (a: Int, b: Int) -> Int
min: (a: Int, b: Int) -> Int
max: (a: Float, b: Float) -> Float
min: (a: Float, b: Float) -> Float

// Возведение в степень
pow: (base: Float, exp: Float) -> Float
sqrt: (x: Float) -> Float

// Логарифм
log: (x: Float) -> Float
log2: (x: Float) -> Float
log10: (x: Float) -> Float
```

### 3.2 Тригонометрические функции

```yaoxiang
// Тригонометрические функции
sin: (x: Float) -> Float
cos: (x: Float) -> Float
tan: (x: Float) -> Float

// Обратные тригонометрические функции
asin: (x: Float) -> Float
acos: (x: Float) -> Float
atan: (x: Float) -> Float
atan2: (y: Float, x: Float) -> Float
```

### 3.3 Константы

```yaoxiang
// Математические константы
pi: Float = 3.141592653589793
e: Float = 2.718281828459045
```

---

## Глава 4: Библиотека работы со строками

### 4.1 Операции со строками

```yaoxiang
// Длина строки
length: (s: String) -> Int

// Конкатенация строк
concat: (a: String, b: String) -> String

// Разделение строки
split: (s: String, delimiter: String) -> List(String)

// Поиск в строке
find: (s: String, pattern: String) -> Option(Int)
contains: (s: String, pattern: String) -> Bool

// Замена в строке
replace: (s: String, old: String, new: String) -> String

// Обрезка строки
trim: (s: String) -> String
trim_left: (s: String) -> String
trim_right: (s: String) -> String
```

### 4.2 Преобразование строк

```yaoxiang
// Преобразование типов
to_string: (x: Int) -> String
to_string: (x: Float) -> String
to_string: (x: Bool) -> String

// Разбор
parse_int: (s: String) -> Result(Int, Error)
parse_float: (s: String) -> Result(Float, Error)
```

---

## Глава 5: Библиотека коллекций

### 5.1 Тип List

```yaoxiang
// Тип List
List: (T: Type) -> Type = {
    data: Array(T),
    length: Int,
    push: (T: Type) -> ((self: List(T), item: T) -> Void),
    pop: (T: Type) -> ((self: List(T)) -> Option(T)),
    get: (T: Type) -> ((self: List(T), index: Int) -> Option(T)),
    set: (T: Type) -> ((self: List(T), index: Int, value: T) -> Void),
    insert: (T: Type) -> ((self: List(T), index: Int, item: T) -> Void),
    remove: (T: Type) -> ((self: List(T), index: Int) -> Option(T)),
    clear: (T: Type) -> ((self: List(T)) -> Void),
    contains: (T: Type) -> ((self: List(T), item: T) -> Bool),
    sort: (T: Type) -> ((self: List(T)) -> List(T)),
    reverse: (T: Type) -> ((self: List(T)) -> List(T)),
    map: (T: Type, R: Type) -> ((self: List(T), f: (T) -> R) -> List(R)),
    filter: (T: Type) -> ((self: List(T), predicate: (T) -> Bool) -> List(T)),
    reduce: (T: Type, R: Type) -> ((self: List(T), initial: R, f: (R, T) -> R) -> R)
}
```

### 5.2 Тип Map

```yaoxiang
// Тип Map
Map: (K: Type, V: Type) -> Type = {
    data: Array((K, V)),
    length: Int,
    insert: (K: Type, V: Type) -> ((self: Map(K, V), key: K, value: V) -> Void),
    get: (K: Type, V: Type) -> ((self: Map(K, V), key: K) -> Option(V)),
    remove: (K: Type, V: Type) -> ((self: Map(K, V), key: K) -> Option(V)),
    contains_key: (K: Type, V: Type) -> ((self: Map(K, V), key: K) -> Bool),
    keys: (K: Type, V: Type) -> ((self: Map(K, V)) -> List(K)),
    values: (K: Type, V: Type) -> ((self: Map(K, V)) -> List(V)),
    clear: (K: Type, V: Type) -> ((self: Map(K, V)) -> Void)
}
```

---

## Глава 6: Библиотека итераторов

### 6.1 Trait Iterator

```yaoxiang
// Iterator trait
Iterator: (T: Type) -> Type = {
    Item: T,
    next: () -> Option(T),
    has_next: () -> Bool,
    map: (R: Type) -> ((f: (T) -> R) -> Iterator(R)),
    filter: (predicate: (T) -> Bool) -> Iterator(T),
    collect: () -> List(T),
    reduce: (R: Type) -> ((initial: R, f: (R, T) -> R) -> R),
    for_each: (f: (T) -> Void) -> Void
}
```

### 6.2 Адаптеры итераторов

```yaoxiang
// Итератор диапазона
Range: Type = {
    start: Int,
    end: Int,
    step: Int,
    Iterator(Int)
}

// Использование
for i in 0..10 {
    print(i)
}

for i in 0..10 step 2 {
    print(i)
}
```

---

## Приложение: Индекс модулей стандартной библиотеки

### A.1 Основные модули

| Модуль | Описание |
|------|------|
| `std.assert` | механизм утверждений — assert времени выполнения + уточняющий тип Assert времени компиляции |
| `std.option` | тип Option |
| `std.result` | тип Result |
| `std.collection` | типы коллекций, такие как List, Map |
| `std.string` | операции со строками |
| `std.array` | операции с массивами |
| `std.iterator` | итераторы |

### A.2 Модули ввода-вывода

| Модуль | Описание |
|------|------|
| `std.io` | стандартный ввод-вывод |
| `std.file` | файловые операции |
| `std.dir` | операции с каталогами |

### A.3 Математические модули

| Модуль | Описание |
|------|------|
| `std.math` | математические функции |
| `std.math.trig` | тригонометрические функции |
| `std.math.log` | логарифмические функции |

### A.4 Утилитарные модули

| Модуль | Описание |
|------|------|
| `std.random` | генерация случайных чисел |
| `std.time` | время и дата |
| `std.regex` | регулярные выражения |