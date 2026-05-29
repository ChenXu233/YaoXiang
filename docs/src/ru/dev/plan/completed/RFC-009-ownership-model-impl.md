# RFC-009 Путь реализации модели владения

> **Версия документа**: v1.0
> **На основе дизайна**: `docs/design/accepted/009-ownership-model.md`
> **Дата создания**: 2025-02-05

## Обзор реализации

Настоящий документ преобразует проектные спецификации RFC-009 в выполнимые шаги реализации на основе существующей архитектуры YaoXiang.

### Существующая база

| Модуль | Расположение | Статус |
|--------|--------------|--------|
| Система владения | `src/middle/passes/lifetime/` | ✅ Базовая часть готова |
| Move-семантика | `move_semantics.rs` | ✅ Реализовано |
| ref-семантика | `ref_semantics.rs` | ✅ Реализовано |
| Проверка циклов | `cycle_check.rs` | ✅ Реализовано |
| mut-проверка | `mut_check.rs` | ✅ Реализовано |

---

## Phase 1: Полевая неизменяемость (P0)

### Цель

Поддержка маркера `mut` для полей в определениях типов, реализация трёхуровневой модели изменяемости:

- Изменяемость привязки (уровень переменной)
- Полевая изменяемость (уровень структуры)
- Изменяемость параметров метода (уровень функции)

### Статус реализации: ✅ Завершено (2026-02-05)

#### Выполненные изменения (обновление 2026-02-05)

1. **Расширение AST** (`ast.rs`)
   - ✅ Создана структура `StructField`: `name: String, is_mut: bool, ty: Type`
   - ✅ `Type::Struct(Vec<StructField>)` заменяет `Type::Struct(Vec<(String, Type)>)`
   - ✅ `Type::NamedStruct { name, fields: Vec<StructField> }`
   - ✅ `Pattern::Struct { name, fields: Vec<(String, bool, Box<Pattern>)> }`

2. **Расширение Parser** (`statements/declarations.rs`)
   - ✅ `parse_struct_type` поддерживает синтаксис `{ x: Float, mut y: Float }`
   - ✅ `parse_named_struct_type` поддерживает синтаксис `Point(x: Float, mut y: Float)`

3. **Система типов** (`type_system/mono.rs`)
   - ✅ `StructType` добавлено `field_mutability: Vec<bool>`
   - ✅ Реализован метод `field_is_mut(&self, field_name: &str) -> Option<bool>`
   - ✅ Обновлена логика преобразования `MonoType::from(ast::Type)`

4. **Выведение типов в шаблонах** (`typecheck/inference/patterns.rs`)
   - ✅ Выведение типов шаблонов поддерживает маркер `is_mut`

5. **Разбор шаблонов Parser** (`parser/pratt/nud.rs`)
   - ✅ Разбор синтаксиса структурных шаблонов поддерживает ключевое слово `mut`

6. **Типы ошибок** (`lifetime/error.rs`)
   - ✅ Добавлен вариант ошибки `ImmutableFieldAssign`
   - ✅ Реализация Display

7. **Расширение IR-инструкций** (`middle/core/ir.rs`)
   - ✅ `StoreField` добавлены `type_name: Option<String>` и `field_name: Option<String>`

8. **Генерация IR** (`middle/core/ir_gen.rs`)
   - ✅ `get_field_mutability` возвращает имя типа
   - ✅ Инструкция StoreField несёт информацию о типе

9. **Проверка изменяемости** (`lifetime/mut_check.rs`)
   - ✅ Проверка изменяемости на уровне привязки
   - ✅ Проверка изменяемости на уровне поля (с передачей таблицы типов)
   - ✅ Обнаружение ошибки `ImmutableFieldAssign`

10. **Генерация кода** (`codegen/translator.rs`)
    - ✅ Исправлено сопоставление с образцом StoreField (использование `..` для игнорирования дополнительных полей)

### Затрагиваемые файлы

| Тип | Файл | Статус |
|-----|------|--------|
| Изменён | `src/frontend/core/parser/ast.rs` | ✅ Завершено |
| Изменён | `src/frontend/core/parser/statements/declarations.rs` | ✅ Завершено |
| Изменён | `src/frontend/core/parser/pratt/nud.rs` | ✅ Завершено |
| Изменён | `src/frontend/core/type_system/mono.rs` | ✅ Завершено |
| Изменён | `src/frontend/typecheck/inference/patterns.rs` | ✅ Завершено |
| Изменён | `src/frontend/typecheck/mod.rs` | ✅ Завершено |
| Изменён | `src/frontend/type_level/auto_derive.rs` | ✅ Завершено |
| Изменён | `src/middle/passes/lifetime/error.rs` | ✅ Завершено |
| Изменён | `src/middle/passes/lifetime/mut_check.rs` | ✅ Завершено |
| Изменён | `src/middle/core/ir_gen.rs` | ✅ Завершено |
| Изменён | `src/middle/passes/codegen/mod.rs` | ✅ Завершено |
| Изменён | `src/middle/passes/mono/cross_module.rs` | ✅ Завершено |
| Изменён | `src/middle/passes/mono/function.rs` | ✅ Завершено |
| Изменён | `src/middle/passes/mono/module_state.rs` | ✅ Завершено |
| Изменён | `src/middle/passes/mono/type_mono.rs` | ✅ Завершено |
| Изменён | `src/frontend/core/type_system/solver.rs` | ✅ Завершено |
| Изменён | `src/frontend/core/type_system/substitute.rs` | ✅ Завершено |
| Изменён | `src/frontend/typecheck/specialization/algorithm.rs` | ✅ Завершено |
| Изменён | `src/frontend/typecheck/specialize.rs` | ✅ Завершено |
| Изменён | `src/frontend/typecheck/overload.rs` | ✅ Завершено |
| Изменён | `src/frontend/typecheck/inference/expressions.rs` | ✅ Завершено |

