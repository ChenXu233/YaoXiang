---
title: "RFC-035: Поддержка MCP-сервера (интеграция с ИИ-агентом)"
status: "Черновик"
author: "Чэнь Сюй (晨煦)"
created: "2026-07-11"
updated: "2026-07-11"
issue: "#154"
---

# RFC-035: Поддержка MCP-сервера (интеграция с ИИ-агентом)

## Краткое содержание

Добавить в YaoXiang MCP-сервер (Model Context Protocol), чтобы ИИ-агенты (Claude Code, Continue, Cody, Zed и др.) могли напрямую запрашивать **AST, ошибки разбора, типы, символы, ссылки и результаты форматирования** исходного кода YaoXiang. Многоразово использовать уже реализованный в RFC-017 бэкенд `World`, добавить подкоманду `yaoxiang mcp`, получить один бинарник с двумя режимами и независимые `World` в разных процессах.

## Мотивация

### Зачем нужна эта функциональность?

RFC-017 сделал так, что YaoXiang **может** быть понят редакторами (hover / goto-def / completion). Но LSP — это протокол, **управляемый позициями**:
- каждый запрос жёстко зависит от `textDocument` URI + `Position`;
- редактор должен сначала открыть файл, сохранить его и поддерживать долгое соединение с LSP-сервером;
- рабочий процесс ИИ-агента — это **фрагменты кода**: «вставить кусок кода» в диалоге и задать вопрос, **не** сохраняя его на диск.

Реально доступные LSP-клиенты для ИИ-агентов (vscode-langservers-extracted, проекты типа `mcp-lsp-bridge`) переводят **только L1**: goto-def, hover. А ведь ИИ хочет делать совсем другое:
- «Этот код **разобран правильно** или нет?» — нужен parse + полный поток диагностик;
- «**Как используется** этот символ в файле?» — нужен lookup_symbol по имени;
- «Как **выглядит** этот код после форматирования?» — нужен format_source;
- «Где **все** ошибки типов?» — нужен typecheck по всей рабочей области.

Перевод возможностей L1 через LSP **не покрывает** эти потребности, так как LSP по своему дизайну их не поддерживает.

### Текущая проблема

1. Плохой опыт вызова LSP из ИИ-агента: требуется мок документа, огромный JSON, жёсткая зависимость от URI.
2. В проекте YaoXiang отсутствует «AI-First» интерфейсный слой: человек использует LSP в IDE, а ИИ-агент LSP использовать не может.
3. Ведущие ИИ-агенты (Claude Code, Continue и др.) уже по умолчанию поддерживают MCP, а для YaoXiang эта экосистема — пустое место.

### Что такое MCP?

MCP (Model Context Protocol) — протокол вызова инструментов ИИ-агентами, выпущенный и открытый под лицензией open-source компанией Anthropic в 2024–2025 годах, ставший де-факто стандартом (его подключили OpenAI, Google, Microsoft, Zed, Continue, Cody и др.). Особенности:
- основан на JSON-RPC 2.0 (родственен LSP);
- три основные примитивы: **Tools** (действия), Resources (данные), Prompts (шаблоны);
- транспорты: `stdio` (дочерний процесс) / потоковый `HTTP` / SSE;
- входные и выходные данные инструментов имеют **JSON Schema** со строгой типизацией (удобно для LLM);
- в версии 2025-06+ опубликована спецификация streamable HTTP; данный RFC совместим также со старым SSE.

**Данный RFC использует только примитив Tools** — выровнено с «предоставлением сервиса» в LSP и не вводит сложности файловой модели из Resources.

## Предложение

### Ключевая идея

Один бинарник — два режима:

