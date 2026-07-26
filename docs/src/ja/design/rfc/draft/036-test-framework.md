---
title: "RFC-036: std.test テストフレームワークと yaoxiang test コマンド"
status: "ドラフト"
author: "晨煦"
created: "2026-07-26"
updated: "2026-07-26"
issue: "#94, #95, #221"
---

# RFC-036: std.test テストフレームワークと yaoxiang test コマンド

## 要約

YaoXiang に標準テストフレームワーク `std.test` モジュールと `yaoxiang test` CLI サブコマンドを導入する。テストファイルは通常の `.yx` ファイルであり、`std.assert.assert` + exit code で合格/失敗を判定する。`std.test` モジュールは純粋な YaoXiang で実装され、初の dogfooding ライブラリとなる。`yaoxiang test` は CLI ツールであり、言語処理系の機能ではない——parser、IR、バイトコード、実行器には一切手を加えない。

## 動機

### なぜテストフレームワークが必要か？

現在の YaoXiang のテスト網羅は Rust 側の `#[test]` と `tests/` 統合テストに依存している。これは以下を意味する：

1. 標準ライブラリ（std.math / std.list / std.dict / std.convert / std.io）の単体テストが YaoXiang で書けない
2. `#117 標準ライブラリモジュール単体テスト網羅` が、利用可能なテスト基盤がないためブロックされている
3. 言語機能の回帰テスト（RFC-032 spawn セマンティクスの変更など）に自動化された手段がない

### 重要な制約

- **17 キーワードの鉄律**：新しいキーワードや構文構造を導入しない
- **言語処理系への変更ゼロ**：parser、IR、バイトコード、実行器には触れない
- **セルフホスティング優先**：テストライブラリは YaoXiang で記述、初の dogfooding ライブラリ

## アーキテクチャ

```
┌──────────────────────────────────────────────────────────────┐
│                    yaoxiang test                              │
│                                                              │
│  CLI 層:  yaoxiang test [--filter --fail-fast --json ...]    │
│              │                                               │
│  検出層:    yaoxiang.toml 読み込み → [tool.test] patterns    │
│              デフォルト: tests/**/*.yx                        │
│              │                                               │
│  実行層:    各ファイルに対して: yaoxiang run <file>           │
│              exit code 確認 → 逐次実行                        │
│              │                                               │
│  レポート層: PASS/FAIL → 集約                                 │
│              --json / --verbose / --fail-fast サポート        │
│                                                              │
│  アサーション層: std.test (純粋な YaoXiang、セルフホスティング)│
│              基盤: std.assert.assert                          │
│              診断: f"Expected {expected}, got {actual}"       │
└──────────────────────────────────────────────────────────────┘
```

### 基本原則

1. **テストフレームワークは言語処理系の機能ではなく、CLI ツールである** — `yaoxiang run` はすでに「テストを実行」でき、`yaoxiang test` は単にすべてのファイルを実行してレポートを表示するだけ
2. **言語処理系への変更ゼロ** — `@test` アノテーションスキャン、バイトコードメタデータセグメント、実行器の特殊エントリポイントは導入しない
3. **セルフホスティング** — `std.test` モジュールは純粋な YaoXiang で実装され、基盤として `std.assert.assert` を呼び出す
4. **テストファイルは通常の `.yx` ファイル** — exit code で合格/失敗を判定

## 詳細設計

### 1. CLI 設計

```
yaoxiang test [OPTIONS] [PATHS]

Arguments:
  [PATHS]...      テストファイルまたはディレクトリを指定（デフォルト: yaoxiang.toml から読み込み、未設定なら tests/）

Options:
  --filter <NAME>     ファイル名に <NAME> を含むテストのみ実行
  --fail-fast         最初の失敗で停止
  --verbose, -v       各テストの詳細な stdout/stderr を表示
  --list              テストファイルを一覧表示するだけで実行しない
  --no-progress       プログレスバーを表示しない（CI 向け）
  --json              JSON 形式の結果を出力（CI 統合用）
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
      "file": "tests/string_test.yx", "passed": false, "time_secs": 0.003,
      "error": "Expected \"hello\", got \"world\"",
      "exit_code": 1
    }
  ]
}
```

