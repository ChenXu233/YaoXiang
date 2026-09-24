---
title: 'RFC-036: std.test テストフレームワークと yaoxiang test コマンド'
status: '受け入れ済み'
author: '晨煦'
created: '2026-07-26'
updated: '2026-09-02'
accepted: '2026-08-02'
issue: '#94, #95, #221, #319'
---

# RFC-036: std.test テストフレームワークと yaoxiang test コマンド

## 概要

YaoXiang に標準テストフレームワーク `std.test` モジュールと `yaoxiang test` CLI サブコマンドを導入する。テストファイルは通常の `.yx` ファイルであり、子プロセスの exit code で全体通過/失敗を判定する。ファイル内部では複数のテスト関数をサポートしており、アサーション失敗は `Err` 値で表現される（値セマンティクス）。スイートが per-test 判定を収集する（§7）。`std.test` モジュールは純粋な YaoXiang で実装されており、最初の dogfooding ライブラリである。`yaoxiang test` は CLI ツールであり、コンパイラ機能ではない。parser、IR、バイトコード、エクゼキュータへの変更は関与しない。

## 動機

### なぜテストフレームワークが必要なのか？

現在の YaoXiang のテストカバレッジは Rust 側の `#[test]` と `tests/` 統合テストに依存している，这意味着：

1. 標準ライブラリ（std.math / std.list / std.dict / std.convert / std.io）のユニットテストは YaoXiang で記述できない
2. 標準ライブラリの各モジュールのユニットテストカバレッジが阻塞されている。利用可能なテストインフラがないため
3. 言語機能の回帰テスト（RFC-032 spawn セマンティクス変更など）に自動化手段がない

### 主な制約

- **17 キーワード鉄則**：新しいキーワードや構文構造を導入しない
- **コンパイラ変更ゼロ**：parser、IR、バイトコード、エクゼキュータに触れない
- **dogfooding 優先**：テストライブラリは YaoXiang で書き最初の dogfooding ライブラリとする

## アーキテクチャ

```
┌──────────────────────────────────────────────────────────────┐
│                    yaoxiang test                              │
│                                                              │
│  CLI層:  yaoxiang test [--filter --fail-fast --json ...]    │
│              │                                               │
│  発見層:    yaoxiang.toml 読み取り → [tool.test] patterns    │
│              デフォルト: tests/**/*.yx                       │
│              │                                               │
│  実行層:    各ファイルに対して: yaoxiang run <file>           │
│              exit code チェック → 直列実行                   │
│              │                                               │
│  レポート層:  PASS/FAIL → 集計                                │
│              --json / --verbose / --fail-fast サポート        │
│                                                              │
│  アサーション層: std.test (純粋 YaoXiang、dogfooding)         │
│              基盤: std.assert.assert                         │
│              診断: f"Expected {expected}, got {actual}"       │
└──────────────────────────────────────────────────────────────┘
```

### 基本原則

1. **テストフレームワークはコンパイラ機能ではなく、CLI ツールである** — `yaoxiang run` はすでに「テスト実行」が可能であり、`yaoxiang test` は単にすべてのファイルを実行しレポートを表示するものである
2. **コンパイラ変更ゼロ** — `@test` アノテーションスキャン、バイトコードメタデータセクション、エクゼキュータ特殊エントリを導入しない
3. **dogfooding** — `std.test` モジュールは純粋な YaoXiang で実装されており、基盤能力は `std.assert` / `std.result` から提供される
4. **テストファイルは通常の `.yx` ファイルである** — ファイルはサブプロセスで実行され、exit code で全体通過/失敗を判定する
5. **アサーション失敗は値であり、プロセスイベントではない** — テスト関数は `Result` を返し、アサーション失敗は `Err` で表現される。スイートが逐一 per-test 判定を収集する（§7）。プロセスレベルの abort はランタイムガーディアンに属し、テストアサーションには使用しない

## 詳細設計

### 1. CLI 設計

```
yaoxiang test [OPTIONS] [PATHS]

Arguments:
  [PATHS]...      テストファイルまたはディレクトリを指定（デフォルト: yaoxiang.toml から読み取り、それ以外の場合は tests/）

Options:
  --filter <NAME>     ファイル名が <NAME> を含むテストのみ実行
  --fail-fast         最初の失敗で停止
  --verbose, -v       各テストの詳細な stdout/stderr を表示
  --list              テストファイルのみ一覧表示、実行しない
  --no-progress       進捗出力を非表示（ヘッダーと PASS 行）；FAIL 明細と集計は保持（CI シナリオ）
  --json              JSON 形式の結果を出力（CI 統合用）
  --parallel          並列実行（コアごとに1つの worker；[tool.test].parallel と OR 結合）
```

