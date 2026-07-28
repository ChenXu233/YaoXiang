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

Добавление в YaoXiang сервера MCP (Model Context Protocol), позволяющего AI-агентам (Claude Code,
Continue, Cody, Zed и др.) напрямую запрашивать **AST, ошибки парсинга, типы, символы, ссылки и
результаты форматирования** исходного кода YaoXiang. Повторное использование уже реализованного
бэкенда `World` из RFC-017, новая подкоманда `yaoxiang mcp`, один бинарный файл с двумя режимами,
множество процессов с независимыми World.

## Мотивация

### Зачем нужен этот функционал?

RFC-017 позволил YaoXiang **быть** понятным редакторам (hover / goto-def / completion). Но LSP — это
протокол, управляемый **позициями**:

- Каждый запрос сильно зависит от `textDocument` URI + `Position`
- Редактор должен сначала открыть файл, сохранить его, поддерживать постоянное соединение с
  LSP-сервером
- Рабочий процесс AI-агента основан на **фрагментах кода**: в диалоге «вставить кусок кода» для
  вопроса, **без** предварительного сохранения

Клиенты LSP, фактически доступные AI-агентам (vscode-langservers-extracted, проекты типа
`mcp-lsp-bridge`), **переводят только L1**: goto-def, hover. AI хочет:

- «Правильно ли **распарсился** этот код» — нужен parse + полный поток диагностики
- «Как используется этот символ **в файле**» — нужен lookup_symbol по имени
- «Как будет выглядеть **отформатированный** код» — нужен format_source
- «Где **все** ошибки типов» — нужен typecheck всего рабочего пространства

Эти возможности L1-перевода LSP **недоступны**, потому что LSP по дизайну их не поддерживает.

### Текущие проблемы

1. Плохой опыт использования LSP AI-агентами: требуется mock-документов, огромный JSON, сильная
   зависимость от URI
2. В проекте YaoXiang отсутствует интерфейсный слой «AI-First»: люди используют LSP в IDE, AI-агенты
   не могут использовать LSP
3. Claude Code / Continue и другие ведущие AI-агенты уже поддерживают MCP по умолчанию, для YaoXiang
   это пустая экосистема

### Что такое MCP?

MCP (Model Context Protocol) — это протокол вызова инструментов AI-агентами, опубликованный и
открытый Anthropic в 2024-2025 годах, ставший фактическим стандартом (OpenAI, Google, Microsoft,
Zed, Continue, Cody и др. подключились). Особенности:

- Основан на JSON-RPC 2.0 (тот же источник, что и LSP)
- Три примитива: **Tools** (действия), Resources (данные), Prompts (шаблоны)
- Транспорт: `stdio` (дочерний процесс) / streamable `HTTP` / SSE
- Ввод/вывод инструментов имеет **строгую типизацию JSON Schema** (удобно для LLM)
- В 2025-06+ опубликована спецификация streamable HTTP, данный RFC одновременно совместим со старым
  SSE

**Данный RFC использует только примитив Tools** — соответствует «предоставлению услуг» LSP, не
вводит сложность модели файлов Resources.

## Предложение

### Основной дизайн

Один бинарный файл с двумя режимами:

