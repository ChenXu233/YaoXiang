---
title: 'RFC 013: Спецификация кодов ошибок'
status: 'Принято'
author: 'Чэньсюй (晨煦)'
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
одноуровневую систему нумерации, аналогичную Rust, в сочетании с JSON-файлами ресурсов для поддержки
нескольких языков, с функцией объяснения ошибок через команду `yaoxiang explain`.

## Мотивация

### Зачем нужна стандартизированная система кодов ошибок?

1. **Пользовательский опыт**: Пользователи могут быстро определить тип и серьёзность ошибки по коду
   ошибки
2. **Организация документации**: Группировка по категориям упрощает написание и поддержку справочной
   документации по ошибкам
3. **Интеграция с инструментами**: IDE/LSP могут предоставлять предложения по быстрому исправлению и
   ссылки на документацию на основе кодов ошибок
4. **Поддержка интернационализации**: Разделение сообщений об ошибках и кодов облегчает перевод на
   несколько языков

### Цели проектирования

- **Простота**: Одноуровневая нумерация, пользователям не нужно запоминать сложные правила
  классификации
- **Дружелюбность**: Формат сообщений об ошибках, аналогичный Rust, с вспомогательной информацией и
  примерами
- **Расширяемость**: Управление через файлы ресурсов, простота добавления новых ошибок и языков
- **Удобство для инструментов**: Команда explain + JSON-вывод, поддержка интеграции с IDE/LSP

---

## Предложение

### Основной дизайн: Одноуровневая система нумерации

Используется четырёхзначная нумерация, сгруппированная по фазам компиляции:

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
| **4** | E4xxx    | Generics и Trait                       |
| **5** | E5xxx    | Модули и импорт                        |
| **6** | E6xxx    | Ошибки времени выполнения              |
| **7** | E7xxx    | Ошибки ввода-вывода и системные ошибки |
| **8** | E8xxx    | Внутренние ошибки компилятора          |
| **9** | E9xxx    | Зарезервировано/Экспериментально       |

### Перечисление категорий ошибок

```rust
/// Категория ошибки
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ErrorCategory {
    Lexer,      // E0xxx: Лексический и синтаксический анализ
    Parser,     // E0xxx: Parser errors
    TypeCheck,  // E1xxx: Проверка типов
    Semantic,   // E2xxx: Семантический анализ
    Generic,    // E4xxx: Generics и Trait
    Module,     // E5xxx: Модули и импорт
    Runtime,    // E6xxx: Ошибки времени выполнения
    Io,         // E7xxx: Ошибки ввода-вывода и системные
    Internal,   // E8xxx: Внутренние ошибки компилятора
}
```

### Определение кода ошибки и универсальный Builder

**Ключевой принцип**: Разделение определения кода ошибки и отображаемого текста

- `ErrorCodeDefinition`: Метаданные кода ошибки (code, category, template), без отображаемого текста
- `locales/*.json`: Отображаемый текст для каждого языка (title, message, help, коды ошибок как
  вложенные объекты)
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

    /// Собрать Diagnostic (рендеринг шаблона завершается в compile-time)
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

| Свойство                                | Описание                                                   |
| --------------------------------------- | ---------------------------------------------------------- |
| **Единый Builder**                      | Один `DiagnosticBuilder` универсален для всех кодов ошибок |
| **Безопасность типов**                  | Удобные методы обеспечивают корректность параметров        |
| **Самодокументируемость**               | `E1001::unknown_variable(name)` говорит сам за себя        |
| **Разделение шаблонов**                 | Шаблоны сообщений отделены от кода, легко для i18n         |
| **Нулевые накладные расходы в runtime** | Рендеринг в compile-time, AOT-бинарник без таблиц поиска   |

---

### Упрощение макросов ошибок

#### Макрос error! (автоматическая инъекция контекста)

