# 並行モデル仕様

本ドキュメントでは、YaoXiang プログラミング言語の並行モデル仕様を定義します。、非同期プログラミング、並行プリミティブ、メモリモデルを含みます。

---

## 第1章：spawn 関数

### 1.1 spawn 関数

```
SpawnFn     ::= Identifier ':' FnType 'spawn' '=' Lambda
FnType      ::= '(' ParamTypes? ')' '->' TypeExpr ('@' Annotation)?
Annotation  ::= 'block' | 'eager'
```

**関数アノテーション**：

| アノテーション | 位置 | 動作 |
|------|------|------|
| `@block` | 戻り値型の後 | 並行最適化を無効化し、完全逐次実行 |
| `@eager` | 戻り値型の後 | 先行評価を強制 |

**構文例**：

```
// spawn 関数：並行実行可能
fetch_data: (url: String) -> JSON spawn = { ... }

// @block 同期関数：完全逐次実行
main: () -> Void @block = { ... }

// @eager 先行関数：即時実行
compute: (n: Int) -> Int @eager = { ... }
```

### 1.2 spawn ブロック

明示的に宣言された並行スコープであり、ブロック内のタスクは spawn 実行されます：

```
SpawnBlock  ::= '(' Pattern (',' Pattern)* ')' '=' 'spawn' '{' Expr (',' Expr)* '}'
```

**例**：

```
// spawn ブロック：明示的並行
(result_a, result_b) = spawn {
    parse(fetch("url1")),
    parse(fetch("url2"))
}
```

### 1.3 spawn ループ

データ並列ループであり、ループ本体が全データ要素に対して spawn 実行されます：

```
SpawnFor    ::= Identifier '=' 'spawn' 'for' Identifier 'in' Expr '{' Expr '}'
```

**例**：

```
// spawn ループ：データ並列
results = spawn for item in items {
    process(item)
}
```

### 1.4 錯誤伝播演算子

```
ErrorPropagate ::= Expr '?'
```

**例**：

```
process: (p: Point) -> Result[Data, Error] = {
    data = fetch_data()?      // 自動的錯誤伝播
    transform(data)?
}
```

---

## 第2章：メモリ管理

### 2.1 所有権モデル

YaoXiang は**所有権モデル**を採用してメモリを管理し、各値は一意の所有者を持ちます：

| セマンティクス | 説明 | 構文 |
|------|------|------|
| **Move** | デフォルトセマンティクス、所有権移転 | `p2 = p` |
| **ref** | 共有（Arc 参照カウント） | `shared = ref p` |
| **clone()** | 明示的コピー | `p2 = p.clone()` |

### 2.2 Move セマンティクス（デフォルト）

```yaoxiang
// 代入 = Move（ゼロコピー）
p: Point = Point(1.0, 2.0)
p2 = p              // Move、p は無効化

// 関数引数 = Move
process: (p: Point) -> Void = {
    // p の所有権が移転する
}

// 戻り値 = Move
create: () -> Point = {
    p = Point(1.0, 2.0)
    return p        // Move、所有権移転
}
```

### 2.3 ref キーワード（Arc）

`ref` キーワードは**参照カウントポインタ**（Arc）を作成し、安全な共有に使用されます：

```yaoxiang
// Arc の作成
p: Point = Point(1.0, 2.0)
shared = ref p      // Arc、スレッドセーフ

// 共有アクセス
spawn(() => print(shared.x))   // 安全

// Arc はライフサイクルを自動管理
// shared がスコープを抜けると、カウントがゼロになり自動解放
```

**特徴**：
- スレッドセーフな参照カウント
- ライフサイクル自動管理
- spawn 境界を越えた安全 garant

### 2.4 clone() 明示的コピー

```yaoxiang
// 明示的に値をコピー
p: Point = Point(1.0, 2.0)
p2 = p.clone()      // p と p2 は独立

// どちらも変更可能で、互いに影響しない
p.x = 0.0           // 正しい
p2.x = 0.0          // 正しい
```

### 2.5 unsafe コードブロック

`unsafe` コードブロックは、生ポインタの使用を許可し、システムレベルプログラミングに使用されます：

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
- ユーザーはダングリングや解放後使用しないように保証する必要がある
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

## 第3章：並行安全性

### 3.1 Send / Sync 制約

| 制約 | セマンティクス | 説明 |
|------|------|------|
| **Send** | スレッド間転送を安全に可能 | 値を別のスレッドに移動できる |
| **Sync** | スレッド間共有を安全に可能 | 不変参照を別のスレッドに共有できる |

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
| `ref T` | ✅ | ✅ | Arc スレッドセーフ |
| `*T` | ❌ | ❌ | 生ポインタは安全ではない |

### 3.2 Send/Sync 制約階層

```
Send ──► スレッド間転送を安全に可能
  │
  └──► Sync ──► スレッド間共有を安全に可能
       │
       └──► Send + Sync を満たす型は自動並行可能

Arc[T] は Send + Sync を実装（スレッドセーフ参照カウント）
Mutex[T] は内部可変性を提供
```

### 3.3 並行安全型

| 型 | セマンティクス | 並行安全 | 説明 |
|------|------|----------|------|
| `T` | 不変データ | ✅ 安全 | デフォルト型、複タスク読み取りで競合なし |
| `Ref[T]` | 可変参照 | ⚠️ 同期必要 | 並行変更可能としてマーク、ロック使用をコンパイル検査 |
| `Atomic[T]` | アトミック型 | ✅ 安全 | 低レベルアトミック操作、ロックフリー並行 |
| `Mutex[T]` | 相互排除ロック包装 | ✅ 安全 | 自動ロック・アンロック、コンパイル時保証 |
| `RwLock[T]` | 読み書きロック包装 | ✅ 安全 | 読み取り多い書き込み少ないシナリオを最適化 |

**構文**：

```
Mutex[T]    // 相互排除ロック包装された可変データ
Atomic[T]   // アトミック型（Int、Float のみ対応）
RwLock[T]   // 読み書きロック包装
```

**with 構文糖衣**：

```
with mutex.lock() {
    // 臨界区間：Mutex により保護
    ...
}
```

---

## 付録：並行構文早見表

### A.1 spawn 構文

```yaoxiang
// spawn 関数
fetch_data: (url: String) -> JSON spawn = { ... }

// spawn ブロック
(result_a, result_b) = spawn {
    parse(fetch("url1")),
    parse(fetch("url2"))
}

// spawn ループ
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
// 相互排除ロック
mutex: Mutex[Int] = Mutex(0)
with mutex.lock() {
    // 臨界区間
}

// アトミック型
counter: Atomic[Int] = Atomic(0)
counter.increment()

// 読み書きロック
data: RwLock[Data] = RwLock(data)
with data.read() {
    // 読み取り操作
}
```