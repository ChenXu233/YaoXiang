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

YaoXiang に標準テストフレームワークである `std.test` モジュールと `yaoxiang test`
CLI サブコマンドを導入する。テストファイルは通常の `.yx` ファイルであり、サブプロセスの exit
code で全体の合格 / 失敗を判定する。ファイル内部は複数のテスト関数をサポートし、断言の失敗は `Err`
値で表現され（値意味論）、スイートが per-test 判定を収集する（§7）。`std.test`
モジュールは純粋な YaoXiang で実装され、最初の dogfooding ライブラリである。`yaoxiang test`
は CLI ツールであり、コンパイラ機能ではない —
parser、IR、バイトコード、エグゼキュータのいずれにも変更を加えない。

## 動機

### なぜテストフレームワークが必要か？

現在の YaoXiang のテストカバレッジは Rust 側の `#[test]` と `tests/`
統合テストに依存している。これは以下を意味する：

1. 標準ライブラリ（std.math / std.list / std.dict / std.convert /
   std.io）の単体テストは YaoXiang では記述できない
2. `#117 標準ライブラリ各モジュールの単体テストカバレッジ`
   は、利用可能なテストインフラがないためブロックされている
3. 言語機能の回帰テスト（例：RFC-032 spawn 意味論変更）に自動化された手段がない

### 重要な制約

- **17 キーワード鉄律**：新しいキーワードや構文構造を導入しない
- **コンパイラへの変更ゼロ**：parser、IR、バイトコード、エグゼキュータに触れない
- **セルフホスティング優先**：テストライブラリは YaoXiang で記述し、最初の dogfooding ライブラリとする

## アーキテクチャ

```
┌──────────────────────────────────────────────────────────────┐
│                    yaoxiang test                              │
│                                                              │
│  CLI 層:  yaoxiang test [--filter --fail-fast --json ...]    │
│              │                                               │
│  発見層:    读取 yaoxiang.toml → [tool.test] patterns         │
│              デフォルト: tests/**/*.yx                        │
│              │                                               │
│  実行層:    各ファイルに対して: yaoxiang run <file>             │
│              exit code を確認 → シリアル実行                  │
│              │                                               │
│  報告層:    PASS/FAIL → 集計                                  │
│              --json / --verbose / --fail-fast をサポート     │
│                                                              │
│  断言層:    std.test（純粋な YaoXiang、セルフホスティング）  │
│              基盤: std.assert.assert                          │
│              診断: f"Expected {expected}, got {actual}"       │
└──────────────────────────────────────────────────────────────┘
```

### コア原則

1. **テストフレームワークはコンパイラ機能ではなく、CLI ツールである** — `yaoxiang run`
   はすでに「テストを実行する」ことができる。`yaoxiang test`
   は単にすべてのファイルを実行してレポートを表示する
2. **コンパイラ変更ゼロ** — `@test`
   アノテーションスキャン、バイトコードメタデータセグメント、エグゼキュータ特殊入口を導入しない
3. **セルフホスティング** — `std.test` モジュールは純粋な YaoXiang で実装され、低層機能は
   `std.assert` / `std.result` から得られる
4. **テストファイルは通常の `.yx` ファイル** — ファイルはサブプロセスとして実行され、exit
   code で全体の合格 / 失敗を判定する
5. **断言の失敗は値であり、プロセスイベントではない** — テスト関数は `Result` を返し、断言の失敗は
   `Err`
   で表現され、スイートが per-test 判定を一つずつ収集する（§7）。プロセスレベルの abort はランタイムガード専用であり、テスト断言には使用しない

## 詳細設計

### 1. CLI 設計

```
yaoxiang test [OPTIONS] [PATHS]

Arguments:
  [PATHS]...      テストファイルまたはディレクトリを指定（デフォルト: yaoxiang.toml から読み取り、未設定なら tests/）

Options:
  --filter <NAME>     ファイル名に <NAME> を含むテストのみ実行
  --fail-fast         最初の失敗で停止
  --verbose, -v       各テストの詳細な stdout/stderr を表示
  --list              テストファイルを列挙するだけで実行しない
  --no-progress       進捗出力を表示しない（ヘッダと PASS 行）。FAIL 明細と集計は保持（CI 用途）
  --json              結果を JSON 形式で出力（CI 連携用）
  --parallel          並列実行（コアごとに 1 worker。[tool.test].parallel とは OR 結合）
```

#### 出力フォーマット

**デフォルト出力**（per-test 判定はファイル内スイート収集から取得、§7 参照）：

```
Running 3 test files...

tests/math_test.yx ........................ PASS (0.002s)
tests/list_test.yx ........................ FAIL (0.003s)
  `-- [FAIL] push_grows_len: Expected 3, got 2
  `-- [ ok ] pop_returns_last
Results: 2 files passed, 1 file failed, 0 skipped (0.006s)
Categories: 2 behavior, 0 compile-error, 0 runtime-error
```

