---
title: 'RFC-035: Поддержка MCP Server (интеграция AI Agent)'
status: 'Черновик'
author: '晨煦'
created: '2026-07-11'
updated: '2026-07-11'
issue: '#154'
---

# RFC-035: Поддержка MCP Server (интеграция AI Agent)

## Аннотация

Добавление MCP (Model Context Protocol) сервера для YaoXiang, позволяющего AI agent
(Claude Code, Continue, Cody, Zed и др.) напрямую запрашивать **AST, ошибки парсинга,
типы, символы, ссылки, результаты форматирования** исходного кода YaoXiang. Повторное
использование уже реализованного в RFC-017 бэкенда `World`, новый подкоманда `yaoxiang
mcp`, один бинарный файл с двумя режимами, множество процессов с независимыми World.

## Мотивация

### Почему эта функция необходима?

RFC-017 позволил YaoXiang **быть** понятным для редакторов (hover / goto-def /
completion). Но LSP — это протокол **с привязкой к позиции**:

- Каждый запрос сильно зависит от `textDocument` URI + `Position`
- Редактор должен сначала открыть файл, сохранить, поддерживать долгое соединение с LSP
  server
- Рабочий процесс AI agent — это **фрагменты кода**: "вставить фрагмент кода" в
  диалоге и задать вопрос, **не** сохраняя предварительно на диск

LSP клиенты, фактически доступные AI agent (vscode-langservers-extracted,
проекты типа `mcp-lsp-bridge`) **переводят только L1**: goto-def, hover. AI хочет
делать:

- «Правильно ли **распарсился** этот код» — нужен parse + полный поток диагностики
- «Как символ **используется в файле**» — нужен lookup_symbol по имени
- «Как будет выглядеть **форматированный** код» — нужен format_source
- «Где **все** ошибки типов» — нужен typecheck для полной рабочей области

Эти возможности L1 **недоступны** через перевод LSP, потому что LSP по дизайну не
поддерживает их.

### Текущие проблемы

1. Плохой опыт вызова LSP из AI agent: требуется мок документов, огромный JSON,
   сильная зависимость от URI
2. В проекте YaoXiang отсутствует интерфейсный слой «AI-First»: люди используют LSP в
   IDE, AI agent не может использовать LSP
3. Claude Code / Continue и другие主流ные AI agent по умолчанию поддерживают MCP, для
   YaoXiang это пустая экосистема

### Что такое MCP?

MCP (Model Context Protocol) — это протокол вызова инструментов для AI agent,
опубликованный и открытый Anthropic в 2024-2025 годах, ставший фактическим стандартом
(OpenAI, Google, Microsoft, Zed, Continue, Cody и др. подключены). Особенности:

- Основан на JSON-RPC 2.0 (тот же источник, что и LSP)
- Три основных примитива: **Tools** (действия), Resources (данные), Prompts (шаблоны)
- Транспорт: `stdio` (дочерний процесс) / streamable `HTTP` / SSE
- Ввод/вывод инструментов имеет строгую типизацию **JSON Schema** (удобно для LLM)
- В 2025-06+ опубликована спецификация streamable HTTP, данный RFC одновременно
  совместим со старым SSE

**Данный RFC использует только примитив Tools** — соответствует "предоставлению
услуг" в LSP, не вводит сложность модели файлов Resources.

## Предложение

### Основной дизайн

Один бинарный файл с двумя режимами:

