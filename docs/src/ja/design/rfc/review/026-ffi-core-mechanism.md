---
title: "RFC-026：FFI コアメカニズム"
status: "審査中"
author: "晨煦"
created: "2026-06-05"
updated: "2026-06-10"
---

# RFC-026：FFI コアメカニズム

> **参考**:
> - [RFC-008: Runtime 並行モデルとスケジューラの疎結合設計](./008-runtime-concurrency-model.md)
> - [RFC-009: 所有権モデル設計](./009-ownership-model.md)
> - [RFC-010: 統一型構文](./010-unified-type-syntax.md)
> - [RFC-024: spawn ブロックベースの並行モデル](./024-concurrency-model.md)

> **廃止**:
> - [RFC-020: 動的モジュールと FFI 統合](./020-dynamic-modules-ffi.md) — 本ドキュメントに統合
> - [RFC-021: ライブラリ駆動 FFI 拡張とクロス言語呼び出しサポート](./021-library-driven-ffi-extension.md) — 本ドキュメントに統合

## 概要

本文書では YaoXiang の FFI（外部関数インターフェース）コアメカニズムを定義する。内容は以下の通り：

1. **FFI 型定義**：`unsafe {}` ブロックを用いて不透明型を定義し、`return` で一つ上のスコープに返す
2. **FFI 関数宣言**：`native("symbol")` 構文を用いて外部関数を宣言する
3. **メソッドバインド**：`[0]` 構文を用いて self 引数の位置を指定する
4. **不透明型の処理**：`unsafe {}` ブロックで不透明型を明示的に定義し、空ボディ `Type = {}` は真空型とする
5. **不透明型のライフサイクル**：`.drop` でデストラクタをバインドし、RAII により自動解放、Null セーフ処理
6. **spawn ブロック内の FFI 挙動**：リソース型は自動的にシリアル化、非リソース型は並列化可能

**コア設計——1 つの原則、統一セマンティクス**：

```
すべての {} 内の return は、内容を一つ上のスコープに返す
デフォルトで return がない場合は Void を返す
```

---

## 動機

### なぜこの設計が必要か？

RFC-020 と RFC-021 はそれぞれ FFI の異なる側面を定義していた：
- RFC-020：動的モジュールと FFI 統合
- RFC-021：ライブラリ駆動 FFI 拡張

両者には重複があり、統一された FFI 仕様へと統合する必要がある。

### 設計目標

1. **統一**：すべての `{}` ブロックの return セマンティクスを一貫させる
2. **安全**：不透明型のフィールドアクセスには unsafe 権限が必要
3. **簡潔**：新しいキーワードや特殊マーカーを必要としない
4. **実用性**：yx-bindgen が自動的にバインディングを生成

---

## 提案

### 1. FFI 型定義

#### 1.1 unsafe ブロック + return セマンティクス

unsafe ブロック内で不透明型を定義し、return で一つ上のスコープに返す：

```yaoxiang
// unsafe ブロック内で不透明型を定義
SqliteDb = unsafe {
    SqliteDb: Type = {
        handle: *Void  // 生ポインタ
    }
    return SqliteDb
}

// SqliteDb は unsafe ブロック外でも使用可能
db = sqlite3_open("test.db")

// ❌ コンパイルエラー：handle フィールドには unsafe 権限が必要
handle = db.handle

// ✅ メソッド呼び出しによるアクセス
db.close()
```

#### 1.2 透過型

透過型は直接定義でき、unsafe ブロックは不要：

```yaoxiang
// 透過型
Point: Type = {
    x: Int32,
    y: Int32
}

// ユーザは直接作成可能
p: Point = Point { x: 1, y: 2 }
```

#### 1.3 不透明型、透過型、真空型

3 つの型の区別は**定義時**に決定され、コンパイラがファイルをまたいで推論する必要はない：

```yaoxiang
// 透過型：フィールドを持つ
Point: Type = { x: Int32, y: Int32 }
// ユーザはフィールドの作成とアクセスが可能
p: Point = Point { x: 1, y: 2 }

// 真空型：空ボディ、unsafe ブロック外
MyMarker: Type = {}
// ゼロサイズ型、自由に作成可能
x: MyMarker = {}

// 不透明型：unsafe ブロックから返される
SqliteDb = unsafe {
    SqliteDb: Type = {
        handle: *Void
    }
    return SqliteDb
}
// SqliteDb は不透明型、直接作成もフィールドアクセスも不可
```

