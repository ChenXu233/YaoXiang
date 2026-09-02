# Спецификация стандартной библиотеки

В этом документе определена спецификация стандартной библиотеки языка программирования YaoXiang,
включая базовую библиотеку, библиотеку ввода-вывода и математическую библиотеку.

---

## Глава 1: Базовая библиотека

### 1.1 Базовые типы

Стандартная библиотека предоставляет реализации следующих базовых типов:

| Тип            | Модуль           | Описание                      |
| -------------- | ---------------- | ----------------------------- |
| `Option(T)`    | `std.option`     | тип опционального значения    |
| `Result(T, E)` | `std.result`     | тип обработки ошибок          |
| `List(T)`      | `std.collection` | динамический массив           |
| `Map(K, V)`    | `std.collection` | хеш-отображение               |
| `String`       | `std.string`     | строковый тип                 |
| `Array(T, N)`  | `std.array`      | массив фиксированного размера |

### 1.2 Тип Option

```
Option: (T: Type) -> Type = { some: (T) -> Option(T), none: () -> Option(T) }
```

**Конструкторы вариантов**:

| Вариант       | Синтаксис            | Описание      |
| ------------- | -------------------- | ------------- |
| `Option.some` | `Option.some(value)` | есть значение |
| `Option.none` | `Option.none()`      | нет значения  |

**Основные методы**:

```yaoxiang
// 检查是否有值
is_some: (self: Option(T)) -> Bool
is_none: (self: Option(T)) -> Bool

// 获取值（可能 panic）
unwrap: (self: Option(T)) -> T

// 获取值或默认值
unwrap_or: (self: Option(T), default: T) -> T

// 映射值
map: (R: Type) -> ((self: Option(T), f: (T) -> R) -> Option(R))
```

### 1.3 Тип Result

```
Result: (T: Type, E: Type) -> Type = { ok: (T) -> Result(T, E), err: (E) -> Result(T, E) }
```

**Конструкторы вариантов**:

| Вариант      | Синтаксис           | Описание        |
| ------------ | ------------------- | --------------- |
| `Result.ok`  | `Result.ok(value)`  | значение успеха |
| `Result.err` | `Result.err(error)` | значение ошибки |

**Основные методы**:

```yaoxiang
// 检查是否成功
is_ok: (self: Result(T, E)) -> Bool
is_err: (self: Result(T, E)) -> Bool

// 获取值（可能 panic）
unwrap: (self: Result(T, E)) -> T

// 获取值或默认值
unwrap_or: (self: Result(T, E), default: T) -> T

// 映射成功值
map: (R: Type) -> ((self: Result(T, E), f: (T) -> R) -> Result(R, E))

// 映射错误值
map_err: (F: Type) -> ((self: Result(T, E), f: (E) -> F) -> Result(T, F))
```

