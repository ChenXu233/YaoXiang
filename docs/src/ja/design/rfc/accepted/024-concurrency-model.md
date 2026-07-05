```yaml
---
title: "RFC-024:spawn ベースの並行ランタイムセマンティクス"
status: "承認済み（改訂版）"
author: "晨煦"
created: "2026-06-05"
updated: "2026-07-05（RFC 同期確認：実装進捗 ~85%、コアランタイムとフロントエンド解析が完了）"

issue: "#89"
---
```

# RFC-024:spawn ベースの並行ランタイムセマンティクス

> **本文書は `spawn` のランタイム動作セマンティクスを定義する**。
> 構文の直交性、AST/IR リファクタリング、型システムの拡張については [RFC-032](./032-spawn-unified-expression.md) を参照。
>
> 2 つの RFC が協調して `spawn` を定義する — 024 は「何をするか」、032 は「どう表現するか」を答える。

> **参考**:
> - [並行モデル仕様](/reference/language-spec/concurrency.md)
> - [RFC-008: Runtime 並行モデルとスケジューラの分離設計](./008-runtime-concurrency-model.md)
> - [RFC-009: 所有権モデル設計](./009-ownership-model.md)
> - [RFC-010: 統一型構文](./010-unified-type-syntax.md)
> - [RFC-032: spawn 統一式修飾 — AST/IR リファクタリング](./032-spawn-unified-expression.md)

## 概要

本文書は YaoXiang プログラミング言語の `spawn` の **ランタイム動作セマンティクス** を定義する：`spawn <expr>` は唯一の並列プリミティブであり、任意の式を修飾でき、呼び出し側は同期的にブロックする。式の形状がタスク分解の粒度を決定し、ランタイムは GMP モデルに従ってスケジュールする — 依存関係のないタスクは作業キューに投入され、worker が奪い合う。

**コア設計 — 1 つのプリミティブ、1 組のルール**：

```
spawn <expr>               ← 唯一の並列プリミティブ
タスク分解は式の形状で決定  ← 唯一のルール
結果の同期ブロック待機      ← 唯一の動作
```

**取り除かれた複雑さ**：
- ❌ `@block`/`@eager`/`@auto` 注釈なし
- ❌ `Send`/`Sync` trait なし
- ❌ `Mutex`/`RwLock`/`Atomic` なし
- ❌ `future`/ノンブロッキングハンドルなし
- ❌ プログラム全体の DAG 解析なし
- ❌ 関数の色付け（async/await）なし

> **ユーザーのメンタルモデル**：通常のコードは順次実行される。複数のことを同時に行いたい場合は、それらを `spawn <expr>` の中に入れる。コールバックも `await` も奇妙な注釈もない。

## 設計の由来

| 文書 | 関係 |
|------|------|
| [RFC-001](/design/rfc/deprecated/001-concurrent-model-error-handling.md) | 本文書により置き換え |
| [RFC-008](./008-runtime-concurrency-model.md) | ランタイムアーキテクチャ、本文書と直交 |
| [RFC-009](./009-ownership-model.md) | 所有権モデル、不変 |
| [RFC-010](./010-unified-type-syntax.md) | 統一型構文 |
| [RFC-032](./032-spawn-unified-expression.md) | AST/IR リファクタリング、本文書と協調して spawn を定義 |

## 動機

### なぜこの設計が必要か？

現在の主流言語の並行モデルには明らかな欠陥がある：

| 言語 | 並行モデル | 問題 |
|------|----------|------|
| Rust | async/await + tokio | 非同期の伝染、関数の色付け、学習曲線が急峻 |
| Go | goroutine | 型安全性なし、データ競合の検出が困難 |
| Python | asyncio | GIL 制限、関数の色付け |
| JavaScript | Promise/async | コールバック地獄、関数の色付け |

### 旧設計（RFC-001）の問題

RFC-001 が提案した 3 層並行アーキテクチャ（L1/L2/L3）には以下の問題がある：

