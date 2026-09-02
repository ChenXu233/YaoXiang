---
title: 'RFC 013: Спецификация кодов ошибок'
status: 'Принят'
author: 'Чэньсюй'
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

Настоящий RFC предлагает спецификацию классификации кодов ошибок для компилятора YaoXiang,
использующую одноуровневую систему нумерации по аналогии с Rust, в сочетании с файлами ресурсов JSON
для поддержки нескольких языков, а также предоставляющую функциональность объяснения ошибок через
команду `yaoxiang explain`.

## Мотивация

### Зачем нужна стандартизированная система кодов ошибок?

1. **Опыт пользователя**: Пользователи могут быстро определить тип и серьёзность ошибки по её коду.
2. **Организация документации**: Группировка по категориям упрощает написание и поддержку справочной
   документации по ошибкам.
3. **Интеграция с инструментами**: IDE/LSP могут предлагать быстрые исправления и ссылки на
   документацию на основе кодов ошибок.
4. **Поддержка интернационализации**: Разделение сообщений об ошибках и кодов упрощает перевод на
   несколько языков.

### Цели проектирования

- **Простота**: Одноуровневая нумерация, пользователям не нужно запоминать сложные правила
  классификации.
- **Дружелюбность**: Формат сообщений об ошибках, аналогичный Rust, со справочной информацией и
  примерами.
- **Расширяемость**: Управление через файлы ресурсов, лёгкость добавления новых ошибок и языков.
- **Удобство для инструментов**: Команда explain + вывод в формате JSON для интеграции с IDE/LSP.

---

## Предложение

### Основная конструкция: одноуровневая система нумерации

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
| **4** | E4xxx    | Дженерики и trait                      |
| **5** | E5xxx    | Модули и импорт                        |
| **6** | E6xxx    | Ошибки выполнения                      |
| **7** | E7xxx    | Ошибки ввода-вывода и системные ошибки |
| **8** | E8xxx    | Внутренние ошибки компилятора          |
| **9** | E9xxx    | Зарезервировано/экспериментально       |

### Перечисление категорий ошибок

```rust
/// Категория ошибки
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ErrorCategory {
    Lexer,      // E0xxx: лексический и синтаксический анализ
    Parser,     // E0xxx: ошибки парсера
    TypeCheck,  // E1xxx: проверка типов
    Semantic,   // E2xxx: семантический анализ
    Generic,    // E4xxx: дженерики и trait
    Module,     // E5xxx: модули и импорт
    Runtime,    // E6xxx: ошибки выполнения
    Io,         // E7xxx: ошибки ввода-вывода и системные ошибки
    Internal,   // E8xxx: внутренние ошибки компилятора
}
```

### Определение кодов ошибок и универсальный Builder

**Ключевой принцип**: Разделение определения кода ошибки и отображаемого текста.

- `ErrorCodeDefinition`: Метаданные кода ошибки (code, category, template), без отображаемого
  текста.
- `locales/*.json`: Отображаемый текст для каждого языка (title, message, help, коды ошибок в виде
  вложенных объектов).
- `DiagnosticBuilder`: Универсальный построитель, заменяющий конструкцию «один trait на ошибку».

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
    pub message_template: &'static str,  // шаблон сообщения с поддержкой {param}
}

