---
title: "RFC 012: F-String テンプレート文字列"
status: "受入済み"
author: "Chen Xu"
created: "2025-01-27"
updated: "2026-02-12"
---

# RFC 012: F-String テンプレート文字列

## 概要

YaoXiang 言語に f-string テンプレート文字列機能を追加し、変数補間、式評価、フォーマット出力をサポートする。f-string は Python スタイルの構文（`f"..."` プレフィックス）を使用し、文字列内で `{expression}` 構文により式を埋め込み、コンパイル時に効率的な文字列操作に変換する。

> **注意**: f-string 構文と動作は Python と一貫性を保ち、具体的な仕様は [Python 公式ドキュメント](https://docs.python.org/3/tutorial/inputoutput.html#formatted-string-literals) を参照。

## 動機

### なぜこの機能が必要なのか？

現在の YaoXiang の文字列連結方法は面倒である：

```yaoxiang
# 現状：+ による連結
name = "Alice"
age = 30
message = "Hello ".concat(name).concat(", age: ").concat(age.to_string())
print(message)

# または format 関数を使用
message2 = format("Hello {}, age: {}", name, age)
```

### 現在の問題

1. **可読性が低い**：文字列連結とフォーマットに多处呼び出しが必要で、コードが冗長
2. **誤りやすい**：手動で型変換を行うと `.to_string()` を見落とす可能性がある
3. **パフォーマンスの懸念**：複数の文字列連結がパフォーマンスに影響を与える可能性がある
4. **表現力が不足**：複雑な式を文字列に直接埋め込む直观的な方法がない

## 提案

### コア設計

f-string を新しい文字列リテラルのプレフィックスとして導入し、以下のをサポート：

- **変数補間**：`f"Hello {name}"`
- **式評価**：`f"Sum: {x + y}"`
- **フォーマット指定子**：`f"Pi: {pi:.2f}"`
- **型安全性**：コンパイル時に式の型をチェック

### 例

```yaoxiang
# 基本的な補間
name = "Alice"
greeting = f"Hello {name}"  # "Hello Alice"

# 式による補間
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

| 変更前 | 変更後 |
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

## 詳細な設計

### 字句解析

コンパイラは字句解析段階で `f` プレフィックス文字列リテラルを認識し、波括弧内の式とオプションのフォーマット指定子を解析する。

### 変換戦略

f-string はコンパイル時に効率的な文字列操作に変換される：

**単純な補間**：
```yaoxiang
f"Hello {name}"
```
に変換：
```yaoxiang
"Hello ".concat(name.to_string())
```

**式による補間**：
```yaoxiang
f"Sum: {x + y}"
```
に変換：
```yaoxiang
"Sum: ".concat((x + y).to_string())
```

**フォーマット指定子**：
```yaoxiang
f"Pi: {pi:.2f}"
```
に変換：
```yaoxiang
format("Pi: {:.2f}", pi)
```

**複数の補間**：
```yaoxiang
f"Hello {name}, you are {age} years old"
```
に変換：
```yaoxiang
"Hello ".concat(name.to_string()).concat(", you are ").concat(age.to_string()).concat(" years old")
```

### 型システムへの影響

- 補間式は `Stringable` インターフェースを実装する必要がある（基本型和字符串は自動的に実装）
- フォーマット指定子は型が相应のフォーマットをサポートする必要がある
- コンパイラは式の型とフォーマット規則の一致をチェックする

### コンパイラの改动

| コンポーネント | 改动 |
|------|------|
| lexer | f プレフィックスを認識し、文字列内補間構文を解析 |
| parser | 新しい FStringLiteral 構文ノードを追加 |
| typecheck | 補間式の型をチェックし、フォーマット規則を検証 |
| codegen | 文字列連結またはフォーマット呼び出しコードを生成 |

### 後方互換性

- ✅ 完全な後方互換性
- 既存の文字列リテラル `"..."` はそのまま維持
- f-string は新しい構文であり、既存のコードに影響しない

## トレードオフ

### 优点

1. **構文が简洁**：定型コードを削減し、可読性を向上
2. **型安全**：コンパイル時にチェックし、実行時エラーを削減
3. **パフォーマンス最適化**：コンパイラが文字列連結を最適化可能
4. **表現力が豊富**：任意の式とフォーマットをサポート
5. **学習コストが低い**：Python エコシステムと一貫性がある

### 缺点

1. **コンパイラの複雑さ**：新しい構文解析と変換ロジックを追加する必要がある
2. **構文の曖昧さ**：既存の文字列構文と区別する必要がある
3. **デバッグの課題**：変換後のコードはソースコードの構造と異なる

## 代替案

| 案 | 为什么不選択 |
|------|--------------|
| 変数補間のみサポート | 複雑なフォーマット要件を満たせない |
| 関数型スタイル `format(...)` を使用 | 構文が简洁でない |
| v2.0 まで延期 | ユーザーは文字列の使いやすさに対して明確なニーズがある |
| 逆引用符或其他プレフィックスを使用 | Python エコシステムと一貫性がない |

## 実装戦略

### 段階分け

1. **段階 1 (v0.9)**:
   - 基本的な f-string 構文サポート
   - 変数と単純な式補間
   - 基本的な型変換

2. **段階 2 (v1.0)**:
   - フォーマット指定子サポート
   - 複雑な式補間
   - パフォーマンス最適化

3. **段階 3 (v1.1)**:
   - デバッグ情報の強化
   - エラーメッセージの改善
   - より多くのフォーマットオプション

### 依存関係

- 外部依存なし
- 基本的な型システムサポートが必要
- 文字列ライブラリの基本機能が必要

### リスク

1. **パフォーマンスリスク**：複数の補間により过多的字符串オブジェクトが生成される可能性
   - **軽減**：コンパイラの最適化により隣接する文字列定数をマージ
2. **型チェックの複雑さ**：フォーマット指定子の型チェック
   - **軽減**：Python の実装を参考に、シンプルで直接的なチェックを使用
3. **構文の曖昧さ**：`{` と `}` のネスト使用
   - **軽減**：明確な構文規則を定め、ネストを制限

## 開放問題

- [x] エスケープされた波括弧をサポートするか？Python と一貫性：二重波括弧で単一波括弧を表す（如 <code v-pre>{{</code> は <code v-pre>{</code> を表し、<code v-pre>}}</code> は <code v-pre>}</code> を表す）
- [x] カスタムフォーマット関数をサポートするか？Python と一貫性：`__format__` メソッドにより型のフォーマット動作をカスタマイズ可能
- [x] フォーマット指定子の完全な仕様？Python と一貫性、上述の BNF を参照
- [x] パフォーマンス最適化の具体的な戦略？Python と一貫性：実行時に連結、追加の最適化は不要
- [x] エラー診断のベストプラクティス？Python と一貫性：错误時に元の f-string の内容と位置を表示

## 付録

### 付録A：フォーマット指定子リファレンス

| 型 | 指定子 | 例 | 出力 |
|------|--------|------|------|
| 整数 | `d` | `f"{42:d}"` | "42" |
| 浮動小数点 | `f` | `f"{3.14:.2f}"` | "3.14" |
| 科学的記数法 | `e` | `f"{1000:e}"` | "1.000000e+03" |
| 文字列 | `s` | `f"{name:s}"` | "Alice" |
| 16進数 | `x` | `f"{255:x}"` | "ff" |

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

# SQL クエリ構築（SQL インジェクションのリスクに注意）
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

## ライフサイクルと運命

RFC には以下の状態遷移がある：

```
┌─────────────┐
│   草案      │  ← 著者作成
└──────┬──────┘
       │
       ▼
┌─────────────┐
│  審査中     │  ← コミュニティ議論
└──────┬──────┘
       │
       ├──────────────────┐
       ▼                  ▼
┌─────────────┐    ┌─────────────┐
│  受入済み   │    │  却下済み   │
└──────┬──────┘    └──────┬──────┘
       │                  │
       ▼                  ▼
┌─────────────┐    ┌─────────────┐
│   accepted/ │    │    rfc/     │
│ (正式設計)  │    │ (保存位置)  │
└─────────────┘    └─────────────┘
```