```text
┌─────────────────────────────────────────────────────────┐
│                    yaoxiang (v0.7.7+)                   │
│  ┌─────────────────┐      ┌──────────────────────────┐   │
│  │ yaoxiang lsp    │      │   yaoxiang mcp           │   │
│  │ (stdio JSON-RPC)│      │   (stdio default         │   │
│  │ RFC-017 уже     │      │    + HTTP optional)      │   │
│  └────────┬────────┘      └──────────┬───────────────┘   │
│           │                         │                    │
│           ▼                         ▼                    │
│  ┌──────────────────────────────────────────────────┐   │
│  │  Shared lib crate (`yaoxiang`)                   │   │
│  │  src/lsp/{server,session,world}.rs               │   │
│  │  src/frontend/{lexer,parser,core}/...            │   │
│  │  src/middle/...                                  │   │
│  └──────────────────────────────────────────────────┘   │
│                                                          │
│  ┌──────────────────────────────────────────────────┐   │
│  │            src/mcp/  ← NEW                        │   │
│  │  ├── mod.rs          (module entry + startup)    │   │
│  │  ├── transport/      (stdio + HTTP/SSE)          │   │
│  │  ├── server.rs       (JSON-RPC message loop)     │   │
│  │  ├── tools/          (6 tool handlers)          │   │
│  │  ├── schema.rs       (input/output JSON Schema)  │   │
│  │  └── project.rs      (project root detection)   │   │
│  └──────────────────────────────────────────────────┘   │
└─────────────────────────────────────────────────────────┘
```

**Ключевые решения**:

- **Тот же бинарный файл**: `yaoxiang` переключается через подкоманды; процессы LSP и
  MCP **не сосуществуют** в одном runtime
- **Многопроцессность с независимыми World**: каждый процесс `yaoxiang mcp` содержит
  один `World`; не влияют друг на друга (нет конкуренции за блокировки, изолированные
  сбои)
- **stdio по умолчанию**: избегание конфликтов портов, нулевая сетевая настройка; HTTP
  как опциональный запасной вариант
- **Повторное использование, а не дублирование**: прямой вызов lib API
  `yaoxiang::frontend` / `yaoxiang::middle` / `yaoxiang::lsp::handlers`, **не**
  через LSP-client

### Набор инструментов (8 инструментов в 3 этапа)

Дизайн по принципу «устранение особых случаев + поэтапная поставка»: stateless
инструменты для чистых источников идут первыми, инструменты рабочей области используют
общий LSP World, инструменты AST-перезаписи добавляются отдельно.

| Название Tool              | Ввод                                                                                          | Вывод                                                        | Повторное использование                                      | Этап         |
| -------------------------- | --------------------------------------------------------------------------------------------- | ------------------------------------------------------------ | ------------------------------------------------------------ | ------------ |
| `parse_source`             | `source: String`, `tab_size?: u32`                                                            | `{ast: Node, diagnostics: Diagnostic[]}`                     | Прямой вызов `frontend::parse`                               | v0.8.x       |
| `format_source`            | `source: String`, `tab_size?: u32`                                                            | `{formatted: String, diff: Hunk[]}`                          | Прямой вызов `formatter::format`                             | v0.8.x       |
| `lookup_symbol`            | `query: String`, `workspace_root?: String`, `kind?: SymbolKind[]`                             | `{symbols: Symbol[]}`                                        | Повторное использование `lsp::handlers::workspace_symbol` (поиск по `query`) | v0.8.x       |
| `find_references`          | `query: String`, `workspace_root?: String`                                                    | `{locations: Location[]}`                                    | Повторное использование `lsp::handlers::references` (по `query` вместо позиции) | v0.8.x       |
| `typecheck`                | `file_paths: String[]`, `project_root: String`                                                | `{diagnostics: Diagnostic[], summary: Counts}`               | Повторное использование `lsp::world::typecheck_full`         | v0.8.x       |
| `explain_diagnostic`       | `code: String` (например, `E0001`), `lang?: String`                                           | `{code, category, title, description, example, help}`        | **Прямой вызов** `util::diagnostic::command::render_explain_output` | **v0.9.x**   |
| `list_imports`             | `file_path: String`, `project_root?: String`                                                  | `{imports: [{module, items, source_file}]}`                    | Повторное использование `middle::passes::module::ModuleGraph::validate_imports` | **v0.9.x**   |
| `rename_symbol`            | `source: String`, `old_name: String`, `new_name: String`, `scope?: "module" \| "function:name"` | `{source: String, edits: Edit[], diagnostics: Diagnostic[]}` | **NEW** `src/middle/rename.rs` (AST перезапись)              | **v0.10.x**  |

**Границы 8 инструментов**:

- `parse_source` / `format_source` — **чистый stateless источник**, не в World
- `lookup_symbol` / `find_references` — принимают `workspace_root` (если не передан,
  используется `--project-root` при запуске)
