# RFC-012 План реализации f-string шаблонных строк

> **Статус**: ✅ Завершено
> **На основе RFC**: RFC-012 F-String Template Strings
> **Стратегия преобразования**: Унифицированный вызов `format()`
> **Дата завершения**: 2025-07

---

## Цели реализации

Добавление синтаксического сахара f-string шаблонных строк в язык YaoXiang:

```yaoxiang
// Вставка переменных
name = "Alice"
greeting = f"Hello {name}"        // → format("Hello {}", name)

// Вставка выражений
x = 10
y = 20
result = f"Sum: {x + y}"         // → format("Sum: {}", x + y)

// Спецификаторы форматирования
pi = 3.14159
s = f"Pi: {pi:.2f}"              // → format("Pi: {:.2f}", pi)

// Множественные вставки
items = [1, 2, 3]
s = f"Count: {len(items)}, sum: {sum(items)}"
```

---

## Архитектурный дизайн

### Ключевые принципы

1. **Единая стратегия преобразования** — все f-string преобразуются в вызовы `format()`
2. **Синтаксический сахар времени компиляции** — без новых возможностей во время выполнения, только предварительная обработка
3. **Расширение вычисления констант** — на уровне IR расширяется вычисление констант для поддержки compile-time вычислений

### Поток данных

```
Исходный код (f"...")
    ↓
Лексер: распознавание префикса f"
    ↓
Парсер: разбор выражений интерполяции
    ↓
AST: новый узел FString
    ↓
Проверка типов: валидация типов выражений
    ↓
Codegen: преобразование в вызов format()
    ↓
IR/Целевой код
```

---

## Шаги реализации

### Фаза 1: Лексер (词法分析)

**Цель**: Распознавание синтаксиса f-string

**Файл**: `src/frontend/core/lexer/`

**Изменения**:

1. **tokens.rs** — новый тип токена
   ```rust
   // Новый токен FStringLiteral (хранит原始ное содержимое f-string)
   FStringLiteral(String),
   ```

2. **tokenizer.rs** — распознавание префикса f"
   ```rust
   // В next_token() добавить
   '"' => {
       // Проверить, является ли предыдущий символ 'f'
       // Если да, вызвать scan_fstring()
       // Иначе вызвать scan_string()
   }
   ```

3. **literals.rs** — реализация сканирования f-string
   ```rust
   pub fn scan_fstring(lexer: &mut Lexer<'_>) -> Option<Token> {
       // Сканировать содержимое f"..."
       // Разобрать {expression} интерполяцию
       // Вернуть FStringLiteral(String)
   }
   ```

**Критерии приёмки**:
- [x] `f"hello"` распознаётся как токен FStringLiteral
- [x] `f"Hello {name}"` правильно разбирает границы интерполяции
- [x] Ошибка: незакрытая `{` даёт понятное сообщение (`UnterminatedFStringInterpolation`)

---

### Фаза 2: Парсер (语法分析)

**Цель**: Разбор f-string в AST узел

**Файл**: `src/frontend/core/parser/`

**Изменения**:

1. **ast.rs** — новый узел AST
   ```rust
   pub enum Expr {
       // ... существующие ...
       /// F-string шаблонная строка
       FString {
           segments: Vec<FStringSegment>,  // Текстовые сегменты и выражения интерполяции
           span: Span,
       },
   }

   pub enum FStringSegment {
       /// Текстовый фрагмент
       Text(String),
       /// Выражение интерполяции
       Interpolation {
           expr: Box<Expr>,
           format_spec: Option<String>,  // Опциональный спецификатор форматирования
       },
   }
   ```

2. **pratt/nud.rs** — разбор f-string литерала
   ```rust
   // В таблицу nud добавить
   TokenKind::FStringLiteral(_) => Some((BP_HIGHEST, Self::parse_fstring)),

   fn parse_fstring(&mut self) -> Option<Expr> {
       // Преобразовать строку FStringLiteral в AST узел FString
   }
   ```

**Критерии приёмки**:
- [x] `f"hello"` парсится в `Expr::FString { segments: [Text("hello")] }`
- [x] `f"hello {x}"` правильно парсит выражение интерполяции
- [x] `f"Pi: {pi:.2f}"` правильно парсит спецификатор формата