### Критерии приёмки

- [x] Синтаксис `type Point { x: Float, mut y: Float }` разбирается корректно
- [x] Синтаксис именованной структуры `type Point(x: Float, mut y: Float)` разбирается корректно
- [x] Конструктор `NamedStruct(Point(x: Float, mut y: Float))` поддерживает изменчивые поля
- [x] `mut p: Point = Point(1.0, 2.0); p.y = 3.0` компилируется успешно (привязка изменчива, поле изменчиво)
- [x] `p.y = 3.0` компилируется при неизменяемой привязке (привязка неизменна, поле изменчиво)
- [x] `p.x = 3.0` не компилируется при неизменяемой привязке (привязка неизменна, поле неизменно) → `ImmutableFieldAssign`
- [x] `p.x = 3.0` компилируется при изменяемой привязке (привязка изменчива, поле записываемо)

### Описание реализации

1. **Изменения структур данных** (завершено)
   - Структура `StructField`: `name, is_mut, ty`
   - `StructType.field_mutability: Vec<bool>`
   - `Pattern::Struct` поддерживает маркер `is_mut`

2. **Слой Parser** (завершено)
   - `parse_struct_type` поддерживает `{ x: Float, mut y: Float }`
   - `parse_named_struct_type` поддерживает `Point(x: Float, mut y: Float)`

3. **Генерация IR** (завершено)
   - Присваивание полю `p.y = value` генерирует инструкцию `StoreField`
   - Метод `get_field_mutability` запрашивает изменяемость поля
   - `StoreField` несёт `type_name` и `field_name` для проверки

4. **MutChecker** (завершено)
   - Проверка изменяемости на уровне привязки: проверяет, объявлена ли переменная как `mut`
   - Проверка изменяемости на уровне поля: проверяет, объявлено ли поле как `mut`
   - Правило: **Привязка изменчива ИЛИ поле изменчиво** → Присваивание разрешено
   - Архитектура: передача `HashMap<String, StructType>` таблицы типов
   - Добавлен парсер: `parse_let_stmt` и `parse_pattern`
   - Генерация IR: `generate_pattern_ir` обрабатывает деструктуризацию шаблонов

### Для последующей оптимизации

(Текущая Phase 1 завершена)

---

## Phase 2: Повторное использование пустого состояния (P1) ✅ Завершено

### Цель

Реализация перехода переменной в `empty` состояние после Move с возможностью повторного присваивания и переиспользования имени переменной.

### Статус реализации: ✅ Завершено (2026-02-05)

#### Выполненные изменения (обновление 2026-02-05)

1. **Расширение ValueState** (`error.rs`)
   - ✅ `ValueState::Owned(Option<TypeId>)` добавлено отслеживание типа
   - ✅ `ValueState::Empty` новый вариант пустого состояния
   - ✅ Добавлен тип `TypeId` идентификатора типа
   - ✅ Добавлены типы ошибок `EmptyStateTypeMismatch` и `ReassignNonEmpty`

2. **Отслеживание пустого состояния** (новый файл `empty_state.rs`)
   - ✅ Создана структура `EmptyStateTracker`
   - ✅ Реализовано отслеживание состояния и проверка типов
   - ✅ Реализовано консервативное слияние состояний ветвлений

3. **Анализ потока управления** (новый файл `control_flow.rs`)
   - ✅ Создана структура `ControlFlowAnalyzer`
   - ✅ Реализована стратегия консервативного слияния `merge_states`
   - ✅ Вспомогательные функции анализа активных переменных

4. **Расширение Move-проверки** (`move_semantics.rs`)
   - ✅ После Move переменная переходит в Empty-состояние (а не Moved)
   - ✅ Переменные в пустом состоянии допускают повторное присваивание
   - ✅ Проверка согласованности типов
   - ✅ Параметры функций переходят в Empty-состояние

5. **Адаптация других проверок**
   - ✅ `clone.rs`: Обновлено для адаптации к Empty-состоянию
   - ✅ `drop_semantics.rs`: Drop для Empty-состояния корректен
   - ✅ `ref_semantics.rs`: Обновлено для адаптации к Empty-состоянию

6. **Регистрация модулей** (`mod.rs`)
   - ✅ Зарегистрированы модули `empty_state` и `control_flow`

### Затрагиваемые файлы

| Тип | Файл | Статус |
|-----|------|--------|
| Изменён | `src/middle/passes/lifetime/error.rs` | ✅ Завершено |
| Новый | `src/middle/passes/lifetime/empty_state.rs` | ✅ Завершено |
| Новый | `src/middle/passes/lifetime/control_flow.rs` | ✅ Завершено |
| Изменён | `src/middle/passes/lifetime/move_semantics.rs` | ✅ Завершено |
| Изменён | `src/middle/passes/lifetime/clone.rs` | ✅ Завершено |
| Изменён | `src/middle/passes/lifetime/drop_semantics.rs` | ✅ Завершено |
| Изменён | `src/middle/passes/lifetime/ref_semantics.rs` | ✅ Завершено |
| Изменён | `src/middle/passes/lifetime/mod.rs` | ✅ Завершено |

### Критерии приёмки

- [x] `p = Point(1.0); p2 = p; p = Point(2.0)` компилируется успешно
- [x] `p = Point(1.0); p2 = p; print(p)` не компилируется (UseAfterMove)
- [x] Ветвления if корректно отслеживают пустое состояние (консервативный анализ)
- [x] `p = "hello"` после Point выдаёт ошибку (EmptyStateTypeMismatch)

