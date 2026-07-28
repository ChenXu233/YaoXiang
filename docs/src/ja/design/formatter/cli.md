---
title: "yaoxiang format コマンドライン用法"
description: フォーマッタのコマンドライン引数と使用方法
---

# コマンドライン用法

---

## A. コマンドライン用法

```bash
# フォーマットファイル（stdout に出力）
yaoxiang format file.yx

# ファイルがフォーマット済みかチェック
yaoxiang format --dry-run file.yx

# フォーマットしてファイルに書き込み
yaoxiang format -w file.yx

# ディレクトリ内のすべての .yx ファイルをフォーマット
yaoxiang format -w src/
```

---

## B. CLI 引数

| 引数             | 説明                     | デフォルト値 |
| ---------------- | ------------------------ | ------------ |
| `--dry-run`      | チェックモード、ファイルは変更しない | false        |
| `-w`, `--write`  | 書き込みモード、ファイルを変更する   | false        |
| `--stdout`       | stdout に出力                     | false        |
| `--indent-width` | インデント幅                       | 4            |
| `--line-width`   | 最大行幅                           | 120          |
| `--use-tabs`     | タブインデントを使用               | false        |
| `--single-quote` | 単一引用符を使用                   | false        |

---

## C. 参考資料

- [Issue #13: yaoxiang format コードフォーマッタの実装](https://github.com/ChenXu233/YaoXiang/issues/13)
- [Rustfmt スタイルガイド](https://rust-lang.github.io/rustfmt/)
- [テスト記述仕様](../test-specification.md)