- `typecheck` — `file_paths` **обязательно**, гарантирует полноту рабочей области
- `explain_diagnostic` — **нулевая зависимость от файлов**, чистый строковый запрос к
  реестру кодов ошибок
- `list_imports` — `file_path` физический файл, вывод результата парсинга импортов
  этого файла
- `rename_symbol` — **чистая AST перезапись источника**, без LSP-style позиционных
  запросов (семантика отличается от существующего `lsp::handlers::rename`)
- ~~`hover` / `completion` / `signature_help`~~ — **все удалено**: AI agent не делает
  «позиционно-чувствительную» семантику, вместо этого `lookup_symbol` по имени

**Время загрузки World**: при запуске сервера сканируется `yaoxiang.toml` и
`src/**/*.yx` по `--project-root`, повторное использование уже реализованного в LSP-017
API `World::load_*` для однократной загрузки `World.documents`. **Не добавляются новые
lib API**.

### Контракт инструментов

**Ввод**: описывается через JSON Schema, каждое поле имеет `description` + `examples`
(LLM автоматически понимает).

**Вывод**: структурированный JSON, единообразно с полем `schemaVersion: "1.0"`:

```jsonc
// Успешный ответ
{
  "schemaVersion": "1.0",
  "isError": false,
  "content": [
    { "type": "json", "json": { /* данные, специфичные для инструмента */ } }
  ]
}

// Диагностика возвращается структурированно (не считается ошибкой инструмента)
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

// Инструментальная ошибка (например, parse_source получил недопустимый UTF-8)
{
  "schemaVersion": "1.0",
  "isError": true,
  "content": [{ "type": "text", "text": "MCP-INVALID-INPUT: source не является допустимым UTF-8" }],
  "errorCode": "MCP-INVALID-INPUT"
}
```

**Система ошибок**:

- **Диагностика (diagnostic)**: ошибки парсинга/типов, используется RFC-013
  (`E0001` и т.д.) — **не является ошибкой tool**
- **Инструментальные ошибки**: с префиксом `MCP-`
  (`MCP-INVALID-INPUT`, `MCP-PROJECT-NOT-FOUND`, `MCP-INTERNAL`) — считается
  `isError: true`
- **panic/crash**: JSON-RPC `-32603 Internal error`, сервер **не завершается**

**Правила разрешения путей** (применяется к `workspace_root` в `lookup_symbol` /
`find_references`, к `file_paths` в `typecheck`):

1. Флаг командной строки `--project-root <dir>` имеет высший приоритет (переопределяет
   по умолчанию)
2. Иначе: поиск `yaoxiang.toml` вверх от cwd до корня файловой системы (по RFC-015)
3. Иначе: cwd сам
4. `file_paths` должен находиться внутри корня проекта (защита от обхода); выход за
   границы → `MCP-PATH-OUTSIDE-PROJECT`

### Транспортный уровень

**stdio (по умолчанию)**:

```bash
yaoxiang mcp
# После запуска читает JSON-RPC из stdin, пишет в stdout, stderr для логов
```

Конфигурация AI agent (Claude Code `.mcp.json` / Continue `config.json`):

```jsonc
{
  "mcpServers": {
    "yaoxiang": {
      "command": "yaoxiang",
      "args": ["mcp", "--project-root", "${workspaceFolder}"],
    },
  },
}
```

**streamable HTTP (опционально)**:

```bash
yaoxiang mcp --http --addr 127.0.0.1:7325  # один HTTP порт, новая спецификация MCP
yaoxiang mcp --http --sse --addr 127.0.0.1:7325  # совместимость со старым SSE (v0.10)
```

**Ограничения безопасности**:

- **Только loopback** (127.0.0.1 / ::1); привязка к публичной сети явно отклоняется с
  ошибкой и завершением
- HTTP **без аутентификации** (loopback по умолчанию доверенный); в будущем добавится
  `--require-token <hex>`
- Режим stdio дочернего процесса естественно изолирован (родительский процесс
  контролирует права)

### Многопроцессность и конкурентность

