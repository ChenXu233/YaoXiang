---
title: 'RFC 013: Спецификация кодов ошибок'
status: 'Принято'
author: '晨煦'
created: '2026-02-02'
updated: '2026-02-12'
issue: '#125'
issues_impl:
  - '#125'
pr_impl:
  - '#7'
  - '#9'
  - '#29'
  - '#66'
---

# RFC 013: Спецификация кодов ошибок

## Резюме

Данный RFC предлагает спецификацию классификации кодов ошибок компилятора YaoXiang, использующую
одноуровневую систему нумерации, аналогичную Rust, с поддержкой многоязычности через файлы ресурсов
JSON и предоставлением функции объяснения ошибок через команду `yaoxiang explain`.

## Мотивация

### Зачем нужна стандартизированная система кодов ошибок?

1. **Пользовательский опыт**: Пользователи, видя код ошибки, могут быстро определить тип и
   серьёзность ошибки
2. **Организация документации**: Группировка по категориям упрощает написание и поддержку справочной
   документации по ошибкам
3. **Интеграция с инструментами**: IDE/LSP могут предоставлять предложения по быстрому исправлению и
   ссылки на документацию на основе кода ошибки
4. **Поддержка интернационализации**: Разделение сообщений об ошибках и кодов упрощает многоязычный
   перевод

### Цели проектирования

- **Простота**: Одноуровневая нумерация, пользователю не нужно запоминать сложные правила
  классификации
- **Удобство**: Формат сообщений об ошибках, аналогичный Rust, со справочной информацией и примерами
- **Расширяемость**: Управление через файлы ресурсов, лёгкость добавления новых ошибок и языков
- **Удобство для инструментов**: Команда explain + вывод в формате JSON, поддержка интеграции с
  IDE/LSP

---

## Предложение

### Основной дизайн: Одноуровневая система нумерации

Используется четырёхзначная нумерация с группировкой по этапам компиляции:

```
Exxxx
││││
│││└── Порядковый номер (000-999)
││└─── Этап компиляции (0-9)
└───── Фиксированный префикс 'E'
```

### Разделение по этапам

| Этап  | Диапазон | Описание                            |
| ----- | -------- | ----------------------------------- |
| **0** | E0xxx    | Лексический и синтаксический анализ |
| **1** | E1xxx    | Проверка типов                      |
| **2** | E2xxx    | Семантический анализ                |
| **3** | E3xxx    | Генерация кода                      |
| **4** | E4xxx    | Дженерики и трейты                  |
| **5** | E5xxx    | Модули и импорт                     |
| **6** | E6xxx    | Ошибки времени выполнения           |
| **7** | E7xxx    | Ошибки ввода-вывода и системные     |
| **8** | E8xxx    | Внутренние ошибки компилятора       |
| **9** | E9xxx    | Зарезервировано/Экспериментально    |

### Перечисление категорий ошибок

```rust
/// Категория ошибки
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ErrorCategory {
    Lexer,      // E0xxx: Лексический и синтаксический анализ
    Parser,     // E0xxx: Parser errors
    TypeCheck,  // E1xxx: Проверка типов
    Semantic,   // E2xxx: Семантический анализ
    Generic,    // E4xxx: Дженерики и трейты
    Module,     // E5xxx: Модули и импорт
    Runtime,    // E6xxx: Ошибки времени выполнения
    Io,         // E7xxx: Ошибки ввода-вывода и системные
    Internal,   // E8xxx: Внутренние ошибки компилятора
}
```

### Определение кода ошибки и универсальный Builder

**Основной принцип**: Разделение определения кода ошибки и отображаемого текста

- `ErrorCodeDefinition`: Метаданные кода ошибки (code, category, template), без отображаемого текста
- `locales/*.json`: Отображаемый текст для каждого языка (title, message, help, коды ошибок являются
  вложенными объектами)
- `DiagnosticBuilder`: Универсальный построитель, заменяющий дизайн trait-per-error

#### Определение кода ошибки

