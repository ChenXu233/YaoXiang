---
title: 'RFC-002: Слой реализации IO для типов ресурсов на основе libuv'
status: 'Черновик'
author: 'Чэньсюй'
created: '2026-01-05'
updated: '2026-07-05'
issue: '#102'
---

# RFC-002: Слой реализации IO для типов ресурсов на основе libuv

> **Ссылки**:
>
> - [RFC-024: Модель параллелизма на основе spawn-блоков](../accepted/024-concurrency-model.md)
> - [RFC-008: Дизайн разделения модели параллелизма Runtime и планировщика](../accepted/008-runtime-concurrency-model.md)
> - [RFC-009: Дизайн модели владения](../accepted/009-ownership-model.md)
> - [Спецификация модели параллелизма](../../../reference/language-spec/concurrency.md)

## Резюме

Данный документ определяет слой реализации IO для YaoXiang: предоставление кроссплатформенных
возможностей IO на основе libuv, в качестве базовой реализации системы типов ресурсов из RFC-024.

**Ключевое позиционирование**:

```
RFC-024: Определение типов ресурсов (FilePath, HttpUrl, DBUrl, Console)
    ↓ Используется
RFC-002: Реализация IO для типов ресурсов (на основе libuv)
    ↓ Нижний уровень
libuv: Кроссплатформенный движок IO (цикл событий + пул потоков)
```

**Чем НЕ является**:

- ❌ Не является «прозрачной асинхронностью» — пользователь явно управляет параллелизмом через
  spawn-блоки
- ❌ Не является «автоматической асинхронизацией» — IO-операции должны явно вызываться внутри
  spawn-блоков
- ❌ Не означает «разработчику не нужно беспокоиться о деталях реализации» — система типов ресурсов
  обеспечивает безопасность параллелизма

**Чем ЯВЛЯЕТСЯ**:

- ✅ Слой реализации IO для типов ресурсов (FilePath, HttpUrl, DBUrl, Console)
- ✅ Унификация кроссплатформенного IO (libuv обрабатывает различия Windows/Linux/macOS)
- ✅ Архитектура с общим циклом событий (один цикл событий libuv обрабатывает все IO)
- ✅ Интеграция с системой типов ресурсов из RFC-024

## Мотивация

### Зачем нужен libuv?

RFC-024 определяет систему типов ресурсов:

- `FilePath` — путь файловой системы
- `HttpUrl` — HTTP-эндпоинт
- `DBUrl` — соединение с базой данных
- `Console` — стандартный вывод

Эти типы ресурсов требуют базовой реализации IO. libuv предоставляет:

| Потребность               | libuv предоставляет                                              |
| ------------------------- | ---------------------------------------------------------------- |
| Кроссплатформенный IO     | Единый API для Windows/Linux/macOS                               |
| Асинхронные возможности   | Общий цикл событий, централизованная обработка IO всех worker-ов |
| Пул потоков               | Выделенный пул потоков для блокирующих операций                  |
| Безопасность параллелизма | Однопоточный цикл событий, естественное отсутствие гонок         |

### Связь с RFC-024

```
┌─────────────────────────────────────────────────────────┐
│  RFC-024: Модель параллелизма                            │
│  - spawn {} блоки (явный параллелизм)                    │
│  - Определение типов ресурсов (FilePath, HttpUrl,        │
│    DBUrl, Console)                                       │
│  - Обнаружение конфликтов ресурсов (автоматическая       │
│    сериализация для одного пути)                         │
└─────────────────────────────────────────────────────────┘
                          ↓ Используется
┌─────────────────────────────────────────────────────────┐
│  RFC-002: Реализация IO для типов ресурсов               │
│  - FilePath → файловый IO libuv                          │
│  - HttpUrl → сетевой IO libuv                            │
│  - DBUrl → пул соединений с базой данных                 │
│  - Console → сериализация стандартного вывода            │
└─────────────────────────────────────────────────────────┘
                          ↓ Нижний уровень
┌─────────────────────────────────────────────────────────┐
│  libuv: Кроссплатформенный движок IO                     │
│  - Цикл событий                                          │
│  - Пул потоков                                           │
│  - Единый кроссплатформенный API                         │
└─────────────────────────────────────────────────────────┘
```

---

## Предложение

### 1. Архитектура libuv

#### 1.1 Архитектура с общим циклом событий

