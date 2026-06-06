```yaml
---
title: "RFC-026：FFI コアメカニズム"
status: "審査中"
author: "晨煦"
created: "2026-06-05"
updated: "2026-06-05"
---

# RFC-026：FFI コアメカニズム

> **参考**:
> - [RFC-008: Runtime 並行モデルとスケジューラ切り離し設計](./008-runtime-concurrency-model.md)
> - [RFC-009: 所有権モデル設計](./009-ownership-model.md)
> - [RFC-010: 統一タイプ構文](./010-unified-type-syntax.md)
> - [RFC-024: spawn ブロックに基づく並行モデル](./024-concurrency-model.md)

> **廃止**:
> - [RFC-020: 動的モジュールと FFI 統合](./020-dynamic-modules-ffi.md) — 内容は本ドキュメントに統合済み
> - [RFC-021: ライブラリ駆動 FFI 拡張と異言語呼び出しサポート](./021-library-driven-ffi-extension.md) — 内容は本ドキュメントに統合済み

## 概要

本ドキュメントは YaoXiang の FFI（外部関数インターフェース）コアメカニズムを定義します：

1. **FFI 型定義**：`unsafe {}` ブロックで不透明型を定義し、`return` で上位スコープに返す
2. **FFI 関数宣言**：`native("symbol")` 構文で外部関数を宣言
3. **メソッドバインディング**：`[0]` 構文で self パラメータの位置を指定
4. **不透明型の処理**：コンパイラが自動的に不透明型とボイド型を判断
5. **spawn ブロック内の FFI 動作**：リソース型は自動的に直列化、非リソース型は並行可能

**コア設計——1つの原則で統一半語論**：

```
すべての {} 内の return は内容を上位スコープに返す
デフォルトで return がない場合は Void を返す
```

---

## 動機

### なぜこの設計が必要なのか？

RFC-020 と RFC-021 はそれぞれ FFI の異なる側面を定義しています：
- RFC-020：動的モジュールと FFI 統合
- RFC-021：ライブラリ駆動 FFI 拡張

両者に重複があるため、統一された FFI 仕様に統合する必要があります。

### 設計目標

1. **統一**：すべての `{}` ブロックの return 意味論を一貫させる
2. **安全**：不透明型のフィールドアクセスには unsafe 権限が必要
3. **簡潔**：新しいキーワードや特殊マーカーが不要
4. **実用的**：yx-bindgen が自動的にバインディングを生成

---

## 提案

### 1. FFI 型定義

#### 1.1 unsafe ブロック + return 意味論

unsafe ブロック内で不透明型を定義し、return で上位スコープに返す：

```yaoxiang
// unsafe ブロック内で不透明型を定義
SqliteDb = unsafe {
    SqliteDb: Type = {
        handle: *Void  // ベアポインタ
    }
    return SqliteDb
}

// SqliteDb は unsafe ブロック外で使用可能
db = sqlite3_open("test.db")

// ❌ コンパイルエラー：handle フィールドには unsafe 権限が必要
handle = db.handle

// ✅ メソッド呼び出し経由
db.close()
```

#### 1.2 透明型

透明型は unsafe ブロックなしで直接定義：

```yaoxiang
// 透明型
Point: Type = {
    x: Int32,
    y: Int32
}

// ユーザーは直接作成可能
p: Point = Point { x: 1, y: 2 }
```

#### 1.3 不透明型の判断

コンパイラが自動的に不透明型とボイド型を判断：

```yaoxiang
// 不透明型（native 関数で参照される）
SqliteDb: Type = {}
sqlite3_open: (filename: String) -> SqliteDb = native("sqlite3_open")
// → SqliteDb は native 関数で参照される → 不透明型