**JSON 出力**（`--json`）：

```json
{
  "summary": {
    "total": 3,
    "passed": 2,
    "failed": 1,
    "skipped": 0,
    "by_kind": { "behavior": 3, "compile-error": 0, "runtime-error": 0, "invalid": 0 },
    "time_secs": 0.006
  },
  "files": [
    { "file": "tests/math_test.yx", "kind": "behavior", "passed": true, "time_secs": 0.002 },
    {
      "file": "tests/list_test.yx",
      "kind": "behavior",
      "passed": false,
      "time_secs": 0.003,
      "exit_code": 1,
      "stderr": "error [E1024]: one is not two",
      "tests": [
        { "name": "push_grows_len", "passed": false, "error": "Expected 3, got 2" },
        { "name": "pop_returns_last", "passed": true }
      ]
    }
  ]
}
```

- 失敗したファイルには追加で `exit_code` と
  `stderr`（ANSI を剥がしたサブプロセスの診断。CI でのフォレンジック用）が付与される。`--verbose` と
  `--json` の併用時はすべてのファイルに `stdout` / `stderr` が付与される
- `--no-progress`
  は進捗出力（ヘッダと PASS 行）のみを抑制する。FAIL 明細と集計は常に出力され、失敗を黙らせない。`--list`
  は各行にテストファイルパスを 1 つずつ出力し、実行はしない
- 各ファイルに `kind`（behavior / compile-error / runtime-error /
  invalid、§8.2）を付与し、summary に `by_kind`
  実行カウント（4 キーで固定。skipped は含まない）を付与する。人間向け集計には `Categories:`
  のカテゴリ分布行が付く
- `--parallel`
  下では人間向け進捗行は**完了順**でストリーミング出力される（ブロック単位でインターリーブしない）。JSON の
  `files` は `file` パスでソートされ出力の安定性を保証する（CI diff フレンドリ）
- ファイル内の per-test `tests`
  配列は §7 のスイート収集から得られ、値化モデル着地後に有効になる（#319）
- 公式 CI（`.github/workflows/ci.yml` の test job）はまさにこの形で消費する：cargo 側で
  `--test integration`（CLI 統合）と `--test yx_runner`（二系統コーパスガード）を実行し、続いて
  `yaoxiang test --json --parallel` を階層別に実走する。デフォルトモード（言語コーパス）と明示的な
  `src/std/tests`（ライブラリ層）がそれぞれ 1 つずつレポートを生成する。集計表（total / passed /
  failed / skipped / time_secs と by_kind）は job summary に書き込まれ、失敗ファイルは `kind` /
  `exit_code` / `stderr` のフォレンジックを出力し、いずれかのスイートが非 0 終了で赤判定となる

### 2. yaoxiang.toml 設定

`[tool.test]` 配下に配置し、RFC-015 の `[tool.*]` サードパーティ拡張規約に従う：

```toml
[project]
name = "my-project"

[tool.test]
patterns = ["tests/**/*.yx"]
exclude = ["tests/fixtures/**"]   # ヒットしたものは発見セットから除外（--list も同様に除外）
parallel = true                   # 並列実行（--parallel flag とは OR 結合）
```

- デフォルト `patterns = ["tests/**/*.yx"]` — ユーザはゼロ設定でそのまま使える
- `exclude` は `patterns` と同じ形式（リテラルパスまたは
  `root/**…`、いずれもパスプレフィックス一致）。excluded はテストではないことを意味する。ランタイム挙動の検証が必要なフィクスチャは
  `yaoxiang run` で直接実行する
- **単一ファイルモード（`yaoxiang test foo.yx`）は設定を読まず直接実行する**。明示的な paths 指定下では
  `exclude` / `parallel` 設定キーはいずれも効かない（flag を除く）
- 将来的に独立リポジトリに分割される可能性がある（`[tool.test]` の位置は不変）

### 3. std.test モジュール（純粋な YaoXiang）

```yaoxiang
// std/test.yx — 純粋な YaoXiang テスト断言ライブラリ（値意味論の標準形態、2026-09-03 着地）
// 最初の dogfooding ライブラリ：YaoXiang のテストライブラリは YaoXiang で書かれている。

use std.result

assert_eq: (a: Any, b: Any) -> Result(Void, String) = (a, b) => {
    if a == b { return result.ok(void) }
    return result.err(f"Expected {b}, got {a}")
}

assert_ne: (a: Any, b: Any) -> Result(Void, String) = (a, b) => {
    if a != b { return result.ok(void) }
    return result.err(f"Expected not equal to {b}, got {a}")
}

assert_true: (cond: Bool) -> Result(Void, String) = (cond) => {
    if cond { return result.ok(void) }
    return result.err(f"Expected true, got {cond}")
}

// assert_not は assert_false と同体。assert_err / assert_err_code は §8.1 参照
```