Каждый процесс `yaoxiang mcp` содержит один `World`, не разделяемый между собой:

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

**Конфликты портов**: конфигурация AI agent «запуск дочернего процесса» — естественно
нулевые конфликты портов. В режиме HTTP пользователь сам управляет распределением
портов. **Изоляция World**: каждый процесс имеет независимое состояние LSP
синхронизации — сбой одного MCP процесса **не влияет** на LSP/другие MCP процессы.
**future Sessions**: в v2 рассматривается分发 (множество `Session` в одном процессе),
**данный RFC это не реализует**.

## Детальный дизайн

### Структуры данных

Новый `src/mcp/project.rs`:

```rust
pub struct ProjectRoot {
    /// Абсолютный путь
    pub root: PathBuf,
    /// Источник стратегии определения корня проекта при загрузке
    pub source: ProjectRootSource,
}

pub enum ProjectRootSource {
    CliFlag,           // yaoxiang mcp --project-root
    AutoDetected,      // Поиск yaoxiang.toml вверх
    FallbackCwd,       // fallback к cwd
}

pub struct ResolvedPath {
    /// Относительный путь от корня проекта (рекомендуется для чтения AI)
    pub relative: String,
    /// Разрешенный абсолютный путь (для операций World)
    pub absolute: PathBuf,
}

impl ProjectRoot {
    /// Разрешение "file_path" в безопасный путь — защита от обхода
    pub fn resolve(&self, file_path: &str) -> Result<ResolvedPath, McpError>;
}
```

Singleton `ProjectRoot` + автоматическая генерация tool schema в `src/mcp/schema.rs`:

```rust
pub struct ProjectRoot {
    /// Абсолютный путь (обязательно с `yaoxiang.toml` или fallback)
    pub root: PathBuf,
    pub source: ProjectRootSource,
}

impl ProjectRoot {
    /// Определение один раз при CLI запуске, кэшируется в контексте `McpServer` — все
    /// инструменты переиспользуют
    pub fn detect(cli_override: Option<PathBuf>) -> Result<Self, McpError>;
}
```

Tool schema генерируется автоматически из input struct с помощью crate `schemars`,
чтобы избежать ручного написания JSON Schema с последующим расхождением:

```rust
#[derive(Deserialize, schemars::JsonSchema)]
pub struct ParseSourceInput {
    /// Полный фрагмент исходного кода YaoXiang — **НЕ** сохраняется на диск, чистый
    /// transient
    pub source: String,
    pub tab_size: Option<u32>,
}
```

**В tool schema `parse_source` / `format_source` нет поля `file_path`** — эти два
инструмента принимают только строковый источник, не участвуют в семантике проекта.
`lookup_symbol` / `find_references` / `typecheck` принимают `workspace_root` или
`file_paths` (обязательность см. в таблице инструментов).

### Изменения компилятора

| Модуль                                 | Изменения                                                               |
| -------------------------------------- | ------------------------------------------------------------------------ |
| `src/lsp/world.rs`                     | **Нулевые изменения** — MCP при запуске вызывает уже существующий API
| `World::load_*` для однократной загрузки рабочей области |                                                |
| `src/lsp/handlers/workspace_symbol.rs` | **Нулевые изменения** — `mcp/tools/lookup.rs` оборачивает и преобразует
| `query` во входные параметры LSP |                                                               |
| `src/lsp/handlers/references.rs`       | **Нулевые изменения** — аналогично                                       |
| `src/lsp/handlers/formatter.rs`        | **Нулевые изменения** — format_source вызывает напрямую                  |
| `src/main.rs`                          | Добавить ветку подкоманды `Mcp`                                          |
| `Cargo.toml`                           | Добавить feature `mcp-server` (или всегда в main binary)               |
| `src/util/diagnostic/`                 | **Нулевые изменения** (RFC-017 уже реализовано)                          |

**Ключевое ограничение**: `src/mcp/` **не разрешено** обратно зависеть от приватных
символов `src/lsp/` — только через публичные API `crate::lsp::` для вызова handlers.

### Обратная совместимость