// ボイド型（native 関数で参照されない）
MyType: Type = {}
// → MyType は native 関数で参照されない → ボイド型
```

**判断ルール**：
- 型が `native` 関数で参照される → 不透明型
- それ以外 → ボイド型

---

### 2. FFI 関数宣言

#### 2.1 native 構文

`native("symbol")` 構文で外部関数を宣言：

```yaoxiang
// FFI 関数宣言
sqlite3_open: (filename: String) -> SqliteDb = native("sqlite3_open")
sqlite3_close: (db: SqliteDb) -> Int32 = native("sqlite3_close")
sqlite3_exec: (db: SqliteDb, sql: String) -> Int32 = native("sqlite3_exec")
```

#### 2.2 パラメータ型のマッピング

FFI 関数のパラメータ型は直接 YaoXiang 型を使用し、コンパイラが自動的に C 型マッピングを処理：

| C 型 | YaoXiang 型 |
|------|-------------|
| `int` | `Int32` |
| `long` | `Int64` |
| `float` | `Float32` |
| `double` | `Float64` |
| `char` | `Char` |
| `char*` | `String` |
| `bool` | `Bool` |
| `size_t` | `Uint` |
| `void*` | `*Void` |
| `struct T*` | `T`（透明型）|
| `typedef struct T T` | `T`（不透明型）|

#### 2.3 戻り値型

FFI 関数の戻り値型は直接 YaoXiang 型を使用：

```yaoxiang
// 不透明型を返す
sqlite3_open: (filename: String) -> SqliteDb = native("sqlite3_open")

// 透明型を返す
get_point: () -> Point = native("get_point")

// 基本型を返す
get_value: () -> Int32 = native("get_value")
```

---

### 3. メソッドバインディング

#### 3.1 [0] 構文

`[0]` 構文で self パラメータが関数パラメータタプル内の位置を指定：

```yaoxiang
// FFI 関数
sqlite3_close: (db: SqliteDb) -> Int32 = native("sqlite3_close")
sqlite3_exec: (db: SqliteDb, sql: String) -> Int32 = native("sqlite3_exec")

// メソッドバインディング（self は位置 0）
SqliteDb.close = sqlite3_close[0]
SqliteDb.exec = sqlite3_exec[0]
```

**呼び出し方法**：
```yaoxiang
db = sqlite3_open("test.db")

// メソッド呼び出し
db.close()  // sqlite3_close(db) と等価
db.exec("SELECT * FROM users")  // sqlite3_exec(db, "SELECT * FROM users") と等価
```

#### 3.2 コンストラクターバインディング

コンストラクターには `[0]` を付けず、普通関数としてバインディング：

```yaoxiang
// FFI 関数
sqlite3_open: (filename: String) -> SqliteDb = native("sqlite3_open")

// コンストラクターバインディング（普通関数）
SqliteDb.open = sqlite3_open
```

**呼び出し方法**：
```yaoxiang
// コンストラクター経由で作成
db = SqliteDb.open("test.db")
```

#### 3.3 バインディングの位置

メソッドバインディングはどこでも配置可能（型はデータコンテナだから）：

```yaoxiang
// 型定義後にバインディング
SqliteDb.close = sqlite3_close[0]

// 他のファイルでバインディング
SqliteDb.exec = sqlite3_exec[0]

// コンパイラが最終的にはすべてチェック
```

---

### 4. 不透明型の処理

#### 4.1 コンパイラの自動処理

コンパイラが自動的に不透明型を判断し、内部で C ポインタ存储を処理：

```text
コンパイラ分析：
    SqliteDb: Type = {}
    sqlite3_open: ... -> SqliteDb = native("sqlite3_open")

推論：
    SqliteDb は不透明型
    内部で自動的に @internal handle: *Void を追加
    ユーザーが直接作成することを禁止
```

#### 4.2 ユーザーコード

```yaoxiang
import sqlite3_bindings

// ✅ コンストラクター経由で作成
db = SqliteDb.open("test.db")

// ❌ コンパイルエラー：不透明型を直接作成不可
db: SqliteDb = {}

// ✅ メソッド呼び出し経由
result = db.exec("SELECT * FROM users")
db.close()
```

---

### 5. spawn ブロック内の FFI 動作

#### 5.1 リソース型は自動的に直列化

FFI 型がリソース型の場合、spawn ブロック内で自動的に直列化：

```yaoxiang
// SqliteDb はリソース型
(a, b) = spawn {
    db1 = SqliteDb.open("db1.sqlite"),  // SqliteDb リソース
    db2 = SqliteDb.open("db2.sqlite")   // 異なるインスタンス、並行可能
}