#### 出力形式

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
  "summary": { "total": 3, "passed": 2, "failed": 1, "skipped": 0, "by_kind": { "behavior": 3, "compile-error": 0, "runtime-error": 0, "invalid": 0 }, "time_secs": 0.006 },
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

- 失敗ファイルは追加で `exit_code` と `stderr` を運ぶ（ANSI 除外後のサブプロセス診断、CI 証跡用）。`--verbose` と `--json` を組み合わせると、すべてのファイルに `stdout` / `stderr` が含まれる
- `--no-progress` は進捗出力のみ抑制（ヘッダーと PASS 行）。FAIL 明細と集計は常に表示され、失敗はサイレントにならない。`--list` はテストファイルパスを1行に1つずつ出力し、実行しない
- 各ファイルには `kind` を添付（behavior / compile-error / runtime-error / invalid、§8.2）。summary には `by_kind` 実行カウントを添付（固定4キー、skipped を含まない）。人間向け集計には `Categories:` カテゴリ分布行を添付
- `--parallel` では、人間の進捗行は**完了順**でストリーミング出力（ブロックは交错しない）、JSON `files` は `file` パスでソートして出力を安定させる（CI diff フレンドリ）
- ファイル内の per-test `tests` 配列は §7 スイート収集から取得し、値化モデル落地後に有効化
- 公式 CI（`.github/workflows/ci.yml` test job）は以下を消費：cargo 側で `--test integration`（CLI 統合）と `--test yx_runner`（双ルートコーパスガーディアン）を実行した後、`yaoxiang test --json --parallel` を分层実実行——デフォルトモード（言語コーパス）と明示的 `src/std/tests`（ライブラリ層）でそれぞれレポートを生成。集計表（total / passed / failed / skipped / time_secs と by_kind）を job summary に書き込み、失敗ファイルには `kind` / `exit_code` / `stderr` を証跡として出力、いずれかのスイートが非ゼロ終了でレッド判定

### 2. yaoxiang.toml 設定

`[tool.test]` 下に配置し、RFC-015 の `[tool.*]` サードパーティ拡張規約に準拠：

```toml
[project]
name = "my-project"

[tool.test]
patterns = ["tests/**/*.yx"]
exclude = ["tests/fixtures/**"]   # マッチしたものは発見セットから除外（--list も同様に除外）
parallel = true                   # 並列実行（--parallel フラグと OR 結合）
```

- デフォルト `patterns = ["tests/**/*.yx"]` — ユーザーはゼロ設定で箱から出すだけで使える
- `exclude` と `patterns` は同じ形態（リテラルパスまたは `root/**…`、すべてパスプレフィックスでマッチ）。exclude されたものはテストではなく、ランタイム動作検証が必要なフィクスチャは `yaoxiang run` で直接実行
- **単一ファイルモード（`yaoxiang test foo.yx`）は直接実行し、設定を読み取らない**——明示的 paths 下では `exclude`/`parallel` 設定キーが有効にならない（フラグは例外）
- 将来は独立リポジトリに分割する可能性あり（`[tool.test]` の位置は不变）

### 3. std.test モジュール（純粋 YaoXiang）

```yaoxiang
// std/test.yx — 純粋 YaoXiang テストアサーションライブラリ（値セマンティクス標準形態、2026-09-03 落地）
// 最初の dogfooding ライブラリ：YaoXiang のテストライブラリは YaoXiang で書く。

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

// assert_not と assert_false は同体；assert_err / assert_err_code は §8.1 を参照
```

- アサーション関数は**値セマンティクス**：失敗は `Err(診断情報)` で表現され、プロセスを abort しない。§7 スイートはこれに従って per-test 判定を収集する。`std.assert.assert` のプロセレベル abort セマンティクスはランタイムガーディアンに保持され、テストアサーションパスには入らない。Ok ペイロードは `Void`（type-system.md 仕様では unit。`()` は空 Tuple で両者は混用しない——2026-09-03 定案）
- **関数族 7 個（2026-09-03 すでに交付、abort 移行版は削除）**：値化
  `assert_eq` / `assert_ne` / `assert_true` / `assert_false` + `assert_not`
  （assert_false と同体、`!assert` の衣替え用）+ `assert_err`（§8.1）
  - `assert_err_code`（§8.1 エラーコードアサーション）
  - `assert_approx_eq(a: Float, b: Float, eps: Float)`（Phase 3、2026-09-07
    交付）：`|a - b| <= eps` 判定、eps は呼び出し側が**明示的に指定**——許容値はテスト契約の一部であり、非表示デフォルト値なし。負の eps は宣言時点で Err、NaN は常に Err
