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
code で全体の合格/不合格を判定する。ファイル内部は複数のテスト関数をサポートし——アサーション失敗は
`Err` 値で表現され（値意味論）、スイートが per-test 判定を収集する（§7）。`std.test`
モジュールは純 YaoXiang で実装され、dogfooding ライブラリ第一号である。`yaoxiang test`
は CLI ツールであり、コンパイラ機能ではない——parser、IR、バイトコード、実行器のいずれにも変更を加えない。

## 動機

### なぜテストフレームワークが必要か？

現在の YaoXiang のテストカバレッジは Rust 側の `#[test]` と `tests/`
統合テストに依存している。これは以下を意味する：

1. 標準ライブラリ（std.math / std.list / std.dict / std.convert /
   std.io）のユニットテストを YaoXiang で書けない
2. 標準ライブラリの各モジュールのユニットテストカバレッジが、利用可能なテストインフラがないためブロックされている
3. 言語機能の回帰テスト（RFC-032 の spawn 意味論変更など）に自動化された手段がない

### 重要な制約

- **17 キーワード鉄則**：新しいキーワードや構文構造を導入しない
- **コンパイラ変更ゼロ**：parser、IR、バイトコード、実行器に触れない
- **セルフホスティング優先**：テストライブラリは YaoXiang で書き、dogfooding ライブラリ第一号とする

## アーキテクチャ

```
┌──────────────────────────────────────────────────────────────┐
│                    yaoxiang test                              │
│                                                              │
│  CLI 層:  yaoxiang test [--filter --fail-fast --json ...]    │
│              │                                               │
│  発見層:    yaoxiang.toml 読み込み → [tool.test] patterns     │
│              デフォルト: tests/**/*.yx                        │
│              │                                               │
│  実行層:    各ファイルに対して: yaoxiang run <file>            │
│              exit code 検査 → シリアル実行                    │
│              │                                               │
│  報告層:    PASS/FAIL → 集計                                  │
│              --json / --verbose / --fail-fast サポート        │
│                                                              │
│  アサーション層: std.test（純 YaoXiang、セルフホスティング）  │
│              底層: std.assert.assert                          │
│              診断: f"Expected {expected}, got {actual}"       │
└──────────────────────────────────────────────────────────────┘
```

### 核心原則

1. **テストフレームワークはコンパイラ機能ではなく、CLI ツールである** — `yaoxiang run`
   はすでに「テストを実行する」ことができ、`yaoxiang test`
   は単にすべてのファイルを走らせてレポートを表示するだけである
2. **コンパイラ変更ゼロ** — `@test`
   注釈スキャン、バイトコードメタデータセグメント、実行器特殊入口を導入しない
3. **セルフホスティング** — `std.test` モジュールは純 YaoXiang 実装で、底層能力は `std.assert` /
   `std.result` 由来
4. **テストファイルは通常の `.yx` ファイルである** — ファイルはサブプロセスとして実行され、exit
   code で全体の合格/不合格を判定
5. **アサーション失敗は値であり、プロセスイベントではない** — テスト関数は `Result`
   を返し、アサーション失敗は `Err`
   で表現され、スイートが per-test 判定を収集する（§7）；プロセスレベルの abort は実行時ガードにのみ属し、テストアサーションには使用しない

## 詳細設計

### 1. CLI 設計

```
yaoxiang test [OPTIONS] [PATHS]

Arguments:
  [PATHS]...      テストファイルまたはディレクトリを指定（デフォルト: yaoxiang.toml から読み取り、なければ tests/）

Options:
  --filter <NAME>     ファイル名に <NAME> を含むテストのみ実行
  --fail-fast         最初の失敗で停止
  --verbose, -v       各テストの詳細な stdout/stderr を表示
  --list              テストファイルを一覧表示するだけで実行しない
  --no-progress       進捗出力を表示しない（ヘッダと PASS 行）；FAIL 明細と集計は保持（CI シナリオ）
  --json              JSON 形式で結果を出力（CI 統合用）
  --parallel          並列実行（コアごとに 1 ワーカー；[tool.test].parallel と OR を取る）
```

#### 出力形式

**デフォルト出力**（per-test 判定はファイル内スイート収集から来る、§7 参照）：

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

- 失敗ファイルは追加で `exit_code` と
  `stderr`（ANSI 除去後のサブプロセス診断、CI フォレンジック用）を携带；`--verbose` と `--json`
  の組み合わせ時はすべてのファイルに `stdout` / `stderr` を携带