```text
┌─────────────────────────────────────────────────────────┐
│                    yaoxiang (v0.7.7+)                   │
│  ┌─────────────────┐      ┌──────────────────────────┐  │
│  │ yaoxiang lsp    │      │   yaoxiang mcp           │  │
│  │ (stdio JSON-RPC)│      │   (stdio по умолчанию    │  │
│  │ Реализован в    │      │    + опционально HTTP)   │  │
│  │ RFC-017         │      │                          │  │
│  └────────┬────────┘      └──────────┬───────────────┘  │
│           │                         │                   │
│           ▼                         ▼                   │
│  ┌──────────────────────────────────────────────────┐  │
│  │  Общий lib-крейт (`yaoxiang`)                     │  │
│  │  src/lsp/{server,session,world}.rs                │  │
│  │  src/frontend/{lexer,parser,core}/...             │  │
│  │  src/middle/...                                   │  │
│  └──────────────────────────────────────────────────┘  │
│                                                          │
│  ┌──────────────────────────────────────────────────┐  │
│  │            src/mcp/  ← новое                      │  │
│  │  ├── mod.rs          (вход в модуль + запуск)     │  │
│  │  ├── transport/      (stdio + HTTP/SSE)           │  │
│  │  ├── server.rs       (цикл сообщений JSON-RPC)    │  │
│  │  ├── tools/          (обработчики 6 инструментов) │  │
│  │  ├── schema.rs       (JSON Schema ввода/вывода)   │  │
│  │  └── project.rs      (определение корня проекта   │  │
│  │                       + разбор путей)             │  │
│  └──────────────────────────────────────────────────┘  │
└─────────────────────────────────────────────────────────┘
```

**Ключевые решения**:
- **Один бинарник**: `yaoxiang` переключается подкомандой; LSP-процесс и MCP-процесс **не сосуществуют** в одной среде исполнения.
- **Независимый `World` в каждом процессе**: каждый процесс `yaoxiang mcp` владеет одним `World`; не влияет на LSP-процесс и другие MCP-процессы (без конкуренции за блокировки, изоляция аварий).
- **stdio по умолчанию**: нет конфликтов портов, нулевая сетевая конфигурация; HTTP — опциональный запасной вариант.
- **Переиспользование, а не дублирование**: напрямую вызывать lib-API `yaoxiang::frontend` / `yaoxiang::middle` / `yaoxiang::lsp::handlers`, **не** через прослойку LSP-клиента.

### Набор инструментов (8 инструментов, поставка в 3 этапа)

Спроектировано по принципу «устранить особые случаи + поэтапность»: сначала stateless-инструменты для чистого исходника, затем инструменты рабочей области поверх LSP World, и наконец инструменты AST-перезаписи.

| Имя инструмента | Вход | Выход | Переиспользование | Этап |
|---|---|---|---|---|
| `parse_source` | `source: String`, `tab_size?: u32` | `{ast: Node, diagnostics: Diagnostic[]}` | Напрямую `frontend::parse` | v0.8.x |
| `format_source` | `source: String`, `tab_size?: u32` | `{formatted: String, diff: Hunk[]}` | Напрямую `formatter::format` | v0.8.x |
| `lookup_symbol` | `query: String`, `workspace_root?: String`, `kind?: SymbolKind[]` | `{symbols: Symbol[]}` | Переиспользовать `lsp::handlers::workspace_symbol` (нечёткий поиск по `query`) | v0.8.x |
| `find_references` | `query: String`, `workspace_root?: String` | `{locations: Location[]}` | Переиспользовать `lsp::handlers::references` (по `query`, а не по позиции) | v0.8.x |
| `typecheck` | `file_paths: String[]`, `project_root: String` | `{diagnostics: Diagnostic[], summary: Counts}` | Переиспользовать `lsp::world::typecheck_full` | v0.8.x |
| `explain_diagnostic` | `code: String` (например, `E0001`), `lang?: String` | `{code, category, title, description, example, help}` | **Напрямую** `util::diagnostic::command::render_explain_output` | **v0.9.x** |
| `list_imports` | `file_path: String`, `project_root?: String` | `{imports: [{module, items, is_public}]}` | Переиспользовать `middle::passes::module::ModuleGraph::validate_imports` | **v0.9.x** |
| `rename_symbol` | `source: String`, `old_name: String`, `new_name: String`, `scope?: "module" \| "function:name"` | `{source: String, edits: Edit[], diagnostics: Diagnostic[]}` | **Добавить** `src/middle/rename.rs` (AST-перезапись) | **v0.10.x** |

