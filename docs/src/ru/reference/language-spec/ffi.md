# Спецификация FFI

Данный документ определяет спецификацию FFI (внешнего интерфейса функций) языка программирования
YaoXiang, включая определение типов, объявление функций, привязку методов и обработку непрозрачных
типов.

> **Подробное описание**: Полное проектирование FFI, обоснование и компромиссы см. в
> [RFC-026: Основной механизм FFI](../../design/rfc/accepted/026-ffi-core-mechanism.md).

---

## Глава 1: Обзор

### 1.1 Основные принципы FFI

```
Все return в {} возвращают содержимое в родительскую область видимости
По умолчанию отсутствие return означает возврат Void
```

### 1.2 Состав FFI

| Компонент          | Описание                                             | Синтаксис              |
| ------------------ | ---------------------------------------------------- | ---------------------- |
| Определение типа   | Определение FFI-типа (непрозрачного или прозрачного) | `unsafe {}` + `return` |
| Объявление функции | Объявление внешней функции                           | `native("symbol")`     |
| Привязка метода    | Привязка метода к типу                               | синтаксис `[0]`        |

---

## Глава 2: Определение FFI-типов

### 2.1 Непрозрачные типы

Непрозрачные типы определяются в блоке `unsafe {}` и возвращаются в родительскую область видимости
через `return`:

```yaoxiang
// В блоке unsafe определяется непрозрачный тип
SqliteDb = unsafe {
    SqliteDb: Type = {
        handle: *Void  // сырой указатель
    }
    return SqliteDb
}

// SqliteDb доступен за пределами блока unsafe
db = sqlite3_open("test.db")

// ❌ Ошибка компиляции: поле handle требует прав unsafe
handle = db.handle

// ✅ Через вызов метода
db.close()
```

### 2.2 Прозрачные типы

Прозрачные типы определяются напрямую и не требуют блока `unsafe {}`:

```yaoxiang
// Прозрачный тип
Point: Type = {
    x: Int32,
    y: Int32
}

// Пользователь может создавать напрямую
p: Point = Point { x: 1, y: 2 }
```

### 2.3 Определение непрозрачных типов

Компилятор автоматически различает непрозрачные и пустые типы:

```yaoxiang
// Непрозрачный тип (на который ссылается native-функция)
SqliteDb: Type = {}
sqlite3_open: (filename: String) -> SqliteDb = native("sqlite3_open")
// → На SqliteDb ссылается native-функция → непрозрачный тип

// Пустой тип (на который не ссылается native-функция)
MyType: Type = {}
// → На MyType не ссылается native-функция → пустой тип
```

**Правила определения**:

- Если на тип ссылается функция `native` → непрозрачный тип
- Иначе → пустой тип

---

## Глава 3: Объявление FFI-функций

### 3.1 Синтаксис native

Для объявления внешних функций используется синтаксис `native("symbol")`:

```yaoxiang
// Объявление FFI-функции
sqlite3_open: (filename: String) -> SqliteDb = native("sqlite3_open")
sqlite3_close: (db: SqliteDb) -> Int32 = native("sqlite3_close")
sqlite3_exec: (db: SqliteDb, sql: String) -> Int32 = native("sqlite3_exec")
```

### 3.2 Сопоставление типов параметров

Типы параметров FFI-функций напрямую используют типы YaoXiang, компилятор автоматически обрабатывает
сопоставление с типами C:

| Тип C                | Тип YaoXiang           |
| -------------------- | ---------------------- |
| `int`                | `Int32`                |
| `long`               | `Int64`                |
| `float`              | `Float32`              |
| `double`             | `Float64`              |
| `char`               | `Char`                 |
| `char*`              | `String`               |
| `bool`               | `Bool`                 |
| `size_t`             | `Uint`                 |
| `void*`              | `*Void`                |
| `struct T*`          | `T` (прозрачный тип)   |
| `typedef struct T T` | `T` (непрозрачный тип) |

### 3.3 Тип возврата

Тип возврата FFI-функций напрямую использует типы YaoXiang:

```yaoxiang
// Возврат непрозрачного типа
sqlite3_open: (filename: String) -> SqliteDb = native("sqlite3_open")

// Возврат прозрачного типа
get_point: () -> Point = native("get_point")

// Возврат базового типа
get_value: () -> Int32 = native("get_value")
```

---

## Глава 4: Привязка методов

### 4.1 Синтаксис [0]

Синтаксис `[0]` используется для указания позиции параметра self в кортеже аргументов функции:

```yaoxiang
// FFI-функция
sqlite3_close: (db: SqliteDb) -> Int32 = native("sqlite3_close")
sqlite3_exec: (db: SqliteDb, sql: String) -> Int32 = native("sqlite3_exec")

// Привязка метода (self на позиции 0)
SqliteDb.close = sqlite3_close[0]
SqliteDb.exec = sqlite3_exec[0]
```

**Способ вызова**:

```yaoxiang
db = sqlite3_open("test.db")

// Вызов метода
db.close()  // эквивалентно sqlite3_close(db)
db.exec("SELECT * FROM users")  // эквивалентно sqlite3_exec(db, "SELECT * FROM users")
```