- `--no-progress`
  は進捗出力（ヘッダと PASS 行）のみを抑制する——FAIL 明細と集計は常に出力され、失敗は静かにされない；`--list`
  は 1 行に 1 テストファイルパスを出力し、実行しない
- 各ファイルに `kind`（behavior / compile-error / runtime-error /
  invalid、§8.2）を付与し、summary に `by_kind`
  実行カウント（固定 4 キー、skipped を含まない）を付与；人間向け集計に `Categories:`
  カテゴリ分布行を付与
- `--parallel`
  下で人間向け進捗行は**完了順**でストリーミング出力（ブロック単位で交差しない）、JSON の `files` は
  `file` パスでソートし出力の安定性を保証（CI diff 友好）
- ファイル内 per-test `tests` 配列は §7 スイート収集から来て、値化モデルの実装と共に有効化される
- 公式 CI（`.github/workflows/ci.yml` test job）はこの通りに消費する：cargo 側で
  `--test integration`（CLI 統合）と `--test yx_runner`（双根コーパスガード）を実行し、その後
  `yaoxiang test --json --parallel` を階層別に実走——デフォルトモード（言語コーパス）と明示的
  `src/std/tests`（ライブラリ層）がそれぞれレポートを生成する；集計表（total / passed / failed /
  skipped / time_secs と by_kind）を job summary に書き込み、失敗ファイルには `kind` / `exit_code` /
  `stderr` のフォレンジックを出力し、いずれかのスイートが非ゼロ終了すれば赤と判定

### 2. yaoxiang.toml 設定

`[tool.test]` 配下に配置し、RFC-015 の `[tool.*]` サードパーティ拡張規約に準拠する：

```toml
[project]
name = "my-project"

[tool.test]
patterns = ["tests/**/*.yx"]
exclude = ["tests/fixtures/**"]   # ヒットしたものは発見セットから除外（--list も同様に除外）
parallel = true                   # 並列実行（--parallel flag と OR を取る）
```

- デフォルト `patterns = ["tests/**/*.yx"]` — ユーザーがゼロコンフィグで即座に使える
- `exclude` は `patterns` と同じ形式（リテラルパスまたは
  `root/**…`、いずれもパスプレフィックスでヒット）；excluded はテストではないことを意味し、ランタイム挙動の検証が必要なフィクスチャは
  `yaoxiang run` で直接実行する
- **単一ファイルモード（`yaoxiang test foo.yx`）は直接実行し、設定を読み込まない**——明示的な paths 下では
  `exclude`/`parallel` 設定キーはいずれも無効（flag を除く）
- 将来的に独立リポジトリへ分割される可能性がある（`[tool.test]` の位置は不変）

### 3. std.test モジュール（純 YaoXiang）

```yaoxiang
// std/test.yx — 純 YaoXiang テストアサーションライブラリ（値意味論標準形態、2026-09-03 実装）
// dogfooding ライブラリ第一号：YaoXiang のテストライブラリは YaoXiang で書く。

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

// assert_not と assert_false は同体；assert_err / assert_err_code は §8.1 参照
```

- アサーション関数は**値意味論**：失敗を `Err(診断情報)` で表現する `Result(Void, String)`
  を返し、プロセスを abort しない——§7 スイートがこれに基づき per-test 判定を収集する。`std.assert.assert`
  のプロセスレベル abort 意味論は実行時ガードのために保持し、テストアサーション経路には入らない。Ok ペイロードは
  `Void`（type-system.md 規範 unit；`()` は空 Tuple であり両者は混用しない——2026-09-03 決定）
- **関数族 7 個（2026-09-03 交付済み、abort 過渡版削除）**：値化 `assert_eq` / `assert_ne` /
  `assert_true` / `assert_false` + `assert_not`（`!assert` 別形態として assert_false と同体）+
  `assert_err`（§8.1）
  - `assert_err_code`（§8.1 エラーコードアサーション）
  - `assert_approx_eq(a: Float, b: Float, eps: Float)`（Phase 3、2026-09-07 交付）：`|a - b| <= eps`
    判定、eps は呼び出し側が**明示的に与える**——許容誤差はテスト契約の一部であり、隠れたデフォルト値は設けない；負の eps は宣言箇所で Err、NaN は常に Err
- `assert_eq` / `assert_ne` は **Any 标注引数** を使用——`==`/`!=`
  と f-string 補間は Any 上で正常に動作し、generics システムに依存しない。**引数は明示的に标注しなければならない**ことに注意：标注なしの引数は native 泛型
  `&Result(T, E)` の呼び出しチェックを通過しない（R1 プローブ実証）
