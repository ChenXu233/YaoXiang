---
title: "RFC 013: Стандарт кодов ошибок"
status: "Принят"
author: "晨煦"
created: "2026-02-02"
updated: "2026-02-12"
---

# RFC 013: Стандарт кодов ошибок

## Аннотация

Настоящий RFC предлагает стандарт классификации кодов ошибок компилятора YaoXiang, использующий одноуровневую систему нумерации в стиле Rust, с JSON-файлами ресурсов для поддержки многоязычности и командой `yaoxiang explain` для предоставления объяснений ошибок.

## Мотивация

### Зачем нужен стандартизированный код ошибки?

1. **Пользовательский опыт**: пользователь, видя код ошибки, может быстро определить тип и серьёзность ошибки
2. **Организация документации**: группировка по категориям упрощает написание и поддержку справочной документации по ошибкам
3. **Интеграция инструментов**: IDE/LSP может предоставлять предложения по быстрому исправлению и ссылки на документацию на основе кодов ошибок
4. **Поддержка интернационализации**: разделение сообщений об ошибках и кодов упрощает перевод на несколько языков

### Цели проектирования

- **Простота**: одноуровневая нумерация, пользователю не нужно запоминать сложные правила классификации
- **Дружелюбность**: формат сообщений об ошибках в стиле Rust, с информацией для справки и примерами
- **Расширяемость**: управление через файлы ресурсов, легко добавлять новые ошибки и новые языки
- **Дружелюбность к инструментам**: команда explain + JSON-вывод, поддержка интеграции IDE/LSP

---

## Предложение

### Основной дизайн: одноуровневая система нумерации

Используется четырёхзначная нумерация, сгруппированная по этапам компиляции:

```
Exxxx
││││
│││└── Порядковый номер (000-999)
││└─── Этап компиляции (0-9)
└───── Фиксированный префикс 'E'
```

### Разделение на этапы

| Этап | Диапазон | Описание |
|------|----------|----------|
| **0** | E0xxx | Лексический и синтаксический анализ |
| **1** | E1xxx | Проверка типов |
| **2** | E2xxx | Семантический анализ |
| **3** | E3xxx | Генерация кода |
| **4** | E4xxx | 泛型 и 特质 |
| **5** | E5xxx | Модули и импорт |
| **6** | E6xxx | Ошибки времени выполнения |
| **7** | E7xxx | Ошибки ввода-вывода и системы |
| **8** | E8xxx | Внутренние ошибки компилятора |
| **9** | E9xxx | Зарезервировано/экспериментальное |

### Перечисление категорий ошибок

```rust
/// Категория ошибки
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ErrorCategory {
    Lexer,      // E0xxx: Лексема и синтаксический анализ
    Parser,     // E0xxx: Ошибки парсера
    TypeCheck,  // E1xxx: Проверка типов
    Semantic,   // E2xxx: Семантический анализ
    Generic,    // E4xxx: 泛型 и 特质
    Module,     // E5xxx: Модули и импорт
    Runtime,    // E6xxx: Ошибки времени выполнения
    Io,         // E7xxx: Ошибки ввода-вывода и системы
    Internal,   // E8xxx: Внутренние ошибки компилятора
}
```

### Определение кодов ошибок и универсальный Builder

**Основной принцип**: разделение определения кодов ошибок и текстов отображения

- `ErrorCodeDefinition`: метаданные кода ошибки (code, category, template), без текстов отображения
- `i18n/*.json`: тексты отображения для каждого языка (title, message, help)
- `DiagnosticBuilder`: универсальный строитель, заменяющий проектирование trait-per-error

#### Определение кодов ошибок

```rust
// diagnostic/codes/mod.rs

use crate::util::span::Span;
use crate::util::diagnostic::{Diagnostic, Severity};

/// Определение кода ошибки (только метаданные, тексты отображения в файлах i18n)
#[derive(Debug, Clone, Copy)]
pub struct ErrorCodeDefinition {
    pub code: &'static str,
    pub category: ErrorCategory,
    pub message_template: &'static str,  // Шаблон сообщения, поддерживает заполнители {param}
}

/// Универсальный строитель диагностики
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

    /// Построить Diagnostic (рендеринг шаблона выполняется на этапе компиляции)
    pub fn build(&self, i18n: &I18nRegistry) -> Diagnostic {
        // Проверить, что все {key} в шаблоне имеют соответствующие параметры
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

#### Краткие методы для каждого кода ошибки

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
    // ... Другие коды ошибок
];
```

#### Преимущества дизайна