### Описание реализации

1. **Дизайн состояний**
   - `Owned(Option<TypeId>)`: Допустимое значение с информацией о типе
   - `Empty`: Пустое состояние, допускает повторное присваивание
   - `Moved`: Перемещено (сохранено для совместимости)
   - `Dropped`: Освобождено

2. **Переходы состояний**
   ```
   Owned ──Move──► Empty ──(Store, тип совпадает)──► Owned
                         ▲
                         │
                    Ошибка: несоответствие типов
   ```

3. **Консервативное слияние ветвлений**
   - Любая ветвь Empty → после слияния Empty
   - Любая ветвь Moved → после слияния Moved
   - Оба Owned → сохранить первый

4. **Проверка типов**
   - При повторном присваивании проверяется согласованность типов
   - При несоответствии типов выдаётся `EmptyStateTypeMismatch`

### Для последующей оптимизации

(Текущая Phase 2 завершена)

---

## Phase 3: Возврат владения (P1) ✅ Завершено

### Цель

Реализация возврата параметров функции после модификации, формирование замкнутого цикла владения, поддержка цепочечных вызовов.

### Статус реализации: ✅ Завершено (2026-02-06)

#### Выполненные изменения (обновление 2026-02-06)

1. **Enum режима потребления** (`ownership_flow.rs`)
   - ✅ Создан enum `ConsumeMode`: `Returns | Consumes | Undetermined`
   - ✅ `Returns`: Параметр возвращается в возвращаемом значении, владение возвращается
   - ✅ `Consumes`: Параметр потребляется, не возвращается
   - ✅ `Undetermined`: Невозможно определить (консервативный анализ)

2. **Анализатор возврата владения** (`ownership_flow.rs`)
   - ✅ Создана структура `OwnershipFlowAnalyzer`
   - ✅ `analyze_function()` анализирует режим потребления функции
   - ✅ `operand_references_param()` проверяет, ссылается ли возвращаемое значение на параметр
   - ✅ `returns_param_directly()` быстрое обнаружение шаблона `return p;`
   - ✅ Консервативная оценка: временные переменные могут ссылаться на параметр

3. **Анализатор цепочечных вызовов** (`chain_calls.rs`)
   - ✅ Создана структура `ChainCallAnalyzer`
   - ✅ `analyze_chain()` анализирует поток владения в цепочке методов
   - ✅ `extract_chain_calls()` извлекает непрерывные виртуальные вызовы методов
   - ✅ `infer_consume_mode()` выводит режим потребления на основе использования
   - ✅ `check_ownership_closure()` проверяет замыкание владения

4. **Расширение типов ошибок** (`error.rs`)
   - ✅ Добавлен вариант ошибки `ConsumedNotReturned`
   - ✅ Для диагностики параметров, которые потребляются, но не возвращаются

5. **Регистрация модулей** (`mod.rs`)
   - ✅ Зарегистрированы модули `ownership_flow` и `chain_calls`

### Затрагиваемые файлы

| Тип | Файл | Статус |
|-----|------|--------|
| Новый | `src/middle/passes/lifetime/ownership_flow.rs` | ✅ Завершено |
| Новый | `src/middle/passes/lifetime/chain_calls.rs` | ✅ Завершено |
| Изменён | `src/middle/passes/lifetime/error.rs` | ✅ Завершено |
| Изменён | `src/middle/passes/lifetime/mod.rs` | ✅ Завершено |

### Критерии приёмки

- [x] `p = p.process()` выводится как режим Returns
- [x] `consume(p)` выводится как режим Consumes
- [x] `p = p.rotate(90).scale(2.0).translate(1.0)` цепочечные вызовы корректны
- [x] Ошибки неверного вывода возврата содержат точные подсказки

### Описание реализации

1. **Дизайн ConsumeMode**
   ```
   ConsumeMode::Returns     → Параметр возвращается в возвращаемом значении
   ConsumeMode::Consumes   → Параметр потребляется, не возвращается
   ConsumeMode::Undetermined → Невозможно определить, консервативный анализ
   ```

2. **Обнаружение ссылок на параметр**
   - Прямая ссылка: `Operand::Arg(idx)` → проверка совпадения индекса
   - Временные переменные: консервативная оценка возможных ссылок на параметр
   - Константы/глобальные переменные: не ссылаются на параметр

3. **Анализ цепочечных вызовов**
   ```ignore
   p.rotate(90)    // Метод 1: rotate
     .scale(2.0)   // Метод 2: scale (obj = temp_1)
     .translate(1.0); // Метод 3: translate (obj = temp_2)
   ```

4. **Проверка замыкания владения**
   - Режим Consumes → владение корректно замкнуто
   - Режим Returns → возвращаемое значение должно использоваться
   - Undetermined → консервативный возврат true

### Покрытие тестами

| Модуль | Кол-во тестов | Описание |
|--------|---------------|----------|
| `ownership_flow` | 10 | Обнаружение ссылок на параметр, вывод режима |
| `chain_calls` | 13 | Цепочечные вызовы, замыкание владения |

### Для последующей оптимизации

(Текущая Phase 3 завершена)

---

## Phase 4: Анализ потребления (P1) ✅ Завершено

### Цель

Реализация полной системы маркеров потребления, отслеживание состояния Consumes/Returns для каждой переменной.

### Статус реализации: ✅ Завершено (2026-02-06)

#### Выполненные изменения (обновление 2026-02-06)

