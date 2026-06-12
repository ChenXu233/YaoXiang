```markdown
---
title: "RFC-014c: Поддержка рабочих пространств"
status: "Черновик"
author: "晨煦"
created: "2026-06-11"
updated: "2026-06-11"
group: "rfc-014"
---

# RFC-014c: Поддержка рабочих пространств

> Данный RFC является под-RFC к [RFC-014: Проектирование системы управления пакетами](../accepted/014-package-manager.md).

## Резюме

Определяет механизм рабочих пространств (workspace) в YaoXiang: совместное использование зависимостей, ссылок на пути, единый lockfile, интеграция с Cargo workspace при совместной разработке нескольких связанных пакетов.

## Мотивация

По мере роста проекта код необходимо разбивать на несколько пакетов. Этим пакетам требуется:
- взаимное ссылочное отношение (зависимости по пути)
- совместное использование версий внешних зависимостей (во избежание дрейфа версий)
- единый lockfile (для гарантии согласованности сборки)
- взаимодействие с Cargo workspace (для FFI-частей)

### Текущие проблемы

- Каждый проект независимо управляет зависимостями, совместное использование невозможно
- Отсутствует механизм автоматической подстановки при публикации для зависимостей по пути
- Отсутствует интеграция с Cargo workspace

## Предложение

### Основной замысел: координирующий слой + самодостаточные члены

Корневое workspace выполняет только координацию; каждый член полностью самодостаточен.

### Корневой yaoxiang.toml

```toml
# Корневой yaoxiang.toml
[workspace.members]
core = "packages/core/yaoxiang.toml"
utils = "packages/utils/yaoxiang.toml"
app = "packages/app/yaoxiang.toml"
```

**Корневой toml выполняет только три задачи:**
1. Объявляет список членов (в форме словаря, где ключ — имя члена, значение — путь к toml)
2. Предоставляет общий lockfile (`yaoxiang.lock`)
3. Предоставляет общий каталог vendor (`.yaoxiang/vendor/`)

**Корневой toml не определяет dependencies.** Зависимости каждого члена записываются в его собственном `yaoxiang.toml`.

### yaoxiang.toml члена

```toml
# packages/core/yaoxiang.toml
[package]
name = "core"
version = "0.1.0"

[dependencies]
json = "^2.0.0"
utils = { workspace = "utils" }    # Ссылка на члена рабочего пространства
regex = "^1.0.0"
```

```toml
# packages/utils/yaoxiang.toml
[package]
name = "utils"
version = "0.2.0"

[dependencies]
regex = "^1.0.0"
```

### Структура рабочего пространства

```
my-workspace/
├── yaoxiang.toml              # Корневая конфигурация рабочего пространства
├── yaoxiang.lock              # Общий lockfile
├── .yaoxiang/
│   └── vendor/                # Общий каталог vendor
├── packages/
│   ├── core/
│   │   ├── yaoxiang.toml      # Конфигурация пакета-члена
│   │   └── src/lib.yx
│   ├── utils/
│   │   ├── yaoxiang.toml
│   │   └── src/lib.yx
│   └── app/
│       ├── yaoxiang.toml
│       └── src/main.yx
└── Cargo.toml                 # Опционально: общее Cargo workspace (для FFI)
```

### Разрешение зависимостей

- Каждый член читает собственный `[dependencies]`
- При разрешении зависимости всех членов объединяются и порождают единый общий lockfile
- Конфликты версий порождают ошибку при генерации lockfile
- Один и тот же пакет в разных членах должен разрешаться в одну и ту же версию

### Зависимости по пути и публикация

При разработке используется ссылка через рабочее пространство:

```toml
[dependencies]
utils = { workspace = "utils" }
```

При публикации автоматически заменяется на версионную зависимость:

```toml
[dependencies]
utils = "^0.2.0"
```

Менеджер пакетов автоматически выполняет эту подстановку при `yaoxiang publish`.

### Интеграция с Cargo Workspace

Если в рабочем пространстве присутствуют FFI-пакеты, можно параллельно определить Cargo workspace:

```toml
# Корневой Cargo.toml
[workspace]
members = ["packages/core/native", "packages/utils/native"]
```

```
my-workspace/
├── yaoxiang.toml          # YaoXiang workspace
├── Cargo.toml             # Cargo workspace (FFI-часть)
├── packages/
│   ├── core/
│   │   ├── src/lib.yx     # Код YaoXiang
│   │   └── native/
│   │       ├── Cargo.toml # Код Rust FFI
│   │       └── src/lib.rs
│   └── utils/
│       ├── src/lib.yx
│       └── native/
│           ├── Cargo.toml
│           └── src/lib.rs
```

Команда `yaoxiang build` автоматически обнаруживает это и вызывает `cargo build` для компиляции native-части.

### Команды CLI

| Команда | Назначение |
|------|------|
| `yaoxiang workspace list` | Перечислить членов рабочего пространства |
| `yaoxiang workspace add <path>` | Добавить члена |
| `yaoxiang workspace remove <name>` | Удалить члена |
| `yaoxiang build` | Собрать всех членов |
| `yaoxiang build core` | Собрать указанного члена |
| `yaoxiang test` | Запустить тесты всех членов |

## Детальное проектирование

### Структура WorkspaceManifest

```rust
struct WorkspaceManifest {
    members: BTreeMap<String, String>,  // name -> toml path
}

