---
title: 'RFC-036: std.test テストフレームワークと yaoxiang test コマンド'
status: '承認済み'
author: '晨煦'
created: '2026-07-26'
updated: '2026-09-02'
accepted: '2026-08-02'
issue: '#94, #95, #221, #319'
---

# RFC-036: std.test テストフレームワークと yaoxiang test コマンド

## 概要

YaoXiang に標準テストフレームワーク `std.test` モジュールと `yaoxiang test`
CLI サブコマンドを導入する。テストファイルは通常の `.yx` ファイルであり、サブプロセスの exit
code で全体のパス/失敗を判定する。ファイル内部は複数のテスト関数をサポートする——アサーション失敗は
`Err` 値で表現され（値意味論）、スイートが per-test 判定を収集する（§7）。`std.test`
モジュールは純粋な YaoXiang で実装され、最初の dogfooding ライブラリである。`yaoxiang test`
は CLI ツールであり、コンパイラ機能ではない——parser、IR、バイトコード、エグゼキュータへの変更は一切伴わない。

## 動機

### なぜテストフレームワークが必要か？

現在の YaoXiang のテストカバレッジは Rust 側の `#[test]` と `tests/`
統合テストに依存している。これは次のような問題がある：

1. 標準ライブラリ（std.math / std.list / std.dict / std.convert /
   std.io）のユニットテストを YaoXiang で記述できない
2. `#117 標準ライブラリ各モジュールのユニットテストカバレッジ`
   がブロックされている。利用可能なテストインフラがないため
3. 言語特性のリグレッションテスト（RFC-032 spawn 意味論変更など）の自動化手段がない

### 重要な制約

- **17 キーワード鉄則**：新しいキーワードや構文構造を導入しない
- **コンパイラ変更ゼロ**：parser、IR、バイトコード、エグゼキュータを触らない
- **セルフブート優先**：テストライブラリは YaoXiang で記述され、最初の dogfooding ライブラリ

## アーキテクチャ

```
┌──────────────────────────────────────────────────────────────┐
│                    yaoxiang test                              │
│                                                              │
│  CLI 層:  yaoxiang test [--filter --fail-fast --json ...]    │
│              │                                               │
│  発見層:    yaoxiang.toml → [tool.test] patterns を読み込み   │
│              デフォルト: tests/**/*.yx                        │
│              │                                               │
│  実行層:    各ファイル: yaoxiang run <file>                   │
│              exit code を検査 → シリアル実行                  │
│              │                                               │
│  報告層:    PASS/FAIL → 集計                                  │
│              --json / --verbose / --fail-fast をサポート      │
│              │                                               │
│  アサート層: std.test (純粋な YaoXiang, セルフブート)         │
│              基盤: std.assert.assert                          │
│              診断: f"Expected {expected}, got {actual}"       │
└──────────────────────────────────────────────────────────────┘
```

### 中核原則

1. **テストフレームワークはコンパイラ機能ではなく、CLI ツールである** — `yaoxiang run`
   はすでに「テストを実行」でき、`yaoxiang test` は単にすべてのファイルを実行してレポートを表示する
2. **コンパイラ変更ゼロ** — `@test`
   アノテーションスキャン、バイトコードメタデータセグメント、エグゼキュータ特殊入口を導入しない
3. **セルフブート** — `std.test` モジュールは純粋な YaoXiang で実装され、基盤機能は `std.assert` /
   `std.result` から提供される
4. **テストファイルは通常の `.yx` ファイル** — ファイルはサブプロセスで実行され、exit
   code で全体のパス/失敗を判定する
5. **アサーション失敗は値であり、プロセスイベントではない** — テスト関数は `Result`
   を返し、アサーション失敗は `Err`
   で表現され、スイートが per-test 判定を収集する（§7）；プロセスレベルの abort は実行時ガードのみに属し、テストアサーションには使用しない

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

**デフォルト出力**（per-test 判定来自文件内套件收集，见 §7）：

```
Running 3 test files...

tests/math_test.yx ........................ PASS (0.002s)
tests/list_test.yx ........................ FAIL (0.003s)
  `-- [FAIL] push_grows_len: Expected 3, got 2
  `-- [ ok ] pop_returns_last
Results: 2 files passed, 1 file failed, 0 skipped (0.006s)
```

**JSON 出力**（`--json`）：

