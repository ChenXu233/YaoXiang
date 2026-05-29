---
title: Проект реализации модели захвата замыканий
status: draft
created: 2026-05-29
---

# Проект реализации модели захвата замыканий

## Цель

Реализовать анализ захвата внешних переменных в замыканиях, выбор способа захвата и генерацию IR.

## Основные правила

```
Тип переменной    Замыкание экранирует    Способ захвата
─────────────────────────────────────────────────────────
Dup                Любой                   Копирование (нулевая стоимость, без побочных эффектов)
Не Dup             Не экранирует           Автоматическое заимствование (&T или &mut T токен)
Не Dup             Экранирует              Move (передача владения)
```

Эти правила — **та же самая логика**, что и автоматический выбор заимствования при вызове функций. Никаких новых концепций не вводится.

## Список задач

### Step 1: Анализ экранирования

**Файл**: `src/frontend/core/typecheck/inference/expressions.rs` (или создать новый `capture.rs`)

Определение «экранирования» замыкания:

```rust
enum ClosureUsage {
    Inline,    // Вызов на месте или передача синхронной функции, не экранирует
    Escaping,  // spawn, return, сохранение в кучу, сохранение в глобальную область
}
```

Правила определения экранирования:

```
lambda как параметр spawn { ... }             → Escaping
lambda как возвращаемое значение              → Escaping
lambda присваивается внешней переменной/полю → Escaping
lambda передаётся параметром функции (не spawn) → Inline (консервативно)
lambda вызывается на месте                    → Inline
```

**Консервативный принцип**: при невозможности определения — считать Escaping.

### Step 2: Анализ захватываемых переменных

**Обход AST тела замыкания**, поиск ссылок на переменные из внешней области видимости.

```rust
struct CaptureInfo {
    captures: Vec<CapturedVar>,
}

struct CapturedVar {
    name: String,           // Имя переменной
    usage: CaptureUsage,    // Способ использования
}

enum CaptureUsage {
    Read,           // Только чтение (достаточно &T)
    Write,          // Чтение и запись (нужен &mut T)
    Move,           // Передача владения (не Dup + экранирование)
    DupCopy,        // Тип Dup — прямое копирование
}
```

**Процесс анализа**:

1. Обход AST тела lambda
2. Запись всех ссылок `Expr::Var(name)`
3. Фильтрация: оставить только переменные из внешней области видимости замыкания
4. Классификация по способу использования:
   - Присваивание/вызов mut-метода → Write
   - Только чтение → Read
   - Перемещение в другое место → Move

### Step 3: Выбор способа захвата

```rust
fn determine_capture_mode(
    var: &CapturedVar,
    ty: &MonoType,
    usage: ClosureUsage,
    is_dup: bool,
) -> CaptureMode {
    match (is_dup, usage) {
        // Тип Dup: прямое копирование — простейший путь
        (true, _) => CaptureMode::Copy,
        
        // Не Dup + экранирование → Move
        (false, ClosureUsage::Escaping) => CaptureMode::Move,
        
        // Не Dup + не экранирует → автоматическое заимствование
        (false, ClosureUsage::Inline) => match var.usage {
            CaptureUsage::Read => CaptureMode::Borrow,     // &T
            CaptureUsage::Write => CaptureMode::BorrowMut, // &mut T
            CaptureUsage::Move => CaptureMode::Move,
            CaptureUsage::DupCopy => unreachable!(),
        },
    }
}

enum CaptureMode {
    Copy,       // Прямое копирование значения
    Borrow,     // &T токен
    BorrowMut,  // &mut T токен
    Move,       // Передача владения
}
```

**Ключевые сценарии**:

```yaoxiang
# 1. &T токен передаётся в замыкание — Dup → Copy, нулевая стоимость
threshold: &Float = &some_float
items.filter(|p| p.x > threshold)
# threshold: &Float → Dup → CaptureMode::Copy
# Компилятор: копирование токена (нулевой размер, нулевые накладные расходы)

# 2. Значение не Dup, замыкание не экранирует — автоматическое заимствование
buffer: Buffer = ...
process(|b| b.read())
# buffer не Dup, замыкание не экранирует → CaptureMode::Borrow
# Компилятор: автоматическое создание &Buffer токена для передачи в замыкание

# 3. Замыкание экранирует — Move
big_data: Data = ...
spawn { use(big_data) }
# big_data не Dup, spawn → Escaping → CaptureMode::Move
```

### Step 4: Генерация IR

**Файл**: `src/middle/core/ir_gen.rs`

```rust
// Текущее (пустая реализация)
Instruction::MakeClosure {
    dst: Operand::Local(result_reg),
    func: closure_name,
    env: Vec::new(),  // ← всегда пустой
}

// Изменить на
Instruction::MakeClosure {
    dst: Operand::Local(result_reg),
    func: closure_name,
    env: captured_vars,  // Vec<(Operand, CaptureMode)>
}
```

Генерация IR для каждой захваченной переменной:

