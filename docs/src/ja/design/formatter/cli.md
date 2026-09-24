---
title: 'yaoxiang format コマンドライン使用方法'
description: フォーマッターツールのコマンドライン引数と使用方法
---

# コマンドライン使用方法

---

## A. コマンドライン使用方法

```bash
# ファイルをフォーマット（stdoutに出力）
yaoxiang format file.yx

# ファイルがフォーマット済みかチェック
yaoxiang format --dry-run file.yx

# ファイルをフォーマットして書き込む
yaoxiang format -w file.yx

# ディレクトリ内のすべての .yx ファイルをフォーマット
yaoxiang format -w src/
```

---

## B. CLI 引数

| 引数             | 説明                                 | デフォルト値 |
| ---------------- | ------------------------------------ | ------------ |
| `--dry-run`      | チェックモード、ファイルを変更しない | false        |
| `-w`, `--write`  | 書き込みモード、ファイルを変更する   | false        |
| `--stdout`       | stdoutに出力                         | false        |
| `--indent-width` | インデント幅                         | 4            |
| `--line-width`   | 最大行幅                             | 120          |
| `--use-tabs`     | タブインデントを使用                 | false        |
| `--single-quote` | シングルクォートを使用               | false        |

---

## C. 参考資料

- [Issue #13: yaoxiang format コードフォーマッターツールの実装](https://github.com/ChenXu233/YaoXiang/issues/13)
- [Rustfmt スタイルガイド](https://rust-lang.github.io/rustfmt/)
- [テスト作成規範](../../dev/test-specification.md)
