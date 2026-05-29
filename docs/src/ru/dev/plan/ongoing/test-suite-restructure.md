# План рефакторинга тестовой системы

> Статус: в планировании
> Ветка: refactor/test-suite
> Дата: 2026-05-10

## I. Зачем нужен рефакторинг

### Текущие проблемы

1. **1752 теста проходят успешно, но не обнаруживают реальные баги**
   - Выражение match возвращает 0 (ir_gen не обрабатывает узел Match)
   - List comprehension возвращает 0 (ir_gen не обрабатывает узел ListComp)
   - Объявление переменной с аннотацией типа `x: Int = 42` не парсится

2. **Интеграционные тесты проверяют только успешную компиляцию, не верифицируют корректность вывода в runtime**
   - `tests/integration/interpreter.rs` только `assert!(result.is_ok())`
   - `tests/integration/execution.rs` полностью закомментирован

3. **E2E .yx файлы без системы**
   - Смесь старого и нового: `closure_test.yx` (старый) и `spec_features_test.yx` (новый) в одном каталоге
   - Беспорядочное именование: `closure_test.yx`, `closure_test2.yx`, `mut_param_test.yx`
   - Нет плана покрытия: нет маппинга на главы спецификации языка

4. **Фрагментация inline-тестов**
   - В `src/frontend/typecheck/tests/` 23 файла, многие тестируют одно и то же
   - Тесты scope разбросаны по 4 файлам
   - Тесты infer разбросаны по 3 файлам
   - `typecheck_fixes.rs` —疑似 исторический патч-артефакт

5. **Codegen тесты изолированы**
   - Все IR ручные, не проходят полный pipeline parser→typecheck→ir_gen
   - Тестируется «переводится ли рукописный IR в байткод», а не «корректен ли результат компиляции исходника»

### Цели рефакторинга

1. **Создать трёхуровневую тестовую систему** с чётким разделением ответственности и стандартами покрытия
2. **E2E тесты также служат benchmark** — каждый .yx файл может измерять время выполнения
3. **Унифицировать внутренние тесты** — единые соглашения, именование, паттерны assertion
4. **Покрыть ключевые пути спецификации языка** — 确保语言规范中定义的语法特性有对应的测试

---

## II. Трёхуровневая тестовая система

### Первый уровень: E2E .yx тестовый набор (tests/yaoxiang/)

Организация по главам спецификации языка, каждый файл соответствует одной синтаксической特性.

```
tests/yaoxiang/
├── 00-smoke/                 # Дымовые тесты
│   └── hello.yx
│
├── 01-basics/                # Базовая грамматика (главы 2/4/5 спецификации)
│   ├── variables.yx          # Объявление переменных + type inference
│   ├── typed_vars.yx         # Аннотированные переменные x: Int = 42
│   ├── operators.yx          # Все операторы
│   ├── literals.yx           # Все 字面量
│   └── comments.yx           # Комментарии
│
├── 02-functions/             # Функции (глава 6 спецификации)
│   ├── definitions.yx        # name: (params) -> Ret = ...
│   ├── lambdas.yx            # Lambda-выражения
│   ├── closures.yx           # Замыкания
│   └── generics.yx           # Обобщённые функции
│
├── 03-control-flow/          # Поток управления (главы 4/5 спецификации)
│   ├── if_else.yx
│   ├── while.yx
│   ├── for.yx
│   ├── match.yx
│   └── list_comp.yx          # List comprehension
│
├── 04-types/                 # Типовая система (глава 3 спецификации)
│   ├── structs.yx            # Point: Type = { x: Float, y: Float }
│   ├── enums.yx              # Color: Type = { red | green | blue }
│   └── generics.yx           # Option: (T: Type) -> Type = ...
│
├── 05-data-structures/       # Коллекции (секция 2.6 спецификации)
│   ├── lists.yx
│   ├── tuples.yx
│   └── dicts.yx
│
├── 06-modules/               # Модульная система (глава 7 спецификации)
│   ├── imports.yx
│   └── lib/
│
└── 07-errors/                # Обработка ошибок (глава 9 спецификации, помечены нереализованные特性)
    ├── result.yx
    └── option.yx
```

