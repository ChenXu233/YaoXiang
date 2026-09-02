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

## 要約

YaoXiang に標準テストフレームワーク `std.test` モジュールと `yaoxiang test`
CLI サブコマンドを導入する。テストファイルは通常の `.yx` ファイルであり、サブプロセスの exit
code で全体のパス/失敗を判定する。ファイル内部は複数のテスト関数をサポートし——アサーション失敗は
`Err` 値で表現され（値意味論）、スイートが per-test 判定を収集する（§7）。`std.test`
モジュールは純粋な YaoXiang で実装された、最初の dogfooding ライブラリである。`yaoxiang test`
は CLI ツールであり、コンパイラ機能ではない——parser、IR、バイトコード、実行器への変更は一切伴わない。

## 動機

### なぜテストフレームワークが必要か？

現在の YaoXiang のテストカバレッジは Rust 側の `#[test]` と `tests/`
統合テストに依存している。これは次のような問題を生む：

1. 標準ライブラリ（std.math / std.list / std.dict / std.convert /
   std.io）の単体テストを YaoXiang で記述できない
2. `#117 標準ライブラリ各モジュールの単体テストカバレッジ`
   が、利用可能なテスト基盤がないためブロックされている
3. 言語機能の回帰テスト（例：RFC-032 spawn 意味論変更）に自動化された手段がない

### 重要な制約

- **17 キーワード鉄則**：新しいキーワードや構文構造を一切導入しない
- **コンパイラ変更ゼロ**：parser、IR、バイトコード、実行器には触れない
- **セルフホスティング優先**：テストライブラリは YaoXiang で書き、最初の dogfooding ライブラリとする

## アーキテクチャ

```
┌──────────────────────────────────────────────────────────────┐
│                    yaoxiang test                              │
│                                                              │
│  CLI 層:   yaoxiang test [--filter --fail-fast --json ...]    │
│              │                                               │
│  発見層:    yaoxiang.toml を読む → [tool.test] patterns       │
│              デフォルト: tests/**/*.yx                        │
│              │                                               │
│  実行層:    各ファイルに対して: yaoxiang run <file>            │
│              exit code をチェック → 逐次実行                  │
│              │                                               │
│  報告層:    PASS/FAIL → 集計                                  │
│              --json / --verbose / --fail-fast をサポート      │
│                                                              │
│  アサーション層: std.test（純粋な YaoXiang、セルフホスティング）│
│              基盤: std.assert.assert                          │
│              診断: f"Expected {expected}, got {actual}"       │
└──────────────────────────────────────────────────────────────┘
```

### 中核原則

1. **テストフレームワークはコンパイラ機能ではなく、CLI ツールである** — `yaoxiang run`
   はすでに「テストを実行できる」のであり、`yaoxiang test`
   はすべてのファイルを走らせてレポートを見せるだけである
2. **コンパイラ変更ゼロ** — `@test`
   アノテーションスキャン、バイトコードメタデータセグメント、実行器の特殊入口を導入しない
3. **セルフホスティング** — `std.test` モジュールは純粋な YaoXiang で実装され、基盤機能は
   `std.assert` / `std.result` から得られる
4. **テストファイルは通常の `.yx` ファイルである** — ファイルはサブプロセスとして実行され、exit
   code で全体のパス/失敗を判定する
5. **アサーション失敗はイベントではなく値である** — テスト関数は `Result` を返し、アサーション失敗は
   `Err`
   で表現される。スイートが per-test 判定を逐次収集する（§7）。プロセスレベルの abort はランタイムガードにのみ属し、テストアサーションには用いない

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
  --list              テストファイルを列挙するだけで実行しない
  --no-progress       進捗出力（ヘッダと PASS 行）を表示しない；FAIL 明細と集計は残す（CI 向け）
  --json              JSON 形式で結果を出力（CI 連携用）
```

#### 出力形式

**デフォルト出力**（per-test 判定はファイル内スイート収集による、§7 参照）：

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
  `stderr`（ANSI 除去済みのサブプロセス診断、CI のフォレンジック用）を付与する； `--verbose` と
  `--json` の組み合わせでは全ファイルに `stdout` / `stderr` を付与する
- `--no-progress`
  は進捗出力（ヘッダと PASS 行）のみを抑制する——FAIL 明細と集計は常に出力され、失敗を静かにしない；
  `--list` は各行にテストファイルパスを 1 つずつ出力し、実行はしない
- ファイル内の per-test `tests`
  配列は §7 のスイート収集によるもので、値化モデルと共に実装される（#319）

### 2. yaoxiang.toml 設定

`[tool.test]` 配下に配置し、RFC-015 の `[tool.*]` サードパーティ拡張規約に従う：

```toml
[project]
name = "my-project"

