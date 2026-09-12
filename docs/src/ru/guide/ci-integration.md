---
title: 'Руководство по интеграции CI'
description: 'Интеграция yx check и yx format в конвейер CI/CD'
---

# Руководство по интеграции CI

Интеграция инструментов статической проверки и форматирования YaoXiang в конвейер CI/CD для
обеспечения качества кода.

## GitHub Actions

```yaml
name: YaoXiang CI

on:
  push:
    branches: [main, dev]
  pull_request:
    branches: [main]

jobs:
  check:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4

      - name: Install YaoXiang
        run: |
          curl -fsSL https://yaoxiang.dev/install.sh | sh
          echo "$HOME/.yaoxiang/bin" >> $GITHUB_PATH

      - name: Type check
        run: yx check --color never --no-progress

      - name: Format check
        run: yx format --dry-run .
```

## GitLab CI

```yaml
yaoxiang-check:
  image: rust:latest
  script:
    - curl -fsSL https://yaoxiang.dev/install.sh | sh
    - export PATH="$HOME/.yaoxiang/bin:$PATH"
    - yx check --color never --no-progress
    - yx format --dry-run .
  rules:
    - if: $CI_MERGE_REQUEST_IID
    - if: $CI_COMMIT_BRANCH == "main"
    - if: $CI_COMMIT_BRANCH == "dev"
```

## Коды выхода

| Код выхода | Значение                   | Поведение CI                  |
| ---------- | -------------------------- | ----------------------------- |
| `0`        | Нет ошибок                 | Успех                         |
| `1`        | Проверка обнаружила ошибки | Сбой                          |
| `2`        | Файлы `.yx` не найдены     | В зависимости от конфигурации |

## Разбор JSON-вывода

Используйте `--json` для получения машиночитаемого вывода:

```bash
yx check --json | jq '.error_count'
```

## Лучшие практики

1. **Параметр пути**: `yx check` по умолчанию проверяет текущую директорию, также можно указать
   путь: `yx check src/`
2. **Разделение проверки и форматирования**: запускайте `check` и `format --dry-run` отдельно для
   удобства локализации проблем
3. **Используйте `--no-progress`**: в среде CI индикатор прогресса не нужен
4. **Используйте `--color never`**: чтобы избежать загрязнения логов ANSI-цветами
5. **Кеширование зависимостей**: используйте механизм кеширования CI для ускорения сборки
