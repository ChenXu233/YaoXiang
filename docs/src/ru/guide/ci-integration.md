---
title: Руководство по интеграции с CI
description: Интеграция yaoxiang check и yaoxiang fmt в конвейеры CI/CD
---

# Руководство по интеграции с CI

Интеграция инструментов статической проверки и форматирования YaoXiang в конвейеры CI/CD для обеспечения качества кода.

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
        run: yaoxiang check --color never --no-progress

      - name: Format check
        run: yaoxiang fmt --check
```

## GitLab CI

```yaml
yaoxiang-check:
  image: rust:latest
  script:
    - curl -fsSL https://yaoxiang.dev/install.sh | sh
    - export PATH="$HOME/.yaoxiang/bin:$PATH"
    - yaoxiang check --color never --no-progress
    - yaoxiang fmt --check
  rules:
    - if: $CI_MERGE_REQUEST_IID
    - if: $CI_COMMIT_BRANCH == "main"
    - if: $CI_COMMIT_BRANCH == "dev"
```

## Коды завершения

| Код завершения | Значение | Поведение в CI |
|----------------|----------|----------------|
| `0` | Нет ошибок | Успешно |
| `1` | Обнаружены ошибки при проверке | Ошибка |
| `2` | Файлы `.yx` не найдены | Зависит от конфигурации |

## Парсинг JSON-вывода

Используйте `--json` для получения вывода в машиночитаемом формате:

```bash
yaoxiang check --json | jq '.error_count'
```

## Лучшие практики

1. **Разделение проверки и форматирования**: запускайте `check` и `fmt --check` отдельно для удобства поиска проблем
2. **Используйте `--no-progress`**: в среде CI индикатор прогресса не нужен
3. **Используйте `--color never`**: избегайте ANSI-кодов цвета, загрязняющих логи
4. **Кэшируйте зависимости**: используйте механизм кэширования CI для ускорения сборки