- `assert_eq` / `assert_ne` はパラメータに **Any 注釈を使用**——`==`/`!=` と f-string 補間は Any 上で正常に動作し、ジェネリクスシステムに依存しない。パラメータは**明示的に注釈が必要**：注釈なしパラメータは native ジェネリクス `&Result(T, E)` の呼び出しチェックを通らない（R1 プローブ実証済み）
- `assert_false` / `assert_not` は `cond == false` で否定を表現（`not` 単項構文は未落地、稳定后将迁移。`!assert` 単項形態はこの依存も同じ、§8.1 を参照）
- ブロック体 + 明示的 `return` 形態：if 式 then 腕の型はチェック時に破棄され、両腕 Result の if 式はチェックのBlind Spotであり、実装はこれを回避する
- `std.test` は native コードに依存せず、純粋 YaoXiang 実装

### 4. 標準ライブラリ載機構（重要な設計）

**Phase 1：バイナリへの埋め込み**

`std/test.yx`（および将来の YaoXiang で書かれたすべての標準ライブラリモジュール）はビルド時にバイナリに埋め込まれる：

```rust
// build.rs またはビルドスクリプト、自動生成
pub const STD_YX_FILES: &[(&str, &str)] = &[
    ("std/test.yx", r#"..."#),  // ソースコードテキスト
    // 将来もっと
];
```

モジュールシステム（RFC-029、2026-08-02 完全に落地）はエントリーを提供：Registry は native モジュールとソースモジュールの両方を保持し、orchestrator は複数ファイル構成を担当する。`use std.test` の解決順序：

1. まず Rust native モジュールを確認（既存メカニズム、`std.assert` など）
2. マッチなし、埋め込み `STD_YX_FILES` を確認——マッチしたら**仮想パス**（例：`<std>/test.yx`）をシードモジュールとして orchestrator に注入し、通常の前端パイプラインを通る（parse → typecheck → IR）
3. マッチなし、ファイルシステム発見を使用（ユーザーモジュール）

埋め込みソースモジュール内部の `use std.assert` は resolver が通常通り native registry に解決——native とソースモジュールは Registry 内で共存し、跨種依存が自然に成立する。埋め込みモジュールは**オンデマンドコンパイル**：import された時のみパイプラインに入る。

利点：

- 単一ファイルモードでも `use std.test` が動作
- 標準ライブラリバージョンはバイナリに厳密バインドされ、バージョン不一致がない
- ユーザーが標準ライブラリパスを設定する必要がない

**将来：ファイルシステム標準ライブラリ**

YaoXiang プロジェクトモードが成熟した後、標準ライブラリはファイルシステム形式に変更される。RFC-014 の更新を参照。

### 5. 発見と実行

**前提条件（2026-08-02 審査決議）**：CLI `run` は orchestrator に接続する。現状 CLI `run` は単一ファイルパイプライン（`run_file_with_diagnostics`）を通り、ユーザーモジュールの import を解決できない。しかし `yaoxiang test` のサブプロセスは CLI 機能を継承し、テストファイルがプロジェクトモジュールを import するのはコアシナリオである。だから Phase 1 ではまず CLI `Run` のソース分支を `run_project`（orchestrator、ディレクトリ再帰発見）に委任する。use に沿ったオンデマンド発見は後に純粋なパフォーマンス最適化として追加される。import なしの単一ファイルは orchestrator を通しても動作等价であり、バイトコード分支は変わらない。

**発見フェーズ**：

1. `[PATHS]` が指定されていれば、直接指定されたパスを使用
2. それ以外の場合は `yaoxiang.toml` の `[tool.test].patterns` を読み取り
3. 設定がなければ、デフォルト `tests/**/*.yx`
4. `--filter` フィルタを適用（ファイル名が含まれる）
5. 発見範囲はテスト分层（§9）である：デフォルト patterns は言語可用性コーパスのみカバー。ライブラリテスト層（例：`src/std/tests/`）は明示的パスまたはパッケージ設定で発見され、デフォルトスキャンに混入しない

