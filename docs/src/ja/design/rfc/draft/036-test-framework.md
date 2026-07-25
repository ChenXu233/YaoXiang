---
title: "RFC-036: std.test テストフレームワークと yaoxiang test コマンド"
status: "ドラフト"
author: "晨煦"
created: "2026-07-25"
updated: "2026-07-25"
issue: "#94"
---

# RFC-036: std.test テストフレームワークと yaoxiang test コマンド

## 概要

YaoXiang に標準テストフレームワーク `std.test` モジュールと `yaoxiang test` CLI サブコマンドを導入する。テスト発見は既存の `@test` 注釈でマークされた関数に基づき（**新構文はゼロ**）、アサーションは std.test モジュールからエクスポートされる純粋関数（`assert_eq`、`assert_ne`、`assert_ok`、`assert_err`）を使用し、テスト実行はコンパイラによる注釈スキャン + ランタイムスケジューリングによって行われる。新しいキーワード、予約語、構文構造は一切導入しない。

## 動機

### なぜテストフレームワークが必要か？

現在の YaoXiang のテストは Rust 側の `#[test]` と `tests/` 統合テストに分散しており、YaoXiang 言語自体のテストカバレッジは Rust で記述されることに依存している。これは以下を意味する：

1. YaoXiang 標準ライブラリ（std.math / std.list / std.dict / std.convert / std.io）の単体テストを YaoXiang で記述できない
2. `#117 標準ライブラリ各モジュールの単体テストカバレッジ` が、利用可能なテストインフラがないためブロックされている
3. 言語機能の回帰テスト（RFC-032 spawn セマンティクスの変更など）に自動化された手段がない

### 重要な制約：新構文ゼロ

YaoXiang は 17 個の中核キーワード（`pub`、`use`、`spawn`、`if`、`else`、`match`、`for`、`in`、`while`、`return`、`break`、`continue`、`as`、`ref`、`true`、`false`、`None`）のみを保持しており、これは設計マニフェストで明確にされた妥協不可能な原則である。

**テストフレームワークは新しいキーワード（`test` キーワードや `assert` キーワードなど）や新しい構文構造（`test` ブロックなど）を絶対に導入してはならない。**

## 提案

### 基本設計

```
┌─────────────────────────────────────────────────────────────────┐
│                      テストアーキテクチャ（3 層）                  │
│                                                                 │
│  ① 発見層：コンパイラが @test 注釈をスキャン → テスト一覧を生成   │
│      新構文は追加されず、@ 注釈は既存                              │
│                                                                 │
│  ② 実行層：yaoxiang test サブコマンド                            │
│      発見 → コンパイル → 並列実行 → レポート                      │
│                                                                 │
│  ③ アサーション層：std.test モジュールの純粋関数                   │
│      assert_eq / assert_ne / assert_ok / assert_err             │
│      新構文はゼロ、純粋関数呼び出し                                │
└─────────────────────────────────────────────────────────────────┘
```

### 使用例

```yaoxiang
# テストファイル：math_test.yx
use std.test
use std.math

# @test 注釈でテスト関数をマーク（@ は既存のトークン、新構文ではない）
@test
fn test_add() -> Void = {
    test.assert_eq(2 + 3, 5)
    test.assert_eq(-1 + 1, 0)
    test.assert_eq(0 + 0, 0)
}

@test
fn test_subtract() -> Void = {
    test.assert_eq(10 - 3, 7)
    test.assert_ne(10 - 3, 8)
}

@test
fn test_divide_by_zero() -> Void = {
    test.assert_err(1 / 0)
}
```

```bash
# すべてのテストを実行
yaoxiang test

# 単一ファイルを実行
yaoxiang test math_test.yx

# 名前に一致するテストを実行
yaoxiang test --filter "add"
```

### 構文変更

**なし。** 本提案は新しい構文構造を一切導入しない。

| 項目 | 状態 |
|------|------|
| 新キーワード | ❌ なし |
| 新予約語 | ❌ なし |
| 新構文構造 | ❌ なし |
| @test 注釈 | ✅ 既存の `@` トークンを再利用 |
| std.test モジュール | ✅ 新規モジュール、純粋関数 |

## 詳細設計

### 1. テスト発見メカニズム

#### 1.1 @test 注釈

`@test` は YaoXiang 既存の `@` 注釈構文を使用する。注釈機構は parser 層で既にサポートされている（RFC-008 における `@block`/`@eager` 注釈の使用が、その機構が利用可能であることを証明している）。

