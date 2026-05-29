# FFI 拡張设计方案

> **状態**: ✅ 完了（全部で 10 のステップが実装済み）
>
> **実装日**: 2025 年

## 一、背景と目標

### 1.1 現状（実施前）

現在の FFI アーキテクチャ：

```rust
type NativeHandler = fn(&[RuntimeValue]) -> Result<RuntimeValue, ExecutorError>;
```

**問題点**：

- native 関数が heap にアクセスできず、List/Dict を返せない
- native 関数がユーザーから渡された YaoXiang 関数を呼び出せない（高階関数を実現できない）
- インタープリタにハードコードされた特殊処理が散在している（len、dict_keys など）

### 1.2 目標

1. ✅ native 関数に heap へのアクセスを許可し、List/Dict を返すようにする
2. ✅ native 関数に YaoXiang 関数を呼び出す能力を与える（高階関数をサポート）
3. ✅ アーキテクチャを統一し、インタープリタのハードコードを取り除く

---

## 二、、全体設計

### 2.1 コア型の定義

```rust
// 実行コンテキスト - native 関数に渡される
pub struct NativeContext<'a> {
    /// ヒープメモリ管理
    pub heap: &'a mut Heap,
    /// コールバック：YaoXiang 関数を呼び出すために使用（高階関数のシナリオ）
    pub call_fn: Option<&'a mut dyn FnMut(&RuntimeValue, &[RuntimeValue]) -> Result<RuntimeValue, ExecutorError>>,
}

// Native 関数シグネチャの変更
pub type NativeHandler = fn(args: &[RuntimeValue], ctx: &mut NativeContext<'_>) -> Result<RuntimeValue, ExecutorError>;
```

> **実装説明**: 最終的な実装では、`Interpreter` 参照を直接保持する代わりに `call_fn` コールバッククロージャを使用しています。
> これにより、Rust の借用チェッカーの自己参照問題（Interpreter が heap と ffi を同時に所有する問題）を回避しています。

### 2.2 モジュール構造

```
src/backends/interpreter/
├── ffi.rs          # 変更点：NativeHandler 型、呼び出し規約
└── executor.rs    # 変更点：native 呼び出し時に Context を構築

src/std/
├── mod.rs         # 変更点：NativeHandler 型定義
├── io.rs          # 変更点：全関数のシグネチャ
├── math.rs        # 変更点：全関数のシグネチャ
├── string.rs      # 変更点：heap アクセスを実装
├── list.rs        # 変更点：heap アクセス + 高階関数
├── dict.rs        # 変更点：heap アクセス
└── ... 其他模块   # 変更点：全関数のシグネチャ
```

### 2.3 呼び出しフロー

```
ユーザーコードが native 関数を呼び出す
    ↓
BytecodeExecutor が CallNative/CallStatic を実行
    ↓
FFIRegistry から NativeHandler を取得
    ↓
NativeContext { heap, call_fn } を構築
    ↓
handler(args, &mut ctx) を呼び出す
    ↓
handler 内部で以下が可能：
  - ctx.heap にアクセスして List/Dict を割り当て/変更
  - ctx.call_function() を呼び出してユーザー関数を実行
    ↓
RuntimeValue を返す
```

---

## 三、詳細な實施ステップ

### ステップ 1：FFI 型定義の変更

**ファイル**：`src/std/mod.rs`

**変更内容**：

1. `NativeContext` 構造体定義を追加
2. `NativeHandler` 型エイリアスを変更
3. `NativeExport` 構造体を変更（任意）

**受入基準**：

- [x] `NativeContext` 構造体が `heap` と `call_fn` フィールドを含む
- [x] `NativeHandler` 型が `fn(args: &[RuntimeValue], ctx: &mut NativeContext<'_>) -> Result<RuntimeValue, ExecutorError>`
- [x] コンパイル通過

**テスト方案**：

- コンパイルテスト：`cargo check` が通過

---

### ステップ 2：FFI Registry の変更

**ファイル**：`src/backends/interpreter/ffi.rs`

**変更内容**：

1. `register()` メソッドのシグネチャを変更
2. `call()` メソッドを変更し、呼び出し時に ctx を渡す

**受入基準**：

- [x] `register(name, handler)` が新しいシグネチャの handler を受け入れる
- [x] `call(name, args, ctx)` が ctx を handler に渡す
- [x] コンパイル通過