| Характеристика | Описание |
|----------------|----------|
| **Единый Builder** | Один `DiagnosticBuilder` универсален для всех кодов ошибок |
| **Типобезопасность** | Краткие методы обеспечивают правильность параметров |
| **Самодокументирование** | `E1001::unknown_variable(name)` понятно с первого взгляда |
| **Разделение шаблона** | Шаблон сообщения отделён от кода, легко делать i18n |
| **Нулевые накладные расходы во время выполнения** | Рендеринг на этапе компиляции, в AOT-бинаре нет поиска по таблицам |

---

### Упрощённый макрос ошибок

#### Макрос error! (автоматическое внедрение контекста)

```rust
/// Макрос для автоматического получения span и конфигурации i18n на этапе компиляции
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
// Когда нужно контролировать вручную
E1001::unknown_variable(&var_name)
    .at(my_span)           // Пользовательский span
    .build(&custom_i18n)   // Пользовательский i18n
```

---

## Детальное проектирование

### Список кодов ошибок

#### E0xxx：Лексический и синтаксический анализ

| Код | Тип ошибки | Описание |
|------|------------|----------|
| E0001 | Invalid character | Исходный код содержит недопустимый символ |
| E0002 | Invalid number literal | Неправильный формат числового 字面量 |
| E0003 | Unterminated string | Многострочная строка без закрывающей кавычки |
| E0004 | Invalid character literal | Некорректный символьный 字面量 |
| E0010 | Expected token | Во время синтаксического анализа ожидался определённый token |
| E0011 | Unexpected token | Встречен неожиданный token |
| E0012 | Invalid syntax | Синтаксическая ошибка в выражении/операторе |
| E0013 | Mismatched brackets | Несоответствие круглых, квадратных или фигурных скобок |
| E0014 | Missing semicolon | В конце оператора отсутствует точка с запятой |

#### E1xxx：Проверка типов

| Код | Тип ошибки | Описание |
|------|------------|----------|
| E1001 | Unknown variable | Ссылка на неопределённую переменную |
| E1002 | Type mismatch | Ожидаемый тип не соответствует фактическому |
| E1003 | Unknown type | Ссылка на несуществующий тип |
| E1010 | Parameter count mismatch | Количество параметров при вызове функции не соответствует определению |
| E1011 | Parameter type mismatch | Ошибка проверки типа параметра |
| E1012 | Return type mismatch | Неправильный тип возвращаемого значения функции |
| E1013 | Function not found | Вызов неопределённой функции |
| E1020 | Cannot infer type | Тип не может быть выведен из контекста |
| E1021 | Type inference conflict | Противоречие типов из нескольких ограничений |
| E1030 | Pattern non-exhaustive | Выражение match не покрывает все случаи |
| E1031 | Unreachable pattern | Паттерн, который никогда не будет сопоставлен |
| E1040 | Operation not supported | Тип не поддерживает данную операцию |
| E1041 | Index out of bounds | Индекс массива/списка выходит за пределы |
| E1042 | Field not found | Обращение к несуществующему полю структуры |

#### E2xxx：Семантический анализ

| Код | Тип ошибки | Описание |
|------|------------|----------|
| E2001 | Scope error | Переменная не находится в текущей области видимости |
| E2002 | Duplicate definition | Повторное определение в одной области видимости |
| E2003 | Lifetime error | Ограничение времени жизни не удовлетворено |
| E2010 | Immutable assignment | Попытка изменить неизменяемую переменную |
| E2011 | Uninitialized use | Использование неинициализированной переменной |
| E2012 | Mutability conflict | Использование изменяемой ссылки в неизменяемом контексте |

#### E4xxx：泛型 и 特质

| Код | Тип ошибки | Описание |
|------|------------|----------|
| E4001 | Generic parameter mismatch | Количество/тип параметров 泛型 не соответствует |
| E4002 | Trait bound violated | Ограничение trait не удовлетворено |
| E4003 | Associated type error | Ошибка определения/использования связанного типа |
| E4004 | Duplicate trait implementation | Повторная реализация того же trait |
| E4005 | Trait not found | Запрашиваемый trait не найден |
| E4006 | Sized bound violated | Ограничение Sized не удовлетворено |

#### E5xxx：Модули и импорт

| Код | Тип ошибки | Описание |
|------|------------|----------|
| E5001 | Module not found | Импортируемый модуль не существует |
| E5002 | Cyclic import | Циклическая зависимость между модулями |
| E5003 | Symbol not exported | Попытка обращения к неэкспортированному символу |
| E5004 | Invalid module path | Неправильный формат пути модуля |
| E5005 | Private access | Обращение к приватному символу |