- `assert_false` / `assert_not` は `cond == false` で否定を表現（`not`
  単項構文は未実装、stable 後に移行可能；`!assert` 単項形態も同制約に依拠、§8.1 参照）
- ブロック体 + 明示的 `return`
  形態：if 式の then 枝の型はチェックで破棄され、両枝 Result の if 式はチェックの盲点であり、実装でこれを回避する
- `std.test` は native コードに一切依存せず、純 YaoXiang 実装

### 4. 標準ライブラリロード機構（重要設計）

**Phase 1：バイナリ埋め込み**

`std/test.yx`（および将来 YaoXiang で書かれるすべての標準ライブラリモジュール）はビルド時にバイナリに埋め込まれる：

```rust
// build.rs またはビルドスクリプト、自動的に生成
pub const STD_YX_FILES: &[(&str, &str)] = &[
    ("std/test.yx", r#"..."#),  // ソースコードテキスト
    // 将来さらに追加
];
```

モジュールシステム（RFC-029、2026-08-02 完全に実装済み）がアクセスポイントを提供：Registry は native モジュールとソースモジュールの両方を保持し、orchestrator が複数ファイル編成を担当する。`use std.test`
の解決順序：

1. まず Rust native モジュールを検索（既存メカニズム、例：`std.assert`）
2. ヒットしなければ、埋め込まれた `STD_YX_FILES`
   を検索——ヒットすれば**仮想パス**（例：`<std>/test.yx`）をシードモジュールとして orchestrator に注入し、通常のフロントエンドパイプライン（parse
   → typecheck → IR）を経由
3. ヒットしなければ、ファイルシステム発見（ユーザーモジュール）

埋め込まれたソースモジュール内部の `use std.assert` は resolver によって正常に native
registry に解決される——native とソースモジュールは Registry に共存し、種類を跨いだ依存が自然に成立する。埋め込みモジュールは**オンデマンドでコンパイル**：import されたときのみパイプラインに入る。

利点：

- 単一ファイルモードでも `use std.test` が動作
- 標準ライブラリのバージョンとバイナリが厳密に結合され、バージョンのミスマッチが発生しない
- ユーザーが標準ライブラリパスを設定する必要がない

**将来：ファイルシステム標準ライブラリ**

YaoXiang のプロジェクトモードが成熟した後、標準ライブラリはファイルシステム形式に変更される。詳細は RFC-014 の更新を参照。

### 5. 発見と実行

**前提条件（2026-08-02 審査決議）**：CLI `run` を orchestrator に接続する。現状 CLI `run`
は単一ファイルパイプライン（`run_file_with_diagnostics`）を通り、ユーザーモジュールのインポートを解析できない；`yaoxiang test`
のサブプロセスモデルは CLI 能力を継承し、テストファイルがプロジェクトモジュールをインポートするのは中心的シナリオである。したがって Phase
1 ではまず CLI `Run` のソースブランチを
`run_project`（orchestrator、ディレクトリ再帰発見）に委譲する；use に沿ったオンデマンド発見はその後の純粋なパフォーマンス最適化として重ねる。import なしの単一ファイルは orchestrator 経由の挙動が等価で、バイトコードブランチは不変。

**発見段階**：

1. `[PATHS]` が指定されていれば、指定されたパスを直接使用
2. そうでなければ `yaoxiang.toml` の `[tool.test].patterns` を読み取る
3. 設定がなければ、デフォルト `tests/**/*.yx`
4. `--filter` でフィルタリング（ファイル名に含まれる）
5. 発見範囲はテスト階層化（§9）：デフォルト patterns は言語可用性コーパスのみをカバー；ライブラリテスト層（例：`src/std/tests/`）は明示的パスまたはパッケージ設定で発見され、デフォルトスキャンには混ざらない

**実行段階**：

1. 各ファイルはヘッダー指示に従って分岐実行される（指示文法は §8.2 参照、`src/util/test_markers.rs`
   で解析、yx_runner と共有）：
   - 振る舞いテスト：`yaoxiang run <file>`
     サブプロセス（ランタイムエラーはデフォルトでソース位置とスタックフレームを携带——debug_map はデフォルトで生成、stack
     trace は `file:line:col` を出力）；`// mode:` は子プロセスの `--runtime` モードを宣言
   - コンパイル期拒否クラス：単一ステップ `yaoxiang check <file>`
   - ランタイム失敗クラス：`check`（必ず通過）+ `run`（必ず失敗）の 2 ステップ