**テスト方案**：

- コンパイルテスト：`cargo check` が通過

---

### ステップ 3：インタープリタの呼び出しポイントの変更

**ファイル**：`src/backends/interpreter/executor.rs`

**変更内容**：

1. `CallNative` バイトコード処理位置を見つける（約 600 行目）
2. native 関数を呼び出す前に `NativeContext` を構築
3. ctx を `ffi.call()` に渡す

**受入基準**：

- [x] native 関数を呼び出す時に NativeContext を作成する
- [x] NativeContext が有効な heap 参照を含む
- [x] NativeContext が高階関数シナリオ用の call_fn コールバックを含む
- [x] コンパイル通過

**テスト方案**：

- コンパイルテスト：`cargo check` が通過

---

### ステップ 4：std.io モジュールの更新

**ファイル**：`src/std/io.rs`

**変更内容**：

1. すべての native 関数のシグネチャを更新
2. `ctx` パラメータを追加

**対象関数**：

- `native_print`
- `native_println`
- `native_read_line`
- `native_read_file`
- `native_write_file`
- `native_append_file`

**受入基準**：

- [x] 全関数のシグネチャが新しい `NativeHandler` 型に準拠
- [x] 関数内部では ctx を使用しない（下位互換性）
- [x] コンパイル通過

**テスト方案**：

- [x] `std.io.print("test")` が正常に動作
- [x] `std.io.println("test")` が正常に動作

---

### ステップ 5：std.math モジュールの更新

**ファイル**：`src/std/math.rs`

**変更内容**：

1. すべての native 関数のシグネチャを更新
2. `ctx` パラメータを追加

**対象関数**：

- `native_abs`, `native_max`, `native_min`, `native_clamp`
- `native_fabs`, `native_fmax`, `native_fmin`, `native_pow`
- `native_sqrt`, `native_floor`, `native_ceil`, `native_round`
- `native_sin`, `native_cos`, `native_tan`
- `native_pi`, `native_e`, `native_tau`

**受入基準**：

- [x] 全関数のシグネチャが新しい型に準拠
- [x] コンパイル通過

**テスト方案**：

- [x] `std.math.abs(-5)` が 5 を返す
- [x] `std.math.sqrt(4)` が 2 を返す

---

### ステップ 6：std.string 完全機能の実装

**ファイル**：`src/std/string.rs`

**変更内容**：

1. 関数シグネチャを変更
2. heap アクセスを実装し、本当の List を返す

**対象関数**：

| 関数 | 実装方式 |
|------|----------|
| `split` | ctx.heap を使用して List を割り当て |
| `chars` | ctx.heap を使用して List を割り当て |
| `trim/upper/lower/replace` | 既に実装済み（heap 不要）|
| `contains/starts_with/ends_with` | 既に実装済み（heap 不要）|

**受入基準**：

- [x] `std.string.split("a,b", ",")` が `["a", "b"]` を返す
- [x] `std.string.chars("abc")` が `["a", "b", "c"]` を返す
- [x] コンパイル通過

**テスト方案**：

```yaoxiang
// split のテスト
let result = std.string.split("hello,world", ",");
assert(std.list.len(result) == 2);

// chars のテスト
let chars = std.string.chars("abc");
assert(std.list.len(chars) == 3);
```

---

### ステップ 7：std.list 完全機能の実装（高階関数を含む）

**ファイル**：`src/std/list.rs`

**変更内容**：

1. 全関数のシグネチャを変更
2. heap アクセスを実装
3. 高階関数呼び出しを実装

**対象関数**：

| 関数 | 実装方式 |
|------|----------|
| `push` | ctx.heap を使用して新しい List を割り当て |
| `pop` | heap から要素を取得 |
| `prepend` | ctx.heap を使用して新しい List を割り当て |
| `reverse` | ctx.heap を使用して新しい List を割り当て |
| `concat` | ctx.heap を使用して新しい List を割り当て |
| `map` | **ユーザー関数を呼び出す** |
| `filter` | **ユーザー関数を呼び出す** |
| `reduce` | **ユーザー関数を呼び出す** |
| `get/set/first/last/slice` | heap アクセス |

**高階関数の実装ポイント**：