- 断言関数は**値意味論**：`Result(Void, String)` を返し、失敗は `Err(診断情報)`
  で表現され、プロセスは abort しない。§7 のスイートがこれにより per-test 判定を収集する。`std.assert.assert`
  のプロセスレベル abort 意味論はランタイムガード用に残され、テスト断言経路には入らない。Ok ペイロードは
  `Void`（type-system.md 仕様での unit。`()` は空 Tuple で、両者は混用しない — 2026-09-03 確定）
- **関数ファミリー 7 個（2026-09-03 配備済み、abort 移行版は削除）**：値化された `assert_eq` /
  `assert_ne` / `assert_true` / `assert_false` + `assert_not`（`!assert` の別衣装版として
  `assert_false` と同体）+ `assert_err`（§8.1）
  - `assert_err_code`（§8.1 エラーコード断言）
  - `assert_approx_eq(a: Float, b: Float, eps: Float)`（Phase 3、2026-09-07 配備）：`|a - b| <= eps`
    で判定。eps は呼び出し側が**明示的に与える**
    — トレランスはテスト契約の一部であり、隠れたデフォルト値は設けない。負の eps は宣言時点で Err、NaN は常に Err
- `assert_eq` / `assert_ne` は **Any でのパラメータアノテーション**を使う — `==` / `!=`
  と f-string 補間は Any 上で正常に動作し、generics システムに依存しない。パラメータは**明示的なアノテーションが必須**である点に注意：アノテーションなしのパラメータは native
  generics `&Result(T, E)` の呼び出しチェックを通過できない（R1 プローブ実証）
- `assert_false` / `assert_not` は `cond == false` で否定を表現する（`not`
  単項構文は未着地のため。安定後に移行可能。`!assert` 単項形態もこれに同じ制約で依存、§8.1 参照）
- ブロック本体 + 明示的な `return`
  形式：if 式の then 節の型はチェックで捨てられ、両節が Result の if 式はチェックの盲点になるため、実装でこれを回避する
- `std.test` は native コードに一切依存せず、純粋な YaoXiang で実装される

### 4. 標準ライブラリの読み込み機構（重要な設計）

**Phase 1：バイナリへの埋め込み**

`std/test.yx`（および将来的に YaoXiang で書かれるすべての標準ライブラリモジュール）はビルド時にバイナリへ埋め込まれる：

```rust
// build.rs またはビルドスクリプトで自動生成
pub const STD_YX_FILES: &[(&str, &str)] = &[
    ("std/test.yx", r#"..."#),  // ソースコードテキスト
    // 将来的にさらに追加
];
```

モジュールシステム（RFC-029、2026-08-02 に完全着地済み）が接続点を提供する：Registry は native モジュールとソースモジュールの両方を保持し、orchestrator が複数ファイルの編成を担当する。`use std.test`
の解決順序：

1. まず Rust native モジュールを検索（既存機構、例：`std.assert`）
2. ヒットしなければ、埋め込まれた `STD_YX_FILES`
   を検索する。ヒットすれば**仮想パス**（例：`<std>/test.yx`）を seed モジュールとして orchestrator に注入し、通常のフロントエンドパイプライン（parse
   → typecheck → IR）を経る
3. それでもヒットしなければ、ファイルシステムでの発見（ユーザーモジュール）

埋め込まれたソースモジュール内部の `use std.assert` は resolver によって通常通り native
registry に解決される —
native とソースモジュールは Registry に共存し、種類をまたいだ依存は自然に成立する。埋め込みモジュールは**オンデマンドでコンパイル**される：import されたときのみパイプラインに入る。

利点：

- 単一ファイルモードでも `use std.test` が機能する
- 標準ライブラリのバージョンとバイナリが厳密に紐付けられ、バージョンの不整合が発生しない
- ユーザが標準ライブラリのパスを設定する必要がない

**将来：ファイルシステム標準ライブラリ**

YaoXiang のプロジェクトモードが成熟した後、標準ライブラリはファイルシステム形式に変更される。詳細は RFC-014 の更新版を参照。

### 5. 発見と実行

**前提条件（2026-08-02 レビューの決議）**：CLI `run` を orchestrator に接続する。現状 CLI `run`
は単一ファイルパイプライン（`run_file_with_diagnostics`）を辿るため、ユーザーモジュール import を解決できない。一方
`yaoxiang test`
のサブプロセスモデルは CLI の能力を継承し、テストファイルがプロジェクトモジュールを import するのが中核シナリオである。したがって Phase
1 ではまず CLI `Run` のソースブランチを
`run_project`（orchestrator、ディレクトリ再帰発見）に委譲する。#247（use 経由のオンデマンド発見）はその後の純粋な性能最適化として重ねる。import を持たない単一ファイルは orchestrator の挙動と等価であり、バイトコードブランチは変わらない。

**発見段階**：