**注釈フォーマット**：

```yaoxiang
@test
fn test_name() -> Void = { ... }
```

**ルール**：
- `@test` は関数定義の直前に置く
- 注釈対象の関数は `() -> Void` シグネチャである必要がある
- 注釈は単なるマークであり、関数の振る舞いを変えない——`@test` 関数は通常の `run` 下でも呼び出し可能な通常関数として残る（ただし自動的に呼び出されることはない）
- テスト発見は `yaoxiang test` サブコマンド下でのみ有効

#### 1.2 テスト発見フロー

```
yaoxiang test
    │
    ▼
┌─────────────────────┐
│ すべての .yx ファイルを │  ← デフォルトで src/ と tests/ ディレクトリをスキャン
│ スキャン              │
│ @test 注釈関数を収集  │
└────────┬────────────┘
         ▼
┌─────────────────────┐
│ フィルタ：            │  ← --filter 引数で関数名にマッチ
│ --filter パターンマッチ│
└────────┬────────────┘
         ▼
┌─────────────────────┐
│ コンパイル：          │  ← 既存コンパイラを再利用、テストマークを
│ 各テストファイル      │     バイトコードに埋め込む
│ テスト登録コードを注入 │
└────────┬────────────┘
         ▼
┌─────────────────────┐
│ 実行：テストを並列実行 │  ← 既存の spawn 並行性モデルを再利用
│ 結果を収集           │
└────────┬────────────┘
         ▼
┌─────────────────────┐
│ レポート：            │
│ 成功/失敗/所要時間    │
│ JUnit XML（オプション）│
└─────────────────────┘
```

### 2. std.test モジュール設計

#### 2.1 モジュール構造

```
src/std/test.rs
    ├── assert_eq(actual, expected)     → Void
    ├── assert_ne(actual, expected)     → Void
    ├── assert_ok(value)                → Void
    ├── assert_err(value)               → Void
    ├── assert_true(cond)               → Void
    ├── assert_false(cond)              → Void
    ├── assert_passes()                 → Void
    └── assert_fails()                  → Void（明示的な失敗）
```

#### 2.2 アサーション関数仕様

すべてのアサーション関数は失敗時に以下の情報を含む `ExecutorError::TestAssertionFailed` を送出する：
- ファイルパス（注釈関数のソース位置から取得）
- 行番号
- 失敗情報（actual vs expected）
- テスト関数名

```yaoxiang
# std.test.assert_eq 実装の擬似コード
fn assert_eq(actual, expected) -> Void = {
    if actual == expected {
        return
    }
    raise TestAssertionFailed(
        "assertion failed: {actual} != {expected}",
        file, line, test_name
    )
}
```

#### 2.3 モジュール登録

既存の `StdModule` trait を通じて登録し、`std.io` / `std.math` などのモジュールと完全に一貫させる：

```rust
impl StdModule for TestModule {
    fn module_path(&self) -> &str { "std.test" }

    fn exports(&self) -> Vec<NativeExport> {
        vec![
            NativeExport::new("assert_eq",  "std.test.assert_eq",  "(a: T, b: T) -> Void", native_assert_eq),
            NativeExport::new("assert_ne",  "std.test.assert_ne",  "(a: T, b: T) -> Void", native_assert_ne),
            NativeExport::new("assert_ok",  "std.test.assert_ok",  "(r: Result(T)) -> Void", native_assert_ok),
            NativeExport::new("assert_err", "std.test.assert_err", "(r: Result(T)) -> Void", native_assert_err),
            NativeExport::new("assert_true", "std.test.assert_true", "(b: Bool) -> Void", native_assert_true),
            NativeExport::new("assert_false","std.test.assert_false","(b: Bool) -> Void", native_assert_false),
            NativeExport::new("fail",       "std.test.fail",       "(msg: String) -> Void", native_fail),
        ]
    }
}
```

**重要**：`assert_eq` / `assert_ne` の `T` にはジェネリクスサポートが必要である。現在のコンパイラにはジェネリクス関数のランタイムディスパッチに関する基盤が既に存在する（RFC-011）。ジェネリクスディスパッチがまだ完全でない場合、第 1 リリースは `Int` / `Float` / `String` / `Bool` の明示的なオーバーロードに限定し、後でアップグレードすることもできる。

### 3. yaoxiang test サブコマンド

#### 3.1 CLI 引数

`Commands` 列挙型に追加する：

