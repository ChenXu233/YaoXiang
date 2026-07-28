---
title: 'YaoXiang コードフォーマット仕様'
description: YaoXiang コードフォーマットツール（yaoxiang fmt）の動作仕様定義、フォーマット原則と適用範囲を規定
---

# YaoXiang コードフォーマット仕様

このドキュメントは `yaoxiang fmt` コードフォーマットツールの動作仕様を定義します。すべてのフォーマット動作は本仕様に従う必要があります。

---

## 目次

- [原則](#原則)
- [適用範囲](#適用範囲)
- [フォーマットルール](./formatting-rules/index.md)
- [設定オプション](./configuration.md)
- [コメントの保持](./comments.md)
- [エラー処理](./error-handling.md)
- [CLI使用方法](./cli.md)

---

## 原則

**原則 1：フォーマットはべき等である。** すでにフォーマットされたコードに再度フォーマットを適用しても、出力は入力と完全に同一である必要がある。

```rust
// ルール: format(format(code)) == format(code)
assert_eq!(format_source(input, &opts), format_source(&format_source(input, &opts).unwrap(), &opts).unwrap());
```

**原則 2：フォーマットは意味を変更しない。** フォーマット前後のコードは同一の AST（抽象構文木）を持たなければならない。

**原則 3：フォーマットはすべてのコメントを保持する。** 単一行コメント、ブロックコメント、ドキュメントコメントは保持され、削除も変更もしてはならない。

**原則 4：設定優先順位。**
設定優先順位は以下のように規定する：CLI 引数 > プロジェクトレベル設定（`yaoxiang.toml`）> ユーザーレベル設定（`~/.config/yaoxiang/config.toml`）> デフォルト値。

## 適用範囲

本仕様はすべての `.yx` ソースファイルのフォーマットに適用される。
