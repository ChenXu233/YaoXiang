---
title: Инкрементальная проверка
description: Дизайн инкрементальной проверки YaoXiang check
---

# Инкрементальная проверка

## Описание проблемы

В режиме watch любое изменение файла вызывает повторную проверку всех файлов (полная повторная
проверка), а дебаунсинг использует busy-wait (проверка каждые 50 мс), что приводит к холостому
вращению CPU.

## Решение

Использовать `CheckSession` для управления состоянием инкрементальной проверки, использовать
`ModuleDependencyGraph::affected_modules` для повторной проверки только затронутых файлов.

## Процесс реализации

```text
首次检查：
  全量检查 → 缓存依赖图 + 每个模块的检查结果

文件变更：
  1. affected_modules(changed_files) → 找出受影响模块
  2. 只重新解析和检查受影响模块
  3. 更新缓存和依赖图
```

## CheckSession

```rust
pub struct CheckSession {
    dep_graph: ModuleDependencyGraph,
    cache: ModuleCache,
    all_files: Vec<PathBuf>,
}

impl CheckSession {
    pub fn check_all(&mut self, files: &[PathBuf]) -> Result<CheckResult>;
    pub fn check_incremental(&mut self, changed_files: &[PathBuf]) -> Result<CheckResult>;
}
```

## Известные ограничения

- Режим watch по-прежнему использует busy-wait дебаунсинг (`Instant::now()` + `recv_timeout` в
  `command.rs`)
- `check_incremental` внутренне всё ещё вызывает `check_files_with_diagnostics` (полный путь), не
  используя по-настоящему инкрементальный подход

## Будущая работа

- A2/P1: заменить busy-wait дебаунсинг на `HotReloader`
- P2/P3: подключить `CheckSession` в режим watch для настоящей инкрементальной проверки
- T9: тесты корректности инкрементальной проверки