### 2. yaoxiang.toml 設定

RFC-015 の `[tool.*]` サードパーティ拡張規約に従い、`[tool.test]` の下に配置する：

```toml
[project]
name = "my-project"

[tool.test]
patterns = ["tests/**/*.yx"]
# 将来的な拡張:
# exclude = ["tests/fixtures/**"]
# parallel = true
```

- デフォルト `patterns = ["tests/**/*.yx"]` — ユーザーは設定不要で即座に使える
- 単一ファイルモード（`yaoxiang test foo.yx`）は設定を無視して直接実行
- 将来的に別リポジトリへ分割する可能性あり（`[tool.test]` の位置は変えない）

### 3. std.test モジュール（純粋な YaoXiang）

```yaoxiang
// std/test.yx — Pure YaoXiang test assertion library
// First dogfooding library: YaoXiang's test library written in YaoXiang

use std.assert

assert_eq: (a: ?, b: ?) -> Void = (a, b) => {
    assert.assert(a == b, f"Expected {b}, got {a}")
}

assert_ne: (a: ?, b: ?) -> Void = (a, b) => {
    assert.assert(a != b, f"Expected not equal to {b}, got {a}")
}

assert_true: (cond: Bool) -> Void = (cond) => {
    assert.assert(cond, f"Expected true, got {cond}")
}

assert_false: (cond: Bool) -> Void = (cond) => {
    assert.assert(!cond, f"Expected false, got {cond}")
}
```

- 4 つのアサーション関数、すべて `f"..."` で診断情報を生成
- `assert_eq` / `assert_ne` の `?` ジェネリクスパラメータは型システムに依存
- `std.test` はネイティブコードに依存せず、純粋な YaoXiang で実装

### 4. 標準ライブラリのロード機構（重要な設計）

**Phase 1：バイナリへの埋め込み**

`std/test.yx`（および将来的に YaoXiang で書かれるすべての標準ライブラリモジュール）はビルド時にバイナリへ埋め込まれる：

```rust
// build.rs またはビルドスクリプト、自動的に生成
pub const STD_YX_FILES: &[(&str, &str)] = &[
    ("std/test.yx", r#"..."#),  // ソースコードテキスト
    // 将来的にさらに追加
];
```

モジュールローダーが `use std.test` を解決する際：
1. まず Rust ネイティブモジュール（既存メカニズム、例：`std.assert`）を検索
2. ヒットしなければ、埋め込まれた `STD_YX_FILES` を検索し、`std/test.yx` のソースコードを見つける
3. そのソースコードをコンパイルしてモジュールシステムに登録

利点：
- 単一ファイルモードでも `use std.test` が動作する
- 標準ライブラリのバージョンがバイナリに厳密に紐付き、版数の不一致が発生しない
- ユーザーが標準ライブラリパスを設定する必要がない

**将来：ファイルシステム標準ライブラリ**

YaoXiang のプロジェクトモードが成熟した後、標準ライブラリはファイルシステム形式に移行する。詳細は RFC-014 の更新を参照。

### 5. 検出と実行

**検出フェーズ**：
1. `[PATHS]` が指定されていれば、そのパスを直接使用
2. 指定されていなければ `yaoxiang.toml` の `[tool.test].patterns` を読み込み
3. 設定がなければ、デフォルトで `tests/**/*.yx`
4. `--filter` でフィルタを適用（ファイル名に含まれるか）

**実行フェーズ**：
1. 各ファイルに対して：サブプロセスとして `yaoxiang run <file>` を起動
2. exit code を確認：0 が PASS、0 以外が FAIL
3. レポート用に stdout/stderr をキャプチャ
4. 逐次実行のみ（Phase 1）、将来的に `--parallel` をサポート
5. `--fail-fast` 指定時、最初の FAIL で即座に停止

### 6. テスト分離

テスト分離はプロセスレベルの境界によって自然に実現される：
- 各テストファイルは独立したサブプロセスで実行される
- 各サブプロセスは独立した Heap、Frame、NativeContext を有する
- 一つのテストファイルで panic が発生しても他のテストファイルに影響しない
- 独立した Heap コンテキスト機構などの追加機構は不要

## 既存システムとの関係

