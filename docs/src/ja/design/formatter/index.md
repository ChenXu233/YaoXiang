```markdown
---
title: "YaoXiang コードフォーマット仕様"
description: YaoXiang コードフォーマットツール（yaoxiang fmt）の動作仕様总則、フォーマット原則と適用範囲を定義
---

# YaoXiang コードフォーマット仕様

本文書は `yaoxiang fmt` コードフォーマットツールの動作仕様を定義します。すべてのフォーマット動作は本仕様に従う必要があります。

---

## ディレクトリ

- [原則](#原則)
- [適用範囲](#適用範囲)
- [フォーマット規則](./formatting-rules/index.md)
- [設定オプション](./configuration.md)
- [コメント保持](./comments.md)
- [エラー処理](./error-handling.md)
- [コマンドライン用法](./cli.md)

---

## 原則

**原則 1：フォーマットは幂等である。** すでにフォーマットされたコードに再度フォーマットをかけた場合、出力は入力と完全に同一でなければならない。

```rust
// ルール：format(format(code)) == format(code)
assert_eq!(format_source(input, &opts), format_source(&format_source(input, &opts).unwrap(), &opts).unwrap());
```

**原則 2：フォーマットは意味論を変えない。** フォーマット前後のコードは同一の AST（抽象構文木）を保持しなければならない。

**原則 3：フォーマットはすべてのコメントを保持する。** 単一行コメント、複数行コメント、ドキュメントコメントは保持され、削除も変更もしてはならない。

**原則 4：設定の優先順位。** 設定優先順位チェーン：CLI 引数 > プロジェクトレベル設定（`yaoxiang.toml`）> ユーザーレベル設定（`~/.config/yaoxiang/config.toml`）> デフォルト値。

## 適用範囲

本仕様はすべての `.yx` ソースファイルのフォーマットに適用されます。
```