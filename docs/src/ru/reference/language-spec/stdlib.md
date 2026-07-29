# Спецификация стандартной библиотеки

В данном документе определяется спецификация стандартной библиотеки языка программирования YaoXiang,
включая базовую библиотеку, библиотеку ввода-вывода и математическую библиотеку.

---

## Глава 1: Базовая библиотека

### 1.1 Базовые типы

Стандартная библиотека предоставляет реализации следующих базовых типов:

| Тип            | Модуль           | Описание                      |
| -------------- | ---------------- | ----------------------------- |
| `Option(T)`    | `std.option`     | Тип опционального значения    |
| `Result(T, E)` | `std.result`     | Тип обработки ошибок          |
| `List(T)`      | `std.collection` | Динамический массив           |
| `Map(K, V)`    | `std.collection` | Хеш-отображение               |
| `String`       | `std.string`     | Строковый тип                 |
| `Array(T, N)`  | `std.array`      | Массив фиксированного размера |

### 1.2 Тип Option

```
Option: (T: Type) -> Type = { some: (T) -> Option(T), none: () -> Option(T) }
```

**Конструкторы вариантов**:

| Вариант       | Синтаксис            | Описание          |
| ------------- | -------------------- | ----------------- |
| `Option.some` | `Option.some(value)` | Имеет значение    |
| `Option.none` | `Option.none()`      | Не имеет значения |

**Часто используемые методы**:

```yaoxiang
// Проверка наличия значения
is_some: (self: Option(T)) -> Bool
is_none: (self: Option(T)) -> Bool

// Получение значения (возможен panic)
unwrap: (self: Option(T)) -> T

// Получение значения или значения по умолчанию
unwrap_or: (self: Option(T), default: T) -> T

// Преобразование значения
map: (R: Type) -> ((self: Option(T), f: (T) -> R) -> Option(R))
```

### 1.3 Тип Result

```
Result: (T: Type, E: Type) -> Type = { ok: (T) -> Result(T, E), err: (E) -> Result(T, E) }
```

**Конструкторы вариантов**:

| Вариант      | Синтаксис           | Описание          |
| ------------ | ------------------- | ----------------- |
| `Result.ok`  | `Result.ok(value)`  | Успешное значение |
| `Result.err` | `Result.err(error)` | Значение ошибки   |

**Часто используемые методы**:

```yaoxiang
// Проверка успешности
is_ok: (self: Result(T, E)) -> Bool
is_err: (self: Result(T, E)) -> Bool

// Получение значения (возможен panic)
unwrap: (self: Result(T, E)) -> T

// Получение значения или значения по умолчанию
unwrap_or: (self: Result(T, E), default: T) -> T

// Преобразование успешного значения
map: (R: Type) -> ((self: Result(T, E), f: (T) -> R) -> Result(R, E))

// Преобразование значения ошибки
map_err: (F: Type) -> ((self: Result(T, E), f: (E) -> F) -> Result(T, F))
```

### 1.4 Распространение ошибок

```
ErrorPropagate ::= Expr '?'
```

Оператор `?` автоматически распространяет ошибки типа Result:

```
// При успехе возвращает значение, при неудаче возвращает err выше
data = fetch_data()?

// Эквивалентно
data = match fetch_data() {
    ok(v) => v
    err(e) => return err(e)
}
```

### 1.5 Утверждения (std.assert)

Модуль `std.assert` предоставляет единый механизм утверждений — runtime `assert` и тип уточнения
`Assert` на этапе компиляции являются двумя сторонами одного примитива.

```yaoxiang
// IsTrue: функция-мост от значения к типу
IsTrue: (b: Bool) -> Type = match b {
    true => Void,      // ⊤, выполнение программы продолжается
    false => Never,    // ⊥, расходимость
}

// Assert: примитив типа уточнения на этапе компиляции
Assert: (cond: Bool) -> Type = IsTrue(cond)

// assert: утверждение в runtime (введение значения Assert)
assert: (cond: Bool, ?msg: String | Error) -> Assert(IsTrue(cond))

// Перегрузка Result
assert: (result: Result) -> Assert(IsTrue(is_ok(result)))
```

**Dispatch 分派**:

| Условие                                                    | Поведение                                                                      |
| ---------------------------------------------------------- | ------------------------------------------------------------------------------ |
| Все свободные переменные cond известны на этапе компиляции | Компилятор вычисляет: true → стирание, false → ошибка компиляции               |
| Существуют свободные переменные в runtime                  | Вставка проверки в runtime, добавление потоково-чувствительных предположений Γ |

`assert(false, "msg")` эквивалентен raise — отдельное ключевое слово throw/raise не требуется.

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

### 2.2 Операции с файлами

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

### 3.1 Базовые математические функции

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

// Логарифмы
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

### 4.1 Строковые операции

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

### 4.2 Преобразования строк

```yaoxiang
// Преобразования типов
to_string: (x: Int) -> String
to_string: (x: Float) -> String
to_string: (x: Bool) -> String

// Парсинг
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

### 6.1 Трейт Iterator

```yaoxiang
// Трейт Iterator
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

### A.1 Базовые модули

| Модуль           | Описание                                                                            |
| ---------------- | ----------------------------------------------------------------------------------- |
| `std.assert`     | Механизм утверждений — assert в runtime + уточнение типа Assert на этапе компиляции |
| `std.option`     | Тип Option                                                                          |
| `std.result`     | Тип Result                                                                          |
| `std.collection` | Типы коллекций List, Map и другие                                                   |
| `std.string`     | Строковые операции                                                                  |
| `std.array`      | Операции с массивами                                                                |
| `std.iterator`   | Итераторы                                                                           |

### A.2 Модули ввода-вывода

| Модуль     | Описание               |
| ---------- | ---------------------- |
| `std.io`   | Стандартный ввод-вывод |
| `std.file` | Операции с файлами     |
| `std.dir`  | Операции с каталогами  |

### A.3 Математические модули

| Модуль          | Описание                   |
| --------------- | -------------------------- |
| `std.math`      | Математические функции     |
| `std.math.trig` | Тригонометрические функции |
| `std.math.log`  | Логарифмические функции    |

### A.4 Вспомогательные модули

| Модуль       | Описание                                                                         |
| ------------ | -------------------------------------------------------------------------------- |
| `std.random` | Генерация случайных чисел                                                        |
| `std.time`   | Дата и время                                                                     |
| `std.assert` | Унификация `Assert(C)` на этапе компиляции и `assert(x > 0)` в runtime (RFC-030) |
| `std.regex`  | Регулярные выражения                                                             |
