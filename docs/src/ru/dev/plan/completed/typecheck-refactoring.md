# План полной переработки процесса проверки типов

> **Статус**: ✅ Завершено  
> **Дата завершения**: Июль 2025  
> **Результаты тестирования**: все 1469 тестов пройдены (1434 + 30 + 5), 0 неудач

## Основные цели

Полностью устранить технический долг, сделать процесс проверки типов четким, лаконичным и легко расширяемым, сохраняя при этом хорошее взаимодействие с существующими модулями特性.

---

## Анализ структуры существующих модулей (до переработки)

> Ниже представлена структура до переработки, только для справки.

```
src/frontend/typecheck/
├── mod.rs                      # Единая точка входа
├── checking/                   # ❌ Проблема: перекрывается с inference
│   ├── mod.rs                 # BodyChecker, AssignmentChecker, SubtypeChecker...
│   ├── assignment.rs
│   ├── bounds.rs
│   ├── compatibility.rs
│   └── subtyping.rs
├── inference/                  # ❌ Проблема: перекрывается с checking
│   ├── mod.rs
│   ├── expressions.rs          # ExprInferrer
│   ├── generics.rs
│   ├── patterns.rs
│   └── statements.rs
├── specialization/             # ✅ Сохраняется (независимая特性)
├── traits/                     # ✅ Сохраняется (независимая特性)
├── gat/                        # ✅ Сохраняется (независимая特性)
├── tests/                      # ✅ Сохраняется
├── overload.rs                 # ✅ Сохраняется (независимая特性)
├── type_eval.rs                # ✅ Сохраняется (независимая特性)
└── specialize.rs              # ✅ Сохраняется (для совместимости)
```

**Проблема**: `checking/` и `inference/` по сути делают одно и то же, но разделены на два каталога!

---

## План переработки: объединение checking/ в inference/

### Структура каталогов (после переработки)

```
src/frontend/typecheck/
├── mod.rs                  # Единая точка входа, экспорт всех модулей
│
# ✅ Объединенные основные модули inference/
├── inference/
│   ├── mod.rs             # Экспорт + TypeChecker основная точка входа
│   ├── scope.rs           # 🆕 Единое управление областями видимости
│   ├── types.rs           # 🆕 Утилиты системы типов
│   ├── statements.rs      # 🆕 Проверка операторов (объединение checking + inference)
│   ├── expressions.rs     # 🆕 Вывод типов выражений (объединение существующего expressions.rs)
│   #
│   # ✅ Перемещено из checking/
│   ├── assignment.rs      # Проверка присваивания
│   ├── subtyping.rs       # Проверка подтипов
│   ├── compatibility.rs   # Проверка совместимости
│   ├── bounds.rs          # Проверка границ
│   #
│   # ✅ Сохранено (улучшено)
│   ├── generics.rs        # Вывод generic-ов
│   └── patterns.rs        # Вывод типов для шаблонов
│
# ✅ Сохранено: независимые特性 модули (без изменений, вызываются через интерфейс)
├── specialization/         # Логика специализации
├── traits/                # Логика trait
├── gat/                   # Логика GAT
├── overload.rs            # Разрешение перегрузки
├── type_eval.rs           # Вычисление типов
├── specialize.rs          # Совместимость
│
# ❌ Удален каталог checking/
└── tests/                  # Тесты
```

### Разделение ответственности модулей

| Модуль | Ответственность | Описание |
|--------|----------------|----------|
| `inference/scope.rs` | Единое управление областями видимости переменных | Все операции добавления/удаления/изменения переменных |
| `inference/types.rs` | Утилиты системы типов | unify, infer_element_type и др. |
| `inference/statements.rs` | Проверка операторов | Var, Fn, For, If, Expr и др. операторы |
| `inference/expressions.rs` | Вывод типов выражений | Lit, Var, BinOp, Call, For и др. выражения |
| `inference/assignment.rs` | Проверка присваивания | Перемещено из checking/ |
| `inference/subtyping.rs` | Проверка подтипов | Перемещено из checking/ |
| `inference/compatibility.rs` | Проверка совместимости | Перемещено из checking/ |
| `inference/bounds.rs` | Проверка границ | Перемещено из checking/ |
| `specialization/*` | Специализация | Независимый плагин |
| `traits/*` | Trait | Независимый плагин |
| `gat/*` | GAT | Независимый плагин |
| `overload.rs` | Разрешение перегрузки | Независимый плагин |

### Ключевые принципы проектирования

1. **Единая точка входа**: `inference/` — единственная точка входа для проверки типов
2. **Единственный экземпляр ScopeManager**: весь процесс проверки использует общий ScopeManager
3. **Независимые特性 модули**: specialization/traits/gat/overload работают как плагины
4. **Без дублирования кода**: удалены дублирующиеся scopes из BodyChecker и ExprInferrer