**Границы 8 инструментов**:
- `parse_source` / `format_source` — **stateless по чистому исходнику**, не входят в `World`.
- `lookup_symbol` / `find_references` — принимают `workspace_root` (если не передан, используется `--project-root` из запуска).
- `typecheck` — `file_paths` **обязательно**, чтобы рабочая область была полной.
- `explain_diagnostic` — **нулевая зависимость от файлов**, чисто строковый запрос в реестр кодов ошибок.
- `list_imports` — `file_path` — физический файл, выдаёт результат разбора import для этого файла.
- `rename_symbol` — **AST-перезапись чистого исходника**, без позиционного запроса в стиле LSP (семантика отличается от существующего `lsp::handlers::rename`).
- ~~`hover` / `completion` / `signature_help`~~ — **все убраны**: ИИ-агенту не нужна «позиционно-чувствительная» семантика, вместо этого поиск по имени через `lookup_symbol`.

**Когда загружается `World`**: при старте сервера по `--project-root` сканируются `yaoxiang.toml` и `src/**/*.yx`, и однажды через уже реализованный в RFC-017 API `World::load_*` всё заливается в `World.documents`. **Никаких** новых API в lib **не добавляется**.

### Контракт инструментов

**Вход**: описывается JSON Schema; каждое поле имеет `description` + `examples` (LLM читает автоматически).

**Выход**: структурированный JSON, единое поле `schemaVersion: "1.0"`:

```jsonc
// Успешный ответ
{
  "schemaVersion": "1.0",
  "isError": false,
  "content": [
    { "type": "json", "json": { /* данные конкретного инструмента */ } }
  ]
}

// Диагностика возвращается структурированно (не считается ошибкой tool)
{
  "schemaVersion": "1.0",
  "isError": false,
  "content": [{ "type": "json", "json": {
    "ast": {...},
    "diagnostics": [
      { "code": "E0001", "severity": "error", "message": "...", "span": [12, 4, 12, 18] }
    ]
  }}]
}

// Ошибка уровня инструмента (например, parse_source получил невалидный UTF-8)
{
  "schemaVersion": "1.0",
  "isError": true,
  "content": [{ "type": "text", "text": "MCP-INVALID-INPUT: source не является валидным UTF-8" }],
  "errorCode": "MCP-INVALID-INPUT"
}
```

**Система ошибок**:
- **Диагностика (diagnostic)**: ошибки разбора/типизации, по RFC-013 (`E0001` и др.) — **не считаются** ошибкой tool.
- **Ошибки уровня инструмента**: с префиксом `MCP-` (`MCP-INVALID-INPUT`, `MCP-PROJECT-NOT-FOUND`, `MCP-INTERNAL`) — трактуются как `isError: true`.
- **panic/crash**: JSON-RPC `-32603 Internal error`; сервер не завершается.

**Правила разбора путей** (применяются к `workspace_root` в `lookup_symbol` / `find_references`, и к `file_paths` в `typecheck`):
1. Аргумент командной строки `--project-root <dir>` — наивысший приоритет (перекрывает умолчание).
2. Иначе: от cwd подниматься вверх и искать `yaoxiang.toml` до корня файловой системы (по RFC-015).
3. Иначе: сам cwd.
4. `file_paths` должен находиться внутри корня проекта (защита от обхода); при выходе за пределы → `MCP-PATH-OUTSIDE-PROJECT`.

### Транспортный уровень

**stdio (по умолчанию)**:

```bash
yaoxiang mcp
# после старта читает JSON-RPC из stdin, пишет в stdout, stderr — для логов
```

Конфигурация ИИ-агента (Claude Code `.mcp.json` / Continue `config.json`):
```jsonc
{
  "mcpServers": {
    "yaoxiang": {
      "command": "yaoxiang",
      "args": ["mcp", "--project-root", "${workspaceFolder}"]
    }
  }
}
```

**streamable HTTP (опционально)**:

```bash
yaoxiang mcp --http --addr 127.0.0.1:7325  # один HTTP-порт, новая спецификация MCP
yaoxiang mcp --http --sse --addr 127.0.0.1:7325  # совместимость со старым SSE (v0.10)
```

