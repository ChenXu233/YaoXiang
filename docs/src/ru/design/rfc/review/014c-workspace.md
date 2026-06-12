---
title: "RFC-014c: Поддержка рабочих пространств"
status: "На рассмотрении"
author: "晨煦 (Chenxu)"
created: "2026-06-11"
updated: "2026-06-11"
group: "rfc-014"
---

# RFC-014c: Поддержка рабочих пространств

> Настоящий RFC является под-RFC документа [RFC-014: Проект системы управления пакетами](../accepted/014-package-manager.md).

## Резюме

Определяет механизм рабочих пространств (workspace) в YaoXiang: совместное использование зависимостей, ссылок на пути, единого lockfile и интеграцию с Cargo workspace при разработке нескольких связанных пакетов.

## Мотивация

По мере роста проекта код необходимо разбивать на несколько пакетов. Этим пакетам требуется:
- ссылаться друг на друга (path-зависимости)
- совместно использовать версии внешних зависимостей (во избежание расхождения версий)
- иметь единый lockfile (для гарантии воспроизводимости сборки)
- взаимодействовать с Cargo workspace (для FFI-частей)

### Текущие проблемы

- Каждый проект управляет зависимостями независимо, совместное использование невозможно
- Отсутствует механизм автоматической замены path-зависимостей при публикации
- Нет интеграции с Cargo workspace

## Предложение

### Ключевая идея: координационный уровень + самодостаточные члены

Корневое рабочее пространство выполняет только координацию, каждый член полностью самодостаточен.

### Корневой yaoxiang.toml

```toml
# Корневой yaoxiang.toml
[workspace.members]
core = "packages/core/yaoxiang.toml"
utils = "packages/utils/yaoxiang.toml"
app = "packages/app/yaoxiang.toml"
```

**Корневой toml выполняет только три функции:**
1. Объявляет список членов (в форме словаря, где key — имя члена, value — путь к toml-файлу)
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
└── Cargo.toml                 # Опционально: общий Cargo workspace (FFI)
```

### Разрешение зависимостей

- Каждый член читает собственную секцию `[dependencies]`
- При разрешении зависимости всех членов объединяются и генерируется общий lockfile
- Конфликты версий вызывают ошибку на этапе генерации lockfile
- Один и тот же пакет в разных членах должен разрешаться в одну и ту же версию

### Ссылки на зависимости в workspace

`{ workspace = "member-name" }` ссылается на **key** секции `[workspace.members]` (а не на `[package].name` члена).

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
utils = { workspace = "utils" }   # ✅ Ссылка на key "utils"
# Даже если в packages/utils/yaoxiang.toml указано name = "my-utils"
```

**Почему используется key, а не name:**
- key контролируется рабочим пространством, он стабилен и уникален
- `[package].name` — это публичное имя, которое может измениться при публикации
- key является ключом BTreeMap, по своей природе уникальным
- При публикации ссылки workspace заменяются на версионные зависимости, и key не утекает в публичный API

### Path-зависимости и публикация

При разработке используется ссылка на рабочее пространство:

```toml
[dependencies]
utils = { workspace = "utils" }
```

При публикации автоматически заменяется на версионную зависимость:

```toml
[dependencies]
utils = "^0.2.0"
```

**Источник версии:** берётся из `[package].version` зависимого члена с префиксом `^`. Registry не проверяется — авторитетным источником версии является `yaoxiang.toml` члена, Registry — лишь канал распространения.

Менеджер пакетов автоматически выполняет эту замену при выполнении `yaoxiang publish`.

### Интеграция с Cargo Workspace

Если в рабочем пространстве имеются FFI-пакеты, можно одновременно определить Cargo workspace:

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

`yaoxiang build` автоматически обнаруживает это и вызывает `cargo build` для компиляции native-части.

### Команды CLI

| Команда | Назначение |
|------|------|
| `yaoxiang workspace list` | Список членов рабочего пространства |
| `yaoxiang workspace add <path>` | Добавить члена |
| `yaoxiang workspace remove <name>` | Удалить члена |
| `yaoxiang build` | Собрать всех членов (в топологическом порядке) |
| `yaoxiang build core` | Собрать указанного члена |
| `yaoxiang test` | Запустить тесты всех членов |

**Поведение `yaoxiang build`:** собирает всех членов в топологическом порядке зависимостей. Если core → utils → app, то порядок сборки: core → utils → app.

## Детальное проектирование

### Структура WorkspaceManifest

Корневой toml использует отдельный тип `WorkspaceManifest`, а не `PackageManifest`:

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

### Ссылки на зависимости workspace

Семантика `{ workspace = "member-name" }`:
- ссылается в `dependencies` на другого члена рабочего пространства
- при разработке разрешается в локальный путь
- при публикации заменяется на версию из Registry
- имя члена должно присутствовать в `[workspace.members]`

### Общий lockfile

- У рабочего пространства только один `yaoxiang.lock` (в корневом каталоге)
- Разрешение зависимостей всех членов объединяется в одном lockfile
- Конфликты версий вызывают ошибку на этапе генерации lockfile с указанием источника конфликта

## Компромиссы

### Преимущества

- Единое управление в проектах с множеством пакетов
- Общий lockfile гарантирует согласованность
- Удобство разработки с path-зависимостями
- Бесшовная интеграция с Cargo workspace

### Недостатки

- Все члены должны использовать одинаковые версии внешних зависимостей (может быть излишне строго)
- Корневой toml не может иметь собственных зависимостей (конструктивное ограничение)
- Интеграция с Cargo workspace увеличивает сложность

## Альтернативы

| Подход | Почему не выбран |
|------|------|
| Независимые проекты + path-зависимости | lockfile не унифицирован, риск расхождения версий |
| По аналогии с npm workspaces | У npm workspaces много проблем, не стоит копировать |
| Прямое использование Cargo workspace | YaoXiang и Cargo — разные экосистемы пакетов |

## Стратегия реализации

### Этапы

| Этап | Содержание |
|------|------|
| Phase 6a | Разбор `[workspace.members]` + WorkspaceManifest |
| Phase 6b | Общий lockfile + объединённое разрешение зависимостей |
| Phase 6c | Ссылки на path-зависимости `{ workspace = "name" }` |
| Phase 6d | Автоматическая замена path-зависимостей при публикации |
| Phase 6e | Интеграция с Cargo workspace |

### Зависимости

- Зависит от RFC-014 Phase 3 (глобальный кеш)
- Опционально зависит от RFC-014b (система сборки, для native-членов)

## Открытые вопросы

- [ ] Допускать ли циклические зависимости между членами?
- [ ] Поддерживать ли секцию `[build]` на уровне workspace?
- [ ] Могут ли члены иметь собственный lockfile (перекрывающий корневой)?
- [ ] Поддерживать ли вложенные workspace?

---

## Ссылки

- [Cargo Workspaces](https://doc.rust-lang.org/cargo/reference/workspaces.html)
- [npm Workspaces](https://docs.npmjs.com/cli/using-npm/workspaces)
- [pnpm Workspaces](https://pnpm.io/workspaces)