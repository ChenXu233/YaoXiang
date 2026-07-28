# Спецификация FFI

В данном документе определяется спецификация FFI (Foreign Function Interface — внешний интерфейс
функций) для языка программирования YaoXiang, включая определения типов, объявления функций,
привязки методов и обработку непрозрачных типов.

> **Подробный дизайн**: Полный дизайн FFI, мотивация и компромиссы описаны в
> [RFC-026: Основной механизм FFI](../design/rfc/accepted/026-ffi-core-mechanism.md).

---

## Глава 1: Обзор

### 1.1 Основные принципы FFI

```
Все return в {} возвращают содержимое в вышестоящую область видимости
По умолчанию без return возвращается Void
```

### 1.2 Составные части FFI

| Компонент          | Описание                                           | Синтаксис              |
| ------------------ | -------------------------------------------------- | ---------------------- |
| Определение типа   | Определение FFI типа (непрозрачный или прозрачный) | `unsafe {}` + `return` |
| Объявление функции | Объявление внешней функции                         | `native("symbol")`     |
| Привязка метода    | Привязка метода к типу                             | Синтаксис `[0]`        |

---

## Глава 2: Определение типов FFI

### 2.1 Непрозрачные типы

Непрозрачные типы определяются в блоке `unsafe {}` и возвращаются в вышестоящую область видимости
через `return`:

```yaoxiang
// Определение непрозрачного типа в unsafe блоке
SqliteDb = unsafe {
    SqliteDb: Type = {
        handle: *Void  // Голый указатель
    }
    return SqliteDb
}

// SqliteDb доступен за пределами unsafe блока
db = sqlite3_open("test.db")

// ❌ Ошибка компиляции: поле handle требует unsafe разрешения
handle = db.handle

// ✅ Через вызов метода
db.close()
```

### 2.2 Прозрачные типы

Прозрачные типы определяются напрямую, без блока `unsafe {}`:

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

Компилятор автоматически различает непрозрачные и вакуумные типы:

```yaoxiang
// Непрозрачный тип (ссылается на native функцию)
SqliteDb: Type = {}
sqlite3_open: (filename: String) -> SqliteDb = native("sqlite3_open")
// → SqliteDb ссылается на native функцию → непрозрачный тип

// Вакуумный тип (не ссылается на native функцию)
MyType: Type = {}
// → MyType не ссылается на native функцию → вакуумный тип
```

**Правила определения**:

- Если тип ссылается на `native` функцию → непрозрачный тип
- В противном случае → вакуумный тип

---

## Глава 3: Объявление FFI функций

### 3.1 Синтаксис native

Для объявления внешних функций используется синтаксис `native("symbol")`:

```yaoxiang
// Объявление FFI функции
sqlite3_open: (filename: String) -> SqliteDb = native("sqlite3_open")
sqlite3_close: (db: SqliteDb) -> Int32 = native("sqlite3_close")
sqlite3_exec: (db: SqliteDb, sql: String) -> Int32 = native("sqlite3_exec")
```

### 3.2 Маппинг типов параметров

Типы параметров FFI функций напрямую используют типы YaoXiang, компилятор автоматически обрабатывает
маппинг типов C:

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

### 3.3 Типы возвращаемых значений

Типы возвращаемых значений FFI функций напрямую используют типы YaoXiang:

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

Для указания позиции параметра self в кортеже параметров функции используется синтаксис `[0]`:

```yaoxiang
// FFI функция
sqlite3_close: (db: SqliteDb) -> Int32 = native("sqlite3_close")
sqlite3_exec: (db: SqliteDb, sql: String) -> Int32 = native("sqlite3_exec")

// Привязка метода (self на позиции 0)
SqliteDb.close = sqlite3_close[0]
SqliteDb.exec = sqlite3_exec[0]
```

**Способы вызова**:

```yaoxiang
db = sqlite3_open("test.db")

// Вызов метода
db.close()  // эквивалентно sqlite3_close(db)
db.exec("SELECT * FROM users")  // эквивалентно sqlite3_exec(db, "SELECT * FROM users")
```

### 4.2 Привязка конструктора

Конструкторы не используют `[0]`, привязываются как обычные функции:

```yaoxiang
// FFI функция
sqlite3_open: (filename: String) -> SqliteDb = native("sqlite3_open")

// Привязка конструктора (обычная функция)
SqliteDb.open = sqlite3_open
```

**Способы вызова**:

```yaoxiang
// Создание через конструктор
db = SqliteDb.open("test.db")
```

### 4.3 Позиция привязки

Привязка методов может быть в любом месте, так как типы являются контейнерами данных:

```yaoxiang
// Привязка после определения типа
SqliteDb.close = sqlite3_close[0]

// Привязка в другом файле
SqliteDb.exec = sqlite3_exec[0]

// Компилятор проверит в любом случае
```

---

## Глава 5: Поведение FFI в spawn блоках

### 5.1 Ресурсные типы автоматически сериализуются

Если FFI тип является ресурсным типом, в spawn блоке происходит автоматическая сериализация:

```yaoxiang
// SqliteDb является ресурсным типом
(a, b) = spawn {
    db1 = SqliteDb.open("db1.sqlite"),  // SqliteDb ресурс
    db2 = SqliteDb.open("db2.sqlite")   // Разные экземпляры, можно параллельно
}

(a, b) = spawn {
    result1 = db.exec("SELECT ..."),  // Тот же SqliteDb
    result2 = db.exec("INSERT ...")   // Автоматическая сериализация
}
```

### 5.2 Нересурсные типы могут быть параллельными

Если FFI тип не является ресурсным типом, в spawn блоке возможно параллельное выполнение:

```yaoxiang
// Float не является ресурсным типом
(a, b) = spawn {
    result1 = sin(1.0),  // Можно параллельно
    result2 = cos(1.0)   // Можно параллельно
}
```

---

## Глава 6: Инструментарий yx-bindgen

### 6.1 Генерируемое содержимое

yx-bindgen генерирует следующее:

- Определения FFI типов (unsafe блок + return)
- Объявления FFI функций (синтаксис native)
- Привязки методов (синтаксис [0])

### 6.2 Пример генерации

```bash
yx-bindgen --header /usr/include/sqlite3.h --output sqlite3_bindings.yx
```

Результат генерации:

```yaoxiang
// sqlite3_bindings.yx
// Автоматически сгенерировано, не редактировать вручную

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
// Объявления FFI функций
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

## Приложение: Краткая справка по синтаксису FFI

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
// Объявление FFI функции
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

// Вызов через метод
db.close()
db.exec("SELECT * FROM users")
```