| 問題 | 説明 |
|------|------|
| メンタルモデルが複雑 | L1/L2/L3 の 3 層抽象が学習負担を増大 |
| 注釈の冗長性 | `@block`/`@eager`/`@auto` 注釈がコードを騒がしくする |
| 解析の複雑さ | プログラム全体の DAG 解析によるコンパイル時間コストが大きい |
| 型制約の複雑さ | `Send`/`Sync` trait が認知負担を増大 |
| 制御不能 | 自動並行動作の予測とデバッグが困難 |

### 設計目標

1. **シンプル**：唯一の並列プリミティブ（`spawn`）、任意の式を修飾可能
2. **明示的**：ユーザーはどこが並列でどこが順次かを明確に把握
3. **安全**：所有権ルールが自然に拡張され、追加の型制約不要
4. **制御可能**：暗黙の並行なし、予期しない並列動作なし
5. **同期**：呼び出し側は同期ブロック、コールバックも `await` もない

---

## 提案

### 1. `{}` ブロックの本質：依存駆動の計算ユニット

YaoXiang において、`{}` は **依存駆動の計算ユニット** である。

| 属性 | 説明 |
|------|------|
| 依存駆動 | ブロックは実行時、内部のすべての変数が準備完了かチェックし、揃っていれば即座に実行、そうでなければブロックして待機 |
| 実行タイミング | 依存関係によって決定され、「即時」か「遅延」かには依存しない |
| 戻り値 | `return` で明示的に値を返す；`return` がない場合、デフォルトで `Void` を返す |
| 構文の統一 | 関数本体、変数初期化、`spawn` 後のどこに現れてもセマンティクスは一貫 |
| スコープの分離 | 変数は厳密に `{}` 内部に限定され、外側スコープには漏れない |

```yaoxiang
// 依存駆動の例
x = compute_x()        // x 準備完了
y = compute_y()        // y 準備完了
result = {
    // x と y に依存し、両方準備完了後に即座に実行
    return x + y
}
```

### 2. spawn 式のセマンティクス

`spawn <expr>` は YaoXiang における **唯一の並列プリミティブ** である。任意の式を修飾でき、式の形状がタスク分解の粒度を決定する。

#### 2.1 タスク生成ルール

| 式の形状 | タスク分解 | 同期セマンティクス |
|-----------|---------|---------|
| `spawn { a, b, c }` | 直接の子式 → N 個の独立タスク | すべてのタスクの完了を待機 |
| `spawn for x in items { body }` | 各反復 → 1 個のタスク | すべての反復の完了を待機 |
| `spawn while cond { body }` | 各反復ラウンド → 1 個のタスク（反復間は条件駆動） | 条件が false になるまで待機 |
| `spawn if c { a } else { b }` | 条件 c を順次評価、選択された分岐全体を → 1 個のタスク | 選択された分岐の完了を待機 |
| `spawn call(x)` | 呼び出し自体 → 1 個のタスク | 呼び出しの完了を待機 |
| `spawn expr`（任意の式） | 式自体 → 1 個のタスク | 式の完了を待機 |

> **設計動機**：なぜ spawn は任意の式を修飾できるのか？詳細は [RFC-032 §コア設計](./032-spawn-unified-expression.md) を参照。
>
> **制御フローの直交性**：`spawn <expr>`（spawn が前）と `<expr> spawn { body }`（spawn が後）のセマンティクスの差異については、[RFC-032 §制御フローの直交性](./032-spawn-unified-expression.md)（コア定義）を参照。逆順に書くすべての組み合わせ（`for ... spawn { }` / `while ... spawn { }` / `if ... spawn { }`）のランタイム動作 — エラー伝播、リソース型、ネストルール — は本文 §2.4 / §2.5 / §2.6 のルールを継承する。

```yaoxiang
// spawn ブロック：直接の子式を並列実行
(a, b) = spawn {
    t1 = fetch("url1")   // 直接の子式 → 並列タスク 1
    t2 = fetch("url2")   // 直接の子式 → 並列タスク 2
    return (t1, t2)      // 明示的にタプルを返す
}

// spawn for：各反復を並列実行
results = spawn for item in items {
    process(item)        // 各反復 → 独立タスク
}

// spawn while：各反復ラウンドを並列実行
spawn while has_next() {
    step()               // 各反復ラウンド → 独立タスク
}

// spawn if：選択された分岐全体を 1 タスクとして
result = spawn if cond {
    branch_a()
} else {
    branch_b()
}
```

