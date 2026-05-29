```yaml
---
title: RFC-013: Стандарт кодов ошибок
---

# RFC 013: Стандарт кодов ошибок

> **Статус**: Принято
> **Автор**: Чэнь Сюй
> **Дата создания**: 2026-02-02
> **Последнее обновление**: 2026-02-12

## Аннотация

Данный RFC предлагает стандарт классификации кодов ошибок компилятора YaoXiang, использующий одноуровневую систему нумерации в стиле Rust совместно с JSON-файлами ресурсов для поддержки нескольких языков и команду `yaoxiang explain` для предоставления объяснений ошибок.

## Мотивация

### Зачем нужны стандартизированные коды ошибок?

1. **Опыт пользователя**: пользователь может быстро определить тип и серьёзность ошибки по коду
2. **Организация документации**: группировка по категориям упрощает написание и поддержку справочной документации
3. **Интеграция с инструментами**: IDE/LSP могут предоставлять предложения по быстрому исправлению и ссылки на документацию на основе кодов ошибок
4. **Поддержка интернационализации**: разделение сообщений об ошибках и кодов упрощает перевод на несколько языков

### Цели проектирования

- **Простота**: одноуровневая нумерация, пользователям не нужно запоминать сложные правила классификации
- **Дружелюбность**: формат сообщений об ошибках в стиле Rust с информацией для справки и примерами
- **Расширяемость**: управление через файлы ресурсов, легко добавлять новые ошибки и новые языки
- **Удобство для инструментов**: команда explain + вывод в формате JSON, поддержка интеграции IDE/LSP

---

## Предложение

### Основной дизайн: одноуровневая система нумерации

Используется четырёхзначная нумерация с группировкой по этапам компиляции:

```
Exxxx
││││
│││└── Порядковый номер (000-999)
││└─── Этап компиляции (0-9)
└───── Фиксированный префикс 'E'
```

### Разделение по этапам

| Этап | Диапазон | Описание |
|------|----------|----------|
| **0** | E0xxx | Лексический и синтаксический анализ |
| **1** | E1xxx | Проверка типов |
| **2** | E2xxx | Семантический анализ |
| **3** | E3xxx | Генерация кода |
| **4** | E4xxx | Обобщения и трайты |
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
    Lexer,      // E0xxx: Лексический и синтаксический анализ
    Parser,     // E0xxx: Ошибки парсера
    TypeCheck,  // E1xxx: Проверка типов
    Semantic,   // E2xxx: Семантический анализ
    Generic,    // E4xxx: Обобщения и трайты
    Module,     // E5xxx: Модули и импорт
    Runtime,    // E6xxx: Ошибки времени выполнения
    Io,         // E7xxx: Ошибки ввода-вывода и системы
    Internal,   // E8xxx: Внутренние ошибки компилятора
}
```

### Определения кодов ошибок и универсальный Builder

**Основной принцип**: разделение определений кодов ошибок и отображаемого текста

- `ErrorCodeDefinition`: метаданные кода ошибки (code, category, template), без отображаемого текста
- `i18n/*.json`: отображаемый текст для каждого языка (title, message, help)
- `DiagnosticBuilder`: универсальный строитель, заменяющий подход с отдельным trait на каждую ошибку

#### Определения кодов ошибок

```rust
// diagnostic/codes/mod.rs

use crate::util::span::Span;
use crate::util::diagnostic::{Diagnostic, Severity};

/// Определение кода ошибки (только метаданные, отображаемый текст в файлах i18n)
#[derive(Debug, Clone, Copy)]
pub struct ErrorCodeDefinition {
    pub code: &'static str,
    pub category: ErrorCategory,
    pub message_template: &'static str,  // Шаблон сообщения с поддержкой заполнителей {param}
}

/// Универсальный строитель диагностик
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

    /// Построить Diagnostic (рендеринг шаблона выполняется во время компиляции)
    pub fn build(&self, i18n: &I18nRegistry) -> Diagnostic {
        // Проверка, что все заполнители {key} в шаблоне имеют соответствующие параметры
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
|------|----------|
| **Единый Builder** | Один `DiagnosticBuilder` для всех кодов ошибок |
| **Типобезопасность** | Краткие методы обеспечивают правильность параметров |
| **Самодокументирование** | `E1001::unknown_variable(name)` сразу понятно |
| **Разделение шаблонов** | Шаблоны сообщений отделены от кода, легко делать i18n |
| **Нулевые накладные расходы во время выполнения** | Рендеринг во время компиляции, в AOT-бинаре нет таблиц |

---

### Упрощение макроса ошибок

#### Макрос error! (автоматическое внедрение контекста)

```rust
/// Макрос для автоматического получения span и конфигурации i18n во время компиляции
macro_rules! error {
    ($code:ident, $($key:ident = $value:expr),* $(,)?) => {
        $code()
            $(.$key($value))*
            .at(crate::util::span::Span::current())
            .build(crate::util::diagnostic::I18nRegistry::current())
    };
}