### 4.2 Привязка конструктора

Конструктор не использует `[0]` и привязывается как обычная функция:

```yaoxiang
// FFI-функция
sqlite3_open: (filename: String) -> SqliteDb = native("sqlite3_open")

// Привязка конструктора (обычная функция)
SqliteDb.open = sqlite3_open
```

**Способ вызова**:

```yaoxiang
// Создание через конструктор
db = SqliteDb.open("test.db")
```

### 4.3 Позиция привязки

Привязка метода может находиться в любом месте, поскольку тип — это контейнер данных:

```yaoxiang
// Привязка после определения типа
SqliteDb.close = sqlite3_close[0]

// Привязка в другом файле
SqliteDb.exec = sqlite3_exec[0]

// Компилятор в любом случае выполнит проверку
```

---

## Глава 5: Поведение FFI в блоках spawn

### 5.1 Автоматическая сериализация ресурсных типов

Если FFI-тип является ресурсным, в блоке spawn он автоматически сериализуется:

```yaoxiang
// SqliteDb — ресурсный тип
(a, b) = spawn {
    db1 = SqliteDb.open("db1.sqlite"),  // ресурс SqliteDb
    db2 = SqliteDb.open("db2.sqlite")   // разные экземпляры, могут выполняться параллельно
}

(a, b) = spawn {
    result1 = db.exec("SELECT ..."),  // тот же SqliteDb
    result2 = db.exec("INSERT ...")   // автоматическая сериализация
}
```

### 5.2 Параллельное выполнение для нересурсных типов

Если FFI-тип не является ресурсным, в блоке spawn допускается параллельное выполнение:

```yaoxiang
// Float — не ресурсный тип
(a, b) = spawn {
    result1 = sin(1.0),  // может выполняться параллельно
    result2 = cos(1.0)   // может выполняться параллельно
}
```

---

## Глава 6: Цепочка инструментов yx-bindgen

### 6.1 Генерируемое содержимое

yx-bindgen генерирует следующее содержимое:

- Определения FFI-типов (блок unsafe + return)
- Объявления FFI-функций (синтаксис native)
- Привязки методов (синтаксис [0])

### 6.2 Пример генерации

```bash
yx-bindgen --header /usr/include/sqlite3.h --output sqlite3_bindings.yx
```

Результат генерации:

```yaoxiang
// sqlite3_bindings.yx
// Автоматически сгенерировано, не редактируйте вручную

// ============================================================================
// Определения типов
// ============================================================================

SqliteDb = unsafe {
    SqliteDb: Type = {
        handle: *Void
    }
    return SqliteDb
}

SqliteStmt = unsafe {
    SqliteStmt: Type = {
        handle: *Void
    }
    return SqliteStmt
}

// ============================================================================
// Объявления FFI-функций
// ============================================================================

sqlite3_open: (filename: String) -> SqliteDb = native("sqlite3_open")
sqlite3_close: (db: SqliteDb) -> Int32 = native("sqlite3_close")
sqlite3_exec: (db: SqliteDb, sql: String) -> Int32 = native("sqlite3_exec")
sqlite3_prepare_v2: (db: SqliteDb, sql: String) -> SqliteStmt = native("sqlite3_prepare_v2")
sqlite3_step: (stmt: SqliteStmt) -> Int32 = native("sqlite3_step")
sqlite3_finalize: (stmt: SqliteStmt) -> Int32 = native("sqlite3_finalize")

// ============================================================================
// Привязки методов
// ============================================================================

// Конструктор (обычная функция)
SqliteDb.open = sqlite3_open

// Метод (self на позиции 0)
SqliteDb.close = sqlite3_close[0]
SqliteDb.exec = sqlite3_exec[0]
SqliteDb.prepare = sqlite3_prepare_v2[0]

// Методы SqliteStmt
SqliteStmt.step = sqlite3_step[0]
SqliteStmt.finalize = sqlite3_finalize[0]
```

---

## Приложение: Краткий справочник по синтаксису FFI

### A.1 Определение типов

```yaoxiang
// Непрозрачный тип
SqliteDb = unsafe {
    SqliteDb: Type = {
        handle: *Void
    }
    return SqliteDb
}

// Прозрачный тип
Point: Type = {
    x: Int32,
    y: Int32
}
```

### A.2 Объявление функций

```yaoxiang
// Объявление FFI-функции
sqlite3_open: (filename: String) -> SqliteDb = native("sqlite3_open")
sqlite3_close: (db: SqliteDb) -> Int32 = native("sqlite3_close")
```

### A.3 Привязка методов

```yaoxiang
// Конструктор (обычная функция)
SqliteDb.open = sqlite3_open

// Метод (self на позиции 0)
SqliteDb.close = sqlite3_close[0]
```

### A.4 Способы вызова

```yaoxiang
// Создание через конструктор
db = SqliteDb.open("test.db")

// Вызов метода
db.close()
db.exec("SELECT * FROM users")
```