---

## Детальное проектирование

### inference/scope.rs - Единое управление областями видимости

```rust
/// Менеджер областей видимости
/// Единственная ответственность: управление стеком областей видимости переменных
pub struct ScopeManager {
    scopes: Vec<HashMap<String, PolyType>>,
}

impl ScopeManager {
    pub fn new() -> Self
    pub fn enter_scope(&mut self)
    pub fn exit_scope(&mut self)
    pub fn add_var(&mut self, name: String, poly: PolyType)
    pub fn get_var(&self, name: &str) -> Option<&PolyType>
    pub fn update_var(&mut self, name: &str, poly: PolyType)
    pub fn var_in_current_scope(&self, name: &str) -> bool
    pub fn var_in_any_scope(&self, name: &str) -> bool
}
```

### inference/types.rs - Утилиты системы типов

```rust
/// Утилиты системы типов
pub struct TypeSystem;

impl TypeSystem {
    /// Унификация двух типов
    pub fn unify(ty1: &MonoType, ty2: &MonoType, solver: &mut TypeConstraintSolver) -> Result<(), Box<Diagnostic>>

    /// Вывод типа элемента из итерируемого типа
    pub fn infer_element_type(iter_ty: &MonoType) -> MonoType

    /// Создание типа списка
    pub fn make_list_type(elem_ty: MonoType) -> MonoType

    /// Проверка, является ли тип итерируемым
    pub fn is_iterable(ty: &MonoType) -> bool

    /// Проверка ограничений trait с помощью特性 модуля
    pub fn check_trait_bounds(ty: &MonoType, bounds: &[TraitBound], trait_table: &TraitTable) -> Result<(), Box<Diagnostic>>

    /// Выполнение создания экземпляра с помощью特性 модуля
    pub fn instantiate(ty: &MonoType, args: &[MonoType]) -> Result<MonoType, Box<Diagnostic>>
}
```

### inference/statements.rs - Проверка операторов

```rust
use crate::inference::scope::ScopeManager;
use crate::inference::types::TypeSystem;
use crate::inference::assignment::AssignmentChecker;
use crate::inference::subtyping::SubtypeChecker;

/// Проверщик операторов
pub struct StatementChecker<'a> {
    scope: &'a mut ScopeManager,
    solver: &'a mut TypeConstraintSolver,
    type_system: &'a TypeSystem,
}

impl<'a> StatementChecker<'a> {
    pub fn new(scope: &'a mut ScopeManager, solver: &'a mut TypeConstraintSolver) -> Self

    pub fn check(&mut self, stmt: &Stmt) -> Result<(), Box<Diagnostic>> {
        match &stmt.kind {
            StmtKind::Var { .. } => self.check_var(),
            StmtKind::Fn { .. } => self.check_fn(),
            StmtKind::For { .. } => self.check_for(),
            StmtKind::If { .. } => self.check_if(),
            StmtKind::Expr { .. } => self.check_expr_stmt(),
            // ...
        }
    }

    fn check_var(&mut self, name: &str, init: Option<&Expr>, annot: Option<&Type>) -> Result<(), Box<Diagnostic>>
    fn check_fn(&mut self, ...) -> Result<(), Box<Diagnostic>>
    fn check_for(&mut self, ...) -> Result<(), Box<Diagnostic>>
}
```

### inference/expressions.rs - Вывод типов выражений

```rust
use crate::inference::scope::ScopeManager;
use crate::inference::types::TypeSystem;

/// Выводчик типов выражений (использует общий ScopeManager)
pub struct ExpressionInferrer<'a> {
    scope: &'a ScopeManager,  // Ссылка только для чтения
    solver: &'a mut TypeConstraintSolver,
    type_system: &'a TypeSystem,
}

impl<'a> ExpressionInferrer<'a> {
    pub fn infer(&mut self, expr: &Expr) -> Result<MonoType, Box<Diagnostic>> {
        match expr {
            Expr::Lit(..) => self.infer_literal(),
            Expr::Var(..) => self.infer_var(),
            Expr::BinOp(..) => self.infer_binop(),
            Expr::Call(..) => self.infer_call(),
            Expr::For(..) => self.infer_for(),
            Expr::Lambda(..) => self.infer_lambda(),
            // ...
        }
    }

    fn infer_literal(&mut self, lit: &Literal) -> Result<MonoType, Box<Diagnostic>>
    fn infer_var(&mut self, name: &str, span: Span) -> Result<MonoType, Box<Diagnostic>>
    fn infer_binop(&mut self, op: &BinOp, left: &Expr, right: &Expr) -> Result<MonoType, Box<Diagnostic>>
}
```

