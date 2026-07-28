---
title: 'RFC 012: F-String テンプレート文字列'
status: '承認済み'
author: 'Chen Xu'
created: '2025-01-27'
updated: '2026-07-05'
issue: '#124'
---

# RFC 012: F-String テンプレート文字列

## 概要

YaoXiang 言語に f-string テンプレート文字列機能を追加し、変数の補間、式の評価、書式設定出力に対応する。f-string は Python スタイルの構文（`f"..."` プレフィックス）を使用し、文字列内で `{expression}` 構文により式を埋め込み、コンパイル時に効率的な文字列操作に変換する。

> **注意**: f-string の構文と動作は Python と一貫性を保ち、具体的な仕様については
> [Python 公式ドキュメント](https://docs.python.org/3/tutorial/inputoutput.html#formatted-string-literals)を参照。

## 動機

### なぜこの機能が必要か？

現在の YaoXiang の文字列連結方式是は冗長である：

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

1. **可読性が低い**：文字列の連結と書式設定に複数回の呼び出しが必要で、コードが冗長
2. **間違いやすい**：手動で型変換を行い、`.to_string()` を見落としやすい
3. **パフォーマンスの考慮**：複数の文字列連結がパフォーマンスに影響する可能性がある
4. **表現力が不足**：複雑な式を文字列に直感的に埋め込めない

## 提案

### コア設計

f-string を新しい文字列リテラルのプレフィックスとして導入し、以下のをサポート：

- **変数の補間**：`f"Hello {name}"`
- **式の評価**：`f"Sum: {x + y}"`
- **書式指定子**：`f"Pi: {pi:.2f}"`
- **型安全性**：コンパイル時に式の型を検査

### 例

```yaoxiang
# 基本的な補間
name = "Alice"
greeting = f"Hello {name}"  # "Hello Alice"

# 式の補間
x = 10
y = 20
result = f"Sum: {x + y}"    # "Sum: 30"

# 書式指定子
pi = 3.14159
formatted = f"Pi: {pi:.2f}"  # "Pi: 3.14"

# 複雑な式
items = [1, 2, 3]
s = f"Count: {len(items)}, sum: {sum(items)}"  # "Count: 3, sum: 6"

# オブジェクトのメソッド呼び出し
user = User("Bob", 25)
bio = f"Name: {user.name}, age: {user.get_age()}"
```

### 構文の変化

| 変更前                           | 変更後                |
| -------------------------------- | --------------------- |
| `"Hello ".concat(name)`          | `f"Hello {name}"`     |
| `format("Value: {}", value)`     | `f"Value: {value}"`   |
| `format("Pi: {:.2f}", pi)`       | `f"Pi: {pi:.2f}"`     |

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

### 構文解析

コンパイラは字句解析段階で `f` プレフィックスの文字列リテラルを認識し、波括弧内の式とオプションの書式指定子を解析する。

### 変換戦略

f-string はコンパイル時に効率的な文字列操作に変換される：

**単純な補間**：

```yaoxiang
f"Hello {name}"
```

に変換される：

```yaoxiang
"Hello ".concat(name.to_string())
```

**式の補間**：

```yaoxiang
f"Sum: {x + y}"
```

に変換される：

```yaoxiang
"Sum: ".concat((x + y).to_string())
```

**書式指定子**：

```yaoxiang
f"Pi: {pi:.2f}"
```

に変換される：

```yaoxiang
format("Pi: {:.2f}", pi)
```

**複数の補間**：

```yaoxiang
f"Hello {name}, you are {age} years old"
```

に変換される：

```yaoxiang
"Hello ".concat(name.to_string()).concat(", you are ").concat(age.to_string()).concat(" years old")
```

### 型システムへの影響

- 補間式は `Stringable` インターフェースを実装する必要がある（基本型と文字列には自動的に実装される）
- 書式指定子は対応する書式設定をサポートする必要がある
- コンパイラは式の型と書式設定ルールの整合性を検査する

### コンパイラの改変

| コンポーネント | 改変内容                                      |
| -------------- | --------------------------------------------- |
| lexer          | f プレフィックスを認識し、文字列内補間構文を解析 |
| parser         | FStringLiteral 構文ノードを新規追加            |
| typecheck      | 補間式の型を検査し、書式設定ルールを検証        |
| codegen        | 文字列連結または書式設定呼び出しコードを生成    |

### 下位互換性

- ✅ 完全な下位互換性
- 既存の文字列リテラル `"..."` は変更なし
- f-string は新規構文であり、既存コードに影響なし

## トレードオフ

### 优点

1. **構文が簡潔**：定型句が減り、可読性が向上
2. **型安全**：コンパイル時に検査され、実行時エラーを減少
3. **パフォーマンスの最適化**：コンパイラが文字列連結を最適化可能
4. **表現力が豊富**：任意の式と書式設定をサポート
5. **学習コストが低い**：Python エコシステムと一貫性あり

### 欠点

1. **コンパイラの複雑さ**：新增の構文解析と変換ロジックが必要
2. **構文の曖昧性**：既存の文字列構文との区別が必要
3. **デバッグの課題**：変換後のコードとソースコードの構造が異なる

## 代替案

| 案                             | 選択しない理由                   |
| ------------------------------ | -------------------------------- |
| 変数の補間のみサポート         | 複雑な書式設定のニーズを満たせない |
| 関数型スタイル `format(...)` の使用 | 構文が簡潔さに欠ける               |
| v2.0 まで延期                   | ユーザーには文字列の使いやすさへの明確なニーズがある |
| バッククォート或其他のプレフィックスを使用 | Python エコシステムと一貫性がない     |

## 実装戦略

### 段階的划分

1. **段階 1 (v0.9)**:
   - 基本的な f-string 構文サポート
   - 変数と単純な式の補間
   - 基本的な型変換

2. **段階 2 (v1.0)**:
   - 書式指定子サポート
   - 複雑な式の補間
   - パフォーマンスの最適化

3. **段階 3 (v1.1)**:
   - デバッグ情報の強化
   - エラーメッセージの改善
   - 追加の書式設定オプション

### 依存関係

- 外部依存なし
- 基本的な型システムサポートが必要
- 文字列ライブラリの基本機能が必要

### リスク

1. **パフォーマンスリスク**：複数の補間により文字列オブジェクトが过多発生する可能性
   - **軽減**：コンパイラが隣接する文字列定数をマージする最適化を行う
2. **型検査の複雑さ**：書式指定子の型検査
   - **軽減**：Python の実装を参照し、シンプルで直接的な検査を使用する
3. **構文の曖昧性**：`{` と `}` のネスト使用
   - **軽減**：明確な構文ルールを定め、ネストを制限する

## 開放的な問題

- [x] エスケープされた波括弧のサポート？Python と同樣に、二重波括弧で単一波括弧を表す，如
      <code v-pre>{{</code> は <code v-pre>{</code> を、<code v-pre>}}</code> は
      <code v-pre>}</code> を表す
- [x] カスタム書式設定関数のサポート？Python と同樣に、`__format__` メソッドを通じて型の書式設定をカスタマイズ可能
- [x] 書式指定子の完全な仕様？Python と同樣、上位の BNF 参照
- [x] パフォーマンス最適化の具体的な戦略？Python と同樣：実行時に連結し、特殊な最適化は不要
- [x] エラー診断のベストプラクティス？Python と同樣：エラー時に元の f-string の内容と位置を表示

## 付録

### 付録A：書式指定子リファレンス

| 型       | 指定子 | 例               | 出力           |
| -------- | ------ | ---------------- | -------------- |
| 整数     | `d`    | `f"{42:d}"`      | "42"           |
| 浮動小数点 | `f`    | `f"{3.14:.2f}"` | "3.14"         |
| 科学的記数法 | `e`    | `f"{1000:e}"`   | "1.000000e+03" |
| 文字列   | `s`    | `f"{name:s}"`    | "Alice"        |
| 十六進数 | `x`    | `f"{255:x}"`     | "ff"           |

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

# 条件付き書式設定
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

## ライフサイクルと归宿

RFC には以下の状態フローがある：

```
┌─────────────┐
│   草案       │  ← 作成者が作成
└──────┬──────┘
       │
       ▼
┌─────────────┐
│  レビュー中  │  ← コミュニティで議論
└──────┬──────┘
       │
       ├──────────────────┐
       ▼                  ▼
┌─────────────┐    ┌─────────────┐
│  承認済み   │    │  拒否済み   │
└──────┬──────┘    └──────┬──────┘
       │                  │
       ▼                  ▼
┌─────────────┐    ┌─────────────┐
│   accepted/ │    │    rfc/     │
│ (正式設計)  │    │ (元の位置を保持) │
└─────────────┘    └─────────────┘
```