[tool.test]
patterns = ["tests/**/*.yx"]
# 将来の拡張:
# exclude = ["tests/fixtures/**"]
# parallel = true
```

- デフォルト `patterns = ["tests/**/*.yx"]` — ユーザー設定ゼロで即座に使える
- 単一ファイルモード（`yaoxiang test foo.yx`）は直接実行し、設定は読まない
- 将来的に別リポジトリに分割する可能性あり（`[tool.test]` の位置は不変）

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

- アサーション関数は**値意味論**：失敗を `Err(診断情報)` で表現し、プロセスは abort しない——
  §7 のスイートが per-test 判定を収集する。`std.assert.assert`
  のプロセスレベル abort 意味論はランタイムガード用に温存し、テストアサーション経路には持ち込まない
- 移行メモ：Phase 1 で実装する 4 関数は
  `std.assert.assert`（abort 意味論）ベースであり、セルフホスティングへの過渡実装である；値意味論族が本 RFC の標準形であり、実装後に置換する（#319）
- `assert_eq` / `assert_ne` は**無注釈パラメータ**（`Any`）を用いる——2026-08-02 実証：`==`/`!=`
  と f-string 補間が Any 上で正常に動作する（Int/String ともに検証済み）、**型システム（generics）に依存しない**。将来的に generics が利用可能になれば注釈を補完可能
- `assert_false` は否定に `cond == false` を用いる（`not` 単項構文は未実装のため、安定後に移行可能；
  `!assert` 単項形も同種の制約に依存する、§8.1 参照）
- エラーコードアサーション（`assert(err.code == "E3017")`）は、`Error` 値が機械可読な `code`
  フィールドを保持することに依存する（§8.1）
- `std.test` はいかなる native コードにも依存せず、純粋な YaoXiang 実装である

### 4. 標準ライブラリのロード機構（重要な設計）

**Phase 1：バイナリへの埋め込み**

`std/test.yx`（および将来的に YaoXiang で書かれるすべての標準ライブラリモジュール）はビルド時にバイナリへ埋め込まれる：

```rust
// build.rs またはビルドスクリプトで自動生成
pub const STD_YX_FILES: &[(&str, &str)] = &[
    ("std/test.yx", r#"..."#),  // ソースコードテキスト
    // 将来的にさらに追加
];
```

モジュールシステム（RFC-029、2026-08-02 に完全に実装済み）が接続点を提供する：Registry は native モジュールとソースモジュールの双方を保持し、orchestrator が複数ファイルの編成を担当する。`use std.test`
の解決順序：

1. まず Rust の native モジュール（既存メカニズム、例：`std.assert`）を照会
2. ヒットしなければ、埋め込まれた `STD_YX_FILES` を照会——ヒットすれば**仮想パス**
   （例：`<std>/test.yx`）をシードモジュールとして orchestrator に注入し、通常のフロントエンドパイプライン（parse
   → typecheck →IR）を経由する
3. ヒットしなければ、ファイルシステム探索（ユーザーモジュール）

埋め込まれたソースモジュール内部の `use std.assert` は resolver によって正常に native
registry に解決される——native とソースモジュールは Registry 内で共存し、種類を跨ぐ依存は自然に成立する。埋め込みモジュールは**オンデマンドでコンパイル**される：import されたときのみパイプラインに入る。

利点：

- 単一ファイルモードでも `use std.test` が機能する
- 標準ライブラリのバージョンがバイナリと厳密に紐づき、バージョン不整合が発生しない
- ユーザーが標準ライブラリのパスを設定する必要がない

**将来：ファイルシステム標準ライブラリ**

YaoXiang のプロジェクトモードが成熟した後、標準ライブラリはファイルシステム形式に移行する。詳細は RFC-014 の更新を参照。

### 5. 発見と実行

**前提条件（2026-08-02 レビュー決議）**：CLI `run` を orchestrator に接続する。現状の CLI `run`
は単一ファイルパイプライン（`run_file_with_diagnostics`）を通るため、ユーザーモジュールの import を解決できない。一方、`yaoxiang test`
のサブプロセスモデルは CLI の能力を継承し、テストファイルがプロジェクトモジュールを import することが中心的なシナリオである。したがって Phase
1 ではまず CLI `Run` のソースコード分岐を
`run_project`（orchestrator、ディレクトリ再帰探索）に委譲する；#247（use に沿ったオンデマンド探索）はその後の純粋な性能最適化として重ねる。import のない単一ファイルは orchestrator でも振る舞いが等価であり、バイトコード分岐は変わらない。

**発見フェーズ**：

1. `[PATHS]` が指定されていれば、そのパスを直接使用
2. そうでなければ `yaoxiang.toml` の `[tool.test].patterns` を読み取る
3. 設定がなければ、デフォルトの `tests/**/*.yx` を使用
4. `--filter` でフィルタを適用（ファイル名に含まれるかどうか）

**実行フェーズ**：

1. 各ファイルに対して：`yaoxiang run --debug-info <file>` でサブプロセスを起動（`--debug-info`
   によりランタイムエラーにソース位置が付与される——2026-08-02 実証で stack trace が `file:line:col`
   を出力することを確認済み）
2. exit code を確認：0 が PASS、0 以外が FAIL
3. レポート用に stdout/stderr をキャプチャ
4. 逐次実行のみ（Phase 1）、将来的に `--parallel` をサポート
5. `--fail-fast` が指定されていれば、最初の FAIL で即座に停止

### 6. テスト分離

テスト分離はプロセスレベルの境界によって自然に実現される：

- 各テストファイルは独立したサブプロセスで実行される
- 各サブプロセスは独立した Heap、Frame、NativeContext を有する
- あるテストファイルのパニックは他のテストファイルに影響しない
- 独立した Heap コンテキスト機構を追加で必要としない

### 7. スイートと複数テスト（値化モデル）

1 つのテストファイルは複数のテストを含むことができる。ファイル内の構成例：

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

- 各テストは `Result((), String)` を返すゼロ引数関数である；アサーション失敗は `Err`
  で表現され（§3 値意味論アサーション族）、プロセスを中断しない——後続のテストは通常通り実行される
- `test.suite`
  が逐次呼び出して収集する：あるテストが Ok 以外ならそのテスト名と診断情報を出力し、Ok は静かである
- ファイルの exit
  code：スイートがすべて Ok なら 0；1 つでも Err があれば非 0（§5 実行フェーズの exit
  code 判定は不変）
- runner はファイル単位でのみ認識し、関数レベルのスキャンは行わない：per-test 判定は完全にスイート内部の収集によるものであり、ファイル内部の構造は runner に対して透過的である——コンパイラ変更ゼロの原則に影響しない
- 明示的に採用しないもの：プロセス内 catch 境界（17 キーワード鉄則）；runner からの関数ごとのエントリ呼び出し（§8.2 コンパイル失敗などの内部シナリオに限定）
- `test.suite` の具体的な API 形（シグネチャ、重複名処理、`--filter`
  とスイート名の相互作用）は実装詳細であり、実装時に #319 で決定する

### 8. ネガティブテスト（期待される失敗）の 3 層設計

ネガティブテストは失敗が発生する層ごとに分割し、それぞれを適切な位置に配置する：

#### 8.1 値レベル反転（汎用、ユーザー向け）

被験操作が `Result` を返し、テストは通常の assert で期待される失敗を表現する：

```yaoxiang
r = range.iter(invalid_range)
test.assert_err(r)
test.assert_eq(result.err_code(r), "E3017")
```

- std.test に `assert_not` / `assert_err` 関数族を追加する；`!assert`
  単項形は not 構文の実装後に提供する（`assert_false` の `cond == false` と同種の制約）
- エラーコードアサーションは `Error` 値の機械可読な `code` フィールド拡張に依存する：
  `Struct { message }` を `{ code, message }` に拡張し（native
  `error_new_with_code`、std からのコード定数のエクスポート）、`err.code == "E3017"`
  のアサーションを可能にする
- Result 化の進展（#301、#316）に伴い、失敗しうる操作が順次 `Result`
  を返すようになれば、コーパス中のファイルレベル負マーカーもファイル内アサーションへと移行する

#### 8.2 コンパイル失敗（言語設計者の内部使用のみ）

コンパイルは全ファイル単位の全か無かであり、ファイル内で「この行はコンパイルされないはず」を表現できない。ファイルレベル特殊マーカーは保持する：

- `[test:error]` マーカーは runner によって読み取られ、逆方向に判定される（run の exit code が非 0 =
  PASS）
- マーカーは**構造化期待コード**に昇格する：runner は先頭の `expected: compile-error EXXXX`
  を解析し、コンパイラの stderr 出力にある `[EXXXX]`
  の実際の値と照合し、コードが一致しなければ FAIL とする（trybuild の stderr スナップショット思想の軽量版、コンパイラ変更ゼロ）
- **本リポジトリのコーパスのみを対象とし、ユーザーテストフレームワークの一部ではない**；実装側は 2 つの runner 判定規約（ディレクトリ規約 vs ヘッダーマーカー）の統一が必要である、#319 参照

#### 8.3 ランタイムハード失敗（Result 化に帰属）

独立した機構は設けない——失敗しうる操作は言語の方向性に従って `Result`
を返し（#301、#316）、テストは統一的に §8.1 で表現する。プロセスレベル abort（アサーション違反、ランタイム引数誤りなど）は Result 化の進展に伴い順次値に収斂し、テストフレームワークはそれ専用の意味論を提供しない。

## 既存システムとの関係

| 項目                                                | 関係                                                                                                |
| --------------------------------------------------- | --------------------------------------------------------------------------------------------------- |
| Rust `#[test]`                                      | 不変、コンパイラ内部テストは引き続き Rust を使用                                                    |
| 既存の `.yx` 統合テスト（`tests/yaoxiang/`）        | `yaoxiang test` によって発見・実行される                                                            |
| `std.assert.assert(cond)`                           | ランタイムガード用に温存；`std.test` の値意味論アサーション族は `std.result` ベースに変更（§3、§7） |
| モジュールシステム（RFC-029）                       | 埋め込みソースモジュールは Registry/orchestrator 経由で接続；CLI `run` の orchestrator 接続が前提   |
| `#200` リファクタ（`io.println` → `assert.assert`） | `yaoxiang test` と完全に同じ方向性                                                                  |
| `@` アノテーション                                  | 使用せず、`@test` を導入しない                                                                      |

## 実装戦略

### Phase 1：中核機能

変更範囲：

- `src/util/diagnostic/mod.rs` / `src/main.rs` — CLI `Run` のソースコード分岐を `run_project`
  に委譲（複数ファイル実行の前提）
- `src/main.rs` — 新規 `Test` サブコマンド
- `src/std/test.yx` — 新規純粋 YaoXiang モジュール
- `build.rs` — `std/*.yx` をバイナリへ埋め込み
- orchestrator / Registry — 埋め込みソースから仮想パスで `.yx` モジュールをロードすることをサポート
- RFC-015 設定解析 — `[tool.test]` セクション
- サブプロセス実行（`--debug-info`）+ レポート

成果物：

- `yaoxiang test` が基本的に利用可能
- `std.test` の 4 つのアサーション関数
- デフォルトの `tests/**/*.yx` 検出
- 逐次実行 + デフォルト出力形式

### Phase 2：完成度向上

- `--filter` / `--fail-fast` / `--verbose` オプション
- `--json` 出力（CI 連携）
- `--list` オプション
- `--no-progress` オプション

### Phase 3：発展

- `--parallel` 並列実行（spawn 並行モデル完成に依存）
- `[tool.test].exclude` 設定
- より多くのアサーション関数（例：Float 用の `assert_approx_eq`）

## リスクと緩和

| リスク                                             | 確率 | 緩和                                                                                                                                                                                                          |
| -------------------------------------------------- | ---- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `f"..."` の Any 上での補間失敗                     | なし | 2026-08-02 に実証済み（Int/String ともに正常）                                                                                                                                                                |
| `yaoxiang.toml` 設定解析が現在の CLI にない        | 低   | 単純な拡張であり、中核機能に影響しない                                                                                                                                                                        |
| CLI run の orchestrator 接続による挙動回帰         | 低   | import のない単一ファイル経路は等価；統合テストがすでに orchestrator をカバー                                                                                                                                 |
| `.yx` ソースファイルのバイナリ埋め込みによる肥大化 | 低   | `.yx` ソースファイルは極めて小さく、無視できる                                                                                                                                                                |
| コーパス増加に伴うテストループ時間の増大           | 高   | 主な要因はファイル単位のフルコンパイル（185 ファイル実測 11.3s）であり、サブプロセス起動ではない；`--parallel` はプロセス側のみを緩和し、コンパイルコストはテストループキャッシュ（#251/#293 スライス）が必要 |

## オープンクエスチョン

- [x] `std/test.yx` 内の `use std.assert`
      の参照が正しく解決されるか？——**解決済み（2026-08-02）**。モジュールシステム（RFC-029）の実装後、native とソースモジュールは Registry 内で共存し、resolver が一様に解決し、種類を跨ぐ依存は自然に成立する
- [x] テスト出力における `f"..."` のジェネリックな `to_string`
      は新しい型制約を導入するか？——**解決済み（2026-08-02）**。無注釈パラメータ（Any）上で
      `==`/`!=`
      と f-string 補間がともに動作することを実証（Int/String 検証済み）、新しい制約は導入されない
- [x] `?` ジェネリックパラメータの実現可能性？——**解決済み（2026-08-02）**：`?`
      型構文は現状存在せず（かつ暗黙的に飲み込まれるため、別 issue で追跡）、Phase
      1 のアサーション関数は無注釈パラメータを使用し、型システム（generics）に依存しない

## 設計決定記録

| 決定                   | 決定内容                                                                                               | 日付       | 理由                                                                                                                                                 |
| ---------------------- | ------------------------------------------------------------------------------------------------------ | ---------- | ---------------------------------------------------------------------------------------------------------------------------------------------------- |
| テストマーカー方式     | `@test` アノテーションを使用せず、テストファイルは通常の `.yx`                                         | 2026-07-26 | コンパイラ変更ゼロ、サブプロセスが隔離を兼ねる                                                                                                       |
| アサーション方式       | `std.test` モジュールの純粋な YaoXiang 関数                                                            | 2026-07-26 | セルフホスティング、native コードなし                                                                                                                |
| テスト実行モデル       | サブプロセス `yaoxiang run <file>` + exit code                                                         | 2026-07-26 | プロセスレベル隔離、コンパイラ変更ゼロ                                                                                                               |
| 標準ライブラリロード   | 現在はバイナリ埋め込み、将来はファイルシステム                                                         | 2026-07-26 | バージョン紐付け、単一ファイルで利用可能                                                                                                             |
| アサーション引数型     | 無注釈パラメータ（Any）、型システム（generics）に依存しない                                            | 2026-08-02 | `?` 型構文は存在しない；Any は比較・補間可能と実証済み                                                                                               |
| 複数ファイル実行       | CLI `run` を `run_project`（orchestrator）に委譲を前提とする                                           | 2026-08-02 | サブプロセスモデルが CLI の能力を継承；#247 は純粋な性能最適化に後退                                                                                 |
| レポートのソース位置   | サブプロセスに `--debug-info` を付与                                                                   | 2026-08-02 | stack trace が `file:line:col` を出力することを実証；埋め込みモジュール（std.test）を経由するフレーム帰属はこの保証外であり、#289 + RFC-034 に属する |
| ネガティブテストの層化 | 値レベル反転が汎用 / コンパイル失敗は runner 構造化マーカー（内部のみ） / ハード失敗は Result 化に帰属 | 2026-09-02 | #319 で決定；暗黙的な [test:error] 規約に代わる                                                                                                      |
| ファイル内複数テスト   | 値化標準モデル：テスト関数は Result を返し、スイートが per-test 判定を収集                             | 2026-09-02 | catch なし、エントリ呼び出しなし（エントリは内部シナリオのみ）                                                                                       |
| Error コード           | Error に機械可読な `code` フィールドを追加                                                             | 2026-09-02 | エラーコードアサーションを支える；コンパイル時コードは runner 比較方式                                                                               |

## 参考文献

- [RFC-014: パッケージ管理システム設計](../accepted/014-package-manager.md)
  — 標準ライブラリのディレクトリ構造
- [RFC-015: 設定システム](../accepted/015-configuration-system.md) — `[tool.test]` 設定セクション
- [RFC-030: assert メカニズム](../review/030-assert-mechanism.md) — 基底依存
- [Rust `#[test]` メカニズム](https://doc.rust-lang.org/book/ch11-01-writing-tests.html) — 参考設計