2. `// skip: <理由>` のファイルは実行をスキップし、レポートの skipped に計上
3. 判定と期待コードの比較は §8.2 判定マトリクスに従う；指示解析が失敗すれば実行せずに直接 FAIL（構築期拒否）
4. レポート用に stdout/stderr をキャプチャ
5. デフォルトはシリアル；`--parallel`（または
   `[tool.test].parallel`）は利用可能コア数でワーカープールを起動し、各ファイルは依然として独立したサブプロセス——skip/invalid は発見順で先行処理、実行結果は完了順でストリーミング出力、JSON はパスでソート（Phase
   3、2026-09-07 交付）
6. `--fail-fast`
   の場合、最初の FAIL ですぐに新規ファイルのスケジュールを停止；並列モードでは進行中ファイルを完走させて計上

### 6. テスト分離

テスト分離はプロセスレベル境界で自然に実現：

- 各テストファイルは独立したサブプロセスで実行
- 各サブプロセスは独立した Heap、Frame、NativeContext を保有
- あるテストファイルのパニックは他のテストファイルに影響しない
- 追加の独立 Heap コンテキストメカニズムは不要
- **並列実行（Phase
  3）は分離境界を拡張しない**：サブプロセス間の作業ディレクトリ（CWD）は共有され、並列テストは CWD 内の同じパスのファイルを占有してはならない——ファイル I/O 系テストは独立したファイル名を使用し終了時にクリーンアップする

### 7. スイートと複数テスト（値化モデル）

テストファイルは複数のテストを含むことができる。ファイル内の組織方式（2026-09-03 実装済み）：

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

main: () -> Void = {
    test.suite([
        ("push_grows_len", () => push_grows_len()),
        ("pop_returns_last", () => pop_returns_last()),
    ])
}
```

- 各テストは `Result(Void, String)` を返すゼロ引数関数；アサーション失敗は `Err`
  で表現され（§3 値意味論アサーション族）、プロセスを中断しない——後続テストは通常通り実行
- `test.suite`
  は順次呼び出して収集：あるテストが非 Ok なら名前と診断を記録、Ok は静か；すべて完走後にいずれかの Err があれば
  `std.assert.assert` で中止し、失敗明細を付与（`N of M test(s) failed` + 各項目
  `[FAIL] 名前: 診断`）——ファイル終了コードは非 0（§5 判定不変）。ここの abort はテストバイナリのランタイムガードであり、アサーション経路ではない；全 Ok なら静かに 0 で終了
- トップレベルテスト関数は**クロージャ形式で登録**（`("name", () => test_fn())`）：トップレベル関数名を値として参照するのは現状サポートされない（IR 層制限、`E3006`）——クロージャ本体からグローバル関数を呼び出すのは影響を受けない
- runner はファイルのみを見て、関数レベルスキャンは行わない：per-test 判定はすべてスイート内収集から来て、ファイル内部構造は runner に対して透過的——コンパイラ変更ゼロ原則は影響を受けない
- 明示的に採用しない：プロセス内 catch 境界（17 キーワード鉄則）；runner の関数単位入口呼び出し（§8.2 コンパイル失敗などの内部シナリオのみ）
- API 形態確定済み（2026-09-03）：`suite(tests: List((String, () -> Result(Void, String)))) -> Void`；重複名はチェックしない（名前はレポート表示用のみ）；`--filter`
  はファイル名でフィルタリングし、スイート内テスト名は認識しない

### 8. ネガティブテスト（期待失敗）3 層設計

ネガティブテストは失敗の発生層で分割し、各層に帰属させる：

#### 8.1 値レベル反転（汎用、ユーザー向け）

被テスト操作が `Result` を返し、テストは通常のアサーションで期待失敗を表現：

```yaoxiang
r = range.iter(invalid_range)
test.assert_err(r)
e = result.unwrap_err(r)
test.assert_eq(result.code(e), "E6009")
// または 1 文にカプセル化（コードは std Error キャリアにのみ存在、E は Error に固定）：
test.assert_err_code(r, "E6009")
```

- `assert_not` / `assert_err` / `assert_err_code`
  は値意味論族と共に交付済み（2026-09-03、§3）；`!assert`
  単項形態は not 構文実装後に提供（`assert_false` の `cond == false` と同制約）
- エラーコードアサーションは `Error` 値が機械可読 `code`
  フィールドを携带することに依存——実装済み：`Error = { code, message }`（native
  `error_new(code, message)`）、読み取りは `result.unwrap_err(r)` でキャリア取得 + `result.code(e)`
  / `result.message(e)` アクセサ経由（設計段階で見積もった `error_new_with_code`
  命名、コード定数エクスポート、`err.code`
  フィールドアクセスはいずれも採用されず——言語に Struct フィールドアクセスがなく、コード定数もエクスポートされていない）
- Result 化の推進に伴い、失敗可能な操作は順次 `Result`
  を返し、コーパス内のファイルレベルネガティブマークはファイル内アサーションに移行する

#### 8.2 ファイルレベルネガティブ指示（言語設計者の内部使用のみ）

コンパイルは全ファイル全部かゼロかのいずれかで、ファイル内で「この行はコンパイルされるべきでない」を表現できない；ランタイム失敗も同様にファイルレベル表現が必要（例：スイートに必ず失敗するテストを含む）。ファイルヘッダーで**構造化指示**として期待を宣言し、runner は期待カテゴリに従って**分岐判定**する（2026-09-03 分岐決定；2026-09-06 指示文法決定・実装）。

ヘッダー指示文法（`// key: value`、先頭 16 行以内、厳格なトークンマッチ）：