1. **Анализатор потребления** (новый файл `consume_analysis.rs`)
   - ✅ Повторное использование `ConsumeMode` и `OwnershipFlowAnalyzer` из Phase 3
   - ✅ `ConsumeAnalyzer` предоставляет запросы режима потребления между функциями
   - ✅ Специальная обработка встроенных функций (consume, clone и т.д.)
   - ✅ Механизм кэширования режима потребления

2. **Трекер жизненного цикла** (новый файл `lifecycle.rs`)
   - ✅ Создана структура `LifecycleTracker`
   - ✅ Запись событий жизненного цикла переменных (создание/потребление/перемещение/освобождение/возврат)
   - ✅ Статистика количества потреблений и чтений
   - ✅ Обнаружение проблем жизненного цикла (освобождение без потребления/многократное потребление/использование после потребления)

3. **Расширение MoveChecker** (расширение `move_semantics.rs`)
   - ✅ Добавлено поле `ConsumeAnalyzer`
   - ✅ `check_call` определяет состояние параметров на основе режима потребления функции
   - ✅ Режим Returns: владение параметром возвращается, не переходит в Empty
   - ✅ Режим Consumes: параметр переходит в Empty

4. **Регистрация модулей** (`mod.rs`)
   - ✅ Зарегистрированы модули `consume_analysis` и `lifecycle`

### Затрагиваемые файлы

| Тип | Файл | Статус |
|-----|------|--------|
| Новый | `src/middle/passes/lifetime/consume_analysis.rs` | ✅ Завершено |
| Новый | `src/middle/passes/lifetime/lifecycle.rs` | ✅ Завершено |
| Изменён | `src/middle/passes/lifetime/move_semantics.rs` | ✅ Завершено |
| Изменён | `src/middle/passes/lifetime/mod.rs` | ✅ Завершено |
| Изменён | `src/middle/passes/lifetime/ownership_flow.rs` | ✅ Завершено |

### Критерии приёмки

- [x] Присваивание/передача параметров/возврат корректно маркируются как Move
- [x] После `consume(x)` x становится пустым (режим Consumes)
- [x] `x = modify(x)` выводится как Returns (повторное использование OwnershipFlowAnalyzer)
- [x] `clone()` корректно копирует, не влияя на оригинальную переменную (обработка встроенных функций)

### Описание реализации

1. **Повторное использование результатов Phase 3**
   - Непосредственное использование enum `ConsumeMode` из `ownership_flow.rs`
   - `OwnershipFlowAnalyzer` для анализа режима потребления на уровне функции

2. **Дизайн анализатора потребления**
   ```
   ConsumeMode::Returns     → Владение параметром возвращается, сохраняется Owned
   ConsumeMode::Consumes   → Параметр потребляется, переходит в Empty
   ConsumeMode::Undetermined → Консервативная оценка перехода в Empty
   ```

3. **Отслеживание жизненного цикла**
   ```
   События: Created → Consumed → Moved → Dropped → Returned
   Обнаружение: освобождение без потребления / многократное потребление / использование после потребления / никогда не использовано
   ```

4. **Интеграция MoveChecker**
   - `check_call` запрашивает режим потребления вызываемой функции
   - Режим Returns: состояние параметра не меняется
   - Режим Consumes: параметр переходит в Empty

---

## Phase 5: Ключевое слово ref = Arc (P1) ✅ Завершено

### Цель

Ключевое слово `ref` реализуется как Arc с подсчётом ссылок для потокобезопасности.

### Статус реализации: ✅ Завершено (2026-02-06)

#### Выполненные изменения (обновление 2026-02-06)

1. **Синтаксический разбор ref** (имеется)
   - ✅ `parser/expr.rs`: `parse_ref` разбирает синтаксис `ref expression`
   - ✅ `ast.rs`: Узел AST `Expr::Ref { expr, span }`

2. **Выведение типов** (имеется)
   - ✅ `typecheck/infer.rs`: `ref T` выводится как `Arc[T]`

3. **Обработка владения** (имеется)
   - ✅ `ref_semantics.rs`: Проверка владения ArcNew/Clone/Drop

4. **Генерация IR** (новое)
   - ✅ `ir_gen.rs`: Добавлена генерация инструкций `Expr::Ref` → `ArcNew`

### Затрагиваемые файлы

| Тип | Файл | Статус |
|-----|------|--------|
| Изменён | `src/frontend/core/parser/expr.rs` | ✅ Имеется |
| Изменён | `src/frontend/typecheck/infer.rs` | ✅ Имеется |
| Изменён | `src/middle/passes/lifetime/ref_semantics.rs` | ✅ Имеется |
| Изменён | `src/middle/core/ir_gen.rs` | ✅ Новое в этой фазе |

### Критерии приёмки

- [x] `ref p` тип выводится как `Arc[Point]`
- [x] `ref p` не потребляет p, p остаётся доступным
- [x] `spawn(() => print(shared.x))` компилируется успешно
- [x] Выражения `ref` могут быть вложенными

### Описание реализации

1. **Генерация IR** (реализовано в этой фазе)
   ```rust
   Expr::Ref { expr, span: _ } => {
       let src_reg = self.next_temp_reg();
       self.generate_expr_ir(expr, src_reg, instructions, constants)?;
       instructions.push(Instruction::ArcNew {
           dst: Operand::Local(result_reg),
           src: Operand::Local(src_reg),
       });
   }
   ```

2. **Семантика владения**
   - `ArcNew`: Создаёт Arc, не влияет на состояние оригинального значения
   - `ArcClone`: Клонирует Arc, не влияет на состояние оригинального значения
   - `ArcDrop`: Освобождает Arc, не влияет на состояние оригинального значения

### Для последующей оптимизации

(Текущая Phase 5 завершена)

---

## Phase 6: Обнаружение циклических ссылок (P1) ✅ Завершено

