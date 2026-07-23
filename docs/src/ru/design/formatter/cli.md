---
title: "Использование командной строки yaoxiang format"
description: Параметры командной строки и инструкции по использованию инструмента форматирования
---

# Использование командной строки

---

## A. Использование командной строки

```bash
# Форматирование файла (вывод в stdout)
yaoxiang format file.yx

# Проверка, отформатирован ли файл
yaoxiang format --dry-run file.yx

# Форматирование и запись в файл
yaoxiang format -w file.yx

# Форматирование всех файлов .yx в каталоге
yaoxiang format -w src/
```

---

## B. Параметры CLI

| Параметр | Описание | Значение по умолчанию |
|------|------|--------|
| `--dry-run` | Проверочный режим, без изменения файлов | false |
| `-w`, `--write` | Режим записи, изменение файлов | false |
| `--stdout` | Вывод в stdout | false |
| `--indent-width` | Ширина отступа | 4 |
| `--line-width` | Макс. ширина строки | 120 |
| `--use-tabs` | Использовать табуляцию | false |
| `--single-quote` | Использовать одинарные кавычки | false |

---

## C. Ссылки

- [Issue #13: Реализация инструмента форматирования кода yaoxiang format](https://github.com/ChenXu233/YaoXiang/issues/13)
- [Руководство по стилю Rustfmt](https://rust-lang.github.io/rustfmt/)
- [Спецификация написания тестов](../test-specification.md)