#### E6xxx：Ошибки времени выполнения

| Код | Тип ошибки | Описание |
|------|------------|----------|
| E6001 | Division by zero | Деление целого числа на ноль |
| E6002 | Assertion failed | Макрос assert! не прошёл |
| E6003 | Arithmetic overflow | Переполнение при арифметических операциях |
| E6004 | Stack overflow | Исчерпание пространства стека |
| E6005 | Heap allocation failed | Сбой выделения памяти |
| E6006 | Runtime index out of bounds | Выход индекса за пределы во время выполнения |
| E6007 | Type cast failed | Попытка преобразования типа в несовместимый тип |

#### E7xxx：Ошибки ввода-вывода и системы

| Код | Тип ошибки | Описание |
|------|------------|----------|
| E7001 | File not found | Попытка чтения несуществующего файла |
| E7002 | Permission denied | Недостаточно прав доступа к файлу |
| E7003 | I/O error | Общая ошибка ввода-вывода |
| E7004 | Network error | Сбой сетевой операции |

#### E8xxx：Внутренние ошибки компилятора

| Код | Тип ошибки | Описание |
|------|------------|----------|
| E8001 | Internal compiler error | Внутренняя ошибка компилятора |
| E8002 | Codegen error | Сбой генерации IR/байткода |
| E8003 | Unimplemented feature | Использование нереализованной функциональности |
| E8004 | Optimization error | Ошибка оптимизации компилятора |

---

### Файлы ресурсов для многоязычности

#### Формат файлов ресурсов

```json
// diagnostic/codes/i18n/en.json
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
// diagnostic/codes/i18n/zh.json
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

```json
// diagnostic/codes/i18n/ru.json
{
  "E1001": {
    "title": "Неизвестная переменная",
    "message": "Ссылаемая переменная не определена",
    "template": "Неизвестная переменная: '{name}'",
    "help": "Проверьте правильность написания имени переменной или сначала определите её",
    "example": "x = 100;",
    "error_output": "error[E1001]: Неизвестная переменная: 'x'\n  --> example.yx:1:1\n   |\n 1 | print(x)\n   | ^ неизвестная переменная 'x'"
  },
  "E1002": {
    "title": "Несоответствие типов",
    "message": "Ожидаемый тип не соответствует фактическому",
    "template": "Ожидался тип '{expected}', найден тип '{found}'",
    "help": "Используйте правильный тип или добавьте преобразование типа",
    "example": "x: Int = \"hello\";",
    "error_output": "error[E1002]: Несоответствие типов\n  --> example.yx:1:12\n   |\n 1 | x: Int = \"hello\";\n   |            ^ ожидался 'Int', найден 'String'"
  }
}
```

#### Реализация I18nRegistry

```rust
// diagnostic/codes/i18n/mod.rs

/// Реестр текстов отображения i18n (загружается из JSON на этапе компиляции, нулевой поиск во время выполнения)
pub struct I18nRegistry {
    /// Заголовки
    titles: HashMap<&'static str, &'static str>,
    /// Описания
    messages: HashMap<&'static str, &'static str>,
    /// Информация для справки
    helps: HashMap<&'static str, &'static str>,
    /// Примеры кода
    examples: HashMap<&'static str, &'static str>,
    /// Примеры вывода ошибок
    error_outputs: HashMap<&'static str, &'static str>,
}

/// Информация об одной ошибке
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
            "ru" => Self::ru(),
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

    /// Рендеринг шаблона (выполняется на этапе компиляции, нулевые накладные расходы во время выполнения)
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

#### Заполнители шаблона

##### Предопределённые заполнители (часто используемые)

| Заполнитель | Назначение | Пример |
|-------------|------------|--------|
| `{name}` | Имя переменной/типа/трейта и др. идентификаторы | `Unknown variable: '{name}'` |
| `{expected}` | Ожидаемый тип | `Expected type '{expected}'` |
| `{found}` | Фактический/найденный тип | `, found type '{found}'` |
| `{method}` | Имя метода | `Method {method} is not a function` |
| `{trait}` | Имя 特质 | `Cannot find trait: {trait}` |
| `{path}` | Путь модуля | `Invalid path: {path}` |
| `{ty}` | Выражение типа | `Invalid type: {ty}` |
| `{message}` | Внутреннее сообщение об ошибке | `Internal error: {message}` |

##### Поддержка произвольных ключей