### Цель

Обнаружение циклических ссылок между задачами, циклы внутри задачи допускаются.

### Статус реализации: ✅ Завершено (2026-02-06)

#### Выполненные изменения (обновление 2026-02-06)

1. **Расширение типов ошибок** (`error.rs`)
   - ✅ Вариант предупреждения `IntraTaskCycle` (цикл внутри задачи, не блокирует компиляцию)
   - ✅ Информационный вариант `UnsafeBypassCycle` (запись绕过)
   - ✅ Реализация Display

2. **Усиление CycleChecker** (`cycle_check.rs`)
   - ✅ Константа ограничения глубины `MAX_DETECTION_DEPTH = 1` (только одноуровневая граница)
   - ✅ Поле `unsafe_ranges` для отслеживания диапазонов unsafe-блоков
   - ✅ Поле `unsafe_bypasses` для записи绕过информации
   - ✅ Метод `is_in_unsafe()` проверяет, находится ли позиция внутри unsafe-блока
   - ✅ Метод `find_spawn_result_direct()` реализует ограничение глубины
   - ✅ Метод `collect_unsafe_ranges()` зарезервирован для интерфейса Phase 7
   - ✅ Оптимизация сообщений об ошибках, включая предложения по решению

3. **Трекер циклов внутри задачи** (новый файл `intra_task_cycle.rs`)
   - ✅ Структура `IntraTaskCycleTracker`
   - ✅ Структура `RefEdge` для отслеживания ref-рёбер
   - ✅ Метод `track_function()` отслеживает циклы внутри функции
   - ✅ Метод `collect_ref_info()` собирает ArcNew/Move/StoreField
   - ✅ Метод `build_ref_graph()` строит граф ссылок
   - ✅ Метод `detect_intra_task_cycles()` DFS-обнаружение циклов
   - ✅ Вывод в режиме предупреждения, не блокирует компиляцию

4. **Интеграция OwnershipChecker** (`mod.rs`)
   - ✅ Добавлено поле `intra_task_tracker`
   - ✅ `check_function()` вызывает отслеживание циклов внутри задачи
   - ✅ Метод `intra_task_warnings()` возвращает предупреждения
   - ✅ Метод `unsafe_bypasses()` возвращает записи绕过

### Затрагиваемые файлы

| Тип | Файл | Статус |
|-----|------|--------|
| Изменён | `src/middle/passes/lifetime/error.rs` | ✅ Завершено |
| Изменён | `src/middle/passes/lifetime/cycle_check.rs` | ✅ Завершено |
| Новый | `src/middle/passes/lifetime/intra_task_cycle.rs` | ✅ Завершено |
| Изменён | `src/middle/passes/lifetime/mod.rs` | ✅ Завершено |

### Критерии приёмки

- [x] Обнаружение ref-циклов между параметрами и返回值ами spawn
- [x] Циклы внутри задачи не вызывают ошибок (утечка контролируема)
- [x] Циклы между задачами вызывают ошибки с точным положением
- [x] unsafe-блоки могу绕过обнаружение (интерфейс зарезервирован, Phase 7 улучшит)

### Описание реализации

1. **Дизайн ограничения глубины**
   - Обнаружение только одноуровневой границы spawn (глубина = 1)
   - `find_spawn_result_direct()` отслеживает максимум один уровень Move
   - Не рекурсивно обнаруживает косвенные ссылки вложенных spawn

2. **Разделение обнаружения циклов**
   ```
   CycleChecker        → Циклы через spawn (ошибка)
   IntraTaskCycleTracker → Циклы внутри задачи (предупреждение)
   ```

3. **Механизм绕过unsafe**
   - `collect_unsafe_ranges()` собирает диапазоны unsafe-блоков
   - `is_in_unsafe()` проверяет позицию инструкции
   - spawn внутри unsafe-блока пропускает обнаружение
   - Текущая версия зарезервировала интерфейс, Phase 7 реализует unsafe-синтаксис

4. **Оптимизация сообщений об ошибках**
   ```
   Циклическая ссылка между задачами: temp_0 → temp_1 → temp_0 (образует цикл).
   Предложение: Используйте Weak для разрыва цикла или绕过обнаружение в unsafe-блоке
   ```

### Покрытие тестами

| Модуль | Кол-во тестов | Описание |
|--------|---------------|----------|
| `cycle_check` | 22 | Циклы между задачами, ограничение глубины, сброс состояния |
| `intra_task_cycle` | 7 | Циклы внутри задачи, самореференция, положение предупреждений |

### Для последующей оптимизации

- После реализации unsafe-синтаксиса в Phase 7 улучшить `collect_unsafe_ranges()` разбор

---

## Phase 7: unsafe + голые указатели (P2) ✅ Завершено

### Цель

Поддержка операций с голыми указателями `*T` в блоках `unsafe`.

### Статус реализации: ✅ Завершено (2026-02-06)

#### Выполненные изменения (обновление 2026-02-06)

1. **Ключевое слово и Token** (`tokens.rs`, `state.rs`)
   - ✅ Добавлено ключевое слово `KwUnsafe`
   - ✅ `state.rs`: Добавлено `"unsafe" => Some(TokenKind::KwUnsafe)`

2. **Расширение AST** (`ast.rs`)
   - ✅ `Expr::Unsafe { body: Box<Block>, span }` - выражение unsafe-блока
   - ✅ `Type::Ptr(Box<Type>)` - тип голого указателя `*T`
   - ✅ `UnOp::Deref` - оператор разыменования