```rust
/// Run YaoXiang tests
Test {
    /// Source files or directories to test (default: all .yx files in src/ and tests/)
    #[arg(value_name = "PATH", num_args = 0..)]
    paths: Vec<PathBuf>,

    /// Filter test names (substring match)
    #[arg(short, long, value_name = "PATTERN")]
    filter: Option<String>,

    /// Run tests serially (disable parallel execution)
    #[arg(long)]
    serial: bool,

    /// Number of parallel workers (0 = auto)
    #[arg(long, default_value = "0")]
    workers: usize,

    /// Output JUnit XML report
    #[arg(long, value_name = "FILE")]
    junit: Option<PathBuf>,

    /// Stop on first failure
    #[arg(long)]
    fail_fast: bool,
}
```

#### 3.2 実行フロー

```rust
match command {
    Commands::Test { paths, filter, serial, workers, junit, fail_fast } => {
        // 1. テストファイルを発見
        let test_files = discover_test_files(&paths);

        // 2. @test 注釈をスキャンしてテスト関数を収集
        let test_cases = scan_test_functions(&test_files, &filter);

        // 3. 各テストファイルをコンパイル（compiler.compile を再利用）
        let compiled = compile_tests(&test_cases);

        // 4. テストを並列実行（runtime spawn を再利用）
        let results = execute_tests(&compiled, serial, workers);

        // 5. レポートを生成
        print_test_report(&results, junit);
    }
}
```

#### 3.3 テスト分離

各テスト関数は独立したランタイムコンテキストで実行される：
- 独立した Heap（リスト/辞書がテスト間で共有汚染されない）
- 独立したレジスタファイル
- 独立した FFI Registry スナップショット（標準ライブラリの状態がテスト間で汚染されない）

実装方法：`execute_function` を呼び出すたびに新しい `Frame` + `Heap` を作成し、既存の分離機構（`src/backends/interpreter/executor/tests/execute.rs` を参照）を再利用する。

### 4. バイトコードレベルの変更

#### 4.1 テストメタデータ

コンパイラは `@test` 注釈をスキャンする際、生成されるバイトコードファイルの先頭にテストメタデータセグメントを追加する：

```
[Bytecode Header]
    ...
[Test Metadata Section]
    test_count: u16
    tests: [
        { name: str, file: str, line: u32, param_count: u8 }
    ]
```

#### 4.2 Executor の変更

`Executor` にテスト実行のエントリポイントを追加する：

```rust
impl Executor<'_> {
    /// テストとしてマークされた関数を実行
    fn run_test(&mut self, test: &TestMeta) -> TestResult {
        let start = Instant::now();
        match self.execute_function(&self.func_by_name(&test.name), &[]) {
            Ok(RuntimeValue::Unit) => TestResult { name: test.name.clone(), passed: true, elapsed, error: None },
            Ok(_) => TestResult { name: test.name.clone(), passed: false, elapsed, error: Some("test returned non-Void".to_string()) },
            Err(e) if is_assertion_error(&e) => TestResult { name: test.name.clone(), passed: false, elapsed, error: Some(e.to_string()) },
            Err(e) => TestResult { name: test.name.clone(), passed: false, elapsed, error: Some(format!("unexpected: {}", e)) },
        }
    }
}
```

### 5. 出力形式

#### 5.1 デフォルト出力

```
Running 5 tests...

PASS test_add (0.002s)
PASS test_subtract (0.001s)
FAIL test_divide_by_zero (0.003s)
  └── assertion failed: 1 / 0
      Expected: Error
      Actual:   ExecutorError: division by zero
PASS test_max_value (0.001s)
PASS test_min_value (0.001s)

Results: 4 passed, 1 failed, 0 skipped (0.007s)
```

#### 5.2 JUnit XML

```xml
<?xml version="1.0" encoding="UTF-8"?>
<testsuites>
  <testsuite name="math_test" tests="5" failures="1" time="0.007">
    <testcase name="test_add" classname="math_test" time="0.002"/>
    <testcase name="test_divide_by_zero" classname="math_test" time="0.003">
      <failure message="assertion failed: 1 / 0">ExecutorError: division by zero</failure>
    </testcase>
  </testsuite>
</testsuites>
```

## 型システムへの影響

**なし。** テストフレームワークは新しい型、型制約、型システム変更を導入しない。すべてのアサーション関数は `Void` を返し、引数は既存のジェネリクス基盤を使用する。

## ランタイム動作

