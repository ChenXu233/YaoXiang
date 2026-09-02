---
title: 'RFC 013: Спецификация кодов ошибок'
status: 'Принято'
author: 'Чэнь Сюй'
created: '2026-02-02'
updated: '2026-09-03'
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

Настоящий RFC предлагает спецификацию классификации кодов ошибок компилятора YaoXiang, использующую
одноуровневую систему нумерации по аналогии с Rust, с поддержкой многоязычности через файлы ресурсов
JSON, а также предоставление функциональности объяснения ошибок через команду `yaoxiang explain`.

## Мотивация

### Зачем нужна стандартизированная система кодов ошибок?

1. **Пользовательский опыт**: Пользователи, видя код ошибки, могут быстро определить её тип и
   серьёзность
2. **Организация документации**: Группировка по категориям упрощает написание и сопровождение
   справочной документации по ошибкам
3. **Интеграция с инструментами**: IDE/LSP могут предоставлять предложения по быстрому исправлению и
   ссылки на документацию на основе кода ошибки
4. **Поддержка интернационализации**: Разделение сообщений и кодов ошибок упрощает многоязычный
   перевод

### Цели проектирования

- **Простота**: Одноуровневая нумерация, пользователю не нужно запоминать сложные правила
  классификации
- **Дружелюбность**: Формат сообщений об ошибках по аналогии с Rust, с вспомогательной информацией и
  примерами
- **Расширяемость**: Управление через файлы ресурсов, лёгкость добавления новых ошибок и языков
- **Инструментальная дружелюбность**: Команда `explain` + вывод в формате JSON, поддержка интеграции
  с IDE/LSP

---

## Предложение

### Основной дизайн: Одноуровневая система нумерации

Используется четырёхзначная нумерация с группировкой по фазам компиляции:

```
Exxxx
││││
│││└── Порядковый номер (000-999)
││└─── Фаза компиляции (0-9)
└───── Фиксированный префикс 'E'
```

### Разделение по фазам

| Фаза  | Диапазон | Описание                               |
| ----- | -------- | -------------------------------------- |
| **0** | E0xxx    | Лексический и синтаксический анализ    |
| **1** | E1xxx    | Проверка типов                         |
| **2** | E2xxx    | Семантический анализ                   |
| **3** | E3xxx    | Генерация кода                         |
| **4** | E4xxx    | Обобщения и трейты                     |
| **5** | E5xxx    | Модули и импорт                        |
| **6** | E6xxx    | Ошибки времени выполнения              |
| **7** | E7xxx    | Ошибки ввода-вывода и системные ошибки |
| **8** | E8xxx    | Внутренние ошибки компилятора          |
| **9** | E9xxx    | Зарезервировано/Экспериментальные      |

### Перечисление категорий ошибок

```rust
/// 错误类别
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ErrorCategory {
    Lexer,      // E0xxx: 词法和语法分析
    Parser,     // E0xxx: Parser errors
    TypeCheck,  // E1xxx: 类型检查
    Semantic,   // E2xxx: 语义分析
    Generic,    // E4xxx: 泛型与特质
    Module,     // E5xxx: 模块与导入
    Runtime,    // E6xxx: 运行时错误
    Io,         // E7xxx: I/O与系统错误
    Internal,   // E8xxx: 内部编译器错误
}
```

### Определение кодов ошибок и универсальный Builder

**Основной принцип**: Разделение определения кода ошибки и отображаемого текста

- `ErrorCodeDefinition`: Метаданные кода ошибки (code, category, template), без отображаемого текста
- `locales/*.json`: Отображаемый текст на разных языках (title, message, help, коды ошибок в виде
  вложенных объектов)
- `DiagnosticBuilder`: Универсальный построитель, заменяющий дизайн с trait-per-error

#### Определение кода ошибки