```json
{
  "summary": { "total": 3, "passed": 2, "failed": 1, "skipped": 0, "time_secs": 0.006 },
  "files": [
    { "file": "tests/math_test.yx", "passed": true, "time_secs": 0.002 },
    {
      "file": "tests/list_test.yx",
      "passed": false,
      "time_secs": 0.003,
      "exit_code": 1,
      "tests": [
        { "name": "push_grows_len", "passed": false, "error": "Expected 3, got 2" },
        { "name": "pop_returns_last", "passed": true }
      ]
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
# 未来可扩展:
# exclude = ["tests/fixtures/**"]
# parallel = true
```

- デフォルト `patterns = ["tests/**/*.yx"]` — ユーザーはゼロコンフィグで即座に使える
- 単一ファイルモード（`yaoxiang test foo.yx`）は設定を一切読まず直接実行
- 将来的に独立したリポジトリに分割する可能性あり（`[tool.test]` の位置は不変）

### 3. std.test モジュール（純粋な YaoXiang）

```yaoxiang
// std/test.yx — Pure YaoXiang test assertion library
// First dogfooding library: YaoXiang's test library written in YaoXiang

use std.result

assert_eq = (a, b) => {
    if a == b { result.ok(()) } else { result.err(f"Expected {b}, got {a}") }
}

assert_ne = (a, b) => {
    if a != b { result.ok(()) } else { result.err(f"Expected not equal to {b}, got {a}") }
}

assert_true = (cond: Bool) => {
    if cond { result.ok(()) } else { result.err(f"Expected true, got {cond}") }
}

assert_false = (cond: Bool) => {
    if cond == false { result.ok(()) } else { result.err(f"Expected false, got {cond}") }
}
```

- アサーション関数は**値意味論**： `Result((), String)` を返し、失敗は `Err(診断情報)`
  で表現され、プロセスを abort しない——§7 のスイートがこれに基づき per-test 判定を収集する。`std.assert.assert`
  のプロセスレベル abort 意味論は実行時ガードのために保持され、テストアサーション経路には入らない
- 移行説明：Phase 1 で実装される 4 関数は
  `std.assert.assert`（abort 意味論）に基づくセルフブートの過渡的実装であり、値意味論族が本 RFC の標準形態であり、落版後に置換される（#319）
- `assert_eq` / `assert_ne` は**型注釈なし引数**（`Any`）を使用——2026-08-02 実証：`==`/`!=`
  と f-string 補間は Any 上で正常に動作（Int/String ともに検証済み）、**汎用システムに依存しない**。将来汎用が用意されれば型注釈を補完可能
- `assert_false` は `cond == false` で否定を表現（`not` 単項構文は未実装、安定した後に移行可能；
  `!assert` 単項形態も同じ制約に依存、§8.1 参照）
- エラーコードアサーション（`assert(err.code == "E3017")`）は、`Error` 値が機械可読の `code`
  フィールドを持つことに依存する（§8.1）
- `std.test` はいかなる native コードにも依存せず、純粋な YaoXiang で実装される

### 4. 標準ライブラリのロード機構（重要な設計）

**Phase 1：バイナリへの埋め込み**

`std/test.yx`（および将来純粋な YaoXiang で記述されるすべての標準ライブラリモジュール）はビルド時にバイナリに埋め込まれる：

```rust
// build.rs 或构建脚本，自动生成
pub const STD_YX_FILES: &[(&str, &str)] = &[
    ("std/test.yx", r#"..."#),  // 源代码文本
    // 未来更多
];
```

モジュールシステム（RFC-029、2026-08-02 に完全に実装済み）が接続点を提供する：Registry は native モジュールとソースモジュールの両方を保持し、orchestrator が複数ファイルの編成を担当する。`use std.test`
の解決順序：

1. まず Rust の native モジュールを検索（既存の仕組み、例えば `std.assert`）
2. ヒットしなければ、埋め込まれた `STD_YX_FILES`
   を検索——ヒットすれば**仮想パス**（例：`<std>/test.yx`）をシードモジュールとして orchestrator に注入し、通常のフロントエンドパイプラインを辿る（parse
   → typecheck → IR）
3. ヒットしなければ、ファイルシステム発見（ユーザーモジュール）へ

埋め込まれたソースモジュール内部の `use std.assert` は resolver により正常に native
registry へ解決される——native とソースモジュールは Registry 内で共存し、種類を跨いだ依存関係が自然に成立する。埋め込まれたモジュールは**オンデマンドでコンパイル**される：import されたときのみパイプラインに入る。

メリット：

- 単一ファイルモードで `use std.test` も機能する
- 標準ライブラリのバージョンがバイナリと厳密に紐づけられ、バージョンの不整合が発生しない
- ユーザーが標準ライブラリのパスを設定する必要がない

**将来：ファイルシステム標準ライブラリ**