```text
// expect: compile-error E1002 [E1003 ...]   コンパイル期拒否テスト
// expect: runtime-error E6003 [...]         ランタイム失敗テスト
// skip: <理由>                              実行をスキップ、skipped に計上
// mode: embedded|standard|full              サブプロセス --runtime モード（run ステップのみ消費）
```

- `expect:` 指示なし = 振る舞いテスト。`expect:` は期待の**唯一の宣言**——2026-09-06 決定で
  `[test:error]` ブールマークと中文 `预期:`
  散文コード抽出を廃止：ブールマークと期待行は 2 つの疎結合事実であり、規律で一致を保つことは必ずドリフトする；英文厳格トークン文法は runner が機械的に解析できるようにし（kind + コードがすべて固定トークン、コード後に余分なトークンがあれば解析失敗）、解析失敗 = 実行せず直接 FAIL——指示宣言エラーの静かに劣化する経路は存在しない（構築期拒否）
- **コンパイルエラークラス**：単一ステップ `check`——必ず失敗し出力に全 `[EXXXX]`
  を含むこと；コンパイル成功 = FAIL（報告すべきなのに報告されない）、拒否されたがコード不一致 =
  FAIL。構文エラー（E1xxx 解析段）と意味エラー（E2xxx+）は独立カテゴリを設けない——期待コード自体が段階を固定する
- **ランタイムエラークラス**：2 ステップ判定——`check` は必ず**成功**（コンパイル期は罪なし）、`run`
  は必ず失敗し出力に全 `[EXXXX]` を含む。コンパイル期で炸裂 =
  FAIL（コンパイルエラークラスと方向が逆の重要判定、「コンパイルが偶然通過、ランタイムが侥幸失敗」の漏検を防ぐ）
- 業界と同型：Rust compiletest `//~ ERROR`、Go `// ERROR "regexp"`、GCC `dg-error`、Clang
  `expected-error`
  はいずれもフィクスチャコメント内で期待を宣言しハーネスが双方向比較する；これらが行レベルアンカーを採用するのは多診断コンパイラが同ファイルの複数期待を区別する必要があるからで、本コンパイラは初錯で停止し 1 ファイル 1 診断のため、ファイルレベルがコンパイラの現実に同型——エラー回復実装後、文法に行アンカー形態を追加できる（Cranelift
  filetests のファイルヘッダ指示 + 関数レベル期待は同じ混合形態）
- 既知のレンダリング負債：parse 期診断は現在 Debug 形態で出力され（`code: "E0012"` でなく
  `[E0012]`）、コードスキャンは両形態を受け入れる；診断レンダリング統一後に厳格形態を回収
- **本リポジトリのコーパスにのみ奉仕し、ユーザーテストフレームワークの一部ではない**；双 runner 判定規約は収束済み（2026-09-03）：yx_runner（cargo
  test）と `yaoxiang test` は `src/util/test_markers.rs`
  でヘッダー指示を解析、06-compile-errors のディレクトリ規約は廃止。レポート層はカテゴリカウントを提供：人間向け集計に
  `Categories:` 行、JSON summary に `by_kind`（behavior / compile-error / runtime-error /
  invalid、skipped は別計上）、各ファイルに `kind`

#### 8.3 ランタイム硬失敗（Result 化に帰属）