```text
┌─────────────────────────────────────────────────────────┐
│                    yaoxiang (v0.7.7+)                   │
│  ┌─────────────────┐      ┌──────────────────────────┐  │
│  │ yaoxiang lsp    │      │   yaoxiang mcp           │  │
│  │ (stdio JSON-RPC)│      │   (stdio default         │  │
│  │ RFC-017 已实现  │      │    + HTTP 可选)          │  │
│  └────────┬────────┘      └──────────┬───────────────┘  │
│           │                         │                   │
│           ▼                         ▼                   │
│  ┌──────────────────────────────────────────────────┐  │
│  │  共享 lib crate（`yaoxiang`）                      │  │
│  │  src/lsp/{server,session,world}.rs                │  │
│  │  src/frontend/{lexer,parser,core}/...             │  │
│  │  src/middle/...                                   │  │
│  └──────────────────────────────────────────────────┘  │
│                                                          │
│  ┌──────────────────────────────────────────────────┐  │
│  │            src/mcp/  ← 新增                       │  │
│  │  ├── mod.rs          （模块入口 + 启动函数）       │  │
│  │  ├── transport/      （stdio + HTTP/SSE）         │  │
│  │  ├── server.rs       （JSON-RPC 消息循环）         │  │
│  │  ├── tools/          （6 个 tool handler）        │  │
│  │  ├── schema.rs       （输入输出 JSON Schema）     │  │
│  │  └── project.rs      （项目根识别 + 路径解析）    │  │
│  └──────────────────────────────────────────────────┘  │
└─────────────────────────────────────────────────────────┘
```

**Ключевые решения**:

- **Один бинарный файл**: `yaoxiang` переключается через подкоманды; LSP-процесс и MCP-процесс **не
  сосуществуют** в одном runtime
- **Множество процессов с независимыми World**: каждый процесс `yaoxiang mcp` содержит один `World`;
  не влияет на LSP-процесс или другие MCP-процессы (без конкуренции за блокировки, изолированные
  сбои)
- **stdio по умолчанию**: избегание конфликтов портов, нулевая сетевая конфигурация; HTTP как
  опциональный запасной вариант
- **Повторное использование, а не дублирование**: прямое использование lib API `yaoxiang::frontend`
  / `yaoxiang::middle` / `yaoxiang::lsp::handlers`, **без** LSP-client-прокси

### Набор инструментов (8 инструментов, 3 этапа поставки)

Дизайн по принципу «устранение особых случаев + поэтапная поставка»: stateless-инструменты для
чистого исходного кода сначала, инструменты рабочего пространства используют общий LSP World,
инструменты перезаписи AST добавляются отдельно.

| Tool 名称            | 输入                                                                                            | 输出                                                         | 复用                                                          | 阶段        |
| -------------------- | ----------------------------------------------------------------------------------------------- | ------------------------------------------------------------ | ------------------------------------------------------------- | ----------- |
| `parse_source`       | `source: String`, `tab_size?: u32`                                                              | `{ast: Node, diagnostics: Diagnostic[]}`                     | 直接调 `frontend::parse`                                      | v0.8.x      |
| `format_source`      | `source: String`, `tab_size?: u32`                                                              | `{formatted: String, diff: Hunk[]}`                          | 直接调 `formatter::format`                                    | v0.8.x      |
| `lookup_symbol`      | `query: String`, `workspace_root?: String`, `kind?: SymbolKind[]`                               | `{symbols: Symbol[]}`                                        | 复用 `lsp::handlers::workspace_symbol`（按 `query` 模糊匹配） | v0.8.x      |
| `find_references`    | `query: String`, `workspace_root?: String`                                                      | `{locations: Location[]}`                                    | 复用 `lsp::handlers::references`（按 `query` 而非位置）       | v0.8.x      |
| `typecheck`          | `file_paths: String[]`, `project_root: String`                                                  | `{diagnostics: Diagnostic[], summary: Counts}`               | 复用 `lsp::world::typecheck_full`                             | v0.8.x      |
| `explain_diagnostic` | `code: String`（如 `E0001`），`lang?: String`                                                   | `{code, category, title, description, example, help}`        | **直接调** `util::diagnostic::command::render_explain_output` | **v0.9.x**  |
| `list_imports`       | `file_path: String`, `project_root?: String`                                                    | `{imports: [{module, items, is_public}]}`                    | 复用 `middle::passes::module::ModuleGraph::validate_imports`  | **v0.9.x**  |
| `rename_symbol`      | `source: String`, `old_name: String`, `new_name: String`, `scope?: "module" \| "function:name"` | `{source: String, edits: Edit[], diagnostics: Diagnostic[]}` | **新加** `src/middle/rename.rs`（AST 改写）                   | **v0.10.x** |

