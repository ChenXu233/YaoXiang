# Справочник по кодам ошибок

Компилятор YaoXiang использует коды ошибок для идентификации различных типов диагностических сообщений. Коды ошибок сгруппированы по диапазонам номеров, каждый код соответствует определённому сценарию ошибки.

---

## E0xxx — Лексический и синтаксический анализ

Ошибки, возникающие на этапах лексического анализатора (Lexer) и синтаксического анализатора (Parser).

| Код ошибки | Шаблон | Описание |
|--------|------|------|
| E0001 | `Invalid character: '{char}'` | Недопустимый символ |
| E0002 | `Invalid number literal: '{literal}'` | Недопустимый числовой литерал |
| E0003 | `Unterminated string starting at line {line}` | Незавершённая строка |
| E0004 | `Invalid character literal: '{literal}'` | Недопустимый символьный литерал |
| E0010 | `Expected {expected}, found {found}` | Ожидаемый токен |
| E0011 | `Unexpected token: '{token}'` | Неожиданный токен |
| E0012 | `Invalid syntax: {reason}` | Недопустимый синтаксис |
| E0013 | `Mismatched {bracket_type}: opened at line {open_line}, column {open_col}, not closed` | Несоответствующая скобка |
| E0014 | `Missing semicolon after {statement}` | Отсутствует точка с запятой |

## E1xxx — Проверка типов

Ошибки, возникающие на этапе проверки типов, охватывающие типы переменных, вызовы функций, сопоставление с образцом, инстанцирование дженериков, семантику параллелизма и распространение ошибок.

| Код ошибки | Шаблон | Описание |
|--------|------|------|
| E1001 | `Unknown variable: '{name}'` | Неизвестная переменная |
| E1002 | `Expected type '{expected}', found type '{found}'` | Несоответствие типов |
| E1003 | `Unknown type: '{type}'` | Неизвестный тип |
| E1010 | `Function '{func}' expects {expected} arguments, found {found}` | Несоответствие количества аргументов |
| E1011 | `Parameter type mismatch: expected '{expected}', found '{found}'` | Несоответствие типа аргумента |
| E1012 | `Return type mismatch: expected '{expected}', found '{found}'` | Несоответствие типа возврата |
| E1013 | `Function not found: '{func}'` | Функция не найдена |
| E1020 | `Cannot infer type for '{expr}'` | Не удаётся вывести тип |
| E1021 | `Type inference conflict: {reason}` | Конфликт вывода типа |
| E1030 | `Pattern non-exhaustive: missing patterns {patterns}` | Неполный паттерн |
| E1031 | `Unreachable pattern: '{pattern}'` | Недостижимый паттерн |
| E1040 | `Operation '{op}' is not supported for type '{type}'` | Операция не поддерживается |
| E1041 | `Index out of bounds: valid range is 0..{max}, found {index}` | Выход индекса за границы |
| E1042 | `Field '{field}' not found in struct '{struct}'` | Поле не найдено |
| E1050 | `Logical operation requires boolean operands, found '{left}' and '{right}'` | Требуются булевы операнды |
| E1051 | `Logical NOT requires boolean operand, found '{type}'` | Логическое НЕ требует булев операнд |
| E1052 | `Cannot dereference type '{type}', expected pointer type` | Недопустимое разыменование |
| E1053 | `Cannot access field on non-struct type '{type}'` | Доступ к полю неструктурного типа |
| E1054 | `Condition must be boolean, found '{type}'` | Условие должно быть булевым |
| E1055 | `Constraint type '{type}' can only be used in generic context` | Ограничение в необобщённом контексте |
| E1060 | `Expected {expected} type argument(s), found {found}` | Несоответствие количества аргументов типа |
| E1061 | `Cannot instantiate generic type with given arguments` | Не удаётся инстанцировать обобщённый тип |
| E1070 | `Unknown label: '{label}'` | Неизвестная метка |
| E1081 | `` `?` is only allowed inside functions returning Result `` | `?` разрешён только в функциях, возвращающих Result |
| E1082 | `` `?` requires a Result expression, found '{type}' `` | `?` может использоваться только с выражением Result |
| E1083 | `` Result error type mismatch for `?`: expected '{expected}', found '{found}' `` | Несоответствие типа ошибки `?` |
| E1090 | `Type: Type = Type` | Невыразимо (пасхалка) |
| E1091 | `Generic meta-type self-reference is not allowed: '{decl}'` | Недопустимая само-ссылка обобщённого мета-типа |

## E2xxx — Семантический анализ

Ошибки, возникающие на этапе семантического анализа, охватывающие области видимости, время жизни переменных, владение и разбор сигнатур функций.