### inference/mod.rs - Единая точка входа

```rust
// Экспорт всех модулей
pub mod scope;
pub mod types;
pub mod statements;
pub mod expressions;
pub mod assignment;
pub mod subtyping;
pub mod compatibility;
pub mod bounds;
pub mod generics;
pub mod patterns;

pub use scope::ScopeManager;
pub use types::TypeSystem;
pub use statements::StatementChecker;
pub use expressions::ExpressionInferrer;
pub use assignment::AssignmentChecker;
pub use subtyping::SubtypeChecker;
pub use compatibility::CompatibilityChecker;
pub use bounds::BoundsChecker;

// Единая точка входа для проверки типов
pub struct TypeChecker {
    scope: ScopeManager,
    solver: TypeConstraintSolver,
    type_system: TypeSystem,
    // Ссылки на特性 модули
    trait_table: TraitTable,
    specialization_context: SpecializationContext,
}

impl TypeChecker {
    pub fn new() -> Self

    pub fn check_module(&mut self, module: &Module) -> Result<TypeCheckResult, Vec<Diagnostic>> {
        // 1. Сбор определений типов
        // 2. Сбор сигнатур функций
        // 3. Проверка всех операторов
        // 4. Решение ограничений
    }
}
```

---

## Этапы переработки

### Этап 1: Создание scope.rs и types.rs ✅

**Цель**: Создание базовых модулей

**Результаты**:
- ✅ `inference/scope.rs` - ScopeManager (с enter_scope/exit_scope/add_var/get_var/update_var/var_in_current_scope/var_in_any_scope/vars/scope_level)
- ✅ `inference/types.rs` - TypeSystem (с unify/infer_element_type/make_list_type/is_iterable)

### Этап 2: Создание statements.rs ✅

**Цель**: Объединение логики проверки операторов из BodyChecker + StmtInferrer

**Результаты**:
- ✅ `inference/statements.rs` - StatementChecker (861 строка, полная логика проверки операторов)

**Детали реализации**:
- StatementChecker владеет `scope: ScopeManager` и `solver: TypeConstraintSolver`
- `check_expr()` передает `&mut self.scope` и `&mut self.solver` в ExpressionInferrer через частичное заимствование в Rust, устраняя копирование переменных
- Сохранены псевдонимы для обратной совместимости: `pub type BodyChecker = StatementChecker;`

### Этап 3: Создание expressions.rs ✅

**Цель**: Объединение логики вывода типов выражений из ExprInferrer

**Результаты**:
- ✅ `inference/expressions.rs` - ExpressionInferrer (897 строк, использует общий ScopeManager)

**Детали реализации**:
- ExpressionInferrer заимствует `scope: &'a mut ScopeManager` и `solver: &'a mut TypeConstraintSolver`
- Сигнатура конструктора: `new(scope, solver, overload_candidates)` / `with_native_signatures(scope, solver, overloads, natives)`
- Сохранены псевдонимы для обратной совместимости: `pub type ExprInferrer<'a> = ExpressionInferrer<'a>;`

### Этап 4: Перемещение файлов из checking/ в inference/ ✅

**Цель**: Объединение checking/ в inference/

**Перемещение**:
- ✅ `checking/assignment.rs` → `inference/assignment.rs`
- ✅ `checking/subtyping.rs` → `inference/subtyping.rs`
- ✅ `checking/compatibility.rs` → `inference/compatibility.rs`
- ✅ `checking/bounds.rs` → `inference/bounds.rs`

### Этап 5: Изменение точки входа mod.rs ✅

**Файл**: `src/frontend/typecheck/mod.rs`

**Изменения**:
- ✅ Удален `pub mod checking;`
- ✅ Обновлен `pub use inference::*;` для экспорта
- ✅ Обновлен `infer_expression()` для использования ScopeManager + ExpressionInferrer
- ✅ Обновлены ссылки TypeChecker на `inference::StatementChecker`

### Этап 6: Удаление старого кода и каталогов ✅

**Удаление**:
- ✅ Каталог `checking/` полностью удален
- ✅ Старый код BodyChecker заменен на StatementChecker
- ✅ ExprInferrer.scopes заменен на общий ScopeManager

### Этап 7: Регрессионное тестирование ✅

```bash
cargo test
# test result: ok. 1434 passed; 0 failed; 4 ignored
# test result: ok. 30 passed; 0 failed
# test result: ok. 5 passed; 0 failed; 11 ignored
```

