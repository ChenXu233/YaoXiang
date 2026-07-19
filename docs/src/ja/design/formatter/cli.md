```markdown
---
title: "yaoxiang format コマンドライン使い方"
description: フォーマットツールのコマンドライン引数と使用方法
---

# コマンドライン使い方

---

## A. コマンドライン使い方

```bash
# ファイルをフォーマット（stdout に出力）
yaoxiang format file.yx

# ファイルがフォーマット済みかチェック
yaoxiang format --dry-run file.yx

# フォーマットしてファイルに書き込む
yaoxiang format -w file.yx

# ディレクトリ内のすべての .yx ファイルをフォーマット
yaoxiang format -w src/
```

---

## B. CLI 引数

| 引数 | 説明 | デフォルト値 |
|------|------|------------|
| `--dry-run` | チェックモード、ファイルを変更しない | false |
| `-w`, `--write` | 書き込みモード、ファイルを変更する | false |
| `--stdout` | stdout に出力 | false |
| `--indent-width` | インデント幅 | 4 |
| `--line-width` | 最大行幅 | 120 |
| `--use-tabs` | タブインデントを使用 | false |
| `--single-quote` | シングルクォートを使用 | false |

---

## C. 参考資料

- [Issue #13: yaoxiang format コードフォーマットツールの実装](https://github.com/ChenXu233/YaoXiang/issues/13)
- [Rustfmt スタイルガイド](https://rust-lang.github.io/rustfmt/)
- [テスト作成規範](../test-specification.md)
```