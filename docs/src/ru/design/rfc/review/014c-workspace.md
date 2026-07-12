```markdown
---
title: "RFC-014c: Поддержка рабочих пространств"
status: "На рассмотрении"
author: "晨煦"
created: "2026-06-11"
updated: "2026-07-05"
group: "rfc-014"
issue: "#113"
---

# RFC-014c: Поддержка рабочих пространств

> Данный RFC является под-RFC для [RFC-014: Дизайн системы управления пакетами](../accepted/014-package-manager.md).

## Краткое содержание

Определяет механизм рабочих пространств (workspace) в YaoXiang: совместное использование зависимостей, ссылок на пути, единого lockfile при совместной разработке нескольких связанных пакетов, а также интеграцию с Cargo workspace.

## Мотивация

По мере роста масштаба проекта код необходимо разбивать на несколько пакетов. Этим пакетам требуется:
- взаимное использование (путевые зависимости)
- совместное использование версий внешних зависимостей (во избежание дрейфа версий)
- единый lockfile (для обеспечения согласованности сборки)
- взаимодействие с Cargo workspace (для FFI-частей)

### Текущая проблема

- Каждый проект независимо управляет зависимостями — совместное использование невозможно
- Отсутствует механизм автоматической замены путевых зависимостей при публикации
- Отсутствует интеграция с Cargo workspace

## Предложение

### Основной дизайн: координирующий слой + самодостаточные члены

Корневой workspace выполняет только координацию; каждый член полностью самодостаточен.

### Корневой yaoxiang.toml

```toml
# Корневой yaoxiang.toml
[workspace.members]
core = "packages/core/yaoxiang.toml"
utils = "packages/utils/yaoxiang.toml"
app = "packages/app/yaoxiang.toml"
```

**Корневой toml выполняет только три функции:**
1. Объявляет список членов (в форме словаря, где key — имя члена, value — путь к toml)
2. Предоставляет общий lockfile (`yaoxiang.lock`)
3. Предоставляет общий каталог vendor (`.yaoxiang/vendor/`)

**Корневой toml не определяет dependencies.** Зависимости каждого члена записываются в его собственном `yaoxiang.toml`.

### Членский yaoxiang.toml

```toml
# packages/core/yaoxiang.toml
[package]
name = "core"
version = "0.1.0"

[dependencies]
json = "^2.0.0"
utils = { workspace = "utils" }    # ссылка на члена рабочего пространства
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
├── yaoxiang.toml              # конфигурация корня рабочего пространства
├── yaoxiang.lock              # общий lockfile
├── .yaoxiang/
│   └── vendor/                # общий каталог vendor
├── packages/
│   ├── core/
│   │   ├── yaoxiang.toml      # конфигурация пакета-члена
│   │   └── src/lib.yx
│   ├── utils/
│   │   ├── yaoxiang.toml
│   │   └── src/lib.yx
│   └── app/
│       ├── yaoxiang.toml
│       └── src/main.yx
└── Cargo.toml                 # опционально: общий Cargo workspace (FFI)
```

### Разрешение зависимостей

- Каждый член читает свой `[dependencies]`
- При разрешении зависимости всех членов объединяются и генерируется единый общий lockfile
- Конфликты версий вызывают ошибку на этапе генерации lockfile
- Один и тот же пакет в разных членах должен разрешаться в одну и ту же версию

### Ссылки на зависимости в workspace

`{ workspace = "member-name" }` ссылается на **key** в `[workspace.members]` (а не на `[package].name` члена).

```toml
# Корневой yaoxiang.toml
[workspace.members]
utils = "packages/utils/yaoxiang.toml"    # key = "utils"
```

```toml
# packages/app/yaoxiang.toml
[package]
name = "app"

[dependencies]
utils = { workspace = "utils" }   # ✅ ссылка на key "utils"
# даже если в packages/utils/yaoxiang.toml указано name = "my-utils"
```

**Почему используется key, а не name:**
- key контролируется рабочим пространством и стабильно уникален
- `[package].name` — публичное имя, которое может меняться при публикации
- key является ключом BTreeMap, что обеспечивает естественную уникальность
- При публикации ссылки workspace заменяются на версионные зависимости, и key не утекает в публичный API

### Путевые зависимости и публикация

Во время разработки используется ссылка на рабочее пространство:

```toml
[dependencies]
utils = { workspace = "utils" }
```

При публикации автоматически заменяется на версионную зависимость:

```toml
[dependencies]
utils = "^0.2.0"
```

**Источник версии:** считывается `[package].version` зависимого члена, к которому добавляется префикс `^`. Registry не проверяется — авторитетным источником версии является `yaoxiang.toml` члена, а Registry — лишь канал распространения.

Менеджер пакетов автоматически выполняет эту замену при выполнении `yaoxiang publish`.

### Интеграция с Cargo Workspace

Если в рабочем пространстве есть FFI-пакеты, можно одновременно определить Cargo workspace:

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
│   │   ├── src/lib.yx     # код YaoXiang
│   │   └── native/
│   │       ├── Cargo.toml # Rust FFI-код
│   │       └── src/lib.rs
│   └── utils/
│       ├── src/lib.yx
│       └── native/
│           ├── Cargo.toml
│           └── src/lib.rs
```