**Носитель ошибки и коды ошибок (#323 M4)**:

Носитель `Error` в модулях std для Err содержит нормализованные коды ошибок, повторно использующие
сегменты E6xxx/E7xxx из RFC-013 (например, E6009 = недопустимый шаг Range) в качестве стабильного
межверсионного контракта — программы могут программно проверять коды, а `yaoxiang explain E6009`
выводит документацию. Индекс кодов см. в разделе RFC-013 «Значения ошибок времени выполнения и
сквозные коды».

```yaoxiang
// Error 值形态：{ code: String, message: String }

// 取出 Err 载体（Ok 时报运行时错误）
unwrap_err: (T, E) -> ((self: Result(T, E)) -> E)

// 读取错误码 / 消息
code: (self: Error) -> String
message: (self: Error) -> String
```

**Пример проверки по коду**:

```yaoxiang
use std.range
use std.result

r = range.iter(1..10..0)      // step=0 → Err(Error)
if result.is_err(r) {
    e = result.unwrap_err(r)
    if result.code(e) == "E6009" {
        // 按 Range 步长非法分支处理
        io.println(result.message(e))
    }
}
```

Пользовательское моделирование ошибок осуществляется через обобщённый параметр E в `Result(T, E)`
(пользовательский набор вариантов); `Error` в std — это удобный резервный носитель, и его система
кодов не налагает ограничений на пользовательский тип E.

### 1.4 Распространение ошибок

```
ErrorPropagate ::= Expr '?'
```

Оператор `?` автоматически распространяет ошибки типа Result:

```
// 成功时返回值，失败时向上返回 err
data = fetch_data()?

// 等价于
data = match fetch_data() {
    ok(v) => v
    err(e) => return err(e)
}
```

### 1.5 Утверждения (std.assert)

Модуль `std.assert` предоставляет унифицированный механизм утверждений — `assert` времени выполнения
и уточняющий тип времени компиляции `Assert` являются двумя сторонами одной и той же примитивы.

```yaoxiang
// IsTrue：值到类型的桥接函数
IsTrue: (b: Bool) -> Type = match b {
    true => Void,      // ⊤，程序继续
    false => Never,    // ⊥，发散
}

// Assert：编译期精化类型原语
Assert: (cond: Bool) -> Type = IsTrue(cond)

// assert：运行时断言（Assert 的值引入子）
assert: (cond: Bool, ?msg: String | Error) -> Assert(IsTrue(cond))

// Result 重载
assert: (result: Result) -> Assert(IsTrue(is_ok(result)))
```

**Диспетчеризация dispatch**:

| Условие                                                    | Поведение                                                                                             |
| ---------------------------------------------------------- | ----------------------------------------------------------------------------------------------------- |
| все свободные переменные cond известны во время компиляции | компилятор вычисляет: true → стирание, false → ошибка компиляции                                      |
| присутствуют свободные переменные времени выполнения       | вставляется проверка времени выполнения, внедряется чувствительное к потоку множество предположений Γ |

`assert(false, "msg")` эквивалентно raise — отдельное ключевое слово throw/raise не требуется.

---

## Глава 2: Библиотека ввода-вывода

### 2.1 Стандартный ввод-вывод

```yaoxiang
// 标准输出
print: (msg: String) -> Void
println: (msg: String) -> Void

// 标准输入
read_line: () -> String
read_char: () -> Char
```

### 2.2 Операции с файлами

```yaoxiang
// 文件类型
File: Type = {
    path: String,
    read: (self: File) -> Result(String, Error),
    write: (self: File, content: String) -> Result(Void, Error),
    append: (self: File, content: String) -> Result(Void, Error),
    close: (self: File) -> Void
}

// 文件操作
open: (path: String) -> Result(File, Error)
create: (path: String) -> Result(File, Error)
delete: (path: String) -> Result(Void, Error)
```

### 2.3 Операции с каталогами

```yaoxiang
// 目录类型
Dir: Type = {
    path: String,
    entries: (self: Dir) -> Result(List(String), Error),
    create: (self: Dir) -> Result(Void, Error),
    delete: (self: Dir) -> Result(Void, Error)
}

// 目录操作
read_dir: (path: String) -> Result(Dir, Error)
create_dir: (path: String) -> Result(Void, Error)
delete_dir: (path: String) -> Result(Void, Error)
```

---

## Глава 3: Математическая библиотека

### 3.1 Базовые математические функции

```yaoxiang
// 绝对值
abs: (x: Int) -> Int
abs: (x: Float) -> Float

// 最大最小值
max: (a: Int, b: Int) -> Int
min: (a: Int, b: Int) -> Int
max: (a: Float, b: Float) -> Float
min: (a: Float, b: Float) -> Float

// 幂运算
pow: (base: Float, exp: Float) -> Float
sqrt: (x: Float) -> Float

// 对数
log: (x: Float) -> Float
log2: (x: Float) -> Float
log10: (x: Float) -> Float
```

### 3.2 Тригонометрические функции

```yaoxiang
// 三角函数
sin: (x: Float) -> Float
cos: (x: Float) -> Float
tan: (x: Float) -> Float

// 反三角函数
asin: (x: Float) -> Float
acos: (x: Float) -> Float
atan: (x: Float) -> Float
atan2: (y: Float, x: Float) -> Float
```

### 3.3 Константы

```yaoxiang
// 数学常量
pi: Float = 3.141592653589793
e: Float = 2.718281828459045
```

---

## Глава 4: Строковая библиотека

### 4.1 Операции со строками

```yaoxiang
// 字符串长度
length: (s: String) -> Int

// 字符串拼接
concat: (a: String, b: String) -> String

// 字符串分割
split: (s: String, delimiter: String) -> List(String)

// 字符串查找
find: (s: String, pattern: String) -> Option(Int)
contains: (s: String, pattern: String) -> Bool

// 字符串替换
replace: (s: String, old: String, new: String) -> String

// 字符串修剪
trim: (s: String) -> String
trim_left: (s: String) -> String
trim_right: (s: String) -> String
```

### 4.2 Преобразование строк

```yaoxiang
// 类型转换
to_string: (x: Int) -> String
to_string: (x: Float) -> String
to_string: (x: Bool) -> String

// 解析
parse_int: (s: String) -> Result(Int, Error)
parse_float: (s: String) -> Result(Float, Error)
```

---

## Глава 5: Библиотека коллекций

### 5.1 Тип List

```yaoxiang
// List 类型
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
// Map 类型
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

### 6.1 Iterator trait

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
// 范围迭代器（Range 是正式类型，运行时身份为三标量不可变记录，
// 不再借 Tuple 外壳；打印 `1..10` / `1..10..2`，结构相等，具名字段）
Range: Type = {
    start: Int,
    end: Int,
    step: Int,
    Iterator(Int)
}

// 使用（迭代器协议：std.range.iter/has_next/next，for 经静态类型派发）
for i in 0..10 {
    print(i)
}

// step 形态（双点，无新关键词）
for i in 0..10..2 {
    print(i)
}
```

> **`Range(Int)` официально реализован** — доступны именованные поля `r.start`/`r.end`/`r.step`; во
> время выполнения `x in r` проходит через `std.range.contains` (проверка границ + выравнивание
> шага), что подтверждает идентификацию канала как предиката интервала
> `x >= r.start && x < r.end && (x - r.start) % r.step == 0` (интервал сохраняется как интервал, без
> материализации). Литерал step=0 отклоняется во время компиляции; динамический step=0 теперь
> возвращается как Result: `std.range.iter` → `Result(Iterator, Error)`, `std.range.contains` →
> `Result(Bool, Error)`; точка потребления распространяет по стеку вызовов через `?` либо явно
> разветвляется через `result.unwrap`; десахаризация `for`/`in` разворачивается в ir_gen с
> распаковкой, а ветвь Err (динамический step=0) явно завершается неудачей (`abort_invalid_step`) и
> никогда не приводит к тихому бесконечному циклу. Инстанцирование интерфейса (объявление тела типа
> `Iterator(Int)`): синтаксис типа и статическая диспетчеризация реализованы на этапах 1-2 RFC-011a
> — элемент применения тела типа `Iterator(Int)` запускает развёртывание подстановки `Self ↦ Range`
> и проверку полноты, после чего генерируется доказательство реализации. Динамическая
> диспетчеризация реализована на этапе 3: имя интерфейса без инстанцирования присутствует в типе
> (`List(Animal)`), конкретные значения автоматически оборачиваются в value variant при попадании в
> экзистенциальную позицию типа, а вызовы методов элемента диспетчеризируются по фактическому типу
> (§6). Уровень протокола времени выполнения модуля `std.range` пока обеспечивается собственными
> методами; миграция на диспетчеризацию через интерфейс остаётся в дальнейшей работе.

---

## Приложение: Указатель модулей стандартной библиотеки

| Модуль           | Описание                                                                         |
| ---------------- | -------------------------------------------------------------------------------- |
| `std.assert`     | Механизм утверждений — runtime assert + уточняющий тип Assert времени компиляции |
| `std.option`     | Тип Option                                                                       |
| `std.result`     | Тип Result                                                                       |
| `std.collection` | Типы коллекций List, Map и др.                                                   |
| `std.string`     | Операции со строками                                                             |
| `std.array`      | Операции с массивами                                                             |
| `std.iterator`   | Итераторы (уровень протокола в настоящее время обеспечивается `std.range`)       |
| `std.range`      | Итератор Range и предикаты интервалов, адаптеры                                  |

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

| Модуль       | Описание                                                                             |
| ------------ | ------------------------------------------------------------------------------------ |
| `std.random` | Генерация случайных чисел                                                            |
| `std.time`   | Дата и время                                                                         |
| `std.assert` | Единый `Assert(C)` времени компиляции и `assert(x > 0)` времени выполнения (RFC-030) |
| `std.regex`  | Регулярные выражения                                                                 |