**params поддерживает произвольные ключи, не ограничиваясь предопределёнными**. Вызывающая сторона может передать любой `key`:

```rust
// Использование произвольного ключа
E1001::unknown_variable(&var_name)
    .param("location", "global scope")
    .param("hint", "try declaring it first")
    .at(span)
    .build(&i18n);

// Определение шаблона
"Unknown variable: '{name}' at {location}. {hint}"
```

> **Примечание**: Не все коды ошибок используют заполнители. Некоторые коды ошибок (например, E0001) — статические сообщения, не требующие параметров.

#### Приоритет языков

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
# Язык сообщений об ошибках, возможные значения: en, zh, ru, ...
default = "ru"
```

#### Конфигурация уровня пользователя

```toml
# ~/.yaoxiang/yaoxiang.toml
[language]
default = "ru"
```

#### Выбор языка на этапе компиляции

```
1. Читается language.default из yaoxiang.toml уровня проекта
2. Если не настроено, читается ~/.yaoxiang/yaoxiang.toml
3. Если оба не настроены, по умолчанию используется "en"
4. Компилятор создаёт I18nRegistry на основе выбранного языка (один раз)
5. Все ошибки используют этот I18nRegistry для рендеринга сообщений
```

#### Ключ к нулевым накладным расходам на поиск по таблицам

**Рендеринг происходит при компиляции пользовательского проекта, а не во время выполнения.**

```
┌─────────────────────────────────────────────────────────────────────────┐
│  Этап 1: Компиляция компилятора YaoXiang на Rust                        │
│                                                                           │
│  JSON упаковывается в бинарный файл компилятора                         │
│  Цель: команда explain может напрямую читать данные i18n                 │
└─────────────────────────────────────────────────────────────────────────┘

┌─────────────────────────────────────────────────────────────────────────┐
│  Этап 2: Компиляция YaoXiang пользовательского проекта (рендеринг здесь)│
│                                                                           │
│  При вызове макроса error!:                                              │
│  1. Читается yaoxiang.toml для получения языковых предпочтений           │
│  2. Из бинарного файла компилятора загружается i18n JSON для языка       │
│  3. Шаблон + параметры → render() → "Unknown variable: 'x'"            │
│  4. Diagnostic.message = уже отрендеренная строка                        │
│                                                                           │
│  В AOT-бинаре хранится готовая строка без шаблона, без поиска           │
└─────────────────────────────────────────────────────────────────────────┘