**Спецификация файлов**:

```yaoxiang
// 01-basics/variables.yx
// 覆盖: 规范 §5.2 Переменные, §6.2 Type inference
// 验证: Базовые объявления, type inference, изменчивость
// 分支: refactor/test-suite
// 状态: ✅ Работает

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

**Механизм assertion**: Тестовый фреймворк Rust перехватывает stdout, верифицирует появление строки `ALL TESTS PASSED` в выводе каждого .yx файла.

**Расширение для benchmark**: Файлы .yx естественно служат перфоманс-бенчмарком — измеряют время каждого прогона. В будущем можно обернуть в `criterion` для отслеживания перфоманс-регрессий.

### Второй уровень: Интеграционные тесты (tests/integration/)

Тестируют полный pipeline компиляции+исполнения, верифицируют выходные значения.

| Текущий файл | Операция | Пояснение |
|-------------|----------|----------|
| `interpreter.rs` | Переписать | Компилировать исходник → 执行 → Assert выходных значений |
| `execution.rs` | Переписать (раскомментировать) | Исправить stack overflow, запускать реальные .yx файлы |
| `codegen.rs` | Сохранить | Сериализация/десериализация байткода |
| `codegen_extended.rs` | Сохранить | Тесты opcode/metadata |
| `fstring.rs` | Сохранить | Дополнить 执行验证 |
| `backends.rs` | Сохранить | Тесты типа RuntimeValue |

**Дополнение**: `tests/yx_runner.rs` — автоматически 发现 и запускает все .yx файлы из `tests/yaoxiang/`.

### Третий уровень: Модульные тесты (src/*/tests/)

Тестируют внутреннюю логику отдельных модулей, имеют доступ к приватному API.

#### 3.1 Тесты Lexer (src/frontend/core/lexer/tests/)

11 файлов → 删除 1 отладочный, оставить 10.

| Операция | Файл |
|---------|------|
| 删除 | `debug_lexer.rs` — только для отладки |
| Оставить | `basic.rs`, `comments.rs`, `keywords.rs`, `literals.rs`, `operators.rs` |
| Оставить | `delimiters.rs`, `errors.rs`, `fstring.rs` |
| Оставить | `rfc004_lexer.rs`, `rfc010_lexer.rs` |

#### 3.2 Тесты Parser (src/frontend/core/parser/tests/)

13 файлов → после ревизии мелкие правки.

| Операция | Файл |
|---------|------|
| Оставить | `basic.rs`, `fn_def.rs`, `syntax_validation.rs`, `old_syntax_rejection.rs` |
| Оставить | `boundary.rs`, `concurrency.rs`, `fstring.rs` |
| Оставить | `ref_test.rs`, `unsafe_ptr.rs`, `state.rs` |
| Ревизия | `binding_enhancements.rs` — проверить на дубли с fn_def |

#### 3.3 Тесты Typecheck (src/frontend/typecheck/tests/)

**Самая проблемная зона**: 23 файла → объединить в 12.

| Операция | Исходный файл | Целевой файл |
|---------|--------------|-------------|
| Объединить | `infer.rs` + `inference.rs` + `types.rs` | `type_inference.rs` |
| Объединить | `scope.rs` + `shadowing.rs` + `use_scope.rs` + `use_block_scope.rs` | `scoping.rs` |
| Объединить | `visibility.rs` + `pub_bind.rs` | `visibility.rs` |
| Ревизия | `typecheck_fixes.rs` | Если это исторические патч-тесты — объединить в соответствующие файлы и 删除 |
| Оставить | `basic.rs`, `check.rs` | — |
| Оставить | `constraint.rs`, `concurrency.rs`, `fstring.rs` | — |
| Оставить | `gat.rs`, `ref_test.rs`, `result_try.rs` | — |
| Оставить | `semantic_tokens.rs`, `traits.rs`, `type_constructor_rules.rs` | — |

#### 3.4 Тесты Middle/Codegen (src/middle/passes/tests/)

| Директория | Операция |
|-----------|---------|
| `codegen/` | Оставить существующие, **дополнить интеграционные codegen тесты** (компиляция из исходника в верификацию IR) |
| `lifetime/` | Оставить как есть |
| `mono/` | Оставить как есть |
| `module/` | Оставить как есть |

## III. Документ стандартов тестирования

В той же директории создать `TEST_STANDARD.md`, содержание:

### Соглашения об именовании

```
Назначение    Паттерн                   Пример
─────────────────────────────────────────────────────
Тестовый модуль  mod_<описание>_tests      mod_parser_basic_tests
Тестовая функция  test_<фича>_<сценарий>   test_parse_fn_def_no_params
E2E файл     <секция>-<фича>.yx       01-basics-variables.yx
```

### Соглашения об assertion

- E2E `.yx` файлы: выводить `ALL TESTS PASSED` в конце
- Интеграционные тесты: верифицировать что stdout содержит ожидаемые значения
- Модульные тесты: верифицировать значения полей структур данных, не использовать `assert!(result.is_ok())` как единственный assertion

### Соглашения о комментариях

```
// Заголовок E2E файла：
// 覆盖: 规范 §X.X
// 验证: Краткое описание
// 分支: refactor/test-suite
// 状态: ✅ Работает / ⚠️ Требует исправления / 🔴 Не реализовано
```

### Обработка нереализованных特性

- E2E `.yx` для несуществующих возможностей: не писать тесты, добавить после реализации
- В модульных тестах, ссылающихся на нереализованные возможности: пометить `#[ignore]`, в комментарии написать "待 XXX 实现后启用"

