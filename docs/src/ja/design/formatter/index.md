---
title: 'YaoXiang コードフォーマット規約'
description:
  'YaoXiang コードフォーマッタ（yx format）の動作規範総則、フォーマット原則と適用範囲を定義する'
---

# YaoXiang コードフォーマット規約

本文書は `yx format`
コードフォーマッタの動作規範を定義する。すべてのフォーマット動作は本規約に従わなければならない。

---

## 目次

- [原則](#原則)
- [適用範囲](#適用範囲)
- [フォーマット規則](./formatting-rules/index.md)
- [設定オプション](./configuration.md)
- [コメント保持](./comments.md)
- [エラー処理](./error-handling.md)
- [コマンドライン用法](./cli.md)

---

## 原則

**原則 1：フォーマットは冪等である。**
既にフォーマットされたコードに再度フォーマットを適用した場合、出力は入力と完全に同一でなければならない。

```rust
// 規則：format(format(code)) == format(code)
assert_eq!(format_source(input, &opts), format_source(&format_source(input, &opts).unwrap(), &opts).unwrap());
```

**原則 2：フォーマットは意味を変更しない。**
フォーマット前後のコードは同一の AST（抽象構文木）を持たなければならない。

**原則 3：フォーマットはすべてのコメントを保持する。**
単一行コメント、複数行コメント、ドキュメントコメントは保持されなければならず、削除または変更してはならない。

**原則 4：設定の優先順位。**
設定の優先順位チェーンは次の通り：CLI 引数 > プロジェクトレベル設定（`yaoxiang.toml`）> ユーザーレベル設定（`~/.config/yaoxiang/config.toml`）> デフォルト値。

## 適用範囲

本規約はすべての `.yx` ソースファイルのフォーマットに適用される。
