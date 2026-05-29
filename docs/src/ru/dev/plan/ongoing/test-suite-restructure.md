```markdown
# План рефакторинга тестовой системы

> Статус: в плане
> Ветка: refactor/test-suite
> Дата: 2026-05-10

## I. Зачем нужен рефакторинг

### Текущие проблемы

1. **1752 теста проходят, но не обнаруживают реальные баги**
   - Выражения match возвращают 0 (ir_gen не обрабатывает узлы Match)
   - Списковые включения возвращают 0 (ir_gen не обрабатывает узлы ListComp)
   - Объявление переменной с аннотацией типа `x: Int = 42` не разбирается

2. **Интеграционные тесты проверяют только успешную компиляцию, не верифицируют корректность вывода**
   - `tests/integration/interpreter.rs` только `assert!(result.is_ok())`
   - `tests/integration/execution.rs` полностью закомментирован

3. **E2E .yx файлы не организованы**
   - Смесь старого и нового: `closure_test.yx` (старый) и `spec_features_test.yx` (новый) в одном каталоге
   - Беспорядочные имена: `closure_test.yx`, `closure_test2.yx`, `mut_param_test.yx`
   - Нет планирования покрытия: нет маппинга на главы языковой спецификации

4. **Встроенные тесты разрознены**
   - `src/frontend/typecheck/tests/` содержит 23 файла, многие тестируют одно и то же
   - Тесты scope разбросаны по 4 файлам
   - Тесты infer разбросаны по 3 файлам
   - `typecheck_fixes.rs` похож на исторический патч

5. **Тесты Codegen изолированы**
   - Все IR написаны вручную, не проходят полный конвейер parser→typecheck→ir_gen
   - Тестируется "может ли ручной IR быть переведён в байткод", а не "корректен ли результат компиляции исходника"

### Цели рефакторинга

1. **Создать трёхуровневую тестовую систему** с чётким разделением ответственности и стандартами покрытия
2. **E2E тесты могут служить бенчмарками** — каждый .yx тестовый файл может измерять время выполнения
3. **Стандартизировать внутренние тесты** — единые соглашения, именование, паттерны assertion
4. **Покрыть ключевые пути языковой спецификации** — обеспечить наличие тестов для каждого синтаксического элемента

---

## II. Трёхуровневая тестовая система

### Первый уровень: E2E .yx тестовый набор (tests/yaoxiang/)

Организация по главам языковой спецификации, каждый файл соответствует одному синтаксическому элементу.

```
tests/yaoxiang/
├── 00-smoke/                 # Дымовые тесты
│   └── hello.yx
│
├── 01-basics/                # Базовая грамматика (главы 2/4/5 спецификации)
│   ├── variables.yx          # Объявление переменных + вывод типов
│   ├── typed_vars.yx         # Переменные с типом x: Int = 42
│   ├── operators.yx          # Все операторы
│   ├── literals.yx           # Все 字面量
│   └── comments.yx           # Комментарии
│
├── 02-functions/             # Функции (глава 6 спецификации)
│   ├── definitions.yx        # name: (params) -> Ret = ...
│   ├── lambdas.yx            # Lambda выражения
│   ├── closures.yx           # Замыкания
│   └── generics.yx           # 泛型 функции
│
├── 03-control-flow/          # Управление потоком (главы 4/5 спецификации)
│   ├── if_else.yx
│   ├── while.yx
│   ├── for.yx
│   ├── match.yx
│   └── list_comp.yx          # Списковые включения
│
├── 04-types/                 # 类型系统 (глава 3 спецификации)
│   ├── structs.yx            # Point: Type = { x: Float, y: Float }
│   ├── enums.yx              # Color: Type = { red | green | blue }
│   └── generics.yx           # Option: (T: Type) -> Type = ...
│
├── 05-data-structures/       # Коллекции (секция 2.6 спецификации)
│   ├── lists.yx
│   ├── tuples.yx
│   └── dicts.yx
│
├── 06-modules/               # Система модулей (глава 7 спецификации)
│   ├── imports.yx
│   └── lib/
│
└── 07-errors/                # Обработка ошибок (глава 9 спецификации, отмечены нереализованные фичи)
    ├── result.yx
    └── option.yx
```

**Соглашения по файлам**:

```yaoxiang
// 01-basics/variables.yx
// 覆盖: 规范 §5.2 变量声明, §6.2 类型推导
// 验证: 基本声明、类型推导、可变性
// 分支: refactor/test-suite
// 状态: ✅ 可运行

use std.io