**Ограничения безопасности**:
- **Слушать только loopback** (127.0.0.1 / ::1); привязка к публичному адресу явно отвергается с ошибкой и завершением.
- HTTP **без аутентификации** (loopback считается доверенным по умолчанию); в будущем — поле `--require-token <hex>`.
- Режим stdio с дочерним процессом изолирован по своей природе (parent-процесс контролирует права).

### Многопроцессность и конкурентность

Каждый процесс `yaoxiang mcp` владеет одним `World`, между собой не делятся:

```text
┌─────────────┐   ┌─────────────┐   ┌─────────────┐
│ yaoxiang    │   │ yaoxiang    │   │ yaoxiang    │
│   lsp       │   │   mcp       │   │   mcp       │
│ (Editor 1)  │   │ (Claude 1)  │   │ (Claude 2)  │
└──────┬──────┘   └──────┬──────┘   └──────┬──────┘
       │ stdio/stdout    │ stdio          │ stdio
   ┌───┴────┐        ┌───┴────┐        ┌───┴────┐
   │ Editor │        │ Claude │        │ Claude │
   └────────┘        └────────┘        └────────┘
```

**Конфликты портов**: ИИ-агент настраивается на «запуск дочернего процесса» — конфликты портов в принципе невозможны. В режиме HTTP пользователь сам управляет распределением портов.
**Изоляция `World`**: у каждого процесса независимое LSP-состояние синхронизации — падение одного MCP-процесса **не влияет** на LSP и другие MCP-процессы.
**Множественные сессии в будущем**: рассмотрение нескольких рабочих областей в одном процессе (несколько `Session` внутри) — только в v2, **в данном RFC не делается**.

## Детальное проектирование

### Структуры данных

Новый файл `src/mcp/project.rs`:

```rust
pub struct ProjectRoot {
    /// Абсолютный путь
    pub root: PathBuf,
    /// Источник стратегии определения корня проекта при загрузке
    pub source: ProjectRootSource,
}

pub enum ProjectRootSource {
    CliFlag,           // yaoxiang mcp --project-root
    AutoDetected,      // подъём вверх с поиском yaoxiang.toml
    FallbackCwd,       // откат на cwd
}

pub struct ResolvedPath {
    /// Путь относительно корня проекта (рекомендуется отдавать ИИ для чтения)
    pub relative: String,
    /// Абсолютный путь после разбора (для операций с World)
    pub absolute: PathBuf,
}

impl ProjectRoot {
    /// Резолвит "file_path" в безопасный путь — защита от обхода
    pub fn resolve(&self, file_path: &str) -> Result<ResolvedPath, McpError>;
}
```

`ProjectRoot` — синглтон + автоматическая генерация схем инструментов в `src/mcp/schema.rs`:

```rust
pub struct ProjectRoot {
    /// Абсолютный путь (должен содержать `yaoxiang.toml` или иметь обратную совместимость)
    pub root: PathBuf,
    pub source: ProjectRootSource,
}

impl ProjectRoot {
    /// Определяется один раз при старте CLI, результат кешируется в контексте `McpServer` — используется всеми инструментами
    pub fn detect(cli_override: Option<PathBuf>) -> Result<Self, McpError>;
}
```

Схемы инструментов автоматически генерируются из input-структур через крейт `schemars`, чтобы избежать ручного дрейфа JSON Schema:

```rust
#[derive(Deserialize, schemars::JsonSchema)]
pub struct ParseSourceInput {
    /// Полный фрагмент исходного кода YaoXiang — **не** сохраняется на диск, чисто transient
    pub source: String,
    pub tab_size: Option<u32>,
}
```

**В схемах инструментов `parse_source` / `format_source` нет поля `file_path`** — эти два инструмента принимают только строку исходника и не участвуют в семантике проекта. `lookup_symbol` / `find_references` / `typecheck` принимают `workspace_root` или `file_paths` (обязательность — см. таблицу инструментов).


### Изменения в компиляторе