- ✅ **Полная обратная совместимость**: новая подкоманда `yaoxiang mcp`, не меняет
  никакого существующего поведения `yaoxiang` / `yaoxiang lsp`
- ✅ **LSP server не трогается**: все возможности, API, внутреннее состояние,
  реализованные в RFC-017, неизменны
- ✅ **Публичные API lib crate не трогаются**: все `pub` пути неизменны; MCP только
  потребляет существующие API — **ноль** новых `pub` методов

### Интеграция с существующими системами

| Существующий модуль                    | Способ интеграции MCP                                                   |
| -------------------------------------- | ----------------------------------------------------------------------- |
| `src/frontend/lexer`                   | parse_source напрямую вызывает lexer                                    |
| `src/frontend/core/parser`             | parse_source напрямую вызывает parser; при неудаче создает `Missing*`
| узлы (RFC-017) |                                                                      |
| `src/frontend/core/typecheck/inference/*` | typecheck переиспользует режим `collect_diagnostics` (RFC-017 §проблема 1) |     |
| `src/middle/`                          | typecheck запускает все middle passes (анализ зависимостей и т.д.)     |
| `src/lsp/world.rs`                     | При запуске вызывается API `World::load_*` (уже есть); World **не**
| принимает «виртуальные документы» |                                                    |
| `src/lsp/handlers/workspace_symbol.rs` | `mcp/tools/lookup.rs` оборачивает, преобразует `query: String` в
| входные параметры LSP (поиск по имени) |                                                    |
| `src/lsp/handlers/references.rs`       | `mcp/tools/find_refs.rs` оборачивает, преобразует `query: String` во
| входные параметры LSP |                                                   |
| `src/lsp/handlers/formatter.rs`        | `mcp/tools/format.rs` вызывает напрямую (если не реализовано, добавить
| `formatter::format_with_diff`) |                                                    |
| `src/util/i18n/`                       | Сообщения об ошибках идут через многоязыковые ресурсные файлы (zh-CN/en) |

### Обработка ошибок

| Источник                               | Обработка                                                                                      |
| -------------------------------------- | ---------------------------------------------------------------------------------------------- |
| Ошибки парсинга                       | `Diagnostic{code:"E0xxx", severity, message, span}` (**не tool ошибка**, возвращается в
| content) |                                                                              |
| Ошибки типов                           | Аналогично                                                                        |
| `file_paths` за границами (`typecheck` tool) | tool-level ошибка `MCP-PATH-OUTSIDE-PROJECT`                                           |
| `source` с недопустимым UTF-8          | tool-level ошибка `MCP-INVALID-INPUT`                                                         |
| panic инструмента                      | JSON-RPC `-32603 Internal error`; сервер **не завершается**                                  |
| Клиент отправляет не JSON-RPC          | Немедленное закрытие потока (stdio EOF), перезапуск = новая сессия                         |

Уровни серьёзности диагностики используют RFC-017 (уже реализовано) `enum ErrorKind {
Error, Warning, Note }`.

### Стратегия тестирования

| Уровень          | Тестирование                                                                                  |
| ---------------- | --------------------------------------------------------------------------------------------- |
| **Unit**         | `src/mcp/project.rs::resolve` обход путей, валидация схемы в `src/mcp/schema.rs`              |
| **Integration**  | mock stdio: запустить сервер, отправить JSON-RPC в stdin, прочитать ответ из stdout, сравнить
| с fixture |                                                                    |
| **E2E**          | Запуск реального процесса `yaoxiang mcp`, цепочка вызовов инструментов в стиле Claude Code:
| parse → исправить → format → typecheck |                                          |
| **Fuzz**         | `cargo-fuzz` для парсинга JSON-RPC MCP (libFuzzer harness)                                   |

Каждый tool должен иметь минимум 1 happy path + 1 scenario с диагностикой + 1
tool-error scenario integration тест.

## Компромиссы

### Преимущества

- **Очень низкая стоимость повторного использования**: `World` / `Session` / `handlers`
  / сбор диагностики уже реализованы (RFC-017), данный RFC — «добавить MCP обёртку»