main = {
    x = 42
    io.println(x)
    // expect: 42

    s = "hello"
    io.println(s)
    // expect: hello

    io.println("ALL TESTS PASSED")
}
```

**Механизм assertion**: Фреймворк тестирования Rust перехватывает stdout и верифицирует появление строки `ALL TESTS PASSED` в выводе каждого .yx файла.

**Расширение для Benchmark**: `.yx` тестовые файлы естественно служат перфоманс-бенчмарками — измеряют время каждого прогона. В будущем можно обернуть в `criterion` для отслеживания регрессий производительности.

### Второй уровень: Интеграционные тесты (tests/integration/)

Тестируют полный конвейер компиляции+исполнения, верифицируют выходные значения.

| Текущий файл | Действие | Описание |
|-------------|---------|---------|
| `interpreter.rs` | Переписать | Компилировать исходник → выполнить → assertion значений вывода |
| `execution.rs` | Переписать (раскомментировать) | Исправить stack overflow, запускать реальные .yx файлы |
| `codegen.rs` | Сохранить | Сериализация/десериализация байткода |
| `codegen_extended.rs` | Сохранить | Тесты opcode/metadata |
| `fstring.rs` | Сохранить | Дополнить верификацию исполнения |
| `backends.rs` | Сохранить | Тесты типа RuntimeValue |

**Дополнение**: `tests/yx_runner.rs` — автоматическое обнаружение и запуск всех .yx файлов из `tests/yaoxiang/`.

### Третий уровень: Модульные тесты (src/*/tests/)

Тестируют внутреннюю логику отдельных модулей, имеют доступ к приватным API.

#### 3.1 Тесты Lexer (src/frontend/core/lexer/tests/)

11 файлов → удалить 1 отладочный, оставить 10.

| Действие | Файл |
|---------|------|
| Удалить | `debug_lexer.rs` — только для отладки |
| Сохранить | `basic.rs`, `comments.rs`, `keywords.rs`, `literals.rs`, `operators.rs` |
| Сохранить | `delimiters.rs`, `errors.rs`, `fstring.rs` |
| Сохранить | `rfc004_lexer.rs`, `rfc010_lexer.rs` |

#### 3.2 Тесты Parser (src/frontend/core/parser/tests/)

13 файлов → минимальная корректировка после ревью.

| Действие | Файл |
|---------|------|
| Сохранить | `basic.rs`, `fn_def.rs`, `syntax_validation.rs`, `old_syntax_rejection.rs` |
| Сохранить | `boundary.rs`, `concurrency.rs`, `fstring.rs` |
| Сохранить | `ref_test.rs`, `unsafe_ptr.rs`, `state.rs` |
| Ревью | `binding_enhancements.rs` — проверить дублирование с fn_def |

#### 3.3 Тесты Typecheck (src/frontend/typecheck/tests/)

**Главная проблемная зона**: 23 файла → объединить в 12.

| Действие | Исходный файл | Целевой файл |
|---------|--------------|-------------|
| Объединить | `infer.rs` + `inference.rs` + `types.rs` | `type_inference.rs` |
| Объединить | `scope.rs` + `shadowing.rs` + `use_scope.rs` + `use_block_scope.rs` | `scoping.rs` |
| Объединить | `visibility.rs` + `pub_bind.rs` | `visibility.rs` |
| Ревью | `typecheck_fixes.rs` | Если это исторический патч — объединить в соответствующий файл и удалить |
| Сохранить | `basic.rs`, `check.rs` | — |
| Сохранить | `constraint.rs`, `concurrency.rs`, `fstring.rs` | — |
| Сохранить | `gat.rs`, `ref_test.rs`, `result_try.rs` | — |
| Сохранить | `semantic_tokens.rs`, `traits.rs`, `type_constructor_rules.rs` | — |

#### 3.4 Тесты Middle/Codegen (src/middle/passes/tests/)

| Каталог | Действие |
|--------|---------|
| `codegen/` | Сохранить, **добавить интеграционные codegen тесты** (от исходника до IR) |
| `lifetime/` | Сохранить без изменений |
| `mono/` | Сохранить без изменений |
| `module/` | Сохранить без изменений |

## III. Документ стандартов тестирования

В том же каталоге создать `TEST_STANDARD.md`:

### Соглашения по именованию

```
Назначение     Паттерн                    Пример
────────────────────────────────────────────────────────────────
Тестовый модуль mod_<описание>_tests       mod_parser_basic_tests
Тестовая функция test_<фича>_<сценарий>   test_parse_fn_def_no_params
E2E файл        <глава>-<фича>.yx          01-basics-variables.yx
```

### Соглашения по assertion

- E2E `.yx` файлы: выводить `ALL TESTS PASSED` в конце
- Интеграционные тесты: верифицировать stdout содержит ожидаемые значения
- Модульные тесты: верифицировать значения полей структур данных, не использовать `assert!(result.is_ok())` как единственный assertion

### Соглашения по комментариям

```yaoxiang
// Заголовок E2E файла:
// 覆盖: 规范 §X.X
// 验证: Краткое описание
// 分支: refactor/test-suite
// 状态: ✅ 可运行 / ⚠️ 待修复 / 🔴 未实现
```

### Обработка нереализованных фич

- E2E `.yx` для несуществующей функциональности: не писать тесты, добавить после реализации
- Модульные тесты, ссылающиеся на нереализованное: пометить `#[ignore]` с комментарием "待 XXX 实现后启用"