3. **Расширение Parser** (`pratt/nud.rs`, `statements/declarations.rs`)
   - ✅ `parse_unsafe()` - разбор синтаксиса `unsafe { ... }`
   - ✅ `parse_unary()` - поддержка синтаксиса разыменования `*expr`
   - ✅ `parse_type_annotation()` - поддержка типа `*T`

4. **Расширение IR-инструкций** (`ir.rs`)
   - ✅ `Instruction::UnsafeBlockStart` - маркер начала unsafe-блока
   - ✅ `Instruction::UnsafeBlockEnd` - маркер конца unsafe-блока
   - ✅ `Instruction::PtrFromRef { dst, src }` - `&value → *T`
   - ✅ `Instruction::PtrDeref { dst, src }` - `*ptr → value`
   - ✅ `Instruction::PtrStore { dst, src }` - `*ptr = value`
   - ✅ `Instruction::PtrLoad { dst, src }` - загрузка указателя

5. **Генерация IR** (`ir_gen.rs`)
   - ✅ `Expr::Unsafe` → инструкции-обёртки `UnsafeBlockStart/End`
   - ✅ `UnOp::Deref` → инструкция `PtrDeref`

6. **Система типов** (`mono.rs`, `cross_module.rs`, `function.rs`, `module_state.rs`, `type_mono.rs`)
   - ✅ `Type::Ptr` → `MonoType::TypeRef("*{...}")`
   - ✅ Преобразование имени типа поддерживает голые указатели

7. **Выведение типов** (`expressions.rs`)
   - ✅ `infer_unary()` поддерживает выведение типа для `Deref`
   - ✅ `infer_expr()` поддерживает выведение типа для `Expr::Unsafe`

8. **Сбор диапазонов Unsafe** (`cycle_check.rs`)
   - ✅ `collect_unsafe_ranges()` разбирает инструкции `UnsafeBlockStart/End`

9. **Unsafe-проверка** (новый файл `unsafe_check.rs`)
   - ✅ Структура `UnsafeChecker`
   - ✅ `check_function()` - проверяет разыменование вне unsafe-блоков
   - ✅ Тип ошибки `UnsafeDeref`

10. **Расширение типов ошибок** (`error.rs`)
    - ✅ Вариант `OwnershipError::UnsafeDeref`
    - ✅ Реализация Display

11. **Генерация кода** (`translator.rs`)
    - ✅ Заглушка для unsafe-блоков и инструкций указателей

### Затрагиваемые файлы

| Тип | Файл | Статус |
|-----|------|--------|
| Изменён | `src/frontend/core/lexer/tokens.rs` | ✅ Завершено |
| Изменён | `src/frontend/core/lexer/state.rs` | ✅ Завершено |
| Изменён | `src/frontend/core/parser/ast.rs` | ✅ Завершено |
| Изменён | `src/frontend/core/parser/pratt/nud.rs` | ✅ Завершено |
| Изменён | `src/frontend/core/parser/statements/declarations.rs` | ✅ Завершено |
| Изменён | `src/middle/core/ir.rs` | ✅ Завершено |
| Изменён | `src/middle/core/ir_gen.rs` | ✅ Завершено |
| Изменён | `src/middle/passes/lifetime/cycle_check.rs` | ✅ Завершено |
| Новый | `src/middle/passes/lifetime/unsafe_check.rs` | ✅ Завершено |
| Изменён | `src/middle/passes/lifetime/error.rs` | ✅ Завершено |
| Изменён | `src/middle/passes/lifetime/mod.rs` | ✅ Завершено |
| Изменён | `src/middle/passes/codegen/translator.rs` | ✅ Завершено |
| Изменён | `src/frontend/core/type_system/mono.rs` | ✅ Завершено |
| Изменён | `src/middle/passes/mono/cross_module.rs` | ✅ Завершено |
| Изменён | `src/middle/passes/mono/function.rs` | ✅ Завершено |
| Изменён | `src/middle/passes/mono/module_state.rs` | ✅ Завершено |
| Изменён | `src/middle/passes/mono/type_mono.rs` | ✅ Завершено |
| Изменён | `src/frontend/typecheck/inference/expressions.rs` | ✅ Завершено |

### Критерии приёмки

- [x] Синтаксис `unsafe { ... }` разбирается корректно
- [x] Тип голого указателя `*T` разбирается корректно
- [x] Синтаксис разыменования `*ptr` разбирается корректно
- [x] `unsafe { *ptr }` компилируется успешно
- [x] `*ptr` вне unsafe-блока выдаёт ошибку `UnsafeDeref`
- [x] Тип голого указателя представляется как `*{type}`
- [x] unsafe-блок генерирует IR-маркеры `UnsafeBlockStart/End`
- [x] `collect_unsafe_ranges()` корректно собирает диапазоны unsafe

### Описание реализации

1. **Дизайн AST**
   ```rust
   Expr::Unsafe {
       body: Box<Block>,
       span: Span,
   }
   Type::Ptr(Box<Type>)  // *T
   UnOp::Deref           // *expr
   ```

2. **Дизайн IR**
   ```
   UnsafeBlockStart
   // инструкции внутри блока...
   UnsafeBlockEnd
   ```

3. **Выведение типа разыменования**
   ```rust
   UnOp::Deref => {
       if let MonoType::TypeRef(inner) = expr {
           // Убрать * префикс для получения внутреннего типа
           let inner_type = inner.trim_start_matches('*').to_string();
           Ok(MonoType::TypeRef(inner_type))
       } else {
           Err(Diagnostic::error("Dereference requires pointer type"))
       }
   }
   ```

4. **Представление типов голых указателей**
   - Разбор: `*T` → `Type::Ptr(Box<Type>)`
   - IR: `PtrFromRef`, `PtrDeref`, `PtrStore`, `PtrLoad`
   - MonoType: `*{type_name}`