/// Использование: достаточно передать параметры, span и i18n внедряются автоматически
return Err(error!(E1001, name = var_name));
return Err(error!(E1002, expected = "bool", found = cond_ty));
```

#### Ручное использование Builder

```rust
// Когда нужен ручной контроль
E1001::unknown_variable(&var_name)
    .at(my_span)           // Свой span
    .build(&custom_i18n)   // Свой i18n
```

---

## Детальный дизайн

### Список кодов ошибок

#### E0xxx: Лексический и синтаксический анализ

| Код | Тип ошибки | Описание |
|------|----------|------|
| E0001 | Invalid character | Исходный код содержит недопустимый символ |
| E0002 | Invalid number literal | Неправильный формат числового литерала |
| E0003 | Unterminated string | Многострочная строка без закрывающей кавычки |
| E0004 | Invalid character literal | Неправильный символьный литерал |
| E0010 | Expected token | Во время синтаксического анализа ожидался определённый токен |
| E0011 | Unexpected token | Встречен неожиданный токен |
| E0012 | Invalid syntax | Синтаксическая ошибка в выражении/операторе |
| E0013 | Mismatched brackets | Несовпадение круглых, квадратных или фигурных скобок |
| E0014 | Missing semicolon | В конце оператора отсутствует точка с запятой |

#### E1xxx: Проверка типов

| Код | Тип ошибки | Описание |
|------|----------|------|
| E1001 | Unknown variable | Ссылка на неопределённую переменную |
| E1002 | Type mismatch | Ожидаемый тип не соответствует фактическому |
| E1003 | Unknown type | Ссылка на несуществующий тип |
| E1010 | Parameter count mismatch | Количество параметров при вызове функции не соответствует определению |
| E1011 | Parameter type mismatch | Ошибка проверки типа параметра |
| E1012 | Return type mismatch | Неправильный тип возвращаемого значения функции |
| E1013 | Function not found | Вызов неопределённой функции |
| E1020 | Cannot infer type | Тип не может быть выведен из контекста |
| E1021 | Type inference conflict | Конфликт ограничений в нескольких местах приводит к противоречию типов |
| E1030 | Pattern non-exhaustive | Выражение match не покрывает все случаи |
| E1031 | Unreachable pattern | Паттерн, который никогда не сможет совпасть |
| E1040 | Operation not supported | Тип не поддерживает данную операцию |
| E1041 | Index out of bounds | Индекс массива/списка выходит за границы |
| E1042 | Field not found | Обращение к несуществующему полю структуры |

#### E2xxx: Семантический анализ

| Код | Тип ошибки | Описание |
|------|----------|------|
| E2001 | Scope error | Переменная находится не в текущей области видимости |
| E2002 | Duplicate definition | Повторное определение в той же области видимости |
| E2003 | Lifetime error | Ограничение времени жизни не удовлетворено |
| E2010 | Immutable assignment | Попытка изменить неизменяемую переменную |
| E2011 | Uninitialized use | Использование неинициализированной переменной |
| E2012 | Mutability conflict | Использование изменяемой ссылки в неизменяемом контексте |

#### E4xxx: Обобщения и трайты

| Код | Тип ошибки | Описание |
|------|----------|------|
| E4001 | Generic parameter mismatch | Несоответствие количества/типа параметров обобщения |
| E4002 | Trait bound violated | Ограничение трайта не удовлетворено |
| E4003 | Associated type error | Ошибка определения/использования ассоциированного типа |
| E4004 | Duplicate trait implementation | Повторная реализация того же трайта |
| E4005 | Trait not found | Не удалось найти требуемый трайт |
| E4006 | Sized bound violated | Ограничение Sized не удовлетворено |

#### E5xxx: Модули и импорт

| Код | Тип ошибки | Описание |
|------|----------|------|
| E5001 | Module not found | Импортируемый модуль не существует |
| E5002 | Cyclic import | Циклическая зависимость между модулями |
| E5003 | Symbol not exported | Попытка обращения к неэкспортированному символу |
| E5004 | Invalid module path | Неправильный формат пути к модулю |
| E5005 | Private access | Обращение к приватному символу |

#### E6xxx: Ошибки времени выполнения

| Код | Тип ошибки | Описание |
|------|----------|------|
| E6001 | Division by zero | Деление целого числа на ноль |
| E6002 | Assertion failed | Ошибка макроса assert! |
| E6003 | Arithmetic overflow | Переполнение при арифметических операциях |
| E6004 | Stack overflow | Исчерпание пространства стека |
| E6005 | Heap allocation failed | Сбой выделения памяти |
| E6006 | Runtime index out of bounds | Выход индекса за границы во время выполнения |
| E6007 | Type cast failed | Попытка привести тип к несовместимому |

#### E7xxx: Ошибки ввода-вывода и системы

| Код | Тип ошибки | Описание |
|------|----------|------|
| E7001 | File not found | Попытка чтения несуществующего файла |
| E7002 | Permission denied | Недостаточно прав на файл |
| E7003 | I/O error | Общая ошибка ввода-вывода |
| E7004 | Network error | Сбой сетевой операции |

#### E8xxx: Внутренние ошибки компилятора

| Код | Тип ошибки | Описание |
|------|----------|------|
| E8001 | Internal compiler error | Внутренняя ошибка компилятора |
| E8002 | Codegen error | Сбой генерации IR/байткода |
| E8003 | Unimplemented feature | Использование нереализованной функции |
| E8004 | Optimization error | Ошибка оптимизации компилятора |

---

### Файлы ресурсов для нескольких языков

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
    "error_output": "error[E1001]: 未知变量：'x'\n  --> example.yx:1:1\n   |\n 1 | print(x)\n   | ^ unknown variable 'x'"
  },
  "E1002": {
    "title": "类型不匹配",
    "message": "期望类型与实际类型不匹配",
    "template": "期望类型 '{expected}'，实际类型 '{found}'",
    "help": "使用正确的类型或添加类型转换",
    "example": "x: Int = \"hello\";",
    "error_output": "error[E1002]: 类型不匹配\n  --> example.yx:1:12\n   |\n 1 | x: Int = \"hello\";\n   |            ^ expected 'Int', found 'String'"
  }
}
```