1. `[PATHS]` が指定されていれば、指定されたパスを直接使用する
2. そうでなければ `yaoxiang.toml` の `[tool.test].patterns` を読み取る
3. 設定がなければ、デフォルトの `tests/**/*.yx`
4. `--filter` でフィルタを適用する（ファイル名の一致）
5. 発見範囲はそのままテスト階層となる（§9）。デフォルト patterns は言語可用性コーパスのみをカバーする。ライブラリテスト層（例：`src/std/tests/`）は明示的なパスまたはパッケージ設定で発見され、デフォルトのスキャンには混ざらない

**実行段階**：

1. 各ファイルはヘッダ命令に応じて実行を分岐する（命令文法は §8.2、`src/util/test_markers.rs`
   で解析され、yx_runner と共有）：
   - 振る舞いテスト：サブプロセス `yaoxiang run --debug-info <file>`（`--debug-info`
     はランタイムエラーにソース位置を付与する — 2026-08-02 実証により stack trace は `file:line:col`
     を出力する）。`// mode:` でサブプロセスの `--runtime` モードを宣言
   - コンパイル時拒否系：単一ステップで `yaoxiang check <file>`
   - ランタイム失敗系：`check`（必ず通る）+ `run`（必ず失敗する）の 2 ステップ
2. `// skip: <理由>` 付きファイルは実行をスキップし、レポートの skipped に算入
3. 判定と予期コードの照合は §8.2 判定マトリクスに従う。命令の解析が失敗したら実行せずに即 FAIL（構築期拒否）
4. stdout/stderr をレポート用にキャプチャ
5. デフォルトはシリアル。`--parallel`（または
   `[tool.test].parallel`）で利用可能コア数に応じて worker プールを起動し、各ファイルは引き続き独立したサブプロセスとなる。skip/invalid は発見順で先に処理し、実行結果は完了順でストリーミング出力、JSON はパスでソート（Phase
   3、2026-09-07 配備）
6. `--fail-fast`
   指定時は、最初の FAIL で新規ファイルのスケジュールを即停止。並列モードでは進行中ファイルは走り切ってから計上

### 6. テスト隔離

テスト隔離はプロセスレベルの境界により自然に実現される：

- 各テストファイルは独立したサブプロセスで実行される
- 各サブプロセスは独立した Heap、Frame、NativeContext を持つ
- あるテストファイルの panic は他のテストファイルに影響しない
- 追加の独立 Heap コンテキスト機構は不要
- **並列実行（Phase
  3）は隔離境界を拡張しない**：サブプロセス間の作業ディレクトリ（CWD）は共有されており、並列テストは同じパス配下のファイルを占有してはならない。ファイル I/O 系のテストは独立したファイル名を使い、終了時にクリーンアップする

### 7. スイートと複数テスト（値化モデル）

1 つのテストファイルに複数のテストを含められ、ファイル内の構成方式は次のとおり（2026-09-03 着地済み）：

```yaoxiang
// tests/list_test.yx
use std.list
use std.result
use std.test

push_grows_len: () -> Result(Void, String) = () => {
    xs = [1]
    extended = list.push(xs, 2)
    test.assert_eq(list.len(extended), 2)
}

pop_returns_last: () -> Result(Void, String) = () => {
    mut xs = [1, 2]
    last = list.pop(xs)
    test.assert_eq(last, 2)
}

main = {
    test.suite([
        ("push_grows_len", () => push_grows_len()),
        ("pop_returns_last", () => pop_returns_last()),
    ])
}
```

- 各テストは `Result(Void, String)` を返すゼロ引数関数。断言の失敗は `Err`
  で表現され（§3 の値意味論断言ファミリー）、プロセスを中断しない — 後続のテストは通常通り実行される
- `test.suite`
  が順次呼び出して収集する。テストが非 Ok なら名前と診断を記録、Ok なら黙殺。すべて完了後、Err があれば
  `std.assert.assert` で中止し、詳細（`N of M test(s) failed` + 各項目の
  `[FAIL] 名前: 診断`）を添える。ファイルの exit
  code は非 0 となる（§5 判定は不変）。ここでの abort はテストバイナリのランタイムガードであり、断言経路ではない。全 Ok なら黙って 0 で終了
- トップレベルテスト関数は**クロージャ形式で列挙**する（`("name", () => test_fn())`）。トップレベル関数名を値として参照する形は現時点では未対応（IR 層の制約、`E3006`）。クロージャ本体からグローバル関数を呼び出すのは影響を受けない
- runner はファイルだけを見て、関数レベルのスキャンはしない：per-test 判定はすべてスイート内収集から得られ、ファイル内部構造は runner に対して透過的 — 「コンパイラ変更ゼロ」原則は影響を受けない
- 明確に採用しないもの：プロセス内 catch 境界（17 キーワード鉄律）、runner からの関数ごとエントリ呼び出し（§8.2 コンパイル失敗など内部シナリオのみ）
- API 形式は確定済み（2026-09-03、#319）：`suite(tests: List((String, () -> Result(Void, String)))) -> Void`。重複名はチェックしない（名前はレポート表示専用）。`--filter`
  はファイル名でのフィルタであり、スイート内テスト名は感知しない