### Покрытие тестами

| Модуль | Кол-во тестов | Описание |
|--------|---------------|----------|
| Parser | - | Синтаксический разбор unsafe/deref/ptr |
| TypeCheck | - | Выведение типов указателей |
| IR Gen | - | Генерация IR для unsafe-блоков и указателей |
| UnsafeCheck | - | Проверка разыменования вне unsafe-блоков |

### Для последующей оптимизации

- Phase 8+ реализует генерацию кода для голых указателей (операции с адресами wasm)
- Добавить отслеживание области видимости `UnsafeBlock`

---

## Phase 8: Weak стандартной библиотеки (P1) ✅ Завершено

### Цель

Реализация модуля `std.weak.Weak`, поддержка слабых ссылок, не блокирующих освобождение цели.

### Статус реализации: ✅ Завершено (2026-02-06)

**Корректировка дизайна**:
- Не реализуем `std.rc` и `std.sync` (т.к. `ref` уже удовлетворяет потребности)
- Реализуем только тип `Weak[T]`

#### Выполненные изменения (обновление 2026-02-06)

1. **Расширение системы типов** (`mono.rs`)
   - ✅ Добавлен вариант `MonoType::Weak(Box<MonoType>)`
   - ✅ Обновлён метод `type_name()`

2. **Распространение ограничений** (`constraint.rs`, `substitute.rs`)
   - ✅ Распространение ограничений Send + Sync для Weak
   - ✅ Логика замены типов для Weak

3. **Проверка типов** (`specialize.rs`, `overload.rs`)
   - ✅ Обработка специализации для Weak
   - ✅ Сопоставление перегрузок для Weak

4. **Поддержка времени выполнения** (`value.rs`)
   - ✅ Добавлен `RuntimeValue::Weak(std::sync::Weak<RuntimeValue>)`
   - ✅ Реализован `upgrade()` возвращающий `Option<RuntimeValue>`
   - ✅ Реализована `from_arc_into_weak()` для создания Weak

5. **Байткод-инструкции** (`bytecode.rs`, `opcode.rs`)
   - ✅ Добавлены `BytecodeInstr::WeakNew` / `WeakUpgrade`
   - ✅ Добавлены `Opcode::WeakNew(0x7E)` / `WeakUpgrade(0x7F)`

6. **Интерпретатор** (`executor.rs`)
   - ✅ `WeakNew`: `Arc → Weak`
   - ✅ `WeakUpgrade`: `Weak → Option<Arc>`

7. **Стандартная библиотека** (`weak.rs`, `mod.rs`)
   - ✅ Создан `src/std/weak.rs`
   - ✅ Зарегистрирован `pub mod weak`

### Затрагиваемые файлы

| Тип | Файл | Статус |
|-----|------|--------|
| Изменён | `src/frontend/core/type_system/mono.rs` | ✅ Завершено |
| Изменён | `src/frontend/core/type_system/constraint.rs` | ✅ Завершено |
| Изменён | `src/frontend/core/type_system/substitute.rs` | ✅ Завершено |
| Изменён | `src/frontend/typecheck/specialize.rs` | ✅ Завершено |
| Изменён | `src/frontend/typecheck/overload.rs` | ✅ Завершено |
| Изменён | `src/backends/common/value.rs` | ✅ Завершено |
| Изменён | `src/backends/common/opcode.rs` | ✅ Завершено |
| Изменён | `src/middle/core/bytecode.rs` | ✅ Завершено |
| Изменён | `src/backends/interpreter/executor.rs` | ✅ Завершено |
| Изменён | `src/middle/passes/codegen/bytecode.rs` | ✅ Завершено |
| Изменён | `src/middle/passes/lifetime/send_sync.rs` | ✅ Завершено |
| Изменён | `src/middle/passes/mono/dce.rs` | ✅ Завершено |
| Изменён | `src/middle/passes/mono/instance.rs` | ✅ Завершено |
| Изменён | `src/middle/passes/mono/instantiation_graph.rs` | ✅ Завершено |
| Изменён | `src/middle/passes/mono/type_mono.rs` | ✅ Завершено |
| Изменён | `src/lib.rs` | ✅ Завершено |
| Новый | `src/std/weak.rs` | ✅ Завершено |
| Изменён | `src/std/mod.rs` | ✅ Завершено |

### Критерии приёмки

- [x] Модуль `use std.weak.Weak` зарегистрирован
- [x] Поддержка типа `MonoType::Weak` в системе типов
- [x] Байткод-инструкции `WeakNew` / `WeakUpgrade`
- [x] Поддержка `RuntimeValue::Weak` во время выполнения
- [x] Распространение ограничений Send + Sync
- [x] Компиляция проходит успешно

### Описание реализации

1. **Дизайн Weak**
   ```
   Arc[T] ──Weak::new()──► Weak[T] ──upgrade()──► Option[Arc[T]]
   ```

2. **Байткод-инструкции**
   ```
   WeakNew { dst, src }    # Arc -> Weak
   WeakUpgrade { dst, src } # Weak -> Option<Arc>
   ```

3. **Поведение во время выполнения**
   - `WeakNew`: Использует `Arc::downgrade()` для создания Weak
   - `WeakUpgrade`: Использует `weak.upgrade()` для возврата Option
   - После освобождения Arc, upgrade возвращает None

### Покрытие тестами

| Модуль | Кол-во тестов | Описание |
|--------|---------------|----------|
| RuntimeValue::Weak | - | Создание и обновление Weak |
| Executor WeakNew | - | Выполнение байткода |
| Executor WeakUpgrade | - | Возврат Option |

### Для последующей оптимизации