```rust
// diagnostic/codes/mod.rs

use crate::util::span::Span;
use crate::util::diagnostic::{Diagnostic, Severity};

/// Определение кода ошибки (только метаданные, отображаемый текст в файлах i18n)
#[derive(Debug, Clone, Copy)]
pub struct ErrorCodeDefinition {
    pub code: &'static str,
    pub category: ErrorCategory,
    pub message_template: &'static str,  // Шаблон сообщения, поддерживает плейсхолдеры {param}
}

/// Универсальный построитель диагностики
pub struct DiagnosticBuilder {
    code: &'static str,
    message_template: &'static str,
    params: Vec<(&'static str, String)>,
    span: Option<Span>,
}

impl DiagnosticBuilder {
    pub fn new(code: &'static str, template: &'static str) -> Self {
        Self {
            code,
            message_template: template,
            params: Vec::new(),
            span: None,
        }
    }

    /// Добавить параметр шаблона
    pub fn param(mut self, key: &'static str, value: impl Into<String>) -> Self {
        self.params.push((key, value.into()));
        self
    }

    /// Установить позицию
    pub fn at(mut self, span: Span) -> Self {
        self.span = Some(span);
        self
    }

    /// Построить Diagnostic (рендеринг шаблона завершается в compile-time)
    pub fn build(&self, i18n: &I18nRegistry) -> Diagnostic {
        // Проверить, что для всех {key} в шаблоне есть соответствующие параметры
        self.validate_params();

        let message = i18n.render(self.message_template, &self.params);
        let help = self.help(i18n);

        Diagnostic {
            severity: Severity::Error,
            code: self.code.to_string(),
            message,
            help,
            span: self.span,
            related: Vec::new(),
        }
    }
}
```

#### Удобные методы для каждого кода ошибки

```rust
// diagnostic/codes/e1xxx.rs

impl ErrorCodeDefinition {
    /// E1001 Неизвестная переменная
    pub fn unknown_variable(name: &str) -> DiagnosticBuilder {
        let def = Self::find("E1001").unwrap();
        DiagnosticBuilder::new(def.code, def.message_template)
            .param("name", name)
    }

    /// E1002 Несоответствие типов
    pub fn type_mismatch(expected: &str, found: &str) -> DiagnosticBuilder {
        let def = Self::find("E1002").unwrap();
        DiagnosticBuilder::new(def.code, def.message_template)
            .param("expected", expected)
            .param("found", found)
    }
}
```

#### Пример использования

```rust
// checking/mod.rs

use crate::util::diagnostic::codes::{ErrorCodeDefinition, E1001};

// Упрощённый способ
return Err(E1001::unknown_variable(&var_name)
    .at(span)
    .build(&i18n_registry));

// Ручной способ
return Err(ErrorCodeDefinition::find("E1001")
    .builder()
    .param("name", var_name)
    .at(span)
    .build(&i18n_registry));
```

#### Пример определения кода ошибки

```rust
// diagnostic/codes/e1xxx.rs

pub static E1XXX: &[ErrorCodeDefinition] = &[
    ErrorCodeDefinition {
        code: "E1001",
        category: ErrorCategory::TypeCheck,
        message_template: "Unknown variable: '{name}'",
    },
    ErrorCodeDefinition {
        code: "E1002",
        category: ErrorCategory::TypeCheck,
        message_template: "Expected type '{expected}', found type '{found}'",
    },
    // ... другие коды ошибок
];
```

#### Преимущества дизайна

| Особенность                                       | Описание                                                    |
| ------------------------------------------------- | ----------------------------------------------------------- |
| **Единый Builder**                                | Один `DiagnosticBuilder` для всех кодов ошибок              |
| **Безопасность типов**                            | Удобные методы гарантируют корректность параметров          |
| **Самодокументируемость**                         | `E1001::unknown_variable(name)` очевидно                    |
| **Разделение шаблонов**                           | Шаблоны сообщений отделены от кода, легко для i18n          |
| **Нулевые накладные расходы во время выполнения** | Рендеринг в compile-time, без таблиц поиска в AOT-бинарнике |

---

### Упрощение макросов ошибок

#### Макрос error! (автоматическая инъекция контекста)

```rust
/// Макрос, автоматически получающий span и конфигурацию i18n в compile-time
macro_rules! error {
    ($code:ident, $($key:ident = $value:expr),* $(,)?) => {
        $code()
            $(.$key($value))*
            .at(crate::util::span::Span::current())
            .build(crate::util::diagnostic::I18nRegistry::current())
    };
}

/// Использование: нужно передать только параметры, span и i18n внедряются автоматически
return Err(error!(E1001, name = var_name));
return Err(error!(E1002, expected = "bool", found = cond_ty));
```