### 8. ネガティブテスト（予期された失敗）の 3 層設計

ネガティブテストは失敗の発生層によって分割し、各層に振り分ける：

#### 8.1 値レベル反転（汎用、ユーザ向け）

被テスト操作が `Result` を返し、テストは通常の断言で予期される失敗を表現する：

```yaoxiang
r = range.iter(invalid_range)
test.assert_err(r)
e = result.unwrap_err(r)
test.assert_eq(result.code(e), "E6009")
// または簡潔に（コードは std Error キャリアにのみ存在し、E は Error に固定）：
test.assert_err_code(r, "E6009")
```

- `assert_not` / `assert_err` / `assert_err_code`
  は値意味論ファミリーとともに配備済み（2026-09-03、§3）。`!assert`
  単項形態は not 構文の着地を待って提供する（`assert_false` の `cond == false` と同じ制約）
- エラーコード断言は、`Error` 値が機械可読の `code` フィールドを持つことに依存する — これは #323
  M4 で着地済み：`Error = { code, message }`（native `error_new(code, message)`）。読み出しは
  `result.unwrap_err(r)` でキャリアを取得し、`result.code(e)` / `result.message(e)`
  のアクセサでアクセスする（設計段階で想定していた `error_new_with_code`
  という命名、コード定数のエクスポート、`err.code`
  フィールドアクセスはいずれも採用されなかった — 言語に Struct フィールドアクセスがなく、コード定数はエクスポートされていないため）
- Result 化の進展（#301、#316）に伴い、失敗しうる操作が順次 `Result`
  を返すようになれば、コーパス内のファイルレベルネガティブマーカーはファイル内断言へと移行していく

#### 8.2 ファイルレベルネガティブ命令（言語設計者内部専用）

コンパイルはファイル全体まるごと有か無かであり、ファイル内で「この行はコンパイルされるべきでない」を表現できない。ランタイム失敗も同様にファイルレベル表現が必要（スイートに必ず失敗するテストを含む場合など）。ファイル先頭で**構造化命令**により期待を宣言し、runner は期待カテゴリに応じて**判定を分岐**する（2026-09-03 分岐確定、2026-09-06 命令文法確定および着地）。

ヘッダ命令文法（`// key: value`、先頭 16 行以内、厳格なトークン一致）：

```text
// expect: compile-error E1002 [E1003 ...]   コンパイル時拒否テスト
// expect: runtime-error E6003 [...]        ランタイム失敗テスト
// skip: <理由>                              実行をスキップ、skipped に計上
// mode: embedded|standard|full              サブプロセスの --runtime モード（run ステップのみ消費）
```

- `expect:` 命令なし = 振る舞いテスト。`expect:` は予期の**唯一の宣言** —
  2026-09-06 に決定。`[test:error]`
  ブールマーカーと中文「预期:」散文コード抽出は廃止。ブールマーカーと期待行という 2 つの疎結合な事実は規律で一致を保つしかなく、必ずドリフトする。英語の厳格なトークン文法により、runner は機械的に解析する（kind とコードは固定トークン。コード後の余計なトークンは解析失敗）。解析失敗 = 実行せず即 FAIL
  — 命令宣言ミスの静かに劣化する経路は存在しない（構築期拒否）
- **コンパイルエラー系**：単一ステップで `check` — 必ず失敗し、出力にすべての `[EXXXX]`
  を含む必要がある。コンパイル通過 =
  FAIL（報告すべきものが報告されていない）。拒否されたがコードが一致しない =
  FAIL。構文エラー（E1xxx 解析段）と意味エラー（E2xxx+）は独立カテゴリを設けない — 予期コード自体が段階を釘付けにする
- **ランタイムエラー系**：2 ステップ判定 — `check` は必ず**成功**（コンパイル時は無罪）、`run`
  は必ず失敗し出力にすべての `[EXXXX]` を含む。コンパイル時に炸裂 =
  FAIL（コンパイルエラー系と方向が逆の重要判定。「コンパイルが想定外に通過し、ランタイムが偶然失敗」する検出漏れを防ぐ）
- 業界と同形：Rust compiletest の `//~ ERROR`、Go の `// ERROR "regexp"`、GCC の
  `dg-error`、Clang の `expected-error`
  もすべてフィクスチャコメント内で期待を宣言し、harness が双方向で照合する。行レベルのアンカリングを使うのは、複数診断コンパイラが同ファイルの複数期待を区別する必要があるためで、本コンパイラは最初のエラーで停止し 1 ファイル 1 診断なので、ファイルレベルがコンパイラの現状と一致する — エラー回復着地後は文法に行アンカー形態を追加可能（Cranelift
  filetests のファイルヘッダ命令 + 関数レベル期待は同じ混合形態）
