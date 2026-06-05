# FFI 仕様書

本書は YaoXiang プログラミング言語の FFI（外部関数インターフェース）仕様を定義ものであり、型定義、関数宣言、メソッドバインディング、不透明型の処理を含みます。

> **詳細設計**：FFI の完全な設計、動機、トレードオフについては [RFC-026: FFI コアメカニズム](../design/rfc/review/026-ffi-core-mechanism.md) を参照してください。

---

## 第1章：概要

### 1.1 FFI のコア原則

```
すべての {} 内の return は内容を上一个作用域に返す
デフォルトで return がない場合は Void を返す
```

### 1.2 FFI の構成要素

| コンポーネント | 説明 | 構文 |
|------|------|------|
| 型定義 | FFI 型の定義（不透明または透明） | `unsafe {}` + `return` |
| 関数宣言 | 外部関数の宣言 | `native("symbol")` |
| メソッドバインディング | 型へのメソッドのバインディング | `[0]` 構文 |

---

## 第2章：FFI 型定義

### 2.1 不透明型

不透明型は `unsafe {}` ブロックで定義され、`return` で上一个作用域に返されます：

```yaoxiang
// unsafe ブロックで不透明型を定義
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

// ✅ メソッド呼び出しでアクセス
db.close()
```

### 2.2 透明型

透明型は直接定義され、`unsafe {}` ブロックは不要です：

```yaoxiang
// 透明型
Point: Type = {
    x: Int32,
    y: Int32
}

// ユーザーは直接作成可能
p: Point = Point { x: 1, y: 2 }
```

### 2.3 不透明型の判断基準

コンパイラは不透明型と真空型を自動判別します：

```yaoxiang
// 不透明型（native 関数で参照されている）
SqliteDb: Type = {}
sqlite3_open: (filename: String) -> SqliteDb = native("sqlite3_open")
// → SqliteDb は native 関数で参照されている → 不透明型

// 真空型（native 関数で参照されていない）
MyType: Type = {}
// → MyType は native 関数で参照されていない → 真空型
```

**判断ルール**：
- 型が `native` 関数で参照されている場合 → 不透明型
- それ以外 → 真空型

---

## 第3章：FFI 関数宣言

### 3.1 native 構文

`native("symbol")` 構文を使用して外部関数を宣言します：

```yaoxiang
// FFI 関数宣言
sqlite3_open: (filename: String) -> SqliteDb = native("sqlite3_open")
sqlite3_close: (db: SqliteDb) -> Int32 = native("sqlite3_close")
sqlite3_exec: (db: SqliteDb, sql: String) -> Int32 = native("sqlite3_exec")
```

### 3.2 引数型のマッピング

FFI 関数の引数型は直接 YaoXiang 型を使用하며、コンパイラが C 型のマッピングを自動処理します：

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

### 3.3 戻り値の型

FFI 関数の戻り値の型は直接 YaoXiang 型を使用します：

```yaoxiang
// 不透明型を返す
sqlite3_open: (filename: String) -> SqliteDb = native("sqlite3_open")

// 透明型を返す
get_point: () -> Point = native("get_point")

// 基本型を返す
get_value: () -> Int32 = native("get_value")
```

---

## 第4章：メソッドバインディング

### 4.1 [0] 構文

`[0]` 構文を使用して、self 引数が関数引数タプル内の位置を指定します：

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

### 4.2 コンストラクタバインディング

コンストラクタには `[0]` を付けず、普通の関数としてバインディングします：

```yaoxiang
// FFI 関数
sqlite3_open: (filename: String) -> SqliteDb = native("sqlite3_open")

// コンストラクタバインディング（普通の関数）
SqliteDb.open = sqlite3_open
```

**呼び出し方法**：
```yaoxiang
// コンストラクタで作成
db = SqliteDb.open("test.db")
```

### 4.3 バインディングの位置

メソッドバインディングはどこでも行えます，这是因为型はデータコンテナであるためです：

```yaoxiang
// 型定義後にバインディング
SqliteDb.close = sqlite3_close[0]

// 他のファイルでバインディング
SqliteDb.exec = sqlite3_exec[0]

// コンパイラは最終的にすべてチェックする
```

---

## 第5章：spawn ブロック内の FFI 動作

### 5.1 リソース型は自動串行化

FFI 型がリソース型の場合、spawn ブロック内で自動串行化されます：

```yaoxiang
// SqliteDb はリソース型
(a, b) = spawn {
    db1 = SqliteDb.open("db1.sqlite"),  // SqliteDb リソース
    db2 = SqliteDb.open("db2.sqlite")   // 異なるインスタンス、並列可能
}

(a, b) = spawn {
    result1 = db.exec("SELECT ..."),  // 同一の SqliteDb
    result2 = db.exec("INSERT ...")   // 自動串行化
}
```

### 5.2 非リソース型は並列可能

FFI 型がリソース型でない場合、spawn ブロック内で並列実行できます：

```yaoxiang
// Float はリソース型ではない
(a, b) = spawn {
    result1 = sin(1.0),  // 並列可能
    result2 = cos(1.0)   // 並列可能
}
```

---

## 第6章：yx-bindgen ツールチェーン

### 6.1 生成内容

yx-bindgen は以下を生成します：
- FFI 型定義（unsafe ブロック + return）
- FFI 関数宣言（native 構文）
- メソッドバインディング（[0] 構文）

### 6.2 生成例

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

// コンストラクタ（普通の関数）
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

## 付録：FFI 構文早見表

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
// コンストラクタ（普通の関数）
SqliteDb.open = sqlite3_open

// メソッド（self は位置 0）
SqliteDb.close = sqlite3_close[0]
```

### A.4 呼び出し方法

```yaoxiang
// コンストラクタで作成
db = SqliteDb.open("test.db")

// メソッド呼び出し
db.close()
db.exec("SELECT * FROM users")
```