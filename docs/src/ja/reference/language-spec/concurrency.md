# 並行モデル規範

本ドキュメントは、YaoXiang プログラミング言語の並行モデル規範を定義ものであり、非同期プログラミング、並行プリミティブ、メモリモデルを含む。

---

## 第1章：並作関数

### 1.1 spawn 関数（並作関数）

```
SpawnFn     ::= Identifier ':' FnType 'spawn' '=' Lambda
FnType      ::= '(' ParamTypes? ')' '->' TypeExpr ('@' Annotation)?
Annotation  ::= 'block' | 'eager'
```

**関数アノテーション**：

| アノテーション | 位置 | 動作 |
|------|------|------|
| `@block` | 戻り値の型の後 | 並行最適化を無効化、完全逐次実行 |
| `@eager` | 戻り値の型の後 | 積極的評価を強制 |

**構文例**：

```
// 並作関数：並行実行可能
fetch_data: (url: String) -> JSON spawn = { ... }

// @block 同期関数：完全逐次実行
main: () -> Void @block = { ... }

// @eager 積極関数：即時実行
compute: (n: Int) -> Int @eager = { ... }
```

### 1.2 spawn ブロック

明示的に宣言された並行領域、ブロック内のタスクは並作実行される：

```
SpawnBlock  ::= '(' Pattern (',' Pattern)* ')' '=' 'spawn' '{' Expr (',' Expr)* '}'
```

**例**：

```yaoxiang
// 並作ブロック：明示的並行
(result_a, result_b) = spawn {
    parse(fetch("url1")),
    parse(fetch("url2"))
}
```

### 1.3 spawn ループ

データ並列ループ、ループ本体が全データ要素に対して並作実行される：

```
SpawnFor    ::= Identifier '=' 'spawn' 'for' Identifier 'in' Expr '{' Expr '}'
```

**例**：

```yaoxiang
// 並作ループ：データ並列
results = spawn for item in items {
    process(item)
}
```

### 1.4 エラー伝播演算子

```
ErrorPropagate ::= Expr '?'
```

**例**：

```yaoxiang
process: (p: Point) -> Result[Data, Error] = {
    data = fetch_data()?      // エラーを自動伝播
    transform(data)?
}
```

---

## 第2章：メモリ管理

### 2.1 所有権モデル

YaoXiang は**所有権モデル**を採用してメモリを管理し、各値には一意の所有者がいる：

| 構文 | 説明 | 構文 |
|------|------|------|
| **Move** | デフォルト構文、所有権移動 | `p2 = p` |
| **ref** | 共有（Arc 参照カウント） | `shared = ref p` |
| **clone()** | 明示的複製 | `p2 = p.clone()` |

### 2.2 Move 構文（デフォルト）

```yaoxiang
// 代入 = Move（ゼロコピー）
p: Point = Point(1.0, 2.0)
p2 = p              // Move、p は無効化

// 関数引数 = Move
process: (p: Point) -> Void = {
    // p の所有権が移動ってくる
}

// 戻り値 = Move
create: () -> Point = {
    p = Point(1.0, 2.0)
    return p        // Move、所有権を移動
}
```

### 2.3 ref キーワード（Arc）

`ref` キーワードは**参照カウントポインタ**（Arc）を作成し、安全な共有に使用する：

```yaoxiang
// Arc の作成
p: Point = Point(1.0, 2.0)
shared = ref p      // Arc、スレッド安全

// 共有アクセス
spawn(() => print(shared.x))   // 安全

// Arc は自動的にライフサイクルを管理
// shared がスコープを離れると、カウンタがゼロになり自動解放
```

**特徴**：

- スレッド安全参照カウント
- ライフサイクル自動管理
- spawn 境界を越えた安全

### 2.4 clone() 明示的複製

```yaoxiang
// 明示的に値を複製
p: Point = Point(1.0, 2.0)
p2 = p.clone()      // p と p2 は独立

// どちらも変更可能，互相に影響しない
p.x = 0.0           // 正しい
p2.x = 0.0          // 正しい
```

### 2.5 unsafe コードブロック

`unsafe` コードブロックは、生ポインタの使用を許可し、システムレベルプログラミングに使用する：

```yaoxiang
// 生ポインタ型
PtrType ::= '*' TypeExpr

// unsafe コードブロック
UnsafeBlock ::= 'unsafe' '{' Stmt* '}'
```