| Код ошибки | Шаблон | Описание |
|--------|------|------|
| E2001 | `Variable '{name}' is not in scope` | Ошибка области видимости |
| E2002 | `Duplicate definition: '{name}' is already defined in this scope` | Дублирующееся определение |
| E2003 | `Ownership constraint violated: {reason}` | Нарушение ограничения владения |
| E2010 | `Cannot assign to immutable variable '{name}'` | Присваивание неизменяемой переменной |
| E2011 | `Use of uninitialized variable '{name}'` | Использование неинициализированной переменной |
| E2012 | `Mutability conflict: cannot use mutable reference in immutable context` | Конфликт изменяемости |
| E2013 | `Cannot shadow existing variable '{name}'` | Затенение переменной |
| E2014 | `'{name}' has been moved and cannot be used` | Использование перемещённой переменной |
| E2090 | `Invalid signature: {reason}` | Недопустимая сигнатура |
| E2091 | `Invalid signature: unknown type '{type_name}'` | Неизвестный тип в сигнатуре |
| E2092 | `Invalid signature: missing '->'` | В сигнатуре отсутствует '->' |
| E2093 | `Invalid signature: duplicate parameter '{name}'` | Дублирующееся имя параметра |
| E2094 | `Invalid signature: generic '{name}' shadows outer generic` | Затенение обобщённого параметра |
| E2095 | `Invalid signature: parameter '{name}' shadows generic` | Имя параметра затеняет обобщённый |

## E4xxx — Дженерики и трейты

Ошибки, связанные с ограничениями дженериков и системой трейтов.

| Код ошибки | Шаблон | Описание |
|--------|------|------|
| E4001 | `Type '{type}' does not satisfy the trait bound '{trait}'` | Нарушение ограничения трейта |
| E4002 | `Trait '{trait}' not found` | Трейт не найден |
| E4003 | `Missing implementation for trait '{trait}' for type '{type}'` | Отсутствует реализация трейта |
| E4004 | `Conflicting trait implementations for '{trait}'` | Конфликтующие реализации трейта |
| E4005 | `Associated type '{assoc_type}' not found in '{container}'` | Ассоциированный тип не найден |

## E5xxx — Модули и импорт

Ошибки, связанные с системой модулей и импортом.

| Код ошибки | Шаблон | Описание |
|--------|------|------|
| E5001 | `Module '{module}' not found` | Модуль не найден |
| E5002 | `Failed to import module '{module}': {reason}` | Ошибка импорта |
| E5003 | `Export '{export}' not found in module '{module}'` | Экспорт не найден |
| E5004 | `Circular dependency detected: {path}` | Циклическая зависимость |
| E5005 | `Invalid module path: '{path}'` | Недопустимый путь модуля |
| E5006 | `Duplicate import: '{name}' is already imported` | Дублирующийся импорт |
| E5007 | `Module '{module}' exports: {available}` | Подсказка об экспортах модуля |

## E6xxx — Время выполнения

Ошибки, возникающие на этапе выполнения.

| Код ошибки | Шаблон | Описание |
|--------|------|------|
| E6001 | `Division by zero in expression: {expr}` | Деление на ноль |
| E6002 | `Null pointer dereference at {location}` | Разыменование нулевого указателя |
| E6003 | `Array index out of bounds: valid range is 0..{max}, found {index}` | Выход индекса массива за границы |
| E6004 | `Stack overflow: recursion depth exceeded limit {limit}` | Переполнение стека |
| E6005 | `Assertion failed: {condition}` | Сбой утверждения |
| E6006 | `Function not found: '{func}'` | Функция не найдена (время выполнения) |
| E6007 | `Runtime error: {message}` | Ошибка времени выполнения |

## E7xxx — Ввод-вывод и система

Ошибки операций ввода-вывода и системного уровня.

| Код ошибки | Шаблон | Описание |
|--------|------|------|
| E7001 | `File not found: '{path}'` | Файл не найден |
| E7002 | `Permission denied: '{path}'` | Доступ запрещён |
| E7003 | `I/O error: {reason}` | Ошибка ввода-вывода |
| E7004 | `Network error: {reason}` | Сетевая ошибка |

## E8xxx — Внутренние ошибки компилятора

Внутренние ошибки компилятора, как правило, указывающие на баг в самом компиляторе. При обнаружении таких ошибок, пожалуйста, сообщайте в [GitHub Issues](https://github.com/yaoxiang/yaoxiang/issues).

| Код ошибки | Шаблон | Описание |
|--------|------|------|
| E8001 | `Internal compiler error: {message}` | Внутренняя ошибка компилятора |
| E8002 | `Unexpected compiler panic: {reason}` | Неожиданная паника |
| E8003 | `Compiler phase error: {phase} - {message}` | Ошибка фазы компилятора |

## W1xxx — Предупреждения

Предупреждения, связанные с обнаружением мёртвого кода. Предупреждения не препятствуют компиляции, но указывают на возможные проблемы в коде.

| Код ошибки | Шаблон | Описание |
|--------|------|------|
| W1001 | `Unused exported function: '{name}'` | Неиспользуемая экспортированная функция |
| W1002 | `Unused exported type: '{name}'` | Неиспользуемый экспортированный тип |
| W1003 | `Unused import: '{name}'` | Неиспользуемый импорт |
| W1004 | `Unused exported variable: '{name}'` | Неиспользуемая экспортированная переменная |
| W1005 | `Unused exported method: '{name}'` | Неиспользуемый экспортированный метод |

---

Всего **83** диагностических кода (78 кодов ошибок + 5 кодов предупреждений).