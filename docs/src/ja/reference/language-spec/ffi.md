# FFI 仕様

本ファイルは YaoXiang プログラミング言語の FFI（外部関数インタフェース）仕様を定義する。型定義、関数宣言、メソッドバインディング、不透明型の処理を含む。

> **詳細設計**：FFI の完全な設計、動機、トレードオフについては [RFC-026: FFI コア機構](../design/rfc/accepted/026-ffi-core-mechanism.md) を参照。

---

## 第一章：概要

### 1.1 FFI の基本原則

```
すべての {} 内の return はその内容を一つ上のスコープに返す
デフォルトで return がない場合は Void を返す
```

### 1.2 FFI の構成要素

| コンポーネント | 説明 | 構文 |
|------|------|------|
| 型定義 | FFI 型（不透明または透明）を定義する | `unsafe {}` + `return` |
| 関数宣言 | 外部関数を宣言する | `native("symbol")` |
| メソッドバインディング | 型にメソッドをバインドする | `[0]` 構文 |

---

## 第二章：FFI 型定義

### 2.1 不透明型

不透明型は `unsafe {}` ブロック内で定義され、`return` によって一つ上のスコープへ返される：

```yaoxiang
// unsafe ブロック内で不透明型を定義
SqliteDb = unsafe {
    SqliteDb: Type = {
        handle: *Void  // 生のポインタ
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

### 2.2 透明型

透明型は `unsafe {}` ブロックを必要とせず、直接定義する：

```yaoxiang
// 透明型
Point: Type = {
    x: Int32,
    y: Int32
}

// ユーザが直接作成可能
p: Point = Point { x: 1, y: 2 }
```

### 2.3 不透明型の判定

コンパイラは不透明型と真空型を自動的に判定する：

```yaoxiang
// 不透明型（native 関数から参照される）
SqliteDb: Type = {}
sqlite3_open: (filename: String) -> SqliteDb = native("sqlite3_open")
// → SqliteDb は native 関数から参照される → 不透明型

// 真空型（native 関数から参照されない）
MyType: Type = {}
// → MyType は native 関数から参照されない → 真空型
```

**判定ルール**：
- 型が `native` 関数から参照される場合 → 不透明型
- そうでない場合 → 真空型

---

## 第三章：FFI 関数宣言

### 3.1 native 構文

外部関数の宣言には `native("symbol")` 構文を使用する：

```yaoxiang
// FFI 関数宣言
sqlite3_open: (filename: String) -> SqliteDb = native("sqlite3_open")
sqlite3_close: (db: SqliteDb) -> Int32 = native("sqlite3_close")
sqlite3_exec: (db: SqliteDb, sql: String) -> Int32 = native("sqlite3_exec")
```

### 3.2 引数型のマッピング

FFI 関数の引数型には YaoXiang 型を直接使用し、C 型のマッピングはコンパイラが自動的に処理する：

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
| `struct T*` | `T`（透明型）|
| `typedef struct T T` | `T`（不透明型）|

### 3.3 戻り型

FFI 関数の戻り型には YaoXiang 型を直接使用する：

```yaoxiang
// 不透明型を返す
sqlite3_open: (filename: String) -> SqliteDb = native("sqlite3_open")

// 透明型を返す
get_point: () -> Point = native("get_point")

// 基本型を返す
get_value: () -> Int32 = native("get_value")
```

---

## 第四章：メソッドバインディング

### 4.1 `[0]` 構文

`[0]` 構文を使用して、関数の引数タプルにおける self 引数の位置を指定する：

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

### 4.2 コンストラクタのバインディング

コンストラクタには `[0]` を付けず、通常の関数としてバインドする：

```yaoxiang
// FFI 関数
sqlite3_open: (filename: String) -> SqliteDb = native("sqlite3_open")

// コンストラクタのバインディング（通常の関数）
SqliteDb.open = sqlite3_open
```

**呼び出し方法**：
```yaoxiang
// コンストラクタを通して生成
db = SqliteDb.open("test.db")
```

### 4.3 バインディングの位置

型はデータコンテナであるため、メソッドバインディングは任意の位置で行える：

```yaoxiang
// 型定義の後にバインド
SqliteDb.close = sqlite3_close[0]

// 他のファイル内でバインド
SqliteDb.exec = sqlite3_exec[0]

// コンパイラが最終的にチェックする
```

---

## 第五章：spawn ブロックにおける FFI の挙動

### 5.1 リソース型の自動直列化

FFI 型がリソース型である場合、spawn ブロック内で自動的に直列化される：

```yaoxiang
// SqliteDb はリソース型
(a, b) = spawn {
    db1 = SqliteDb.open("db1.sqlite"),  // SqliteDb リソース
    db2 = SqliteDb.open("db2.sqlite")   // 異なるインスタンスなので並列可能
}

(a, b) = spawn {
    result1 = db.exec("SELECT ..."),  // 同一の SqliteDb
    result2 = db.exec("INSERT ...")   // 自動的に直列化
}
```

### 5.2 非リソース型は並列実行可能

FFI 型がリソース型でない場合、spawn ブロック内で並列実行できる：

```yaoxiang
// Float はリソース型ではない
(a, b) = spawn {
    result1 = sin(1.0),  // 並列可能
    result2 = cos(1.0)   // 並列可能
}
```

---

## 第六章：yx-bindgen ツールチェーン

### 6.1 生成内容

yx-bindgen は以下の内容を生成する：
- FFI 型定義（unsafe ブロック + return）
- FFI 関数宣言（native 構文）
- メソッドバインディング（`[0]` 構文）

### 6.2 生成例

```bash
yx-bindgen --header /usr/include/sqlite3.h --output sqlite3_bindings.yx
```

生成結果：

```yaoxiang
// sqlite3_bindings.yx
// 自動生成されたファイル、手動で編集しないこと

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

// コンストラクタ（通常の関数）
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

## 付録：FFI 構文クイックリファレンス

### A.1 型定義

```yaoxiang
// 不透明型
SqliteDb = unsafe {
    SqliteDb: Type = {
        handle: *Void
    }
    return SqliteDb
}

// 透明型
Point: Type = {
    x: Int32,
    y: Int32
}
```

### A.2 関数宣言

```yaoxiang
// FFI 関数宣言
sqlite3_open: (filename: String) -> SqliteDb = native("sqlite3_open")
sqlite3_close: (db: SqliteDb) -> Int32 = native("sqlite3_close")
```

### A.3 メソッドバインディング

```yaoxiang
// コンストラクタ（通常の関数）
SqliteDb.open = sqlite3_open

// メソッド（self は位置 0）
SqliteDb.close = sqlite3_close[0]
```

### A.4 呼び出し方法

```yaoxiang
// コンストラクタを通して生成
db = SqliteDb.open("test.db")

// メソッド呼び出し
db.close()
db.exec("SELECT * FROM users")
```