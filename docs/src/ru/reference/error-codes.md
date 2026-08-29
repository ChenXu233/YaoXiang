# Справочник по кодам ошибок

Компилятор YaoXiang использует коды ошибок для идентификации различных типов диагностических
сообщений. Коды ошибок сгруппированы по числовым диапазонам, каждый код соответствует определённому
сценарию ошибки.

---

## E0xxx -- Лексический и синтаксический анализ

Ошибки, возникающие на этапах лексического анализатора (Lexer) и синтаксического анализатора
(Parser).

| Код ошибки | Шаблон                                                                                 | Описание                        |
| ---------- | -------------------------------------------------------------------------------------- | ------------------------------- |
| E0001      | `Invalid character: '{char}'`                                                          | Недопустимый символ             |
| E0002      | `Invalid number literal: '{literal}'`                                                  | Недопустимый числовой литерал   |
| E0003      | `Unterminated string starting at line {line}`                                          | Незавершённая строка            |
| E0004      | `Invalid character literal: '{literal}'`                                               | Недопустимый символьный литерал |
| E0010      | `Expected {expected}, found {found}`                                                   | Ожидаемая лексема               |
| E0011      | `Unexpected token: '{token}'`                                                          | Неожиданная лексема             |
| E0012      | `Invalid syntax: {reason}`                                                             | Недопустимый синтаксис          |
| E0013      | `Mismatched {bracket_type}: opened at line {open_line}, column {open_col}, not closed` | Несоответствие скобок           |
| E0014      | `Missing semicolon after {statement}`                                                  | Отсутствует точка с запятой     |

## E1xxx -- Проверка типов

Ошибки, возникающие на этапе проверки типов, охватывающие типы переменных, вызовы функций,
сопоставление с образцом, инстанцирование обобщений, семантику конкурентности и распространение
ошибок.

| Код ошибки | Шаблон                                                                             | Описание                                                  |
| ---------- | ---------------------------------------------------------------------------------- | --------------------------------------------------------- |
| E1001      | `Unknown variable: '{name}'`                                                       | Неизвестная переменная                                    |
| E1002      | `Expected type '{expected}', found type '{found}'`                                 | Несоответствие типов                                      |
| E1003      | `Unknown type: '{type}'`                                                           | Неизвестный тип                                           |
| E1010      | `Function '{func}' expects {expected} arguments, found {found}`                    | Несоответствие количества аргументов                      |
| E1011      | `Parameter type mismatch: expected '{expected}', found '{found}'`                  | Несоответствие типа параметра                             |
| E1012      | `Return type mismatch: expected '{expected}', found '{found}'`                     | Несоответствие возвращаемого типа                         |
| E1013      | `Function not found: '{func}'`                                                     | Функция не найдена                                        |
| E1020      | `Cannot infer type for '{expr}'`                                                   | Невозможно вывести тип                                    |
| E1021      | `Type inference conflict: {reason}`                                                | Конфликт вывода типов                                     |
| E1030      | `Pattern non-exhaustive: missing patterns {patterns}`                              | Неполное сопоставление с образцом                         |
| E1031      | `Unreachable pattern: '{pattern}'`                                                 | Недостижимый образец                                      |
| E1040      | `Operation '{op}' is not supported for type '{type}'`                              | Операция не поддерживается                                |
| E1041      | `Index out of bounds: valid range is 0..{max}, found {index}`                      | Выход индекса за границы                                  |
| E1042      | `Field '{field}' not found in struct '{struct}'`                                   | Поле не найдено                                           |
| E1050      | `Logical operation requires boolean operands, found '{left}' and '{right}'`        | Требуются логические операнды                             |
| E1051      | `Logical NOT requires boolean operand, found '{type}'`                             | Логическое NOT требует логический операнд                 |
| E1052      | `Cannot dereference type '{type}', expected pointer type`                          | Недопустимое разыменование                                |
| E1053      | `Cannot access field on non-struct type '{type}'`                                  | Доступ к полю не-структурного типа                        |
| E1054      | `Condition must be boolean, found '{type}'`                                        | Несоответствие типа условия                               |
| E1055      | `Constraint type '{type}' can only be used in generic context`                     | Ограничение вне обобщённого контекста                     |
| E1060      | `Expected {expected} type argument(s), found {found}`                              | Несоответствие количества аргументов типа                 |
| E1061      | `Cannot instantiate generic type with given arguments`                             | Невозможно инстанцировать обобщённый тип                  |
| E1070      | `Unknown label: '{label}'`                                                         | Неизвестная метка                                         |
| E1081      | `` `?` is only allowed inside functions returning Result ``                        | `?` допустимо только в функциях, возвращающих Result      |
| E1082      | `` `?` requires a Result expression, found '{type}' ``                             | `?` может применяться только к выражениям Result          |
| E1083      | ``Result error type mismatch for `?`: expected '{expected}', found '{found}'``     | Несоответствие типа ошибки для `?`                        |
| E1090      | `Type: Type = Type`                                                                | Непроизносимое (пасхалка)                                 |
| E1091      | `Generic meta-type self-reference is not allowed: '{decl}'`                        | Недопустимый метатип обобщения                            |
| E1062      | `Const generic constraint violation: {reason}`                                     | Нарушение ограничения const-обобщения                     |
| E1064      | `Invalid binding position(s) {positions} for function with {total} parameter(s)`   | Недопустимый индекс позиции связывания (RFC-004)          |
| E1095      | `Unknown interface: '{name}'`                                                      | Неизвестный интерфейс (RFC-011a)                          |
| E1096      | `Interface '{name}' expects {expected} type argument(s), found {found}`            | Несоответствие числа аргументов интерфейса                |
| E1097      | `Interface member '{member}' conflicts with field of type '{type}'`                | Конфликт имени члена интерфейса и поля                    |
| E1098      | `Type '{type}' does not implement '{interface}.{method}'`                          | Метод интерфейса не реализован                            |
| E1099      | `Signature mismatch for '{type}.{method}': expected '{expected}', found '{found}'` | Несоответствие сигнатуры метода интерфейса                |
| E1100      | `Duplicate implementation of '{type}.{method}' (override is not allowed)`          | Дублирующая реализация метода (переопределение запрещено) |