| Модуль | Изменение |
|---|---|
| `src/lsp/world.rs` | **Без изменений** — MCP при старте вызывает уже существующий API `World::load_*` для одноразовой загрузки рабочей области |
| `src/lsp/handlers/workspace_symbol.rs` | **Без изменений** — `mcp/tools/lookup.rs` оборачивает `query` во входные параметры LSP |
| `src/lsp/handlers/references.rs` | **Без изменений** — то же самое |
| `src/lsp/handlers/formatter.rs` | **Без изменений** — `format_source` вызывается напрямую |
| `src/main.rs` | Добавить ветвление подкоманды `Mcp` |
| `Cargo.toml` | Добавить feature `mcp-server` (или оставить в основном бинарнике) |
| `src/util/diagnostic/` | **Без изменений** (реализовано в RFC-017) |

**Ключевое ограничение**: `src/mcp/` **не должен** иметь обратных зависимостей на приватные символы `src/lsp/` — только через публичный API `crate::lsp::` вызывать handlers.

### Обратная совместимость

- ✅ **Полная обратная совместимость**: новая подкоманда `yaoxiang mcp`, ни одно существующее поведение `yaoxiang` / `yaoxiang lsp` не меняется.
- ✅ **LSP-сервер не трогаем**: все возможности, API и внутреннее состояние, реализованные в RFC-017, остаются неизменными.
- ✅ **Публичный API lib-крейта не трогаем**: все `pub`-пути без изменений; MCP только потребляет существующий API — **ноль** новых `pub`-методов.

### Интеграция с существующими системами

| Существующий модуль | Способ интеграции с MCP |
|---|---|
| `src/frontend/lexer` | `parse_source` напрямую вызывает lexer |
| `src/frontend/core/parser` | `parse_source` напрямую вызывает parser; при сбое порождаются узлы `Missing*` (RFC-017) |
| `src/frontend/core/typecheck/inference/*` | `typecheck` переиспользует шаблон `collect_diagnostics` (RFC-017 §проблема 1) |
| `src/middle/` | `typecheck` прогоняет все middle-проходы (анализ зависимостей и др.) |
| `src/lsp/world.rs` | При старте вызывается API `World::load_*` (уже есть); `World` **не** принимает «виртуальных документов» |
| `src/lsp/handlers/workspace_symbol.rs` | `mcp/tools/lookup.rs` оборачивает, превращая `query: String` во входные параметры LSP (поиск по имени) |
| `src/lsp/handlers/references.rs` | `mcp/tools/find_refs.rs` оборачивает, превращая `query: String` во входные параметры LSP |
| `src/lsp/handlers/formatter.rs` | `mcp/tools/format.rs` вызывается напрямую (если не реализовано — добавить `formatter::format_with_diff`) |
| `src/util/i18n/` | Сообщения об ошибках идут через многоязычные файлы ресурсов (zh-CN/en) |

### Обработка ошибок

| Источник | Обработка |
|---|---|
| Ошибка разбора | `Diagnostic{code:"E0xxx", severity, message, span}` (**не ошибка tool**, возвращается в `content`) |
| Ошибка типов | То же самое |
| Выход `file_paths` за пределы (инструмент `typecheck`) | Ошибка уровня инструмента `MCP-PATH-OUTSIDE-PROJECT` |
| Невалидный UTF-8 в `source` | Ошибка уровня инструмента `MCP-INVALID-INPUT` |
| Panic инструмента | JSON-RPC `-32603 Internal error`; сервер **не завершается** |
| Клиент отправил не JSON-RPC | Разрыв потока (stdio EOF), перезапуск — новая сессия |

Уровни серьёзности диагностик берутся из реализованного в RFC-017 `enum ErrorKind { Error, Warning, Note }`.

### Стратегия тестирования

| Уровень | Тесты |
|---|---|
| **Unit** | `src/mcp/project.rs::resolve` — обход пути, `src/mcp/schema.rs` — валидация схем |
| **Integration** | mock stdio: поднимается сервер, в stdin заливается JSON-RPC, из stdout читается ответ, сравнение с фикстурой |
| **E2E** | Запускается реальный процесс `yaoxiang mcp`, цепочка вызовов в стиле Claude Code: parse → правка → format → typecheck |
| **Fuzz** | `cargo-fuzz` для разбора JSON-RPC в MCP (каркас libFuzzer) |

Каждый tool должен иметь как минимум: 1 happy path + 1 сценарий с диагностикой + 1 сценарий с ошибкой tool в integration-тестах.