```
┌─────────────────────────────────────────────────────────┐
│                    Runtime                               │
├─────────────────────────────────────────────────────────┤
│                                                         │
│  ┌─────────────┐  ┌─────────────┐  ┌─────────────┐    │
│  │  Worker 0   │  │  Worker 1   │  │  Worker N   │    │
│  │  Вычислит.  │  │  Вычислит.  │  │  Вычислит.  │    │
│  │  задачи     │  │  задачи     │  │  задачи     │    │
│  └──────┬──────┘  └──────┬──────┘  └──────┬──────┘    │
│         │                │                │            │
│         └────────────────┼────────────────┘            │
│                          ↓                              │
│  ┌─────────────────────────────────────────────────┐  │
│  │      Цикл событий libuv (выделенный поток)       │  │
│  │      Обрабатывает все IO-операции                │  │
│  └─────────────────────────────────────────────────┘  │
│                                                         │
└─────────────────────────────────────────────────────────┘
```

**Ключевые особенности**:

- Один общий цикл событий libuv (работает в выделенном потоке)
- IO-операции всех worker-ов отправляются в этот общий цикл событий
- Однопоточный цикл событий естественно исключает гонки
- Высокая эффективность использования ресурсов — не нужно создавать цикл событий для каждого
  worker-а

#### 1.2 Механизмы обеспечения безопасности параллелизма

| Возможность libuv           | Соответствие в YaoXiang                          | Безопасность параллелизма     |
| --------------------------- | ------------------------------------------------ | ----------------------------- |
| Однопоточный цикл событий   | Последовательное выполнение в spawn-блоке        | Естественное отсутствие гонок |
| Изоляция пула потоков       | Блокирующие операции не блокируют основной поток | Отсутствие общего состояния   |
| Асинхронные обратные вызовы | Планировщик DAG управляет зависимостями          | Детерминированное выполнение  |

### 2. Сопоставление IO для типов ресурсов

#### 2.1 FilePath → файловый IO libuv

```rust
// std.io 模块（基于 libuv）
pub struct IoModule;

impl StdModule for IoModule {
    fn exports(&self) -> Vec<NativeExport> {
        vec![
            // 文件操作 → libuv fs_* API
            NativeExport::new("read_file", "std.io.read_file",
                "(path: FilePath) -> String", native_read_file),
            NativeExport::new("write_file", "std.io.write_file",
                "(path: FilePath, content: String) -> Bool", native_write_file),
            NativeExport::new("append_file", "std.io.append_file",
                "(path: FilePath, content: String) -> Bool", native_append_file),
            // Console 操作 → libuv tty API
            NativeExport::new("print", "std.io.print",
                "(...args) -> ()", native_print),
            NativeExport::new("println", "std.io.println",
                "(...args) -> ()", native_println),
        ]
    }
}

// libuv 文件 IO 实现
fn native_read_file(args: &[RuntimeValue], ctx: &mut NativeContext) -> Result<RuntimeValue, ExecutorError> {
    let path = extract_file_path(args)?;

    // 提交到 libuv 事件循环
    // libuv 异步读取文件
    // 返回结果
    ctx.uv_loop.fs_read(path)
}
```

#### 2.2 HttpUrl → сетевой IO libuv

```rust
// std.net 模块（基于 libuv）
pub struct NetModule;

impl StdModule for NetModule {
    fn exports(&self) -> Vec<NativeExport> {
        vec![
            // HTTP 操作 → libuv http API
            NativeExport::new("http_get", "std.net.http_get",
                "(url: HttpUrl) -> Response", native_http_get),
            NativeExport::new("http_post", "std.net.http_post",
                "(url: HttpUrl, body: String) -> Response", native_http_post),
        ]
    }
}

// libuv 网络 IO 实现
fn native_http_get(args: &[RuntimeValue], ctx: &mut NativeContext) -> Result<RuntimeValue, ExecutorError> {
    let url = extract_http_url(args)?;

    // 提交到 libuv 事件循环
    // libuv 异步 HTTP 请求
    // 返回结果
    ctx.uv_loop.http_get(url)
}
```

#### 2.3 DBUrl → пул соединений с базой данных

```rust
// std.db 模块（基于 libuv）
pub struct DbModule;

impl StdModule for DbModule {
    fn exports(&self) -> Vec<NativeExport> {
        vec![
            // 数据库操作 → libuv 线程池
            NativeExport::new("query", "std.db.query",
                "(url: DBUrl, sql: String) -> Rows", native_query),
        ]
    }
}

// libuv 数据库 IO 实现
fn native_query(args: &[RuntimeValue], ctx: &mut NativeContext) -> Result<RuntimeValue, ExecutorError> {
    let url = extract_db_url(args)?;
    let sql = extract_sql(args)?;

    // 提交到 libuv 线程池
    // 数据库查询在线程池执行
    // 完成后回调通知主线程
    ctx.uv_loop.db_query(url, sql)
}
```

#### 2.4 Console → сериализация стандартного вывода