**Границы 8 инструментов**:

- `parse_source` / `format_source` — **чистый stateless источник**, не входят в World
- `lookup_symbol` / `find_references` — подключают `workspace_root` (если не передано, используется
  `--project-root` при запуске)
- `typecheck` — **обязательный** `file_paths`, гарантирует полноту рабочего пространства
- `explain_diagnostic` — **нулевая зависимость от файлов**, чистый строковый запрос таблицы кодов
  ошибок
- `list_imports` — `file_path` физический файл, выводит результат парсинга импортов этого файла
- `rename_symbol` — **чистая перезапись AST источника**, без LSP-style позиционных запросов
  (семантика отличается от существующего `lsp::handlers::rename`)
- ~~`hover` / `completion` / `signature_help`~~ — **все удалено**: AI-агенты не выполняют
  «позиционно-чувствительную» семантику, вместо этого используется `lookup_symbol` для поиска по
  имени

**Время загрузки World**: при запуске сервера сканируется `yaoxiang.toml` и `src/**/*.yx` по
`--project-root`, повторно используется уже реализованный API `World::load_*` из LSP-017 для
однократной загрузки `World.documents`. **Не** добавляется никаких новых lib API.

### Контракт инструментов

**Ввод**: описывается через JSON Schema, каждое поле имеет `description` + `examples` (LLM
автоматически понимает).

**Вывод**: структурированный JSON, единообразно с полем `schemaVersion: "1.0"`:

```jsonc
// 成功响应
{
  "schemaVersion": "1.0",
  "isError": false,
  "content": [
    { "type": "json", "json": { /* 工具特定数据 */ } }
  ]
}

// 诊断被结构化返回（不视作 tool 错误）
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

// 工具级错误（如 parse_source 接收非法 UTF-8）
{
  "schemaVersion": "1.0",
  "isError": true,
  "content": [{ "type": "text", "text": "MCP-INVALID-INPUT: source 不是合法 UTF-8" }],
  "errorCode": "MCP-INVALID-INPUT"
}
```

**Система ошибок**:

- **Диагностика (diagnostic)**: ошибки парсинга/типов, с использованием RFC-013 (`E0001` и т.д.) —
  **не считается ошибкой tool**
- **Ошибки уровня инструмента**: с префиксом `MCP-` (`MCP-INVALID-INPUT`, `MCP-PROJECT-NOT-FOUND`,
  `MCP-INTERNAL`) — считается `isError: true`
- **panic/crash**: JSON-RPC `-32603 Internal error`, server **не завершает работу**

**Правила разрешения путей** (применимо к `workspace_root` для `lookup_symbol` / `find_references`,
к `file_paths` для `typecheck`):

1. Команда `--project-root <dir>` имеет наивысший приоритет (переопределяет значение по умолчанию)
2. Иначе: поиск `yaoxiang.toml` вверх от cwd до корня файловой системы (по RFC-015)
3. Иначе: сам cwd
4. `file_paths` должен находиться внутри корня проекта (защита от обхода); выход за границы →
   `MCP-PATH-OUTSIDE-PROJECT`

### Транспортный уровень

**stdio (по умолчанию)**:

```bash
yaoxiang mcp
# После запуска читает JSON-RPC из stdin, пишет в stdout, stderr для логов
```