## Компромиссы

### Преимущества

- **Минимальные затраты на переиспользование**: `World` / `Session` / `handlers` / сбор диагностик — всё уже реализовано (RFC-017); данный RFC — «оболочка MCP поверх».
- **AI-First интерфейс**: контракт инструментов в 3–5 раз понятнее LSP; LLM читает схему напрямую.
- **Изоляция процессов**: развязка с LSP-сессией редактора и с другими MCP-процессами, **нулевая конкуренция за блокировки**.
- **stdio-дружественность**: все ведущие ИИ-агенты по умолчанию используют режим дочернего процесса — подключение с нулевой конфигурацией.
- **YAGNI соблюдён**: данный RFC убирает Resources, Sessions, межпроцессное состояние, удалённый MCP — откроем в v2.

### Недостатки

- **Разделение протоколов**: в будущем LSP / MCP / DAP будут эволюционировать независимо — затраты на поддержание согласованности.
- **HTTP-режим — гражданин второго сорта**: ограничение loopback позиционирует его как локальный инструмент; удалённые сценарии потребуют перепроектирования в v2.
- **Затраты на повторный parse**: ИИ, итеративно правя исходник, повторно вызывает `parse_source` и заново прогоняет lexer+parser. **Смягчение**: `DocumentCache` из RFC-017 всё ещё ускоряет повторный parse **дискового** исходника с тем же содержимым; для чистого transient-исходника однократный parse неизбежен.
- **Затраты на тестовое покрытие**: 5 tool × 3 сценария = минимум 15 integration-тестов.

## Альтернативы

| Альтернатива | Почему не выбрана |
|---|---|
| **Встраивание двух протоколов в один процесс** (LSP+MCPlistener сосуществуют) | stdin/stdout может иметь только одного потребителя; HTTP тоже пришлось бы держать вместе — сложность > выгоды |
| **MCP как мост к LSP-клиенту** | Лишний слой IPC; LSP по дизайну не умеет искать символы по имени — MCP нужно то, что LSP не даёт |
| **gRPC / собственный протокол** | Отход от де-факто стандарта; у MCP уже есть SDK для сообщества (TypeScript, Python, Rust) с готовой экосистемой |
| **Переиспользовать все возможности LSP-обработчиков** (набор инструментов L3) | Много работы по адаптации позиция↔намерение; убывающая предельная отдача |
| **Только HTTP в первой версии** (без stdio) | Claude Code / Continue и др. по умолчанию используют stdio — слишком высокий порог входа |

## Стратегия реализации

### Зависимости