```rust
// Console 操作自动串行化（RFC-024 资源类型规则）
// 所有 Console 操作在同一线程内顺序执行
fn native_print(args: &[RuntimeValue], ctx: &mut NativeContext) -> Result<RuntimeValue, ExecutorError> {
    let output = format_args(args);

    // Console 操作串行化
    // libuv tty 写入
    ctx.uv_loop.tty_write(output)
}
```

### 3. Интеграция со spawn-блоками

#### 3.1 С точки зрения пользователя

```yaoxiang
# 资源类型定义（RFC-024）
FilePath: Resource
HttpUrl: Resource

# IO 操作（RFC-002 实现）
File.read: (FilePath) -> String
HTTP.get: (HttpUrl) -> Response

# 用户显式并发（RFC-024）
(a, b) = spawn {
    read_file("data.txt"),      # 资源类型 FilePath，底层 libuv
    fetch("http://example.com") # 资源类型 HttpUrl，底层 libuv
}
# 编译器：FilePath 和 HttpUrl 无冲突，可以并行
```

#### 3.2 Анализ на этапе компиляции

```
编译器分析 spawn 块：
1. 识别资源类型操作
2. 检测资源冲突（同路径/同 URL 自动串行）
3. 生成 DAG 执行计划
4. 标记 IO 节点（提交到 libuv）
```

#### 3.3 Выполнение во время выполнения

```
运行时执行 spawn 块：
1. Worker 0 提交 IO 任务 → 共享事件循环
2. Worker 1 提交 IO 任务 → 共享事件循环
3. 事件循环统一处理所有 IO 操作
4. IO 完成后通知对应的 Worker
5. Worker 继续执行后续任务
```

### 4. Трёхуровневая архитектура Runtime и libuv

| Уровень          | Использование libuv | Асинхронные возможности      | Сценарии применения                      |
| ---------------- | ------------------- | ---------------------------- | ---------------------------------------- |
| Embedded Runtime | Без libuv           | Без асинхронности            | WASM, игровые скрипты                    |
| Standard Runtime | Общий цикл событий  | Асинхронный IO               | Веб-сервисы, конвейеры данных            |
| Full Runtime     | Общий цикл событий  | Асинхронный IO + параллелизм | Научные вычисления, массовый параллелизм |

**Embedded Runtime**: без libuv, немедленное выполнение, без асинхронных возможностей.

**Standard Runtime**: общий цикл событий libuv, все IO-операции обрабатываются асинхронно.

**Full Runtime**: общий цикл событий libuv, многопоточный параллелизм + асинхронный IO.

---

## Детальное проектирование

### 1. Структура Rust-привязок

```rust
// libuv 绑定模块
pub mod uv {
    // 事件循环
    pub struct UvLoop {
        loop_handle: *mut uv_loop_t,
    }

    // 文件操作
    pub trait FileOps {
        fn fs_read(&self, path: &str) -> Result<String, UvError>;
        fn fs_write(&self, path: &str, content: &str) -> Result<(), UvError>;
        fn fs_append(&self, path: &str, content: &str) -> Result<(), UvError>;
    }

    // 网络操作
    pub trait NetOps {
        fn http_get(&self, url: &str) -> Result<Response, UvError>;
        fn http_post(&self, url: &str, body: &str) -> Result<Response, UvError>;
    }

    // 数据库操作
    pub trait DbOps {
        fn db_query(&self, url: &str, sql: &str) -> Result<Rows, UvError>;
    }

    // Console 操作
    pub trait ConsoleOps {
        fn tty_write(&self, data: &str) -> Result<(), UvError>;
    }
}
```

### 2. Структура модулей стандартной библиотеки

```
src/std/
├── io.rs          # FilePath IO（基于 libuv）
├── net.rs         # HttpUrl IO（基于 libuv）
├── db.rs          # DBUrl IO（基于 libuv）
├── console.rs     # Console IO（基于 libuv）
└── mod.rs         # 模块注册
```

### 3. Интеграция с планировщиком DAG

```rust
// IO 节点接口（RFC-008 定义）
trait IoScheduler {
    // 提交 IO 任务，返回句柄
    fn submit_io(&self, task: IoTask) -> IoHandle;

    // IO 完成时由 libuv 调用，唤醒 DAG 节点
    fn on_io_complete(&self, handle: IoHandle);
}

// libuv 实现
impl IoScheduler for UvLoop {
    fn submit_io(&self, task: IoTask) -> IoHandle {
        match task.resource_type {
            ResourceType::FilePath => self.fs_read(task.path),
            ResourceType::HttpUrl => self.http_get(task.url),
            ResourceType::DBUrl => self.db_query(task.url, task.sql),
            ResourceType::Console => self.tty_write(task.data),
        }
    }

    fn on_io_complete(&self, handle: IoHandle) {
        // 通知 DAG 调度器唤醒下游节点
        self.dag_scheduler.wake_dependents(handle.node_id);
    }
}
```