独立した機構は設けない——失敗しうる操作は言語の方向に従って `Result`
を返し、テストは統一的に §8.1 で表現。プロセスレベル abort（アサーション違反、ランタイム引数誤りなど）は Result 化の推進に伴い値に収斂し、テストフレームワークはこれに専用の意味論を提供しない。（注意：§8.2 の「ランタイムエラークラス」マーク判定は runner が**まだ Result 化できない操作**に対してファイルレベル検証を行う経路であり、本節の意味論方向と矛盾しない——後者は終点で、前者は移行期の経路）

### 9. テスト体系の階層化：言語コーパスとライブラリテスト（2026-09-03 決定）

テストは**被テスト対象**によって 2 層に分けられ、それぞれ帰属とメンテナが異なる；マークシステム（§8.2）とアサーションライブラリ（§3）は両層で汎用：

**第 1 層：言語可用性コーパス（`tests/yaoxiang/`）**

- 被テスト対象は**言語そのもの**——parser、型システム、モジュール、並列、所有権、コンパイル期拒否、ランタイム意味論；ディレクトリは言語仕様章に従って組織
- std はコーパス内で**アサーションツール**（`std.assert` /
  `std.test`）としてのみ機能し、テスト対象には決してならない——ライブラリの API 振る舞いは言語可用性に属さない
- コーパス内は §8 に従って振る舞いテスト / コンパイル期拒否テスト / ランタイム失敗テストの 3 類に分岐判定

**第 2 層：ライブラリテスト（ライブラリに随伴）**

- 被テスト対象は**ライブラリの公開 API 契約**（例：`list.push` 振る舞い、`result.code` 意味論）
- テストは**ライブラリ自身の package に記述**：std の package は `src/std/`
  で、その yx レベルテストは `src/std/tests/`
  に帰属（実装体と同じ場所；ディレクトリは Rust ユニットテストと共存、ファイル型は交わらない）；`std.test`
  自身のテストもここにある（std.test で std.test をテスト、セルフホスティング閉ループ）
- 将来ユーザーパッケージも同一慣例に従う：テストは package 内、パッケージの `[tool.test]`
  発見に従う（RFC-014 パッケージ管理のテスト配置をここで予行）
- 発見はデフォルト patterns に入らない（デフォルト `tests/**/*.yx`
  は言語層のみカバー）：ライブラリテスト層は明示的パス（`yaoxiang test src/std/tests`）またはパッケージ設定で発見；CI は階層別に実行

移行注記：**移行済み（2026-09-06）**——元 `tests/yaoxiang/07-std/`
の 19 ファイルは 1 つずつ甄別した結果、すべてライブラリテスト（被テスト対象はすべて std モジュールの API 契約；`?`
伝播、自動借用、泛型インスタンス化などの言語特性はその中での役割はキャリアであって被テスト対象ではない）で、全体を
`src/std/tests/` に移行し 07-std ディレクトリを撤销；yx_runner は双根発見（`tests/yaoxiang/` +
`src/std/tests/`）に変更、デフォルト patterns はライブラリ層を含まない（統合テストが当該契約を固化）。言語コーパスには此后 std-API テストはゼロ。

## 既存システムとの関係

| 項目                                             | 関係                                                                                                  |
| ------------------------------------------------ | ----------------------------------------------------------------------------------------------------- |
| Rust `#[test]`                                   | 不変、コンパイラ内部テストは Rust を継続使用                                                          |
| 既存の `.yx` 統合テスト（`tests/yaoxiang/`）     | `yaoxiang test` が発見・実行                                                                          |
| `std.assert.assert(cond)`                        | ランタイムガードのために保持；`std.test` 値意味論アサーション族は `std.result` に基づく（§3、§7）     |
| モジュールシステム（RFC-029）                    | 埋め込みソースモジュールは Registry/orchestrator 経由でアクセス；CLI `run` の orchestrator 接続が前提 |
| コーパス再構成（`io.println` → `assert.assert`） | `yaoxiang test` と完全に同じ方向                                                                      |
| `@` 注釈                                         | 使用せず、`@test` を導入しない                                                                        |

## 実装戦略

### Phase 1：核心機能

変更範囲：

- `src/util/diagnostic/mod.rs` / `src/main.rs` — CLI `Run` ソースブランチを `run_project`
  に委譲（複数ファイル実行の前提）