---

### Фаза 3: Проверка типов (类型检查)

**Цель**: Валидация типов выражений интерполяции

**Файл**: `src/frontend/typecheck/inference/`

**Изменения**:

1. **expressions.rs** — выведение типов
   ```rust
   // Новый вывод типа для f-string
   fn infer_fstring(&mut self, segments: &[FStringSegment]) -> Result<MonoType> {
       // f-string всегда возвращает тип String
       // Проверить, что каждый тип выражения интерполяции реализует трайт Stringable
   }
   ```

2. **Генерация ограничений** (при необходимости)
   ```rust
   // Для выражений интерполяции добавить ограничение Stringable
   ```

**Критерии приёмки**:
- [x] `f"{42}"` имеет тип String
- [x] `f"{some_int}"` правильно проверяет Int → Stringable
- [ ] Ошибка: типы без поддержки Stringable дают понятную ошибку (после доработки системы трайтов)

---

### Фаза 4: Codegen (代码生成)

**Цель**: Преобразование в вызов format()

**Файл**: `src/middle/core/ir_gen.rs` или новый `fstring.rs`

**Изменения**:

1. **Преобразование в вызов format()**
   ```rust
   // Пример преобразования
   f"Hello {name}" → format("Hello {}", name)
   f"Pi: {pi:.2f}" → format("Pi: {:.2f}", pi)
   ```

2. **Генерация IR**
   ```rust
   fn gen_fstring(&mut self, segments: &[FStringSegment]) -> Operand {
       // Построить вызов format
       // format_str: "Hello {}"
       // args: [name]
   }
   ```

**Критерии приёмки**:
- [x] `f"hello"` генерирует корректный вызов format
- [x] `f"x = {x}"` правильно передаёт параметры
- [x] `f"Pi: {pi:.2f}"` спецификатор формата правильно передаётся

---

### Фаза 5: Оптимизация вычисления констант (常量求值优化)

**Цель**: Compile-time вычисления

**Файл**: `src/middle/core/ir_gen.rs`

**Изменения**:

1. **Расширение eval_const_expr**
   ```rust
   fn eval_const_expr(&self, expr: &Expr) -> Option<ConstValue> {
       match expr {
           // Существующее
           Expr::Lit(lit) => eval_literal(lit),

           // Новое: рекурсивное вычисление f-string
           Expr::FString { segments } => {
               let mut result = String::new();
               for seg in segments {
                   match seg {
                       FStringSegment::Text(s) => result.push_str(s),
                       FStringSegment::Interpolation { expr, .. } => {
                           // Рекурсивно вычислить выражение
                           let val = self.eval_const_expr(expr)?;
                           result.push_str(&val.to_string());
                       }
                   }
               }
               Some(ConstValue::String(result))
           }

           // Существующее: поддержка константных вызовов format()
           Expr::Call { func, args } if is_const_format(func) => {
               self.eval_const_format(args)
           }
       }
   }
   ```

2. **Внедрение констант**
   ```rust
   // В gen_expr
   if let Some(const_val) = self.eval_const_expr(expr) {
       // Использовать константное значение напрямую, без генерации runtime-вызова
       return Operand::Const(const_val);
   }
   ```

**Критерии приёмки**:
- [x] `f"hello"` вычисляется в константу "hello" во время компиляции
- [x] `f"x = {1+2}"` вычисляется в "x = 3" во время компиляции
- [x] Не-константные вставки корректно генерируют runtime-вызовы

---

## Дизайн тестирования

### Модульные тесты

#### 1. Тесты лексера

**Файл**: `src/frontend/core/lexer/tests/fstring.rs` (новый)

```rust
#[test]
fn test_fstring_basic() {
    let mut lexer = Lexer::new(r#"f"hello""#);
    let token = lexer.next_token().unwrap();
    assert!(matches!(token.kind, TokenKind::FStringLiteral(_)));
}

#[test]
fn test_fstring_with_interpolation() {
    let mut lexer = Lexer::new(r#"f"hello {name}""#);
    let token = lexer.next_token().unwrap();
    // Проверить, что токен содержит маркеры интерполяции
}

#[test]
fn test_fstring_unclosed_brace_error() {
    let mut lexer = Lexer::new(r#"f"hello {name""#);
    // Проверить сообщение об ошибке
}
```