#### Реализация I18nRegistry

```rust
// diagnostic/codes/i18n/mod.rs

/// Реестр отображаемого текста i18n (загружается из JSON во время компиляции, нулевые накладные расходы во время выполнения)
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

/// Информация об одном коде ошибки
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

    /// Рендеринг шаблона (выполняется во время компиляции, нулевые накладные расходы во время выполнения)
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
|--------|------|------|
| `{name}` | Имя переменной/типа/трайта и других идентификаторов | `Unknown variable: '{name}'` |
| `{expected}` | Ожидаемый тип | `Expected type '{expected}'` |
| `{found}` | Фактический/найденный тип | `, found type '{found}'` |
| `{method}` | Имя метода | `Method {method} is not a function` |
| `{trait}` | Имя трайта | `Cannot find trait: {trait}` |
| `{path}` | Путь к модулю | `Invalid path: {path}` |
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

> **Примечание**: Не все коды ошибок используют заполнители. Некоторые коды ошибок (например, E0001) — это статические сообщения, не требующие параметров.

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
# Язык сообщений об ошибках: en, zh, ja, ...
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
1. Чтение language.default из yaoxiang.toml проекта
2. Если не настроено, чтение ~/.yaoxiang/yaoxiang.toml пользователя
3. Если ничего не настроено, используется "en" по умолчанию
4. Компилятор создаёт I18nRegistry на основе выбранного языка (один раз)
5. Все ошибки используют этот I18nRegistry для рендеринга сообщений
```

#### Ключ к нулевым накладным расходам при поиске в таблицах

**Рендеринг происходит во время компиляции пользовательского проекта, а не во время выполнения.**

