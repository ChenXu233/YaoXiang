---
title: 'RFC-036: std.test テストフレームワークと yaoxiang test コマンド'
status: '承認済み'
author: '晨煦'
created: '2026-07-26'
updated: '2026-08-02'
accepted: '2026-08-02'
issue: '#94, #95, #221'
---

# RFC-036: std.test テストフレームワークと yaoxiang test コマンド

## 概要

YaoXiang に標準テストフレームワーク `std.test` モジュールと `yaoxiang test`
CLI サブコマンドを導入する。テストファイルは通常の `.yx` ファイルであり、`std.assert.assert` + exit
code で合格/失敗を判定する。`std.test`
モジュールは純粋な YaoXiang で実装され、初の dogfooding ライブラリである。`yaoxiang test`
は CLI ツールであり、コンパイラ機能ではない——parser、IR、バイトコード、実行器には一切変更を加えない。

## 動機

### なぜテストフレームワークが必要なのか？

現在の YaoXiang のテストカバレッジは Rust 側の `#[test]` と `tests/`
統合テストに依存している。これは以下を意味する：

1. 標準ライブラリ（std.math / std.list / std.dict / std.convert /
   std.io）のユニットテストを YaoXiang で記述できない
2. `#117 標準ライブラリ各モジュールのユニットテストカバレッジ`
   がブロックされている、利用可能なテスト基盤がないため
3. 言語機能の回帰テスト（例：RFC-032 spawn セマンティクスの変更）に自動化手段がない

### 重要な制約

- **17 キーワード鉄則**：新しいキーワードや構文構造を導入しない
- **コンパイラ変更ゼロ**：parser、IR、バイトコード、実行器には触れない
- **セルフホスティング優先**：テストライブラリは YaoXiang で記述され、初の dogfooding ライブラリとなる

## アーキテクチャ

```
┌──────────────────────────────────────────────────────────────┐
│                    yaoxiang test                              │
│                                                              │
│  CLI 层:  yaoxiang test [--filter --fail-fast --json ...]    │
│              │                                               │
│  发现层:    读取 yaoxiang.toml → [tool.test] patterns         │
│              默认: tests/**/*.yx                              │
│              │                                               │
│  执行层:    对每个文件: yaoxiang run <file>                    │
│              检查 exit code → 串行执行                        │
│              │                                               │
│  报告层:    PASS/FAIL → 汇总                                  │
│              支持 --json / --verbose / --fail-fast            │
│                                                              │
│  断言层:    std.test (纯 YaoXiang，自举)                      │
│              底层: std.assert.assert                          │
│              诊断: f"Expected {expected}, got {actual}"       │
└──────────────────────────────────────────────────────────────┘
```

### 中核原則

1. **テストフレームワークはコンパイラ機能ではなく、CLI ツールである** — `yaoxiang run`
   はすでに「テストを実行する」ことができるため、`yaoxiang test`
   は単にすべてのファイルを実行してレポートを提示してくれるだけである
2. **コンパイラ変更ゼロ** — `@test`
   アノテーションスキャン、バイトコードメタデータセグメント、実行器の特殊入口を導入しない
3. **セルフホスティング** — `std.test` モジュールは純粋な YaoXiang で実装され、底層で
   `std.assert.assert` を呼び出す
4. **テストファイルは通常の `.yx` ファイル** — exit code で合格/失敗を判定する

## 詳細設計

### 1. CLI 設計

```
yaoxiang test [OPTIONS] [PATHS]

Arguments:
  [PATHS]...      指定测试文件或目录（默认: 从 yaoxiang.toml 读取，否则 tests/）

Options:
  --filter <NAME>     只跑文件名包含 <NAME> 的测试
  --fail-fast         遇到第一个失败就停止
  --verbose, -v       显示每个测试的详细 stdout/stderr
  --list              只列出测试文件，不跑
  --no-progress       不显示进度条（CI 场景）
  --json              输出 JSON 格式结果（CI 集成用）
```

#### 出力フォーマット

**デフォルト出力**：

```
Running 5 tests from 3 files...

tests/math_test.yx ........................ PASS (0.002s)
tests/list_test.yx ........................ PASS (0.001s)
tests/string_test.yx ...................... FAIL (0.003s)
  `-- Expected "hello", got "world"
      at tests/string_test.yx:12:5

