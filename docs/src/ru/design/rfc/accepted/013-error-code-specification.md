---
title: 'RFC 013: Спецификация кодов ошибок'
status: 'Принято'
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

Данный RFC предлагает спецификацию классификации кодов ошибок для компилятора YaoXiang, использующую
одноуровневую систему нумерации, аналогичную Rust, совместно с файлами ресурсов JSON для обеспечения
многоязычной поддержки, а также предоставляет функциональность объяснения ошибок через команду
`yaoxiang explain`.

## Мотивация

### Зачем нужны стандартизированные коды ошибок?

1. **Пользовательский опыт**: пользователь, видя код ошибки, может быстро определить тип и
   серьёзность ошибки
2. **Организация документации**: группировка по категориям упрощает составление и сопровождение
   справочной документации по ошибкам
3. **Интеграция с инструментами**: IDE/LSP могут предоставлять предложения по быстрому исправлению и
   ссылки на документацию на основе кодов ошибок
4. **Поддержка интернационализации**: отделение сообщений об ошибках от кодов упрощает многоязычный
   перевод

### Цели проектирования

- **Простота**: одноуровневая нумерация, пользователю не нужно запоминать сложные правила
  классификации
- **Дружелюбность**: формат сообщений об ошибках, похожий на Rust, со справочной информацией и
  примерами
- **Расширяемость**: управление через файлы ресурсов, простое добавление новых ошибок и языков
- **Удобство для инструментов**: команда `explain` + вывод в формате JSON, поддержка интеграции с
  IDE/LSP

---

## Предложение

### Основная концепция: одноуровневая система нумерации

Используется четырёхзначная нумерация, сгруппированная по этапам компиляции:

```
Exxxx
││││
│││└── 序号 (000-999)
││└─── 编译阶段 (0-9)
└───── 固定前缀 'E'
```

### Разделение по этапам

| 阶段  | 范围  | 描述           |
| ----- | ----- | -------------- |
| **0** | E0xxx | 词法与语法分析 |
| **1** | E1xxx | 类型检查       |
| **2** | E2xxx | 语义分析       |
| **3** | E3xxx | 代码生成       |
| **4** | E4xxx | 泛型与特质     |
| **5** | E5xxx | 模块与导入     |
| **6** | E6xxx | 运行时错误     |
| **7** | E7xxx | I/O 与系统错误 |
| **8** | E8xxx | 内部编译器错误 |
| **9** | E9xxx | 保留/实验性    |

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

### Определение кода ошибки и универсальный Builder

**Основной принцип**: определения кодов ошибок отделены от отображаемого текста

- `ErrorCodeDefinition`: метаданные кода ошибки (code, category, template), без отображаемого текста
- `locales/*.json`: отображаемый текст для разных языков (title, message, help, коды ошибок как
  вложенные объекты)
- `DiagnosticBuilder`: универсальный построитель, заменяющий дизайн с трейтом на каждую ошибку

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

#### Методы быстрого доступа для каждого кода ошибки

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

| 特性             | 说明                                     |
| ---------------- | ---------------------------------------- |
| **单一 Builder** | 一个 `DiagnosticBuilder` 通用所有错误码  |
| **类型安全**     | 快捷方法确保参数正确性                   |
| **自文档**       | `E1001::unknown_variable(name)` 一目了然 |
| **模板分离**     | 消息模板与代码分离，易于 i18n            |
| **零运行时开销** | 编译期渲染，AOT 二进制无查表             |

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