`yaoxiang build` автоматически обнаруживает и вызывает `cargo build` для компиляции native-частей.

### Команды CLI

| Команда | Функция |
|------|------|
| `yaoxiang workspace list` | список членов рабочего пространства |
| `yaoxiang workspace add <path>` | добавить члена |
| `yaoxiang workspace remove <name>` | удалить члена |
| `yaoxiang build` | собрать всех членов (в топологическом порядке зависимостей) |
| `yaoxiang build core` | собрать указанного члена |
| `yaoxiang test` | запустить тесты всех членов |

**Поведение `yaoxiang build`:** собирает всех членов в топологическом порядке зависимостей. Если имеется core → utils → app, порядок сборки: core → utils → app.

## Подробный дизайн

### Структура WorkspaceManifest

Корневой toml использует отдельный тип `WorkspaceManifest`, а не повторно использует `PackageManifest`:

```rust
struct WorkspaceManifest {
    workspace: WorkspaceConfig,
}

struct WorkspaceConfig {
    members: BTreeMap<String, String>,  // key -> путь к toml
}

struct Workspace {
    root: PathBuf,
    manifest: WorkspaceManifest,
    members: Vec<WorkspaceMember>,
    lock: LockFile,
}

struct WorkspaceMember {
    name: String,           // key из [workspace.members]
    root: PathBuf,
    manifest: PackageManifest,
}
```

**Логика обнаружения:** при загрузке toml, если присутствует секция `[workspace]`, файл разбирается как `WorkspaceManifest`, иначе — как `PackageManifest`.

### Ссылки на зависимости в workspace

Семантика `{ workspace = "member-name" }`:
- ссылка в `dependencies` на другого члена рабочего пространства
- во время разработки разрешается в локальный путь
- при публикации заменяется на версию из Registry
- имя члена должно присутствовать в `[workspace.members]`

### Общий lockfile

- В рабочем пространстве имеется только один `yaoxiang.lock` (в корневом каталоге)
- Разрешение зависимостей всех членов объединяется в один lockfile
- Конфликты версий вызывают ошибку на этапе генерации lockfile вместе с информацией об источниках конфликта

## Компромиссы

### Преимущества

- Единое управление многопакетными проектами
- Общий lockfile обеспечивает согласованность
- Удобство разработки с путевыми зависимостями
- Бесшовная интеграция с Cargo workspace

### Недостатки

- Все члены должны использовать одинаковые версии внешних зависимостей (может оказаться слишком строгим)
- Корневой toml не может иметь собственных зависимостей (ограничение дизайна)
- Интеграция с Cargo workspace увеличивает сложность

## Альтернативы

| Решение | Почему не выбрано |
|------|-----------|
| Независимые проекты + path-зависимости | lockfile не единый, риск дрейфа версий |
| Подобно npm workspaces | у npm в workspace много проблем,不值得模仿 |
| Прямое повторное использование Cargo workspace | YaoXiang и Cargo — разные экосистемы пакетов |

## Стратегия реализации

### Разбивка по фазам

| Фаза | Содержание |
|------|------|
| Phase 6a | Разбор `[workspace.members]` + WorkspaceManifest |
| Phase 6b | Общий lockfile + объединённое разрешение зависимостей |
| Phase 6c | `{ workspace = "name" }` — ссылки на путевые зависимости |
| Phase 6d | Автоматическая замена путевых зависимостей при публикации |
| Phase 6e | Интеграция с Cargo workspace |

### Зависимости

- Зависит от RFC-014 Phase 3 (глобальный кэш)
- Опционально зависит от RFC-014b (система сборки, для native-членов)

## Открытые вопросы

- [ ] Допускаются ли циклические зависимости между членами?
- [ ] Поддерживается ли конфигурация `[build]` на уровне workspace?
- [ ] Могут ли члены иметь собственный lockfile (переопределяющий корневой)?
- [ ] Поддерживаются ли вложенные workspace?

---

## Ссылки

- [Cargo Workspaces](https://doc.rust-lang.org/cargo/reference/workspaces.html)
- [npm Workspaces](https://docs.npmjs.com/cli/using-npm/workspaces)
- [pnpm Workspaces](https://pnpm.io/workspaces)
```