- **AI-First интерфейс**: контракт инструментов в 3-5 раз интуитивнее LSP; LLM напрямую
  читает schema
- **Изоляция процессов**: отдельно от LSP сессий редактора и других MCP процессов,
  **нулевая конкуренция за блокировки**
- **stdio дружелюбность**: все主流ные AI agent используют режим дочернего процесса по
  умолчанию, нулевая конфигурация для подключения
- **YAGNI пройден**: данный RFC удаляет Resources, Sessions, межпроцессное состояние,
  удалённый MCP — v2 откроет это

### Недостатки

- **Расщепление протоколов**: будущая эволюция LSP / MCP / DAP как трёх отдельных
  протоколов создаёт стоимость поддержания согласованности
- **HTTP режим как гражданин второго сорта**: ограничение loopback позиционирует как
  локальный инструмент, удалённые сценарии требуют перепроектирования в v2
- **Повторные расходы на parse**: когда AI многократно微调ирует исходный код и вызывает
  `parse_source`, происходит повторный lexer+parser. **Смягчение**: `DocumentCache` из
  RFC-017 может ускорить повторный парсинг того же source на диске; чистый transient
  source проходит парсинг один раз неизбежно
- **Стоимость покрытия тестами**: 5 инструментов × 3 сценария = минимум 15 integration
  тестов в начале

## Альтернативные решения

| Решение                                | Почему не выбрано                                                      |
| -------------------------------------- | ----------------------------------------------------------------------- |
| **Процесс с двумя протоколами внутри**
| (LSP+MCP listener сосуществуют)       | stdin/stdout может использовать только один потребитель; HTTP тоже
| должен сосуществовать — сложность > выгоды |                                          |
| **MCP как мост LSP-client**            | Ещё один слой IPC; LSP по дизайну не поддерживает поиск символов по
| имени — возможности, нужные MCP, LSP не может提供 |                                      |
| **Через gRPC / свой протокол**         | Отклонение от фактического стандарта; в сообществе уже есть MCP SDK
| (TypeScript, Python, Rust) с экосистемой |                                             |
| **Повторное использование всех
| возможностей LSP handler** (набор инструментов L3) | Большой объём работы по адаптации position↔intent; предельная
| полезность убывает |                                                            |
| **В первой версии только HTTP** (без
| stdio)                                 | Claude Code / Continue и др. по умолчанию используют stdio, порог
| входа слишком высок    |                                                         |

## Стратегия реализации

### Зависимости

- **Сильная зависимость**: реализация LSP из RFC-017 (уже реализовано)
- **Сильная зависимость**: система кодов ошибок из RFC-013 (уже реализовано)
- **Сильная зависимость**: определение корня проекта из RFC-014 / RFC-015 (частично
  реализовано)