#### Ручное использование Builder

```rust
// Когда требуется ручное управление
E1001::unknown_variable(&var_name)
    .at(my_span)           // Пользовательский span
    .build(&custom_i18n)   // Пользовательский i18n
```

---

## Детальный дизайн

### Список кодов ошибок

#### E0xxx: Лексический и синтаксический анализ

| Код   | Тип ошибки                | Описание                                                      |
| ----- | ------------------------- | ------------------------------------------------------------- |
| E0001 | Invalid character         | Исходный код содержит недопустимые символы                    |
| E0002 | Invalid number literal    | Неверный формат числового литерала                            |
| E0003 | Unterminated string       | Многострочная строка без закрывающей кавычки                  |
| E0004 | Invalid character literal | Некорректный символьный литерал                               |
| E0010 | Expected token            | Ожидается определённый токен во время синтаксического анализа |
| E0011 | Unexpected token          | Встречен неожиданный токен                                    |
| E0012 | Invalid syntax            | Синтаксическая ошибка выражения/оператора                     |
| E0013 | Mismatched brackets       | Несоответствие круглых, квадратных или фигурных скобок        |
| E0014 | Missing semicolon         | В конце оператора отсутствует точка с запятой                 |

#### E1xxx: Проверка типов

| Код   | Тип ошибки               | Описание                                                          |
| ----- | ------------------------ | ----------------------------------------------------------------- |
| E1001 | Unknown variable         | Ссылка на неопределённую переменную                               |
| E1002 | Type mismatch            | Ожидаемый тип не соответствует фактическому                       |
| E1003 | Unknown type             | Ссылка на несуществующий тип                                      |
| E1010 | Parameter count mismatch | Количество аргументов вызова функции не соответствует определению |
| E1011 | Parameter type mismatch  | Проверка типа аргумента не прошла                                 |
| E1012 | Return type mismatch     | Неверный тип возвращаемого значения функции                       |
| E1013 | Function not found       | Вызов неопределённой функции                                      |
| E1020 | Cannot infer type        | Контекст не позволяет вывести тип                                 |
| E1021 | Type inference conflict  | Множественные ограничения приводят к противоречию типов           |
| E1030 | Pattern non-exhaustive   | Выражение match не покрывает все случаи                           |
| E1031 | Unreachable pattern      | Шаблон, который никогда не может совпасть                         |
| E1040 | Operation not supported  | Тип не поддерживает данную операцию                               |
| E1041 | Index out of bounds      | Индекс массива/списка вне диапазона                               |
| E1042 | Field not found          | Обращение к несуществующему полю структуры                        |

#### E2xxx: Семантический анализ

| Код   | Тип ошибки           | Описание                                                 |
| ----- | -------------------- | -------------------------------------------------------- |
| E2001 | Scope error          | Переменная не находится в текущей области видимости      |
| E2002 | Duplicate definition | Повторное определение в одной области видимости          |
| E2003 | Lifetime error       | Ограничения времени жизни не выполнены                   |
| E2010 | Immutable assignment | Попытка изменить неизменяемую переменную                 |
| E2011 | Uninitialized use    | Использование неинициализированной переменной            |
| E2012 | Mutability conflict  | Использование изменяемой ссылки в неизменяемом контексте |

#### E4xxx: Дженерики и трейты

| Код   | Тип ошибки                     | Описание                                               |
| ----- | ------------------------------ | ------------------------------------------------------ |
| E4001 | Generic parameter mismatch     | Несоответствие количества/типа дженерик-параметров     |
| E4002 | Trait bound violated           | Не выполнено ограничение trait                         |
| E4003 | Associated type error          | Ошибка определения/использования ассоциированного типа |
| E4004 | Duplicate trait implementation | Повторная реализация одного trait                      |
| E4005 | Trait not found                | Не удаётся найти требуемый trait                       |
| E4006 | Sized bound violated           | Не выполнено ограничение Sized                         |

#### E5xxx: Модули и импорт