YaoXiang のプロジェクトモードが成熟した後、標準ライブラリはファイルシステム形式に移行する。詳細は RFC-014 の更新を参照。

### 5. 発見と実行

**前提条件（2026-08-02 審査決議）**：CLI `run` を orchestrator に接続する。現状 CLI `run`
は単一ファイルパイプライン（`run_file_with_diagnostics`）を辿り、ユーザーモジュールの import を解決できない。しかし
`yaoxiang test`
のサブプロセスモデルは CLI の能力を継承し、テストファイルがプロジェクトモジュールを import するのは中核シナリオである。よって Phase
1 ではまず CLI `Run` のソースブランチを
`run_project`（orchestrator、ディレクトリ再帰発見）に委譲する；#247（use に沿ったオンデマンド発見）はその後の純粋な性能最適化として重ねる。import がない単一ファイルは orchestrator の振る舞いが等価であり、バイトコードブランチは不変。

**発見フェーズ**：

1. `[PATHS]` が指定されていれば、指定されたパスを直接使用
2. そうでなければ `yaoxiang.toml` の `[tool.test].patterns` を読み込む
3. 設定がなければ、デフォルト `tests/**/*.yx`
4. `--filter` によるフィルタリングを適用（ファイル名に含まれる）

**実行フェーズ**：

1. 各ファイルに対して：`yaoxiang run --debug-info <file>` でサブプロセスを起動（`--debug-info`
   により実行時エラーにソースコード位置が付与される——2026-08-02 実証、stack trace が `file:line:col`
   を出力）
2. exit code をチェック：0 なら PASS、0 以外なら FAIL
3. レポート用に stdout/stderr をキャプチャ
4. シリアル実行のみ（Phase 1）、将来 `--parallel` をサポート
5. `--fail-fast` が指定されていれば、最初の FAIL で即座に停止

### 6. テスト分離

テスト分離はプロセスレベルの境界により自然に実現される：

- 各テストファイルは独立したサブプロセスで実行される
- 各サブプロセスは独立した Heap、Frame、NativeContext を有する
- あるテストファイルのパニックは他のテストファイルに影響しない
- 追加の独立 Heap コンテキスト機構は不要

### 7. スイートと複数テスト（値化モデル）

1 つのテストファイルは複数のテストを含むことができる。ファイル内の組織：

```yaoxiang
// tests/list_test.yx
use std.test
use std.list

push_grows_len = () => {
    xs = []
    list.append(xs, 1)
    test.assert_eq(list.len(xs), 1)
}

pop_returns_last = () => {
    xs = [1, 2]
    test.assert_eq(list.pop(xs), 2)
}

main = {
    test.suite([
        ("push_grows_len", push_grows_len),
        ("pop_returns_last", pop_returns_last),
    ])
}
```

- 各テストは `Result((), String)` を返すゼロ引数関数；アサーション失敗は `Err`
  で表現され（§3 値意味論アサーション族）、プロセスを中断しない——後続のテストは通常通り実行される
- `test.suite`
  は順次呼び出し収集する：あるテストが Ok でない場合、そのテストの名前と診断情報を出力し、Ok なら静かに通過
- ファイル終了コード：スイートがすべて Ok なら 0；いずれか Err があれば 0 以外（§5 実行フェーズの exit
  code 判定は不変）
- runner はファイルのみを見、関数レベルのスキャンは行わない：per-test 判定は完全にスイート内部の収集に由来し、ファイル内部の構造は runner に対して透過的——コンパイラ変更ゼロの原則に影響しない
- 明確に採用しないもの：プロセス内 catch 境界（17 キーワード鉄律）；runner からの関数ごとのエントリ呼び出し（§8.2 コンパイル失敗など内部シナリオのみ）
- `test.suite` の具体的な API 形式（シグネチャ、重複名の処理、`--filter`
  とスイート名の相互作用）は実装詳細であり、落版時に #319 で決定する

### 8. 負方向テスト（予期された失敗）三層設計

負方向テストは失敗が発生する層ごとに分割し、各層に帰属させる：

#### 8.1 値レベル反転（汎用、ユーザー向け）

被テスト操作が `Result` を返す場合、テストは通常のアサーションで予期された失敗を表現する：

```yaoxiang
r = range.iter(invalid_range)
test.assert_err(r)
test.assert_eq(result.err_code(r), "E3017")
```

- std.test に `assert_not` / `assert_err` 関数族を追加；`!assert`
  単項形態は not 構文の実装後に提供する（`assert_false` の `cond == false` と同じ制約）