- 既知の描画負債：parse 段階の診断は現在 Debug 形式で出力される（`code: "E0012"` 形式であり
  `[E0012]` ではない）。コードスキャナは両形式を受け入れる。診断描画の統一後に厳格形式を取り戻す
- **本リポジトリのコーパス専用であり、ユーザテストフレームワークの一部ではない**。二系統 runner 判定規約は収束済み（2026-09-03、#319）：yx_runner（cargo
  test）と `yaoxiang test` は `src/util/test_markers.rs`
  を共有してヘッダ命令を解析する。06-compile-errors のディレクトリ規約は廃止。レポート層はカテゴリ別カウントを提供する：人間向け集計に
  `Categories:` 行、JSON summary に `by_kind`（behavior / compile-error / runtime-error /
  invalid。skipped は別途計上）、各ファイルに `kind`

#### 8.3 ランタイムハード失敗（Result 化に統合）

独立機構は設けない — 失敗しうる操作は言語の方向性に従い `Result`
を返す（#301、#316）。テストは §8.1 で統一的に表現する。プロセスレベル abort（例：断言違反、ランタイム引数不整合）は Result 化の進展に伴い順次値に集約され、テストフレームワークはそれに専用の意味論を提供しない。（注意：§8.2 の「ランタイムエラー系」マーカー判定は runner による**まだ Result 化できない操作**のファイルレベル検証経路であり、本節の意味論方向と矛盾するものではない — 後者は到達点、前者は移行期の経路）

### 9. テスト体系の階層化：言語コーパスとライブラリテスト（2026-09-03 確定）

テストは**被テスト対象**によって 2 層に分かれ、それぞれ帰属とメンテナが異なる。マーカーシステム（§8.2）と断言ライブラリ（§3）は両層で共通：

**第 1 層：言語可用性コーパス（`tests/yaoxiang/`）**

- 被テスト対象は**言語そのもの**
  — パーサ、型システム、モジュール、並行、所有権、コンパイル時拒否、ランタイム意味論。ディレクトリは言語仕様章に従い構成
- std はコーパス内で**断言ツール**としてのみ使われ（`std.assert` /
  `std.test`）、被テスト対象にはならない — ライブラリの API 振る舞いは言語可用性に属さない
- コーパス内は §8 に従い振る舞いテスト / コンパイル時拒否テスト / ランタイム失敗テストの 3 種に分岐判定

**第 2 層：ライブラリテスト（ライブラリに付随）**

- 被テスト対象は**ライブラリの公開 API 契約**（例：`list.push` の振る舞い、`result.code` の意味論）
- テストは**ライブラリ自身の package 内に記述**：std の package は `src/std/`
  で、yx レベルのテストは `src/std/tests/`
  に置く（実装体と同じ場所。ディレクトリは Rust 単体テストと共存し、ファイル種別は重ならない）。`std.test`
  自身のテストもここに置く（std.test で std.test をテストし、セルフホスティングのループが閉じる）
- 将来のユーザ package も同じ慣例に従う：テストは package 内、package の `[tool.test]`
  で発見される（RFC-014 パッケージ管理のテストレイアウトをここでプレビューする）
- 発見はデフォルト patterns に入らない（デフォルトの `tests/**/*.yx`
  は言語層のみをカバー）：ライブラリテスト層は明示的なパス（`yaoxiang test src/std/tests`）または package 設定で発見される。CI は階層別に実行する

移行メモ：**移行済み（2026-09-06）** — 旧 `tests/yaoxiang/07-std/`
の 19 ファイルは 1 つずつ精査した結果、すべてライブラリテストと判明した（被テスト対象はいずれも std モジュールの API 契約。`?`
伝播、自動借用、generics インスタンス化などの言語機能はその中での役割は carrier であり被テスト対象ではない）。これらは一括して
`src/std/tests/` へ移行し、07-std ディレクトリは廃止。yx_runner は 2 系統発見（`tests/yaoxiang/` +
`src/std/tests/`）に変更、デフォルト patterns はライブラリ層を含まない（統合テストがこの規約を固定）。言語コーパスにはそれ以降 std-API テストが一切含まれない。

## 既存システムとの関係

| 項目                                                      | 関係                                                                                                |
| --------------------------------------------------------- | --------------------------------------------------------------------------------------------------- |
| Rust `#[test]`                                            | 不変。コンパイラ内部テストは引き続き Rust を使用                                                    |
| 既存の `.yx` 統合テスト（`tests/yaoxiang/`）              | `yaoxiang test` で発見・実行される                                                                  |
| `std.assert.assert(cond)`                                 | ランタイムガード用に保持。`std.test` の値意味論断言ファミリーは `std.result` を基盤に変更（§3、§7） |
| モジュールシステム（RFC-029）                             | 埋め込みソースモジュールは Registry/orchestrator 経由で接続。CLI `run` の orchestrator 接続が前提   |
| `#200` リファクタリング（`io.println` → `assert.assert`） | `yaoxiang test` と完全に同じ方向                                                                    |
| `@` アノテーション                                        | 使用しない。`@test` を導入しない                                                                    |