#### 2.2 スコープの分離

spawn 式は独立したスコープを作成し、内部の変数は外部に影響しない：

```yaoxiang
x = 10
result = spawn {
    x = 20              // これは spawn 式内のローカル x
    compute(x)
}
// x は依然として 10

result = spawn for item in items {
    item = item + 1     // 反復ローカル item、各反復は独立したコピー
    process(item)
}
// 外側の item は影響を受けない
```

**反復変数**（for の `x`）は各ラウンドで独立したコピーを持ち、反復終了時に自動破棄される。

#### 2.3 所有権ルール

変数が spawn 式に入った後、外部では使用できなくなる（Move セマンティクス）：

```yaoxiang
data = load_data()
result = spawn {
    process(data)       // data の所有権が spawn 式に移動
}
// ここで data は使用不可（move 済み）
```

複数のタスク間で共有する必要がある場合は、`ref` を使用：

```yaoxiang
data = load_data()
shared = ref data       // コンパイラが自動的に Rc または Arc を選択

result = spawn {
    process_a(shared),  // 共有参照
    process_b(shared)   // 共有参照
}
```

**反復をまたぐ共有**：`ref` で外側にキャプチャし、反復間で同じ参照を共有する。

#### 2.4 エラー伝播ルール

##### `spawn { a, b, c }`（ブロック）

1. すべてのタスクの完了を待機（一部が失敗しても）
2. 最初に出会ったエラーを伝播
3. `?` でエラー伝播点を明示

```yaoxiang
(a, b) = spawn {
    fetch("url1")?,     // 失敗する可能性あり
    fetch("url2")?      // 失敗する可能性あり
}
// いずれかのタスクが失敗した場合、spawn 式全体は最初のエラーを伝播する
```

##### `spawn for x in items { body? }`

- すべての反復の完了を待ってから最初のエラーを返す
- 失敗した反復の後も残りの反復は **実行継続**（キャンセルしない）
- `?` でエラー伝播点を明示

```yaoxiang
results = spawn for item in items {
    process(item)?      // いずれかの反復が失敗 → すべて完了を待機 → 最初のエラーを伝播
}
```

##### `spawn while cond { body? }`

while 自身のエラーセマンティクスを継承：

- step が `?` でエラーを伝播 → spawn while 全体失敗、次のラウンドには進まない
- step がエラーを伝播しない（エラーが吸収される）→ 次の反復ラウンドへ

```yaoxiang
spawn while has_next() {
    item = next()       // エラーを伝播しない場合、失敗しても次のラウンドへ
    process(item)
}
```

##### `spawn if c { a } else { b }`

- 条件 c は **順次評価**
- c の評価エラー → 全体エラー
- 選択された分岐内のエラー → 全体エラー

```yaoxiang
result = spawn if cond()? {  // cond は順次評価、失敗 → 全体エラー
    fetch_a()?
} else {
    fetch_b()?
}
```

#### 2.5 リソース型ルール

コンパイラはリソース型の使用を追跡し、並行安全性を確保する：

| リソース型 | 説明 | コンパイラの動作 |
|----------|------|-----------|
| `FilePath` | ファイルシステムパス | 同じパスの操作は自動的にシリアル化 |
| `HttpUrl` | HTTP エンドポイント | 同じ URL の操作は自動的にシリアル化 |
| `DBUrl` | データベース接続 | 同じ接続の操作は自動的にシリアル化 |
| `Console` | 標準出力 | すべての Console 操作は自動的にシリアル化 |

##### `spawn { ... }` ブロック内

```yaoxiang
// 同一ファイルの操作は自動的にシリアル化される
(a, b) = spawn {
    read_file("data.txt"),      // 先に実行
    write_file("data.txt", x)   // 読み取り完了を待機
}
```

##### `spawn for ... { ... }` 反復をまたぐ同一リソース

すべての反復が同一のリソース型を操作する場合、コンパイラは **自動的にシリアルに降格** する（spawn が通常の for に降格、エラーは出ない）：