(a, b) = spawn {
    result1 = db.exec("SELECT ..."),  // 同じ SqliteDb
    result2 = db.exec("INSERT ...")   // 自動的に直列化
}
```

#### 5.2 非リソース型は並行可能

FFI 型がリソース型でない場合、spawn ブロック内で並行可能：

```yaoxiang
// Float はリソース型ではない
(a, b) = spawn {
    result1 = sin(1.0),  // 並行可能
    result2 = cos(1.0)   // 並行可能
}
```

---

### 6. yx-bindgen ツールチェーン

#### 6.1 生成内容

yx-bindgen は以下を生成：
- FFI 型定義（unsafe ブロック + return）
- FFI 関数宣言（native 構文）
- メソッドバインディング（[0] 構文）

#### 6.2 生成例

```bash
yx-bindgen --header /usr/include/sqlite3.h --output sqlite3_bindings.yx
```

生成結果：

```yaoxiang
// sqlite3_bindings.yx
// 自動生成、手動編集禁止

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
// メソッドバインディング
// ============================================================================

// コンストラクター（普通関数）
SqliteDb.open = sqlite3_open

// メソッド（self は位置 0）
SqliteDb.close = sqlite3_close[0]
SqliteDb.exec = sqlite3_exec[0]
SqliteDb.prepare = sqlite3_prepare_v2[0]

// SqliteStmt のメソッド
SqliteStmt.step = sqlite3_step[0]
SqliteStmt.finalize = sqlite3_finalize[0]
```

---

## トレードオフ

### 利点

1. **統一意味論**：すべての `{}` ブロックの return 意味論が一貫
2. **新しいキーワードが不要**：既存の unsafe と return を使用
3. **特殊マーカーが不要**：コンパイラが自動的に不透明型を判断
4. **安全**：不透明型のフィールドアクセスには unsafe 権限が必要
5. **実用的**：yx-bindgen が自動的にバインディングを生成

### 欠点

1. **unsafe ブロックのスコープ**：`{}` ブロックの return 意味論を変更する必要がある
2. **コンパイラの複雑さ**：不透明型を自動的に判断する必要がある
3. **yx-bindgen のメンテナンス**：新しい C ライブラリをサポートするために継続的に更新が必要

---

## 実装戦略

### フェーズ 1：コアメカニズム (v0.8)

- [ ] unsafe ブロックの return 意味論を実装
- [ ] FFI 型定義を実装
- [ ] FFI 関数宣言を実装
- [ ] メソッドバインディングを実装

### フェーズ 2：不透明型 (v0.9)

- [ ] コンパイラが自動的に不透明型を判断する機能を実装
- [ ] 内部 handle 存储を実装
- [ ] 不透明型の直接作成禁止を実装

### フェーズ 3：ツールチェーン (v1.0)

- [ ] yx-bindgen を実装
- [ ] Linux/macOS/Windows をサポート
- [ ] 統合テスト

---

## 他の RFC との関係

- **RFC-008**：Runtime 並行モデル、FFI 呼び出しは独立タスクとして実行
- **RFC-009**：所有権モデル、unsafe ブロックの意味論
- **RFC-010**：統一タイプ構文、`{}` ブロックの return 意味論
- **RFC-024**：並行モデル、spawn ブロック内の FFI 動作

---

## 設計判断記録

| 判断 | 決定 | 理由 | 日付 |
|------|------|------|------|
| FFI 型定義 | unsafe ブロック + return | 意味論を統一、新しいキーワードが不要 | 2026-06-05 |
| 不透明型判断 | コンパイラが自動判断 | 特殊マーカーが不要 | 2026-06-05 |
| メソッドバインディング | [0] 構文 | self の位置を明確化 | 2026-06-05 |
| コンストラクター | 普通関数のバインディング | 特殊構文が不要 | 2026-06-05 |
| spawn ブロックの動作 | リソース型は自動的に直列化 | 安全、並行モデルに適合 | 2026-06-05 |

---

## 参考文献

### YaoXiang 公式ドキュメント

- [RFC-008 Runtime 並行モデル](./008-runtime-concurrency-model.md)
- [RFC-009 所有権モデル](./009-ownership-model.md)
- [RFC-010 統一タイプ構文](./010-unified-type-syntax.md)
- [RFC-024 並行モデル](./024-concurrency-model.md)

### 外部参照

- [Rust FFI](https://doc.rust-lang.org/nomicon/ffi.html)
- [Python ctypes](https://docs.python.org/3/library/ctypes.html)
- [Rust libloading](https://docs.rs/libloading/latest/libloading/)

---

## ライフサイクルと行き先

| 状態 | 場所 | 説明 |
|------|------|------|
| **審査中** | `docs/design/rfc/` | コミュニティ議論がオープン |
| **承認済み** | `docs/design/rfc/accepted/` | 正式設計ドキュメント |
```