**例**：

```yaoxiang
p: Point = Point(1.0, 2.0)

// 生ポインタは unsafe ブロック内でのみ使用可能
unsafe {
    ptr: *Point = &p     // 生ポインタ取得
    (*ptr).x = 0.0       // 逆参照
}
```

**制限**：

- 生ポインタは `unsafe` ブロック内でのみ使用可能
- ユーザーはダングリング、使用後解放がないことを保証
- Send/Sync チェックに参加しない

### 2.6 所有権構文 BNF

```bnf
// === 所有権式 ===

// Move（デフォルト）
MoveExpr     ::= Expr

// ref Arc
RefExpr      ::= 'ref' Expr

// clone
CloneExpr    ::= Expr '.clone' '(' ')'

// === 生ポインタ（unsafe のみ） ===

PtrType       ::= '*' TypeExpr
UnsafeBlock   ::= 'unsafe' '{' Stmt* '}'
```

---

## 第3章：並行安全

### 3.1 Send / Sync 制約

| 制約 | 構文 | 説明 |
|------|------|------|
| **Send** | スレッド間を安全に転送可能 | 値を別のスレッドに移動できる |
| **Sync** | スレッド間を安全に共有可能 | 不変参照を別のスレッドに共有できる |

**自動導出**：

```
// Send 導出規則
Struct[T1, T2]: Send ⇐ T1: Send かつ T2: Send

// Sync 導出規則
Struct[T1, T2]: Sync ⇐ T1: Sync かつ T2: Sync
```

**型制約**：

| 型 | Send | Sync | 説明 |
|------|------|------|------|
| `T`（値） | ✅ | ✅ | 不変データ |
| `ref T` | ✅ | ✅ | Arc スレッド安全 |
| `*T` | ❌ | ❌ | 生ポインタ 安全ではない |

### 3.2 Send/Sync 制約階層

```
Send ──► スレッド間を安全に転送可能
  │
  └──► Sync ──► スレッド間を安全に共有可能
       │
       └──► Send + Sync を満たす型は自動並行可能

Arc[T] は Send + Sync を実装（スレッド安全参照カウント）
Mutex[T] は内部可変性を提供
```

### 3.3 並行安全型

| 型 | 構文 | 並行安全 | 説明 |
|------|------|----------|------|
| `T` | 不変データ | ✅ 安全 | デフォルト型、複数タスク読み取りで競合なし |
| `Ref[T]` | 可変参照 | ⚠️ 同期が必要 | 並行変更可能としてマーク、コンパイル時にロック使用検査 |
| `Atomic[T]` | アトミック型 | ✅ 安全 | 下位層アトミック操作、ロックフリー並行 |
| `Mutex[T]` | ミューテックスラッパー | ✅ 安全 | 自動ロック解除、コンパイル時保証 |
| `RwLock[T]` | 読書鎖ラッパー | ✅ 安全 | 読み取り多用書き込み少量シナリオ最適化 |

**構文**：

```yaoxiang
Mutex[T]    // ミューテックスラッパーの可変データ
Atomic[T]   // アトミック型（Int、Float のみ）
RwLock[T]   // 読書鎖ラッパー
```

**with 糖衣構文**：

```yaoxiang
with mutex.lock() {
    // 臨界区間：Mutex によって保護
    ...
}
```

---

## 付録：並行構文早見表

### A.1 spawn 構文

```yaoxiang
// 並作関数
fetch_data: (url: String) -> JSON spawn = { ... }

// 並作ブロック
(result_a, result_b) = spawn {
    parse(fetch("url1")),
    parse(fetch("url2"))
}

// 並作ループ
results = spawn for item in items {
    process(item)
}
```

### A.2 所有権構文

```yaoxiang
// Move（デフォルト）
p2 = p

// ref Arc
shared = ref p

// clone
p2 = p.clone()

// unsafe
unsafe {
    ptr: *Point = &p
    (*ptr).x = 0.0
}
```

### A.3 並行安全型

```yaoxiang
// ミューテックス
mutex: Mutex[Int] = Mutex(0)
with mutex.lock() {
    // 臨界区間
}

// アトミック型
counter: Atomic[Int] = Atomic(0)
counter.increment()

// 読書鎖
data: RwLock[Data] = RwLock(data)
with data.read() {
    // 読み取り操作
}
```