#### 2. Тесты парсера

**Файл**: `src/frontend/core/parser/tests/fstring.rs` (новый)

```rust
#[test]
fn test_parse_fstring_text() {
    let tokens = tokenize(r#"f"hello""#);
    let ast = parse(tokens);
    assert_matches!(ast, Expr::FString { segments, .. }
        if segments.len() == 1
    );
}

#[test]
fn test_parse_fstring_interpolation() {
    let tokens = tokenize(r#"f"hello {name}""#);
    let ast = parse(tokens);
    // Проверить segments = [Text("hello "), Interpolation(Var("name"))]
}

#[test]
fn test_parse_fstring_format_spec() {
    let tokens = tokenize(r#"f"Pi: {pi:.2f}""#);
    let ast = parse(tokens);
    // Проверить format_spec = Some(".2f")
}
```

#### 3. Тесты проверки типов

**Файл**: `src/frontend/typecheck/tests/fstring.rs` (новый)

```rust
#[test]
fn test_fstring_type_int() {
    let code = r#"
        x = 10
        s = f"value: {x}"
    "#;
    check_types(code);
}

#[test]
fn test_fstring_type_not_stringable() {
    let code = r#"
        struct NotStringable
        x = NotStringable()
        s = f"value: {x}"  // Должна быть ошибка
    "#;
    check_type_error(code, "does not implement Stringable");
}
```

#### 4. Тесты Codegen

**Файл**: `tests/integration/fstring.rs` (новый)

```rust
#[test]
fn test_fstring_basic() {
    let result = run(r#"
        print(f"hello world")
    "#);
    assert_eq!(result, "hello world");
}

#[test]
fn test_fstring_interpolation() {
    let result = run(r#"
        name = "Alice"
        print(f"Hello {name}")
    "#);
    assert_eq!(result, "Hello Alice");
}

#[test]
fn test_fstring_format_spec() {
    let result = run(r#"
        pi = 3.14159
        print(f"Pi: {pi:.2f}")
    "#);
    assert_eq!(result, "Pi: 3.14");
}

#[test]
fn test_fstring_expression() {
    let result = run(r#"
        x = 10
        y = 20
        print(f"{x} + {y} = {x + y}")
    "#);
    assert_eq!(result, "10 + 20 = 30");
}

#[test]
fn test_fstring_const_eval() {
    let result = run(r#"
        x = f"hello {1+2}"
        print(x)
    "#);
    // Результат вычисления константы
    assert_eq!(result, "hello 3");
}
```

### Интеграционные тесты

```rust
// Тест реальных сценариев
#[test]
fn test_fstring_logging() {
    let code = r#"
        log(level: String, msg: String) = () => {
            timestamp = "2024-01-01"
            print(f"[{timestamp}] {level}: {msg}")
        }
        log("INFO", "system started")
    "#;
    // Ожидаемый вывод: [2024-01-01] INFO: system started
}

#[test]
fn test_fstring_json_like() {
    let code = r#"
        name = "Alice"
        age = 30
        print(f"{{"name": "{name}", "age": {age}}}")
    "#;
    // Ожидаемый вывод: { "name": "Alice", "age": 30 }
}
```

---

## Список ключевых файлов

| Файл | Тип изменения | Описание |
|------|---------------|----------|
| `src/frontend/core/lexer/tokens.rs` | Изменение | Новый FStringLiteral |
| `src/frontend/core/lexer/tokenizer.rs` | Изменение | Распознавание префикса f" |
| `src/frontend/core/lexer/literals.rs` | Изменение | Сканирование f-string |
| `src/frontend/core/parser/ast.rs` | Изменение | Новый узел FString |
| `src/frontend/core/parser/pratt/nud.rs` | Изменение | Разбор f-string |
| `src/frontend/typecheck/inference/expressions.rs` | Изменение | Выведение типов |
| `src/middle/core/ir_gen.rs` | Изменение | Генерация кода + вычисление констант |
| `src/frontend/core/lexer/tests/fstring.rs` | Новый | Тесты лексера |
| `src/frontend/core/parser/tests/fstring.rs` | Новый | Тесты парсера |
| `src/frontend/typecheck/tests/fstring.rs` | Новый | Тесты проверки типов |
| `tests/integration/fstring.rs` | Новый | Интеграционные тесты |