```yaoxiang
// すべての反復が同一ファイルパスに書き込み → 自動的にシリアルに降格
results = spawn for item in items {
    write_file("data.txt", item)
}
// コンパイラが自動的にすべての反復をシリアル化する
```

> **設計理由**：spawn キーワードは依然として並列意図を表現する；リソース競合時、コンパイラは直接拒否するより自動的に降格する方が最小驚きの原則に沿う。

##### `spawn while ... { ... }` が `&mut` をキャプチャ

**コンパイル時エラー**：`spawn while` は `&mut` 型の外部変数のキャプチャを許可しない：

```yaoxiang
iter = make_iter()
spawn while iter.has_next() {       // コンパイル時エラー
    item = iter.next()              // iter は &mut、反復をまたぐ可変共有 = データ競合
}
```

> **`Sync` trait を再導入しない**：RFC-024 の「Send/Sync なし」コミットメントと一貫。ユーザーが `ref` または非 spawn の記述に変更することを要求する。

##### `spawn if c { ... } else { ... }` 両分岐の同一リソース

**合法で警告なし**：if 条件は排他的で、最大 1 つの分岐しか実行されないため、並行競合は存在しない：

```yaoxiang
result = spawn if use_cache {
    load_from_cache(key)            // 分岐 1：cache 読み取り
} else {
    fetch(key)                      // 分岐 2：URL 読み取り
}
```

#### 2.6 spawn のネスト

spawn 式はネスト可能で、内側は **独立した並行ドメイン** を作成する：

```yaoxiang
(a, b) = spawn {
    x = spawn {
        fetch("url1"),
        fetch("url2")
    },
    y = compute(x)
}
```

**ネストのセマンティクス**：
- 内側の spawn は独立した並行ドメイン（独立したタスクキュー、独立したエラー伝播）
- 内側のエラーは独立して外側に伝播する（外側タスクは内側完了待機時にエラーを受け取る）
- 内側のリソース型ルールは独立して追跡される（外側と統合してチェックされない）

```yaoxiang
// spawn for の中に spawn while をネスト
results = spawn for x in items {
    inner = spawn while has_more(x) {
        step(x)
    }
    process(inner)
}
```

### 3. 旧設計との決別

| 旧設計（RFC-001） | 新設計（RFC-024 + RFC-032） |
|------------------|---------------------------|
| プログラム全体の自動 DAG 解析 | spawn 式内の解析のみ |
| `@block`/`@eager`/`@auto` 注釈 | 注釈なし、依存駆動 |
| `Send`/`Sync` trait | 不要、所有権 + ref で自動処理 |
| `future`/ノンブロッキングハンドル | 同期ブロック、コールバックなし |
| `Mutex`/`RwLock`/`Atomic` | `ref` が自動的に Rc/Arc を選択 |
| L1/L2/L3 の 3 層メンタルモデル | 通常コードは順次、spawn 式が並列 |
| 関数の色付け（async/await） | 関数の色付けなし |
| `spawn` は `{}` ブロックのみ修飾 | `spawn` は任意の式を修飾（RFC-032 を参照） |

### 4. return ルール

YaoXiang の return ルールは統一され明確：

| 書き方 | 戻り値 | 説明 |
|------|--------|------|
| `= expr`（波括弧なし） | `expr` を直接返す | 式が値そのもの |
| `= { ... }`（波括弧あり） | `return` 必須、なければ `Void` を返す | ブロックは明示的な return が必要 |

```yaoxiang
// 波括弧なし：直接返す
add: (a: Int, b: Int) -> Int = a + b

// 波括弧あり：return 必須
process: (data: Data) -> Result = {
    validated = validate(data)?
    return ok(transform(validated))
}

// 波括弧ありだが return なし：Void を返す
log: (message: String) -> Void = {
    print(message)  // return なし、Void を返す
}
```

### 5. ユーザーのメンタルモデル

