---
title: "RFC 012: F-String テンプレート文字列"
status: "Accepted"
author: "Chen Xu"
created: "2025-01-27"
updated: "2026-07-05"
issue: "#124"
---

# RFC 012: F-String テンプレート文字列

## 概要

YaoXiang 言語に f-string テンプレート文字列機能を追加し、変数の補間、式の評価、フォーマット出力をサポートします。f-string は Python スタイルの構文（`f"..."` 接頭辞）を使用し、文字列内で `{expression}` 構文によって式を埋め込み、コンパイル時に効率的な文字列操作に変換されます。

> **注意**: f-string の構文と動作は Python と一貫しており、具体的な仕様は [Python 公式ドキュメント](https://docs.python.org/3/tutorial/inputoutput.html#formatted-string-literals)を参照してください。

## 動機

### なぜこの機能が必要なのか？

現在の YaoXiang の文字列連結方法はやや煩雑です：

```yaoxiang
# 現状：+ による連結
name = "Alice"
age = 30
message = "Hello ".concat(name).concat(", age: ").concat(age.to_string())
print(message)

# または format 関数を使用
message2 = format("Hello {}, age: {}", name, age)
```

### 現在の問題点

1. **可読性が低い**：文字列の連結とフォーマットに複数の呼び出しが必要で、コードが冗長になる
2. **エラーが発生しやすい**：手動で型変換を処理する必要があり、`.to_string()` を忘れることがある
3. **パフォーマンスの懸念**：複数回の文字列連結がパフォーマンスに影響する可能性がある
4. **表現力が不足**：複雑な式を直感的に文字列に埋め込むことができない

## 提案

### 中核設計

f-string を新しい文字列リテラルの接頭辞として導入し、以下をサポートします：
- **変数の補間**：`f"Hello {name}"`
- **式の評価**：`f"Sum: {x + y}"`
- **フォーマット指定子**：`f"Pi: {pi:.2f}"`
- **型安全性**：コンパイル時に式型をチェック

### 例

```yaoxiang
# 基本的な補間
name = "Alice"
greeting = f"Hello {name}"  # "Hello Alice"

# 式の補間
x = 10
y = 20
result = f"Sum: {x + y}"    # "Sum: 30"

# フォーマット指定子
pi = 3.14159
formatted = f"Pi: {pi:.2f}"  # "Pi: 3.14"

# 複雑な式
items = [1, 2, 3]
s = f"Count: {len(items)}, sum: {sum(items)}"  # "Count: 3, sum: 6"

# オブジェクトメソッド呼び出し
user = User("Bob", 25)
bio = f"Name: {user.name}, age: {user.get_age()}"
```

### 構文の変化

| 以前 | 以後 |
|------|------|
| `"Hello ".concat(name)` | `f"Hello {name}"` |
| `format("Value: {}", value)` | `f"Value: {value}"` |
| `format("Pi: {:.2f}", pi)` | `f"Pi: {pi:.2f}"` |

### 構文仕様

```
FStringLiteral ::= 'f' '"' FStringContent* '"'
FStringContent ::= FStringChar | EscapeSequence | FStringInterpolation
FStringInterpolation ::= '{' Expression (':' FormatSpec)? '}'
FormatSpec      ::= [width] ['.' precision] type
width           ::= digit+
precision       ::= digit+
type            ::= 'b' | 'c' | 'd' | 'e' | 'E' | 'f' | 'F' | 'g' | 'G' | 'n' | 'o' | 's' | 'x' | 'X' | '%'
```

## 詳細設計

### 構文解析

コンパイラは字句解析段階で `f` 接頭辞の文字列リテラルを識別し、波括弧内の式とオプションのフォーマット指定子を解析します。

### 変換戦略

f-string はコンパイル時に効率的な文字列操作に変換されます：

**単純な補間**：
```yaoxiang
f"Hello {name}"
```
以下に変換されます：
```yaoxiang
"Hello ".concat(name.to_string())
```

**式の補間**：
```yaoxiang
f"Sum: {x + y}"
```
以下に変換されます：
```yaoxiang
"Sum: ".concat((x + y).to_string())
```

**フォーマット指定子**：
```yaoxiang
f"Pi: {pi:.2f}"
```
以下に変換されます：
```yaoxiang
format("Pi: {:.2f}", pi)
```

**複数の補間**：
```yaoxiang
f"Hello {name}, you are {age} years old"
```
以下に変換されます：
```yaoxiang
"Hello ".concat(name.to_string()).concat(", you are ").concat(age.to_string()).concat(" years old")
```

### 型システムへの影響

- 補間式は `Stringable` インターフェースを実装している必要があります（基本型と文字列には自動実装）
- フォーマット指定子は型が対応するフォーマットをサポートしていることを要求します
- コンパイラは式型とフォーマット規則の整合性をチェックします

### コンパイラの変更

| コンポーネント | 変更内容 |
|------|------|
| lexer | f 接頭辞の認識、文字列内補間構文の解析 |
| parser | FStringLiteral 構文ノードの追加 |
| typecheck | 補間式型のチェック、フォーマット規則の検証 |
| codegen | 文字列連結またはフォーマット呼び出しコードの生成 |

### 後方互換性

- ✅ 完全な後方互換性
- 既存の文字列リテラル `"..."` は変更されません
- f-string は新しい構文であり、既存のコードには影響しません

## トレードオフ

### 利点

1. **構文が簡潔**：ボイラープレートコードを減らし、可読性を向上
2. **型安全性**：コンパイル時チェックにより、ランタイムエラーを削減
3. **パフォーマンス最適化**：コンパイラが文字列連結を最適化可能
4. **表現力が強力**：任意の式とフォーマットをサポート
5. **学習コストが低い**：Python エコシステムと一致

### 欠点

1. **コンパイラの複雑度増加**：新しい構文解析と変換ロジックが必要
2. **構文の曖昧性**：既存の文字列構文との区別が必要
3. **デバッグの課題**：コンパイル後のコードがソースコード構造と異なる

## 代替案

| 案 | 選択しなかった理由 |
|------|--------------|
| 変数の補間のみサポート | 複雑なフォーマット要求を満たせない |
| 関数型スタイル `format(...)` を使用 | 構文が十分に簡潔でない |
| v2.0 まで延期 | ユーザーが文字列の利便性について明確な要求を持っている |
| バッククォートやその他の接頭辞を使用 | Python エコシステムと一致しない |

## 実装戦略

### 段階分け

1. **段階 1 (v0.9)**:
   - 基本的な f-string 構文のサポート
   - 変数と単純な式の補間
   - 基本的な型変換

2. **段階 2 (v1.0)**:
   - フォーマット指定子のサポート
   - 複雑な式の補間
   - パフォーマンス最適化

3. **段階 3 (v1.1)**:
   - デバッグ情報の強化
   - エラーメッセージの改善
   - より多くのフォーマットオプション

### 依存関係

- 外部依存なし
- 基本的な型システムのサポートが必要
- 文字列ライブラリの基本機能が必要

### リスク

1. **パフォーマンスリスク**：複数の補間により過度な文字列オブジェクトが生成される可能性がある
   - **緩和策**：コンパイラが隣接する文字列定数のマージを最適化
2. **型チェックの複雑性**：フォーマット指定子の型チェック
   - **緩和策**：Python の実装を参考に、シンプルで直接的なチェックを採用
3. **構文の曖昧性**：`{` と `}` のネスト使用
   - **緩和策**：構文規則を明確化し、ネストを制限

## オープンな問題

- [x] エスケープされた波括弧をサポートするか？Python と一致：二重波括弧で単一波括弧を表す。例えば <code v-pre>{{</code> は <code v-pre>{</code> を、<code v-pre>}}</code> は <code v-pre>}</code> を表す
- [x] カスタムフォーマット関数をサポートするか？Python と一致：`__format__` メソッドによる型のフォーマット動作のカスタマイズをサポート
- [x] フォーマット指定子の完全な仕様は？Python と一致、上記の BNF を参照
- [x] パフォーマンス最適化の具体戦略は？Python と一致：ランタイム連結、特別な最適化は不要
- [x] エラー診断のベストプラクティスは？Python と一致：エラー報告時に元の f-string の内容と位置を表示

## 付録

### 付録A：フォーマット指定子リファレンス

| 型 | 指定子 | 例 | 出力 |
|------|--------|------|------|
| 整数 | `d` | `f"{42:d}"` | "42" |
| 浮動小数点 | `f` | `f"{3.14:.2f}"` | "3.14" |
| 指数表記 | `e` | `f"{1000:e}"` | "1.000000e+03" |
| 文字列 | `s` | `f"{name:s}"` | "Alice" |
| 十六進数 | `x` | `f"{255:x}"` | "ff" |

### 付録B：使用シナリオの例

```yaoxiang
# ログ記録
log(level: String, msg: String, count: Int) = () => {
    timestamp = get_timestamp()
    print(f"[{timestamp}] {level}: {msg} (count: {count})")
}

# JSON 構築
json = "{\n    \"name\": \"".concat(user.name).concat("\",\n    \"age\": ")
    .concat(user.age.to_string()).concat(",\n    \"email\": \"")
    .concat(user.email).concat("\"\n}")

# SQL クエリ構築（SQL インジェクションリスクに注意）
query = f"SELECT * FROM users WHERE age > {min_age} AND status = '{status}'"

# デバッグ情報
debug_info = f"Point({x:.2f}, {y:.2f}) at {timestamp}"

# 条件付きフォーマット
status_msg = if is_active {
    f"User {name} is active"
} else {
    f"User {name} is inactive"
}
```

---

## 参考文献

- [Python f-strings](https://docs.python.org/3/tutorial/inputoutput.html#formatted-string-literals)
- [Rust format! macro](https://doc.rust-lang.org/std/macro.format.html)
- [JavaScript template literals](https://developer.mozilla.org/en-US/docs/Web/JavaScript/Reference/Template_literals)
- [C# interpolated strings](https://docs.microsoft.com/en-us/dotnet/csharp/language-reference/tokens/interpolated)

---

## ライフサイクルと結末

RFC には以下の状態遷移があります：

```
┌─────────────┐
│   草案      │  ← 著者による作成
└──────┬──────┘
       │
       ▼
┌─────────────┐
│  審査中     │  ← コミュニティによる議論
└──────┬──────┘
       │
       ├──────────────────┐
       ▼                  ▼
┌─────────────┐    ┌─────────────┐
│  承認済み   │    │   却下      │
└──────┬──────┘    └──────┬──────┘
       │                  │
       ▼                  ▼
┌─────────────┐    ┌─────────────┐
│   accepted/ │    │    rfc/     │
│ (正式設計)  │    │ (元の位置)  │
└─────────────┘    └─────────────┘
```