```rust
fn native_map(args: &[RuntimeValue], ctx: &mut NativeContext<'_>) -> Result<RuntimeValue, ExecutorError> {
    // args[0] はリスト、args[1] はユーザー関数
    let list_handle = /* args[0] から抽出 */;
    let func_value = /* args[1] から抽出 */;

    // リスト要素を取得（借用衝突を避けるために clone）
    let items = match ctx.heap.get(list_handle) {
        Some(HeapValue::List(items)) => items.clone(),
        _ => return Err(...)
    };

    // 各要素に対してユーザー関数を呼び出す
    let mut result_items = Vec::new();
    for item in &items {
        let mapped = ctx.call_function(&func_value, &[item.clone()])?;
        result_items.push(mapped);
    }

    // 新しいリストを返す
    let new_handle = ctx.heap.allocate(HeapValue::List(result_items));
    Ok(RuntimeValue::List(new_handle))
}
```

**受入基準**：

- [x] `std.list.push([1, 2], 3)` が `[1, 2, 3]` を返す
- [x] `std.list.pop([1, 2, 3])` が `3` と残りの `[1, 2]` を返す
- [x] `std.list.map([1, 2], x => x * 2)` が `[2, 4]` を返す
- [x] `std.list.filter([1, 2, 3], x => x > 1)` が `[2, 3]` を返す
- [x] `std.list.reduce([1, 2, 3], (acc, x) => acc + x, 0)` が `6` を返す
- [x] コンパイル通過

**テスト方案**：

```yaoxiang
// push のテスト
let list1 = std.list.push([1, 2], 3);
assert(std.list.len(list1) == 3);

// map のテスト
let doubled = std.list.map([1, 2, 3], x => x * 2);
assert(std.list.get(doubled, 0) == 2);

// filter のテスト
let filtered = std.list.filter([1, 2, 3, 4], x => x > 2);
assert(std.list.len(filtered) == 2);

// reduce のテスト
let sum = std.list.reduce([1, 2, 3], (acc, x) => acc + x, 0);
assert(sum == 6);
```

---

### ステップ 8：std.dict 完全機能の実装

**ファイル**：`src/std/dict.rs`

**変更内容**：

1. 全関数のシグネチャを変更
2. heap アクセスを実装
3. Any 型キーをサポート

**対象関数**：

| 関数 | 実装方式 |
|------|----------|
| `get` | heap から Dict を取得し、キーを検索 |
| `set` | ctx.heap を使用して新しい Dict を割り当て |
| `has` | heap から Dict を取得し、キーをチェック |
| `keys/values/entries` | ctx.heap を使用して List を割り当て |
| `delete` | ctx.heap を使用して新しい Dict を割り当て |
| `merge` | ctx.heap を使用して 2 つの Dict をマージ |

**受入基準**：

- [x] `std.dict.get({a: 1}, "a")` が `1` を返す
- [x] `std.dict.set({a: 1}, "b", 2)` が `{a: 1, b: 2}` を返す
- [x] `std.dict.keys({a: 1, b: 2})` が `["a", "b"]` を返す
- [x] `std.dict.has({a: 1}, "a")` が `true` を返す
- [x] コンパイル通過

**テスト方案**：

```yaoxiang
// get のテスト
let d = {name: "tom", age: 20};
assert(std.dict.get(d, "name") == "tom");

// set のテスト
let d1 = {a: 1};
let d2 = std.dict.set(d1, "b", 2);
assert(std.dict.has(d2, "b") == true);

// keys のテスト
let keys = std.dict.keys({x: 1, y: 2});
assert(std.list.len(keys) == 2);
```

---

### ステップ 9：他の std モジュールの更新

**対象ファイル**：

- `src/std/net.rs`
- `src/std/time.rs`
- `src/std/os.rs`
- `src/std/concurrent.rs`
- `src/std/weak.rs`
- `src/std/ffi.rs`（テストコードがある場合）

**変更内容**：

- すべての native 関数のシグネチャを更新し、ctx パラメータを追加
- ctx を使用する必要のない関数は無視可能

**受入基準**：

- [x] すべての std モジュールのコンパイル通過
- [x] 既存機能が影響を受けない

---

### ステップ 10：インタープリタのハードコードのクリーンアップ

**ファイル**：`src/backends/interpreter/executor.rs`

**削除対象コード**：

- `len()` 特殊処理（約 609-634 行目）
- `dict_keys()` 特殊処理（約 637-666 行目）

**注意**：

