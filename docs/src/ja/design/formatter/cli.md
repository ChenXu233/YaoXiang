---
title: "yaoxiang fmt コマンドライン用法"
description: フォーマットのコマンドラインパラメータと使用方法
---

# コマンドライン用法

---

## A. コマンドライン用法

```bash
# ファイルをフォーマット（stdout に出力）
yaoxiang fmt file.yx

# ファイルがフォーマット済みかチェック
yaoxiang fmt --check file.yx

# フォーマットしてファイルに書き込み
yaoxiang fmt --write file.yx

# ディレクトリ内のすべての .yx ファイルをフォーマット
yaoxiang fmt --write src/
```

---

## B. CLI パラメータ

| パラメータ | 説明 | デフォルト値 |
|------|------|--------|
| `--check` | チェックモード、ファイルは変更しない | false |
| `--write` | 書き込みモード、ファイルを変更する | false |
| `--stdout` | stdout に出力 | false |
| `--indent-width` | インデント幅 | 4 |
| `--line-width` | 最大行幅 | 120 |
| `--use-tabs` | タブを使用 | false |
| `--single-quote` | シングルクォートを使用 | false |

---

## C. 参考資料

- [Issue #13: yaoxiang fmt コードフォーマットの実装](https://github.com/ChenXu233/YaoXiang/issues/13)
- [Rustfmt スタイルガイド](https://rust-lang.github.io/rustfmt/)
- [テスト記述規則](../test-specification.md)