---

## IV. План выполнения

### Phase 0: Подготовительные работы

- [ ] Создать ветку `refactor/test-suite` от `dev`
- [ ] Провести ревью `typecheck_fixes.rs` и `binding_enhancements.rs`, решить об удалении
- [ ] Провести ревью `tests/integration/execution.rs` на предмет stack overflow

### Phase 1: Фреймворк E2E тестов

- [ ] Создать `tests/yx_runner.rs` — автоматическое обнаружение и запуск `tests/yaoxiang/**/*.yx`
- [ ] Создать новую структуру каталогов `tests/yaoxiang/`
- [ ] Написать 00-smoke дымовые тесты
- [ ] Написать слой 01-basics (текущая работающая грамматика)
- [ ] Написать слой 02-functions

### Phase 2: Исправление runtime багов + соответствующие тесты

- [ ] Исправить выражения match (добавить обработку Match в ir_gen)
- [ ] Исправить списковые включения (добавить обработку ListComp в ir_gen)
- [ ] Исправить объявление переменных с типом `x: Int = 42`
- [ ] Добавить соответствующие .yx E2E тесты для перечисленных исправлений

### Phase 3: Переработка интеграционных тестов

- [ ] Переписать `tests/integration/interpreter.rs` (верификация значений runtime вывода)
- [ ] Переписать `tests/integration/execution.rs` (исправить stack overflow)
- [ ] Добавить интеграционные codegen тесты (от исходника до IR)

### Phase 4: Объединение встроенных тестов

- [ ] Объединение тестов typecheck 23→12
- [ ] Удалить `debug_lexer.rs`
- [ ] Проверить дублирование в тестах parser

### Phase 5: Создание документа стандартов тестирования

- [ ] Создать `TEST_STANDARDS.md` в корне `tests/yaoxiang/`

---

## V. Способы верификации

```bash
# Все тесты
cargo test

# E2E тесты
cargo test --test yx_runner

# Модульные тесты
cargo test --lib

# Ручной запуск .yx файла
cargo run -- run tests/yaoxiang/01-basics/variables.yx

# Запуск benchmark
cargo bench
```

---

## VI. Список затрагиваемых файлов

### Новые файлы
- `tests/yx_runner.rs` — E2E тестовый runner
- `tests/yaoxiang/TEST_STANDARDS.md` — Стандарты тестирования
- `tests/yaoxiang/00-smoke/hello.yx`
- `tests/yaoxiang/01-basics/variables.yx`
- `tests/yaoxiang/01-basics/typed_vars.yx`
- `tests/yaoxiang/01-basics/operators.yx`
- `tests/yaoxiang/01-basics/literals.yx`
- `tests/yaoxiang/01-basics/comments.yx`
- `tests/yaoxiang/02-functions/definitions.yx`
- `tests/yaoxiang/02-functions/lambdas.yx`
- `tests/yaoxiang/02-functions/closures.yx`
- `tests/yaoxiang/03-control-flow/if_else.yx`
- `tests/yaoxiang/03-control-flow/while.yx`
- `tests/yaoxiang/03-control-flow/for.yx`
- `tests/yaoxiang/03-control-flow/match.yx`
- `tests/yaoxiang/05-data-structures/lists.yx`
- `tests/yaoxiang/05-data-structures/tuples.yx`
- `tests/yaoxiang/06-modules/imports.yx`
- `tests/yaoxiang/06-modules/lib/math.yx`

### Удаляемые файлы
- `tests/yaoxiang/closure_test.yx`
- `tests/yaoxiang/closure_test2.yx`
- `tests/yaoxiang/list_test.yx`
- `tests/yaoxiang/mut_param_test.yx`
- `tests/yaoxiang/mut_param_error_test.yx`
- `tests/yaoxiang/impl_status_test.yx`
- `tests/yaoxiang/spec_basics_test.yx`
- `tests/yaoxiang/spec_features_test.yx`
- `tests/yaoxiang/spec_functions_test.yx`
- `tests/yaoxiang/spec_types_test.yx`
- `src/frontend/core/lexer/tests/debug_lexer.rs` (待确认)

### Изменяемые файлы
- `tests/integration/interpreter.rs` — переписать
- `tests/integration/execution.rs` — переписать
- `src/frontend/core/ir_gen.rs` — исправить match и listcomp
- `src/frontend/typecheck/` — исправить `x: Int = 42`
- `src/frontend/typecheck/tests/` — объединить 23→12 файлов
```