**ルール**：
- **フィールドを持つ** → 透過型
- **空ボディ + unsafe ブロック外** → 真空型（ゼロサイズ）
- **unsafe ブロックから返される** → 不透明型

`native` 関数は既に明示的に定義された型のみを参照でき、型の属性は変更しない。型の属性は定義時に決定され、使用時に推論されることはない。

---

### 2. FFI 関数宣言

#### 2.1 native 構文

`native("symbol")` 構文を用いて外部関数を宣言：

```yaoxiang
// FFI 関数宣言
sqlite3_open: (filename: String) -> SqliteDb = native("sqlite3_open")
sqlite3_close: (db: SqliteDb) -> Int32 = native("sqlite3_close")
sqlite3_exec: (db: SqliteDb, sql: String) -> Int32 = native("sqlite3_exec")
```

#### 2.2 引数型のマッピング

FFI 関数の引数型は YaoXiang 型をそのまま使用し、コンパイラが C 型のマッピングを自動的に処理する：

| C 型 | YaoXiang 型 |
|--------|---------------|
| `int` | `Int32` |
| `long` | `Int64` |
| `float` | `Float32` |
| `double` | `Float64` |
| `char` | `Char` |
| `char*` | `String` |
| `bool` | `Bool` |
| `size_t` | `Uint` |
| `void*` | `*Void` |
| `struct T*` | `T`（透過型）|
| `typedef struct T T` | `T`（不透明型）|

#### 2.3 戻り型

FFI 関数の戻り型も YaoXiang 型をそのまま使用：

```yaoxiang
// 不透明型を返す
sqlite3_open: (filename: String) -> SqliteDb = native("sqlite3_open")

// 透過型を返す
get_point: () -> Point = native("get_point")

// 基本型を返す
get_value: () -> Int32 = native("get_value")
```

---

### 3. メソッドバインド

#### 3.1 [0] 構文

`[0]` 構文を用いて、関数の引数タプル内における self 引数の位置を指定する：

```yaoxiang
// FFI 関数
sqlite3_close: (db: SqliteDb) -> Int32 = native("sqlite3_close")
sqlite3_exec: (db: SqliteDb, sql: String) -> Int32 = native("sqlite3_exec")

// メソッドバインド（self は位置 0）
SqliteDb.close = sqlite3_close[0]
SqliteDb.exec = sqlite3_exec[0]
```

**呼び出し方式**：
```yaoxiang
db = sqlite3_open("test.db")

// メソッド呼び出し
db.close()  // sqlite3_close(db) と同等
db.exec("SELECT * FROM users")  // sqlite3_exec(db, "SELECT * FROM users") と同等
```

#### 3.2 コンストラクタのバインド

コンストラクタには `[0]` を付けず、通常の関数としてバインドする：

```yaoxiang
// FFI 関数
sqlite3_open: (filename: String) -> SqliteDb = native("sqlite3_open")

// コンストラクタバインド（通常の関数）
SqliteDb.open = sqlite3_open
```

**呼び出し方式**：
```yaoxiang
// コンストラクタで生成
db = SqliteDb.open("test.db")
```

#### 3.3 バインド位置

型はデータコンテナであるため、メソッドバインドは任意の位置で行える：

```yaoxiang
// 型定義後にバインド
SqliteDb.close = sqlite3_close[0]

// 他のファイルでバインド
SqliteDb.exec = sqlite3_exec[0]

// コンパイラが最終的にチェックする
```

---

### 4. 不透明型の処理

#### 4.1 不透明型の内部ストレージ

`unsafe {}` ブロック内で定義された `handle: *Void` はコンパイラが自動的に管理する：

```text
コンパイラの処理：
    SqliteDb = unsafe {
        SqliteDb: Type = {
            handle: *Void         // ← コンパイラが内部的に C ポインタを保持
        }
        return SqliteDb
    }

結果：
    SqliteDb は不透明型（unsafe ブロックで明示的に定義）
    フィールドアクセスには unsafe 権限が必要
    ユーザが直接作成することは禁止（native コンストラクタ経由が必須）
```

コンパイラが逆推論する必要はない——型が不透明かどうかは定義方法で決定され、明確かつ予測可能である。

#### 4.2 ユーザコード

