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
CLI サブコマンドを導入する。テストファイルは通常の `.yx` ファイルであり、`std.assert.assert` と exit
code で合格/不合格を判定する。 `std.test`
モジュールは純粋な YaoXiang で実装され、最初の dogfooding ライブラリとなる。 `yaoxiang test`
は CLI ツールであり、コンパイラ機能ではない——parser、IR、バイトコード、エグゼキュータへの変更は一切行わない。

## 動機

### なぜテストフレームワークが必要なのか？

現在の YaoXiang のテストカバレッジは Rust 側の `#[test]` と `tests/`
統合テストに依存している。これは次のような問題を引き起こしている：

1. 標準ライブラリ（std.math / std.list / std.dict / std.convert /
   std.io）のユニットテストを YaoXiang で記述できない
2. `#117 標準ライブラリ各モジュールのユニットテストカバレッジ`
   が、利用可能なテスト基盤がないためブロックされている
3. 言語機能の回帰テスト（RFC-032 の spawn セマンティクス変更など）に自動化された手段がない

### 重要な制約

- **17 キーワード鉄則**：新しいキーワードや構文構造を導入しない
- **コンパイラ変更ゼロ**：parser、IR、バイトコード、エグゼキュータに触れない
- **セルフホスティング優先**：テストライブラリを YaoXiang で記述する、最初の dogfooding ライブラリ

## アーキテクチャ

```
┌──────────────────────────────────────────────────────────────┐
│                    yaoxiang test                              │
│                                                              │
│  CLI 層:     yaoxiang test [--filter --fail-fast --json ...] │
│              │                                               │
│  検出層:     yaoxiang.toml を読み込み → [tool.test] patterns  │
│              デフォルト: tests/**/*.yx                        │
│              │                                               │
│  実行層:     各ファイルに対して: yaoxiang run <file>           │
│              exit code を確認 → 逐次実行                      │
│              │                                               │
│  レポート層: PASS/FAIL → 集計                                 │
│              --json / --verbose / --fail-fast をサポート     │
│                                                              │
│  断言層:     std.test (純粋な YaoXiang、セルフホスティング)  │
│              基盤: std.assert.assert                          │
│              診断: f"Expected {expected}, got {actual}"       │
└──────────────────────────────────────────────────────────────┘
```

### 核心原則

1. **テストフレームワークはコンパイラ機能ではなく、CLI ツールである** — `yaoxiang run`
   はすでに「テストを実行」でき、`yaoxiang test`
   は単にすべてのファイルを実行してレポートを表示するヘルパーに過ぎない
2. **コンパイラ変更ゼロ** — `@test`
   アノテーションスキャン、バイトコードメタデータセグメント、エグゼキュータ特殊エントリを導入しない
3. **セルフホスティング** — `std.test` モジュールは純粋な YaoXiang で実装され、基盤として
   `std.assert.assert` を呼び出す
4. **テストファイルは通常の `.yx` ファイル** — exit code で合格/不合格を判定する

## 詳細設計

### 1. CLI 設計

```
yaoxiang test [OPTIONS] [PATHS]

Arguments:
  [PATHS]...      テストファイルまたはディレクトリを指定（デフォルト: yaoxiang.toml から読み込み、なければ tests/）

Options:
  --filter <NAME>     ファイル名に <NAME> を含むテストのみ実行
  --fail-fast         最初の失敗で停止
  --verbose, -v       各テストの詳細な stdout/stderr を表示
  --list              テストファイルを列挙するのみで実行しない
  --no-progress       プログレスバーを表示しない（CI シナリオ）
  --json              結果を JSON 形式で出力（CI 連携用）
```

#### 出力形式

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

`[tool.test]` 配下に配置し、RFC-015 の `[tool.*]` サードパーティ拡張規約に準拠する：

```toml
[project]
name = "my-project"

[tool.test]
patterns = ["tests/**/*.yx"]
# 将来拡張:
# exclude = ["tests/fixtures/**"]
# parallel = true
```