---

## Зависимости и риски

### Зависимости

- **Имеется**: функция `format()` (`src/std/string.rs`)
- **Имеется**: фреймворк вычисления констант (`ir_gen.rs::eval_const_expr`)
- **Не требуется**: новых внешних зависимостей

### Риски

1. **Разбор вложенных фигурных скобок**: сценарий `{ { x } }`
   - Решение: RFC ограничивает вложенное использование

2. **Сложность спецификаторов форматирования**
   - Решение: повторное использование логики разбора существующей функции format

---

## Вехи

- [x] Фаза 1: Лексер распознаёт f-string
- [x] Фаза 2: Парсер разбирает в AST
- [x] Фаза 3: Проверка типов
- [x] Фаза 4: Codegen преобразует в format()
- [x] Фаза 5: Оптимизация вычисления констант
- [x] Полное тестовое покрытие (27 тестов: 10 лексер + 6 парсер + 4 проверка типов + 7 интеграция)

---

## Приложение

### Справочные реализации

- Python f-strings: https://docs.python.org/3/tutorial/inputoutput.html
- Rust format!: https://doc.rust-lang.org/std/macro.format.html

### Связанные RFC

- RFC-012: F-String Template Strings (на основе данного документа)

---

## Журнал реализации

### Фактически изменённые файлы

| Файл | Тип изменения | Конкретные изменения |
|------|---------------|---------------------|
| `src/frontend/core/lexer/tokens.rs` | Изменение | Добавлен токен `FStringLiteral(String)` и ошибка `UnterminatedFStringInterpolation` |
| `src/frontend/core/lexer/tokenizer.rs` | Изменение | В `scan_identifier()` добавлено определение префикса `f"` и вызов `scan_fstring()` |
| `src/frontend/core/lexer/literals.rs` | Изменение | Добавлена функция `scan_fstring()` (~180 строк), поддержка `{}` интерполяции, экранирование `{ }` скобок |
| `src/frontend/core/lexer/mod.rs` | Изменение | В `log_token()` добавлена ветка FStringLiteral; подключён модуль тестов fstring |
| `src/frontend/core/parser/ast.rs` | Изменение | Добавлен AST узел `FString` и enum `FStringSegment` |
| `src/frontend/core/parser/pratt/nud.rs` | Изменение | Добавлены `parse_fstring()`, `parse_fstring_segments()`, `split_format_spec()` |
| `src/frontend/typecheck/inference/expressions.rs` | Изменение | В `infer_expr()` добавлена ветка `Expr::FString`, возвращает `MonoType::String` |
| `src/middle/core/ir_gen.rs` | Изменение | В трёх местах (`get_expr_span()`, `eval_const_expr()`, `generate_expr_ir()`) добавлена обработка FString |
| `src/frontend/core/lexer/tests/fstring.rs` | Новый | 10 тестов лексера |
| `src/frontend/core/parser/tests/fstring.rs` | Новый | 6 тестов парсера |
| `src/frontend/typecheck/tests/fstring.rs` | Новый | 4 теста проверки типов |
| `tests/integration/fstring.rs` | Новый | 7 сквозных интеграционных тестов |
| `tests/integration.rs` | Изменение | Регистрация модуля интеграционных тестов fstring |

### Ключевые моменты реализации

1. **Лексер**: f-string сохраняется как один токен `FStringLiteral`, маркеры `{}` интерполяции остаются в содержимом строки
2. **Парсер**: `parse_fstring_segments()` разбивает原始 содержимое на сегменты `Text`/`Interpolation`, выражения интерполяции повторно парсятся через полный лексер+парсер
3. **Генерация кода**: преобразование в вызов `std.string.format()` с позиционными заполнителями `{0}`, `{1}` и т.д.; спецификаторы формата как `{0:.2f}` передаются напрямую
4. **Оптимизация констант**: когда все выражения интерполяции являются compile-time константами (и без спецификаторов формата), весь f-string сворачивается в константную строку во время компиляции