- Добавить полные тестовые случаи для интерпретатора
- Добавить тесты проверки типов

---

## Зависимости

```
Phase 1 (неизменяемость полей)
    │
    ├─► Phase 2 (повторное использование пустого состояния)
    │       │
    │       └─► Phase 3 (возврат владения)
    │
    ├─► Phase 4 (анализ потребления)
    │       │
    │       └─► Phase 5 (ref = Arc)
    │               │
    │               └─► Phase 6 (обнаружение циклов)
    │
    ├─► Phase 7 (unsafe + голые указатели)
    │
    └─► Phase 8 (Rc/Arc/Weak)
```

---

## Перечень файлов

### Новые файлы

| Файл | Phase | Описание |
|------|-------|----------|
| `src/middle/passes/lifetime/empty_state.rs` | P2 | Отслеживание пустого состояния |
| `src/middle/passes/lifetime/control_flow.rs` | P2 | Анализ потока управления |
| `src/middle/passes/lifetime/ownership_flow.rs` | P3 ✅ | Вывод возврата владения |
| `src/middle/passes/lifetime/chain_calls.rs` | P3 ✅ | Анализ цепочечных вызовов |
| `src/middle/passes/lifetime/consume_analysis.rs` | P4 ✅ | Система маркеров потребления |
| `src/middle/passes/lifetime/lifecycle.rs` | P4 ✅ | Отслеживание жизненного цикла переменных |
| `src/middle/passes/lifetime/unsafe_check.rs` | P7 | unsafe-проверка |
| `src/middle/passes/lifetime/intra_task_cycle.rs` | P6 ✅ | Обработка циклов внутри задачи |
| `src/std/rc.rs` | P8 | Реализация Rc/Weak |
| `src/std/sync.rs` | P8 | Реализация Arc |

### Изменённые файлы

| Файл | Phase | Содержание изменений |
|------|-------|----------------------|
| `src/frontend/core/parser/ast.rs` | P1 | Создание StructField, изменение Type/Pattern |
| `src/frontend/core/parser/statements/declarations.rs` | P1 | Parser поддерживает mut-поля |
| `src/frontend/core/parser/pratt/nud.rs` | P1 | Разбор структурных шаблонов поддерживает mut |
| `src/frontend/core/type_system/mono.rs` | P1 | StructType добавлено field_mutability |
| `src/frontend/typecheck/inference/patterns.rs` | P1 | Выведение типов шаблонов поддерживает is_mut |
| `src/frontend/typecheck/mod.rs` | P1 | Адаптация StructField |
| `src/frontend/type_level/auto_derive.rs` | P1 | Адаптация StructField |
| `src/frontend/core/type_system/solver.rs` | P1 | Адаптация field_mutability |
| `src/frontend/core/type_system/substitute.rs` | P1 | Адаптация field_mutability |
| `src/frontend/typecheck/specialization/algorithm.rs` | P1 | Адаптация field_mutability |
| `src/frontend/typecheck/specialize.rs` | P1 | Адаптация field_mutability |
| `src/frontend/typecheck/overload.rs` | P1 | Адаптация field_mutability |
| `src/middle/passes/lifetime/error.rs` | P1 | Добавлен ImmutableFieldAssign |
| `src/middle/passes/lifetime/mut_check.rs` | P1 | Расширение проверки StoreField |
| `src/middle/core/ir_gen.rs` | P1 | Адаптация StructField |
| `src/middle/passes/codegen/mod.rs` | P1 | Адаптация StructField |
| `src/middle/passes/mono/cross_module.rs` | P1 | Адаптация field_mutability |
| `src/middle/passes/mono/function.rs` | P1 | Адаптация StructField |
| `src/middle/passes/mono/module_state.rs` | P1 | Адаптация StructField |
| `src/middle/passes/mono/type_mono.rs` | P1 | Адаптация field_mutability |
| `src/middle/passes/lifetime/move_semantics.rs` | P2, P4 ✅ | Проверка пустого состояния, анализ потребления |
| `src/middle/passes/lifetime/error.rs` | P3 | Диагностика ошибок возврата |
| `src/middle/passes/lifetime/ownership_flow.rs` | P4 | ConsumeMode добавлен Copy |
| `src/frontend/core/parser/expr.rs` | P5 | Разбор выражений ref |
| `src/frontend/typecheck/infer.rs` | P5 | Выведение типов ref |
| `src/middle/passes/lifetime/ref_semantics.rs` | P5 | Обработка владения ref |
| `src/middle/passes/lifetime/cycle_check.rs` | P6 ✅ | Обнаружение циклов между задачами, ограничение глубины,绕过unsafe |
| `src/middle/passes/lifetime/error.rs` | P6 ✅ | IntraTaskCycle, UnsafeBypassCycle |
| `src/middle/passes/lifetime/mod.rs` | P6 ✅ | Интеграция IntraTaskCycleTracker |
| `src/frontend/core/parser/block.rs` | P7 | Разбор синтаксиса unsafe |

---

## Временные оценки

| Phase | Сложность | Оценка сроков |
|-------|-----------|---------------|
| P1: Неизменяемость полей | Средняя | 3-4 дня |
| P2: Повторное использование пустого состояния | Средняя | 2-3 дня |
| P3: Возврат владения | Низкая | 1-2 дня |
| P4: Анализ потребления | Средняя | 2-3 дня |
| P5: ref = Arc | Низкая | 1 день (имеется база) |
| P6: Обнаружение циклов | Средняя | 2 дня (имеется база) |
| P7: unsafe + голые указатели | Средняя | 2-3 дня |
| P8: Rc/Arc/Weak | Средняя | 3-4 дня |

**Итого**: примерно 16-22 рабочих дня