- デフォルト `patterns = ["tests/**/*.yx"]` — ユーザーは設定不要でそのまま使える
- 単一ファイルモード（`yaoxiang test foo.yx`）は設定を無視して直接実行する
- 将来的には独立したリポジトリに分割される可能性がある（`[tool.test]` の位置は変更しない）

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
    assert.assert(!cond, f"Expected false, got {cond}")
}
```

- 4 つのアサーション関数、すべて `f"..."` で診断情報を生成する
- `assert_eq` / `assert_ne` は**注釈なし引数**（`Any`）を使用——2026-08-02 実証：`==`/`!=`
  と f-string 補間は Any 上で正常動作（Int/String ともに検証済み）、**ジェネリクスシステムに依存しない**。将来的にジェネリクスが利用可能になれば注釈を追加可能
- `std.test` は native コードに一切依存せず、純粋な YaoXiang で実装される

### 4. 標準ライブラリのロード機構（重要な設計）

**Phase 1：バイナリへの埋め込み**

`std/test.yx`（および将来 YaoXiang で記述されるすべての標準ライブラリモジュール）はビルド時にバイナリに埋め込まれる：

```rust
// build.rs またはビルドスクリプトにより自動生成
pub const STD_YX_FILES: &[(&str, &str)] = &[
    ("std/test.yx", r#"..."#),  // ソースコードテキスト
    // 将来的にさらに追加
];
```

モジュールシステム（RFC-029、2026-08-02 に完全に実装済み）が接続点を提供する：Registry は native モジュールとソースモジュールの両方を保持し、orchestrator が複数ファイルの編成を担当する。
`use std.test` の解決順序：

1. まず Rust native モジュール（既存の仕組み、例えば `std.assert`）を検索
2. 見つからなければ、埋め込まれた `STD_YX_FILES` を検索——ヒットした場合は**仮想パス**
   （例：`<std>/test.yx`）をシードモジュールとして orchestrator に注入し、通常のフロントエンドパイプライン（parse
   → typecheck → IR）を経由
3. 見つからなければ、ファイルシステム検出（ユーザーモジュール）に進む

埋め込まれたソースモジュール内部の `use std.assert` は resolver によって正常に native
registry に解決される——native とソースモジュールは Registry に共存し、種類を跨いだ依存関係が自然に成立する。埋め込みモジュールは**オンデマンドでコンパイル**：import されたときのみパイプラインに入る。

利点：

- 単一ファイルモードでも `use std.test` が動作する
- 標準ライブラリのバージョンがバイナリと厳密に紐付けられ、バージョンの不一致が発生しない
- ユーザーが標準ライブラリのパスを設定する必要がない

**将来：ファイルシステム標準ライブラリ**

YaoXiang のプロジェクトモードが成熟した後、標準ライブラリはファイルシステム形式に変更される。詳細は RFC-014 の更新版を参照。

### 5. 検出と実行

**前提条件（2026-08-02 レビュー決議）**：CLI `run` を orchestrator に接続する。現状 CLI `run`
は単一ファイルパイプライン（`run_file_with_diagnostics`）を通り、ユーザーモジュールのインポートを解析できない。一方
`yaoxiang test`
のサブプロセスモデルは CLI の機能を継承し、テストファイルがプロジェクトモジュールをインポートすることが中心的なシナリオである。したがって Phase
1 ではまず CLI `Run` のソースブランチを
`run_project`（orchestrator、ディレクトリ再帰検出）に委譲する；#247（`use`
経由のオンデマンド検出）はその後に純粋なパフォーマンス最適化として重ね合わせる。import なしの単一ファイルは orchestrator 経由でも等価な動作であり、バイトコードの分岐は変わらない。

**検出フェーズ**：

1. `[PATHS]` が指定された場合は、指定されたパスを直接使用する
2. そうでなければ `yaoxiang.toml` の `[tool.test].patterns` を読み込む
3. 設定がない場合はデフォルト `tests/**/*.yx`
4. `--filter` でフィルタを適用（ファイル名を含む）

**実行フェーズ**：

1. 各ファイルに対して：`yaoxiang run --debug-info <file>` でサブプロセスを起動（`--debug-info`
   によりランタイムエラーにソース位置が付与される——2026-08-02 実証 stack trace が `file:line:col`
   を出力）
2. exit code を確認：0 なら PASS、0 以外なら FAIL
3. レポート用に stdout/stderr をキャプチャ
4. 逐次実行のみ（Phase 1）、将来的に `--parallel` をサポート
5. `--fail-fast` の場合、最初の FAIL で直ちに停止

### 6. テストの分離

テスト分離はプロセスレベルの境界によって自然に実現される：

- 各テストファイルは独立したサブプロセスで実行される
- 各サブプロセスは独立した Heap、Frame、NativeContext を有する
- あるテストファイルのパニックが他のテストファイルに影響しない
- 独立した Heap コンテキスト機構を追加する必要はない

## 既存システムとの関係

| 項目                                                      | 関係                                                                                                      |
| --------------------------------------------------------- | --------------------------------------------------------------------------------------------------------- |
| Rust `#[test]`                                            | 変更なし、コンパイラ内部テストは引き続き Rust を使用                                                      |
| 既存の `.yx` 統合テスト（`tests/yaoxiang/`）              | `yaoxiang test` によって検出・実行される                                                                  |
| `std.assert.assert(cond)`                                 | 保持、`std.test` が基盤としてこれに依存する                                                               |
| モジュールシステム（RFC-029）                             | 埋め込みソースモジュールは Registry/orchestrator 経由で接続；CLI `run` を orchestrator に接続するのが前提 |
| `#200` リファクタリング（`io.println` → `assert.assert`） | `yaoxiang test` と完全に一致する方向性                                                                    |
| `@` アノテーション                                        | 使用しない、`@test` は導入しない                                                                          |

## 実装戦略

### Phase 1：コア機能

変更範囲：

- `src/util/diagnostic/mod.rs` / `src/main.rs` — CLI `Run` のソースブランチを `run_project`
  に委譲（複数ファイル実行の前提）
- `src/main.rs` — `Test` サブコマンドを追加
- `src/std/test.yx` — 純粋な YaoXiang モジュールを追加
- `build.rs` — `std/*.yx` をバイナリに埋め込む
- orchestrator / Registry — 埋め込まれたソースから仮想パスで `.yx`
  モジュールを読み込む機能をサポート
- RFC-015 設定解析 — `[tool.test]` セクション
- サブプロセス実行（`--debug-info`）+ レポート

成果物：

- `yaoxiang test` が基本的に利用可能
- `std.test` の 4 つのアサーション関数
- デフォルトの `tests/**/*.yx` 検出
- 逐次実行 + デフォルト出力形式

### Phase 2：改善

- `--filter` / `--fail-fast` / `--verbose` オプション
- `--json` 出力（CI 連携）
- `--list` オプション
- `--no-progress` オプション

### Phase 3：上級

- `--parallel` 並列実行（spawn 並行モデルの完成に依存）
- `[tool.test].exclude` 設定
- より多くのアサーション関数（例：Float 用の `assert_approx_eq`）

## リスクと緩和策

| リスク                                                       | 確率 | 緩和                                                                       |
| ------------------------------------------------------------ | ---- | -------------------------------------------------------------------------- |
| `f"..."` が Any 上で補間失敗                                 | なし | 2026-08-02 に実証済み（Int/String ともに正常）                             |
| サブプロセス起動のオーバーヘッドがテスト速度に影響           | 中   | Phase 1 は逐次実行、許容範囲；Phase 3 で並列化により緩和                   |
| `yaoxiang.toml` 設定解析が現在の CLI に存在しない            | 低   | 単純な拡張、コア機能に影響しない                                           |
| CLI run を orchestrator に接続することで挙動のリグレッション | 低   | import なしの単一ファイルパスは等価；orchestrator は統合テストでカバー済み |
| 埋め込み `.yx` ソースファイルでバイナリサイズが増大          | 低   | `.yx` ソースファイルは非常に小さく、無視できる                             |

## 未解決問題

- [x] `std/test.yx` 内の `use std.assert`
      参照が正しく解決されるか？——**解決済み（2026-08-02）**。モジュールシステム（RFC-029）実装後、native とソースモジュールが Registry に共存し、resolver が統一的に解決し、種類を跨いだ依存関係が自然に成立する
- [x] テスト出力の `f"..."` のジェネリック `to_string`
      が新しい型制約を導入するか？——**解決済み（2026-08-02）**。注釈なし引数（Any）上で `==`/`!=`
      と f-string 補間がともに動作することを実証（Int/String で検証済み）、新しい制約を導入しない
- [x] `?` ジェネリックパラメータの実現可能性？——**解決済み（2026-08-02）**：`?`
      型構文は現状存在しない（かつ静かに飲み込まれる、単独 issue で追跡中）、Phase
      1 のアサーション関数は注釈なし引数を使用し、ジェネリクスシステムに依存しない

## 設計決定の記録

| 決定                   | 決定内容                                                         | 日付       | 理由                                                                             |
| ---------------------- | ---------------------------------------------------------------- | ---------- | -------------------------------------------------------------------------------- |
| テストマーカー方式     | `@test` アノテーションを使用しない、テストファイルは通常の `.yx` | 2026-07-26 | コンパイラ変更ゼロ、サブプロセスで分離を実現                                     |
| アサーション方式       | `std.test` モジュールは純粋な YaoXiang 関数                      | 2026-07-26 | セルフホスティング、native コードなし                                            |
| テスト実行モデル       | サブプロセス `yaoxiang run <file>` + exit code                   | 2026-07-26 | プロセスレベル分離、コンパイラ変更ゼロ                                           |
| 標準ライブラリのロード | 現状バイナリに埋め込み、将来ファイルシステムへ                   | 2026-07-26 | バージョン紐付け、単一ファイルで利用可能                                         |
| アサーション引数の型   | 注釈なし引数（Any）、ジェネリクスシステムに依存しない            | 2026-08-02 | `?` 型構文が存在しない；Any は比較・補間可能と実証                               |
| 複数ファイル実行       | CLI `run` を `run_project`（orchestrator）に委譲を前提とする     | 2026-08-02 | サブプロセスモデルが CLI の能力を継承；#247 は純粋なパフォーマンス最適化に格下げ |
| レポートのソース位置   | サブプロセスに `--debug-info` を付与                             | 2026-08-02 | stack trace が `file:line:col` を出力することを実証                              |

## 参考文献

- [RFC-014: パッケージ管理システム設計](../accepted/014-package-manager.md)
  — 標準ライブラリディレクトリ構造
- [RFC-015: 設定システム](../accepted/015-configuration-system.md) — `[tool.test]` 設定セクション
- [RFC-030: assert 断言機構](../review/030-assert-mechanism.md) — 基盤依存
- [Rust `#[test]` 機構](https://doc.rust-lang.org/book/ch11-01-writing-tests.html) — 参考設計