## E2xxx -- Семантический анализ

Ошибки, возникающие на этапе семантического анализа, охватывающие область видимости, время жизни
переменных, владение и разбор сигнатур функций.

| Код ошибки | Шаблон                                                                   | Описание                                         |
| ---------- | ------------------------------------------------------------------------ | ------------------------------------------------ |
| E2001      | `Variable '{name}' is not in scope`                                      | Ошибка области видимости                         |
| E2002      | `Duplicate definition: '{name}' is already defined in this scope`        | Дублирующее определение                          |
| E2003      | `Ownership constraint violated: {reason}`                                | Ошибка владения                                  |
| E2010      | `Cannot assign to immutable variable '{name}'`                           | Присваивание неизменяемой переменной             |
| E2011      | `Use of uninitialized variable '{name}'`                                 | Использование неинициализированной переменной    |
| E2012      | `Mutability conflict: cannot use mutable reference in immutable context` | Конфликт изменяемости                            |
| E2013      | `Cannot shadow existing variable '{name}'`                               | Затенение переменной                             |
| E2014      | `'{name}' has been moved and cannot be used`                             | Использование перемещённой переменной            |
| E2090      | `Invalid signature: {reason}`                                            | Недопустимая сигнатура                           |
| E2091      | `Invalid signature: unknown type '{type_name}'`                          | Неизвестный тип в сигнатуре                      |
| E2092      | `Invalid signature: missing '->'`                                        | Отсутствует стрелка в сигнатуре                  |
| E2093      | `Invalid signature: duplicate parameter '{name}'`                        | Дублирующее имя параметра                        |
| E2094      | `Invalid signature: generic '{name}' shadows outer generic`              | Затенение внешнего обобщённого параметра         |
| E2095      | `Invalid signature: parameter '{name}' shadows generic`                  | Затенение обобщённого параметра именем параметра |

## E4xxx -- Обобщения и трейты

Ошибки, связанные с ограничениями обобщений и системой трейтов.