**実行フェーズ**：

1. 各ファイルに対してヘッダー命令で実行を分流（命令文法については §8.2、`src/util/test_markers.rs` で解析、yx_runner と共用）：
   - 振る舞いテスト：`yaoxiang run <file>` サブプロセス
     （ランタイムエラーはデフォルトでソース位置とスタックフレームを含む——debug_map はデフォルトで生成、
     stack trace は `file:line:col` を出力）；`// mode:` でサブプロセス `--runtime` モードを宣言
   - コンパイル期否認型：単一ステップ `yaoxiang check <file>`
   - ランタイム失敗型：`check`（成功必須）+ `run`（失敗必須）の2ステップ
2. `// skip: <理由>` のファイルは実行をスキップし、報告の skipped に计入
3. 判定と予期コードの比較は §8.2 判定マトリックスによる。命令解析失敗は実行せず直接 FAIL（構築期否認）
4. stdout/stderr をキャプチャしてレポートに使用
5. デフォルトは直列実行；`--parallel`（または `[tool.test].parallel`）で利用可能なコア数だけ worker プールを起動、
   各ファイルは独立サブプロセス——skip/invalid は発見順で先に処理し、実行結果は完了順でストリーミング出力、JSON はパスでソート（Phase 3、2026-09-07 交付）
6. `--fail-fast` の場合、最初の FAIL で直ちに新しいファイルのスケジューリングを停止。並列モードでは実行中のファイルは完了まで走り、結果に计入

### 6. テスト隔離

テスト隔離はプロセレベル境界で自然に実装される：

- 各テストファイルは独立サブプロセスで実行
- 各サブプロセスは独立した Heap、Frame、NativeContext を持つ
- 1つのテストファイルの panic は他のテストファイルに影響しない
- 追加の独立 Heap コンテキストメカニズムは不要
- **並列実行（Phase 3）は隔離境界を扩展しない**：サブプロセス間の作業ディレクトリ（CWD）は共有される。
  並列テストは CWD 内の同じパスにファイルを使用してはならない。ファイル I/O テストは独立ファイル名を使用し、終了時にクリーンアップする

### 7. スイートと複数テスト（値化モデル）

1つのテストファイルには複数のテストを含められる。ファイル内組織（2026-09-03 落地済み）：

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

- 各テストは `Result(Void, String)` を返すゼロパラメータ関数である。アサーション失敗は `Err` で表現され（§3 値セマンティクスアサーション族）、プロセスを中断しない——後続テストは通常通り実行される
- `test.suite` は逐一呼び出して収集する：テストが Ok でなければ名前と診断を記録し、Ok ならサイレント。全実行後、いずれかの Err があれば `std.assert.assert` で中止し、失敗明細を添付（`N of M test(s) failed` + 各 `[FAIL] 名前: 診断`）——ファイルの exit code は非 0（§5 判定は変わらない）。这里的 abort はテストバイナリのランタイムガーディアンであり、アサーションパスではない。全 Ok ならサイレントに exit 0
- トップレベルテスト関数は**クロージャ形態で登録**（`("name", () => test_fn())`）。トップレベル関数名を値参照することは現在サポートされていない（IR レベルの制限、`E3006`）。クロージャ本体からのグローバル関数呼び出しは影響を受けない
- runner はファイルのみを見て、関数レベルスキャンは行わない。per-test 判定は完全にスイート内収集から得られ、ファイル内部構造は runner にとって透明——コンパイラ変更ゼロ原則に影響しない
- 明確に不採用：プロセス内 catch 境界（17 キーワード鉄則）。runner による逐一関数エントリ呼び出し（§8.2 コンパイル失敗などの内部シナリオにのみ適用）
- API 形態は定案済み（2026-09-03）：`suite(tests: List((String, () -> Result(Void, String)))) -> Void`。
  重複名は検出しない（名前はレポート表示のみに使用）。`--filter` はファイル名でフィルタリングし、スイート内のテスト名は感知しない

### 8. 負向テスト（予期失敗）三層設計

負向テストは失敗発生層に従って分割され、各層が帰属する：

#### 8.1 値レベル逆（汎用、エンドユーザー向け）

テスト対象操作が `Result` を返す場合、テストは通常のアサーションで予期失敗を表現：

```yaoxiang
r = range.iter(invalid_range)
test.assert_err(r)
e = result.unwrap_err(r)
test.assert_eq(result.code(e), "E6009")
// または1文でラップ（コードは std Error キャリアにのみ存在、E は Error に固定）：
test.assert_err_code(r, "E6009")
```