/// Универсальный построитель диагностических сообщений
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

    /// Добавление параметра шаблона
    pub fn param(mut self, key: &'static str, value: impl Into<String>) -> Self {
        self.params.push((key, value.into()));
        self
    }

    /// Установка позиции
    pub fn at(mut self, span: Span) -> Self {
        self.span = Some(span);
        self
    }

    /// Построение Diagnostic (рендеринг шаблона выполняется во время компиляции)
    pub fn build(&self, i18n: &I18nRegistry) -> Diagnostic {
        // Проверка, что для всех {key} в шаблоне заданы соответствующие параметры
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

#### Вспомогательные методы для каждого кода ошибки

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

#### Пример определения кодов ошибок

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

#### Преимущества конструкции

| Свойство                                 | Описание                                                         |
| ---------------------------------------- | ---------------------------------------------------------------- |
| **Единый Builder**                       | Один `DiagnosticBuilder` подходит для всех кодов ошибок          |
| **Безопасность типов**                   | Вспомогательные методы гарантируют корректность параметров       |
| **Самодокументируемость**                | `E1001::unknown_variable(name)` сразу понятно                    |
| **Разделение шаблонов**                  | Шаблоны сообщений отделены от кода, что упрощает i18n            |
| **Нулевые накладные расходы в рантайме** | Рендеринг во время компиляции, в AOT-бинарнике нет таблиц поиска |

---

### Упрощение макросов ошибок

#### Макрос `error!` (автоматическая подстановка контекста)

```rust
/// Макрос, автоматически получающий span и конфигурацию i18n во время компиляции
macro_rules! error {
    ($code:ident, $($key:ident = $value:expr),* $(,)?) => {
        $code()
            $(.$key($value))*
            .at(crate::util::span::Span::current())
            .build(crate::util::diagnostic::I18nRegistry::current())
    };
}

/// Использование: достаточно передать параметры, span и i18n подставляются автоматически
return Err(error!(E1001, name = var_name));
return Err(error!(E1002, expected = "bool", found = cond_ty));
```

#### Ручное использование Builder

```rust
// При необходимости ручного управления
E1001::unknown_variable(&var_name)
    .at(my_span)           // пользовательский span
    .build(&custom_i18n)   // пользовательский i18n
```

---

## Детальное проектирование

### Список кодов ошибок

#### E0xxx: Лексический и синтаксический анализ

| Код   | Тип ошибки                | Описание                                               |
| ----- | ------------------------- | ------------------------------------------------------ |
| E0001 | Invalid character         | Исходный код содержит недопустимый символ              |
| E0002 | Invalid number literal    | Некорректный формат числового литерала                 |
| E0003 | Unterminated string       | Многострочная строка без закрывающей кавычки           |
| E0004 | Invalid character literal | Некорректный символьный литерал                        |
| E0010 | Expected token            | Ожидался определённый токен при синтаксическом анализе |
| E0011 | Unexpected token          | Обнаружен неожиданный токен                            |
| E0012 | Invalid syntax            | Синтаксическая ошибка в выражении/инструкции           |
| E0013 | Mismatched brackets       | Несоответствие круглых, квадратных или фигурных скобок |
| E0014 | Missing semicolon         | В конце инструкции отсутствует точка с запятой         |
| E0016 | Expected expression       | Ожидалось выражение                                    |
| E0018 | Keyword as name           | Ключевое слово не может использоваться как имя         |

#### E1xxx: Проверка типов

| Код   | Тип ошибки                                             | Описание                                                          |
| ----- | ------------------------------------------------------ | ----------------------------------------------------------------- |
| E1001 | Unknown variable                                       | Ссылка на неопределённую переменную                               |
| E1002 | Type mismatch                                          | Ожидаемый тип не соответствует фактическому                       |
| E1003 | Unknown type                                           | Ссылка на несуществующий тип                                      |
| E1010 | Parameter count mismatch                               | Количество аргументов вызова функции не соответствует определению |
| E1011 | Parameter type mismatch                                | Неудачная проверка типов аргументов                               |
| E1012 | Return type mismatch                                   | Некорректный тип возвращаемого значения функции                   |
| E1013 | Function not found                                     | Вызов неопределённой функции                                      |
| E1020 | Cannot infer type                                      | Невозможно вывести тип из контекста                               |
| E1021 | Type inference conflict                                | Противоречие типов из-за нескольких ограничений                   |
| E1030 | Pattern non-exhaustive                                 | Выражение match не покрывает все случаи                           |
| E1031 | Unreachable pattern                                    | Шаблон, который никогда не может совпасть                         |
| E1040 | Operation not supported                                | Тип не поддерживает данную операцию                               |
| E1041 | Index out of bounds                                    | Индекс массива/списка вне допустимого диапазона                   |
| E1042 | Field not found                                        | Обращение к несуществующему полю структуры                        |
| E1050 | Boolean operand required                               | Требуется булев операнд                                           |
| E1051 | Logical NOT requires boolean operand                   | Логическое NOT требует булев операнд                              |
| E1052 | Invalid dereference                                    | Недопустимое разыменование                                        |
| E1053 | Non-struct field access                                | Обращение к полю не-структуры                                     |
| E1054 | Conditional type mismatch                              | Несоответствие условного типа                                     |
| E1055 | Constraint in non-generic context                      | Ограничение в не-generic контексте                                |
| E1060 | Type parameter count mismatch                          | Несоответствие количества типовых параметров                      |
| E1061 | Cannot instantiate generic                             | Невозможно инстанцировать дженерик                                |
| E1062 | Const generic constraint failed                        | Ошибка ограничения const-дженерика                                |
| E1064 | Invalid binding position                               | Недопустимая позиция связывания (RFC-004)                         |
| E1071 | Type definitions are only allowed at module level      | Определения типов допускаются только на уровне модуля             |
| E1081 | `?` can only be used within functions returning Result | `?` допускается только в функциях, возвращающих Result            |
| E1082 | `?` can only be used with Result expressions           | `?` может использоваться только с выражениями Result              |
| E1083 | Error type mismatch for `?`                            | Несоответствие типа ошибки для `?`                                |
| E1090 | Type universe easter egg                               | Type: Type = Type пасхалка (уровень Note)                         |
| E1091 | Invalid generic meta type                              | Недопустимый мета-тип дженерика                                   |
| E1092 | Invalid refinement type argument form                  | Недопустимая форма аргумента уточняющего типа                     |
| E1093 | Refinement argument count mismatch                     | Несоответствие количества уточняющих аргументов                   |
| E1094 | Unused compile-time value parameter                    | Неиспользуемый параметр значения времени компиляции               |
| E1095 | Unknown interface                                      | Неизвестный интерфейс                                             |
| E1096 | Interface arity mismatch                               | Несоответствие арности интерфейса                                 |
| E1097 | Interface member name conflict                         | Конфликт имён членов интерфейса                                   |
| E1098 | Interface method not implemented                       | Метод интерфейса не реализован                                    |
| E1099 | Interface method signature mismatch                    | Несоответствие сигнатуры метода интерфейса                        |
| E1100 | Duplicate interface method implementation              | Дублирование реализации метода интерфейса                         |
| E1101 | Type does not implement interface                      | Тип не реализует интерфейс                                        |
| E1102 | Loop control statement outside of a loop               | Управляющая конструкция цикла вне цикла                           |

#### E2xxx: Семантический анализ

| Код   | Тип ошибки                        | Описание                                                 |
| ----- | --------------------------------- | -------------------------------------------------------- |
| E2001 | Scope error                       | Переменная не в текущей области видимости                |
| E2002 | Duplicate definition              | Повторное определение в одной области видимости          |
| E2003 | Lifetime error                    | Ограничения времени жизни не выполнены                   |
| E2010 | Immutable assignment              | Попытка изменить неизменяемую переменную                 |
| E2011 | Uninitialized use                 | Использование неинициализированной переменной            |
| E2012 | Mutability conflict               | Использование изменяемой ссылки в неизменяемом контексте |
| E2013 | Variable shadowing                | Затенение переменной                                     |
| E2014 | Use of moved value                | Использование перемещённого значения                     |
| E2016 | Immutable assignment              | Неизменяемое присваивание                                |
| E2018 | Mutable/immutable borrow conflict | Конфликт изменяемого/неизменяемого заимствования         |
| E2019 | Double free                       | Двойное освобождение                                     |
| E2020 | Use after free                    | Использование после освобождения                         |
| E2027 | Unsafe dereference                | Разыменование unsafe                                     |
| E2090 | Invalid signature                 | Ошибка разбора сигнатуры функции                         |
| E2091 | Unknown type in signature         | Неизвестный тип в сигнатуре                              |
| E2092 | Missing arrow in signature        | В сигнатуре отсутствует стрелка возврата                 |
| E2093 | Duplicate parameter name          | Дублирование имени параметра                             |
| E2094 | Generic parameter shadowing       | Затенение параметра дженерика                            |
| E2095 | Parameter name shadows generic    | Имя параметра затеняет дженерик                          |

#### E3xxx: Генерация кода

| Код   | Тип ошибки                             | Описание                                             |
| ----- | -------------------------------------- | ---------------------------------------------------- |
| E3004 | Unsupported iterator                   | Неподдерживаемый итератор                            |
| E3005 | IR generation error                    | Внутренняя ошибка генерации IR                       |
| E3006 | Unresolved variable                    | Переменная не разрешена на этапе генерации IR        |
| E3007 | Top-level initializer must be constant | Инициализатор верхнего уровня должен быть константой |
| E3014 | Register overflow                      | Переполнение регистров                               |
| E3017 | Invalid operand (code generation)      | Недопустимый операнд (генерация кода)                |

#### E4xxx: Дженерики и trait

| Код   | Тип ошибки                              | Описание                                                      |
| ----- | --------------------------------------- | ------------------------------------------------------------- |
| E4001 | Generic parameter mismatch              | Несоответствие количества/типа параметров дженерика           |
| E4002 | Trait bound violated                    | Нарушено ограничение trait                                    |
| E4003 | Associated type error                   | Ошибка определения/использования ассоциированного типа        |
| E4004 | Duplicate trait implementation          | Дублирование реализации одного trait                          |
| E4005 | Trait not found                         | Требуемый trait не найден                                     |
| E4006 | Sized bound violated                    | Нарушение ограничения Sized (зарезервировано, не реализовано) |
| E4010 | Division by zero in constant expression | Деление на ноль в константном выражении                       |
| E4011 | Constant overflow                       | Переполнение константы                                        |
| E4012 | Constant recursion too deep             | Слишком глубокая рекурсия в константе                         |
| E4014 | Constant evaluation failed              | Ошибка вычисления константы                                   |
| E4018 | Refinement predicate violation          | Нарушение уточняющего предиката                               |
| E4019 | Type equality does not hold             | Равенство типов не выполняется                                |
| E4020 | Proof function required                 | Требуется функция-доказательство для проверки ограничения     |

#### E5xxx: Модули и импорт

| Код   | Тип ошибки            | Описание                                                        |
| ----- | --------------------- | --------------------------------------------------------------- |
| E5001 | Module not found      | Импортируемый модуль не существует                              |
| E5002 | Cyclic import         | Циклическая зависимость между модулями                          |
| E5003 | Symbol not exported   | Попытка доступа к неэкспортируемому символу                     |
| E5004 | Invalid module path   | Некорректный формат пути модуля                                 |
| E5005 | Private access        | Доступ к приватному символу                                     |
| E5006 | Duplicate import      | Дублирование импорта                                            |
| E5007 | Module export listing | Список экспорта модуля (сопутствующее информационное сообщение) |

#### E6xxx: Ошибки выполнения

| Код   | Тип ошибки                  | Описание                                                         |
| ----- | --------------------------- | ---------------------------------------------------------------- |
| E6001 | Division by zero            | Целочисленное деление на ноль                                    |
| E6002 | ~~Assertion failed~~        | ~~Зарезервировано (отсутствует языковая концепция, удалено)~~    |
| E6003 | Runtime index out of bounds | Выход индекса за границы во время выполнения (подключение #280)  |
| E6004 | Stack overflow              | Исчерпание стека                                                 |
| E6005 | Assertion failed            | Сбой assert (подключение #280)                                   |
| E6006 | Function not found          | Функция не найдена во время выполнения                           |
| E6007 | Runtime error (generic)     | Универсальная ошибка выполнения                                  |
| E6008 | Key not found               | Отсутствие ключа в Dict (#299 §4)                                |
| E6009 | Invalid range step          | Недопустимый шаг Range (step=0, Result-фикация std.range #316)   |
| E6010 | Integer parse failed        | Ошибка разбора целого числа (std.string.parse_int)               |
| E6011 | Float parse failed          | Ошибка разбора числа с плавающей точкой (std.string.parse_float) |

> **Редакция #280 (2026-08-09)**: Исходная таблица кодов была определена по семантическому черновику
> в стиле Rust (Assertion failed/Arithmetic overflow/Heap allocation failed/Type cast failed), что
> не соответствовало фактическим потребностям реализации. YaoXiang не имеет концепций нулевого
> указателя/сбоя выделения кучи/преобразования типов (семантика значений + безопасность памяти в
> стиле Rust), а обнаружение переполнения в рантайме не реализовано. После корректировки:
>
> - E6002 удалён (исходный Assertion failed перенесён в E6005; исходная семантика нулевого указателя
>   не имеет языковой концепции)
> - E6003 изменён с Arithmetic overflow на Runtime index out of bounds (реальная поверхность
>   срабатывания, #279/#271)
> - E6005 изменён с Heap allocation failed на Assertion failed (реальный путь std.assert)
> - E6006 изменён с Runtime index out of bounds на Function not found (реализация уже была такой,
>   #255)
> - E6007 изменён с Type cast failed на универсальный Runtime error (унифицированная точка сбора для
>   несопоставленных вариантов ExecutorError)

#### E7xxx: Ошибки ввода-вывода и системные ошибки

| Код   | Тип ошибки        | Описание                             |
| ----- | ----------------- | ------------------------------------ |
| E7001 | File not found    | Попытка чтения несуществующего файла |
| E7002 | Permission denied | Недостаточно прав доступа к файлу    |
| E7003 | I/O error         | Универсальная ошибка ввода-вывода    |
| E7004 | Network error     | Сбой сетевой операции                |

#### E8xxx: Внутренние ошибки компилятора

| Код   | Тип ошибки              | Описание                                                         |
| ----- | ----------------------- | ---------------------------------------------------------------- |
| E8001 | Internal compiler error | Внутренняя ошибка компилятора                                    |
| E8002 | Codegen error           | Сбой генерации IR/байткода                                       |
| E8003 | Unimplemented feature   | Использование нереализованной функциональности                   |
| E8004 | Optimization error      | Ошибка оптимизации компилятора (зарезервировано, не реализовано) |

#### W1xxx: Коды предупреждений

| Код   | Тип предупреждения                           | Описание                                                                      |
| ----- | -------------------------------------------- | ----------------------------------------------------------------------------- |
| W1001 | Unused exported function                     | Неиспользуемая экспортируемая функция                                         |
| W1002 | Unused exported type                         | Неиспользуемый экспортируемый тип                                             |
| W1003 | Unused import                                | Неиспользуемый импорт                                                         |
| W1004 | Unused exported variable                     | Неиспользуемая экспортируемая переменная                                      |
| W1005 | Unused exported method                       | Неиспользуемый экспортируемый метод                                           |
| W1063 | Const generic constraint cannot be evaluated | Ограничение const-дженерика не может быть вычислено                           |
| W1080 | Constraint demoted to runtime check          | Ограничение не доказуемо во время компиляции, понижено до проверки в рантайме |

> Правило нумерации W-кодов: изоморфно E-кодам с группировкой по фазам (W + сегмент тысяч фазы),
> W1xxx = предупреждения фазы проверки типов.
>
> **Канал вывода (#321 M2)**: Диагностика W-кодов по умолчанию помечается `Severity::Warning`
> билдером (явное указание имеет приоритет), сбор и отображение идут по тому же пути, что и ошибки
> (рендеринг с префиксом `warning[W####]`), но не прерывают компиляцию и не влияют на код успешного
> завершения. `yaoxiang check --deny-warnings` повышает предупреждения до ошибок (ненулевой код
> выхода при наличии предупреждений) для строгого режима CI. Подавление по коду (атрибуты allow и
> т.п.) — пункт для будущего расширения.

### Спецификация качества сообщений

> Этот раздел введён в #322 (M3 Единый путь сообщений и качество, 2026-09-03). Принудительно
> проверяется в CI скриптом `scripts/audit_diagnostics.py`.

1. **Единый путь сообщений**: Все видимые пользователю диагностические сообщения должны проходить
   через авторитетные вспомогательные методы реестра и рендеринг шаблонов locales, код передаёт
   только структурированные параметры. Запрещено обходить реестр и напрямую конструировать сырые
   значения вроде `Diagnostic::error(...)` — этот путь обходит проверку кодов и i18n.
2. **Легитимность кодов**: Запрещено использовать незарегистрированные коды и псевдокоды (например,
   `E_INTERNAL`); литералы используемых кодов должны быть определены в реестре. Внутренние ошибки
   всегда попадают на E8001 (`internal_error`).
3. **Отображение типов**: Отображение типа должно различать форму до и после инстанцирования (#286:
   `Expected 'Container', found 'Container'` — голые имена неразличимы).
4. **Изоляция внутреннего состояния солвера**: Промежуточные TypeVar солвера (форма отображения
   `t<N>`) не должны попадать в видимые пользователю сообщения (#287). Тестовый якорь:
   `test_type_error_message_no_solver_typevar_leak`.
5. **Граница E8xxx**: E8xxx предназначены только для проблем внутренней согласованности компилятора
   (ICE). Исправляемые пользователем ошибки запрещено затыкать через E8001; сообщения ICE должны
   содержать инструкции по минимальному воспроизведению.

---

### Значения ошибок рантайма и сквозная связь с кодами

> Этот раздел введён в #323 (M4 Значения ошибок рантайма с кодами, 2026-09-03). Семантическое
> пространство E6xxx/E7xxx одновременно обслуживает два канала, пространство кодов едино, каналы
> отображения различаются.

#### Два канала

| Канал                                  | Носитель                                                          | Способ отображения                                                  |
| -------------------------------------- | ----------------------------------------------------------------- | ------------------------------------------------------------------- |
| Канал диагностики компилятора/CLI      | Жёсткие ошибки уровня хоста, например `ExecutorError`             | stderr `error[E####]:` (#280/#281 уже подключены E6003/E6005/E6007) |
| Канал значений ошибок внутри программы | Носитель Err `Error` из `Result(T, Error)` стандартной библиотеки | Языковое значение, потребляемое программой через match/сравнение    |

#### Структура Error (начиная с v0.8, ломающее изменение)

```
Error { code: String, message: String }
```

- `code` повторно использует нумерацию E6xxx/E7xxx из данной спецификации, в строковом виде
  (например, `"E6008"`).
- **Стабильный контракт**: Семантика выделенных кодов не изменяется между версиями; одна и та же
  семантика не переиспользует удалённые коды (прецедент E6002).
- **Поверхность потребления**: Программное сравнение `e.code == "E6xxx"` — единственный
  программируемый контракт; документация сквозная через `yaoxiang explain E6xxx`; инструментарий
  (LSP / DAP, см. RFC-034) использует код как exceptionId.
- **Аксессоры**: `std.result.code(e)` / `std.result.message(e)`.
- **Пользовательские ошибки**: E в `Result(T, E)` — это параметр дженерика, для серьёзного
  моделирования используются пользовательские типы; `Error` из std — лишь удобный запасной носитель,
  его система кодов не ограничивает пользовательский тип E.

#### Правила выделения кодов

1. Коды значений ошибок рантайма и диагностические коды компилятора совместно используют
   пространство E6xxx/E7xxx, новые коды выделяются по **реальной поверхности срабатывания**, без
   резервирования под гипотетические сценарии.
2. Сначала регистрация, затем использование: Новый код попадает в авторитетный реестр и проходит
   трёхстороннюю проверку согласованности (codes/*.rs ↔ locales ↔ таблица кодов в данном документе)
   перед выпуском. Источником регистрации кодов значений ошибок рантайма является таблица
   `RUNTIME_ERROR_CODES` в `src/std/result.rs` (проверяется тем же `scripts/check_error_codes.py`,
   что и диагностические коды).
3. E7xxx зарезервирован под значения ошибок std.io / std.net (в настоящее время пуст, будет
   задействован при Result-фикации io/net).
4. Точки выпуска (#323 M4): Различные модули std создают значения Error через
   `error_new(code, message)`; на стороне потребления `std.result.unwrap_err` извлекает носитель
   Err, `std.result.code/message` читают поля.

#### Путь развития (линия C, не реализовано)

После завершения усовершенствования сопоставления с образцом (RFC-039) `Error` может быть обновлён
до `{ kind: ErrorKind, message: String }`, где `code` становится атрибутом, производным от kind
(определение варианта — это и есть реестр кодов). Во время переходного периода стабильный контракт
code данного раздела остаётся неизменным; это обновление — отдельное решение, не является
обязательством данного раздела.

---

### Файлы ресурсов для нескольких языков

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

/// Реестр отображаемого текста i18n (загружается во время компиляции из JSON, нулевой поиск в рантайме)
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
    /// Получение реестра по коду языка
    pub fn new(lang: &str) -> Self {
        match lang {
            "zh" => Self::zh(),
            _ => Self::en(),
        }
    }

    /// Получение информации об ошибке
    pub fn get_info(&self, code: &str) -> Option<ErrorInfo<'_>> {
        Some(ErrorInfo {
            title: self.titles.get(code)?,
            message: self.messages.get(code)?,
            help: self.helps.get(code)?,
            example: self.examples.get(code).copied(),
            error_output: self.error_outputs.get(code).copied(),
        })
    }

    /// Рендеринг шаблона (выполняется во время компиляции, нулевые накладные расходы в рантайме)
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

| Плейсхолдер  | Назначение                                     | Пример                              |
| ------------ | ---------------------------------------------- | ----------------------------------- |
| `{name}`     | Имя переменной/типа/trait и т.п. идентификатор | `Unknown variable: '{name}'`        |
| `{expected}` | Ожидаемый тип                                  | `Expected type '{expected}'`        |
| `{found}`    | Фактический/найденный тип                      | `, found type '{found}'`            |
| `{method}`   | Имя метода                                     | `Method {method} is not a function` |
| `{trait}`    | Имя trait                                      | `Cannot find trait: {trait}`        |
| `{path}`     | Путь модуля                                    | `Invalid path: {path}`              |
| `{ty}`       | Выражение типа                                 | `Invalid type: {ty}`                |
| `{message}`  | Сообщение внутренней ошибки                    | `Internal error: {message}`         |

##### Поддержка произвольных ключей

**params поддерживает произвольные ключи, не ограничиваясь предопределёнными**. Вызывающая сторона
может передавать любой `key`:

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

> **Примечание**: Не все коды ошибок используют плейсхолдеры. Некоторые коды ошибок (например,
> E0001) имеют статические сообщения и не требуют параметров.

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
# Язык сообщений об ошибках, доступные значения: en, zh, ja, ...
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
2. Если не настроено, чтение из пользовательского ~/.yaoxiang/yaoxiang.toml
3. Если ни одно не настроено, по умолчанию используется "en"
4. Компилятор создаёт I18nRegistry для выбранного языка (однократно)
5. Все ошибки рендерятся с использованием этого I18nRegistry
```

#### Ключ к нулевым накладным расходам на поиск

**Рендеринг происходит при компиляции пользовательского проекта, а не во время выполнения.**

```
┌─────────────────────────────────────────────────────────────────────────┐
│  Фаза 1: Rust компилирует компилятор YaoXiang                            │
│                                                                           │
│  JSON упаковывается в бинарник компилятора                               │
│  Цель: команда explain может напрямую читать данные i18n                  │
└─────────────────────────────────────────────────────────────────────────┘

┌─────────────────────────────────────────────────────────────────────────┐
│  Фаза 2: YaoXiang компилирует пользовательский проект (здесь рендеринг) │
│                                                                           │
│  При вызове макроса error!:                                              │
│  1. Чтение yaoxiang.toml для получения языковых предпочтений             │
│  2. Загрузка JSON i18n для соответствующего языка из бинарника компилятора│
│  3. Шаблон + параметры → render() → "Unknown variable: 'x'"            │
│  4. Diagnostic.message = отрендеренная строка                             │
│                                                                           │
│  AOT-бинарник напрямую хранит финальные строки, без шаблонов, без поиска │
└─────────────────────────────────────────────────────────────────────────┘

┌─────────────────────────────────────────────────────────────────────────┐
│  Фаза 3: Выполнение пользовательской программы                          │
│                                                                           │
│  println!("{}", diagnostic.message)                                      │
│  // Прямой вывод финальной строки, без какого-либо поиска                │
└─────────────────────────────────────────────────────────────────────────┘
```

| Компонент                    | Обязанность                                    | Момент рендеринга                        |
| ---------------------------- | ---------------------------------------------- | ---------------------------------------- |
| `I18nRegistry`               | Предоставление шаблонов и отображаемого текста | При компиляции пользовательского проекта |
| `DiagnosticBuilder.render()` | Шаблон + параметры → финальная строка          | При компиляции пользовательского проекта |
| `Diagnostic.message`         | Отрендеренная строка                           | Хранение финального результата           |
| AOT-бинарник                 | Содержит финальные строки                      | Прямое использование в рантайме          |

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
error[E1001]: Unknown variable: x
  --> src/main.yx:5:12
   5 |   print(x)
          ^
          help: Did you mean to define it?
```

---

### Уровни серьёзности

Серьёзность ошибок управляется перечислением `DiagnosticLevel`, декапленным от нумерации кода
ошибки:

```rust
pub enum DiagnosticLevel {
    Error,    // приводит к сбою компиляции
    Warning,  // не влияет на компиляцию, но рекомендуется исправить
    Note,     // дополнительная информация
    Help,     // предложение по исправлению
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
yaoxiang explain <КОД_ОШИБКИ> [ОПЦИИ]
```

#### Опции

| Опция           | Описание                                        |
| --------------- | ----------------------------------------------- |
| `--lang <код>`  | Указать язык (en-US, zh-CN, по умолчанию en-US) |
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
  "examples": ["let {name} = value;"],
  "language": "en-US"
}
```

---

### Обратная совместимость

Поскольку настоящий RFC проектирует систему кодов ошибок с нуля, проблемы обратной совместимости
отсутствуют.

**Стратегия будущей миграции** (для справки в последующих версиях):

1. Сохранение соответствия между старыми и новыми кодами ошибок
2. Одновременное отображение старых и новых кодов в период миграции
3. Предоставление графика устаревания

---

## Стратегия реализации

### Фаза первая: Базовая инфраструктура кодов ошибок

1. Создание структуры каталога `src/diagnostics/`
2. Реализация перечисления `ErrorCode`
3. Реализация `Diagnostic` и `DiagnosticLevel`
4. Создание каталога файлов ресурсов и примеров JSON

### Фаза вторая: Команда explain

1. Реализация CLI-команды `yaoxiang explain`
2. Поддержка опций `--lang` и `--json`
3. Интеграция загрузки файлов ресурсов
4. Реализация рендеринга шаблонов с параметрами

### Фаза третья: Интеграция на этапе компиляции

1. Обновление всех точек вывода ошибок для использования новой системы
2. Реализация инъекции параметров шаблона сообщения
3. Добавление логики приоритета языка
4. Покрытие модульными тестами

### Фаза четвёртая: Интеграция с IDE/LSP

1. Интеграция вывода JSON команды explain в сервер LSP
2. Отображение ссылок на коды ошибок в IDE
3. Показ объяснения ошибки при наведении
4. Предложения быстрых исправлений

---

## Приложение

### Полная сводная таблица кодов ошибок

| Диапазон | Категория                              |
| -------- | -------------------------------------- |
| E0xxx    | Лексический и синтаксический анализ    |
| E1xxx    | Проверка типов                         |
| E2xxx    | Семантический анализ                   |
| E3xxx    | Генерация кода                         |
| E4xxx    | Дженерики и trait                      |
| E5xxx    | Модули и импорт                        |
| E6xxx    | Ошибки выполнения                      |
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

- [Указатель ошибок компилятора Rust](https://doc.rust-lang.org/error_codes/error-index.html)
- [Формат сообщений об ошибках GCC](https://gcc.gnu.org/onlinedocs/gcc-13.1.0/gcc/Warning-Options.html)
- [Формат диагностики Clang](https://clang.llvm.org/diagnostics.html)
- [Language Server Protocol](https://microsoft.github.io/language-server-protocol/)