| Код   | Тип ошибки          | Описание                                       |
| ----- | ------------------- | ---------------------------------------------- |
| E5001 | Module not found    | Импортируемый модуль не существует             |
| E5002 | Cyclic import       | Циклическая зависимость между модулями         |
| E5003 | Symbol not exported | Попытка доступа к не экспортированному символу |
| E5004 | Invalid module path | Неверный формат пути модуля                    |
| E5005 | Private access      | Доступ к приватному символу                    |

#### E6xxx: Ошибки времени выполнения

| Код   | Тип ошибки                  | Описание                                     |
| ----- | --------------------------- | -------------------------------------------- |
| E6001 | Division by zero            | Деление целого числа на ноль                 |
| E6002 | ~~Assertion failed~~        | ~~Зарезервировано (концепция удалена, #280)~~|
| E6003 | Runtime index out of bounds | Выход индекса за границы во время выполнения |
| E6004 | Stack overflow              | Исчерпание пространства стека                |
| E6005 | Assertion failed            | Сбой утверждения                             |
| E6006 | Function not found          | Функция не найдена во время выполнения       |
| E6007 | Runtime error (generic)     | Общая ошибка выполнения                      |

#### E7xxx: Ошибки ввода-вывода и системные

| Код   | Тип ошибки        | Описание                             |
| ----- | ----------------- | ------------------------------------ |
| E7001 | File not found    | Попытка чтения несуществующего файла |
| E7002 | Permission denied | Недостаточно прав доступа к файлу    |
| E7003 | I/O error         | Общая ошибка ввода-вывода            |
| E7004 | Network error     | Сбой сетевой операции                |

#### E8xxx: Внутренние ошибки компилятора

| Код   | Тип ошибки              | Описание                              |
| ----- | ----------------------- | ------------------------------------- |
| E8001 | Internal compiler error | Внутренняя ошибка компилятора         |
| E8002 | Codegen error           | Сбой генерации IR/байткода            |
| E8003 | Unimplemented feature   | Использование нереализованной функции |
| E8004 | Optimization error      | Ошибка оптимизации компилятора        |

---

### Файлы ресурсов для многоязычной поддержки

#### Формат файла ресурсов

```json
// locales/en.json
{
  "E1001": {
    "title": "Unknown variable",
    "message": "Referenced variable is not defined",
    "template": "Unknown variable: '{name}'",
    "help": "Check if the variable name is spelled correctly, or define it first",
    "example": "x = 100;",
    "error_output": "error[E1001]: Unknown variable: 'x'\n  --> example.yx:1:1\n   |\n 1 | print(x)\n   | ^ unknown variable 'x'"
  },
  "E1002": {
    "title": "Type mismatch",
    "message": "Expected type does not match actual type",
    "template": "Expected type '{expected}', found type '{found}'",
    "help": "Use the correct type or add a type conversion",
    "example": "x: Int = \"hello\";",
    "error_output": "error[E1002]: Type mismatch\n  --> example.yx:1:12\n   |\n 1 | x: Int = \"hello\";\n   |            ^ expected 'Int', found 'String'"
  }
}
```

```json
// locales/zh.json
{
  "E1001": {
    "title": "未知变量",
    "message": "引用的变量未定义",
    "template": "未知变量：'{name}'",
    "help": "检查变量名是否拼写正确，或先定义它",
    "example": "x = 100;",
    "error_output": "error[E1001]: 未知变量：'x'\n  --> example.yx:1:1\n   |\n 1 | print(x)\n   | ^ 未知变量 'x'"
  },
  "E1002": {
    "title": "类型不匹配",
    "message": "期望类型与实际类型不匹配",
    "template": "期望类型 '{expected}'，实际类型 '{found}'",
    "help": "使用正确的类型或添加类型转换",
    "example": "x: Int = \"hello\";",
    "error_output": "error[E1002]: 类型不匹配\n  --> example.yx:1:12\n   |\n 1 | x: Int = \"hello\";\n   |            ^ 期望 'Int'，找到 'String'"
  }
}
```

#### Реализация I18nRegistry

```rust
// locales/*.json (объекты кодов ошибок)

/// Реестр отображаемого текста i18n (загружается из JSON в compile-time, нулевой поиск во время выполнения)
pub struct I18nRegistry {
    /// Заголовки
    titles: HashMap<&'static str, &'static str>,
    /// Описания
    messages: HashMap<&'static str, &'static str>,
    /// Справочная информация
    helps: HashMap<&'static str, &'static str>,
    /// Примеры кода
    examples: HashMap<&'static str, &'static str>,
    /// Примеры вывода ошибок
    error_outputs: HashMap<&'static str, &'static str>,
}

/// Информация об отдельном коде ошибки
#[derive(Clone, Copy)]
pub struct ErrorInfo<'a> {
    pub title: &'a str,
    pub message: &'a str,
    pub help: &'a str,
    pub example: Option<&'a str>,
    pub error_output: Option<&'a str>,
}

impl I18nRegistry {
    /// Получить реестр по коду языка
    pub fn new(lang: &str) -> Self {
        match lang {
            "zh" => Self::zh(),
            _ => Self::en(),
        }
    }

    /// Получить информацию об ошибке
    pub fn get_info(&self, code: &str) -> Option<ErrorInfo<'_>> {
        Some(ErrorInfo {
            title: self.titles.get(code)?,
            message: self.messages.get(code)?,
            help: self.helps.get(code)?,
            example: self.examples.get(code).copied(),
            error_output: self.error_outputs.get(code).copied(),
        })
    }

    /// Рендеринг шаблона (завершается в compile-time, нулевые накладные расходы во время выполнения)
    pub fn render(&self, template: &'static str, params: &[(&str, String)]) -> String {
        let mut result = String::with_capacity(template.len() + 64);
        let mut chars = template.chars().peekable();

        while let Some(c) = chars.next() {
            if c == '{' {
                let mut key = String::new();
                while let Some(&c) = chars.peek() {
                    if c == '}' {
                        chars.next();
                        if let Some((_, value)) = params.iter().find(|(k, _)| k == &key) {
                            result.push_str(value);
                        } else {
                            result.push_str(&format!("{{{}}}", key));
                        }
                        break;
                    }
                    key.push(c);
                    chars.next();
                }
            } else {
                result.push(c);
            }
        }
        result
    }
}
```

#### Плейсхолдеры шаблонов

##### Предопределённые плейсхолдеры (часто используемые)

| Плейсхолдер  | Назначение                                         | Пример                              |
| ------------ | -------------------------------------------------- | ----------------------------------- |
| `{name}`     | Имя переменной/типа/trait и других идентификаторов | `Unknown variable: '{name}'`        |
| `{expected}` | Ожидаемый тип                                      | `Expected type '{expected}'`        |
| `{found}`    | Фактический/найденный тип                          | `, found type '{found}'`            |
| `{method}`   | Имя метода                                         | `Method {method} is not a function` |
| `{trait}`    | Имя trait                                          | `Cannot find trait: {trait}`        |
| `{path}`     | Путь модуля                                        | `Invalid path: {path}`              |
| `{ty}`       | Выражение типа                                     | `Invalid type: {ty}`                |
| `{message}`  | Внутреннее сообщение об ошибке                     | `Internal error: {message}`         |

##### Поддержка произвольных ключей

**params поддерживает произвольные ключи, не ограничиваясь предопределёнными**. Вызывающая сторона
может передавать любые `key`:

```rust
// Использование произвольных ключей
E1001::unknown_variable(&var_name)
    .param("location", "global scope")
    .param("hint", "try declaring it first")
    .at(span)
    .build(&i18n);

// Определение шаблона
"Unknown variable: '{name}' at {location}. {hint}"
```

> **Примечание**: Не все коды ошибок используют плейсхолдеры. Некоторые коды ошибок (например,
> E0001) являются статическими сообщениями и не требуют параметров.

#### Приоритет языка

```
1. yaoxiang.toml [language.default]
2. ~/.yaoxiang/yaoxiang.toml [language.default]
3. Значение по умолчанию: en
```

### Конфигурация yaoxiang.toml

#### Конфигурация уровня проекта

```toml
# yaoxiang.toml
[project]
name = "my-project"
version = "0.1.0"

[language]
# Язык сообщений об ошибках, опции: en, zh, ja, ...
default = "zh"
```

#### Конфигурация уровня пользователя

```toml
# ~/.yaoxiang/yaoxiang.toml
[language]
default = "zh"
```

#### Выбор языка во время компиляции

```
1. Прочитать language.default из yaoxiang.toml уровня проекта
2. Если не настроено, прочитать ~/.yaoxiang/yaoxiang.toml уровня пользователя
3. Если ни то, ни другое не настроено, использовать "en" по умолчанию
4. Компилятор создаёт I18nRegistry на основе выбранного языка (один раз)
5. Все ошибки используют этот I18nRegistry для рендеринга сообщений
```

#### Ключ к нулевым накладным расходам на поиск

**Рендеринг происходит во время компиляции пользовательского проекта, а не во время выполнения.**

```
┌─────────────────────────────────────────────────────────────────────────┐
│  Этап 1: Rust компилирует компилятор YaoXiang                          │
│                                                                           │
│  JSON упаковывается в бинарник компилятора                               │
│  Цель: команда explain может напрямую читать данные i18n                 │
└─────────────────────────────────────────────────────────────────────────┘

┌─────────────────────────────────────────────────────────────────────────┐
│  Этап 2: YaoXiang компилирует пользовательский проект (здесь происходит рендеринг) │
│                                                                           │
│  При вызове макроса error!:                                              │
│  1. Прочитать yaoxiang.toml для получения языковых предпочтений           │
│  2. Загрузить i18n JSON для соответствующего языка из бинарника компилятора │
│  3. Шаблон + параметры → render() → "Unknown variable: 'x'"            │
│  4. Diagnostic.message = отрендеренная строка                            │
│                                                                           │
│  AOT-бинарник напрямую хранит финальные строки, без шаблонов, без поиска  │
└─────────────────────────────────────────────────────────────────────────┘

┌─────────────────────────────────────────────────────────────────────────┐
│  Этап 3: Время выполнения пользовательской программы                    │
│                                                                           │
│  println!("{}", diagnostic.message)                                      │
│  // Прямой вывод финальной строки, без какого-либо поиска                │
└─────────────────────────────────────────────────────────────────────────┘
```

| Компонент                    | Ответственность                            | Время рендеринга                         |
| ---------------------------- | ------------------------------------------ | ---------------------------------------- |
| `I18nRegistry`               | Предоставляет шаблоны и отображаемый текст | При компиляции пользовательского проекта |
| `DiagnosticBuilder.render()` | Шаблон + параметры → финальная строка      | При компиляции пользовательского проекта |
| `Diagnostic.message`         | Отрендеренная строка                       | Хранит финальный результат               |
| AOT-бинарник                 | Содержит финальные строки                  | Прямое использование во время выполнения |

---

### Формат сообщений об ошибках

Сообщения об ошибках используют следующий формат:

```
error[E####]: <Краткое описание>
  --> <файл>:<строка>:<столбец>
   <строка> | <фрагмент кода>
          ^^^<подсветка>
```

#### Полный пример

```
error[E1001]: Unknown variable: x
  --> src/main.yx:5:12
   5 |   print(x)
          ^
          help: Did you mean to define it?
```

---

### Уровни серьёзности

Уровень серьёзности ошибки управляется перечислением `DiagnosticLevel`, отделённым от номера кода
ошибки:

```rust
pub enum DiagnosticLevel {
    Error,    // Приводит к сбою компиляции
    Warning,  // Не влияет на компиляцию, но рекомендуется исправить
    Note,     // Дополнительная информация
    Help,     // Предложение по исправлению
}
```

| Уровень | Префикс           | Описание                   |
| ------- | ----------------- | -------------------------- |
| Error   | `error[E####]:`   | Приводит к сбою компиляции |
| Warning | `warning[E####]:` | Не влияет на компиляцию    |
| Note    | `note[E####]:`    | Дополнительная информация  |
| Help    | `help[E####]:`    | Предложение по исправлению |

---

### Команда `yaoxiang explain`

#### Синтаксис команды

```bash
yaoxiang explain <ERROR_CODE> [OPTIONS]
```

#### Опции

| Опция           | Описание                                        |
| --------------- | ----------------------------------------------- |
| `--lang <code>` | Указать язык (en-US, zh-CN, по умолчанию en-US) |
| `--json`        | Вывод в формате JSON (для IDE/LSP)              |
| `--json-pretty` | Форматированный вывод JSON                      |
| `--examples`    | Показывать только примеры кода                  |
| `--help`        | Показать справочную информацию                  |

#### Примеры использования

```bash
# По умолчанию на английском
$ yaoxiang explain E1001
error[E1001]: Unknown variable: {name}
  --> <file>:<line>:<col>

Help: Did you mean to define it?

Example:
  let {name} = value;

# Вывод на китайском
$ yaoxiang explain E1001 --lang zh
error[E1001]: 未知变量: {name}
  --> <file>:<line>:<col>

帮助: 你是否想要定义它？

示例:
  let {name} = value;

# Вывод в JSON (интеграция с LSP)
$ yaoxiang explain E1001 --json
{
  "code": "E1001",
  "message": "Unknown variable: {name}",
  "help": "Did you mean to define it?",
  "examples": ["let {name} = value;"],
  "language": "en-US"
}
```

#### Формат вывода JSON

```json
{
  "code": "E1001",
  "message": "Unknown variable: {name}",
  "help": "Did you mean to define it?",
  "examples": ["let {name} = value;"],
  "language": "en-US"
}
```

---

### Обратная совместимость

Поскольку данный RFC проектирует систему кодов ошибок с нуля, проблемы обратной совместимости
отсутствуют.

**Стратегия будущей миграции** (для справки в последующих версиях):

1. Сохранить отображение старых кодов ошибок на новые
2. Во время миграции одновременно отображать старые и новые коды
3. Предоставить график устаревания

---

## Стратегия реализации

### Этап первый: Базовая инфраструктура кодов ошибок

1. Создать структуру каталога `src/diagnostics/`
2. Реализовать перечисление `ErrorCode`
3. Реализовать `Diagnostic` и `DiagnosticLevel`
4. Создать каталог файлов ресурсов и примеры JSON

### Этап второй: Команда explain

1. Реализовать CLI-команду `yaoxiang explain`
2. Поддержка опций `--lang` и `--json`
3. Интеграция загрузки файлов ресурсов
4. Реализация рендеринга шаблонов параметров

### Этап третий: Интеграция на этапе компиляции

1. Обновить все точки сообщения об ошибках для использования новой системы
2. Реализовать инъекцию параметров шаблона сообщения
3. Добавить логику приоритета языка
4. Покрытие модульными тестами

### Этап четвёртый: Интеграция с IDE/LSP

1. LSP-сервер интегрирует JSON-вывод команды explain
2. Отображение ссылок на коды ошибок в IDE
3. Отображение объяснения ошибки при наведении
4. Предложения по быстрому исправлению

---

## Приложение

### Полная таблица кодов ошибок

| Диапазон | Категория                           |
| -------- | ----------------------------------- |
| E0xxx    | Лексический и синтаксический анализ |
| E1xxx    | Проверка типов                      |
| E2xxx    | Семантический анализ                |
| E3xxx    | Генерация кода                      |
| E4xxx    | Дженерики и трейты                  |
| E5xxx    | Модули и импорт                     |
| E6xxx    | Ошибки времени выполнения           |
| E7xxx    | Ошибки ввода-вывода и системные     |
| E8xxx    | Внутренние ошибки компилятора       |
| E9xxx    | Зарезервировано                     |

### Поддерживаемые языки

| Код   | Язык         | Статус       |
| ----- | ------------ | ------------ |
| en-US | English (US) | По умолчанию |
| zh-CN | 简体中文     | В планах     |

### Сравнение примеров сообщений об ошибках

```
# Английский (en-US)
error[E1001]: Unknown variable: x
  --> src/main.yx:5:12
   5 |   print(x)
          ^
          help: Did you mean to define it?

# Китайский (zh-CN)
error[E1001]: 未知变量: x
  --> src/main.yx:5:12
   5 |   print(x)
          ^
          帮助: 你是否想要定义它？
```

## Ссылки

- [Индекс ошибок компилятора Rust](https://doc.rust-lang.org/error_codes/error-index.html)
- [Формат сообщений об ошибках GCC](https://gcc.gnu.org/onlinedocs/gcc-13.1.0/gcc/Warning-Options.html)
- [Формат диагностики Clang](https://clang.llvm.org/diagnostics.html)
- [Language Server Protocol](https://microsoft.github.io/language-server-protocol/)