> **通常のコードは順次実行される。**
>
> **複数のことを同時に行いたい場合は、それらを `spawn <expr>` の中に入れる。**
>
> 式の形状がタスクの分解方法を決定する：ブロック内の各直接の子式は並列、for の各反復は並列、if の選択された分岐は 1 タスクとして。
>
> **spawn 式全体は同期的にブロックし、すべてのタスクの完了を待機する。**
>
> **コールバックも `await` も奇妙な注釈もない。**

```yaoxiang
// 通常コード：順次実行
a = compute_a()         // 先に実行
b = compute_b(a)        // a に依存、a 完了後に実行
c = compute_c(b)        // b に依存、b 完了後に実行

// 並列が必要な場合：spawn を使用
(x, y, z) = spawn {
    fetch("url1"),      // 並列
    fetch("url2"),      // 並列
    fetch("url3")       // 並列
}
// すべて完了を待ってから続行
process(x, y, z)

// データ並列：spawn for
results = spawn for item in items {
    process(item)
}
```

---

## トレードオフ

### 利点

1. **シンプル**：唯一の並列プリミティブ（`spawn`）、任意の式を修飾可能
2. **明示的**：ユーザーはどこが並列でどこが順次かを明確に把握、暗黙の並行なし
3. **安全**：所有権ルールが自然に拡張され、`Send`/`Sync` などの追加の型制約不要
4. **制御可能**：自動並列動作なし、予期しない並行問題を回避
5. **同期**：呼び出し側は同期ブロック、コードが理解しやすくデバッグしやすい
6. **関数の色付けなし**：async/await の関数の色付け問題が存在しない
7. **コンパイル効率**：DAG 解析は spawn 式内のみ、コンパイル時間が制御可能
8. **直交性**：spawn は任意の制御フロー構造と自然に組み合わせ可能（詳細は RFC-032）

### 欠点

1. **明示的な spawn が必要**：自動並列化できず、ユーザーが手動で並列ポイントをマークする必要がある
2. **spawn 式内の DAG 解析**：コンパイラが spawn 式内で依存解析を行う必要がある
3. **旧コードとの非互換性**：旧 RFC-001 パターンを使用するコードは移行が必要

---

## 代替案

| 案 | なぜ採用しないか |
|------|--------------|
| プログラム全体の自動 DAG（RFC-001） | 複雑度が高く、コンパイル時間が長く、動作が制御不能 |
| async/await | 関数の色付け、学習曲線が急峻、コード可読性が低い |
| goroutine | 型安全性なし、データ競合の検出が困難 |
| Actor モデル | メッセージパッシングが複雑、デバッグが困難 |
| CSP（Go channel） | 型安全性なし、デッドロックの検出が困難 |
| `spawn` は `{}` ブロックのみ修飾 | 直交性を破壊、`spawn for` が特殊ケースになる（RFC-032 を参照） |

---

## 実装戦略

### コンパイル時解析

1. **式の形状認識**：spawn 後の式の形状に応じてタスク分解を決定（詳細は RFC-032 §DAG 解析）
2. **DAG 構築**：spawn 式内の依存関係を解析
3. **トポロジカルソート**：spawn 式内の実行順序を決定
4. **並列識別**：spawn 式内の依存関係のない部分木を識別
5. **エスケープ解析**：`ref` → Rc か Arc
6. **リソース競合検出**：リソース型の潜在的競合を検出

### モジュール構成

spawn 関連コードは `frontend/core/spawn/` に統一配置：

```
frontend/core/spawn/
├── mod.rs           # spawn モジュールエントリ
├── placement.rs     # spawn 出現位置の合法性チェック
└── analysis.rs      # タスク識別、依存解析、リソース競合検出
```

> **移行説明**（2026-06-11）：既存の `frontend/core/typecheck/passes/spawn_placement.rs` は `frontend/core/spawn/placement.rs` に移行される。`typecheck/passes/` ディレクトリ下の `spawn_placement` モジュール宣言も同期して削除する必要がある。

### ランタイム実行

[RFC-008](./008-runtime-concurrency-model.md) の Runtime アーキテクチャを参照：

- **Embedded Runtime**：spawn サポートなし、即时実行
- **Standard Runtime**：spawn 式をサポート
- **Full Runtime**：Standard + WorkStealer 負荷分散