- `assert_not` / `assert_err` / `assert_err_code` は値セマンティクス族と一緒にすでに交付済み（2026-09-03、§3）。
  `!assert` 単項形態は not 構文落地後に提供（`assert_false` の `cond == false` と同じ制約）
- エラーコードアサーションは `Error` 値が機械可読な `code` フィールドを運ぶ必要がある——すでに落地済み：
  `Error = { code, message }`（native `error_new(code, message)`）。読み取りは
  `result.unwrap_err(r)` でキャリアを取得し、`result.code(e)` / `result.message(e)` アクセサーでアクセス
  （設計段階で推定された `error_new_with_code` 命名、コード定数エクスポート、`err.code` フィールドアクセス
  はすべて不採用——言語に Struct フィールドアクセスがなく、コード定数はエクスポートされていない）
- Result 化に伴い、失敗可能な操作は逐一 `Result` を返すようになり、コーパス内のファイルレベル
  負向マークがファイル内アサーションに移行していく

#### 8.2 ファイルレベル負向命令（言語設計者内部使用のみ）

コンパイルはファイル全体で全か無かで、ファイル内で「この行はコンパイルされない」を表現できない。ランタイム失敗，同样需要ファイルレベル
表現（例如スイートに失敗するテストがある場合）。ファイルヘッダーに**構造化命令**で予期を宣言し、runner は予期カテゴリに従って**分流判定**を行う
（2026-09-03 定案分流；2026-09-06 命令文法定案暨落地）。

ヘッダー命令文法（`// key: value`、先頭16行以内、厳格トークンマッチ）：

```text
// expect: compile-error E1002 [E1003 ...]   コンパイル期否認テスト
// expect: runtime-error E6003 [...]         ランタイム失敗テスト
// skip: <理由>                              実行をスキップし、skipped に计入
// mode: embedded|standard|full              サブプロセス --runtime モード（run ステップでのみ消費）
```

- `expect:` 命令なし = 振る舞いテスト。`expect:` は予期の**唯一の宣言**である——2026-09-06
  定案で `[test:error]` ブールマークと中国語 `预期:` 散文から碼を剥がす方式是廃止。
  ブールマークと予期行は2つの疎結合事実であり、纪律で整合を保とうとしても必ずずれる。
  英語厳格トークン文法なら runner は機械的に解析できる（kind + コードはすべて固定トークン、
  コード後に余分なトークンがあると解析失敗）。解析失敗 = 実行せず直接 FAIL——
  命令宣言錯誤のサイレント劣化チャネルは存在しない（構築期否認）
- **コンパイルエラー類**：単一ステップ `check`——失敗必須かつ出力がすべての `[EXXXX]` を含む必要あり。
  コンパイル成功 = FAIL（報告すべきものを報告しなかった）、拒否だがコード不合致 = FAIL。
  構文エラー（E1xxx 解析セグメント）と意味エラー（E2xxx+）は独立カテゴリを設けない——
  予期コード自体がセグメントを固定している
- **ランタイムエラー類**：2ステップ判定——`check` は**成功必須**（コンパイル期は無実）、
  `run` は失敗必須かつ出力がすべての `[EXXXX]` を含む。コンパイル期から爆発 = FAIL
  （コンパイルエラー類と逆方向の重要な判定、「コンパイルが期待に反して通過、ランタイムで幸運にも失敗」
  の漏れ検出を防止）
- 業界との同型：Rust compiletest `//~ ERROR`、Go `// ERROR "regexp"`、GCC
  `dg-error`、Clang `expected-error` はすべてフィクスチャコメント内に予期を宣言し、harness が双方比較する。
  它们采用行级アンカー是因为多診断コンパイラは同ファイル内の複数の予期を区別する必要があるが、
  当コンパイラは初エラーで停止、一ファイル一診断であり、ファイルレベルはコンパイラの現実に同型——
  エラー回復落地後に文法に行アンカー形態を追加できる（Cranelift filetests のファイルヘッダー命令 +
  関数レベル予期はこの同じ混合形態）
- 既知のレンダリング債務：parse 期診断は現在 Debug 形態で出力（`code: "E0012"` 而不是
  `[E0012]`）、コードスキャンは両方の形態を受け入れる。診断レンダリング統合後に厳格形態を取り戻す