| 代码  | 错误类型                  | 说明                                                         |
| ----- | ------------------------- | ------------------------------------------------------------ |
| E0001 | Invalid character         | Исходный код содержит недопустимый символ                    |
| E0002 | Invalid number literal    | Неверный формат числового литерала                           |
| E0003 | Unterminated string       | Многострочная строка без закрывающей кавычки                 |
| E0004 | Invalid character literal | Некорректный символьный литерал                              |
| E0010 | Expected token            | Во время синтаксического анализа ожидался определённый токен |
| E0011 | Unexpected token          | Встречен неожиданный токен                                   |
| E0012 | Invalid syntax            | Синтаксическая ошибка в выражении/инструкции                 |
| E0013 | Mismatched brackets       | Несоответствие круглых, квадратных или фигурных скобок       |
| E0014 | Missing semicolon         | В конце инструкции отсутствует точка с запятой               |
| E0016 | Expected expression       | Ожидается выражение                                          |
| E0018 | Keyword as name           | Ключевое слово не может использоваться как имя               |

#### E1xxx: Проверка типов

| 代码  | 错误类型                                               | 说明                                                      |
| ----- | ------------------------------------------------------ | --------------------------------------------------------- |
| E1001 | Unknown variable                                       | Ссылка на неопределённую переменную                       |
| E1002 | Type mismatch                                          | Ожидаемый тип не соответствует фактическому               |
| E1003 | Unknown type                                           | Ссылка на несуществующий тип                              |
| E1010 | Parameter count mismatch                               | Количество аргументов вызова не соответствует определению |
| E1011 | Parameter type mismatch                                | Проверка типа аргумента не пройдена                       |
| E1012 | Return type mismatch                                   | Неверный тип возвращаемого значения функции               |
| E1013 | Function not found                                     | Вызов неопределённой функции                              |
| E1020 | Cannot infer type                                      | Невозможно вывести тип из контекста                       |
| E1021 | Type inference conflict                                | Множественные ограничения приводят к противоречию типов   |
| E1030 | Pattern non-exhaustive                                 | Выражение `match` не покрывает все случаи                 |
| E1031 | Unreachable pattern                                    | Недостижимый шаблон                                       |
| E1040 | Operation not supported                                | Тип не поддерживает данную операцию                       |
| E1041 | Index out of bounds                                    | Индекс массива/списка вне допустимого диапазона           |
| E1042 | Field not found                                        | Обращение к несуществующему полю структуры                |
| E1050 | Boolean operand required                               | Требуется логический операнд                              |
| E1051 | Logical NOT requires boolean operand                   | Логическое NOT требует логического операнда               |
| E1052 | Invalid dereference                                    | Недопустимое разыменование                                |
| E1053 | Non-struct field access                                | Обращение к полю не-структуры                             |
| E1054 | Conditional type mismatch                              | Несоответствие условного типа                             |
| E1055 | Constraint in non-generic context                      | Ограничение в не-обобщённом контексте                     |
| E1060 | Type parameter count mismatch                          | Несоответствие количества параметров типа                 |
| E1061 | Cannot instantiate generic                             | Невозможно инстанцировать дженерик                        |
| E1062 | Const generic constraint failed                        | Невыполнение ограничения const-дженерика                  |
| E1064 | Invalid binding position                               | Неверная позиция привязки (RFC-004)                       |
| E1071 | Type definitions are only allowed at module level      | Определения типов допустимы только на уровне модуля       |
| E1081 | `?` can only be used within functions returning Result | `?` допустим только в функциях, возвращающих Result       |
| E1082 | `?` can only be used with Result expressions           | `?` может использоваться только с выражениями Result      |
| E1083 | Error type mismatch for `?`                            | Несоответствие типа ошибки для `?`                        |
| E1090 | Type universe easter egg                               | Пасхалка Type: Type = Type (уровень Note)                 |
| E1091 | Invalid generic meta type                              | Недопустимый мета-тип дженерика                           |
| E1092 | Invalid refinement type argument form                  | Недопустимая форма аргумента уточнённого типа             |
| E1093 | Refinement argument count mismatch                     | Несоответствие количества уточняющих аргументов           |
| E1094 | Unused compile-time value parameter                    | Неиспользуемый параметр значения времени компиляции       |
| E1095 | Unknown interface                                      | Неизвестный интерфейс                                     |
| E1096 | Interface arity mismatch                               | Несоответствие арности интерфейса                         |
| E1097 | Interface member name conflict                         | Конфликт имён членов интерфейса                           |
| E1098 | Interface method not implemented                       | Метод интерфейса не реализован                            |
| E1099 | Interface method signature mismatch                    | Несоответствие сигнатуры метода интерфейса                |
| E1100 | Duplicate interface method implementation              | Дублирующаяся реализация метода интерфейса                |
| E1101 | Type does not implement interface                      | Тип не реализует интерфейс                                |
| E1102 | Loop control statement outside of a loop               | Управляющая инструкция цикла вне цикла                    |