### 依存関係

- RFC-008（Runtime アーキテクチャ）→ 完了済み
- RFC-009（所有権モデル）→ 完了済み
- RFC-010（統一型構文）→ 完了済み
- RFC-011（ジェネリクスシステム）→ 完了済み
- RFC-032（AST/IR リファクタリング）→ 本文書と協調して spawn を定義

---

## 設計決定の記録

| 決定 | 結論 | 理由 | 日付 |
|------|------|------|------|
| 並列プリミティブ | `spawn <expr>` | シンプル、明示的、制御可能 | 2026-06-05 |
| spawn の修飾範囲 | 任意の式 | 構文の直交性、`spawn for` の特殊化を除去 | 2026-07-04 |
| タスク分解 | 式の形状で決定 | 表現力が強い、ルールが統一 | 2026-07-04 |
| 実行モデル | 同期ブロック | 理解しやすく、デバッグしやすい | 2026-06-05 |
| DAG 解析範囲 | spawn 式内のみ | コンパイル効率、動作の制御可能性 | 2026-06-05 |
| 共有メカニズム | `ref` が Rc/Arc を自動選択 | ユーザー判断を簡素化 | 2026-06-05 |
| 注釈 | なし | コードのノイズを削減 | 2026-06-05 |
| Send/Sync | 削除 | 所有権 + ref で十分 | 2026-06-05 |
| Mutex/RwLock | 削除 | ref が自動処理 | 2026-06-05 |
| future/ハンドル | 削除 | 同期ブロックの方がシンプル | 2026-06-05 |
| 関数の色付け | なし | async/await の問題を回避 | 2026-06-05 |
| リソース型 | 組み込み + ユーザー定義 | 自動シリアル化 | 2026-06-05 |
| `spawn {}` エラー | すべて完了を待機、最初のエラーを伝播 | 決定論的な動作 | 2026-06-05 |
| `spawn for` エラー | すべて完了を待機、最初のエラーを伝播 | `spawn {}` と一貫 | 2026-07-04 |
| `spawn while` エラー | while のエラーセマンティクスを継承 | while の標準動作 | 2026-07-04 |
| `spawn if` 条件エラー | c を順次評価、失敗 → 全体エラー | 直感に沿う | 2026-07-04 |
| `spawn for` 同一リソース | 自動的にシリアルに降格 | 安全な降格、粗暴な拒否ではない | 2026-07-04 |
| `spawn while` の `&mut` キャプチャ | コンパイル時エラー | データ競合を回避、Sync を導入しない | 2026-07-04 |
| `spawn if` 同一リソース | 合法で警告なし | 排他分岐は競合を構成しない | 2026-07-04 |
| spawn のネスト | 内側は独立した並行ドメイン | 独立したタスクキュー、エラー、リソース | 2026-07-04 |

---

## 参考文献

### YaoXiang 公式ドキュメント

- [並行モデル仕様](/reference/language-spec/concurrency.md)
- [RFC-001 並作モデル（廃止済み）](/design/rfc/deprecated/001-concurrent-model-error-handling.md)
- [RFC-008 Runtime 並行モデル](./008-runtime-concurrency-model.md)
- [RFC-009 所有権モデル](./009-ownership-model.md)
- [RFC-010 統一型構文](./010-unified-type-syntax.md)
- [RFC-011 ジェネリクスシステム](./011-generic-type-system.md)
- [RFC-032 spawn 統一式修飾 — AST/IR リファクタリング](./032-spawn-unified-expression.md)

### 外部参考

- [Rust async book](https://rust-lang.github.io/async-book/)
- [Go concurrency patterns](https://go.dev/blog/pipelines)
- [Erlang concurrency](https://www.erlang.org/doc/getting_concurrency/getting_concurrency.html)
- [Structured concurrency](https://en.wikipedia.org/wiki/Structured_concurrency)

---

## ライフサイクルと帰属

| 状態 | 場所 | 説明 |
|------|------|------|
| **承認済み（改訂版）** | `docs/design/rfc/accepted/` | RFC-032 と協調して spawn を定義（ランタイムセマンティクス） |