```rust
// diagnostic/codes/mod.rs

use crate::util::span::Span;
use crate::util::diagnostic::{Diagnostic, Severity};

/// 错误码定义（仅元数据，展示文案在 i18n 文件）
#[derive(Debug, Clone, Copy)]
pub struct ErrorCodeDefinition {
    pub code: &'static str,
    pub category: ErrorCategory,
    pub message_template: &'static str,  // 消息模板，支持 {param} 占位符
}

/// 通用诊断构建器
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

    /// 添加模板参数
    pub fn param(mut self, key: &'static str, value: impl Into<String>) -> Self {
        self.params.push((key, value.into()));
        self
    }

    /// 设置位置
    pub fn at(mut self, span: Span) -> Self {
        self.span = Some(span);
        self
    }

    /// 构建 Diagnostic（模板渲染在编译期完成）
    pub fn build(&self, i18n: &I18nRegistry) -> Diagnostic {
        // 检查模板中所有 {key} 都有对应参数
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

#### Сокращённые методы для каждого кода ошибки

```rust
// diagnostic/codes/e1xxx.rs

impl ErrorCodeDefinition {
    /// E1001 未知变量
    pub fn unknown_variable(name: &str) -> DiagnosticBuilder {
        let def = Self::find("E1001").unwrap();
        DiagnosticBuilder::new(def.code, def.message_template)
            .param("name", name)
    }

    /// E1002 类型不匹配
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

// 简化方式
return Err(E1001::unknown_variable(&var_name)
    .at(span)
    .build(&i18n_registry));

// 手动方式
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
    // ... 其他错误码
];
```

#### Преимущества дизайна

| Свойство                                          | Описание                                                      |
| ------------------------------------------------- | ------------------------------------------------------------- |
| **Единый Builder**                                | Один `DiagnosticBuilder` универсален для всех кодов           |
| **Типобезопасность**                              | Сокращённые методы обеспечивают корректность параметров       |
| **Самодокументируемость**                         | `E1001::unknown_variable(name)` очевиден                      |
| **Разделение шаблонов**                           | Шаблоны сообщений отделены от кода, удобно для i18n           |
| **Нулевые накладные расходы во время выполнения** | Рендеринг во время компиляции, AOT-бинарник без таблиц поиска |

---

### Упрощение макросов ошибок

#### Макрос `error!` (автоматическая инъекция контекста)

```rust
/// 编译期自动获取 span 和 i18n 配置的宏
macro_rules! error {
    ($code:ident, $($key:ident = $value:expr),* $(,)?) => {
        $code()
            $(.$key($value))*
            .at(crate::util::span::Span::current())
            .build(crate::util::diagnostic::I18nRegistry::current())
    };
}

/// 使用：只需传参数，span 和 i18n 自动注入
return Err(error!(E1001, name = var_name));
return Err(error!(E1002, expected = "bool", found = cond_ty));
```

#### Ручное использование Builder

```rust
// 需要手动控制时
E1001::unknown_variable(&var_name)
    .at(my_span)           // 自定义 span
    .build(&custom_i18n)   // 自定义 i18n