| Код ошибки | Шаблон                                                         | Описание                        |
| ---------- | -------------------------------------------------------------- | ------------------------------- |
| E4001      | `Type '{type}' does not satisfy the trait bound '{trait}'`     | Нарушение ограничения обобщения |
| E4002      | `Trait '{trait}' not found`                                    | Трейт не найден                 |
| E4003      | `Missing implementation for trait '{trait}' for type '{type}'` | Отсутствует реализация трейта   |
| E4004      | `Conflicting trait implementations for '{trait}'`              | Конфликт реализаций трейта      |
| E4005      | `Associated type '{assoc_type}' not found in '{container}'`    | Ассоциированный тип не найден   |

## E5xxx -- Модули и импорт

Ошибки, связанные с системой модулей и импортом.

| Код ошибки | Шаблон                                             | Описание                      |
| ---------- | -------------------------------------------------- | ----------------------------- |
| E5001      | `Module '{module}' not found`                      | Модуль не найден              |
| E5002      | `Failed to import module '{module}': {reason}`     | Ошибка импорта                |
| E5003      | `Export '{export}' not found in module '{module}'` | Экспорт не найден             |
| E5004      | `Circular dependency detected: {path}`             | Циклическая зависимость       |
| E5005      | `Invalid module path: '{path}'`                    | Недопустимый путь модуля      |
| E5006      | `Duplicate import: '{name}' is already imported`   | Дублирующий импорт            |
| E5007      | `Module '{module}' exports: {available}`           | Подсказка об экспортах модуля |

## E6xxx -- Время выполнения

Ошибки, возникающие на этапе выполнения.

| Код ошибки | Шаблон                                                              | Описание                              |
| ---------- | ------------------------------------------------------------------- | ------------------------------------- |
| E6001      | `Division by zero in expression: {expr}`                            | Деление на ноль                       |
| E6003      | `Array index out of bounds: valid range is 0..{max}, found {index}` | Выход индекса массива за границы      |
| E6004      | `Stack overflow: recursion depth exceeded limit {limit}`            | Переполнение стека                    |
| E6005      | `Assertion failed: {condition}`                                     | Сбой утверждения                      |
| E6006      | `Function not found: '{func}'`                                      | Функция не найдена (время выполнения) |
| E6007      | `Runtime error: {message}`                                          | Ошибка времени выполнения             |

## E7xxx -- I/O и система

Ошибки операций ввода-вывода и системного уровня.

| Код ошибки | Шаблон                        | Описание            |
| ---------- | ----------------------------- | ------------------- |
| E7001      | `File not found: '{path}'`    | Файл не найден      |
| E7002      | `Permission denied: '{path}'` | Доступ запрещён     |
| E7003      | `I/O error: {reason}`         | Ошибка ввода-вывода |
| E7004      | `Network error: {reason}`     | Сетевая ошибка      |

## E8xxx -- Внутренние ошибки компилятора

Внутренние ошибки компилятора, обычно указывающие на баг в самом компиляторе. При обнаружении таких
ошибок, пожалуйста, сообщайте о них в [GitHub Issues](https://github.com/yaoxiang/yaoxiang/issues).

| Код ошибки | Шаблон                                      | Описание                      |
| ---------- | ------------------------------------------- | ----------------------------- |
| E8001      | `Internal compiler error: {message}`        | Внутренняя ошибка компилятора |
| E8002      | `Unexpected compiler panic: {reason}`       | Неожиданная паника            |
| E8003      | `Compiler phase error: {phase} - {message}` | Ошибка фазы компиляции        |

## W1xxx -- Предупреждения

Предупреждения, связанные с обнаружением мёртвого кода. Предупреждения не препятствуют компиляции,
но указывают на возможные проблемы в коде.

| Код ошибки | Шаблон                               | Описание                                 |
| ---------- | ------------------------------------ | ---------------------------------------- |
| W1001      | `Unused exported function: '{name}'` | Неиспользуемая экспортируемая функция    |
| W1002      | `Unused exported type: '{name}'`     | Неиспользуемый экспортируемый тип        |
| W1003      | `Unused import: '{name}'`            | Неиспользуемый импорт                    |
| W1004      | `Unused exported variable: '{name}'` | Неиспользуемая экспортируемая переменная |
| W1005      | `Unused exported method: '{name}'`   | Неиспользуемый экспортируемый метод      |

| W1063 | `Const generic constraint not evaluable at compile time` | Ограничение const-обобщения не
вычислимо во время компиляции |

---

Всего **85** диагностических кодов (79 кодов ошибок + 6 кодов предупреждений).