- **Новые зависимости** (Rust crates):
  - `mcp-rust-sdk` (待评估, см.
    [modelcontextprotocol/rust-sdk](https://github.com/modelcontextprotocol/rust-sdk))
  - `tokio` (**уже есть**, optional feature)
  - `axum` (для режима HTTP) или напрямую `hyper` — 待评估
- **Ноль изменений языковой спецификации**: чистое инкрементальное добавление
  инструментальной цепочки

### Этапы реализации

| Этап                        | Содержание                                                                                                                                                                                                                   | Оценка времени |
| --------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | -------------- |
| **v0.8.x (MVP)**            | `src/mcp/{mod.rs, server.rs, transport/stdio.rs, project.rs, schema.rs}` + `parse_source` +
| `format_source` + `lookup_symbol` + `find_references` + `typecheck` (**5 инструментов**) + подкоманда
| `yaoxiang mcp` + `World::load_*` при запуске | **3-4 недели** |
| **v0.9.x (YaoXiang Intelligence)** | `+ explain_diagnostic` (**прямой вызов** `render_explain_output`) + `+ list_imports`
| (обёртка над `ModuleGraph::validate_imports`) + unit/integration тесты                                | **1-2 недели** |
| **v0.10.x (AST + HTTP)**     | `+ rename_symbol` (**NEW** `src/middle/rename.rs`, AST перезапись) + streamable HTTP transport +
| performance tuning (parse_source P99 < 100ms)                                                    | **2-3 недели** |

**Почему 3 этапа**: MVP сначала проверяет работоспособность stdio + 5 инструментов и
валидность дизайна интерфейса; v0.9.x добавляет низкорисковые инструменты «YaoXiang
specific» без адаптации для проверки корректности интеграции; v0.10.x открывает новый
высокорисковый модуль «AST перезапись» (отдельная PR review более сфокусирована).

### Риски

1. **Активность поддержки `mcp-rust-sdk`**: выпущен только в 2025 году, API может
   сильно измениться. **Смягчение**: если окажется нестабильным, написать лёгкий
   JSON-RPC 2.0 + tool dispatcher самостоятельно (< 500 строк)
2. **Повторные расходы на parse**: когда AI многократно微调ирует исходный код и вызывает
   `parse_source`, происходит повторный lexer+parser. **Смягчение**: `DocumentCache` из
   RFC-017 может ускорить повторный парсинг того же source на диске; чистый transient
   source проходит парсинг один раз неизбежно
3. **Совместимость схемы AI agent**: разные agent имеют разную строгость MCP schema.
   **Смягчение**: использование crate `schemars` для автоматической генерации schema из
   Rust input structs, ноль ручного расхождения
4. **Межплатформенное разрешение путей**: Windows чувствителен к регистру путей, UNC
   пути, границы `\\`. **Смягчение**: для разрешения путей использовать `camino::Utf8Path`
   вместо `std::path`
5. **MCP tool schema и LSP входные параметры не 1:1**: LSP `workspace_symbol` принимает
   `(query)`; при передаче внутрь LSP нужно обернуть в позицию+URI чтобы
   существующий handler мог переиспользовать. **Смягчение**: адаптация в
   `mcp/tools/lookup.rs`, детали инкапсулированы на стороне MCP
6. **`rename_symbol` AST перезапись и семантика LSP `rename` различаются**: LSP
   `textDocument/rename` это URI + позиция + new_name → WorkspaceEdit; MCP
   `rename_symbol` это source + old_name + new_name → новый source. **Нельзя
   переиспользовать напрямую**. **Смягчение**: реализовать отдельно
   `src/middle/rename.rs`, scope-aware перезапись ссылок, реализация не мешает LSP
   handler

## Открытые вопросы

- [ ] Выбор `mcp-rust-sdk` / самостоятельная реализация? (@Chen Xu: сначала оценить
  rust-sdk версию за июнь, потом решить)
- [ ] Путь аутентификации HTTP? (RFC для v0.10)
- [ ] Нужно ли при запуске MCP выводить `tools/list` для主动ного обнаружения
  инструментов AI? (Требуется стандартом MCP, **реализовать по умолчанию**)
- [ ] Должен ли `typecheck` поддерживать `mode: "fast|full"` (fast = только подмножество
  текущего файла, full = вся рабочая область)?
- [ ] Реалистичен ли performance budget parse_source P99 < 100ms? (Нужен benchmark
  фактических затрат `DocumentCache` из RFC-017 в режиме source-string)

## Ссылки

- [RFC-017: Дизайн поддержки Language Server Protocol (LSP)](../accepted/017-lsp-support.md)
- [RFC-013: Дизайн спецификации кодов ошибок](../accepted/013-error-code-specification.md)
- [RFC-014: Дизайн системы управления пакетами](../accepted/014-package-manager.md)
- [RFC-015: Дизайн системы конфигурации YaoXiang](../accepted/015-configuration-system.md)
- [Спецификация MCP](https://modelcontextprotocol.io/)
- [MCP Rust SDK](https://github.com/modelcontextprotocol/rust-sdk)
- [JSON-RPC 2.0](https://www.jsonrpc.org/specification)
- [Спецификация LSP 3.18](https://github.com/microsoft/language-server-protocol/blob/main/specifications/specification-3-18.md)
- [Rust Analyzer](https://rust-analyzer.github.io/) —— Ссылка на интеграцию M2 / MCP
- [Реализация MCP от zed-industries/zed](https://github.com/zed-industries/zed/tree/main/crates/mcp)