```

---

## Детальный дизайн

### Список кодов ошибок

#### E0xxx: Лексический и синтаксический анализ

| Код   | Тип ошибки                | Описание                                                |
| ----- | ------------------------- | ------------------------------------------------------- |
| E0001 | Invalid character         | Исходный код содержит недопустимый символ               |
| E0002 | Invalid number literal    | Некорректный формат числового литерала                  |
| E0003 | Unterminated string       | Многострочная строка без закрывающей кавычки            |
| E0004 | Invalid character literal | Некорректный символьный литерал                         |
| E0010 | Expected token            | Ожидается определённый токен при синтаксическом анализе |
| E0011 | Unexpected token          | Обнаружен неожиданный токен                             |
| E0012 | Invalid syntax            | Синтаксическая ошибка выражения/оператора               |
| E0013 | Mismatched brackets       | Несоответствие круглых, квадратных или фигурных скобок  |
| E0014 | Missing semicolon         | В конце оператора отсутствует точка с запятой           |

#### E1xxx: Проверка типов

| Код   | Тип ошибки               | Описание                                                          |
| ----- | ------------------------ | ----------------------------------------------------------------- |
| E1001 | Unknown variable         | Ссылка на неопределённую переменную                               |
| E1002 | Type mismatch            | Ожидаемый тип не соответствует фактическому                       |
| E1003 | Unknown type             | Ссылка на несуществующий тип                                      |
| E1010 | Parameter count mismatch | Количество аргументов вызова не соответствует определению функции |
| E1011 | Parameter type mismatch  | Неудачная проверка типа аргумента                                 |
| E1012 | Return type mismatch     | Некорректный тип возвращаемого значения функции                   |
| E1013 | Function not found       | Вызов неопределённой функции                                      |
| E1020 | Cannot infer type        | Контекст не позволяет вывести тип                                 |
| E1021 | Type inference conflict  | Множественные ограничения приводят к противоречию типов           |
| E1030 | Pattern non-exhaustive   | Выражение `match` не покрывает все случаи                         |
| E1031 | Unreachable pattern      | Образец, который никогда не может совпасть                        |
| E1040 | Operation not supported  | Тип не поддерживает данную операцию                               |
| E1041 | Index out of bounds      | Индекс массива/списка вне допустимого диапазона                   |
| E1042 | Field not found          | Обращение к несуществующему полю структуры                        |

#### E2xxx: Семантический анализ

| Код   | Тип ошибки           | Описание                                                 |
| ----- | -------------------- | -------------------------------------------------------- |
| E2001 | Scope error          | Переменная не находится в текущей области видимости      |
| E2002 | Duplicate definition | Повторное определение в одной области видимости          |
| E2003 | Lifetime error       | Ограничения времени жизни не выполняются                 |
| E2010 | Immutable assignment | Попытка изменить неизменяемую переменную                 |
| E2011 | Uninitialized use    | Использование неинициализированной переменной            |
| E2012 | Mutability conflict  | Использование изменяемой ссылки в неизменяемом контексте |

#### E4xxx: Обобщения и трейты

| Код   | Тип ошибки                     | Описание                                               |
| ----- | ------------------------------ | ------------------------------------------------------ |
| E4001 | Generic parameter mismatch     | Несоответствие количества/типов параметров обобщения   |
| E4002 | Trait bound violated           | Не выполнено ограничение трейта                        |
| E4003 | Associated type error          | Ошибка определения/использования ассоциированного типа |
| E4004 | Duplicate trait implementation | Повторная реализация того же трейта                    |
| E4005 | Trait not found                | Не удаётся найти требуемый трейт                       |
| E4006 | Sized bound violated           | Не выполнено ограничение Sized                         |

#### E5xxx: Модули и импорт

| Код   | Тип ошибки          | Описание                                        |
| ----- | ------------------- | ----------------------------------------------- |
| E5001 | Module not found    | Импортируемый модуль не существует              |
| E5002 | Cyclic import       | Циклическая зависимость между модулями          |
| E5003 | Symbol not exported | Попытка обращения к неэкспортированному символу |
| E5004 | Invalid module path | Некорректный формат пути модуля                 |
| E5005 | Private access      | Обращение к приватному символу                  |

#### E6xxx: Ошибки времени выполнения

| Код   | Тип ошибки                  | Описание                                                     |
| ----- | --------------------------- | ------------------------------------------------------------ |
| E6001 | Division by zero            | Целочисленное деление на ноль                                |
| E6002 | ~~Assertion failed~~        | ~~Зарезервировано (нет языковой концепции, удалено)~~        |
| E6003 | Runtime index out of bounds | Выход индекса за границы во время выполнения (привязка #280) |
| E6004 | Stack overflow              | Исчерпание стекового пространства                            |
| E6005 | Assertion failed            | Сбой `assert` (привязка #280)                                |
| E6006 | Function not found          | Функция не найдена во время выполнения                       |
| E6007 | Runtime error (generic)     | Универсальная ошибка времени выполнения                      |

> **Редакция #280 (2026-08-09)**: Исходная таблица кодов была определена согласно черновику
> семантики Rust (Assertion failed/Arithmetic overflow/Heap allocation failed/Type cast failed), что
> не соответствовало реальным потребностям реализации. YaoXiang не имеет концепций нулевого
> указателя, сбоя аллокации в куче, преобразования типов (семантика значений + безопасность памяти
> Rust), и пути обнаружения переполнения во время выполнения не реализованы. После корректировки:
>
> - E6002 удалён (исходный Assertion failed перемещён в E6005; семантика исходного нулевого
>   указателя не имеет языковой концепции)
> - E6003 изменён с Arithmetic overflow на Runtime index out of bounds (реальная поверхность
>   срабатывания, #279/#271)
> - E6005 изменён с Heap allocation failed на Assertion failed (реальный путь std.assert)
> - E6006 изменён с Runtime index out of bounds на Function not found (реализация уже была такой,
>   #255)
> - E6007 изменён с Type cast failed на универсальный Runtime error (унифицированная точка падения
>   для несопоставленных вариантов ExecutorError)

#### E7xxx: Ошибки ввода-вывода и системные ошибки

| Код   | Тип ошибки        | Описание                             |
| ----- | ----------------- | ------------------------------------ |
| E7001 | File not found    | Попытка чтения несуществующего файла |
| E7002 | Permission denied | Недостаточно прав доступа к файлу    |
| E7003 | I/O error         | Универсальная ошибка ввода-вывода    |
| E7004 | Network error     | Сбой сетевой операции                |

#### E8xxx: Внутренние ошибки компилятора

| Код   | Тип ошибки              | Описание                                  |
| ----- | ----------------------- | ----------------------------------------- |
| E8001 | Internal compiler error | Внутренняя ошибка компилятора             |
| E8002 | Codegen error           | Сбой генерации IR/байт-кода               |
| E8003 | Unimplemented feature   | Использование нереализованной возможности |
| E8004 | Optimization error      | Ошибка оптимизации компилятора            |

---

### Связь значений ошибок времени выполнения с кодами

> Данный раздел введён задачей #323 (M4: Значение Error времени выполнения с кодом, 2026-09-03).
> Семантическое пространство E6xxx/E7xxx одновременно обслуживает два канала, пространство кодов
> общее, каналы представления различаются.

#### Два канала

| Канал                                  | Носитель                                                 | Способ представления                                                  |
| -------------------------------------- | -------------------------------------------------------- | --------------------------------------------------------------------- |
| Канал диагностики компилятора/CLI      | `ExecutorError` и другие критические ошибки уровня хоста | stderr `error[E####]:` (E6003/E6005/E6007 подключены через #280/#281) |
| Канал значений ошибок внутри программы | `Error` в `Result(T, Error)` стандартной библиотеки      | Значение языка, потребляемое программой через `match`/сравнение       |

#### Структура `Error` (с v0.8, критическое изменение)

```
Error { code: String, message: String }
```

- `code` повторно использует нумерацию E6xxx/E7xxx данной спецификации в строковом виде (например,
  `"E6008"`).
- **Стабильный контракт**: Семантика уже выделенных кодов не изменяется между версиями; одна и та же
  семантика не использует повторно ранее удалённые коды (прецедент E6002).
- **Поверхность потребления**: Сравнение `e.code == "E6xxx"` внутри программы является единственным
  программируемым контрактом определения; документация `yaoxiang explain E6xxx` сквозная;
  инструментарий (LSP / DAP, см. RFC-034) использует код в качестве `exceptionId`.
- **Аксессоры**: `std.result.code(e)` / `std.result.message(e)`.
- **Пользовательские ошибки**: `E` в `Result(T, E)` является параметром обобщения, серьёзное
  моделирование осуществляется через пользовательские типы; `Error` стандартной библиотеки — лишь
  удобный запасной носитель, его система кодов не ограничивает пользовательский тип `E`.

#### Правила выделения кодов

1. Коды значений ошибок времени выполнения совместно используют пространство E6xxx/E7xxx с кодами
   диагностики времени компиляции; новые коды выделяются по **реальной поверхности срабатывания**,
   без резервирования под гипотетические сценарии.
2. Сначала регистрация, затем использование: новый код должен быть внесён в авторитетный реестр и
   пройти трёхстороннюю проверку согласованности (codes/*.rs ↔ locales ↔ таблица кодов данного
   документа), прежде чем может быть выпущен.
3. E7xxx зарезервирован для значений ошибок std.io / std.net (в настоящее время пуст, будет
   задействован при переводе io/net на Result).

#### Путь развития (линия C, не реализовано)

После завершения обеспечения полноты сопоставления с образцом (RFC-039) `Error` может быть обновлён
до `{ kind: ErrorKind, message: String }`, где `code` становится атрибутом, выводимым из `kind`
(определение варианта одновременно является регистром кодов). В период развития стабильный контракт
`code` данного раздела остаётся неизменным; данное обновление является независимым решением и не
является обязательством данного раздела.

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
// locales/*.json（错误码对象）

/// i18n 展示文案注册表（编译期从 JSON 加载，运行时零查表）
pub struct I18nRegistry {
    /// 标题
    titles: HashMap<&'static str, &'static str>,
    /// 描述
    messages: HashMap<&'static str, &'static str>,
    /// 帮助信息
    helps: HashMap<&'static str, &'static str>,
    /// 示例代码
    examples: HashMap<&'static str, &'static str>,
    /// 错误输出示例
    error_outputs: HashMap<&'static str, &'static str>,
}

/// 单个错误码信息
#[derive(Clone, Copy)]
pub struct ErrorInfo<'a> {
    pub title: &'a str,
    pub message: &'a str,
    pub help: &'a str,
    pub example: Option<&'a str>,
    pub error_output: Option<&'a str>,
}

impl I18nRegistry {
    /// 根据语言代码获取注册表
    pub fn new(lang: &str) -> Self {
        match lang {
            "zh" => Self::zh(),
            _ => Self::en(),
        }
    }

    /// 获取错误信息
    pub fn get_info(&self, code: &str) -> Option<ErrorInfo<'_>> {
        Some(ErrorInfo {
            title: self.titles.get(code)?,
            message: self.messages.get(code)?,
            help: self.helps.get(code)?,
            example: self.examples.get(code).copied(),
            error_output: self.error_outputs.get(code).copied(),
        })
    }

    /// 渲染模板（编译期完成，运行时零开销）
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

#### Плейсхолдеры шаблона

##### Предопределённые плейсхолдеры (часто используемые)

| Плейсхолдер  | Назначение                        | Пример                              |
| ------------ | --------------------------------- | ----------------------------------- |
| `{name}`     | Имя переменной/типа/трейта и т.д. | `Unknown variable: '{name}'`        |
| `{expected}` | Ожидаемый тип                     | `Expected type '{expected}'`        |
| `{found}`    | Фактический/найденный тип         | `, found type '{found}'`            |
| `{method}`   | Имя метода                        | `Method {method} is not a function` |
| `{trait}`    | Имя трейта                        | `Cannot find trait: {trait}`        |
| `{path}`     | Путь модуля                       | `Invalid path: {path}`              |
| `{ty}`       | Выражение типа                    | `Invalid type: {ty}`                |
| `{message}`  | Внутреннее сообщение об ошибке    | `Internal error: {message}`         |

##### Поддержка произвольных ключей

**`params` поддерживает произвольные ключи, не ограничиваясь предопределёнными**. Вызывающая сторона
может передавать любой `key`:

```rust
// 使用任意 key
E1001::unknown_variable(&var_name)
    .param("location", "global scope")
    .param("hint", "try declaring it first")
    .at(span)
    .build(&i18n);

// 模板定义
"Unknown variable: '{name}' at {location}. {hint}"
```

> **Примечание**: Не все коды ошибок используют плейсхолдеры. Некоторые коды ошибок (например,
> E0001) имеют статическое сообщение и не требуют параметров.

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
# 错误消息语言，可选：en, zh, ja, ...
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
1. Чтение language.default из yaoxiang.toml уровня проекта
2. Если не настроено, чтение ~/.yaoxiang/yaoxiang.toml уровня пользователя
3. Если ни одно не настроено, по умолчанию используется "en"
4. Компилятор создаёт I18nRegistry на основе выбранного языка (однократно)
5. Все ошибки отображаются с использованием этого I18nRegistry
```

#### Ключ к нулевым накладным расходам на поиск

**Рендеринг происходит во время компиляции пользовательского проекта, а не во время выполнения.**

```
┌─────────────────────────────────────────────────────────────────────────┐
│  Фаза 1: Компиляция компилятора YaoXiang в Rust                          │
│                                                                           │
│  JSON упаковывается в бинарник компилятора                               │
│  Цель: команда explain может напрямую читать данные i18n                 │
└─────────────────────────────────────────────────────────────────────────┘

┌─────────────────────────────────────────────────────────────────────────┐
│  Фаза 2: Компиляция пользовательского проекта YaoXiang (рендеринг здесь)│
│                                                                           │
│  При вызове макроса error!:                                               │
│  1. Чтение yaoxiang.toml для получения языковых предпочтений             │
│  2. Загрузка JSON i18n соответствующего языка из бинарника компилятора   │
│  3. Шаблон + параметры → render() → "Unknown variable: 'x'"             │
│  4. Diagnostic.message = отрендеренная строка                            │
│                                                                           │
│  AOT-бинарник напрямую хранит итоговые строки, без шаблонов, без поиска  │
└─────────────────────────────────────────────────────────────────────────┘

┌─────────────────────────────────────────────────────────────────────────┐
│  Фаза 3: Выполнение пользовательской программы                          │
│                                                                           │
│  println!("{}", diagnostic.message)                                      │
│  // Прямой вывод итоговой строки, без какого-либо поиска                 │
└─────────────────────────────────────────────────────────────────────────┘
```

| Компонент                    | Обязанность                                    | Момент рендеринга                         |
| ---------------------------- | ---------------------------------------------- | ----------------------------------------- |
| `I18nRegistry`               | Предоставление шаблонов и отображаемого текста | При компиляции пользовательского проекта  |
| `DiagnosticBuilder.render()` | Шаблон + параметры → итоговая строка           | При компиляции пользовательского проекта  |
| `Diagnostic.message`         | Отрендеренная строка                           | Хранит конечный результат                 |
| AOT-бинарник                 | Содержит итоговые строки                       | Используется напрямую во время выполнения |

---

### Формат сообщения об ошибке

Сообщения об ошибках используют следующий формат:

```
error[E####]: <краткое описание>
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

Уровни серьёзности ошибок управляются перечислением `DiagnosticLevel` и не зависят от нумерации
кодов ошибок:

```rust
pub enum DiagnosticLevel {
    Error,    // 导致编译失败
    Warning,  // 不影响编译，但建议修复
    Note,     // 补充信息
    Help,     // 修复建议
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
# Английский по умолчанию
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

# Вывод JSON (интеграция с LSP)
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

1. Сохранять соответствие старых кодов ошибок новым
2. В период миграции одновременно отображать как старые, так и новые коды
3. Предоставить график устаревания

---

## Стратегия реализации

### Фаза первая: Базовая инфраструктура кодов ошибок

1. Создать структуру каталога `src/diagnostics/`
2. Реализовать перечисление `ErrorCode`
3. Реализовать `Diagnostic` и `DiagnosticLevel`
4. Создать каталог файлов ресурсов и примеры JSON

### Фаза вторая: Команда explain

1. Реализовать CLI-команду `yaoxiang explain`
2. Поддержка опций `--lang` и `--json`
3. Интеграция загрузки файлов ресурсов
4. Реализация рендеринга шаблонов параметров

### Фаза третья: Интеграция на этапе компиляции

1. Обновить все точки сообщения об ошибках для использования новой системы
2. Реализовать инъекцию параметров шаблона сообщения
3. Добавить логику приоритета языка
4. Покрытие модульными тестами

### Фаза четвёртая: Интеграция с IDE/LSP

1. LSP-сервер интегрирует JSON-вывод explain
2. Отображение ссылок на коды ошибок в IDE
3. Отображение объяснения ошибки при наведении
4. Предложения по быстрому исправлению

---

## Приложение

### Полная справочная таблица кодов ошибок

| Диапазон | Категория                              |
| -------- | -------------------------------------- |
| E0xxx    | Лексический и синтаксический анализ    |
| E1xxx    | Проверка типов                         |
| E2xxx    | Семантический анализ                   |
| E3xxx    | Генерация кода                         |
| E4xxx    | Обобщения и трейты                     |
| E5xxx    | Модули и импорт                        |
| E6xxx    | Ошибки времени выполнения              |
| E7xxx    | Ошибки ввода-вывода и системные ошибки |
| E8xxx    | Внутренние ошибки компилятора          |
| E9xxx    | Зарезервировано                        |

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

- [Rust compiler errors index](https://doc.rust-lang.org/error_codes/error-index.html)
- [GCC error message format](https://gcc.gnu.org/onlinedocs/gcc-13.1.0/gcc/Warning-Options.html)
- [Clang diagnostics format](https://clang.llvm.org/diagnostics.html)
- [Language Server Protocol](https://microsoft.github.io/language-server-protocol/)
