# Справочник кодов ошибок

Компилятор YaoXiang использует коды ошибок для идентификации различных типов диагностических сообщений. Коды ошибок сгруппированы по диапазонам номеров, каждый код соответствует определённому сценарию ошибки.

---

## E0xxx -- Лексический и синтаксический анализ

Ошибки, возникающие на этапах лексического анализатора (Lexer) и синтаксического анализатора (Parser).

| Код ошибки | Шаблон | Описание |
|------------|--------|----------|
| E0001 | `Invalid character: '{char}'` | Недопустимый символ |
| E0002 | `Invalid number literal: '{literal}'` | Недопустимый числовой литерал |
| E0003 | `Unterminated string starting at line {line}` | Незавершённая строка |
| E0004 | `Invalid character literal: '{literal}'` | Недопустимый символьный литерал |
| E0010 | `Expected {expected}, found {found}` | Ожидаемая лексема |
| E0011 | `Unexpected token: '{token}'` | Неожиданная лексема |
| E0012 | `Invalid syntax: {reason}` | Недопустимый синтаксис |
| E0013 | `Mismatched {bracket_type}: opened at line {open_line}, column {open_col}, not closed` | Несовпадение скобок |
| E0014 | `Missing semicolon after {statement}` | Отсутствует точка с запятой |

## E1xxx -- Проверка типов

Ошибки этапа проверки типов, охватывающие типы переменных, вызовы функций, сопоставление с образцом, создание экземпляров обобщённых типов, семантику параллелизма и распространение ошибок.

| Код ошибки | Шаблон | Описание |
|------------|--------|----------|
| E1001 | `Unknown variable: '{name}'` | Неизвестная переменная |
| E1002 | `Expected type '{expected}', found type '{found}'` | Несоответствие типов |
| E1003 | `Unknown type: '{type}'` | Неизвестный тип |
| E1010 | `Function '{func}' expects {expected} arguments, found {found}` | Несоответствие количества аргументов |
| E1011 | `Parameter type mismatch: expected '{expected}', found '{found}'` | Несоответствие типа параметра |
| E1012 | `Return type mismatch: expected '{expected}', found '{found}'` | Несоответствие возвращаемого типа |
| E1013 | `Function not found: '{func}'` | Функция не найдена |
| E1020 | `Cannot infer type for '{expr}'` | Невозможно вывести тип |
| E1021 | `Type inference conflict: {reason}` | Конфликт вывода типа |
| E1030 | `Pattern non-exhaustive: missing patterns {patterns}` | Неполное сопоставление образцов |
| E1031 | `Unreachable pattern: '{pattern}'` | Недостижимый образец |
| E1040 | `Operation '{op}' is not supported for type '{type}'` | Операция не поддерживается |
| E1041 | `Index out of bounds: valid range is 0..{max}, found {index}` | Индекс за границами |
| E1042 | `Field '{field}' not found in struct '{struct}'` | Поле не найдено |
| E1050 | `Logical operation requires boolean operands, found '{left}' and '{right}'` | Требуются логические операнды |
| E1051 | `Logical NOT requires boolean operand, found '{type}'` | Логическое NOT требует логический операнд |
| E1052 | `Cannot dereference type '{type}', expected pointer type` | Недопустимое разыменование |
| E1053 | `Cannot access field on non-struct type '{type}'` | Доступ к полю неструктурного типа |
| E1054 | `Condition must be boolean, found '{type}'` | Несоответствие типа условия |
| E1055 | `Constraint type '{type}' can only be used in generic context` | Ограничение в не обобщённом контексте |
| E1060 | `Expected {expected} type argument(s), found {found}` | Несоответствие количества параметров типа |
| E1061 | `Cannot instantiate generic type with given arguments` | Невозможно создать экземпляр обобщённого типа |
| E1070 | `Unknown label: '{label}'` | Неизвестная метка |
| E1080 | `` `spawn` is only allowed inside @block scope (current: @{mode}) `` | `spawn` разрешён только внутри области видимости `@block` |
| E1081 | `` `?` is only allowed inside functions returning Result `` | `?` разрешён только внутри функций, возвращающих Result |
| E1082 | `` `?` requires a Result expression, found '{type}' `` | `?` может использоваться только с выражением Result |
| E1083 | `` Result error type mismatch for `?`: expected '{expected}', found '{found}' `` | Несоответствие типа ошибки для `?` |
| E1090 | `Type: Type = Type` | Невыразимо (пасхалка) |
| E1091 | `Generic meta-type self-reference is not allowed: '{decl}'` | Недопустимая самоссылка обобщённого метакласса |

## E2xxx -- Семантический анализ

Ошибки этапа семантического анализа, охватывающие области видимости, время жизни переменных, владение и разрешение сигнатур функций.