- `src/main.rs` — `Test` サブコマンドを新規追加
- `src/std/test.yx` — 純 YaoXiang モジュールを新規追加
- `build.rs` — `std/*.yx` をバイナリに埋め込み
- orchestrator / Registry — 埋め込みソースから仮想パスで `.yx` モジュールをロードすることをサポート
- RFC-015 設定解析 — `[tool.test]` セクション
- サブプロセス実行 + レポート

成果物：

- `yaoxiang test` が基本的に使用可能
- `std.test` 4 つのアサーション関数
- デフォルト `tests/**/*.yx` 発見
- シリアル実行 + デフォルト出力形式

### Phase 2：完備

- `--filter` / `--fail-fast` / `--verbose` パラメータ
- `--json` 出力（CI 統合）
- `--list` オプション
- `--no-progress` オプション

### Phase 3：高度化（2026-09-07 交付済み）

- `--parallel` 並列実行（ワーカープール + 各ファイル独立サブプロセス；`[tool.test].parallel`
  設定キーも同効）
- `[tool.test].exclude` 設定（プレフィックスヒット除外、`--list` も同様に除外）
- `assert_approx_eq`（Float 明示 eps アサーション、§3）

## リスクと緩和

| リスク                                         | 確率 | 緩和                                                                                                                                                                            |
| ---------------------------------------------- | ---- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `f"..."` が Any 上で補間失敗                   | 無   | 2026-08-02 実証済み（Int/String ともに正常）                                                                                                                                    |
| `yaoxiang.toml` 設定解析が現在の CLI にない    | 低   | 簡単な拡張で、核心機能に影響しない                                                                                                                                              |
| CLI run の orchestrator 接続で挙動回帰         | 低   | import なしの単一ファイルパスは等価；統合テストが orchestrator をカバー済み                                                                                                     |
| 埋め込み `.yx` ソースファイルでバイナリ増大    | 低   | `.yx` ソースファイルは非常に小さく、無視可能                                                                                                                                    |
| テストループの所要時間がコーパス増加に伴い増大 | 高   | 主因は各ファイルの全量コンパイル（185 ファイル実測 11.3s）、サブプロセス起動ではない；`--parallel` はプロセス側のみ緩和、コンパイルコストはテストループキャッシュスライスが必要 |

## オープン問題

- [x] `std/test.yx` 中の `use std.assert`
      参照は正しく解決できるか？——**解決済み（2026-08-02）**。モジュールシステム（RFC-029）実装後、native とソースモジュールは Registry に共存し、resolver が統一解決し、種類を跨いだ依存が自然に成立
- [x] テスト出力の `f"..."` の泛型 `to_string`
      は新しい型制約を導入するか？——**解決済み（2026-08-02）**。実証で标注なし引数（Any）上の
      `==`/`!=` と f-string 補間はいずれも動作（Int/String 検証通過）、新しい制約を導入しない
- [x] `?` 泛型引数の実現可能性？——**解決済み（2026-08-02）**：`?`
      型構文は現状存在せず（かつ静かに飲み込まれる、別 issue で追跡中）、Phase
      1 アサーション関数は标注なし引数を使い、泛型システムに依存しない

## 設計決定記録