struct Workspace {
    root: PathBuf,
    manifest: WorkspaceManifest,
    members: Vec<WorkspaceMember>,
    lock: LockFile,
}

struct WorkspaceMember {
    name: String,
    root: PathBuf,
    manifest: PackageManifest,
}
```

### Ссылка на зависимость в workspace

Семантика `{ workspace = "member-name" }`:
- в `dependencies` ссылается на другого члена рабочего пространства
- при разработке разрешается в локальный путь
- при публикации заменяется на версию из Registry
- имя члена должно присутствовать в `[workspace.members]`

### Совместное использование lockfile

- В рабочем пространстве существует только один `yaoxiang.lock` (в корневом каталоге)
- Разрешение зависимостей всех членов объединяется в единый lockfile
- Конфликты версий порождают ошибку при генерации lockfile вместе со сведениями об источниках конфликта

## Компромиссы

### Достоинства

- Единое управление мультипакетными проектами
- Общий lockfile гарантирует согласованность
- Удобство разработки при использовании зависимостей по пути
- Бесшовная интеграция с Cargo workspace

### Недостатки

- Все члены должны использовать одинаковые версии внешних зависимостей (может оказаться слишком строгим)
- Корневой toml не может иметь собственных зависимостей (ограничение дизайна)
- Интеграция с Cargo workspace повышает сложность

## Альтернативы

| Вариант | Почему не выбран |
|------|-----------|
| Независимые проекты + зависимости по пути | lockfile не объединён, риск дрейфа версий |
| Аналог npm workspaces | у npm workspaces множество проблем, не стоит подражать |
| Прямое повторное использование Cargo workspace | YaoXiang и Cargo — разные экосистемы пакетов |

## Стратегия реализации

### Разбиение на этапы

| Этап | Содержание |
|------|------|
| Phase 6a | Разбор `[workspace.members]` + WorkspaceManifest |
| Phase 6b | Общий lockfile + объединённое разрешение зависимостей |
| Phase 6c | Ссылка на зависимость по пути `{ workspace = "name" }` |
| Phase 6d | Автоматическая подстановка зависимостей по пути при публикации |
| Phase 6e | Интеграция с Cargo workspace |

### Зависимости

- Зависит от RFC-014 Phase 3 (глобальный кэш)
- Опционально зависит от RFC-014b (система сборки, для native-членов)

## Открытые вопросы

- [ ] Допускать ли циклические зависимости между членами?
- [ ] Поддерживать ли конфигурацию `[build]` на уровне workspace?
- [ ] Может ли член иметь собственный lockfile (перекрывающий корневой)?
- [ ] Поддерживать ли вложенные workspace?

---

## Ссылки

- [Cargo Workspaces](https://doc.rust-lang.org/cargo/reference/workspaces.html)
- [npm Workspaces](https://docs.npmjs.com/cli/using-npm/workspaces)
- [pnpm Workspaces](https://pnpm.io/workspaces)
```