```yaoxiang
import sqlite3_bindings

// ✅ コンストラクタで生成
db = SqliteDb.open("test.db")

// ❌ コンパイルエラー：不透明型は直接作成できない
db: SqliteDb = {}

// ✅ メソッド呼び出し
result = db.exec("SELECT * FROM users")
db.close()
```

---

### 5. spawn ブロック内の FFI 挙動

#### 5.1 リソース型の自動シリアル化

FFI 型がリソース型の場合、spawn ブロック内で自動的にシリアル化される：

```yaoxiang
// SqliteDb はリソース型
(a, b) = spawn {
    db1 = SqliteDb.open("db1.sqlite"),  // SqliteDb リソース
    db2 = SqliteDb.open("db2.sqlite")   // 異なるインスタンス、並列化可能
}

(a, b) = spawn {
    result1 = db.exec("SELECT ..."),  // 同一の SqliteDb
    result2 = db.exec("INSERT ...")   // 自動的にシリアル化
}
```

#### 5.2 非リソース型は並列化可能

FFI 型がリソース型でない場合、spawn ブロック内で並列化可能：

```yaoxiang
// Float はリソース型ではない
(a, b) = spawn {
    result1 = sin(1.0),  // 並列化可能
    result2 = cos(1.0)   // 並列化可能
}
```

---

### 6. yx-bindgen ツールチェイン

#### 6.1 生成内容

yx-bindgen は以下を生成する：
- FFI 型定義（unsafe ブロック + return）
- FFI 関数宣言（native 構文）
- メソッドバインド（[0] 構文）

#### 6.2 生成例

```bash
yx-bindgen --header /usr/include/sqlite3.h --output sqlite3_bindings.yx
```

生成結果：

```yaoxiang
// sqlite3_bindings.yx
// 自動生成、手動編集不可

// ============================================================================
// 型定義
// ============================================================================

SqliteDb = unsafe {
    SqliteDb: Type = {
        handle: *Void
    }
    return SqliteDb
}

SqliteStmt = unsafe {
    SqliteStmt: Type = {
        handle: *Void
    }
    return SqliteStmt
}

// ============================================================================
// FFI 関数宣言
// ============================================================================

sqlite3_open: (filename: String) -> SqliteDb = native("sqlite3_open")
sqlite3_close: (db: SqliteDb) -> Int32 = native("sqlite3_close")
sqlite3_exec: (db: SqliteDb, sql: String) -> Int32 = native("sqlite3_exec")
sqlite3_prepare_v2: (db: SqliteDb, sql: String) -> SqliteStmt = native("sqlite3_prepare_v2")
sqlite3_step: (stmt: SqliteStmt) -> Int32 = native("sqlite3_step")
sqlite3_finalize: (stmt: SqliteStmt) -> Int32 = native("sqlite3_finalize")

// ============================================================================
// メソッドバインド
// ============================================================================

// コンストラクタ（通常の関数）
SqliteDb.open = sqlite3_open

// メソッド（self は位置 0）
SqliteDb.close = sqlite3_close[0]
SqliteDb.exec = sqlite3_exec[0]
SqliteDb.prepare = sqlite3_prepare_v2[0]

// SqliteStmt のメソッド
SqliteStmt.step = sqlite3_step[0]
SqliteStmt.finalize = sqlite3_finalize[0]

// ============================================================================
// デストラクタ
// ============================================================================

SqliteDb.drop = sqlite3_close[0]
SqliteStmt.drop = sqlite3_finalize[0]
```

---

### 7. 不透明型のライフサイクル管理

不透明型は RFC-009 の所有権モデルに従うものであり、新たな概念は導入しない。

#### 7.1 基本原則

- **Move セマンティクス**：不透明型はデフォルトで Move、代入/引数渡し/戻り値 = 所有権の移動、コピー不可
- **RAII 解放**：スコープ終了時に自動的にデストラクタを呼び出す
- **消費追跡**：明示的にデストラクトされた変数は消費され、再利用不可

#### 7.2 デストラクタのバインド

`.drop` 規約を用いてデストラクタをバインドする。構文は通常のメソッドバインドと完全に同一：

```yaoxiang
// デストラクタバインド（self は位置 0）
SqliteDb.drop = sqlite3_close[0]
SqliteStmt.drop = sqlite3_finalize[0]
```