┌─────────────────────────────────────────────────────────────────────────┐
│  Этап 3: Время выполнения пользовательской программы                     │
│                                                                           │
│  println!("{}", diagnostic.message)                                      │
│  // Прямой вывод готовой строки, без какого-либо поиска                   │
└─────────────────────────────────────────────────────────────────────────┘
```

| Компонент | Ответственность | Момент рендеринга |
|-----------|-----------------|-------------------|
| `I18nRegistry` | Предоставление шаблона и текстов отображения | При компиляции пользовательского проекта |
| `DiagnosticBuilder.render()` | Шаблон + параметры → финальная строка | При компиляции пользовательского проекта |
| `Diagnostic.message` | Отрендеренная строка | Хранение финального результата |
| AOT-бинарь | Содержит финальные строки | Прямое использование во время выполнения |

---

### Формат сообщений об ошибках

Сообщения об ошибках используют следующий формат:

```
error[E####]: <краткое описание>
  --> <файл>:<строка>:<столбец>
   <строка> | <фрагмент кода>
          ^^^<подсветка>
```

#### Полный пример

```
error[E1001]: Неизвестная переменная: x
  --> src/main.yx:5:12
   5 |   print(x)
          ^
          help: Возможно, вы хотели её определить?
```

---

### Уровни серьёзности

Серьёзность ошибок управляется через перечисление `DiagnosticLevel`, отделённое от нумерации кодов ошибок:

```rust
pub enum DiagnosticLevel {
    Error,    // Приводит к сбою компиляции
    Warning,  // Не влияет на компиляцию, но рекомендуется исправить
    Note,     // Дополнительная информация
    Help,     // Предложение по исправлению
}
```

| Уровень | Префикс | Описание |
|---------|---------|----------|
| Error | `error[E####]:` | Приводит к сбою компиляции |
| Warning | `warning[E####]:` | Не влияет на компиляцию |
| Note | `note[E####]:` | Дополнительная информация |
| Help | `help[E####]:` | Предложение по исправлению |

---

### Команда `yaoxiang explain`

#### Синтаксис команды

```bash
yaoxiang explain <ERROR_CODE> [OPTIONS]
```

#### Опции

| Опция | Описание |
|-------|----------|
| `--lang <code>` | Указать язык (en-US, ru-RU, zh-CN, по умолчанию en-US) |
| `--json` | Вывод в формате JSON (для IDE/LSP) |
| `--json-pretty` | Форматированный вывод JSON |
| `--examples` | Показать только примеры кода |
| `--help` | Показать справочную информацию |

#### Примеры использования

```bash
# По умолчанию на английском
$ yaoxiang explain E1001
error[E1001]: Unknown variable: {name}
  --> <file>:<line>:<col>

Help: Did you mean to define it?

Example:
  let {name} = value;

# Вывод на русском
$ yaoxiang explain E1001 --lang ru
error[E1001]: Неизвестная переменная: {name}
  --> <file>:<line>:<col>

Справка: Возможно, вы хотели её определить?

Пример:
  let {name} = value;

# Вывод в формате JSON (интеграция с LSP)
$ yaoxiang explain E1001 --json
{
  "code": "E1001",
  "message": "Unknown variable: {name}",
  "help": "Did you mean to define it?",
  "examples": ["let {name} = value;"],
  "language": "en-US"
}
```

#### Формат JSON-вывода

```json
{
  "code": "E1001",
  "message": "Unknown variable: {name}",
  "help": "Did you mean to define it?",
  "examples": [
    "let {name} = value;"
  ],
  "language": "en-US"
}
```

---

### Обратная совместимость

Поскольку данный RFC разрабатывает систему кодов ошибок с нуля, проблем обратной совместимости не существует.

**Стратегия миграции в будущем** (для справки в последующих версиях):

1. Сохранить отображение старых кодов ошибок на новые
2. Во время миграции отображать одновременно старые и новые коды
3. Предоставить график устаревания

---

## Стратегия реализации

### Этап первый: Базовая инфраструктура кодов ошибок

1. Создать структуру каталогов `src/diagnostics/`
2. Реализовать перечисление `ErrorCode`
3. Реализовать `Diagnostic` и `DiagnosticLevel`
4. Создать каталог файлов ресурсов и примеры JSON

### Этап второй: Команда explain

1. Реализовать CLI-команду `yaoxiang explain`
2. Поддержать опции `--lang` и `--json`
3. Интегрировать загрузку файлов ресурсов
4. Реализовать рендеринг шаблона параметров

### Этап третий: Интеграция с компилятором

1. Обновить все точки отчётности об ошибках для использования новой системы
2. Реализовать внедрение параметров шаблона сообщений
3. Добавить логику приоритета языков
4. Покрыть юнит-тестами

### Этап четвёртый: Интеграция с IDE/LSP

1. Интегрировать JSON-вывод explain в LSP-сервер
2. Отображать ссылки на коды ошибок в IDE
3. Показывать объяснения ошибок при наведении
4. Предложения по быстрому исправлению

---

## Приложения

### Полная справочная таблица кодов ошибок

| Диапазон | Категория |
|----------|-----------|
| E0xxx | Лексический и синтаксический анализ |
| E1xxx | Проверка типов |
| E2xxx | Семантический анализ |
| E3xxx | Генерация кода |
| E4xxx | 泛型 и 特质 |
| E5xxx | Модули и импорт |
| E6xxx | Ошибки времени выполнения |
| E7xxx | Ошибки ввода-вывода и системы |
| E8xxx | Внутренние ошибки компилятора |
| E9xxx | Зарезервировано |

### Поддерживаемые языки

| Код | Язык | Статус |
|------|------|--------|
| en-US | English (US) | По умолчанию |
| zh-CN | 简体中文 | Запланировано |
| ru-RU | Русский | Запланировано |

### Сравнение примеров сообщений об ошибках

```
# English (en-US)
error[E1001]: Unknown variable: x
  --> src/main.yx:5:12
   5 |   print(x)
          ^
          help: Did you mean to define it?

# Русский (ru-RU)
error[E1001]: Неизвестная переменная: x
  --> src/main.yx:5:12
   5 |   print(x)
          ^
          справка: Возможно, вы хотели её определить?
```

## Список литературы

- [Индекс ошибок компилятора Rust](https://doc.rust-lang.org/error_codes/error-index.html)
- [Формат сообщений об ошибках GCC](https://gcc.gnu.org/onlinedocs/gcc-13.1.0/gcc/Warning-Options.html)
- [Формат диагностики Clang](https://clang.llvm.org/diagnostics.html)
- [Language Server Protocol](https://microsoft.github.io/language-server-protocol/)