| 項目 | 関係 |
|------|------|
| Rust `#[test]` | 変更しない、コンパイラ内部のテストは引き続き Rust を使用 |
| 既存の `.yx` 統合テスト（`tests/yaoxiang/`） | `yaoxiang test` によって検出・実行される |
| `std.assert.assert(cond)` | 維持、`std.test` の基盤として依存 |
| `#200` リファクタリング（`io.println` → `assert.assert`） | `yaoxiang test` と完全に同じ方向性 |
| `@` アノテーション | 使用しない、`@test` は導入しない |

## 実装戦略

### Phase 1：コア機能

変更範囲：
- `src/main.rs` — 新規 `Test` サブコマンド追加
- `src/std/test.yx` — 純粋な YaoXiang モジュールを新規追加
- `build.rs` — `std/*.yx` をバイナリに埋め込み
- モジュールローダー — 埋め込みソースからの `.yx` モジュールロードをサポート
- RFC-015 設定解析 — `[tool.test]` セクション
- サブプロセス実行 + レポート

成果物：
- `yaoxiang test` が基本的に利用可能
- `std.test` の 4 つのアサーション関数
- デフォルトの `tests/**/*.yx` 検出
- 逐次実行 + デフォルト出力形式

### Phase 2：機能拡充

- `--filter` / `--fail-fast` / `--verbose` オプション
- `--json` 出力（CI 統合）
- `--list` オプション
- `--no-progress` オプション

### Phase 3：高度な機能

- `--parallel` 並列実行（spawn 並行モデルの完成に依存）
- `[tool.test].exclude` 設定
- より多くのアサーション関数（Float 用 `assert_approx_eq` など）

## リスクと緩和策

| リスク | 確率 | 緩和策 |
|------|------|------|
| `f"..."` がジェネリック型への補間で失敗する | 低 | `std.assert.assert` で基本型が動作することはすでに検証済み |
| サブプロセス起動のオーバーヘッドがテスト速度に影響する | 中 | Phase 1 は逐次実行で許容、Phase 3 で並列化により緩和 |
| `yaoxiang.toml` 設定解析が現在の CLI にない | 低 | 単純な拡張であり、コア機能に影響しない |
| `std.test` でジェネリック `?` が利用できない | 低 | `Any` 型への縮退、または型特殊化が可能 |
| `.yx` ソースファイルのバイナリへの埋め込みがサイズを増やす | 低 | `.yx` ソースファイルは非常に小さく、無視できる |

## 未解決の問題

- [ ] `std/test.yx` 内の `use std.assert` の参照がモジュールローダーで正しく解決されるか？埋め込みソースモジュール間の依存関係を検証する必要がある
- [ ] テスト出力の `f"..."` のジェネリック `to_string` が新しい型制約を導入しないか？要検証

## 設計決定の記録

| 決定 | 結論 | 日付 | 理由 |
|------|------|------|------|
| テストのマーカー方式 | `@test` アノテーションを使用せず、テストファイルは通常の `.yx` | 2026-07-26 | 言語処理系への変更ゼロ、サブプロセスがそのまま分離になる |
| アサーション方式 | `std.test` モジュールは純粋な YaoXiang 関数 | 2026-07-26 | セルフホスティング、ネイティブコード不要 |
| テスト実行モデル | サブプロセス `yaoxiang run <file>` + exit code | 2026-07-26 | プロセスレベル分離、言語処理系への変更ゼロ |
| 標準ライブラリのロード | 現在はバイナリに埋め込み、将来的にファイルシステム | 2026-07-26 | バージョン固定、単一ファイルで動作 |
| ジェネリックアサーション | `?` ジェネリクスパラメータに依存 | 2026-07-26 | 特殊化を導入せず、型システムを信頼 |

## 参考文献

- [RFC-014: パッケージ管理システム設計](../accepted/014-package-manager.md) — 標準ライブラリのディレクトリ構造
- [RFC-015: 設定システム](../accepted/015-configuration-system.md) — `[tool.test]` 設定セクション
- [RFC-030: assert アサーション機構](../review/030-assert-mechanism.md) — 基盤依存
- [Rust `#[test]` メカニズム](https://doc.rust-lang.org/book/ch11-01-writing-tests.html) — 参考設計