- `@test` 注釈付き関数は `yaoxiang run` 下では振る舞いが変わらない——通常の関数のままである
- `yaoxiang test` サブコマンドのみがテスト発見と自動呼び出しをトリガーする
- テスト失敗で送出される `TestAssertionFailed` は通常の `ExecutorError` と同一のエラー処理パイプラインを共有する

## コンパイラの変更

| モジュール | 変更 |
|------|------|
| `src/frontend/core/parser/` | 既存の注釈解析（`@` トークン）を再利用、変更なし |
| `src/middle/core/ir_gen.rs` | `@test` 注釈をスキャンして AST ノードをマーク |
| `src/middle/passes/codegen/` | バイトコード先頭にテストメタデータセグメントを書き込む |
| `src/backends/interpreter/executor/` | `run_test` エントリポイントを追加 |
| `src/std/test.rs` | `std.test` モジュールの実装を追加 |
| `src/main.rs` | `Test` サブコマンド分岐を追加 |

## 後方互換性

- ✅ **完全な後方互換**：新しい `yaoxiang test` サブコマンドを追加するのみで、既存のサブコマンドの振る舞いは変更しない
- ✅ **既存コードに影響なし**：`@test` 注釈はオプションであり、注釈のない関数は一切影響を受けない
- ✅ **std モジュール登録は独立**：`std.test` は新しい `StdModule` 登録であり、既存モジュールに影響しない

## トレードオフ

### 利点

1. **新構文ゼロ**：YaoXiang の 17 キーワード制約に完全準拠
2. **既存機構を活用**：`@` 注釈、`StdModule` trait、`NativeExport`、spawn 並行性モデルをすべて再利用
3. **Rust のテスト体験と整合**：`#[test]` → `@test`、`assert_eq!` → `test.assert_eq()`
4. **拡張可能**：将来 `@benchmark`、`@ignore` などの注釈を追加でき、構文変更は不要

### 欠点

1. `assert_eq(a, b)` は関数呼び出しでありマクロではないため、Rust のように `a = 42, b = 43` という正確な値を出力することはできない（native 実装内で値をキャプチャする必要がある）
2. ジェネリクスアサーション（`assert_eq<T>`）はジェネリクスのランタイムディスパッチに依存する。この機能が完全でない場合、第 1 リリースでは型の特殊化が必要
3. 注釈スキャンはコンパイル時に行われるため、テストを動的に登録することはできない

## 代替案

| 案 | 説明 | 利点 | 欠点 |
|------|------|------|------|
| **A: 本案（@test + std.test モジュール）** | 注釈 + 純粋関数 | 新構文ゼロ、既存機構を活用 | マクロではなく関数呼び出し、診断情報がやや弱い |
| **B: テストブロック構文 `test { }`** | 新キーワード `test` を導入 | 構文が簡潔 | ❌ 17 キーワード制約に違反、parser の大幅変更が必要 |
| **C: 命名規約（`test_*` 接頭辞）** | 純粋な命名規約、`test_` 接頭辞の関数をスキャン | 注釈の変更不要 | 名前空間の汚染、誤マッチしやすく、明示的なマークがない |
| **D: 外部テストランナー** | 独立した Rust ツールが .yx テストをコンパイルして実行 | コンパイラを変更しない | アーキテクチャが分裂し、2 つのコンパイルフローになる |

**A を選択**：新構文ゼロ制約を満たす前提下では、注釈方式が最も明示的なマーク方式であり、名前空間の汚染も最小限である。

## 実装戦略

### Phase 1：インフラ（v0.7.8）

- `std.test` モジュール：`assert_eq` / `assert_ne` / `assert_ok` / `assert_err` / `fail`
- 初版は `Int` / `Float` / `String` / `Bool` の特殊化に限定
- 注釈解析の確認（`@` が parser 層で正しく AST に渡されることを確認）

### Phase 2：テスト発見と実行（v0.7.8）

- IR 層で `@test` 注釈をスキャンしてマーク
- バイトコードのテストメタデータセグメント
- `yaoxiang test` サブコマンドのスケルトン
- 基本的なテスト実行 + デフォルト出力

### Phase 3：改善（v0.7.9）

- `assert_true` / `assert_false`
- `--filter` / `--serial` / `--fail-fast` 引数
- JUnit XML 出力
- テスト分離（独立した Heap コンテキスト）
- ジェネリクスアサーションのサポート（ジェネリクスのランタイムディスパッチが完成している場合）

### 依存関係