Results: 2 passed, 1 failed, 0 skipped (0.006s)
```

**JSON 出力**（`--json`）：

```json
{
  "summary": { "total": 3, "passed": 2, "failed": 1, "skipped": 0, "time_secs": 0.006 },
  "tests": [
    { "file": "tests/math_test.yx", "passed": true, "time_secs": 0.002 },
    {
      "file": "tests/string_test.yx",
      "passed": false,
      "time_secs": 0.003,
      "error": "Expected \"hello\", got \"world\"",
      "exit_code": 1
    }
  ]
}
```

### 2. yaoxiang.toml 設定

`[tool.test]` の下に配置し、RFC-015 の `[tool.*]` サードパーティ拡張規約に準拠する：

```toml
[project]
name = "my-project"

[tool.test]
patterns = ["tests/**/*.yx"]
# 未来可扩展:
# exclude = ["tests/fixtures/**"]
# parallel = true
```

- デフォルト `patterns = ["tests/**/*.yx"]` — ユーザーはゼロコンフィグで即座に使い始められる
- 単一ファイルモード（`yaoxiang test foo.yx`）は直接実行され、設定は読み込まれない
- 将来的に独立リポジトリへ分割される可能性がある（`[tool.test]` の位置は変わらない）

### 3. std.test モジュール（純粋な YaoXiang）

```yaoxiang
// std/test.yx — Pure YaoXiang test assertion library
// First dogfooding library: YaoXiang's test library written in YaoXiang

use std.assert

assert_eq = (a, b) => {
    assert.assert(a == b, f"Expected {b}, got {a}")
}

assert_ne = (a, b) => {
    assert.assert(a != b, f"Expected not equal to {b}, got {a}")
}

assert_true = (cond: Bool) => {
    assert.assert(cond, f"Expected true, got {cond}")
}