```rust
/// Макрос для автоматического получения span и конфигурации i18n в compile-time
macro_rules! error {
    ($code:ident, $($key:ident = $value:expr),* $(,)?) => {
        $code()
            $(.$key($value))*
            .at(crate::util::span::Span::current())
            .build(crate::util::diagnostic::I18nRegistry::current())
    };
};

/// Использование: нужно передать только параметры, span и i18n инжектируются автоматически
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

| Код   | Тип ошибки                | Описание                                               |
| ----- | ------------------------- | ------------------------------------------------------ |
| E0001 | Invalid character         | Исходный код содержит недопустимые символы             |
| E0002 | Invalid number literal    | Неверный формат числового литерала                     |
| E0003 | Unterminated string       | Многострочная строка без закрывающей кавычки           |
| E0004 | Invalid character literal | Некорректный символьный литерал                        |
| E0010 | Expected token            | Ожидался определённый токен при синтаксическом анализе |
| E0011 | Unexpected token          | Обнаружен неожиданный токен                            |
| E0012 | Invalid syntax            | Синтаксическая ошибка выражения/инструкции             |
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
| E1010 | Parameter count mismatch                               | Количество аргументов вызова не соответствует определению функции |
| E1011 | Parameter type mismatch                                | Проверка типа аргумента не прошла                                 |
| E1012 | Return type mismatch                                   | Неверный тип возвращаемого значения функции                       |
| E1013 | Function not found                                     | Вызов неопределённой функции                                      |
| E1020 | Cannot infer type                                      | Контекст не позволяет вывести тип                                 |
| E1021 | Type inference conflict                                | Множественные ограничения приводят к противоречию типов           |
| E1030 | Pattern non-exhaustive                                 | Выражение match не покрывает все случаи                           |
| E1031 | Unreachable pattern                                    | Шаблон, который никогда не может быть сопоставлен                 |
| E1040 | Operation not supported                                | Операция не поддерживается для данного типа                       |
| E1041 | Index out of bounds                                    | Индекс массива/списка вне допустимого диапазона                   |
| E1042 | Field not found                                        | Доступ к несуществующему полю структуры                           |
| E1050 | Boolean operand required                               | Требуется булевый операнд                                         |
| E1051 | Logical NOT requires boolean operand                   | Логическое NOT требует булевого операнда                          |
| E1052 | Invalid dereference                                    | Недопустимое разыменование                                        |
| E1053 | Non-struct field access                                | Доступ к полю не у структуры                                      |
| E1054 | Conditional type mismatch                              | Несоответствие условного типа                                     |
| E1055 | Constraint in non-generic context                      | Ограничение в не-generic контексте                                |
| E1060 | Type parameter count mismatch                          | Несоответствие количества параметров типа                         |
| E1061 | Cannot instantiate generic                             | Невозможно инстанцировать дженерик                                |
| E1062 | Const generic constraint failed                        | Сбой ограничения const-дженерика                                  |
| E1064 | Invalid binding position                               | Недопустимая позиция привязки (RFC-004)                           |
| E1071 | Type definitions are only allowed at module level      | Определения типов разрешены только на уровне модуля               |
| E1081 | `?` can only be used within functions returning Result | `?` допустим только в функциях, возвращающих Result               |
| E1082 | `?` can only be used with Result expressions           | `?` может использоваться только с выражениями Result              |
| E1083 | Error type mismatch for `?`                            | Несоответствие типа ошибки для `?`                                |
| E1090 | Type universe easter egg                               | Type: Type = Type пасхалка (уровень Note)                         |
| E1091 | Invalid generic meta type                              | Недопустимый мета-тип дженерика                                   |
| E1092 | Invalid refinement type argument form                  | Недопустимая форма аргумента уточняющего типа                     |
| E1093 | Refinement argument count mismatch                     | Несоответствие количества аргументов уточнения                    |
| E1094 | Unused compile-time value parameter                    | Неиспользуемый параметр значения compile-time                     |
| E1095 | Unknown interface                                      | Неизвестный интерфейс                                             |
| E1096 | Interface arity mismatch                               | Несоответствие арности интерфейса                                 |
| E1097 | Interface member name conflict                         | Конфликт имён членов интерфейса                                   |
| E1098 | Interface method not implemented                       | Метод интерфейса не реализован                                    |
| E1099 | Interface method signature mismatch                    | Несоответствие сигнатуры метода интерфейса                        |
| E1100 | Duplicate interface method implementation              | Дублирующаяся реализация метода интерфейса                        |
| E1101 | Type does not implement interface                      | Тип не реализует интерфейс                                        |
| E1102 | Loop control statement outside of a loop               | Управляющий оператор цикла вне цикла                              |

#### E2xxx: Семантический анализ

| Код   | Тип ошибки                        | Описание                                                 |
| ----- | --------------------------------- | -------------------------------------------------------- |
| E2001 | Scope error                       | Переменная не в текущей области видимости                |
| E2002 | Duplicate definition              | Дублирующееся определение в одной области видимости      |
| E2003 | Lifetime error                    | Ограничения времени жизни не удовлетворены               |
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
| E2093 | Duplicate parameter name          | Дублирующееся имя параметра                              |
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

#### E4xxx: Generics и Trait

| Код   | Тип ошибки                              | Описание                                                             |
| ----- | --------------------------------------- | -------------------------------------------------------------------- |
| E4001 | Generic parameter mismatch              | Несоответствие количества/типа параметров дженерика                  |
| E4002 | Trait bound violated                    | Не удовлетворено ограничение trait                                   |
| E4003 | Associated type error                   | Ошибка определения/использования ассоциированного типа               |
| E4004 | Duplicate trait implementation          | Дублирующаяся реализация того же trait                               |
| E4005 | Trait not found                         | Требуемый trait не найден                                            |
| E4006 | Sized bound violated                    | Не удовлетворено ограничение Sized (зарезервировано, не реализовано) |
| E4010 | Division by zero in constant expression | Деление на ноль в константном выражении                              |
| E4011 | Constant overflow                       | Переполнение константы                                               |
| E4012 | Constant recursion too deep             | Слишком глубокая рекурсия в константе                                |
| E4014 | Constant evaluation failed              | Сбой вычисления константы                                            |
| E4018 | Refinement predicate violation          | Нарушение предиката уточнения                                        |
| E4019 | Type equality does not hold             | Равенство типов не выполняется                                       |

#### E5xxx: Модули и импорт

| Код   | Тип ошибки            | Описание                                                        |
| ----- | --------------------- | --------------------------------------------------------------- |
| E5001 | Module not found      | Импортируемый модуль не существует                              |
| E5002 | Cyclic import         | Циклическая зависимость между модулями                          |
| E5003 | Symbol not exported   | Попытка доступа к неэкспортированному символу                   |
| E5004 | Invalid module path   | Неверный формат пути модуля                                     |
| E5005 | Private access        | Доступ к приватному символу                                     |
| E5006 | Duplicate import      | Дублирующийся импорт                                            |
| E5007 | Module export listing | Список экспорта модуля (сопутствующее информационное сообщение) |

#### E6xxx: Ошибки времени выполнения

| Код   | Тип ошибки                  | Описание                                                        |
| ----- | --------------------------- | --------------------------------------------------------------- |
| E6001 | Division by zero            | Целочисленное деление на ноль                                   |
| E6002 | ~~Assertion failed~~        | ~~Зарезервировано (нет языковой концепции, удалено)~~           |
| E6003 | Runtime index out of bounds | Выход индекса за границы во время выполнения (подключение #280) |
| E6004 | Stack overflow              | Исчерпание стека                                                |
| E6005 | Assertion failed            | Сбой assert (подключение #280)                                  |
| E6006 | Function not found          | Функция не найдена во время выполнения                          |
| E6007 | Runtime error (generic)     | Универсальная ошибка времени выполнения                         |
| E6008 | Key not found               | Отсутствующий ключ в Dict (#299 §4)                             |

> **Редакция #280 (2026-08-09)**: Таблица кодов изначально была определена по черновику
> Rust-семантики (Assertion failed/Arithmetic overflow/Heap allocation failed/Type cast failed), что
> не соответствовало реальным потребностям реализации. В YaoXiang нет концепций нулевого
> указателя/сбоя выделения кучи/приведения типов (семантика значений + безопасность памяти Rust), а
> пути обнаружения переполнения в runtime не реализованы. После корректировки:
>
> - E6002 удалён (исходный Assertion failed перенесён в E6005; семантика нулевого указателя не имеет
>   языковой концепции)
> - E6003 изменён с Arithmetic overflow на Runtime index out of bounds (реальная поверхность
>   срабатывания, #279/#271)
> - E6005 изменён с Heap allocation failed на Assertion failed (реальный путь std.assert)
> - E6006 изменён с Runtime index out of bounds на Function not found (реализация уже давно такова,
>   #255)
> - E6007 изменён с Type cast failed на универсальный Runtime error (точка сбора для
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
| E8002 | Codegen error           | Сбой генерации IR/байт-кода                                      |
| E8003 | Unimplemented feature   | Использование нереализованной функции                            |
| E8004 | Optimization error      | Ошибка оптимизации компилятора (зарезервировано, не реализовано) |

#### W1xxx: Коды предупреждений

| Код   | Тип предупреждения                           | Описание                                         |
| ----- | -------------------------------------------- | ------------------------------------------------ |
| W1001 | Unused exported function                     | Неиспользуемая экспортированная функция          |
| W1002 | Unused exported type                         | Неиспользуемый экспортированный тип              |
| W1003 | Unused import                                | Неиспользуемый импорт                            |
| W1004 | Unused exported variable                     | Неиспользуемая экспортированная переменная       |
| W1005 | Unused exported method                       | Неиспользуемый экспортированный метод            |
| W1063 | Const generic constraint cannot be evaluated | Невозможно вычислить ограничение const-дженерика |

> Правило распределения кодов W: изоморфны кодам E, сгруппированы по фазам (W + сегмент тысяч фазы).
> W1xxx = предупреждения фазы проверки типов. Предупреждения выдаются через отдельный канал (не
> прерывают компиляцию), механизмы подавления/повышения см. в #321 (M2).

---

### Связь значений ошибок времени выполнения с кодами

> Этот раздел введён в #323 (M4 Значения Error времени выполнения с кодами, 2026-09-03).
> Семантическое пространство E6xxx/E7xxx одновременно обслуживает два канала, пространство кодов
> едино, каналы представления различаются.

#### Два канала

| Канал                             | Носитель                                                  | Способ представления                                                |
| --------------------------------- | --------------------------------------------------------- | ------------------------------------------------------------------- |
| Канал диагностики компилятора/CLI | Фатальные ошибки уровня хоста, такие как `ExecutorError`  | stderr `error[E####]:` (#280/#281 уже подключены E6003/E6005/E6007) |
| Канал значений ошибок в программе | Носитель Err `Error` из std-библиотеки `Result(T, Error)` | Значение языка, потребляемое программой через match/сравнение       |

#### Структура Error (с v0.8, breaking change)

```
Error { code: String, message: String }
```

- `code` повторно использует нумерацию E6xxx/E7xxx из данной спецификации в строковом виде
  (например, `"E6008"`).
- **Стабильный контракт**: Семантика уже выделенных кодов не изменяется между версиями; одна и та же
  семантика не использует повторно удалённые коды (прецедент E6002).
- **Поверхность потребления**: Сравнение `e.code == "E6xxx"` в программе — единственный
  программируемый контракт определения; документация `yaoxiang explain E6xxx` согласована;
  инструментарий (LSP / DAP, см. RFC-034) использует код в качестве exceptionId.
- **Аксессоры**: `std.result.code(e)` / `std.result.message(e)`.
- **Пользовательские ошибки**: `E` в `Result(T, E)` — это параметр дженерика; для серьёзного
  моделирования следует использовать пользовательские типы; `Error` из std — лишь удобный запасной
  носитель, и его система кодов не ограничивает пользовательские типы E.

#### Правила распределения кодов

1. Коды значений ошибок времени выполнения и коды диагностики compile-time разделяют пространство
   E6xxx/E7xxx, новые коды распределяются по **реальной поверхности срабатывания**, без
   резервирования под гипотетические сценарии.
2. Сначала регистрация, потом использование: новый код попадает в авторитетный реестр и проходит
   трёхстороннюю проверку согласованности (codes/*.rs ↔ locales ↔ таблица кодов данного документа)
   перед отправкой.
3. E7xxx зарезервирован для значений ошибок std.io / std.net (в настоящее время пустует, будет
   задействован при превращении io/net в Result).

#### Путь развития (линия C, не реализовано)

После завершения полноты сопоставления с образцом (RFC-039), `Error` может быть обновлён до
`{ kind: ErrorKind, message: String }`, при этом `code` станет атрибутом, производным от kind
(определение варианта — это реестр кодов). В период развития стабильный контракт кода в данном
разделе остаётся неизменным; это обновление является отдельным решением и не составляет
обязательства данного раздела.

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

/// Реестр отображаемого текста i18n (загружается из JSON в compile-time, нулевые таблицы поиска в runtime)
pub struct I18nRegistry {
    /// Заголовки
    titles: HashMap<&'static str, &'static str>,
    /// Описания
    messages: HashMap<&'static str, &'static str>,
    /// Вспомогательная информация
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

    /// Рендеринг шаблона (завершается в compile-time, нулевые накладные расходы в runtime)
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

| Плейсхолдер  | Назначение                       | Пример                              |
| ------------ | -------------------------------- | ----------------------------------- |
| `{name}`     | Имя переменной/типа/trait и т.д. | `Unknown variable: '{name}'`        |
| `{expected}` | Ожидаемый тип                    | `Expected type '{expected}'`        |
| `{found}`    | Фактический/найденный тип        | `, found type '{found}'`            |
| `{method}`   | Имя метода                       | `Method {method} is not a function` |
| `{trait}`    | Имя trait                        | `Cannot find trait: {trait}`        |
| `{path}`     | Путь модуля                      | `Invalid path: {path}`              |
| `{ty}`       | Выражение типа                   | `Invalid type: {ty}`                |
| `{message}`  | Внутреннее сообщение об ошибке   | `Internal error: {message}`         |

##### Поддержка произвольных ключей

**params поддерживает произвольные ключи, не ограничиваясь предопределёнными**. Вызывающий может
передать любой `key`:

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
# Язык сообщений об ошибках, возможные значения: en, zh, ja, ...
default = "zh"
```

#### Конфигурация уровня пользователя

```toml
# ~/.yaoxiang/yaoxiang.toml
[language]
default = "zh"
```

#### Выбор языка в compile-time

```
1. Чтение language.default из yaoxiang.toml уровня проекта
2. Если не настроен, чтение из yaoxiang.toml уровня пользователя
3. Если ничего не настроено, по умолчанию используется "en"
4. Компилятор создаёт I18nRegistry в соответствии с выбранным языком (один раз)
5. Все ошибки используют этот I18nRegistry для рендеринга сообщений
```

#### Ключ к нулевым накладным расходам на таблицы поиска

**Рендеринг происходит при компиляции пользовательского проекта, а не во время выполнения.**

```
┌─────────────────────────────────────────────────────────────────────────┐
│  Фаза 1: Компиляция компилятора YaoXiang с помощью Rust               │
│                                                                           │
│  JSON упаковывается в бинарник компилятора                              │
│  Цель: команда explain может напрямую читать данные i18n               │
└─────────────────────────────────────────────────────────────────────────┘

┌─────────────────────────────────────────────────────────────────────────┐
│  Фаза 2: Компиляция пользовательского проекта YaoXiang                 │
│          (здесь происходит рендеринг)                                  │
│                                                                           │
│  При вызове макроса error!:                                              │
│  1. Чтение yaoxiang.toml для получения языковых предпочтений             │
│  2. Загрузка i18n JSON соответствующего языка из бинарника компилятора   │
│  3. Шаблон + параметры → render() → "Unknown variable: 'x'"            │
│  4. Diagnostic.message = отрендеренная строка                            │
│                                                                           │
│  AOT-бинарник напрямую хранит конечные строки, без шаблонов, без таблиц │
└─────────────────────────────────────────────────────────────────────────┘

┌─────────────────────────────────────────────────────────────────────────┐
│  Фаза 3: Выполнение пользовательской программы                          │
│                                                                           │
│  println!("{}", diagnostic.message)                                      │
│  // Прямой вывод конечной строки, без каких-либо таблиц поиска           │
└─────────────────────────────────────────────────────────────────────────┘
```

| Компонент                    | Ответственность                            | Момент рендеринга                        |
| ---------------------------- | ------------------------------------------ | ---------------------------------------- |
| `I18nRegistry`               | Предоставляет шаблоны и отображаемый текст | При компиляции пользовательского проекта |
| `DiagnosticBuilder.render()` | Шаблон + параметры → конечная строка       | При компиляции пользовательского проекта |
| `Diagnostic.message`         | Отрендеренная строка                       | Хранит конечный результат                |
| AOT-бинарник                 | Содержит конечные строки                   | Используется напрямую в runtime          |

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

Уровень серьёзности ошибки управляется через перечисление `DiagnosticLevel`, разделённое с
нумерацией кодов ошибок:

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
# Английский по умолчанию
$ yaoxiang explain E1001
error[E1001]: Unknown variable: {name}
  --> <file>:<line>:<col>

Help: Did you mean to define it?

Example:
  let {name} = value;

# Русский вывод
$ yaoxiang explain E1001 --lang ru
error[E1001]: Неизвестная переменная: {name}
  --> <файл>:<строка>:<столбец>

Помощь: Возможно, вы хотели определить её?

Пример:
  let {name} = value;

# JSON-вывод (интеграция с LSP)
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
  "examples": ["let {name} = value;"],
  "language": "en-US"
}
```

---

### Обратная совместимость

Поскольку данный RFC проектирует систему кодов ошибок с нуля, проблем обратной совместимости не
существует.

**Стратегия будущей миграции** (для справки в последующих версиях):

1. Поддерживать отображение старых кодов ошибок на новые
2. В период миграции одновременно отображать старые и новые коды
3. Предоставить график устаревания

---

## Стратегия реализации

### Фаза первая: Базовая инфраструктура кодов ошибок

1. Создать структуру каталогов `src/diagnostics/`
2. Реализовать перечисление `ErrorCode`
3. Реализовать `Diagnostic` и `DiagnosticLevel`
4. Создать каталог файлов ресурсов и примеры JSON

### Фаза вторая: Команда explain

1. Реализовать CLI-команду `yaoxiang explain`
2. Поддержка опций `--lang` и `--json`
3. Интеграция загрузки файлов ресурсов
4. Реализация рендеринга шаблонов параметров

### Фаза третья: Интеграция на этапе компиляции

1. Обновить все точки вывода ошибок для использования новой системы
2. Реализовать инъекцию параметров шаблона сообщения
3. Добавить логику приоритета языков
4. Покрытие модульными тестами

### Фаза четвёртая: Интеграция с IDE/LSP

1. Интеграция JSON-вывода explain в LSP-сервер
2. Отображение ссылок на коды ошибок в IDE
3. Отображение объяснения ошибки при наведении
4. Предложения по быстрому исправлению

---

## Приложение

### Полная таблица быстрого поиска кодов ошибок

| Диапазон | Категория                              |
| -------- | -------------------------------------- |
| E0xxx    | Лексический и синтаксический анализ    |
| E1xxx    | Проверка типов                         |
| E2xxx    | Семантический анализ                   |
| E3xxx    | Генерация кода                         |
| E4xxx    | Generics и Trait                       |
| E5xxx    | Модули и импорт                        |
| E6xxx    | Ошибки времени выполнения              |
| E7xxx    | Ошибки ввода-вывода и системные ошибки |
| E8xxx    | Внутренние ошибки компилятора          |
| E9xxx    | Зарезервировано                        |

### Поддерживаемые языки

| Код   | Язык                 | Статус       |
| ----- | -------------------- | ------------ |
| en-US | English (US)         | По умолчанию |
| zh-CN | Упрощённый китайский | В планах     |

### Сравнение примеров сообщений об ошибках

```
# Английский (en-US)
error[E1001]: Unknown variable: x
  --> src/main.yx:5:12
   5 |   print(x)
          ^
          help: Did you mean to define it?

# Русский (ru)
error[E1001]: Неизвестная переменная: x
  --> src/main.yx:5:12
   5 |   print(x)
          ^
          помощь: Возможно, вы хотели определить её?
```

## Ссылки

- [Индекс ошибок компилятора Rust](https://doc.rust-lang.org/error_codes/error-index.html)
- [Формат сообщений об ошибках GCC](https://gcc.gnu.org/onlinedocs/gcc-13.1.0/gcc/Warning-Options.html)
- [Формат диагностики Clang](https://clang.llvm.org/diagnostics.html)
- [Language Server Protocol](https://microsoft.github.io/language-server-protocol/)