#### E2xxx: Семантический анализ

| 代码  | 错误类型                          | 说明                                                     |
| ----- | --------------------------------- | -------------------------------------------------------- |
| E2001 | Scope error                       | Переменная не в текущей области видимости                |
| E2002 | Duplicate definition              | Дублирующееся определение в одной области видимости      |
| E2003 | Lifetime error                    | Ограничение времени жизни не выполнено                   |
| E2010 | Immutable assignment              | Попытка изменения неизменяемой переменной                |
| E2011 | Uninitialized use                 | Использование неинициализированной переменной            |
| E2012 | Mutability conflict               | Использование изменяемой ссылки в неизменяемом контексте |
| E2013 | Variable shadowing                | Затенение переменной                                     |
| E2014 | Use of moved value                | Использование перемещённого значения                     |
| E2016 | Immutable assignment              | Присваивание неизменяемому                               |
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

| 代码  | 错误类型                               | 说明                                                 |
| ----- | -------------------------------------- | ---------------------------------------------------- |
| E3004 | Unsupported iterator                   | Неподдерживаемый итератор                            |
| E3005 | IR generation error                    | Внутренняя ошибка генерации IR                       |
| E3006 | Unresolved variable                    | Переменная не разрешена на этапе генерации IR        |
| E3007 | Top-level initializer must be constant | Инициализатор верхнего уровня должен быть константой |
| E3014 | Register overflow                      | Переполнение регистров                               |
| E3017 | Invalid operand (code generation)      | Недопустимый операнд (генерация кода)                |

#### E4xxx: Дженерики и трейты

| 代码  | 错误类型                                | 说明                                                         |
| ----- | --------------------------------------- | ------------------------------------------------------------ |
| E4001 | Generic parameter mismatch              | Несоответствие количества/типа параметров дженерика          |
| E4002 | Trait bound violated                    | Нарушено ограничение трейта                                  |
| E4003 | Associated type error                   | Ошибка определения/использования ассоциированного типа       |
| E4004 | Duplicate trait implementation          | Дублирующаяся реализация одного трейта                       |
| E4005 | Trait not found                         | Требуемый трейт не найден                                    |
| E4006 | Sized bound violated                    | Нарушено ограничение Sized (зарезервировано, не реализовано) |
| E4010 | Division by zero in constant expression | Деление на ноль в константном выражении                      |
| E4011 | Constant overflow                       | Переполнение константы                                       |
| E4012 | Constant recursion too deep             | Слишком глубокая рекурсия в константе                        |
| E4014 | Constant evaluation failed              | Сбой вычисления константы                                    |
| E4018 | Refinement predicate violation          | Нарушение предиката уточнения                                |
| E4019 | Type equality does not hold             | Равенство типов не выполняется                               |

#### E5xxx: Модули и импорт

| 代码  | 错误类型              | 说明                                                       |
| ----- | --------------------- | ---------------------------------------------------------- |
| E5001 | Module not found      | Импортируемый модуль не существует                         |
| E5002 | Cyclic import         | Циклическая зависимость между модулями                     |
| E5003 | Symbol not exported   | Попытка доступа к неэкспортированному символу              |
| E5004 | Invalid module path   | Неверный формат пути модуля                                |
| E5005 | Private access        | Доступ к приватному символу                                |
| E5006 | Duplicate import      | Дублирующийся импорт                                       |
| E5007 | Module export listing | Список экспорта модуля (сопутствующее сообщение-подсказка) |