```
┌─────────────────────────────────────────────────────────────────────────┐
│  Этап 1: Компиляция компилятора YaoXiang на Rust                        │
│                                                                           │
│  JSON упаковывается в бинарный файл компилятора                          │
│  Цель: команда explain может напрямую читать данные i18n                  │
└─────────────────────────────────────────────────────────────────────────┘

┌─────────────────────────────────────────────────────────────────────────┐
│  Этап 2: Компиляция пользовательского проекта YaoXiang (рендеринг здесь)│
│                                                                           │
│  При вызове макроса error!:                                              │
│  1. Чтение языковых предпочтений из yaoxiang.toml                        │
│  2. Загрузка JSON i18n для соответствующего языка из бинарника           │
│  3. Шаблон + параметры → render() → "Unknown variable: 'x'"              │
│  4. Diagnostic.message = уже отрендеренная строка                        │
│                                                                           │
│  AOT-бинарь напрямую хранит финальные строки, без шаблонов, без таблиц  │
└─────────────────────────────────────────────────────────────────────────┘

┌─────────────────────────────────────────────────────────────────────────┐
│  Этап 3: Время выполнения пользовательской программы                     │
│                                                                           │
│  println!("{}", diagnostic.message)                                     │
│  // Прямой вывод финальной строки, без поиска в таблицах                 │
└─────────────────────────────────────────────────────────────────────────┘
```

| Компонент | Ответственность | Момент рендеринга |
|------|------|----------|
| `I18nRegistry` | Предоставление шаблонов и отображаемого текста | Во время компиляции пользовательского проекта |
| `DiagnosticBuilder.render()` | Шаблон + параметры → финальная строка | Во время компиляции пользовательского проекта |
| `Diagnostic.message` | Отрендеренная строка | Хранение финального результата |
| AOT-бинарь | Содержит финальные строки | Используется напрямую во время выполнения |

---

### Формат сообщений об ошибках

Сообщения об ошибках имеют следующий формат:

```
error[E####]: <Краткое описание>
  --> <файл>:<строка>:<столбец>
   <строка> | <фрагмент кода>
          ^^^<выделение>
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

Серьёзность ошибок управляется перечислением `DiagnosticLevel` и не связана с нумерацией кодов ошибок:

```rust
pub enum DiagnosticLevel {
    Error,    // Приводит к сбою компиляции
    Warning,  // Не влияет на компиляцию, но рекомендуется исправить
    Note,     // Дополнительная информация
    Help,     // Предложение по исправлению
}
```

| Уровень | Префикс | Описание |
|------|------|----------|
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
|------|------|
| `--lang <code>` | Указать язык (en-US, zh-CN, по умолчанию en-US) |
| `--json` | Вывод в формате JSON (для IDE/LSP) |
| `--json-pretty` | Форматированный вывод JSON |
| `--examples` | Показывать только примеры кода |
| `--help` | Показать справку |

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

#### Формат вывода JSON

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

Поскольку данный RFC проектирует систему кодов ошибок с нуля, проблем с обратной совместимостью не существует.

**Стратегия миграции в будущем** (для справки в последующих версиях):

1. Сохранить отображение старых кодов ошибок в новые
2. Во время миграции отображать оба кода (старый и новый) одновременно
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
4. Реализовать рендеринг параметров шаблона

### Этап третий: Интеграция с компилятором

1. Обновить все точки сообщений об ошибках для использования новой системы
2. Реализовать внедрение параметров шаблона сообщения
3. Добавить логику приоритета языков
4. Покрыть модульными тестами

### Этап четвёртый: Интеграция с IDE/LSP

1. Интеграция вывода explain в формате JSON с LSP-сервером
2. Отображение ссылок на коды ошибок в IDE
3. Показ объяснения ошибки при наведении
4. Предложения по быстрому исправлению

---

## Приложение

### Таблица быстрого поиска кодов ошибок

| Диапазон | Категория |
|------|------|
| E0xxx | Лексический и синтаксический анализ |
| E1xxx | Проверка типов |
| E2xxx | Семантический анализ |
| E3xxx | Генерация кода |
| E4xxx | Обобщения и трайты |
| E5xxx | Модули и импорт |
| E6xxx | Ошибки времени выполнения |
| E7xxx | Ошибки ввода-вывода и системы |
| E8xxx | Внутренние ошибки компилятора |
| E9xxx | Зарезервировано |

### Поддерживаемые языки

| Код | Язык | Статус |
|------|------|------|
| en-US | English (US) | По умолчанию |
| zh-CN | 简体中文 | Запланировано |

### Сравнение примеров сообщений об ошибках

```
# English (en-US)
error[E1001]: Unknown variable: x
  --> src/main.yx:5:12
   5 |   print(x)
          ^
          help: Did you mean to define it?

# 中文 (zh-CN)
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
```