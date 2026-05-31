---
title: Инкрементная проверка
description: YaoXiang check Дизайн инкрементной проверки
---

# Инкрементная проверка

## Описание проблемы

В режиме watch любое изменение файла приводит к повторной проверке всех файлов (полная повторная проверка), а дебаунсинг использует busy-wait (проверка каждые 50 мс), что вызывает бесполезную нагрузку на CPU.

## Решение

Использовать `CheckSession` для управления состоянием инкрементной проверки, используя `ModuleDependencyGraph::affected_modules` для повторной проверки только затронутых файлов.

## Процесс реализации

```text
Первоначальная проверка:
  Полная проверка → Кэшированный граф зависимостей + результат проверки каждого модуля

Изменение файла:
  1. affected_modules(changed_files) → Найти затронутые модули
  2. Только повторно разобрать и проверить затронутые модули
  3. Обновить кэш и граф зависимостей
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

- Режим watch по-прежнему использует busy-wait дебаунсинг (`Instant::now()` + `recv_timeout` в `command.rs`)
- `check_incremental` внутренне всё ещё вызывает `check_files_with_diagnostics` (полные пути), не используя по-настоящему инкремент

## Будущая работа

- A2/P1: Заменить busy-wait дебаунсинг на `HotReloader`
- P2/P3: Подключить режим watch к `CheckSession` для реализации настоящей инкрементной проверки
- T9: Тест корректности инкрементной проверки