#### E6xxx: Ошибки времени выполнения

| 代码  | 错误类型                    | 说明                                                            |
| ----- | --------------------------- | --------------------------------------------------------------- |
| E6001 | Division by zero            | Целочисленное деление на ноль                                   |
| E6002 | ~~Assertion failed~~        | ~~Зарезервировано (нет языковой концепции, удалено)~~           |
| E6003 | Runtime index out of bounds | Выход индекса за границы во время выполнения (подключение #280) |
| E6004 | Stack overflow              | Исчерпание стека                                                |
| E6005 | Assertion failed            | Сбой `assert` (подключение #280)                                |
| E6006 | Function not found          | Функция не найдена во время выполнения                          |
| E6007 | Runtime error (generic)     | Общая ошибка времени выполнения                                 |
| E6008 | Key not found               | Отсутствует ключ в Dict (#299 §4)                               |

> **Редакция #280 (2026-08-09)**: таблица кодов была изначально определена по черновику семантики
> Rust (Assertion failed/Arithmetic overflow/Heap allocation failed/Type cast failed), что не
> соответствует фактическим потребностям реализации. В YaoXiang отсутствуют концепции нулевого
> указателя/сбоя кучи/приведения типов (семантика значений + безопасность памяти в стиле Rust), пути
> переполнения во время выполнения не имеют реализованного обнаружения. После калибровки:
>
> - E6002 удалён (исходный Assertion failed перемещён в E6005; семантика нулевого указателя не имеет
>   языковой концепции)
> - E6003 изменён с Arithmetic overflow на Runtime index out of bounds (реальная поверхность
>   срабатывания, #279/#271)
> - E6005 изменён с Heap allocation failed на Assertion failed (реальный путь `std.assert`)
> - E6006 изменён с Runtime index out of bounds на Function not found (реализация уже была такой,
>   #255)
> - E6007 изменён с Type cast failed на общий Runtime error (единая точка для неотображённых
>   вариантов ExecutorError)

#### E7xxx: Ошибки ввода-вывода и системные ошибки

| 代码  | 错误类型          | 说明                                 |
| ----- | ----------------- | ------------------------------------ |
| E7001 | File not found    | Попытка чтения несуществующего файла |
| E7002 | Permission denied | Недостаточно прав доступа к файлу    |
| E7003 | I/O error         | Общая ошибка ввода-вывода            |
| E7004 | Network error     | Сбой сетевой операции                |

#### E8xxx: Внутренние ошибки компилятора

| 代码  | 错误类型                | 说明                                                             |
| ----- | ----------------------- | ---------------------------------------------------------------- |
| E8001 | Internal compiler error | Внутренняя ошибка компилятора                                    |
| E8002 | Codegen error           | Сбой генерации IR/байткода                                       |
| E8003 | Unimplemented feature   | Использование нереализованной функции                            |
| E8004 | Optimization error      | Ошибка оптимизации компилятора (зарезервировано, не реализовано) |

#### W1xxx: Коды предупреждений

| 代码  | 警告类型                                     | 说明                                                |
| ----- | -------------------------------------------- | --------------------------------------------------- |
| W1001 | Unused exported function                     | Неиспользуемая экспортированная функция             |
| W1002 | Unused exported type                         | Неиспользуемый экспортированный тип                 |
| W1003 | Unused import                                | Неиспользуемый импорт                               |
| W1004 | Unused exported variable                     | Неиспользуемая экспортированная переменная          |
| W1005 | Unused exported method                       | Неиспользуемый экспортированный метод               |
| W1063 | Const generic constraint cannot be evaluated | Ограничение const-дженерика не может быть вычислено |

> Правила расположения W-кодов: изоморфны E-кодам, с группировкой по этапам (W + сегмент тысяч
> этапа), W1xxx = предупреждения этапа проверки типов.
>
> **Канал выдачи (#321 M2)**: диагностика W-кодов по умолчанию помечается builder'ом как
> `Severity::Warning` по префиксу W (явное указание имеет приоритет), сбор и представление идут по
> тому же пути, что и ошибки (рендеринг с префиксом `warning[W####]`), но не прерывают компиляцию и
> не влияют на успешный код возврата. `yaoxiang check --deny-warnings` повышает предупреждения до
> ошибок (при наличии предупреждений выход с ненулевым кодом), используется для строгого режима CI.
> Подавление per-code (атрибут `allow` и т.п.) — пункт для будущего расширения.

---

### Значения ошибок времени выполнения и сквозное использование кодов

> Данный раздел введён в #323 (M4 Значения `Error` времени выполнения с кодами, 2026-09-03).
> Семантическое пространство E6xxx/E7xxx одновременно обслуживает два канала, пространство кодов
> общее, каналы представления различаются.

#### Два канала

| 通道                | 载体                                          | 呈现方式                                                     |
| ------------------- | --------------------------------------------- | ------------------------------------------------------------ |
| 编译器/CLI 诊断通道 | `ExecutorError` 等宿主层硬错误                | stderr `error[E####]:`（#280/#281 已接线 E6003/E6005/E6007） |
| 程序内错误值通道    | std 库 `Result(T, Error)` 的 Err 载体 `Error` | 语言值，由程序 match/比较消费                                |

#### Структура `Error` (начиная с v0.8, разрушающее изменение)

```
Error { code: String, message: String }
```

- `code` повторно использует нумерацию E6xxx/E7xxx данной спецификации в строковом виде (например,
  `"E6008"`).
- **Стабильный контракт**: семантика назначенных кодов не меняется между версиями; удалённые коды не
  используются повторно для той же семантики (прецедент E6002).
- **Сторона потребления**: сравнение `e.code == "E6xxx"` внутри программы — единственный
  программируемый контракт определения; документация `yaoxiang explain E6xxx` сквозная;
  инструментарий (LSP/DAP, см. RFC-034) использует код как `exceptionId`.
- **Аксессоры**: `std.result.code(e)` / `std.result.message(e)`.
- **Ошибки, определяемые пользователем**: E в `Result(T, E)` — параметр дженерика; серьёзное
  моделирование выполняется через пользовательские типы; std `Error` — лишь удобный резервный
  носитель, его система кодов не ограничивает пользовательский тип E.

#### Правила назначения кодов

1. Коды значений ошибок времени выполнения и диагностические коды времени компиляции совместно
   используют пространство E6xxx/E7xxx; новые коды назначаются по **реальной поверхности
   срабатывания**, без резервирования под воображаемые сценарии.
2. Сначала регистрация, потом использование: новые коды попадают в авторитетный реестр и проходят
   трёхстороннюю проверку согласованности (codes/*.rs ↔ locales ↔ таблица кодов данного документа),
   после чего могут выдаваться.
3. E7xxx — сегмент, зарезервированный для значений ошибок `std.io` / `std.net` (в настоящее время
   пуст, активируется при превращении io/net в Result).

#### Путь развития (линия C, не реализовано)

После завершения доработки сопоставления с образцом (RFC-039), `Error` может быть обновлён до
`{ kind: ErrorKind, message: String }`, при этом `code` становится атрибутом, производным от kind
(определение варианта служит реестром кодов). В период развития стабильный контракт `code` данного
раздела остаётся неизменным; данное обновление — независимое решение и не является обязательством
данного раздела.

---

### Многоязычные файлы ресурсов

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

| 占位符       | 用途                         | 示例                                |
| ------------ | ---------------------------- | ----------------------------------- |
| `{name}`     | 变量名/类型名/特质名等标识符 | `Unknown variable: '{name}'`        |
| `{expected}` | 期望类型                     | `Expected type '{expected}'`        |
| `{found}`    | 实际/找到的类型              | `, found type '{found}'`            |
| `{method}`   | 方法名                       | `Method {method} is not a function` |
| `{trait}`    | 特质名                       | `Cannot find trait: {trait}`        |
| `{path}`     | 模块路径                     | `Invalid path: {path}`              |
| `{ty}`       | 类型表达式                   | `Invalid type: {ty}`                |
| `{message}`  | 内部错误消息                 | `Internal error: {message}`         |

##### Поддержка произвольных ключей

**`params` поддерживает произвольные ключи, не ограничиваясь предопределёнными**. Вызывающая сторона
может передать любой `key`:

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

> **Примечание**: не все коды ошибок используют плейсхолдеры. Некоторые коды ошибок (например,
> E0001) — статические сообщения, параметры не требуются.

#### Приоритет языка

```
1. yaoxiang.toml [language.default]
2. ~/.yaoxiang/yaoxiang.toml [language.default]
3. 默认值: en
```

### Конфигурация `yaoxiang.toml`

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

1. Чтение `language.default` из `yaoxiang.toml` уровня проекта
2. Если не сконфигурировано, чтение из `~/.yaoxiang/yaoxiang.toml` уровня пользователя
3. Если не сконфигурировано нигде, по умолчанию используется `"en"`
4. Компилятор создаёт `I18nRegistry` согласно выбранному языку (однократно)
5. Все ошибки используют этот `I18nRegistry` для рендеринга сообщений

#### Ключ к нулевым накладным расходам на поиск по таблицам

**Рендеринг происходит во время компиляции пользовательского проекта, а не во время выполнения.**

```
┌─────────────────────────────────────────────────────────────────────────┐
│  阶段 1: Rust 编译 YaoXiang 编译器                                      │
│                                                                           │
│  JSON 打包进编译器二进制                                                 │
│  目的：explain 指令能直接读取 i18n 数据                                  │
└─────────────────────────────────────────────────────────────────────────┘

┌─────────────────────────────────────────────────────────────────────────┐
│  阶段 2: YaoXiang 编译用户项目（渲染发生在这里）                          │
│                                                                           │
│  error! 宏调用时：                                                       │
│  1. 读取 yaoxiang.toml 获取语言偏好                                      │
│  2. 从编译器二进制加载对应语言的 i18n JSON                                │
│  3. 模板 + 参数 → render() → "Unknown variable: 'x'"                    │
│  4. Diagnostic.message = 已渲染的字符串                                   │
│                                                                           │
│  AOT 二进制直接存储最终字符串，无模板，无查表                            │
└─────────────────────────────────────────────────────────────────────────┘

┌─────────────────────────────────────────────────────────────────────────┐
│  阶段 3: 用户程序运行时                                                  │
│                                                                           │
│  println!("{}", diagnostic.message)                                      │
│  // 直接输出最终字符串，无任何查表                                        │
└─────────────────────────────────────────────────────────────────────────┘
```

| 组件                         | 职责                     | 渲染时机       |
| ---------------------------- | ------------------------ | -------------- |
| `I18nRegistry`               | 提供模板和展示文案       | 编译用户项目时 |
| `DiagnosticBuilder.render()` | 模板 + 参数 → 最终字符串 | 编译用户项目时 |
| `Diagnostic.message`         | 已渲染的字符串           | 存储最终结果   |
| AOT 二进制                   | 包含最终字符串           | 运行时直接用   |

---

### Формат сообщения об ошибке

Сообщения об ошибках используют следующий формат:

```
error[E####]: <简短描述>
  --> <文件>:<行>:<列>
   <行> | <代码片段>
          ^^^<高亮>
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

Серьёзность ошибки управляется перечислением `DiagnosticLevel`, отделённым от нумерации кода ошибки:

```rust
pub enum DiagnosticLevel {
    Error,    // 导致编译失败
    Warning,  // 不影响编译，但建议修复
    Note,     // 补充信息
    Help,     // 修复建议
}
```

| 级别    | 前缀              | 说明         |
| ------- | ----------------- | ------------ |
| Error   | `error[E####]:`   | 导致编译失败 |
| Warning | `warning[E####]:` | 不影响编译   |
| Note    | `note[E####]:`    | 补充信息     |
| Help    | `help[E####]:`    | 修复建议     |

---

### Команда `yaoxiang explain`

#### Синтаксис команды

```bash
yaoxiang explain <ERROR_CODE> [OPTIONS]
```

#### Опции

| 选项            | 描述                                |
| --------------- | ----------------------------------- |
| `--lang <code>` | 指定语言 (en-US, zh-CN，默认 en-US) |
| `--json`        | JSON 格式输出（供 IDE/LSP 使用）    |
| `--json-pretty` | 格式化的 JSON 输出                  |
| `--examples`    | 只显示示例代码                      |
| `--help`        | 显示帮助信息                        |

#### Примеры использования

```bash
# 默认英文
$ yaoxiang explain E1001
error[E1001]: Unknown variable: {name}
  --> <file>:<line>:<col>

Help: Did you mean to define it?

Example:
  let {name} = value;

# 中文输出
$ yaoxiang explain E1001 --lang zh
error[E1001]: 未知变量: {name}
  --> <file>:<line>:<col>

帮助: 你是否想要定义它？

示例:
  let {name} = value;

# JSON 输出（LSP 集成）
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

Поскольку данный RFC разрабатывает систему кодов ошибок с нуля, проблем обратной совместимости не
существует.

**Стратегия миграции в будущем** (для справки в последующих версиях):

1. Сохранение отображения старых кодов ошибок в новые
2. Одновременное отображение старых и новых кодов в период миграции
3. Предоставление графика устаревания

---

## Стратегия реализации

### Этап 1: Базовая инфраструктура кодов ошибок

1. Создание структуры каталогов `src/diagnostics/`
2. Реализация перечисления `ErrorCode`
3. Реализация `Diagnostic` и `DiagnosticLevel`
4. Создание каталога файлов ресурсов и примеров JSON

### Этап 2: Команда `explain`

1. Реализация CLI-команды `yaoxiang explain`
2. Поддержка опций `--lang` и `--json`
3. Интеграция загрузки файлов ресурсов
4. Реализация рендеринга шаблонов параметров

### Этап 3: Интеграция на этапе компиляции

1. Обновление всех точек отчёта об ошибках для использования новой системы
2. Реализация инъекции параметров в шаблоны сообщений
3. Добавление логики приоритета языка
4. Покрытие модульными тестами

### Этап 4: Интеграция с IDE/LSP

1. Интеграция JSON-вывода `explain` в LSP-сервер
2. Отображение ссылок на коды ошибок в IDE
3. Отображение объяснения ошибки при наведении
4. Предложения по быстрому исправлению

---

## Приложение

### Полная сводная таблица кодов ошибок

| 范围  | 类别           |
| ----- | -------------- |
| E0xxx | 词法与语法分析 |
| E1xxx | 类型检查       |
| E2xxx | 语义分析       |
| E3xxx | 代码生成       |
| E4xxx | 泛型与特质     |
| E5xxx | 模块与导入     |
| E6xxx | 运行时错误     |
| E7xxx | I/O 与系统错误 |
| E8xxx | 内部编译器错误 |
| E9xxx | 保留           |

### Поддерживаемые языки

| 代码  | 语言         | 状态   |
| ----- | ------------ | ------ |
| en-US | English (US) | 默认   |
| zh-CN | 简体中文     | 计划中 |

### Сравнение примеров сообщений об ошибках

```
# 英文 (en-US)
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