| 決定                       | 決定                                                                                                                                                                                                                                                                              | 日付                     | 理由                                                                                                                                                                                                                                               |
| -------------------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ------------------------ | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| テストマーク方式           | `@test` 注釈を使用せず、テストファイルは通常の `.yx`                                                                                                                                                                                                                              | 2026-07-26               | コンパイラ変更ゼロ、サブプロセスで隔離                                                                                                                                                                                                             |
| アサーション方式           | `std.test` モジュール純 YaoXiang 関数                                                                                                                                                                                                                                             | 2026-07-26               | セルフホスティング、native コードなし                                                                                                                                                                                                              |
| テスト実行モデル           | サブプロセス `yaoxiang run <file>` + exit code                                                                                                                                                                                                                                    | 2026-07-26               | プロセスレベル隔離、コンパイラ変更ゼロ                                                                                                                                                                                                             |
| 標準ライブラリロード       | 当面はバイナリ埋め込み、将来ファイルシステム                                                                                                                                                                                                                                      | 2026-07-26               | バージョン結合、単一ファイル対応                                                                                                                                                                                                                   |
| アサーション引数型         | 标注なし引数（Any）、泛型システムに依存しない                                                                                                                                                                                                                                     | 2026-08-02               | `?` 型構文が存在しない；Any 上で比較・補間可能を実証                                                                                                                                                                                               |
| 複数ファイル実行           | CLI `run` を `run_project`（orchestrator）に委譲を前提とする                                                                                                                                                                                                                      | 2026-08-02               | サブプロセスモデルが CLI 能力を継承；use に沿ったオンデマンド発見は純粋なパフォーマンス最適化に退く                                                                                                                                                |
| レポートソース位置         | ランタイムエラーはデフォルトで携带                                                                                                                                                                                                                                                | 2026-09-07               | debug_map はデフォルトで生成、stack trace は `file:line:col` を出力；埋め込みモジュール（std.test）中継を経たフレームの帰属はここで保証されず、RFC-034 に属する                                                                                    |
| ネガティブテスト階層化     | 値レベル反転汎用 / コンパイル失敗 runner 構造化マーク（内部のみ）/ 硬失敗は Result 化に帰属                                                                                                                                                                                       | 2026-09-02               | 値化モデル決定；暗黙の [test:error] 規約に取って代わる                                                                                                                                                                                             |
| ファイル内複数テスト       | 値化標準モデル：テスト関数が Result を返し、スイートが per-test 判定を収集                                                                                                                                                                                                        | 2026-09-02               | catch なし、非入口呼び出し（入口は内部シナリオのみ）                                                                                                                                                                                               |
| Error コード               | Error に機械可読 `code` フィールドを追加                                                                                                                                                                                                                                          | 2026-09-02               | エラーコードアサーションをサポート；コンパイル期コードは runner 比較を経る                                                                                                                                                                         |
| アサーションライブラリ形態 | 値意味論族 7 関数実装、`Result(Void, String)` 契約；abort 過渡版削除                                                                                                                                                                                                              | 2026-09-03               | Void は規範 unit（`()` は空 Tuple で混用しない）；ネスト位置は Any 剛性、标注なし引数は native 泛型チェックを通過しない——引数は明示的に标注しなければならない（R1 プローブ実証）                                                                   |
| テスト体系階層化           | 言語コーパス（`tests/yaoxiang/`）とライブラリテスト（ライブラリ随伴、std → `src/std/tests/`）の 2 層；std はコーパス内でアサーションツールとしてのみ機能                                                                                                                          | 2026-09-03               | 被テスト対象が帰属とメンテナを決定；ライブラリテストはパッケージ配置に従い、RFC-014 の予行演習                                                                                                                                                     |
| ネガティブマーク分岐判定   | 期待カテゴリに従って分岐：コンパイルエラークラスは `check` 必ず失敗、ランタイムエラークラスは `check` 必ず通過 + `run` 必ず失敗；レポートはカテゴリカウントを提供                                                                                                                 | 2026-09-03（09-06 実装） | カテゴリ混在判定は「コンパイルが偶然通過、ランタイムが侥幸失敗」の漏検を起こす；期待コードが段階を固定し、構文エラーは独立カテゴリを設けない                                                                                                       |
| ヘッダー指示文法           | 期待は英文構造化指示で宣言（`// expect:` / `// skip:` / `// mode:`、厳格トークン文法、解析失敗で直接 FAIL）；`[test:error]` ブールマークと中文 `预期:` 散文コード抽出を廃止                                                                                                       | 2026-09-06               | 期待はフィクスチャ内容の属性であり、in-fixture 宣言は業界と同型（compiletest / Go / GCC / Clang いずれもそう）、中央リストは必ず腐る；ブールマーク + 期待行の 2 事実を規律で結合させるのは欠陥面；構造化文法は runner に人手を介さず機械判定させる |
| 並列実行モデル             | `--parallel` でコアごとにワーカープールを起動（各ファイルは依然として独立サブプロセス）、skip/invalid は発見順で先行処理、実行結果は完了順でストリーミング出力、JSON はパスでソート；`--fail-fast` はスケジュールを停止（進行中は完走計上）；CWD は依然共有、分離境界を拡張しない | 2026-09-07               | サブプロセスモデル下で並列 = OS スレッドスケジューリング spawn、yx 層並列不要；所要時間の主因は各ファイル全量コンパイル（リスク表）、並列はプロセス側のみ緩和——テストループキャッシュスライスが主緩和策                                            |

## 参考文献

- [RFC-014: パッケージ管理システム設計](../accepted/014-package-manager.md)
  — 標準ライブラリディレクトリ構造
- [RFC-015: 設定システム](../accepted/015-configuration-system.md) — `[tool.test]` 設定セクション
- [RFC-030: assert アサーション機構](../review/030-assert-mechanism.md) — 底層依存
- [Rust `#[test]` 機構](https://doc.rust-lang.org/book/ch11-01-writing-tests.html) — 参考設計