コンパイラは `.drop` バインドを認識し、スコープ終了時に自動的に呼び出す。**新しいキーワードも trait システムも導入しない**——これはメソッドバインド + RAII であり、RFC-009 で既に約束されているセマンティクスである。

#### 7.3 自動デストラクト

```yaoxiang
{
    db = SqliteDb.open("test.db")
    stmt = db.prepare("SELECT * FROM users")
    stmt.step()
    // ← スコープ終了、定義と逆順で自動的にデストラクト：
    //   stmt.drop()  → sqlite3_finalize(stmt)
    //   db.drop()    → sqlite3_close(db)
}
```

**デストラクト順序**：定義順序の逆順（後で定義されたものが先にデストラクトされる）、RAII セマンティクスと一致する。

#### 7.4 明示的デストラクト

```yaoxiang
db = SqliteDb.open("test.db")
db.close()              // 明示的デストラクト。close と drop は同じ——バインドした名前がそのまま使われる
db.exec("...")          // ❌ コンパイルエラー：db は既に消費済み（Move 後は読み取り不可）
```

「close vs drop」という個別の概念は存在しない。`SqliteDb.drop = sqlite3_close[0]` の後では、`db.close()` と `db.drop()` は同じ関数である。

#### 7.5 デストラクトと Move

```yaoxiang
db = SqliteDb.open("test.db")
db2 = db                // Move：所有権が db2 に移動
// ここで db は無効
// ← スコープ終了、自動的に db2.drop() を呼び出す

// 関数による消費
process_db: (db: SqliteDb) -> Void = {
    result = db.exec("...")
    // ← 関数終了、db はここでデストラクト
}

db = SqliteDb.open("test.db")
process_db(db)          // 関数に Move、関数終了時にデストラクト
// ここで db は無効
```

#### 7.6 Null 処理

```yaoxiang
// null を返す可能性あり → ? マークでオプショナル型、ユーザの処理が必須
sqlite3_open: (filename: String) -> ?SqliteDb = native("sqlite3_open")

db = SqliteDb.open("test.db")
match db {
    Some(db) => {
        db.exec("SELECT * FROM users")
        // ← スコープ終了、自動的に db.drop() を呼び出す
    }
    None => print("オープン失敗")
}

// null を返さない → マークしない、null の場合は panic
// C 関数が決して null を返さないと約束されているシナリオに使用
sqlite3_get_global: () -> SqliteDb = native("sqlite3_get_global")
```

**設計原則**：C が null を返すケースは、ユーザに処理させる（`?`）か、panic で露呈させるかのいずれかである。「静かに無視する」という第 3 の選択肢は存在しない。

#### 7.7 デストラクト失敗の処理

```yaoxiang
// デストラクタはエラーコードを返す可能性がある
sqlite3_close: (db: SqliteDb) -> Int32 = native("sqlite3_close")
SqliteDb.drop = sqlite3_close[0]

// コンパイラの挙動：
//   debug モード：デストラクトの戻り値 != 0 → panic（問題を露呈）
//   release モード：戻り値を無視（C 標準では close 失敗はメモリ安全性に影響しない）
```

#### 7.8 spawn ブロック内のデストラクト

```yaoxiang
// リソース型は spawn ブロック内で自動的にシリアル化され、デストラクトも本質的に安全
{
    db = SqliteDb.open("test.db")
    result = db.exec("...")
}  // ← シリアル化保証：exec 完了後に drop、並行競合なし

// spawn 境界をまたぐ Move
db = SqliteDb.open("test.db")
spawn {
    use(db)             // spawn に Move
    // ← spawn 終了、自動的にデストラクト
}
```

#### 7.9 デストラクタが不要な型

不透明型に対して `.drop` のバインドは強制されない。デストラクタをバインドしない型は、スコープ終了時に何も行わない——静的データのラッパやグローバルハンドルなど、解放が不要なシナリオに適している。

コンパイラは debug モードにおいて `.drop` がバインドされていない不透明型に対して lint ヒントを出し（デフォルトは warn）、ユーザに確認を促す。

#### 7.10 ライフサイクルルールまとめ

| シナリオ | 挙動 | 出所 |
|------|------|------|
| 不透明型の代入 | Move（コピー不可） | RFC-009 |
| `.drop` バインド | メソッドバインド構文 `[0]` | 本文書 §3 |
| スコープ終了 | 逆順で自動的に `.drop()` を呼び出す | RFC-009 RAII |
| 明示的 `.close()` | 変数を消費、以後使用不可 | RFC-009 Move セマンティクス |
| Null 戻り | `?T` オプショナル / 直接 panic | 本文書 §7.6 |
| spawn ブロック内 | 自動シリアル化、デストラクト安全 | RFC-024 |