---

## Компромиссы

### Преимущества

1. **Кроссплатформенная унификация**: libuv обрабатывает различия Windows/Linux/macOS
2. **Асинхронные возможности IO**: общий цикл событий обрабатывает все IO, без необходимости в
   async/await
3. **Безопасность параллелизма**: однопоточный цикл событий естественно исключает гонки
4. **Эффективность ресурсов**: один цикл событий, малые накладные расходы памяти
5. **Совместимость с RFC-024**: система типов ресурсов обеспечивает безопасность параллелизма
6. **Зрелость и стабильность**: libuv проверен в крупных масштабах в Node.js

### Недостатки

1. **Зависимость от C-библиотеки**: требуется привязка к C-библиотеке libuv
2. **Ограничения самоосвоения (bootstrap)**: после самоосвоения может потребоваться замена на
   нативную реализацию YaoXiang
3. **Поддержка WASM**: требуется дополнительная работа по адаптации

---

## Альтернативные варианты

| Вариант           | Почему не выбран                                                                               |
| ----------------- | ---------------------------------------------------------------------------------------------- |
| Rust std::io      | Синхронный блокирующий, не может работать со spawn-блоками для асинхронности                   |
| tokio             | Разработан для Rust async/await, не совместим с моделью явного параллелизма YaoXiang           |
| mio               | Предоставляет только низкоуровневые асинхронные примитивы, не имеет высокоуровневых IO-функций |
| Реализация с нуля | Сложно и подвержено ошибкам, не сравнимо со зрелостью libuv                                    |

---

## Стратегия реализации

### Этапы

1. **Этап 1 (v0.3)**: привязки libuv, базовый файловый IO
2. **Этап 2 (v0.5)**: сетевой IO, поддержка HTTP
3. **Этап 3 (v0.7)**: IO базы данных, пул соединений
4. **Этап 4 (v1.0)**: адаптация WASM, оптимизация производительности

### Зависимости

- RFC-024 (модель параллелизма) → Завершён
- RFC-008 (архитектура Runtime) → Завершён
- RFC-009 (модель владения) → Завершён
- RFC-011 (система дженериков) → Завершён

---

## Запись проектных решений

| Решение                              | Выбор                                 | Причина                                                                  | Дата       |
| ------------------------------------ | ------------------------------------- | ------------------------------------------------------------------------ | ---------- |
| Слой реализации IO                   | libuv                                 | Кроссплатформенность, асинхронные возможности, безопасность параллелизма | 2025-01-05 |
| Позиционирование                     | Слой реализации IO для типов ресурсов | Интеграция с системой типов ресурсов из RFC-024                          | 2026-06-16 |
| Архитектура цикла событий            | Общий цикл событий                    | Высокая эффективность ресурсов, отсутствие дублирования                  | 2026-06-16 |
| Безопасность параллелизма            | Однопоточный цикл событий             | Естественное отсутствие гонок, совместимость с RFC-024                   | 2026-06-16 |
| Переписывание стандартной библиотеки | std.io/std.net на основе libuv        | Кроссплатформенная унификация, асинхронные возможности                   | 2026-06-16 |

---

## Открытые вопросы

- [ ] Схема адаптации libuv для среды WASM
- [ ] Проектирование пула соединений с базой данных
- [ ] Полная реализация HTTP-клиента
- [ ] Кроссплатформенная согласованность событий файловой системы
- [ ] Проектирование механизма тайм-аута для сетевого IO
- [ ] Стратегия замены libuv после самоосвоения

---

## Ссылки

### Официальная документация YaoXiang

- [RFC-024 Модель параллелизма](../accepted/024-concurrency-model.md)
- [RFC-008 Архитектура Runtime](../accepted/008-runtime-concurrency-model.md)
- [RFC-009 Модель владения](../accepted/009-ownership-model.md)
- [Спецификация модели параллелизма](../../../reference/language-spec/concurrency.md)

### Внешние ссылки

- [Официальная документация libuv](https://docs.libuv.org/)
- [Цикл событий Node.js](https://nodejs.org/en/docs/guides/event-loop-timers-and-nexttick/)
- [Rust-привязки libuv](https://github.com/libuv/libuv)

---

## Жизненный цикл и судьба

| Статус       | Расположение             | Описание                  |
| ------------ | ------------------------ | ------------------------- |
| **Черновик** | `docs/design/rfc/draft/` | На повторном рассмотрении |