- エラーコードアサーションは、`Error` 値が機械可読の `code`
  フィールドを持つ拡張に依存する：`Struct { message }` から `{ code, message }` へ増加（native
  `error_new_with_code`、std はコード定数をエクスポート）し、 `err.code == "E3017"`
  がアサート可能になる
- Result 化の推進（#301、#316）に伴い、失敗する可能性のある操作は順次 `Result`
  を返し、コーパス内のファイルレベルの負方向マーカーはファイル内アサーションに移行する

#### 8.2 コンパイル失敗（言語設計者内部使用のみ）

コンパイルは全ファイル全か無かであり、ファイル内で「この行はコンパイルされるべきでない」を表現できない。ファイルレベルの特殊マーカーを保持する：

- `[test:error]` マーカーは runner により読み取られ、逆方向に判定される（run 終了コードが 0 以外 =
  PASS）
- マーカーは**構造化された予期コード**にアップグレード：runner はヘッダの「`予期: コンパイルエラー EXXXX`」を解析し、コンパイラの stderr 出力の
  `[EXXXX]` と実コードを比較し、コード不一致 = FAIL（trybuild
  stderr スナップショット思想の軽量版、コンパイラ変更ゼロ）
- **本リポジトリのコーパスにのみ使用され、ユーザーテストフレームワークの一部ではない**；実装側は二系統の runner 判定規約（ディレクトリ規約 vs ヘッダマーカー）の統一が必要、#319 参照

#### 8.3 実行時ハード失敗（Result 化に帰属）

独立した機構を設けない——失敗する可能性のある操作は言語の方向性に従って `Result`
を返し（#301、#316）、テストは一律 §8.1 で表現する。プロセスレベルの abort（アサーション違反、実行時引数誤りなど）は Result 化に伴い徐々に値に収束し、テストフレームワークはそれ専用の意味論を提供しない。

## 既存システムとの関係

| 項目                                                | 関係                                                                                                |
| --------------------------------------------------- | --------------------------------------------------------------------------------------------------- |
| Rust `#[test]`                                      | 不変、コンパイラ内部テストは引き続き Rust を使用                                                    |
| 既存の `.yx` 統合テスト（`tests/yaoxiang/`）        | `yaoxiang test` に発見・実行される                                                                  |
| `std.assert.assert(cond)`                           | 実行時ガードのために保持；`std.test` 値意味論アサーション族は `std.result` に基づく（§3、§7）       |
| モジュールシステム（RFC-029）                       | 埋め込みソースモジュールは Registry/orchestrator を介して接続；CLI `run` の orchestrator 接続が前提 |
| `#200` リファクタ（`io.println` → `assert.assert`） | `yaoxiang test` と完全に同じ方向性                                                                  |
| `@` アノテーション                                  | 使用しない、`@test` を導入しない                                                                    |

## 実装戦略

### Phase 1：コア機能

変更範囲：

- `src/util/diagnostic/mod.rs` / `src/main.rs` — CLI `Run` ソースブランチが `run_project`
  を委譲（複数ファイル実行の前提）
- `src/main.rs` — 新規 `Test` サブコマンド
- `src/std/test.yx` — 新規純粋 YaoXiang モジュール
- `build.rs` — `std/*.yx` をバイナリに埋め込み
- orchestrator / Registry — 埋め込みソースからの仮想パスによる `.yx` モジュールロードをサポート
- RFC-015 設定解析 — `[tool.test]` セクション
- サブプロセス実行（`--debug-info`）+ レポート

成果物：

- `yaoxiang test` が基本的に使用可能
- `std.test` 4 つのアサーション関数
- デフォルト `tests/**/*.yx` 発見
- シリアル実行 + デフォルト出力フォーマット

### Phase 2：改善

- `--filter` / `--fail-fast` / `--verbose` パラメータ
- `--json` 出力（CI 統合）
- `--list` オプション
- `--no-progress` オプション

### Phase 3：上級

- `--parallel` 並列実行（spawn 並行モデルの完成に依存）
- `[tool.test].exclude` 設定
- さらなるアサーション関数（Float 用 `assert_approx_eq` など）

## リスクと緩和策