```rust
for captured in &captures {
    let src = self.lookup_local(&captured.name);
    match captured.mode {
        CaptureMode::Copy => {
            // Тип Dup: инструкция Mov для копирования (оптимизация нулевой стоимости — Step 5)
            instructions.push(Instruction::Move {
                dst: new_temp(),
                src,
            });
        }
        CaptureMode::Borrow => {
            // Автоматическое заимствование: создание ReadToken
            instructions.push(Instruction::Borrow {
                dst: new_temp(),
                src,
                mutable: false,
            });
        }
        CaptureMode::BorrowMut => {
            instructions.push(Instruction::Borrow {
                dst: new_temp(),
                src,
                mutable: true,
            });
        }
        CaptureMode::Move => {
            // Move: передача владения
            instructions.push(Instruction::Move {
                dst: new_temp(),
                src,
            });
        }
    }
}
```

### Step 5: Оптимизация ZST — устранение токенов

Когда `CaptureMode::Copy` используется для `&T`, тип `&T` является типом нулевого размера. Инструкция `Instruction::Move` копирует 0 байт данных → **необходимо устранить в оптимизирующем проходе IR**.

Два варианта реализации:

**Вариант A: Пропуск при генерации IR**
```rust
CaptureMode::Copy if is_zero_sized_type(ty) => {
    // Не генерируется никаких IR-инструкций
    // Тело замыкания напрямую ссылается на внешнюю переменную (на этапе компиляции)
}
```

**Вариант B: Оптимизирующий проход IR**
```rust
// Новый проход ZstElimination:
// Сканировать все Move dst, src; если тип src — ZST, удалить инструкцию
// dst заменить на src (алиас)
```

**Рекомендуется вариант A** — на этапе генерации уже известно, что это ZST, не требуется последующая оптимизация.

### Step 6: Обнаружение конфликтов токенов заимствования

После захвата замыканием токена `&mut T` оригинальная область видимости не может одновременно использовать этот токен:

```yaoxiang
tok = &mut point        # Создание WriteToken
closure = |x| {
    tok.shift(x, 0.0)   # tok захвачен замыканием
}
tok.shift(1.0, 0.0)     # ❌ Ошибка компиляции: WriteToken для tok уже удерживается замыканием
```

Это покрывается существующим обнаружением конфликтов токенов (RFC-009 v9, раздел 2.6) — borrow checker обрабатывает это при flow-sensitive анализе активности.

## Список изменений файлов

| # | Файл | Изменения |
|---|------|-----------|
| 1 | `typecheck/inference/capture.rs` (новый) | Анализ захвата + анализ экранирования + выбор模式 |
| 2 | `typecheck/inference/expressions.rs` | Вызов анализа захвата при выводе типа lambda |
| 3 | `middle/core/ir_gen.rs` | Заполнение MakeClosure env, пропуск ZST |
| 4 | `middle/core/ir.rs` | Возможно, нужна инструкция Borrow (если требуется в IR) |
| 5 | `middle/passes/lifetime/mod.rs` | Регистрация проверок заимствования для замыканий (если есть новые проверки) |

Общий объём изменений: ~300 строк.

## Порядок реализации

1. **Анализ захвата** (capture.rs) — чистый обход AST, возврат списка захваченных переменных
2. **Анализ экранирования** — определение, экранирует ли замыкание
3. **Выбор模式** — решение CaptureMode на основе Dup/не Dup + экранирование/не экранирование
4. **Генерация IR** — заполнение MakeClosure env
5. **Оптимизация ZST** — пропуск IR-инструкций для Dup + ZST

1-3 — слой чистой проверки типов (фронтенд). 4-5 — слой генерации IR (middle-end). Можно реализовать раздельно.

## Сценарии верификации

```yaoxiang
# ✅ Сценарий 1: Копирование Dup токена (наиболее важный сценарий)
filter_by: (items: List(Point), threshold: &Float) -> List(Point) = {
    items.filter(|p| p.x > threshold)
}

# ✅ Сценарий 2: Автоматическое заимствование для не Dup
process_buffer: (buf: Buffer) -> Void = {
    transform(|b| b.read())  # buf не экранирует → &T заимствование
}

# ✅ Сценарий 3: Принудительный Move между задачами
spawn_worker: (data: Data) -> Void = {
    spawn { use(data) }  # Экранирует → Move
}

# ❌ Сценарий 4: Конфликт заимствования с последующим использованием
bad: (buf: Buffer) -> Void = {
    closure = |b| b.write()
    buf.read()  # ❌ buf уже захвачен замыканием
}
```

## Ссылки

- [RFC-009 v9 Модель владения](../../design/rfc/accepted/009-ownership-model.md) — система токенов заимствования
- [RFC-007 Унификация синтаксиса функций](../../design/rfc/accepted/007-function-syntax-unification.md) — определение lambda
- Отчёт о пробелах: дефицит генерации IR (пустая реализация MakeClosure env)