**Обновления тестовых файлов**:
- ✅ `tests/shadowing.rs` - обновлены пути импорта BodyChecker, добавлен параметр ScopeManager в ExprInferrer
- ✅ `tests/scope.rs` - добавлен параметр ScopeManager в ExprInferrer
- ✅ `tests/infer.rs` - 39 обновлений сигнатуры ExprInferrer, тесты StmtInferrer переписаны на StatementChecker
- ✅ `tests/constraint.rs` - 6 обновлений путей импорта с checking:: на inference::
- ✅ `tests/basic.rs` - 18 обновлений сигнатуры ExprInferrer

---

## Код, требующий очистки

### 1. BodyChecker → statements.rs

| Исходное расположение | Целевое расположение |
|--------|--------|
| `checking/mod.rs` - `BodyChecker` | `inference/statements.rs` - `StatementChecker` |
| `check_stmt`, `check_var_stmt` и др. | `StatementChecker::check_*` |

### 2. ExprInferrer → expressions.rs

| Исходное расположение | Целевое расположение |
|--------|--------|
| `inference/expressions.rs` - `ExprInferrer` | `inference/expressions.rs` - `ExpressionInferrer` |
| Поле `scopes` | Использование `ScopeManager` |

### 3. checking/ → inference/

| Исходное расположение | Целевое расположение |
|--------|--------|
| `checking/assignment.rs` | `inference/assignment.rs` |
| `checking/subtyping.rs` | `inference/subtyping.rs` |
| `checking/compatibility.rs` | `inference/compatibility.rs` |
| `checking/bounds.rs` | `inference/bounds.rs` |

### 4. Удаление

| Удаляемый элемент | Описание |
|--------|------|
| Каталог `checking/` | Полностью удален |
| Структура `BodyChecker` | Перенесена в StatementChecker |
| `ExprInferrer.scopes` | Заменено на ScopeManager |

---

## Дизайн расширяемости

### Добавление нового типа оператора

```rust
// inference/statements.rs
impl StatementChecker {
    pub fn check(&mut self, stmt: &Stmt) -> Result<(), Box<Diagnostic>> {
        match &stmt.kind {
            // ... существующие операторы
            StmtKind::Match { .. } => self.check_match(),  // 🆕
            StmtKind::While { .. } => self.check_while(),  // 🆕
        }
    }
}
```

### Добавление нового типа выражения

```rust
// inference/expressions.rs
impl ExpressionInferrer {
    pub fn infer(&mut self, expr: &Expr) -> Result<MonoType, Box<Diagnostic>> {
        match expr {
            // ... существующие выражения
            Expr::Macro { .. } => self.infer_macro(),  // 🆕
            Expr::Await { .. } => self.infer_await(),  // 🆕
        }
    }
}
```

---

## Критерии приемки

### Архитектурная приемка

- [x] `inference/scope.rs` самостоятельно отвечает за управление областями видимости
- [x] `inference/statements.rs` самостоятельно отвечает за проверку операторов
- [x] `inference/expressions.rs` самостоятельно отвечает за вывод типов выражений
- [x] `inference/types.rs` предоставляет утилиты системы типов
- [x] `inference/assignment.rs`, `subtyping.rs`, `compatibility.rs`, `bounds.rs` работают корректно
- [x]特性 модули (specialization/traits/gat/overload) остаются независимыми
- [x] Удален каталог `checking/`
- [x] Нет логики ручной синхронизации переменных (используется паттерн частичного заимствования общего ScopeManager в Rust)

### Функциональная приемка

| Тестовый случай | Ожидаемый результат |
|---------|--------|
| `nums = [1,2,3]; for n in nums { print(n) }` | Компиляция успешна |
| `x = 10; for i in 1..3 { x = i }` | Компиляция успешна |
| `entry: FileEntry = item` | Аннотации типов работают корректно |

### Регрессионное тестирование

```bash
cargo test
```

Ожидание: все тесты пройдены

---

## План тестирования

### Этап 1: Модульные тесты

| Название теста | Модуль |
|---------|------|
| test_enter_scope | scope.rs |
| test_exit_scope | scope.rs |
| test_add_var | scope.rs |
| test_get_var_outer | scope.rs |
| test_unify_int | types.rs |
| test_infer_element_type | types.rs |

### Этап 2: Интеграционные тесты

| Название теста | Тестовый код | Ожидаемый результат |
|---------|---------|---------|
| test_for_list | `for n in [1,2,3] { print(n) }` | Компиляция успешна |
| test_var_scope | Корректная область видимости переменных | Пройден |
| test_type_annotation | `x: Int = 1` | Компиляция успешна |
| test_generic_fn | Generic функции | Работают корректно |
| test_trait_bound | Ограничения trait | Работают корректно |

### Этап 3: Регрессионное тестирование

```bash
cargo test
```

Ожидание: все тесты пройдены