- ✅ 先にステップ 6-8 を完了し、std ライブラリ関数が正常に動作することを確認
- その後、内蔵の `len()` を `std.list.len()` で置き換える
- 内蔵の `dict_keys()` を `std.dict.keys()` で置き換える

> **実装説明**: 実際の実装では、コンパイラ IR 生成段階で裸名 `"len"` と `"dict_keys"` の呼び出しが生成されるため、
> `register_all()` で汎用的な `builtin_len` と `builtin_dict_keys` 関数を追加で登録しています。
> これらは List/Tuple/Array/Dict/String/Bytes の長さ計算と辞書キー抽出をそれぞれ処理します。

**受入基準**：

- [x] `len()` ハードコードを削除した後、`len([1,2,3])` が引き続き動作（builtin_len FFI 登録経由）
- [x] `dict_keys()` ハードコードを削除した後、`dict_keys({a:1})` が引き続き動作（builtin_dict_keys FFI 登録経由）
- [x] コンパイル通過

---

## 四、テスト方案

### 4.1 ユニットテスト

`src/std/` ディレクトリにテストを追加：

```rust
#[cfg(test)]
mod tests {
    // string tests
    #[test]
    fn test_split() { ... }

    // list tests
    #[test]
    fn test_push() { ... }
    #[test]
    fn test_map() { ... }

    // dict tests
    #[test]
    fn test_get() { ... }
}
```

### 4.2 統合テスト

テストファイル `tests/std_primitives.yx` を作成：

```yaoxiang
// 文字列テスト
let s1 = std.string.trim("  hello  ");
assert(s1 == "hello");

let s2 = std.string.split("a,b,c", ",");
assert(std.list.len(s2) == 3);

// リストテスト
let l1 = std.list.push([1, 2], 3);
assert(std.list.len(l1) == 3);

let doubled = std.list.map([1, 2, 3], x => x * 2);
assert(std.list.get(doubled, 0) == 2);

// 辞書テスト
let d = std.dict.set({a: 1}, "b", 2);
assert(std.dict.has(d, "b") == true);

// 高階関数テスト
let filtered = std.list.filter([1, 2, 3, 4, 5], x => x > 2);
assert(std.list.len(filtered) == 3);

let sum = std.list.reduce([1, 2, 3, 4], (acc, x) => acc + x, 0);
assert(sum == 10);
```

### 4.3 回帰テスト

既存機能が影響を受けていないことを確認：

```bash
# 既存のテストを実行
cargo test

# 統合テストを実行
cargo run -- tests/std_primitives.yx
```

---

## 五、リスクとロールバック

### 5.1 リスク

| リスク | 影響 | 緩和措置 |
|------|------|----------|
| 変更量大 | バグ導入の可能性 | ステップ分け、各ステップでコンパイルテスト |
| 既存の native 関数を破壊 | 実行時エラー | 全 std モジュールのシグネチャを更新 |
| 高階関数呼び出しが複雑 | 実装難易度高い | 既存の interpreter 呼び出しロジックを参照 |

### 5.2 ロールバック方案

問題が発生した場合、git でロールバック可能：

```bash
git checkout -- src/std/ src/backends/interpreter/ffi.rs src/backends/interpreter/executor.rs
```

---

## 六、タイム見積もり

| ステップ | 見積時間 |
|------|----------|
| ステップ 1-3（FFI コア） | 1-2 時間 |
| ステップ 4-5（io/math の更新）| 30 分 |
| ステップ 6（string 完全版）| 30 分 |
| ステップ 7（list + 高階関数）| 1-2 時間 |
| ステップ 8（dict）| 1 時間 |
| ステップ 9-10（クリーンアップ）| 30 分 |
| **合計** | **5-6 時間** |

---

## 七、まとめ

**完了後の能力**：

```yaoxiang
// 文字列
std.string.split("a,b,c", ",")  // ["a", "b", "c"]
std.string.chars("hi")          // ["h", "i"]

// リスト
std.list.push([1,2], 3)         // [1, 2, 3]
std.list.map([1,2], x => x*2)   // [2, 4]
std.list.filter([1,2,3], x => x>1)  // [2, 3]
std.list.reduce([1,2,3], (a,x)=>a+x, 0)  // 6

// 辞書
std.dict.get({a:1}, "a")       // 1
std.dict.keys({a:1, b:2})      // ["a", "b"]
```