| Код ошибки | Шаблон | Описание |
|------------|--------|----------|
| E2001 | `Variable '{name}' is not in scope` | Ошибка области видимости |
| E2002 | `Duplicate definition: '{name}' is already defined in this scope` | Повторное определение |
| E2003 | `Ownership constraint violated: {reason}` | Ошибка владения |
| E2010 | `Cannot assign to immutable variable '{name}'` | Присваивание неизменяемой переменной |
| E2011 | `Use of uninitialized variable '{name}'` | Использование неинициализированной переменной |
| E2012 | `Mutability conflict: cannot use mutable reference in immutable context` | Конфликт изменчивости |
| E2013 | `Cannot shadow existing variable '{name}'` | Затенение переменной |
| E2014 | `Function calls are not allowed in top-level variable initializers` | Вызовы функций не поддерживаются в инициализаторах переменных верхнего уровня |
| E2090 | `Invalid signature: {reason}` | Недопустимая сигнатура |
| E2091 | `Invalid signature: unknown type '{type_name}'` | Неизвестный тип в сигнатуре |
| E2092 | `Invalid signature: missing '->'` | В сигнатуре отсутствует стрелка |
| E2093 | `Invalid signature: duplicate parameter '{name}'` | Повторяющееся имя параметра |
| E2094 | `Invalid signature: generic '{name}' shadows outer generic` | Параметр типа затеняет внешний обобщённый тип |
| E2095 | `Invalid signature: parameter '{name}' shadows generic` | Имя параметра затеняет обобщённый тип |

## E4xxx -- Обобщения и trait'ы

Ошибки, связанные с ограничениями обобщённых типов и системой trait'ов.

| Код ошибки | Шаблон | Описание |
|------------|--------|----------|
| E4001 | `Type '{type}' does not satisfy the trait bound '{trait}'` | Нарушение ограничения обобщённого типа |
| E4002 | `Trait '{trait}' not found` | trait не найден |
| E4003 | `Missing implementation for trait '{trait}' for type '{type}'` | Отсутствует реализация trait |
| E4004 | `Conflicting trait implementations for '{trait}'` | Конфликт реализаций trait |
| E4005 | `Associated type '{assoc_type}' not found in '{container}'` | Ассоциированный тип не найден |

## E5xxx -- Модули и импорт

Ошибки системы модулей и импорта.

| Код ошибки | Шаблон | Описание |
|------------|--------|----------|
| E5001 | `Module '{module}' not found` | Модуль не найден |
| E5002 | `Failed to import module '{module}': {reason}` | Ошибка импорта |
| E5003 | `Export '{export}' not found in module '{module}'` | Экспорт не найден |
| E5004 | `Circular dependency detected: {path}` | Циклическая зависимость |
| E5005 | `Invalid module path: '{path}'` | Недопустимый путь модуля |
| E5006 | `Duplicate import: '{name}' is already imported` | Повторный импорт |
| E5007 | `Module '{module}' exports: {available}` | Подсказка экспортов модуля |

## E6xxx -- Runtime

Ошибки, возникающие на этапе выполнения.

| Код ошибки | Шаблон | Описание |
|------------|--------|----------|
| E6001 | `Division by zero in expression: {expr}` | Деление на ноль |
| E6002 | `Null pointer dereference at {location}` | Разыменование нулевого указателя |
| E6003 | `Array index out of bounds: valid range is 0..{max}, found {index}` | Индекс массива за границами |
| E6004 | `Stack overflow: recursion depth exceeded limit {limit}` | Переполнение стека |
| E6005 | `Assertion failed: {condition}` | Ошибка проверки |
| E6006 | `Function not found: '{func}'` | Функция не найдена (runtime) |
| E6007 | `Runtime error: {message}` | Ошибка выполнения |

## E7xxx -- Ввод-вывод и система

Ошибки операций ввода-вывода и системного уровня.

| Код ошибки | Шаблон | Описание |
|------------|--------|----------|
| E7001 | `File not found: '{path}'` | Файл не найден |
| E7002 | `Permission denied: '{path}'` | Доступ запрещён |
| E7003 | `I/O error: {reason}` | Ошибка ввода-вывода |
| E7004 | `Network error: {reason}'` | Сетевая ошибка |

## E8xxx -- Внутренние ошибки компилятора

Внутренние ошибки компилятора, обычно указывающие на ошибку в самом компиляторе. При обнаружении таких ошибок сообщите о них в [GitHub Issues](https://github.com/yaoxiang/yaoxiang/issues).

| Код ошибки | Шаблон | Описание |
|------------|--------|----------|
| E8001 | `Internal compiler error: {message}` | Внутренняя ошибка компилятора |
| E8002 | `Unexpected compiler panic: {reason}` | Неожиданный сбой компилятора |
| E8003 | `Compiler phase error: {phase} - {message}` | Ошибка этапа компиляции |

## W1xxx -- Предупреждения

Предупреждения, связанные с обнаружением мёртвого кода. Предупреждения не останавливают компиляцию, но указывают на возможные проблемы в коде.

| Код предупреждения | Шаблон | Описание |
|--------------------|--------|----------|
| W1001 | `Unused exported function: '{name}'` | Неиспользуемая экспортируемая функция |
| W1002 | `Unused exported type: '{name}'` | Неиспользуемый экспортируемый тип |
| W1003 | `Unused import: '{name}'` | Неиспользуемый импорт |
| W1004 | `Unused exported variable: '{name}'` | Неиспользуемая экспортируемая переменная |
| W1005 | `Unused exported method: '{name}'` | Неиспользуемый экспортируемый метод |

---

Всего **83** диагностических кода (78 кодов ошибок + 5 кодов предупреждений).