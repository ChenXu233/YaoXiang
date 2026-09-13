---
title: 'Инкрементальная проверка'
description: 'Дизайн инкрементальной проверки YaoXiang'
---

# Инкрементальная проверка

## Описание проблемы

В режиме watch любое изменение файла приводит к повторной проверке всех файлов (полная
перепроверка), а для дебаунса используется busy-wait (проверка каждые 50 мс), что вызывает холостую
нагрузку на CPU.

## Решение

Использовать `CheckSession` для управления состоянием инкрементальной проверки, применяя
`ModuleDependencyGraph::affected_modules` для повторной проверки только затронутых файлов.

## Процесс реализации

```text
Первая проверка:
  Полная проверка → кэш графа зависимостей + результаты проверки каждого модуля

Изменение файла:
  1. affected_modules(changed_files) → найти затронутые модули
  2. Повторно разобрать и проверить только затронутые модули
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

- Режим watch по-прежнему использует busy-wait дебаунс (`Instant::now()` + `recv_timeout` в
  `command.rs`)
- `check_incremental` внутри всё ещё вызывает `check_files_with_diagnostics` (полный путь), не
  используя по-настоящему инкрементальность

## Будущая работа

- A2/P1: заменить busy-wait дебаунс на `HotReloader`
- P2/P3: интегрировать `CheckSession` в режим watch для реализации настоящей инкрементальной
  проверки
- T9: тесты корректности инкрементальной проверки
