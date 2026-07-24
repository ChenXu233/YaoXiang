title: "yaoxiang format コマンドラインユース"
description: フォーマットのコマンドラインパラメータと使用方法
---

# コマンドラインユース

---

## A. コマンドラインユース

```bash
# ファイルをフォーマットする（stdout に出力）
yaoxiang format file.yx

# ファイルがフォーマット済みかチェックする
yaoxiang format --dry-run file.yx

# フォーマットしてファイルに書き込む
yaoxiang format -w file.yx

# ディレクトリ内のすべての .yx ファイルをフォーマットする
yaoxiang format -w src/
```

---

## B. CLI パラメータ

| パラメータ | 説明 | デフォルト値 |
|------|------|--------|
| `--dry-run` | チェックモード、ファイルを変更しない | false |
| `-w`, `--write` | 書き込みモード、ファイルを変更する | false |
| `--stdout` | stdout に出力する | false |
| `--indent-width` | インデント幅 | 4 |
| `--line-width` | 最大行幅 | 120 |
| `--use-tabs` | tab インデントを使用 | false |
| `--single-quote` | 単一引用符を使用 | false |

---

## C. 参考資料

- [Issue #13: yaoxiang format コードフォーマットの実装](https://github.com/ChenXu233/YaoXiang/issues/13)
- [Rustfmt スタイルガイド](https://rust-lang.github.io/rustfmt/)
- [テスト記述仕様](../test-specification.md)