- **本リポジトリコーパス専用であり、ユーザーテストフレームワークの一部ではない**。
  双 runner 判定契約はすでに閉口済み（2026-09-03）：yx_runner（cargo test）と `yaoxiang test` は
  `src/util/test_markers.rs` を共用してヘッダー命令を解析し、06-compile-errors のディレクトリ約定は廃止。
  レポート層はカテゴリカウントを提供する：人間向け集計には `Categories:` 行を追加し、
  JSON summary には `by_kind` を追加（behavior / compile-error / runtime-error / invalid、skipped は別途计数）、
  各ファイルには `kind` を添付

#### 8.3 ランタイムハード失敗（Result 化に 수용）

独立メカニズムはない——失敗する操作は言語方向に従って `Result` を返すようになり、
テストは §8.1 で統一的に表現する。プロセレベル abort（如アサーション違反、ランタイムパラメータ不正）は
Result 化に伴い徐々に値に收敛し、テストフレームワークはこれに专门语义を提供しない。
（注意：§8.2 の「ランタイムエラー類」マーク判定は runner が**まだ Result 化できない操作**に対して
行うファイルレベル検証チャネルであり、本節の意味方向と矛盾しない——後者は終点であり、
前者は移行期チャネルである）

### 9. テスト体系分层：言語コーパスとライブラリテスト（2026-09-03 定案）

テストは**テスト対象**に従って二層に分层し、各層には帰属とメンテナンス担当がある。
マークシステム（§8.2）とアサーションライブラリ（§3）は両レベルで共通：

**第一層：言語可用性コーパス（`tests/yaoxiang/`）**

- テスト対象は**言語そのもの**——パーサー、タイプシステム、モジュール、並行、所有権、コンパイル期否認、
  ランタイム意味。ディレクトリは言語仕様セクションで構成
- std はコーパス内で**アサーション工具としてのみ使用**（`std.assert` / `std.test`），
  決してテストされない——ライブラリの API 動作は言語可用性に属さない
- コーパス内 按 §8 で振る舞いテスト / コンパイル期否認テスト / ランタイム失敗テストの3類に分流して判定

**第二層：ライブラリテスト（ライブラリと共に）**

- テスト対象は**ライブラリの公開 API 契約**（例：`list.push` の動作、`result.code` の意味）
- テストは**ライブラリ自身の包里に書く**：std のパッケージは `src/std/` であり、
  その yx レベルテストは `src/std/tests/` に帰属する（実装体と同じ場所にある。
  ディレクトリは Rust ユニットテストと共存し、ファイルタイプは交差しない）。
  `std.test` 自身のテストもここにある（std.test で std.test をテスト、dogfooding 閉口）
- 将来のパライーブ者も同じ慣例を使用：テストはパッケージ内に置き、パッケージの `[tool.test]` で発見
  （RFC-014 パッケージ管理のテストレイアウトはこれでプレビュー済み）
- 発見はデフォルト patterns に入らない（デフォルト `tests/**/*.yx` は言語層のみカバー）。
  ライブラリテスト層は明示的パス（`yaoxiang test src/std/tests`）またはパッケージ設定で発見される。
  CI は分层実行

移行メモ：**すでに移行済み（2026-09-06）**——旧 `tests/yaoxiang/07-std/` の19ファイルは
逐一甄別後すべてライブラリテストであった（テスト対象はすべて std モジュールの API 契約。
`?` 伝播、自動借用、ジェネリクスインスタンス化が其中的役割はキャリア而非テスト対象であり）、
全体として `src/std/tests/` に移行し、07-std ディレクトリを撤销。
yx_runner は双ルート発見（`tests/yaoxiang/` + `src/std/tests/`）に改め、
デフォルト patterns はライブラリ層を含まない（統合テストがこの契約を固化）。
言語コーパスは从此ゼロ std-API テスト。

## 既存システムとの関係

| 項目                                          | 関係                              |
| --------------------------------------------- | --------------------------------- |
| Rust `#[test]`                                | そのまま、コーダ内部テストは Rust で継続   |
| 既存 `.yx` 統合テスト（`tests/yaoxiang/`）      | `yaoxiang test` に発見・実行される     |
| `std.assert.assert(cond)`                     | ランタイムガーディアンに保持；`std.test` 値セマンティクスアサーション族は `std.result` に改める（§3、§7） |
| モジュールシステム（RFC-029）                            | 埋め込みソースモジュールは Registry/orchestrator を経由して接続。CLI `run` が orchestrator に接続するのは前提 |
| コーパスリファクタリング（`io.println` → `assert.assert`） | `yaoxiang test` と完全に一致する方向 |
| `@` アノテーション                                      | 使用しない、`@test` を導入しない            |