Конфигурация AI-агента (Claude Code `.mcp.json` / Continue `config.json`):

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
yaoxiang mcp --http --addr 127.0.0.1:7325  # Один HTTP-порт, новая спецификация MCP
yaoxiang mcp --http --sse --addr 127.0.0.1:7325  # Совместимость со старым SSE (v0.10)
```

**Ограничения безопасности**:

- **Только loopback** (127.0.0.1 / ::1); привязка к публичной сети явно отклоняется с ошибкой
- HTTP **без аутентификации** (loopback по умолчанию доверенный); в будущем добавление
  `--require-token <hex>`
- Режим stdio дочернего процесса естественно изолирован (родительский процесс контролирует права)

### Многопроцессность и параллелизм

Каждый процесс `yaoxiang mcp` содержит один `World`, не разделяемый:

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

**Конфликт портов**: AI-агенты настраивают «запуск дочернего процесса» — естественно отсутствие
конфликтов портов. В режиме HTTP пользователь сам управляет распределением портов. **Изоляция
World**: каждый процесс имеет независимое состояние LSP-синхронизации — сбой одного MCP-процесса
**не влияет** на LSP/другие MCP-процессы. **future Sessions**: только в v2 рассматривается
диспетчеризация нескольких рабочих пространств (несколько `Session` в одном процессе), **данный RFC
это не делает**.

## Детальный дизайн

### Структуры данных

Новый `src/mcp/project.rs`:

```rust
pub struct ProjectRoot {
    /// 绝对路径
    pub root: PathBuf,
    /// 加载时识别项目根的策略来源
    pub source: ProjectRootSource,
}

pub enum ProjectRootSource {
    CliFlag,           // yaoxiang mcp --project-root
    AutoDetected,      // 向上找 yaoxiang.toml
    FallbackCwd,       // fallback 到 cwd
}

pub struct ResolvedPath {
    /// 相对项目根的相对路径（推荐给 AI 读）
    pub relative: String,
    /// 解析后的绝对路径（用于 World 操作）
    pub absolute: PathBuf,
}

impl ProjectRoot {
    /// 把"file_path"解析为安全路径——防穿越
    pub fn resolve(&self, file_path: &str) -> Result<ResolvedPath, McpError>;
}
```

Синглтон `ProjectRoot` + автогенерация схемы инструментов в `src/mcp/schema.rs`:

```rust
pub struct ProjectRoot {
    /// 绝对路径（必含 `yaoxiang.toml` 或向下兼容回退）
    pub root: PathBuf,
    pub source: ProjectRootSource,
}