| 依存 | 状態 | 説明 |
|------|------|------|
| `@` 注釈解析 | ✅ 既存 | lexer に既に `TokenKind::At` あり、parser で受け渡しを確認する必要あり |
| `StdModule` trait | ✅ 既存 | `src/std/mod.rs` に完全な実装あり |
| `NativeExport` | ✅ 既存 | 標準ライブラリ関数の登録機構 |
| ジェネリクスのランタイムディスパッチ | ⚠️ 一部実装 | `assert_eq<T>` ジェネリクス版に影響、Phase 1 は型の特殊化で回避可能 |
| `yaoxiang` CLI (clap) | ✅ 既存 | `Commands` 列挙型を直接拡張可能 |
| 並列実行モデル | ⚠️ ドラフト | RFC-024 は承認済みだが実装中、Phase 2 はシングルスレッド実装で対応可能 |

### リスク

1. **注釈解析の不完全性**：parser 層での `@test` 処理が不完全な場合（`@block`/`@eager` のみ認識）、注釈解析の拡張が必要。**緩和策**：注釈は本質的に `@` + 識別子であり、汎用的な注釈解析への拡張は作業量が少ない
2. **ジェネリクスディスパッチの未成熟**：`assert_eq<T>` はジェネリクスのランタイムサポートが必要。**緩和策**：Phase 1 ではジェネリクスディスパッチに依存せず、`Int` / `Float` / `String` / `Bool` の 4 つの明示的な関数で対応
3. **テスト分離の不足**：`std` モジュールのグローバル状態がテスト間で汚染される可能性。**緩和策**：テスト実行ごとに新しい `NativeContext` + `Heap` を作成

## オープンな問題

- [ ] `@test` 注釈は引数をサポートするか（例：`@test(reason = "flaky")`）？現時点ではサポートせず、シンプルに保つ
- [ ] テストモジュールの命名規約？推奨規約：テストファイルはソースファイルと同名に `_test` 接尾辞を付けたもの（例：`math.rs` → `math_test.yx`）
- [ ] 特定のテストをスキップする `@ignore` 注釈をサポートするか？Phase 1 では行わず、Phase 3 で検討
- [ ] アサーションの診断情報フォーマット？`actual` / `expected` の表示方法を決定する必要がある（現在の `RuntimeValue` は `format_value_with_prefix` をサポート済み）

## 付録A：既存のテストインフラとの関係

| 項目 | 既存 | 本 RFC | 関係 |
|------|------|--------|------|
| Rust `#[test]` | `src/**/tests/` ディレクトリ | 変更なし | コンパイラ内部のテストは引き続き Rust を使用 |
| YaoXiang `.yx` 統合テスト | `tests/yaoxiang/` | 変更なし | 既存の .yx 回帰テストファイルは引き続き統合テストとして使用 |
| `std.assert.assert(cond)` | 既存 | 変更なし | 通常コードでの使用のために保持 |
| `TestAssertionFailed` エラー | なし | 新規 | 新しいエラータイプ、既存のエラーコード仕様（RFC-013）に準拠 |

## 付録B：設計決定記録

| 決定 | 決定内容 | 日付 | 理由 |
|------|------|------|------|
| テストのマーク方式 | `@test` 注釈 | 2026-07-25 | 新構文ゼロ、`@` は既存 |
| アサーション方式 | `std.test` モジュールの関数 | 2026-07-25 | 純粋関数、マクロなし、構文変更なし |
| テスト実行モデル | 独立した Heap コンテキスト | 2026-07-25 | テスト間の状態汚染を回避 |
| ジェネリクスアサーション | Phase 1 は型の特殊化、Phase 3 でジェネリクス | 2026-07-25 | ジェネリクスディスパッチ未完成のリスクを回避 |

## 参考文献

- [RFC-008: Runtime 並行性モデル](../accepted/008-runtime-concurrency-model.md) — `@` 注釈機構の参考
- [RFC-013: エラーコード仕様](../accepted/013-error-code-specification.md) — `TestAssertionFailed` エラーコード
- [RFC-030: assert アサーション機構](../review/030-assert-mechanism.md) — 既存の `assert(cond)` のランタイム実装
- [RFC-011: ジェネリクスシステム](../accepted/011-generic-type-system.md) — ジェネリクスアサーションの型制約
- [Rust `#[test]` 機構](https://doc.rust-lang.org/book/ch11-01-writing-tests.html) — 参考設計
- [Go `testing` パッケージ](https://pkg.go.dev/testing) — 参考設計