| リスク                                                       | 確率 | 緩和策                                                                                                                                                                                              |
| ------------------------------------------------------------ | ---- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `f"..."` が Any 上で補間に失敗                               | なし | 2026-08-02 実証済み（Int/String ともに正常）                                                                                                                                                        |
| `yaoxiang.toml` 設定解析が現在の CLI に存在しない            | 低   | 単純な拡張、コア機能に影響しない                                                                                                                                                                    |
| CLI run の orchestrator 接続が振る舞いのリグレッションを導入 | 低   | import なし単一ファイルパスは等価；統合テストは orchestrator をカバー済み                                                                                                                           |
| `.yx` ソースファイルをバイナリに埋め込むと体積が増加         | 低   | `.yx` ソースファイルは非常に小さく、無視可能                                                                                                                                                        |
| テスト実行時間がコーパスの成長に伴い増大                     | 高   | 主な要因はファイルごとの全量コンパイル（185 ファイル実測 11.3s）、サブプロセス起動ではない；`--parallel` はプロセス側のみ緩和、コンパイルコストはテストループキャッシュ（#251/#293 スライス）が必要 |

## オープン問題

- [x] `std/test.yx` 中の `use std.assert`
      の参照は正しく解決されるか？——**解決済み（2026-08-02）**。モジュールシステム（RFC-029）実装後、native とソースモジュールは Registry 内で共存し、resolver が統一的に解決し、種類を跨いだ依存関係が自然に成立する
- [x] テスト出力中の `f"..."` の汎用 `to_string`
      は新しい型制約を導入するか？——**解決済み（2026-08-02）**。実証、型注釈なし引数（Any）上で
      `==`/`!=` と f-string 補間がともに動作（Int/String 検証済み）、新しい制約は導入しない
- [x] `?` 汎用パラメータの実現可能性？——**解決済み（2026-08-02）**：`?`
      型構文は現在存在せず（かつ静かに飲み込まれるため、別途 issue で追跡）、Phase
      1 アサーション関数は型注釈なし引数を使用し、汎用システムに依存しない

## 設計決定記録

| 決定                     | 決定内容                                                                                        | 日付       | 理由                                                                                                                                     |
| ------------------------ | ----------------------------------------------------------------------------------------------- | ---------- | ---------------------------------------------------------------------------------------------------------------------------------------- |
| テストマーカー方式       | `@test` アノテーションを使用せず、テストファイルは通常の `.yx`                                  | 2026-07-26 | コンパイラ変更ゼロ、サブプロセスがそのまま分離                                                                                           |
| アサーション方式         | `std.test` モジュール純粋 YaoXiang 関数                                                         | 2026-07-26 | セルフブート、native コードなし                                                                                                          |
| テスト実行モデル         | サブプロセス `yaoxiang run <file>` + exit code                                                  | 2026-07-26 | プロセスレベル分離、コンパイラ変更ゼロ                                                                                                   |
| 標準ライブラリロード     | 現時点ではバイナリ埋め込み、将来ファイルシステム                                                | 2026-07-26 | バージョン紐付け、単一ファイルで利用可能                                                                                                 |
| アサーション引数型       | 型注釈なし引数（Any）、汎用システムに依存しない                                                 | 2026-08-02 | `?` 型構文は存在しない；Any は実証により比較・補間可能                                                                                   |
| 複数ファイル実行         | CLI `run` は `run_project`（orchestrator）を委譲として前提                                      | 2026-08-02 | サブプロセスモデルが CLI 能力を継承；#247 は純粋な性能最適化に降格                                                                       |
| レポートソースコード位置 | サブプロセスに `--debug-info` を付与                                                            | 2026-08-02 | 実証、stack trace が `file:line:col` を出力；埋め込みモジュール（std.test）を経由したフレーム帰属は本保証対象外、#289 + RFC-034 に属する |
| 負方向テスト階層         | 値レベル反転汎用 / コンパイル失敗 runner 構造化マーカー（内部のみ）/ ハード失敗 Result 化に帰属 | 2026-09-02 | #319 で決定；暗黙の [test:error] 規約を置換                                                                                              |
| ファイル内複数テスト     | 値化標準モデル：テスト関数は Result を返し、スイートが per-test 判定を収集                      | 2026-09-02 | catch なし、エントリ呼び出しなし（エントリは内部シナリオのみ）                                                                           |
| Error コード             | Error に機械可読 `code` フィールドを追加                                                        | 2026-09-02 | エラーコードアサーションをサポート；コンパイル時コードは runner 比較を経る                                                               |

## 参考文献

- [RFC-014: パッケージ管理システム設計](../accepted/014-package-manager.md)
  — 標準ライブラリディレクトリ構造
- [RFC-015: 設定システム](../accepted/015-configuration-system.md) — `[tool.test]` 設定セクション
- [RFC-030: assert アサーション機構](../review/030-assert-mechanism.md) — 基盤依存
- [Rust `#[test]` 機構](https://doc.rust-lang.org/book/ch11-01-writing-tests.html) — 参考設計