## 実装戦略

### Phase 1：コア機能

変更範囲：

- `src/util/diagnostic/mod.rs` / `src/main.rs` — CLI `Run` のソースブランチを
  `run_project`（複数ファイル実行の前提）に委譲
- `src/main.rs` — `Test` サブコマンドを新設
- `src/std/test.yx` — 純粋な YaoXiang モジュールを新設
- `build.rs` — `std/*.yx` をバイナリに埋め込む
- orchestrator / Registry — 埋め込みソースから仮想パスで `.yx` モジュールを読み込めるようにする
- RFC-015 設定解析 — `[tool.test]` セクション
- サブプロセス実行（`--debug-info`）+ レポート

成果物：

- `yaoxiang test` が基本的に使える
- `std.test` 4 つの断言関数
- デフォルトの `tests/**/*.yx` 発見
- シリアル実行 + デフォルト出力フォーマット

### Phase 2：完成度向上

- `--filter` / `--fail-fast` / `--verbose` パラメータ
- `--json` 出力（CI 連携）
- `--list` オプション
- `--no-progress` オプション

### Phase 3：高度化（2026-09-07 配備済み）

- `--parallel` 並列実行（worker プール + 各ファイル独立サブプロセス。`[tool.test].parallel`
  設定キーも同効）
- `[tool.test].exclude` 設定（プレフィックス一致で除外、`--list` も同様に除外）
- `assert_approx_eq`（Float 明示 eps 断言、§3）

## リスクと緩和

| リスク                                          | 確率 | 緩和                                                                                                                                                                                                  |
| ----------------------------------------------- | ---- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `f"..."` の Any 上での補間失敗                  | なし | 2026-08-02 実証済み（Int / String ともに正常）                                                                                                                                                        |
| `yaoxiang.toml` 設定解析が現行 CLI にない       | 低   | 簡単な拡張であり、コア機能に影響しない                                                                                                                                                                |
| CLI run の orchestrator 接続が挙動退行を起こす  | 低   | import を持たない単一ファイル経路は等価。orchestrator は統合テストでカバー済み                                                                                                                        |
| 埋め込み `.yx` ソースファイルによるバイナリ肥大 | 低   | `.yx` ソースファイルは極小であり、無視できる                                                                                                                                                          |
| コーパス成長に伴うテストループ時間増大          | 高   | 主因は各ファイルのフルコンパイル（185 ファイル実測 11.3s）であり、サブプロセス起動ではない。`--parallel` はプロセス側のみ緩和し、コンパイルコストはテストループキャッシュ（#251/#293 スライス）が必要 |

## オープンな課題

- [x] `std/test.yx` 内の `use std.assert` 参照が正しく解決されるか？—
      **解決済み（2026-08-02）**。モジュールシステム（RFC-029）着地後、native とソースモジュールは Registry に共存し、resolver が統一解決するため、種類をまたいだ依存は自然に成立する
- [x] テスト出力における `f"..."` の generics `to_string` が新しい型制約を導入しないか？—
      **解決済み（2026-08-02）**。実証により、アノテーションなしパラメータ（Any）上の `==` / `!=`
      と f-string 補間はいずれも動作（Int / String で検証済み）。新しい制約を導入しない
- [x] `?` generics パラメータの実現可能性？— **解決済み（2026-08-02）**：`?`
      型構文は現状存在せず（静かに飲み込まれる。別途 issue で追跡中）、Phase
      1 の断言関数はアノテーションなしパラメータを使い、generics システムに依存しない

## 設計決定の記録