## 実装戦略

### Phase 1：コア機能

変更範囲：

- `src/util/diagnostic/mod.rs` / `src/main.rs` — CLI `Run` ソース分支を `run_project` に委任（多ファイル実行前提）
- `src/main.rs` — 新規 `Test` サブコマンド
- `src/std/test.yx` — 新規純粋 YaoXiang モジュール
- `build.rs` — `std/*.yx` をバイナリに埋め込み
- orchestrator / Registry — 埋め込みソースから仮想パスで `.yx` モジュールをロードするサポート
- RFC-015 設定解析 — `[tool.test]` セクション
- サブプロセス実行 + レポート

成果物：

- `yaoxiang test` 基本動作可能
- `std.test` 4つのアサーション関数
- デフォルト `tests/**/*.yx` 発見
- 直列実行 + デフォルト出力形式

### Phase 2：完善

- `--filter` / `--fail-fast` / `--verbose` パラメータ
- `--json` 出力（CI 統合）
- `--list` オプション
- `--no-progress` オプション

### Phase 3：上級（2026-09-07 すでに交付）

- `--parallel` 並列実行（worker プール + 各ファイル独立サブプロセス。[tool.test].parallel` 設定キーも同効）
- `[tool.test].exclude` 設定（プレフィックスマッチで除外、`--list` も同様に除外）
- `assert_approx_eq`（Float 明示的 eps アサーション、§3）

## リスクと 완화

| リスク                                    | 確率 | 緩和                                        |
| --------------------------------------- | ---- | ------------------------------------------- |
| `f"..."` が Any での補間失敗              | なし   | 2026-08-02 すでに実証（Int/String とも正常）      |
| `yaoxiang.toml` 設定解析が現在の CLI にない | 低   | 単純な拡張であり、コア機能に影響しない                    |
| CLI run が orchestratorに接続引起的動作回帰    | 低   | import なし単一ファイルパスは同等。統合テストが orchestrator をカバーしている |
| 埋め込み `.yx` ソースファイルをバイナリに追加によるサイズ増加       | 低   | `.yx` ソースファイルは非常に小さく、無視可能                    |
| テスト回路時間がコーパス成長に伴い増加                  | 高   | 主要項目は各ファイルフルコンパイル（185ファイル実測 11.3s）であり、サブプロセス起動ではない。`--parallel` はプロセスの問題のみ缓解し、コンパイルコストはテスト回路キャッシュスライスが必要 |

## 開放問題

- [x] `std/test.yx` 中の `use std.assert` の参照は正しく解決できるか？——**すでに解決（2026-08-02）**。
      モジュールシステム（RFC-029）落地後 native とソースモジュールは Registry 内で共存し、resolver が統一的に解決し、跨種依存が自然に成立する
- [x] テスト出力中の `f"..."` のジェネリクス `to_string` が新しい型制約を導入するか？——**すでに解決（2026-08-02）**。
      実証済み：注釈なしパラメータ（Any）上の `==`/`!=` と f-string 補間はどちらも動作する（Int/String 実証済み）。新しい制約を導入しない
- [x] `?` ジェネリクスパラメータの可能性？——**すでに解決（2026-08-02）**：`?` 型構文は現時点では存在しない（かつサイレントに食べられる、
      すでに別 issue で追跡中）。Phase 1 アサーション関数は注釈なしパラメータを使用し、ジェネリクスシステムに依存しない

## 設計決定記録

| 決定         | 決定                                      | 日付       | 理由                       |
| ------------ | ----------------------------------------- | ---------- | -------------------------- |
| テストマーク方式 | `@test` アノテーションを使用せず、テストファイルは通常の `.yx` | 2026-07-26 | コンパイラ変更ゼロ、サブプロセス即隔離 |
| アサーション方式     | `std.test` モジュールは純粋 YaoXiang 関数           | 2026-07-26 | dogfooding、native コードなし       |
| テスト実行モデル | サブプロセス `yaoxiang run <file>` + exit code  | 2026-07-26 | プロセレベル隔離、コンパイラ変更ゼロ   |
| 標準ライブラリ載   | 現在はバイナリに埋め込み、将来はファイルシステム              | 2026-07-26 | バージョン、バインディング、単一ファイル可以使用       |
| アサーション参数型 | 注釈なしパラメータ（Any）、ジェネリクスシステムに依存しない         | 2026-08-02 | `?` 型構文が存在しない；Any 実証済み：比較可能、補間可能 |
| 多ファイル実行   | CLI `run` が `run_project`（orchestrator）を委任作為前提 | 2026-08-02 | サブプロセスは CLI 機能を継承する。use に沿ったオンデマンド発見は純粋なパフォーマンス最適化 |
| レポートソース位置 | ランタイムエラーはデフォルトで携带する                      | 2026-09-07 | debug_map はデフォルトで生成し、stack trace は `file:line:col` を出力。埋め込みモジュール（std.test）を経由するフレームの帰属はこの保証外、RFC-034 に属する |
| 負向テスト分层 | 値レベル逆は汎用 / コンパイル失敗 runner 構造化マーク（内部のみ）/ ハード失敗は Result 化に 수용 | 2026-09-02 | 値化モデル定案。[test:error] 暗黙の約束に取って代わる |
| ファイル内複数テスト | 値化標準モデル：テスト関数は Result を返し、スイートが per-test 判定を収集 | 2026-09-02 | catch なし、エントリ呼び出しなし（エントリは内部シナリオにのみ適用） |
| Error コード | Error に機械可読な `code` フィールドを追加            | 2026-09-02 | エラーコードアサーションをサポート。コンパイル期コードは runner で比較 |
| アサーションライブラリ形態 | 値セマンティクス族 7 関数落地、`Result(Void, String)` 契約。abort 移行版は削除 | 2026-09-03 | Void は仕様 unit（`()` は空 Tuple で混用しない）。ネスト位置は Any の刚性であり、注釈なしパラメータは native ジェネリクスチェックを通らない——パラメータは明示的に注釈が必要（R1 プローブ実証済み） |
| テスト体系分层 | 言語コーパス（`tests/yaoxiang/`）とライブラリテスト（ライブラリと共に、std → `src/std/tests/`）の二層に分层。std はコーパス内でアサーション工具としてのみ使用 | 2026-09-03 | テスト対象が帰属とメンテナンス担当を決める。ライブラリテストはパッケージレイアウトに伴い、RFC-014 をプレビューする |
| 負向マーク分流判定 | 予期カテゴリに従って分流：コンパイルエラー類は `check` が失敗必須、ランタイムエラー類は `check` が通過必須 + `run` が失敗必須。レポートにはカテゴリカウントを提供 | 2026-09-03（09-06 落地） | カテゴリ混判だと「コンパイルが期待に反して通過、ランタイムで幸運にも失敗」が漏れ検出される。予期コードがセグメントを固定し、構文エラーは独立カテゴリを設けない |
| ヘッダー命令文法 | 予期は英語構造化命令で宣言（`// expect:` / `// skip:` / `// mode:`、厳格トークン文法、解析失敗は直接 FAIL）。`[test:error]` ブールマークと中国語 `预期:` 散文から碼を剥がす方式是廃止 | 2026-09-06 | 予期はフィクスチャ内容の属性であり、フィクスチャ内宣言は業界と同型（compiletest / Go / GCC / Clang はすべてそう）。中央リストは腐る運命。ブールマーク + 予期行の2事実耦合は纪律依存であり欠陥面。構造化文法なら runner は機械的に判断し、人間の参加がない |
| 並列実行モデル | `--parallel` はコアごとに worker プールを起動（各ファイルはまだ独立サブプロセス）、skip/invalid は発見順で先に処理、実行結果は完了順でストリーミング出力、JSON はパスでソート。`--fail-fast` はスケジューリングを停止（実行中のものは完了まで実行して计入）。CWD は共有のまま、隔離境界を扩展しない | 2026-09-07 | サブプロセスモデル下では並列 = OS スレッドが spawn をスケジューリング、yx レベルの並行は不要。時間消費の主要項目は各ファイルフルコンパイル（リスク表）であり、並列はプロセスの問題のみ缓解——テスト回路キャッシュスライスが主要缓解である |

## 参考文献

- [RFC-014: パッケージ管理システム設計](../accepted/014-package-manager.md) — 標準ライブラリディレクトリ構造
- [RFC-015: 設定システム](../accepted/015-configuration-system.md) — `[tool.test]` 設定セクション
- [RFC-030: assert アサーション機構](./030-assert-mechanism.md) — 基盤依存
- [Rust `#[test]` 機構](https://doc.rust-lang.org/book/ch11-01-writing-tests.html) — 参考設計