- **Сильная зависимость**: реализация LSP из RFC-017 (готово).
- **Сильная зависимость**: система кодов ошибок из RFC-013 (готово).
- **Сильная зависимость**: определение корня проекта из RFC-014 / RFC-015 (частично готово).
- **Новые зависимости** (Rust-крейты):
  - `mcp-rust-sdk` (предстоит оценить, см. [modelcontextprotocol/rust-sdk](https://github.com/modelcontextprotocol/rust-sdk))
  - `tokio` (**уже есть**, optional feature)
  - `axum` (для HTTP-режима) или напрямую `hyper` — предстоит оценить
- **Без изменений языковой спецификации**: чисто инкремент в инструментарии.

### Этапы (синхронизировано с #154)

| Этап | Содержание | Оценка срока |
|---|---|---|
| **v0.8.x (MVP)** | `src/mcp/{mod.rs, server.rs, transport/stdio.rs, project.rs, schema.rs}` + `parse_source` + `format_source` + `lookup_symbol` + `find_references` + `typecheck` (**5 инструментов**) + подкоманда `yaoxiang mcp` + `World::load_*` при старте | **3–4 недели** |
| **v0.9.x (YaoXiang Intelligence)** | `+ explain_diagnostic` (**напрямую** вызывает `render_explain_output`) + `+ list_imports` (обёртка над `ModuleGraph::validate_imports`) + unit/integration-тесты | **1–2 недели** |
| **v0.10.x (AST + HTTP)** | `+ rename_symbol` (**добавить** `src/middle/rename.rs`, AST-перезапись) + streamable HTTP transport + оптимизация производительности (parse_source P99 < 100 мс) | **2–3 недели** |


**Почему 3 этапа**: сначала MVP со stdio и 5 инструментами — валидируем разумность дизайна интерфейса; v0.9.x добавляет низкорисковые и не требующие адаптации «YaoXiang-специфичные» инструменты — валидируем корректность интеграции; v0.10.x открывает высокорисковый модуль «AST-перезаписи» — отдельный PR проще фокусировать на ревью.

### Риски

1. **Активность поддержки `mcp-rust-sdk`**: выпущен в 2025 году, API может резко меняться. **Смягчение**: оценить стабильность; если нестабилен — написать лёгкий JSON-RPC 2.0 + tool dispatcher своими силами (< 500 строк).
2. **Затраты на повторный parse**: ИИ, итеративно правя исходник, повторно вызывает `parse_source` и заново прогоняет lexer+parser. **Смягчение**: `DocumentCache` из RFC-017 всё ещё ускоряет повторный parse **дискового** исходника с тем же содержимым; для чистого transient-исходника однократный parse неизбежен.
3. **Совместимость схем MCP у разных ИИ-агентов**: разные агенты по-разному строги к схемам MCP. **Смягчение**: использовать крейт `schemars` для автогенерации схем из Rust-структур ввода — ноль ручного дрейфа.
4. **Кросс-платформенность разбора путей**: в Windows пути нечувствительны к регистру, есть UNC-пути, граница `\\`. **Смягчение**: использовать `camino::Utf8Path` вместо `std::path` для разбора путей.
5. **Схема входа MCP-инструмента не 1:1 с параметрами LSP**: LSP-`workspace_symbol` принимает `(query)`; при передаче внутрь LSP нужно обернуть в позицию+URI, чтобы существующий handler мог быть переиспользован. **Смягчение**: адаптер делать в `mcp/tools/lookup.rs`, инкапсулируя детали на стороне MCP.
6. **AST-перезапись `rename_symbol` отличается по семантике от LSP `rename`**: LSP `textDocument/rename` — это URI + позиция + new_name → WorkspaceEdit; MCP `rename_symbol` — это source + old_name + new_name → новый source. **Прямое переиспользование невозможно**. **Смягчение**: реализовать отдельно `src/middle/rename.rs`, scope-aware перезапись ссылок, без пересечений с реализацией LSP-обработчика.

## Открытые вопросы

- [ ] Выбор `mcp-rust-sdk` или собственная реализация? (@Chen Xu: сначала оценить июньскую версию rust-sdk, затем решить)
- [ ] Путь аутентификации HTTP? (отдельный RFC в v0.10)
- [ ] Нужно ли при старте MCP выводить `tools/list` для активного обнаружения ИИ? (требуется стандартом MCP, **реализуется по умолчанию**)
- [ ] Поддерживает ли `typecheck` `mode: "fast|full"` (fast — только подмножество текущего файла, full — вся рабочая область)?
- [ ] Реалистичен ли бюджет производительности parse_source P99 < 100 мс? (нужен бенчмарк `DocumentCache` из RFC-017 в режиме source-string)

## Ссылки

- [RFC-017: Поддержка Language Server Protocol (LSP)](./accepted/017-lsp-support.md)
- [RFC-013: Спецификация системы кодов ошибок](./accepted/013-error-code-specification.md)
- [RFC-014: Дизайн системы управления пакетами](./accepted/014-package-manager.md)
- [RFC-015: Дизайн системы конфигурации YaoXiang](./accepted/015-configuration-system.md)
- [Спецификация MCP](https://modelcontextprotocol.io/)
- [MCP Rust SDK](https://github.com/modelcontextprotocol/rust-sdk)
- [JSON-RPC 2.0](https://www.jsonrpc.org/specification)
- [Спецификация LSP 3.18](https://github.com/microsoft/language-server-protocol/blob/main/specifications/specification-3-18.md)
- [Rust Analyzer](https://rust-analyzer.github.io/) — референс по интеграции M2 / MCP
- [Реализация MCP в zed-industries/zed](https://github.com/zed-industries/zed/tree/main/crates/mcp)