| 決定                         | 決定内容                                                                                                                                                                                                                                                                            | 日付                     | 根拠                                                                                                                                                                                                                                                                  |
| ---------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ------------------------ | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| テストマーカー方式           | `@test` アノテーション不使用。テストファイルは通常の `.yx`                                                                                                                                                                                                                          | 2026-07-26               | コンパイラ変更ゼロ、サブプロセスで隔離                                                                                                                                                                                                                                |
| 断言方式                     | `std.test` モジュールは純粋な YaoXiang 関数                                                                                                                                                                                                                                         | 2026-07-26               | セルフホスティング、native コードなし                                                                                                                                                                                                                                 |
| テスト実行モデル             | サブプロセス `yaoxiang run <file>` + exit code                                                                                                                                                                                                                                      | 2026-07-26               | プロセスレベル隔離、コンパイラ変更ゼロ                                                                                                                                                                                                                                |
| 標準ライブラリ読み込み       | 現状はバイナリ埋め込み、将来的にファイルシステム                                                                                                                                                                                                                                    | 2026-07-26               | バージョン固定、単一ファイルでも利用可                                                                                                                                                                                                                                |
| 断言パラメータ型             | アノテーションなしパラメータ（Any）、generics システムに非依存                                                                                                                                                                                                                      | 2026-08-02               | `?` 型構文が存在しない。Any 上で比較・補間可能を実証                                                                                                                                                                                                                  |
| 複数ファイル実行             | CLI `run` を `run_project`（orchestrator）に委譲を前提とする                                                                                                                                                                                                                        | 2026-08-02               | サブプロセスモデルが CLI 能力を継承。#247 は純性能最適化に降格                                                                                                                                                                                                        |
| レポートのソース位置         | サブプロセスに `--debug-info` を付与                                                                                                                                                                                                                                                | 2026-08-02               | 実証で stack trace が `file:line:col` を出力。埋め込みモジュール（std.test）を経由したフレームの帰属はここでは保証対象外（#289 + RFC-034）                                                                                                                            |
| ネガティブテスト階層化       | 値レベル反転汎用 / コンパイル失敗 runner 構造化マーカー（内部のみ）/ ハード失敗は Result 化に統合                                                                                                                                                                                   | 2026-09-02               | #319 確定。暗黙の [test:error] 規約を置き換え                                                                                                                                                                                                                         |
| ファイル内複数テスト         | 値化標準モデル：テスト関数は Result を返し、スイートが per-test 判定を収集                                                                                                                                                                                                          | 2026-09-02               | catch なし、エントリ呼び出しなし（エントリは内部シナリオのみ）                                                                                                                                                                                                        |
| Error コード                 | Error に機械可読 `code` フィールドを追加                                                                                                                                                                                                                                            | 2026-09-02               | エラーコード断言を支える。コンパイル時コードは runner 照合経由                                                                                                                                                                                                        |
| 断言ライブラリ形態           | 値意味論ファミリー 7 関数着地、`Result(Void, String)` 契約。abort 移行版は削除                                                                                                                                                                                                      | 2026-09-03               | Void は仕様 unit（`()` は空 Tuple で混用しない）。ネスト位置は Any 剛性、アノテーションなしパラメータは native generics チェックを通過できない — パラメータは明示アノテーション必須（R1 プローブ実証）                                                                |
| テスト体系階層化             | 言語コーパス（`tests/yaoxiang/`）とライブラリテスト（ライブラリ随伴、std → `src/std/tests/`）の 2 層。std はコーパス内で断言ツール専用                                                                                                                                              | 2026-09-03               | 被テスト対象が帰属とメンテナを決定。ライブラリテスト随伴配置は RFC-014 のプレビュー                                                                                                                                                                                   |
| ネガティブマーカーの分岐判定 | 期待カテゴリで分岐：コンパイルエラー系は `check` が必ず失敗、ランタイムエラー系は `check` が必ず通って `run` が必ず失敗。レポートはカテゴリ別カウントを提供                                                                                                                         | 2026-09-03（09-06 着地） | カテゴリ混在判定では「コンパイルが想定外に通過し、ランタイムが偶然失敗」を検出漏れ。予期コードが段階を釘付けにし、構文エラーは独立カテゴリを設けない                                                                                                                  |
| ヘッダ命令文法               | 期待を英語の構造化命令で宣言（`// expect:` / `// skip:` / `// mode:`、厳格なトークン文法、解析失敗で即 FAIL）。`[test:error]` ブールマーカーと中文「预期:」散文コード抽出は廃止                                                                                                     | 2026-09-06               | 期待はフィクスチャ内容の属性であり、in-fixture 宣言は業界と同形（compiletest / Go / GCC / Clang いずれも同様）。中央リストは必ず腐敗する。ブールマーカー + 期待行の二事実を規律で結びつけるのは欠陥面。構造化文法により runner は機械的に判定し、人間の介在なしにすむ |
| 並列実行モデル               | `--parallel` でコアあたり worker プールを起動（各ファイルは引き続き独立サブプロセス）。skip/invalid は発見順で先に処理、実行結果は完了順でストリーミング出力、JSON はパスでソート。`--fail-fast` はスケジュール停止（進行中は走り切ってから計上）。CWD は共有、隔離境界は拡張しない | 2026-09-07               | サブプロセスモデルでの並列 = OS スレッドでの spawn スケジューリングであり、yx 層の並行は不要。時間消費の主因は各ファイルのフルコンパイル（リスク表参照）であり、並列はプロセス側のみ緩和する。#293 のキャッシュスライスが主緩和                                       |

## 参考文献

- [RFC-014: パッケージ管理システム設計](../accepted/014-package-manager.md)
  — 標準ライブラリディレクトリ構造
- [RFC-015: 設定システム](../accepted/015-configuration-system.md) — `[tool.test]` 設定セクション
- [RFC-030: assert 断言機構](../review/030-assert-mechanism.md) — 低層依存
- [Rust `#[test]` 機構](https://doc.rust-lang.org/book/ch11-01-writing-tests.html) — 参考設計