impl ProjectRoot {
    /// CLI 启动时识别一次，结果缓存在 `McpServer` 上下文里——所有工具复用
    pub fn detect(cli_override: Option<PathBuf>) -> Result<Self, McpError>;
}
```

Схема инструментов генерируется автоматически из input struct с помощью crate `schemars`, чтобы
избежать ручного расхождения JSON Schema:

```rust
#[derive(Deserialize, schemars::JsonSchema)]
pub struct ParseSourceInput {
    /// 完整 YaoXiang 源码片段——**不**保存到磁盘，纯 transient
    pub source: String,
    pub tab_size: Option<u32>,
}
```

**В схеме инструментов `parse_source` / `format_source` нет поля `file_path`** — эти два инструмента
принимают только строковый источник, не участвуют в семантике проекта. `lookup_symbol` /
`find_references` / `typecheck` принимают `workspace_root` или `file_paths` (обязательность см. в
таблице инструментов).

### Изменения компилятора

| 模块                                   | 改动                                                                     |
| -------------------------------------- | ------------------------------------------------------------------------ |
| `src/lsp/world.rs`                     | **零改动**——MCP 启动时调 LSP 已有的 `World::load_*` API 一次性加载工作区 |
| `src/lsp/handlers/workspace_symbol.rs` | **零改动**——`mcp/tools/lookup.rs` 包一层把 `query` 转 LSP 入参           |
| `src/lsp/handlers/references.rs`       | **零改动**——同上                                                         |
| `src/lsp/handlers/formatter.rs`        | **零改动**——format_source 直接调                                         |
| `src/main.rs`                          | 加 `Mcp` 子命令分支                                                      |
| `Cargo.toml`                           | 加 `mcp-server` feature（或主二进制始终带）                              |
| `src/util/diagnostic/`                 | **零改动**（RFC-017 已落地）                                             |

**Ключевое ограничение**: `src/mcp/` **не** допускает обратной зависимости от приватных символов
`src/lsp/` — только через публичный API `crate::lsp::` для вызова handlers.

### Обратная совместимость

- ✅ **Полная обратная совместимость**: новая подкоманда `yaoxiang mcp`, не изменяет никакое
  существующее поведение `yaoxiang` / `yaoxiang lsp`
- ✅ **LSP server не меняется**: все возможности, API, внутреннее состояние, реализованные в
  RFC-017, остаются неизменными
- ✅ **Публичный API lib crate не меняется**: все пути `pub` остаются неизменными; MCP только
  потребляет существующие API — **ноль** новых методов `pub`

### Интеграция с существующими системами

| 现有模块                                  | MCP 集成方式                                                                 |
| ----------------------------------------- | ---------------------------------------------------------------------------- |
| `src/frontend/lexer`                      | parse_source 直接调 lexer                                                    |
| `src/frontend/core/parser`                | parse_source 直接调 parser；失败产出 `Missing*` 节点（RFC-017）              |
| `src/frontend/core/typecheck/inference/*` | typecheck 复用 `collect_diagnostics` 模式（RFC-017 §问题1）                  |
| `src/middle/`                             | typecheck 跑全部 middle pass（依赖分析等）                                   |
| `src/lsp/world.rs`                        | 启动时调 `World::load_*` API（已有）；World **不**接受任何"虚拟文档"         |
| `src/lsp/handlers/workspace_symbol.rs`    | `mcp/tools/lookup.rs` 包一层，把 `query: String` 转 LSP 入参（按名查）       |
| `src/lsp/handlers/references.rs`          | `mcp/tools/find_refs.rs` 包一层，把 `query: String` 转 LSP 入参              |
| `src/lsp/handlers/formatter.rs`           | `mcp/tools/format.rs` 直接调（若未实现，新加 `formatter::format_with_diff`） |
| `src/util/i18n/`                          | 错误消息走多语种资源文件（zh-CN/en）                                         |

### Обработка ошибок

| 来源                                  | 处理                                                                                       |
| ------------------------------------- | ------------------------------------------------------------------------------------------ |
| 解析错                                | `Diagnostic{code:"E0xxx", severity, message, span}`（**非 tool 错误**，在 content 里返回） |
| 类型错                                | 同上                                                                                       |
| `file_paths` 越界（`typecheck` 工具） | tool 级错误 `MCP-PATH-OUTSIDE-PROJECT`                                                     |
| `source` 非法 UTF-8                   | tool 级错误 `MCP-INVALID-INPUT`                                                            |
| 工具 panic                            | JSON-RPC `-32603 Internal error`；server **不退出**                                        |
| 客户端发非 JSON-RPC                   | 直接断流（stdio EOF），重启即新会话                                                        |

Уровни серьёзности диагностики используют RFC-017 (уже реализованный)
`enum ErrorKind { Error, Warning, Note }`.

### Стратегия тестирования

| 层              | 测试                                                                                    |
| --------------- | --------------------------------------------------------------------------------------- |
| **Unit**        | `src/mcp/project.rs::resolve` 路径穿越、`src/mcp/schema.rs` schema 校验                 |
| **Integration** | mock stdio：起一个 server，stdin 灌 JSON-RPC，stdout 读响应，比对 fixture               |
| **E2E**         | 跑 `yaoxiang mcp` 真进程，Claude Code 风格的工具调用链：parse → 修 → format → typecheck |
| **Fuzz**        | MCP JSON-RPC 解析的 `cargo-fuzz`（libFuzzer harness）                                   |

Каждый tool должен иметь минимум 1 happy path + 1 диагностический сценарий + 1 tool-error сценарий в
integration-тестах.

## Компромиссы

### Преимущества

- **Крайне низкая стоимость повторного использования**: `World` / `Session` / `handlers` / сбор
  диагностики уже реализованы (RFC-017), данный RFC — это «обёртка MCP»
- **AI-First интерфейс**: контракт инструментов в 3-5 раз интуитивнее LSP; LLM напрямую читает
  schema
- **Изоляция множества процессов**: разделение от LSP-сессий редактора и других MCP-процессов,
  **нулевая конкуренция за блокировки**
- **stdio удобство**: все ведущие AI-агенты по умолчанию используют режим дочернего процесса,
  нулевая конфигурация для подключения
- **YAGNI выполнен**: данный RFC убирает Resources, Sessions, межпроцессное состояние, удалённый MCP
  — v2 откроет это

### Недостатки

- **Разделение протоколов**: будущие LSP / MCP / DAP три набора протоколов развиваются независимо,
  стоимость поддержания согласованности
- **HTTP-режим — второй сорт**: ограничение на loopback позиционирует как локальный инструмент,
  удалённые сценарии требуют перепроектирования в v2
- **Повторные затраты на parse**: AI многократно микронастраивает исходный код и многократно
  вызывает `parse_source` с повторным lexer+parser. **Смягчение**: `DocumentCache` из RFC-017 всё
  ещё может ускорить повторный парсинг **дискового** источника с тем же source; для чисто transient
  source однократный парсинг неизбежен
- **Стоимость покрытия тестами**: 5 инструментов × 3 сценария = 15 integration-тестов в начале

## Альтернативные решения

| 方案                                         | 为什么不选                                                           |
| -------------------------------------------- | -------------------------------------------------------------------- |
| **进程内嵌入双协议**（LSP+MCPlistener 共存） | stdin/stdout 只能一个消费者；HTTP 也得并存——复杂度 > 收益            |
| **MCP 作为 LSP-client 桥接**                 | 多一层 IPC；LSP 设计就不支持按名查符号——MCP 想要的能力 LSP 给不了    |
| **走 gRPC / 自定义协议**                     | 偏离事实标准；社区已有 MCP SDK（TypeScript、Python、Rust），自带生态 |
| **复用 LSP handler 全部能力**（L3 工具集）   | 大量 position↔intent 适配工作；边际收益递减                          |
| **首个版本只做 HTTP**（不 stdio）            | Claude Code / Continue 等默认 stdio，门槛过高                        |

## Стратегия реализации

### Зависимости

- **Сильная зависимость**: реализация LSP из RFC-017 (уже реализована)
- **Сильная зависимость**: система кодов ошибок из RFC-013 (уже реализована)
- **Сильная зависимость**: определение корня проекта из RFC-014 / RFC-015 (частично реализовано)
- **Новые зависимости** (Rust crate):
  - `mcp-rust-sdk` (待评估, 参考
    [modelcontextprotocol/rust-sdk](https://github.com/modelcontextprotocol/rust-sdk))
  - `tokio` (**已有**, optional feature)
  - `axum` (HTTP 模式) или `hyper` 直接——待评估
- **零语言规范变化**: чистое приращение toolchain

### Этапы (与 #154 同步)

| 阶段                       | 内容                                                                                                                                                                                                                          | 时长估算   |
| -------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ---------- |
| **v0.8.x (MVP)**           | `src/mcp/{mod.rs, server.rs, transport/stdio.rs, project.rs, schema.rs}` + `parse_source` + `format_source` + `lookup_symbol` + `find_references` + `typecheck`（**5 工具**）+ `yaoxiang mcp` 子命令 + 启动时 `World::load_*` | **3-4 周** |
| **v0.9.x (YaoXiang 智能)** | `+ explain_diagnostic`（**直接调** `render_explain_output`）+ `+ list_imports`（包 `ModuleGraph::validate_imports`） + 单元/集成测试                                                                                          | **1-2 周** |
| **v0.10.x (AST + HTTP)**   | `+ rename_symbol`（**新加** `src/middle/rename.rs`，AST 改写）+ streamable HTTP transport + 性能调优（parse_source P99 < 100ms）                                                                                              | **2-3 周** |

**Почему 3 этапа**: MVP сначала запускает stdio + 5 инструментов для проверки обоснованности дизайна
интерфейса; v0.9.x добавляет маложирские инструменты «специфичные для YaoXiang» с нулевой адаптацией
для проверки корректности интеграции; v0.10.x открывает новый высокорисковый модуль «перезапись AST»
(отдельный PR review более сфокусирован).

### Риски

1. **`mcp-rust-sdk` активность поддержки**: выпущен только в 2025 году, API может резко измениться.
   **Смягчение**: если оценить как нестабильный, написать лёгкий JSON-RPC 2.0 + tool dispatcher
   самостоятельно (< 500 строк)
2. **Повторные затраты на parse**: AI многократно микронастраивает исходный код и многократно
   вызывает `parse_source` с повторным lexer+parser. **Смягчение**: `DocumentCache` из RFC-017 всё
   ещё может ускорить повторный парсинг **дискового** источника с тем же source; для чисто transient
   source однократный парсинг неизбежен
3. **Совместимость схемы AI-агента**: разные агенты имеют разную строгость MCP schema.
   **Смягчение**: использование crate `schemars` для автоматической генерации схемы из Rust input
   struct, нулевое расхождение ручной работы
4. **Мультиплатформенное разрешение путей**: Windows пути нечувствительны к регистру, UNC-пути,
   границы `\\`. **Смягчение**: использовать `camino::Utf8Path` вместо `std::path` для разрешения
   путей
5. **MCP tool schema и LSP ввод не 1:1**: LSP `workspace_symbol` принимает `(query)`; при передаче
   во внутренний LSP нужно обернуть в позицию+URI, чтобы существующий handler мог использовать
   повторно. **Смягчение**: адаптационный слой в `mcp/tools/lookup.rs`, детали инкапсулированы на
   MCP-стороне
6. **`rename_symbol` перезапись AST отличается от LSP `rename` семантикой**: LSP
   `textDocument/rename` — это URI + позиция + new_name → WorkspaceEdit; MCP `rename_symbol` — это
   source + old_name + new_name → новый source. **Нельзя использовать повторно напрямую**.
   **Смягчение**: реализовать отдельно `src/middle/rename.rs`, scope-aware перезапись ссылок, не
   мешает реализации LSP handler

## Открытые вопросы

- [ ] `mcp-rust-sdk` выбор / самостоятельная реализация? (@Chen Xu: сначала оценка rust-sdk версии
      за июнь, потом решение)
- [ ] Путь HTTP-аутентификации? (RFC v0.10 откроет снова)
- [ ] Нужно ли MCP при запуске выводить `tools/list` для активного обнаружения AI? (Требование
      стандарта MCP, **реализовать по умолчанию**)
- [ ] Должен ли `typecheck` поддерживать `mode: "fast|full"` (fast = только подмножество текущего
      файла, full = всё рабочее пространство)?
- [ ] Реалистичен ли бюджет производительности parse_source P99 < 100ms? (Нужен benchmark
      фактических затрат `DocumentCache` из RFC-017 в режиме source-string)

## Ссылки

- [RFC-017: Дизайн поддержки протокола языкового сервера (LSP)](./accepted/017-lsp-support.md)
- [RFC-013: Дизайн спецификации кодов ошибок](./accepted/013-error-code-specification.md)
- [RFC-014: Дизайн системы управления пакетами](./accepted/014-package-manager.md)
- [RFC-015: Дизайн системы конфигурации YaoXiang](./accepted/015-configuration-system.md)
- [Спецификация MCP](https://modelcontextprotocol.io/)
- [MCP Rust SDK](https://github.com/modelcontextprotocol/rust-sdk)
- [JSON-RPC 2.0](https://www.jsonrpc.org/specification)
- [Спецификация LSP 3.18](https://github.com/microsoft/language-server-protocol/blob/main/specifications/specification-3-18.md)
- [Rust Analyzer](https://rust-analyzer.github.io/) —— Ссылка на интеграцию M2 / MCP
- [Реализация MCP от zed-industries/zed](https://github.com/zed-industries/zed/tree/main/crates/mcp)
