# Спецификация FFI

Настоящий документ определяет спецификацию FFI (интерфейса внешних функций) языка программирования YaoXiang, включая определение типов, объявление функций, привязку методов и обработку непрозрачных типов.

> **Подробное описание**: полное описание FFI, мотивация и компромиссы подробно изложены в [RFC-026: Основной механизм FFI](../design/rfc/accepted/026-ffi-core-mechanism.md).

---

## Глава 1: Обзор

### 1.1 Основные принципы FFI

```
Все return в {} возвращают содержимое в вышестоящую область видимости
По умолчанию отсутствие return возвращает Void
```

### 1.2 Состав FFI

| Компонент | Описание | Синтаксис |
|------|------|------|
| Определение типов | Определение FFI-типов (непрозрачных или прозрачных) | `unsafe {}` + `return` |
| Объявление функций | Объявление внешних функций | `native("symbol")` |
| Привязка методов | Привязка методов к типам | синтаксис `[0]` |

---

## Глава 2: Определение FFI-типов

### 2.1 Непрозрачный тип

Непрозрачный тип определяется в блоке `unsafe {}` и возвращается в вышестоящую область видимости через `return`:

```yaoxiang
// Определение непрозрачного типа в блоке unsafe
SqliteDb = unsafe {
    SqliteDb: Type = {
        handle: *Void  // Голый указатель
    }
    return SqliteDb
}

// SqliteDb доступен за пределами блока unsafe
db = sqlite3_open("test.db")

// ❌ Ошибка компиляции: для доступа к полю handle требуются права unsafe
handle = db.handle

// ✅ Через вызов метода
db.close()
```

### 2.2 Прозрачный тип

Прозрачный тип определяется напрямую, без блока `unsafe {}`:

```yaoxiang
// Прозрачный тип
Point: Type = {
    x: Int32,
    y: Int32
}

// Пользователь может создавать его напрямую
p: Point = Point { x: 1, y: 2 }
```

### 2.3 Определение непрозрачного типа

Компилятор автоматически различает непрозрачный тип и пустой тип:

```yaoxiang
// Непрозрачный тип (на который ссылается функция native)
SqliteDb: Type = {}
sqlite3_open: (filename: String) -> SqliteDb = native("sqlite3_open")
// → На SqliteDb ссылается функция native → непрозрачный тип

// Пустой тип (на который не ссылается функция native)
MyType: Type = {}
// → На MyType не ссылается функция native → пустой тип
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

### 3.2 Отображение типов параметров

Для типов параметров FFI-функций используются типы YaoXiang напрямую, а отображение на C-типы компилятор выполняет автоматически:

| C-тип | Тип YaoXiang |
|--------|---------------|
| `int` | `Int32` |
| `long` | `Int64` |
| `float` | `Float32` |
| `double` | `Float64` |
| `char` | `Char` |
| `char*` | `String` |
| `bool` | `Bool` |
| `size_t` | `Uint` |
| `void*` | `*Void` |
| `struct T*` | `T` (прозрачный тип) |
| `typedef struct T T` | `T` (непрозрачный тип) |

### 3.3 Тип возврата

Для типа возврата FFI-функций используются типы YaoXiang напрямую:

```yaoxiang
// Возврат непрозрачного типа
sqlite3_open: (filename: String) -> SqliteDb = native("sqlite3_open")

// Возврат прозрачного типа
get_point: () -> Point = native("get_point")

// Возврат примитивного типа
get_value: () -> Int32 = native("get_value")
```

---

## Глава 4: Привязка методов

### 4.1 Синтаксис [0]

Синтаксис `[0]` задаёт позицию параметра self в кортеже параметров функции:

```yaoxiang
// FFI-функция
sqlite3_close: (db: SqliteDb) -> Int32 = native("sqlite3_close")
sqlite3_exec: (db: SqliteDb, sql: String) -> Int32 = native("sqlite3_exec")

// Привязка метода (self находится в позиции 0)
SqliteDb.close = sqlite3_close[0]
SqliteDb.exec = sqlite3_exec[0]
```

**Способ вызова**:
```yaoxiang
db = sqlite3_open("test.db")

// Вызов метода
db.close()  // Эквивалентно sqlite3_close(db)
db.exec("SELECT * FROM users")  // Эквивалентно sqlite3_exec(db, "SELECT * FROM users")
```

### 4.2 Привязка конструктора

К конструктору не добавляется `[0]` — он привязывается как обычная функция:

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

### 4.3 Расположение привязки

Привязка метода может находиться в любом месте, поскольку тип — это контейнер данных:

```yaoxiang
// Привязка после определения типа
SqliteDb.close = sqlite3_close[0]

// Привязка в другом файле
SqliteDb.exec = sqlite3_exec[0]

// Компилятор выполнит финальную проверку в любом случае
```

---

## Глава 5: Поведение FFI в блоках spawn

### 5.1 Автоматическая сериализация ресурсных типов

Если FFI-тип является ресурсным, в блоке spawn он автоматически сериализуется:

```yaoxiang
// SqliteDb — ресурсный тип
(a, b) = spawn {
    db1 = SqliteDb.open("db1.sqlite"),  // Ресурс SqliteDb
    db2 = SqliteDb.open("db2.sqlite")   // Разные экземпляры, могут выполняться параллельно
}

(a, b) = spawn {
    result1 = db.exec("SELECT ..."),  // Тот же SqliteDb
    result2 = db.exec("INSERT ...")   // Автоматическая сериализация
}
```

### 5.2 Параллельное выполнение для нересурсных типов

Если FFI-тип не является ресурсным, в блоке spawn он может выполняться параллельно:

```yaoxiang
// Float не является ресурсным типом
(a, b) = spawn {
    result1 = sin(1.0),  // Может выполняться параллельно
    result2 = cos(1.0)   // Может выполняться параллельно
}
```

---

## Глава 6: Инструментарий yx-bindgen

### 6.1 Генерируемое содержимое

yx-bindgen генерирует следующее:
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
// Сгенерировано автоматически, не редактируйте вручную

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

// Методы (self находится в позиции 0)
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

// Метод (self находится в позиции 0)
SqliteDb.close = sqlite3_close[0]
```

### A.4 Способы вызова

```yaoxiang
// Создание через конструктор
db = SqliteDb.open("test.db")

// Вызов через метод
db.close()
db.exec("SELECT * FROM users")
```