---

## IV. План выполнения

### Phase 0: Подготовительная работа

- [ ] Создать ветку `refactor/test-suite` от `dev`
- [ ] Ревизия `typecheck_fixes.rs` и `binding_enhancements.rs`, 确定是否删除
- [ ] Ревизия `tests/integration/execution.rs` — проблема stack overflow

### Phase 1: Фреймворк E2E тестов

- [ ] Создать `tests/yx_runner.rs` — автоматически 发现 и запускать `tests/yaoxiang/**/*.yx`
- [ ] Создать новую структуру директории `tests/yaoxiang/`
- [ ] Написать 00-smoke дымовые тесты
- [ ] Написать слой 01-basics (текущая рабочая грамматика)
- [ ] Написать слой 02-functions

### Phase 2: Исправление runtime багов + соответствующие тесты

- [ ] Исправить match выражение (добавить Match обработку в ir_gen)
- [ ] Исправить list comprehension (добавить ListComp обработку в ir_gen)
- [ ] Исправить аннотацию типа переменной `x: Int = 42`
- [ ] Добавить соответствующие .yx E2E тесты для перечисленных исправлений

### Phase 3: Переписать интеграционные тесты

- [ ] Переписать `tests/integration/interpreter.rs` (верификация выходных значений)
- [ ] Переписать `tests/integration/execution.rs` (исправить stack overflow)
- [ ] Дополнить интеграционные codegen тесты (от исходника к IR)

### Phase 4: Объединение inline-тестов

- [ ] Объединить typecheck тесты 23→12
- [ ] 删除 `debug_lexer.rs`
- [ ] Ревизия дублирующих parser тестов

### Phase 5: Создать документ стандартов тестирования

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

## VI. Список затронутых файлов

### Новые файлы
- `tests/yx_runner.rs` — E2E test runner
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

### Файлы для 删除
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

### Файлы для 修改
- `tests/integration/interpreter.rs` — переписать
- `tests/integration/execution.rs` — переписать
- `src/frontend/core/ir_gen.rs` — исправить match и listcomp
- `src/frontend/typecheck/` — исправить `x: Int = 42`
- `src/frontend/typecheck/tests/` — объединить 23→12 файлов