---

## トレードオフ

### 利点

1. **統一セマンティクス**：すべての `{}` ブロックの return セマンティクスが一貫
2. **新しいキーワード不要**：既存の unsafe と return を使用
3. **明示的な定義**：型の属性は定義時に決定、unsafe ブロックから返される → 不透明、空ボディ → 真空、推論不要
4. **ライフサイクルは新概念ゼロ**：`.drop` = メソッドバインド + RAII、trait システムなし、新しいキーワードなし
5. **安全**：不透明型のフィールドアクセスには unsafe 権限が必要、デストラクト後の変数は使用不可
6. **実用性**：yx-bindgen がバインディング（デストラクタ含む）を自動生成

### 欠点

1. **unsafe ブロックのスコープ**：`{}` ブロックの return セマンティクスの変更が必要
2. **yx-bindgen の保守**：新しい C ライブラリのサポートを継続的に更新する必要がある

---

## 実装戦略

### フェーズ 1：コアメカニズム (v0.8)

- [ ] `{}` ブロックの return セマンティクスの実装
- [ ] FFI 型定義の実装
- [ ] FFI 関数宣言の実装
- [ ] メソッドバインドの実装

### フェーズ 2：ライフサイクル管理 (v0.9)

- [ ] `.drop` デストラクタバインドの実装
- [ ] スコープ終了時の自動デストラクトの実装
- [ ] デストラクト後の変数消費チェックの実装
- [ ] `?T` と FFI null 戻り統合の実装
- [ ] 内部 handle ストレージの実装
- [ ] 不透明型の直接生成禁止の実装

### フェーズ 3：ツールチェイン (v1.0)

- [ ] yx-bindgen の実装
- [ ] Linux/macOS/Windows サポート
- [ ] 統合テスト

---

## 他の RFC との関係

- **RFC-008**：Runtime 並行モデル、FFI 呼び出しは独立したタスクとして扱われる
- **RFC-009**：所有権モデル——Move セマンティクス、RAII、`?` オプショナル型、不透明型のライフサイクル管理は完全にこれに基づく
- **RFC-010**：統一型構文、`{}` ブロックの return セマンティクス
- **RFC-024**：並行モデル、spawn ブロック内の FFI 挙動とデストラクト安全性

---

## 設計決定記録

| 決定 | 結論 | 理由 | 日付 |
|------|------|------|------|
| FFI 型定義 | unsafe ブロック + return | 統一セマンティクス、新しいキーワード不要 | 2026-06-05 |
| 不透明型の判定 | unsafe ブロックで明示的定義 | 型の属性は定義時に決定、外部推論に依存しない | 2026-06-05 |
| メソッドバインド | `[0]` 構文 | self の位置を明確化 | 2026-06-05 |
| コンストラクタ | 通常の関数バインド | 特殊構文不要 | 2026-06-05 |
| spawn ブロック挙動 | リソース型の自動シリアル化 | 安全性、並行モデルと整合 | 2026-06-05 |
| デストラクタ | `.drop = native_fn[0]` | メソッドバインド + RAII、新概念ゼロ | 2026-06-10 |
| Null 処理 | `?T` オプショナル / 直接 panic | C の問題を隠さない | 2026-06-10 |

---

## 参考文献

### YaoXiang 公式ドキュメント

- [RFC-008 Runtime 並行モデル](./008-runtime-concurrency-model.md)
- [RFC-009 所有権モデル](./009-ownership-model.md)
- [RFC-010 統一型構文](./010-unified-type-syntax.md)
- [RFC-024 並行モデル](./024-concurrency-model.md)

### 外部参考

- [Rust FFI](https://doc.rust-lang.org/nomicon/ffi.html)
- [Python ctypes](https://docs.python.org/3/library/ctypes.html)
- [Rust libloading](https://docs.rs/libloading/latest/libloading/)

---

## ライフサイクルと帰属

| 状態 | 場所 | 説明 |
|------|------|------|
| **審査中** | `docs/design/rfc/` | コミュニティでの議論受付中 |
| **承認済み** | `docs/design/rfc/accepted/` | 正式な設計ドキュメント |