assert_false = (cond: Bool) => {
    assert.assert(cond == false, f"Expected false, got {cond}")
}
```

- 4 つのアサーション関数、すべて `f"..."` で診断情報を生成する
- `assert_eq` / `assert_ne` は**型注釈なしのパラメータ**（`Any`）を使用——2026-08-02 実証：`==`/`!=`
  と f-string 補間は Any 上で正常に動作（Int/String ともに検証済み）、**型システム（generics）に依存しない**。将来的に generics 対応後は型注釈を追加可能
- `assert_true` / `assert_false` のパラメータに `Bool` 型注釈；`assert_false` は `cond == false`
  で否定を表現する（`not` 単項構文は #251 で権威化が進行中、安定後に移行可能）
- `std.test` は native コードに一切依存せず、純粋な YaoXiang で実装される

### 4. 標準ライブラリのロード機構（重要な設計）

**Phase 1：バイナリへの埋め込み**

`std/test.yx`（および将来的に YaoXiang で記述されるすべての標準ライブラリモジュール）はビルド時にバイナリへ埋め込まれる：

```rust
// build.rs 或构建脚本，自动生成
pub const STD_YX_FILES: &[(&str, &str)] = &[
    ("std/test.yx", r#"..."#),  // 源代码文本
    // 未来更多
];
```

モジュールシステム（RFC-029、2026-08-02 に完全に実装済み）が接続ポイントを提供する：Registry は native モジュールとソースモジュールの両方を保持し、orchestrator が複数ファイルの編成を担当する。`use std.test`
の解決順序：

1. まず Rust の native モジュールを検索（既存の仕組み、例えば `std.assert`）
2. ヒットしない場合、埋め込まれた `STD_YX_FILES`
   を検索——ヒットすれば**仮想パス**（例：`<std>/test.yx`）をシードモジュールとして orchestrator に注入し、通常のパイプライン（parse
   → typecheck → IR）を通る
3. ヒットしない場合、ファイルシステム検出（ユーザーモジュール）へ進む

埋め込まれたソースモジュール内部の `use std.assert` は resolver によって通常通り native
registry へ解決される——native とソースモジュールは Registry 内で共存し、種類をまたいだ依存関係が自然に成立する。埋め込まれたモジュールは**オンデマンドでコンパイル**される：import されたときのみパイプラインに入る。

利点：

- 単一ファイルモードでも `use std.test` が動作する
- 標準ライブラリのバージョンとバイナリが厳密に結合され、バージョン不整合が発生しない
- ユーザーが標準ライブラリパスを設定する必要がない

**将来：ファイルシステム標準ライブラリ**

YaoXiang のプロジェクトモードが成熟した後、標準ライブラリはファイルシステム形式へ移行する。詳細は RFC-014 の更新を参照。

### 5. 検出と実行

**前提条件（2026-08-02 レビュー決議）**：CLI `run` を orchestrator へ接続する。現状 CLI `run`
は単一ファイルパイプライン（`run_file_with_diagnostics`）を通り、ユーザーモジュールのインポートを解析できない。一方、`yaoxiang test`
の子プロセスモデルは CLI 能力を継承し、テストファイルがプロジェクトモジュールをインポートすることは中核的なシナリオである。したがって Phase
1 ではまず CLI `Run` のソースブランチを
`run_project`（orchestrator、ディレクトリ再帰検出）へ委譲する；#247（use に沿ったオンデマンド検出）はその後の純粋なパフォーマンス最適化として重ねる。import なしの単一ファイルは orchestrator 経由でも動作が等価であり、バイトコードブランチは変わらない。

**検出フェーズ**：

1. `[PATHS]` が指定されている場合、指定されたパスを直接使用する
2. そうでなければ `yaoxiang.toml` の `[tool.test].patterns` を読み込む
3. 設定がない場合、デフォルトで `tests/**/*.yx`
4. `--filter` を適用してフィルタリングする（ファイル名に含まれるか）

**実行フェーズ**：

1. 各ファイルに対して：`yaoxiang run --debug-info <file>` で子プロセスを起動する（`--debug-info`
   はランタイムエラーにソース位置を付与する——2026-08-02 実証で stack trace が `file:line:col`
   を出力）
2. exit code を確認：0 なら PASS、0 以外なら FAIL
3. レポート用に stdout/stderr をキャプチャする
4. シリアル実行のみ（Phase 1）、将来的に `--parallel` をサポート
5. `--fail-fast` の場合、最初の FAIL で即座に停止する

### 6. テスト分離

テスト分離はプロセスレベルの境界によって自然に実現される：

- 各テストファイルは独立した子プロセスで実行される
- 各子プロセスは独立した Heap、Frame、NativeContext を有する
- 一つのテストファイルで panic が発生しても他のテストファイルに影響しない
- 追加の独立 Heap コンテキスト機構は不要

## 既存システムとの関係

| 項目                                                | 関係                                                                                                  |
| --------------------------------------------------- | ----------------------------------------------------------------------------------------------------- |
| Rust `#[test]`                                      | 変更なし、コンパイラ内部のテストは引き続き Rust を使用                                                |
| 既存の `.yx` 統合テスト（`tests/yaoxiang/`）        | `yaoxiang test` で検出・実行される                                                                    |
| `std.assert.assert(cond)`                           | 保持し、`std.test` の底層がこれに依存する                                                             |
| モジュールシステム（RFC-029）                       | 埋め込まれたソースモジュールは Registry/orchestrator 経由で接続；CLI `run` の orchestrator 接続が前提 |
| `#200` リファクタ（`io.println` → `assert.assert`） | `yaoxiang test` と完全一致する方向性                                                                  |
| `@` アノテーション                                  | 使用せず、`@test` も導入しない                                                                        |

## 実装戦略

### Phase 1：中核機能

変更範囲：

- `src/util/diagnostic/mod.rs` / `src/main.rs` — CLI `Run` のソースブランチを `run_project`
  へ委譲（複数ファイル実行の前提）
- `src/main.rs` — 新規 `Test` サブコマンド
- `src/std/test.yx` — 新規の純粋 YaoXiang モジュール
- `build.rs` — `std/*.yx` をバイナリへ埋め込む
- orchestrator / Registry — 埋め込まれたソースから仮想パスで `.yx`
  モジュールをロードすることをサポート
- RFC-015 設定解析 — `[tool.test]` セクション
- 子プロセス実行（`--debug-info`）+ レポート

成果物：

- `yaoxiang test` が基本的に使用可能になる
- `std.test` の 4 つのアサーション関数
- デフォルトの `tests/**/*.yx` 検出
- シリアル実行 + デフォルト出力フォーマット

### Phase 2：完成

- `--filter` / `--fail-fast` / `--verbose` パラメータ
- `--json` 出力（CI 統合用）
- `--list` オプション
- `--no-progress` オプション

### Phase 3：高度化

- `--parallel` 並列実行（spawn 並行モデルの完成に依存）
- `[tool.test].exclude` 設定
- より多くのアサーション関数（例：Float 用 `assert_approx_eq`）

## リスクと緩和

| リスク                                                        | 確率 | 緩和                                                                       |
| ------------------------------------------------------------- | ---- | -------------------------------------------------------------------------- |
| `f"..."` が Any 上の補間で失敗する                            | なし | 2026-08-02 に実証済み（Int/String ともに正常）                             |
| 子プロセス起動のオーバーヘッドがテスト速度に影響する          | 中   | Phase 1 はシリアル実行で許容範囲；Phase 3 の並列化で緩和                   |
| `yaoxiang.toml` 設定解析が現在の CLI に存在しない             | 低   | 簡単な拡張であり、コア機能に影響しない                                     |
| CLI run の orchestrator 接続が動作回帰を引き起こす            | 低   | import なしの単一ファイルパスは等価；orchestrator は統合テストでカバー済み |
| 埋め込まれた `.yx` ソースファイルがバイナリサイズを増加させる | 低   | `.yx` ソースファイルは非常に小さく、無視できる                             |

## 未解決問題

- [x] `std/test.yx` 内の `use std.assert`
      の参照が正しく解決されるか？——**解決済み（2026-08-02）**。モジュールシステム（RFC-029）の実装後、native とソースモジュールは Registry 内で共存し、resolver が統一的に解決するため、種類をまたいだ依存関係が自然に成立する
- [x] テスト出力の `f"..."` の汎用 `to_string`
      が新しい型制約を導入するか？——**解決済み（2026-08-02）**。実証により型注釈なしのパラメータ（Any）上で
      `==`/`!=` と f-string 補間がともに動作（Int/String で検証済み）、新しい制約を導入しない
- [x] `?` 汎用パラメータの実現可能性は？——**解決済み（2026-08-02）**：`?`
      型構文は現時点で存在せず（かつ静かに飲み込まれるため、別の issue で追跡中）、Phase
      1 のアサーション関数は型注釈なしのパラメータを使用し、generics 型システムに依存しない

## 設計決定記録

| 決定                   | 決定内容                                                              | 日付       | 理由                                                                         |
| ---------------------- | --------------------------------------------------------------------- | ---------- | ---------------------------------------------------------------------------- |
| テストマーカー方式     | `@test` アノテーションを使用せず、テストファイルは通常の `.yx` とする | 2026-07-26 | コンパイラ変更ゼロ、子プロセスがそのまま分離となる                           |
| アサーション方式       | `std.test` モジュールは純粋な YaoXiang 関数                           | 2026-07-26 | セルフホスティング、native コードなし                                        |
| テスト実行モデル       | 子プロセス `yaoxiang run <file>` + exit code                          | 2026-07-26 | プロセスレベル分離、コンパイラ変更ゼロ                                       |
| 標準ライブラリのロード | 現在はバイナリへ埋め込み、将来的にファイルシステム化                  | 2026-07-26 | バージョン結合、単一ファイルで動作                                           |
| アサーション引数の型   | 型注釈なしのパラメータ（Any）、generics 型システムに依存しない        | 2026-08-02 | `?` 型構文が存在しない；Any 上で比較・補間が実証済み                         |
| 複数ファイル実行       | CLI `run` を `run_project`（orchestrator）へ委譲することを前提とする  | 2026-08-02 | 子プロセスモデルが CLI 能力を継承；#247 は純粋なパフォーマンス最適化に格下げ |
| レポートのソース位置   | 子プロセスに `--debug-info` を付与する                                | 2026-08-02 | 実証で stack trace が `file:line:col` を出力する                             |

## 参考文献

- [RFC-014: パッケージ管理システム設計](../accepted/014-package-manager.md)
  — 標準ライブラリのディレクトリ構造
- [RFC-015: 設定システム](../accepted/015-configuration-system.md) — `[tool.test]` 設定セクション
- [RFC-030: assert メカニズム](../review/030-assert-mechanism.md) — 底層の依存関係
- [Rust `#[test`] メカニズム](https://doc.rust-lang.org/book/ch11-01-writing-tests.html) — 参考設計
